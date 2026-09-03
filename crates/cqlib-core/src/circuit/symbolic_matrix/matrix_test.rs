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
use crate::circuit::gate::StandardGate;
use crate::circuit::symbolic_matrix::gate::standard_gate_symbolic_matrix;
use crate::circuit::symbolic_matrix::test_utils::assert_matrix_approx_eq;
use crate::circuit::{Circuit, Parameter, Qubit};
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::HashMap;
use std::f64::consts::PI;

// --- SymbolicComplex tests ---

#[test]
fn test_symbolic_complex_zero() {
    let z = SymbolicComplex::zero();
    assert!(z.is_zero_exact());
    assert!(!z.is_one_exact());
}

#[test]
fn test_symbolic_complex_one() {
    let o = SymbolicComplex::one();
    assert!(o.is_one_exact());
    assert!(!o.is_zero_exact());
}

#[test]
fn test_symbolic_complex_i() {
    let i = SymbolicComplex::i();
    assert!(!i.is_zero_exact());
    assert!(!i.is_one_exact());
}

#[test]
fn test_symbolic_complex_exp_i() {
    let z = SymbolicComplex::exp_i(PI / 4.0);
    let evaluated = z.evaluate(&None).unwrap();
    assert!((evaluated.re - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-10);
    assert!((evaluated.im - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-10);
}

#[test]
fn test_symbolic_complex_from_real() {
    let z = SymbolicComplex::from_real(2.5);
    assert!(!z.is_zero_exact());
    assert!(z.im.is_zero());
    let evaluated = z.evaluate(&None).unwrap();
    assert!((evaluated.re - 2.5).abs() < 1e-12);
    assert!(evaluated.im.abs() < 1e-12);
}

#[test]
fn test_symbolic_complex_from_complex() {
    let c = Complex64::new(1.0, 2.0);
    let z = SymbolicComplex::from_complex(c);
    let evaluated = z.evaluate(&None).unwrap();
    assert!((evaluated.re - 1.0).abs() < 1e-12);
    assert!((evaluated.im - 2.0).abs() < 1e-12);
}

#[test]
fn test_symbolic_complex_simplifies_to_zero() {
    let zero = SymbolicComplex::zero();
    assert!(zero.simplifies_to_zero().unwrap());

    let nonzero = SymbolicComplex::one();
    assert!(!nonzero.simplifies_to_zero().unwrap());
}

#[test]
fn test_symbolic_complex_replace() {
    let theta = Parameter::symbol("theta");
    let z = SymbolicComplex::new(theta.clone(), theta);
    let replaced = z.replace("theta", Parameter::from(1.0));
    let evaluated = replaced.evaluate(&None).unwrap();
    assert!((evaluated.re - 1.0).abs() < 1e-12);
    assert!((evaluated.im - 1.0).abs() < 1e-12);
}

#[test]
fn test_symbolic_complex_arithmetic() {
    let a = SymbolicComplex::new(1.0, 2.0);
    let b = SymbolicComplex::new(3.0, 4.0);

    let sum = &a + &b;
    let sum_eval = sum.evaluate(&None).unwrap();
    assert!((sum_eval.re - 4.0).abs() < 1e-12);
    assert!((sum_eval.im - 6.0).abs() < 1e-12);

    let diff = &a - &b;
    let diff_eval = diff.evaluate(&None).unwrap();
    assert!((diff_eval.re - (-2.0)).abs() < 1e-12);
    assert!((diff_eval.im - (-2.0)).abs() < 1e-12);

    let prod = &a * &b;
    let prod_eval = prod.evaluate(&None).unwrap();
    // (1+2i)(3+4i) = 3+4i+6i+8i² = 3-8 + 10i = -5+10i
    assert!((prod_eval.re - (-5.0)).abs() < 1e-12);
    assert!((prod_eval.im - 10.0).abs() < 1e-12);

    let neg = -&a;
    let neg_eval = neg.evaluate(&None).unwrap();
    assert!((neg_eval.re - (-1.0)).abs() < 1e-12);
    assert!((neg_eval.im - (-2.0)).abs() < 1e-12);
}

#[test]
fn test_symbolic_complex_complex64_mul() {
    let a = SymbolicComplex::new(1.0, 2.0);
    let c = Complex64::new(3.0, 4.0);

    let prod = &a * c;
    let prod_eval = prod.evaluate(&None).unwrap();
    assert!((prod_eval.re - (-5.0)).abs() < 1e-12);
    assert!((prod_eval.im - 10.0).abs() < 1e-12);

    let prod2 = c * &a;
    assert!((prod2.evaluate(&None).unwrap().re - (-5.0)).abs() < 1e-12);
}

#[test]
fn test_symbolic_complex_display() {
    let real = SymbolicComplex::from_real(2.5);
    assert!(!real.to_string().contains('i'));

    let imag = SymbolicComplex::new(0.0, 2.0);
    assert!(imag.to_string().contains('i'));

    let mixed = SymbolicComplex::new(1.0, 2.0);
    let s = mixed.to_string();
    assert!(s.contains('+'));
    assert!(s.contains('i'));
}

// --- evaluate_symbolic_matrix tests ---

#[test]
fn test_evaluate_symbolic_matrix_identity() {
    let eye = symbolic_eye(4);
    let evaluated = evaluate_symbolic_matrix(&eye, &None).unwrap();
    let expected = Array2::eye(4);
    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_evaluate_symbolic_matrix_with_bindings() {
    let theta = Parameter::symbol("theta");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::RX, &[theta]).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("theta", PI / 2.0);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings)).unwrap();
    let expected = StandardGate::RX.matrix(&[PI / 2.0]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-12);
}

