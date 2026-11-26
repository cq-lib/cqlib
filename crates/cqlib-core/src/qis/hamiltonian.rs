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

//! Hamiltonian representation and operations.
//!
//! This module provides the [`Hamiltonian`] struct, which represents a quantum
//! operator as a linear combination of Pauli strings (tensor products of Pauli operators)
//! with complex coefficients. It is primarily used for defining system energies,
//! observables for expectation value calculations, and operators for time evolution.
//!
//! General algebra permits non-Hermitian operators. Real observables and unitary
//! evolution validate Hermiticity at their consumption boundaries.

use crate::qis::error::validate_coefficient;
use crate::qis::pauli::PauliString;
use crate::qis::{Phase, QisError};
use ndarray::Array2;
use num_complex::Complex64;
use std::fmt;
use std::ops::Add;

/// A quantum Hamiltonian represented as a sum of Pauli strings.
///
/// A `Hamiltonian` is essentially a sparse representation of a $2^N \times 2^N$
/// matrix, expressed as $H = \sum_k c_k P_k$, where $c_k$ is a complex coefficient
/// and $P_k$ is an $N$-qubit Pauli string.
///
/// Despite its name, this type also represents non-Hermitian operators. Checked
/// algebra returns phase-normalized, merged terms without approximate pruning.
/// [`PartialEq`] compares stored structure; use [`Self::equiv`] for algebraic comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Hamiltonian {
    /// The number of qubits this operator acts on.
    pub num_qubits: usize,
    /// The list of terms: (PauliString, Coefficient).
    pub terms: Vec<(PauliString, Complex64)>,
}

impl fmt::Display for Hamiltonian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (op, coeff)) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, " + ")?;
            }
            // Format output as: (1.0+0.0j) * XZ
            write!(f, "({}) * {}", coeff, op)?;
        }
        Ok(())
    }
}

impl Hamiltonian {
    /// Returns a validated Pauli sum with phases absorbed and exact zeros removed.
    /// Retains the declared qubit count, including for zero and scalar operators.
    pub fn normalized(&self) -> Result<Self, QisError> {
        Self::from_terms(self.num_qubits, self.terms.iter().cloned())
    }

    fn validate(&self) -> Result<(), QisError> {
        for (pauli, coefficient) in &self.terms {
            if pauli.num_qubits != self.num_qubits {
                return Err(QisError::QubitMismatch {
                    expected: self.num_qubits,
                    actual: pauli.num_qubits,
                });
            }
            for actual in [pauli.x.len(), pauli.z.len()] {
                if actual != self.num_qubits {
                    return Err(QisError::DimensionMismatch {
                        expected: self.num_qubits,
                        actual,
                    });
                }
            }
            validate_coefficient(*coefficient)?;
        }
        Ok(())
    }

    /// Shared contract for real-valued observables and Hamiltonian evolution.
    /// After merging and absorbing Pauli phases, imaginary parts with magnitude
    /// greater than `1e-10` are rejected. Accepted imaginary roundoff residuals
    /// are projected to zero, and terms with exactly zero real part are removed.
    /// Every nonzero real coefficient is retained, however small.
    pub fn normalized_hermitian(&self) -> Result<Self, QisError> {
        let mut normalized = self.normalized()?;
        if normalized
            .terms
            .iter()
            .any(|(_, coefficient)| coefficient.im.abs() > 1e-10)
        {
            return Err(QisError::NotHermitian);
        }
        for (_, coefficient) in &mut normalized.terms {
            coefficient.im = 0.0;
        }
        normalized
            .terms
            .retain(|(_, coefficient)| coefficient.re != 0.0);
        Ok(normalized)
    }

