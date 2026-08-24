// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::{
    BlockContext, BlockMatchCache, CompiledConditions, CompiledRuleSet, ExactCommutationCache,
    ExactCommutationMemo, PatchPlanStep, ReplacementItem, RewritePatch,
    SerialExactCommutationCache, operations_commute, patch_application_plan, scan_anchor,
    select_candidate_patches, select_rewrites_for_anchor_ranges,
};
use crate::circuit::{Circuit, Instruction, Parameter, ParameterValue, Qubit, StandardGate};
use crate::compile::knowledge::matcher::conditions_hold as knowledge_conditions_hold;
use crate::compile::knowledge::matcher::{ConcreteOperationView, MatchBindings, match_rule_item};
use crate::compile::knowledge::rule::{Condition, RuleItem};
use crate::compile::knowledge::{RuleKind, RuleLibrary};
use crate::compile::transform::rewrite::RewriteConfig;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

fn builtin_rules() -> CompiledRuleSet {
    CompiledRuleSet::from_library(RuleLibrary::builtin_rules().unwrap()).unwrap()
}

fn numeric_binding(symbol: &str, value: Parameter) -> MatchBindings {
    let item = RuleItem::standard(
        StandardGate::RX,
        &[0],
        vec![ParameterValue::Param(Parameter::symbol(symbol))],
    );
    let instruction = Instruction::Standard(StandardGate::RX);
    let params = [value];
    let mut bindings = MatchBindings::new();
    assert!(
        match_rule_item(
            &item,
            ConcreteOperationView::new(&instruction, &[Qubit::new(0)], &params),
            &mut bindings,
        )
        .unwrap()
    );
    bindings
}

#[test]
fn compiled_numeric_conditions_match_symbolic_modulo_semantics() {
    let theta = Parameter::symbol("theta");
    let conditions = [Condition::EqMod(
        theta,
        Parameter::pi(),
        Parameter::from(4.0) * Parameter::pi(),
    )];
    let compiled = CompiledConditions::compile(&conditions).unwrap();

    assert_eq!(
        compiled.evaluate(&numeric_binding("theta", Parameter::pi())),
        Some(true)
    );
    assert_eq!(
        compiled.evaluate(&numeric_binding("theta", Parameter::from(0.25))),
        Some(false)
    );
    assert_eq!(
        compiled.evaluate(&numeric_binding("theta", Parameter::symbol("runtime"))),
        None
    );

    let condition_sets = [
        vec![Condition::Eq(
            Parameter::symbol("theta") * Parameter::from(2.0),
            Parameter::pi(),
        )],
        vec![Condition::EqMod(
            Parameter::symbol("theta"),
            Parameter::pi(),
            Parameter::from(4.0) * Parameter::pi(),
        )],
        vec![Condition::EqMod(
            Parameter::symbol("theta"),
            Parameter::from(1.0),
            Parameter::from(0.0),
        )],
    ];
    for conditions in condition_sets {
        let compiled = CompiledConditions::compile(&conditions).unwrap();
        for value in [
            -3.0 * std::f64::consts::PI,
            0.0,
            0.5 * std::f64::consts::PI,
            std::f64::consts::PI,
            5.0 * std::f64::consts::PI,
            1.0 + crate::compile::PARAMETER_EQ_TOLERANCE / 2.0,
            1.0 + crate::compile::PARAMETER_EQ_TOLERANCE * 2.0,
        ] {
            let bindings = numeric_binding("theta", Parameter::from(value));
            assert_eq!(
                compiled.evaluate(&bindings),
                Some(knowledge_conditions_hold(Some(&conditions), &bindings)),
                "conditions={conditions:?}, value={value}"
            );
        }
    }
}

#[test]
fn builtin_rule_model_is_qubit_bijection_invariant() {
    assert!(builtin_rules().is_qubit_bijection_invariant());
}

#[derive(Default)]
struct CountingExactCommutationCache {
    entries: RefCell<HashMap<(u64, u64), bool>>,
    computations: AtomicUsize,
}

impl ExactCommutationCache for CountingExactCommutationCache {
    fn exact_or_insert_with(
        &self,
        lhs_id: u64,
        rhs_id: u64,
        compute: impl FnOnce() -> bool,
    ) -> bool {
        let key = if lhs_id <= rhs_id {
            (lhs_id, rhs_id)
        } else {
            (rhs_id, lhs_id)
        };
        if let Some(result) = self.entries.borrow().get(&key) {
            return *result;
        }
        self.computations.fetch_add(1, Ordering::Relaxed);
        let result = compute();
        self.entries.borrow_mut().insert(key, result);
        result
    }
}

