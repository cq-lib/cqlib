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

//! Contracts for fermion-to-qubit operators and occupation-state encodings.

pub mod jordan_wigner;
pub use jordan_wigner::JordanWignerMapper;

use super::FermionicOp;
use crate::qis::{Hamiltonian, QisError};

/// Maps finite fermionic operators, including non-Hermitian operators, to Pauli sums.
/// Circuit synthesis and Hermiticity requirements belong to downstream consumers.
pub trait FermionToQubitMapper {
    /// Maps an operator while preserving its declared mode space.
    fn map_operator(&self, op: &FermionicOp) -> Result<Hamiltonian, QisError>;
    /// Returns the qubit count required to encode the given mode space.
    fn num_qubits(&self, num_modes: usize) -> usize;
}

/// Encodes occupations into computational-basis bits, where `bits[p]` addresses qp.
/// This is separate from operator mapping because some encodings do not map
/// occupations to individual computational basis states.
pub trait OccupationStateMapper {
    /// Maps a vector of zero/one occupations; its length defines the mode count.
    /// An empty vector encodes the zero-mode vacuum as an empty bit vector.
    fn map_occupation(&self, occupation: &[u8]) -> Result<Vec<bool>, QisError>;
}
