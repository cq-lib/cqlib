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

use super::pauli_rotation::{decompose_pauli_rotation_n_clean, decompose_pauli_rotation_no_aux};
use super::rzz::{decompose_mc_rzz_n_clean, decompose_mc_rzz_no_aux};
use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_equal_up_to_global_phase, assert_standard_operation,
    assert_value_operations_equal, circuit_from_value_operations, mc_gate_matrix,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, ParameterValue, Qubit, StandardGate, circuit_to_matrix, operation::ValueOperation,
};
use crate::compile::error::CompilerError;
use std::f64::consts::PI;

fn assert_is_rx(operation: &ValueOperation, qubit: Qubit, angle: f64) {
    assert!(matches!(
        operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::RX))
    ));
    assert_eq!(operation.qubits.as_slice(), &[qubit]);
    assert!(matches!(
        operation.params.as_slice(),
        [ParameterValue::Fixed(value)] if value.to_bits() == angle.to_bits()
    ));
}

fn assert_crz(operation: &ValueOperation, control: Qubit, target: Qubit, theta: f64) {
    assert!(matches!(
        operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::CRZ))
    ));
    assert_eq!(operation.qubits.as_slice(), &[control, target]);
    assert!(matches!(
        operation.params.as_slice(),
        [ParameterValue::Fixed(value)] if value.to_bits() == theta.to_bits()
    ));
}

// ── zero-controls fast path ──

#[test]
fn zero_controls_emits_standard_gate_for_all_four_rotations() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let theta = ParameterValue::Fixed(0.731);

    for rotation in [
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::RZX,
    ] {
        let operations =
            decompose_pauli_rotation_no_aux(rotation, &theta, &[], first, second).unwrap();

        assert_eq!(operations.len(), 1);
        assert!(matches!(
            operations[0].instruction,
            ValueInstruction::Instruction(Instruction::Standard(gate)) if gate == rotation
        ));
        assert_eq!(operations[0].qubits.as_slice(), &[first, second]);
        assert!(matches!(
            operations[0].params.as_slice(),
            [ParameterValue::Fixed(value)] if value.to_bits() == 0.731_f64.to_bits()
        ));
    }
}

// ── basis-change structure for one control ──

#[test]
fn rxx_with_one_control_has_h_conjugations_around_rzz() {
    let control = Qubit::new(0);
    let first = Qubit::new(1);
    let second = Qubit::new(2);
    let theta = ParameterValue::Fixed(0.8);

    let operations =
        decompose_pauli_rotation_no_aux(StandardGate::RXX, &theta, &[control], first, second)
            .unwrap();

    // H(first), H(second); CX(first,second); CRZ(control,second); CX(first,second); H(first), H(second)
    assert_eq!(operations.len(), 7);

    assert_standard_operation(&operations[0], StandardGate::H, &[first]);
    assert_standard_operation(&operations[1], StandardGate::H, &[second]);
    assert_standard_operation(&operations[2], StandardGate::CX, &[first, second]);
    assert_crz(&operations[3], control, second, 0.8);
    assert_standard_operation(&operations[4], StandardGate::CX, &[first, second]);
    assert_standard_operation(&operations[5], StandardGate::H, &[first]);
    assert_standard_operation(&operations[6], StandardGate::H, &[second]);
}

#[test]
fn ryy_with_one_control_has_rx_conjugations_around_rzz() {
    let control = Qubit::new(0);
    let first = Qubit::new(1);
    let second = Qubit::new(2);
    let theta = ParameterValue::Fixed(0.8);

    let operations =
        decompose_pauli_rotation_no_aux(StandardGate::RYY, &theta, &[control], first, second)
            .unwrap();

    // RX(pi/2), RX(pi/2); CX; CRZ; CX; RX(-pi/2), RX(-pi/2)
    assert_eq!(operations.len(), 7);

    assert_is_rx(&operations[0], first, PI / 2.0);
    assert_is_rx(&operations[1], second, PI / 2.0);
    assert_standard_operation(&operations[2], StandardGate::CX, &[first, second]);
    assert_crz(&operations[3], control, second, 0.8);
    assert_standard_operation(&operations[4], StandardGate::CX, &[first, second]);
    assert_is_rx(&operations[5], first, -PI / 2.0);
    assert_is_rx(&operations[6], second, -PI / 2.0);
}

