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

//! Single-qubit Pauli operators and arithmetic.
//!
//! This module provides the [`Pauli`] enum representing the four single-qubit
//! Pauli matrices (I, X, Y, Z) and [`PauliString`] for multi-qubit Pauli operators.
//!
//! # Features
//! - Matrix representation conversion
//! - Symplectic encoding for efficient computation
//! - Group multiplication with phase tracking
//! - Commutator computation using bitwise operations
//!
//! # Examples
//!
//! ```
//! use cqlib_core::qis::pauli::{Pauli, PauliString};
//!
//! // Create a single-qubit Pauli operator
//! let x = Pauli::X;
//! let z = Pauli::Z;
//!
//! // Pauli multiplication with phase tracking: X * Z = -iY
//! let (p, phase) = x.mul_with_phase(z);
//!
//! // Create a multi-qubit Pauli string
//! let mut ps = PauliString::new(3);
//! ps.set_pauli(0, Pauli::X);
//! ps.set_pauli(1, Pauli::Z);
//! println!("{}", ps.to_string());
//! // ps now represents X ⊗ Z ⊗ I
//! ```

use crate::qis::error::{PauliStringParseError, QisError};
use bitvec::prelude::*;
use ndarray::{Array2, arr2};
use num_complex::Complex64;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign};
use std::str::FromStr;

/// Phase factor in the Pauli group, isomorphic to Z4 (the cyclic group of order 4).
///
/// Represents powers of the imaginary unit: $i^n$ where $n \in \{0, 1, 2, 3\}$.
///
/// # Mathematical Mapping
///
/// | Variant | Value | Exponent |
/// |---------|-------|----------|
/// | `Plus`  | $1$   | 0        |
/// | `I`     | $i$   | 1        |
/// | `Minus` | $-1$  | 2        |
/// | `MinusI`| $-i$  | 3        |
///
/// Arithmetic follows the exponent addition rule: $i^a \cdot i^b = i^{(a+b) \bmod 4}$.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::pauli::Phase;
///
/// let p1 = Phase::I;      // i
/// let p2 = Phase::I;      // i
/// let p3 = p1 + p2;       // i * i = -1
/// assert_eq!(p3, Phase::Minus);
///
/// // Convert to complex number
/// let c = Phase::MinusI.to_complex();  // -i
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Phase {
    /// $i^0 = 1$
    Plus = 0,
    /// $i^1 = i$
    I = 1,
    /// $i^2 = -1$
    Minus = 2,
    /// $i^3 = -i$
    MinusI = 3,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Phase::Plus => "1",
            Phase::I => "i",
            Phase::Minus => "-1",
            Phase::MinusI => "-i",
        };
        f.write_str(s)
    }
}

impl From<u8> for Phase {
    /// Converts a `u8` to a `Phase` by taking modulo 4.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::Phase;
    ///
    /// assert_eq!(Phase::from(0), Phase::Plus);
    /// assert_eq!(Phase::from(5), Phase::I);    // 5 % 4 = 1
    /// assert_eq!(Phase::from(6), Phase::Minus);// 6 % 4 = 2
    /// ```
    fn from(val: u8) -> Self {
        match val % 4 {
            0 => Phase::Plus,
            1 => Phase::I,
            2 => Phase::Minus,
            3 => Phase::MinusI,
            _ => unreachable!(),
        }
    }
}

impl Add for Phase {
    type Output = Phase;

    /// Adds two phases (multiplication in the group).
    ///
    /// # Formula
    ///
    /// $i^a \cdot i^b = i^{(a+b) \bmod 4}$
    fn add(self, other: Phase) -> Phase {
        Phase::from(self as u8 + other as u8)
    }
}

impl Add<u8> for Phase {
    type Output = Phase;

    fn add(self, rhs: u8) -> Phase {
        Phase::from(self as u8 + rhs)
    }
}

