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

use super::*;
use crate::circuit::gate::{FrozenCircuit, MCGate, UnitaryGate};
use crate::circuit::symbolic_matrix::gate::{
    apply_gate_to_matrix, apply_standard_gate_to_matrix, circuit_to_symbolic_matrix,
    standard_gate_symbolic_matrix,
};
use crate::circuit::symbolic_matrix::matrix::{
    evaluate_symbolic_matrix, substitute_symbolic_matrix, symbolic_eye,
};
use crate::circuit::symbolic_matrix::test_utils::assert_matrix_approx_eq;
use crate::circuit::{Circuit, Directive, Instruction, ParameterValue, Qubit};
use crate::circuit::{ClassicalType, circuit_to_matrix};
use ndarray::{Array2, array};
use num_complex::Complex64;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

fn assert_standard_gate_matches(gate: StandardGate, params: &[f64]) {
    let symbolic_params: Vec<Parameter> = params.iter().map(|&v| Parameter::from(v)).collect();
    let symbolic = standard_gate_symbolic_matrix(gate, &symbolic_params).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = gate.matrix(params).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);
}

#[test]
fn test_gphase_symbolic_gate_and_controls_match_scalar_formula() {
    let theta = Parameter::symbol("theta");
    for controls in 0..=2 {
        let base =
            standard_gate_symbolic_matrix(StandardGate::GPhase, std::slice::from_ref(&theta))
                .unwrap();
        assert_eq!(base.dim(), (1, 1));
        let matrix = control_matrix(&base, controls);
        let dim = 1 << controls;
        assert_eq!(matrix.dim(), (dim, dim));
        for angle in [0.0, 0.37, -0.51, PI] {
            let bindings = HashMap::from([("theta", angle)]);
            let actual = evaluate_symbolic_matrix(&matrix, &Some(bindings)).unwrap();
            let mut expected = Array2::eye(dim);
            expected[[dim - 1, dim - 1]] = Complex64::from_polar(1.0, angle);
            assert_matrix_approx_eq(&actual, &expected, 1e-12);
        }
    }
}

#[test]
fn test_gphase_symbolic_markers_accumulate_across_numeric_runs() {
    for num_qubits in [0, 1, 3] {
        let mut circuit = Circuit::new(num_qubits);
        if num_qubits > 0 {
            circuit.h(Qubit::new(0)).unwrap();
        }
        for phase in [Parameter::from(-0.21), Parameter::symbol("theta")] {
            circuit
                .append(
                    Instruction::Standard(StandardGate::GPhase),
                    [] as [Qubit; 0],
                    [phase.into()],
                    None,
                )
                .unwrap();
        }
        if num_qubits > 0 {
            circuit
                .rz(Qubit::new(0), Parameter::symbol("alpha"))
                .unwrap();
        }
        circuit
            .append(
                Instruction::Standard(StandardGate::GPhase),
                [] as [Qubit; 0],
                [0.12.into()],
                None,
            )
            .unwrap();
        circuit.set_global_phase(Parameter::from(0.17));
        let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
        let mut baseline = Circuit::new(num_qubits);
        if num_qubits > 0 {
            baseline.h(Qubit::new(0)).unwrap();
            baseline.rz(Qubit::new(0), Parameter::from(0.23)).unwrap();
        }
        let baseline = circuit_to_matrix(&baseline, None).unwrap();
        for theta in [0.0, 0.37, -0.51, PI] {
            let bindings = HashMap::from([("theta", theta), ("alpha", 0.23)]);
            let actual = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
            let expected = &baseline * Complex64::from_polar(1.0, theta - 0.21 + 0.12 + 0.17);
            assert_matrix_approx_eq(&actual, &expected, 1e-12);
        }
    }
}

#[test]
fn test_gphase_symbolic_controlled_circuit_with_constant_and_symbolic_params() {
    for controls in [vec![], vec![2usize], vec![2, 0]] {
        for phase in [Parameter::from(0.37), Parameter::symbol("theta")] {
            let mut circuit = Circuit::new(3);
            circuit.h(Qubit::new(1)).unwrap();
            circuit
                .multi_control(
                    StandardGate::GPhase,
                    controls.iter().map(|&q| Qubit::new(q as u32)),
                    [] as [Qubit; 0],
                    [phase.into()],
                )
                .unwrap();
            for order in [[0, 1, 2], [2, 0, 1]] {
                let spectator_bit = order.iter().position(|&q| q == 1).unwrap();
                let mask = controls.iter().fold(0, |mask, control| {
                    mask | (1 << order.iter().position(|q| q == control).unwrap())
                });
                let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&order)).unwrap();
                let bindings = HashMap::from([("theta", 0.37)]);
                let actual = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
                let mut expected = Array2::zeros((8, 8));
                for row in 0..8 {
                    for col in 0..8 {
                        if (row ^ col) & !(1 << spectator_bit) == 0 {
                            let sign = if row & col & (1 << spectator_bit) != 0 {
                                -1.0
                            } else {
                                1.0
                            };
                            let angle = if row & mask == mask { 0.37 } else { 0.0 };
                            expected[[row, col]] =
                                Complex64::from_polar(1.0, angle) * sign / std::f64::consts::SQRT_2;
                        }
                    }
                }
                assert_matrix_approx_eq(&actual, &expected, 1e-12);
            }
        }
    }
}

#[test]
fn test_fixed_standard_gate_matches_numeric_matrix() {
    let symbolic = standard_gate_symbolic_matrix(StandardGate::H, &[]).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let numeric = StandardGate::H.matrix(&[]).unwrap();

    assert_matrix_approx_eq(&evaluated, numeric.as_ref(), 1e-12);
}

