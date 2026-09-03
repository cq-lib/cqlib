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

//! Transformer entry point and circuit rebuild logic for knowledge rewrite.

use crate::circuit::operation::ValueOperation;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, Instruction, Operation, Parameter, ParameterValue,
    Qubit, StandardGate, ValueClassicalControlOp, ValueControlBody, ValueInstruction,
    ValueSwitchCase,
};
use crate::compile::error::CompilerError;
use crate::compile::knowledge::library::RuleLibrary;
use crate::compile::knowledge::matcher::KnowledgeInstructionKey as RewriteInstructionKey;
use crate::compile::transform::rewrite::basis::{TargetContext, validate_final_target};
use crate::compile::transform::rewrite::config::RewriteConfig;
use crate::compile::transform::rewrite::diagnostics::KnowledgeRewriteDiagnostics;
use crate::compile::transform::rewrite::matcher::{
    BlockMatchCache, CompiledRuleSet, PatchPlanStep, ReplacementItem, RewritePatch,
    RunActiveRuleSet, is_gphase_instruction, patch_application_plan, resolve_operation_param,
    select_rewrites_for_anchor_ranges,
};

use crate::compile::transform::lowering_support::LoweringTarget;
use crate::compile::transform::rebuild::{CircuitRebuildContext, ClassicalRemap};
use crate::compile::transform::{TransformOutcome, Transformer};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

static BUILTIN_COMPILED_RULES: OnceLock<Result<Arc<CompiledRuleSet>, String>> = OnceLock::new();
/// Operation count (recursed into control-flow bodies) above which
/// [`KnowledgeRewriter`] prefers the incremental engines over per-round full
/// scans.
///
/// Strictly speaking the incremental engines are correct at any size, but
/// below this threshold a full scan is cheap enough that workspace
/// bookkeeping (match-cache construction, dirty-range tracking) costs more
/// than it saves. This intentionally deviates from an "incremental by
/// default" policy to protect small-circuit latency. The knob is independent
/// of the workload thresholds in `matcher.rs`, which control rayon
/// parallelism once a scan happens; adjust each only against its own
/// measurements.
pub(super) const SMALL_CIRCUIT_FULL_SCAN_THRESHOLD: usize = 4_096;

/// Execution strategy for the rewrite fixpoint.
///
/// `Auto` is the production default and picks per circuit size; see
/// [`SMALL_CIRCUIT_FULL_SCAN_THRESHOLD`]. The pinned variants exist only for
/// differential tests and are not compiled into production builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewriteExecutionPolicy {
    Auto,
    #[cfg(test)]
    FullScan,
    #[cfg(test)]
    Incremental,
}

/// Aggregate statistics produced by one knowledge rewrite run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeRewriteStats {
    /// Number of fixpoint rounds actually executed.
    pub rounds_executed: u8,
    /// Number of selected rule patches emitted into rebuilt sequences.
    pub rules_applied: usize,
    /// Number of operation sequences whose selected patch set was non-empty.
    pub changed_sequences: usize,
    /// Whether the run observed a stable round before hitting `max_rounds`.
    pub reached_fixpoint: bool,
}

impl KnowledgeRewriteStats {
    fn merge_round(&mut self, other: &RoundStats) {
        self.rules_applied += other.rules_applied;
        self.changed_sequences += other.changed_sequences;
    }
}

/// Public result for running the rewriter directly.
#[derive(Debug, Clone)]
pub struct KnowledgeRewriteResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub stats: KnowledgeRewriteStats,
    /// Incremental-workset and condition-evaluation counters for this run.
    pub diagnostics: KnowledgeRewriteDiagnostics,
}

/// Workflow-only result which leaves a stable input circuit in place instead
/// of cloning it solely to populate the standalone result shape.
pub(crate) struct KnowledgeRewriteSessionResult {
    pub(crate) circuit: Option<Circuit>,
    pub(crate) changed: bool,
    pub(crate) stats: KnowledgeRewriteStats,
    pub(crate) diagnostics: KnowledgeRewriteDiagnostics,
}

struct LinearRewriteResult {
    circuit: Option<Circuit>,
    changed: bool,
    stats: KnowledgeRewriteStats,
    diagnostics: KnowledgeRewriteDiagnostics,
}

impl LinearRewriteResult {
    fn into_public(self, source: &Circuit) -> KnowledgeRewriteResult {
        KnowledgeRewriteResult {
            circuit: self.circuit.unwrap_or_else(|| source.clone()),
            changed: self.changed,
            stats: self.stats,
            diagnostics: self.diagnostics,
        }
    }