fn select_with_test_cache(
    operations: &[crate::circuit::Operation],
    block_cache: &BlockMatchCache,
    rules: &CompiledRuleSet,
    config: &RewriteConfig,
    commutation_cache: &CountingExactCommutationCache,
) -> Vec<RewritePatch> {
    let block = BlockContext::new(operations, block_cache).unwrap();
    let mut candidates = Vec::new();
    for anchor in 0..block.len() {
        candidates
            .extend(scan_anchor(&block, anchor, rules, config, None, commutation_cache).unwrap());
    }
    select_candidate_patches(candidates, block.len(), None).unwrap()
}

fn apply_cancellation_patches(
    operations: &[crate::circuit::Operation],
    patches: &[RewritePatch],
) -> Vec<crate::circuit::Operation> {
    let mut rewritten = Vec::new();
    for step in patch_application_plan(operations.len(), patches).unwrap() {
        match step {
            PatchPlanStep::Replacements(patch) => assert!(patch.replacements.is_empty()),
            PatchPlanStep::DropMatched => {}
            PatchPlanStep::Keep(position) => rewritten.push(operations[position].clone()),
        }
    }
    rewritten
}

fn patch_signatures(patches: &[RewritePatch]) -> Vec<(usize, Vec<usize>, usize)> {
    patches
        .iter()
        .map(|patch| {
            (
                patch.rule_id,
                patch.matched_positions.clone(),
                patch.replacements.len(),
            )
        })
        .collect()
}

#[test]
fn serial_cache_normalizes_both_directions_and_caches_negative_results() {
    let memo = ExactCommutationMemo::new();
    let cache = SerialExactCommutationCache::new(&memo);
    let computations = AtomicUsize::new(0);

    assert!(cache.exact_or_insert_with(2, 7, || {
        computations.fetch_add(1, Ordering::Relaxed);
        true
    }));
    assert!(cache.exact_or_insert_with(7, 2, || {
        computations.fetch_add(1, Ordering::Relaxed);
        false
    }));
    assert!(!cache.exact_or_insert_with(3, 9, || {
        computations.fetch_add(1, Ordering::Relaxed);
        false
    }));
    assert!(!cache.exact_or_insert_with(9, 3, || {
        computations.fetch_add(1, Ordering::Relaxed);
        true
    }));

    assert_eq!(computations.load(Ordering::Relaxed), 2);
}

#[test]
fn concurrent_positive_and_negative_queries_compute_each_pair_once() {
    let memo = ExactCommutationMemo::new();
    let closure_calls = AtomicUsize::new(0);

    let results = (0..256)
        .into_par_iter()
        .map(|index| {
            let positive = index % 2 == 0;
            let (lhs, rhs) = if positive { (11, 29) } else { (31, 47) };
            let (lhs, rhs) = if index % 4 < 2 {
                (lhs, rhs)
            } else {
                (rhs, lhs)
            };
            memo.exact_or_insert_with(lhs, rhs, || {
                closure_calls.fetch_add(1, Ordering::Relaxed);
                positive
            })
        })
        .collect::<Vec<_>>();

    for (index, result) in results.into_iter().enumerate() {
        assert_eq!(result, index % 2 == 0);
    }
    assert_eq!(closure_calls.load(Ordering::Relaxed), 2);
}

#[test]
fn real_commuting_match_computes_only_unique_source_pairs() {
    let qubit = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(qubit, 0.125).unwrap();
    circuit.s(qubit).unwrap();
    circuit.t(qubit).unwrap();
    circuit.rz(qubit, 0.25).unwrap();

    let operations = circuit.operations().to_vec();
    let cache = BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let rules = builtin_rules();
    let commutation_cache = CountingExactCommutationCache::default();
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge])
        .with_max_window_ops(4);

    let patches = select_with_test_cache(&operations, &cache, &rules, &config, &commutation_cache);

    assert_eq!(patches.len(), 1);
    assert_eq!(commutation_cache.computations.load(Ordering::Relaxed), 4);
}

