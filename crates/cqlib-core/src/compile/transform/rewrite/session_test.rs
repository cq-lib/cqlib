// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::{
    GateInstructionHistogram, KnowledgeRewriteSession, RewriteExecutionRecord,
    config_is_proven_subset, map_and_invalidate_ranges, valid_clean_gap_bijections,
    valid_exact_clean_gaps, valid_replacements,
};
use crate::circuit::{Circuit, ClassicalExpr, Instruction, Qubit, StandardGate};
use crate::compile::knowledge::library::RuleKind;
use crate::compile::transform::rewrite::RewriteConfig;
use crate::compile::transform::{OperationReplacement, QubitBijection, RewriteEdits};

fn fixed_point_record(
    config: RewriteConfig,
    qubit_bijection_invariant: bool,
) -> RewriteExecutionRecord {
    RewriteExecutionRecord {
        config,
        proof_reach: 1,
        qubit_bijection_invariant,
        reached_fixpoint: true,
        workspace: None,
    }
}

#[test]
fn compatible_configuration_changes_only_shrink_the_proof_policy() {
    let proof = RewriteConfig::production()
        .with_max_window_ops(16)
        .with_max_pattern_len(8)
        .recurse_control_flow(true)
        .skip_labeled_ops(false)
        .with_preserve_two_qubit_connectivity(false);
    let requested = proof
        .clone()
        .with_max_rounds(1)
        .with_max_window_ops(8)
        .with_max_pattern_len(4)
        .recurse_control_flow(false)
        .skip_labeled_ops(true)
        .with_preserve_two_qubit_connectivity(true)
        .with_enabled_kinds(vec![RuleKind::Cancel]);

    let circuit = Circuit::new(1);
    let histogram = GateInstructionHistogram::from_circuit(&circuit);
    assert!(config_is_proven_subset(
        &requested,
        &proof,
        Some(&histogram),
    ));
    assert!(!config_is_proven_subset(
        &proof,
        &requested,
        Some(&histogram),
    ));
}

#[test]
fn exact_edits_shift_existing_dirty_ranges_and_invalidate_with_old_reach() {
    let replacements = vec![OperationReplacement {
        old: 2..4,
        new: 2..5,
    }];
    assert!(valid_replacements(20, 21, &replacements));
    let dirty = vec![10..12, 18..19];
    assert_eq!(
        map_and_invalidate_ranges(&dirty, &replacements, 20, 21, 3),
        vec![0..5, 11..13, 19..20]
    );
}

#[test]
fn malformed_or_overlapping_edit_scripts_are_rejected() {
    assert!(!valid_replacements(
        10,
        10,
        &[
            OperationReplacement {
                old: 4..6,
                new: 4..6,
            },
            OperationReplacement {
                old: 5..7,
                new: 5..7,
            },
        ]
    ));
}

#[test]
fn clean_gap_qubit_bijection_validates_resolved_operations() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut before = Circuit::new(2);
    before.rx(q0, 0.25).unwrap();
    before.cx(q0, q1).unwrap();
    let mut after = Circuit::new(3);
    after.rx(q2, 0.25).unwrap();
    after.cx(q2, q1).unwrap();
    let bijections = vec![QubitBijection {
        pairs: vec![(q0, q2), (q1, q1)],
    }];

    assert!(valid_clean_gap_bijections(
        &before,
        &after,
        &[],
        &bijections,
    ));

    let incomplete = vec![QubitBijection {
        pairs: vec![(q0, q2)],
    }];
    assert!(!valid_clean_gap_bijections(
        &before,
        &after,
        &[],
        &incomplete,
    ));

    let non_bijective = vec![QubitBijection {
        pairs: vec![(q0, q2), (q1, q2)],
    }];
    assert!(!valid_clean_gap_bijections(
        &before,
        &after,
        &[],
        &non_bijective,
    ));
}

#[test]
fn clean_gap_qubit_bijection_rejects_operation_or_parameter_changes() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut before = Circuit::new(2);
    before.rx(q0, 0.25).unwrap();
    let bijections = vec![QubitBijection {
        pairs: vec![(q0, q1)],
    }];

    let mut changed_instruction = Circuit::new(2);
    changed_instruction.ry(q1, 0.25).unwrap();
    assert!(!valid_clean_gap_bijections(
        &before,
        &changed_instruction,
        &[],
        &bijections,
    ));

    let mut changed_parameter = Circuit::new(2);
    changed_parameter.rx(q1, 0.5).unwrap();
    assert!(!valid_clean_gap_bijections(
        &before,
        &changed_parameter,
        &[],
        &bijections,
    ));
}

