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

//! Observable trait for computing expectation values.
//!
//! This module defines the [`Observable`] trait, which provides a unified interface
//! for computing expectation values of quantum observables across different quantum
//! state representations (statevectors and density matrices).
//!
//! # Supported Observables
//!
//! - [`Hamiltonian`]: A sum of Pauli strings with complex coefficients, commonly
//!   used to represent system energies and physical observables.
//! - [`PauliString`]: A single multi-qubit Pauli operator, useful for individual
//!   measurements and stabilizer formalism.
//!
//! # Expectation Value Formulas
//!
//! For a statevector $|\psi\rangle$:
//! $$\langle O \rangle = \langle\psi|O|\psi\rangle$$
//!
//! For a density matrix $\rho$:
//! $$\langle O \rangle = \text{Tr}(\rho O)$$
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::qis::{Statevector, Hamiltonian, PauliString, Observable};
//! use num_complex::Complex64;
//!
//! // Create a state
//! let mut sv = Statevector::new(2);
//! sv.apply_h(0);
//! sv.apply_cx(0, 1);
//!
//! // Create an observable H = ZZ
//! let ps: PauliString = "ZZ".parse().unwrap();
//! let h = Hamiltonian::from_pauli(ps);
//!
//! // Compute expectation value
//! let exp = h.expectation_statevector(&sv).unwrap();
//! // For Bell state, <ZZ> = 1.0
//! ```

use crate::qis::error::QisError;
use crate::qis::hamiltonian::Hamiltonian;
use crate::qis::pauli::{PauliString, Phase};
use crate::qis::state::{DensityMatrix, Statevector};
use num_complex::Complex64;
use rayon::prelude::*;
use std::collections::HashMap;

const VARIANCE_TOLERANCE: f64 = 1e-10;

/// Trait for quantum observables that can compute expectation values.
///
/// This trait is implemented by types representing quantum observables,
/// allowing them to compute expectation values across different quantum
/// state representations.
///
/// # Type Implementations
///
/// - [`Hamiltonian`]: Implements this trait for multi-term Hamiltonians
/// - [`PauliString`]: Implements this trait for single Pauli operators
///
/// # Errors
///
/// Methods may return [`QisError::QubitMismatch`] if the observable
/// and the quantum state have incompatible qubit counts.
pub trait Observable {
    /// Computes the expectation value for a statevector: $\langle\psi|O|\psi\rangle$.
    ///
    /// # Arguments
    ///
    /// * `sv` - The statevector representing the pure quantum state.
    ///
    /// # Returns
    ///
    /// The expectation value as a real number (`f64`), or an error if the
    /// qubit counts do not match.
    fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError>;

    /// Computes the expectation value for a density matrix: $\text{Tr}(\rho O)$.
    ///
    /// # Arguments
    ///
    /// * `dm` - The density matrix representing the quantum state.
    ///
    /// # Returns
    ///
    /// The expectation value as a real number (`f64`), or an error if the
    /// qubit counts do not match.
    fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError>;

    /// Computes the expectation value from measurement probabilities.
    ///
    /// This method allows computing expectation values from classical
    /// measurement outcomes, useful for shot-based simulators and
    /// real quantum hardware results.
    ///
    /// # Arguments
    ///
    /// * `measurements` - A slice of tuples containing the measurement basis
    ///   (as a [`PauliString`]) and a map from state strings to their observed
    ///   probabilities.
    ///
    /// # Returns
    ///
    /// The expectation value as a real number (`f64`), or an error if no
    /// compatible measurement basis is found.
    fn expectation_probs(
        &self,
        measurements: &[(PauliString, HashMap<String, f64>)],
    ) -> Result<f64, QisError>;

    /// Returns the number of qubits this observable acts on.
    ///
    /// Used for dimension validation before computing expectation values.
    fn num_qubits(&self) -> usize;

