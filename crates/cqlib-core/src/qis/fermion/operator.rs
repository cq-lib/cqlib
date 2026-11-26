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

//! Structured fermionic operators and canonical anticommutation algebra.

use crate::qis::QisError;
use crate::qis::error::validate_coefficient;
use num_complex::Complex64;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Add, Mul, Neg, Sub};

/// A creation or annihilation operator on a zero-based fermionic mode.
///
/// A sequence `[A, B]` denotes the mathematical product `A B`: `B` acts first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FermionicLadder {
    /// Creates a particle in the given mode.
    Create(usize),
    /// Removes a particle from the given mode.
    Annihilate(usize),
}

impl FermionicLadder {
    /// Returns the mode on which this operator acts.
    pub fn mode(self) -> usize {
        match self {
            Self::Create(mode) | Self::Annihilate(mode) => mode,
        }
    }

    /// Exchanges creation and annihilation.
    pub fn adjoint(self) -> Self {
        match self {
            Self::Create(mode) => Self::Annihilate(mode),
            Self::Annihilate(mode) => Self::Create(mode),
        }
    }
}

/// A finite complex linear combination of ordered fermionic products.
///
/// The declared mode count is retained even for zero and identity operators.
/// Equal ordered products are merged automatically; only exact zero coefficients
/// are removed. Products are not automatically normal ordered. Use [`Self::chop`]
/// for explicit numerical truncation and [`Self::equiv`] for algebraic equivalence.
/// Structural equality also compares the declared mode count.
///
/// Fallible arithmetic operators return `Result`, for example `(&a + &b)?`.
#[derive(Debug, Clone, PartialEq)]
pub struct FermionicOp {
    num_modes: usize,
    terms: BTreeMap<Vec<FermionicLadder>, Complex64>,
}

pub(crate) fn validate_tolerance(atol: f64) -> Result<(), QisError> {
    if !atol.is_finite() || atol < 0.0 {
        return Err(QisError::InvalidParameterValue(
            "tolerance must be finite and nonnegative".into(),
        ));
    }
    Ok(())
}

impl FermionicOp {
    /// Creates the zero operator on the declared mode space, including zero modes.
    pub fn new(num_modes: usize) -> Self {
        Self {
            num_modes,
            terms: BTreeMap::new(),
        }
    }

    /// Creates the identity. An empty product is the identity, not the zero operator.
    pub fn identity(num_modes: usize) -> Self {
        let mut result = Self::new(num_modes);
        result.terms.insert(Vec::new(), Complex64::new(1.0, 0.0));
        result
    }

    /// Creates a weighted ordered product. Even zero-weight terms are validated.
    pub fn from_term(
        num_modes: usize,
        term: impl IntoIterator<Item = FermionicLadder>,
        coefficient: Complex64,
    ) -> Result<Self, QisError> {
        let mut result = Self::new(num_modes);
        result.add_term(term, coefficient)?;
        Ok(result)
    }

    /// Creates a sum of products, merging duplicates without numerical truncation.
    pub fn from_terms(
        num_modes: usize,
        terms: impl IntoIterator<Item = (Vec<FermionicLadder>, Complex64)>,
    ) -> Result<Self, QisError> {
        let mut result = Self::new(num_modes);
        for (term, coefficient) in terms {
            result.add_term(term, coefficient)?;
        }
        Ok(result)
    }

    /// Adds a weighted product atomically; an error leaves this operator unchanged.
    pub fn add_term(
        &mut self,
        term: impl IntoIterator<Item = FermionicLadder>,
        coefficient: Complex64,
    ) -> Result<(), QisError> {
        validate_coefficient(coefficient)?;
        let term: Vec<_> = term.into_iter().collect();
        for ladder in &term {
            let mode = ladder.mode();
            if mode >= self.num_modes {
                return Err(QisError::ModeOutOfBounds {
                    mode,
                    num_modes: self.num_modes,
                });
            }
        }
        let merged = self.terms.get(&term).copied().unwrap_or_default() + coefficient;
        validate_coefficient(merged)?;
        if merged == Complex64::new(0.0, 0.0) {
            self.terms.remove(&term);
        } else {
            self.terms.insert(term, merged);
        }
        Ok(())
    }

