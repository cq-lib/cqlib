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

//! Compiler pipeline for lowering and optimizing quantum circuits.
//!
//! This module is the public entry point for cqlib circuit compilation. It
//! takes a logical [`Circuit`](crate::circuit::Circuit), applies a deterministic
//! staged lowering workflow, and returns a rebuilt circuit together with
//! step-level diagnostics.
//!
//! The compiler workflow is intentionally a staged lowering pipeline rather
//! than a dynamic pass manager. High-level circuit representations are lowered
//! through stable compiler layers:
//!
//! ```text
//! logical circuit
//!   -> canonicalized logical IR
//!   -> expanded circuit-backed definitions
//!   -> decomposed unitary and multi-controlled gates
//!   -> global commutation-set cancellation
//!   -> knowledge-rule optimization
//!   -> optional physical layout and SABRE routing
//!   -> optional target-basis translation
//!   -> canonicalized output
//!   -> exact device instruction lowering
//!   -> native-input canonicalization and fixed-point optimization
//!   -> final device validation
//! ```
//!
//! # Public Entry Points
//!
//! - [`compile`] is the recommended user-facing API.
//! - [`CompileConfig`] describes optimization effort, one explicit
//!   [`CompileTarget`], and resource policy.
//! - [`CompileResult`] returns the compiled circuit and the workflow step
//!   report.
//! - [`CompilerWorkflow`] is useful when callers want to construct and inspect
//!   a workflow explicitly.
//!
//! Lower-level modules such as [`transform`], [`sabre`], [`knowledge`], and
//! [`commutation`] expose reusable compiler infrastructure. They are intended
//! for advanced users and internal composition; ordinary compilation should
//! start with [`compile`].
//!
//! # Target Constraints
//!
//! [`CompileTarget::Logical`] keeps logical operations,
//! [`CompileTarget::Basis`] lowers to an explicit basis, and
//! [`CompileTarget::Device`] routes and lowers for one device. The
//! [`CompileTarget::TopologyBasis`] target routes on a device topology and
//! lowers to a non-empty caller-supplied basis without claiming native-device
//! compatibility. Strict device compilation
//! uses an exact-plan-aware SABRE
//! movement graph and checks local
//! unary and ordered two-qubit capabilities during routing. Its following
//! device-lowering stage emits the shared exact-qargs plans, then
//! [`Device::validate_circuit`](crate::device::Device::validate_circuit)
//! validates the completed output.
//! A successful device compilation therefore never returns a circuit rejected
//! by the configured device.
//!
//! [`DeviceCompileTarget::initial_layout`] may skip automatic initial-layout
//! selection. [`DeviceCompileTarget::seed`] controls only device layout and
//! routing heuristics. [`CompileResult::device_metadata`] records the initial
//! and final layouts whenever compilation performs device-topology routing.
//!
//! # Classical Control and High-Level Operations
//!
//! The workflow preserves classical-control structure. Transforms that support
//! control-flow bodies recurse into them and report whether they changed the IR
//! through [`TransformOutcome`](transform::TransformOutcome). The workflow does
//! not pre-scan control-flow trees to decide whether a transform should run.
//! This module does not currently lower dynamic classical control into a
//! hardware runtime instruction format.
//!
//! # Step Reports
//!
//! [`WorkflowStepReport`] records the workflow-local stage and step name, plus
//! whether the step changed the circuit, was skipped, or emitted a short
//! reason. Step names describe workflow positions such as `route.sabre` or
//! `translate.target_basis`; they are not required to equal
//! [`Transformer::name`](transform::Transformer::name).
//!
//! # Examples
//!
//! Compile a logical circuit with default logical optimization:
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
//! use cqlib_core::compile::resource::ResourcePolicy;
//!
//! let mut circuit = Circuit::new(2);
//! circuit.h(Qubit::new(0)).unwrap();
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//!
//! let result = compile(
//!     &circuit,
//!     CompileConfig {
//!         mode: CompileMode::Normal,
//!         target: CompileTarget::Logical,
//!         resource_policy: ResourcePolicy::default(),
//!     },
//! )
//! .unwrap();
//!
//! assert_eq!(result.mode, CompileMode::Normal);
//! assert!(!result.steps.is_empty());
//! ```
//!
//! Compile to an explicit target basis:
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Instruction, Qubit, StandardGate};
//! use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
//! use cqlib_core::compile::resource::ResourcePolicy;
//!
//! let mut circuit = Circuit::new(2);
//! circuit.h(Qubit::new(0)).unwrap();
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//!
//! let result = compile(
//!     &circuit,
//!     CompileConfig {
//!         mode: CompileMode::Normal,
//!         target: CompileTarget::Basis(vec![
//!             Instruction::Standard(StandardGate::H),
//!             Instruction::Standard(StandardGate::CZ),
//!         ]),
//!         resource_policy: ResourcePolicy::default(),
//!     },
//! )
//! .unwrap();
//!
//! assert!(
//!     result
//!         .steps
//!         .iter()
//!         .any(|step| step.name == "translate.target_basis" && step.changed)
//! );
//! ```
//!
//! Compile for a device topology:
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, DeviceCompileTarget, compile};
//! use cqlib_core::compile::resource::ResourcePolicy;
//! use cqlib_core::device::Device;
//! use cqlib_core::circuit::{Instruction, StandardGate};
//!
//! let mut circuit = Circuit::new(2);
//! circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
//! let device = Device::bidirectional_line("line-2", 2)
//!     .unwrap()
//!     .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
//!     .unwrap();
//!
//! let result = compile(
//!     &circuit,
//!     CompileConfig {
//!         mode: CompileMode::Normal,
//!         target: CompileTarget::Device(DeviceCompileTarget {
//!             device,
//!             initial_layout: None,
//!             seed: Some(7),
//!         }),
//!         resource_policy: ResourcePolicy::default(),
//!     },
//! )
//! .unwrap();
//!
//! assert!(
//!     result
//!         .steps
//!         .iter()
//!         .any(|step| step.name == "route.sabre" && !step.skipped)
//! );
//! ```
//!
//! Route from a supplied initial layout:
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, DeviceCompileTarget, compile};
//! use cqlib_core::compile::resource::ResourcePolicy;
//! use cqlib_core::device::{Device, Layout};
//!
//! let mut circuit = Circuit::new(1);
//! circuit.h(Qubit::new(0)).unwrap();
//! let layout = Layout::from_pairs(&[(0, 2)], 3).unwrap();
//!
//! let result = compile(
//!     &circuit,
//!     CompileConfig {
//!         mode: CompileMode::Normal,
//!         target: CompileTarget::Device(DeviceCompileTarget {
//!             device: Device::line("line-3", 3)
//!                 .unwrap()
//!                 .with_native_gates(vec![
//!                     cqlib_core::circuit::Instruction::Standard(
//!                         cqlib_core::circuit::StandardGate::H,
//!                     ),
//!                 ])
//!                 .unwrap(),
//!             initial_layout: Some(layout),
//!             seed: Some(11),
//!         }),
//!         resource_policy: ResourcePolicy::default(),
//!     },
//! )
//! .unwrap();
//!
//! assert_eq!(result.circuit.operations()[0].qubits.as_slice(), &[Qubit::new(2)]);
//! ```
//!
//! Inspect workflow step reports:
//!
//! ```rust
//! use cqlib_core::circuit::Circuit;
//! use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
//! use cqlib_core::compile::resource::ResourcePolicy;
//!
//! let result = compile(
//!     &Circuit::new(1),
//!     CompileConfig {
//!         mode: CompileMode::Enhanced,
//!         target: CompileTarget::Logical,
//!         resource_policy: ResourcePolicy::default(),
//!     },
//! )
//! .unwrap();
//!
//! let routing = result
//!     .steps
//!     .iter()
//!     .find(|step| step.name == "route.sabre")
//!     .unwrap();
//! assert!(routing.skipped);
//! ```