impl AddAssign for Phase {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl AddAssign<u8> for Phase {
    fn add_assign(&mut self, rhs: u8) {
        *self = *self + rhs;
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for Phase {
    type Output = Phase;

    /// Multiplies two phases (same as addition in Z4).
    fn mul(self, rhs: Self) -> Self::Output {
        self + rhs
    }
}

impl Phase {
    /// Converts the phase to a complex number.
    ///
    /// # Returns
    ///
    /// The complex value of $i^n$.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::Phase;
    /// use num_complex::Complex64;
    ///
    /// assert_eq!(Phase::Plus.to_complex(), Complex64::new(1.0, 0.0));
    /// assert_eq!(Phase::I.to_complex(), Complex64::new(0.0, 1.0));
    /// assert_eq!(Phase::Minus.to_complex(), Complex64::new(-1.0, 0.0));
    /// assert_eq!(Phase::MinusI.to_complex(), Complex64::new(0.0, -1.0));
    /// ```
    pub fn to_complex(&self) -> Complex64 {
        match self {
            Phase::Plus => Complex64::new(1.0, 0.0),
            Phase::I => Complex64::new(0.0, 1.0),
            Phase::Minus => Complex64::new(-1.0, 0.0),
            Phase::MinusI => Complex64::new(0.0, -1.0),
        }
    }
}

/// Single-qubit Pauli operators.
///
/// The four Pauli matrices form the basis of single-qubit quantum operations and
/// are fundamental to quantum error correction and stabilizer formalism.
///
/// # Matrix Representations
///
/// | Operator | Matrix | Description |
/// |----------|--------|-------------|
/// | `I` | $\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}$ | Identity |
/// | `X` | $\begin{pmatrix} 0 & 1 \\ 1 & 0 \end{pmatrix}$ | Bit-flip (Pauli-X) |
/// | `Y` | $\begin{pmatrix} 0 & -i \\ i & 0 \end{pmatrix}$ | Pauli-Y |
/// | `Z` | $\begin{pmatrix} 1 & 0 \\ 0 & -1 \end{pmatrix}$ | Phase-flip (Pauli-Z) |
///
/// # Multiplication Rules
///
/// Pauli operators anticommute: $\sigma_a \sigma_b = -\sigma_b \sigma_a$ for $a \neq b$.
///
/// ```
/// use cqlib_core::qis::pauli::Pauli;
///
/// // X * Z = -iY (phase is tracked separately)
/// let (p, _) = Pauli::X.mul_with_phase(Pauli::Z);
/// assert_eq!(p, Pauli::Y);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pauli {
    /// Pauli-X (bit-flip) operator: $\sigma_x = |0\rangle\langle1| + |1\rangle\langle0|$.
    X,
    /// Pauli-Y operator: $\sigma_y = -i|0\rangle\langle1| + i|1\rangle\langle0|$.
    Y,
    /// Pauli-Z (phase-flip) operator: $\sigma_z = |0\rangle\langle0| - |1\rangle\langle1|$.
    Z,
    /// Identity operator: $I = |0\rangle\langle0| + |1\rangle\langle1|$.
    I,
}

impl TryFrom<char> for Pauli {
    type Error = QisError;

    fn try_from(s: char) -> Result<Self, Self::Error> {
        match s {
            'I' => Ok(Pauli::I),
            'X' => Ok(Pauli::X),
            'Y' => Ok(Pauli::Y),
            'Z' => Ok(Pauli::Z),
            other => Err(QisError::InvalidParameterValue(format!(
                "invalid Pauli character '{}'",
                other
            ))),
        }
    }
}

impl FromStr for Pauli {
    type Err = QisError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(QisError::InvalidParameterValue(
                "empty string cannot be converted to Pauli".to_string(),
            ));
        }
        if s.len() > 1 {
            return Err(QisError::InvalidParameterValue(format!(
                "expected a single Pauli character, got \"{}\"",
                s
            )));
        }
        s.chars().next().unwrap().try_into()
    }
}

impl fmt::Display for Pauli {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Pauli::I => "I",
            Pauli::X => "X",
            Pauli::Y => "Y",
            Pauli::Z => "Z",
        };
        write!(f, "{}", s)
    }
}

impl Pauli {
    /// Returns the symplectic representation `(x, z)` for efficient computation.
    ///
    /// The symplectic encoding maps Pauli operators to binary pairs:
    ///
    /// | Operator | x | z |
    /// |----------|---|---|
    /// | `I`      | 0 | 0 |
    /// | `X`      | 1 | 0 |
    /// | `Y`      | 1 | 1 |
    /// | `Z`      | 0 | 1 |
    ///
    /// This encoding enables efficient multiplication via XOR and phase lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::Pauli;
    ///
    /// assert_eq!(Pauli::I.to_symplectic(), (0, 0));
    /// assert_eq!(Pauli::X.to_symplectic(), (1, 0));
    /// assert_eq!(Pauli::Y.to_symplectic(), (1, 1));
    /// assert_eq!(Pauli::Z.to_symplectic(), (0, 1));
    /// ```
    pub fn to_symplectic(&self) -> (u8, u8) {
        match self {
            Pauli::I => (0, 0),
            Pauli::X => (1, 0),
            Pauli::Y => (1, 1),
            Pauli::Z => (0, 1),
        }
    }

