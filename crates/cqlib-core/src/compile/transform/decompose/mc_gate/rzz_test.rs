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

use super::rzz::{decompose_mc_rzz_n_clean, decompose_mc_rzz_no_aux};
use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_equal_up_to_global_phase, assert_standard_operation,
    assert_value_operations_equal, circuit_from_value_operations, mc_gate_matrix,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
};
use crate::compile::error::CompilerError;
#[test]
fn zero_controls_emits_bare_rzz() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let theta = ParameterValue::Fixed(0.731);

    let operations = decompose_mc_rzz_no_aux(&theta, &[], first, second).unwrap();

    assert_eq!(operations.len(), 1);
    assert!(matches!(
        operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZZ))
    ));
    assert_eq!(operations[0].qubits.as_slice(), &[first, second]);
    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Fixed(value)] if value.to_bits() == 0.731_f64.to_bits()
    ));
}

#[test]
fn one_control_structure_is_cx_crz_cx() {
    let control = Qubit::new(0);
    let first = Qubit::new(1);
    let second = Qubit::new(2);
    let theta = ParameterValue::Fixed(0.8);

    let operations = decompose_mc_rzz_no_aux(&theta, &[control], first, second).unwrap();

    assert_eq!(operations.len(), 3);
    assert_standard_operation(&operations[0], StandardGate::CX, &[first, second]);
    assert!(matches!(
        operations[1].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::CRZ))
    ));
    assert_eq!(operations[1].qubits.as_slice(), &[control, second]);
    assert!(matches!(
        operations[1].params.as_slice(),
        [ParameterValue::Fixed(value)] if value.to_bits() == 0.8_f64.to_bits()
    ));
    assert_standard_operation(&operations[2], StandardGate::CX, &[first, second]);
}

#[test]
fn no_ancilla_decompositions_match_mcgate_semantics() {
    for num_controls in 1..=4 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let first = Qubit::new(num_controls as u32);
        let second = Qubit::new(num_controls as u32 + 1);
        let total = (num_controls + 2) as usize;
        let theta = ParameterValue::Fixed(0.731);

        let operations = decompose_mc_rzz_no_aux(&theta, &controls, first, second).unwrap();
        let actual =
            circuit_to_matrix(&circuit_from_value_operations(total, operations), None).unwrap();
        let mut qubits = controls.clone();
        qubits.extend([first, second]);
        let expected = mc_gate_matrix(
            total,
            num_controls as u8,
            StandardGate::RZZ,
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

#[test]
fn clean_decomposition_preserves_ancilla_subspace() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let first = Qubit::new(3);
    let second = Qubit::new(4);
    let clean_ancillas = [Qubit::new(5), Qubit::new(6)];
    let theta = ParameterValue::Fixed(0.731);

    let operations =
        decompose_mc_rzz_n_clean(&theta, &controls, first, second, &clean_ancillas).unwrap();
    let actual = circuit_to_matrix(&circuit_from_value_operations(7, operations), None).unwrap();
    let mut qubits = controls.to_vec();
    qubits.extend([first, second]);
    let expected = mc_gate_matrix(
        7,
        3,
        StandardGate::RZZ,
        qubits,
        [ParameterValue::Fixed(0.731)],
    );
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

#[test]
fn symbolic_theta_passed_through_to_center_rotation() {
    use super::rotation::decompose_rotation_no_aux;

    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let theta = Parameter::symbol("theta");
    let param = ParameterValue::Param(theta.clone());

    let operations = decompose_mc_rzz_no_aux(&param, &controls, first, second).unwrap();

    // Build expected: CX + MCRZ + CX
    let mut expected = vec![];
    expected.push(operations[0].clone()); // CX(first, second)
    expected
        .extend(decompose_rotation_no_aux(StandardGate::RZ, &param, &controls, second).unwrap());
    expected.push(operations[operations.len() - 1].clone()); // CX(first, second)

    assert_value_operations_equal(&operations, &expected);
}

#[test]
fn duplicate_interaction_qubits_are_rejected() {
    let first = Qubit::new(0);
    let error =
        decompose_mc_rzz_no_aux(&ParameterValue::Fixed(0.731), &[], first, first).unwrap_err();
    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.rzz",
            ref reason,
        } if reason == "RZZ interaction qubits must be distinct; both are Q0"
    ));
}

#[test]
fn control_overlapping_with_interaction_qubits_is_rejected() {
    let control = Qubit::new(0);
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let error = decompose_mc_rzz_no_aux(&ParameterValue::Fixed(0.731), &[control], first, second)
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.rzz",
            ref reason,
        } if reason == "RZZ interaction qubits must not appear in controls; duplicate Q0"
    ));
}

#[test]
fn ancilla_overlapping_with_interaction_qubits_is_rejected() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let ancilla = Qubit::new(1);
    let error = decompose_mc_rzz_n_clean(
        &ParameterValue::Fixed(0.731),
        &[],
        first,
        second,
        &[ancilla],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.rzz",
            ref reason,
        } if reason == "RZZ interaction qubits must not appear in ancillas; duplicate Q1"
    ));
}

#[test]
fn extra_clean_ancillas_are_ignored() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let used_ancilla = Qubit::new(4);
    let theta = ParameterValue::Fixed(0.731);

    let operations = decompose_mc_rzz_n_clean(
        &theta,
        &controls,
        first,
        second,
        &[used_ancilla, Qubit::new(5)],
    )
    .unwrap();

    assert!(!operations.is_empty());
}

#[test]
fn insufficient_ancillas_error_propagates_from_center_rotation() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let first = Qubit::new(3);
    let second = Qubit::new(4);
    let error =
        decompose_mc_rzz_n_clean(&ParameterValue::Fixed(0.731), &controls, first, second, &[])
            .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ref reason,
        } if reason == "clean-accumulator MC-SU(2) decomposition with 3 controls requires 2 clean ancillas, got 0"
    ));
}
