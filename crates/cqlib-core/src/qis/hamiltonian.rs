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
//! observable as a linear combination of Pauli strings (tensor products of Pauli operators)
//! with complex coefficients. It is primarily used for defining system energies,
//! observables for expectation value calculations, and operators for time evolution.

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
    /// Returns [`QisError::QubitMismatch`] when a term's qubit count differs
    /// from [`Hamiltonian::num_qubits`].
    pub fn to_matrix(&self) -> Result<Array2<Complex64>, QisError> {
        for (pauli, _) in &self.terms {
            if pauli.num_qubits != self.num_qubits {
                return Err(QisError::QubitMismatch {
                    expected: self.num_qubits,
                    actual: pauli.num_qubits,
                });
            }
        }

        let dim = 1usize << self.num_qubits;
        let mut matrix = Array2::from_elem((dim, dim), Complex64::new(0.0, 0.0));
        for (pauli, coefficient) in &self.terms {
            let term = pauli.to_matrix().mapv(|value| value * *coefficient);
            matrix += &term;
        }
        Ok(matrix)
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
        if self.terms.is_empty() {
            return;
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
                } else {
                    // New operator found, save the previous one if its coefficient is significant
                    // Filter out terms with a coefficient near zero
                    if current_coeff.norm() > 1e-10 {
                        merged.push((current_op, current_coeff));
                    }
                    current_op = op;
                    current_coeff = coeff;
                }
            }
            // Push the last accumulated term
            if current_coeff.norm() > 1e-10 {
                merged.push((current_op, current_coeff));
            }
        }

        self.terms = merged;
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
