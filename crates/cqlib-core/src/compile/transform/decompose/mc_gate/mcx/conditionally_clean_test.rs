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
    conditionally_clean::{build_linear_depth_ladder, build_log_depth_ladder},
    decompose_mcx_1_clean_kg24, decompose_mcx_1_dirty, decompose_mcx_2_clean,
    decompose_mcx_2_dirty, decompose_mcx_small,
    relative_phase::emit_relative_phase_toffoli,
    test_utils::{EPSILON, selected_basis_states},
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Qubit, StandardGate, circuit_to_matrix, operation::ValueOperation,
};
use crate::compile::error::CompilerError;
use crate::compile::transform::decompose::mc_gate::test_utils::{
    assert_statevectors_equal_up_to_global_phase, assert_value_operations_equal,
    assert_value_operations_only_use_qubits, circuit_from_value_operations,
    single_nonzero_matrix_output, statevector_after_value_operations,
};
use crate::qis::Statevector;
use num_complex::Complex64;
use std::collections::{BTreeMap, HashMap};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

type OneAncillaDecomposer =
    fn(&[Qubit], Qubit, Qubit) -> Result<Vec<ValueOperation>, CompilerError>;
type TwoAncillaDecomposer =
    fn(&[Qubit], Qubit, [Qubit; 2]) -> Result<Vec<ValueOperation>, CompilerError>;

fn controls(num_controls: usize) -> Vec<Qubit> {
    (0..num_controls)
        .map(|index| Qubit::new(index as u32))
        .collect()
}

fn add_sparse_amplitude(
    state: &mut BTreeMap<usize, Complex64>,
    basis_state: usize,
    amplitude: Complex64,
) {
    let sum = state.entry(basis_state).or_default();
    *sum += amplitude;
    if sum.norm() < EPSILON {
        state.remove(&basis_state);
    }
}

fn apply_sparse_phase(
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
        add_sparse_amplitude(next, basis_state, output_amplitude);
    }
}

fn apply_sparse_operations_to_basis_state(
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
                    add_sparse_amplitude(&mut next, basis_state ^ (1 << target), amplitude);
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
                    add_sparse_amplitude(&mut next, cleared, amplitude * FRAC_1_SQRT_2);
                    add_sparse_amplitude(
                        &mut next,
                        cleared | (1 << target),
                        amplitude * sign * FRAC_1_SQRT_2,
                    );
                }
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::T)) => {
                apply_sparse_phase(&mut next, state, operation.qubits[0], PI / 4.0);
            }
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::TDG)) => {
                apply_sparse_phase(&mut next, state, operation.qubits[0], -PI / 4.0);
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
                    add_sparse_amplitude(&mut next, output, amplitude);
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
                    add_sparse_amplitude(&mut next, output, amplitude);
                }
            }
            ref instruction => panic!("unexpected instruction: {instruction:?}"),
        }
        state = next;
    }

    state
}

fn assert_selected_exact_mcx_semantics(
    num_controls: usize,
    ancillas: &[Qubit],
    clean_ancillas: bool,
    operations: &[ValueOperation],
) {
    let control_mask = (1_usize << num_controls) - 1;
    let target_mask = 1_usize << num_controls;
    let mut data_inputs = selected_basis_states(num_controls + 1);
    data_inputs.extend([control_mask, control_mask | target_mask]);
    data_inputs.sort_unstable();
    data_inputs.dedup();
    let ancilla_values = if clean_ancillas {
        0..1
    } else {
        0..1 << ancillas.len()
    };
    let mut global_phase = None;

    for ancilla_value in ancilla_values {
        for data_input in data_inputs.iter().copied() {
            let input_basis_state = data_input | (ancilla_value << (num_controls + 1));
            let expected_output = if input_basis_state & control_mask == control_mask {
                input_basis_state ^ target_mask
            } else {
                input_basis_state
            };
            let outputs = apply_sparse_operations_to_basis_state(operations, input_basis_state);
            assert_eq!(
                outputs.len(),
                1,
                "input basis state {input_basis_state} has outputs {outputs:?}"
            );
            let (output_basis_state, amplitude) = outputs.into_iter().next().unwrap();

            assert_eq!(output_basis_state, expected_output);
            assert!((amplitude.norm() - 1.0).abs() < EPSILON);
            let expected_phase = global_phase.get_or_insert(amplitude);
            assert!(
                (amplitude - *expected_phase).norm() < EPSILON,
                "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
            );
        }
    }
}