    fn into_session(self) -> KnowledgeRewriteSessionResult {
        KnowledgeRewriteSessionResult {
            circuit: self.circuit,
            changed: self.changed,
            stats: self.stats,
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RoundStats {
    rules_applied: usize,
    changed_sequences: usize,
    representation_changes: usize,
    diagnostics: KnowledgeRewriteDiagnostics,
}

impl RoundStats {
    fn changed(&self) -> bool {
        self.rules_applied > 0 || self.representation_changes > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ScopeSegment {
    IfThen(usize),
    IfElse(usize),
    WhileBody(usize),
    ForBody(usize),
    SwitchCase { control: usize, case: usize },
    SwitchDefault(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct ScopeId(Vec<ScopeSegment>);

impl ScopeId {
    fn child(&self, segment: ScopeSegment) -> Self {
        let mut path = self.0.clone();
        path.push(segment);
        Self(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BlockId {
    scope: ScopeId,
    slot: usize,
}

#[derive(Debug, Clone)]
enum BlockWorkset {
    Full,
    Ranges(Vec<Range<usize>>),
}

#[derive(Debug, Clone, Default)]
struct RewriteWorkset {
    blocks: HashMap<BlockId, BlockWorkset>,
    full_scopes: HashSet<ScopeId>,
    caches: HashMap<BlockId, Arc<BlockMatchCache>>,
}

/// Cached matching state for the workflow's single flat rewrite block.
///
/// The owning [`KnowledgeRewriteSession`](super::session::KnowledgeRewriteSession)
/// advances this cache through verified transform edits, so a later rewrite
/// resolves and indexes only replacement operations.
#[derive(Debug, Clone)]
pub(crate) struct LinearRewriteWorkspace {
    cache: BlockMatchCache,
}

impl LinearRewriteWorkspace {
    pub(super) fn after_linear_replacements(
        self,
        after: &Circuit,
        replacements: &[super::OperationReplacement],
        qubits_renamed: bool,
    ) -> Option<Self> {
        Some(Self {
            cache: self
                .cache
                .after_linear_replacements(after, replacements, qubits_renamed)?,
        })
    }
}

impl RewriteWorkset {
    fn block_ranges(&self, block: &BlockId) -> Option<&[Range<usize>]> {
        if self.full_scopes.contains(&block.scope) {
            return None;
        }
        match self.blocks.get(block) {
            Some(BlockWorkset::Full) => None,
            Some(BlockWorkset::Ranges(ranges)) => Some(ranges),
            None => Some(&[]),
        }
    }

    fn mark_full_scope(&mut self, scope: ScopeId) {
        self.blocks.retain(|block, _| block.scope != scope);
        self.caches.retain(|block, _| block.scope != scope);
        self.full_scopes.insert(scope);
    }
}

/// Transformer that optimizes circuits using the compiler knowledge base.
#[derive(Debug, Clone)]
pub struct KnowledgeRewriter {
    config: RewriteConfig,
    execution_policy: RewriteExecutionPolicy,
}

impl KnowledgeRewriter {
    /// Creates a knowledge rewriter with the supplied configuration.
    pub fn new(config: RewriteConfig) -> Self {
        Self {
            config,
            execution_policy: RewriteExecutionPolicy::Auto,
        }
    }

    /// Creates a knowledge rewriter using conservative production defaults.
    pub fn production() -> Self {
        Self::new(RewriteConfig::production())
    }

    /// Creates a knowledge rewriter using explicit lowering defaults.
    pub fn lowering() -> Self {
        Self::new(RewriteConfig::lowering())
    }

    pub const fn config(&self) -> &RewriteConfig {
        &self.config
    }

    /// Returns the maximum forward matcher reach for this configuration.
    ///
    /// Workflow-lifetime rewrite certificates store this exact value beside
    /// their clean-anchor proof. Later mutation invalidation must use the
    /// stored proof reach rather than deriving a potentially narrower value
    /// from a subsequent configuration.
    pub(crate) fn proof_reach(&self) -> Result<usize, CompilerError> {
        Ok(builtin_compiled_rules()?.max_match_reach(&self.config))
    }

    /// Returns whether clean-anchor proofs produced by this rewriter survive
    /// a bijective renaming of runtime qubits.
    ///
    /// The current rule language binds dense rule-local qubit labels and its
    /// conditions observe parameters only. It cannot name a physical qubit or
    /// inspect a device domain. Keep this capability explicit so a future
    /// domain-aware rule model must opt out before routing edits can
    /// advance its proofs.
    pub(crate) fn proof_is_qubit_bijection_invariant(&self) -> Result<bool, CompilerError> {
        Ok(builtin_compiled_rules()?.is_qubit_bijection_invariant())
    }

    #[cfg(test)]
    pub(crate) fn force_full_scan(mut self) -> Self {
        self.execution_policy = RewriteExecutionPolicy::FullScan;
        self
    }

    #[cfg(test)]
    pub(crate) fn force_incremental(mut self) -> Self {
        self.execution_policy = RewriteExecutionPolicy::Incremental;
        self
    }

    /// Runs knowledge-based local rewrite to a fixpoint or round limit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::{Circuit, Qubit};
    /// use cqlib_core::compile::transform::KnowledgeRewriter;
    ///
    /// let mut circuit = Circuit::new(1);
    /// circuit.x(Qubit::new(0)).unwrap();
    /// circuit.x(Qubit::new(0)).unwrap();
    ///
    /// let result = KnowledgeRewriter::production().run(&circuit).unwrap();
    /// assert!(result.stats.rounds_executed >= 1);
    /// assert!(result.stats.reached_fixpoint);
    /// let _rewritten = result.circuit;
    /// ```
    pub fn run(&self, circuit: &Circuit) -> Result<KnowledgeRewriteResult, CompilerError> {
        self.run_internal(circuit, true)
    }

    fn run_internal(
        &self,
        circuit: &Circuit,
        collect_diagnostics: bool,
    ) -> Result<KnowledgeRewriteResult, CompilerError> {
        if self.config.max_rounds() == 0 {
            return Err(CompilerError::InvalidInput(
                "rewrite max_rounds must be greater than zero".to_string(),
            ));
        }

        let rules = builtin_compiled_rules()?;
        let target_context = TargetContext::from_config(&self.config, rules.as_ref())?;
        let incremental = match self.execution_policy {
            RewriteExecutionPolicy::Auto => {
                recursive_operation_count(circuit.operations()) >= SMALL_CIRCUIT_FULL_SCAN_THRESHOLD
            }
            #[cfg(test)]
            RewriteExecutionPolicy::FullScan => false,
            #[cfg(test)]
            RewriteExecutionPolicy::Incremental => true,
        };
        if incremental && is_linear_workspace_eligible(circuit) {
            return self
                .run_linear_workspace(
                    circuit,
                    rules.as_ref(),
                    target_context.as_ref(),
                    None,
                    None,
                    collect_diagnostics,
                )
                .map(|(result, _)| result.into_public(circuit));
        }
        let active_rules = rules.active_rules(&self.config, target_context.as_ref());

        let mut current = circuit.clone();
        let mut aggregate = KnowledgeRewriteStats::default();
        let mut diagnostics = KnowledgeRewriteDiagnostics::default();
        let mut changed = false;
        let mut workset: Option<RewriteWorkset> = None;

        for round in 1..=self.config.max_rounds() {
            aggregate.rounds_executed = round;
            let (next, round_stats, next_workset) = RoundRewriter::run(
                &current,
                rules.as_ref(),
                &active_rules,
                &self.config,
                target_context.as_ref(),
                workset.as_ref(),
                collect_diagnostics,
            )?;
            diagnostics.merge(round_stats.diagnostics);
            if !round_stats.changed() {
                aggregate.reached_fixpoint = true;
                break;
            }

            changed = true;
            aggregate.merge_round(&round_stats);
            current = next;
            workset = incremental.then_some(next_workset);
        }

        validate_final_target(&current, &self.config)?;

        Ok(KnowledgeRewriteResult {
            circuit: current,
            changed,
            stats: aggregate,
            diagnostics,
        })
    }

    /// Workflow entry point which always establishes a reusable flat
    /// workspace when the circuit shape permits it, independent of the
    /// standalone small-circuit policy.
    pub(crate) fn run_for_session(
        &self,
        circuit: &Circuit,
        reconciled: Option<(Vec<Range<usize>>, Option<LinearRewriteWorkspace>)>,
        collect_diagnostics: bool,
    ) -> Result<
        (
            KnowledgeRewriteSessionResult,
            Option<LinearRewriteWorkspace>,
        ),
        CompilerError,
    > {
        if !is_linear_workspace_eligible(circuit) {
            let fell_back_from_incremental = reconciled.is_some();
            return self
                .run_internal(circuit, collect_diagnostics)
                .map(|mut result| {
                    if collect_diagnostics && fell_back_from_incremental {
                        result.diagnostics.full_scan_fallbacks =
                            result.diagnostics.full_scan_fallbacks.saturating_add(1);
                    }
                    let circuit = result.changed.then_some(result.circuit);
                    (
                        KnowledgeRewriteSessionResult {
                            circuit,
                            changed: result.changed,
                            stats: result.stats,
                            diagnostics: result.diagnostics,
                        },
                        None,
                    )
                });
        }
        if self.config.max_rounds() == 0 {
            return Err(CompilerError::InvalidInput(
                "rewrite max_rounds must be greater than zero".to_string(),
            ));
        }
        let rules = builtin_compiled_rules()?;
        let target_context = TargetContext::from_config(&self.config, rules.as_ref())?;
        let (ranges, workspace) = reconciled.map_or((None, None), |(ranges, workspace)| {
            (Some(ranges), workspace)
        });
        self.run_linear_workspace(
            circuit,
            rules.as_ref(),
            target_context.as_ref(),
            ranges,
            workspace,
            collect_diagnostics,
        )
        .map(|(result, workspace)| (result.into_session(), Some(workspace)))
    }

    /// Runs the rewrite fixpoint on a flat, gate-only circuit without
    /// rebuilding a [`Circuit`] per round.
    ///
    /// Eligible circuits (see [`is_linear_workspace_eligible`]) form a single
    /// rewrite block, so the fixpoint runs on one persistent operation vector
    /// plus a [`BlockMatchCache`]: each round scans only the dirty anchor
    /// ranges produced by the previous round's patches, splices the selected
    /// patches in place, and
    /// folds global-phase replacements into a running phase delta. The final
    /// [`Circuit`] is materialized exactly once, after the fixpoint.
    ///
    /// Statistics intentionally match the round-based engine
    /// ([`RoundRewriter`]) field by field: one selected patch counts as one
    /// applied rule, a patch-producing round counts as one changed sequence
    /// (the whole circuit is a single block), and the final candidate-free
    /// round still counts toward `rounds_executed`. The
    /// FullScan/Incremental differential tests guard this equivalence.
    fn run_linear_workspace(
        &self,
        circuit: &Circuit,
        rules: &CompiledRuleSet,
        target_context: Option<&TargetContext>,
        initial_anchor_ranges: Option<Vec<Range<usize>>>,
        workspace: Option<LinearRewriteWorkspace>,
        collect_diagnostics: bool,
    ) -> Result<(LinearRewriteResult, LinearRewriteWorkspace), CompilerError> {
        let mut operations = Cow::Borrowed(circuit.operations());
        let mut cache = match workspace {
            Some(workspace) if workspace.cache.len() == operations.len() => workspace.cache,
            _ => BlockMatchCache::new_with_diagnostics(circuit, &operations, collect_diagnostics)?,
        };
        let mut anchor_ranges = initial_anchor_ranges;
        let mut aggregate = KnowledgeRewriteStats::default();
        let diagnostics_before = cache.diagnostics();
        let mut changed = false;
        let mut phase_delta = Parameter::from(0.0);
        let max_match_reach = rules.max_match_reach(&self.config);
        let active_rules = rules.active_rules(&self.config, target_context);

        for round in 1..=self.config.max_rounds() {
            aggregate.rounds_executed = round;
            let patches = select_rewrites_for_anchor_ranges(
                operations.as_ref(),
                &cache,
                rules,
                &active_rules,
                &self.config,
                target_context,
                anchor_ranges.as_deref(),
            )?;
            if patches.is_empty() {
                aggregate.reached_fixpoint = true;
                break;
            }

            changed = true;
            aggregate.rules_applied = aggregate.rules_applied.saturating_add(patches.len());
            aggregate.changed_sequences = aggregate.changed_sequences.saturating_add(1);
            let next_cache = cache
                .into_rewritten(operations.as_ref(), &patches)
                .ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "linear rewrite workspace could not materialize its selected patches"
                            .to_string(),
                    )
                })?;
            let (next_ranges, expected_len) =
                dirty_ranges_after_patches(operations.len(), &patches, max_match_reach);
            // The dirty ranges are clamped to this length; a mismatch with the
            // rewritten block would silently drop dirty anchors, so it is a
            // hard invariant, not a debug assertion.
            if expected_len != next_cache.len() {
                return Err(CompilerError::InvariantViolation(format!(
                    "linear rewrite dirty-range length {expected_len} does not match rewritten block length {}",
                    next_cache.len()
                )));
            }
            let (next_operations, round_phase) =
                apply_linear_patches(operations.as_ref(), &patches)?;
            phase_delta = &phase_delta + &round_phase;
            operations = Cow::Owned(next_operations);
            cache = next_cache;
            anchor_ranges = Some(next_ranges);
        }

        let output = if changed {
            let value_operations = operations
                .into_owned()
                .into_iter()
                .enumerate()
                .map(|(position, operation)| ValueOperation {
                    instruction: ValueInstruction::from_instruction(operation.instruction),
                    qubits: operation.qubits,
                    params: cache
                        .params(position)
                        .iter()
                        .cloned()
                        .map(ParameterValue::from)
                        .collect(),
                    label: operation.label,
                })
                .collect();
            Some(CircuitRebuildContext::new(circuit).finish(
                circuit.qubits(),
                value_operations,
                &circuit.global_phase() + &phase_delta,
            )?)
        } else {
            None
        };
        validate_final_target(output.as_ref().unwrap_or(circuit), &self.config)?;
        let mut diagnostics = KnowledgeRewriteDiagnostics::default();
        diagnostics.merge_matcher(cache.diagnostics().saturating_delta(diagnostics_before));
        Ok((
            LinearRewriteResult {
                circuit: output,
                changed,
                stats: aggregate,
                diagnostics,
            },
            LinearRewriteWorkspace { cache },
        ))
    }
}

fn builtin_compiled_rules() -> Result<Arc<CompiledRuleSet>, CompilerError> {
    match BUILTIN_COMPILED_RULES.get_or_init(|| {
        let library = RuleLibrary::builtin_rules().map_err(|err| err.to_string())?;
        CompiledRuleSet::from_library(library)
            .map(Arc::new)
            .map_err(|err| err.to_string())
    }) {
        Ok(rules) => Ok(Arc::clone(rules)),
        Err(message) => Err(CompilerError::InvariantViolation(message.clone())),
    }
}

// Transformer integration exposes only the generic transform outcome; direct
// callers should use `KnowledgeRewriter::run` when rewrite statistics matter.
impl Transformer for KnowledgeRewriter {
    fn name(&self) -> &'static str {
        "knowledge_rewrite"
    }

    fn transform(
        &self,
        circuit: &Circuit,
        _analysis: Option<&crate::compile::transform::CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError> {
        let result = self.run(circuit)?;
        Ok(if result.changed {
            TransformOutcome::Changed(result.circuit)
        } else {
            TransformOutcome::Unchanged
        })
    }
}

/// Rewrites a circuit with the supplied configuration.
pub fn rewrite_circuit(
    circuit: &Circuit,
    config: RewriteConfig,
) -> Result<KnowledgeRewriteResult, CompilerError> {
    KnowledgeRewriter::new(config).run(circuit)
}

struct RoundRewriter<'a> {
    source: &'a Circuit,
    rules: &'a CompiledRuleSet,
    active_rules: &'a RunActiveRuleSet,
    config: &'a RewriteConfig,
    target_context: Option<&'a TargetContext>,
    collect_diagnostics: bool,
    workset: Option<&'a RewriteWorkset>,
    next_workset: RewriteWorkset,
    max_match_reach: usize,
    rebuild: CircuitRebuildContext,
    stats: RoundStats,
}

impl<'a> RoundRewriter<'a> {
    fn run(
        source: &'a Circuit,
        rules: &'a CompiledRuleSet,
        active_rules: &'a RunActiveRuleSet,
        config: &'a RewriteConfig,
        target_context: Option<&'a TargetContext>,
        workset: Option<&'a RewriteWorkset>,
        collect_diagnostics: bool,
    ) -> Result<(Circuit, RoundStats, RewriteWorkset), CompilerError> {
        let mut rewriter = Self {
            source,
            rules,
            active_rules,
            config,
            target_context,
            collect_diagnostics,
            workset,
            next_workset: RewriteWorkset::default(),
            max_match_reach: rules.max_match_reach(config),
            rebuild: CircuitRebuildContext::new(source),
            stats: RoundStats::default(),
        };
        let mut phase_delta = Parameter::from(0.0);
        let root_classical = rewriter.rebuild.root_classical().clone();
        let mut operations = Vec::with_capacity(source.operations().len());

        rewriter.apply_sequence(
            source.operations(),
            &root_classical,
            &ScopeId::default(),
            LoweringTarget::top_level(&mut operations, &mut phase_delta),
        )?;
        let global_phase = &source.global_phase() + &phase_delta;
        let circuit = rewriter
            .rebuild
            .finish(source.qubits(), operations, global_phase)?;

        Ok((circuit, rewriter.stats, rewriter.next_workset))
    }

    fn apply_sequence(
        &mut self,
        operations: &[Operation],
        classical_remap: &ClassicalRemap,
        scope: &ScopeId,
        mut target: LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        let mut cursor = 0;
        let mut block_slot = 0;
        let mut control_slot = 0;
        while cursor < operations.len() {
            if RewriteInstructionKey::from_instruction(&operations[cursor].instruction).is_none() {
                let current_control_slot = control_slot;
                if matches!(
                    operations[cursor].instruction,
                    Instruction::ClassicalControl(_)
                ) {
                    control_slot += 1;
                }
                self.emit_original_operation(
                    &operations[cursor],
                    classical_remap,
                    scope,
                    current_control_slot,
                    &mut target,
                )?;
                cursor += 1;
                block_slot += 1;
                continue;
            }

            let block_start = cursor;
            while cursor < operations.len()
                && RewriteInstructionKey::from_instruction(&operations[cursor].instruction)
                    .is_some()
            {
                cursor += 1;
            }

            let block = &operations[block_start..cursor];
            let block_id = BlockId {
                scope: scope.clone(),
                slot: block_slot,
            };
            let ranges = self
                .workset
                .map(|workset| workset.block_ranges(&block_id))
                .unwrap_or(None);
            let cache = match self
                .workset
                .filter(|workset| !workset.full_scopes.contains(scope))
                .and_then(|workset| workset.caches.get(&block_id))
            {
                Some(cache) if cache.len() == block.len() => Arc::clone(cache),
                _ => Arc::new(BlockMatchCache::new_with_diagnostics(
                    self.source,
                    block,
                    self.collect_diagnostics,
                )?),
            };
            let diagnostics_before = cache.diagnostics();
            let patches = select_rewrites_for_anchor_ranges(
                block,
                cache.as_ref(),
                self.rules,
                self.active_rules,
                self.config,
                self.target_context,
                ranges,
            )?;
            self.stats
                .diagnostics
                .merge_matcher(cache.diagnostics().saturating_delta(diagnostics_before));
            if patches.is_empty() {
                self.next_workset
                    .caches
                    .insert(block_id.clone(), Arc::clone(&cache));
                if target.is_top_level()
                    && block
                        .iter()
                        .any(|operation| is_gphase_instruction(&operation.instruction))
                {
                    self.next_workset.mark_full_scope(scope.clone());
                }
                for operation in block {
                    self.emit_original_operation(
                        operation,
                        classical_remap,
                        scope,
                        control_slot,
                        &mut target,
                    )?;
                }
            } else {
                self.stats.changed_sequences += 1;
                self.record_next_block_workset(&block_id, block, &patches);
                if let Some(next_cache) = cache.rewritten(block, &patches) {
                    self.next_workset
                        .caches
                        .insert(block_id.clone(), Arc::new(next_cache));
                }
                self.emit_rewritten_block(block, patches, classical_remap, &mut target)?;
            }
        }

        Ok(())
    }

    /// Records the next round's rescan state for a block that just had
    /// patches applied.
    ///
    /// Global-phase involvement (in the block or in any replacement) and
    /// block-summary changes both invalidate anchor-level incremental state,
    /// so the block (or its whole scope) is marked for a full rescan;
    /// otherwise only the dirty ranges around the applied patches are
    /// rescanned. See [`dirty_ranges_after_patches`] for the range math.
    fn record_next_block_workset(
        &mut self,
        block_id: &BlockId,
        block: &[Operation],
        patches: &[RewritePatch],
    ) {
        let contains_gphase = block
            .iter()
            .any(|operation| is_gphase_instruction(&operation.instruction))
            || patches.iter().any(|patch| {
                patch
                    .replacements
                    .iter()
                    .any(|replacement| is_gphase_instruction(&replacement.instruction))
            });
        if contains_gphase {
            self.next_workset.mark_full_scope(block_id.scope.clone());
            return;
        }

        if self.block_summary_changes(block, patches) {
            self.next_workset
                .blocks
                .insert(block_id.clone(), BlockWorkset::Full);
            return;
        }

        let (ranges, _) = dirty_ranges_after_patches(block.len(), patches, self.max_match_reach);
        self.next_workset
            .blocks
            .insert(block_id.clone(), BlockWorkset::Ranges(ranges));
    }

    fn block_summary_changes(&self, block: &[Operation], patches: &[RewritePatch]) -> bool {
        let mut old_keys = HashMap::<RewriteInstructionKey, usize>::new();
        let mut old_qubits = HashMap::<Qubit, usize>::new();
        for operation in block {
            if let Some(key) = RewriteInstructionKey::from_instruction(&operation.instruction) {
                *old_keys.entry(key).or_default() += 1;
            }
            for &qubit in &operation.qubits {
                *old_qubits.entry(qubit).or_default() += 1;
            }
        }
        let mut new_keys = old_keys.clone();
        let mut new_qubits = old_qubits.clone();
        for patch in patches {
            for &position in &patch.matched_positions {
                let operation = &block[position];
                if let Some(key) = RewriteInstructionKey::from_instruction(&operation.instruction) {
                    decrement_count(&mut new_keys, &key);
                }
                for qubit in &operation.qubits {
                    decrement_count(&mut new_qubits, qubit);
                }
            }
            for replacement in &patch.replacements {
                *new_keys.entry(replacement.key.clone()).or_default() += 1;
                for &qubit in &replacement.qubits {
                    *new_qubits.entry(qubit).or_default() += 1;
                }
            }
        }

        old_keys.keys().collect::<HashSet<_>>() != new_keys.keys().collect::<HashSet<_>>()
            || old_qubits.keys().collect::<HashSet<_>>()
                != new_qubits.keys().collect::<HashSet<_>>()
    }

    fn emit_rewritten_block(
        &mut self,
        block: &[Operation],
        patches: Vec<RewritePatch>,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        // Drives off the shared patch application plan; see
        // `patch_application_plan` for the application order.
        for step in patch_application_plan(block.len(), &patches)? {
            match step {
                PatchPlanStep::Replacements(patch) => {
                    self.stats.rules_applied += 1;
                    for replacement in &patch.replacements {
                        self.emit_replacement(replacement, target)?;
                    }
                }
                PatchPlanStep::DropMatched => {}
                PatchPlanStep::Keep(position) => {
                    let operation = &block[position];
                    self.emit_operation(
                        operation.instruction.clone(),
                        operation.qubits.clone(),
                        operation.params.as_slice(),
                        operation.label.clone(),
                        classical_remap,
                        target,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn emit_original_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        scope: &ScopeId,
        control_slot: usize,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if !self.config.recurses_control_flow() {
            return self.emit_preserved_operation(operation, classical_remap, target);
        }

        if let Instruction::ClassicalControl(control) = &operation.instruction {
            let instruction =
                self.rewrite_control_flow(control, classical_remap, scope, control_slot)?;
            let qubits = instruction.used_qubits().into_iter().collect();
            return self.emit_value_operation(
                ValueInstruction::ClassicalControl(instruction),
                qubits,
                CircuitRebuildContext::resolve_source_params(
                    self.source,
                    operation.params.as_slice(),
                )?,
                operation.label.clone(),
                target,
            );
        }

        self.emit_preserved_operation(operation, classical_remap, target)
    }

    fn rewrite_control_flow(
        &mut self,
        control: &ClassicalControlOp,
        classical_remap: &ClassicalRemap,
        parent_scope: &ScopeId,
        control_slot: usize,
    ) -> Result<ValueClassicalControlOp, CompilerError> {
        let cc = match control {
            ClassicalControlOp::If(op) => {
                let then_scope = parent_scope.child(ScopeSegment::IfThen(control_slot));
                let mut then_body = Vec::with_capacity(op.then_body().operations().len());
                let mut then_phase = Parameter::from(0.0);
                self.apply_sequence(
                    op.then_body().operations(),
                    classical_remap,
                    &then_scope,
                    LoweringTarget::control_flow_body(&mut then_body, &mut then_phase),
                )?;
                if !then_phase.is_zero() {
                    self.next_workset.mark_full_scope(then_scope);
                }
                self.prepend_body_phase(&mut then_body, then_phase);

                let else_body = op
                    .else_body()
                    .map(|body| {
                        let else_scope = parent_scope.child(ScopeSegment::IfElse(control_slot));
                        let mut rewritten = Vec::with_capacity(body.operations().len());
                        let mut body_phase = Parameter::from(0.0);
                        self.apply_sequence(
                            body.operations(),
                            classical_remap,
                            &else_scope,
                            LoweringTarget::control_flow_body(&mut rewritten, &mut body_phase),
                        )?;
                        if !body_phase.is_zero() {
                            self.next_workset.mark_full_scope(else_scope);
                        }
                        self.prepend_body_phase(&mut rewritten, body_phase);
                        Ok::<_, CompilerError>(rewritten)
                    })
                    .transpose()?;

                ValueClassicalControlOp::If {
                    condition: classical_remap.remap_expr(op.condition())?,
                    then_body: ValueControlBody::new(then_body),
                    else_body: else_body.map(ValueControlBody::new),
                }
            }
            ClassicalControlOp::While(op) => {
                let body_scope = parent_scope.child(ScopeSegment::WhileBody(control_slot));
                let mut body = Vec::with_capacity(op.body().operations().len());
                let mut body_phase = Parameter::from(0.0);
                self.apply_sequence(
                    op.body().operations(),
                    classical_remap,
                    &body_scope,
                    LoweringTarget::control_flow_body(&mut body, &mut body_phase),
                )?;
                if !body_phase.is_zero() {
                    self.next_workset.mark_full_scope(body_scope);
                }
                self.prepend_body_phase(&mut body, body_phase);

                ValueClassicalControlOp::While {
                    condition: classical_remap.remap_expr(op.condition())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::For(op) => {
                let body_scope = parent_scope.child(ScopeSegment::ForBody(control_slot));
                let mut body = Vec::with_capacity(op.body().operations().len());
                let mut body_phase = Parameter::from(0.0);
                self.apply_sequence(
                    op.body().operations(),
                    classical_remap,
                    &body_scope,
                    LoweringTarget::control_flow_body(&mut body, &mut body_phase),
                )?;
                if !body_phase.is_zero() {
                    self.next_workset.mark_full_scope(body_scope);
                }
                self.prepend_body_phase(&mut body, body_phase);

                ValueClassicalControlOp::For {
                    var: classical_remap.remap_var(op.var())?,
                    start: classical_remap.remap_expr(op.start())?,
                    stop: classical_remap.remap_expr(op.stop())?,
                    step: classical_remap.remap_expr(op.step())?,
                    body: ValueControlBody::new(body),
                }
            }
            ClassicalControlOp::Switch(op) => {
                let cases = op
                    .cases()
                    .iter()
                    .enumerate()
                    .map(|(case_index, case)| {
                        let case_scope = parent_scope.child(ScopeSegment::SwitchCase {
                            control: control_slot,
                            case: case_index,
                        });
                        let mut rewritten = Vec::with_capacity(case.body().operations().len());
                        let mut body_phase = Parameter::from(0.0);
                        self.apply_sequence(
                            case.body().operations(),
                            classical_remap,
                            &case_scope,
                            LoweringTarget::control_flow_body(&mut rewritten, &mut body_phase),
                        )?;
                        if !body_phase.is_zero() {
                            self.next_workset.mark_full_scope(case_scope);
                        }
                        self.prepend_body_phase(&mut rewritten, body_phase);
                        Ok::<_, CompilerError>(ValueSwitchCase::new(
                            case.value(),
                            ValueControlBody::new(rewritten),
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let default = op
                    .default()
                    .map(|body| {
                        let default_scope =
                            parent_scope.child(ScopeSegment::SwitchDefault(control_slot));
                        let mut rewritten = Vec::with_capacity(body.operations().len());
                        let mut body_phase = Parameter::from(0.0);
                        self.apply_sequence(
                            body.operations(),
                            classical_remap,
                            &default_scope,
                            LoweringTarget::control_flow_body(&mut rewritten, &mut body_phase),
                        )?;
                        if !body_phase.is_zero() {
                            self.next_workset.mark_full_scope(default_scope);
                        }
                        self.prepend_body_phase(&mut rewritten, body_phase);
                        Ok::<_, CompilerError>(ValueControlBody::new(rewritten))
                    })
                    .transpose()?;

                ValueClassicalControlOp::Switch {
                    target: classical_remap.remap_expr(op.target())?,
                    cases,
                    default,
                }
            }
            ClassicalControlOp::Break => ValueClassicalControlOp::Break,
            ClassicalControlOp::Continue => ValueClassicalControlOp::Continue,
        };

        Ok(cc)
    }

    fn emit_operation(
        &mut self,
        instruction: Instruction,
        qubits: SmallVec<[Qubit; 3]>,
        params: &[CircuitParam],
        label: Option<Box<str>>,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if is_gphase_instruction(&instruction) {
            if target.is_top_level() {
                self.stats.representation_changes += 1;
            }
            target.accumulate_phase(self.source_gphase_param(params)?);
            return Ok(());
        }

        let instruction = self
            .rebuild
            .remap_non_control_instruction(&instruction, classical_remap)?;
        let params = CircuitRebuildContext::resolve_source_params(self.source, params)?;
        self.emit_value_operation(instruction, qubits, params, label, target)
    }

    fn emit_preserved_operation(
        &mut self,
        operation: &Operation,
        classical_remap: &ClassicalRemap,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if is_gphase_instruction(&operation.instruction) {
            if target.is_top_level() {
                self.stats.representation_changes += 1;
            }
            target.accumulate_phase(self.source_gphase_param(&operation.params)?);
            return Ok(());
        }

        let operation =
            self.rebuild
                .remap_preserved_operation(self.source, operation, classical_remap)?;
        target.push(operation);
        Ok(())
    }

    fn emit_value_operation(
        &mut self,
        instruction: ValueInstruction,
        qubits: SmallVec<[Qubit; 3]>,
        params: SmallVec<[ParameterValue; 1]>,
        label: Option<Box<str>>,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        target.push(ValueOperation {
            instruction,
            qubits,
            params,
            label,
        });
        Ok(())
    }

    fn emit_replacement(
        &mut self,
        replacement: &ReplacementItem,
        target: &mut LoweringTarget<'_>,
    ) -> Result<(), CompilerError> {
        if is_gphase_instruction(&replacement.instruction) {
            target.accumulate_phase(Self::replacement_gphase_param(replacement)?);
            return Ok(());
        }

        let params = replacement.params.iter().cloned().collect();
        self.emit_value_operation(
            ValueInstruction::from_instruction(replacement.instruction.clone()),
            replacement.qubits.clone(),
            params,
            None,
            target,
        )
    }

    fn prepend_body_phase(&mut self, body: &mut Vec<ValueOperation>, phase: Parameter) {
        if phase.is_zero() {
            return;
        }

        body.insert(
            0,
            ValueOperation {
                instruction: ValueInstruction::from_instruction(Instruction::Standard(
                    StandardGate::GPhase,
                )),
                qubits: SmallVec::new(),
                params: smallvec::smallvec![ParameterValue::from(phase)],
                label: None,
            },
        );
    }

    fn source_gphase_param(&self, params: &[CircuitParam]) -> Result<Parameter, CompilerError> {
        if params.len() != 1 {
            return Err(CompilerError::InvariantViolation(
                "GPhase operation must contain one parameter".to_string(),
            ));
        }
        resolve_operation_param(self.source, &params[0])
    }

    fn replacement_gphase_param(replacement: &ReplacementItem) -> Result<Parameter, CompilerError> {
        let phase = replacement.params.first().ok_or_else(|| {
            CompilerError::InvariantViolation(
                "GPhase replacement must contain one parameter".to_string(),
            )
        })?;
        Ok(Parameter::from(phase))
    }
}

/// Returns whether the circuit can run on the linear workspace engine.
///
/// Eligible circuits have no classical storage and consist of a single
/// rewrite block: every operation has a rewrite instruction key (no barriers,
/// measurements, control flow, or other unkeyed instructions) and none is a
/// global-phase operation, whose absorption into the phase delta the
/// workspace does not model in its operation vector.
pub(super) fn is_linear_workspace_eligible(circuit: &Circuit) -> bool {
    circuit.classical_vars().is_empty()
        && circuit.classical_values().is_empty()
        && circuit.operations().iter().all(|operation| {
            RewriteInstructionKey::from_instruction(&operation.instruction).is_some()
                && !is_gphase_instruction(&operation.instruction)
        })
}

/// Computes the dirty anchor ranges for the round after `patches` is applied
/// to a block, plus the block length after application.
///
/// Matching at an anchor reads at most `max_match_reach` operations ahead
/// (see [`CompiledRuleSet::max_match_reach`]), so a patch covering
/// `first..=last` can only change match results for anchors in
/// `[first - max_match_reach, last]`. Ranges are computed in the
/// post-application coordinate space by tracking the running length delta,
/// and each patch's full span — including the skipped positions between its
/// matched ones — is dirtied because the selector's overlap rule owns the
/// whole span.
///
/// Global-phase replacements never occupy a block position (they are folded
/// into the phase delta), so they are excluded from the inserted count. In
/// the degenerate case where every range clamps away but operations remain
/// (only reachable with a zero maximum reach), a small prefix range is
/// returned so the next round never starts from an unjustified "fully clean"
/// state; those extra anchors cannot produce candidates a full scan would
/// not also find.
fn dirty_ranges_after_patches(
    old_len: usize,
    patches: &[RewritePatch],
    max_match_reach: usize,
) -> (Vec<Range<usize>>, usize) {
    let mut ranges = Vec::with_capacity(patches.len());
    let mut delta = 0isize;
    let mut new_len = old_len as isize;
    for patch in patches {
        let inserted = patch
            .replacements
            .iter()
            .filter(|replacement| !is_gphase_instruction(&replacement.instruction))
            .count() as isize;
        let removed = patch.matched_positions.len() as isize;
        let new_start = (patch.first_position as isize + delta).max(0) as usize;
        delta += inserted - removed;
        new_len += inserted - removed;
        let after_span = (patch.last_position as isize + 1 + delta).max(0) as usize;
        let dirty_start = new_start.saturating_sub(max_match_reach);
        let dirty_end = after_span.max(new_start.saturating_add(1));
        ranges.push(dirty_start..dirty_end);
    }

    let new_len = new_len.max(0) as usize;
    for range in &mut ranges {
        range.end = range.end.min(new_len);
    }
    ranges.retain(|range| range.start < range.end);
    merge_ranges(&mut ranges);
    if ranges.is_empty() && new_len > 0 {
        ranges.push(0..new_len.min(max_match_reach.saturating_add(1)));
    }
    (ranges, new_len)
}

/// Splices the selected patches into the linear workspace operation vector.
///
/// Drives off the shared [`patch_application_plan`]; see its documentation
/// for the application order. Global-phase replacements are folded into the
/// returned phase delta instead of occupying a position. Replacement
/// operations carry placeholder `Fixed(0.0)` parameters because the real
/// resolved parameters live in the round's [`BlockMatchCache`]; matching and
/// final materialization only ever read parameter values from the cache,
/// while the placeholder preserves the correct parameter count for local
/// cost accounting.
fn apply_linear_patches(
    operations: &[Operation],
    patches: &[RewritePatch],
) -> Result<(Vec<Operation>, Parameter), CompilerError> {
    let steps = patch_application_plan(operations.len(), patches)?;
    let mut output = Vec::with_capacity(operations.len());
    let mut phase_delta = Parameter::from(0.0);
    let mut operations = operations.iter();
    for step in steps {
        match step {
            PatchPlanStep::Replacements(patch) => {
                for replacement in &patch.replacements {
                    if is_gphase_instruction(&replacement.instruction) {
                        phase_delta =
                            &phase_delta + &RoundRewriter::replacement_gphase_param(replacement)?;
                        continue;
                    }
                    output.push(Operation {
                        instruction: replacement.instruction.clone(),
                        qubits: replacement.qubits.clone(),
                        params: replacement
                            .params
                            .iter()
                            .map(|_| CircuitParam::Fixed(0.0))
                            .collect(),
                        label: None,
                    });
                }
            }
            PatchPlanStep::DropMatched => {
                operations.next();
            }
            PatchPlanStep::Keep(_) => {
                output.push(
                    operations
                        .next()
                        .expect(
                            "patch application plan produces exactly one step per block position",
                        )
                        .clone(),
                );
            }
        }
    }
    Ok((output, phase_delta))
}

fn decrement_count<K>(counts: &mut HashMap<K, usize>, key: &K)
where
    K: Eq + Hash,
{
    let remove = counts.get_mut(key).is_some_and(|count| {
        *count = count.saturating_sub(1);
        *count == 0
    });
    if remove {
        counts.remove(key);
    }
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_by_key(|range| range.start);
    let mut merged = Vec::<Range<usize>>::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    *ranges = merged;
}

fn recursive_operation_count(operations: &[Operation]) -> usize {
    let mut count = operations.len();
    for operation in operations {
        let Instruction::ClassicalControl(control) = &operation.instruction else {
            continue;
        };
        count = count.saturating_add(match control {
            ClassicalControlOp::If(op) => recursive_operation_count(op.then_body().operations())
                .saturating_add(
                    op.else_body()
                        .map_or(0, |body| recursive_operation_count(body.operations())),
                ),
            ClassicalControlOp::While(op) => recursive_operation_count(op.body().operations()),
            ClassicalControlOp::For(op) => recursive_operation_count(op.body().operations()),
            ClassicalControlOp::Switch(op) => op
                .cases()
                .iter()
                .map(|case| recursive_operation_count(case.body().operations()))
                .fold(0usize, usize::saturating_add)
                .saturating_add(
                    op.default()
                        .map_or(0, |body| recursive_operation_count(body.operations())),
                ),
            ClassicalControlOp::Break | ClassicalControlOp::Continue => 0,
        });
    }
    count
}
