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
    decompose_mcx_n_dirty, decompose_mcx_small,
    dirty_v_chain::decompose_relative_phase_mcx_n_dirty,
    test_utils::{
        EPSILON, assert_rccx_expansion, selected_basis_states, single_nonzero_statevector_output,
    },
};
use crate::circuit::{Qubit, StandardGate, circuit_to_matrix, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::{
    assert_standard_operation, assert_value_operations_equal, circuit_from_value_operations,
    single_nonzero_matrix_output, statevector_after_value_operations,
};
use crate::qis::Statevector;
use num_complex::Complex64;

#[derive(Debug, Clone, Copy)]
enum SemanticMode {
    Exact,
    RelativePhase,
}

fn assert_action_gadget(
    operations: &[ValueOperation],
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    let expected = [
        (StandardGate::H, vec![target]),
        (StandardGate::T, vec![target]),
        (StandardGate::CX, vec![first_control, target]),
        (StandardGate::TDG, vec![target]),
        (StandardGate::CX, vec![second_control, target]),
    ];
    assert_eq!(operations.len(), expected.len());
    for (operation, (gate, qubits)) in operations.iter().zip(expected) {
        assert_standard_operation(operation, gate, &qubits);
    }
}

fn assert_reset_gadget(
    operations: &[ValueOperation],
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    let expected = [
        (StandardGate::CX, vec![second_control, target]),
        (StandardGate::T, vec![target]),
        (StandardGate::CX, vec![first_control, target]),
        (StandardGate::TDG, vec![target]),
        (StandardGate::H, vec![target]),
    ];
    assert_eq!(operations.len(), expected.len());
    for (operation, (gate, qubits)) in operations.iter().zip(expected) {
        assert_standard_operation(operation, gate, &qubits);
    }
}

fn assert_duplicate_error(
    controls: &[Qubit],
    target: Qubit,
    dirty_ancillas: &[Qubit],
    duplicate: Qubit,
) {
    let error = decompose_mcx_n_dirty(controls, target, dirty_ancillas).unwrap_err();

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

fn assert_dirty_ancilla_semantics(num_controls: usize, mode: SemanticMode) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let dirty_ancillas: Vec<_> = (num_controls + 1..2 * num_controls - 1)
        .map(|id| Qubit::new(id as u32))
        .collect();
    let num_qubits = 2 * num_controls - 1;
    let operations = match mode {
        SemanticMode::Exact => decompose_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap(),
        SemanticMode::RelativePhase => {
            decompose_relative_phase_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap()
        }
    };
    let matrix =
        circuit_to_matrix(&circuit_from_value_operations(num_qubits, operations), None).unwrap();
    let mut global_phase = None;

    for input_basis_state in 0..1 << num_qubits {
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
        for ancilla in &dirty_ancillas {
            assert_eq!(
                output_basis_state & (1 << ancilla.index()),
                input_basis_state & (1 << ancilla.index())
            );
        }
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);

        if matches!(mode, SemanticMode::Exact) {
            let expected_phase = global_phase.get_or_insert(amplitude);
            assert!(
                (amplitude - *expected_phase).norm() < EPSILON,
                "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
            );
        }
    }
}

fn assert_dirty_ancilla_selected_basis_semantics(num_controls: usize, mode: SemanticMode) {
    let controls: Vec<_> = (0..num_controls).map(|id| Qubit::new(id as u32)).collect();
    let target = Qubit::new(num_controls as u32);
    let dirty_ancillas: Vec<_> = (num_controls + 1..2 * num_controls - 1)
        .map(|id| Qubit::new(id as u32))
        .collect();
    let num_qubits = 2 * num_controls - 1;
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << target.index();
    let dirty_mask = (1_usize << dirty_ancillas.len()) - 1;
    let alternating_dirty = (0..dirty_ancillas.len())
        .filter(|index| index % 2 == 0)
        .fold(0_usize, |state, index| state | (1_usize << index));
    let operations = match mode {
        SemanticMode::Exact => decompose_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap(),
        SemanticMode::RelativePhase => {
            decompose_relative_phase_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap()
        }
    };
    let mut data_inputs = selected_basis_states(num_controls + 1);
    data_inputs.extend([control_mask, control_mask | target_mask]);
    data_inputs.sort_unstable();
    data_inputs.dedup();
    let mut dirty_inputs = vec![
        0,
        dirty_mask,
        alternating_dirty,
        dirty_mask ^ alternating_dirty,
    ];
    dirty_inputs.sort_unstable();
    dirty_inputs.dedup();
    let mut global_phase = None;

    for dirty_input in dirty_inputs {
        for data_input in data_inputs.iter().copied() {
            let input_basis_state = data_input | (dirty_input << (num_controls + 1));
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
            assert!((amplitude.norm() - 1.0).abs() < EPSILON);
            if matches!(mode, SemanticMode::Exact) {
                let expected_phase = global_phase.get_or_insert(amplitude);
                assert!(
                    (amplitude - *expected_phase).norm() < EPSILON,
                    "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
                );
            }
        }
    }
}

