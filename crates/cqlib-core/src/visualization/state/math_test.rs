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
use crate::qis::Statevector;
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
