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
use crate::circuit::circuit_to_matrix;
use crate::circuit::gate::StandardGate;
use crate::circuit::symbolic_matrix::{
    SymbolicComplex, standard_gate_symbolic_matrix, symbolic_eye,
};
use crate::circuit::test_utils::assert_matrix_approx_eq;
use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::transform::decompose::expand_definitions;
use ndarray::array;
use num_complex::Complex64;

const fn c(re: f64, im: f64) -> Complex64 {
    Complex64::new(re, im)
}

use std::f64::consts::PI;
use std::sync::Arc;

#[test]
fn test_new_initializes_all_getters() {
    let gate = UnitaryGate::new("MyGate", 3, 2);
    assert_eq!(gate.label(), "MyGate");
    assert_eq!(gate.num_qubits(), 3);
    assert_eq!(gate.num_params(), 2);
    assert!(gate.matrix().is_none());
    assert!(gate.symbolic_matrix().is_none());
    assert!(gate.matrix_params().is_none());
    assert!(gate.circuit().is_none());
}

#[test]
fn test_with_matrix_success() {
    let h = c(1.0 / f64::sqrt(2.0), 0.0);
    let matrix = array![[h, h], [h, -h]];
    let gate = UnitaryGate::new("Hadamard", 1, 0)
        .with_matrix(matrix.clone())
        .unwrap();

    assert!(gate.matrix().is_some());
    let mat = gate.matrix().unwrap();
    assert_eq!(mat.shape(), &[2, 2]);
    assert_eq!(mat[[0, 0]], h);
    assert_eq!(mat[[0, 1]], h);
    assert_eq!(mat[[1, 0]], h);
    assert_eq!(mat[[1, 1]], -h);
}

#[test]
fn test_with_matrix_rejects_parameterized_gate() {
    let matrix = Array2::eye(2).mapv(|v| c(v, 0.0));
    let err = UnitaryGate::new("BadStatic", 1, 1)
        .with_matrix(matrix)
        .unwrap_err();

    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 0,
            actual: 1
        }
    ));
}

