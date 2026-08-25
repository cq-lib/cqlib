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
use super::*;
use crate::circuit::{CircuitParam, Operation, Parameter, Qubit, StandardGate};
use crate::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
use ndarray::Array2;
use ndarray::linalg::kron;

fn view(order: usize, operation: &Operation) -> OperationView<'_> {
    OperationView::new(
        order,
        operation,
        operation
            .params
            .iter()
            .map(|param| match param {
                CircuitParam::Fixed(value) => Parameter::from(*value),
                CircuitParam::Index(_) => unreachable!(),
            })
            .collect(),
    )
}

fn standard_op(gate: StandardGate, qubits: &[Qubit]) -> Operation {
    Operation {
        instruction: Instruction::Standard(gate),
        qubits: qubits.iter().copied().collect(),
        params: Default::default(),
        label: None,
    }
}

fn cx_config() -> TwoQubitBlockResynthesisConfig {
    TwoQubitBlockResynthesisConfig::normal(
        TwoQubitSynthesisTarget::from_standard_gates(
            vec![StandardGate::U],
            vec![StandardGate::CX],
            true,
        )
        .unwrap(),
    )
}

fn block(
    qubits: [Qubit; 2],
    matched_orders: Vec<usize>,
    matched_1q_count: usize,
    matched_2q_count: usize,
) -> TwoQubitNumericBlock {
    TwoQubitNumericBlock::maximal_closed_run(
        qubits,
        matched_orders,
        matched_1q_count,
        matched_2q_count,
        false,
    )
}

fn block_with_crossed(
    qubits: [Qubit; 2],
    matched_orders: Vec<usize>,
    crossed_orders: Vec<usize>,
) -> TwoQubitNumericBlock {
    let matched_2q_count = matched_orders.len();
    TwoQubitNumericBlock::dag_dependency_closed(
        qubits,
        matched_orders,
        crossed_orders,
        0,
        matched_2q_count,
        false,
    )
}

fn dag_block(qubits: [Qubit; 2], matched_orders: Vec<usize>) -> TwoQubitNumericBlock {
    let matched_2q_count = matched_orders.len();
    TwoQubitNumericBlock::dag_dependency_closed(
        qubits,
        matched_orders,
        vec![],
        0,
        matched_2q_count,
        false,
    )
}

#[test]
fn reversed_two_qubit_gate_uses_swap_conjugation() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let cx = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    let swap = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
    let expected = swap.dot(&cx).dot(&swap);
    let op = standard_op(StandardGate::CX, &[q1, q0]);
    let ops = vec![view(0, &op)];

    assert_eq!(
        block_matrix(&block([q0, q1], vec![0], 0, 1), &ops).unwrap(),
        expected
    );
}

#[test]
fn single_qubit_expansion_respects_canonical_order() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let h = standard_op(StandardGate::H, &[q0]);
    let x = standard_op(StandardGate::X, &[q1]);
    let ops = vec![view(0, &h), view(1, &x)];
    let h_matrix = StandardGate::H.matrix(&[]).unwrap().into_owned();
    let x_matrix = StandardGate::X.matrix(&[]).unwrap().into_owned();
    let expected = kron(&h_matrix.view(), &x_matrix.view());

    assert_eq!(
        block_matrix(&block([q0, q1], vec![0, 1], 2, 0), &ops).unwrap(),
        expected
    );
}

#[test]
fn block_matrix_multiplies_operations_in_source_order() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let h = standard_op(StandardGate::H, &[q0]);
    let cx = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &h), view(1, &cx)];
    let h_matrix = StandardGate::H.matrix(&[]).unwrap().into_owned();
    let identity = Array2::eye(2);
    let expanded_h = kron(&h_matrix.view(), &identity.view());
    let expected = StandardGate::CX.matrix(&[]).unwrap().dot(&expanded_h);

    assert_eq!(
        block_matrix(&block([q0, q1], vec![0, 1], 1, 1), &ops).unwrap(),
        expected
    );
}

#[test]
fn select_patches_keeps_strictly_improving_non_overlapping_candidate() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &first), view(1, &second)];
    let blocks = vec![block([q0, q1], vec![0, 1], 0, 2)];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].matched_orders, vec![0, 1]);
    assert!(patches[0].replacement.is_empty());
}

#[test]
fn select_patches_deduplicates_identical_blocks_before_selection() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &first), view(1, &second)];
    let blocks = vec![
        block([q0, q1], vec![0, 1], 0, 2),
        block([q0, q1], vec![0, 1], 0, 2),
    ];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert_eq!(patches.len(), 1);
}

#[test]
fn select_patches_uses_non_overlapping_greedy_selection() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let third = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &first), view(1, &second), view(2, &third)];
    let blocks = vec![
        block([q0, q1], vec![0, 1], 0, 2),
        block([q0, q1], vec![1, 2], 0, 2),
    ];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].matched_orders, vec![0, 1]);
}

#[test]
fn select_patches_rejects_forged_noncommuting_source_crossing() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let crossed = standard_op(StandardGate::H, &[q0]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let third = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![
        view(0, &first),
        view(1, &crossed),
        view(2, &second),
        view(3, &third),
    ];
    let blocks = vec![block_with_crossed([q0, q1], vec![0, 2, 3], vec![1])];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert!(patches.is_empty());
}

#[test]
fn select_patches_accepts_when_replacement_commutes_with_disjoint_crossed_op() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let crossed = standard_op(StandardGate::H, &[q2]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let third = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![
        view(0, &first),
        view(1, &crossed),
        view(2, &second),
        view(3, &third),
    ];
    let blocks = vec![block_with_crossed([q0, q1], vec![0, 2, 3], vec![1])];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].crossed_orders, vec![1]);
}

