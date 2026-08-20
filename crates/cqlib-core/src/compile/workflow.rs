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

//! Workflow-level orchestration for the new compiler optimization pipeline.
//!
//! The workflow is a staged composition layer, not an optimization algorithm.
//! It resolves target constraints, runs completed compiler transforms in a
//! deterministic order, and records only the postconditions it can actually
//! verify with the compiler capabilities currently implemented.
//!
//! The normal workflow follows the stable pass order:
//! canonicalize input, expand circuit-backed definitions, apply production
//! knowledge rewrite, decompose unitary and multi-controlled gates,
//! canonicalize again, optimize the decomposed circuit, optionally lower to a
//! routing-compatible basis, optionally route on a device, optionally translate
//! to the resolved target basis, and canonicalize the output representation. A
//! device workflow then lowers every gate to exact ordered native capabilities,
//! closes a bounded native-optimization loop, and validates the completed
//! physical circuit before returning it.
//!
//! The enhanced workflow uses the same required correctness stages but raises
//! rewrite budgets, uses stronger SABRE trial settings, performs a
//! post-routing cleanup pass, and adds a target-aware cleanup pass after
//! target-basis translation. This keeps `Normal` suitable for predictable
//! production compilation while giving `Enhanced` more chances to recover
//! simplifications exposed by decomposition, routing, and lowering.
//!
//! Stages are deliberately ordered around compiler invariants. Early
//! canonicalization gives later passes a stable representation, definition and
//! high-level gate decomposition remove operations that routing cannot accept,
//! routing runs before final target-basis cleanup because it may insert SWAPs,
//! and output canonicalization removes representation noise before exact device
//! lowering. Native optimization is re-legalized and costed on exact physical
//! qargs; device validation remains terminal, with no transform after it.

use crate::circuit::{Circuit, ClassicalControlOp, Instruction, Operation, StandardGate};
use crate::compile::CompilerError;
use crate::compile::resource::ResourceLimits;
use crate::compile::sabre::SabreConfig;
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext, TwoQubitSynthesisTarget,
};
use crate::compile::transform::decompose::{
    DecomposeDefinitions, DecomposeMcGates, DecomposeUnitaries, McGateDecomposeConfig,
    UnitaryDecomposeConfig,
};
use crate::compile::transform::native_optimization::NativeOptimizer;
use crate::compile::transform::{
    Canonicalizer, CircuitAnalysis, CommutativeCancellation, DeviceLowerer, KnowledgeRewriter,
    LayoutObjective, LowerToRoutingBasis, OptimizeOneQubitRuns, ResynthesizeTwoQubitBlocks,
    RewriteConfig, TargetBasisCostModel, TargetBasisLowerer, TransformOutcome, Transformer,
    TwoQubitBlockResynthesisConfig, VirtualPermutation, VirtualPermutationElisionStatus,
    elide_virtual_permutations, route_sabre, route_with_layout,
};
use crate::device::{Device, Topology};
use std::borrow::Cow;
use std::sync::Arc;

use super::{
    CompileConfig, CompileMode, CompileResult, CompileTarget, DeviceCompilationMetadata,
    DeviceCompileTarget,
};

/// Per-step execution record produced by a workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepReport {
    /// Coarse workflow stage, following the staged-pass-manager model.
    pub stage: &'static str,
    /// Workflow-local step name.
    pub name: &'static str,
    /// Whether this step changed the circuit representation.
    pub changed: bool,
    /// Whether the step was intentionally skipped.
    pub skipped: bool,
    /// Optional skip or configuration note.
    pub reason: Option<String>,
}

struct WorkflowState {
    current: Circuit,
    analysis: CircuitAnalysis,
    changed: bool,
    steps: Vec<WorkflowStepReport>,
    prepared_target_basis: Option<PreparedTargetBasis>,
    two_qubit_target: TwoQubitSynthesisTarget,
    virtual_permutation: Option<VirtualPermutation>,
    device_metadata: Option<DeviceCompilationMetadata>,
    one_qubit_optimizer: Option<OptimizeOneQubitRuns>,
    pending_one_qubit_resynthesis: bool,
}

struct PreparedTargetBasis {
    instructions: Arc<[Instruction]>,
    lowerer: Arc<TargetBasisLowerer>,
    cost_model: Arc<TargetBasisCostModel>,
}

impl PreparedTargetBasis {
    fn new(target_basis: Vec<Instruction>) -> Result<Self, CompilerError> {
        let instructions: Arc<[Instruction]> = target_basis.into();
        let lowerer = Arc::new(TargetBasisLowerer::from_shared_basis(Arc::clone(
            &instructions,
        ))?);
        let cost_model = Arc::new(TargetBasisCostModel::from_lowerer(Arc::clone(&lowerer))?);
        Ok(Self {
            instructions,
            lowerer,
            cost_model,
        })
    }
}