#[test]
fn test_classical_data_has_no_symbolic_matrix_representation() {
    let mut circuit = Circuit::new(1);
    let bit = circuit.var(ClassicalType::Bit);
    circuit.measure_into(Qubit::new(0), bit).unwrap();

    let err = circuit_to_symbolic_matrix(&circuit, None).unwrap_err();
    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_parametric_rx_symbolic_matrix_evaluates() {
    let theta = Parameter::symbol("theta");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::RX, &[theta]).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", PI / 2.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = StandardGate::RX.matrix(&[PI / 2.0]).unwrap();

    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-12);
}

#[test]
fn test_parametric_u_symbolic_matrix_evaluates() {
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let lambda = Parameter::symbol("lambda");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::U, &[theta, phi, lambda]).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.25);
    bindings.insert("phi", -0.5);
    bindings.insert("lambda", 0.75);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = StandardGate::U.matrix(&[0.25, -0.5, 0.75]).unwrap();

    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-12);
}

#[test]
fn test_all_non_parametric_standard_gates_match_numeric() {
    let gates = [
        StandardGate::I,
        StandardGate::H,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::T,
        StandardGate::TDG,
        StandardGate::X,
        StandardGate::Y,
        StandardGate::Z,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::Y2P,
        StandardGate::Y2M,
        StandardGate::SWAP,
        StandardGate::CX,
        StandardGate::CY,
        StandardGate::CZ,
        StandardGate::CCX,
    ];
    for gate in gates {
        assert_standard_gate_matches(gate, &[]);
    }
}

#[test]
fn test_all_single_param_standard_gates_match_numeric() {
    let test_param = 0.42;
    let gates = [
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::Phase,
        StandardGate::GPhase,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::RZX,
        StandardGate::CRX,
        StandardGate::CRY,
        StandardGate::CRZ,
        StandardGate::XY,
        StandardGate::XY2P,
        StandardGate::XY2M,
    ];
    for gate in gates {
        assert_standard_gate_matches(gate, &[test_param]);
    }
}

#[test]
fn test_all_multi_param_standard_gates_match_numeric() {
    assert_standard_gate_matches(StandardGate::RXY, &[0.37, -0.91]);
    assert_standard_gate_matches(StandardGate::FSIM, &[0.25, 0.17]);
    assert_standard_gate_matches(StandardGate::U, &[0.73, -0.42, 1.15]);
}

#[test]
fn test_parametric_standard_gates_with_symbolic_params_match_numeric() {
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let lambda = Parameter::symbol("lambda");

    // Single-param gates with a symbolic parameter
    for gate in [
        StandardGate::RX,
        StandardGate::RY,
        StandardGate::RZ,
        StandardGate::Phase,
        StandardGate::RXX,
        StandardGate::RYY,
        StandardGate::RZZ,
        StandardGate::RZX,
        StandardGate::CRX,
        StandardGate::CRY,
        StandardGate::CRZ,
        StandardGate::XY,
        StandardGate::XY2P,
        StandardGate::XY2M,
    ] {
        let symbolic = standard_gate_symbolic_matrix(gate, std::slice::from_ref(&theta)).unwrap();
        let mut bindings = HashMap::new();
        bindings.insert("theta", 0.63);
        let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
        let expected = gate.matrix(&[0.63]).unwrap();
        assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);
    }

    // Two-param gates
    let sym_rxy =
        standard_gate_symbolic_matrix(StandardGate::RXY, &[theta.clone(), phi.clone()]).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.51);
    bindings.insert("phi", -0.33);
    let evaluated = evaluate_symbolic_matrix(&sym_rxy, &Some(bindings.clone())).unwrap();
    let expected = StandardGate::RXY.matrix(&[0.51, -0.33]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);

    let sym_fsim =
        standard_gate_symbolic_matrix(StandardGate::FSIM, &[theta.clone(), phi.clone()]).unwrap();
    let evaluated = evaluate_symbolic_matrix(&sym_fsim, &Some(bindings.clone())).unwrap();
    let expected = StandardGate::FSIM.matrix(&[0.51, -0.33]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);

    // Three-param gate
    let sym_u = standard_gate_symbolic_matrix(
        StandardGate::U,
        &[theta.clone(), phi.clone(), lambda.clone()],
    )
    .unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.11);
    bindings.insert("phi", 0.22);
    bindings.insert("lambda", 0.33);
    let evaluated = evaluate_symbolic_matrix(&sym_u, &Some(bindings.clone())).unwrap();
    let expected = StandardGate::U.matrix(&[0.11, 0.22, 0.33]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);
}

#[test]
fn test_standard_gate_parameter_count_mismatch() {
    // RX expects 1 parameter
    let err = standard_gate_symbolic_matrix(StandardGate::RX, &[]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        }
    ));

    let err = standard_gate_symbolic_matrix(
        StandardGate::RX,
        &[Parameter::from(1.0), Parameter::from(2.0)],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 2
        }
    ));

    // U expects 3 parameters
    let err = standard_gate_symbolic_matrix(StandardGate::U, &[Parameter::from(1.0)]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 3,
            actual: 1
        }
    ));

    // H expects 0 parameters
    let err = standard_gate_symbolic_matrix(StandardGate::H, &[Parameter::from(1.0)]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 0,
            actual: 1
        }
    ));
}

