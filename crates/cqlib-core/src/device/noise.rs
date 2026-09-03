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

//! # Noise Model Module
//!
//! This module provides types for simulating quantum noise in device simulations.
//! It includes single-qubit and two-qubit noise channels, readout errors,
//! and a comprehensive noise model for realistic quantum circuit simulation.
//!
//! ## Noise Channels
//!
//! - **Single-qubit**: Bit-flip, phase-flip, depolarizing, amplitude damping, phase damping
//! - **Two-qubit**: Two-qubit depolarizing
//! - **Readout**: Measurement error modeling

use crate::circuit::{Qubit, StandardGate};
use crate::qis::pauli::Pauli;
use ndarray::linalg::kron;
use ndarray::{Array2, array};
use num_complex::Complex64;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use thiserror::Error;

const ZERO: Complex64 = Complex64::new(0.0, 0.0);
const ONE: Complex64 = Complex64::new(1.0, 0.0);

/// Defines errors that can occur when building or applying a noise model.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum NoiseError {
    /// Error for an invalid probability value, which must be in [0, 1].
    #[error("Invalid noise probability: {value} (must be in [0, 1]). Context: {context}")]
    InvalidProbability { value: f64, context: String },

    /// Error when a gate is applied to non-distinct qubits.
    #[error("Qubit collision detected: {qubits:?}. All qubits in a gate must be distinct.")]
    QubitCollision { qubits: Vec<usize> },

    /// Error for incorrect number of qubits provided to an operation.
    #[error("Inconsistent arity: expected {expected} qubits, but got {actual}.")]
    InconsistentArity { expected: u8, actual: u8 },

    /// Internal error in the noise model.
    #[error("Internal noise model error: {0}")]
    Internal(String),
}

/// Represents single-qubit quantum noise channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SingleQubitNoise {
    /// Bit-flip noise with probability $p$.
    /// Kraus operators: $E_0 = \sqrt{1-p} I$, $E_1 = \sqrt{p} X$.
    BitFlip(f64),
    /// Phase-flip noise with probability $p$.
    /// Kraus operators: $E_0 = \sqrt{1-p} I$, $E_1 = \sqrt{p} Z$.
    PhaseFlip(f64),
    /// General Pauli noise with probabilities $p_x$, $p_y$, $p_z$.
    /// Kraus operators: $\sqrt{1-p_x-p_y-p_z} I$, $\sqrt{p_x} X$, $\sqrt{p_y} Y$, $\sqrt{p_z} Z$.
    Pauli { px: f64, py: f64, pz: f64 },
    /// Depolarizing noise with parameter $p$.
    /// Kraus operators: $\sqrt{1-p} I$, $\sqrt{p/3} X$, $\sqrt{p/3} Y$, $\sqrt{p/3} Z$.
    Depolarizing(f64),
    /// Amplitude damping channel with damping parameter $\gamma$.
    /// Kraus operators: $E_0 = \begin{pmatrix} 1 & 0 \\ 0 & \sqrt{1-\gamma} \end{pmatrix}$, $E_1 = \begin{pmatrix} 0 & \sqrt{\gamma} \\ 0 & 0 \end{pmatrix}$.
    AmplitudeDamping(f64),
    /// Phase damping channel with scattering probability $\lambda$.
    /// Kraus operators: $E_0 = \begin{pmatrix} 1 & 0 \\ 0 & \sqrt{1-\lambda} \end{pmatrix}$, $E_1 = \begin{pmatrix} 0 & 0 \\ 0 & \sqrt{\lambda} \end{pmatrix}$.
    PhaseDamping(f64),
}

