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

use super::phase::{decompose_phase_n_clean, decompose_phase_no_aux};
use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_approx_eq, assert_standard_operation,
    assert_value_operations_only_use_qubits, circuit_from_value_operations, mc_gate_matrix,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
};
use crate::compile::error::CompilerError;
#[test]
fn zero_controls_emit_original_standard_phase_gates() {
    let target = Qubit::new(3);
    for phase in [
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::T,
        StandardGate::TDG,
    ] {
        let operations = decompose_phase_no_aux(phase, None, &[], target).unwrap();

        assert_eq!(operations.len(), 1);
        assert_standard_operation(&operations[0], phase, &[target]);
    }

    let theta = ParameterValue::Fixed(0.731);
    let operations =
        decompose_phase_no_aux(StandardGate::Phase, Some(&theta), &[], target).unwrap();
    assert_eq!(operations.len(), 1);
    assert!(matches!(
        operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::Phase))
    ));
    assert_eq!(operations[0].qubits.as_slice(), &[target]);
    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Fixed(value)] if value.to_bits() == 0.731_f64.to_bits()
    ));
}

#[test]
fn one_control_phase_emits_conditional_phase_and_crz() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    let operations = decompose_phase_no_aux(
        StandardGate::Phase,
        Some(&ParameterValue::Fixed(0.8)),
        &[control],
        target,
    )
    .unwrap();

    assert_eq!(operations.len(), 2);
    assert!(matches!(
        operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::Phase))
    ));
    assert_eq!(operations[0].qubits.as_slice(), &[control]);
    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Fixed(theta)] if theta.to_bits() == 0.4_f64.to_bits()
    ));
    assert!(matches!(
        operations[1].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::CRZ))
    ));
    assert_eq!(operations[1].qubits.as_slice(), &[control, target]);
    assert!(matches!(
        operations[1].params.as_slice(),
        [ParameterValue::Fixed(theta)] if theta.to_bits() == 0.8_f64.to_bits()
    ));
}

#[test]
fn no_ancilla_decompositions_match_mcgate_phase_matrices_exactly() {
    let cases = [
        (StandardGate::S, None),
        (StandardGate::SDG, None),
        (StandardGate::T, None),
        (StandardGate::TDG, None),
        (StandardGate::Phase, Some(0.731)),
    ];
    for num_controls in 1..=4 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let target = Qubit::new(num_controls as u32);
        for (phase, theta) in cases {
            let parameter = theta.map(ParameterValue::Fixed);
            let actual = circuit_to_matrix(
                &circuit_from_value_operations(
                    num_controls + 1,
                    decompose_phase_no_aux(phase, parameter.as_ref(), &controls, target).unwrap(),
                ),
                None,
            )
            .unwrap();
            let mut qubits = controls.clone();
            qubits.push(target);
            let expected = mc_gate_matrix(
                num_controls + 1,
                controls.len() as u8,
                phase,
                qubits,
                theta.map(ParameterValue::Fixed),
            );

            assert_selected_matrix_columns_approx_eq(
                &actual,
                &expected,
                0..expected.ncols(),
                EPSILON,
            );
        }
    }
}

#[test]
fn clean_decomposition_matches_clean_subspace_and_restores_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let operations = decompose_phase_n_clean(
        StandardGate::Phase,
        Some(&ParameterValue::Fixed(0.731)),
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
        StandardGate::Phase,
        qubits,
        Some(ParameterValue::Fixed(0.731)),
    );
    let clean_mask = clean_ancillas
        .iter()
        .fold(0_usize, |mask, qubit| mask | (1 << qubit.index()));
    let clean_columns = (0..expected.ncols()).filter(|state| state & clean_mask == 0);

    assert_selected_matrix_columns_approx_eq(&actual, &expected, clean_columns, EPSILON);
}

#[test]
fn symbolic_theta_is_recursively_scaled_without_evaluation() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);
    let theta = Parameter::symbol("theta");
    let operations = decompose_phase_no_aux(
        StandardGate::Phase,
        Some(&ParameterValue::Param(theta.clone())),
        &controls,
        target,
    )
    .unwrap();

    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &((theta.clone() * 0.5) * 0.5)
    ));
    assert!(matches!(
        operations[1].params.as_slice(),
        [ParameterValue::Param(parameter)] if parameter == &(theta * 0.5)
    ));
}

#[test]
fn extra_clean_ancillas_are_ignored_without_validation_or_use() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);
    let used_ancilla = Qubit::new(3);
    let operations = decompose_phase_n_clean(
        StandardGate::Phase,
        Some(&ParameterValue::Fixed(0.731)),
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
fn invalid_gate_and_parameter_combinations_are_rejected() {
    let target = Qubit::new(0);
    for (phase, theta, expected_reason) in [
        (
            StandardGate::Phase,
            None,
            "Phase decomposition requires one theta parameter",
        ),
        (
            StandardGate::S,
            Some(ParameterValue::Fixed(0.731)),
            "S decomposition does not accept a theta parameter",
        ),
        (
            StandardGate::H,
            None,
            "multi-controlled phase decomposition supports only S, SDG, T, TDG, or Phase, got H",
        ),
    ] {
        let error = decompose_phase_no_aux(phase, theta.as_ref(), &[], target).unwrap_err();

        assert!(matches!(
            error,
            CompilerError::TransformFailed {
                name: "decompose.phase",
                ref reason,
            } if reason == expected_reason
        ));
    }
}

#[test]
fn duplicate_qubit_errors_are_propagated_without_rewriting() {
    let duplicate = Qubit::new(0);
    let error = decompose_phase_no_aux(
        StandardGate::Phase,
        Some(&ParameterValue::Fixed(0.731)),
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