#[test]
fn test_with_matrix_rejects_wrong_dimensions() {
    // 1-qubit gate expects 2x2, but we give 4x4
    let matrix = Array2::eye(4).mapv(|v| c(v, 0.0));
    let err = UnitaryGate::new("WrongDim", 1, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));

    // 2-qubit gate expects 4x4, but we give 2x2
    let matrix = Array2::eye(2).mapv(|v| c(v, 0.0));
    let err = UnitaryGate::new("WrongDim", 2, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_with_matrix_rejects_non_square() {
    let matrix = array![
        [c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)],
    ];
    let err = UnitaryGate::new("NonSquare", 1, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_with_symbolic_matrix_success() {
    let gate = UnitaryGate::new("Rx", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();

    assert!(gate.symbolic_matrix().is_some());
    assert_eq!(gate.matrix_params().unwrap(), ["theta"]);
    assert!(gate.matrix().is_none());
}

#[test]
fn test_symbolic_matrix_rejects_wrong_shape() {
    let matrix = array![[SymbolicComplex::one()]];
    let err = UnitaryGate::new("BadSymbolic", 1, 1)
        .with_symbolic_matrix(["theta"], matrix)
        .unwrap_err();

    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_symbolic_matrix_rejects_param_count_mismatch() {
    let err = UnitaryGate::new("BadSymbolic", 1, 2)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap_err();

    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 2,
            actual: 1
        }
    ));
}

#[test]
fn test_symbolic_matrix_rejects_duplicate_params() {
    let err = UnitaryGate::new("BadSymbolic", 1, 2)
        .with_symbolic_matrix(
            ["theta", "theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap_err();

    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_symbolic_matrix_rejects_undeclared_symbol() {
    let gate = UnitaryGate::new("BadFactory", 1, 1).with_symbolic_matrix(
        ["phi"],
        standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
    );

    let err = gate.unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_matrix_for_params_returns_borrowed_for_static_matrix() {
    let matrix = Array2::eye(2).mapv(|v| c(v, 0.0));
    let gate = UnitaryGate::new("Id", 1, 0).with_matrix(matrix).unwrap();

    let result = gate.matrix_for_params(&[]).unwrap();
    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(result.nrows(), 2);
}

#[test]
fn test_matrix_for_params_returns_owned_for_symbolic_matrix() {
    let gate = UnitaryGate::new("Rx", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();

    let result = gate.matrix_for_params(&[PI]).unwrap();
    assert!(matches!(result, Cow::Owned(_)));
    // RX(pi) ~= -i X  => off-diagonals should be ~ -1, diagonals ~ 0
    assert!((result[[0, 0]].re).abs() < 1e-10);
    assert!((result[[0, 1]].re).abs() < 1e-10);
    assert!((result[[0, 1]].im + 1.0).abs() < 1e-10);
    assert!((result[[1, 0]].re).abs() < 1e-10);
    assert!((result[[1, 0]].im + 1.0).abs() < 1e-10);
    assert!((result[[1, 1]].re).abs() < 1e-10);
}

#[test]
fn test_matrix_for_params_circuit_backed_static() {
    let mut inner = Circuit::new(1);
    inner.h(Qubit::new(0)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let gate = UnitaryGate::new("HCircuit", 1, 0)
        .with_circuit(frozen)
        .unwrap();

    let result = gate.matrix_for_params(&[]).unwrap();
    assert!(matches!(result, Cow::Owned(_)));
    assert_eq!(result.shape(), &[2, 2]);

    let expected = 1.0 / f64::sqrt(2.0);
    assert!((result[[0, 0]].re - expected).abs() < 1e-10);
    assert!((result[[0, 1]].re - expected).abs() < 1e-10);
    assert!((result[[1, 0]].re - expected).abs() < 1e-10);
    assert!((result[[1, 1]].re + expected).abs() < 1e-10);
}

#[test]
fn test_matrix_for_params_circuit_backed_parameterized() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let gate = UnitaryGate::new("RxCircuit", 1, 1)
        .with_circuit(frozen)
        .unwrap();

    let result = gate.matrix_for_params(&[PI]).unwrap();
    assert!(matches!(result, Cow::Owned(_)));
    assert_eq!(result.shape(), &[2, 2]);
    // RX(pi) ~= -i X
    assert!((result[[0, 1]].im + 1.0).abs() < 1e-10);
    assert!((result[[1, 0]].im + 1.0).abs() < 1e-10);
}

#[test]
fn test_circuit_backed_parameter_binding_ignores_unreferenced_interned_symbols() {
    let mut inner = Circuit::new(1);
    inner.add_parameter(Parameter::symbol("stale"));
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let gate = UnitaryGate::new("RxCircuit", 1, 1)
        .with_circuit(frozen)
        .unwrap();
    let result = gate.matrix_for_params(&[PI]).unwrap();

    assert!((result[[0, 1]].im + 1.0).abs() < 1e-10);
    assert!((result[[1, 0]].im + 1.0).abs() < 1e-10);
}

#[test]
fn test_matrix_for_params_wrong_param_count() {
    let matrix = Array2::eye(2).mapv(|v| c(v, 0.0));
    let gate = UnitaryGate::new("Id", 1, 0).with_matrix(matrix).unwrap();

    // Too many params for a 0-param gate
    let err = gate.matrix_for_params(&[1.0]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 0,
            actual: 1
        }
    ));

    // Too few params for a parameterized symbolic gate
    let gate = UnitaryGate::new("Rx", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();

    let err = gate.matrix_for_params(&[]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        }
    ));
}

#[test]
fn test_matrix_for_params_rejects_nan() {
    let gate = UnitaryGate::new("Rx", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();

    let err = gate.matrix_for_params(&[f64::NAN]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::InvalidParameterValue(idx, val)
    if idx == 0 && val.is_nan()));
}

#[test]
fn test_matrix_for_params_rejects_infinity() {
    let gate = UnitaryGate::new("Rx", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();

    let err = gate.matrix_for_params(&[f64::INFINITY]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::InvalidParameterValue(idx, val)
    if idx == 0 && val.is_infinite()));

    let err = gate.matrix_for_params(&[f64::NEG_INFINITY]).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::InvalidParameterValue(idx, val)
    if idx == 0 && val.is_infinite()));
}

#[test]
fn test_matrix_for_params_no_representation() {
    let gate = UnitaryGate::new("Empty", 1, 0);
    let err = gate.matrix_for_params(&[]).unwrap_err();
    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_with_circuit_success() {
    let mut inner = Circuit::new(2);
    inner.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let gate = UnitaryGate::new("CNOT", 2, 0)
        .with_circuit(frozen.clone())
        .unwrap();
    assert!(gate.circuit().is_some());
    assert_eq!(gate.circuit().as_ref().unwrap().circuit().qubits().len(), 2);
}

#[test]
fn test_with_circuit_validates_signature() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("theta")).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let gate = UnitaryGate::new("CircuitBacked", 1, 1)
        .with_circuit(frozen.clone())
        .unwrap();
    assert_eq!(gate.num_params(), 1);

    let err = UnitaryGate::new("WrongParamCount", 1, 0)
        .with_circuit(frozen.clone())
        .unwrap_err();
    assert!(matches!(
        err,
        CircuitError::ParameterCountMismatch {
            expected: 0,
            actual: 1
        }
    ));

    let err = UnitaryGate::new("WrongQubitCount", 2, 1)
        .with_circuit(frozen)
        .unwrap_err();
    assert!(matches!(
        err,
        CircuitError::QubitCountMismatch {
            expected: 2,
            actual: 1
        }
    ));
}