    /// Returns the 2x2 complex matrix representation.
    ///
    /// Suitable for small system debugging and verification. For larger systems,
    /// prefer the symplectic representation for performance.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::Pauli;
    /// use num_complex::Complex64;
    ///
    /// let x_mat = Pauli::X.to_matrix();
    /// // X = [[0, 1], [1, 0]]
    /// assert_eq!(x_mat[[0, 1]], Complex64::new(1.0, 0.0));
    /// ```
    pub fn to_matrix(&self) -> Array2<Complex64> {
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        let i = Complex64::new(0.0, 1.0);
        let neg_one = Complex64::new(-1.0, 0.0);
        let neg_i = Complex64::new(0.0, -1.0);

        match self {
            Pauli::I => arr2(&[[one, zero], [zero, one]]),
            Pauli::X => arr2(&[[zero, one], [one, zero]]),
            Pauli::Y => arr2(&[[zero, neg_i], [i, zero]]),
            Pauli::Z => arr2(&[[one, zero], [zero, neg_one]]),
        }
    }

    /// Multiplies two Pauli operators, returning the result and phase factor.
    ///
    /// Implements the Pauli group multiplication rules:
    /// - $XY = iZ$, $YX = -iZ$
    /// - $YZ = iX$, $ZY = -iX$
    /// - $ZX = iY$, $XZ = -iY$
    ///
    /// # Arguments
    ///
    /// * `other` - The right-hand side Pauli operator.
    ///
    /// # Returns
    ///
    /// A tuple `(result, phase)` where `result` is the resulting Pauli operator
    /// and `phase` is the phase factor from the multiplication.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, Phase};
    ///
    /// // X * Y = iZ
    /// let (p, ph) = Pauli::X.mul_with_phase(Pauli::Y);
    /// assert_eq!(p, Pauli::Z);
    /// assert_eq!(ph, Phase::I);
    ///
    /// // Y * X = -iZ (anticommutation)
    /// let (p, ph) = Pauli::Y.mul_with_phase(Pauli::X);
    /// assert_eq!(p, Pauli::Z);
    /// assert_eq!(ph, Phase::MinusI);
    /// ```
    pub fn mul_with_phase(&self, other: Pauli) -> (Pauli, Phase) {
        match (self, other) {
            (Pauli::I, p) => (p, Phase::Plus),
            (p, Pauli::I) => (*p, Phase::Plus),
            (Pauli::X, Pauli::X) => (Pauli::I, Phase::Plus),
            (Pauli::Y, Pauli::Y) => (Pauli::I, Phase::Plus),
            (Pauli::Z, Pauli::Z) => (Pauli::I, Phase::Plus),
            (Pauli::X, Pauli::Y) => (Pauli::Z, Phase::I), // XY = iZ
            (Pauli::X, Pauli::Z) => (Pauli::Y, Phase::MinusI), // XZ = -iY
            (Pauli::Y, Pauli::X) => (Pauli::Z, Phase::MinusI), // YX = -iZ
            (Pauli::Y, Pauli::Z) => (Pauli::X, Phase::I), // YZ = iX
            (Pauli::Z, Pauli::X) => (Pauli::Y, Phase::I), // ZX = iY
            (Pauli::Z, Pauli::Y) => (Pauli::X, Phase::MinusI), // ZY = -iX
        }
    }
}

/// Multi-qubit Pauli string operator in symplectic representation.
///
/// A Pauli string is a tensor product of single-qubit Pauli operators across
/// multiple qubits: $P = \bigotimes_{i=0}^{N-1} P_i$ where $P_i \in \{I, X, Y, Z\}$.
///
/// This representation uses the symplectic encoding for efficient storage
/// and manipulation (complexity $O(N/64)$ for $N$ qubits).
///
/// # Memory Layout
///
/// - `x[i]` = 1 indicates an X component on qubit `i`
/// - `z[i]` = 1 indicates a Z component on qubit `i`
/// - `(x[i], z[i])` pairs encode: I=(0,0), X=(1,0), Y=(1,1), Z=(0,1)
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::pauli::{Pauli, PauliString};
///
/// // Create a 3-qubit Pauli string: X ⊗ Z ⊗ I
/// let mut ps = PauliString::new(3);
/// ps.set_pauli(0, Pauli::X);
/// ps.set_pauli(1, Pauli::Z);
///
/// // Check commutation with another Pauli string
/// let mut other = PauliString::new(3);
/// other.set_pauli(0, Pauli::Z);
/// other.set_pauli(1, Pauli::X);
///
/// assert!(ps.commutes_with(&other)); // [X⊗Z, Z⊗X] ≠ 0
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PauliString {
    /// Number of qubits ($N$) in the string.
    pub num_qubits: usize,
    /// Global phase factor ($\pm 1, \pm i$).
    pub phase: Phase,
    /// Z-component bit vector of length `num_qubits`.
    ///
    /// `z[i]` = 1 indicates the $i$-th qubit has a Z component.
    pub z: BitVec,
    /// X-component bit vector of length `num_qubits`.
    ///
    /// `x[i]` = 1 indicates the $i$-th qubit has an X component.
    pub x: BitVec,
}

