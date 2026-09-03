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

use super::{ExtrapolateMethod, ZNEMitigation};
use crate::circuit::CircuitError;
use crate::circuit::Qubit;
use crate::circuit::circuit_impl::Circuit;
use crate::circuit::gate::{Instruction, StandardGate};
use crate::error_mitigation::ErrorMitigationError;
use crate::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

fn single_qubit_z_hamiltonian() -> Hamiltonian {
    let mut pauli_string = PauliString::new(1);
    pauli_string.set_pauli(0, Pauli::Z);
    Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("single-qubit Z Hamiltonian should be valid")
}

fn estimator_hmat(
    circuit: &Circuit,
    hamiltonian_arg: Option<&Hamiltonian>,
    shot_number: Option<usize>,
) -> (f64, f64) {
    assert_eq!(shot_number, None);
    let hamiltonian = hamiltonian_arg.expect("ZNE estimator should receive a Hamiltonian");
    let c_mat = circuit.to_matrix(None).unwrap();
    let h_mat = hamiltonian
        .to_matrix()
        .expect("ZNE estimator should receive dimensionally valid Hamiltonian terms");
    let dim = c_mat.nrows();

    assert_eq!(
        c_mat.ncols(),
        dim,
        "circuit matrix must be square, got {}x{}",
        dim,
        c_mat.ncols()
    );
    assert_eq!(
        h_mat.nrows(),
        dim,
        "hamiltonian row dimension mismatch: expected {}, got {}",
        dim,
        h_mat.nrows()
    );
    assert_eq!(
        h_mat.ncols(),
        dim,
        "hamiltonian column dimension mismatch: expected {}, got {}",
        dim,
        h_mat.ncols()
    );

    let psi = c_mat.column(0).to_owned();
    let mut expectation = Complex64::new(0.0, 0.0);
    for i in 0..dim {
        for j in 0..dim {
            expectation += psi[i].conj() * h_mat[(i, j)] * psi[j];
        }
    }

    (expectation.re, 0.0)
}

#[test]
fn test_zne_new_sets_noise_factors() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 3]);

    assert_eq!(zne.fold_levels(), &[0, 1, 3]);
    assert_eq!(zne.noise_factors(), &[1, 3, 7]);
}

#[test]
fn test_fold_circuits_global_level_one() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.s(q0).unwrap();

    let zne = ZNEMitigation::new(circuit, vec![1]);
    let folded = zne.fold_circuits(None).unwrap();

    assert_eq!(folded.len(), 1);
    let ops = folded[0].operations();
    assert_eq!(ops.len(), 3);
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::S)
    ));
    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::SDG)
    ));
    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::S)
    ));
}

#[test]
fn test_fold_circuits_selective_gate_set() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();
    circuit.s(q0).unwrap();

    let zne = ZNEMitigation::new(circuit, vec![1]);
    let gate_set = vec![Instruction::Standard(StandardGate::S)];
    let folded = zne.fold_circuits(Some(&gate_set)).unwrap();

    assert_eq!(folded.len(), 1);
    let ops = folded[0].operations();
    assert_eq!(ops.len(), 4);
    assert!(matches!(
        ops[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        ops[1].instruction,
        Instruction::Standard(StandardGate::S)
    ));
    assert!(matches!(
        ops[2].instruction,
        Instruction::Standard(StandardGate::SDG)
    ));
    assert!(matches!(
        ops[3].instruction,
        Instruction::Standard(StandardGate::S)
    ));
}

#[test]
fn test_fold_circuits_negative_level_returns_error() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![-1]);
    let result = zne.fold_circuits(None);

    assert!(matches!(
        result,
        Err(CircuitError::InvalidControlOperation(_))
    ));
}

#[test]
fn test_run_em_sequence_with_matrix_estimator() {
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();

    let zne = ZNEMitigation::new(circuit, vec![0]);
    let h = single_qubit_z_hamiltonian();

    let values = zne.run_em_sequence(None, &h, &estimator_hmat).unwrap();
    assert_eq!(values.len(), 1);
    assert!((values[0] + 1.0).abs() < 1e-10);
}

#[test]
fn test_run_em_sequence_with_custom_hexp_calc() {
    fn custom_hexp(
        _circuit: &Circuit,
        hamiltonian_arg: Option<&Hamiltonian>,
        shot_number: Option<usize>,
    ) -> (f64, f64) {
        assert!(hamiltonian_arg.is_some());
        assert_eq!(shot_number, None);
        (42.0, 0.25)
    }

    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.h(q0).unwrap();

    let zne = ZNEMitigation::new(circuit, vec![0, 1]);
    let h = single_qubit_z_hamiltonian();

    let values = zne.run_em_sequence(None, &h, &custom_hexp).unwrap();
    assert_eq!(values, vec![42.0, 42.0]);
}

