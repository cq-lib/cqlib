// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Public compiler entry point.
//!
//! The compiler entry point runs the configured [`CompilerWorkflow`]
//! and returns the optimized circuit plus step-level diagnostics. The workflow
//! starts from a logical circuit, applies canonicalization, definition
//! expansion, knowledge-based rewrite, unitary and multi-controlled-gate
//! decomposition, optional device layout/routing, and target lowering.
//!
//! [`CompileTarget`] makes the target contract explicit: compilation is either
//! logical, lowered to a requested basis, or routed and legalized for one
//! concrete device. Device compilation uses the device topology for capacity
//! checks and SABRE routing, then checks every emitted standard operation
//! against the device's ordered native capabilities before final validation.
//!
//! [`CompileMode::Normal`] selects conservative production defaults.
//! [`CompileMode::Enhanced`] keeps the same semantic contract but spends more
//! rewrite, resynthesis, native-optimization, and routing effort, runs additional
//! cleanup around routing and target lowering, and, for a strict device target,
//! selects among fully lowered and validated routes with a bounded SABRE Pareto
//! beam.

use super::workflow::CompilerWorkflow;
use crate::circuit::{Circuit, Instruction};
use crate::compile::resource::ResourcePolicy;
use crate::compile::transform::VirtualPermutation;
use crate::compile::{CompilerError, WorkflowStepReport};
use crate::device::{Device, Layout};

/// Optimization effort selected for the compiler workflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CompileMode {
    /// Conservative logical optimization using production pass defaults.
    #[default]
    Normal,
    /// A richer staged workflow with stronger pass budgets, target-aware
    /// cleanup, and validated SABRE Pareto route selection for strict device
    /// targets.
    Enhanced,
}

/// User-facing configuration for [`compile`].
///
/// The configuration describes logical optimization effort, optional target
/// constraints, and the ancillary-resource permissions available before layout.
#[derive(Debug, Clone)]
pub struct CompileConfig {
    /// Optimization workflow mode.
    pub mode: CompileMode,
    /// Mutually exclusive logical, basis, or physical-device target.
    pub target: CompileTarget,
    /// Ancillary-resource permission for pre-layout decomposition passes.
    ///
    /// This controls whether logical clean ancillas may be allocated or dirty
    /// input qubits may be borrowed. A device target derives hard capacity
    /// from its usable physical qubits rather than this policy.
    pub resource_policy: ResourcePolicy,
}

/// Target contract selected for a compilation run.
///
/// Device targets intentionally stay inline so the public configuration API
/// does not require allocation solely to select a target. Compile
/// configurations are coarse workflow objects, not per-operation data.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum CompileTarget {
    /// Compile in logical qubit space without target-specific lowering.
    Logical,
    /// Lower to an explicit standard-gate basis.
    Basis(Vec<Instruction>),
    /// Route and lower for one concrete target device.
    Device(DeviceCompileTarget),
    /// Route on a device topology while lowering to an explicit gate basis.
    ///
    /// This target uses the device for capacity, layout, and
    /// routing only. It does not require the output basis to match the
    /// device's native capabilities and therefore does not perform exact
    /// device-native lowering or final device validation.
    TopologyBasis {
        /// Device-specific layout and routing inputs.
        device_target: DeviceCompileTarget,
        /// Explicit standard-gate basis required at the output.
        basis: Vec<Instruction>,
    },
}

/// Device-specific compilation inputs.
#[derive(Debug, Clone)]
pub struct DeviceCompileTarget {
    /// Device whose ordered native capabilities constrain the output.
    pub device: Device,
    /// Optional caller-supplied initial logical-to-physical layout.
    pub initial_layout: Option<Layout>,
    /// Optional deterministic seed for device layout and routing heuristics.
    pub seed: Option<u32>,
}

/// Physical-layout information produced by device compilation.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceCompilationMetadata {
    /// Logical-to-physical layout before routing begins.
    pub initial_layout: Layout,
    /// Original logical-output-to-physical layout after all routed swaps and
    /// pre-layout virtual permutations.
    pub final_layout: Layout,
    /// Original logical output to rewritten logical output mapping accumulated
    /// before physical layout. The initial circuit inputs remain identity-mapped.
    pub virtual_permutation: VirtualPermutation,
}

/// Result returned by [`compile`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompileResult {
    /// Optimized circuit.
    pub circuit: Circuit,
    /// Whether any workflow step changed the input representation.
    pub changed: bool,
    /// Workflow mode used for this run.
    pub mode: CompileMode,
    /// Step-level report for the retained output path, in workflow order.
    ///
    /// Enhanced strict-device compilation may evaluate other route branches.
    /// Discarded branches are summarized by `select.sabre_pareto_beam` rather
    /// than retained as complete step sequences here.
    pub steps: Vec<WorkflowStepReport>,
    /// Physical-layout data when compilation used device-topology routing.
    pub device_metadata: Option<DeviceCompilationMetadata>,
}

impl CompileResult {
    /// Returns the first workflow report with the requested step name.
    pub fn step(&self, name: &str) -> Option<&WorkflowStepReport> {
        self.steps.iter().find(|step| step.name == name)
    }

    /// Returns whether any report with the requested name changed the circuit.
    ///
    /// Skipped reports are not considered changes.
    pub fn step_changed(&self, name: &str) -> bool {
        self.steps
            .iter()
            .any(|step| step.name == name && step.changed && !step.skipped)
    }
}

/// Runs the configured compiler workflow over `circuit`.
///
/// The returned result records the optimized circuit and step-level reports for
/// the retained output path in workflow order. A device target additionally
/// returns its initial and final layouts. Errors are reported when a configured
/// target, native realization, or candidate validation cannot be satisfied.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::circuit::{Circuit, Qubit};
/// use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
/// use cqlib_core::compile::resource::ResourcePolicy;
///
/// let mut circuit = Circuit::new(2);
/// circuit.h(Qubit::new(0)).unwrap();
/// circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
///
/// let result = compile(
///     &circuit,
///     CompileConfig {
///         mode: CompileMode::Normal,
///         target: CompileTarget::Logical,
///         resource_policy: ResourcePolicy::default(),
///     },
/// )
/// .unwrap();
///
/// assert_eq!(result.mode, CompileMode::Normal);
/// assert!(!result.steps.is_empty());
/// assert_eq!(result.circuit.qubits().len(), 2);
/// ```
pub fn compile(circuit: &Circuit, config: CompileConfig) -> Result<CompileResult, CompilerError> {
    CompilerWorkflow::try_new(config)?.run(circuit)
}

/// Runs the configured compiler workflow by consuming `circuit`.
///
/// This is equivalent to [`compile`] but avoids cloning the complete input
/// circuit when the caller no longer needs it.
pub fn compile_owned(
    circuit: Circuit,
    config: CompileConfig,
) -> Result<CompileResult, CompilerError> {
    CompilerWorkflow::try_new(config)?.run_owned(circuit)
}

#[cfg(test)]
#[path = "./compile_test.rs"]
mod compile_test;
