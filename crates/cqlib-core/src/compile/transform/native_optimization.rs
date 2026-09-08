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

//! Exact physical optimization after device instruction lowering.
//!
//! [`NativeOptimizer`] closes a bounded loop over independent stage branches.
//! Every round evaluates `baseline -> A`, `baseline -> B -> C`, and
//! `A -> B -> C`, where A/C are local one-qubit optimization and B is
//! exact-physical two-qubit resynthesis. Each legalized stage is a returnable
//! quality checkpoint. A non-improving intermediate may feed its dependent
//! stage inside one branch, but it cannot replace the round cursor or suppress
//! a sibling branch. Every transform recursively visits structured
//! control-flow bodies.
//!
//! Candidate circuits are accepted under a Native-only quality policy. The
//! balanced policy first protects the immutable entry circuit's entangler
//! count/depth, total depth, calibrated error, and makespan in every
//! control-flow scope; candidates inside that envelope are then ranked against
//! the best returnable circuit. This avoids assigning arbitrary execution
//! counts to conditional or loop bodies. The optimizer restores the best whole
//! circuit seen; independently optimal control-flow bodies are never spliced
//! together.
//!
//! One immutable exact-physical synthesis context is shared by resynthesis,
//! local optimization, and scope costing for the lifetime of a single run. The
//! catalog is rebuilt transactionally only when scope costing finds an
//! unprepared root; a prepared-but-unsupported root remains a real failure.

use crate::circuit::{
    Circuit, ClassicalControlOp, Directive, Instruction, Operation, Parameter, ParameterValue,
    Qubit, StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction,
    ValueOperation, ValueSwitchCase,
};
use crate::compile::CompilerError;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::device_planning::cost::{RobustDurationKey, RobustErrorKey};
use crate::compile::sabre::MetricAvailability;
use crate::compile::transform::decompose::unitary::{
    DeviceContextCostFailure, DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext,
    OneQubitUnitaryDecomposition, synthesize_numeric_1q_unitary,
};
use crate::compile::transform::native_quality::{
    NativeQualityPolicy, NativeQualityVector, NativeQualityViolation,
};
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::resynthesis::{
    NativeResynthesisPolicy, NativeResynthesisSession, NativeWorksetStats,
    TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks_incremental,
};
use crate::compile::transform::target_basis::{TargetBasisCost, TargetBasisCostModel};
use crate::compile::transform::{
    Canonicalizer, CircuitAnalysis, DeviceLowerer, RewriteEdits, TransformOutcome, Transformer,
};
use crate::device::Device;
use ndarray::Array2;
use num_complex::Complex64;
use smallvec::{SmallVec, smallvec};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use std::sync::Arc;

const PHASE_EPS: f64 = 1e-12;

struct NativeStageCandidate {
    circuit: Arc<Circuit>,
    costs: Vec<NativeQualityVector>,
    context: DeviceTwoQubitSynthesisContext,
    improves_best: bool,
}

enum NativeStageOutcome {
    Unchanged,
    Unavailable,
    Candidate(NativeStageCandidate),
}

struct NativeBestCheckpoint<'a> {
    circuit: &'a mut Arc<Circuit>,
    costs: &'a mut Vec<NativeQualityVector>,
    context: &'a mut DeviceTwoQubitSynthesisContext,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeOptimizationSummary {
    /// Number of native two-qubit operations across all control-flow scopes.
    pub native_two_qubit_ops: u64,
    /// Sum of native two-qubit depth across all control-flow scopes.
    pub native_two_qubit_depth: u64,
    /// Sum of total native depth across all control-flow scopes.
    pub total_native_depth: u64,
    /// Number of native operations across all control-flow scopes.
    pub native_total_ops: u64,
    /// Summed predicted log error, or `None` when calibration is unavailable.
    pub predicted_log_error: Option<f64>,
    /// Number of operations whose error metric was unavailable.
    pub unavailable_error_count: u64,
    /// Number of operations whose error metric was imputed.
    pub imputed_error_count: u64,
}

/// Exact final quality for every control-flow scope in deterministic traversal
/// order. Aggregate summaries remain useful diagnostics, but only this
/// checkpoint is strong enough to authorize workflow-level candidate
/// replacement.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeExactQualityCheckpoint {
    scopes: Vec<NativeQualityVector>,
}

impl NativeExactQualityCheckpoint {
    fn new(scopes: Vec<NativeQualityVector>) -> Self {
        Self { scopes }
    }

    /// Returns whether this checkpoint strictly Pareto-dominates `incumbent`
    /// in every corresponding control-flow scope.
    pub(crate) fn strictly_dominates(&self, incumbent: &Self) -> bool {
        if self.scopes.len() != incumbent.scopes.len() {
            return false;
        }
        let mut strict = false;
        for (candidate, incumbent) in self.scopes.iter().zip(&incumbent.scopes) {
            let Some(scope_strict) = candidate.exact_pareto_dominance(*incumbent) else {
                return false;
            };
            strict |= scope_strict;
        }
        strict
    }

    /// Requires the full exact Pareto contract and a strict entangler-count or
    /// entangler-depth improvement in at least one corresponding scope.
    pub(crate) fn strictly_dominates_with_two_qubit_gain(&self, incumbent: &Self) -> bool {
        if !self.strictly_dominates(incumbent) {
            return false;
        }
        self.scopes
            .iter()
            .zip(&incumbent.scopes)
            .any(|(candidate, incumbent)| {
                candidate.physical.native_two_qubit_ops < incumbent.physical.native_two_qubit_ops
                    || candidate.physical.native_two_qubit_depth
                        < incumbent.physical.native_two_qubit_depth
            })
    }

    /// Deterministic final SABRE-beam ordering. Lower is better.
    pub(crate) fn compare_for_sabre_beam(&self, other: &Self) -> Ordering {
        let left = self.summary();
        let right = other.summary();
        left.native_two_qubit_ops
            .cmp(&right.native_two_qubit_ops)
            .then_with(|| {
                left.native_two_qubit_depth
                    .cmp(&right.native_two_qubit_depth)
            })
            .then_with(|| left.total_native_depth.cmp(&right.total_native_depth))
            .then_with(|| left.native_total_ops.cmp(&right.native_total_ops))
            .then_with(|| {
                aggregate_error(&self.scopes)
                    .compare_by(aggregate_error(&other.scopes), RobustErrorKey::compare)
            })
            .then_with(|| {
                aggregate_duration(&self.scopes).compare_by(
                    aggregate_duration(&other.scopes),
                    RobustDurationKey::compare,
                )
            })
            .then_with(|| {
                aggregate_makespan(&self.scopes)
                    .compare_by(aggregate_makespan(&other.scopes), |left, right| {
                        left.total_cmp(&right)
                    })
            })
    }

    pub(crate) fn summary(&self) -> NativeOptimizationSummary {
        summarize_scope_costs(&self.scopes)
    }
}

fn aggregate_error(scopes: &[NativeQualityVector]) -> MetricAvailability<RobustErrorKey> {
    aggregate_optional_metric(
        scopes.iter().map(|quality| quality.physical.error),
        |left, right| left.combine(right),
    )
}

fn aggregate_duration(scopes: &[NativeQualityVector]) -> MetricAvailability<RobustDurationKey> {
    aggregate_optional_metric(
        scopes.iter().map(|quality| quality.physical.duration),
        |left, right| left.combine(right),
    )
}

fn aggregate_makespan(scopes: &[NativeQualityVector]) -> MetricAvailability<f64> {
    aggregate_optional_metric(
        scopes.iter().map(|quality| quality.physical.makespan),
        |left, right| left + right,
    )
}

fn aggregate_optional_metric<T: Copy>(
    metrics: impl IntoIterator<Item = MetricAvailability<T>>,
    combine: impl Fn(T, T) -> T,
) -> MetricAvailability<T> {
    let mut aggregate = None;
    for metric in metrics {
        aggregate = Some(match (aggregate, metric) {
            (None, metric) => metric,
            (Some(MetricAvailability::Disabled), MetricAvailability::Disabled) => {
                MetricAvailability::Disabled
            }
            (Some(MetricAvailability::Available(left)), MetricAvailability::Available(right)) => {
                MetricAvailability::Available(combine(left, right))
            }
            (Some(MetricAvailability::Inconsistent), _)
            | (_, MetricAvailability::Inconsistent)
            | (Some(MetricAvailability::Disabled), MetricAvailability::Available(_))
            | (Some(MetricAvailability::Available(_)), MetricAvailability::Disabled) => {
                MetricAvailability::Inconsistent
            }
        });
    }
    aggregate.unwrap_or(MetricAvailability::Disabled)
}