/// Iterator over a Pauli string in ascending qubit-index order.
pub struct PauliIter<'a> {
    pauli: &'a PauliString,
    indices: std::ops::Range<usize>,
}

impl Iterator for PauliIter<'_> {
    type Item = Pauli;

    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(|index| self.pauli.get_pauli(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl ExactSizeIterator for PauliIter<'_> {}
impl std::iter::FusedIterator for PauliIter<'_> {}

impl<'a> IntoIterator for &'a PauliString {
    type Item = Pauli;
    type IntoIter = PauliIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Display for PauliString {
    /// Formats the Pauli string as “+XYZ” or “-iZIX”.
    ///
    /// Qubits are displayed in reverse index order (highest index first),
    /// which is the conventional tensor product notation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let phase_str = match self.phase {
            Phase::Plus => "+",
            Phase::I => "+i",
            Phase::Minus => "-",
            Phase::MinusI => "-i",
        };
        write!(f, "{}", phase_str)?;

        // Iterate in reverse order for conventional tensor product notation
        for i in (0..self.num_qubits).rev() {
            let char_code = match (self.x[i], self.z[i]) {
                (false, false) => 'I',
                (true, false) => 'X',
                (false, true) => 'Z',
                (true, true) => 'Y',
            };
            write!(f, "{}", char_code)?;
        }

        Ok(())
    }
}

impl PauliString {
    /// Iterates over operators from qubit index 0 upward.
    pub fn iter(&self) -> PauliIter<'_> {
        PauliIter {
            pauli: self,
            indices: 0..self.num_qubits,
        }
    }

    /// Creates a new identity Pauli string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::PauliString;
    ///
    /// let ps = PauliString::new(3);
    /// assert_eq!(ps.to_string(), "+III");
    /// ```
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            phase: Phase::Plus,
            z: bitvec![0; num_qubits],
            x: bitvec![0; num_qubits],
        }
    }

    /// Sets the Pauli operator at the specified qubit index.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= num_qubits`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    ///
    /// let mut ps = PauliString::new(2);
    /// ps.set_pauli(0, Pauli::X);
    /// ps.set_pauli(1, Pauli::Z);
    /// assert_eq!(ps.to_string(), "+ZX");
    /// ```
    pub fn set_pauli(&mut self, idx: usize, p: Pauli) {
        assert!(
            idx < self.num_qubits,
            "Index {} out of bounds for {} qubits",
            idx,
            self.num_qubits
        );
        let (x_val, z_val) = p.to_symplectic();
        self.x.set(idx, x_val == 1);
        self.z.set(idx, z_val == 1);
    }

    /// Gets the Pauli operator at the specified qubit index.
    ///
    /// # Arguments
    ///
    /// * `idx` - The qubit index.
    ///
    /// # Returns
    ///
    /// The Pauli operator at the specified index.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    ///
    /// // PauliString uses highest-index-first string convention (same as Display):
    /// // "XZI" → qubit 2 = X, qubit 1 = Z, qubit 0 = I
    /// let ps: PauliString = "XZI".parse().unwrap();
    /// assert_eq!(ps.get_pauli(0), Pauli::I);
    /// assert_eq!(ps.get_pauli(1), Pauli::Z);
    /// assert_eq!(ps.get_pauli(2), Pauli::X);
    /// ```
    pub fn get_pauli(&self, idx: usize) -> Pauli {
        assert!(
            idx < self.num_qubits,
            "Index {} out of bounds for {} qubits",
            idx,
            self.num_qubits
        );
        let x_bit = self.x[idx];
        let z_bit = self.z[idx];
        match (x_bit, z_bit) {
            (false, false) => Pauli::I,
            (true, false) => Pauli::X,
            (false, true) => Pauli::Z,
            (true, true) => Pauli::Y,
        }
    }

    /// Returns the dense complex matrix representation of this Pauli string.
    ///
    /// Qubit zero is the least-significant tensor factor, so an $N$-qubit
    /// string is expanded as $P_{N-1} \otimes \cdots \otimes P_0$. The
    /// string's global [`Phase`] is included in the returned matrix.
    ///
    /// This method is intended for small-system analysis and verification.
    /// The returned matrix has dimensions $2^N \times 2^N$ and requires
    /// $O(4^N)$ memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::PauliString;
    /// use num_complex::Complex64;
    ///
    /// let pauli: PauliString = "X".parse().unwrap();
    /// let matrix = pauli.to_matrix();
    /// assert_eq!(matrix[[0, 1]], Complex64::new(1.0, 0.0));
    /// assert_eq!(matrix[[1, 0]], Complex64::new(1.0, 0.0));
    /// ```
    pub fn to_matrix(&self) -> Array2<Complex64> {
        let mut matrix = Array2::from_elem((1, 1), Complex64::new(1.0, 0.0));
        for qubit in (0..self.num_qubits).rev() {
            matrix = kronecker_product(&matrix, &self.get_pauli(qubit).to_matrix());
        }

        let phase = self.phase.to_complex();
        matrix.mapv_inplace(|value| value * phase);
        matrix
    }

    /// Returns the Pauli operator at the specified qubit index, or an error if out of bounds.
    ///
    /// # Arguments
    ///
    /// * idx - The qubit index.
    ///
    /// # Returns
    ///
    /// The Pauli operator at the specified index, or [QisError::IndexOutOfBounds].
    pub fn try_get_pauli(&self, idx: usize) -> Result<Pauli, QisError> {
        if idx >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: idx,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        Ok(self.get_pauli(idx))
    }

    /// Sets the Pauli operator at the specified qubit index, or returns an error if out of bounds.
    ///
    /// # Arguments
    ///
    /// * idx - The qubit index.
    /// * pauli - The Pauli operator to set.
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or [QisError::IndexOutOfBounds] if the index is out of bounds.
    pub fn try_set_pauli(&mut self, idx: usize, pauli: Pauli) -> Result<(), QisError> {
        if idx >= self.num_qubits {
            return Err(QisError::IndexOutOfBounds {
                index: idx,
                max: self.num_qubits.saturating_sub(1),
            });
        }
        self.set_pauli(idx, pauli);
        Ok(())
    }
    /// Checks if this Pauli string commutes with another.
    ///
    /// Two Pauli strings commute if their symplectic inner product is 0 (mod 2).
    ///
    /// # Complexity
    ///
    /// O(N/64) using bitwise operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    ///
    /// let mut p1 = PauliString::new(2);
    /// p1.set_pauli(0, Pauli::X);
    ///
    /// let mut p2 = PauliString::new(2);
    /// p2.set_pauli(0, Pauli::X);
    /// assert!(p1.commutes_with(&p2)); // Same operator commutes
    ///
    /// let mut p3 = PauliString::new(2);
    /// p3.set_pauli(0, Pauli::Z);
    /// assert!(!p1.commutes_with(&p3)); // X and Z anticommute
    /// ```
    pub fn commutes_with(&self, other: &Self) -> bool {
        assert_eq!(self.num_qubits, other.num_qubits);

        // Symplectic inner product (mod 2)
        let term1 = self.x.clone() & &other.z;
        let term2 = self.z.clone() & &other.x;
        let anti_commutations = (term1 ^ term2).count_ones();

        anti_commutations & 1 == 0
    }

    /// Converts the X bit vector to a usize mask.
    ///
    /// The i-th bit of the returned value corresponds to the X component of qubit i.
    ///
    /// # Examples
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    ///
    /// let mut ps = PauliString::new(3);
    /// ps.set_pauli(0, Pauli::X);
    /// ps.set_pauli(2, Pauli::Y); // Y has both X and Z
    /// assert_eq!(ps.x_mask(), 0b101); // X on qubit 0 and Z on qubit 2
    /// ```
    pub fn x_mask(&self) -> usize {
        self.x.iter().enumerate().fold(
            0usize,
            |acc, (i, bit)| {
                if *bit { acc | (1 << i) } else { acc }
            },
        )
    }

    /// Converts the Z bit vector to a usize mask.
    ///
    /// The i-th bit of the returned value corresponds to the Z component of qubit i.
    pub fn z_mask(&self) -> usize {
        self.z.iter().enumerate().fold(
            0usize,
            |acc, (i, bit)| {
                if *bit { acc | (1 << i) } else { acc }
            },
        )
    }

    /// Returns the support of the Pauli string (indices of non-identity operators).
    ///
    /// The support is the set of qubit indices where the Pauli operator is not I.
    /// This is useful for identifying which qubits participate in a Pauli operation.
    ///
    /// # Returns
    ///
    /// A vector of qubit indices where the Pauli operator is X, Y, or Z (not I).
    /// Indices are returned in ascending order.
    ///
    /// # Examples
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    ///
    /// // "XZI" uses highest-index-first convention: qubit 2=X, qubit 1=Z, qubit 0=I
    /// let ps: PauliString = "XZI".parse().unwrap();
    /// assert_eq!(ps.support(), vec![1, 2]);
    ///
    /// // Y ⊗ I ⊗ Y has support on qubits 0 and 2
    /// let mut ps = PauliString::new(3);
    /// ps.set_pauli(0, Pauli::Y);
    /// ps.set_pauli(2, Pauli::Y);
    /// assert_eq!(ps.support(), vec![0, 2]);
    ///
    /// // Identity has empty support
    /// let ps = PauliString::new(3);
    /// assert!(ps.support().is_empty());
    /// ```
    pub fn support(&self) -> Vec<usize> {
        let mut support = Vec::new();
        for i in 0..self.num_qubits {
            // Non-identity if either x or z bit is set
            if self.x[i] || self.z[i] {
                support.push(i);
            }
        }
        support
    }

    /// Computes the phase factor contributed by Y operators.
    ///
    /// In the symplectic representation, Y = iXZ. When there are `n` Y operators,
    /// the total phase contributed is i^n, which cycles through {1, i, -1, -i}.
    ///
    /// # Returns
    /// The complex phase factor (1, i, -1, or -i) corresponding to i^n where n is the Y count.
    ///
    /// # Examples
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    /// use num_complex::Complex64;
    ///
    /// // No Y operators: phase = 1
    /// let ps1: PauliString = "XZI".parse().unwrap();
    /// assert_eq!(ps1.y_phase(), Complex64::new(1.0, 0.0));
    ///
    /// // One Y operator: phase = i
    /// let ps2: PauliString = "YII".parse().unwrap();
    /// assert_eq!(ps2.y_phase(), Complex64::new(0.0, 1.0));
    ///
    /// // Two Y operators: phase = -1 (i^2)
    /// let mut ps3 = PauliString::new(2);
    /// ps3.set_pauli(0, Pauli::Y);
    /// ps3.set_pauli(1, Pauli::Y);
    /// assert_eq!(ps3.y_phase(), Complex64::new(-1.0, 0.0));
    /// ```
    pub fn y_phase(&self) -> Complex64 {
        // Count Y operators (where both X and Z are set)
        let y_count: u32 = self
            .x
            .iter()
            .zip(self.z.iter())
            .filter(|(x_bit, z_bit)| **x_bit && **z_bit)
            .count() as u32;

        // Y = iXZ, so Y^n contributes i^n
        match y_count % 4 {
            0 => Complex64::new(1.0, 0.0),
            1 => Complex64::new(0.0, 1.0),
            2 => Complex64::new(-1.0, 0.0),
            3 => Complex64::new(0.0, -1.0),
            _ => unreachable!(),
        }
    }

    /// Computes the expectation value given a probability distribution over computational basis states.
    ///
    /// This calculates ⟨P⟩ = Σ_s p(s) ⟨s|P|s⟩, where p(s) is the probability of basis state |s⟩.
    ///
    /// # Important Notes on Conventions
    /// - The string keys in `probs` use **little-endian** convention: the rightmost character corresponds to qubit 0.
    ///   For example, "01" means qubit 0 = 1, qubit 1 = 0 (state |10⟩ in big-endian notation).
    /// - If this Pauli string contains X or Y operators (non-diagonal), the expectation value is 0
    ///   for any probability distribution over computational basis states.
    /// - For Pauli strings containing only Z and I operators, the expectation is:
    ///   ⟨P⟩ = phase × Σ_s p(s) × (-1)^{Σ_i (z[i] × s[i])}
    ///
    /// # Arguments
    /// * `probs` - A HashMap mapping state strings (e.g., "00", "01") to their probabilities.
    ///   The string uses little-endian: index 0 (leftmost) is qubit n-1, index n-1 (rightmost) is qubit 0.
    ///
    /// # Returns
    /// The expectation value as a real number (f64).
    ///
    /// # Errors
    /// Returns `QisError::DimensionMismatch` if any state string has length different from `self.num_qubits`.
    /// Returns `QisError::PauliStringParseError` if any state string contains characters other than '0' or '1'.
    ///
    /// # Examples
    /// ```
    /// use cqlib_core::qis::pauli::{Pauli, PauliString};
    /// use std::collections::HashMap;
    ///
    /// // Create Z on qubit 0
    /// let mut ps = PauliString::new(2);
    /// ps.set_pauli(0, Pauli::Z);
    ///
    /// // Probability distribution: 50% |00⟩, 50% |01⟩ (little-endian strings)
    /// let mut probs = HashMap::new();
    /// probs.insert("00".to_string(), 0.5); // |00⟩: qubit1=0, qubit0=0
    /// probs.insert("01".to_string(), 0.5); // |01⟩: qubit1=0, qubit0=1
    ///
    /// // ⟨Z⟩ = 0.5 × 1 + 0.5 × (-1) = 0
    /// let exp = ps.expectation(&probs).unwrap();
    /// assert!((exp).abs() < 1e-10);
    /// ```
    pub fn expectation(
        &self,
        probs: &std::collections::HashMap<String, f64>,
    ) -> Result<f64, crate::qis::error::QisError> {
        // Check if this Pauli string contains X or Y operators
        // X is represented by x[i]=1, z[i]=0
        // Y is represented by x[i]=1, z[i]=1
        // So if any bit in x is set, there is X or Y, and expectation is 0
        if self.x.iter().any(|bit| *bit) {
            return Ok(0.0);
        }

        // Only Z and I operators remain (diagonal in computational basis)
        let z_mask = self.z_mask();
        let global_phase = self.phase.to_complex();

        let mut exp_value = 0.0;

        for (state_str, prob) in probs {
            if state_str.len() != self.num_qubits {
                return Err(crate::qis::error::QisError::DimensionMismatch {
                    expected: self.num_qubits,
                    actual: state_str.len(),
                });
            }

            // Parse state string to index using little-endian convention
            // "01" -> qubit 0 = 1, qubit 1 = 0 -> index = 0b01 = 1
            let mut state_idx = 0usize;
            for (i, c) in state_str.chars().rev().enumerate() {
                match c {
                    '1' => state_idx |= 1 << i,
                    '0' => {}
                    _ => {
                        return Err(crate::qis::error::QisError::PauliStringParseError(
                            crate::qis::error::PauliStringParseError::InvalidCharacter(c),
                        ));
                    }
                }
            }

            // Calculate eigenvalue: (-1)^{number of overlapping Z bits that are 1}
            // For each qubit i with Z operator (z[i]=1) and state s[i]=1, contribute factor -1
            let overlap = state_idx & z_mask;
            let parity = overlap.count_ones();
            let eigenvalue = if parity & 1 == 0 { 1.0 } else { -1.0 };

            exp_value += prob * eigenvalue;
        }

        // Apply global phase and return real part
        // For Hermitian observables (phase = ±1), this is just ±exp_value
        Ok((exp_value * global_phase).re)
    }

    /// Computes the phase exponent for multiplying two Pauli operators.
    ///
    /// Returns the exponent `n` such that `P1 * P2 = i^n * P3`.
    ///
    /// # Phase table
    ///
    /// | Product | Phase | Exponent |
    /// |---------|-------|----------|
    /// | X * Z   | -i    | 3        |
    /// | Z * X   | i     | 1        |
    /// | X * Y   | i     | 1        |
    /// | Y * X   | -i    | 3        |
    /// | Z * Y   | -i    | 3        |
    /// | Y * Z   | i     | 1        |
    #[inline]
    fn calculate_phase_step(x1: bool, z1: bool, x2: bool, z2: bool) -> u8 {
        match (x1, z1, x2, z2) {
            // Same operator or identity: no phase
            (x1, z1, x2, z2) if x1 == x2 && z1 == z2 => 0,
            (false, false, _, _) => 0, // self is I
            (_, _, false, false) => 0, // other is I

            // X * Z = -i (exponent 3)
            (true, false, false, true) => 3,
            // Z * X = i (exponent 1)
            (false, true, true, false) => 1,

            // X * Y = i (exponent 1)
            (true, false, true, true) => 1,
            // Y * X = -i (exponent 3)
            (true, true, true, false) => 3,

            // Z * Y = -i (exponent 3)
            (false, true, true, true) => 3,
            // Y * Z = i (exponent 1)
            (true, true, false, true) => 1,

            _ => 0,
        }
    }
}

