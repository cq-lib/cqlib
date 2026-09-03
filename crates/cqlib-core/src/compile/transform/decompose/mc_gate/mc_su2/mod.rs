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

//! Multi-controlled single-qubit rotation synthesis primitives.
//!
//! This module implements explicit algorithm choices for rotations whose
//! single-qubit target gate is `RX`, `RY`, or `RZ`. Callers provide flattened
//! controls and select either an ancillary-qubit-free decomposition or a
//! clean-accumulator decomposition. Direct callers are responsible for
//! providing controls and ancillary qubits that satisfy the selected
//! algorithm's contract.
//!
//! Normal compiler execution should use the circuit-level
//! [`decompose_mc_gates`](super::decompose_mc_gates) entry point or the
//! [`DecomposeMcGates`](super::DecomposeMcGates) transformer. That layer owns
//! resource planning, algorithm selection, control-flow traversal, and circuit
//! rebuild.
//!
//! The ancillary-qubit-free implementation is an axis-rotation specialization
//! of the linear multi-controlled SU(2) construction from Vale et al.,
//! *Circuit Decomposition of Multicontrolled Special Unitary Single-Qubit
//! Gates*, IEEE Trans. Quantum Eng. 5 (2024),
//! [arXiv:2302.06377](https://arxiv.org/abs/2302.06377).
//!
//! # Examples
//!
//! Decompose a multi-controlled `RY` rotation without ancillary qubits:
//!
//! ```
//! use cqlib_core::circuit::{ParameterValue, Qubit};
//! use cqlib_core::compile::transform::decompose::mc_gate::mc_su2::{
//!     Su2RotationAxis, decompose_mc_su2_no_aux,
//! };
//!
//! let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
//! let target = Qubit::new(3);
//! let operations = decompose_mc_su2_no_aux(
//!     Su2RotationAxis::Y,
//!     &ParameterValue::Fixed(0.5),
//!     &controls,
//!     target,
//! )?;
//!
//! assert!(!operations.is_empty());
//! # Ok::<(), cqlib_core::compile::error::CompilerError>(())
//! ```
//!
//! Decompose a three-control `RZ` rotation with one clean accumulator and one
//! clean MCX workspace qubit. All consumed ancillary qubits must enter in
//! `|0>` and are restored to `|0>`:
//!
//! ```
//! use cqlib_core::circuit::{ParameterValue, Qubit};
//! use cqlib_core::compile::transform::decompose::mc_gate::mc_su2::{
//!     Su2RotationAxis, decompose_mc_su2_n_clean,
//! };
//!
//! let controls = [Qubit::new(0), Qubit::new(1), Qubit::new(2)];
//! let target = Qubit::new(3);
//! let clean_ancillas = [Qubit::new(4), Qubit::new(5)];
//! let operations = decompose_mc_su2_n_clean(
//!     Su2RotationAxis::Z,
//!     &ParameterValue::Fixed(0.5),
//!     &controls,
//!     target,
//!     &clean_ancillas,
//! )?;
//!
//! assert!(!operations.is_empty());
//! # Ok::<(), cqlib_core::compile::error::CompilerError>(())
//! ```

use crate::circuit::StandardGate;

mod clean_accumulator;
mod no_auxiliary;
mod utils;

#[cfg(test)]
mod clean_accumulator_test;
#[cfg(test)]
mod no_auxiliary_test;

pub use clean_accumulator::decompose_mc_su2_n_clean;
pub use no_auxiliary::decompose_mc_su2_no_aux;

pub(super) const DECOMPOSE_MC_SU2_NAME: &str = "decompose.mc_su2";

/// Axis of a single-qubit special-unitary rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Su2RotationAxis {
    /// Rotation around the Pauli-X axis.
    X,
    /// Rotation around the Pauli-Y axis.
    Y,
    /// Rotation around the Pauli-Z axis.
    Z,
}

impl Su2RotationAxis {
    /// Returns the single-qubit rotation gate for this axis.
    pub fn rotation_gate(self) -> StandardGate {
        match self {
            Self::X => StandardGate::RX,
            Self::Y => StandardGate::RY,
            Self::Z => StandardGate::RZ,
        }
    }

    /// Returns the controlled rotation gate for this axis.
    pub fn controlled_rotation_gate(self) -> StandardGate {
        match self {
            Self::X => StandardGate::CRX,
            Self::Y => StandardGate::CRY,
            Self::Z => StandardGate::CRZ,
        }
    }
}
