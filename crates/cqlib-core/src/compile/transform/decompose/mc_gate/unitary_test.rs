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

use super::test_utils::{
    EPSILON, assert_fixed_parameter_operation, assert_selected_matrix_columns_approx_eq,
    assert_value_operations_only_use_qubits, circuit_from_value_operations, mc_gate_matrix,
};
use super::unitary::{decompose_unitary_n_clean, decompose_unitary_no_aux};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
};
use crate::compile::error::CompilerError;
#[test]
fn zero_controls_emit_original_standard_u() {
    let target = Qubit::new(3);
    let theta = ParameterValue::Param(Parameter::symbol("theta"));
    let phi = ParameterValue::Fixed(0.2);
    let lambda = ParameterValue::Fixed(-0.3);
    let operations = decompose_unitary_no_aux(&theta, &phi, &lambda, &[], target).unwrap();

    assert_eq!(operations.len(), 1);
    assert!(matches!(
        operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::U))
    ));
    assert_eq!(operations[0].qubits.as_slice(), &[target]);
    assert_eq!(operations[0].params.len(), 3);
    assert!(matches!(
        &operations[0].params[0],
        ParameterValue::Param(parameter) if parameter == &Parameter::symbol("theta")
    ));
    assert!(matches!(
        operations[0].params[1],
        ParameterValue::Fixed(value) if value.to_bits() == 0.2_f64.to_bits()
    ));
    assert!(matches!(
        operations[0].params[2],
        ParameterValue::Fixed(value) if value.to_bits() == (-0.3_f64).to_bits()
    ));
}

#[test]
fn one_control_emits_conditional_phase_and_zyz_rotations() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    let operations = decompose_unitary_no_aux(
        &ParameterValue::Fixed(0.2),
        &ParameterValue::Fixed(0.4),
        &ParameterValue::Fixed(0.6),
        &[control],
        target,
    )
    .unwrap();

    assert_eq!(operations.len(), 4);
    assert_fixed_parameter_operation(&operations[0], StandardGate::Phase, &[control], 0.5);
    assert_fixed_parameter_operation(&operations[1], StandardGate::CRZ, &[control, target], 0.6);
    assert_fixed_parameter_operation(&operations[2], StandardGate::CRY, &[control, target], 0.2);
    assert_fixed_parameter_operation(&operations[3], StandardGate::CRZ, &[control, target], 0.4);
}

#[test]
fn no_ancilla_decompositions_match_mcgate_unitary_matrices_exactly() {
    let (theta, phi, lambda) = (0.731, -0.418, 0.293);
    for num_controls in 1..=4 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let target = Qubit::new(num_controls as u32);
        let operations = decompose_unitary_no_aux(
            &ParameterValue::Fixed(theta),
            &ParameterValue::Fixed(phi),
            &ParameterValue::Fixed(lambda),
            &controls,
            target,
        )
        .unwrap();
        let actual = circuit_to_matrix(
            &circuit_from_value_operations(num_controls + 1, operations),
            None,
        )
        .unwrap();
        let mut qubits = controls.clone();
        qubits.push(target);
        let expected = mc_gate_matrix(
            num_controls + 1,
            controls.len() as u8,
            StandardGate::U,
            qubits,
            [
                ParameterValue::Fixed(theta),
                ParameterValue::Fixed(phi),
                ParameterValue::Fixed(lambda),
            ],
        );

        assert_selected_matrix_columns_approx_eq(&actual, &expected, 0..expected.ncols(), EPSILON);
    }
}

#[test]
fn clean_decomposition_matches_clean_subspace_and_restores_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let operations = decompose_unitary_n_clean(
        &ParameterValue::Fixed(0.731),
        &ParameterValue::Fixed(-0.418),
        &ParameterValue::Fixed(0.293),
        &controls,
        target,
        &clean_ancillas,
    )
    .unwrap();
    let actual = circuit_to_matrix(&circuit_from_value_operations(6, operations), None).unwrap();
    let mut qubits = controls.to_vec();
    qubits.push(target);
    let expected = mc_gate_matrix(
        6,
        controls.len() as u8,
        StandardGate::U,
        qubits,
        [
            ParameterValue::Fixed(0.731),
            ParameterValue::Fixed(-0.418),
            ParameterValue::Fixed(0.293),
        ],
    );
    let clean_mask = clean_ancillas
        .iter()
        .fold(0_usize, |mask, qubit| mask | (1 << qubit.index()));
    let clean_columns = (0..expected.ncols()).filter(|state| state & clean_mask == 0);

    assert_selected_matrix_columns_approx_eq(&actual, &expected, clean_columns, EPSILON);
}

#[test]
fn symbolic_parameters_are_preserved_in_conditional_phase_and_rotations() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let lambda = Parameter::symbol("lambda");
    let operations = decompose_unitary_no_aux(
        &ParameterValue::Param(theta.clone()),
        &ParameterValue::Param(phi.clone()),
        &ParameterValue::Param(lambda.clone()),
        &[control],
        target,
    )
    .unwrap();

    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &((phi.clone() + lambda.clone()) * 0.5)
    ));
    assert!(matches!(
        operations[1].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &lambda
    ));
    assert!(matches!(
        operations[2].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &theta
    ));
    assert!(matches!(
        operations[3].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &phi
    ));
}

#[test]
fn extra_clean_ancillas_are_ignored_without_validation_or_use() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);
    let used_ancilla = Qubit::new(3);
    let operations = decompose_unitary_n_clean(
        &ParameterValue::Fixed(0.731),
        &ParameterValue::Fixed(-0.418),
        &ParameterValue::Fixed(0.293),
        &controls,
        target,
        &[used_ancilla, target, target],
    )
    .unwrap();

    assert_value_operations_only_use_qubits(
        &operations,
        &[controls[0], controls[1], target, used_ancilla],
    );
}

#[test]
fn mc_su2_errors_are_propagated_without_rewriting() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error = decompose_unitary_n_clean(
        &ParameterValue::Fixed(0.731),
        &ParameterValue::Fixed(-0.418),
        &ParameterValue::Fixed(0.293),
        &controls,
        Qubit::new(3),
        &[],
    )
    .unwrap_err();

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
    let error = decompose_unitary_no_aux(
        &ParameterValue::Fixed(0.731),
        &ParameterValue::Fixed(-0.418),
        &ParameterValue::Fixed(0.293),
        &[duplicate],
        duplicate,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ref reason,
        } if reason
            == &format!(
                "MC-SU(2) controls, target, and ancillas must be distinct; duplicate {duplicate}"
            )
    ));
}