fn kronecker_product(left: &Array2<Complex64>, right: &Array2<Complex64>) -> Array2<Complex64> {
    let (left_rows, left_cols) = left.dim();
    let (right_rows, right_cols) = right.dim();
    let mut result = Array2::from_elem(
        (left_rows * right_rows, left_cols * right_cols),
        Complex64::new(0.0, 0.0),
    );

    for left_row in 0..left_rows {
        for left_col in 0..left_cols {
            for right_row in 0..right_rows {
                for right_col in 0..right_cols {
                    result[[
                        left_row * right_rows + right_row,
                        left_col * right_cols + right_col,
                    ]] = left[[left_row, left_col]] * right[[right_row, right_col]];
                }
            }
        }
    }

    result
}

/// Multiplies two Pauli strings, returning a new instance.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::pauli::{Pauli, PauliString};
///
/// let mut p1 = PauliString::new(2);
/// p1.set_pauli(0, Pauli::X);
///
/// let mut p2 = PauliString::new(2);
/// p2.set_pauli(0, Pauli::Z);
///
/// let product = &p1 * &p2;
/// assert_eq!(product.to_string(), "-iIY");
/// ```
impl Mul for &PauliString {
    type Output = PauliString;

    fn mul(self, rhs: Self) -> PauliString {
        let mut result = self.clone();
        result *= rhs;
        result
    }
}

