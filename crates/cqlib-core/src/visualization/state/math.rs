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

//! Bloch-vector and Pauli expectation utilities for state visualization.
//!
//! This module computes reduced single-qubit Bloch vectors and Pauli-basis coefficients
//! from density-matrix data produced by [`state_to_density_matrix`].

use super::data::{StateVisualizationSource, checked_density_len, state_to_density_matrix};
use crate::visualization::VisualizationError;
use num_complex::Complex64;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Pauli {
    /// Identity operator.
    I,
    /// Pauli-X bit-flip operator.
    X,
    /// Pauli-Y operator with phase convention `Y|0> = i|1>`, `Y|1> = -i|0>`.
    Y,
    /// Pauli-Z phase-flip operator.
    Z,
}

/// Compute one reduced single-qubit Bloch vector for each qubit.
///
/// Each returned vector contains expectation values
/// $\langle X \rangle$, $\langle Y \rangle$, and $\langle Z \rangle$ for the
/// corresponding reduced single-qubit state.
///
/// # Arguments
///
/// * `state` - Input core state object accepted by state visualization routines.
///
/// # Returns
///
/// A vector of `(qubit_index, [x, y, z])` tuples with components clamped to the unit
/// sphere when numerical noise exceeds physical bounds.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the state payload is invalid.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::local_bloch_vectors;
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let vectors = local_bloch_vectors(&state).unwrap();
/// assert_eq!(vectors.len(), 1);
/// assert!((vectors[0].1[2] - 1.0).abs() < 1e-10);
/// ```
pub fn local_bloch_vectors<S: StateVisualizationSource + ?Sized>(
    state: &S,
) -> Result<Vec<(usize, [f64; 3])>, VisualizationError> {
    let (num_qubits, rho) = state_to_density_matrix(state)?;
    let mut vectors = Vec::with_capacity(num_qubits);
    for qubit in 0..num_qubits {
        let x = pauli_expectation(num_qubits, &rho, &[(qubit, Pauli::X)]).re;
        let y = pauli_expectation(num_qubits, &rho, &[(qubit, Pauli::Y)]).re;
        let z = pauli_expectation(num_qubits, &rho, &[(qubit, Pauli::Z)]).re;
        vectors.push((qubit, clamp_bloch([clean(x), clean(y), clean(z)])));
    }
    Ok(vectors)
}

/// Compute all Pauli-basis coefficients for a density matrix.
///
/// The returned labels enumerate tensor products over `I`, `X`, `Y`, and `Z`.
/// Values are expectation coefficients `Tr(rho P)` in the same qubit indexing convention
/// used by the statevector basis: qubit `q` corresponds to bit `1 << q`.
pub(crate) fn pauli_coefficients(
    num_qubits: usize,
    rho: &[Complex64],
    reverse_bits: bool,
) -> Result<Vec<(String, f64)>, VisualizationError> {
    let total = checked_density_len(num_qubits)?;
    if rho.len() != total {
        return Err(VisualizationError::InvalidInput(format!(
            "density matrix length {} does not match 4^{num_qubits}",
            rho.len()
        )));
    }

    let mut out = Vec::with_capacity(total);
    for idx in 0..total {
        let mut code = idx;
        let mut ops = Vec::with_capacity(num_qubits);
        for qubit in 0..num_qubits {
            let op = match code % 4 {
                0 => Pauli::I,
                1 => Pauli::X,
                2 => Pauli::Y,
                _ => Pauli::Z,
            };
            ops.push((qubit, op));
            code /= 4;
        }
        let display_qubits: Vec<usize> = if reverse_bits {
            (0..num_qubits).rev().collect()
        } else {
            (0..num_qubits).collect()
        };
        let label = display_qubits
            .iter()
            .map(|qubit| match ops[*qubit].1 {
                Pauli::I => 'I',
                Pauli::X => 'X',
                Pauli::Y => 'Y',
                Pauli::Z => 'Z',
            })
            .collect();
        let value = clean(pauli_expectation(num_qubits, rho, &ops).re);
        out.push((label, value));
    }
    Ok(out)
}

/// Compute `Tr(rho P)` for a sparse Pauli product.
///
/// `rho` is a row-major `2^n x 2^n` density matrix. Each `(qubit, Pauli)` pair acts on
/// the bit position `1 << qubit`, matching Cqlib's little-endian state indexing.
pub(crate) fn pauli_expectation(
    num_qubits: usize,
    rho: &[Complex64],
    ops: &[(usize, Pauli)],
) -> Complex64 {
    let dim = 1usize << num_qubits;
    let mut result = Complex64::new(0.0, 0.0);
    for basis in 0..dim {
        let mut mapped = basis;
        let mut phase = Complex64::new(1.0, 0.0);
        for &(qubit, op) in ops {
            let bit = (basis >> qubit) & 1;
            match op {
                Pauli::I => {}
                Pauli::X => mapped ^= 1usize << qubit,
                Pauli::Y => {
                    mapped ^= 1usize << qubit;
                    phase *= if bit == 0 {
                        Complex64::new(0.0, 1.0)
                    } else {
                        Complex64::new(0.0, -1.0)
                    };
                }
                Pauli::Z => {
                    if bit == 1 {
                        phase = -phase;
                    }
                }
            }
        }
        result += phase * rho[basis * dim + mapped];
    }
    result
}

/// Remove tiny numerical noise from values displayed in plots.
pub(crate) fn clean(value: f64) -> f64 {
    if value.abs() < 1e-12 { 0.0 } else { value }
}

/// Project slightly non-physical Bloch vectors back onto the unit sphere.
///
/// This only corrects numerical overshoot; valid vectors inside the sphere are unchanged.
pub(crate) fn clamp_bloch(mut vector: [f64; 3]) -> [f64; 3] {
    let norm = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if norm > 1.0 {
        for item in &mut vector {
            *item /= norm;
        }
    }
    vector
}