#[test]
fn dirty_v_chain_trivial_cases_match_trivial_decomposition() {
    let target = Qubit::new(2);

    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let expected = decompose_mcx_small(controls, target).unwrap();
        let exact = decompose_mcx_n_dirty(controls, target, &[]).unwrap();
        let relative = decompose_relative_phase_mcx_n_dirty(controls, target, &[]).unwrap();

        assert_value_operations_equal(&exact, &expected);
        assert_value_operations_equal(&relative, &expected);
    }
}

#[test]
fn dirty_v_chain_trivial_cases_ignore_extra_ancillas() {
    let target = Qubit::new(2);
    let extra_ancillas = [target, Qubit::new(0), target];

    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let expected = decompose_mcx_small(controls, target).unwrap();
        let exact = decompose_mcx_n_dirty(controls, target, &extra_ancillas).unwrap();
        let relative =
            decompose_relative_phase_mcx_n_dirty(controls, target, &extra_ancillas).unwrap();

        assert_value_operations_equal(&exact, &expected);
        assert_value_operations_equal(&relative, &expected);
    }
}

#[test]
fn dirty_v_chain_exact_with_three_controls_emits_two_ladders() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let dirty_ancillas = [Qubit::new(4)];

    let operations = decompose_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap();

    assert_eq!(operations.len(), 20);
    assert_standard_operation(
        &operations[0],
        StandardGate::CCX,
        &[controls[2], dirty_ancillas[0], target],
    );
    assert_rccx_expansion(
        &operations[1..10],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
    assert_standard_operation(
        &operations[10],
        StandardGate::CCX,
        &[controls[2], dirty_ancillas[0], target],
    );
    assert_rccx_expansion(
        &operations[11..],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
}

#[test]
fn dirty_v_chain_exact_with_four_controls_emits_two_ladders() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)];
    let target = Qubit::new(4);
    let dirty_ancillas = [Qubit::new(5), Qubit::new(6)];

    let operations = decompose_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap();

    assert_eq!(operations.len(), 40);
    for offset in [0, 20] {
        assert_standard_operation(
            &operations[offset],
            StandardGate::CCX,
            &[controls[3], dirty_ancillas[1], target],
        );
        assert_action_gadget(
            &operations[offset + 1..offset + 6],
            controls[2],
            dirty_ancillas[0],
            dirty_ancillas[1],
        );
        assert_rccx_expansion(
            &operations[offset + 6..offset + 15],
            controls[0],
            controls[1],
            dirty_ancillas[0],
        );
        assert_reset_gadget(
            &operations[offset + 15..offset + 20],
            controls[2],
            dirty_ancillas[0],
            dirty_ancillas[1],
        );
    }
}

#[test]
fn dirty_v_chain_exact_with_five_controls_emits_two_internal_gadgets_per_ladder() {
    let controls = [
        Qubit::new(0),
        Qubit::new(1),
        Qubit::new(2),
        Qubit::new(3),
        Qubit::new(4),
    ];
    let target = Qubit::new(5);
    let dirty_ancillas = [Qubit::new(6), Qubit::new(7), Qubit::new(8)];

    let operations = decompose_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap();

    assert_eq!(operations.len(), 60);
    for offset in [0, 30] {
        assert_standard_operation(
            &operations[offset],
            StandardGate::CCX,
            &[controls[4], dirty_ancillas[2], target],
        );
        assert_action_gadget(
            &operations[offset + 1..offset + 6],
            controls[3],
            dirty_ancillas[1],
            dirty_ancillas[2],
        );
        assert_action_gadget(
            &operations[offset + 6..offset + 11],
            controls[2],
            dirty_ancillas[0],
            dirty_ancillas[1],
        );
        assert_rccx_expansion(
            &operations[offset + 11..offset + 20],
            controls[0],
            controls[1],
            dirty_ancillas[0],
        );
        assert_reset_gadget(
            &operations[offset + 20..offset + 25],
            controls[2],
            dirty_ancillas[0],
            dirty_ancillas[1],
        );
        assert_reset_gadget(
            &operations[offset + 25..offset + 30],
            controls[3],
            dirty_ancillas[1],
            dirty_ancillas[2],
        );
    }
}