#[test]
fn test_symbolic_circuit_matches_bound_numeric_circuit() {
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), theta).unwrap();
    circuit.rz(Qubit::new(0), phi).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.37);
    bindings.insert("phi", -0.91);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_entangling_circuit_matches_numeric_circuit() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.rz(Qubit::new(1), theta).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.42);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_single_qubit_optimized_path_matches_numeric_circuit() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(2), theta).unwrap();
    circuit.rz(Qubit::new(1), 0.25).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[2, 0, 1])).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", -0.73);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, Some(&[2, 0, 1])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_two_qubit_optimized_path_matches_numeric_circuit() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(3);
    circuit.rxx(Qubit::new(2), Qubit::new(0), theta).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.ry(Qubit::new(0), 0.41).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[1, 2, 0])).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.64);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, Some(&[1, 2, 0])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_general_gate_path_matches_numeric_circuit() {
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(1)).unwrap();
    circuit
        .ccx(Qubit::new(2), Qubit::new(0), Qubit::new(1))
        .unwrap();
    circuit.rz(Qubit::new(2), 0.17).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[2, 0, 1])).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, Some(&[2, 0, 1])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_global_phase_is_included() {
    let phi = Parameter::symbol("phi");
    let mut circuit = Circuit::new(1);
    circuit.x(Qubit::new(0)).unwrap();
    circuit.set_global_phase(phi);

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("phi", PI);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = array![
        [Complex64::new(0.0, 0.0), Complex64::new(-1.0, 0.0)],
        [Complex64::new(-1.0, 0.0), Complex64::new(0.0, 0.0)],
    ];

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_qubits_order_matches_numeric_path() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.ry(Qubit::new(0), theta).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[1, 0])).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", -0.33);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, Some(&[1, 0])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_invalid_qubits_order_errors() {
    let circuit = Circuit::new(2);
    let err = circuit_to_symbolic_matrix(&circuit, Some(&[0, 0])).unwrap_err();

    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_symbolic_measure_has_no_matrix() {
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::Directive(Directive::Measure),
            [Qubit::new(0)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let err = circuit_to_symbolic_matrix(&circuit, None).unwrap_err();

    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_empty_circuit_identity() {
    let circuit = Circuit::new(2);
    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = Array2::eye(4);
    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_zero_qubit_circuit_identity() {
    let circuit = Circuit::new(0);
    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = Array2::eye(1);
    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);

    let numeric = circuit_to_matrix(&circuit, None).unwrap();
    assert_matrix_approx_eq(&evaluated, &numeric, 1e-12);
}

#[test]
fn test_delay_is_skipped() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    circuit
        .append(
            Instruction::Delay,
            [Qubit::new(0)],
            [ParameterValue::Fixed(1.0)],
            None,
        )
        .unwrap();
    circuit.x(Qubit::new(0)).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_barrier_is_skipped() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Barrier),
            [Qubit::new(0), Qubit::new(1)],
            [],
            None,
        )
        .unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();

    let mut expected = Circuit::new(2);
    expected.h(Qubit::new(0)).unwrap();
    expected.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let expected_matrix = circuit_to_matrix(&expected, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected_matrix, 1e-12);
}

#[test]
fn test_reset_has_no_matrix() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    circuit
        .append(
            Instruction::Directive(Directive::Reset),
            [Qubit::new(0)],
            [],
            None,
        )
        .unwrap();

    let err = circuit_to_symbolic_matrix(&circuit, None).unwrap_err();
    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_qubits_order_missing_qubit() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    // Order missing qubit 1
    let err = circuit_to_symbolic_matrix(&circuit, Some(&[0])).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_qubits_order_extra_qubit() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    // Order includes qubit 2 which is not in the circuit
    let err = circuit_to_symbolic_matrix(&circuit, Some(&[0, 1, 2])).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_qubits_order_duplicate_qubit() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let err = circuit_to_symbolic_matrix(&circuit, Some(&[0, 0])).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_global_phase_with_bindings() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), theta.clone()).unwrap();
    circuit.set_global_phase(theta);

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", PI / 3.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_circuit_gate_replaces_inner_symbols() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let gate = inner.to_gate("InnerRx").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::from(Parameter::symbol("x") * 2.0)],
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.2);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_circuit_gate_reversed_bits() {
    // Inner circuit: CNOT(q0 -> q1)  (asymmetric, so bit-order matters)
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let gate = inner.to_gate("CnotGate").unwrap();

    // Apply as CircuitGate to (q1, q0) in a 2-qubit circuit
    let mut circuit = Circuit::new(2);
    circuit
        .append(gate, [Qubit::new(1), Qubit::new(0)], [], None)
        .unwrap();
    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();

    // Direct CNOT(q1 -> q0) should produce the same matrix
    let mut expected_circuit = Circuit::new(2);
    expected_circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();
    let expected = circuit_to_matrix(&expected_circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_circuit_gate_simultaneous_substitution() {
    // Inner circuit: RX(a + b)
    let mut inner = Circuit::new(1);
    inner
        .rx(
            Qubit::new(0),
            Parameter::symbol("a") + Parameter::symbol("b"),
        )
        .unwrap();
    let gate = inner.to_gate("AddGate").unwrap();

    // Outer call swaps a <-> b: parameters are [b, a]
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [
                ParameterValue::from(Parameter::symbol("b")),
                ParameterValue::from(Parameter::symbol("a")),
            ],
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("a", 1.0);
    bindings.insert("b", 2.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();

    // After simultaneous substitution a->b, b->a, the expression becomes b + a = 2 + 1 = 3
    let expected = StandardGate::RX.matrix(&[3.0]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);
}

#[test]
fn test_symbolic_circuit_gate_param_count_mismatch_is_rejected_on_append() {
    let theta = Parameter::symbol("theta");
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), theta).unwrap();
    let gate = inner.to_gate("RxGate").unwrap();

    let mut circuit = Circuit::new(1);
    let err = circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::Fixed(1.0), ParameterValue::Fixed(2.0)],
            None,
        )
        .unwrap_err();

    assert!(matches!(err, CircuitError::ParameterCountMismatch { .. }));
    assert!(circuit.operations().is_empty());
}

