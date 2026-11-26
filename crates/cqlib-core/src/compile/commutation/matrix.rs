// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Small local-matrix fallback for commutation checks.
//!
//! This module proves commutation by embedding both operations into the matrix
//! space of their combined qubit support, multiplying them in both orders, and
//! comparing the products up to a global phase.  It is intended as a bounded
//! fallback for concrete parameters after cheaper symbolic checks fail.
//!
//! The fallback does not attempt symbolic evaluation.  Any parameter that
//! cannot be evaluated to a concrete `f64`, unsupported matrix construction, or
//! support wider than the configured qubit bound produces `None`.

use crate::circuit::{Instruction, Parameter, Qubit};
use crate::compile::commutation::checker::{Commutation, CommutationResult};
use crate::compile::{NUMERIC_ZERO_TOLERANCE, UNIT_PHASE_NORM_TOLERANCE};
use ndarray::Array2;
use num_complex::Complex64;
use std::borrow::Cow;

const MATRIX_TOLERANCE: f64 = 1e-10;

/// Attempts matrix-based commutation on the union support of two operations.
///
/// The returned phase satisfies `lhs * rhs = exp(i * phase) * rhs * lhs` on the
/// expanded local Hilbert space.  The caller controls the maximum support size
/// to prevent exponential blow-up.
pub fn matrix_commutation(
    lhs_inst: &Instruction,
    lhs_qubits: &[Qubit],
    lhs_params: &[Parameter],
    rhs_inst: &Instruction,
    rhs_qubits: &[Qubit],
    rhs_params: &[Parameter],
    max_matrix_qubits: usize,
) -> CommutationResult {
    let mut combined_qubits = Vec::with_capacity(lhs_qubits.len() + rhs_qubits.len());
    for &qubit in lhs_qubits.iter().chain(rhs_qubits) {
        if !combined_qubits.contains(&qubit) {
            combined_qubits.push(qubit);
        }
    }
    combined_qubits.sort_unstable();

    if combined_qubits.len() > max_matrix_qubits {
        return None;
    }

    let lhs_values = lhs_params
        .iter()
        .map(|param| param.evaluate(&None).ok())
        .collect::<Option<Vec<_>>>()?;
    let rhs_values = rhs_params
        .iter()
        .map(|param| param.evaluate(&None).ok())
        .collect::<Option<Vec<_>>>()?;
    let lhs_matrix = lhs_inst.matrix(&lhs_values)?;
    let rhs_matrix = rhs_inst.matrix(&rhs_values)?;

    let lhs_positions = lhs_qubits
        .iter()
        .map(|qubit| combined_qubits.iter().position(|item| item == qubit))
        .collect::<Option<Vec<_>>>()?;
    let rhs_positions = rhs_qubits
        .iter()
        .map(|qubit| combined_qubits.iter().position(|item| item == qubit))
        .collect::<Option<Vec<_>>>()?;

    let expanded_lhs = expand_unitary(lhs_matrix, &lhs_positions, combined_qubits.len())?;
    let expanded_rhs = expand_unitary(rhs_matrix, &rhs_positions, combined_qubits.len())?;

    let lhs_rhs = expanded_lhs.dot(&expanded_rhs);
    let rhs_lhs = expanded_rhs.dot(&expanded_lhs);

    let phase = phase_between(&lhs_rhs, &rhs_lhs)?;

    if phase.abs() <= MATRIX_TOLERANCE {
        Some(Commutation::Exact)
    } else {
        let two_pi = 2.0 * std::f64::consts::PI;
        let mut normalized =
            (phase + std::f64::consts::PI).rem_euclid(two_pi) - std::f64::consts::PI;
        if normalized <= -std::f64::consts::PI {
            normalized += two_pi;
        }
        Some(Commutation::UpToGlobalPhase(Parameter::from(normalized)))
    }
}

/// Expands a gate matrix to the full local space described by `total_qubits`.
///
/// `gate_positions` maps each gate-local qubit to its position in the sorted
/// combined support used by [`matrix_commutation`].  Bit extraction follows the
/// gate-local matrix convention: earlier qubits correspond to more
/// significant bits in the local basis index.
fn expand_unitary(
    matrix: Cow<'_, Array2<Complex64>>,
    gate_positions: &[usize],
    total_qubits: usize,
) -> Option<Array2<Complex64>> {
    let dim = 1usize.checked_shl(total_qubits as u32)?;

    if gate_positions.is_empty() {
        if matrix.nrows() != 1 || matrix.ncols() != 1 {
            return None;
        }
        let mut expanded = Array2::<Complex64>::eye(dim);
        let scalar = matrix[[0, 0]];
        expanded.iter_mut().for_each(|value| *value *= scalar);
        return Some(expanded);
    }

    let expected = 1usize.checked_shl(gate_positions.len() as u32)?;
    if matrix.nrows() != expected || matrix.ncols() != expected {
        return None;
    }
    if gate_positions
        .iter()
        .any(|&position| position >= total_qubits)
    {
        return None;
    }

    let mut xor_mask = dim - 1;
    for &position in gate_positions {
        xor_mask ^= 1usize << (total_qubits - 1 - position);
    }

    let mut mapped_indices = vec![0usize; dim];
    for (index, mapped) in mapped_indices.iter_mut().enumerate() {
        for (gate_index, &position) in gate_positions.iter().enumerate() {
            let bit = total_qubits - 1 - position;
            if (index & (1usize << bit)) != 0 {
                *mapped |= 1usize << (gate_positions.len() - 1 - gate_index);
            }
        }
    }

    let mut expanded = Array2::<Complex64>::zeros((dim, dim));
    for row in 0..dim {
        for col in 0..dim {
            if (row & xor_mask) == (col & xor_mask) {
                expanded[[row, col]] = matrix[[mapped_indices[row], mapped_indices[col]]];
            }
        }
    }
    Some(expanded)
}

/// Returns the global phase relating two matrices, if they differ only by one.
///
/// The first non-zero entry establishes the candidate unit complex ratio; every
/// entry is then checked against that ratio with matrix tolerance.  Mismatched
/// zero structure or a non-unit candidate ratio means the matrices are not equal
/// up to global phase.
fn phase_between(lhs: &Array2<Complex64>, rhs: &Array2<Complex64>) -> Option<f64> {
    if lhs.dim() != rhs.dim() {
        return None;
    }

    let mut ratio = None;
    for (&lhs_value, &rhs_value) in lhs.iter().zip(rhs.iter()) {
        let lhs_zero = lhs_value.norm() <= NUMERIC_ZERO_TOLERANCE;
        let rhs_zero = rhs_value.norm() <= NUMERIC_ZERO_TOLERANCE;
        if lhs_zero != rhs_zero {
            return None;
        }
        if lhs_zero {
            continue;
        }

        let candidate = lhs_value / rhs_value;
        let norm = candidate.norm();
        if !norm.is_finite() || (norm - 1.0).abs() > UNIT_PHASE_NORM_TOLERANCE {
            return None;
        }
        ratio = Some(candidate);
        break;
    }

    let ratio = ratio.unwrap_or_else(|| Complex64::new(1.0, 0.0));
    for (&lhs_value, &rhs_value) in lhs.iter().zip(rhs.iter()) {
        if (lhs_value - ratio * rhs_value).norm() > MATRIX_TOLERANCE {
            return None;
        }
    }

    Some(ratio.im.atan2(ratio.re))
}
