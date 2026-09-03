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

use super::qcis::{decompose_qcis_n_clean, decompose_qcis_no_aux};
use super::test_utils::{
    EPSILON, assert_selected_matrix_columns_approx_eq, circuit_from_value_operations,
    mc_gate_matrix,
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{
    Instruction, Parameter, ParameterValue, Qubit, StandardGate, circuit_to_matrix,
};
use crate::compile::error::CompilerError;
#[test]
fn zero_controls_emit_original_qcis_gate() {
    let target = Qubit::new(0);
    for (gate, params) in [
        (StandardGate::X2P, vec![]),
        (StandardGate::X2M, vec![]),
        (StandardGate::Y2P, vec![]),
        (StandardGate::Y2M, vec![]),
        (StandardGate::XY2P, vec![ParameterValue::Fixed(0.731)]),
        (StandardGate::XY2M, vec![ParameterValue::Fixed(0.731)]),
    ] {
        let operations = decompose_qcis_n_clean(gate, &params, &[], target, &[target]).unwrap();
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            operations[0].instruction,
            ValueInstruction::Instruction(Instruction::Standard(actual)) if actual == gate
        ));
        assert_eq!(operations[0].qubits.as_slice(), &[target]);
        if params.is_empty() {
            assert!(operations[0].params.is_empty());
        } else {
            assert!(matches!(
                operations[0].params.as_slice(),
                [ParameterValue::Fixed(phi)] if phi.to_bits() == 0.731_f64.to_bits()
            ));
        }
    }
}

#[test]
fn xy_half_rotations_emit_target_basis_changes_around_crx() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    for (gate, theta) in [
        (StandardGate::XY2P, std::f64::consts::PI / 2.0),
        (StandardGate::XY2M, -std::f64::consts::PI / 2.0),
    ] {
        let operations =
            decompose_qcis_no_aux(gate, &[ParameterValue::Fixed(0.731)], &[control], target)
                .unwrap();
        assert_eq!(operations.len(), 3);
        assert!(matches!(
            operations[0].instruction,
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZ))
        ));
        assert!(matches!(
            operations[0].params.as_slice(),
            [ParameterValue::Fixed(phi)] if phi.to_bits() == (-0.731_f64).to_bits()
        ));
        assert!(matches!(
            operations[1].instruction,
            ValueInstruction::Instruction(Instruction::Standard(StandardGate::CRX))
        ));
        assert_eq!(operations[1].qubits.as_slice(), &[control, target]);
        assert!(matches!(
            operations[1].params.as_slice(),
            [ParameterValue::Fixed(actual)] if actual.to_bits() == theta.to_bits()
        ));
        assert!(matches!(
            operations[2].params.as_slice(),
            [ParameterValue::Fixed(phi)] if phi.to_bits() == 0.731_f64.to_bits()
        ));
    }
}

#[test]
fn symbolic_xy_angle_is_preserved() {
    let phi = Parameter::symbol("phi");
    let operations = decompose_qcis_no_aux(
        StandardGate::XY2P,
        &[ParameterValue::from(phi.clone())],
        &[Qubit::new(0)],
        Qubit::new(1),
    )
    .unwrap();

    assert!(matches!(
        operations[0].params.as_slice(),
        [ParameterValue::Param(actual)] if actual == &-phi.clone()
    ));
    assert!(matches!(
        operations[2].params.as_slice(),
        [ParameterValue::Param(actual)] if actual == &phi
    ));
}

#[test]
fn no_aux_decompositions_match_mcgate_semantics_exactly() {
    for num_controls in 1..=3 {
        let controls: Vec<_> = (0..num_controls)
            .map(|index| Qubit::new(index as u32))
            .collect();
        let target = Qubit::new(num_controls as u32);
        let total = num_controls + 1;
        for (gate, params) in [
            (StandardGate::X2P, vec![]),
            (StandardGate::X2M, vec![]),
            (StandardGate::Y2P, vec![]),
            (StandardGate::Y2M, vec![]),
            (StandardGate::XY2P, vec![ParameterValue::Fixed(0.731)]),
            (StandardGate::XY2M, vec![ParameterValue::Fixed(0.731)]),
        ] {
            let actual = circuit_to_matrix(
                &circuit_from_value_operations(
                    total,
                    decompose_qcis_no_aux(gate, &params, &controls, target).unwrap(),
                ),
                None,
            )
            .unwrap();
            let mut qubits = controls.clone();
            qubits.push(target);
            let expected = mc_gate_matrix(
                total,
                controls.len() as u8,
                gate,
                qubits,
                params.iter().cloned(),
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
    let params = [ParameterValue::Fixed(0.731)];
    let actual = circuit_to_matrix(
        &circuit_from_value_operations(
            6,
            decompose_qcis_n_clean(
                StandardGate::XY2P,
                &params,
                &controls,
                target,
                &clean_ancillas,
            )
            .unwrap(),
        ),
        None,
    )
    .unwrap();
    let mut qubits = controls.to_vec();
    qubits.push(target);
    let expected = mc_gate_matrix(
        6,
        controls.len() as u8,
        StandardGate::XY2P,
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
fn invalid_gate_and_parameter_counts_are_rejected() {
    let target = Qubit::new(0);
    for (gate, params, expected_reason) in [
        (
            StandardGate::X2P,
            vec![ParameterValue::Fixed(0.731)],
            "X2P decomposition requires 0 parameters, got 1",
        ),
        (
            StandardGate::XY2P,
            vec![],
            "XY2P decomposition requires 1 parameters, got 0",
        ),
        (
            StandardGate::XY2M,
            vec![ParameterValue::Fixed(0.1), ParameterValue::Fixed(0.2)],
            "XY2M decomposition requires 1 parameters, got 2",
        ),
        (
            StandardGate::H,
            vec![],
            "multi-controlled QCIS decomposition supports only X2P, X2M, Y2P, Y2M, XY2P, or XY2M, got H",
        ),
    ] {
        let error = decompose_qcis_no_aux(gate, &params, &[], target).unwrap_err();
        assert!(matches!(
            error,
            CompilerError::TransformFailed {
                name: "decompose.qcis",
                ref reason,
            } if reason == expected_reason
        ));
    }
}

#[test]
fn mc_su2_errors_are_propagated_without_rewriting() {
    let duplicate = Qubit::new(0);
    let error = decompose_qcis_no_aux(StandardGate::X2P, &[], &[duplicate], duplicate).unwrap_err();
    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ..
        }
    ));

    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error =
        decompose_qcis_n_clean(StandardGate::Y2P, &[], &controls, Qubit::new(3), &[]).unwrap_err();
    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.mc_su2",
            ..
        }
    ));
}
