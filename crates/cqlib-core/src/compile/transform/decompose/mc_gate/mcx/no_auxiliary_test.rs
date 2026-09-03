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
    decompose_mcx_no_aux, decompose_mcx_small,
    no_auxiliary::{
        Hp24Path, IncrementDirection, emit_increment_n_dirty, emit_increment_one_dirty,
        emit_increment_two_dirty, emit_relative_mcx_with_internal_dirty_qubits, select_hp24_path,
    },
    test_utils::{EPSILON, selected_basis_states},
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{Instruction, ParameterValue, Qubit, StandardGate, operation::ValueOperation};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::assert_value_operations_equal;
use num_complex::Complex64;
use std::collections::{BTreeMap, HashSet};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

// This bound counts the explicitly expanded Phase, CX, CCX, H, T, and TDG
// operations. It is intentionally looser than the paper's CX-only bound while
// still ruling out subset enumeration and exponential parity polynomials.
const LINEAR_OPERATION_BOUND_FACTOR: usize = 300;
const LINEAR_OPERATION_BOUND_OFFSET: usize = 1_000;

fn assert_duplicate_error(controls: &[Qubit], target: Qubit, duplicate: Qubit) {
    let error = decompose_mcx_no_aux(controls, target).unwrap_err();

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

fn qubits(start: usize, count: usize) -> Vec<Qubit> {
    (start..start + count)
        .map(|index| Qubit::new(index as u32))
        .collect()
}

fn add_amplitude(state: &mut BTreeMap<usize, Complex64>, basis_state: usize, amplitude: Complex64) {
    let sum = state.entry(basis_state).or_default();
    *sum += amplitude;
    if sum.norm() < EPSILON {
        state.remove(&basis_state);
    }
}

fn phase_parameter(operation: &ValueOperation) -> f64 {
    match operation.params.as_slice() {
        [ParameterValue::Fixed(parameter)] => *parameter,
        parameters => panic!("expected one fixed parameter, got {parameters:?}"),
    }
}

fn apply_operations_to_basis_state(
    operations: &[ValueOperation],
    input_basis_state: usize,
) -> BTreeMap<usize, Complex64> {
    let mut state = BTreeMap::from([(input_basis_state, Complex64::new(1.0, 0.0))]);

    for operation in operations {
        let mut next = BTreeMap::new();
        match operation.instruction {
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::X)) => {
                let target = operation.qubits[0].index();
                for (basis_state, amplitude) in state {
                    add_amplitude(&mut next, basis_state ^ (1 << target), amplitude);
                }
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::H)) => {
                let target = operation.qubits[0].index();
                for (basis_state, amplitude) in state {
                    let sign = if basis_state & (1 << target) == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let cleared = basis_state & !(1 << target);
                    add_amplitude(&mut next, cleared, amplitude * FRAC_1_SQRT_2);
                    add_amplitude(
                        &mut next,
                        cleared | (1 << target),
                        amplitude * sign * FRAC_1_SQRT_2,
                    );
                }
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::T)) => {
                apply_phase(&mut next, state, operation.qubits[0], PI / 4.0);
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::TDG)) => {
                apply_phase(&mut next, state, operation.qubits[0], -PI / 4.0);
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::Phase)) => {
                apply_phase(
                    &mut next,
                    state,
                    operation.qubits[0],
                    phase_parameter(operation),
                );
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::CX)) => {
                let control = operation.qubits[0].index();
                let target = operation.qubits[1].index();
                for (basis_state, amplitude) in state {
                    let output = if basis_state & (1 << control) != 0 {
                        basis_state ^ (1 << target)
                    } else {
                        basis_state
                    };
                    add_amplitude(&mut next, output, amplitude);
                }
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::CCX)) => {
                let first_control = operation.qubits[0].index();
                let second_control = operation.qubits[1].index();
                let target = operation.qubits[2].index();
                for (basis_state, amplitude) in state {
                    let output = if basis_state & (1 << first_control) != 0
                        && basis_state & (1 << second_control) != 0
                    {
                        basis_state ^ (1 << target)
                    } else {
                        basis_state
                    };
                    add_amplitude(&mut next, output, amplitude);
                }
            }
            ref instruction => panic!("unexpected instruction: {instruction:?}"),
        }
        state = next;
    }

    state
}