    /// Creates a single creation operator.
    pub fn creation(num_modes: usize, mode: usize) -> Result<Self, QisError> {
        Self::from_term(num_modes, [FermionicLadder::Create(mode)], 1.0.into())
    }

    /// Creates a single annihilation operator.
    pub fn annihilation(num_modes: usize, mode: usize) -> Result<Self, QisError> {
        Self::from_term(num_modes, [FermionicLadder::Annihilate(mode)], 1.0.into())
    }

    /// Creates the number operator `a†_p a_p`.
    pub fn number(num_modes: usize, mode: usize) -> Result<Self, QisError> {
        Self::from_term(
            num_modes,
            [
                FermionicLadder::Create(mode),
                FermionicLadder::Annihilate(mode),
            ],
            1.0.into(),
        )
    }

    /// Creates the total particle-number operator, or zero for zero modes.
    pub fn total_number(num_modes: usize) -> Self {
        let terms = (0..num_modes)
            .map(|mode| {
                (
                    vec![
                        FermionicLadder::Create(mode),
                        FermionicLadder::Annihilate(mode),
                    ],
                    1.0.into(),
                )
            })
            .collect();
        Self { num_modes, terms }
    }

    /// Creates the unit-weight Hermitian excitation generator `G = i(T - T†)`.
    ///
    /// For `sources = [i_0, ..., i_{r-1}]` and `targets = [a_0, ..., a_{r-1}]`,
    /// the ordered excitation is
    /// `T = a†_{a_0} ... a†_{a_{r-1}} a_{i_{r-1}} ... a_{i_0}`.
    /// The rightmost operator acts first. Thus `[i, j] -> [a, b]` means
    /// `T = a†_a a†_b a_j a_i`.
    ///
    /// Input order is preserved, without implicit sorting or normal ordering.
    /// An odd permutation of either list negates `G`; applying odd permutations
    /// to both lists preserves it. Explicit [`Self::normal_order`] preserves
    /// these fermionic signs. No factorial or other normalization is applied.
    ///
    /// Mapping `G` and evolving it with [`crate::circuit::ansatz::PauliEvolutionAnsatz`]
    /// implements `exp(-i θ G) = exp(θ (T - T†))`. The evolution parameter belongs
    /// to the ansatz, not this operator. The generator conserves total particle
    /// number. Reference occupations and spin interpretations are caller choices.
    ///
    /// # Errors
    ///
    /// Returns [`QisError::InvalidParameterValue`] if the lists are empty, have
    /// unequal lengths, contain repeated modes, or overlap. Returns
    /// [`QisError::ModeOutOfBounds`] if an index is outside `0..num_modes`.
    ///
    /// # Example
    /// ```
    /// use cqlib_core::qis::FermionicOp;
    /// let single = FermionicOp::excitation_generator(4, &[0], &[2])?;
    /// let double = FermionicOp::excitation_generator(4, &[0, 1], &[2, 3])?;
    /// assert!(single.is_hermitian(0.0)?);
    /// assert!(double.is_hermitian(0.0)?);
    /// # Ok::<(), cqlib_core::qis::QisError>(())
    /// ```
    pub fn excitation_generator(
        num_modes: usize,
        sources: &[usize],
        targets: &[usize],
    ) -> Result<Self, QisError> {
        if sources.len() != targets.len() {
            return Err(QisError::InvalidParameterValue(
                "excitation sources and targets must have equal lengths".into(),
            ));
        }
        if sources.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "excitation sources and targets must be nonempty".into(),
            ));
        }

        let mut source_modes = BTreeSet::new();
        let mut target_modes = BTreeSet::new();
        for (label, modes, seen) in [
            ("sources", sources, &mut source_modes),
            ("targets", targets, &mut target_modes),
        ] {
            for &mode in modes {
                if mode >= num_modes {
                    return Err(QisError::ModeOutOfBounds { mode, num_modes });
                }
                if !seen.insert(mode) {
                    return Err(QisError::InvalidParameterValue(format!(
                        "excitation {label} contain repeated mode {mode}"
                    )));
                }
            }
        }
        if let Some(mode) = source_modes.intersection(&target_modes).next() {
            return Err(QisError::InvalidParameterValue(format!(
                "excitation sources and targets overlap at mode {mode}"
            )));
        }

        let term = targets.iter().copied().map(FermionicLadder::Create).chain(
            sources
                .iter()
                .rev()
                .copied()
                .map(FermionicLadder::Annihilate),
        );
        let excitation = Self::from_term(num_modes, term, 1.0.into())?;
        excitation
            .try_sub(&excitation.adjoint())?
            .scaled(Complex64::new(0.0, 1.0))
    }

    /// Returns the declared mode count.
    pub fn num_modes(&self) -> usize {
        self.num_modes
    }

    /// Returns the number of stored ordered products.
    pub fn num_terms(&self) -> usize {
        self.terms.len()
    }

    /// Iterates over products and their coefficients in deterministic structural order.
    pub fn terms(&self) -> impl ExactSizeIterator<Item = (&[FermionicLadder], Complex64)> + '_ {
        self.terms
            .iter()
            .map(|(term, coefficient)| (term.as_slice(), *coefficient))
    }

    /// Reports whether the stored sum is zero. Algebraic cancellations require normal ordering.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    fn check_modes(&self, other: &Self) -> Result<(), QisError> {
        if self.num_modes != other.num_modes {
            return Err(QisError::ModeMismatch {
                expected: self.num_modes,
                actual: other.num_modes,
            });
        }
        Ok(())
    }

    /// Adds operators on the same mode space, without modifying either operand.
    pub fn try_add(&self, other: &Self) -> Result<Self, QisError> {
        self.check_modes(other)?;
        let mut result = self.clone();
        for (term, coefficient) in other.terms() {
            result.add_term(term.iter().copied(), coefficient)?;
        }
        Ok(result)
    }

    /// Subtracts operators on the same mode space.
    pub fn try_sub(&self, other: &Self) -> Result<Self, QisError> {
        self.check_modes(other)?;
        let mut result = self.clone();
        for (term, coefficient) in other.terms() {
            result.add_term(term.iter().copied(), -coefficient)?;
        }
        Ok(result)
    }

    /// Forms `self * other`; the right operand acts first. Does not normal order.
    pub fn try_mul(&self, other: &Self) -> Result<Self, QisError> {
        self.check_modes(other)?;
        let mut result = Self::new(self.num_modes);
        for (left, a) in self.terms() {
            for (right, b) in other.terms() {
                result.add_term(left.iter().chain(right).copied(), a * b)?;
            }
        }
        Ok(result)
    }

    /// Scales all coefficients. The factor is validated even for the zero operator.
    pub fn scaled(&self, factor: Complex64) -> Result<Self, QisError> {
        validate_coefficient(factor)?;
        Self::from_terms(
            self.num_modes,
            self.terms
                .iter()
                .map(|(term, c)| (term.clone(), *c * factor)),
        )
    }

    /// Takes the conjugate transpose, reversing each product without reordering it.
    pub fn adjoint(&self) -> Self {
        Self {
            num_modes: self.num_modes,
            terms: self
                .terms
                .iter()
                .map(|(term, coefficient)| {
                    (
                        term.iter().rev().map(|ladder| ladder.adjoint()).collect(),
                        coefficient.conj(),
                    )
                })
                .collect(),
        }
    }

    /// Expands using the canonical anticommutation relations.
    ///
    /// Creators precede annihilators; each group uses ascending mode indices.
    /// This is an equivalent operator expansion, including contraction terms:
    /// `a_p a†_p = I - a†_p a_p`. Small nonzero coefficients are retained.
    /// An explicit work stack avoids recursion in long products.
    /// Sorting a branch of k ladders takes O(k²) comparisons; contractions can
    /// produce additional branches.
    pub fn normal_order(&self) -> Result<Self, QisError> {
        let mut result = Self::new(self.num_modes);
        let mut pending: Vec<_> = self
            .terms
            .iter()
            .map(|(term, c)| (term.clone(), *c, 0usize))
            .collect();
        while let Some((mut term, mut coefficient, mut index)) = pending.pop() {
            let mut complete = true;
            while index + 1 < term.len() {
                let left = term[index];
                let right = term[index + 1];
                if left == right {
                    complete = false;
                    break;
                }
                if left > right {
                    // Swapping distinct fermionic ladders contributes a minus sign.
                    // Equal-mode annihilator/creator pairs also contribute a contraction.
                    if let (FermionicLadder::Annihilate(p), FermionicLadder::Create(q)) =
                        (left, right)
                        && p == q
                    {
                        let mut contracted = term.clone();
                        contracted.drain(index..index + 2);
                        // The prefix is already sorted. Removing this pair only
                        // requires rechecking the new boundary with that prefix.
                        pending.push((contracted, coefficient, index.saturating_sub(1)));
                    }
                    term.swap(index, index + 1);
                    coefficient = -coefficient;
                    // Revisit the changed boundary, rather than scanning from zero.
                    index = index.saturating_sub(1);
                } else {
                    index += 1;
                }
            }
            if complete {
                result.add_term(term, coefficient)?;
            }
        }
        Ok(result)
    }

    /// Explicitly removes coefficients whose complex magnitude is at most `atol`.
    /// Invalid tolerances leave the operator unchanged.
    pub fn chop(&mut self, atol: f64) -> Result<(), QisError> {
        validate_tolerance(atol)?;
        self.terms
            .retain(|_, coefficient| coefficient.norm() > atol);
        Ok(())
    }

    /// Compares operators algebraically after normal ordering their difference.
    /// Different mode counts are an error; `atol` must be finite and nonnegative.
    pub fn equiv(&self, other: &Self, atol: f64) -> Result<bool, QisError> {
        validate_tolerance(atol)?;
        let difference = self.try_sub(other)?.normal_order()?;
        Ok(difference
            .terms
            .values()
            .all(|coefficient| coefficient.norm() <= atol))
    }

    /// Checks Hermiticity algebraically, allowing non-normal-ordered inputs.
    pub fn is_hermitian(&self, atol: f64) -> Result<bool, QisError> {
        self.equiv(&self.adjoint(), atol)
    }
}

