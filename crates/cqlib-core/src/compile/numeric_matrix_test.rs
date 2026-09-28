// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::one_qubit_run_matrix;
use crate::circuit::{
    Directive, Instruction, Parameter, ParameterValue, Qubit, StandardGate,
    ValueClassicalControlOp, ValueInstruction, ValueOperation,
};
use ndarray::{Array2, array};
use num_complex::Complex64;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn operation(gate: StandardGate, params: &[f64]) -> ValueOperation {
    ValueOperation::from_standard(
        gate,
        [Qubit::new(0)],
        params.iter().copied().map(ParameterValue::Fixed),
    )
}

fn assert_matrix(operations: &[ValueOperation], expected: Array2<Complex64>) {
    let actual = one_qubit_run_matrix(operations).expect("valid one-qubit run");
    assert_eq!(actual.dim(), expected.dim());
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert!((actual - expected).norm() < 1e-12, "{actual} != {expected}");
    }
}

#[test]
fn empty_run_is_identity() {
    assert_matrix(&[], Array2::eye(2));
}

#[test]
fn noncommuting_gates_follow_circuit_execution_order() {
    // H followed by Z negates the second row, not the second column.
    assert_matrix(
        &[
            operation(StandardGate::H, &[]),
            operation(StandardGate::Z, &[]),
        ],
        array![[1.0, 1.0], [-1.0, 1.0]].mapv(|value| Complex64::new(value * FRAC_1_SQRT_2, 0.0)),
    );
}

#[test]
fn scalar_phase_is_preserved() {
    assert_matrix(
        &[
            operation(StandardGate::X, &[]),
            operation(StandardGate::Y, &[]),
            operation(StandardGate::X, &[]),
            operation(StandardGate::Y, &[]),
        ],
        -Array2::eye(2),
    );
}

#[test]
fn fixed_rotation_is_reconstructed_without_interpreting_labels() {
    let mut rz = operation(StandardGate::RZ, &[PI]);
    rz.label = Some("preserve-at-pass-boundary".into());
    assert_matrix(
        &[rz],
        array![
            [Complex64::new(0.0, -1.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(0.0, 1.0)]
        ],
    );
}

#[test]
fn rejects_wrong_parameter_counts_for_all_standard_one_qubit_gates() {
    for gate in StandardGate::all()
        .iter()
        .copied()
        .filter(|gate| gate.num_qubits() == 1)
    {
        let params = vec![0.25; gate.num_params()];
        assert!(
            one_qubit_run_matrix(&[operation(gate, &params)]).is_some(),
            "{gate:?}"
        );
        if !params.is_empty() {
            assert!(
                one_qubit_run_matrix(&[operation(gate, &params[1..])]).is_none(),
                "{gate:?}"
            );
        }
        assert!(
            one_qubit_run_matrix(&[operation(gate, &vec![0.25; params.len() + 1])]).is_none(),
            "{gate:?}"
        );
    }
}

#[test]
fn rejects_symbolic_and_nonfinite_parameters() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(one_qubit_run_matrix(&[operation(StandardGate::RZ, &[value])]).is_none());
    }
    let mut rz = operation(StandardGate::RZ, &[0.0]);
    rz.params[0] = ParameterValue::Param(Parameter::symbol("theta"));
    assert!(one_qubit_run_matrix(&[rz]).is_none());
}

#[test]
fn rejects_incompatible_qubit_support() {
    let h = operation(StandardGate::H, &[]);
    let mut other_qubit = h.clone();
    other_qubit.qubits[0] = Qubit::new(1);
    assert!(one_qubit_run_matrix(&[h.clone(), other_qubit]).is_none());

    let mut no_qubits = h.clone();
    no_qubits.qubits.clear();
    assert!(one_qubit_run_matrix(&[no_qubits]).is_none());
    let mut extra_qubit = h;
    extra_qubit.qubits.push(Qubit::new(1));
    assert!(one_qubit_run_matrix(&[extra_qubit]).is_none());

    let cx = ValueOperation::from_standard(StandardGate::CX, [Qubit::new(0), Qubit::new(1)], []);
    assert!(one_qubit_run_matrix(&[cx]).is_none());
    let phase =
        ValueOperation::from_standard(StandardGate::GPhase, [], [ParameterValue::Fixed(0.2)]);
    assert!(one_qubit_run_matrix(&[phase]).is_none());
}

#[test]
fn rejects_nonstandard_and_control_flow_instructions() {
    for instruction in [
        ValueInstruction::Instruction(Instruction::Directive(Directive::Barrier)),
        ValueInstruction::Instruction(Instruction::Delay),
        ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break),
    ] {
        let mut invalid = operation(StandardGate::H, &[]);
        invalid.instruction = instruction;
        assert!(one_qubit_run_matrix(&[invalid]).is_none());
    }
}
