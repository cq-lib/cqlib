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
//! The shared workflow follows the stable pass order:
//! canonicalize input, expand circuit-backed definitions, apply production
//! knowledge rewrite, decompose unitary and multi-controlled gates,
//! canonicalize again, optimize the decomposed circuit, optionally lower to a
//! routing-compatible basis, optionally route on a device, optionally translate
//! to the resolved target basis, and canonicalize the output representation. A
//! strict-device candidate suffix then lowers every gate to exact ordered native
//! capabilities, closes a bounded native-optimization loop, and validates the
//! completed physical circuit.
//!
//! The enhanced workflow uses the same correctness contract but raises rewrite,
//! resynthesis, native-optimization, and SABRE search budgets, performs
//! post-routing cleanup, and adds target-aware cleanup when an explicit target
//! basis is present. For a strict [`CompileTarget::Device`], it also snapshots
//! the immutable pre-routing prefix. Candidate zero first completes the entire
//! routing-to-validation suffix; a bounded two-tier SABRE Pareto beam then
//! explores alternative routes from the same prefix, with every candidate
//! independently traversing that same suffix through device validation. An
//! exploratory candidate replaces candidate zero only when it preserves the
//! exact native-quality contract in every control-flow scope and strictly
//! improves native two-qubit count or depth. Otherwise the validated candidate
//! zero remains the result.
//!
//! Stages are deliberately ordered around compiler invariants. Early
//! canonicalization gives later passes a stable representation, definition and
//! high-level gate decomposition remove operations that routing cannot accept,
//! routing runs before final target-basis cleanup because it may insert SWAPs,
//! and output canonicalization removes representation noise before exact device
//! lowering. Native optimization is re-legalized and costed on exact physical
//! qargs. Validation is terminal within each finalized candidate suffix. In an
//! enhanced strict-device workflow, `select.sabre_pareto_beam` is the final
//! orchestration and reporting step; it applies no transform after validation
//! and can select only an already validated candidate.

use crate::circuit::{Circuit, ClassicalControlOp, Instruction, Operation, StandardGate};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::resource::ResourceLimits;
use crate::compile::sabre::{RoutingTarget, SabreConfig};
use crate::compile::transform::analysis::WorkflowCircuitAnalysis;
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext, TwoQubitSynthesisTarget,
};
use crate::compile::transform::decompose::{
    DecomposeDefinitions, DecomposeMcGates, DecomposeUnitaries, McGateDecomposeConfig,
    UnitaryDecomposeConfig,
};
use crate::compile::transform::layout::PhysicalLayoutGraph;
use crate::compile::transform::native_optimization::NativeOptimizer;
use crate::compile::transform::resynthesis::{
    WorkflowResynthesisSession, resynthesize_two_qubit_blocks_workflow,
};
use crate::compile::transform::transformer::{PassApplicability, WorkflowPass};
use crate::compile::transform::{
    CanonicalizeConfig, Canonicalizer, CircuitAnalysis, CommutativeCancellation, DeviceLowerer,
    KnowledgeRewriteDiagnostics, KnowledgeRewriteSession, KnowledgeRewriter, LayoutObjective,
    LowerToRoutingBasis, NativeQualityPolicy, OptimizeOneQubitRuns, ResynthesizeTwoQubitBlocks,
    RewriteConfig, RewriteEdits, RewriteExecutionRecord, TargetBasisCostModel, TargetBasisLowerer,
    TransformOutcome, Transformer, TwoQubitBlockResynthesisConfig, VirtualPermutation,
    VirtualPermutationElisionStatus, elide_virtual_permutations, route_sabre_tracked_on_topology,
    route_sabre_tracked_with_session_on_physical, route_with_layout_tracked_on_topology,
    route_with_layout_tracked_with_session_on_physical,
};
use std::sync::{Arc, Mutex, OnceLock};

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

/// Evidence that one exact circuit revision satisfies one canonicalization
/// configuration. Any circuit mutation invalidates the evidence implicitly by
/// advancing the revision.
#[derive(Debug, Clone)]
struct CanonicalizationProof {
    circuit_revision: u64,
    config: CanonicalizeConfig,
}