#[test]
fn test_circuit_gate_matches_standard_sequence() {
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let gate = inner.to_gate("CnotGate").unwrap();

    let mut cg_circuit = Circuit::new(2);
    cg_circuit
        .append(gate.clone(), [Qubit::new(0), Qubit::new(1)], [], None)
        .unwrap();

    let mut native_circuit = Circuit::new(2);
    native_circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let cg_symbolic = circuit_to_symbolic_matrix(&cg_circuit, None).unwrap();
    let native_symbolic = circuit_to_symbolic_matrix(&native_circuit, None).unwrap();
    let cg_eval = evaluate_symbolic_matrix(&cg_symbolic, &None).unwrap();
    let native_eval = evaluate_symbolic_matrix(&native_symbolic, &None).unwrap();

    assert_matrix_approx_eq(&cg_eval, &native_eval, 1e-12);
}

#[test]
fn test_circuit_gate_different_qubits_order() {
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let gate = inner.to_gate("CnotGate").unwrap();

    let mut cg_circuit = Circuit::new(2);
    cg_circuit
        .append(gate, [Qubit::new(0), Qubit::new(1)], [], None)
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&cg_circuit, Some(&[1, 0])).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&cg_circuit, Some(&[1, 0])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_circuit_gate_parameter_substitution_order() {
    // Inner circuit: RX(a) * RZ(b) — order matters because RX and RZ don't commute
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("a")).unwrap();
    inner.rz(Qubit::new(0), Parameter::symbol("b")).unwrap();
    let gate = inner.to_gate("OrderedGate").unwrap();

    // Outer call: [phi, theta] maps a->phi, b->theta
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [
                ParameterValue::from(Parameter::symbol("phi")),
                ParameterValue::from(Parameter::symbol("theta")),
            ],
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("phi", 0.5);
    bindings.insert("theta", 0.3);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    // Expected: RX(0.5) then RZ(0.3)
    let mut expected_circuit = Circuit::new(1);
    expected_circuit.rx(Qubit::new(0), 0.5).unwrap();
    expected_circuit.rz(Qubit::new(0), 0.3).unwrap();
    let expected = circuit_to_matrix(&expected_circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_substitution_collision_detection() {
    let mut inner = Circuit::new(1);
    inner
        .rx(
            Qubit::new(0),
            Parameter::symbol("__cqlib_internal_sub_theta"),
        )
        .unwrap();
    let gate = inner.to_gate("BadGate").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::from(Parameter::symbol("x"))],
            None,
        )
        .unwrap();

    let err = circuit_to_symbolic_matrix(&circuit, None).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_substitution_rejects_internal_prefix_in_input_matrix() {
    let theta = Parameter::symbol("__cqlib_internal_sub_theta");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::RX, &[theta]).unwrap();
    let replacements = HashMap::from([("theta".to_string(), Parameter::from(1.0))]);

    let result = substitute_symbolic_matrix(symbolic, &replacements);

    assert!(matches!(result, Err(CircuitError::InvalidOperation(_))));
}

#[test]
fn test_substitution_cross_dependency() {
    let mut inner = Circuit::new(1);
    inner
        .rx(
            Qubit::new(0),
            Parameter::symbol("a") + Parameter::symbol("b"),
        )
        .unwrap();
    let gate = inner.to_gate("AddGate").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [
                ParameterValue::from(Parameter::symbol("b")),
                ParameterValue::from(Parameter::symbol("a")),
            ],
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("a", 1.5);
    bindings.insert("b", 2.5);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let expected = StandardGate::RX.matrix(&[4.0]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-10);
}

#[test]
fn test_circuit_gate_storage_order_constant_symbolic_matrix() {
    for order in [[7, 2], [1, 0], [2, 7], [0, 1]] {
        let order = order.map(Qubit::new);
        let mut inner = Circuit::from_qubits(order.to_vec()).unwrap();
        inner.x(order[0]).unwrap();
        let gate = inner.to_gate("StoredOrderX").unwrap();
        for targets in [[4, 9], [9, 4]] {
            let targets = targets.map(Qubit::new);
            let mut circuit = Circuit::from_qubits(vec![Qubit::new(9), Qubit::new(4)]).unwrap();
            circuit.append(gate.clone(), targets, [], None).unwrap();
            let mut expected = Circuit::from_qubits(vec![Qubit::new(9), Qubit::new(4)]).unwrap();
            expected.x(targets[0]).unwrap();
            for output_order in [None, Some([9, 4].as_slice())] {
                let symbolic = circuit_to_symbolic_matrix(&circuit, output_order).unwrap();
                let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
                assert_matrix_approx_eq(
                    &evaluated,
                    &circuit_to_matrix(&expected, output_order).unwrap(),
                    1e-10,
                );
            }
        }
    }
}