/// Result of one bounded exact-physical native optimization run.
#[derive(Debug, Clone)]
pub struct NativeOptimizationResult {
    /// Best whole-circuit point accepted by the optimizer.
    pub circuit: Circuit,
    /// Whether the returned circuit differs from the supplied input.
    pub changed: bool,
    /// Number of optimization rounds entered, including a terminal stable round.
    pub rounds: u8,
    /// Whether the last explored state was discarded in favor of an earlier
    /// best returnable point.
    pub restored_best: bool,
    /// Exact physical cost summary at optimizer entry.
    pub before: NativeOptimizationSummary,
    /// Exact physical cost summary for the returned circuit.
    pub after: NativeOptimizationSummary,
}

/// Bounded native optimization loop with minimum-point restoration.
///
/// Inputs must already be routed and lowered to exact native instructions for
/// `device`. Most callers should use the compiler workflow; this lower-level
/// entry point is intended for diagnostics and custom physical pipelines.
pub struct NativeOptimizer<'a> {
    device: Cow<'a, Device>,
    planning_session: Arc<DevicePlanningSession>,
    resynthesis: TwoQubitBlockResynthesisConfig,
    max_rounds: u8,
    max_stale_rounds: u8,
    quality_policy: NativeQualityPolicy,
}

impl<'a> NativeOptimizer<'a> {
    /// Production native-loop budget used by normal compilation.
    pub const NORMAL_MAX_ROUNDS: u8 = 2;
    /// Production stale-round budget used by normal compilation.
    pub const NORMAL_MAX_STALE_ROUNDS: u8 = 1;
    /// Production native-loop budget used by enhanced compilation.
    pub const ENHANCED_MAX_ROUNDS: u8 = 8;
    /// Production stale-round budget used by enhanced compilation.
    pub const ENHANCED_MAX_STALE_ROUNDS: u8 = 3;

    /// Creates a reusable optimizer borrowing `device`.
    pub fn new(
        device: &'a Device,
        resynthesis: TwoQubitBlockResynthesisConfig,
        max_rounds: u8,
        max_stale_rounds: u8,
    ) -> Result<Self, CompilerError> {
        validate_native_optimization_budgets(max_rounds, max_stale_rounds)?;
        Ok(Self {
            device: Cow::Borrowed(device),
            planning_session: Arc::new(DevicePlanningSession::new(device)),
            resynthesis,
            max_rounds,
            max_stale_rounds,
            quality_policy: NativeQualityPolicy::EntanglerFirst,
        })
    }