#[test]
fn test_equality_based_on_uuid() {
    let gate_a = UnitaryGate::new("A", 1, 0);
    let gate_b = UnitaryGate::new("A", 1, 0);
    assert_ne!(
        gate_a, gate_b,
        "Two independently created gates must not be equal"
    );

    let gate_a_clone = gate_a.clone();
    assert_eq!(gate_a, gate_a_clone, "Clone must share the same identity");
}

#[test]
fn test_hash_consistent_with_equality() {
    use std::collections::hash_map::DefaultHasher;

    let gate = UnitaryGate::new("HashTest", 2, 3);
    let clone = gate.clone();

    let mut hasher_a = DefaultHasher::new();
    gate.hash(&mut hasher_a);
    let hash_a = hasher_a.finish();

    let mut hasher_b = DefaultHasher::new();
    clone.hash(&mut hasher_b);
    let hash_b = hasher_b.finish();

    assert_eq!(hash_a, hash_b);
}

// Formatting (Display, Debug)

#[test]
fn test_display_outputs_label() {
    let gate = UnitaryGate::new("FooBar", 1, 0);
    assert_eq!(format!("{}", gate), "FooBar");
}

#[test]
fn test_debug_contains_struct_name_and_fields() {
    let gate = UnitaryGate::new("DebugGate", 1, 0);
    let debug = format!("{:?}", gate);
    assert!(debug.starts_with("UnitaryGate {"));
    assert!(debug.contains("id"));
    assert!(debug.contains("DebugGate"));
    assert!(debug.contains("num_qubits: 1"));
    assert!(debug.contains("num_params: 0"));
    assert!(debug.contains("matrix: None"));
    assert!(debug.contains("matrix_params: None"));
    assert!(debug.contains("circuit: None"));
}

#[test]
fn test_debug_symbolic_matrix_shows_params() {
    let gate = UnitaryGate::new("ParamGate", 1, 1)
        .with_symbolic_matrix(
            ["theta"],
            standard_gate_symbolic_matrix(StandardGate::RX, &[Parameter::symbol("theta")]).unwrap(),
        )
        .unwrap();
    let debug = format!("{:?}", gate);
    assert!(debug.contains("matrix_params: Some([\"theta\"])"));
}

// A. Unitarity and numeric validation