    /// Computes the variance of this observable for a statevector.
    ///
    /// Var(O) = <O^2> - <O>^2.  Returns an error when the observable is not
    /// Hermitian, the qubit counts do not match, or a numerical issue arises.
    ///
    /// # Default Implementation
    ///
    /// Returns QisError::UnsupportedOperation. Observable types that support
    /// variance calculation must override this method.
    fn variance_statevector(&self, _sv: &Statevector) -> Result<f64, QisError> {
        Err(QisError::UnsupportedOperation(
            "variance_statevector not implemented for this observable type".into(),
        ))
    }
}

fn apply_pauli_string_to_statevector(
    pauli_str: &PauliString,
    sv: &Statevector,
    coeff: Complex64,
) -> Vec<Complex64> {
    let x_mask = pauli_str.x_mask();
    let z_mask = pauli_str.z_mask();
    let y_phase = pauli_str.y_phase();
    let global_phase = pauli_str.phase.to_complex();
    let base_factor = coeff * global_phase * y_phase;
    let sv_data = sv.data();

    (0..sv_data.len())
        .into_par_iter()
        .map(|j| {
            let source_j = j ^ x_mask;
            // P = phase * i^num_y * X^x * Z^z: Z acts on the source.
            let z_parity = (source_j & z_mask).count_ones();
            let sign = if z_parity % 2 == 1 { -1.0 } else { 1.0 };
            sv_data[source_j] * base_factor * sign
        })
        .collect()
}

fn finalize_variance(variance: f64) -> Result<f64, QisError> {
    if variance < -VARIANCE_TOLERANCE {
        // The current QisError model has no dedicated numerical error variant.
        return Err(QisError::InvalidParameterValue(format!(
            "variance computation produced a negative result beyond tolerance: {}",
            variance
        )));
    }

    if variance < 0.0 {
        Ok(0.0)
    } else {
        Ok(variance)
    }
}