impl SingleQubitNoise {
    /// Checks if the noise parameters are valid probabilities.
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::BitFlip(p)
            | Self::PhaseFlip(p)
            | Self::Depolarizing(p)
            | Self::AmplitudeDamping(p)
            | Self::PhaseDamping(p) => (0.0..=1.0).contains(&p),
            Self::Pauli { px, py, pz } => {
                px >= 0.0 && py >= 0.0 && pz >= 0.0 && (px + py + pz) <= 1.0
            }
        }
    }

    fn validate(&self) -> Result<(), NoiseError> {
        let probability = |value: f64, context: &str| {
            if value.is_finite() && (0.0..=1.0).contains(&value) {
                Ok(())
            } else {
                Err(NoiseError::InvalidProbability {
                    value,
                    context: context.to_string(),
                })
            }
        };
        match *self {
            Self::BitFlip(p) => probability(p, "bit-flip probability"),
            Self::PhaseFlip(p) => probability(p, "phase-flip probability"),
            Self::Depolarizing(p) => probability(p, "single-qubit depolarizing probability"),
            Self::AmplitudeDamping(p) => probability(p, "amplitude-damping probability"),
            Self::PhaseDamping(p) => probability(p, "phase-damping probability"),
            Self::Pauli { px, py, pz } => {
                probability(px, "Pauli X probability")?;
                probability(py, "Pauli Y probability")?;
                probability(pz, "Pauli Z probability")?;
                let total = px + py + pz;
                if total <= 1.0 {
                    Ok(())
                } else {
                    Err(NoiseError::InvalidProbability {
                        value: total,
                        context: "sum of Pauli probabilities".to_string(),
                    })
                }
            }
        }
    }

    /// Returns the Kraus operators for the single-qubit noise channel.
    pub fn to_kraus(&self) -> Vec<Array2<Complex64>> {
        match *self {
            Self::BitFlip(p) => {
                vec![
                    Pauli::I.to_matrix() * (1.0 - p).sqrt(),
                    Pauli::X.to_matrix() * p.sqrt(),
                ]
            }
            Self::PhaseFlip(p) => {
                vec![
                    Pauli::I.to_matrix() * (1.0 - p).sqrt(),
                    Pauli::Z.to_matrix() * p.sqrt(),
                ]
            }
            Self::Depolarizing(p) => {
                let p_i = (1.0 - p).sqrt();
                let p_other = (p / 3.0).sqrt();
                vec![
                    Pauli::I.to_matrix() * p_i,
                    Pauli::X.to_matrix() * p_other,
                    Pauli::Y.to_matrix() * p_other,
                    Pauli::Z.to_matrix() * p_other,
                ]
            }
            Self::AmplitudeDamping(gamma) => {
                vec![
                    array![[ONE, ZERO], [ZERO, Complex64::from((1.0 - gamma).sqrt())]],
                    array![[ZERO, Complex64::from(gamma.sqrt())], [ZERO, ZERO]],
                ]
            }
            Self::PhaseDamping(lambda) => {
                vec![
                    array![[ONE, ZERO], [ZERO, Complex64::from((1.0 - lambda).sqrt())]],
                    array![[ZERO, ZERO], [ZERO, Complex64::from(lambda.sqrt())]],
                ]
            }
            Self::Pauli { px, py, pz } => {
                let pi = (1.0 - px - py - pz).sqrt();
                vec![
                    Pauli::I.to_matrix() * pi,
                    Pauli::X.to_matrix() * px.sqrt(),
                    Pauli::Y.to_matrix() * py.sqrt(),
                    Pauli::Z.to_matrix() * pz.sqrt(),
                ]
            }
        }
    }
}

/// Represents two-qubit quantum noise channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TwoQubitNoise {
    /// Standard 2-qubit depolarizing channel.
    /// Error probability $p/15$ for each of the 15 non-identity 2-qubit Pauli operators $P_i \otimes P_j$.
    Depolarizing(f64),

    /// Independent single-qubit noise applied to each qubit in a two-qubit gate.
    /// Channel is $\mathcal{E}_{q0} \otimes \mathcal{E}_{q1}$.
    Independent {
        q0_noise: SingleQubitNoise,
        q1_noise: SingleQubitNoise,
    },

    /// Correlated Pauli error on two qubits with probability $p$.
    /// Kraus operators: $\sqrt{1-p} I \otimes I$ and $\sqrt{p} P_{q0} \otimes P_{q1}$.
    CorrelatedPauli { op_q0: Pauli, op_q1: Pauli, p: f64 },
}

