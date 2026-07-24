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

//! Core symbolic matrix types and utilities.
//!
//! This module defines [`SymbolicComplex`], [`SymbolicMatrix`], and the
//! low-level matrix manipulation helpers shared across the `symbolic_matrix`
//! submodule:
//!
//! - [`symbolic_eye`] — identity matrix construction,
//! - [`evaluate_symbolic_matrix`] — deferred evaluation with parameter bindings,
//! - [`substitute_symbolic_matrix`] — simultaneous symbol replacement,
//! - [`apply_symbolic_diagonal_gate`], [`apply_symbolic_permutation_gate`],
//!   [`apply_numeric_diagonal_gate`], [`apply_numeric_permutation_gate`] —
//!   fast-path gate application helpers.
//!
//! The internal parallel-safety helper `UnsafeSymbolicSlice` lives here because it
//! is shared by both the symbolic and numeric gate-application paths.

use crate::circuit::CircuitError;
use crate::circuit::error::ParameterError;
use crate::circuit::parameter::Parameter;
use crate::circuit::symbolic_matrix::PARALLEL_THRESHOLD_OPS;
use ndarray::Array2;
use ndarray::parallel::prelude::*;
use num_complex::Complex64;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Mul, Neg, Sub};

/// Complex value whose real and imaginary components are symbolic parameters.
///
/// Arithmetic preserves symbolic expressions, allowing matrices to remain
/// unevaluated until a parameter binding is supplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicComplex {
    /// Real part of the symbolic complex number.
    pub re: Parameter,
    /// Imaginary part of the symbolic complex number.
    pub im: Parameter,
}

/// A dense symbolic matrix in the same row-major ndarray shape used by the
/// numerical circuit matrix API.
pub type SymbolicMatrix = Array2<SymbolicComplex>;

impl Default for SymbolicComplex {
    /// Returns the additive identity `0 + 0i`.
    fn default() -> Self {
        Self::zero()
    }
}

impl SymbolicComplex {
    /// Creates a new `SymbolicComplex` from separate real and imaginary parts.
    ///
    /// Both arguments accept anything that implements [`Into<Parameter>`],
    /// so you can pass `f64`, [`Parameter`], or symbolic expressions directly.
    pub fn new(re: impl Into<Parameter>, im: impl Into<Parameter>) -> Self {
        Self {
            re: re.into(),
            im: im.into(),
        }
    }

    /// Returns the additive identity `0 + 0i`.
    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Returns the multiplicative identity `1 + 0i`.
    pub fn one() -> Self {
        Self::new(1.0, 0.0)
    }

    /// Returns the imaginary unit `0 + i`.
    pub fn i() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Creates a purely real symbolic complex number with zero imaginary part.
    pub fn from_real(value: impl Into<Parameter>) -> Self {
        Self::new(value, 0.0)
    }

    /// Creates a `SymbolicComplex` from a concrete [`Complex64`] value.
    pub fn from_complex(value: Complex64) -> Self {
        Self::new(value.re, value.im)
    }

    /// Builds `exp(i·θ)` as `cos(θ) + i·sin(θ)`.
    ///
    /// This is the fundamental building block for rotation-gate matrices
    /// (e.g. `RZ`, `Phase`, `GPhase`) and phase factors.
    pub fn exp_i(theta: impl Into<Parameter>) -> Self {
        let theta = theta.into();
        Self::new(theta.cos(), theta.sin())
    }

    /// Evaluates both the real and imaginary parts under the given parameter
    /// bindings and returns a concrete [`Complex64`].
    ///
    /// Returns [`ParameterError`] if any symbol required by either part is
    /// missing from `bindings`.
    pub fn evaluate(
        &self,
        bindings: &Option<HashMap<&str, f64>>,
    ) -> Result<Complex64, ParameterError> {
        Ok(Complex64::new(
            self.re.evaluate(bindings)?,
            self.im.evaluate(bindings)?,
        ))
    }

    /// Applies algebraic simplification to both the real and imaginary parts.
    ///
    /// Delegates to [`Parameter::simplify`] with domain-safe rules enabled.
    pub fn simplify(&self) -> Result<Self, ParameterError> {
        Ok(Self::new(self.re.simplify()?, self.im.simplify()?))
    }

