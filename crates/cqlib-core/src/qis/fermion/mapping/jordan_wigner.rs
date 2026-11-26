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

//! Jordan–Wigner mapping using sparse Pauli arithmetic.

use super::{FermionToQubitMapper, OccupationStateMapper};
use crate::qis::fermion::{FermionicLadder, FermionicOp};
use crate::qis::{Hamiltonian, Pauli, PauliString, QisError};
use num_complex::Complex64;
use std::collections::BTreeMap;

/// Jordan–Wigner encoding with mode p mapped to qubit p.
///
/// The Z chain covers modes `0..p`: creation is `Z_chain (X_p - iY_p)/2`,
/// annihilation is `Z_chain (X_p + iY_p)/2`. Qubit zero is the least-significant
/// basis bit, and the rightmost character of a displayed Pauli string.
/// Mapping never constructs a dense matrix or implicitly normal orders its input.
/// A product of k ladders can expand to 2^k terms before sparse merging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JordanWignerMapper;

impl JordanWignerMapper {
    /// Creates a stateless mapper.
    pub fn new() -> Self {
        Self
    }
}

impl FermionToQubitMapper for JordanWignerMapper {
    fn num_qubits(&self, num_modes: usize) -> usize {
        num_modes
    }

    fn map_operator(&self, op: &FermionicOp) -> Result<Hamiltonian, QisError> {
        let n = self.num_qubits(op.num_modes());
        let mut result = Hamiltonian::new(n);
        // Build only the components needed by this input, and reuse them across terms.
        let mut components = BTreeMap::new();
        for (term, coefficient) in op.terms() {
            let mut product = Hamiltonian::new(n);
            // Merge the unit-weight product before applying the input coefficient,
            // so representable small results are not lost in ladder intermediates.
            product.add_term(PauliString::new(n), Complex64::new(1.0, 0.0))?;
            for ladder in term {
                let p = ladder.mode();
                let (x, y) = components.entry(p).or_insert_with(|| {
                    let mut x = PauliString::new(n);
                    let mut y = PauliString::new(n);
                    for j in 0..p {
                        x.set_pauli(j, Pauli::Z);
                        y.set_pauli(j, Pauli::Z);
                    }
                    x.set_pauli(p, Pauli::X);
                    y.set_pauli(p, Pauli::Y);
                    (x, y)
                });
                let imaginary = match ladder {
                    FermionicLadder::Create(_) => -0.5,
                    FermionicLadder::Annihilate(_) => 0.5,
                };
                let mut next = Hamiltonian::new(n);
                for (pauli, c) in &product.terms {
                    next.add_term(pauli * &*x, *c * 0.5)?;
                    next.add_term(pauli * &*y, *c * Complex64::new(0.0, imaginary))?;
                }
                product = next.normalized()?;
                if product.terms.is_empty() {
                    break;
                }
            }
            result.terms.extend(
                product
                    .terms
                    .into_iter()
                    .map(|(pauli, c)| (pauli, c * coefficient)),
            );
        }
        result.normalized()
    }
}

impl OccupationStateMapper for JordanWignerMapper {
    fn map_occupation(&self, occupation: &[u8]) -> Result<Vec<bool>, QisError> {
        occupation
            .iter()
            .enumerate()
            .map(|(index, value)| match value {
                0 => Ok(false),
                1 => Ok(true),
                value => Err(QisError::InvalidOccupation {
                    index,
                    value: *value,
                }),
            })
            .collect()
    }
}