#[test]
fn real_non_commuting_match_caches_failed_proof_across_rules() {
    let qubit = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(qubit, 0.125).unwrap();
    circuit.h(qubit).unwrap();
    circuit.rz(qubit, 0.25).unwrap();

    let operations = circuit.operations().to_vec();
    let cache = BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let rules = builtin_rules();
    let commutation_cache = CountingExactCommutationCache::default();
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge])
        .with_max_window_ops(3);

    let patches = select_with_test_cache(&operations, &cache, &rules, &config, &commutation_cache);

    assert!(patches.is_empty());
    assert_eq!(commutation_cache.computations.load(Ordering::Relaxed), 1);
}

#[test]
fn retained_operation_pair_hits_after_patch_shifts_positions() {
    let qubit = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(qubit).unwrap();
    circuit.x(qubit).unwrap();
    circuit.rz(qubit, 0.125).unwrap();
    circuit.s(qubit).unwrap();

    let operations = circuit.operations().to_vec();
    let cache = BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let retained_ids = [cache.operation_ids[2], cache.operation_ids[3]];
    let rules = builtin_rules();
    let block = BlockContext::new(&operations, &cache).unwrap();
    let selection_cache = SerialExactCommutationCache::new(cache.commutation_memo.as_ref());
    assert!(operations_commute(
        &block,
        2,
        3,
        &rules.commutation,
        &selection_cache,
    ));
    assert!(cache.commutation_memo.exact_or_insert_with(
        retained_ids[0],
        retained_ids[1],
        || panic!("retained pair should already be cached"),
    ));

    let patch = RewritePatch {
        rule_id: 0,
        static_cost_delta: -2,
        first_position: 0,
        last_position: 1,
        matched_positions: vec![0, 1],
        replacements: vec![],
    };
    let next_operations = operations[2..].to_vec();
    let next_cache = cache.into_rewritten(&operations, &[patch]).unwrap();
    assert_eq!(next_cache.operation_ids, retained_ids);

    let next_block = BlockContext::new(&next_operations, &next_cache).unwrap();
    let next_selection_cache =
        SerialExactCommutationCache::new(next_cache.commutation_memo.as_ref());
    assert!(operations_commute(
        &next_block,
        0,
        1,
        &rules.commutation,
        &next_selection_cache,
    ));
    assert!(next_cache.commutation_memo.exact_or_insert_with(
        retained_ids[1],
        retained_ids[0],
        || panic!("shifted retained pair should hit the cross-round memo"),
    ));
}

#[test]
fn replacement_gets_fresh_identity_and_cannot_hit_consumed_pair() {
    let qubit = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(qubit).unwrap();
    circuit.z(qubit).unwrap();

    let operations = circuit.operations().to_vec();
    let cache = BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let consumed_id = cache.operation_ids[0];
    let retained_id = cache.operation_ids[1];
    let rules = builtin_rules();
    let block = BlockContext::new(&operations, &cache).unwrap();
    let selection_cache = SerialExactCommutationCache::new(cache.commutation_memo.as_ref());
    assert!(!operations_commute(
        &block,
        0,
        1,
        &rules.commutation,
        &selection_cache,
    ));
    assert!(
        !cache
            .commutation_memo
            .exact_or_insert_with(consumed_id, retained_id, || panic!(
                "consumed pair should already be cached"
            ),)
    );

    let patch = RewritePatch {
        rule_id: 0,
        static_cost_delta: 0,
        first_position: 0,
        last_position: 0,
        matched_positions: vec![0],
        replacements: vec![ReplacementItem {
            instruction: Instruction::Standard(StandardGate::X),
            qubits: SmallVec::from_slice(&[qubit]),
            params: SmallVec::new(),
            key: cache.instruction_keys[0].clone(),
        }],
    };
    let next_cache = cache.into_rewritten(&operations, &[patch]).unwrap();
    assert_ne!(next_cache.operation_ids[0], consumed_id);
    let recomputations = AtomicUsize::new(0);
    assert!(!next_cache.commutation_memo.exact_or_insert_with(
        next_cache.operation_ids[0],
        retained_id,
        || {
            recomputations.fetch_add(1, Ordering::Relaxed);
            false
        },
    ));
    assert_eq!(recomputations.load(Ordering::Relaxed), 1);

    let next_block = BlockContext::new(&operations, &next_cache).unwrap();
    let next_selection_cache =
        SerialExactCommutationCache::new(next_cache.commutation_memo.as_ref());
    assert!(!operations_commute(
        &next_block,
        0,
        1,
        &rules.commutation,
        &next_selection_cache,
    ));
}