impl WorkflowState {
    fn apply_transform(
        &mut self,
        stage: &'static str,
        name: &'static str,
        transform: impl FnOnce(&Circuit, &CircuitAnalysis) -> Result<TransformOutcome, CompilerError>,
    ) -> Result<bool, CompilerError> {
        let changed = match transform(&self.current, &self.analysis)? {
            TransformOutcome::Unchanged => false,
            TransformOutcome::Changed(circuit) => {
                self.analysis = CircuitAnalysis::analyze(&circuit);
                self.current = circuit;
                self.changed = true;
                true
            }
        };
        self.steps.push(WorkflowStepReport {
            stage,
            name,
            changed,
            skipped: false,
            reason: None,
        });
        Ok(changed)
    }

    fn record_skipped(
        &mut self,
        stage: &'static str,
        name: &'static str,
        reason: impl Into<String>,
    ) {
        self.steps.push(WorkflowStepReport {
            stage,
            name,
            changed: false,
            skipped: true,
            reason: Some(reason.into()),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewritePhase {
    PreDecomposition,
    PostDecomposition,
    PostRouting,
    TargetCleanup,
}

/// Compiler optimization workflow built from completed compiler transforms.
pub struct CompilerWorkflow {
    config: CompileConfig,
}

impl CompilerWorkflow {
    /// Creates a compiler workflow from a complete configuration.
    pub const fn new(config: CompileConfig) -> Self {
        Self { config }
    }

    /// Returns the workflow configuration.
    pub const fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// Runs the workflow over `circuit` and returns the rebuilt circuit plus
    /// execution metadata.
    pub fn run(&self, circuit: &Circuit) -> Result<CompileResult, CompilerError> {
        let prepared_target_basis = self.prepare_target_basis()?;
        let one_qubit_optimizer = match &self.config.target {
            CompileTarget::Logical => Some(OptimizeOneQubitRuns::logical()),
            CompileTarget::Basis(_) | CompileTarget::TopologyBasis { .. } => {
                let prepared = prepared_target_basis.as_ref().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "explicit target basis was not prepared".to_string(),
                    )
                })?;
                Some(OptimizeOneQubitRuns::basis_with_cost_model(Arc::clone(
                    &prepared.cost_model,
                )))
            }
            CompileTarget::Device(_) => None,
        };
        let two_qubit_target = prepared_target_basis
            .as_ref()
            .map_or_else(TwoQubitSynthesisTarget::unconstrained, |prepared| {
                TwoQubitSynthesisTarget::from_cost_model(Arc::clone(&prepared.cost_model))
            });
        let mut state = WorkflowState {
            current: circuit.clone(),
            analysis: CircuitAnalysis::analyze(circuit),
            changed: false,
            steps: Vec::new(),
            prepared_target_basis,
            two_qubit_target,
            virtual_permutation: None,
            device_metadata: None,
            one_qubit_optimizer,
            pending_one_qubit_resynthesis: false,
        };

        self.record_pre_init(&mut state);
        self.validate_resources(circuit, &mut state)?;
        self.lower_init(&mut state)?;
        self.lower_decompose(&mut state)?;
        self.lower_optimize(&mut state)?;
        self.lower_routing_basis(&mut state)?;
        self.lower_physical(&mut state)?;
        self.lower_target(&mut state)?;
        self.lower_output(&mut state)?;
        self.lower_device_instructions(&mut state)?;
        self.canonicalize_native_input(&mut state)?;
        self.optimize_native_instructions(&mut state)?;
        self.validate_device(&mut state)?;
        self.validate_topology_target(&mut state)?;

        Ok(CompileResult {
            circuit: state.current,
            changed: state.changed,
            mode: self.config.mode,
            steps: state.steps,
            device_metadata: state.device_metadata,
        })
    }

    /// Establishes a stable high-level IR before gate-specific lowering.
    ///
    /// Definition expansion precedes the first rewrite pass so knowledge rules
    /// see the operations contained by user-defined gates.
    fn lower_init(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        state.apply_transform("init", "canonicalize.input", |circuit, analysis| {
            Canonicalizer::production().transform(circuit, Some(analysis))
        })?;
        self.apply_definition_decomposition(state)?;
        let rewrite_config = self.rewrite_config(RewritePhase::PreDecomposition)?;
        state.apply_transform(
            "optimization",
            "optimize.pre_decomposition",
            |circuit, analysis| {
                KnowledgeRewriter::new(rewrite_config).transform(circuit, Some(analysis))
            },
        )?;
        Ok(())
    }

    /// Lowers opaque unitary and multi-controlled operations.
    ///
    /// Routing and target-basis translation only operate on concrete operation
    /// families, so this stage runs before physical and target lowering.
    fn lower_decompose(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        self.apply_unitary_decomposition(state)?;
        self.apply_mc_gate_decomposition(state)?;
        state.apply_transform(
            "optimization",
            "canonicalize.after_decomposition",
            |circuit, analysis| Canonicalizer::production().transform(circuit, Some(analysis)),
        )?;
        state.apply_transform(
            "optimization",
            "optimize.commutative_cancellation",
            |circuit, analysis| CommutativeCancellation::new().transform(circuit, Some(analysis)),
        )?;
        self.apply_virtual_permutation_elision(state)?;
        self.apply_two_qubit_resynthesis(
            state,
            "optimization",
            "resynthesize.two_qubit_blocks",
            DeviceSynthesisPlacement::PreLayoutEnvelope,
        )?;
        state.pending_one_qubit_resynthesis =
            self.apply_one_qubit_optimization(state, "optimize.one_qubit.post_decomposition")?;
        Ok(())
    }

    fn lower_optimize(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let rewrite_config = self.rewrite_config(RewritePhase::PostDecomposition)?;
        state.apply_transform(
            "optimization",
            "optimize.post_decomposition",
            |circuit, analysis| {
                KnowledgeRewriter::new(rewrite_config).transform(circuit, Some(analysis))
            },
        )?;
        self.close_one_qubit_resynthesis(state)
    }

    /// Applies optional routing-basis lowering before physical routing.
    ///
    /// This stage is intentionally separate from final target-basis
    /// translation. It only guarantees that standard gates unsupported by
    /// SABRE's arity model, such as `CCX`, are lowered to operations with at
    /// most two qubits before layout and routing run.
    fn lower_routing_basis(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        self.apply_routing_basis_decomposition(state)
    }

    /// Applies optional physical lowering from logical to physical qubits.
    fn lower_physical(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        self.apply_layout_and_routing(state)?;
        if self.config.mode == CompileMode::Enhanced {
            self.apply_post_routing_resynthesis(state)?;
            self.apply_post_routing_cleanup(state)?;
        }
        Ok(())
    }

    /// Applies optional target-basis translation and target-aware cleanup.
    ///
    /// This stage runs after routing because routing may insert SWAPs and expose
    /// new target-aware rewrite opportunities.
    fn lower_target(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        self.apply_target_translation(state)?;
        if self.config.mode == CompileMode::Enhanced {
            if let Some(cleanup_config) = self.target_cleanup_config(state)? {
                state.apply_transform(
                    "optimization",
                    "optimize.target_cleanup",
                    |circuit, analysis| {
                        KnowledgeRewriter::new(cleanup_config).transform(circuit, Some(analysis))
                    },
                )?;
            } else {
                state.record_skipped(
                    "optimization",
                    "optimize.target_cleanup",
                    "no explicit target basis configured",
                );
            }
        }
        if matches!(
            self.config.target,
            CompileTarget::Basis(_) | CompileTarget::TopologyBasis { .. }
        ) {
            let changed =
                self.apply_one_qubit_optimization(state, "optimize.one_qubit.post_translation")?;
            if changed {
                self.apply_target_translation_named(
                    state,
                    "translate.target_basis.after_one_qubit",
                )?;
            } else {
                state.record_skipped(
                    "translation",
                    "translate.target_basis.after_one_qubit",
                    "post-translation one-qubit optimization was stable",
                );
            }
            self.validate_explicit_target_basis(state)?;
        } else {
            state.record_skipped(
                "optimization",
                "optimize.one_qubit.post_translation",
                "no explicit basis target configured",
            );
            state.record_skipped(
                "translation",
                "translate.target_basis.after_one_qubit",
                "no explicit basis target configured",
            );
        }
        Ok(())
    }

    fn lower_output(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        state.apply_transform("output", "canonicalize.output", |circuit, analysis| {
            Canonicalizer::production().transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn lower_device_instructions(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let Some(target) = self.strict_device_target() else {
            state.record_skipped(
                "translation",
                "lower.device_instructions",
                self.strict_device_skip_reason(),
            );
            return Ok(());
        };
        let lowerer = DeviceLowerer::new(&target.device);
        state.apply_transform(
            "translation",
            "lower.device_instructions",
            |circuit, analysis| lowerer.transform(circuit, Some(analysis)),
        )?;
        Ok(())
    }

    fn canonicalize_native_input(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        if self.strict_device_target().is_none() {
            state.record_skipped(
                "optimization",
                "canonicalize.native_input",
                self.strict_device_skip_reason(),
            );
            return Ok(());
        }
        // The native optimizer canonicalizes its input on entry (paired with
        // its entry device validation), so a separate workflow pass here would
        // be an immediately duplicated full-circuit pass.
        state.record_skipped(
            "optimization",
            "canonicalize.native_input",
            "native optimizer canonicalizes its input on entry",
        );
        Ok(())
    }

    fn optimize_native_instructions(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let Some(target) = self.strict_device_target() else {
            state.record_skipped(
                "optimization",
                "optimize.native_fixed_point",
                self.strict_device_skip_reason(),
            );
            return Ok(());
        };
        let (max_rounds, max_stale_rounds) = match self.config.mode {
            CompileMode::Normal => (2, 1),
            CompileMode::Enhanced => (8, 3),
        };
        let optimizer = NativeOptimizer::new(
            &target.device,
            self.two_qubit_resynthesis_config_for_state(state),
            max_rounds,
            max_stale_rounds,
        );
        let result = optimizer.run(&state.current)?;
        if result.changed {
            state.analysis = CircuitAnalysis::analyze(&result.circuit);
        }
        state.current = result.circuit;
        state.changed |= result.changed;
        state.steps.push(WorkflowStepReport {
            stage: "optimization",
            name: "optimize.native_fixed_point",
            changed: result.changed,
            skipped: false,
            reason: Some(format!(
                "rounds={}; restored_best={}; native_2q_ops={}->{}; native_2q_depth={}->{}; native_depth={}->{}; native_ops={}->{}; predicted_log_error={:?}->{:?}; unavailable_error_count={}->{}; imputed_error_count={}->{}",
                result.rounds,
                result.restored_best,
                result.before.native_two_qubit_ops,
                result.after.native_two_qubit_ops,
                result.before.native_two_qubit_depth,
                result.after.native_two_qubit_depth,
                result.before.total_native_depth,
                result.after.total_native_depth,
                result.before.native_total_ops,
                result.after.native_total_ops,
                result.before.predicted_log_error,
                result.after.predicted_log_error,
                result.before.unavailable_error_count,
                result.after.unavailable_error_count,
                result.before.imputed_error_count,
                result.after.imputed_error_count,
            )),
        });
        Ok(())
    }

    fn validate_device(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let Some(target) = self.strict_device_target() else {
            state.record_skipped(
                "validation",
                "validate.device",
                self.strict_device_skip_reason(),
            );
            return Ok(());
        };
        target.device.validate_circuit(&state.current)?;
        state.steps.push(WorkflowStepReport {
            stage: "validation",
            name: "validate.device",
            changed: false,
            skipped: false,
            reason: None,
        });
        Ok(())
    }

    /// Validates the final circuit against a topology-basis target.
    ///
    /// Topology-basis compilation routes onto a physical coupling graph but
    /// does not lower to exact ordered device capabilities, so the strict
    /// device validator does not apply. Instead, the loose-topology device
    /// built for routing approximates this contract: `Device::validate_circuit`
    /// is a full device validator, and here it is configured with bidirectional
    /// coupling edges and the explicit basis as native gates, so the check
    /// covers valid qubits, undirected connectivity, and basis membership.
    fn validate_topology_target(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let CompileTarget::TopologyBasis { device_target, .. } = &self.config.target else {
            return Ok(());
        };
        let device = self.routing_device(device_target)?;
        device.validate_circuit(&state.current)?;
        state.steps.push(WorkflowStepReport {
            stage: "validation",
            name: "validate.topology",
            changed: false,
            skipped: false,
            reason: None,
        });
        Ok(())
    }

    /// Builds the rewrite configuration for a workflow phase.
    ///
    /// Rewrite phases use the production optimizer. Enhanced mode only
    /// increases bounded search budgets rather than changing correctness
    /// requirements.
    ///
    /// Compiler invariant: once a circuit has been physically routed, every
    /// two-qubit operation acts on a device coupling edge, so any rewrite
    /// phase that runs after routing must preserve undirected two-qubit
    /// connectivity. New physical phases added to the workflow must opt into
    /// this policy; pre-routing phases and basis-only cleanup (which never
    /// routes) stay unrestricted.
    fn rewrite_config(&self, phase: RewritePhase) -> Result<RewriteConfig, CompilerError> {
        let preserve_connectivity = match phase {
            RewritePhase::PreDecomposition | RewritePhase::PostDecomposition => false,
            RewritePhase::PostRouting => true,
            RewritePhase::TargetCleanup => self.routing_device_target().is_some(),
        };
        let mut config =
            RewriteConfig::production().with_preserve_two_qubit_connectivity(preserve_connectivity);

        if self.config.mode == CompileMode::Enhanced {
            config = config
                .with_max_rounds(16)
                .with_max_window_ops(32)
                .with_max_pattern_len(12);
        }

        Ok(config)
    }

    fn target_cleanup_config(
        &self,
        state: &WorkflowState,
    ) -> Result<Option<RewriteConfig>, CompilerError> {
        let Some(prepared) = state.prepared_target_basis.as_ref() else {
            return Ok(None);
        };

        self.rewrite_config(RewritePhase::TargetCleanup)?
            .with_target_instructions(prepared.instructions.to_vec())
            .map(Some)
    }

    fn apply_definition_decomposition(
        &self,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        state.apply_transform("init", "decompose.definitions", |circuit, analysis| {
            DecomposeDefinitions.transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn apply_unitary_decomposition(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let config = self.unitary_decompose_config_for_state(state);
        let decomposer = if let Some(target) = self
            .strict_device_target()
            .filter(|_| state.analysis.has_unitary_gates)
        {
            let context = DeviceTwoQubitSynthesisContext::build(
                &target.device,
                &state.current,
                DeviceSynthesisPlacement::PreLayoutEnvelope,
            )?;
            DecomposeUnitaries::new_device_aware(config, context)
        } else {
            DecomposeUnitaries::new(config)
        };
        state.apply_transform("translation", "decompose.unitary", |circuit, analysis| {
            decomposer.transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn unitary_decompose_config_for_state(&self, state: &WorkflowState) -> UnitaryDecomposeConfig {
        UnitaryDecomposeConfig {
            two_qubit_target: state.two_qubit_target.clone(),
            ..Default::default()
        }
    }

    fn apply_mc_gate_decomposition(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let config = self.mc_gate_decompose_config();
        state.apply_transform("translation", "decompose.mc_gates", |circuit, analysis| {
            DecomposeMcGates::new(config).transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn apply_two_qubit_resynthesis(
        &self,
        state: &mut WorkflowState,
        stage: &'static str,
        name: &'static str,
        placement: DeviceSynthesisPlacement,
    ) -> Result<bool, CompilerError> {
        let config = self.two_qubit_resynthesis_config_for_state(state);
        let resynthesizer = if let Some(target) = self
            .strict_device_target()
            .filter(|_| ResynthesizeTwoQubitBlocks::is_applicable(&state.current))
        {
            let context =
                DeviceTwoQubitSynthesisContext::build(&target.device, &state.current, placement)?;
            ResynthesizeTwoQubitBlocks::new_device_aware(config, context)
        } else {
            ResynthesizeTwoQubitBlocks::new(config)
        };
        state.apply_transform(stage, name, |circuit, analysis| {
            resynthesizer.transform(circuit, Some(analysis))
        })
    }

    fn apply_post_routing_resynthesis(
        &self,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        if self.routing_device_target().is_none() {
            state.record_skipped(
                "optimization",
                "resynthesize.two_qubit_blocks.post_routing",
                "routing was skipped",
            );
            return Ok(());
        }
        self.apply_two_qubit_resynthesis(
            state,
            "optimization",
            "resynthesize.two_qubit_blocks.post_routing",
            DeviceSynthesisPlacement::ExactPhysical,
        )?;
        Ok(())
    }

    fn two_qubit_resynthesis_config_for_state(
        &self,
        state: &WorkflowState,
    ) -> TwoQubitBlockResynthesisConfig {
        match self.config.mode {
            CompileMode::Normal => {
                TwoQubitBlockResynthesisConfig::normal(state.two_qubit_target.clone())
            }
            CompileMode::Enhanced => {
                TwoQubitBlockResynthesisConfig::enhanced(state.two_qubit_target.clone())
            }
        }
    }

    fn apply_routing_basis_decomposition(
        &self,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        if self.routing_device_target().is_none() {
            state.record_skipped(
                "translation",
                "decompose.routing_basis",
                "no target device configured",
            );
            return Ok(());
        }

        let preferred_basis = state
            .prepared_target_basis
            .as_ref()
            .map(|prepared| prepared.instructions.to_vec());
        state.apply_transform(
            "translation",
            "decompose.routing_basis",
            |circuit, analysis| {
                LowerToRoutingBasis::new(preferred_basis).transform(circuit, Some(analysis))
            },
        )?;
        Ok(())
    }

    fn mc_gate_decompose_config(&self) -> McGateDecomposeConfig {
        McGateDecomposeConfig {
            resource_policy: self.config.resource_policy,
            resource_limits: self.resource_limits(),
        }
    }

    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            max_total_qubits: self
                .routing_device_target()
                .map(|target| target.device.num_usable_qubits()),
        }
    }

    /// Performs capacity-style resource preflight before lowering starts.
    ///
    /// Detailed ancillary leasing is still enforced by the decomposition
    /// resource manager when a specific synthesis candidate is selected.
    fn validate_resources(
        &self,
        circuit: &Circuit,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        let resource_limits = self.resource_limits();
        if let Some(max_total_qubits) = resource_limits.max_total_qubits
            && circuit.qubits().len() > max_total_qubits
        {
            return Err(CompilerError::InvalidInput(format!(
                "source circuit uses {} logical qubits but target capacity is {max_total_qubits}",
                circuit.qubits().len()
            )));
        }
        state.steps.push(WorkflowStepReport {
            stage: "pre_init",
            name: "validate.resources",
            changed: false,
            skipped: false,
            reason: resource_limits
                .max_total_qubits
                .map(|capacity| format!("target capacity permits {capacity} total logical qubits")),
        });
        Ok(())
    }

    /// Runs layout selection and SABRE routing, or records a skipped routing step.
    ///
    /// A caller-supplied initial layout bypasses layout search but still uses
    /// the same SABRE router and trial settings. Without a supplied layout, the
    /// workflow uses the topology/direction-only compiler SABRE objective.
    fn apply_layout_and_routing(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let Some(target) = self.routing_device_target() else {
            state.record_skipped("routing", "route.sabre", "no target device configured");
            return Ok(());
        };

        let virtual_permutation = state.virtual_permutation.clone().ok_or_else(|| {
            CompilerError::InvariantViolation(
                "routing started before virtual-permutation preparation".to_string(),
            )
        })?;

        let routing_device = self.routing_device(target)?;
        let device = routing_device.as_ref();
        let config = sabre_config_for_mode(self.config.mode, target.seed);
        let (route_changed, swap_count, trials_evaluated, supplied_layout) =
            if let Some(initial_layout) = target.initial_layout.as_ref() {
                let routed = route_with_layout(&state.current, device, initial_layout, &config)?;
                let route_changed = routed.changed(&state.current);
                let swap_count = routed.swap_count();
                let trials_evaluated = routed.diagnostics().trials_evaluated;
                let final_layout =
                    virtual_permutation.compose_final_layout(routed.final_layout())?;
                state.device_metadata = Some(DeviceCompilationMetadata {
                    initial_layout: routed.initial_layout().clone(),
                    final_layout,
                    virtual_permutation: virtual_permutation.clone(),
                });
                state.current = routed.into_circuit();
                (route_changed, swap_count, trials_evaluated, true)
            } else {
                let objective = LayoutObjective::topology_only();
                let routed = route_sabre(&state.current, device, &objective, &config)?;
                let route_changed = routed.changed(&state.current);
                let swap_count = routed.swap_count();
                let trials_evaluated = routed.diagnostics().trials_evaluated;
                let final_layout =
                    virtual_permutation.compose_final_layout(routed.final_layout())?;
                state.device_metadata = Some(DeviceCompilationMetadata {
                    initial_layout: routed.initial_layout().clone(),
                    final_layout,
                    virtual_permutation: virtual_permutation.clone(),
                });
                state.current = routed.into_routed().into_circuit();
                (route_changed, swap_count, trials_evaluated, false)
            };
        state.changed |= route_changed;

        let reason = if supplied_layout {
            format!(
                "inserted {} swap operations using {} routing trials from supplied initial layout",
                swap_count, trials_evaluated
            )
        } else {
            format!(
                "inserted {} swap operations using {} routing trials",
                swap_count, trials_evaluated
            )
        };

        state.steps.push(WorkflowStepReport {
            stage: "routing",
            name: "route.sabre",
            changed: route_changed,
            skipped: false,
            reason: Some(reason),
        });
        Ok(())
    }

    /// Removes static logical SWAPs before numeric two-qubit optimization can
    /// absorb them into a strictly equivalent unitary realization.
    ///
    /// The resulting output permutation remains valid through later logical
    /// rewrites because those stages preserve logical wire identity.
    fn apply_virtual_permutation_elision(
        &self,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        if self.routing_device_target().is_none() {
            state.record_skipped(
                "optimization",
                "optimize.virtual_permutation",
                "no target device configured",
            );
            return Ok(());
        }

        let elision = elide_virtual_permutations(&state.current)?;
        let elision_status = elision.status();
        let elided_swap_count = elision.elided_swap_count();
        state.virtual_permutation = Some(elision.virtual_permutation().clone());
        match elision_status {
            VirtualPermutationElisionStatus::Changed => {
                state.current = elision.into_changed_circuit().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "changed virtual-permutation result did not contain a circuit".to_string(),
                    )
                })?;
                state.analysis = CircuitAnalysis::analyze(&state.current);
                state.changed = true;
                state.steps.push(WorkflowStepReport {
                    stage: "optimization",
                    name: "optimize.virtual_permutation",
                    changed: true,
                    skipped: false,
                    reason: Some(format!(
                        "represented {elided_swap_count} logical swap operations as an output permutation"
                    )),
                });
            }
            VirtualPermutationElisionStatus::Unchanged => {
                state.record_skipped(
                    "optimization",
                    "optimize.virtual_permutation",
                    "no eligible logical swap operations",
                );
            }
            VirtualPermutationElisionStatus::SkippedControlFlow => {
                state.record_skipped(
                    "optimization",
                    "optimize.virtual_permutation",
                    "structured control flow may have path-dependent output permutations",
                );
            }
        }
        Ok(())
    }

    fn apply_post_routing_cleanup(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        if self.routing_device_target().is_none() {
            state.record_skipped(
                "optimization",
                "optimize.post_routing",
                "routing was skipped",
            );
            return Ok(());
        }

        let rewrite_config = self.rewrite_config(RewritePhase::PostRouting)?;
        state.apply_transform(
            "optimization",
            "optimize.post_routing",
            |circuit, analysis| {
                KnowledgeRewriter::new(rewrite_config).transform(circuit, Some(analysis))
            },
        )?;
        Ok(())
    }

    fn apply_commutative_cancellation(
        &self,
        state: &mut WorkflowState,
        name: &'static str,
    ) -> Result<bool, CompilerError> {
        state.apply_transform("optimization", name, |circuit, analysis| {
            CommutativeCancellation::new().transform(circuit, Some(analysis))
        })
    }

    fn apply_one_qubit_optimization(
        &self,
        state: &mut WorkflowState,
        name: &'static str,
    ) -> Result<bool, CompilerError> {
        let Some(optimizer) = state.one_qubit_optimizer.take() else {
            state.record_skipped(
                "optimization",
                name,
                "device target uses exact-physical native one-qubit optimization",
            );
            return Ok(false);
        };
        let result = state.apply_transform("optimization", name, |circuit, analysis| {
            optimizer.transform(circuit, Some(analysis))
        });
        state.one_qubit_optimizer = Some(optimizer);
        result
    }

    fn close_one_qubit_resynthesis(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        if matches!(self.config.target, CompileTarget::Device(_)) {
            state.record_skipped(
                "optimization",
                "optimize.one_qubit_fixed_point",
                "device target closes local optimization after exact native lowering",
            );
            state.pending_one_qubit_resynthesis = false;
            return Ok(());
        }

        let max_rounds = match self.config.mode {
            CompileMode::Normal => 2,
            CompileMode::Enhanced => 4,
        };
        let before = state.current.clone();
        let mut rounds = 0u8;
        let mut pending_resynthesis = state.pending_one_qubit_resynthesis;
        while rounds < max_rounds {
            rounds += 1;
            let changed_cc = self.apply_commutative_cancellation(
                state,
                "optimize.commutative_cancellation.after_rewrite",
            )?;
            let changed_1q =
                self.apply_one_qubit_optimization(state, "optimize.one_qubit.after_rewrite")?;
            // The pre-loop pending flag is consumed exactly once: after this
            // point the flag is only ever reassigned from `cleanup_changed`.
            let should_resynthesize = pending_resynthesis || changed_cc || changed_1q;
            if !should_resynthesize {
                break;
            }

            let resynthesis_changed = self.apply_two_qubit_resynthesis(
                state,
                "optimization",
                "resynthesize.two_qubit_blocks.after_one_qubit",
                DeviceSynthesisPlacement::PreLayoutEnvelope,
            )?;
            let cleanup_changed = if resynthesis_changed {
                self.apply_one_qubit_optimization(state, "optimize.one_qubit.after_resynthesis")?
            } else {
                state.record_skipped(
                    "optimization",
                    "optimize.one_qubit.after_resynthesis",
                    "two-qubit resynthesis was stable",
                );
                false
            };
            let round_changed = changed_cc || changed_1q || resynthesis_changed || cleanup_changed;
            pending_resynthesis = cleanup_changed;
            if !round_changed {
                break;
            }
        }
        state.pending_one_qubit_resynthesis = false;
        state.steps.push(WorkflowStepReport {
            stage: "optimization",
            name: "optimize.one_qubit_fixed_point",
            changed: state.current != before,
            skipped: false,
            reason: Some(format!("rounds={rounds}; max_rounds={max_rounds}")),
        });
        Ok(())
    }

    fn apply_target_translation(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        self.apply_target_translation_named(state, "translate.target_basis")
    }

    fn apply_target_translation_named(
        &self,
        state: &mut WorkflowState,
        name: &'static str,
    ) -> Result<(), CompilerError> {
        let Some(prepared) = state.prepared_target_basis.as_ref() else {
            state.record_skipped("translation", name, "no target basis configured");
            return Ok(());
        };
        let lowerer = Arc::clone(&prepared.lowerer);

        if !lowerer.requires_lowering(&state.current) {
            state.record_skipped(
                "translation",
                name,
                "circuit already satisfies the explicit target basis",
            );
            return Ok(());
        }

        state.apply_transform("translation", name, |circuit, analysis| {
            lowerer.transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn validate_explicit_target_basis(
        &self,
        state: &mut WorkflowState,
    ) -> Result<(), CompilerError> {
        let Some(prepared) = state.prepared_target_basis.as_ref() else {
            return Ok(());
        };
        let allowed = prepared
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Standard(gate) => Some(*gate),
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>();
        validate_operations_in_target_basis(state.current.operations(), &allowed)
    }

    /// Prepares one shared explicit-basis planning graph for this workflow run.
    ///
    /// Device capabilities are local and ordered, so they are handled by the
    /// exact device-lowering stage.
    fn prepare_target_basis(&self) -> Result<Option<PreparedTargetBasis>, CompilerError> {
        let target_basis = match &self.config.target {
            CompileTarget::Basis(target_basis)
            | CompileTarget::TopologyBasis {
                basis: target_basis,
                ..
            } => Some(target_basis),
            CompileTarget::Logical | CompileTarget::Device(_) => None,
        };
        let Some(target_basis) = target_basis else {
            return Ok(None);
        };
        validate_workflow_target_basis_config(target_basis)?;
        PreparedTargetBasis::new(target_basis.to_vec()).map(Some)
    }

    fn record_pre_init(&self, state: &mut WorkflowState) {
        let reason = match (&self.config.target, &state.prepared_target_basis) {
            (CompileTarget::Basis(_), Some(prepared)) => Some(format!(
                "resolved explicit target basis with {} instructions",
                prepared.instructions.len()
            )),
            (CompileTarget::Device(_), _) => {
                Some("resolved device target with ordered native capabilities".to_string())
            }
            (CompileTarget::TopologyBasis { .. }, Some(prepared)) => Some(format!(
                "resolved device topology with explicit target basis containing {} instructions",
                prepared.instructions.len()
            )),
            (CompileTarget::Logical, _) => Some("no target constraints configured".to_string()),
            (CompileTarget::Basis(_) | CompileTarget::TopologyBasis { .. }, None) => None,
        };

        state.steps.push(WorkflowStepReport {
            stage: "pre_init",
            name: "resolve.target",
            changed: false,
            skipped: false,
            reason,
        });
    }

    fn routing_device_target(&self) -> Option<&DeviceCompileTarget> {
        match &self.config.target {
            CompileTarget::Device(target) => Some(target),
            CompileTarget::TopologyBasis { device_target, .. } => Some(device_target),
            CompileTarget::Logical | CompileTarget::Basis(_) => None,
        }
    }

    fn strict_device_target(&self) -> Option<&DeviceCompileTarget> {
        match &self.config.target {
            CompileTarget::Device(target) => Some(target),
            CompileTarget::Logical
            | CompileTarget::Basis(_)
            | CompileTarget::TopologyBasis { .. } => None,
        }
    }

    fn strict_device_skip_reason(&self) -> &'static str {
        if matches!(self.config.target, CompileTarget::TopologyBasis { .. }) {
            "topology target does not require exact device-native output"
        } else {
            "no target device configured"
        }
    }

    fn routing_device<'target>(
        &self,
        target: &'target DeviceCompileTarget,
    ) -> Result<Cow<'target, Device>, CompilerError> {
        if matches!(self.config.target, CompileTarget::Device(_)) {
            return Ok(Cow::Borrowed(&target.device));
        }

        let qubits = target.device.qubits().collect::<Vec<_>>();
        let couplings = target
            .device
            .topology()
            .undirected_edges()
            .flat_map(|(a, b)| {
                [
                    (a, b, "loose-topology".to_string()),
                    (b, a, "loose-topology".to_string()),
                ]
            })
            .collect::<Vec<_>>();
        let topology = Topology::new(qubits.clone(), couplings).map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "failed to construct loose routing topology: {error}"
            ))
        })?;
        let mut device = Device::new(
            format!("{} (loose topology)", target.device.name()),
            qubits.into_iter().collect(),
            topology,
        )
        .map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "failed to construct loose routing device: {error}"
            ))
        })?;
        device
            .set_invalid_qubits(target.device.invalid_qubits().collect())
            .map_err(|error| {
                CompilerError::InvariantViolation(format!(
                    "failed to copy loose routing availability: {error}"
                ))
            })?;
        let gates = match &self.config.target {
            CompileTarget::TopologyBasis { basis, .. } => basis.clone(),
            CompileTarget::Logical | CompileTarget::Basis(_) | CompileTarget::Device(_) => {
                unreachable!("strict and non-routing targets returned before loose device setup")
            }
        };
        device.set_native_gates(gates).map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "failed to configure loose routing instructions: {error}"
            ))
        })?;
        Ok(Cow::Owned(device))
    }
}

