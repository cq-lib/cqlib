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

//! Regressions for noisy identity gates, observable validation and circuit phase.
use cqlib_core::circuit::{Circuit, Instruction, Parameter, Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, noise::SingleQubitNoise};
use cqlib_core::error_mitigation::ZNEMitigation;
use cqlib_core::qis::entropy::renyi_entropy;
use cqlib_core::qis::error::QisError;
use cqlib_core::qis::{
    DensityMatrix, DensityMatrixNoise, Hamiltonian, Observable, PauliString, Statevector,
};
use num_complex::Complex64 as C;
use std::collections::HashMap;

#[test]
fn identity_noise_respects_labels_and_repetition() {
    for ids in [[0, 1], [12, 5]] {
        for p in [0.0, 0.25, 1.0] {
            for count in [1, 2, 3] {
                let qs = ids.map(Qubit::new);
                let mut c = Circuit::from_qubits(qs.to_vec()).unwrap();
                for _ in 0..count {
                    c.i(qs[1]).unwrap();
                }
                let mut model = NoiseModel::new();
                model
                    .add_single_qubit_error(StandardGate::I, qs[1], SingleQubitNoise::BitFlip(p))
                    .unwrap();
                let sim = DensityMatrixNoise::from_circuit(&c, Some(model)).unwrap();
                let flipped = (1.0 - (1.0 - 2.0 * p).powi(count)) / 2.0;
                for (actual, expected) in
                    sim.probabilities()
                        .iter()
                        .zip([1.0 - flipped, 0.0, flipped, 0.0])
                {
                    assert!((actual - expected).abs() < 1e-12);
                }
                assert_eq!(
                    DensityMatrixNoise::from_circuit(&c, None)
                        .unwrap()
                        .probabilities(),
                    vec![1.0, 0.0, 0.0, 0.0]
                );
            }
        }
    }
    let mut sim = DensityMatrixNoise::new(1, None);
    assert!(
        sim.apply_standard_gate_noise(StandardGate::I, &[1], &[])
            .is_err()
    );
}

#[test]
fn nonfinite_pure_states_are_rejected() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        for amp in [C::new(bad, 0.0), C::new(0.0, bad)] {
            let state = vec![amp, C::new(0.0, 0.0)];
            assert!(matches!(
                Statevector::from_state(1, state.clone()),
                Err(QisError::NotNormalized)
            ));
            assert!(matches!(
                DensityMatrix::from_state(1, state),
                Err(QisError::NotNormalized)
            ));
        }
    }
    for n in [0, 1] {
        let mut v = vec![C::new(0.0, 0.0); 1 << n];
        v[0] = C::new(0.0, 1.0);
        assert!(Statevector::from_state(n, v.clone()).is_ok());
        assert!(DensityMatrix::from_state(n, v).is_ok());
    }
}

#[test]
fn nonhermitian_observables_are_rejected_even_at_zero_expectation() {
    let sv = Statevector::new(1);
    let dm = DensityMatrix::new(1);
    let probs = HashMap::from([("0".to_string(), 1.0)]);
    for label in ["iZ", "-iI", "iX"] {
        let p: PauliString = label.parse().unwrap();
        let h = Hamiltonian::from_pauli(p.clone());
        let measurements = vec![(p.clone(), probs.clone())];
        for obs in [&p as &dyn Observable, &h as &dyn Observable] {
            assert!(matches!(
                obs.expectation_statevector(&sv),
                Err(QisError::NotHermitian)
            ));
            assert!(matches!(
                obs.expectation_density_matrix(&dm),
                Err(QisError::NotHermitian)
            ));
            assert!(matches!(
                obs.expectation_probs(&measurements),
                Err(QisError::NotHermitian)
            ));
        }
        assert!(matches!(p.expectation(&probs), Err(QisError::NotHermitian)));
    }
}

#[test]
fn hermiticity_is_checked_after_phase_and_duplicate_merging() {
    let sv = Statevector::new(1);
    let dm = DensityMatrix::new(1);
    let probs = vec![(
        "Z".parse().unwrap(),
        HashMap::from([("0".to_string(), 1.0)]),
    )];
    for terms in [
        vec![("iZ", C::new(0.0, -1.0))],
        vec![("Z", C::new(1.0, 2.0)), ("Z", C::new(0.0, -2.0))],
        vec![("Z", C::new(1e-11, 0.0))],
    ] {
        let h = Hamiltonian::from_list(
            terms
                .iter()
                .map(|(p, c)| (p.parse().unwrap(), *c))
                .collect(),
        )
        .unwrap();
        let expected = if terms[0].1.re == 1e-11 { 1e-11 } else { 1.0 };
        for actual in [
            h.expectation_statevector(&sv).unwrap(),
            h.expectation_density_matrix(&dm).unwrap(),
            h.expectation_probs(&probs).unwrap(),
        ] {
            assert!((actual - expected).abs() < 1e-14);
        }
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let h = Hamiltonian::from_list(vec![("Z".parse().unwrap(), C::new(bad, 0.0))]).unwrap();
        assert!(matches!(
            h.expectation_statevector(&sv),
            Err(QisError::InvalidParameterValue(_))
        ));
        assert!(matches!(
            h.expectation_density_matrix(&dm),
            Err(QisError::InvalidParameterValue(_))
        ));
        assert!(matches!(
            h.expectation_probs(&probs),
            Err(QisError::InvalidParameterValue(_))
        ));
    }
    let h = Hamiltonian::from_pauli("iZZ".parse().unwrap());
    assert!(matches!(
        h.expectation_statevector(&sv),
        Err(QisError::QubitMismatch { .. })
    ));
}