    /// Creates a reusable optimizer owning an immutable device snapshot.
    ///
    /// This is useful for language bindings and long-lived optimizer objects:
    /// repeated runs reuse the same run-local planning cache without requiring
    /// a self-referential wrapper.
    pub fn new_owned(
        device: Device,
        resynthesis: TwoQubitBlockResynthesisConfig,
        max_rounds: u8,
        max_stale_rounds: u8,
    ) -> Result<NativeOptimizer<'static>, CompilerError> {
        validate_native_optimization_budgets(max_rounds, max_stale_rounds)?;
        let planning_session = Arc::new(DevicePlanningSession::new(&device));
        Ok(NativeOptimizer {
            device: Cow::Owned(device),
            planning_session,
            resynthesis,
            max_rounds,
            max_stale_rounds,
            quality_policy: NativeQualityPolicy::EntanglerFirst,
        })
    }

    /// Creates an optimizer with the exact budget used by normal compilation.
    pub fn normal(device: &'a Device) -> Self {
        Self::new(
            device,
            TwoQubitBlockResynthesisConfig::normal(Default::default()),
            Self::NORMAL_MAX_ROUNDS,
            Self::NORMAL_MAX_STALE_ROUNDS,
        )
        .expect("production native optimization budgets must be valid")
    }

    /// Creates an optimizer with the exact budget used by enhanced compilation.
    pub fn enhanced(device: &'a Device) -> Self {
        Self::new(
            device,
            TwoQubitBlockResynthesisConfig::enhanced(Default::default()),
            Self::ENHANCED_MAX_ROUNDS,
            Self::ENHANCED_MAX_STALE_ROUNDS,
        )
        .expect("production native optimization budgets must be valid")
    }

    /// Device snapshot costed and validated by this optimizer.
    pub fn device(&self) -> &Device {
        self.device.as_ref()
    }

    /// Two-qubit resynthesis configuration used in every round.
    pub fn resynthesis(&self) -> &TwoQubitBlockResynthesisConfig {
        &self.resynthesis
    }

    /// Maximum number of rounds entered by one run.
    pub const fn max_rounds(&self) -> u8 {
        self.max_rounds
    }

    /// Number of consecutive non-improving rounds allowed before stopping.
    pub const fn max_stale_rounds(&self) -> u8 {
        self.max_stale_rounds
    }

    /// Candidate quality policy used by this optimizer.
    pub const fn quality_policy(&self) -> NativeQualityPolicy {
        self.quality_policy
    }

    /// Selects the Native-only candidate quality policy.
    pub fn with_quality_policy(mut self, quality_policy: NativeQualityPolicy) -> Self {
        self.quality_policy = quality_policy;
        self
    }

    pub(crate) fn with_session(
        device: &'a Device,
        resynthesis: TwoQubitBlockResynthesisConfig,
        max_rounds: u8,
        max_stale_rounds: u8,
        planning_session: Arc<DevicePlanningSession>,
    ) -> Self {
        debug_assert!(max_rounds > 0);
        debug_assert!(max_stale_rounds > 0);
        Self {
            device: Cow::Borrowed(device),
            planning_session,
            resynthesis,
            max_rounds,
            max_stale_rounds,
            quality_policy: NativeQualityPolicy::EntanglerFirst,
        }
    }

    pub fn run(&self, circuit: &Circuit) -> Result<NativeOptimizationResult, CompilerError> {
        self.run_with_policy(circuit, NativeResynthesisPolicy::Incremental)
            .map(|(result, _)| result)
    }

    pub(crate) fn run_with_exact_quality_and_stats(
        &self,
        circuit: &Circuit,
    ) -> Result<
        (
            NativeOptimizationResult,
            NativeExactQualityCheckpoint,
            NativeWorksetStats,
        ),
        CompilerError,
    > {
        let initial = Canonicalizer::production()
            .transform(circuit, None)?
            .into_circuit(circuit);
        self.run_canonicalized(circuit, initial, NativeResynthesisPolicy::Incremental)
    }

    pub(crate) fn run_with_proven_canonical_input_exact_quality_and_stats(
        &self,
        circuit: &Circuit,
    ) -> Result<
        (
            NativeOptimizationResult,
            NativeExactQualityCheckpoint,
            NativeWorksetStats,
        ),
        CompilerError,
    > {
        self.run_canonicalized(
            circuit,
            circuit.clone(),
            NativeResynthesisPolicy::Incremental,
        )
    }

    pub(crate) fn run_with_policy(
        &self,
        circuit: &Circuit,
        policy: NativeResynthesisPolicy,
    ) -> Result<(NativeOptimizationResult, NativeWorksetStats), CompilerError> {
        let initial = Canonicalizer::production()
            .transform(circuit, None)?
            .into_circuit(circuit);
        self.run_canonicalized(circuit, initial, policy)
            .map(|(result, _, stats)| (result, stats))
    }

    /// Runs from an input whose production-canonical postcondition has already
    /// been established by this optimizer or its workflow caller.
    fn run_canonicalized(
        &self,
        source: &Circuit,
        initial: Circuit,
        policy: NativeResynthesisPolicy,
    ) -> Result<
        (
            NativeOptimizationResult,
            NativeExactQualityCheckpoint,
            NativeWorksetStats,
        ),
        CompilerError,
    > {
        self.device().validate_circuit(&initial)?;
        // Native rounds can carry very large circuits. Share immutable states
        // between the exploration cursor, the best returnable checkpoint, and
        // the exact round-boundary cycle detector.
        let initial = Arc::new(initial);
        let mut current = initial.clone();
        let mut best = initial;
        // This exact-physical context is immutable and run-scoped. All consumers
        // on the exploration path share its Arc-backed catalog until a candidate
        // exposes missing coverage.
        let mut context = DeviceTwoQubitSynthesisContext::build_with_session(
            self.device(),
            &current,
            DeviceSynthesisPlacement::ExactPhysical,
            Arc::clone(&self.planning_session),
        )?;
        let mut best_context = context.clone();
        let mut best_costs = scope_costs_with_context(&best, &context).map_err(scope_cost_error)?;
        let entry_costs = best_costs.clone();
        let before = summarize_scope_costs(&best_costs);
        let mut rounds = 0;
        let mut stale = 0_u8;
        // Incremental operation identity is branch-local. Exact synthesis
        // artifacts remain run-scoped and are moved between these sessions
        // immediately around the A->B call.
        let mut baseline_session = NativeResynthesisSession::new(policy);
        let mut after_local_session = NativeResynthesisSession::new(policy);
        let mut seen_states = vec![current.clone()];

        'optimization: while rounds < self.max_rounds && stale < self.max_stale_rounds {
            rounds += 1;
            let round_start = current.clone();
            let round_context = context.clone();
            let mut improved_in_round = false;

            // A is generated from the immutable round baseline. Its legal
            // result remains available to the dependent A->B->C branch even
            // when the standalone A checkpoint is rejected.
            let phase_a_branch = match OptimizeNativeLocalGates::with_quality_policy(
                round_context.clone(),
                self.quality_policy,
            )
            .transform(&round_start, None)?
            {
                TransformOutcome::Unchanged => None,
                TransformOutcome::Changed(candidate) => match self.evaluate_stage_candidate(
                    &round_start,
                    Arc::new(candidate),
                    &best_costs,
                    &entry_costs,
                    &round_context,
                    &mut after_local_session,
                )? {
                    NativeStageOutcome::Candidate(candidate) => {
                        let branch = (candidate.circuit.clone(), candidate.context.clone());
                        improved_in_round |= install_best_checkpoint(
                            &candidate,
                            &mut best,
                            &mut best_costs,
                            &mut best_context,
                        );
                        Some(branch)
                    }
                    NativeStageOutcome::Unchanged | NativeStageOutcome::Unavailable => None,
                },
            };

            // B/C from the immutable baseline is a sibling search path. It is
            // deliberately evaluated even when A produced a legal candidate.
            improved_in_round |= self.explore_resynthesis_branch(
                &round_start,
                &round_context,
                NativeBestCheckpoint {
                    circuit: &mut best,
                    costs: &mut best_costs,
                    context: &mut best_context,
                },
                &entry_costs,
                &mut baseline_session,
            )?;

            if let Some((phase_a, phase_a_context)) = phase_a_branch {
                // Only exact synthesis artifacts cross the branch boundary;
                // incremental operation IDs, diffs, and worksets stay local.
                baseline_session.swap_synthesis_cache(&mut after_local_session);
                let branch_result = self.explore_resynthesis_branch(
                    &phase_a,
                    &phase_a_context,
                    NativeBestCheckpoint {
                        circuit: &mut best,
                        costs: &mut best_costs,
                        context: &mut best_context,
                    },
                    &entry_costs,
                    &mut after_local_session,
                );
                baseline_session.swap_synthesis_cache(&mut after_local_session);
                improved_in_round |= branch_result?;
            }

            // Only a quality-accepted whole-circuit checkpoint may seed the
            // next round. Rejected A/B/C states have already served their
            // bounded dependent edge and cannot contaminate later rounds.
            current = best.clone();
            context = best_context.clone();

            if current.as_ref() == round_start.as_ref() {
                break;
            }

            // Only round-boundary exploration states participate in cycle
            // detection. The same circuit at different A/B/C phase positions
            // does not imply the same remaining deterministic transition.
            if let Some(seen) = seen_states
                .iter()
                .find(|seen| seen.as_ref() == current.as_ref())
            {
                current = seen.clone();
                baseline_session.record_cycle_early_exit();
                break 'optimization;
            }
            seen_states.push(current.clone());

            if improved_in_round {
                stale = 0;
            } else {
                stale = stale.saturating_add(1);
            }
        }

        let restored_best = current.as_ref() != best.as_ref();
        let after = summarize_scope_costs(&best_costs);
        let exact_quality = NativeExactQualityCheckpoint::new(best_costs);
        drop(current);
        drop(seen_states);
        let best = Arc::try_unwrap(best).unwrap_or_else(|shared| shared.as_ref().clone());
        let result = NativeOptimizationResult {
            changed: best != *source,
            circuit: best,
            rounds,
            restored_best,
            before,
            after,
        };
        let mut stats = baseline_session.stats();
        stats.merge_workset_from(after_local_session.stats());
        Ok((result, exact_quality, stats))
    }

    /// Evaluates one detached `B -> C` branch. B and C are separate returnable
    /// checkpoints, while raw B remains available to C even when B itself is
    /// not lowerable or does not improve the whole-circuit quality point.
    fn explore_resynthesis_branch(
        &self,
        branch_start: &Arc<Circuit>,
        branch_context: &DeviceTwoQubitSynthesisContext,
        best: NativeBestCheckpoint<'_>,
        entry_costs: &[NativeQualityVector],
        session: &mut NativeResynthesisSession,
    ) -> Result<bool, CompilerError> {
        let resynthesis_outcome = resynthesize_two_qubit_blocks_incremental(
            branch_start,
            self.resynthesis.clone(),
            branch_context.clone(),
            session,
            self.quality_policy,
        )?;
        let TransformOutcome::Changed(raw_phase_b) = resynthesis_outcome else {
            return Ok(false);
        };
        let raw_phase_b = Arc::new(raw_phase_b);
        let phase_b_outcome = self.evaluate_stage_candidate(
            branch_start,
            raw_phase_b.clone(),
            best.costs,
            entry_costs,
            branch_context,
            session,
        )?;
        let cleanup_context = match phase_b_outcome {
            NativeStageOutcome::Candidate(candidate) => {
                let context = candidate.context.clone();
                let improved =
                    install_best_checkpoint(&candidate, best.circuit, best.costs, best.context);
                (context, improved)
            }
            NativeStageOutcome::Unchanged | NativeStageOutcome::Unavailable => {
                (branch_context.clone(), false)
            }
        };
        let (cleanup_context, mut improved) = cleanup_context;

        // C follows generated B, not accepted B. This preserves the useful
        // coupled edge without binding B's acceptance to C's acceptance.
        if let TransformOutcome::Changed(cleaned) =
            OptimizeNativeLocalGates::with_quality_policy(cleanup_context, self.quality_policy)
                .transform(&raw_phase_b, None)?
            && let NativeStageOutcome::Candidate(candidate) = self.evaluate_stage_candidate(
                branch_start,
                Arc::new(cleaned),
                best.costs,
                entry_costs,
                branch_context,
                session,
            )?
        {
            improved |= install_best_checkpoint(&candidate, best.circuit, best.costs, best.context);
        }
        Ok(improved)
    }

    /// Legalizes, canonicalizes, validates, and scores one detached stage
    /// candidate. A quality failure prevents only installation as `best`; the
    /// legal candidate and its costing context remain available for bounded
    /// exploration by a dependent stage.
    fn evaluate_stage_candidate(
        &self,
        baseline: &Arc<Circuit>,
        generated: Arc<Circuit>,
        best_costs: &[NativeQualityVector],
        entry_costs: &[NativeQualityVector],
        context: &DeviceTwoQubitSynthesisContext,
        resynthesis_session: &mut NativeResynthesisSession,
    ) -> Result<NativeStageOutcome, CompilerError> {
        let legalized = match DeviceLowerer::with_session(self.device(), &self.planning_session)
            .transform(&generated, None)
        {
            Ok(TransformOutcome::Unchanged) => generated,
            Ok(TransformOutcome::Changed(legalized)) => Arc::new(legalized),
            // A standalone checkpoint may be unavailable even though a later
            // cleanup of the raw generated circuit can still be lowerable.
            Err(CompilerError::DeviceLoweringFailed(_)) => {
                return Ok(NativeStageOutcome::Unavailable);
            }
            Err(error) => return Err(error),
        };
        let candidate = match Canonicalizer::production().transform(&legalized, None)? {
            TransformOutcome::Unchanged => legalized,
            TransformOutcome::Changed(candidate) => Arc::new(candidate),
        };
        if candidate.as_ref() == baseline.as_ref() {
            return Ok(NativeStageOutcome::Unchanged);
        }

        // A non-improving candidate may become the next exploration state, so
        // validation is mandatory before returning it in every build profile.
        self.device().validate_circuit(&candidate)?;
        let mut candidate_context = context.clone();
        let candidate_costs =
            self.candidate_costs_with_reuse(&candidate, &mut candidate_context)?;
        let improves_best = match scope_quality_decision(
            &candidate_costs,
            best_costs,
            entry_costs,
            self.quality_policy,
        ) {
            Ok(()) => true,
            Err(violation) => {
                resynthesis_session.record_quality_rejection(violation);
                false
            }
        };
        Ok(NativeStageOutcome::Candidate(NativeStageCandidate {
            circuit: candidate,
            costs: candidate_costs,
            context: candidate_context,
            improves_best,
        }))
    }

    /// Costs a candidate with the run-scoped context, rebuilding transactionally
    /// at most once when (and only when) the catalog did not prepare a required root.
    fn candidate_costs_with_reuse(
        &self,
        candidate: &Circuit,
        context: &mut DeviceTwoQubitSynthesisContext,
    ) -> Result<Vec<NativeQualityVector>, CompilerError> {
        match scope_costs_with_context(candidate, context) {
            Ok(costs) => Ok(costs),
            Err(ScopeCostError::Context(DeviceContextCostFailure::Unprepared(_))) => {
                let rebuilt = DeviceTwoQubitSynthesisContext::build_with_session(
                    self.device(),
                    candidate,
                    DeviceSynthesisPlacement::ExactPhysical,
                    Arc::clone(&self.planning_session),
                )?;
                let costs = match scope_costs_with_context(candidate, &rebuilt) {
                    Ok(costs) => costs,
                    Err(ScopeCostError::Context(DeviceContextCostFailure::Unprepared(state))) => {
                        return Err(CompilerError::InvariantViolation(format!(
                            "rebuilt native optimizer context was not prepared for {state:?}"
                        )));
                    }
                    Err(error) => return Err(scope_cost_error(error)),
                };
                *context = rebuilt;
                Ok(costs)
            }
            Err(error) => Err(scope_cost_error(error)),
        }
    }
}

