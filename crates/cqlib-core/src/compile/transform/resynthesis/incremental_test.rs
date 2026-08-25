// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ClassicalExpr, Instruction, Parameter, Qubit,
};
use crate::compile::transform::resynthesis::commutation::{CachedCommutation, OperationView};
use smallvec::SmallVec;

fn views(circuit: &Circuit) -> Vec<OperationView<'_>> {
    circuit
        .operations()
        .iter()
        .enumerate()
        .map(|(order, operation)| {
            let params = operation
                .params
                .iter()
                .map(|parameter| match parameter {
                    CircuitParam::Fixed(value) => Parameter::from(*value),
                    CircuitParam::Index(index) => circuit.parameters()[*index as usize].clone(),
                })
                .collect::<SmallVec<[_; 3]>>();
            OperationView::new(order, operation, params)
        })
        .collect()
}

fn collect(
    session: &mut NativeResynthesisSession,
    circuit: &Circuit,
    config: &TwoQubitBlockResynthesisConfig,
) -> Vec<TwoQubitNumericBlock> {
    session.begin_round(config);
    let views = views(circuit);
    let mut commutation = CachedCommutation::new(config.commutation.clone());
    let blocks = session
        .collect_blocks(
            &NativeScopeId::default(),
            circuit,
            circuit.operations(),
            &views,
            &mut commutation,
            config,
        )
        .unwrap();
    session.finish_round();
    blocks
}

fn three_pair_circuit(insert_local_change: bool) -> Circuit {
    let mut circuit = Circuit::new(6);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    if insert_local_change {
        circuit.rz(Qubit::new(0), 0.125).unwrap();
    }
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(3)).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(3)).unwrap();
    circuit.cx(Qubit::new(4), Qubit::new(5)).unwrap();
    circuit.cx(Qubit::new(4), Qubit::new(5)).unwrap();
    circuit
}

fn branch_circuit(angle: f64) -> Circuit {
    let mut circuit = Circuit::new(4);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.rz(Qubit::new(0), angle)?;
                body.cx(Qubit::new(0), Qubit::new(1))?;
                body.cx(Qubit::new(0), Qubit::new(1))
            },
            |body| {
                body.cx(Qubit::new(2), Qubit::new(3))?;
                body.cx(Qubit::new(2), Qubit::new(3))
            },
        )
        .unwrap();
    circuit
}

fn collect_if_scopes(
    session: &mut NativeResynthesisSession,
    circuit: &Circuit,
    config: &TwoQubitBlockResynthesisConfig,
) {
    session.begin_round(config);
    let root = NativeScopeId::default();
    let root_views = views(circuit);
    let mut root_commutation = CachedCommutation::new(config.commutation.clone());
    session
        .collect_blocks(
            &root,
            circuit,
            circuit.operations(),
            &root_views,
            &mut root_commutation,
            config,
        )
        .unwrap();
    let parent = session.current_operation_key(&root, 0).unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(operation)) =
        &circuit.operations()[0].instruction
    else {
        panic!("expected if operation");
    };
    for (scope, body) in [
        (
            root.child(NativeScopeSegment::IfThen(parent)),
            operation.then_body(),
        ),
        (
            root.child(NativeScopeSegment::IfElse(parent)),
            operation.else_body().unwrap(),
        ),
    ] {
        let body_views = body
            .operations()
            .iter()
            .enumerate()
            .map(|(order, operation)| {
                let params = operation
                    .params
                    .iter()
                    .map(|parameter| match parameter {
                        CircuitParam::Fixed(value) => Parameter::from(*value),
                        CircuitParam::Index(index) => circuit.parameters()[*index as usize].clone(),
                    })
                    .collect::<SmallVec<[_; 3]>>();
                OperationView::new(order, operation, params)
            })
            .collect::<Vec<_>>();
        let mut commutation = CachedCommutation::new(config.commutation.clone());
        session
            .collect_blocks(
                &scope,
                circuit,
                body.operations(),
                &body_views,
                &mut commutation,
                config,
            )
            .unwrap();
    }
    session.finish_round();
}

#[test]
fn unchanged_scope_reuses_every_anchor_without_recollection() {
    let circuit = three_pair_circuit(false);
    let config = TwoQubitBlockResynthesisConfig::default();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    let first = collect(&mut session, &circuit, &config);
    let before = session.stats();
    let second = collect(&mut session, &circuit, &config);
    let after = session.stats();

    assert_eq!(first, second);
    assert_eq!(after.anchors_recomputed, before.anchors_recomputed);
    assert_eq!(after.anchors_reused - before.anchors_reused, 6);
    assert_eq!(after.scopes_unchanged - before.scopes_unchanged, 1);
}

