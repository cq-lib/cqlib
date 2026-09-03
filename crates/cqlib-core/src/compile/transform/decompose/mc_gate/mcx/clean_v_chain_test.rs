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
    decompose_mcx_n_clean, decompose_mcx_small,
    test_utils::{
        EPSILON, assert_rccx_expansion, selected_basis_states, single_nonzero_statevector_output,
    },
};
use crate::circuit::{Qubit, StandardGate, circuit_to_matrix};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::{
    assert_standard_operation, assert_value_operations_equal, circuit_from_value_operations,
    single_nonzero_matrix_output, statevector_after_value_operations,
};
use crate::qis::Statevector;
use num_complex::Complex64;

fn assert_duplicate_error(
    controls: &[Qubit],
    target: Qubit,
    clean_ancillas: &[Qubit],
    duplicate: Qubit,
) {
    let error = decompose_mcx_n_clean(controls, target, clean_ancillas).unwrap_err();

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

fn assert_clean_subspace_semantics(num_controls: usize) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let clean_ancillas: Vec<_> = (num_controls + 1..2 * num_controls - 1)
        .map(|id| Qubit::new(id as u32))
        .collect();
    let num_qubits = 2 * num_controls - 1;
    let operations = decompose_mcx_n_clean(&controls, target, &clean_ancillas).unwrap();
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
        for ancilla in &clean_ancillas {
            assert_eq!(output_basis_state & (1 << ancilla.index()), 0);
        }
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);

        let expected_phase = global_phase.get_or_insert(amplitude);
        assert!(
            (amplitude - *expected_phase).norm() < EPSILON,
            "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
        );
    }
}

fn assert_clean_subspace_selected_basis_semantics(num_controls: usize) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let clean_ancillas: Vec<_> = (num_controls + 1..2 * num_controls - 1)
        .map(|id| Qubit::new(id as u32))
        .collect();
    let num_qubits = 2 * num_controls - 1;
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << target.index();
    let operations = decompose_mcx_n_clean(&controls, target, &clean_ancillas).unwrap();
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
        for ancilla in &clean_ancillas {
            assert_eq!(output_basis_state & (1 << ancilla.index()), 0);
        }
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);
        let expected_phase = global_phase.get_or_insert(amplitude);
        assert!(
            (amplitude - *expected_phase).norm() < EPSILON,
            "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
        );
    }
}

#[test]
fn clean_v_chain_without_controls_matches_trivial_decomposition() {
    let target = Qubit::new(0);

    let operations = decompose_mcx_n_clean(&[], target, &[]).unwrap();

    assert_value_operations_equal(&operations, &decompose_mcx_small(&[], target).unwrap());
}

#[test]
fn clean_v_chain_with_one_control_matches_trivial_decomposition() {
    let controls = [Qubit::new(0)];
    let target = Qubit::new(1);

    let operations = decompose_mcx_n_clean(&controls, target, &[]).unwrap();

    assert_value_operations_equal(
        &operations,
        &decompose_mcx_small(&controls, target).unwrap(),
    );
}

#[test]
fn clean_v_chain_with_two_controls_matches_trivial_decomposition() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);

    let operations = decompose_mcx_n_clean(&controls, target, &[]).unwrap();

    assert_value_operations_equal(
        &operations,
        &decompose_mcx_small(&controls, target).unwrap(),
    );
}

#[test]
fn clean_v_chain_trivial_cases_ignore_extra_ancillas() {
    let target = Qubit::new(2);
    let extra_ancillas = [target, Qubit::new(0), target];

    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let operations = decompose_mcx_n_clean(controls, target, &extra_ancillas).unwrap();

        assert_value_operations_equal(&operations, &decompose_mcx_small(controls, target).unwrap());
    }
}

#[test]
fn clean_v_chain_with_three_controls_emits_compute_target_uncompute() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4)];

    let operations = decompose_mcx_n_clean(&controls, target, &clean_ancillas).unwrap();

    assert_eq!(operations.len(), 19);
    assert_rccx_expansion(
        &operations[..9],
        controls[0],
        controls[1],
        clean_ancillas[0],
    );
    assert_standard_operation(
        &operations[9],
        StandardGate::CCX,
        &[controls[2], clean_ancillas[0], target],
    );
    assert_rccx_expansion(
        &operations[10..],
        controls[0],
        controls[1],
        clean_ancillas[0],
    );
}

