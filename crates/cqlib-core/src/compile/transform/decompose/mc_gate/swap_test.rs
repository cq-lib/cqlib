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

use super::swap::{decompose_swap_n_clean, decompose_swap_no_aux};
use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_approx_eq, assert_standard_operation,
    assert_value_operations_only_use_qubits, circuit_from_value_operations, mc_gate_matrix,
};
use crate::circuit::{Qubit, StandardGate, circuit_to_matrix};
use crate::compile::error::CompilerError;

#[test]
fn zero_controls_emit_original_standard_swap_and_ignore_clean_ancillas() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let operations = decompose_swap_n_clean(&[], first, second, &[first]).unwrap();

    assert_eq!(operations.len(), 1);
    assert_standard_operation(&operations[0], StandardGate::SWAP, &[first, second]);
}

#[test]
fn one_control_emits_fredkin_as_three_ccx_operations() {
    let control = Qubit::new(0);
    let first = Qubit::new(1);
    let second = Qubit::new(2);
    let operations = decompose_swap_n_clean(&[control], first, second, &[]).unwrap();

    assert_eq!(operations.len(), 3);
    assert_standard_operation(&operations[0], StandardGate::CCX, &[control, first, second]);
    assert_standard_operation(&operations[1], StandardGate::CCX, &[control, second, first]);
    assert_standard_operation(&operations[2], StandardGate::CCX, &[control, first, second]);
}

#[test]
fn no_aux_decompositions_match_mcgate_semantics_exactly() {
    for num_controls in 1..=3 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let first = Qubit::new(num_controls as u32);
        let second = Qubit::new(num_controls as u32 + 1);
        let total = num_controls + 2;
        let actual = circuit_to_matrix(
            &circuit_from_value_operations(
                total,
                decompose_swap_no_aux(&controls, first, second).unwrap(),
            ),
            None,
        )
        .unwrap();
        let mut qubits = controls.clone();
        qubits.extend([first, second]);
        let expected = mc_gate_matrix(total, controls.len() as u8, StandardGate::SWAP, qubits, []);

        assert_selected_matrix_columns_approx_eq(&actual, &expected, 0..expected.ncols(), EPSILON);
    }
}

#[test]
fn clean_decomposition_matches_clean_subspace_and_restores_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let first = Qubit::new(3);
    let second = Qubit::new(4);
    let clean_ancillas = [Qubit::new(5), Qubit::new(6)];
    let actual = circuit_to_matrix(
        &circuit_from_value_operations(
            7,
            decompose_swap_n_clean(&controls, first, second, &clean_ancillas).unwrap(),
        ),
        None,
    )
    .unwrap();
    let mut qubits = controls.to_vec();
    qubits.extend([first, second]);
    let expected = mc_gate_matrix(7, controls.len() as u8, StandardGate::SWAP, qubits, []);
    let clean_mask = clean_ancillas
        .iter()
        .fold(0_usize, |mask, qubit| mask | (1 << qubit.index()));
    let clean_columns = (0..expected.ncols()).filter(|state| state & clean_mask == 0);

    assert_selected_matrix_columns_approx_eq(&actual, &expected, clean_columns, EPSILON);
}

#[test]
fn extra_clean_ancillas_are_ignored_without_validation_or_use() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let used_ancilla = Qubit::new(4);
    let operations =
        decompose_swap_n_clean(&controls, first, second, &[used_ancilla, first, first]).unwrap();

    assert_value_operations_only_use_qubits(
        &operations,
        &[controls[0], controls[1], first, second, used_ancilla],
    );
}

#[test]
fn insufficient_clean_ancillas_are_rejected() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error = decompose_swap_n_clean(&controls, Qubit::new(3), Qubit::new(4), &[]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.swap",
            ref reason,
        } if reason
            == "clean-accumulator multi-controlled SWAP decomposition with 3 controls requires 2 clean ancillas, got 0"
    ));
}

#[test]
fn duplicate_qubits_are_rejected() {
    let duplicate = Qubit::new(0);
    let error = decompose_swap_no_aux(&[duplicate], duplicate, Qubit::new(1)).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.swap",
            ref reason,
        } if reason
            == "multi-controlled SWAP controls, targets, and ancillas must be distinct; duplicate Q0"
    ));
}
