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

//! C ABI for QIS module (Statevector, DensityMatrix, StabilizerState, etc.).

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod density_matrix;
pub mod density_matrix_noise;
pub mod entropy;
pub mod hamiltonian;
pub mod measurement;
pub mod metrics;
pub mod pauli;
pub mod stabilizer;
pub mod statevector;

use cqlib_core::circuit::{ClassicalValue, Measurement};
use cqlib_core::qis::{
    DensityMatrix, DensityMatrixNoise, Hamiltonian, PauliString, StabilizerState, Statevector,
};

/// Opaque handle around [`Statevector`] for C ABI.
pub struct CStatevector {
    pub inner: Statevector,
}

/// Opaque handle around [`DensityMatrix`] for C ABI.
pub struct CDensityMatrix {
    pub inner: DensityMatrix,
}

/// Opaque handle around [`StabilizerState`] for C ABI.
pub struct CStabilizerState {
    pub inner: StabilizerState,
}

/// Opaque handle around [`Hamiltonian`] for C ABI.
pub struct CHamiltonian {
    pub inner: Hamiltonian,
}

/// Opaque handle around [`PauliString`] for C ABI.
pub struct CPauliString {
    pub inner: PauliString,
}

/// Opaque handle around [`DensityMatrixNoise`] for C ABI.
pub struct CDensityMatrixNoise {
    pub inner: DensityMatrixNoise,
}

/// Opaque handle around [`Measurement`] for C ABI.
pub struct CMeasurement {
    pub inner: Measurement,
}

/// Opaque handle around [`ClassicalValue`] for C ABI.
pub struct CClassicalValue {
    pub inner: ClassicalValue,
}

/// Owned list of measurement outcome bitstrings returned by `*_sample_shots`.
///
/// Each entry is a big-endian binary string (MSB left, qubit N-1 first,
/// qubit 0 last) suitable for direct use from C.
pub struct COutcomeList {
    pub items: Vec<String>,
}

/// Validates a qubit index against a simulator width, mapping to `QisError`.
#[inline]
pub(crate) fn check_qubit(num_qubits: usize, qubit: u32) -> Result<usize, i32> {
    if qubit as usize >= num_qubits {
        return Err(-2);
    }
    Ok(qubit as usize)
}

/// Maps a `QisError` to a negative C error code.
pub(crate) fn qis_err_code(err: &cqlib_core::qis::QisError) -> i32 {
    use cqlib_core::qis::QisError;
    match err {
        QisError::IndexOutOfBounds { .. } => -2,
        QisError::InvalidParameterValue(_) | QisError::InvalidStateDimension(_) => -8,
        QisError::NotNormalized => -8,
        QisError::QubitMismatch { .. } => -8,
        QisError::CircuitError(_) => -3,
        QisError::UnsupportedOperation(_) => -7,
        _ => -7,
    }
}

pub use density_matrix::*;
pub use density_matrix_noise::*;
pub use entropy::*;
pub use hamiltonian::*;
pub use measurement::*;
pub use metrics::*;
pub use pauli::*;
pub use stabilizer::*;
pub use statevector::*;