#[test]
fn clean_v_chain_with_four_controls_emits_compute_target_reverse_uncompute() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)];
    let target = Qubit::new(4);
    let clean_ancillas = [Qubit::new(5), Qubit::new(6)];

    let operations = decompose_mcx_n_clean(&controls, target, &clean_ancillas).unwrap();

    assert_eq!(operations.len(), 37);
    assert_rccx_expansion(
        &operations[..9],
        controls[0],
        controls[1],
        clean_ancillas[0],
    );
    assert_rccx_expansion(
        &operations[9..18],
        controls[2],
        clean_ancillas[0],
        clean_ancillas[1],
    );
    assert_standard_operation(
        &operations[18],
        StandardGate::CCX,
        &[controls[3], clean_ancillas[1], target],
    );
    assert_rccx_expansion(
        &operations[19..28],
        controls[2],
        clean_ancillas[0],
        clean_ancillas[1],
    );
    assert_rccx_expansion(
        &operations[28..],
        controls[0],
        controls[1],
        clean_ancillas[0],
    );
}

#[test]
fn clean_v_chain_with_five_controls_emits_expected_operation_count() {
    let controls = [
        Qubit::new(0),
        Qubit::new(1),
        Qubit::new(2),
        Qubit::new(3),
        Qubit::new(4),
    ];
    let target = Qubit::new(5);
    let clean_ancillas = [Qubit::new(6), Qubit::new(7), Qubit::new(8)];

    let operations = decompose_mcx_n_clean(&controls, target, &clean_ancillas).unwrap();

    assert_eq!(operations.len(), 55);
}

#[test]
fn clean_v_chain_rejects_insufficient_clean_ancillas() {
    let error = decompose_mcx_n_clean(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)],
        Qubit::new(4),
        &[Qubit::new(5)],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mcx",
            ref reason,
        } if reason
            == "clean-ancilla MCX decomposition with 4 controls requires 2 clean ancillas, got 1"
    ));
}

#[test]
fn clean_v_chain_rejects_duplicate_controls() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, duplicate],
        Qubit::new(3),
        &[Qubit::new(4)],
        duplicate,
    );
}

#[test]
fn clean_v_chain_rejects_target_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        duplicate,
        &[Qubit::new(4)],
        duplicate,
    );
}

#[test]
fn clean_v_chain_rejects_used_ancilla_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        Qubit::new(3),
        &[duplicate],
        duplicate,
    );
}

#[test]
fn clean_v_chain_rejects_used_ancilla_matching_target() {
    let duplicate = Qubit::new(3);
    assert_duplicate_error(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        duplicate,
        &[duplicate],
        duplicate,
    );
}

#[test]
fn clean_v_chain_rejects_duplicate_used_ancillas() {
    let duplicate = Qubit::new(5);
    assert_duplicate_error(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)],
        Qubit::new(4),
        &[duplicate, duplicate],
        duplicate,
    );
}

#[test]
fn clean_v_chain_ignores_unused_extra_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let used_ancilla = Qubit::new(4);
    let operations = decompose_mcx_n_clean(&controls, target, &[used_ancilla]).unwrap();
    let operations_with_extras = decompose_mcx_n_clean(
        &controls,
        target,
        &[used_ancilla, controls[0], target, used_ancilla],
    )
    .unwrap();

    assert_value_operations_equal(&operations_with_extras, &operations);
}

#[test]
fn clean_v_chain_validates_all_used_qubits_before_returning_operations() {
    let duplicate = Qubit::new(0);
    let result = decompose_mcx_n_clean(
        &[duplicate, Qubit::new(1), Qubit::new(2), Qubit::new(3)],
        Qubit::new(4),
        &[Qubit::new(5), duplicate],
    );

    assert!(result.is_err());
}

#[test]
fn clean_v_chain_with_three_controls_matches_mcx_on_clean_subspace() {
    assert_clean_subspace_semantics(3);
}

#[test]
fn clean_v_chain_with_four_controls_matches_mcx_on_clean_subspace() {
    assert_clean_subspace_semantics(4);
}

#[test]
fn clean_v_chain_with_five_controls_matches_mcx_on_clean_subspace() {
    assert_clean_subspace_semantics(5);
}

#[test]
fn clean_v_chain_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [6, 7, 8] {
        assert_clean_subspace_selected_basis_semantics(num_controls);
    }
}