#[test]
fn test_run_em_sequence_rejects_hamiltonian_qubit_mismatch() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0]);

    let mut pauli_string = PauliString::new(2);
    pauli_string.set_pauli(0, Pauli::Z);
    let hamiltonian = Hamiltonian::from_list(vec![(pauli_string, Complex64::new(1.0, 0.0))])
        .expect("two-qubit mismatch Hamiltonian should be valid");

    let err = zne
        .run_em_sequence(None, &hamiltonian, &estimator_hmat)
        .unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::HamiltonianQubitCountMismatch {
            expected: 1,
            actual: 2
        }
    ));
}

#[test]
fn test_poly_extrapolate_linear_intercept() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]); // noise factors: [1, 3, 5]

    // y = 0.75 + 2x
    let noisy_results = vec![2.75, 6.75, 10.75];
    let extrapolated = zne.poly_extrapolate(&noisy_results, 1).unwrap();

    assert!((extrapolated - 0.75).abs() < 1e-10);
}

#[test]
fn test_poly_extrapolate_quadratic_intercept() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]); // noise factors: [1, 3, 5]

    // y = 1.25 - 0.5x + 0.2x^2
    let noisy_results = vec![0.95, 1.55, 3.75];
    let extrapolated = zne.poly_extrapolate(&noisy_results, 2).unwrap();

    assert!((extrapolated - 1.25).abs() < 1e-10);
}

#[test]
fn test_poly_extrapolate_returns_error_on_length_mismatch() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);

    let noisy_results = vec![1.0, 2.0];
    let err = zne.poly_extrapolate(&noisy_results, 1).unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::NoisyResultsLengthMismatch {
            expected: 3,
            actual: 2
        }
    ));
}

#[test]
fn test_poly_extrapolate_returns_error_on_invalid_degree() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1]);

    let noisy_results = vec![1.0, 2.0];
    let err = zne.poly_extrapolate(&noisy_results, 2).unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::InvalidPolynomialDegree {
            degree: 2,
            num_points: 2
        }
    ));
}

#[test]
fn test_exp_extrapolate_recover_zero_noise_value() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]); // noise factors: [1, 3, 5]

    let a = 2.5_f64;
    let tau = 4.0_f64;
    let noisy_results: Vec<f64> = zne
        .noise_factors()
        .iter()
        .map(|&x| a * (-(x as f64) / tau).exp())
        .collect();

    let extrapolated = zne.exp_extrapolate(&noisy_results).unwrap();
    assert!((extrapolated - a).abs() < 1e-10);
}

#[test]
fn test_exp_extrapolate_returns_error_on_length_mismatch() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);

    let noisy_results = vec![1.0, 2.0];
    let err = zne.exp_extrapolate(&noisy_results).unwrap_err();

    assert!(matches!(
        err,
        ErrorMitigationError::NoisyResultsLengthMismatch {
            expected: 3,
            actual: 2
        }
    ));
}

#[test]
fn test_exp_extrapolate_returns_error_on_non_positive_values() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);

    let noisy_results = vec![1.0, 0.0, 0.5];
    let err = zne.exp_extrapolate(&noisy_results).unwrap_err();

    assert!(matches!(err, ErrorMitigationError::NonPositiveNoisyResults));
}

#[test]
fn test_extrapolate_api_polynomial() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]); // noise factors: [1, 3, 5]

    // y = 0.5 + x
    let noisy_results = vec![1.5, 3.5, 5.5];
    let extrapolated = zne
        .extrapolate(&noisy_results, ExtrapolateMethod::Polynomial, 1)
        .unwrap();

    assert!((extrapolated - 0.5).abs() < 1e-10);
}

#[test]
fn test_extrapolate_api_exponential() {
    let circuit = Circuit::new(1);
    let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]); // noise factors: [1, 3, 5]

    let a = 1.8_f64;
    let tau = 2.2_f64;
    let noisy_results: Vec<f64> = zne
        .noise_factors()
        .iter()
        .map(|&x| a * (-(x as f64) / tau).exp())
        .collect();

    // Degree is ignored for exponential mode.
    let extrapolated = zne
        .extrapolate(&noisy_results, ExtrapolateMethod::Exponential, 99)
        .unwrap();
    assert!((extrapolated - a).abs() < 1e-10);
}