pub mod commutation;
pub mod compiler;
pub(crate) mod device_planning;
pub mod error;
pub mod knowledge;
pub mod physical_target;
pub mod resource;
pub mod sabre;
pub mod transform;
pub mod workflow;

#[cfg(test)]
mod test_utils;

/// Tolerance for proving equality between compiler parameter expressions.
pub(crate) const PARAMETER_EQ_TOLERANCE: f64 = 1e-12;

pub(crate) fn compare_some_first_by<T>(
    left: Option<T>,
    right: Option<T>,
    compare: impl FnOnce(T, T) -> std::cmp::Ordering,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Tolerance for treating a scalar as numerically zero.
pub(crate) const NUMERIC_ZERO_TOLERANCE: f64 = 1e-14;

/// Tolerance for checking whether a candidate phase ratio has unit norm.
pub(crate) const UNIT_PHASE_NORM_TOLERANCE: f64 = 1e-8;

pub use commutation::{
    Commutation, CommutationChecker, CommutationConfig, CommutationResult, algebraic_commutation,
    check_commutation,
};
pub use compiler::{
    CompileConfig, CompileMode, CompileResult, CompileTarget, DeviceCompilationMetadata,
    DeviceCompileTarget, compile,
};
pub use error::{CompilerError, SabreRoutingFailure};
pub use sabre::{
    SabreConfig, SabreHeuristicConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    SabreVf2PrepassConfig, normalize_initial_layout, sabre_route, validate_reachable_interactions,
};
pub use transform::{KnowledgeRewriteDiagnostics, VirtualPermutation};
pub use workflow::{CompilerWorkflow, WorkflowStepReport};