fn apply_phase(
    next: &mut BTreeMap<usize, Complex64>,
    state: BTreeMap<usize, Complex64>,
    target: Qubit,
    theta: f64,
) {
    let phase = Complex64::from_polar(1.0, theta);
    for (basis_state, amplitude) in state {
        let output_amplitude = if basis_state & (1 << target.index()) != 0 {
            amplitude * phase
        } else {
            amplitude
        };
        add_amplitude(next, basis_state, output_amplitude);
    }
}

fn single_nonzero_output(
    operations: &[ValueOperation],
    input_basis_state: usize,
) -> (usize, Complex64) {
    let outputs = apply_operations_to_basis_state(operations, input_basis_state);
    assert_eq!(
        outputs.len(),
        1,
        "input basis state {input_basis_state} has outputs {outputs:?}"
    );
    outputs.into_iter().next().unwrap()
}

fn assert_exact_mcx_semantics(num_controls: usize) {
    let controls = qubits(0, num_controls);
    let target = Qubit::new(num_controls as u32);
    let num_qubits = num_controls + 1;
    let operations = decompose_mcx_no_aux(&controls, target).unwrap();

    for input_basis_state in 0..1 << num_qubits {
        let (output_basis_state, amplitude) = single_nonzero_output(&operations, input_basis_state);
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
        assert!((amplitude - Complex64::new(1.0, 0.0)).norm() < EPSILON);
    }
}

fn assert_increment_semantics(
    data_width: usize,
    dirty_width: usize,
    add_operations: &[ValueOperation],
    subtract_operations: &[ValueOperation],
) {
    let total_width = data_width + dirty_width;
    let data_mask = (1usize << data_width) - 1;
    let dirty_mask = ((1usize << dirty_width) - 1) << data_width;

    for input_basis_state in 0..1 << total_width {
        let input_data = input_basis_state & data_mask;
        let input_dirty = input_basis_state & dirty_mask;

        let (add_output, add_amplitude) = single_nonzero_output(add_operations, input_basis_state);
        assert_eq!(add_output & data_mask, (input_data + 1) & data_mask);
        assert_eq!(add_output & dirty_mask, input_dirty);
        assert!((add_amplitude.norm() - 1.0).abs() < EPSILON);

        let (subtract_output, subtract_amplitude) =
            single_nonzero_output(subtract_operations, input_basis_state);
        assert_eq!(
            subtract_output & data_mask,
            input_data.wrapping_sub(1) & data_mask
        );
        assert_eq!(subtract_output & dirty_mask, input_dirty);
        assert!((subtract_amplitude.norm() - 1.0).abs() < EPSILON);

        let mut round_trip = add_operations.to_vec();
        round_trip.extend_from_slice(subtract_operations);
        let (round_trip_output, round_trip_amplitude) =
            single_nonzero_output(&round_trip, input_basis_state);
        assert_eq!(round_trip_output, input_basis_state);
        assert!((round_trip_amplitude.norm() - 1.0).abs() < EPSILON);
    }
}

fn assert_only_uses_input_qubits(controls: &[Qubit], target: Qubit, operations: &[ValueOperation]) {
    let allowed: HashSet<_> = controls.iter().copied().chain([target]).collect();
    for operation in operations {
        assert!(matches!(
            operation.instruction,
            ValueInstruction::Instruction(Instruction::Standard(_))
        ));
        assert!(operation.qubits.iter().all(|qubit| allowed.contains(qubit)));
    }
}

fn assert_selected_exact_mcx_semantics(num_controls: usize) {
    let controls = qubits(0, num_controls);
    let target = Qubit::new(num_controls as u32);
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << target.index();
    let operations = decompose_mcx_no_aux(&controls, target).unwrap();
    let mut inputs = selected_basis_states(num_controls + 1);
    inputs.extend([control_mask, control_mask | target_mask]);
    inputs.sort_unstable();
    inputs.dedup();

    for input_basis_state in inputs {
        let expected_output = if input_basis_state & control_mask == control_mask {
            input_basis_state ^ target_mask
        } else {
            input_basis_state
        };
        let (output_basis_state, amplitude) = single_nonzero_output(&operations, input_basis_state);

        assert_eq!(output_basis_state, expected_output);
        assert!((amplitude - Complex64::new(1.0, 0.0)).norm() < EPSILON);
    }
}

