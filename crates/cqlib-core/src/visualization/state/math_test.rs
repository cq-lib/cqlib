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

//! Tests for state visualization math helpers.

use super::math::pauli_coefficients;
use super::*;
use crate::qis::pauli::PauliString;
use crate::qis::{DensityMatrix, Statevector};
use num_complex::Complex64;

fn plus_state() -> Statevector {
    Statevector::from_state(
        1,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        ],
    )
    .unwrap()
}

fn plus_i_state() -> Statevector {
    Statevector::from_state(
        1,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(0.0, 1.0 / 2.0_f64.sqrt()),
        ],
    )
    .unwrap()
}

#[test]
fn bloch_vectors_for_basic_states() {
    let zero = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
        .unwrap();
    assert!((local_bloch_vectors(&zero).unwrap()[0].1[2] - 1.0).abs() < 1e-10);
    assert!((local_bloch_vectors(&plus_state()).unwrap()[0].1[0] - 1.0).abs() < 1e-10);
    assert!((local_bloch_vectors(&plus_i_state()).unwrap()[0].1[1] - 1.0).abs() < 1e-10);
}

#[test]
fn bell_state_has_zero_local_bloch_vectors() {
    let bell = Statevector::from_state(
        2,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        ],
    )
    .unwrap();
    for (_, vector) in local_bloch_vectors(&bell).unwrap() {
        assert!(vector.iter().all(|value| value.abs() < 1e-10));
    }
}

#[test]
fn paulivec_contains_expected_plus_x() {
    let (_, rho) = state_to_density_matrix(&plus_state()).unwrap();
    let coeffs = pauli_coefficients(1, &rho, false).unwrap();
    assert!(
        coeffs
            .iter()
            .any(|(label, value)| label == "X" && (*value - 1.0).abs() < 1e-10)
    );
}

#[test]
fn paulivec_contains_expected_plus_y() {
    let (_, rho) = state_to_density_matrix(&plus_i_state()).unwrap();
    let coeffs = pauli_coefficients(1, &rho, false).unwrap();
    assert!(
        coeffs
            .iter()
            .any(|(label, value)| label == "Y" && (*value - 1.0).abs() < 1e-10)
    );
}

#[test]
fn paulivec_labels_identify_the_correct_qubit() {
    // q0 = |+>, q1 = |0>: IX measures q0, XI measures q1.
    let state = Statevector::from_state(
        2,
        vec![
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    )
    .unwrap();
    let (_, rho) = state_to_density_matrix(&state).unwrap();
    for reverse_bits in [false, true] {
        let coeffs = pauli_coefficients(2, &rho, reverse_bits).unwrap();
        for (label, expected) in [("IX", 1.0), ("XI", 0.0), ("IZ", 0.0), ("ZI", 1.0)] {
            let displayed = if reverse_bits {
                label.chars().rev().collect::<String>()
            } else {
                label.to_string()
            };
            let value = coeffs.iter().find(|(s, _)| *s == displayed).unwrap().1;
            assert!((value - expected).abs() < 1e-10, "{displayed}: {value}");
        }
    }
}

fn assert_paulivec_matches_observables(
    state: &impl StateVisualizationSource,
    expectation: impl Fn(&PauliString) -> f64,
) {
    let (num_qubits, rho) = state_to_density_matrix(state).unwrap();
    for reverse_bits in [false, true] {
        let coeffs = pauli_coefficients(num_qubits, &rho, reverse_bits).unwrap();
        assert_eq!(coeffs.len(), 4usize.pow(num_qubits as u32));
        for (label, value) in coeffs {
            let semantic_label = if reverse_bits {
                label.chars().rev().collect::<String>()
            } else {
                label.clone()
            };
            let pauli: PauliString = semantic_label.parse().unwrap();
            let expected = expectation(&pauli);
            assert!(
                (value - expected).abs() < 1e-10,
                "{num_qubits} qubits, reverse_bits={reverse_bits}, {label}: {value} != {expected}"
            );
        }
    }
}

#[test]
fn paulivec_coefficients_match_observables_for_pure_and_mixed_states() {
    for num_qubits in 1..=3 {
        let dim = 1 << num_qubits;
        // Asymmetric complex amplitudes exercise X, Y, Z and negative coefficients.
        let mut amplitudes: Vec<_> = (0..dim)
            .map(|i| Complex64::new(((i + 1) as f64 * 0.71).sin(), ((i + 2) as f64 * 0.43).cos()))
            .collect();
        let norm = amplitudes.iter().map(|a| a.norm_sqr()).sum::<f64>().sqrt();
        for amplitude in &mut amplitudes {
            *amplitude /= norm;
        }
        let pure = Statevector::from_state(num_qubits, amplitudes).unwrap();
        assert_paulivec_matches_observables(&pure, |p| pure.expectation(p).unwrap());

        // A convex mixture with |11...1><11...1| is a genuine mixed state.
        let (_, mut rho) = state_to_density_matrix(&pure).unwrap();
        for value in &mut rho {
            *value *= 0.7;
        }
        rho[dim * dim - 1] += Complex64::new(0.3, 0.0);
        let mixed = DensityMatrix::from_density_matrix_state(num_qubits, rho).unwrap();
        assert_paulivec_matches_observables(&mixed, |p| mixed.expectation(p).unwrap());
    }
}