// --- substitute_symbolic_matrix tests ---

#[test]
fn test_substitute_symbolic_matrix_empty_replacements() {
    let eye = symbolic_eye(2);
    let result = substitute_symbolic_matrix(eye, &HashMap::new()).unwrap();
    let evaluated = evaluate_symbolic_matrix(&result, &None).unwrap();
    let expected = Array2::eye(2);
    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}

#[test]
fn test_substitute_symbolic_matrix_single_replacement() {
    let theta = Parameter::symbol("theta");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::RX, &[theta]).unwrap();
    let replacements = HashMap::from([("theta".to_string(), Parameter::from(PI / 2.0))]);
    let substituted = substitute_symbolic_matrix(symbolic, &replacements).unwrap();
    let evaluated = evaluate_symbolic_matrix(&substituted, &None).unwrap();
    let expected = StandardGate::RX.matrix(&[PI / 2.0]).unwrap();
    assert_matrix_approx_eq(&evaluated, expected.as_ref(), 1e-12);
}

#[test]
fn test_substitute_symbolic_matrix_collision_detection() {
    let theta = Parameter::symbol("__cqlib_internal_sub_theta");
    let symbolic = standard_gate_symbolic_matrix(StandardGate::RX, &[theta]).unwrap();
    let replacements = HashMap::from([(
        "__cqlib_internal_sub_theta".to_string(),
        Parameter::from(1.0),
    )]);
    let result = substitute_symbolic_matrix(symbolic, &replacements);
    assert!(result.is_err());
}

// --- Fast-path (diagonal / permutation) tests ---

#[test]
fn test_diagonal_fast_path_matches_numeric_matrix() {
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(3);
    circuit.rz(Qubit::new(0), theta.clone()).unwrap();
    circuit.phase(Qubit::new(1), theta.clone() * 2.0).unwrap();
    circuit.crz(Qubit::new(1), Qubit::new(2), theta).unwrap();

    let symbolic =
        crate::circuit::symbolic_matrix::circuit_to_symbolic_matrix(&circuit, Some(&[2, 0, 1]))
            .unwrap();

    let mut bindings = HashMap::new();
    bindings.insert("theta", 0.37);
    let evaluated = evaluate_symbolic_matrix(&symbolic, &Some(bindings.clone())).unwrap();
    let bound = circuit.assign_parameters(&Some(bindings)).unwrap();
    let expected = crate::circuit::circuit_to_matrix(&bound, Some(&[2, 0, 1])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-10);
}

#[test]
fn test_permutation_fast_path_matches_numeric_matrix() {
    let mut circuit = Circuit::new(4);
    circuit.x(Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(3)).unwrap();
    circuit.swap(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit
        .ccx(Qubit::new(3), Qubit::new(1), Qubit::new(0))
        .unwrap();

    let symbolic =
        crate::circuit::symbolic_matrix::circuit_to_symbolic_matrix(&circuit, Some(&[3, 1, 0, 2]))
            .unwrap();
    let evaluated = evaluate_symbolic_matrix(&symbolic, &None).unwrap();
    let expected = crate::circuit::circuit_to_matrix(&circuit, Some(&[3, 1, 0, 2])).unwrap();

    assert_matrix_approx_eq(&evaluated, &expected, 1e-12);
}