/// In-place multiplication of Pauli strings.
///
/// Updates `self` with the product `self * rhs`.
///
/// # Panics
///
/// Panics if qubit counts differ.
///
/// # Examples
///
/// ```
/// use cqlib_core::qis::pauli::{Pauli, PauliString};
///
/// let mut p1 = PauliString::new(2);
/// p1.set_pauli(0, Pauli::X);
///
/// let p2 = PauliString::new(2);
/// p1 *= &p2; // Multiply by identity
/// assert_eq!(p1.to_string(), "+IX");
/// ```
impl MulAssign<&PauliString> for PauliString {
    fn mul_assign(&mut self, rhs: &PauliString) {
        assert_eq!(self.num_qubits, rhs.num_qubits, "Qubit count mismatch");

        // Calculate phase update before modifying bit vectors
        for i in 0..self.num_qubits {
            let step = PauliString::calculate_phase_step(self.x[i], self.z[i], rhs.x[i], rhs.z[i]);
            self.phase += step;
        }
        self.phase += rhs.phase;

        // Update bit vectors using XOR (symplectic addition)
        self.x ^= &rhs.x;
        self.z ^= &rhs.z;
    }
}

impl FromStr for PauliString {
    type Err = PauliStringParseError;

    /// Parses a PauliString from a string.
    ///
    /// The format is: `[+|-][i|j]<pauli operators>`
    /// where pauli operators are I, X, Y, or Z.
    ///
    /// Qubits are in reverse order: the first character corresponds to the highest qubit index.
    ///
    /// # Examples
    /// ```
    /// use cqlib_core::qis::pauli::PauliString;
    ///
    /// // Parse without phase prefix
    /// let ps: PauliString = "XZI".parse().unwrap();
    /// assert_eq!(ps.to_string(), "+XZI");
    ///
    /// // Parse with + phase (explicit)
    /// let ps: PauliString = "+XYZ".parse().unwrap();
    /// assert_eq!(ps.to_string(), "+XYZ");
    ///
    /// // Parse with -i phase
    /// let ps: PauliString = "-iZII".parse().unwrap();
    /// assert_eq!(ps.to_string(), "-iZII");
    ///
    /// // Parse with +j (alternative for +i)
    /// let ps: PauliString = "+jX".parse().unwrap();
    /// assert_eq!(ps.to_string(), "+iX");
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PauliStringParseError::EmptyString);
        }

        let mut chars = s.chars().peekable();

        // Parse optional phase
        let mut phase = Phase::Plus;

        // Check for sign
        if let Some(&c) = chars.peek() {
            if c == '+' || c == '-' {
                let sign = if c == '+' { 0 } else { 2 }; // Plus = 0, Minus = 2
                chars.next();

                // Check for 'i' or 'j' after sign
                if let Some(&c) = chars.peek() {
                    if c == 'i' || c == 'j' {
                        chars.next();
                        phase = if sign == 0 { Phase::I } else { Phase::MinusI };
                    } else {
                        phase = if sign == 0 { Phase::Plus } else { Phase::Minus };
                    }
                } else {
                    // Just "+" or "-" without operators
                    return Err(PauliStringParseError::NoOperators);
                }
            } else if c == 'i' || c == 'j' {
                // Implied + sign
                chars.next();
                phase = Phase::I;
            }
        }

        // Collect Pauli operators
        let mut operators: Vec<char> = Vec::new();
        for c in chars {
            match c {
                'I' | 'X' | 'Y' | 'Z' => operators.push(c),
                c => return Err(PauliStringParseError::InvalidCharacter(c)),
            }
        }

        if operators.is_empty() {
            return Err(PauliStringParseError::NoOperators);
        }

        // Create PauliString with correct dimensions
        let num_qubits = operators.len();
        let mut result = PauliString::new(num_qubits);
        result.phase = phase;

        // Set operators in reverse order (highest index first in string)
        for (i, &op) in operators.iter().rev().enumerate() {
            let pauli = match op {
                'I' => Pauli::I,
                'X' => Pauli::X,
                'Y' => Pauli::Y,
                'Z' => Pauli::Z,
                _ => unreachable!(),
            };
            result.set_pauli(i, pauli);
        }

        Ok(result)
    }
}

#[cfg(test)]
#[path = "./pauli_test.rs"]
mod tests;