    /// Replaces every occurrence of `symbol` in both the real and imaginary
    /// parts with the given `value`.
    ///
    /// This is used by [`substitute_symbolic_matrix`] to bind
    /// [`CircuitGate`](crate::circuit::CircuitGate) parameters.
    pub fn replace(&self, symbol: &str, value: impl Into<Parameter>) -> Self {
        let value = value.into();
        Self::new(
            self.re.replace(symbol, value.clone()),
            self.im.replace(symbol, value),
        )
    }

    /// Returns `true` if both the real and imaginary parts are exactly zero.
    pub fn is_zero_exact(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }

    /// Returns `true` if the real part is one and the imaginary part is zero.
    pub fn is_one_exact(&self) -> bool {
        self.re.is_one() && self.im.is_zero()
    }

    /// Returns `true` if this value simplifies to exactly zero.
    ///
    /// First checks [`Self::is_zero_exact`]; if that fails, applies
    /// [`simplify`](Self::simplify) and checks again.
    pub fn simplifies_to_zero(&self) -> Result<bool, ParameterError> {
        if self.is_zero_exact() {
            return Ok(true);
        }
        let simplified = self.simplify()?;
        Ok(simplified.is_zero_exact())
    }
}

// Arithmetic operator implementations for `SymbolicComplex`.
//
// Each binary operator is implemented for all four ownership combinations
// (owned/owned, owned/ref, ref/owned, ref/ref) to avoid unnecessary clones
// when one or both operands are consumed. Because `Parameter::clone` is O(1)
// (Arc reference-counted), the cost difference is small, but the explicit
// implementations allow the compiler to elide Arc increments when the
// owned value's fields can be moved instead of cloned.

impl Add for SymbolicComplex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Add<&SymbolicComplex> for SymbolicComplex {
    type Output = SymbolicComplex;

    fn add(self, rhs: &SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(self.re + rhs.re.clone(), self.im + rhs.im.clone())
    }
}

impl Add<SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn add(self, rhs: SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(self.re.clone() + rhs.re, self.im.clone() + rhs.im)
    }
}

impl Add<&SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn add(self, rhs: &SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(
            self.re.clone() + rhs.re.clone(),
            self.im.clone() + rhs.im.clone(),
        )
    }
}

/// Subtraction — same four ownership combinations as addition.
impl Sub for SymbolicComplex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Sub<&SymbolicComplex> for SymbolicComplex {
    type Output = SymbolicComplex;

    fn sub(self, rhs: &SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(self.re - rhs.re.clone(), self.im - rhs.im.clone())
    }
}

impl Sub<SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn sub(self, rhs: SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(self.re.clone() - rhs.re, self.im.clone() - rhs.im)
    }
}

impl Sub<&SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn sub(self, rhs: &SymbolicComplex) -> Self::Output {
        SymbolicComplex::new(
            self.re.clone() - rhs.re.clone(),
            self.im.clone() - rhs.im.clone(),
        )
    }
}

/// Complex multiplication: `(a + bi)(c + di) = (ac − bd) + (ad + bc)i`.
///
/// The four ownership variants let the compiler move (rather than clone)
/// `Parameter` fields from owned operands where possible.
impl Mul for SymbolicComplex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let re = self.re.clone() * rhs.re.clone() - self.im.clone() * rhs.im.clone();
        let im = self.re * rhs.im + self.im * rhs.re;
        Self::new(re, im)
    }
}

impl Mul<&SymbolicComplex> for SymbolicComplex {
    type Output = SymbolicComplex;

    fn mul(self, rhs: &SymbolicComplex) -> Self::Output {
        let re = self.re.clone() * rhs.re.clone() - self.im.clone() * rhs.im.clone();
        let im = self.re * rhs.im.clone() + self.im * rhs.re.clone();
        SymbolicComplex::new(re, im)
    }
}

impl Mul<SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn mul(self, rhs: SymbolicComplex) -> Self::Output {
        let re = self.re.clone() * rhs.re.clone() - self.im.clone() * rhs.im.clone();
        let im = self.re.clone() * rhs.im + self.im.clone() * rhs.re;
        SymbolicComplex::new(re, im)
    }
}

