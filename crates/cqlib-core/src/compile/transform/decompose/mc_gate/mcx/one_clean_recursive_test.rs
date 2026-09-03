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
    decompose_mcx_1_clean_b95, decompose_mcx_small,
    dirty_v_chain::{decompose_mcx_n_dirty, decompose_relative_phase_mcx_n_dirty},
    test_utils::{EPSILON, selected_basis_states, single_nonzero_statevector_output},
    utils::invert_parameter_free_operations,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Directive, Instruction, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
    operation::ValueOperation,
};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::{
    assert_value_operations_equal, circuit_from_value_operations, single_nonzero_matrix_output,
    statevector_after_value_operations,
};
use crate::qis::Statevector;
use num_complex::Complex64;
use smallvec::smallvec;

fn assert_duplicate_error(
    controls: &[Qubit],
    target: Qubit,
    clean_ancilla: Qubit,
    duplicate: Qubit,
) {
    let error = decompose_mcx_1_clean_b95(controls, target, clean_ancilla).unwrap_err();

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

fn assert_one_clean_ancilla_semantics(num_controls: usize) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let clean_ancilla = Qubit::new(num_controls as u32 + 1);
    let num_qubits = num_controls + 2;
    let operations = decompose_mcx_1_clean_b95(&controls, target, clean_ancilla).unwrap();
    let matrix =
        circuit_to_matrix(&circuit_from_value_operations(num_qubits, operations), None).unwrap();
    let mut global_phase = None;

    for input_basis_state in 0..1 << (num_controls + 1) {
        let (output_basis_state, amplitude) =
            single_nonzero_matrix_output(&matrix, input_basis_state, EPSILON);
        let all_controls_set = controls
            .iter()
            .all(|control| input_basis_state & (1 << control.index()) != 0);
        let expected_output = if all_controls_set {
            input_basis_state ^ (1 << target.index())
        } else {
            input_basis_state
        };

        assert_eq!(output_basis_state, expected_output);
        for control in &controls {
            assert_eq!(
                output_basis_state & (1 << control.index()),
                input_basis_state & (1 << control.index())
            );
        }
        assert_eq!(output_basis_state & (1 << clean_ancilla.index()), 0);
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);

        let expected_phase = global_phase.get_or_insert(amplitude);
        assert!(
            (amplitude - *expected_phase).norm() < EPSILON,
            "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
        );
    }
}

fn assert_one_clean_ancilla_selected_basis_semantics(num_controls: usize) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let clean_ancilla = Qubit::new(num_controls as u32 + 1);
    let num_qubits = num_controls + 2;
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << target.index();
    let operations = decompose_mcx_1_clean_b95(&controls, target, clean_ancilla).unwrap();
    let mut inputs = selected_basis_states(num_controls + 1);
    inputs.extend([control_mask, control_mask | target_mask]);
    inputs.sort_unstable();
    inputs.dedup();
    let mut global_phase = None;

    for input_basis_state in inputs {
        let mut amplitudes = vec![Complex64::new(0.0, 0.0); 1 << num_qubits];
        amplitudes[input_basis_state] = Complex64::new(1.0, 0.0);
        let initial_state = Statevector::from_state(num_qubits, amplitudes).unwrap();
        let output = statevector_after_value_operations(&initial_state, &operations);
        let (output_basis_state, amplitude) = single_nonzero_statevector_output(&output);
        let expected_output = if input_basis_state & control_mask == control_mask {
            input_basis_state ^ target_mask
        } else {
            input_basis_state
        };

        assert_eq!(output_basis_state, expected_output);
        assert_eq!(output_basis_state & (1 << clean_ancilla.index()), 0);
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);
        let expected_phase = global_phase.get_or_insert(amplitude);
        assert!(
            (amplitude - *expected_phase).norm() < EPSILON,
            "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
        );
    }
}

#[test]
fn one_clean_recursive_trivial_cases_match_trivial_decomposition() {
    let target = Qubit::new(2);

    for (controls, clean_ancilla) in [
        (vec![], target),
        (vec![Qubit::new(0)], target),
        (vec![Qubit::new(0), Qubit::new(1)], Qubit::new(0)),
    ] {
        let expected = decompose_mcx_small(&controls, target).unwrap();
        let operations = decompose_mcx_1_clean_b95(&controls, target, clean_ancilla).unwrap();

        assert_value_operations_equal(&operations, &expected);
    }
}