macro_rules! binary_operator {
    ($trait:ident, $method:ident, $checked:ident) => {
        impl $trait<&FermionicOp> for &FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn $method(self, rhs: &FermionicOp) -> Self::Output {
                self.$checked(rhs)
            }
        }
        impl $trait<FermionicOp> for FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn $method(self, rhs: FermionicOp) -> Self::Output {
                self.$checked(&rhs)
            }
        }
        impl $trait<&FermionicOp> for FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn $method(self, rhs: &FermionicOp) -> Self::Output {
                self.$checked(rhs)
            }
        }
        impl $trait<FermionicOp> for &FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn $method(self, rhs: FermionicOp) -> Self::Output {
                self.$checked(&rhs)
            }
        }
    };
}

binary_operator!(Add, add, try_add);
binary_operator!(Sub, sub, try_sub);
binary_operator!(Mul, mul, try_mul);

macro_rules! scalar_operator {
    ($scalar:ty) => {
        impl Mul<$scalar> for &FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn mul(self, rhs: $scalar) -> Self::Output {
                self.scaled(rhs.into())
            }
        }
        impl Mul<$scalar> for FermionicOp {
            type Output = Result<FermionicOp, QisError>;
            fn mul(self, rhs: $scalar) -> Self::Output {
                self.scaled(rhs.into())
            }
        }
        impl Mul<&FermionicOp> for $scalar {
            type Output = Result<FermionicOp, QisError>;
            fn mul(self, rhs: &FermionicOp) -> Self::Output {
                rhs.scaled(self.into())
            }
        }
        impl Mul<FermionicOp> for $scalar {
            type Output = Result<FermionicOp, QisError>;
            fn mul(self, rhs: FermionicOp) -> Self::Output {
                rhs.scaled(self.into())
            }
        }
    };
}

scalar_operator!(f64);
scalar_operator!(Complex64);

impl Neg for &FermionicOp {
    type Output = FermionicOp;
    fn neg(self) -> Self::Output {
        let mut result = self.clone();
        for coefficient in result.terms.values_mut() {
            *coefficient = -*coefficient;
        }
        result
    }
}

impl Neg for FermionicOp {
    type Output = Self;
    fn neg(mut self) -> Self::Output {
        for coefficient in self.terms.values_mut() {
            *coefficient = -*coefficient;
        }
        self
    }
}