fn assert_selected_increment_semantics(
    data_width: usize,
    dirty_width: usize,
    add_operations: &[ValueOperation],
    subtract_operations: &[ValueOperation],
) {
    let total_width = data_width + dirty_width;
    let data_mask = (1_usize << data_width) - 1;
    let dirty_mask = ((1_usize << dirty_width) - 1) << data_width;
    let mut inputs = selected_basis_states(total_width);
    inputs.extend([
        data_mask,
        dirty_mask,
        data_mask | dirty_mask,
        (data_mask >> 1) | dirty_mask,
    ]);
    inputs.sort_unstable();
    inputs.dedup();

    for input_basis_state in inputs {
        let input_data = input_basis_state & data_mask;
        let input_dirty = input_basis_state & dirty_mask;
        let (add_output, add_amplitude) = single_nonzero_output(add_operations, input_basis_state);
        assert_eq!(add_output & data_mask, (input_data + 1) & data_mask);
        assert_eq!(add_output & dirty_mask, input_dirty);
        assert!((add_amplitude.norm() - 1.0).abs() < EPSILON);

        let (subtract_output, subtract_amplitude) =
            single_nonzero_output(subtract_operations, input_basis_state);
        assert_eq!(
            subtract_output & data_mask,
            input_data.wrapping_sub(1) & data_mask
        );
        assert_eq!(subtract_output & dirty_mask, input_dirty);
        assert!((subtract_amplitude.norm() - 1.0).abs() < EPSILON);

        let mut round_trip = add_operations.to_vec();
        round_trip.extend_from_slice(subtract_operations);
        let (round_trip_output, round_trip_amplitude) =
            single_nonzero_output(&round_trip, input_basis_state);
        assert_eq!(round_trip_output, input_basis_state);
        assert!((round_trip_amplitude.norm() - 1.0).abs() < EPSILON);
    }
}

fn assert_selected_relative_mcx_semantics(num_controls: usize) {
    let controls = qubits(0, num_controls);
    let target = Qubit::new(num_controls as u32);
    let dirty_workspace = qubits(num_controls + 1, num_controls - 2);
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << target.index();
    let dirty_mask = ((1_usize << dirty_workspace.len()) - 1) << (num_controls + 1);
    let mut inputs = selected_basis_states(num_controls + 1 + dirty_workspace.len());
    inputs.extend([
        control_mask,
        control_mask | target_mask,
        control_mask | dirty_mask,
        control_mask | target_mask | dirty_mask,
    ]);
    inputs.sort_unstable();
    inputs.dedup();

    let mut operations = vec![];
    emit_relative_mcx_with_internal_dirty_qubits(
        &mut operations,
        &controls,
        target,
        &dirty_workspace,
    )
    .unwrap();

    for input_basis_state in inputs {
        let expected_output = if input_basis_state & control_mask == control_mask {
            input_basis_state ^ target_mask
        } else {
            input_basis_state
        };
        let (output_basis_state, amplitude) = single_nonzero_output(&operations, input_basis_state);

        assert_eq!(output_basis_state, expected_output);
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);
    }
}

#[test]
fn hp24_trivial_cases_match_trivial_decomposition() {
    let target = Qubit::new(2);

    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let operations = decompose_mcx_no_aux(controls, target).unwrap();
        let expected = decompose_mcx_small(controls, target).unwrap();
        assert_value_operations_equal(&operations, &expected);
    }
}

#[test]
fn hp24_rejects_duplicate_controls() {
    let duplicate = Qubit::new(0);
    assert_duplicate_error(
        &[duplicate, Qubit::new(1), duplicate],
        Qubit::new(3),
        duplicate,
    );
}

#[test]
fn hp24_rejects_target_matching_control() {
    let duplicate = Qubit::new(1);
    assert_duplicate_error(
        &[Qubit::new(0), duplicate, Qubit::new(2)],
        duplicate,
        duplicate,
    );
}