fn sabre_config_for_mode(mode: CompileMode, seed: Option<u32>) -> SabreConfig {
    let mut config = SabreConfig {
        seed: seed.map(u64::from),
        ..SabreConfig::default()
    };

    if mode == CompileMode::Enhanced {
        config.layout_trials = 24;
        config.layout_assignment_budget = 5_000_000;
        if let Some(vf2) = &mut config.vf2_prepass {
            vf2.call_limit = 5_000_000;
        }
        config.refinement_iterations = 2;
        config.routing_trials = 2;
    }

    config
}

fn validate_workflow_target_basis_config(
    target_basis: &[Instruction],
) -> Result<(), CompilerError> {
    if target_basis.is_empty() {
        return Err(CompilerError::InvalidInput(
            "workflow target basis must not be empty".to_string(),
        ));
    }

    // Rewrite lowering can represent `McGate` as a target instruction, but the
    // current workflow decomposes all multi-controlled gates before target-basis
    // translation. Native multi-controlled target support therefore needs an
    // explicit workflow policy before it can be accepted here.
    for instruction in target_basis {
        if !matches!(instruction, Instruction::Standard(_)) {
            return Err(CompilerError::InvalidInput(format!(
                "unsupported workflow target instruction {instruction:?}"
            )));
        }
    }
    Ok(())
}