impl Mul<&SymbolicComplex> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn mul(self, rhs: &SymbolicComplex) -> Self::Output {
        let re = self.re.clone() * rhs.re.clone() - self.im.clone() * rhs.im.clone();
        let im = self.re.clone() * rhs.im.clone() + self.im.clone() * rhs.re.clone();
        SymbolicComplex::new(re, im)
    }
}

/// Unary negation — owned and reference variants.
impl Neg for SymbolicComplex {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.re, -self.im)
    }
}

impl Neg for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn neg(self) -> Self::Output {
        SymbolicComplex::new(-self.re.clone(), -self.im.clone())
    }
}

/// Complex64 × SymbolicComplex multiplication.
///
/// These four impls let numeric gate matrices (`Array2<Complex64>`) be applied
/// directly to a `SymbolicMatrix` without first wrapping every element in a
/// `SymbolicComplex`.  Because `Complex64` is `Copy`, only the owned/referenced
/// combinations for `SymbolicComplex` are needed.
impl Mul<Complex64> for SymbolicComplex {
    type Output = Self;

    fn mul(self, rhs: Complex64) -> Self::Output {
        Self::new(
            self.re.clone() * rhs.re - self.im.clone() * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl Mul<Complex64> for &SymbolicComplex {
    type Output = SymbolicComplex;

    fn mul(self, rhs: Complex64) -> Self::Output {
        SymbolicComplex::new(
            self.re.clone() * rhs.re - self.im.clone() * rhs.im,
            self.re.clone() * rhs.im + self.im.clone() * rhs.re,
        )
    }
}

impl Mul<SymbolicComplex> for Complex64 {
    type Output = SymbolicComplex;

    fn mul(self, rhs: SymbolicComplex) -> Self::Output {
        rhs * self
    }
}

impl Mul<&SymbolicComplex> for Complex64 {
    type Output = SymbolicComplex;

    fn mul(self, rhs: &SymbolicComplex) -> Self::Output {
        rhs * self
    }
}

/// Formats the symbolic complex number in a human-readable form.
///
/// - Pure real: `3.14`
/// - Pure imaginary: `2.0i`
/// - Mixed: `1.0 + 2.0i` or `1.0 - 2.0i` (negative imaginary part)
impl fmt::Display for SymbolicComplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im.is_zero() {
            write!(f, "{}", self.re)
        } else if self.re.is_zero() {
            write!(f, "{}i", self.im)
        } else {
            let im_str = self.im.to_string();
            if im_str.starts_with('-') {
                write!(f, "{} - {}i", self.re, im_str.trim_start_matches('-'))
            } else {
                write!(f, "{} + {}i", self.re, im_str)
            }
        }
    }
}

/// Internal prefix for temporary symbol names used during two-phase
/// substitution (see [`substitute_symbolic_matrix`]).
///
/// User-defined symbol names containing this prefix are rejected to prevent
/// collisions with the temporary names generated by the substitution algorithm.
const INTERNAL_SUB_PREFIX: &str = "__cqlib_internal_sub";

/// Simultaneously substitutes multiple symbols in a [`SymbolicMatrix`].
///
/// When replacement values reference symbols that are also keys in the
/// `replacements` map (e.g. swapping `a` and `b`), a two-phase approach is
/// used: symbols are first renamed to temporary names prefixed with
/// `INTERNAL_SUB_PREFIX`, then the temporary names are replaced with the
/// actual values. This avoids the non-deterministic ordering artefacts that
/// would arise from sequential substitution.
///
/// # Errors
///
/// - [`CircuitError::InvalidOperation`] if any key or replacement value
///   contains the reserved internal substitution prefix, which would collide with the
///   algorithm's temporary symbol names.
pub fn substitute_symbolic_matrix(
    matrix: SymbolicMatrix,
    replacements: &HashMap<String, Parameter>,
) -> Result<SymbolicMatrix, CircuitError> {
    if replacements.is_empty() {
        return Ok(matrix);
    }

    for value in matrix.iter() {
        for sym in value
            .re
            .get_symbols()
            .into_iter()
            .chain(value.im.get_symbols())
        {
            if sym.contains(INTERNAL_SUB_PREFIX) {
                return Err(CircuitError::InvalidOperation(format!(
                    "symbol name '{}' in input matrix collides with internal substitution prefix '{}'",
                    sym, INTERNAL_SUB_PREFIX
                )));
            }
        }
    }

    // Guard against symbol names that collide with our internal temp prefix.
    for key in replacements.keys() {
        if key.contains(INTERNAL_SUB_PREFIX) {
            return Err(CircuitError::InvalidOperation(format!(
                "symbol name '{}' collides with internal substitution prefix '{}'",
                key, INTERNAL_SUB_PREFIX
            )));
        }
    }
    for param in replacements.values() {
        for sym in param.get_symbols() {
            if sym.contains(INTERNAL_SUB_PREFIX) {
                return Err(CircuitError::InvalidOperation(format!(
                    "symbol name '{}' in replacement value collides with internal substitution prefix '{}'",
                    sym, INTERNAL_SUB_PREFIX
                )));
            }
        }
    }

    let dim = matrix.raw_dim();
    let (mut raw, _) = matrix.into_raw_vec_and_offset();

    // Fast path: single replacement never needs temp symbols.
    if replacements.len() == 1 {
        let (symbol, replacement) = replacements.iter().next().unwrap();
        raw.par_iter_mut().for_each(|v| {
            *v = v.replace(symbol, replacement.clone());
        });
        return Ok(Array2::from_shape_vec(dim, raw).expect("valid shape for owned matrix"));
    }

    // Check whether any replacement value contains a symbol that is also a key.
    // If there is no overlap, we can skip the expensive two-phase temp-symbol dance.
    let keys: HashSet<&str> = replacements.keys().map(|s| s.as_str()).collect();
    let has_overlap = replacements.values().any(|param| {
        param
            .get_symbols()
            .iter()
            .any(|sym| keys.contains(sym.as_str()))
    });

    if has_overlap {
        let temp_replacements: Vec<(String, String, Parameter)> = replacements
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    format!("{}{}", INTERNAL_SUB_PREFIX, k),
                    v.clone(),
                )
            })
            .collect();

        raw.par_iter_mut().for_each(|v| {
            for (symbol, temp_key, _) in &temp_replacements {
                *v = v.replace(symbol, Parameter::symbol(temp_key));
            }
            for (_, temp_key, replacement) in &temp_replacements {
                *v = v.replace(temp_key, replacement.clone());
            }
        });
    } else {
        let reps: Vec<(String, Parameter)> = replacements
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        raw.par_iter_mut().for_each(|v| {
            for (symbol, replacement) in &reps {
                *v = v.replace(symbol, replacement.clone());
            }
        });
    }

    Ok(Array2::from_shape_vec(dim, raw).expect("valid shape for owned matrix"))
}