#[test]
fn select_patches_rejects_forged_crossing_over_hard_boundary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let mut boundary = standard_op(StandardGate::H, &[q2]);
    boundary.label = Some("boundary".into());
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &first), view(1, &boundary), view(2, &second)];
    let forged = block_with_crossed([q0, q1], vec![0, 2], vec![1]);
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    assert!(
        select_patches(vec![forged], &ops, &commutation, &config)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn certified_block_with_unmarked_pair_operation_is_structurally_rejected() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let operations = [
        standard_op(StandardGate::CX, &[q0, q1]),
        standard_op(StandardGate::H, &[q0]),
        standard_op(StandardGate::CX, &[q0, q1]),
    ];
    let ops = operations
        .iter()
        .enumerate()
        .map(|(order, operation)| view(order, operation))
        .collect::<Vec<_>>();
    let forged =
        TwoQubitNumericBlock::dag_dependency_closed([q0, q1], vec![0, 2], vec![], 0, 2, false);
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    assert!(
        select_patches(vec![forged], &ops, &commutation, &config)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn patch_span_validation_accounts_for_synthesis_phase() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let first = standard_op(StandardGate::CX, &[q0, q1]);
    let second = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &first), view(1, &second)];
    let block = block([q0, q1], vec![0, 1], 0, 2);

    assert!(
        patch_preserves_relevant_span(&block, &ops, &[], 0.0).unwrap(),
        "identity replacement should preserve a CX inverse pair"
    );
    assert!(
        !patch_preserves_relevant_span(&block, &ops, &[], 0.25).unwrap(),
        "non-zero synthesis phase must be part of span validation"
    );
}

#[test]
fn patch_span_validation_rejects_too_many_relevant_qubits() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut operations = vec![
        standard_op(StandardGate::CX, &[q0, q1]),
        standard_op(StandardGate::CX, &[q0, q1]),
    ];
    let mut crossed = Vec::new();
    for index in 2..=6 {
        let order = operations.len();
        operations.push(standard_op(StandardGate::H, &[Qubit::new(index)]));
        crossed.push(order);
    }
    let ops = operations
        .iter()
        .enumerate()
        .map(|(order, operation)| view(order, operation))
        .collect::<Vec<_>>();
    let block = block_with_crossed([q0, q1], vec![0, 1], crossed);

    assert!(!patch_preserves_relevant_span(&block, &ops, &[], 0.0).unwrap());
}

#[test]
fn certified_block_is_not_limited_by_test_oracle_qubit_cap() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut operations = vec![
        standard_op(StandardGate::CX, &[q0, q1]),
        standard_op(StandardGate::CX, &[q0, q1]),
    ];
    let mut crossed = Vec::new();
    for index in 2..=6 {
        let order = operations.len();
        operations.push(standard_op(StandardGate::H, &[Qubit::new(index)]));
        crossed.push(order);
    }
    let ops = operations
        .iter()
        .enumerate()
        .map(|(order, operation)| view(order, operation))
        .collect::<Vec<_>>();
    let certified = block_with_crossed([q0, q1], vec![0, 1], crossed);
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    assert!(!patch_preserves_relevant_span(&certified, &ops, &[], 0.0).unwrap());
    let patches = select_patches(vec![certified], &ops, &commutation, &config).unwrap();
    assert_eq!(patches.len(), 1);
    assert!(patches[0].replacement.is_empty());
}

#[test]
fn generic_best_first_matches_eager_reference_and_prunes_matrix_synthesis() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let operations = [
        standard_op(StandardGate::CX, &[q0, q1]),
        standard_op(StandardGate::CX, &[q0, q1]),
        standard_op(StandardGate::CX, &[q0, q1]),
    ];
    let ops = operations
        .iter()
        .enumerate()
        .map(|(order, operation)| view(order, operation))
        .collect::<Vec<_>>();
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());
    let eager = select_patches(
        vec![
            block([q0, q1], vec![0, 1], 0, 2),
            block([q0, q1], vec![1, 2], 0, 2),
        ],
        &ops,
        &commutation,
        &config,
    )
    .unwrap();
    let mut cache = TwoQubitSynthesisCache::default();
    let patches = select_patches_with_device(
        vec![
            dag_block([q0, q1], vec![0, 1]),
            dag_block([q0, q1], vec![1, 2]),
        ],
        &ops,
        &commutation,
        &config,
        None,
        &mut cache,
    )
    .unwrap();

    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].matched_orders, vec![0, 1]);
    assert_eq!(patches[0].matched_orders, eager[0].matched_orders);
    assert_eq!(patches[0].replacement, eager[0].replacement);
    assert_eq!(patches[0].after_cost, eager[0].after_cost);
    assert_eq!(cache.stats().selection.matrix_attempts, 1);
    assert_eq!(cache.stats().selection.lower_bound_pruned, 1);
}

#[test]
fn replacement_validation_rejects_out_of_pair_qubits_and_non_finite_phase() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let block = dag_block([q0, q1], vec![0, 1]);
    let replacement = vec![ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::H)),
        qubits: smallvec::smallvec![q2],
        params: Default::default(),
        label: None,
    }];

    assert!(!replacement_respects_block(&block, &replacement, 0.0));
    assert!(!replacement_respects_block(&block, &[], f64::NAN));
}

#[test]
fn select_patches_rejects_single_cx_without_cost_improvement() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let cx = standard_op(StandardGate::CX, &[q0, q1]);
    let ops = vec![view(0, &cx)];
    let blocks = vec![block([q0, q1], vec![0], 0, 1)];
    let config = cx_config();
    let commutation = CachedCommutation::new(config.commutation.clone());

    let patches = select_patches(blocks, &ops, &commutation, &config).unwrap();

    assert!(patches.is_empty());
}