fn install_best_checkpoint(
    candidate: &NativeStageCandidate,
    best: &mut Arc<Circuit>,
    best_costs: &mut Vec<NativeQualityVector>,
    best_context: &mut DeviceTwoQubitSynthesisContext,
) -> bool {
    if !candidate.improves_best {
        return false;
    }
    *best = candidate.circuit.clone();
    best_costs.clone_from(&candidate.costs);
    *best_context = candidate.context.clone();
    true
}

fn validate_native_optimization_budgets(
    max_rounds: u8,
    max_stale_rounds: u8,
) -> Result<(), CompilerError> {
    if max_rounds == 0 {
        return Err(CompilerError::InvalidInput(
            "native optimizer max_rounds must be greater than zero".to_string(),
        ));
    }
    if max_stale_rounds == 0 {
        return Err(CompilerError::InvalidInput(
            "native optimizer max_stale_rounds must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
enum ScopeCostError {
    Compiler(CompilerError),
    Context(DeviceContextCostFailure),
}

impl From<CompilerError> for ScopeCostError {
    fn from(error: CompilerError) -> Self {
        Self::Compiler(error)
    }
}

fn scope_cost_error(error: ScopeCostError) -> CompilerError {
    match error {
        ScopeCostError::Compiler(error) => error,
        ScopeCostError::Context(_) => CompilerError::InvariantViolation(
            "native optimizer could not cost a legalized control-flow scope".to_string(),
        ),
    }
}

fn scope_costs_with_context(
    circuit: &Circuit,
    context: &DeviceTwoQubitSynthesisContext,
) -> Result<Vec<NativeQualityVector>, ScopeCostError> {
    let mut costs = Vec::new();
    collect_scope_costs(circuit.operations(), context, &mut costs)?;
    Ok(costs)
}

fn collect_scope_costs(
    operations: &[Operation],
    context: &DeviceTwoQubitSynthesisContext,
    output: &mut Vec<NativeQualityVector>,
) -> Result<(), ScopeCostError> {
    let mut accumulator = context
        .exact_sequence_cost_accumulator()
        .map_err(ScopeCostError::Context)?;
    for operation in operations {
        match &operation.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => accumulator
                .add_gate(&operation.instruction, &operation.qubits)
                .map_err(ScopeCostError::Context)?,
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    collect_scope_costs(op.then_body().operations(), context, output)?;
                    if let Some(body) = op.else_body() {
                        collect_scope_costs(body.operations(), context, output)?;
                    }
                }
                ClassicalControlOp::While(op) => {
                    collect_scope_costs(op.body().operations(), context, output)?;
                }
                ClassicalControlOp::For(op) => {
                    collect_scope_costs(op.body().operations(), context, output)?;
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        collect_scope_costs(case.body().operations(), context, output)?;
                    }
                    if let Some(body) = op.default() {
                        collect_scope_costs(body.operations(), context, output)?;
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
            | Instruction::ClassicalData(_)
            | Instruction::Directive(_)
            | Instruction::Delay => {}
        }
    }
    output.push(NativeQualityVector::for_operations(
        accumulator.finish(),
        operations,
    ));
    Ok(())
}

fn scope_quality_decision(
    candidate: &[NativeQualityVector],
    current: &[NativeQualityVector],
    entry: &[NativeQualityVector],
    policy: NativeQualityPolicy,
) -> Result<(), NativeQualityViolation> {
    if candidate.len() != current.len() || candidate.len() != entry.len() {
        return Err(NativeQualityViolation::ScopeShape);
    }
    let mut improved = false;
    for ((candidate, current), entry) in candidate.iter().zip(current).zip(entry) {
        if let Some(violation) = candidate.admissibility_violation_against(*entry, policy) {
            return Err(violation);
        }
        match candidate.compare(*current, policy) {
            std::cmp::Ordering::Less => improved = true,
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => return Err(NativeQualityViolation::Rank),
        }
    }
    if improved {
        Ok(())
    } else {
        Err(NativeQualityViolation::Rank)
    }
}

fn summarize_scope_costs(costs: &[NativeQualityVector]) -> NativeOptimizationSummary {
    let mut predicted_log_error = Some(0.0);
    let mut unavailable_error_count = 0;
    let mut imputed_error_count = 0;
    for quality in costs {
        let cost = quality.physical;
        match cost.error {
            MetricAvailability::Available(error) => {
                if let Some(total) = &mut predicted_log_error {
                    *total += error.log_error;
                }
                unavailable_error_count += u64::from(error.unavailable_count);
                imputed_error_count += u64::from(error.imputed_count);
            }
            MetricAvailability::Disabled | MetricAvailability::Inconsistent => {
                predicted_log_error = None;
            }
        }
    }
    NativeOptimizationSummary {
        native_two_qubit_ops: costs
            .iter()
            .map(|quality| u64::from(quality.physical.native_two_qubit_ops))
            .sum(),
        native_two_qubit_depth: costs
            .iter()
            .map(|quality| u64::from(quality.physical.native_two_qubit_depth))
            .sum(),
        total_native_depth: costs
            .iter()
            .map(|quality| u64::from(quality.physical.total_native_depth))
            .sum(),
        native_total_ops: costs
            .iter()
            .map(|quality| u64::from(quality.physical.native_total_ops))
            .sum(),
        predicted_log_error,
        unavailable_error_count,
        imputed_error_count,
    }
}

/// Performs target-costed one-qubit fusion and exact frame propagation.
#[derive(Debug, Clone)]
pub(crate) struct OptimizeNativeLocalGates {
    device_context: DeviceTwoQubitSynthesisContext,
    quality_policy: NativeQualityPolicy,
}

impl OptimizeNativeLocalGates {
    pub(crate) fn with_quality_policy(
        device_context: DeviceTwoQubitSynthesisContext,
        quality_policy: NativeQualityPolicy,
    ) -> Self {
        Self {
            device_context,
            quality_policy,
        }
    }
}

impl Transformer for OptimizeNativeLocalGates {
    fn name(&self) -> &'static str {
        "optimize.native_local_gates"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let policy = LocalOptimizationPolicy::Device {
            context: self.device_context.clone(),
            quality_policy: self.quality_policy,
        };
        LocalOneQPass::run(circuit, &policy)
    }
}

/// Cost policy used by the shared one-qubit/frame optimization engine.
#[derive(Debug, Clone)]
pub(crate) enum LocalOptimizationPolicy {
    Logical,
    Basis(Arc<TargetBasisCostModel>),
    Device {
        context: DeviceTwoQubitSynthesisContext,
        quality_policy: NativeQualityPolicy,
    },
}

/// Runs the shared one-qubit/frame optimizer with an explicit cost policy.
pub(crate) fn optimize_one_qubit_runs_with_policy(
    circuit: &Circuit,
    policy: &LocalOptimizationPolicy,
) -> Result<TransformOutcome, CompilerError> {
    LocalOneQPass::run(circuit, policy)
}

pub(crate) fn optimize_one_qubit_runs_with_policy_and_edits(
    circuit: &Circuit,
    policy: &LocalOptimizationPolicy,
) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
    LocalOneQPass::run_with_rewrite_edits(circuit, policy)
}

struct LocalOneQPass<'source, 'policy> {
    source: &'source Circuit,
    policy: &'policy LocalOptimizationPolicy,
    rebuild: CircuitRebuildContext,
}

struct SequenceRewrite {
    operations: Vec<ValueOperation>,
    /// Output-to-input correspondence for the current sequence. Rebuilt or
    /// generated operations are `None`; unchanged operations retain their
    /// source order.
    provenance: Option<Vec<Option<usize>>>,
    phase_delta: f64,
    changed: bool,
}

impl<'source, 'policy> LocalOneQPass<'source, 'policy> {
    fn run(
        source: &'source Circuit,
        policy: &'policy LocalOptimizationPolicy,
    ) -> Result<TransformOutcome, CompilerError> {
        Self::run_internal(source, policy, false).map(|(outcome, _)| outcome)
    }

    fn run_with_rewrite_edits(
        source: &'source Circuit,
        policy: &'policy LocalOptimizationPolicy,
    ) -> Result<(TransformOutcome, RewriteEdits), CompilerError> {
        let (outcome, edits) = Self::run_internal(source, policy, true)?;
        Ok((
            outcome,
            edits.expect("rewrite edits were requested for the one-qubit pass"),
        ))
    }

    fn run_internal(
        source: &'source Circuit,
        policy: &'policy LocalOptimizationPolicy,
        track_rewrite_edits: bool,
    ) -> Result<(TransformOutcome, Option<RewriteEdits>), CompilerError> {
        let rebuild = CircuitRebuildContext::new(source);
        let root_classical = rebuild.root_classical().clone();
        let mut pass = Self {
            source,
            policy,
            rebuild,
        };
        let rewrite =
            pass.process_sequence(source.operations(), &root_classical, track_rewrite_edits)?;
        if !rewrite.changed {
            return Ok((
                TransformOutcome::Unchanged,
                track_rewrite_edits.then(|| {
                    RewriteEdits::linear(
                        source.operations().len(),
                        source.operations().len(),
                        Vec::new(),
                    )
                }),
            ));
        }
        debug_assert!(
            rewrite
                .provenance
                .as_ref()
                .is_none_or(|provenance| rewrite.operations.len() == provenance.len())
        );
        let edits = rewrite.provenance.as_ref().map(|provenance| {
            RewriteEdits::from_operation_provenance(source.operations().len(), provenance)
        });
        let mut global_phase = source.global_phase();
        if rewrite.phase_delta.abs() > PHASE_EPS {
            global_phase = global_phase + Parameter::from(rewrite.phase_delta);
        }
        let circuit = pass
            .rebuild
            .finish(source.qubits(), rewrite.operations, global_phase)?;
        Ok((TransformOutcome::Changed(circuit), edits))
    }

    fn process_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        track_provenance: bool,
    ) -> Result<SequenceRewrite, CompilerError> {
        let mut values = Vec::with_capacity(operations.len());
        let mut provenance = track_provenance.then(|| Vec::with_capacity(operations.len()));
        let mut nested_changed = false;
        for (order, operation) in operations.iter().enumerate() {
            if let Instruction::ClassicalControl(control) = &operation.instruction {
                let (instruction, changed) = self.rebuild_control_flow(control, classical_remap)?;
                values.push(ValueOperation {
                    qubits: instruction.used_qubits().into_iter().collect(),
                    instruction: ValueInstruction::ClassicalControl(instruction),
                    params: CircuitRebuildContext::resolve_source_params(
                        self.source,
                        &operation.params,
                    )?,
                    label: operation.label.clone(),
                });
                if let Some(provenance) = &mut provenance {
                    provenance.push((!changed).then_some(order));
                }
                nested_changed |= changed;
            } else {
                values.push(self.rebuild.remap_preserved_operation(
                    self.source,
                    operation,
                    classical_remap,
                )?);
                if let Some(provenance) = &mut provenance {
                    provenance.push(Some(order));
                }
            }
        }

        let optimized = match self.policy {
            LocalOptimizationPolicy::Device { .. } => {
                // Preserve the existing native behavior: frame movement is
                // speculative within a native round, while the outer minimum
                // point controller decides whether the whole round survives.
                let framed = propagate_frames(values, provenance)?;
                let fused = fuse_one_qubit_runs(framed.operations, framed.provenance, self.policy)?;
                ValueRewrite {
                    operations: fused.operations,
                    provenance: fused.provenance,
                    phase_delta: framed.phase_delta + fused.phase_delta,
                    changed: framed.changed || fused.changed,
                }
            }
            LocalOptimizationPolicy::Logical | LocalOptimizationPolicy::Basis(_) => {
                optimize_transactional(values, provenance, self.policy)?
            }
        };
        Ok(SequenceRewrite {
            operations: optimized.operations,
            provenance: optimized.provenance,
            phase_delta: optimized.phase_delta,
            changed: nested_changed || optimized.changed,
        })
    }

    fn rebuild_body(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
    ) -> Result<(ValueControlBody, bool), CompilerError> {
        let mut rewrite = self.process_sequence(operations, classical_remap, false)?;
        if rewrite.phase_delta.abs() > PHASE_EPS {
            rewrite.operations.insert(
                0,
                ValueOperation {
                    instruction: ValueInstruction::from_instruction(Instruction::Standard(
                        StandardGate::GPhase,
                    )),
                    qubits: SmallVec::new(),
                    params: smallvec![ParameterValue::Fixed(rewrite.phase_delta)],
                    label: None,
                },
            );
            rewrite.changed = true;
        }
        Ok((ValueControlBody::new(rewrite.operations), rewrite.changed))
    }

    fn rebuild_control_flow(
        &mut self,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
    ) -> Result<(ValueClassicalControlOp, bool), CompilerError> {
        Ok(match control {
            ClassicalControlOp::If(op) => {
                let (then_body, then_changed) =
                    self.rebuild_body(op.then_body().operations(), classical_remap)?;
                let else_rewrite = op
                    .else_body()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?;
                let else_changed = else_rewrite.as_ref().is_some_and(|(_, changed)| *changed);
                (
                    ValueClassicalControlOp::If {
                        condition: classical_remap.remap_expr(op.condition())?,
                        then_body,
                        else_body: else_rewrite.map(|(body, _)| body),
                    },
                    then_changed || else_changed,
                )
            }
            ClassicalControlOp::While(op) => {
                let (body, changed) = self.rebuild_body(op.body().operations(), classical_remap)?;
                (
                    ValueClassicalControlOp::While {
                        condition: classical_remap.remap_expr(op.condition())?,
                        body,
                    },
                    changed,
                )
            }
            ClassicalControlOp::For(op) => {
                let (body, changed) = self.rebuild_body(op.body().operations(), classical_remap)?;
                (
                    ValueClassicalControlOp::For {
                        var: classical_remap.remap_var(op.var())?,
                        start: classical_remap.remap_expr(op.start())?,
                        stop: classical_remap.remap_expr(op.stop())?,
                        step: classical_remap.remap_expr(op.step())?,
                        body,
                    },
                    changed,
                )
            }
            ClassicalControlOp::Switch(op) => {
                let mut changed = false;
                let cases = op
                    .cases()
                    .iter()
                    .map(|case| {
                        let (body, body_changed) =
                            self.rebuild_body(case.body().operations(), classical_remap)?;
                        changed |= body_changed;
                        Ok(ValueSwitchCase::new(case.value(), body))
                    })
                    .collect::<Result<Vec<_>, CompilerError>>()?;
                let default_rewrite = op
                    .default()
                    .map(|body| self.rebuild_body(body.operations(), classical_remap))
                    .transpose()?;
                changed |= default_rewrite
                    .as_ref()
                    .is_some_and(|(_, body_changed)| *body_changed);
                (
                    ValueClassicalControlOp::Switch {
                        target: classical_remap.remap_expr(op.target())?,
                        cases,
                        default: default_rewrite.map(|(body, _)| body),
                    },
                    changed,
                )
            }
            ClassicalControlOp::Break => (ValueClassicalControlOp::Break, false),
            ClassicalControlOp::Continue => (ValueClassicalControlOp::Continue, false),
        })
    }
}