/// Simplify all elements of a symbolic matrix.
pub fn simplify_matrix(m: &SymbolicMatrix) -> Result<SymbolicMatrix, ParameterError> {
    let mut out = m.clone();
    out.as_slice_mut()
        .expect("symbolic matrix must be contiguous")
        .par_iter_mut()
        .try_for_each(|elem| -> Result<(), ParameterError> {
            *elem = elem.simplify()?;
            Ok(())
        })?;
    Ok(out)
}

/// Returns a `dim × dim` symbolic identity matrix.
pub fn symbolic_eye(dim: usize) -> SymbolicMatrix {
    let mut matrix = Array2::from_elem((dim, dim), SymbolicComplex::zero());
    for i in 0..dim {
        matrix[[i, i]] = SymbolicComplex::one();
    }
    matrix
}

/// Applies a symbolic permutation gate to the target qubit positions of
/// `matrix`.
///
/// Each output row is constructed by scaling exactly one input row by its
/// corresponding permutation factor. Uses rayon when the matrix element
/// count exceeds the configured parallel threshold.
pub fn apply_symbolic_permutation_gate(
    matrix: &mut SymbolicMatrix,
    permutation: &[(usize, SymbolicComplex)],
    bits: &[usize],
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let gate_dim = 1usize << bits.len();
    let sorted_bits = sorted_bits(bits);
    let offsets = gate_offsets(bits);
    let loop_limit = dim >> bits.len();
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize, input: &mut Vec<SymbolicComplex>| {
        let base = expand_base_index(i, &sorted_bits);
        unsafe {
            input.resize(gate_dim * cols, SymbolicComplex::zero());
            for local_row in 0..gate_dim {
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                for col in 0..cols {
                    input[local_row * cols + col] = (*row_ptr.add(col)).clone();
                }
            }

            for (local_row, (source_row, factor)) in permutation.iter().enumerate() {
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                let source_start = source_row * cols;
                for col in 0..cols {
                    *row_ptr.add(col) = factor * &input[source_start + col];
                }
            }
        }
    };

    if parallel {
        (0..loop_limit)
            .into_par_iter()
            .for_each_init(Vec::new, |input, i| process_idx(i, input));
    } else {
        let mut input = Vec::new();
        for i in 0..loop_limit {
            process_idx(i, &mut input);
        }
    }
}

