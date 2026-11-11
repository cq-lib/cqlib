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

//! Block-ZXZ step 3: Gray-code expansion of uniformly controlled `Rz`
//! (UCRz) gates.
//!
//! Function-by-function mirror of the Python `decompose/block_ZXZ/ucrz.py`.

use super::matrix::Cm;
use super::step1::{is_matrix_diagonal, is_matrix_unitary};
use crate::circuit::{Circuit, Qubit};
use num_complex::Complex64;

/// Returns the binary reflected Gray code of `index`.
pub fn gray_code(index: usize) -> usize {
    index ^ (index >> 1)
}

/// Returns the Gray-code sequence of length `2^k`.
pub fn gray_code_sequence(num_controls: usize) -> Vec<usize> {
    (0..(1usize << num_controls)).map(gray_code).collect()
}

/// Returns the local CNOT control switching order.
///
/// Uses the local numbering convention `0..num_controls-1` and includes the
/// closing CNOT (the cyclic transition between the last and first codes).
pub fn gray_control_positions(num_controls: usize) -> Vec<usize> {
    if num_controls == 0 {
        return Vec::new();
    }
    let sequence = gray_code_sequence(num_controls);
    let len = sequence.len();
    let mut controls = Vec::with_capacity(len);
    for index in 0..len {
        let current = sequence[index];
        let next = sequence[(index + 1) % len];
        let diff = current ^ next;
        controls.push(diff.trailing_zeros() as usize);
    }
    controls
}

/// Parity of the bitwise inner product (popcount of `left & right` mod 2).
fn binary_dot_parity(left: usize, right: usize) -> usize {
    (left & right).count_ones() as usize % 2
}

/// Builds the Gray matrix `M^k` (real matrix,
/// `M[i, j] = (-1)^{bit(i) . gray(j)}`).
pub fn gray_matrix(num_controls: usize) -> Cm {
    let dim = 1usize << num_controls;
    let gray_codes = gray_code_sequence(num_controls);
    let mut out = Cm::zeros((dim, dim));
    for row in 0..dim {
        for (col, &gray) in gray_codes.iter().enumerate() {
            let sign = if binary_dot_parity(row, gray) == 0 {
                1.0
            } else {
                -1.0
            };
            out[[row, col]] = Complex64::new(sign, 0.0);
        }
    }
    out
}

/// Extracts the UCRz target angles `alpha = -2 * arg(D)` from a diagonal
/// unitary `D`.
pub fn extract_ucrz_alpha_from_diagonal(d_matrix: &Cm, atol: f64) -> Result<Vec<f64>, String> {
    let (r, c) = d_matrix.dim();
    if r != c {
        return Err("D must be a square matrix.".into());
    }
    if !is_matrix_diagonal(d_matrix, atol) {
        return Err("D must be diagonal.".into());
    }
    if !is_matrix_unitary(d_matrix, atol) {
        return Err("D must be unitary.".into());
    }
    Ok((0..r).map(|i| -2.0 * d_matrix[[i, i]].arg()).collect())
}

/// Solves the Gray-code linear system `M^k theta = alpha`; returns
/// `(theta, M^k)`.
pub fn solve_ucrz_theta_from_alpha(alpha: &[f64]) -> Result<(Vec<f64>, Cm), String> {
    if !is_power_of_two(alpha.len()) {
        return Err("alpha length must be a power of two.".into());
    }
    let num_controls = (alpha.len() as f64).log2().round() as usize;
    let m = gray_matrix(num_controls);
    // theta = (M^T @ alpha) / 2^k
    let dim = alpha.len();
    let mut theta = vec![0.0f64; dim];
    for row in 0..dim {
        let mut sum = 0.0;
        for (col, &a) in alpha.iter().enumerate() {
            sum += m[[col, row]].re * a;
        }
        theta[row] = sum / (dim as f64);
    }
    Ok((theta, m))
}

fn is_power_of_two(value: usize) -> bool {
    value > 0 && (value & (value - 1)) == 0
}

/// Derives the UCRz numeric description from `D` (mirrors the Python
/// `step3_ucrz_from_diagonal`).
pub fn step3_ucrz_from_diagonal(d_matrix: &Cm, atol: f64) -> Result<UcrzData, String> {
    let alpha = extract_ucrz_alpha_from_diagonal(d_matrix, atol)?;
    let (theta, gray_matrix) = solve_ucrz_theta_from_alpha(&alpha)?;
    let num_controls = (alpha.len() as f64).log2().round() as usize;
    Ok(UcrzData {
        alpha,
        theta,
        gray_codes: gray_code_sequence(num_controls),
        controls: gray_control_positions(num_controls),
        gray_matrix,
        num_controls,
        target: num_controls,
    })
}