struct ValueRewrite {
    operations: Vec<ValueOperation>,
    provenance: Option<Vec<Option<usize>>>,
    phase_delta: f64,
    changed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct LogicalOneQCost {
    one_qubit_ops: usize,
    affected_region_depth: usize,
    total_gate_ops: usize,
}

impl LocalOptimizationPolicy {
    fn strictly_better(
        &self,
        candidate: &[ValueOperation],
        source: &[ValueOperation],
    ) -> Result<bool, CompilerError> {
        Ok(match self {
            Self::Logical => logical_one_qubit_cost(candidate) < logical_one_qubit_cost(source),
            Self::Basis(model) => {
                let Some(before) = basis_cost(model, source)? else {
                    return Ok(false);
                };
                let Some(after) = basis_cost(model, candidate)? else {
                    return Ok(false);
                };
                after.two_qubit_ops <= before.two_qubit_ops
                    && compare_basis_cost(after, before).is_lt()
            }
            Self::Device {
                context,
                quality_policy,
            } => {
                let Some(before) = exact_local_sequence_cost(context, source, "source")? else {
                    return Ok(false);
                };
                let Some(after) = exact_local_sequence_cost(context, candidate, "candidate")?
                else {
                    return Ok(false);
                };
                let before = NativeQualityVector::for_value_operations(before, source);
                let after = NativeQualityVector::for_value_operations(after, candidate);
                after.admissible_against(before, *quality_policy)
                    && after.compare(before, *quality_policy).is_lt()
            }
        })
    }
}

fn exact_local_sequence_cost(
    context: &DeviceTwoQubitSynthesisContext,
    operations: &[ValueOperation],
    role: &str,
) -> Result<Option<crate::compile::transform::decompose::unitary::DevicePhysicalCost>, CompilerError>
{
    match context.exact_sequence_cost_diagnostic(operations) {
        Ok(cost) => Ok(Some(cost)),
        Err(DeviceContextCostFailure::Unsupported(failure)) => {
            let _ = failure;
            Ok(None)
        }
        Err(DeviceContextCostFailure::Unprepared(state)) => {
            Err(CompilerError::InvariantViolation(format!(
                "native local optimization context was not prepared for {role} state {state:?}"
            )))
        }
        Err(DeviceContextCostFailure::WrongPlacement) => Err(CompilerError::InvariantViolation(
            "native local optimization requires an exact-physical context".to_string(),
        )),
        Err(DeviceContextCostFailure::InvalidOperation(reason)) => {
            Err(CompilerError::InvariantViolation(format!(
                "invalid native local optimization {role}: {reason}"
            )))
        }
    }
}

fn optimize_transactional(
    operations: Vec<ValueOperation>,
    provenance: Option<Vec<Option<usize>>>,
    policy: &LocalOptimizationPolicy,
) -> Result<ValueRewrite, CompilerError> {
    debug_assert!(
        provenance
            .as_ref()
            .is_none_or(|provenance| operations.len() == provenance.len())
    );
    let framed = propagate_frames(operations.clone(), provenance.clone())?;
    let fused_after_frames = fuse_one_qubit_runs(framed.operations, framed.provenance, policy)?;
    let combined_phase = framed.phase_delta + fused_after_frames.phase_delta;
    let combined_changed = framed.changed || fused_after_frames.changed;
    if combined_changed && policy.strictly_better(&fused_after_frames.operations, &operations)? {
        return Ok(ValueRewrite {
            operations: fused_after_frames.operations,
            provenance: fused_after_frames.provenance,
            phase_delta: combined_phase,
            changed: true,
        });
    }

    // A neutral or harmful frame movement must not hide an independently
    // useful one-qubit fusion on the original sequence.
    fuse_one_qubit_runs(operations, provenance, policy)
}

fn logical_one_qubit_cost(operations: &[ValueOperation]) -> LogicalOneQCost {
    let mut depths = HashMap::<Qubit, usize>::new();
    let mut cost = LogicalOneQCost::default();
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = operation.instruction
        else {
            continue;
        };
        if gate == StandardGate::GPhase {
            continue;
        }
        cost.total_gate_ops += 1;
        if gate.num_qubits() == 1 && operation.qubits.len() == 1 {
            cost.one_qubit_ops += 1;
        }
        if operation.qubits.is_empty() {
            continue;
        }
        let next = operation
            .qubits
            .iter()
            .filter_map(|qubit| depths.get(qubit))
            .max()
            .copied()
            .unwrap_or(0)
            + 1;
        for &qubit in &operation.qubits {
            depths.insert(qubit, next);
        }
        cost.affected_region_depth = cost.affected_region_depth.max(next);
    }
    cost
}

