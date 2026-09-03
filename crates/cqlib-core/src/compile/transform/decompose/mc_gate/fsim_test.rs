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
    EPSILON, assert_selected_matrix_columns_approx_eq, assert_value_operations_equal,
    assert_value_operations_only_use_qubits, circuit_from_value_operations, mc_gate_matrix,
};
use super::{
    fsim::{decompose_fsim_n_clean, decompose_fsim_no_aux},
    pauli_rotation::decompose_pauli_rotation_no_aux,
    phase::decompose_phase_no_aux,
    rotation::decompose_rotation_no_aux,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
};
use crate::compile::error::CompilerError;

#[test]
fn zero_controls_emit_original_standard_fsim_and_ignore_clean_ancillas() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    let theta = Parameter::symbol("theta");
    let params = [
        ParameterValue::Param(theta.clone()),
        ParameterValue::Fixed(-0.418),
    ];
    let operations = decompose_fsim_n_clean(&params, &[], first, second, &[first]).unwrap();

    assert_eq!(operations.len(), 1);
    assert!(matches!(
        operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::FSIM))
    ));
    assert_eq!(operations[0].qubits.as_slice(), &[first, second]);
    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Param(actual_theta), ParameterValue::Fixed(actual_phi)]
            if actual_theta == &theta && actual_phi.to_bits() == (-0.418_f64).to_bits()
    ));
}

#[test]
fn symbolic_decomposition_matches_interaction_phase_and_rotation_formula() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let theta = ParameterValue::Param(Parameter::symbol("theta"));
    let phi = Parameter::symbol("phi");
    let params = [theta.clone(), ParameterValue::Param(phi.clone())];

    let operations = decompose_fsim_no_aux(&params, &controls, first, second).unwrap();
    let mut expected =
        decompose_pauli_rotation_no_aux(StandardGate::RXX, &theta, &controls, first, second)
            .unwrap();
    expected.extend(
        decompose_pauli_rotation_no_aux(StandardGate::RYY, &theta, &controls, first, second)
            .unwrap(),
    );
    expected.extend(
        decompose_phase_no_aux(
            StandardGate::Phase,
            Some(&ParameterValue::from(phi.clone() * -0.5)),
            &controls,
            first,
        )
        .unwrap(),
    );
    let mut flattened_controls = controls.to_vec();
    flattened_controls.push(first);
    expected.extend(
        decompose_rotation_no_aux(
            StandardGate::RZ,
            &ParameterValue::from(phi * -1.0),
            &flattened_controls,
            second,
        )
        .unwrap(),
    );

    assert_value_operations_equal(&operations, &expected);
}

#[test]
fn no_aux_decompositions_match_mcgate_semantics_exactly() {
    let params = [ParameterValue::Fixed(0.731), ParameterValue::Fixed(-0.418)];
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
                decompose_fsim_no_aux(&params, &controls, first, second).unwrap(),
            ),
            None,
        )
        .unwrap();
        let mut qubits = controls.clone();
        qubits.extend([first, second]);
        let expected = mc_gate_matrix(
            total,
            controls.len() as u8,
            StandardGate::FSIM,
            qubits,
            params.iter().cloned(),
        );

        assert_selected_matrix_columns_approx_eq(&actual, &expected, 0..expected.ncols(), EPSILON);
    }
}

#[test]
fn clean_decomposition_matches_clean_subspace_and_restores_ancillas() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let first = Qubit::new(2);
    let second = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let params = [ParameterValue::Fixed(0.731), ParameterValue::Fixed(-0.418)];
    let actual = circuit_to_matrix(
        &circuit_from_value_operations(
            6,
            decompose_fsim_n_clean(&params, &controls, first, second, &clean_ancillas).unwrap(),
        ),
        None,
    )
    .unwrap();
    let mut qubits = controls.to_vec();
    qubits.extend([first, second]);
    let expected = mc_gate_matrix(
        6,
        controls.len() as u8,
        StandardGate::FSIM,
        qubits,
        params.iter().cloned(),
    );
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
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let params = [ParameterValue::Fixed(0.731), ParameterValue::Fixed(-0.418)];
    let operations = decompose_fsim_n_clean(
        &params,
        &controls,
        first,
        second,
        &[clean_ancillas[0], clean_ancillas[1], first, first],
    )
    .unwrap();

    assert_value_operations_only_use_qubits(
        &operations,
        &[
            controls[0],
            controls[1],
            first,
            second,
            clean_ancillas[0],
            clean_ancillas[1],
        ],
    );
}

#[test]
fn invalid_parameter_counts_are_rejected() {
    let first = Qubit::new(0);
    let second = Qubit::new(1);
    for params in [vec![], vec![ParameterValue::Fixed(0.731)]] {
        let error = decompose_fsim_no_aux(&params, &[], first, second).unwrap_err();

        assert!(matches!(
            error,
            CompilerError::TransformFailed {
                name: "decompose.fsim",
                ref reason,
            } if reason == &format!("FSIM decomposition requires 2 parameters, got {}", params.len())
        ));
    }
}

#[test]
fn duplicate_qubits_are_rejected() {
    let duplicate = Qubit::new(0);
    let params = [ParameterValue::Fixed(0.731), ParameterValue::Fixed(-0.418)];
    let error = decompose_fsim_no_aux(&params, &[duplicate], duplicate, Qubit::new(1)).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.fsim",
            ref reason,
        } if reason
            == "multi-controlled FSIM controls and targets must be distinct; duplicate Q0"
    ));
}

#[test]
fn insufficient_clean_ancilla_errors_are_propagated_without_rewriting() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let params = [ParameterValue::Fixed(0.731), ParameterValue::Fixed(-0.418)];
    let error = decompose_fsim_n_clean(
        &params,
        &controls,
        Qubit::new(2),
        Qubit::new(3),
        &[Qubit::new(4)],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ref reason,
        } if reason
            == "clean-accumulator MC-SU(2) decomposition with 3 controls requires 2 clean ancillas, got 1"
    ));
}
