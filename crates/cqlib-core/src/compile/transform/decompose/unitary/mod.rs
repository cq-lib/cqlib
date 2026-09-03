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

//! Numeric synthesis for matrix-backed custom unitary gates.
//!
//! This module lowers custom [`UnitaryGate`](crate::circuit::UnitaryGate)
//! operations that have a concrete numeric matrix representation. It is the
//! matrix-synthesis branch of [`decompose`](super), separate from
//! circuit-backed definition expansion and multi-controlled gate lowering.
//!
//! # Supported inputs
//!
//! The circuit-facing pass supports fixed numeric one- and two-qubit unitary
//! matrices. A parameterized matrix is supported only after every argument at
//! the call site resolves to a finite numeric value. Three-qubit and larger
//! matrices are rejected explicitly.
//!
//! This module does not expand circuit-backed definitions or adapt gates to
//! hardware topology. It also does not perform target-basis lowering itself;
//! the crate-internal `euler_1q` module only provides the pure numeric
//! synthesis candidates that the target-basis lowerer consumes for
//! fixed-parameter `U` gates. Run
//! [`expand_definitions`](super::definition::expand_definitions) before matrix
//! synthesis when a circuit may contain unitary gates backed by subcircuits.
//!
//! # Internal layers
//!
//! - [`decompose`] owns the circuit traversal entry point, parameter-table
//!   rebuilding, control-flow handling, and global-phase propagation.
//! - [`unitary_1q`] decomposes a concrete 2x2 matrix into a local `U` gate and a
//!   scalar global phase.
//! - `euler_1q` re-synthesizes one numeric `U` decomposition into
//!   parameter-aware half-rotation candidate sequences (RZ/X2P/X2M/X
//!   families), including degenerate-angle short forms and exact global-phase
//!   bookkeeping.
//! - [`unitary_2q`] converts a concrete 4x4 matrix into local `U` gates plus a
//!   selectable two-qubit interaction basis.
//! - [`two_qubit_kak`] owns the circuit-agnostic KAK / Weyl numerical
//!   primitive used by the two-qubit emitter.

pub mod decompose;
pub(crate) mod device_synthesis;
pub(crate) mod euler_1q;
mod matrix;
pub mod two_qubit_kak;
pub mod unitary_1q;
pub mod unitary_2q;

#[cfg(test)]
mod unitary_2q_test;

pub use decompose::{
    DecomposeUnitaries, UnitaryDecomposeConfig, decompose_unitaries,
    decompose_unitaries_with_rule_stats,
};
pub use two_qubit_kak::{KakDecomposition, kak_decompose};
pub use unitary_1q::{OneQubitUnitaryDecomposition, synthesize_numeric_1q_unitary};
pub use unitary_2q::{
    TargetAwareSynthesisCost, TwoQubitSynthesisCandidate, TwoQubitSynthesisRequest,
    TwoQubitSynthesisTarget, TwoQubitUnitaryDecomposeBasis, TwoQubitUnitarySynthesisResult,
    plan_numeric_2q_unitary, synthesize_numeric_2q_unitary, target_aware_cost_of_value_operations,
    validate_numeric_2q_candidate,
};

pub(crate) use device_synthesis::{
    DeviceContextCostFailure, DevicePhysicalCost, DevicePreLayoutEvaluation,
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext,
};