#[test]
fn one_clean_recursive_rejects_duplicate_controls() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, duplicate],
        Qubit::new(3),
        Qubit::new(4),
        duplicate,
    );
}

#[test]
fn one_clean_recursive_rejects_target_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        duplicate,
        Qubit::new(4),
        duplicate,
    );
}

#[test]
fn one_clean_recursive_rejects_clean_ancilla_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        Qubit::new(3),
        duplicate,
        duplicate,
    );
}

#[test]
fn one_clean_recursive_rejects_clean_ancilla_matching_target() {
    let duplicate = Qubit::new(3);
    assert_duplicate_error(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        duplicate,
        duplicate,
        duplicate,
    );
}

#[test]
fn one_clean_recursive_emits_b95_four_segment_structure() {
    for num_controls in 3..=6 {
        let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
        let target = Qubit::new(num_controls as u32);
        let clean_ancilla = Qubit::new(num_controls as u32 + 1);
        let middle = (num_controls + 2) / 2;
        let first_controls = &controls[..middle];
        let first_dirty_ancillas = &controls[middle..middle + first_controls.len() - 2];
        let mut second_controls = controls[middle..].to_vec();
        second_controls.push(clean_ancilla);
        let second_dirty_ancillas = &controls[..second_controls.len() - 2];

        let first = decompose_relative_phase_mcx_n_dirty(
            first_controls,
            clean_ancilla,
            first_dirty_ancillas,
        )
        .unwrap();
        let second =
            decompose_mcx_n_dirty(&second_controls, target, second_dirty_ancillas).unwrap();
        let inverse_first = invert_parameter_free_operations(&first).unwrap();
        let mut expected = first;
        expected.extend(second.iter().cloned());
        expected.extend(inverse_first);
        expected.extend(second);

        let operations = decompose_mcx_1_clean_b95(&controls, target, clean_ancilla).unwrap();
        assert_value_operations_equal(&operations, &expected);
    }
}

#[test]
fn one_clean_recursive_with_three_controls_matches_mcx() {
    assert_one_clean_ancilla_semantics(3);
}

#[test]
fn one_clean_recursive_with_four_controls_matches_mcx() {
    assert_one_clean_ancilla_semantics(4);
}

#[test]
fn one_clean_recursive_with_five_controls_matches_mcx() {
    assert_one_clean_ancilla_semantics(5);
}

#[test]
fn invert_parameter_free_operations_reverses_and_inverts_operations() {
    let operations = vec![
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::H)),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::CX)),
            qubits: smallvec![Qubit::new(0), Qubit::new(1)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::T)),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::TDG)),
            qubits: smallvec![Qubit::new(2)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::CCX)),
            qubits: smallvec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            params: smallvec![],
            label: None,
        },
    ];
    let expected = vec![
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::CCX)),
            qubits: smallvec![Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::T)),
            qubits: smallvec![Qubit::new(2)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::TDG)),
            qubits: smallvec![Qubit::new(1)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::CX)),
            qubits: smallvec![Qubit::new(0), Qubit::new(1)],
            params: smallvec![],
            label: None,
        },
        ValueOperation {
            instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::H)),
            qubits: smallvec![Qubit::new(0)],
            params: smallvec![],
            label: None,
        },
    ];

    let inverse = invert_parameter_free_operations(&operations).unwrap();

    assert_value_operations_equal(&inverse, &expected);
}

#[test]
fn invert_parameter_free_operations_rejects_parameters() {
    let operation = ValueOperation {
        instruction: ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZ)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![ParameterValue::Fixed(0.5)],
        label: None,
    };

    let error = invert_parameter_free_operations(&[operation]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason == "MCX operation inversion requires parameter-free operations"
    ));
}

#[test]
fn invert_parameter_free_operations_rejects_non_invertible_instructions() {
    let operation = ValueOperation {
        instruction: ValueInstruction::Instruction(Instruction::Directive(Directive::Measure)),
        qubits: smallvec![Qubit::new(0)],
        params: smallvec![],
        label: None,
    };

    let error = invert_parameter_free_operations(&[operation]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason
            == "MCX operation inversion does not support instruction Directive(Measure)"
    ));
}

#[test]
fn one_clean_recursive_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [6, 7, 8, 9, 10] {
        assert_one_clean_ancilla_selected_basis_semantics(num_controls);
    }
}
