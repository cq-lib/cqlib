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

use super::relative_phase::emit_relative_phase_toffoli;
use super::test_utils::EPSILON;
use crate::circuit::test_utils::assert_matrix_approx_eq;
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Circuit, Instruction, Qubit, StandardGate, circuit_to_matrix, operation::ValueOperation,
};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::{
    assert_standard_operation, circuit_from_value_operations, single_nonzero_matrix_output,
};
use ndarray::Array2;
use smallvec::smallvec;
fn emit_rccx() -> Vec<ValueOperation> {
    let mut operations = vec![];
    emit_relative_phase_toffoli(&mut operations, Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    operations
}

fn assert_duplicate_error(
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
    duplicate: Qubit,
) {
    let error = emit_relative_phase_toffoli(&mut vec![], first_control, second_control, target)
        .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason
            == &format!(
                "MCX controls, target, and ancillas must be distinct; duplicate {duplicate}"
            )
    ));
}

#[test]
fn relative_phase_toffoli_emits_rccx_sequence() {
    let first_control = Qubit::new(0);
    let second_control = Qubit::new(1);
    let target = Qubit::new(2);
    let mut operations = vec![];

    emit_relative_phase_toffoli(&mut operations, first_control, second_control, target).unwrap();

    let expected = [
        (StandardGate::H, vec![target]),
        (StandardGate::T, vec![target]),
        (StandardGate::CX, vec![second_control, target]),
        (StandardGate::TDG, vec![target]),
        (StandardGate::CX, vec![first_control, target]),
        (StandardGate::T, vec![target]),
        (StandardGate::CX, vec![second_control, target]),
        (StandardGate::TDG, vec![target]),
        (StandardGate::H, vec![target]),
    ];

    assert_eq!(operations.len(), expected.len());
    for (operation, (gate, qubits)) in operations.iter().zip(expected) {
        assert_standard_operation(operation, gate, &qubits);
    }
}

#[test]
fn relative_phase_toffoli_appends_after_existing_operations() {
    let prefix = ValueOperation {
        instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::X)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };
    let mut operations = vec![prefix];

    emit_relative_phase_toffoli(&mut operations, Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    assert_eq!(operations.len(), 10);
    assert_standard_operation(&operations[0], StandardGate::X, &[Qubit::new(0)]);
    assert_standard_operation(&operations[1], StandardGate::H, &[Qubit::new(2)]);
}

#[test]
fn relative_phase_toffoli_rejects_matching_controls() {
    let duplicate = Qubit::new(0);
    assert_duplicate_error(duplicate, duplicate, Qubit::new(2), duplicate);
}

#[test]
fn relative_phase_toffoli_rejects_first_control_matching_target() {
    let duplicate = Qubit::new(0);
    assert_duplicate_error(duplicate, Qubit::new(1), duplicate, duplicate);
}

#[test]
fn relative_phase_toffoli_rejects_second_control_matching_target() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(Qubit::new(0), duplicate, duplicate, duplicate);
}

#[test]
fn relative_phase_toffoli_does_not_append_operations_after_validation_error() {
    let prefix = ValueOperation {
        instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::X)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };
    let mut operations = vec![prefix];

    emit_relative_phase_toffoli(&mut operations, Qubit::new(0), Qubit::new(0), Qubit::new(2))
        .unwrap_err();

    assert_eq!(operations.len(), 1);
    assert_standard_operation(&operations[0], StandardGate::X, &[Qubit::new(0)]);
}

#[test]
fn relative_phase_toffoli_matches_ccx_computational_basis_mapping() {
    let rccx_matrix =
        circuit_to_matrix(&circuit_from_value_operations(3, emit_rccx()), None).unwrap();
    let mut ccx_circuit = Circuit::new(3);
    ccx_circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    let ccx_matrix = circuit_to_matrix(&ccx_circuit, None).unwrap();

    for input_basis_state in 0..8 {
        let (rccx_output, rccx_amplitude) =
            single_nonzero_matrix_output(&rccx_matrix, input_basis_state, EPSILON);
        let (ccx_output, _) = single_nonzero_matrix_output(&ccx_matrix, input_basis_state, EPSILON);

        assert_eq!(rccx_output, ccx_output);
        assert!((rccx_amplitude.norm() - 1.0).abs() < EPSILON);
    }
}

#[test]
fn relative_phase_toffoli_is_self_inverse() {
    let mut operations = emit_rccx();
    emit_relative_phase_toffoli(&mut operations, Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    let matrix = circuit_to_matrix(&circuit_from_value_operations(3, operations), None).unwrap();
    let identity = Array2::eye(8);

    assert_matrix_approx_eq(&matrix, &identity, EPSILON);
}
