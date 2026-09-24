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
fn local_cleanup_fuses_ops_on_shared_qubit() {
    let q0 = Qubit::new(0);
    let ops = [
        ValueOperation::from_standard(StandardGate::H, [q0], []),
        ValueOperation::from_standard(StandardGate::X, [q0], []),
    ];

    let cost = replacement_cost(&ops);

    assert_eq!(cost.lowered_total_ops, 1);
    assert_eq!(cost.lowered_depth, 1);
}

#[test]
fn explicit_basis_scoring_reuses_xyx_and_removes_zero_rotation() {
    let target = TwoQubitSynthesisTarget::from_standard_gates(
        vec![StandardGate::RX, StandardGate::RY],
        vec![StandardGate::CZ],
        true,
    )
    .unwrap();
    let q = [Qubit::new(0), Qubit::new(1)];
    let operations = [
        ValueOperation::from_standard(
            StandardGate::U,
            [q[0]],
            [0.37.into(), 0.23.into(), (-0.41).into()],
        ),
        ValueOperation::from_standard(StandardGate::CZ, q, []),
        ValueOperation::from_standard(StandardGate::RX, [q[1]], [0.0.into()]),
    ];
    let candidate = target_aware_cost_of_value_operations(
        &operations,
        &target,
        TwoQubitUnitaryDecomposeBasis::Cz,
    )
    .unwrap();
    // Generic U uses the completed three-rotation XYX candidate. The one
    // cleanup round removes the trailing native zero rotation.
    assert_eq!(candidate.lowered_two_qubit_ops, 1);
    assert_eq!(candidate.lowered_depth, 4);
    assert_eq!(candidate.lowered_total_ops, 4);
    assert_eq!(candidate.parameterized_ops, 3);
    let source = cost_of_source_value_operations(&operations, &target).unwrap();
    assert_eq!(
        source,
        ResynthesisCost {
            backend_order: 0,
            ..candidate
        }
    );
}

#[test]
fn local_cleanup_corrects_original_cost_reversal() {
    use crate::circuit::{Circuit, circuit_to_matrix};
    use crate::compile::transform::decompose::unitary::{
        TwoQubitSynthesisRequest, plan_numeric_2q_unitary,
    };

    let q = [Qubit::new(0), Qubit::new(1)];
    let operations = vec![
        ValueOperation::from_standard(StandardGate::CX, q, []),
        ValueOperation::from_standard(StandardGate::H, [q[1]], []),
        ValueOperation::from_standard(StandardGate::H, [q[0]], []),
    ];
    let target = TwoQubitSynthesisTarget::from_standard_gates(
        vec![StandardGate::RZ, StandardGate::X2P],
        vec![StandardGate::CZ],
        true,
    )
    .unwrap();
    let model = target.lowering_cost_model().unwrap();
    let raw_source = model
        .cost_of_fixed_operations(q.to_vec(), operations.clone())
        .unwrap();
    let source_cost = cost_of_source_value_operations(&operations, &target).unwrap();
    assert_eq!(
        (
            raw_source.two_qubit_ops,
            raw_source.depth,
            raw_source.total_ops,
            raw_source.parameterized_ops
        ),
        (1, 10, 13, 8),
    );
    assert_eq!(
        source_cost,
        ResynthesisCost {
            lowered_two_qubit_ops: 1,
            lowered_depth: 6,
            lowered_total_ops: 7,
            parameterized_ops: 4,
            backend_order: 0,
        }
    );
    let circuit = Circuit::from_operations(q.to_vec(), operations, None, None).unwrap();
    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    let candidates = plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
        matrix: &matrix,
        qubits: q,
        target: target.clone(),
    })
    .unwrap();
    assert!(!candidates.is_empty());
    for candidate in candidates {
        let raw_candidate = model
            .cost_of_fixed_operations(q.to_vec(), candidate.operations.clone())
            .unwrap();
        // Lowering alone favors the candidate; local cleanup reverses that
        // ordering because lowering exposes two adjacent H gates that cancel.
        assert_eq!(
            (
                raw_candidate.two_qubit_ops,
                raw_candidate.depth,
                raw_candidate.total_ops,
                raw_candidate.parameterized_ops
            ),
            (1, 7, 11, 6),
        );
        assert_eq!(candidate.cost.lowered_total_ops, 9);
        assert_eq!(candidate.cost.lowered_depth, 6);
        assert!(candidate.cost >= source_cost);
    }
}
