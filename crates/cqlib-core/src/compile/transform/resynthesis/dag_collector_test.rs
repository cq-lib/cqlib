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
use crate::circuit::{Circuit, CircuitParam, Parameter, ParameterValue, Qubit};
use crate::compile::commutation::CommutationConfig;
use smallvec::SmallVec;

fn config() -> TwoQubitBlockResynthesisConfig {
    TwoQubitBlockResynthesisConfig {
        max_block_ops: 16,
        max_crossed_ops: 4,
        max_scan_span: 16,
        ..TwoQubitBlockResynthesisConfig::default()
    }
}

fn checker() -> CachedCommutation {
    CachedCommutation::new(CommutationConfig {
        enable_rule_oracle: true,
        enable_matrix_fallback: false,
        max_matrix_qubits: 4,
    })
}

fn views(circuit: &Circuit) -> Vec<OperationView<'_>> {
    circuit
        .operations()
        .iter()
        .enumerate()
        .map(|(order, operation)| {
            let params = operation
                .params
                .iter()
                .map(|param| match param {
                    CircuitParam::Fixed(value) => Parameter::from(*value),
                    CircuitParam::Index(index) => circuit
                        .parameters()
                        .get_index(*index as usize)
                        .cloned()
                        .unwrap(),
                })
                .collect::<SmallVec<[_; 3]>>();
            OperationView::new(order, operation, params)
        })
        .collect()
}

#[test]
fn dag_collector_collects_simple_linear_block() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        blocks
            .iter()
            .any(|block| block.matched_orders == vec![0, 1])
    );
}

#[test]
fn dag_collector_ignores_disjoint_source_order_interleaving() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q2).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    let block = blocks
        .iter()
        .find(|block| block.matched_orders == vec![0, 2])
        .unwrap();
    assert!(block.crossed_orders.is_empty());
}

#[test]
fn shared_commuting_operation_can_be_crossed() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cz(q0, q1).unwrap();
    circuit.cz(q0, q2).unwrap();
    circuit.cz(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        blocks
            .iter()
            .any(|block| { block.matched_orders == vec![0, 2] && block.crossed_orders == vec![1] })
    );
}

#[test]
fn shared_symbolic_operation_is_a_hard_boundary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit.rz(q0, ParameterValue::from("theta")).unwrap();
    circuit.cz(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        blocks
            .iter()
            .all(|block| block.matched_orders != vec![0, 2])
    );
}

#[test]
fn shared_non_commuting_operation_blocks_dag_expansion() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        !blocks
            .iter()
            .any(|block| block.matched_orders == vec![0, 2])
    );
}

#[test]
fn left_expansion_stops_at_unaccepted_anchor_dependency() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.rx(q1, 0.17).unwrap();
    circuit.ry(q2, -0.19).unwrap();
    circuit.swap(q0, q2).unwrap();
    circuit.swap(q1, q2).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        !blocks
            .iter()
            .any(|block| block.qubits == [q1, q2] && block.matched_orders.contains(&0)),
        "the anchor SWAP(q1,q2) must not move left across SWAP(q0,q2)"
    );
}

#[test]
fn boundary_stops_dag_expansion() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.barrier(vec![q0, q1]).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config()).unwrap();

    assert!(
        !blocks
            .iter()
            .any(|block| block.matched_orders == vec![0, 2])
    );
}

#[test]
fn dag_scan_span_counts_visited_operation_nodes_per_direction() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.x(q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let mut config = config();
    config.max_scan_span = 1;
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config).unwrap();

    assert!(
        !blocks
            .iter()
            .any(|block| block.matched_orders == vec![0, 1, 2, 3])
    );
}

#[test]
fn crossed_budget_bounds_dag_collection() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cz(q0, q1).unwrap();
    circuit.rz(q0, 0.25).unwrap();
    circuit.cz(q0, q1).unwrap();
    let mut config = config();
    config.max_crossed_ops = 0;
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config).unwrap();

    assert!(
        !blocks
            .iter()
            .any(|block| block.matched_orders == vec![0, 2])
    );
}

#[test]
fn block_size_budget_bounds_dag_collection() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.x(q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let mut config = config();
    config.max_block_ops = 2;
    let views = views(&circuit);
    let mut checker = checker();

    let blocks = collect_two_qubit_blocks_dag(&views, &mut checker, &config).unwrap();

    assert!(blocks.iter().all(|block| block.matched_orders.len() <= 2));
}

#[test]
fn per_anchor_collection_matches_full_scan_order_and_blocks() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let q3 = Qubit::new(3);
    let mut circuit = Circuit::new(4);
    circuit.cz(q0, q1).unwrap();
    circuit.rz(q0, 0.25).unwrap();
    circuit.cz(q0, q1).unwrap();
    circuit.cx(q2, q3).unwrap();
    circuit.h(q3).unwrap();
    circuit.cx(q2, q3).unwrap();
    let views = views(&circuit);
    let config = config();

    let mut full_checker = checker();
    let expected = collect_two_qubit_blocks_dag(&views, &mut full_checker, &config).unwrap();

    let context = DagCollectionContext::build(&views).unwrap();
    let mut anchor_checker = checker();
    let mut actual = Vec::new();
    for anchor in 0..views.len() {
        let (block, _) = context
            .collect_anchor(&views, anchor, &mut anchor_checker, &config)
            .unwrap();
        if let Some(block) = block {
            actual.push(block);
        }
    }

    assert_eq!(actual, expected);
}