fn basis_cost(
    model: &TargetBasisCostModel,
    operations: &[ValueOperation],
) -> Result<Option<TargetBasisCost>, CompilerError> {
    let operations = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(_))
            ) && operation.params.iter().all(
                |parameter| matches!(parameter, ParameterValue::Fixed(value) if value.is_finite()),
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return Ok(Some(TargetBasisCost::default()));
    }
    let mut qubits = operations
        .iter()
        .flat_map(|operation| operation.qubits.iter().copied())
        .collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());
    qubits.dedup();
    match model.cost_of_fixed_operations(qubits, operations) {
        Ok(cost) => Ok(Some(cost)),
        Err(CompilerError::InvalidInput(_)) | Err(CompilerError::TransformFailed { .. }) => {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn compare_basis_cost(left: TargetBasisCost, right: TargetBasisCost) -> std::cmp::Ordering {
    left.two_qubit_ops
        .cmp(&right.two_qubit_ops)
        .then_with(|| left.depth.cmp(&right.depth))
        .then_with(|| left.total_ops.cmp(&right.total_ops))
        .then_with(|| left.parameterized_ops.cmp(&right.parameterized_ops))
}

/// Replaces fixed numeric 1Q runs only when the synthesized unitary has a
/// strictly better exact-physical device cost than the original run.
fn fuse_one_qubit_runs(
    operations: Vec<ValueOperation>,
    provenance: Option<Vec<Option<usize>>>,
    policy: &LocalOptimizationPolicy,
) -> Result<ValueRewrite, CompilerError> {
    debug_assert!(
        provenance
            .as_ref()
            .is_none_or(|provenance| operations.len() == provenance.len())
    );
    let runs = collect_one_qubit_runs(&operations);
    let mut replacements = HashMap::<usize, (Vec<usize>, Vec<ValueOperation>)>::new();
    let mut phase_delta = 0.0;

    for run in runs.into_iter().filter(|run| run.len() >= 2) {
        let source_ops = run
            .iter()
            .map(|order| operations[*order].clone())
            .collect::<Vec<_>>();
        let Some(matrix) = one_qubit_run_matrix(&source_ops) else {
            continue;
        };
        let Ok(decomposition) = synthesize_numeric_1q_unitary(&matrix) else {
            continue;
        };
        let qubit = source_ops[0].qubits[0];
        let mut candidate = Vec::new();
        if decomposition.theta.abs() > PHASE_EPS
            || decomposition.phi.abs() > PHASE_EPS
            || decomposition.lambda.abs() > PHASE_EPS
        {
            candidate.push(u_operation(qubit, decomposition));
        }
        if !policy.strictly_better(&candidate, &source_ops)? {
            continue;
        }
        phase_delta += decomposition.global_phase;
        replacements.insert(run[0], (run, candidate));
    }

    if replacements.is_empty() {
        return Ok(ValueRewrite {
            operations,
            provenance,
            phase_delta: 0.0,
            changed: false,
        });
    }

    let mut skipped = HashSet::new();
    for (first, (orders, _)) in &replacements {
        skipped.extend(orders.iter().copied().filter(|order| order != first));
    }
    let mut output = Vec::with_capacity(operations.len());
    let mut output_provenance = provenance
        .as_ref()
        .map(|provenance| Vec::with_capacity(provenance.len()));
    let mut source_orders = provenance.unwrap_or_default().into_iter();
    for (order, operation) in operations.into_iter().enumerate() {
        let source_order = source_orders.next().flatten();
        if let Some((_, replacement)) = replacements.remove(&order) {
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.extend(std::iter::repeat_n(None, replacement.len()));
            }
            output.extend(replacement);
        } else if !skipped.contains(&order) {
            output.push(operation);
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.push(source_order);
            }
        }
    }
    Ok(ValueRewrite {
        operations: output,
        provenance: output_provenance,
        phase_delta,
        changed: true,
    })
}