#[test]
fn test_circuit_gate_storage_order_nested_parameterized_inverse() {
    for middle_order in [None, Some([0, 1]), Some([5, 1])] {
        let mut body = Circuit::from_qubits(vec![Qubit::new(7), Qubit::new(2)]).unwrap();
        body.rx(Qubit::new(7), Parameter::symbol("theta")).unwrap();
        body.cx(Qubit::new(7), Qubit::new(2)).unwrap();
        if let Some(order) = middle_order {
            let gate = body.to_gate("Inner").unwrap();
            body = Circuit::from_qubits(order.map(Qubit::new).to_vec()).unwrap();
            body.append(
                gate,
                order.map(Qubit::new),
                [ParameterValue::from(Parameter::symbol("theta"))],
                None,
            )
            .unwrap();
        }
        let mut outer = Circuit::new(2);
        outer
            .append(
                body.to_gate("Body").unwrap(),
                [Qubit::new(0), Qubit::new(1)],
                [ParameterValue::from(Parameter::symbol("x"))],
                None,
            )
            .unwrap();
        for inverse in [false, true] {
            let circuit = if inverse {
                outer.inverse().unwrap()
            } else {
                outer.clone()
            };
            let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
            for angle in [0.37, -0.61] {
                let bindings = Some(HashMap::from([("x", angle)]));
                let bound = circuit.assign_parameters(&bindings).unwrap();
                let mut expected = Circuit::new(2);
                expected.rx(Qubit::new(0), angle).unwrap();
                expected.cx(Qubit::new(0), Qubit::new(1)).unwrap();
                if inverse {
                    expected = expected.inverse().unwrap();
                }
                let expected_matrix = circuit_to_matrix(&expected, None).unwrap();
                assert_matrix_approx_eq(
                    &evaluate_symbolic_matrix(&symbolic, &bindings).unwrap(),
                    &expected_matrix,
                    1e-10,
                );
                assert_matrix_approx_eq(
                    &circuit_to_matrix(&bound, None).unwrap(),
                    &expected_matrix,
                    1e-10,
                );
                assert_matrix_approx_eq(
                    &circuit_to_matrix(&bound.decompose().unwrap(), None).unwrap(),
                    &expected_matrix,
                    1e-10,
                );
            }
        }
    }
}

#[test]
fn test_circuit_gate_storage_order_cache_isolation() {
    use crate::circuit::gate::CircuitGate;

    let mut inner = Circuit::from_qubits(vec![Qubit::new(7), Qubit::new(2)]).unwrap();
    inner.rx(Qubit::new(7), Parameter::symbol("theta")).unwrap();
    inner.cx(Qubit::new(7), Qubit::new(2)).unwrap();
    let frozen = FrozenCircuit::new(inner.clone());
    let gate = CircuitGate::new("Cached", frozen.clone()).unwrap();
    // Materializing either cache first must not determine the other's basis.
    for gate_first in [false, true] {
        let frozen = if gate_first {
            FrozenCircuit::new(inner.clone())
        } else {
            frozen.clone()
        };
        let gate = if gate_first {
            CircuitGate::new("Cached", frozen.clone()).unwrap()
        } else {
            gate.clone()
        };
        if gate_first {
            gate.symbolic_matrix().unwrap();
        } else {
            frozen.symbolic_matrix().unwrap();
        }
        let gate_matrix = gate.symbolic_matrix().unwrap();
        assert!(Arc::ptr_eq(
            &gate_matrix,
            &gate.clone().symbolic_matrix().unwrap()
        ));
        assert_eq!(frozen, FrozenCircuit::new(inner.clone()));
        let bindings = Some(HashMap::from([("theta", 0.37)]));
        let assigned = inner.assign_parameters(&bindings).unwrap();
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(&gate_matrix, &bindings).unwrap(),
            &circuit_to_matrix(&assigned, Some(&[7, 2])).unwrap(),
            1e-10,
        );
        let default_matrix = frozen.symbolic_matrix().unwrap();
        assert!(Arc::ptr_eq(
            &default_matrix,
            &frozen.symbolic_matrix().unwrap()
        ));
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(&default_matrix, &bindings).unwrap(),
            &circuit_to_matrix(&assigned, None).unwrap(),
            1e-10,
        );
        // Circuit-backed UnitaryGate keeps its existing numeric-sorted basis.
        let unitary = UnitaryGate::new("Frozen", 2, 1)
            .with_circuit(Arc::new(frozen))
            .unwrap();
        let mut unitary_circuit = Circuit::new(2);
        unitary_circuit
            .append(
                Instruction::UnitaryGate(Box::new(unitary)),
                [Qubit::new(0), Qubit::new(1)],
                [ParameterValue::from(Parameter::symbol("x"))],
                None,
            )
            .unwrap();
        let mut unitary_expected = Circuit::new(2);
        unitary_expected.rx(Qubit::new(1), 0.37).unwrap();
        unitary_expected.cx(Qubit::new(1), Qubit::new(0)).unwrap();
        let bindings = Some(HashMap::from([("x", 0.37)]));
        let expected = circuit_to_matrix(&unitary_expected, None).unwrap();
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(
                &circuit_to_symbolic_matrix(&unitary_circuit, None).unwrap(),
                &bindings,
            )
            .unwrap(),
            &expected,
            1e-10,
        );
        assert_matrix_approx_eq(
            &circuit_to_matrix(&unitary_circuit.assign_parameters(&bindings).unwrap(), None)
                .unwrap(),
            &expected,
            1e-10,
        );
        let mut circuit = Circuit::new(2);
        let mut expected = Circuit::new(2);
        for (targets, scale) in [([0, 1], 1.0), ([1, 0], 2.0)] {
            circuit
                .append(
                    Instruction::CircuitGate(Box::new(gate.clone())),
                    targets.map(Qubit::new),
                    [ParameterValue::from(Parameter::symbol("x") * scale)],
                    None,
                )
                .unwrap();
            expected.rx(Qubit::new(targets[0]), 0.37 * scale).unwrap();
            expected
                .cx(Qubit::new(targets[0]), Qubit::new(targets[1]))
                .unwrap();
        }
        let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(&symbolic, &bindings).unwrap(),
            &circuit_to_matrix(&expected, None).unwrap(),
            1e-10,
        );
    }
}