#[test]
fn maximal_preparation_does_not_build_bounded_dag_eagerly() {
    let circuit = three_pair_circuit(false);
    let config = TwoQubitBlockResynthesisConfig::default();
    let scope = NativeScopeId::default();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    session.begin_round(&config);
    session
        .prepare_scope_for_maximal(&scope, &circuit, circuit.operations(), &config)
        .unwrap();
    let operation_views = views(&circuit);
    let first = session
        .collect_maximal_blocks(&scope, &operation_views)
        .unwrap();
    assert!(!first.is_empty());
    assert_eq!(session.stats().anchors_recomputed, 0);
    assert_eq!(session.stats().scopes_full_scan, 0);
    session.finish_round();

    session.begin_round(&config);
    let reused = session
        .reuse_unchanged_maximal_blocks(&scope, &circuit, circuit.operations())
        .unwrap()
        .expect("unchanged maximal workset should be reusable");
    assert_eq!(first, reused);
    assert!(session.stats().maximal_blocks_reused >= reused.len());
}

#[test]
fn local_insertion_recomputes_touched_pair_and_reuses_disjoint_anchors() {
    let before_circuit = three_pair_circuit(false);
    let after_circuit = three_pair_circuit(true);
    let config = TwoQubitBlockResynthesisConfig::default();
    let mut incremental = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);
    let mut full = NativeResynthesisSession::new(NativeResynthesisPolicy::FullScan);

    collect(&mut incremental, &before_circuit, &config);
    let before_stats = incremental.stats();
    let incremental_blocks = collect(&mut incremental, &after_circuit, &config);
    let full_blocks = collect(&mut full, &after_circuit, &config);
    let delta_reused = incremental.stats().anchors_reused - before_stats.anchors_reused;
    let delta_recomputed = incremental.stats().anchors_recomputed - before_stats.anchors_recomputed;

    assert_eq!(incremental_blocks, full_blocks);
    assert!(delta_reused > 0);
    assert!(delta_recomputed < 6);
}

#[test]
fn changed_if_branch_does_not_invalidate_unchanged_sibling_scope() {
    let before = branch_circuit(0.125);
    let after = branch_circuit(0.25);
    let config = TwoQubitBlockResynthesisConfig::default();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    collect_if_scopes(&mut session, &before, &config);
    let first = session.stats();
    collect_if_scopes(&mut session, &after, &config);
    let second = session.stats();

    assert!(second.scopes_unchanged > first.scopes_unchanged);
    assert!(second.anchors_reused > first.anchors_reused);
}

#[test]
fn inserting_hard_boundary_forces_scope_full_scan() {
    let before = three_pair_circuit(false);
    let mut after = Circuit::new(6);
    after.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    after.barrier(vec![Qubit::new(0), Qubit::new(1)]).unwrap();
    after.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    after.cx(Qubit::new(2), Qubit::new(3)).unwrap();
    after.cx(Qubit::new(2), Qubit::new(3)).unwrap();
    after.cx(Qubit::new(4), Qubit::new(5)).unwrap();
    after.cx(Qubit::new(4), Qubit::new(5)).unwrap();
    let config = TwoQubitBlockResynthesisConfig::default();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    collect(&mut session, &before, &config);
    let first = session.stats();
    collect(&mut session, &after, &config);
    let second = session.stats();

    assert_eq!(second.scopes_full_scan - first.scopes_full_scan, 1);
    assert_eq!(second.anchors_reused - first.anchors_reused, 0);
}

#[test]
fn fixed_parameter_signed_zero_is_treated_as_an_exact_change() {
    let build = |angle| {
        let mut circuit = three_pair_circuit(false);
        circuit.rz(Qubit::new(0), angle).unwrap();
        circuit
    };
    let before = build(-0.0);
    let after = build(0.0);
    let config = TwoQubitBlockResynthesisConfig::default();
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    collect(&mut session, &before, &config);
    let first = session.stats();
    collect(&mut session, &after, &config);
    let second = session.stats();

    assert!(second.anchors_recomputed > first.anchors_recomputed);
    assert!(second.anchors_reused > first.anchors_reused);
}

#[test]
fn config_change_invalidates_all_cached_anchor_collections() {
    let circuit = three_pair_circuit(false);
    let first_config = TwoQubitBlockResynthesisConfig::default();
    let mut second_config = first_config.clone();
    second_config.max_scan_span = second_config.max_scan_span.saturating_sub(1);
    let mut session = NativeResynthesisSession::new(NativeResynthesisPolicy::Incremental);

    collect(&mut session, &circuit, &first_config);
    let first = session.stats();
    collect(&mut session, &circuit, &second_config);
    let second = session.stats();

    assert_eq!(second.anchors_reused, first.anchors_reused);
    assert_eq!(second.anchors_recomputed - first.anchors_recomputed, 6);
    assert_eq!(second.scopes_full_scan - first.scopes_full_scan, 1);
}

#[test]
fn equal_operation_snapshots_have_equal_fast_hashes() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    let operation = &circuit.operations()[0];

    let left = OperationSnapshot::new(&circuit, operation).unwrap();
    let right = OperationSnapshot::new(&circuit, &operation.clone()).unwrap();

    assert_eq!(left.fast_hash, right.fast_hash);
    assert!(left.exact_eq(&right));
}