struct WorkflowState {
    current: Circuit,
    analysis: Option<WorkflowCircuitAnalysis>,
    circuit_revision: u64,
    canonical_proof: Option<CanonicalizationProof>,
    rewrite_session: KnowledgeRewriteSession,
    rewrite_diagnostics: KnowledgeRewriteDiagnostics,
    collect_rewrite_diagnostics: bool,
    changed: bool,
    steps: Vec<WorkflowStepReport>,
    prepared_target_basis: Option<Arc<PreparedTargetBasis>>,
    two_qubit_target: TwoQubitSynthesisTarget,
    virtual_permutation: Option<VirtualPermutation>,
    device_metadata: Option<DeviceCompilationMetadata>,
    one_qubit_optimizer: Option<OptimizeOneQubitRuns>,
    pending_one_qubit_resynthesis: bool,
    resynthesis_session: WorkflowResynthesisSession,
    routing_physical: Option<Arc<PhysicalLayoutGraph>>,
    topology_routing_target: Option<Arc<RoutingTarget>>,
    planning_session: Option<Arc<DevicePlanningSession>>,
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

/// Target-derived data shared by every run of one compiler workflow.
///
/// Circuit analysis, SABRE DAGs, interaction requirements, and route metadata
/// deliberately remain in [`WorkflowState`] or the routing call that owns them.
struct PreparedCompileTarget {
    target_basis: Option<Arc<PreparedTargetBasis>>,
    one_qubit_optimizer: Option<OptimizeOneQubitRuns>,
    two_qubit_target: TwoQubitSynthesisTarget,
    routing_physical: Option<Arc<PhysicalLayoutGraph>>,
    topology_routing_target: Option<Arc<RoutingTarget>>,
    planning_session: Option<Arc<DevicePlanningSession>>,
}

impl PreparedCompileTarget {
    fn new(config: &CompileConfig) -> Result<Self, CompilerError> {
        let target_basis = match &config.target {
            CompileTarget::Basis(target_basis)
            | CompileTarget::TopologyBasis {
                basis: target_basis,
                ..
            } => {
                validate_workflow_target_basis_config(target_basis)?;
                Some(Arc::new(PreparedTargetBasis::new(target_basis.to_vec())?))
            }
            CompileTarget::Logical | CompileTarget::Device(_) => None,
        };
        let one_qubit_optimizer = match &config.target {
            CompileTarget::Logical => Some(OptimizeOneQubitRuns::logical()),
            CompileTarget::Basis(_) | CompileTarget::TopologyBasis { .. } => {
                let prepared = target_basis.as_ref().ok_or_else(|| {
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
        let two_qubit_target = target_basis
            .as_ref()
            .map_or_else(TwoQubitSynthesisTarget::unconstrained, |prepared| {
                TwoQubitSynthesisTarget::from_cost_model(Arc::clone(&prepared.cost_model))
            });
        let (routing_physical, topology_routing_target, planning_session) = match &config.target {
            CompileTarget::TopologyBasis { device_target, .. } => {
                let physical = Arc::new(PhysicalLayoutGraph::from_device(&device_target.device)?);
                let routing = Arc::new(RoutingTarget::from_physical(&physical)?);
                (Some(physical), Some(routing), None)
            }
            CompileTarget::Device(target) => {
                let physical = Arc::new(PhysicalLayoutGraph::from_device(&target.device)?);
                let session = Arc::new(DevicePlanningSession::new(&target.device));
                (Some(physical), None, Some(session))
            }
            CompileTarget::Logical | CompileTarget::Basis(_) => (None, None, None),
        };
        Ok(Self {
            target_basis,
            one_qubit_optimizer,
            two_qubit_target,
            routing_physical,
            topology_routing_target,
            planning_session,
        })
    }
}

impl WorkflowState {
    fn analysis(&mut self) -> &WorkflowCircuitAnalysis {
        if self.analysis.is_none() {
            self.analysis = Some(WorkflowCircuitAnalysis::analyze(&self.current));
        }
        self.analysis
            .as_ref()
            .expect("workflow analysis was initialized above")
    }

    fn skip_if_proven_noop(
        &mut self,
        stage: &'static str,
        name: &'static str,
        pass: &impl WorkflowPass,
    ) -> bool {
        let applicability = pass.applicability(self.analysis());
        let PassApplicability::ProvenNoOp(reason) = applicability else {
            return false;
        };
        self.record_skipped(stage, name, reason);
        true
    }

    fn advance_circuit_revision(&mut self) {
        self.circuit_revision = self.circuit_revision.wrapping_add(1);
        if self.circuit_revision == 0 {
            // Prevent a wrapped revision from aliasing an ancient proof.
            self.rewrite_session.invalidate();
            self.canonical_proof = None;
        }
    }

    fn adopt_changed_circuit(&mut self, circuit: Circuit, edits: &RewriteEdits) {
        let old_revision = self.circuit_revision;
        let new_revision = old_revision.wrapping_add(1);
        if new_revision == 0 {
            self.rewrite_session.invalidate();
            self.canonical_proof = None;
        } else {
            self.rewrite_session.apply_rewrite_edits(
                old_revision,
                new_revision,
                &self.current,
                &circuit,
                edits,
            );
        }
        self.analysis = None;
        self.current = circuit;
        self.circuit_revision = new_revision;
        self.changed = true;
    }

    fn apply_transform(
        &mut self,
        stage: &'static str,
        name: &'static str,
        transform: impl FnOnce(&Circuit, &CircuitAnalysis) -> Result<TransformOutcome, CompilerError>,
    ) -> Result<bool, CompilerError> {
        let tracks_rewrite_edits = self.rewrite_session.tracks_rewrite_edits();
        self.apply_transform_with_edits(stage, name, |circuit, analysis| {
            let outcome = transform(circuit, analysis)?;
            let edits = match &outcome {
                TransformOutcome::Unchanged => RewriteEdits::linear(
                    circuit.operations().len(),
                    circuit.operations().len(),
                    Vec::new(),
                ),
                TransformOutcome::Changed(after) if tracks_rewrite_edits => {
                    RewriteEdits::between_linear_circuits(circuit, after)
                }
                TransformOutcome::Changed(_) => RewriteEdits::Unknown,
            };
            Ok((outcome, edits))
        })
    }

    fn apply_transform_without_analysis_with_edits(
        &mut self,
        stage: &'static str,
        name: &'static str,
        transform: impl FnOnce(&Circuit) -> Result<(TransformOutcome, RewriteEdits), CompilerError>,
    ) -> Result<bool, CompilerError> {
        let changed = match transform(&self.current)? {
            (TransformOutcome::Unchanged, _) => false,
            (TransformOutcome::Changed(circuit), edits) => {
                self.adopt_changed_circuit(circuit, &edits);
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

    fn apply_transform_with_edits(
        &mut self,
        stage: &'static str,
        name: &'static str,
        transform: impl FnOnce(
            &Circuit,
            &CircuitAnalysis,
        ) -> Result<(TransformOutcome, RewriteEdits), CompilerError>,
    ) -> Result<bool, CompilerError> {
        if self.analysis.is_none() {
            self.analysis = Some(WorkflowCircuitAnalysis::analyze(&self.current));
        }
        let analysis = self
            .analysis
            .as_ref()
            .expect("workflow analysis was initialized above")
            .public();
        let changed = match transform(&self.current, analysis)? {
            (TransformOutcome::Unchanged, _) => false,
            (TransformOutcome::Changed(circuit), edits) => {
                self.adopt_changed_circuit(circuit, &edits);
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

    fn apply_canonicalize(
        &mut self,
        stage: &'static str,
        name: &'static str,
        canonicalizer: &Canonicalizer,
    ) -> Result<bool, CompilerError> {
        if self.has_canonical_proof(canonicalizer.config()) {
            // Proof reuse is an executed no-op, rather than a skipped workflow
            // stage: the same postcondition is established without rerunning
            // the canonicalizer.
            self.steps.push(WorkflowStepReport {
                stage,
                name,
                changed: false,
                skipped: false,
                reason: None,
            });
            return Ok(false);
        }
        let changed = self.apply_transform_without_analysis_with_edits(stage, name, |circuit| {
            canonicalizer.transform_with_rewrite_edits(circuit)
        })?;
        self.canonical_proof = Some(CanonicalizationProof {
            circuit_revision: self.circuit_revision,
            config: canonicalizer.config().clone(),
        });
        Ok(changed)
    }

    fn has_canonical_proof(&self, config: &CanonicalizeConfig) -> bool {
        self.canonical_proof.as_ref().is_some_and(|proof| {
            proof.circuit_revision == self.circuit_revision && &proof.config == config
        })
    }

    fn apply_knowledge_rewrite(
        &mut self,
        stage: &'static str,
        name: &'static str,
        config: RewriteConfig,
    ) -> Result<bool, CompilerError> {
        if self
            .rewrite_session
            .reusable_proof(self.circuit_revision, &config)
            .is_some()
        {
            if self.collect_rewrite_diagnostics {
                self.rewrite_diagnostics.merge(KnowledgeRewriteDiagnostics {
                    direct_reuses: 1,
                    ..KnowledgeRewriteDiagnostics::default()
                });
            }
            self.steps.push(WorkflowStepReport {
                stage,
                name,
                changed: false,
                skipped: false,
                reason: None,
            });
            return Ok(false);
        }

        let rewriter = KnowledgeRewriter::new(config.clone());
        let proof_reach = rewriter.proof_reach()?;
        let qubit_bijection_invariant = rewriter.proof_is_qubit_bijection_invariant()?;
        let reconciliation = self
            .rewrite_session
            .take_reconciled_linear_state(self.circuit_revision, &config);
        let (result, workspace) = rewriter.run_for_session(
            &self.current,
            reconciliation.state,
            self.collect_rewrite_diagnostics,
        )?;
        if self.collect_rewrite_diagnostics && reconciliation.full_scan_fallback {
            self.rewrite_diagnostics.full_scan_fallbacks = self
                .rewrite_diagnostics
                .full_scan_fallbacks
                .saturating_add(1);
        }
        if self.collect_rewrite_diagnostics {
            self.rewrite_diagnostics.merge(result.diagnostics);
        }
        let changed = result.changed;
        if changed {
            let circuit = result.circuit.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "knowledge rewrite reported a change without an output circuit".to_string(),
                )
            })?;
            self.analysis = None;
            self.current = circuit;
            self.advance_circuit_revision();
            self.changed = true;
        }
        self.rewrite_session.record_execution(
            &self.current,
            self.circuit_revision,
            RewriteExecutionRecord {
                config,
                proof_reach,
                qubit_bijection_invariant,
                reached_fixpoint: result.stats.reached_fixpoint,
                workspace,
            },
        );
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
    prepared_target: OnceLock<Arc<PreparedCompileTarget>>,
    prepare_lock: Mutex<()>,
}

impl CompilerWorkflow {
    /// Creates a compiler workflow from a complete configuration.
    ///
    /// Target preparation is deferred until the first run. Use
    /// [`Self::try_new`] when construction should validate and prepare the
    /// complete target eagerly.
    pub const fn new(config: CompileConfig) -> Self {
        Self {
            config,
            prepared_target: OnceLock::new(),
            prepare_lock: Mutex::new(()),
        }
    }

    /// Creates a workflow and eagerly prepares all target-derived invariants.
    pub fn try_new(config: CompileConfig) -> Result<Self, CompilerError> {
        let workflow = Self::new(config);
        workflow.prepared_target()?;
        Ok(workflow)
    }

    /// Returns the workflow configuration.
    pub const fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// Runs the workflow over `circuit` and returns the rebuilt circuit plus
    /// execution metadata.
    pub fn run(&self, circuit: &Circuit) -> Result<CompileResult, CompilerError> {
        self.run_owned(circuit.clone())
    }

    /// Runs the workflow by consuming `circuit`.
    ///
    /// This avoids the workflow's defensive input clone when the caller
    /// already owns an isolated circuit value. The borrowed [`Self::run`]
    /// entry point remains the compatibility path for callers that need to
    /// retain their input.
    pub fn run_owned(&self, circuit: Circuit) -> Result<CompileResult, CompilerError> {
        self.run_internal(circuit, false).map(|(result, _)| result)
    }

    /// Runs the workflow and also returns aggregate rewrite diagnostics.
    ///
    /// This explicit entry point keeps benchmark-only attribution separate
    /// from [`CompileResult`]'s stable semantic equality and normal API.
    pub fn run_with_rewrite_diagnostics(
        &self,
        circuit: &Circuit,
    ) -> Result<(CompileResult, KnowledgeRewriteDiagnostics), CompilerError> {
        self.run_owned_with_rewrite_diagnostics(circuit.clone())
    }

    /// Runs the workflow by consuming `circuit` and collects rewrite
    /// diagnostics for benchmark attribution.
    ///
    /// This is the owned-input counterpart of
    /// [`Self::run_with_rewrite_diagnostics`] and avoids its defensive clone.
    pub fn run_owned_with_rewrite_diagnostics(
        &self,
        circuit: Circuit,
    ) -> Result<(CompileResult, KnowledgeRewriteDiagnostics), CompilerError> {
        self.run_internal(circuit, true)
    }

    fn run_internal(
        &self,
        circuit: Circuit,
        collect_rewrite_diagnostics: bool,
    ) -> Result<(CompileResult, KnowledgeRewriteDiagnostics), CompilerError> {
        let prepared_target = self.prepared_target()?;
        let analysis = WorkflowCircuitAnalysis::analyze(&circuit);
        let mut state = WorkflowState {
            current: circuit,
            analysis: Some(analysis),
            circuit_revision: 0,
            canonical_proof: None,
            rewrite_session: KnowledgeRewriteSession::default(),
            rewrite_diagnostics: KnowledgeRewriteDiagnostics::default(),
            collect_rewrite_diagnostics,
            changed: false,
            steps: Vec::new(),
            prepared_target_basis: prepared_target.target_basis.clone(),
            two_qubit_target: prepared_target.two_qubit_target.clone(),
            virtual_permutation: None,
            device_metadata: None,
            one_qubit_optimizer: prepared_target.one_qubit_optimizer.clone(),
            pending_one_qubit_resynthesis: false,
            resynthesis_session: WorkflowResynthesisSession::default(),
            routing_physical: prepared_target.routing_physical.clone(),
            topology_routing_target: prepared_target.topology_routing_target.clone(),
            planning_session: prepared_target.planning_session.clone(),
        };

        self.record_pre_init(&mut state);
        self.validate_resources(&mut state)?;
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

        Ok((
            CompileResult {
                circuit: state.current,
                changed: state.changed,
                mode: self.config.mode,
                steps: state.steps,
                device_metadata: state.device_metadata,
            },
            state.rewrite_diagnostics,
        ))
    }

    /// Establishes a stable high-level IR before gate-specific lowering.
    ///
    /// Definition expansion precedes the first rewrite pass so knowledge rules
    /// see the operations contained by user-defined gates.
    fn lower_init(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        state.apply_canonicalize("init", "canonicalize.input", &Canonicalizer::production())?;
        self.apply_definition_decomposition(state)?;
        let rewrite_config = self.rewrite_config(RewritePhase::PreDecomposition)?;
        state.apply_knowledge_rewrite(
            "optimization",
            "optimize.pre_decomposition",
            rewrite_config,
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
        state.apply_canonicalize(
            "optimization",
            "canonicalize.after_decomposition",
            &Canonicalizer::production(),
        )?;
        state.apply_transform_without_analysis_with_edits(
            "optimization",
            "optimize.commutative_cancellation",
            |circuit| CommutativeCancellation::new().transform_with_rewrite_edits(circuit),
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
        state.apply_knowledge_rewrite(
            "optimization",
            "optimize.post_decomposition",
            rewrite_config,
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
                state.apply_knowledge_rewrite(
                    "optimization",
                    "optimize.target_cleanup",
                    cleanup_config,
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
        state.apply_canonicalize(
            "output",
            "canonicalize.output",
            &Canonicalizer::production(),
        )?;
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
        let planning_session = state.planning_session.clone();
        let lowerer = if let Some(session) = planning_session.as_deref() {
            DeviceLowerer::with_session(&target.device, session)
        } else {
            DeviceLowerer::new(&target.device)
        };
        state.apply_transform_without_analysis_with_edits(
            "translation",
            "lower.device_instructions",
            |circuit| Ok((lowerer.transform(circuit, None)?, RewriteEdits::Unknown)),
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
        // The native optimizer owns this entry postcondition: it reuses an
        // exact workflow proof for the current revision, or canonicalizes the
        // input itself when device lowering invalidated that proof. A separate
        // workflow pass here would duplicate one of those paths.
        state.record_skipped(
            "optimization",
            "canonicalize.native_input",
            "native optimizer establishes canonical input on entry",
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
            CompileMode::Normal => (
                NativeOptimizer::NORMAL_MAX_ROUNDS,
                NativeOptimizer::NORMAL_MAX_STALE_ROUNDS,
            ),
            CompileMode::Enhanced => (
                NativeOptimizer::ENHANCED_MAX_ROUNDS,
                NativeOptimizer::ENHANCED_MAX_STALE_ROUNDS,
            ),
        };
        let planning_session = state.planning_session.clone().ok_or_else(|| {
            CompilerError::InvariantViolation(
                "strict device workflow has no planning session".to_string(),
            )
        })?;
        let optimizer = NativeOptimizer::with_session(
            &target.device,
            self.two_qubit_resynthesis_config_for_state(state),
            max_rounds,
            max_stale_rounds,
            planning_session,
        )
        .with_quality_policy(NativeQualityPolicy::BalancedDepth);
        let input_is_canonical = state.has_canonical_proof(Canonicalizer::production().config());
        let (result, native_stats) = if input_is_canonical {
            optimizer.run_with_proven_canonical_input_and_stats(&state.current)?
        } else {
            optimizer.run_with_stats(&state.current)?
        };
        if result.changed {
            state.analysis = None;
            state.advance_circuit_revision();
        }
        state.current = result.circuit;
        state.changed |= result.changed;
        state.steps.push(WorkflowStepReport {
            stage: "optimization",
            name: "optimize.native_fixed_point",
            changed: result.changed,
            skipped: false,
            reason: Some(format!(
                "quality_policy=balanced_depth; rounds={}; restored_best={}; native_2q_ops={}->{}; native_2q_depth={}->{}; native_depth={}->{}; native_ops={}->{}; predicted_log_error={:?}->{:?}; unavailable_error_count={}->{}; imputed_error_count={}->{}; quality_rejections=scope_shape:{},2q_ops:{},2q_depth:{},total_depth:{},error:{},makespan:{},rank:{}",
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
                native_stats.quality_scope_shape_rejections,
                native_stats.quality_two_qubit_ops_rejections,
                native_stats.quality_two_qubit_depth_rejections,
                native_stats.quality_total_depth_rejections,
                native_stats.quality_error_rejections,
                native_stats.quality_makespan_rejections,
                native_stats.quality_rank_rejections,
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
    /// This validator is deliberately independent from SABRE target
    /// preparation. It checks usable physical qubits, undirected connectivity,
    /// and the explicit output basis without constructing a planning session or
    /// native-plan catalog.
    fn validate_topology_target(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let CompileTarget::TopologyBasis {
            device_target,
            basis,
        } = &self.config.target
        else {
            return Ok(());
        };
        let allowed = basis
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Standard(gate) => Some(*gate),
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>();
        validate_operations_in_target_basis(state.current.operations(), &allowed)?;
        validate_operations_on_topology(state.current.operations(), &device_target.device)?;
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
        let decomposer = DecomposeDefinitions;
        if state.skip_if_proven_noop("init", "decompose.definitions", &decomposer) {
            return Ok(());
        }
        state.apply_transform("init", "decompose.definitions", |circuit, analysis| {
            decomposer.transform(circuit, Some(analysis))
        })?;
        Ok(())
    }

    fn apply_unitary_decomposition(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let config = self.unitary_decompose_config_for_state(state);
        let scheduling_decomposer = DecomposeUnitaries::new(config.clone());
        if state.skip_if_proven_noop("translation", "decompose.unitary", &scheduling_decomposer) {
            return Ok(());
        }
        let decomposer = if let Some(target) = self.strict_device_target() {
            let planning_session = state.planning_session.clone().ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "strict device workflow has no planning session".to_string(),
                )
            })?;
            let context = DeviceTwoQubitSynthesisContext::build_with_session(
                &target.device,
                &state.current,
                DeviceSynthesisPlacement::PreLayoutEnvelope,
                planning_session,
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
        let decomposer = DecomposeMcGates::new(config);
        if state.skip_if_proven_noop("translation", "decompose.mc_gates", &decomposer) {
            return Ok(());
        }
        state.apply_transform("translation", "decompose.mc_gates", |circuit, analysis| {
            decomposer.transform(circuit, Some(analysis))
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
        let device_context = if let Some(target) = self
            .strict_device_target()
            .filter(|_| ResynthesizeTwoQubitBlocks::is_applicable(&state.current))
        {
            let planning_session = state.planning_session.clone().ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "strict device workflow has no planning session".to_string(),
                )
            })?;
            let context = DeviceTwoQubitSynthesisContext::build_with_session(
                &target.device,
                &state.current,
                placement,
                planning_session,
            )?;
            Some(context)
        } else {
            None
        };
        let mut session = std::mem::take(&mut state.resynthesis_session);
        let result = state.apply_transform_without_analysis_with_edits(stage, name, |circuit| {
            resynthesize_two_qubit_blocks_workflow(circuit, config, device_context, &mut session)
        });
        state.resynthesis_session = session;
        result
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

        if state.skip_if_proven_noop(
            "translation",
            "decompose.routing_basis",
            &LowerToRoutingBasis::default(),
        ) {
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
    fn validate_resources(&self, state: &mut WorkflowState) -> Result<(), CompilerError> {
        let resource_limits = self.resource_limits();
        if let Some(max_total_qubits) = resource_limits.max_total_qubits
            && state.current.num_qubits() > max_total_qubits
        {
            return Err(CompilerError::InvalidInput(format!(
                "source circuit uses {} logical qubits but target capacity is {max_total_qubits}",
                state.current.num_qubits()
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

        let device = &target.device;
        let config = sabre_config_for_mode(self.config.mode, target.seed);
        let (
            routed_circuit,
            route_edits,
            route_changed,
            swap_count,
            trials_evaluated,
            supplied_layout,
        ) = if let Some(initial_layout) = target.initial_layout.as_ref() {
            let routed = if let Some(session) = state.planning_session.as_deref() {
                let physical = state.routing_physical.as_deref().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "strict-device routing has no prepared physical graph".to_string(),
                    )
                })?;
                route_with_layout_tracked_with_session_on_physical(
                    &state.current,
                    device,
                    physical,
                    initial_layout,
                    &config,
                    session,
                )?
            } else {
                let routing = state.topology_routing_target.as_deref().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "topology routing has no prepared routing target".to_string(),
                    )
                })?;
                route_with_layout_tracked_on_topology(
                    &state.current,
                    routing,
                    initial_layout,
                    &config,
                )?
            };
            let route_changed = routed.routed().changed(&state.current);
            let swap_count = routed.routed().swap_count();
            let trials_evaluated = routed.routed().diagnostics().trials_evaluated;
            let final_layout =
                virtual_permutation.compose_final_layout(routed.routed().final_layout())?;
            state.device_metadata = Some(DeviceCompilationMetadata {
                initial_layout: routed.routed().initial_layout().clone(),
                final_layout,
                virtual_permutation: virtual_permutation.clone(),
            });
            let edits = routed.rewrite_edits(&state.current);
            (
                routed.into_routed().into_circuit(),
                edits,
                route_changed,
                swap_count,
                trials_evaluated,
                true,
            )
        } else {
            let objective = LayoutObjective::topology_only();
            let routed = if let Some(session) = state.planning_session.as_deref() {
                let physical = state.routing_physical.clone().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "strict-device routing has no prepared physical graph".to_string(),
                    )
                })?;
                route_sabre_tracked_with_session_on_physical(
                    &state.current,
                    device,
                    &objective,
                    &config,
                    physical,
                    session,
                )?
            } else {
                let physical = state.routing_physical.clone().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "topology routing has no prepared physical graph".to_string(),
                    )
                })?;
                let routing = state.topology_routing_target.clone().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "topology routing has no prepared routing target".to_string(),
                    )
                })?;
                route_sabre_tracked_on_topology(
                    &state.current,
                    &objective,
                    &config,
                    physical,
                    routing,
                )?
            };
            let route_changed = routed.routed().changed(&state.current);
            let swap_count = routed.routed().swap_count();
            let trials_evaluated = routed.routed().diagnostics().trials_evaluated;
            let final_layout =
                virtual_permutation.compose_final_layout(routed.routed().final_layout())?;
            state.device_metadata = Some(DeviceCompilationMetadata {
                initial_layout: routed.routed().initial_layout().clone(),
                final_layout,
                virtual_permutation: virtual_permutation.clone(),
            });
            let edits = routed.rewrite_edits(&state.current);
            (
                routed.into_routed().into_circuit(),
                edits,
                route_changed,
                swap_count,
                trials_evaluated,
                false,
            )
        };
        if route_changed {
            state.adopt_changed_circuit(routed_circuit, &route_edits);
        } else {
            state.current = routed_circuit;
        }

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
                let circuit = elision.into_changed_circuit().ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "changed virtual-permutation result did not contain a circuit".to_string(),
                    )
                })?;
                let edits = RewriteEdits::between_linear_circuits(&state.current, &circuit);
                state.adopt_changed_circuit(circuit, &edits);
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
        state.apply_knowledge_rewrite("optimization", "optimize.post_routing", rewrite_config)?;
        Ok(())
    }

    fn apply_commutative_cancellation(
        &self,
        state: &mut WorkflowState,
        name: &'static str,
    ) -> Result<bool, CompilerError> {
        state.apply_transform_without_analysis_with_edits("optimization", name, |circuit| {
            CommutativeCancellation::new().transform_with_rewrite_edits(circuit)
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
        let result =
            state.apply_transform_without_analysis_with_edits("optimization", name, |circuit| {
                optimizer.transform_with_rewrite_edits(circuit)
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
        let before_revision = state.circuit_revision;
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
            changed: state.circuit_revision != before_revision,
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

        if state.skip_if_proven_noop("translation", name, lowerer.as_ref()) {
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

    fn prepared_target(&self) -> Result<&PreparedCompileTarget, CompilerError> {
        if let Some(prepared) = self.prepared_target.get() {
            return Ok(prepared);
        }
        let _guard = self
            .prepare_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(prepared) = self.prepared_target.get() {
            return Ok(prepared);
        }
        let prepared = Arc::new(PreparedCompileTarget::new(&self.config)?);
        self.prepared_target.set(prepared).map_err(|_| {
            CompilerError::InvariantViolation(
                "compiler target was initialized concurrently while holding its preparation lock"
                    .to_string(),
            )
        })?;
        self.prepared_target.get().map(Arc::as_ref).ok_or_else(|| {
            CompilerError::InvariantViolation(
                "compiler target preparation completed without publishing its result".to_string(),
            )
        })
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

fn validate_operations_on_topology(
    operations: &[Operation],
    device: &crate::device::Device,
) -> Result<(), CompilerError> {
    use crate::device::PhysicalQubit;

    for operation in operations {
        for qubit in operation.qubits.iter().copied() {
            let physical = PhysicalQubit::from_qubit(qubit);
            if !device.is_usable_qubit(physical) {
                return Err(CompilerError::InvariantViolation(format!(
                    "topology-basis output references unusable physical qubit {physical}"
                )));
            }
        }
        match &operation.instruction {
            Instruction::Standard(gate) if gate.num_qubits() == 2 => {
                let [left, right] = operation.qubits.as_slice() else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "two-qubit gate {gate:?} has {} physical arguments",
                        operation.qubits.len()
                    )));
                };
                let left = PhysicalQubit::from_qubit(*left);
                let right = PhysicalQubit::from_qubit(*right);
                if !device
                    .topology()
                    .supports_coupling_either_direction(left, right)
                {
                    return Err(CompilerError::InvariantViolation(format!(
                        "topology-basis output gate {gate:?} uses non-adjacent physical qubits {left} and {right}"
                    )));
                }
            }
            Instruction::Standard(gate) if gate.num_qubits() > 2 => {
                return Err(CompilerError::InvariantViolation(format!(
                    "topology-basis output contains undecomposed gate {gate:?}"
                )));
            }
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    validate_operations_on_topology(op.then_body().operations(), device)?;
                    if let Some(body) = op.else_body() {
                        validate_operations_on_topology(body.operations(), device)?;
                    }
                }
                ClassicalControlOp::While(op) => {
                    validate_operations_on_topology(op.body().operations(), device)?;
                }
                ClassicalControlOp::For(op) => {
                    validate_operations_on_topology(op.body().operations(), device)?;
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        validate_operations_on_topology(case.body().operations(), device)?;
                    }
                    if let Some(body) = op.default() {
                        validate_operations_on_topology(body.operations(), device)?;
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            Instruction::McGate(_) | Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                return Err(CompilerError::InvariantViolation(format!(
                    "topology-basis output contains undecomposed instruction {}",
                    operation.instruction
                )));
            }
            Instruction::Standard(_)
            | Instruction::Directive(_)
            | Instruction::ClassicalData(_)
            | Instruction::Delay => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "./workflow_test.rs"]
mod workflow_test;