impl Observable for Hamiltonian {
    fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError> {
        if sv.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: sv.num_qubits,
            });
        }

        let mut expected_value = 0.0;

        // Iterate over each term in the Hamiltonian
        for (pauli_str, coeff) in &self.terms {
            let x_mask = pauli_str.x_mask();
            let z_mask = pauli_str.z_mask();

            // Calculate Y phase factor and combine with PauliString's global phase
            let y_phase = pauli_str.y_phase();
            let global_phase = pauli_str.phase.to_complex();
            let base_factor = global_phase * y_phase;

            // Compute term expectation value in parallel
            let sv_data = sv.data();
            let term_expectation: Complex64 = sv_data
                .par_iter()
                .enumerate()
                .map(|(j, amp)| {
                    // Output j receives the amplitude from source_j.
                    let source_j = j ^ x_mask;
                    let source_amp = sv_data[source_j];

                    // Z acts before X, so its phase depends on the source.
                    let z_parity = (source_j & z_mask).count_ones();
                    let sign = if z_parity % 2 == 1 { -1.0 } else { 1.0 };

                    let phase_factor = base_factor * sign;

                    // Contribution: conj(amp_j) * (P * psi)_j
                    amp.conj() * source_amp * phase_factor
                })
                .sum();

            // Hamiltonian is Hermitian, so expectation value is real
            expected_value += (term_expectation * coeff).re;
        }

        Ok(expected_value)
    }

    fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError> {
        if dm.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: dm.num_qubits,
            });
        }

        let mut expected_value = 0.0;
        let n = self.num_qubits;
        let dim = 1 << n; // 2^n

        for (pauli_str, coeff) in &self.terms {
            let x_mask = pauli_str.x_mask();
            let z_mask = pauli_str.z_mask();

            // Calculate Y phase factor and combine with PauliString's global phase
            let y_phase = pauli_str.y_phase();
            let global_phase = pauli_str.phase.to_complex();
            let base_factor = global_phase * y_phase;

            // Parallel computation: Tr(ρ * P) = Σ_j ρ[j, j^x] * phase(j, P)
            let term_expectation: Complex64 = (0..dim)
                .into_par_iter()
                .map(|j| {
                    // X operator: flip bits where x_mask is 1 to get column index
                    let col = j ^ x_mask;

                    // Flat index: (row << n) | col
                    let flat_index = (j << n) | col;
                    let rho_elem = dm.data()[flat_index];

                    // Z operator: add phase (-1)^(number of overlapping Z bits)
                    let z_parity = (j & z_mask).count_ones();
                    let sign = if z_parity % 2 == 1 { -1.0 } else { 1.0 };

                    let phase_factor = base_factor * sign;

                    // Contribution to trace
                    rho_elem * phase_factor
                })
                .sum();

            // Hamiltonian is Hermitian, so expectation value is real
            expected_value += (term_expectation * coeff).re;
        }

        Ok(expected_value)
    }

    fn expectation_probs(
        &self,
        measurements: &[(PauliString, HashMap<String, f64>)],
    ) -> Result<f64, QisError> {
        let mut expected_value = 0.0;

        for (term_pauli, coeff) in &self.terms {
            let term_x_mask = term_pauli.x_mask();
            let term_z_mask = term_pauli.z_mask();
            let p_active = term_x_mask | term_z_mask;

            // Identity term contributes its coefficient * global_phase directly
            if p_active == 0 {
                let term_contrib = (term_pauli.phase.to_complex() * coeff).re;
                expected_value += term_contrib;
                continue;
            }

            // Find a compatible measurement basis.
            // A measurement M is compatible with P if P_i == M_i for all non-Identity P_i.
            let mut compatible_measurement = None;
            for (m_pauli, probs) in measurements {
                let m_x_mask = m_pauli.x_mask();
                let m_z_mask = m_pauli.z_mask();

                if (m_x_mask & p_active) == term_x_mask && (m_z_mask & p_active) == term_z_mask {
                    compatible_measurement = Some(probs);
                    break;
                }
            }

            if let Some(probs) = compatible_measurement {
                let mut exp_val = 0.0;

                for (state_str, prob) in probs {
                    if state_str.len() != self.num_qubits {
                        return Err(QisError::DimensionMismatch {
                            expected: self.num_qubits,
                            actual: state_str.len(),
                        });
                    }

                    let mut state_idx = 0usize;
                    for (i, c) in state_str.chars().rev().enumerate() {
                        match c {
                            '1' => state_idx |= 1 << i,
                            '0' => {}
                            _ => {
                                return Err(QisError::PauliStringParseError(
                                    crate::qis::error::PauliStringParseError::InvalidCharacter(c),
                                ));
                            }
                        }
                    }

                    let parity = (state_idx & p_active).count_ones();
                    let eigenvalue = if parity % 2 == 1 { -1.0 } else { 1.0 };
                    exp_val += prob * eigenvalue;
                }

                // Ignore y_phase, only apply global phase and coefficient
                let term_contrib = (exp_val * term_pauli.phase.to_complex() * coeff).re;
                expected_value += term_contrib;
            } else {
                return Err(QisError::UnsupportedOperation(format!(
                    "No compatible measurement basis found for term {}",
                    term_pauli
                )));
            }
        }

        Ok(expected_value)
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn variance_statevector(&self, sv: &Statevector) -> Result<f64, QisError> {
        if sv.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: sv.num_qubits,
            });
        }

        let mut simplified = self.clone();
        simplified.simplify();

        for (_, coeff) in &mut simplified.terms {
            if coeff.im.abs() > VARIANCE_TOLERANCE {
                return Err(QisError::NotHermitian);
            }
            coeff.im = 0.0;
        }

        let expectation = simplified.expectation_statevector(sv)?;
        let mut applied = vec![Complex64::new(0.0, 0.0); sv.data().len()];
        for (pauli_str, coeff) in &simplified.terms {
            let term_state = apply_pauli_string_to_statevector(pauli_str, sv, *coeff);
            for (acc, term_amp) in applied.iter_mut().zip(term_state) {
                *acc += term_amp;
            }
        }

        let second_moment: f64 = applied.iter().map(|amp| amp.norm_sqr()).sum();
        finalize_variance(second_moment - expectation * expectation)
    }
}

