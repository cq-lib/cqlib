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

//! Fermionic algebra, qubit mappings and occupation encodings.
//!
//! Modes are numbered from zero. Ordered products act right-to-left. The Fock
//! basis is `|n_{N-1} ... n_0> = (a†_0)^n_0 ... (a†_{N-1})^n_{N-1} |vac>`.
//! Qubit zero is the least-significant bit; state indices are `sum(n_p * 2^p)`.
//! Core does not assign spin or chemical-orbital meanings to these indices.
//! Zero and identity operators retain their declared mode count, including zero
//! modes. Circuit consumers can have stricter requirements: basis encoding needs
//! nonempty bits, and `PauliEvolutionAnsatz` needs a nonzero Hermitian operator.
//! Numeric evolution rejects empty input sums but returns identity evolution
//! when a nonempty sum cancels to zero.
//!
//! # Particle-number observable
//! ```
//! use cqlib_core::qis::{FermionicOp, FermionToQubitMapper, JordanWignerMapper,
//!     Observable, Statevector};
//! let mapper = JordanWignerMapper::new();
//! let number = mapper.map_operator(&FermionicOp::total_number(3))?;
//! let mut state = Statevector::new(3);
//! state.apply_x(0);
//! state.apply_x(2);
//! assert_eq!(number.expectation_statevector(&state)?, 2.0);
//! # Ok::<(), cqlib_core::qis::QisError>(())
//! ```
//!
//! # Parameterized excitation evolution
//! ```
//! use cqlib_core::qis::{FermionicOp,
//!     FermionToQubitMapper, JordanWignerMapper};
//! use cqlib_core::circuit::ansatz::{Ansatz, PauliEvolutionAnsatz};
//! let h = FermionicOp::excitation_generator(4, &[0, 1], &[2, 3])?;
//! // e^(-i H theta) = e^(theta (T - T†)).
//! let mapped = JordanWignerMapper::new().map_operator(&h)?;
//! let circuit = PauliEvolutionAnsatz::new(mapped)?.build_circuit("excitation")?;
//! assert_eq!(circuit.num_qubits(), 4);
//! # Ok::<(), cqlib_core::qis::QisError>(())
//! ```
//!
//! Each excitation can have an independent parameter: build one evolution
//! block per generator with a distinct `with_time_param_name`, then append
//! the blocks using [`crate::circuit::Circuit::compose`]. A single ansatz built
//! from a sum of generators instead has one shared time parameter.
//!
//! # Occupation-state encoding
//! ```
//! use cqlib_core::qis::{JordanWignerMapper, OccupationStateMapper, Statevector};
//! use cqlib_core::circuit::ansatz::{Ansatz, BasisEncoding};
//! let bits = JordanWignerMapper::new().map_occupation(&[1, 0, 0])?;
//! let circuit = BasisEncoding::new(bits).build_circuit("reference")?;
//! let state = Statevector::from_circuit(&circuit)?;
//! assert_eq!(state.probabilities()[1], 1.0); // displayed as |001>
//! # Ok::<(), cqlib_core::qis::QisError>(())
//! ```

pub mod mapping;
pub mod operator;

pub use mapping::{FermionToQubitMapper, JordanWignerMapper, OccupationStateMapper};
pub use operator::{FermionicLadder, FermionicOp};

#[cfg(test)]
mod tests;