#[test]
fn hp24_matches_exact_mcx_semantics() {
    for num_controls in 3..=6 {
        assert_exact_mcx_semantics(num_controls);
    }
}

#[test]
fn hp24_selects_expected_figure_paths() {
    assert_eq!(select_hp24_path(22), Hp24Path::TwoDirty);
    assert_eq!(select_hp24_path(23), Hp24Path::TwoDirty);
    assert_eq!(select_hp24_path(24), Hp24Path::OneDirty);
    assert_eq!(select_hp24_path(25), Hp24Path::TwoDirty);
}

#[test]
fn hp24_path_boundaries_emit_linear_input_only_operations() {
    for total_qubits in [22, 23, 24, 25] {
        let controls = qubits(0, total_qubits - 1);
        let target = Qubit::new((total_qubits - 1) as u32);
        let operations = decompose_mcx_no_aux(&controls, target).unwrap();

        assert!(!operations.is_empty());
        assert!(
            operations.len()
                <= LINEAR_OPERATION_BOUND_FACTOR * controls.len() + LINEAR_OPERATION_BOUND_OFFSET
        );
        assert_only_uses_input_qubits(&controls, target, &operations);
    }
}

#[test]
fn hp24_operation_count_remains_linear() {
    for num_controls in [8, 16, 32] {
        let controls = qubits(0, num_controls);
        let target = Qubit::new(num_controls as u32);
        let operations = decompose_mcx_no_aux(&controls, target).unwrap();

        assert!(
            operations.len()
                <= LINEAR_OPERATION_BOUND_FACTOR * num_controls + LINEAR_OPERATION_BOUND_OFFSET
        );
        assert_only_uses_input_qubits(&controls, target, &operations);
    }
}

