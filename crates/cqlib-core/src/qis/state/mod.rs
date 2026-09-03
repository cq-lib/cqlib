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

//! Quantum state representations for simulation.
//!
//! This module provides different representations of quantum states:
//!
//! - [`Statevector`]: Represents pure quantum states as a vector of complex amplitudes.
//!   Efficient for simulating ideal quantum circuits without noise.
//!
//! - [`DensityMatrix`]: Represents mixed quantum states as a density matrix.
//!   Capable of simulating both pure and mixed states, including quantum channels
//!   via Kraus operators.
//!
//! - [`DensityMatrixNoise`]: Extends the density matrix simulator with a configurable
//!   noise model for realistic quantum simulations including gate errors and readout noise.
//!
//! - [`StabilizerState`]: Simulates Clifford circuits exponentially faster than
//!   state-vector methods using the Aaronson-Gottesman symplectic tableau algorithm.
//!   Supports thousands of qubits; rejects non-Clifford gates with a clear error.
//!
//! # Choosing a Simulator
//!
//! | Simulator | Use Case | Max Qubits | Noise Support |
//! |-----------|----------|------------|---------------|
//! | [`Statevector`] | Ideal universal circuits | ~30 | No |
//! | [`DensityMatrix`] | Mixed states, quantum channels | ~15 | Kraus operators |
//! | [`DensityMatrixNoise`] | Realistic device simulation | ~15 | Full noise model |
//! | [`StabilizerState`] | Clifford-only circuits | 10 000+ | No |
//!
//! # Examples
//!
//! ```rust
//! use cqlib_core::qis::{Statevector, DensityMatrix};
//!
//! // Pure state simulation
//! let mut sv = Statevector::new(3);
//! sv.apply_h(0);
//! sv.apply_cx(0, 1);
//! let probs_sv = sv.probabilities();
//!
//! // Mixed state simulation
//! let mut dm = DensityMatrix::new(3);
//! dm.apply_h(0);
//! dm.apply_cx(0, 1);
//! let probs_dm = dm.probabilities();
//!
//! // Results should be identical for pure states
//! assert!((probs_sv[0] - probs_dm[0]).abs() < 1e-10);
//! ```
//!
//! Stabilizer sampling with aggregated measurement counts:
//!
//! ```rust
//! use cqlib_core::qis::StabilizerState;
//! use std::collections::HashMap;
//!
//! let mut state = StabilizerState::new(2);
//! state.apply_h(0).unwrap();
//! state.apply_cx(0, 1).unwrap();
//!
//! let mut counts = HashMap::new();
//! for outcome in state.sample_shots(1000) {
//!     *counts.entry(outcome.to_bitstring(2)).or_insert(0usize) += 1;
//! }
//!
//! assert!(counts.keys().all(|bits| bits == "00" || bits == "11"));
//! ```

mod aligned_buffer;
pub mod classical;
pub mod density_matrix;
pub mod density_matrix_noise;
pub mod stabilizer;
pub mod statevector;

pub use classical::{ClassicalState, RuntimeValue};
pub use density_matrix::DensityMatrix;
pub use density_matrix_noise::DensityMatrixNoise;
pub use stabilizer::StabilizerState;
pub use statevector::Statevector;

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;