    /// Creates a new empty Hamiltonian.
    ///
    /// The resulting Hamiltonian represents the zero operator for the given
    /// number of qubits.
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            terms: Vec::new(),
        }
    }

    /// Creates a Hamiltonian from a single Pauli string with a coefficient of 1.0.
    ///
    /// # Arguments
    /// * `pauli` - The Pauli string to wrap into a Hamiltonian.
    pub fn from_pauli(pauli: PauliString) -> Self {
        pauli.into()
    }

    /// Returns the dense complex matrix represented by this Hamiltonian.
    ///
    /// Every term must act on the same number of qubits as the Hamiltonian.
    /// An empty Hamiltonian produces the zero matrix for its declared qubit
    /// count.
    ///
    /// This method is intended for small-system analysis and verification.
    /// The returned matrix has dimensions $2^N \times 2^N$ and requires
    /// $O(4^N)$ memory.
    ///
    /// # Errors
    ///
    /// Validates term widths, bit-vector lengths and finite coefficients. Returns
    /// [`QisError::UnsupportedOperation`] if the dense allocation size overflows
    /// or its reservation fails. Matrix-entry overflow is also reported.
    pub fn to_matrix(&self) -> Result<Array2<Complex64>, QisError> {
        let normalized = self.normalized()?;
        let size_error = || {
            QisError::UnsupportedOperation(
                "dense Pauli matrix exceeds addressable allocation size".into(),
            )
        };
        let shift = u32::try_from(self.num_qubits).map_err(|_| size_error())?;
        let dim = 1usize.checked_shl(shift).ok_or_else(size_error)?;
        let len = dim.checked_mul(dim).ok_or_else(size_error)?;
        let bytes = len
            .checked_mul(std::mem::size_of::<Complex64>())
            .ok_or_else(size_error)?;
        if bytes > isize::MAX as usize {
            return Err(size_error());
        }
        let mut data = Vec::new();
        data.try_reserve_exact(len).map_err(|error| {
            QisError::UnsupportedOperation(format!("cannot allocate dense Pauli matrix: {error}"))
        })?;
        data.resize(len, Complex64::new(0.0, 0.0));
        // P|column> flips X bits; Z parity is evaluated before those flips.
        for (pauli, coefficient) in &normalized.terms {
            let x = pauli.x_mask();
            let z = pauli.z_mask();
            let factor = *coefficient * pauli.y_phase();
            validate_coefficient(factor)?;
            for column in 0..dim {
                let sign = if (column & z).count_ones() & 1 == 0 {
                    1.0
                } else {
                    -1.0
                };
                let entry = &mut data[(column ^ x) * dim + column];
                *entry += factor * sign;
                validate_coefficient(*entry)?;
            }
        }
        Array2::from_shape_vec((dim, dim), data)
            .map_err(|error| QisError::UnsupportedOperation(error.to_string()))
    }

    /// Creates a Hamiltonian from a list of Pauli strings and their corresponding coefficients.
    ///
    /// # Arguments
    /// * `ops` - A vector of tuples, each containing a [`PauliString`] and a [`Complex64`] coefficient.
    ///
    /// # Errors
    /// Returns `QisError::QubitMismatch` if not all Pauli strings in the list act on the same number of qubits.
    pub fn from_list(
        ops: Vec<(PauliString, Complex64)>,
    ) -> Result<Self, crate::qis::error::QisError> {
        Self::try_from(ops)
    }
}

impl From<PauliString> for Hamiltonian {
    fn from(pauli: PauliString) -> Self {
        let num_qubits = pauli.num_qubits;
        Self {
            num_qubits,
            terms: vec![(pauli, Complex64::new(1.0, 0.0))],
        }
    }
}

impl Hamiltonian {
    /// The identity on the declared register; at zero qubits this is scalar one.
    pub fn identity(num_qubits: usize) -> Self {
        Self::from_pauli(PauliString::new(num_qubits))
    }

    /// Constructs a validated, phase-normalized sum, merging duplicate terms.
    /// Only exact zeros are removed, and empty input retains `num_qubits`.
    ///
    /// Invalid widths, bit-vector lengths, nonfinite coefficients and intermediate
    /// coefficient overflow return an error. No approximate truncation occurs.
    pub fn from_terms(
        num_qubits: usize,
        terms: impl IntoIterator<Item = (PauliString, Complex64)>,
    ) -> Result<Self, QisError> {
        let mut result = Self {
            num_qubits,
            terms: terms.into_iter().collect(),
        };
        result.validate()?;
        result.merge_terms(0.0, true)?;
        Ok(result)
    }

