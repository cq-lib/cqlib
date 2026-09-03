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

use super::hadamard::{decompose_hadamard_n_clean, decompose_hadamard_no_aux};
use super::test_utils::{
    EPSILON, assert_fixed_parameter_operation, assert_selected_matrix_columns_approx_eq,
    assert_standard_operation, circuit_from_value_operations, mc_gate_matrix,
};
use crate::circuit::{Qubit, StandardGate, circuit_to_matrix};
use crate::compile::error::CompilerError;
use std::f64::consts::PI;

#[test]
fn zero_controls_emit_original_standard_h_and_ignore_clean_ancillas() {
    let target = Qubit::new(0);
    let operations = decompose_hadamard_n_clean(&[], target, &[target]).unwrap();

    assert_eq!(operations.len(), 1);
    assert_standard_operation(&operations[0], StandardGate::H, &[target]);
}

#[test]
fn one_control_emits_conditional_phase_and_zy_rotations() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    let operations = decompose_hadamard_no_aux(&[control], target).unwrap();

    assert_eq!(operations.len(), 3);
    assert_fixed_parameter_operation(&operations[0], StandardGate::Phase, &[control], PI / 2.0);
    assert_fixed_parameter_operation(&operations[1], StandardGate::CRZ, &[control, target], PI);
    assert_fixed_parameter_operation(
        &operations[2],
        StandardGate::CRY,
        &[control, target],
        PI / 2.0,
    );
}

#[test]
fn no_aux_decompositions_match_mcgate_semantics_exactly() {
    for num_controls in 1..=4 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let target = Qubit::new(num_controls as u32);
        let actual = circuit_to_matrix(
            &circuit_from_value_operations(
                num_controls + 1,
                decompose_hadamard_no_aux(&controls, target).unwrap(),
            ),
            None,
        )
        .unwrap();
        let mut qubits = controls.clone();
        qubits.push(target);
        let expected = mc_gate_matrix(
            num_controls + 1,
            controls.len() as u8,
            StandardGate::H,
            qubits,
            [],
        );

        assert_selected_matrix_columns_approx_eq(&actual, &expected, 0..expected.ncols(), EPSILON);
    }
}

#[test]
fn clean_decomposition_matches_clean_subspace_and_restores_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let actual = circuit_to_matrix(
        &circuit_from_value_operations(
            6,
            decompose_hadamard_n_clean(&controls, target, &clean_ancillas).unwrap(),
        ),
        None,
    )
    .unwrap();
    let mut qubits = controls.to_vec();
    qubits.push(target);
    let expected = mc_gate_matrix(6, controls.len() as u8, StandardGate::H, qubits, []);
    let clean_mask = clean_ancillas
        .iter()
        .fold(0_usize, |mask, qubit| mask | (1 << qubit.index()));
    let clean_columns = (0..expected.ncols()).filter(|state| state & clean_mask == 0);

    assert_selected_matrix_columns_approx_eq(&actual, &expected, clean_columns, EPSILON);
}

#[test]
fn underlying_mc_su2_errors_are_propagated_without_rewriting() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error = decompose_hadamard_n_clean(&controls, Qubit::new(3), &[]).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ref reason,
        } if reason
            == "clean-accumulator MC-SU(2) decomposition with 2 controls requires 1 clean ancillas, got 0"
    ));
}

#[test]
fn duplicate_qubit_errors_are_propagated_without_rewriting() {
    let duplicate = Qubit::new(0);
    let error = decompose_hadamard_no_aux(&[duplicate], duplicate).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ..
        }
    ));
}
