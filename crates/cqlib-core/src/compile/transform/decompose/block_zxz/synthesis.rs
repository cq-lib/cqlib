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

//! Recursive synthesis of a numeric unitary, including its global phase.
//! [Krol & Al-Ars, §§4–5](https://arxiv.org/html/2403.13692v2#S4).
//! Each layer splits the highest matrix bit. Local circuit qubit zero is
//! the least significant bit. The four recursive blocks are emitted in
//! execution order, sharing the A.2 diagonal across recursive boundaries.

use super::factors::{demux, is_block_diagonal, nearest_unitary, split, validate_input};
use super::leaf;
use super::ucrz::{self, Chain};
use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::synthesize_numeric_1q_unitary;
use ndarray::{Array1, Array2, s};
use num_complex::Complex64;

pub(super) type Matrix = Array2<Complex64>;

/// Controls the two independent CX reductions in Section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockZxzDecomposeOptions {
    /// Absorb the two outer UCRZ boundary CX gates into the middle multiplexer.
    pub opt_a1: bool,
    /// Transfer a diagonal from each two-qubit leaf to its successor.
    pub opt_a2: bool,
    /// Allow diagonal transfer on block-diagonal inputs when A.2 is enabled.
    pub opt_a2_force: bool,
}

impl Default for BlockZxzDecomposeOptions {
    fn default() -> Self {
        Self {
            opt_a1: true,
            opt_a2: true,
            opt_a2_force: false,
        }
    }
}

pub(super) fn failed(reason: String) -> CompilerError {
    CompilerError::TransformFailed {
        name: "block_zxz",
        reason,
    }
}

/// Synthesizes a finite square 2^n-dimensional unitary (n ≥ 1) in the
/// U/RZ/H/CX basis. The circuit includes the scalar synthesis phase.
/// Both CX optimizations default on, with automatic A.2 suppression for
/// block-diagonal inputs. See [`decompose_block_zxz_with_options`] for
/// absolute tolerance and error semantics.
pub fn decompose_block_zxz(
    matrix: &Array2<Complex64>,
    atol: f64,
) -> Result<Circuit, CompilerError> {
    decompose_block_zxz_with_options(matrix, atol, &BlockZxzDecomposeOptions::default())
}

/// Synthesizes a matrix in computational basis order, with local q0 as
/// its least significant bit. The returned circuit reconstructs the full
/// input matrix, including global phase. Numerical fallback can add CX gates.
///
/// `atol` is a finite, nonnegative absolute tolerance. It validates input
/// unitarity (`||M†M - I||_max <= atol`) and governs reconstruction checks on
/// demultiplexers and two-qubit leaves. Near-unitary inputs that pass input
/// validation are projected onto the nearest exact unitary `U_proj` (in every
/// unitarily invariant norm) when needed, and synthesis reconstructs `U_proj`
/// within `atol`. A KAK leaf without a validated CX template is split once
/// more into one-qubit gates and checked against the original 4×4 matrix.
///
/// # Errors
/// Returns an input error for invalid tolerance, dimensions, non-finite
/// entries or failed unitarity. Numerical factorization or leaf synthesis
/// can fail when its residual exceeds the applicable tolerance.
pub fn decompose_block_zxz_with_options(
    matrix: &Array2<Complex64>,
    atol: f64,
    options: &BlockZxzDecomposeOptions,
) -> Result<Circuit, CompilerError> {
    let unitarity_error = validate_input(matrix, atol)?;
    let mut options = *options;
    options.opt_a2 &= options.opt_a2_force || !is_block_diagonal(matrix, atol);
    // Project near-unitary inputs that exceed floating-point roundoff onto the
    // nearest exact unitary before recursive splitting so coherent low-amplitude
    // perturbations are not amplified across layers. If an unprojected input
    // with nonzero unitarity error still fails numerically, retry on the polar
    // projection.
    let roundoff = 16.0 * (matrix.nrows() as f64).sqrt() * f64::EPSILON;
    if unitarity_error > roundoff {
        let projected = nearest_unitary(matrix)?;
        let mut synthesis = Synthesis::new(projected.nrows(), atol, options);
        synthesis.append(&projected, true)?;
        return Ok(synthesis.output);
    }
    let mut synthesis = Synthesis::new(matrix.nrows(), atol, options);
    match synthesis.append(matrix, true) {
        Ok(()) => Ok(synthesis.output),
        Err(CompilerError::TransformFailed { .. }) if unitarity_error > 0.0 => {
            let projected = nearest_unitary(matrix)?;
            let mut retry = Synthesis::new(projected.nrows(), atol, options);
            retry.append(&projected, true)?;
            Ok(retry.output)
        }
        Err(err) => Err(err),
    }
}

