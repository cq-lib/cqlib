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
use crate::circuit::{
    Circuit, CircuitParam, Instruction, Parameter, ParameterValue, Qubit, StandardGate,
};
use smallvec::SmallVec;
use std::collections::HashSet;

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
fn collects_one_unbounded_run_and_ignores_disjoint_interleaving() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    for index in 0..80 {
        circuit.rx(q0, 0.01 * (index as f64 + 1.0)).unwrap();
        circuit.h(q2).unwrap();
        if index % 2 == 0 {
            circuit.cx(q0, q1).unwrap();
        } else {
            circuit.cx(q1, q0).unwrap();
        }
        circuit.rz(q1, -0.02 * (index as f64 + 1.0)).unwrap();
    }
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].matched_2q_count, 80);
    assert_eq!(blocks[0].matched_1q_count, 160);
    assert_eq!(blocks[0].matched_orders.len(), 240);
    assert!(
        blocks[0]
            .matched_orders
            .iter()
            .all(|order| !views[*order].operation.qubits.contains(&q2))
    );
}

#[test]
fn changing_partner_partitions_runs_without_shared_operations() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.h(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.x(q0).unwrap();
    circuit.cx(q0, q2).unwrap();
    circuit.z(q2).unwrap();
    circuit.cx(q2, q0).unwrap();
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].matched_orders, vec![0, 1, 2]);
    assert_eq!(blocks[1].matched_orders, vec![3, 4, 5]);
    let unique = blocks
        .iter()
        .flat_map(|block| block.matched_orders.iter().copied())
        .collect::<HashSet<_>>();
    assert_eq!(unique.len(), 6);
}

#[test]
fn boundary_on_other_pair_wire_rejects_older_pending_prefix() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit.measure(q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].matched_orders, vec![2, 3]);
}

#[test]
fn labeled_and_symbolic_operations_are_unconditional_boundaries() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.h(q0).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::H),
            [q1],
            std::iter::empty::<ParameterValue>(),
            Some("preserve"),
        )
        .unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.rz(q1, Parameter::symbol("theta")).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].matched_orders, vec![2, 3]);
    assert_eq!(blocks[1].matched_orders, vec![5, 6]);
}

#[test]
fn zero_qubit_phase_is_a_scope_wide_boundary() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit
        .append(
            Instruction::Standard(StandardGate::GPhase),
            std::iter::empty::<Qubit>(),
            [ParameterValue::Fixed(0.37)],
            None,
        )
        .unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].matched_orders, vec![0, 1]);
    assert_eq!(blocks[1].matched_orders, vec![3, 4]);
}

#[test]
fn reset_delay_and_control_flow_close_runs_on_touched_qubits() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    circuit.reset(q0).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.delay(q1, ParameterValue::Fixed(4.0)).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit
        .if_(crate::circuit::ClassicalExpr::bool_literal(true), |body| {
            body.h(q0)
        })
        .unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.cx(q0, q1).unwrap();
    let views = views(&circuit);

    let blocks = collect_maximal_two_qubit_runs(&views);

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].matched_orders, vec![6, 7]);
}