fn assert_duplicate_error(error: CompilerError, duplicate: Qubit) {
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

fn assert_one_ancilla_trivial_cases(decompose: OneAncillaDecomposer) {
    let target = Qubit::new(2);
    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let operations = decompose(controls, target, target).unwrap();
        let expected = decompose_mcx_small(controls, target).unwrap();
        assert_value_operations_equal(&operations, &expected);
        assert!(operations.iter().all(|operation| {
            operation
                .qubits
                .iter()
                .filter(|qubit| **qubit == target)
                .count()
                == 1
        }));
    }
}

fn assert_two_ancilla_trivial_cases(decompose: TwoAncillaDecomposer) {
    let target = Qubit::new(2);
    for controls in [
        &[][..],
        &[Qubit::new(0)][..],
        &[Qubit::new(0), Qubit::new(1)][..],
    ] {
        let operations = decompose(controls, target, [target, target]).unwrap();
        let expected = decompose_mcx_small(controls, target).unwrap();
        assert_value_operations_equal(&operations, &expected);
        assert!(operations.iter().all(|operation| {
            operation
                .qubits
                .iter()
                .filter(|qubit| **qubit == target)
                .count()
                == 1
        }));
    }
}

fn assert_one_ancilla_duplicate_cases(decompose: OneAncillaDecomposer) {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    assert_duplicate_error(
        decompose(
            &[Qubit::new(0), Qubit::new(1), Qubit::new(0)],
            Qubit::new(3),
            Qubit::new(4),
        )
        .unwrap_err(),
        Qubit::new(0),
    );
    assert_duplicate_error(
        decompose(&controls, controls[1], Qubit::new(4)).unwrap_err(),
        controls[1],
    );
    assert_duplicate_error(
        decompose(&controls, Qubit::new(3), controls[2]).unwrap_err(),
        controls[2],
    );
    assert_duplicate_error(
        decompose(&controls, Qubit::new(3), Qubit::new(3)).unwrap_err(),
        Qubit::new(3),
    );
}

fn assert_two_ancilla_duplicate_cases(decompose: TwoAncillaDecomposer) {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    assert_duplicate_error(
        decompose(
            &[Qubit::new(0), Qubit::new(1), Qubit::new(0)],
            Qubit::new(3),
            [Qubit::new(4), Qubit::new(5)],
        )
        .unwrap_err(),
        Qubit::new(0),
    );
    assert_duplicate_error(
        decompose(&controls, controls[1], [Qubit::new(4), Qubit::new(5)]).unwrap_err(),
        controls[1],
    );
    assert_duplicate_error(
        decompose(&controls, Qubit::new(3), [controls[2], Qubit::new(5)]).unwrap_err(),
        controls[2],
    );
    assert_duplicate_error(
        decompose(&controls, Qubit::new(3), [Qubit::new(3), Qubit::new(5)]).unwrap_err(),
        Qubit::new(3),
    );
    assert_duplicate_error(
        decompose(&controls, Qubit::new(3), [Qubit::new(4), Qubit::new(4)]).unwrap_err(),
        Qubit::new(4),
    );
}

fn assert_exact_mcx_semantics(
    num_controls: usize,
    ancillas: &[Qubit],
    clean_ancillas: bool,
    operations: Vec<ValueOperation>,
) {
    let controls = controls(num_controls);
    let target = Qubit::new(num_controls as u32);
    let num_qubits = num_controls + ancillas.len() + 1;
    let matrix =
        circuit_to_matrix(&circuit_from_value_operations(num_qubits, operations), None).unwrap();
    let ancilla_mask = ancillas
        .iter()
        .fold(0, |mask, ancilla| mask | (1 << ancilla.index()));
    let mut global_phase = None;

    for input_basis_state in 0..1 << num_qubits {
        if clean_ancillas && input_basis_state & ancilla_mask != 0 {
            continue;
        }

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
        assert_eq!(
            output_basis_state & ancilla_mask,
            input_basis_state & ancilla_mask
        );
        assert!((amplitude.norm() - 1.0).abs() < EPSILON);

        let expected_phase = global_phase.get_or_insert(amplitude);
        assert!(
            (amplitude - *expected_phase).norm() < EPSILON,
            "input basis state {input_basis_state} has relative phase {amplitude}, expected global phase {expected_phase}"
        );
    }
}

fn expected_exact_mcx_state(initial_state: &Statevector, num_controls: usize) -> Statevector {
    let control_mask = (1 << num_controls) - 1;
    let target_mask = 1 << num_controls;
    let mut expected = vec![Complex64::new(0.0, 0.0); initial_state.data().len()];

    for (basis_state, amplitude) in initial_state.data().iter().copied().enumerate() {
        let output = if basis_state & control_mask == control_mask {
            basis_state ^ target_mask
        } else {
            basis_state
        };
        expected[output] = amplitude;
    }

    Statevector::from_state(initial_state.num_qubits, expected).unwrap()
}