#[test]
fn dirty_v_chain_relative_phase_with_three_controls_emits_endpoint_gadgets() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let dirty_ancillas = [Qubit::new(4)];

    let operations =
        decompose_relative_phase_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap();

    assert_eq!(operations.len(), 28);
    assert_action_gadget(&operations[..5], controls[2], dirty_ancillas[0], target);
    assert_rccx_expansion(
        &operations[5..14],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
    assert_reset_gadget(&operations[14..19], controls[2], dirty_ancillas[0], target);
    assert_rccx_expansion(
        &operations[19..],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
}

#[test]
fn dirty_v_chain_relative_phase_with_four_controls_emits_two_ladders() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)];
    let target = Qubit::new(4);
    let dirty_ancillas = [Qubit::new(5), Qubit::new(6)];

    let operations =
        decompose_relative_phase_mcx_n_dirty(&controls, target, &dirty_ancillas).unwrap();

    assert_eq!(operations.len(), 48);
    assert_action_gadget(&operations[..5], controls[3], dirty_ancillas[1], target);
    assert_action_gadget(
        &operations[5..10],
        controls[2],
        dirty_ancillas[0],
        dirty_ancillas[1],
    );
    assert_rccx_expansion(
        &operations[10..19],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
    assert_reset_gadget(
        &operations[19..24],
        controls[2],
        dirty_ancillas[0],
        dirty_ancillas[1],
    );
    assert_reset_gadget(&operations[24..29], controls[3], dirty_ancillas[1], target);
    assert_action_gadget(
        &operations[29..34],
        controls[2],
        dirty_ancillas[0],
        dirty_ancillas[1],
    );
    assert_rccx_expansion(
        &operations[34..43],
        controls[0],
        controls[1],
        dirty_ancillas[0],
    );
    assert_reset_gadget(
        &operations[43..],
        controls[2],
        dirty_ancillas[0],
        dirty_ancillas[1],
    );
}

#[test]
fn dirty_v_chain_rejects_insufficient_dirty_ancillas() {
    let error = decompose_mcx_n_dirty(
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
            == "dirty-ancilla MCX decomposition with 4 controls requires 2 dirty ancillas, got 1"
    ));
}

#[test]
fn dirty_v_chain_rejects_duplicate_controls() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, duplicate],
        Qubit::new(3),
        &[Qubit::new(4)],
        duplicate,
    );
}

#[test]
fn dirty_v_chain_rejects_target_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        duplicate,
        &[Qubit::new(4)],
        duplicate,
    );
}

#[test]
fn dirty_v_chain_rejects_used_ancilla_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        Qubit::new(3),
        &[duplicate],
        duplicate,
    );
}

#[test]
fn dirty_v_chain_rejects_used_ancilla_matching_target() {
    let duplicate = Qubit::new(3);
    assert_duplicate_error(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        duplicate,
        &[duplicate],
        duplicate,
    );
}

#[test]
fn dirty_v_chain_rejects_duplicate_used_ancillas() {
    let duplicate = Qubit::new(5);
    assert_duplicate_error(
        &[Qubit::new(0), Qubit::new(1), Qubit::new(2), Qubit::new(3)],
        Qubit::new(4),
        &[duplicate, duplicate],
        duplicate,
    );
}

#[test]
fn dirty_v_chain_ignores_unused_extra_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let used_ancilla = Qubit::new(4);
    let operations = decompose_mcx_n_dirty(&controls, target, &[used_ancilla]).unwrap();
    let operations_with_extras = decompose_mcx_n_dirty(
        &controls,
        target,
        &[used_ancilla, controls[0], target, used_ancilla],
    )
    .unwrap();
    let relative_operations =
        decompose_relative_phase_mcx_n_dirty(&controls, target, &[used_ancilla]).unwrap();
    let relative_operations_with_extras = decompose_relative_phase_mcx_n_dirty(
        &controls,
        target,
        &[used_ancilla, controls[0], target, used_ancilla],
    )
    .unwrap();

    assert_value_operations_equal(&operations_with_extras, &operations);
    assert_value_operations_equal(&relative_operations_with_extras, &relative_operations);
}

#[test]
fn dirty_v_chain_exact_with_three_controls_matches_mcx() {
    assert_dirty_ancilla_semantics(3, SemanticMode::Exact);
}

#[test]
fn dirty_v_chain_exact_with_four_controls_matches_mcx() {
    assert_dirty_ancilla_semantics(4, SemanticMode::Exact);
}

#[test]
fn dirty_v_chain_exact_with_five_controls_matches_mcx() {
    assert_dirty_ancilla_semantics(5, SemanticMode::Exact);
}

#[test]
fn dirty_v_chain_relative_phase_with_three_controls_matches_mcx_bit_flip_behavior() {
    assert_dirty_ancilla_semantics(3, SemanticMode::RelativePhase);
}

#[test]
fn dirty_v_chain_relative_phase_with_four_controls_matches_mcx_bit_flip_behavior() {
    assert_dirty_ancilla_semantics(4, SemanticMode::RelativePhase);
}

#[test]
fn dirty_v_chain_exact_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [6, 7, 8] {
        assert_dirty_ancilla_selected_basis_semantics(num_controls, SemanticMode::Exact);
    }
}

#[test]
fn dirty_v_chain_relative_phase_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [5, 6, 7, 8] {
        assert_dirty_ancilla_selected_basis_semantics(num_controls, SemanticMode::RelativePhase);
    }
}