#[test]
fn dirty_increment_adds_subtracts_and_restores_workspace() {
    for data_width in 1..=6 {
        let data = qubits(0, data_width);
        let dirty_workspace = qubits(data_width, data_width);
        let mut add_operations = vec![];
        emit_increment_n_dirty(
            &mut add_operations,
            &data,
            &dirty_workspace,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_n_dirty(
            &mut subtract_operations,
            &data,
            &dirty_workspace,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_increment_semantics(
            data_width,
            data_width,
            &add_operations,
            &subtract_operations,
        );
    }
}

#[test]
fn one_dirty_increment_adds_subtracts_and_restores_workspace() {
    for data_width in [1, 3, 5] {
        let data = qubits(0, data_width);
        let dirty = Qubit::new(data_width as u32);
        let mut add_operations = vec![];
        emit_increment_one_dirty(
            &mut add_operations,
            &data,
            dirty,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_one_dirty(
            &mut subtract_operations,
            &data,
            dirty,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_increment_semantics(data_width, 1, &add_operations, &subtract_operations);
    }
}

#[test]
fn two_dirty_increment_adds_subtracts_and_restores_workspace() {
    for data_width in 1..=6 {
        let data = qubits(0, data_width);
        let first_dirty = Qubit::new(data_width as u32);
        let second_dirty = Qubit::new((data_width + 1) as u32);
        let mut add_operations = vec![];
        emit_increment_two_dirty(
            &mut add_operations,
            &data,
            first_dirty,
            second_dirty,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_two_dirty(
            &mut subtract_operations,
            &data,
            first_dirty,
            second_dirty,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_increment_semantics(data_width, 2, &add_operations, &subtract_operations);
    }
}

#[test]
fn relative_mcx_matches_bit_flip_behavior_up_to_phase() {
    for num_controls in 1..=6 {
        let controls = qubits(0, num_controls);
        let target = Qubit::new(num_controls as u32);
        let dirty_workspace = qubits(num_controls + 1, num_controls);
        let mut operations = vec![];
        emit_relative_mcx_with_internal_dirty_qubits(
            &mut operations,
            &controls,
            target,
            &dirty_workspace,
        )
        .unwrap();

        for input_basis_state in 0..1 << (2 * num_controls + 1) {
            let (output_basis_state, amplitude) =
                single_nonzero_output(&operations, input_basis_state);
            let all_controls_set = controls
                .iter()
                .all(|control| input_basis_state & (1 << control.index()) != 0);
            let expected_output = if all_controls_set {
                input_basis_state ^ (1 << target.index())
            } else {
                input_basis_state
            };

            assert_eq!(output_basis_state, expected_output);
            assert!((amplitude.norm() - 1.0).abs() < EPSILON);
        }
    }
}

#[test]
fn large_relative_mcx_explicitly_uses_internal_dirty_qubits_only() {
    let controls = qubits(0, 11);
    let target = Qubit::new(11);
    let dirty_workspace = qubits(12, 9);
    let mut operations = vec![];
    emit_relative_mcx_with_internal_dirty_qubits(
        &mut operations,
        &controls,
        target,
        &dirty_workspace,
    )
    .unwrap();

    let allowed: HashSet<_> = controls
        .iter()
        .copied()
        .chain([target])
        .chain(dirty_workspace.iter().copied())
        .collect();
    assert!(!operations.is_empty());
    assert!(
        operations
            .iter()
            .flat_map(|operation| &operation.qubits)
            .all(|qubit| allowed.contains(qubit))
    );
}

#[test]
fn hp24_one_dirty_path_matches_selected_basis_semantics_at_total_width_24() {
    assert_selected_exact_mcx_semantics(23);
}

#[test]
fn hp24_two_dirty_path_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [7, 8, 16, 21, 22, 24] {
        assert_selected_exact_mcx_semantics(num_controls);
    }
}

#[test]
fn large_dirty_increment_matches_selected_basis_semantics() {
    for data_width in [10, 11, 12, 16] {
        let data = qubits(0, data_width);
        let dirty_workspace = qubits(data_width, data_width);
        let mut add_operations = vec![];
        emit_increment_n_dirty(
            &mut add_operations,
            &data,
            &dirty_workspace,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_n_dirty(
            &mut subtract_operations,
            &data,
            &dirty_workspace,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_selected_increment_semantics(
            data_width,
            data_width,
            &add_operations,
            &subtract_operations,
        );
    }
}

#[test]
fn large_one_dirty_increment_matches_selected_basis_semantics() {
    for data_width in [7, 9, 11, 21] {
        let data = qubits(0, data_width);
        let dirty = Qubit::new(data_width as u32);
        let mut add_operations = vec![];
        emit_increment_one_dirty(
            &mut add_operations,
            &data,
            dirty,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_one_dirty(
            &mut subtract_operations,
            &data,
            dirty,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_selected_increment_semantics(data_width, 1, &add_operations, &subtract_operations);
    }
}

#[test]
fn one_dirty_increment_rejects_zero_and_even_data_widths() {
    for data_width in [0, 2, 4] {
        let data = qubits(0, data_width);
        let mut operations = vec![];
        let error = emit_increment_one_dirty(
            &mut operations,
            &data,
            Qubit::new(data_width as u32),
            IncrementDirection::AddOne,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            CompilerError::TransformFailed {
                name: "decompose.mcx",
                ref reason,
            } if reason
                == &format!(
                    "HP24 one-dirty increment requires a positive odd data width, got {data_width}"
                )
        ));
        assert!(operations.is_empty());
    }
}

#[test]
fn large_two_dirty_increment_matches_selected_basis_semantics() {
    for data_width in [7, 8, 11, 12, 22] {
        let data = qubits(0, data_width);
        let first_dirty = Qubit::new(data_width as u32);
        let second_dirty = Qubit::new((data_width + 1) as u32);
        let mut add_operations = vec![];
        emit_increment_two_dirty(
            &mut add_operations,
            &data,
            first_dirty,
            second_dirty,
            IncrementDirection::AddOne,
        )
        .unwrap();
        let mut subtract_operations = vec![];
        emit_increment_two_dirty(
            &mut subtract_operations,
            &data,
            first_dirty,
            second_dirty,
            IncrementDirection::SubtractOne,
        )
        .unwrap();

        assert_selected_increment_semantics(data_width, 2, &add_operations, &subtract_operations);
    }
}

#[test]
fn large_relative_mcx_matches_selected_basis_semantics() {
    assert_selected_relative_mcx_semantics(11);
}