    fn check_width(&self, other: &Self) -> Result<(), QisError> {
        if self.num_qubits != other.num_qubits {
            return Err(QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: other.num_qubits,
            });
        }
        Ok(())
    }

    fn check_materialized_width(width: usize) -> Result<(), QisError> {
        // BitSlice reserves metadata bits, so its length bound is below usize::MAX.
        // Zero sums need no bit vectors and may retain larger declared widths.
        if width > bitvec::slice::BitSlice::<usize, bitvec::order::Lsb0>::MAX_BITS {
            return Err(QisError::UnsupportedOperation(
                "Pauli register width exceeds the BitVec representation limit".into(),
            ));
        }
        Ok(())
    }

    /// Adds two operators on the same register, without modifying either input.
    /// Unlike legacy `Add`, this validates inputs and merges the resulting terms.
    pub fn try_add(&self, other: &Self) -> Result<Self, QisError> {
        self.check_width(other)?;
        let left = self.normalized()?;
        let right = other.normalized()?;
        Self::from_terms(self.num_qubits, left.terms.into_iter().chain(right.terms))
    }

    /// Subtracts two operators on the same register, retaining every nonzero term.
    pub fn try_sub(&self, other: &Self) -> Result<Self, QisError> {
        self.check_width(other)?;
        let left = self.normalized()?;
        let right = other.normalized()?;
        Self::from_terms(
            self.num_qubits,
            left.terms
                .into_iter()
                .chain(right.terms.into_iter().map(|(p, c)| (p, -c))),
        )
    }

    /// Forms the matrix product `self * other`; the right operand acts first.
    /// Both inputs are validated even when either represents zero. Products may
    /// be non-Hermitian. Expansion takes O(m k n) work before sorting/merging
    /// and may generate m k terms, for term counts m, k and register width n.
    ///
    /// ```
    /// use cqlib_core::qis::{Hamiltonian, QisError};
    /// use num_complex::Complex64;
    /// let x = Hamiltonian::from_pauli("X".parse()?);
    /// let z = Hamiltonian::from_pauli("Z".parse()?);
    /// let minus_i_y = Hamiltonian::from_terms(1, [
    ///     ("Y".parse()?, Complex64::new(0.0, -1.0)),
    /// ])?;
    /// assert!(x.try_mul(&z)?.equiv(&minus_i_y, 0.0)?);
    /// # Ok::<(), QisError>(())
    /// ```
    pub fn try_mul(&self, other: &Self) -> Result<Self, QisError> {
        self.check_width(other)?;
        let left = self.normalized()?;
        let right = other.normalized()?;
        Self::from_terms(
            self.num_qubits,
            left.terms
                .iter()
                .flat_map(|(p, a)| right.terms.iter().map(move |(q, b)| (p * q, *a * *b))),
        )
    }

    /// Returns a scaled sum, checking the factor even for the zero operator.
    pub fn scaled(&self, factor: Complex64) -> Result<Self, QisError> {
        validate_coefficient(factor)?;
        let normalized = self.normalized()?;
        Self::from_terms(
            self.num_qubits,
            normalized.terms.into_iter().map(|(p, c)| (p, c * factor)),
        )
    }

    /// Returns the conjugate transpose after absorbing Pauli phases.
    /// Phase-free Pauli strings are Hermitian, so their coefficients are conjugated.
    pub fn adjoint(&self) -> Result<Self, QisError> {
        let mut result = self.normalized()?;
        for (_, coefficient) in &mut result.terms {
            *coefficient = coefficient.conj();
        }
        Ok(result)
    }

    fn validate_tolerance(atol: f64) -> Result<(), QisError> {
        if !atol.is_finite() || atol < 0.0 {
            return Err(QisError::InvalidParameterValue(
                "Pauli coefficient tolerance must be finite and nonnegative".into(),
            ));
        }
        Ok(())
    }

    /// Normalizes, then removes coefficients with complex magnitude at most `atol`.
    /// On any validation or merging error, the stored operator is unchanged.
    pub fn chop(&mut self, atol: f64) -> Result<(), QisError> {
        Self::validate_tolerance(atol)?;
        let mut result = self.normalized()?;
        result.terms.retain(|(_, c)| c.norm() > atol);
        *self = result;
        Ok(())
    }

    /// Compares normalized coefficients of `self - other` against `atol`.
    /// Width mismatch is an error. This coefficient tolerance is not a matrix
    /// operator-norm bound, and differs from structural `PartialEq`.
    pub fn equiv(&self, other: &Self, atol: f64) -> Result<bool, QisError> {
        Self::validate_tolerance(atol)?;
        Ok(self
            .try_sub(other)?
            .terms
            .iter()
            .all(|(_, c)| c.norm() <= atol))
    }

    /// Checks `self.equiv(self.adjoint(), atol)` as for `FermionicOp`.
    /// On a normalized sum this bounds twice the imaginary coefficient magnitude.
    /// It does not project coefficients to real values; `normalized_hermitian`
    /// retains its separate consumer contract (imaginary tolerance `1e-10`).
    pub fn is_hermitian(&self, atol: f64) -> Result<bool, QisError> {
        Self::validate_tolerance(atol)?;
        self.equiv(&self.adjoint()?, atol)
    }

    /// Returns `self ⊗ other`: `other` occupies low qubits and `self` high qubits.
    /// Register widths are added even for zero operators. No dense matrix is built.
    /// A nonzero result with a width beyond BitVec's length limit returns an error.
    ///
    /// ```
    /// use cqlib_core::qis::{Hamiltonian, QisError};
    /// let x = Hamiltonian::from_pauli("X".parse()?);
    /// let z = Hamiltonian::from_pauli("Z".parse()?);
    /// assert!(x.tensor(&z)?.equiv(&Hamiltonian::from_pauli("XZ".parse()?), 0.0)?);
    /// # Ok::<(), QisError>(())
    /// ```
    pub fn tensor(&self, other: &Self) -> Result<Self, QisError> {
        let width = self
            .num_qubits
            .checked_add(other.num_qubits)
            .ok_or_else(|| {
                QisError::InvalidParameterValue("tensor register width overflow".into())
            })?;
        let left = self.normalized()?;
        let right = other.normalized()?;
        if !left.terms.is_empty() && !right.terms.is_empty() {
            Self::check_materialized_width(width)?;
        }
        Self::from_terms(
            width,
            left.terms.iter().flat_map(|(p, a)| {
                right.terms.iter().map(move |(q, b)| {
                    let mut product = q.clone();
                    product.x.extend_from_bitslice(&p.x);
                    product.z.extend_from_bitslice(&p.z);
                    product.num_qubits = width;
                    (product, *a * *b)
                })
            }),
        )
    }

    /// Relabels qubits using `mapping[source] = destination`, inserting identities
    /// on unused target qubits. Requires exactly one distinct destination per
    /// source qubit, each less than `target_width`, even for zero/identity input.
    /// This is a qubit embedding, not a fermionic mode permutation or a trace.
    /// If terms must be constructed, unrepresentable target widths return
    /// [`QisError::UnsupportedOperation`] before BitVec allocation.
    pub fn remap_qubits(&self, mapping: &[usize], target_width: usize) -> Result<Self, QisError> {
        if mapping.len() != self.num_qubits {
            return Err(QisError::InvalidParameterValue(
                "qubit mapping length mismatch".into(),
            ));
        }
        let mut destinations = std::collections::HashSet::with_capacity(mapping.len());
        for &destination in mapping {
            if destination >= target_width || !destinations.insert(destination) {
                return Err(QisError::InvalidParameterValue(
                    "qubit mapping must contain distinct, in-range destinations".into(),
                ));
            }
        }
        let normalized = self.normalized()?;
        if !normalized.terms.is_empty() {
            Self::check_materialized_width(target_width)?;
        }
        Self::from_terms(
            target_width,
            normalized.terms.into_iter().map(|(p, c)| {
                let mut mapped = PauliString::new(target_width);
                for (source, &destination) in mapping.iter().enumerate() {
                    mapped.x.set(destination, p.x[source]);
                    mapped.z.set(destination, p.z[source]);
                }
                (mapped, c)
            }),
        )
    }
}