impl Observable for PauliString {
    fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError> {
        if sv.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: sv.num_qubits,
            });
        }

        let x_mask = self.x_mask();
        let z_mask = self.z_mask();
        let y_phase = self.y_phase();
        let global_phase = self.phase.to_complex();
        let base_factor = global_phase * y_phase;

        let sv_data = sv.data();
        let term_expectation: Complex64 = sv_data
            .par_iter()
            .enumerate()
            .map(|(j, amp)| {
                let source_j = j ^ x_mask;
                let source_amp = sv_data[source_j];
                // Z acts on the source before X moves it to output j.
                let z_parity = (source_j & z_mask).count_ones();
                let sign = if z_parity % 2 == 1 { -1.0 } else { 1.0 };
                let phase_factor = base_factor * sign;
                amp.conj() * source_amp * phase_factor
            })
            .sum();

        Ok(term_expectation.re)
    }

    fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError> {
        if dm.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: dm.num_qubits,
            });
        }

        let n = self.num_qubits;
        let dim = 1 << n;
        let x_mask = self.x_mask();
        let z_mask = self.z_mask();
        let y_phase = self.y_phase();
        let global_phase = self.phase.to_complex();
        let base_factor = global_phase * y_phase;

        let term_expectation: Complex64 = (0..dim)
            .into_par_iter()
            .map(|j| {
                let col = j ^ x_mask;
                let flat_index = (j << n) | col;
                let rho_elem = dm.data()[flat_index];
                let z_parity = (j & z_mask).count_ones();
                let sign = if z_parity % 2 == 1 { -1.0 } else { 1.0 };
                let phase_factor = base_factor * sign;
                rho_elem * phase_factor
            })
            .sum();

        Ok(term_expectation.re)
    }

    fn expectation_probs(
        &self,
        measurements: &[(PauliString, HashMap<String, f64>)],
    ) -> Result<f64, QisError> {
        let term_x_mask = self.x_mask();
        let term_z_mask = self.z_mask();
        let p_active = term_x_mask | term_z_mask;

        if p_active == 0 {
            return Ok(self.phase.to_complex().re);
        }

        let mut compatible_measurement = None;
        for (m_pauli, probs) in measurements {
            let m_x_mask = m_pauli.x_mask();
            let m_z_mask = m_pauli.z_mask();

            if (m_x_mask & p_active) == term_x_mask && (m_z_mask & p_active) == term_z_mask {
                compatible_measurement = Some(probs);
                break;
            }
        }

        if let Some(probs) = compatible_measurement {
            let mut exp_val = 0.0;

            for (state_str, prob) in probs {
                if state_str.len() != self.num_qubits {
                    return Err(QisError::QubitMismatch {
                        expected: self.num_qubits,
                        actual: state_str.len(),
                    });
                }

                let mut state_idx = 0usize;
                for (i, c) in state_str.chars().rev().enumerate() {
                    match c {
                        '1' => state_idx |= 1 << i,
                        '0' => {}
                        _ => {
                            return Err(QisError::UnsupportedOperation(format!(
                                "Invalid character '{}' in state string",
                                c
                            )));
                        }
                    }
                }

                let parity = (state_idx & p_active).count_ones();
                let eigenvalue = if parity % 2 == 1 { -1.0 } else { 1.0 };
                exp_val += prob * eigenvalue;
            }

            Ok((exp_val * self.phase.to_complex()).re)
        } else {
            Err(QisError::UnsupportedOperation(format!(
                "No compatible measurement basis found for term {}",
                self
            )))
        }
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn variance_statevector(&self, sv: &Statevector) -> Result<f64, QisError> {
        if sv.num_qubits != self.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: sv.num_qubits,
            });
        }

        match self.phase {
            Phase::Plus | Phase::Minus => {}
            Phase::I | Phase::MinusI => return Err(QisError::NotHermitian),
        }

        let expectation = self.expectation_statevector(sv)?;
        let second_moment: f64 =
            apply_pauli_string_to_statevector(self, sv, Complex64::new(1.0, 0.0))
                .iter()
                .map(|amp| amp.norm_sqr())
                .sum();
        finalize_variance(second_moment - expectation * expectation)
    }
}

#[cfg(test)]
#[path = "./observable_test.rs"]
mod observable_test;