fn collect_one_qubit_runs(operations: &[ValueOperation]) -> Vec<Vec<usize>> {
    let mut active = BTreeMap::<Qubit, Vec<usize>>::new();
    let mut runs = Vec::new();
    for (order, operation) in operations.iter().enumerate() {
        if is_fixed_numeric_one_qubit_gate(operation) {
            active.entry(operation.qubits[0]).or_default().push(order);
            continue;
        }

        let global_boundary = operation.qubits.is_empty()
            || matches!(operation.instruction, ValueInstruction::ClassicalControl(_))
            || matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Directive(Directive::Barrier))
            );
        if global_boundary {
            runs.extend(std::mem::take(&mut active).into_values());
        } else {
            for qubit in &operation.qubits {
                if let Some(run) = active.remove(qubit) {
                    runs.push(run);
                }
            }
        }
    }
    runs.extend(active.into_values());
    runs
}

fn is_fixed_numeric_one_qubit_gate(operation: &ValueOperation) -> bool {
    matches!(
        &operation.instruction,
        ValueInstruction::Instruction(Instruction::Standard(gate))
            if gate.num_qubits() == 1
                && operation.qubits.len() == 1
                && operation.label.is_none()
                && operation.params.iter().all(|param| {
                    matches!(param, ParameterValue::Fixed(value) if value.is_finite())
                })
                && gate.matrix(&fixed_params(operation).unwrap_or_default()).is_ok()
    )
}

fn fixed_params(operation: &ValueOperation) -> Option<Vec<f64>> {
    operation
        .params
        .iter()
        .map(|param| match param {
            ParameterValue::Fixed(value) if value.is_finite() => Some(*value),
            ParameterValue::Fixed(_) | ParameterValue::Param(_) => None,
        })
        .collect()
}

fn one_qubit_run_matrix(operations: &[ValueOperation]) -> Option<Array2<Complex64>> {
    let mut matrix = Array2::<Complex64>::eye(2);
    for operation in operations {
        let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction
        else {
            return None;
        };
        let gate_matrix = gate.matrix(&fixed_params(operation)?).ok()?;
        matrix = gate_matrix.dot(&matrix);
    }
    Some(matrix)
}

fn u_operation(qubit: Qubit, decomposition: OneQubitUnitaryDecomposition) -> ValueOperation {
    ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::U)),
        qubits: smallvec![qubit],
        params: smallvec![
            ParameterValue::Fixed(decomposition.theta),
            ParameterValue::Fixed(decomposition.phi),
            ParameterValue::Fixed(decomposition.lambda),
        ],
        label: None,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct QubitFrame {
    z_angle: f64,
    pauli_x: bool,
    pauli_z: bool,
}

impl QubitFrame {
    fn is_empty(self) -> bool {
        self.z_angle.abs() <= PHASE_EPS && !self.pauli_x && !self.pauli_z
    }

    fn multiply_pauli(&mut self, x: bool, z: bool) -> u8 {
        // The new gate is later in circuit order, so the accumulated matrix is
        // P_new * P_pending. In canonical X^x Z^z order this contributes -1 when
        // the new Z anticommutes with a pending X.
        let phase = if z && self.pauli_x { 2 } else { 0 };
        self.pauli_x ^= x;
        self.pauli_z ^= z;
        phase
    }
}

/// Ejects pending Z/Pauli frames forward through a closed table of proven gate
/// identities and materializes a frame at the first unsupported boundary.
///
/// A pending frame occurs earlier in circuit time than the operation currently
/// being visited. Moving it forward therefore conjugates it by that operation.
/// Frames never cross structured control flow, labels, barriers, or resets.
fn propagate_frames(
    operations: Vec<ValueOperation>,
    provenance: Option<Vec<Option<usize>>>,
) -> Result<ValueRewrite, CompilerError> {
    debug_assert!(
        provenance
            .as_ref()
            .is_none_or(|provenance| operations.len() == provenance.len())
    );
    let mut frames = BTreeMap::<Qubit, QubitFrame>::new();
    let mut output = Vec::with_capacity(operations.len());
    let mut output_provenance = provenance
        .as_ref()
        .map(|provenance| Vec::with_capacity(provenance.len()));
    let mut source_orders = provenance.unwrap_or_default().into_iter();
    let mut phase_delta = 0.0;
    let mut changed = false;

    for mut operation in operations {
        let source_order = source_orders.next().flatten();
        if operation.label.is_none()
            && let Some((qubit, z_angle, phase)) = z_carrier(&operation)
        {
            flush_pauli(
                qubit,
                &mut frames,
                &mut output,
                &mut output_provenance,
                &mut phase_delta,
            );
            frames.entry(qubit).or_default().z_angle += z_angle;
            phase_delta += phase;
            changed = true;
            continue;
        }
        if operation.label.is_none()
            && let Some((qubit, x, z, phase)) = pauli_carrier(&operation)
        {
            flush_z(qubit, &mut frames, &mut output, &mut output_provenance);
            let frame = frames.entry(qubit).or_default();
            let extra_phase = frame.multiply_pauli(x, z);
            phase_delta += phase + f64::from(extra_phase) * FRAC_PI_2;
            changed = true;
            continue;
        }

        let instruction = match &operation.instruction {
            ValueInstruction::Instruction(instruction) => Some(instruction.clone()),
            ValueInstruction::ClassicalControl(_) => None,
        };

        if operation.label.is_none()
            && matches!(
                instruction.as_ref(),
                Some(Instruction::Standard(StandardGate::SWAP))
            )
            && operation.qubits.len() == 2
        {
            let left = frames.remove(&operation.qubits[0]).unwrap_or_default();
            let right = frames.remove(&operation.qubits[1]).unwrap_or_default();
            frames.insert(operation.qubits[0], right);
            frames.insert(operation.qubits[1], left);
            output.push(operation);
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.push(source_order);
            }
            changed |= !left.is_empty() || !right.is_empty();
            continue;
        }

        if operation.label.is_none()
            && let Some(gate @ (StandardGate::CX | StandardGate::CZ)) =
                instruction.as_ref().and_then(|i| {
                    let Instruction::Standard(gate @ (StandardGate::CX | StandardGate::CZ)) = i
                    else {
                        return None;
                    };
                    Some(*gate)
                })
            && operation.qubits.len() == 2
        {
            let pair = [operation.qubits[0], operation.qubits[1]];
            if gate == StandardGate::CX {
                flush_z(pair[1], &mut frames, &mut output, &mut output_provenance);
            }
            if pair.iter().any(|qubit| {
                frames
                    .get(qubit)
                    .is_some_and(|frame| frame.pauli_x || frame.pauli_z)
            }) {
                for qubit in pair {
                    flush_z(qubit, &mut frames, &mut output, &mut output_provenance);
                }
                phase_delta += propagate_clifford_paulis(gate, pair, &mut frames);
                changed = true;
            }
            output.push(operation);
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.push(source_order);
            }
            continue;
        }

        if operation.label.is_none()
            && let Some(Instruction::Standard(gate)) = instruction.as_ref()
            && is_z_diagonal(*gate)
        {
            let blocked = operation
                .qubits
                .iter()
                .any(|qubit| frames.get(qubit).is_some_and(|frame| frame.pauli_x));
            if !blocked {
                output.push(operation);
                if let Some(output_provenance) = &mut output_provenance {
                    output_provenance.push(source_order);
                }
                continue;
            }
        }

        if operation.label.is_none() && absorb_z_into_xy_axis(&mut operation, &frames) {
            output.push(operation);
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.push(None);
            }
            changed = true;
            continue;
        }

        if instruction
            .as_ref()
            .is_some_and(|instruction| instruction.has_measurement())
        {
            for qubit in &operation.qubits {
                if frames.get(qubit).is_some_and(|frame| frame.pauli_x) {
                    flush_frame(
                        *qubit,
                        &mut frames,
                        &mut output,
                        &mut output_provenance,
                        &mut phase_delta,
                    );
                } else {
                    changed |= frames.remove(qubit).is_some_and(|frame| !frame.is_empty());
                }
            }
            output.push(operation);
            if let Some(output_provenance) = &mut output_provenance {
                output_provenance.push(source_order);
            }
            continue;
        }

        let global_boundary = operation.qubits.is_empty()
            || operation.label.is_some()
            || matches!(operation.instruction, ValueInstruction::ClassicalControl(_))
            || matches!(
                instruction.as_ref(),
                Some(Instruction::Directive(
                    Directive::Barrier | Directive::Reset
                ))
            );
        if global_boundary {
            flush_all(
                &mut frames,
                &mut output,
                &mut output_provenance,
                &mut phase_delta,
            );
        } else {
            for qubit in &operation.qubits {
                flush_frame(
                    *qubit,
                    &mut frames,
                    &mut output,
                    &mut output_provenance,
                    &mut phase_delta,
                );
            }
        }
        output.push(operation);
        if let Some(output_provenance) = &mut output_provenance {
            output_provenance.push(source_order);
        }
    }
    flush_all(
        &mut frames,
        &mut output,
        &mut output_provenance,
        &mut phase_delta,
    );

    Ok(ValueRewrite {
        operations: output,
        provenance: output_provenance,
        phase_delta,
        changed,
    })
}

