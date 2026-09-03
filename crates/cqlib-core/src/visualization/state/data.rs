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

//! State validation and density-matrix conversion for visualization.
//!
//! Functions in this module normalize core QIS state types into row-major density
//! matrices used by state plotting backends.

use crate::qis::{DensityMatrix, Statevector};
use crate::visualization::VisualizationError;
use num_complex::Complex64;

/// Core state types accepted by state visualization functions.
///
/// This trait is an adapter over Cqlib's QIS state objects, not a separate visualization
/// data container. Public plotting APIs stay generic so callers can pass `Statevector` and
/// `DensityMatrix` directly.
///
/// Implementors must return row-major density-matrix data with length `4^num_qubits`.
/// Invalid dimensions should be reported as [`VisualizationError::InvalidInput`].
pub trait StateVisualizationSource {
    /// Number of qubits represented by the state.
    fn num_qubits(&self) -> usize;

    /// Return row-major density-matrix data for visualization.
    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError>;
}

impl StateVisualizationSource for Statevector {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError> {
        let num_qubits = self.num_qubits;
        let data = self.data();
        let dim = checked_dim(num_qubits)?;
        let density_len = checked_density_len(num_qubits)?;
        if data.len() != dim {
            return Err(VisualizationError::InvalidInput(format!(
                "statevector length {} does not match 2^{num_qubits}",
                data.len()
            )));
        }
        let mut rho = vec![Complex64::new(0.0, 0.0); density_len];
        for row in 0..dim {
            for col in 0..dim {
                rho[row * dim + col] = data[row] * data[col].conj();
            }
        }
        Ok(rho)
    }
}

impl StateVisualizationSource for DensityMatrix {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError> {
        let num_qubits = self.num_qubits;
        let density_len = checked_density_len(num_qubits)?;
        let data = self.data();
        if data.len() != density_len {
            return Err(VisualizationError::InvalidInput(format!(
                "density matrix length {} does not match 4^{num_qubits}",
                data.len()
            )));
        }
        Ok(data.to_vec())
    }
}

/// Convert a core state object into a row-major density matrix.
///
/// For a pure state, this forms $\rho = |\psi\rangle\langle\psi|$.
///
/// # Arguments
///
/// * `state` - Input state object with dimensions validated against `num_qubits`.
///
/// # Returns
///
/// A tuple `(num_qubits, rho)` where `rho` is stored in row-major order with length
/// `4^num_qubits`.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when qubit counts or buffer lengths are
/// inconsistent.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::state_to_density_matrix;
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let (num_qubits, rho) = state_to_density_matrix(&state).unwrap();
/// assert_eq!(num_qubits, 1);
/// assert_eq!(rho.len(), 4);
/// ```
pub fn state_to_density_matrix<S: StateVisualizationSource + ?Sized>(
    state: &S,
) -> Result<(usize, Vec<Complex64>), VisualizationError> {
    let num_qubits = state.num_qubits();
    let rho = state.density_matrix_data()?;
    let density_len = checked_density_len(num_qubits)?;
    if rho.len() != density_len {
        return Err(VisualizationError::InvalidInput(format!(
            "density matrix length {} does not match 4^{num_qubits}",
            rho.len()
        )));
    }
    Ok((num_qubits, rho))
}

/// Compute Hilbert-space dimension `2^num_qubits` with overflow protection.
pub(crate) fn checked_dim(num_qubits: usize) -> Result<usize, VisualizationError> {
    if num_qubits >= usize::BITS as usize {
        return Err(VisualizationError::InvalidInput(
            "too many qubits for state visualization".to_string(),
        ));
    }
    Ok(1usize << num_qubits)
}

/// Computes density-matrix storage length `4^num_qubits` with overflow protection.
///
/// State-city and Pauli-vector plots operate on flattened density matrices. This helper
/// keeps all matrix-size checks on a single path so large states fail with a clear
/// visualization error before allocation or indexing.
pub(crate) fn checked_density_len(num_qubits: usize) -> Result<usize, VisualizationError> {
    let dim = checked_dim(num_qubits)?;
    dim.checked_mul(dim).ok_or_else(|| {
        VisualizationError::InvalidInput("too many qubits for state visualization".to_string())
    })
}

/// Build computational-basis labels in display order.
///
/// Labels are generated from little-endian basis indices and optionally reversed for
/// frontends that display the most-significant qubit on the opposite side.
pub(crate) fn basis_labels(num_qubits: usize, reverse_bits: bool) -> Vec<String> {
    let dim = 1usize << num_qubits;
    (0..dim)
        .map(|value| {
            let raw = format!("{value:0num_qubits$b}");
            if reverse_bits {
                raw.chars().rev().collect()
            } else {
                raw
            }
        })
        .collect()
}