/// Applies a symbolic diagonal gate to the target qubit positions of `matrix`.
///
/// Only the rows whose diagonal entry is not exactly one are scaled, skipping
/// identity-scale rows. Uses rayon when the matrix element count exceeds
/// the configured parallel threshold.
pub fn apply_symbolic_diagonal_gate(
    matrix: &mut SymbolicMatrix,
    diagonal: &[SymbolicComplex],
    bits: &[usize],
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let sorted_bits = sorted_bits(bits);
    let offsets = gate_offsets(bits);
    let loop_limit = dim >> bits.len();
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize| {
        let base = expand_base_index(i, &sorted_bits);
        unsafe {
            for (local_row, scale) in diagonal.iter().enumerate() {
                if scale.is_one_exact() {
                    continue;
                }
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                for col in 0..cols {
                    let old = std::mem::take(&mut *row_ptr.add(col));
                    *row_ptr.add(col) = scale * old;
                }
            }
        }
    };

    if parallel {
        (0..loop_limit).into_par_iter().for_each(process_idx);
    } else {
        (0..loop_limit).for_each(process_idx);
    }
}

/// Applies a numeric permutation gate to the target qubit positions of
/// `matrix`.
///
/// Numeric variant of [`apply_symbolic_permutation_gate`] where the
/// permutation factors are concrete [`Complex64`] values.
pub fn apply_numeric_permutation_gate(
    matrix: &mut SymbolicMatrix,
    permutation: &[(usize, Complex64)],
    bits: &[usize],
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let gate_dim = 1usize << bits.len();
    let sorted_bits = sorted_bits(bits);
    let offsets = gate_offsets(bits);
    let loop_limit = dim >> bits.len();
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize, input: &mut Vec<SymbolicComplex>| {
        let base = expand_base_index(i, &sorted_bits);
        unsafe {
            input.resize(gate_dim * cols, SymbolicComplex::zero());
            for local_row in 0..gate_dim {
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                for col in 0..cols {
                    input[local_row * cols + col] = (*row_ptr.add(col)).clone();
                }
            }

            for (local_row, (source_row, factor)) in permutation.iter().copied().enumerate() {
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                let source_start = source_row * cols;
                for col in 0..cols {
                    *row_ptr.add(col) = factor * &input[source_start + col];
                }
            }
        }
    };

    if parallel {
        (0..loop_limit)
            .into_par_iter()
            .for_each_init(Vec::new, |input, i| process_idx(i, input));
    } else {
        let mut input = Vec::new();
        for i in 0..loop_limit {
            process_idx(i, &mut input);
        }
    }
}

/// Applies a numeric diagonal gate to the target qubit positions of `matrix`.
///
/// Numeric variant of [`apply_symbolic_diagonal_gate`] where the diagonal
/// entries are concrete [`Complex64`] values.
pub fn apply_numeric_diagonal_gate(
    matrix: &mut SymbolicMatrix,
    diagonal: &[Complex64],
    bits: &[usize],
) {
    let dim = matrix.nrows();
    let cols = matrix.ncols();
    let sorted_bits = sorted_bits(bits);
    let offsets = gate_offsets(bits);
    let loop_limit = dim >> bits.len();
    let total_ops = dim.saturating_mul(cols);
    let parallel = total_ops >= PARALLEL_THRESHOLD_OPS;

    let slice = matrix
        .as_slice_mut()
        .expect("Symbolic matrix must be contiguous");
    let unsafe_slice = UnsafeSymbolicSlice::new(slice);

    let process_idx = |i: usize| {
        let base = expand_base_index(i, &sorted_bits);
        unsafe {
            for (local_row, scale) in diagonal.iter().copied().enumerate() {
                if numeric_is_one(scale) {
                    continue;
                }
                let row_ptr = unsafe_slice.row_ptr(base | offsets[local_row], cols);
                for col in 0..cols {
                    let old = std::mem::take(&mut *row_ptr.add(col));
                    *row_ptr.add(col) = scale * old;
                }
            }
        }
    };

    if parallel {
        (0..loop_limit).into_par_iter().for_each(process_idx);
    } else {
        (0..loop_limit).for_each(process_idx);
    }
}