fn z_carrier(operation: &ValueOperation) -> Option<(Qubit, f64, f64)> {
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return None;
    };
    let qubit = *operation.qubits.first()?;
    Some(match gate {
        StandardGate::RZ => (qubit, *fixed_params(operation)?.first()?, 0.0),
        StandardGate::Phase => {
            let angle = *fixed_params(operation)?.first()?;
            (qubit, angle, angle / 2.0)
        }
        StandardGate::S => (qubit, FRAC_PI_2, FRAC_PI_4),
        StandardGate::SDG => (qubit, -FRAC_PI_2, -FRAC_PI_4),
        StandardGate::T => (qubit, FRAC_PI_4, FRAC_PI_4 / 2.0),
        StandardGate::TDG => (qubit, -FRAC_PI_4, -FRAC_PI_4 / 2.0),
        _ => return None,
    })
}

fn pauli_carrier(operation: &ValueOperation) -> Option<(Qubit, bool, bool, f64)> {
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return None;
    };
    let qubit = *operation.qubits.first()?;
    Some(match gate {
        StandardGate::X => (qubit, true, false, 0.0),
        StandardGate::Y => (qubit, true, true, FRAC_PI_2),
        StandardGate::Z => (qubit, false, true, 0.0),
        _ => return None,
    })
}

fn absorb_z_into_xy_axis(
    operation: &mut ValueOperation,
    frames: &BTreeMap<Qubit, QubitFrame>,
) -> bool {
    let Some(&qubit) = operation.qubits.first() else {
        return false;
    };
    if operation.qubits.len() != 1 {
        return false;
    }
    let Some(frame) = frames.get(&qubit) else {
        return false;
    };
    if frame.z_angle.abs() <= PHASE_EPS || frame.pauli_x || frame.pauli_z {
        return false;
    }
    let ValueInstruction::Instruction(Instruction::Standard(gate)) = &operation.instruction else {
        return false;
    };
    let axis_index = match gate {
        StandardGate::RXY => 1,
        StandardGate::XY | StandardGate::XY2P | StandardGate::XY2M => 0,
        _ => return false,
    };
    let Some(ParameterValue::Fixed(axis)) = operation.params.get_mut(axis_index) else {
        return false;
    };
    // G(phi) RZ(a) = RZ(a) G(phi-a), so the pending frame can stay virtual.
    *axis -= frame.z_angle;
    true
}

fn is_z_diagonal(gate: StandardGate) -> bool {
    matches!(
        gate,
        StandardGate::CZ | StandardGate::CRZ | StandardGate::RZZ
    )
}

#[derive(Clone, Copy, Default)]
struct TwoQubitPauli {
    phase: u8,
    x: [bool; 2],
    z: [bool; 2],
}

impl TwoQubitPauli {
    fn multiply(self, right: Self) -> Self {
        let anti = self
            .z
            .iter()
            .zip(right.x)
            .filter(|(z, x)| **z && *x)
            .count() as u8;
        Self {
            phase: (self.phase + right.phase + 2 * anti) % 4,
            x: [self.x[0] ^ right.x[0], self.x[1] ^ right.x[1]],
            z: [self.z[0] ^ right.z[0], self.z[1] ^ right.z[1]],
        }
    }
}

/// Conjugates a two-qubit Pauli frame through CX or CZ using their stabilizer
/// generator images in canonical `X0 X1 Z0 Z1` multiplication order.
fn propagate_clifford_paulis(
    gate: StandardGate,
    qubits: [Qubit; 2],
    frames: &mut BTreeMap<Qubit, QubitFrame>,
) -> f64 {
    let input = TwoQubitPauli {
        phase: 0,
        x: qubits.map(|qubit| frames.get(&qubit).is_some_and(|frame| frame.pauli_x)),
        z: qubits.map(|qubit| frames.get(&qubit).is_some_and(|frame| frame.pauli_z)),
    };
    let generators = match gate {
        StandardGate::CX => [
            TwoQubitPauli {
                x: [true, true],
                ..Default::default()
            },
            TwoQubitPauli {
                x: [false, true],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, true],
                ..Default::default()
            },
        ],
        StandardGate::CZ => [
            TwoQubitPauli {
                x: [true, false],
                z: [false, true],
                ..Default::default()
            },
            TwoQubitPauli {
                x: [false, true],
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [true, false],
                ..Default::default()
            },
            TwoQubitPauli {
                z: [false, true],
                ..Default::default()
            },
        ],
        _ => unreachable!("only Clifford propagation gates are passed here"),
    };
    let enabled = [input.x[0], input.x[1], input.z[0], input.z[1]];
    let result = generators
        .into_iter()
        .zip(enabled)
        .filter(|(_, enabled)| *enabled)
        .fold(TwoQubitPauli::default(), |acc, (generator, _)| {
            acc.multiply(generator)
        });
    for (index, qubit) in qubits.into_iter().enumerate() {
        let frame = frames.entry(qubit).or_default();
        frame.pauli_x = result.x[index];
        frame.pauli_z = result.z[index];
    }
    f64::from(result.phase) * FRAC_PI_2
}

fn flush_all(
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    provenance: &mut Option<Vec<Option<usize>>>,
    phase_delta: &mut f64,
) {
    let qubits = frames.keys().copied().collect::<Vec<_>>();
    for qubit in qubits {
        flush_frame(qubit, frames, output, provenance, phase_delta);
    }
}

fn flush_frame(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    provenance: &mut Option<Vec<Option<usize>>>,
    phase_delta: &mut f64,
) {
    flush_pauli(qubit, frames, output, provenance, phase_delta);
    flush_z(qubit, frames, output, provenance);
    if frames.get(&qubit).is_some_and(|frame| frame.is_empty()) {
        frames.remove(&qubit);
    }
}

fn flush_pauli(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    provenance: &mut Option<Vec<Option<usize>>>,
    phase_delta: &mut f64,
) {
    let Some(frame) = frames.get_mut(&qubit) else {
        return;
    };
    let gate = match (frame.pauli_x, frame.pauli_z) {
        (false, false) => None,
        (true, false) => Some(StandardGate::X),
        (false, true) => Some(StandardGate::Z),
        (true, true) => {
            *phase_delta -= FRAC_PI_2;
            Some(StandardGate::Y)
        }
    };
    if let Some(gate) = gate {
        output.push(ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
            qubits: smallvec![qubit],
            params: SmallVec::new(),
            label: None,
        });
        if let Some(provenance) = provenance {
            provenance.push(None);
        }
    }
    frame.pauli_x = false;
    frame.pauli_z = false;
}

fn flush_z(
    qubit: Qubit,
    frames: &mut BTreeMap<Qubit, QubitFrame>,
    output: &mut Vec<ValueOperation>,
    provenance: &mut Option<Vec<Option<usize>>>,
) {
    let Some(frame) = frames.get_mut(&qubit) else {
        return;
    };
    if frame.z_angle.abs() > PHASE_EPS {
        output.push(ValueOperation {
            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                StandardGate::RZ,
            )),
            qubits: smallvec![qubit],
            params: smallvec![ParameterValue::Fixed(frame.z_angle)],
            label: None,
        });
        if let Some(provenance) = provenance {
            provenance.push(None);
        }
    }
    frame.z_angle = 0.0;
}

#[cfg(test)]
#[path = "native_optimization_test.rs"]
mod native_optimization_test;