fn validate_operations_in_target_basis(
    operations: &[Operation],
    allowed: &std::collections::HashSet<StandardGate>,
) -> Result<(), CompilerError> {
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(gate)
                if *gate != StandardGate::GPhase && !allowed.contains(gate) =>
            {
                return Err(CompilerError::InvariantViolation(format!(
                    "target-basis one-qubit cleanup left gate {gate:?} outside the configured basis"
                )));
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    validate_operations_in_target_basis(op.then_body().operations(), allowed)?;
                    if let Some(body) = op.else_body() {
                        validate_operations_in_target_basis(body.operations(), allowed)?;
                    }
                }
                ClassicalControlOp::While(op) => {
                    validate_operations_in_target_basis(op.body().operations(), allowed)?;
                }
                ClassicalControlOp::For(op) => {
                    validate_operations_in_target_basis(op.body().operations(), allowed)?;
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        validate_operations_in_target_basis(case.body().operations(), allowed)?;
                    }
                    if let Some(body) = op.default() {
                        validate_operations_in_target_basis(body.operations(), allowed)?;
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            Instruction::McGate(_) | Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                return Err(CompilerError::InvariantViolation(format!(
                    "target-basis one-qubit cleanup left gate-like instruction {} outside the configured basis",
                    operation.instruction
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "./workflow_test.rs"]
mod workflow_test;