fn assert_dirty_superposition_restored(
    num_controls: usize,
    num_ancillas: usize,
    operations: Vec<ValueOperation>,
) {
    let num_qubits = num_controls + num_ancillas + 1;
    let control_mask = (1 << num_controls) - 1;
    let mut amplitudes = vec![Complex64::new(0.0, 0.0); 1 << num_qubits];
    let amplitude = if num_ancillas == 1 {
        FRAC_1_SQRT_2
    } else {
        0.5
    };

    for ancilla_value in 0..1 << num_ancillas {
        amplitudes[control_mask | (ancilla_value << (num_controls + 1))] =
            Complex64::new(amplitude, 0.0);
    }

    let initial_state = Statevector::from_state(num_qubits, amplitudes).unwrap();
    let expected = expected_exact_mcx_state(&initial_state, num_controls);
    let actual = statevector_after_value_operations(&initial_state, &operations);
    assert_statevectors_equal_up_to_global_phase(&actual, &expected, EPSILON);
}

fn toffoli_level_gate_count(operations: &[ValueOperation]) -> usize {
    let ccx_count = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(StandardGate::CCX))
            )
        })
        .count();
    let h_count = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(StandardGate::H))
            )
        })
        .count();
    assert_eq!(h_count % 2, 0);
    ccx_count + h_count / 2
}

fn asap_depth(operations: &[ValueOperation]) -> usize {
    let mut qubit_depths = HashMap::new();
    let mut circuit_depth = 0;

    for operation in operations {
        let operation_depth = operation
            .qubits
            .iter()
            .filter_map(|qubit| qubit_depths.get(qubit))
            .copied()
            .max()
            .unwrap_or(0)
            + 1;
        for qubit in &operation.qubits {
            qubit_depths.insert(*qubit, operation_depth);
        }
        circuit_depth = circuit_depth.max(operation_depth);
    }

    circuit_depth
}

fn append_expected_workspace_step(
    operations: &mut Vec<ValueOperation>,
    first_control: Qubit,
    second_control: Qubit,
    target: Qubit,
) {
    emit_relative_phase_toffoli(operations, first_control, second_control, target).unwrap();
    operations.push(ValueOperation::from_standard(StandardGate::X, [target], []));
}

#[test]
fn conditionally_clean_trivial_cases_ignore_unused_ancillas() {
    assert_one_ancilla_trivial_cases(decompose_mcx_1_clean_kg24);
    assert_one_ancilla_trivial_cases(decompose_mcx_1_dirty);
    assert_two_ancilla_trivial_cases(decompose_mcx_2_clean);
    assert_two_ancilla_trivial_cases(decompose_mcx_2_dirty);
}

#[test]
fn conditionally_clean_nontrivial_cases_reject_duplicate_qubits() {
    assert_one_ancilla_duplicate_cases(decompose_mcx_1_clean_kg24);
    assert_one_ancilla_duplicate_cases(decompose_mcx_1_dirty);
    assert_two_ancilla_duplicate_cases(decompose_mcx_2_clean);
    assert_two_ancilla_duplicate_cases(decompose_mcx_2_dirty);
}

#[test]
fn one_conditionally_clean_ancilla_has_exact_mcx_semantics() {
    for num_controls in 3..=6 {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancilla = Qubit::new(num_controls as u32 + 1);
        let operations = decompose_mcx_1_clean_kg24(&controls, target, ancilla).unwrap();
        assert_exact_mcx_semantics(num_controls, &[ancilla], true, operations);
    }
}

#[test]
fn one_conditionally_dirty_ancilla_has_exact_mcx_semantics() {
    for num_controls in 3..=5 {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancilla = Qubit::new(num_controls as u32 + 1);
        let operations = decompose_mcx_1_dirty(&controls, target, ancilla).unwrap();
        assert_exact_mcx_semantics(num_controls, &[ancilla], false, operations);
    }
}

#[test]
fn two_conditionally_clean_ancillas_have_exact_mcx_semantics() {
    for num_controls in 3..=7 {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancillas = [
            Qubit::new(num_controls as u32 + 1),
            Qubit::new(num_controls as u32 + 2),
        ];
        let operations = decompose_mcx_2_clean(&controls, target, ancillas).unwrap();
        assert_exact_mcx_semantics(num_controls, &ancillas, true, operations);
    }
}