impl TwoQubitNoise {
    /// Checks if the noise parameters are valid.
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::Depolarizing(p) | Self::CorrelatedPauli { p, .. } => (0.0..=1.0).contains(&p),
            Self::Independent { q0_noise, q1_noise } => q0_noise.is_valid() && q1_noise.is_valid(),
        }
    }

    fn validate(&self) -> Result<(), NoiseError> {
        match *self {
            Self::Depolarizing(p) => SingleQubitNoise::Depolarizing(p).validate().map_err(|_| {
                NoiseError::InvalidProbability {
                    value: p,
                    context: "two-qubit depolarizing probability".to_string(),
                }
            }),
            Self::CorrelatedPauli { p, .. } => {
                SingleQubitNoise::BitFlip(p).validate().map_err(|_| {
                    NoiseError::InvalidProbability {
                        value: p,
                        context: "correlated-Pauli probability".to_string(),
                    }
                })
            }
            Self::Independent { q0_noise, q1_noise } => {
                q0_noise.validate()?;
                q1_noise.validate()
            }
        }
    }

    /// Returns the Kraus operators for the two-qubit noise channel.
    pub fn to_kraus(&self) -> Vec<Array2<Complex64>> {
        match self {
            Self::Independent { q0_noise, q1_noise } => {
                let ks0 = q0_noise.to_kraus();
                let ks1 = q1_noise.to_kraus();
                let mut result = Vec::with_capacity(ks0.len() * ks1.len());
                for e in &ks0 {
                    for f in &ks1 {
                        result.push(kron(e, f));
                    }
                }
                result
            }
            Self::Depolarizing(p) => {
                let mut ops = Vec::with_capacity(16);
                let p_i = (1.0 - p).sqrt();
                let p_other = (p / 15.0).sqrt();

                // Performance optimization: cache the 4 basic Pauli matrices
                // to avoid repeated construction in the loop.
                let pauli_matrices = [
                    Pauli::I.to_matrix(),
                    Pauli::X.to_matrix(),
                    Pauli::Y.to_matrix(),
                    Pauli::Z.to_matrix(),
                ];

                for (i, m0) in pauli_matrices.iter().enumerate() {
                    for (j, m1) in pauli_matrices.iter().enumerate() {
                        let m_combined = kron(m0, m1);
                        if i == 0 && j == 0 {
                            ops.push(m_combined * p_i);
                        } else {
                            ops.push(m_combined * p_other);
                        }
                    }
                }
                ops
            }
            Self::CorrelatedPauli { op_q0, op_q1, p } => {
                let i_mat = Pauli::I.to_matrix();
                vec![
                    kron(&i_mat, &i_mat) * (1.0 - p).sqrt(),
                    kron(&op_q0.to_matrix(), &op_q1.to_matrix()) * p.sqrt(),
                ]
            }
        }
    }
}

/// Represents asymmetric readout errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadoutError {
    /// Probability of measuring 0 given that the true state is 1: $P(0|1)$.
    pub p_0_given_1: f64,
    /// Probability of measuring 1 given that the true state is 0: $P(1|0)$.
    pub p_1_given_0: f64,
}

impl ReadoutError {
    /// Checks if the probabilities are valid.
    pub fn is_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.p_0_given_1) && (0.0..=1.0).contains(&self.p_1_given_0)
    }

    fn validate(&self) -> Result<(), NoiseError> {
        for (value, context) in [
            (self.p_0_given_1, "readout P(0|1)"),
            (self.p_1_given_0, "readout P(1|0)"),
        ] {
            if !(value.is_finite() && (0.0..=1.0).contains(&value)) {
                return Err(NoiseError::InvalidProbability {
                    value,
                    context: context.to_string(),
                });
            }
        }
        Ok(())
    }
}

