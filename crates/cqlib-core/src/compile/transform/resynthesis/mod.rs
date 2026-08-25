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

//! Numeric block resynthesis transforms.
//!
//! This module optimizes already-lowered standard-gate circuits by collecting
//! fixed-parameter one- and two-qubit operations over the same qubit pair,
//! rebuilding their exact 4x4 unitary, and resynthesizing that unitary with the
//! configured two-qubit basis. Maximal uninterrupted pair runs are collected
//! in one linear pass without a size budget. A bounded dependency collector
//! then supplements them with candidates that require legal commutation
//! crossing. A candidate is accepted only when its local cost is strictly lower
//! and every crossed operation still commutes with the synthesized replacement.
//!
//! The pass is intentionally conservative:
//!
//! - symbolic parameters, non-standard gates, measurements, resets, delays,
//!   labels, directives, and control-flow instructions are collection
//!   boundaries;
//! - supplemental commutation-aware collection is bounded by `max_scan_span`,
//!   `max_block_ops`, and `max_crossed_ops`;
//! - control-flow bodies are optimized recursively, with body-local synthesis
//!   phase emitted as a leading `GPhase`.
//!
//! # Example
//!
//! ```
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::circuit::{Instruction, StandardGate};
//! use cqlib_core::compile::transform::{
//!     ResynthesizeTwoQubitBlocks, TransformOutcome, TwoQubitBlockResynthesisConfig, Transformer,
//! };
//! use cqlib_core::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
//!
//! let q0 = Qubit::new(0);
//! let q1 = Qubit::new(1);
//! let mut circuit = Circuit::new(2);
//! circuit.cx(q0, q1)?;
//! circuit.cx(q0, q1)?;
//!
//! let target = TwoQubitSynthesisTarget::from_instructions(Some(&[
//!     Instruction::Standard(StandardGate::U),
//!     Instruction::Standard(StandardGate::CX),
//! ]))?;
//! let config = TwoQubitBlockResynthesisConfig::normal(target);
//! let result = ResynthesizeTwoQubitBlocks::new(config).transform(&circuit, None)?;
//! let TransformOutcome::Changed(resynthesized) = result else {
//!     panic!("the cancelling block should be resynthesized");
//! };
//! assert!(resynthesized.operations().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod collector;
mod commutation;
mod config;
mod cost;
mod dag_collector;
mod incremental;
mod maximal_run;
mod resynthesizer;
mod selector;
mod synthesis_cache;

pub use config::TwoQubitBlockResynthesisConfig;
pub(crate) use incremental::{
    NativeResynthesisPolicy, NativeResynthesisSession, NativeWorksetStats,
};
pub use resynthesizer::{ResynthesizeTwoQubitBlocks, resynthesize_two_qubit_blocks};
pub(crate) use resynthesizer::{
    WorkflowResynthesisSession, resynthesize_two_qubit_blocks_incremental,
    resynthesize_two_qubit_blocks_workflow,
};