#[test]
fn test_with_matrix_rejects_non_unitary() {
    // Dimension is correct (2x2) but not unitary: first row norm != 1
    let matrix = array![[c(2.0, 0.0), c(0.0, 0.0)], [c(0.0, 0.0), c(1.0, 0.0)],];
    let err = UnitaryGate::new("NonUnitary", 1, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_with_matrix_rejects_matrix_with_nan() {
    let matrix = array![[c(f64::NAN, 0.0), c(0.0, 0.0)], [c(0.0, 0.0), c(1.0, 0.0)],];
    let err = UnitaryGate::new("NanMat", 1, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_with_matrix_rejects_matrix_with_inf() {
    let matrix = array![
        [c(f64::INFINITY, 0.0), c(0.0, 0.0)],
        [c(0.0, 0.0), c(1.0, 0.0)],
    ];
    let err = UnitaryGate::new("InfMat", 1, 0)
        .with_matrix(matrix)
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_matrix_for_params_rejects_non_unitary_from_symbolic_matrix() {
    let matrix = array![
        [SymbolicComplex::from_real(2.0), SymbolicComplex::zero()],
        [SymbolicComplex::zero(), SymbolicComplex::one()],
    ];
    let gate = UnitaryGate::new("BadFactory", 1, 1)
        .with_symbolic_matrix(["theta"], matrix)
        .unwrap();
    let err = gate.matrix_for_params(&[0.0]).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
}

#[test]
fn test_matrix_for_params_rejects_invalid_symbolic_expression() {
    let invalid_at_evaluation = SymbolicComplex::from_real(Parameter::symbol("theta").sqrt());
    let matrix = array![
        [invalid_at_evaluation, SymbolicComplex::zero()],
        [SymbolicComplex::zero(), SymbolicComplex::one()],
    ];
    let gate = UnitaryGate::new("BadFactory", 1, 1)
        .with_symbolic_matrix(["theta"], matrix)
        .unwrap();
    let err = gate.matrix_for_params(&[-1.0]).unwrap_err();
    assert!(matches!(err, CircuitError::SymbolicParameterError));
}

#[test]
fn test_matrix_and_circuit_both_set_prefers_matrix() {
    let mut inner = Circuit::new(1);
    inner.x(Qubit::new(0)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));

    let matrix = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];
    let gate = UnitaryGate::new("Both", 1, 0)
        .with_matrix(matrix)
        .unwrap()
        .with_circuit(frozen)
        .unwrap();

    let result = gate.matrix_for_params(&[]).unwrap();
    assert!(matches!(result, Cow::Borrowed(_)));
}

#[test]
fn test_symbolic_matrix_on_zero_params_gate() {
    let gate = UnitaryGate::new("ZeroParam", 1, 0)
        .with_symbolic_matrix(std::iter::empty::<&str>(), symbolic_eye(2))
        .unwrap();
    let result = gate.matrix_for_params(&[]).unwrap();
    assert!(matches!(result, Cow::Owned(_)));
    assert_eq!(result[[0, 0]], c(1.0, 0.0));
    assert_eq!(result[[1, 1]], c(1.0, 0.0));
}

// C. Circuit-backed edge cases

#[test]
fn test_circuit_backed_with_measure_fails() {
    let mut inner = Circuit::new(1);
    inner.measure(Qubit::new(0)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));
    let gate = UnitaryGate::new("MeasureGate", 1, 0)
        .with_circuit(frozen)
        .unwrap();
    let err = gate.matrix_for_params(&[]).unwrap_err();
    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_circuit_backed_with_reset_fails() {
    let mut inner = Circuit::new(1);
    inner.reset(Qubit::new(0)).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));
    let gate = UnitaryGate::new("ResetGate", 1, 0)
        .with_circuit(frozen)
        .unwrap();
    let err = gate.matrix_for_params(&[]).unwrap_err();
    assert!(matches!(err, CircuitError::NoMatrixRepresentation));
}

#[test]
fn test_circuit_backed_global_phase_preserved() {
    let mut inner = Circuit::new(1);
    inner.x(Qubit::new(0)).unwrap();
    inner.set_global_phase((PI / 2.0).into());
    let frozen = Arc::new(FrozenCircuit::new(inner));
    let gate = UnitaryGate::new("PhaseX", 1, 0)
        .with_circuit(frozen)
        .unwrap();

    let result = gate.matrix_for_params(&[]).unwrap();
    // X with global phase PI/2 => i * X
    assert!((result[[0, 1]].re).abs() < 1e-10);
    assert!((result[[0, 1]].im - 1.0).abs() < 1e-10);
    assert!((result[[1, 0]].re).abs() < 1e-10);
    assert!((result[[1, 0]].im - 1.0).abs() < 1e-10);
    assert!((result[[0, 0]].norm()).abs() < 1e-10);
    assert!((result[[1, 1]].norm()).abs() < 1e-10);
}

#[test]
fn test_circuit_backed_multi_param_binding_order() {
    let mut inner = Circuit::new(1);
    inner.rx(Qubit::new(0), Parameter::symbol("alpha")).unwrap();
    inner.ry(Qubit::new(0), Parameter::symbol("beta")).unwrap();
    let frozen = Arc::new(FrozenCircuit::new(inner));
    let gate = UnitaryGate::new("MultiParam", 1, 2)
        .with_circuit(frozen)
        .unwrap();

    // alpha = PI/2, beta = 0 => RX(pi/2) then RY(0) = RX(pi/2)
    let result = gate.matrix_for_params(&[PI / 2.0, 0.0]).unwrap();
    let expected_cos = 1.0 / f64::sqrt(2.0);
    assert!((result[[0, 0]].re - expected_cos).abs() < 1e-10);
    assert!((result[[0, 0]].im).abs() < 1e-10);
    assert!((result[[0, 1]].re).abs() < 1e-10);
    assert!((result[[0, 1]].im + expected_cos).abs() < 1e-10);
    assert!((result[[1, 0]].re).abs() < 1e-10);
    assert!((result[[1, 0]].im + expected_cos).abs() < 1e-10);
    assert!((result[[1, 1]].re - expected_cos).abs() < 1e-10);
    assert!((result[[1, 1]].im).abs() < 1e-10);
}

// D. Multi-qubit expansion and integration

#[test]
fn test_three_qubit_unitary_matrix_expansion() {
    let mat = Array2::eye(8).mapv(|v| c(v, 0.0));
    let u_gate = UnitaryGate::new("Id3", 3, 0).with_matrix(mat).unwrap();

    let mut circuit = Circuit::new(3);
    circuit
        .unitary(u_gate, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)])
        .unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_eq!(matrix.shape(), &[8, 8]);
    for i in 0..8 {
        for j in 0..8 {
            if i == j {
                assert!((matrix[[i, j]] - c(1.0, 0.0)).norm() < 1e-10);
            } else {
                assert!(matrix[[i, j]].norm() < 1e-10);
            }
        }
    }
}