impl TryFrom<Vec<(PauliString, Complex64)>> for Hamiltonian {
    type Error = QisError;

    fn try_from(ops: Vec<(PauliString, Complex64)>) -> Result<Self, Self::Error> {
        if ops.is_empty() {
            // Handle the empty case by returning a 0-qubit empty Hamiltonian.
            // Depending on design, this could also panic, but allowing an empty
            // list representing the zero operator is a valid fallback.
            return Ok(Self::new(0));
        }
        let n = ops[0].0.num_qubits;
        // Verify that all operators have the same dimension
        for (op, _) in &ops {
            if op.num_qubits != n {
                return Err(crate::qis::error::QisError::QubitMismatch {
                    expected: n,
                    actual: op.num_qubits,
                });
            }
        }
        Ok(Self {
            num_qubits: n,
            terms: ops,
        })
    }
}

impl Hamiltonian {
    /// Adds a new Pauli string term with a given coefficient to the Hamiltonian.
    ///
    /// # Arguments
    /// * `op` - The Pauli string operator to add.
    /// * `coeff` - The complex coefficient for this term.
    ///
    /// # Errors
    /// Returns `QisError::QubitMismatch` if the number of qubits in the `op` does not match the Hamiltonian's `num_qubits`.
    pub fn add_term(
        &mut self,
        op: PauliString,
        coeff: Complex64,
    ) -> Result<(), crate::qis::error::QisError> {
        if op.num_qubits != self.num_qubits {
            return Err(crate::qis::error::QisError::QubitMismatch {
                expected: self.num_qubits,
                actual: op.num_qubits,
            });
        }
        self.terms.push((op, coeff));
        Ok(())
    }