#[test]
fn folding_preserves_full_matrix_including_global_phase() {
    for gates in [false, true] {
        let mut c = Circuit::from_qubits(vec![Qubit::new(5)]).unwrap();
        c.set_global_phase(Parameter::from(0.37));
        if gates {
            c.h(Qubit::new(5)).unwrap();
            c.t(Qubit::new(5)).unwrap();
        }
        let expected = c.to_matrix(None).unwrap();
        let zne = ZNEMitigation::new(c, vec![0, 1, 2]);
        let selected = [Instruction::Standard(StandardGate::H)];
        for selection in [None, Some(selected.as_slice()), Some([].as_slice())] {
            for folded in zne.fold_circuits(selection).unwrap() {
                let actual = folded.to_matrix(None).unwrap();
                assert!(
                    actual
                        .iter()
                        .zip(expected.iter())
                        .all(|(a, b)| (*a - *b).norm() < 1e-12)
                );
            }
        }
    }
}

#[test]
fn hermiticity_tolerance_and_merged_overflow() {
    for (imaginary, valid) in [(0.5e-10, true), (2e-10, false)] {
        let h =
            Hamiltonian::from_list(vec![("Z".parse().unwrap(), C::new(1.0, imaginary))]).unwrap();
        assert_eq!(
            h.expectation_statevector(&Statevector::new(1)).is_ok(),
            valid
        );
        assert_eq!(
            h.expectation_density_matrix(&DensityMatrix::new(1)).is_ok(),
            valid
        );
    }
    let h = Hamiltonian::from_list(vec![("Z".parse().unwrap(), C::new(f64::MAX, 0.0)); 2]).unwrap();
    assert!(matches!(
        h.expectation_statevector(&Statevector::new(1)),
        Err(QisError::InvalidParameterValue(_))
    ));
}

#[test]
fn renyi_entropy_of_nondiagonal_pure_state_is_zero() {
    let state = [1.0, 2.0, 3.0, 4.0]
        .map(|v| C::new(v / 30.0_f64.sqrt(), 0.0))
        .to_vec();
    let dm = DensityMatrix::from_state(2, state).unwrap();
    for alpha in [0.01, 0.1, 0.5, 1.0, 2.0, f64::INFINITY] {
        let actual = renyi_entropy(&dm, alpha).unwrap();
        assert!(actual.abs() < 1e-10, "alpha={alpha}, entropy={actual}");
    }
}

#[test]
fn renyi_entropy_preserves_rank_deficient_spectrum_under_unitary_rotation() {
    // The first two columns of the four-dimensional Fourier matrix are
    // orthonormal, so this state has spectrum [0.8, 0.2, 0, 0].
    let column = [
        C::new(1.0, 0.0),
        C::new(0.0, 1.0),
        C::new(-1.0, 0.0),
        C::new(0.0, -1.0),
    ];
    let mut rho = Vec::new();
    let mut diagonal = vec![C::new(0.0, 0.0); 16];
    diagonal[0] = 0.8.into();
    diagonal[5] = 0.2.into();
    for i in 0..4 {
        for j in 0..4 {
            rho.push((C::new(0.8, 0.0) + 0.2 * column[i] * column[j].conj()) / 4.0);
        }
    }
    for data in [diagonal, rho] {
        let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();
        for alpha in [0.01, 0.1, 0.5, 1.0, 2.0, f64::INFINITY] {
            let expected = if alpha == 1.0 {
                -0.8_f64 * 0.8_f64.log2() - 0.2_f64 * 0.2_f64.log2()
            } else if alpha == f64::INFINITY {
                -0.8_f64.log2()
            } else {
                (0.8_f64.powf(alpha) + 0.2_f64.powf(alpha)).log2() / (1.0 - alpha)
            };
            assert!((renyi_entropy(&dm, alpha).unwrap() - expected).abs() < 1e-10);
        }
    }
}

#[test]
fn variance_validates_coefficients_before_merging_and_retains_small_terms() {
    let sv = Statevector::new(1);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for coeff in [C::new(bad, 0.0), C::new(0.0, bad)] {
            let h = Hamiltonian::from_list(vec![("Z".parse().unwrap(), coeff)]).unwrap();
            assert!(matches!(
                h.variance_statevector(&sv),
                Err(QisError::InvalidParameterValue(_))
            ));
        }
    }
    let h = Hamiltonian::from_list(vec![("Z".parse().unwrap(), C::new(f64::MAX, 0.0)); 2]).unwrap();
    assert!(matches!(
        h.variance_statevector(&sv),
        Err(QisError::InvalidParameterValue(_))
    ));
    for terms in [
        vec![("iX", C::new(0.0, -1.0))],
        vec![("X", C::new(1.0, 2.0)), ("X", C::new(0.0, -2.0))],
    ] {
        let h = Hamiltonian::from_list(
            terms
                .into_iter()
                .map(|(p, c)| (p.parse().unwrap(), c))
                .collect(),
        )
        .unwrap();
        assert!((h.variance_statevector(&sv).unwrap() - 1.0).abs() < 1e-12);
    }
    let h = Hamiltonian::from_list(vec![("X".parse().unwrap(), C::new(1e-11, 0.0))]).unwrap();
    assert!((h.variance_statevector(&sv).unwrap() - 1e-22).abs() < 1e-34);
    let invalid = Hamiltonian::from_pauli("iZ".parse().unwrap());
    assert!(matches!(
        invalid.variance_statevector(&sv),
        Err(QisError::NotHermitian)
    ));
    assert!(matches!(
        invalid.variance_statevector(&Statevector::new(2)),
        Err(QisError::QubitMismatch { .. })
    ));
}
