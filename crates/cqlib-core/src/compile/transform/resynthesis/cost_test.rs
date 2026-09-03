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
use crate::circuit::{Instruction, Qubit, StandardGate, ValueOperation};
use crate::compile::transform::decompose::unitary::{
    TwoQubitSynthesisTarget, TwoQubitUnitaryDecomposeBasis, target_aware_cost_of_value_operations,
};

fn target() -> TwoQubitSynthesisTarget {
    TwoQubitSynthesisTarget::from_standard_gates(
        vec![StandardGate::H, StandardGate::X, StandardGate::U],
        vec![StandardGate::CX],
        true,
    )
    .unwrap()
}

fn replacement_cost(ops: &[ValueOperation]) -> ResynthesisCost {
    target_aware_cost_of_value_operations(ops, &target(), TwoQubitUnitaryDecomposeBasis::Cx)
        .unwrap()
}

#[test]
fn cost_order_prefers_fewer_two_qubit_ops_before_total_gate_count() {
    let lower_two_qubit = ResynthesisCost {
        lowered_two_qubit_ops: 1,
        lowered_total_ops: 8,
        ..ResynthesisCost::default()
    };
    let higher_two_qubit = ResynthesisCost {
        lowered_two_qubit_ops: 2,
        lowered_total_ops: 2,
        ..ResynthesisCost::default()
    };

    assert!(lower_two_qubit < higher_two_qubit);
}

#[test]
fn gphase_replacements_are_cost_free() {
    let op = ValueOperation::from_standard(StandardGate::GPhase, [], [0.3.into()]);

    assert_eq!(replacement_cost(&[op]), ResynthesisCost::default());
}

#[test]
fn replacement_cost_counts_two_qubit_gate_and_depth() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let op = ValueOperation::from_standard(StandardGate::CX, [q0, q1], []);

    let cost = replacement_cost(&[op]);

    assert_eq!(cost.lowered_two_qubit_ops, 1);
    assert_eq!(cost.lowered_total_ops, 1);
    assert_eq!(cost.lowered_depth, 1);
}

#[test]
fn non_standard_replacement_is_rejected_by_exact_lowering_cost() {
    let q0 = Qubit::new(0);
    let unsupported = ValueOperation {
        instruction: crate::circuit::ValueInstruction::from_instruction(Instruction::Delay),
        qubits: smallvec::smallvec![q0],
        params: smallvec::smallvec![],
        label: None,
    };

    assert!(
        target_aware_cost_of_value_operations(
            &[unsupported],
            &target(),
            TwoQubitUnitaryDecomposeBasis::Cx,
        )
        .is_err()
    );
}

#[test]
fn depth_estimate_keeps_disjoint_single_qubit_ops_parallel() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let ops = [
        ValueOperation::from_standard(StandardGate::H, [q0], []),
        ValueOperation::from_standard(StandardGate::X, [q1], []),
    ];

    let cost = replacement_cost(&ops);

    assert_eq!(cost.lowered_total_ops, 2);
    assert_eq!(cost.lowered_depth, 1);
}

#[test]
fn depth_estimate_serializes_ops_on_shared_qubit() {
    let q0 = Qubit::new(0);
    let ops = [
        ValueOperation::from_standard(StandardGate::H, [q0], []),
        ValueOperation::from_standard(StandardGate::X, [q0], []),
    ];

    let cost = replacement_cost(&ops);

    assert_eq!(cost.lowered_total_ops, 2);
    assert_eq!(cost.lowered_depth, 2);
}