#[test]
fn test_frozen_circuit_symbolic_matrix_cache_reuses_arc() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let gate_instruction = inner.to_gate("CachedRx").unwrap();
    let Instruction::CircuitGate(gate) = gate_instruction else {
        panic!("to_gate should return CircuitGate");
    };

    let first = gate.symbolic_matrix().unwrap();
    let second = gate.symbolic_matrix().unwrap();

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn test_cached_circuit_gate_preserves_parameter_substitution() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let gate = inner.to_gate("CachedRx").unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .append(
            gate.clone(),
            [Qubit::new(0)],
            [ParameterValue::from(Parameter::symbol("x"))],
            None,
        )
        .unwrap();
    circuit
        .append(
            gate,
            [Qubit::new(0)],
            [ParameterValue::from(Parameter::symbol("x") * 2.0)],
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.23);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_symbolic_unitary_gate_numeric_matrix() {
    let x = array![
        [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
    ];
    let gate = UnitaryGate::new("XLike", 1, 0)
        .with_matrix(x.clone())
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(gate)),
            [Qubit::new(0)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();

    assert_matrix_approx_eq(&evaluated, &x, 1e-12);
}

#[test]
fn test_symbolic_unitary_gate_circuit_definition() {
    let mut inner = Circuit::new(1);
    inner.h(Qubit::new(0)).unwrap();
    let gate = UnitaryGate::new("HInner", 1, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(gate)),
            [Qubit::new(0)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_symbolic_parameterized_unitary_symbolic_matrix_with_fixed_param() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::Phase, &[Parameter::symbol("theta")])
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::Fixed(PI / 3.0)],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = crate::circuit::gate::gate_matrix::phase_gate(PI / 3.0);

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_symbolic_parameterized_unitary_symbolic_matrix_preserves_unbound_symbol() {
    let gate = UnitaryGate::new("CustomPhase", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::Phase, &[Parameter::symbol("theta")])
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::from(Parameter::symbol("theta"))],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", PI / 7.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = crate::circuit::gate::gate_matrix::phase_gate(PI / 7.0);

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_symbolic_parameterized_unitary_circuit_definition_preserves_symbol() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let gate = UnitaryGate::new("InnerRX", 1, 1)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::from(Parameter::symbol("phi"))],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("phi", PI / 4.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = crate::circuit::gate::gate_matrix::rx_gate(PI / 4.0);

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_symbolic_circuit_backed_unitary_ignores_unreferenced_interned_symbols() {
    let mut inner = Circuit::new(1);
    inner.add_parameter(Parameter::symbol("stale"));
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let frozen = FrozenCircuit::new(inner);
    assert_eq!(
        frozen
            .used_symbols()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["theta"]
    );

    let gate = UnitaryGate::new("InnerRX", 1, 1)
        .with_circuit(Arc::new(frozen))
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::from(Parameter::symbol("phi"))],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("phi", PI / 4.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = crate::circuit::gate::gate_matrix::rx_gate(PI / 4.0);

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_symbolic_unitary_explicit_symbolic_matrix_precedes_circuit() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let gate = UnitaryGate::new("CircuitBackedRX", 1, 1)
        .with_symbolic_matrix(["theta"], symbolic_eye(2))
        .unwrap()
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();

    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![ParameterValue::from(Parameter::symbol("phi"))],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("phi", PI / 5.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = Array2::eye(2).mapv(|v| Complex64::new(v, 0.0));

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_unitary_gate_circuit_2qubit_no_rev_bug() {
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let gate = UnitaryGate::new("CnotU", 2, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(gate)),
            [Qubit::new(1), Qubit::new(0)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();

    let mut expected_circuit = Circuit::new(2);
    expected_circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();
    let expected = circuit_to_matrix(&expected_circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_unitary_gate_circuit_3qubit_asymmetric() {
    let mut inner = Circuit::new(3);
    inner
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    let gate = UnitaryGate::new("CCXU", 3, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();

    let mut circuit = Circuit::new(3);
    circuit
        .append(
            Instruction::UnitaryGate(Box::new(gate)),
            [Qubit::new(2), Qubit::new(0), Qubit::new(1)],
            std::iter::empty(),
            None,
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[2, 0, 1])).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();

    let mut expected_circuit = Circuit::new(3);
    expected_circuit
        .ccx(Qubit::new(2), Qubit::new(0), Qubit::new(1))
        .unwrap();
    let expected = circuit_to_matrix(&expected_circuit, Some(&[2, 0, 1])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_unitary_gate_parameter_substitution_order() {
    // Inner circuit: RY(a) * RX(b)
    let mut inner = Circuit::new(1);
    inner.ry(Qubit::new(0), Parameter::symbol("a")).unwrap();
    inner.rx(Qubit::new(0), Parameter::symbol("b")).unwrap();
    let gate = UnitaryGate::new("OrderedU", 1, 2)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();

    // Outer call swaps: [y, x] maps a->y, b->x
    let mut circuit = Circuit::new(1);
    circuit
        .unitary_with_params(
            gate,
            vec![Qubit::new(0)],
            vec![
                ParameterValue::from(Parameter::symbol("y")),
                ParameterValue::from(Parameter::symbol("x")),
            ],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.7);
    bindings.insert("y", 0.2);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    // Expected: RY(0.2) then RX(0.7)
    let mut expected_circuit = Circuit::new(1);
    expected_circuit.ry(Qubit::new(0), 0.2).unwrap();
    expected_circuit.rx(Qubit::new(0), 0.7).unwrap();
    let expected = circuit_to_matrix(&expected_circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_mcgate_symbolic_matrix_matches_numeric() {
    let mut circuit = Circuit::new(4);
    circuit
        .multi_control(
            StandardGate::X,
            [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
            [Qubit::new(3)],
            [],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_mcgate_parametric_symbolic_matrix_matches_numeric() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(3);
    circuit
        .multi_control(
            StandardGate::RX,
            [Qubit::new(0), Qubit::new(1)],
            [Qubit::new(2)],
            [ParameterValue::from(theta.clone())],
        )
        .unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.55);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_mcgate_zero_added_controls_preserves_symbolic_controlled_rotations() {
    for gate in [StandardGate::CRX, StandardGate::CRY, StandardGate::CRZ] {
        let mut circuit = Circuit::new(2);
        circuit
            .append(
                Instruction::McGate(Box::new(MCGate::new(0, gate))),
                [Qubit::new(0), Qubit::new(1)],
                [ParameterValue::from(Parameter::symbol("theta"))],
                None,
            )
            .unwrap();
        let symbolic = circuit_to_symbolic_matrix(&circuit, Some(&[1, 0])).unwrap();
        assert_eq!(symbolic.dim(), (4, 4));
        for theta in [0.0, 0.37, -0.51, PI] {
            let bindings = HashMap::from([("theta", theta)]);
            let actual = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
            let expected = gate.matrix(&[theta]).unwrap();
            assert_matrix_approx_eq(&actual, expected.as_ref(), 1e-12);
        }
    }
}

#[test]
fn test_mcgate_inherent_controls_symbolic_matrix_matches_bound_numeric() {
    for gate in [StandardGate::CRX, StandardGate::CRY, StandardGate::CRZ] {
        for added_controls in 0..=2 {
            let mc_gate = MCGate::new(added_controls, gate);
            assert_eq!(mc_gate.num_ctrl_qubits(), added_controls as usize + 1);
            let num_qubits = mc_gate.num_qubits() + 1;
            let qubit_ids = [9usize, 2, 17, 5, 30][..num_qubits].to_vec();
            let qubits: Vec<Qubit> = qubit_ids.iter().map(|&id| Qubit::new(id as u32)).collect();
            // Leave a spectator between gate operands in the circuit's stored order.
            let operands: Vec<Qubit> = qubits
                .iter()
                .enumerate()
                .filter_map(|(index, &qubit)| (index != 1).then_some(qubit))
                .collect();
            for reverse in [false, true] {
                let mut operands = operands.clone();
                if reverse {
                    operands.reverse();
                }
                let mut circuit = Circuit::from_qubits(qubits.clone()).unwrap();
                circuit.h(operands[0]).unwrap();
                circuit.x(qubits[1]).unwrap();
                circuit
                    .append(
                        Instruction::McGate(Box::new(mc_gate.clone())),
                        operands.clone(),
                        [ParameterValue::from(Parameter::symbol("theta"))],
                        None,
                    )
                    .unwrap();
                circuit.rz(*operands.last().unwrap(), 0.23).unwrap();
                for order in [None, Some(qubit_ids.as_slice())] {
                    let symbolic = circuit_to_symbolic_matrix(&circuit, order).unwrap();
                    let dim = 1 << num_qubits;
                    assert_eq!(symbolic.dim(), (dim, dim));
                    for theta in [0.0, 0.37, -0.51, PI] {
                        let bindings = HashMap::from([("theta", theta)]);
                        let actual =
                            evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
                        let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
                        let expected = circuit_to_matrix(&bound, order).unwrap();
                        assert_matrix_approx_eq(&actual, &expected, 1e-12);
                        let constant_symbolic = circuit_to_symbolic_matrix(&bound, order).unwrap();
                        let constant = evaluate_symbolic_matrix(&constant_symbolic, &None).unwrap();
                        assert_matrix_approx_eq(&constant, &expected, 1e-12);
                    }
                }
            }
        }
    }
}

#[test]
fn test_numeric_path_matches_symbolic_path_for_no_param_gates() {
    // H, CX, X2P — all non-parametric gates should take the numeric fast path
    // and still produce the same result as the purely numeric circuit_to_matrix.
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.x2p(Qubit::new(2)).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_numeric_path_matches_symbolic_path_for_constant_params() {
    // RX, RY, RZ with concrete constants — numeric fast path should match.
    let mut circuit = Circuit::new(2);
    circuit.rx(Qubit::new(0), 0.42).unwrap();
    circuit.ry(Qubit::new(1), -0.17).unwrap();
    circuit.rz(Qubit::new(0), 1.23).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = circuit_to_matrix(&circuit, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_numeric_path_with_mixed_symbolic_and_constant_params() {
    // One symbolic parameter and one constant — only the symbolic gate should
    // take the slow path; the constant gates should still use the fast path.
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(1), theta.clone()).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.ry(Qubit::new(0), 0.31).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.73);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_apply_standard_gate_to_matrix_directly() {
    // Directly exercise apply_standard_gate_to_matrix for both paths.

    // Numeric path: H on qubit 0 of a 2-qubit system.
    let mut matrix = symbolic_eye(4);
    apply_standard_gate_to_matrix(&mut matrix, StandardGate::H, &[0], &[]).unwrap();
    let evaluated = evaluate_symbolic_matrix(&matrix, &None).unwrap();
    let expected = {
        let mut c = Circuit::new(2);
        c.h(Qubit::new(0)).unwrap();
        circuit_to_matrix(&c, None).unwrap()
    };
    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);

    // Symbolic path: RX(theta) on qubit 1.
    let theta = Parameter::symbol("theta");
    let mut matrix = symbolic_eye(4);
    apply_standard_gate_to_matrix(
        &mut matrix,
        StandardGate::RX,
        &[1],
        std::slice::from_ref(&theta),
    )
    .unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", PI / 4.0);
    let evaluated = evaluate_symbolic_matrix(&matrix, &Some(bindings.clone())).unwrap();
    let expected = {
        let mut c = Circuit::new(2);
        c.rx(Qubit::new(1), PI / 4.0).unwrap();
        circuit_to_matrix(&c, None).unwrap()
    };
    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_apply_gate_to_matrix_dimension_mismatch() {
    // 2x2 gate applied to 2 bits (should expect 4x4)
    let mut matrix = symbolic_eye(4);
    let gate = symbolic_eye(2);
    let err = apply_gate_to_matrix(&mut matrix, &gate, &[0, 1]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::QubitCountMismatch {
            expected: 1,
            actual: 2
        }
    ));

    // 4x4 gate applied to 1 bit (should expect 2x2)
    let mut matrix = symbolic_eye(2);
    let gate = symbolic_eye(4);
    let err = apply_gate_to_matrix(&mut matrix, &gate, &[0]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 1
        }
    ));
}

#[test]
fn test_apply_gate_to_matrix_rejects_duplicate_bits() {
    let mut matrix = symbolic_eye(4);
    let gate = symbolic_eye(4);
    let err = apply_gate_to_matrix(&mut matrix, &gate, &[0, 0]).unwrap_err();

    assert!(matches!(err, CircuitError::DuplicateQubits));
}

#[test]
fn test_apply_gate_to_matrix_rejects_out_of_range_bits() {
    let mut matrix = symbolic_eye(4);
    let gate = symbolic_eye(2);
    let err = apply_gate_to_matrix(&mut matrix, &gate, &[2]).unwrap_err();

    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_symbolic_rejects_duplicate_qubits_in_operation() {
    let mut circuit = Circuit::new(2);
    let err = circuit
        .append(
            Instruction::Standard(StandardGate::CX),
            [Qubit::new(0), Qubit::new(0)],
            [],
            None,
        )
        .unwrap_err();

    assert!(matches!(err, CircuitError::DuplicateQubits));
}

#[test]
fn test_symbolic_wrong_qubit_count_is_rejected_on_append() {
    let mut circuit = Circuit::new(2);
    let err = circuit
        .append(
            Instruction::Standard(StandardGate::CX),
            [Qubit::new(0)],
            [],
            None,
        )
        .unwrap_err();

    assert!(matches!(err, CircuitError::QubitCountMismatch { .. }));
    assert!(circuit.operations().is_empty());
}

#[test]
fn test_apply_standard_gate_to_matrix_dimension_mismatch() {
    // CX is a 2-qubit gate; applying it to 1 bit should fail
    let mut matrix = symbolic_eye(2);
    let err = apply_standard_gate_to_matrix(&mut matrix, StandardGate::CX, &[0], &[]).unwrap_err();
    assert!(matches!(err, CircuitError::QubitCountMismatch { .. }));
}

#[test]
fn test_serial_parallel_consistency_single_qubit() {
    let mut small = Circuit::new(1);
    small.h(Qubit::new(0)).unwrap();
    let small_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&small, None).unwrap(), &None)
            .unwrap();

    let mut large = Circuit::new(10);
    large.h(Qubit::new(5)).unwrap();
    let large_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&large, None).unwrap(), &None)
            .unwrap();

    let small_numeric = circuit_to_matrix(&small, None).unwrap();
    let large_numeric = circuit_to_matrix(&large, None).unwrap();

    assert_matrix_approx_eq(&small_eval, &small_numeric, 1e-10);
    assert_matrix_approx_eq(&large_eval, &large_numeric, 1e-10);
}

#[test]
fn test_serial_parallel_consistency_two_qubit() {
    let mut small = Circuit::new(2);
    small.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let small_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&small, None).unwrap(), &None)
            .unwrap();

    let mut large = Circuit::new(10);
    large.cx(Qubit::new(3), Qubit::new(7)).unwrap();
    let large_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&large, None).unwrap(), &None)
            .unwrap();

    let small_numeric = circuit_to_matrix(&small, None).unwrap();
    let large_numeric = circuit_to_matrix(&large, None).unwrap();

    assert_matrix_approx_eq(&small_eval, &small_numeric, 1e-10);
    assert_matrix_approx_eq(&large_eval, &large_numeric, 1e-10);
}

#[test]
fn test_serial_parallel_consistency_general_gate() {
    let mut small = Circuit::new(3);
    small
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    let small_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&small, None).unwrap(), &None)
            .unwrap();

    let mut large = Circuit::new(10);
    large
        .ccx(Qubit::new(2), Qubit::new(5), Qubit::new(8))
        .unwrap();
    let large_eval =
        evaluate_symbolic_matrix(&circuit_to_symbolic_matrix(&large, None).unwrap(), &None)
            .unwrap();

    let small_numeric = circuit_to_matrix(&small, None).unwrap();
    let large_numeric = circuit_to_matrix(&large, None).unwrap();

    assert_matrix_approx_eq(&small_eval, &small_numeric, 1e-10);
    assert_matrix_approx_eq(&large_eval, &large_numeric, 1e-10);
}

#[test]
fn test_random_small_circuit_consistency() {
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");

    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(1), theta.clone()).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.rz(Qubit::new(2), phi.clone()).unwrap();
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();
    circuit.ry(Qubit::new(0), 0.31).unwrap();

    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.73);
    bindings.insert("phi", -0.42);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();

    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = circuit_to_matrix(&bound, None).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}
