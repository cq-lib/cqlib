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

//! Circuit decomposition transforms and synthesis building blocks.
//!
//! This module separates decomposition by the representation that is available
//! for an operation:
//!
//! - [`definition`] expands gates that already carry an implementation circuit.
//!   Use this first for [`CircuitGate`](crate::circuit::gate::CircuitGate) and
//!   circuit-backed [`UnitaryGate`](crate::circuit::UnitaryGate) operations.
//! - [`unitary`] synthesizes remaining custom `UnitaryGate` operations from
//!   fixed numeric matrices. It currently supports one- and two-qubit matrices.
//! - [`mc_gate`] provides explicit algorithmic synthesis primitives for
//!   multi-controlled gates, plus the circuit-level resource-aware
//!   `decompose_mc_gates` entry point.
//!
//! The workflow-facing adapters [`DecomposeDefinitions`],
//! [`DecomposeUnitaries`], and [`DecomposeMcGates`] implement
//! [`Transformer`](crate::compile::transform::Transformer). They are expected
//! to report `changed = false` when no operation in their scope was lowered.
//!
//! # Recommended Order
//!
//! Run definition expansion before matrix synthesis so circuit-backed unitary
//! gates are expanded before the matrix-only [`decompose_unitaries`] stage is
//! reached. Run multi-controlled gate decomposition after unitary synthesis so
//! the circuit-level planner can apply resource policy and control-flow
//! traversal to the remaining high-level controlled operations.
//!
//! These are decomposition entry points, not a complete compiler pipeline.
//! Target-basis lowering belongs to [`rewrite`](crate::compile::transform::rewrite),
//! initial placement belongs to [`layout`](crate::compile::transform::layout),
//! and SWAP insertion belongs to [`routing`](crate::compile::transform::routing).
//! Directed-coupling legalization and scheduling are separate compiler
//! concerns.

pub mod definition;
pub mod mc_gate;
pub mod rule;
pub mod unitary;

pub use definition::{DecomposeDefinitions, expand_definitions};
pub use mc_gate::{
    DecomposeMcGates, McGateDecomposeConfig, decompose_mc_gates, decompose_mc_gates_with_rule_stats,
};
pub use rule::{
    DecompositionAlgorithm, DecompositionRule, DecompositionRuleCache, DecompositionRuleStats,
    McGateRuleRequest, NumericUnitaryRuleRequest, NumericUnitarySynthesisKey, ResourceSignature,
    RuntimeAncillaKind,
};
pub use unitary::decompose::{
    DecomposeUnitaries, UnitaryDecomposeConfig, decompose_unitaries,
    decompose_unitaries_with_rule_stats,
};