#[test]
fn strict_linear_edits_reject_a_qubit_domain_change() {
    let q0 = Qubit::new(0);
    let mut before = Circuit::new(1);
    before.x(q0).unwrap();
    let mut after = Circuit::new(2);
    after.x(q0).unwrap();
    let config = RewriteConfig::production();
    let mut session = KnowledgeRewriteSession::default();
    session.record_execution(&before, 0, fixed_point_record(config.clone(), true));
    session.apply_rewrite_edits(
        0,
        1,
        &before,
        &after,
        &RewriteEdits::linear(1, 1, Vec::new()),
    );

    assert!(session.reusable_proof(1, &config).is_none());
}

#[test]
fn strict_linear_edits_verify_every_declared_clean_gap() {
    let q0 = Qubit::new(0);
    let mut before = Circuit::new(1);
    before.x(q0).unwrap();
    before.h(q0).unwrap();
    let mut after = Circuit::new(1);
    after.y(q0).unwrap();
    after.h(q0).unwrap();

    assert!(valid_exact_clean_gaps(
        &before,
        &after,
        &[OperationReplacement {
            old: 0..1,
            new: 0..1,
        }],
    ));
    assert!(!valid_exact_clean_gaps(&before, &after, &[]));
}

#[test]
fn qubit_bijection_edits_require_a_compatible_rewrite_proof() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut before = Circuit::new(1);
    before.x(q0).unwrap();
    let mut after = Circuit::new(2);
    after.x(q1).unwrap();
    let edits = RewriteEdits::linear_modulo_qubit_bijection(
        1,
        1,
        Vec::new(),
        vec![QubitBijection {
            pairs: vec![(q0, q1)],
        }],
    );
    let mut compatible = KnowledgeRewriteSession::default();
    compatible.record_execution(
        &before,
        0,
        fixed_point_record(RewriteConfig::production(), true),
    );
    compatible.apply_rewrite_edits(0, 1, &before, &after, &edits);
    assert!(
        compatible
            .reusable_proof(1, &RewriteConfig::production())
            .is_some()
    );

    let mut incompatible = KnowledgeRewriteSession::default();
    incompatible.record_execution(
        &before,
        0,
        fixed_point_record(RewriteConfig::production(), false),
    );
    incompatible.apply_rewrite_edits(0, 1, &before, &after, &edits);
    assert!(
        incompatible
            .reusable_proof(1, &RewriteConfig::production())
            .is_none()
    );
}

#[test]
fn target_aware_policy_reuses_only_when_every_observed_gate_is_physical() {
    let q0 = Qubit::new(0);
    let proof = RewriteConfig::production();
    let target_x = proof
        .clone()
        .with_target_instructions(vec![Instruction::Standard(StandardGate::X)])
        .unwrap();
    let mut physical = Circuit::new(1);
    physical.x(q0).unwrap();
    let physical_histogram = GateInstructionHistogram::from_circuit(&physical);
    assert!(config_is_proven_subset(
        &target_x,
        &proof,
        Some(&physical_histogram),
    ));

    let mut unsupported = Circuit::new(1);
    unsupported.h(q0).unwrap();
    let unsupported_histogram = GateInstructionHistogram::from_circuit(&unsupported);
    assert!(!config_is_proven_subset(
        &target_x,
        &proof,
        Some(&unsupported_histogram),
    ));

    let mut nested = Circuit::new(1);
    nested
        .if_(ClassicalExpr::bool_literal(true), |body| body.h(q0))
        .unwrap();
    let nested_histogram = GateInstructionHistogram::from_circuit(&nested);
    assert!(!config_is_proven_subset(
        &target_x,
        &proof,
        Some(&nested_histogram),
    ));
    assert!(config_is_proven_subset(
        &target_x.clone().recurse_control_flow(false),
        &proof,
        Some(&nested_histogram),
    ));
}

#[test]
fn changing_or_removing_an_existing_target_never_reuses_its_proof() {
    let circuit = Circuit::new(1);
    let base = RewriteConfig::production();
    let target_x = base
        .clone()
        .with_target_instructions(vec![Instruction::Standard(StandardGate::X)])
        .unwrap();
    let target_h = base
        .clone()
        .with_target_instructions(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let histogram = GateInstructionHistogram::from_circuit(&circuit);

    assert!(!config_is_proven_subset(
        &target_h,
        &target_x,
        Some(&histogram),
    ));
    assert!(!config_is_proven_subset(&base, &target_x, Some(&histogram),));
}