    /// Simplifies the Hamiltonian by combining terms with the same Pauli string.
    ///
    /// This method performs two primary optimizations:
    /// 1. **Phase Normalization**: It absorbs any internal phases (e.g., $+i, -1$)
    ///    from the `PauliString` into the complex coefficient, normalizing all Pauli
    ///    strings to the $+1$ phase (`Phase::Plus`).
    /// 2. **Term Aggregation**: It groups terms with identical underlying bit vectors
    ///    (X and Z masks) and sums their coefficients (e.g., $0.5 X + 0.3 X \to 0.8 X$).
    ///    Terms with coefficients whose absolute value is near zero ($< 10^{-10}$) are removed.
    ///
    /// This method is crucial for optimizing performance before executing quantum simulations.
    pub fn simplify(&mut self) {
        self.simplify_with_tolerance(1e-10);
    }

    // Evolution must merge terms without pruning small, nonzero coefficients.
    pub(crate) fn simplify_with_tolerance(&mut self, tolerance: f64) {
        // Legacy simplification retains its infallible API and numerical behavior.
        self.merge_terms(tolerance, false)
            .expect("unchecked merging cannot fail");
    }

    fn merge_terms(&mut self, tolerance: f64, checked: bool) -> Result<(), QisError> {
        if self.terms.is_empty() {
            return Ok(());
        }

        // 1. Normalize the Phase of all PauliStrings to Plus (+1), and multiply the phase into the coefficient.
        //    Example: (0.5) * (-iY)  ->  (0.5 * -i) * Y
        let mut simplified_terms: Vec<(PauliString, Complex64)> =
            Vec::with_capacity(self.terms.len());

        for (op, coeff) in self.terms.drain(..) {
            let mut pure_op = op.clone();
            let phase_val = pure_op.phase.to_complex(); // Get 1, i, -1, -i

            // Normalize the operator's phase
            pure_op.phase = Phase::Plus;

            // Multiply the phase into the coefficient
            let new_coeff = coeff * phase_val;
            if checked {
                validate_coefficient(new_coeff)?;
            }

            simplified_terms.push((pure_op, new_coeff));
        }

        // 2. Sort and merge
        // Since PauliStrings (with Phase::Plus) are hashable/sortable, we group them by their x and z vectors.
        // We use a simple custom sort to ensure deterministic merging.

        // Sort primarily by the z vector, then by the x vector
        simplified_terms.sort_by(|a, b| {
            // Compare the underlying bitvecs
            a.0.z.cmp(&b.0.z).then_with(|| a.0.x.cmp(&b.0.x))
        });

        // 3. Merge adjacent identical terms
        let mut merged: Vec<(PauliString, Complex64)> = Vec::new();
        if let Some(first) = simplified_terms.first() {
            let mut current_op = first.0.clone();
            let mut current_coeff = first.1;

            for (op, coeff) in simplified_terms.into_iter().skip(1) {
                if op.x == current_op.x && op.z == current_op.z {
                    // Identical operator found, accumulate the coefficient
                    current_coeff += coeff;
                    if checked {
                        validate_coefficient(current_coeff)?;
                    }
                } else {
                    // New operator found, save the previous one if its coefficient is significant
                    // Filter out terms with a coefficient near zero
                    if (tolerance == 0.0 && current_coeff != Complex64::new(0.0, 0.0))
                        || current_coeff.norm() > tolerance
                    {
                        merged.push((current_op, current_coeff));
                    }
                    current_op = op;
                    current_coeff = coeff;
                }
            }
            // Push the last accumulated term
            if (tolerance == 0.0 && current_coeff != Complex64::new(0.0, 0.0))
                || current_coeff.norm() > tolerance
            {
                merged.push((current_op, current_coeff));
            }
        }

        self.terms = merged;
        Ok(())
    }