const MAX_GATE_ARITY: usize = 3;

/// A key identifying a specific operation on specific qubits, used to map noise parameters.
#[derive(Debug, Clone, Copy, Eq)]
pub struct OperationKey {
    /// The standard quantum gate.
    gate: StandardGate,
    /// Fixed-size array to store qubit indices without heap allocation.
    /// Unused slots are zero-padded but ignored in Hash and Eq.
    qubits: [usize; MAX_GATE_ARITY],
    /// The actual number of qubits involved in the operation.
    arity: u8,
}

// Manually implement Hash to only hash valid data, ignoring padding.
impl Hash for OperationKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.gate.hash(state);
        self.qubits().hash(state);
    }
}

impl PartialEq for OperationKey {
    fn eq(&self, other: &Self) -> bool {
        // 1. Gate types must match
        if self.gate != other.gate {
            return false;
        }
        // 2. Arity must match
        if self.arity != other.arity {
            return false;
        }
        // 3. Compare only the valid slice of qubit indices, ignoring padding
        self.qubits() == other.qubits()
    }
}

impl OperationKey {
    /// Constructor for a single-qubit gate operation key.
    pub fn new_single(gate: StandardGate, q0: Qubit) -> Self {
        Self {
            gate,
            qubits: [q0.index(), 0, 0],
            arity: 1,
        }
    }

    /// Constructor for a two-qubit gate operation key.
    /// Returns an error if the qubits are identical.
    pub fn new_double(gate: StandardGate, q0: Qubit, q1: Qubit) -> Result<Self, NoiseError> {
        if q0 == q1 {
            return Err(NoiseError::QubitCollision {
                qubits: vec![q0.index(), q1.index()],
            });
        }
        Ok(Self {
            gate,
            qubits: [q0.index(), q1.index(), 0],
            arity: 2,
        })
    }

    /// Constructor for a three-qubit gate operation key.
    /// Returns an error if any qubits overlap.
    pub fn new_triple(
        gate: StandardGate,
        q0: Qubit,
        q1: Qubit,
        q2: Qubit,
    ) -> Result<Self, NoiseError> {
        if q0 == q1 || q1 == q2 || q0 == q2 {
            return Err(NoiseError::QubitCollision {
                qubits: vec![q0.index(), q1.index(), q2.index()],
            });
        }
        Ok(Self {
            gate,
            qubits: [q0.index(), q1.index(), q2.index()],
            arity: 3,
        })
    }

    /// Returns a slice of the actual qubits involved, ignoring padding.
    pub fn qubits(&self) -> &[usize] {
        &self.qubits[..self.arity as usize]
    }

    /// Returns the standard gate associated with the operation.
    pub fn gate(&self) -> &StandardGate {
        &self.gate
    }
}

/// A noise model containing error definitions for different operations and measurements.
#[derive(Debug, Clone, Default)]
pub struct NoiseModel {
    /// Readout errors mapped by qubit.
    readout_errors: HashMap<Qubit, ReadoutError>,
    /// Single-qubit errors mapped by operation key.
    /// Distinct storage to prevent logical confusion.
    single_gate_errors: HashMap<OperationKey, Vec<SingleQubitNoise>>,
    /// Two-qubit errors mapped by operation key.
    two_gate_errors: HashMap<OperationKey, Vec<TwoQubitNoise>>,
}

impl NoiseModel {
    /// Creates a new, empty noise model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a readout error for a specific qubit.
    pub fn add_readout_error(
        &mut self,
        qubit: Qubit,
        error: ReadoutError,
    ) -> Result<(), NoiseError> {
        error.validate().map_err(|error| match error {
            NoiseError::InvalidProbability { value, context } => NoiseError::InvalidProbability {
                value,
                context: format!("{context} for qubit {qubit:?}"),
            },
            other => other,
        })?;
        self.readout_errors.insert(qubit, error);
        Ok(())
    }

