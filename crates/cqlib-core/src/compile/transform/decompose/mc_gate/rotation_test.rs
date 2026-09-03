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

use super::test_utils::assert_value_operations_equal;
use super::{
    mc_su2::{Su2RotationAxis, decompose_mc_su2_n_clean, decompose_mc_su2_no_aux},
    rotation::{decompose_rotation_n_clean, decompose_rotation_no_aux},
};
use crate::circuit::value_instruction::ValueInstruction;
use crate::circuit::{Instruction, Parameter, ParameterValue, Qubit, StandardGate};
use crate::compile::error::CompilerError;
#[test]
fn standard_rotation_fast_paths_are_preserved() {
    let control = Qubit::new(0);
    let target = Qubit::new(1);
    let theta = ParameterValue::Fixed(0.731);

    let cases = [
        (StandardGate::RX, StandardGate::RX, StandardGate::CRX),
        (StandardGate::RY, StandardGate::RY, StandardGate::CRY),
        (StandardGate::RZ, StandardGate::RZ, StandardGate::CRZ),
    ];
    for (rotation, expected_zero, expected_one) in cases {
        let zero = decompose_rotation_no_aux(rotation, &theta, &[], target).unwrap();
        assert_eq!(zero.len(), 1);
        assert!(matches!(
            zero[0].instruction,
            ValueInstruction::Instruction(Instruction::Standard(gate)) if gate == expected_zero
        ));
        assert_eq!(zero[0].qubits.as_slice(), &[target]);

        let one = decompose_rotation_no_aux(rotation, &theta, &[control], target).unwrap();
        assert_eq!(one.len(), 1);
        assert!(matches!(
            one[0].instruction,
            ValueInstruction::Instruction(Instruction::Standard(gate)) if gate == expected_one
        ));
        assert_eq!(one[0].qubits.as_slice(), &[control, target]);
    }
}

#[test]
fn controlled_rotation_forms_use_flattened_controls_without_adding_controls() {
    let controls = [Qubit::new(0), Qubit::new(1)];
    let target = Qubit::new(2);
    let theta = ParameterValue::Fixed(0.731);

    let cases = [
        (StandardGate::CRX, Su2RotationAxis::X),
        (StandardGate::CRY, Su2RotationAxis::Y),
        (StandardGate::CRZ, Su2RotationAxis::Z),
    ];
    for (rotation, axis) in cases {
        assert_value_operations_equal(
            &decompose_rotation_no_aux(rotation, &theta, &controls, target).unwrap(),
            &decompose_mc_su2_no_aux(axis, &theta, &controls, target).unwrap(),
        );
    }
}

#[test]
fn public_rotation_algorithms_delegate_to_matching_mc_su2_algorithms() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let target = Qubit::new(3);
    let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
    let theta = ParameterValue::Param(Parameter::symbol("theta"));

    for (rotation, axis) in [
        (StandardGate::RX, Su2RotationAxis::X),
        (StandardGate::RY, Su2RotationAxis::Y),
        (StandardGate::RZ, Su2RotationAxis::Z),
    ] {
        assert_value_operations_equal(
            &decompose_rotation_no_aux(rotation, &theta, &controls, target).unwrap(),
            &decompose_mc_su2_no_aux(axis, &theta, &controls, target).unwrap(),
        );
        assert_value_operations_equal(
            &decompose_rotation_n_clean(rotation, &theta, &controls, target, &clean_ancillas)
                .unwrap(),
            &decompose_mc_su2_n_clean(axis, &theta, &controls, target, &clean_ancillas).unwrap(),
        );
    }
}

#[test]
fn invalid_rotation_is_rejected_before_mc_su2_synthesis() {
    let duplicate = Qubit::new(0);
    let error = decompose_rotation_no_aux(
        StandardGate::Phase,
        &ParameterValue::Fixed(0.731),
        &[duplicate],
        duplicate,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::TransformFailed {
            name: "decompose.rotation",
            ref reason,
        } if reason
            == "multi-controlled rotation decomposition supports only RX, CRX, RY, CRY, RZ, or CRZ, got Phase"
    ));
}

#[test]
fn mc_su2_errors_are_propagated_without_rewriting() {
    let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
    let error = decompose_rotation_n_clean(
        StandardGate::RY,
        &ParameterValue::Fixed(0.731),
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
            == "clean-accumulator MC-SU(2) decomposition with 3 controls requires 2 clean ancillas, got 0"
    ));
}