#[test]
fn test_custom_unitary_in_composite_circuit() {
    let mat = array![[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)],];
    let u_gate = UnitaryGate::new("CustomX", 1, 0).with_matrix(mat).unwrap();

    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.unitary(u_gate, vec![Qubit::new(1)]).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let matrix = circuit_to_matrix(&circuit, None).unwrap();
    assert_eq!(matrix.shape(), &[4, 4]);

    // Verify the circuit is unitary
    let conj_t = matrix.t().mapv(|x| x.conj());
    let product = conj_t.dot(&matrix);
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { c(1.0, 0.0) } else { c(0.0, 0.0) };
            assert!((product[[i, j]] - expected).norm() < 1e-10);
        }
    }
}

/// Check complete operators, including spectator qubits and nested CircuitGate
/// definitions, against an independently constructed positional reference.
fn assert_circuit_definition_order(transform: impl Fn(&Circuit) -> Circuit) {
    let mut inner = Circuit::from_qubits(vec![Qubit::new(2), Qubit::new(0)]).unwrap();
    inner.x(Qubit::new(2)).unwrap();
    let gate = UnitaryGate::new("asymmetric_x", 2, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let mut expected = Circuit::new(2);
    expected.x(Qubit::new(1)).unwrap();
    let expected = circuit_to_matrix(&expected, None).unwrap();
    assert_matrix_approx_eq(
        &circuit_to_matrix(&circuit, None).unwrap(),
        &expected,
        1e-12,
    );
    assert_matrix_approx_eq(
        &circuit_to_matrix(&transform(&circuit), None).unwrap(),
        &expected,
        1e-12,
    );

    let mut leaf = Circuit::from_qubits(vec![Qubit::new(4), Qubit::new(1)]).unwrap();
    leaf.h(Qubit::new(4)).unwrap();
    leaf.cx(Qubit::new(4), Qubit::new(1)).unwrap();
    let leaf = leaf.to_gate("storage_order_leaf").unwrap();

    for order in [[9, 2, 5], [5, 9, 2], [2, 5, 9]] {
        let mut inner = Circuit::from_qubits(order.map(Qubit::new).to_vec()).unwrap();
        inner.rx(Qubit::new(9), 0.37).unwrap();
        inner.ry(Qubit::new(2), -0.51).unwrap();
        inner.cx(Qubit::new(5), Qubit::new(9)).unwrap();
        inner.cx(Qubit::new(9), Qubit::new(2)).unwrap();
        inner.rz(Qubit::new(5), 0.29).unwrap();
        inner
            .append(leaf.clone(), [Qubit::new(9), Qubit::new(5)], [], None)
            .unwrap();
        // Explicit matrices use MSB-first qargs. Reverse the definition's
        // numeric order when also supplying an equivalent matrix representation.
        let explicit_matrix = circuit_to_matrix(&inner, Some(&[9, 5, 2])).unwrap();
        let gate = UnitaryGate::new("numeric_order_unitary", 3, 0)
            .with_circuit(Arc::new(FrozenCircuit::new(inner)))
            .unwrap();
        for with_matrix in [false, true] {
            let gate = if with_matrix {
                gate.clone().with_matrix(explicit_matrix.clone()).unwrap()
            } else {
                gate.clone()
            };
            for args in [
                [0, 2, 3],
                [0, 3, 2],
                [2, 0, 3],
                [2, 3, 0],
                [3, 0, 2],
                [3, 2, 0],
            ] {
                let [a, b, c] = args.map(Qubit::new);
                let mut circuit = Circuit::new(4);
                circuit.unitary(gate.clone(), vec![a, b, c]).unwrap();
                let mut expected = Circuit::new(4);
                expected.rx(c, 0.37).unwrap();
                expected.ry(a, -0.51).unwrap();
                expected.cx(b, c).unwrap();
                expected.cx(c, a).unwrap();
                expected.rz(b, 0.29).unwrap();
                expected.h(c).unwrap();
                expected.cx(c, b).unwrap();
                let expected = circuit_to_matrix(&expected, None).unwrap();
                assert_matrix_approx_eq(
                    &circuit_to_matrix(&circuit, None).unwrap(),
                    &expected,
                    1e-12,
                );
                assert_matrix_approx_eq(
                    &circuit_to_matrix(&transform(&circuit), None).unwrap(),
                    &expected,
                    1e-12,
                );
            }
        }
    }
}

#[test]
fn circuit_definition_order_survives_expansion() {
    assert_circuit_definition_order(|circuit| expand_definitions(circuit).unwrap());
}

#[test]
fn circuit_definition_order_survives_qasm2() {
    assert_circuit_definition_order(|circuit| {
        crate::ir::qasm2::load::loads(&crate::ir::qasm2::dump::dumps(circuit).unwrap()).unwrap()
    });
}

#[test]
fn circuit_definition_order_survives_qasm3() {
    assert_circuit_definition_order(|circuit| {
        crate::ir::qasm3::load::loads(&crate::ir::qasm3::dump::dumps(circuit).unwrap()).unwrap()
    });
}

#[test]
fn circuit_definition_order_survives_compilation() {
    use crate::compile::resource::ResourcePolicy;
    use crate::compile::{CompileConfig, CompileMode, CompileTarget, compile};
    for mode in [CompileMode::Normal, CompileMode::Enhanced] {
        assert_circuit_definition_order(|circuit| {
            compile(
                circuit,
                CompileConfig {
                    mode,
                    target: CompileTarget::Logical,
                    resource_policy: ResourcePolicy::default(),
                },
            )
            .unwrap()
            .circuit
        });
    }
}

#[test]
fn circuit_definition_order_preserves_device_layout_and_basis() {
    use crate::compile::resource::ResourcePolicy;
    use crate::compile::{CompileConfig, CompileMode, CompileTarget, DeviceCompileTarget, compile};
    use crate::device::{Device, Layout};

    let mut inner = Circuit::from_qubits(vec![Qubit::new(2), Qubit::new(0)]).unwrap();
    inner.x(Qubit::new(2)).unwrap();
    inner.cx(Qubit::new(2), Qubit::new(0)).unwrap();
    let gate = UnitaryGate::new("device_unitary", 2, 0)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .unitary(gate, vec![Qubit::new(0), Qubit::new(1)])
        .unwrap();
    let device = Device::line("unitary-ports", 2)
        .unwrap()
        .with_native_gates(vec![
            StandardGate::X.into(),
            StandardGate::H.into(),
            StandardGate::CX.into(),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 1), (1, 0)], 2).unwrap();
    // Internal 2 -> argument 1 -> physical 0. The CX direction is native on
    // the device even though its logical arguments are in reverse order.
    let mut expected = Circuit::new(2);
    expected.x(Qubit::new(0)).unwrap();
    expected.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    for mode in [CompileMode::Normal, CompileMode::Enhanced] {
        let result = compile(
            &circuit,
            CompileConfig {
                mode,
                target: CompileTarget::Device(DeviceCompileTarget {
                    device: device.clone(),
                    initial_layout: Some(layout.clone()),
                    seed: Some(47),
                }),
                resource_policy: ResourcePolicy::default(),
            },
        )
        .unwrap();
        device.validate_circuit(&result.circuit).unwrap();
        let metadata = result.device_metadata.unwrap();
        assert_eq!(metadata.initial_layout, layout);
        assert_eq!(metadata.final_layout, layout);
        assert_matrix_approx_eq(
            &circuit_to_matrix(&result.circuit, None).unwrap(),
            &circuit_to_matrix(&expected, None).unwrap(),
            1e-9,
        );
    }
}