    /// Scales all terms in the Hamiltonian by a complex factor.
    ///
    /// # Arguments
    /// * `factor` - The complex scalar to multiply each coefficient by.
    pub fn scale(&mut self, factor: Complex64) {
        for (_, coeff) in &mut self.terms {
            *coeff *= factor;
        }
    }

    /// Returns `true` if all pairs of Pauli terms in this Hamiltonian mutually commute.
    ///
    /// When `true`, the time evolution $e^{-iHt}$ is **exact** via term-by-term
    /// decomposition: $e^{-iHt} = \prod_k e^{-i c_k t P_k}$, with no Trotter
    /// approximation error regardless of term ordering.
    ///
    /// When `false`, at least one pair of terms does not commute, meaning that
    /// product-formula decompositions (Trotter, Suzuki) are only approximations.
    ///
    /// # Complexity
    ///
    /// $O(m^2 \cdot n / 64)$ where $m$ is the number of terms and $n$ is the number
    /// of qubits. For typical sparse Hamiltonians ($m \ll 2^n$) this is efficient.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::hamiltonian::Hamiltonian;
    ///
    /// // ZZ and IZ commute
    /// let mut h = Hamiltonian::new(2);
    /// h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
    /// h.add_term("IZ".parse().unwrap(), 0.5.into()).unwrap();
    /// assert!(h.all_terms_commute());
    ///
    /// // X and Z do not commute (single qubit)
    /// let mut h2 = Hamiltonian::new(1);
    /// h2.add_term("X".parse().unwrap(), 1.0.into()).unwrap();
    /// h2.add_term("Z".parse().unwrap(), 1.0.into()).unwrap();
    /// assert!(!h2.all_terms_commute());
    /// ```
    pub fn all_terms_commute(&self) -> bool {
        for i in 0..self.terms.len() {
            for j in (i + 1)..self.terms.len() {
                if !self.terms[i].0.commutes_with(&self.terms[j].0) {
                    return false;
                }
            }
        }
        true
    }
}

impl Add for Hamiltonian {
    type Output = Self;

    /// Adds two Hamiltonians together.
    ///
    /// Note: This performs a simple lazy concatenation of the term lists.
    /// It does not automatically merge identical terms. To optimize the resulting
    /// Hamiltonian, call [`simplify`](Hamiltonian::simplify) after addition.
    ///
    /// # Panics
    /// Panics if the number of qubits of the two Hamiltonians do not match.
    fn add(mut self, rhs: Self) -> Self {
        assert_eq!(self.num_qubits, rhs.num_qubits);
        // Directly extend the list of terms without immediate merging (lazy evaluation)
        self.terms.extend(rhs.terms);
        self
    }
}

#[cfg(test)]
#[path = "hamiltonian_test.rs"]
mod tests;