/// All recursive layers write to the same circuit. A pending diagonal belongs
/// to the two lowest qubits and survives intermediate gates on higher targets.
pub(super) struct Synthesis {
    pub(super) output: Circuit,
    pending: Option<Array1<Complex64>>,
    atol: f64,
    options: BlockZxzDecomposeOptions,
}

impl Synthesis {
    pub(super) fn new(dim: usize, atol: f64, options: BlockZxzDecomposeOptions) -> Self {
        Self {
            output: Circuit::new(dim.ilog2() as usize),
            pending: None,
            atol,
            options,
        }
    }

    fn append(&mut self, u: &Matrix, last: bool) -> Result<(), CompilerError> {
        match u.nrows() {
            2 => {
                let gate = synthesize_numeric_1q_unitary(u)?;
                self.output
                    .u(Qubit::new(0), gate.theta, gate.phi, gate.lambda)?;
                self.output.set_global_phase(
                    self.output.global_phase() + Parameter::from(gate.global_phase),
                );
                Ok(())
            }
            4 => leaf::append(
                &mut self.output,
                u,
                &mut self.pending,
                self.options.opt_a2 && !last,
                self.atol,
            ),
            // A tolerance is appropriate for the optimization policy above, but
            // skipping the off-diagonal blocks must preserve every nonzero entry.
            _ if is_block_diagonal(u, 0.0) => self.append_block_diagonal(u, last),
            _ => self.append_split(u, last),
        }
    }

    fn append_block_diagonal(&mut self, u: &Matrix, last: bool) -> Result<(), CompilerError> {
        let k = u.nrows() / 2;
        let upper = u.slice(s![..k, ..k]);
        let lower = u.slice(s![k.., k..]);
        if upper == lower {
            // I ⊗ upper leaves the high qubit idle, including for a scalar phase.
            return self.append(&upper.to_owned(), last);
        }
        let mux = demux(&upper.to_owned(), &lower.to_owned(), self.atol)?;
        self.append(&mux.w, false)?;
        ucrz::append(&mut self.output, &mux.d, Chain::Full)?;
        self.append(&mux.v, last)
    }

    pub(super) fn append_split(&mut self, u: &Matrix, last: bool) -> Result<(), CompilerError> {
        let blocks = split(u)?;
        let k = u.nrows() / 2;
        let c = demux(&Matrix::eye(k), &blocks.c, self.atol)?;
        let a = demux(&blocks.a1, &blocks.a2, self.atol)?;
        let upper = a.w.dot(&c.v);
        let mut lower = a.w.dot(&blocks.b).dot(&c.v);
        if self.options.opt_a1 {
            // Eq. (11): moving the closing CX through H turns it into CZ.
            // In the lower branch conjugation by Z ⊗ I flips these two blocks.
            for ((i, j), value) in lower.indexed_iter_mut() {
                if (i < k / 2) != (j < k / 2) {
                    *value = -*value;
                }
            }
        }
        let b = demux(&upper, &lower, self.atol)?;
        let top = Qubit::new(k.ilog2());
        self.append(&c.w, false)?;
        ucrz::append(
            &mut self.output,
            &c.d,
            if self.options.opt_a1 {
                Chain::OpenTail
            } else {
                Chain::Full
            },
        )?;
        self.output.h(top)?;
        self.append(&b.w, false)?;
        ucrz::append(&mut self.output, &b.d, Chain::Full)?;
        self.append(&b.v, false)?;
        self.output.h(top)?;
        ucrz::append(
            &mut self.output,
            &a.d,
            if self.options.opt_a1 {
                Chain::OpenHead
            } else {
                Chain::Full
            },
        )?;
        self.append(&a.v, last)
    }
}

#[cfg(test)]
#[path = "./synthesis_test.rs"]
pub(super) mod synthesis_test;