#[test]
fn two_conditionally_dirty_ancillas_have_exact_mcx_semantics() {
    for num_controls in 3..=6 {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancillas = [
            Qubit::new(num_controls as u32 + 1),
            Qubit::new(num_controls as u32 + 2),
        ];
        let operations = decompose_mcx_2_dirty(&controls, target, ancillas).unwrap();
        assert_exact_mcx_semantics(num_controls, &ancillas, false, operations);
    }
}

#[test]
fn dirty_ancilla_superpositions_are_restored() {
    let one_controls = controls(5);
    let target = Qubit::new(5);
    let one_ancilla = Qubit::new(6);
    let one_operations = decompose_mcx_1_dirty(&one_controls, target, one_ancilla).unwrap();
    assert_dirty_superposition_restored(5, 1, one_operations);

    let two_controls = controls(6);
    let target = Qubit::new(6);
    let two_ancillas = [Qubit::new(7), Qubit::new(8)];
    let two_operations = decompose_mcx_2_dirty(&two_controls, target, two_ancillas).unwrap();
    assert_dirty_superposition_restored(6, 2, two_operations);
}

#[test]
fn linear_ladder_uses_rccx_then_x_propagation_steps() {
    let controls = controls(7);
    let ancilla = Qubit::new(7);
    let ladder = build_linear_depth_ladder(ancilla, &controls).unwrap();
    let mut expected = vec![];
    append_expected_workspace_step(&mut expected, controls[2], controls[3], controls[1]);
    append_expected_workspace_step(&mut expected, controls[4], controls[5], controls[3]);
    append_expected_workspace_step(&mut expected, controls[6], controls[3], controls[2]);
    append_expected_workspace_step(&mut expected, controls[2], controls[1], controls[0]);

    assert_value_operations_equal(&ladder.operations, &expected);
    assert_eq!(ladder.final_control, controls[0]);
}

#[test]
fn logarithmic_ladder_reduces_frontier_and_skips_only_initial_rccx() {
    for (num_controls, expected_frontier) in [
        (3, vec![2]),
        (4, vec![1]),
        (5, vec![0]),
        (6, vec![0, 5]),
        (8, vec![0, 3]),
        (9, vec![0, 2]),
        (16, vec![0, 1, 5]),
    ] {
        let controls = controls(num_controls);
        let ancilla = Qubit::new(num_controls as u32);
        let ladder = build_log_depth_ladder(ancilla, &controls, false).unwrap();
        let skipped = build_log_depth_ladder(ancilla, &controls, true).unwrap();
        let expected_frontier: Vec<_> = expected_frontier
            .into_iter()
            .map(|index| controls[index])
            .collect();

        assert_eq!(ladder.remaining_controls, expected_frontier);
        assert_eq!(skipped.remaining_controls, expected_frontier);
        assert_eq!(
            toffoli_level_gate_count(&ladder.operations),
            toffoli_level_gate_count(&skipped.operations) + 1
        );
        assert_value_operations_equal(&ladder.operations[9..], &skipped.operations);
    }
}

#[test]
fn conditionally_clean_paths_have_expected_toffoli_level_counts() {
    for num_controls in 3..=32 {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let first_ancilla = Qubit::new(num_controls as u32 + 1);
        let second_ancilla = Qubit::new(num_controls as u32 + 2);

        let one_clean = decompose_mcx_1_clean_kg24(&controls, target, first_ancilla).unwrap();
        let one_dirty = decompose_mcx_1_dirty(&controls, target, first_ancilla).unwrap();
        let two_clean =
            decompose_mcx_2_clean(&controls, target, [first_ancilla, second_ancilla]).unwrap();
        let two_dirty =
            decompose_mcx_2_dirty(&controls, target, [first_ancilla, second_ancilla]).unwrap();

        assert_eq!(toffoli_level_gate_count(&one_clean), 2 * num_controls - 3);
        assert_eq!(toffoli_level_gate_count(&one_dirty), 4 * num_controls - 8);
        assert_eq!(toffoli_level_gate_count(&two_clean), 2 * num_controls - 3);
        assert_eq!(toffoli_level_gate_count(&two_dirty), 4 * num_controls - 8);
    }
}