#[test]
fn circuit_definition_order_preserves_nested_symbolic_parameters_and_phase() {
    use crate::circuit::symbolic_matrix::{circuit_to_symbolic_matrix, evaluate_symbolic_matrix};
    let mut inner = Circuit::from_qubits(vec![Qubit::new(2), Qubit::new(0)]).unwrap();
    inner.rx(Qubit::new(2), Parameter::symbol("theta")).unwrap();
    inner.cx(Qubit::new(2), Qubit::new(0)).unwrap();
    inner.set_global_phase(Parameter::symbol("theta") * 0.25);
    let gate = UnitaryGate::new("symbolic_unitary", 2, 1)
        .with_circuit(Arc::new(FrozenCircuit::new(inner)))
        .unwrap();
    let mut middle = Circuit::from_qubits(vec![Qubit::new(7), Qubit::new(3)]).unwrap();
    middle
        .unitary_with_params(
            gate,
            vec![Qubit::new(3), Qubit::new(7)],
            [Parameter::symbol("alpha").into()],
        )
        .unwrap();
    let wrapper = middle.to_gate("storage_order_wrapper").unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .append(
            wrapper,
            [Qubit::new(2), Qubit::new(0)],
            [crate::circuit::ParameterValue::Param(
                Parameter::symbol("t") + 0.1_f64,
            )],
            None,
        )
        .unwrap();
    let mut expected = Circuit::new(3);
    expected
        .rx(Qubit::new(2), Parameter::symbol("t") + 0.1)
        .unwrap();
    expected.cx(Qubit::new(2), Qubit::new(0)).unwrap();
    expected.set_global_phase((Parameter::symbol("t") + 0.1) * 0.25);

    // Build the nested symbolic matrix before expansion, exercising numeric-order
    // UnitaryGate and storage-order CircuitGate caches in the same expression.
    let symbolic = circuit_to_symbolic_matrix(&circuit, None).unwrap();
    let expanded = expand_definitions(&circuit).unwrap();
    let expanded_symbolic = circuit_to_symbolic_matrix(&expanded, None).unwrap();
    for value in [0.0, 0.37, -0.51] {
        let bindings = Some(HashMap::from([("t", value)]));
        let expected =
            circuit_to_matrix(&expected.assign_parameters(&bindings).unwrap(), None).unwrap();
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(&symbolic, &bindings).unwrap(),
            &expected,
            1e-12,
        );
        assert_matrix_approx_eq(
            &evaluate_symbolic_matrix(&expanded_symbolic, &bindings).unwrap(),
            &expected,
            1e-12,
        );
        let bound = circuit.assign_parameters(&bindings).unwrap();
        assert_matrix_approx_eq(&circuit_to_matrix(&bound, None).unwrap(), &expected, 1e-12);
        assert_matrix_approx_eq(
            &circuit_to_matrix(&expand_definitions(&bound).unwrap(), None).unwrap(),
            &expected,
            1e-12,
        );
    }
}