/// Evaluates a [`SymbolicMatrix`] to a numerical `Array2<Complex64>` by
/// binding concrete values to every free symbol.
///
/// # Parallelism
///
/// When the matrix storage is contiguous (the common case), evaluation is
/// parallelised with rayon. Otherwise it falls back to a sequential scan.
///
/// # Errors
///
/// Returns [`ParameterError`] if any symbol required by the matrix elements
/// is missing from `bindings`.
pub fn evaluate_symbolic_matrix(
    matrix: &SymbolicMatrix,
    bindings: &Option<HashMap<&str, f64>>,
) -> Result<Array2<Complex64>, ParameterError> {
    let mut out = Array2::from_elem(matrix.raw_dim(), Complex64::new(0.0, 0.0));

    if let (Some(m), Some(o)) = (matrix.as_slice(), out.as_slice_mut()) {
        m.par_iter().zip(o.par_iter_mut()).try_for_each(
            |(v, o)| -> Result<(), ParameterError> {
                *o = v.evaluate(bindings)?;
                Ok(())
            },
        )?;
    } else {
        for ((row, col), value) in matrix.indexed_iter() {
            out[[row, col]] = value.evaluate(bindings)?;
        }
    }

    Ok(out)
}

/// Helper struct for raw pointer access to split mutable borrow across threads.
///
/// # Safety
/// This struct is `Send` + `Sync` only because the caller guarantees that
/// concurrent accesses target disjoint row indices. Each worker thread must
/// process a distinct subset of row indices so that no two threads ever write
/// to the same memory location.
pub(crate) struct UnsafeSymbolicSlice<'a> {
    ptr: *mut SymbolicComplex,
    _marker: PhantomData<&'a mut [SymbolicComplex]>,
}

unsafe impl<'a> Sync for UnsafeSymbolicSlice<'a> {}
unsafe impl<'a> Send for UnsafeSymbolicSlice<'a> {}

impl<'a> UnsafeSymbolicSlice<'a> {
    pub fn new(slice: &'a mut [SymbolicComplex]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// Caller must ensure that the returned pointer is only used while the
    /// underlying slice is alive, and that concurrent accesses to the same
    /// row index do not occur.
    pub unsafe fn row_ptr(&self, row_idx: usize, cols: usize) -> *mut SymbolicComplex {
        unsafe { self.ptr.add(row_idx * cols) }
    }
}

fn expand_base_index(mut compact: usize, sorted_bits: &[usize]) -> usize {
    for &q in sorted_bits {
        let mask = (1usize << q) - 1;
        let left = (compact & !mask) << 1;
        let right = compact & mask;
        compact = left | right;
    }
    compact
}

fn sorted_bits(bits: &[usize]) -> SmallVec<[usize; 8]> {
    let mut sorted: SmallVec<[usize; 8]> = bits.iter().copied().collect();
    sorted.sort();
    sorted
}

fn gate_offsets(bits: &[usize]) -> Vec<usize> {
    let gate_dim = 1usize << bits.len();
    let mut offsets = vec![0usize; gate_dim];
    for (k, offset_ref) in offsets.iter_mut().enumerate() {
        let mut offset = 0usize;
        for (j, &physical_bit) in bits.iter().enumerate() {
            if (k >> j) & 1 == 1 {
                offset |= 1usize << physical_bit;
            }
        }
        *offset_ref = offset;
    }
    offsets
}

pub(crate) fn numeric_is_zero(value: Complex64) -> bool {
    value.re == 0.0 && value.im == 0.0
}

pub(crate) fn numeric_is_one(value: Complex64) -> bool {
    value.re == 1.0 && value.im == 0.0
}

#[cfg(test)]
#[path = "./matrix_test.rs"]
mod matrix_test;