/// Synthesizes a local UCRz circuit from the Gray-expanded angles.
///
/// Mirrors the Python `synthesize_ucrz_circuit`.
pub fn synthesize_ucrz_circuit(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    if theta.is_empty() {
        return Err("theta must contain at least one rotation angle.".into());
    }
    if !is_power_of_two(theta.len()) {
        return Err("theta length must be a power of two.".into());
    }
    let num_controls = (theta.len() as f64).log2().round() as usize;

    if num_controls == 0 {
        if !controls.is_empty() {
            return Err("For zero controls, controls must be empty.".into());
        }
        if target != 0 {
            return Err("For zero controls, the local target must be 0.".into());
        }
        let mut circuit = Circuit::new(1);
        circuit
            .rz(Qubit::new(0), theta[0])
            .map_err(|e| e.to_string())?;
        return Ok(circuit);
    }

    if controls.len() != theta.len() {
        return Err("controls and theta must have the same length.".into());
    }
    let expected_controls = gray_control_positions(num_controls);
    if controls != &expected_controls[..] {
        return Err("controls must follow the local Gray-code convention.".into());
    }
    if target != num_controls {
        return Err("Under local numbering, target must equal num_controls.".into());
    }

    let mut circuit = Circuit::new(num_controls + 1);
    for (&angle, &control) in theta.iter().zip(controls.iter()) {
        circuit
            .rz(Qubit::new(target as u32), angle)
            .map_err(|e| e.to_string())?;
        circuit
            .cx(Qubit::new(control as u32), Qubit::new(target as u32))
            .map_err(|e| e.to_string())?;
    }
    Ok(circuit)
}

/// A.1 optimization: the UCRz chain with the trailing closing CNOT removed
/// (used for the C-side chain).
///
/// The standard chain enacts, in time order,
/// `[Rz(t0), C0, Rz(t1), C1, ..., Rz(tm), CXc]` whose matrix is
/// `M = CXc @ Rest`; this function returns the circuit for `Rest` (one CNOT
/// fewer). During layer assembly the missing CNOT is absorbed into the
/// B-tilde demultiplexer via the identity `H @ CXc = CZ @ H`.
pub fn synthesize_ucrz_circuit_reduced_tail(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    let mut circuit = synthesize_ucrz_circuit(theta, controls, target)?;
    // Drop the temporally last gate (the closing CX).
    let n = circuit.operations().len();
    circuit.remove_operation(n - 1).map_err(|e| e.to_string())?;
    Ok(circuit)
}

/// A.1 optimization: the time-reversed UCRz chain without the leading
/// closing CNOT (used for the A-side chain).
///
/// Because every `Rz` and `CX` matrix is symmetric, the fully reversed chain
/// `[CXc, Rz(tm), C(m-1), Rz(t(m-1)), ..., C0, Rz(t0)]` implements the very
/// same diagonal unitary as the forward chain. This function drops the
/// leading `CXc` (absorbed next to the Hadamard on its earlier side through
/// `CXc @ H = H @ CZ` into the B-tilde demultiplexer) and returns the rest.
pub fn synthesize_ucrz_circuit_reversed_reduced_head(
    theta: &[f64],
    controls: &[usize],
    target: usize,
) -> Result<Circuit, String> {
    if theta.is_empty() || !is_power_of_two(theta.len()) {
        return Err("theta length must be a positive power of two.".into());
    }
    let num_controls = (theta.len() as f64).log2().round() as usize;

    if num_controls == 0 {
        // A single Rz has no CNOT to remove.
        let mut circuit = Circuit::new(1);
        circuit
            .rz(Qubit::new(0), theta[0])
            .map_err(|e| e.to_string())?;
        return Ok(circuit);
    }

    if controls.len() != theta.len() {
        return Err("controls and theta must have the same length.".into());
    }
    if controls != &gray_control_positions(num_controls)[..] {
        return Err("controls must follow the local Gray-code convention.".into());
    }
    if target != num_controls {
        return Err("Under local numbering, target must equal num_controls.".into());
    }

    // Time order (leading CXc removed):
    // [Rz(tm), C(m-1), Rz(t(m-1)), ..., C0, Rz(t0)]
    let m = theta.len() - 1;
    let mut circuit = Circuit::new(num_controls + 1);
    circuit
        .rz(Qubit::new(target as u32), theta[m])
        .map_err(|e| e.to_string())?;
    for idx in (0..m).rev() {
        circuit
            .cx(Qubit::new(controls[idx] as u32), Qubit::new(target as u32))
            .map_err(|e| e.to_string())?;
        circuit
            .rz(Qubit::new(target as u32), theta[idx])
            .map_err(|e| e.to_string())?;
    }
    Ok(circuit)
}

/// UCRz numeric description (mirrors the Python `dict` return value).
#[derive(Debug, Clone)]
pub struct UcrzData {
    /// Target UCRz angle vector.
    pub alpha: Vec<f64>,
    /// Gray-expanded circuit rotation angle vector.
    pub theta: Vec<f64>,
    /// Gray-code sequence.
    pub gray_codes: Vec<usize>,
    /// Local control positions.
    pub controls: Vec<usize>,
    /// Gray matrix `M^k`.
    pub gray_matrix: Cm,
    /// Number of control qubits `k`.
    pub num_controls: usize,
    /// Local target qubit (equals `num_controls` under this convention).
    pub target: usize,
}

/// Turns `D` into the UCRz numeric data plus a local circuit in one step.
///
/// Mirrors the Python `step3_ucrz_to_circuit`.
pub fn step3_ucrz_to_circuit(d: &Cm, atol: f64) -> Result<UcrzCircuitData, String> {
    let data = step3_ucrz_from_diagonal(d, atol)?;
    let circuit = synthesize_ucrz_circuit(&data.theta, &data.controls, data.target)?;
    Ok(UcrzCircuitData { data, circuit })
}

/// UCRz result including the circuit.
#[derive(Debug)]
pub struct UcrzCircuitData {
    pub data: UcrzData,
    pub circuit: Circuit,
}