#[test]
fn retained_negative_pair_is_reused_through_multiple_rewrite_rounds() {
    let qubit = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.rz(qubit, 0.125).unwrap();
    circuit.h(qubit).unwrap();
    circuit.rz(qubit, 0.25).unwrap();
    circuit.x(qubit).unwrap();
    circuit.h(qubit).unwrap();
    circuit.h(qubit).unwrap();
    circuit.x(qubit).unwrap();
    circuit.y(qubit).unwrap();

    let rules = builtin_rules();
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge, RuleKind::Cancel])
        .with_max_window_ops(8);
    let mut operations = circuit.operations().to_vec();
    let mut cache = BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let retained_ids = [cache.operation_ids[0], cache.operation_ids[1]];
    let memo = cache.commutation_memo.clone();

    let mut changed_rounds = 0;
    loop {
        let patches =
            select_rewrites_for_anchor_ranges(&operations, &cache, &rules, &config, None, None)
                .unwrap();
        assert!(
            !memo.exact_or_insert_with(retained_ids[0], retained_ids[1], || {
                panic!("the retained negative pair should have been proved during matching")
            })
        );
        if patches.is_empty() {
            break;
        }

        changed_rounds += 1;
        let next_operations = apply_cancellation_patches(&operations, &patches);
        let next_cache = cache.into_rewritten(&operations, &patches).unwrap();
        assert_eq!(next_cache.operation_ids[0..2], retained_ids);
        assert!(
            !memo.exact_or_insert_with(retained_ids[1], retained_ids[0], || {
                panic!("retained IDs should preserve the negative result across rounds")
            })
        );
        operations = next_operations;
        cache = next_cache;
    }

    assert_eq!(changed_rounds, 2);
    let gates = operations
        .iter()
        .map(|operation| match operation.instruction {
            Instruction::Standard(gate) => gate,
            ref other => panic!("unexpected instruction {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        gates,
        vec![
            StandardGate::RZ,
            StandardGate::H,
            StandardGate::RZ,
            StandardGate::Y,
        ]
    );
}

#[test]
fn serial_and_parallel_anchor_scans_select_identical_patches() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.rz(q0, 0.125).unwrap();
    circuit.s(q0).unwrap();
    circuit.t(q0).unwrap();
    circuit.rz(q0, 0.25).unwrap();
    circuit.x(q1).unwrap();
    circuit.x(q1).unwrap();

    let operations = circuit.operations().to_vec();
    let serial_block_cache =
        BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let parallel_block_cache =
        BlockMatchCache::new_with_diagnostics(&circuit, &operations, true).unwrap();
    let serial_block = BlockContext::new(&operations, &serial_block_cache).unwrap();
    let parallel_block = BlockContext::new(&operations, &parallel_block_cache).unwrap();
    let rules = builtin_rules();
    let config = RewriteConfig::production()
        .with_enabled_kinds(vec![RuleKind::Merge, RuleKind::Cancel])
        .with_max_window_ops(6);
    let anchors = (0..operations.len()).collect::<Vec<_>>();

    let serial_cache =
        SerialExactCommutationCache::new(serial_block_cache.commutation_memo.as_ref());
    let mut serial_candidates = Vec::new();
    for &anchor in &anchors {
        serial_candidates.extend(
            scan_anchor(&serial_block, anchor, &rules, &config, None, &serial_cache).unwrap(),
        );
    }
    let serial_patches =
        select_candidate_patches(serial_candidates, operations.len(), None).unwrap();

    let parallel_cache = parallel_block_cache.commutation_memo.as_ref();
    let parallel_scans = anchors
        .par_iter()
        .map(|&anchor| {
            scan_anchor(
                &parallel_block,
                anchor,
                &rules,
                &config,
                None,
                parallel_cache,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let parallel_candidates = parallel_scans.into_iter().flatten().collect();
    let parallel_patches =
        select_candidate_patches(parallel_candidates, operations.len(), None).unwrap();

    assert_eq!(
        patch_signatures(&serial_patches),
        patch_signatures(&parallel_patches)
    );
}