#[test]
fn conditionally_clean_paths_only_use_supplied_qubits() {
    let controls = controls(12);
    let target = Qubit::new(12);
    let ancillas = [Qubit::new(13), Qubit::new(14)];
    let mut allowed_qubits = controls.clone();
    allowed_qubits.push(target);
    allowed_qubits.extend(ancillas);

    for operations in [
        decompose_mcx_1_clean_kg24(&controls, target, ancillas[0]).unwrap(),
        decompose_mcx_1_dirty(&controls, target, ancillas[0]).unwrap(),
        decompose_mcx_2_clean(&controls, target, ancillas).unwrap(),
        decompose_mcx_2_dirty(&controls, target, ancillas).unwrap(),
    ] {
        assert_value_operations_only_use_qubits(&operations, &allowed_qubits);
    }
}

#[test]
fn one_ancilla_depth_is_linear_and_two_ancilla_depth_is_logarithmic() {
    let mut previous_one_depth = 0;
    for num_controls in [8, 16, 32, 64, 128, 256, 512, 1024] {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancillas = [
            Qubit::new(num_controls as u32 + 1),
            Qubit::new(num_controls as u32 + 2),
        ];
        let one = decompose_mcx_1_clean_kg24(&controls, target, ancillas[0]).unwrap();
        let two = decompose_mcx_2_clean(&controls, target, ancillas).unwrap();
        let one_depth = asap_depth(&one);
        let two_depth = asap_depth(&two);
        let log_controls = num_controls.ilog2() as usize + 1;

        assert!(one_depth > previous_one_depth);
        assert!(one_depth <= 40 * num_controls);
        assert!(
            two_depth <= 80 * log_controls,
            "two-ancilla depth {two_depth} is too large for {num_controls} controls"
        );
        previous_one_depth = one_depth;
    }
}

#[test]
fn one_conditionally_clean_ancilla_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [7, 8, 9, 12] {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancilla = Qubit::new(num_controls as u32 + 1);
        let operations = decompose_mcx_1_clean_kg24(&controls, target, ancilla).unwrap();

        assert_selected_exact_mcx_semantics(num_controls, &[ancilla], true, &operations);
    }
}

#[test]
fn one_conditionally_dirty_ancilla_matches_selected_basis_semantics_at_large_widths() {
    for num_controls in [6, 7, 8, 9, 12] {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancilla = Qubit::new(num_controls as u32 + 1);
        let operations = decompose_mcx_1_dirty(&controls, target, ancilla).unwrap();

        assert_selected_exact_mcx_semantics(num_controls, &[ancilla], false, &operations);
    }
}

#[test]
fn two_conditionally_clean_ancillas_match_selected_basis_semantics_at_large_widths() {
    for num_controls in [8, 9, 12, 16] {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancillas = [
            Qubit::new(num_controls as u32 + 1),
            Qubit::new(num_controls as u32 + 2),
        ];
        let operations = decompose_mcx_2_clean(&controls, target, ancillas).unwrap();

        assert_selected_exact_mcx_semantics(num_controls, &ancillas, true, &operations);
    }
}

#[test]
fn two_conditionally_dirty_ancillas_match_selected_basis_semantics_at_large_widths() {
    for num_controls in [7, 8, 9, 12, 16] {
        let controls = controls(num_controls);
        let target = Qubit::new(num_controls as u32);
        let ancillas = [
            Qubit::new(num_controls as u32 + 1),
            Qubit::new(num_controls as u32 + 2),
        ];
        let operations = decompose_mcx_2_dirty(&controls, target, ancillas).unwrap();

        assert_selected_exact_mcx_semantics(num_controls, &ancillas, false, &operations);
    }
}

#[test]
fn dirty_ancilla_superpositions_are_restored_at_large_widths() {
    let one_controls = controls(12);
    let target = Qubit::new(12);
    let one_ancilla = Qubit::new(13);
    let one_operations = decompose_mcx_1_dirty(&one_controls, target, one_ancilla).unwrap();
    assert_dirty_superposition_restored(12, 1, one_operations);

    let two_controls = controls(12);
    let target = Qubit::new(12);
    let two_ancillas = [Qubit::new(13), Qubit::new(14)];
    let two_operations = decompose_mcx_2_dirty(&two_controls, target, two_ancillas).unwrap();
    assert_dirty_superposition_restored(12, 2, two_operations);
}

#[test]
fn linear_ladder_final_control_matches_width_boundaries() {
    for (num_controls, expected_final_control) in [(3, 2), (4, 1), (5, 0), (6, 0), (7, 0), (8, 0)] {
        let controls = controls(num_controls);
        let ancilla = Qubit::new(num_controls as u32);
        let ladder = build_linear_depth_ladder(ancilla, &controls).unwrap();

        assert_eq!(ladder.final_control, controls[expected_final_control]);
    }
}