    /// Adds single-qubit noise to a specific gate on a target qubit.
    pub fn add_single_qubit_error(
        &mut self,
        gate: StandardGate,
        qubit: Qubit,
        noise: SingleQubitNoise,
    ) -> Result<(), NoiseError> {
        noise.validate().map_err(|error| match error {
            NoiseError::InvalidProbability { value, context } => NoiseError::InvalidProbability {
                value,
                context: format!("{context} for gate {gate:?}"),
            },
            other => other,
        })?;
        let key = OperationKey::new_single(gate, qubit);
        self.single_gate_errors.entry(key).or_default().push(noise);
        Ok(())
    }

    /// Adds two-qubit noise to a specific gate on target qubits.
    pub fn add_two_qubit_error(
        &mut self,
        gate: StandardGate,
        q0: Qubit,
        q1: Qubit,
        noise: TwoQubitNoise,
    ) -> Result<(), NoiseError> {
        noise.validate().map_err(|error| match error {
            NoiseError::InvalidProbability { value, context } => NoiseError::InvalidProbability {
                value,
                context: format!("{context} for gate {gate:?}"),
            },
            other => other,
        })?;
        let key = OperationKey::new_double(gate, q0, q1)?;
        self.two_gate_errors.entry(key).or_default().push(noise);
        Ok(())
    }

    /// Retrieves the readout error for a given qubit, if present.
    pub fn get_readout_error(&self, key: &Qubit) -> Option<&ReadoutError> {
        self.readout_errors.get(key)
    }

    /// Retrieves the list of single-qubit errors for a given operation key.
    pub fn get_single_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<SingleQubitNoise>> {
        self.single_gate_errors.get(key)
    }

    /// Retrieves the list of two-qubit errors for a given operation key.
    pub fn get_two_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<TwoQubitNoise>> {
        self.two_gate_errors.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::gate::standard_gate::StandardGate;

    #[test]
    fn operation_keys_allow_effective_controlled_arity_and_reject_collisions() {
        let q0 = Qubit::new(0);
        let q1 = Qubit::new(1);

        assert_eq!(
            OperationKey::new_single(StandardGate::CX, q0).qubits(),
            &[0]
        );
        assert_eq!(
            OperationKey::new_double(StandardGate::H, q0, q1)
                .unwrap()
                .qubits(),
            &[0, 1]
        );
        assert_eq!(
            OperationKey::new_triple(StandardGate::CX, q0, q1, Qubit::new(2))
                .unwrap()
                .qubits(),
            &[0, 1, 2]
        );
        assert!(matches!(
            OperationKey::new_double(StandardGate::CX, q0, q0),
            Err(NoiseError::QubitCollision { .. })
        ));
        assert!(matches!(
            OperationKey::new_triple(StandardGate::CCX, q0, q1, q1),
            Err(NoiseError::QubitCollision { .. })
        ));
    }

    #[test]
    fn noise_errors_report_the_actual_invalid_probability() {
        let mut model = NoiseModel::new();

        match model.add_single_qubit_error(
            StandardGate::H,
            Qubit::new(0),
            SingleQubitNoise::BitFlip(-0.25),
        ) {
            Err(NoiseError::InvalidProbability { value, context }) => {
                assert_eq!(value, -0.25);
                assert!(context.contains("bit-flip"));
            }
            result => panic!("unexpected result: {result:?}"),
        }

        match model.add_readout_error(
            Qubit::new(0),
            ReadoutError {
                p_0_given_1: 1.5,
                p_1_given_0: 0.0,
            },
        ) {
            Err(NoiseError::InvalidProbability { value, context }) => {
                assert_eq!(value, 1.5);
                assert!(context.contains("P(0|1)"));
            }
            result => panic!("unexpected result: {result:?}"),
        }
    }
}