#[test]
fn rzx_with_one_control_has_h_on_second_around_rzz() {
    let control = Qubit::new(0);
    let first = Qubit::new(1);
    let second = Qubit::new(2);
    let theta = ParameterValue::Fixed(0.8);

    let operations =
        decompose_pauli_rotation_no_aux(StandardGate::RZX, &theta, &[control], first, second)
            .unwrap();

    // H(second); CX(first,second); CRZ(control,second); CX(first,second); H(second)
    assert_eq!(operations.len(), 5);

    assert_standard_operation(&operations[0], StandardGate::H, &[second]);
    assert_standard_operation(&operations[1], StandardGate::CX, &[first, second]);
    assert_crz(&operations[2], control, second, 0.8);
    assert_standard_operation(&operations[3], StandardGate::CX, &[first, second]);
    assert_standard_operation(&operations[4], StandardGate::H, &[second]);
}

// ── RZZ delegation ──

#[test]
fn rzz_delegates_directly_to_mc_rzz() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let theta = ParameterValue::Fixed(0.731);

    assert_value_operations_equal(
        &decompose_pauli_rotation_no_aux(StandardGate::RZZ, &theta, &controls, first, second)
            .unwrap(),
        &decompose_mc_rzz_no_aux(&theta, &controls, first, second).unwrap(),
    );

    let clean_ancillas = [Qubit::new(4)];
    assert_value_operations_equal(
        &decompose_pauli_rotation_n_clean(
            StandardGate::RZZ,
            &theta,
            &controls,
            first,
            second,
            &clean_ancillas,
        )
        .unwrap(),
        &decompose_mc_rzz_n_clean(&theta, &controls, first, second, &clean_ancillas).unwrap(),
    );
}

// ── matrix semantics ──

#[test]
fn pauli_rotation_decompositions_match_mcgate_semantics() {
    let rotations = [
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::RZX,
    ];
    for num_controls in 1..=3 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let first = Qubit::new(num_controls as u32);
        let second = Qubit::new(num_controls as u32 + 1);
        let total = (num_controls + 2) as usize;
        let theta = ParameterValue::Fixed(0.731);

        for rotation in rotations {
            let operations =
                decompose_pauli_rotation_no_aux(rotation, &theta, &controls, first, second)
                    .unwrap();
            let actual =
                circuit_to_matrix(&circuit_from_value_operations(total, operations), None).unwrap();
            let mut qubits = controls.clone();
            qubits.extend([first, second]);
            let expected = mc_gate_matrix(
                total,
                num_controls as u8,
                rotation,
                qubits,
                [ParameterValue::Fixed(0.731)],
            );

            assert_selected_matrix_columns_equal_up_to_global_phase(
                &actual,
                &expected,
                0..expected.ncols(),
                EPSILON,
            );
        }
    }
}

#[test]
fn clean_pauli_rotation_preserves_ancilla_subspace() {
    let rotations = [StandardGate::RXX, StandardGate::RYY, StandardGate::RZX];
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let first = Qubit::new(3);
    let second = Qubit::new(4);
    let clean_ancillas = [Qubit::new(5), Qubit::new(6)];
    let theta = ParameterValue::Fixed(0.731);

    for rotation in rotations {
        let operations = decompose_pauli_rotation_n_clean(
            rotation,
            &theta,
            &controls,
            first,
            second,
            &clean_ancillas,
        )
        .unwrap();
        let actual =
            circuit_to_matrix(&circuit_from_value_operations(7, operations), None).unwrap();
        let mut qubits = controls.to_vec();
        qubits.extend([first, second]);
        let expected = mc_gate_matrix(7, 3, rotation, qubits, [ParameterValue::Fixed(0.731)]);
        let clean_mask = clean_ancillas
            .iter()
            .fold(0_usize, |mask, qubit| mask | (1 << qubit.index()));
        let clean_columns = (0..expected.ncols()).filter(|state| state & clean_mask == 0);

        assert_selected_matrix_columns_equal_up_to_global_phase(
            &actual,
            &expected,
            clean_columns,
            EPSILON,
        );
    }
}

// ── error paths ──

#[test]
fn unsupported_rotation_is_rejected_with_clear_message() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let error = decompose_pauli_rotation_no_aux(
        StandardGate::H,
        &ParameterValue::Fixed(0.731),
        &[],
        first,
        second,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.pauli_rotation",
            ref reason,
        } if reason == "multi-controlled Pauli rotation decomposition supports only RXX, RYY, RZZ, or RZX, got H"
    ));
}

#[test]
fn underlying_rzz_errors_are_propagated() {
    // RXX with overlapping control/interaction qubit: the error from rzz
    // (detecting the overlap) is propagated through pauli_rotation.
    let control = Qubit::new(0);
    let first = Qubit::new(0); // overlaps with control
    let second = Qubit::new(1);
    let error = decompose_pauli_rotation_no_aux(
        StandardGate::RXX,
        &ParameterValue::Fixed(0.731),
        &[control],
        first,
        second,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.rzz",
            ref reason,
        } if reason == "RZZ interaction qubits must not appear in controls; duplicate Q0"
    ));
}
