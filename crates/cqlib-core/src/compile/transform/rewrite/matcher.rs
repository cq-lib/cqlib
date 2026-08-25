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

//! Rule compilation and dependency-aware sequence matching.
//!
//! The matcher prepares the knowledge-rule library for repeated local scans.
//! Rules are indexed by their first instruction key, filtered by rewrite mode,
//! qubit width, target-basis constraints, and static cost before any expensive
//! matching is attempted.
//!
//! Matching is dependency-aware rather than strictly contiguous. A rule item may
//! be matched after commuting past unrelated operations when the compiler
//! commutation checker proves that reordering is safe for the candidate block.
//! This lets local identities apply across independent gates without changing
//! the relative order of non-commuting operations.
//!
//! Every successful match becomes a candidate patch with a before/after local
//! cost. Candidate patches are sorted by final cost, original cost reduction,
//! covered span, static size delta, source position, and rule id. The selector
//! then greedily keeps the first non-overlapping patches, producing a stable
//! patch set that never rewrites the same operation span twice in one round.
//!
//! Incremental fixpoint execution reuses a per-block [`BlockMatchCache`]
//! across rounds and restricts rescans to dirty anchor ranges derived from
//! the previously applied patches; see [`select_rewrites_for_anchor_ranges`]
//! for the exact workset contract. When dirty anchors cover at least
//! [`DIRTY_FULL_SCAN_PERCENT`] percent of the eligible anchors, the block is
//! rescanned in full because incremental bookkeeping stops paying off.
//!
//! Candidate generation for large anchor sets runs on rayon. This is
//! deterministic: workers accumulate candidates locally, and the comparator
//! in [`select_candidate_patches`] is a strict total order (source position
//! and rule id break all ties), so the selected patch set never depends on
//! thread scheduling.

use crate::circuit::{
    Circuit, CircuitParam, Instruction, Operation, Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::commutation::{CommutationChecker, CommutationConfig};
use crate::compile::error::CompilerError;
use crate::compile::knowledge::library::{RuleKind, RuleLibrary};
use crate::compile::knowledge::matcher::KnowledgeInstructionKey as RewriteInstructionKey;
use crate::compile::knowledge::matcher::{
    ConcreteOperationView, MatchBindings, conditions_hold as knowledge_conditions_hold,
    instantiate_target as knowledge_instantiate_target,
    match_rule_item_with_keys as knowledge_match_rule_item_with_keys,
};
use crate::compile::knowledge::rule::{Condition, Rule, RuleItem};
use crate::compile::transform::rewrite::basis::TargetContext;
use crate::compile::transform::rewrite::config::{GPhaseCost, LocalRewriteCost, RewriteConfig};
use crate::compile::transform::rewrite::diagnostics::MatcherDiagnostics;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use symb_anafis::CompiledEvaluator;

/// Anchor count above which candidate generation switches to rayon.
///
/// Below this threshold, thread-pool dispatch costs more than the matching
/// work it saves. This knob is independent of
/// `SMALL_CIRCUIT_FULL_SCAN_THRESHOLD` in `rewriter.rs` (which selects the
/// incremental engines); the two intentionally live in separate files next to
/// the code they tune, so adjust each only against its own measurements.
const PARALLEL_ANCHOR_THRESHOLD: usize = 4_096;
/// Dirty-to-eligible anchor percentage at which a block rescan falls back to
/// a full scan. Past this point the restricted scan touches most of the block
/// anyway, and full scans have cheaper per-anchor bookkeeping.
const DIRTY_FULL_SCAN_PERCENT: usize = 50;
/// Shards used by the pass-local exact-commutation memo.
///
/// Large anchor scans run on rayon, so unrelated operation pairs should not
/// contend on one global lock. The shard count is a power of two so selecting
/// a shard remains cheaper than another hash-table lookup.
const COMMUTATION_CACHE_SHARDS: usize = 64;
/// Shards and bounded admission for parameter-only rule evaluation results.
///
/// The cache is owned by a rewrite workspace, never process-global.  A fixed
/// per-shard ceiling keeps adversarial symbolic workloads from growing it
/// without bound while retaining lock locality for parallel anchor scans.
const RULE_EVALUATION_CACHE_SHARDS: usize = 64;
const RULE_EVALUATION_CACHE_ENTRIES_PER_SHARD: usize = 64;
/// Very short windows are cheaper to scan linearly than to binary-search.
const OCCURRENCE_INDEX_MIN_WINDOW: usize = 8;
/// Dense keys retain contiguous scanning to avoid index indirection.
const OCCURRENCE_INDEX_MAX_DENSITY_PERCENT: usize = 25;

type OperationId = u64;
type CommutationCacheKey = (OperationId, OperationId);

/// Cached outcome of an exact-commutation proof attempt.
///
/// Absence from the map is the third state, `Unknown`. A negative entry means
/// only that the configured checker did not prove exact commutation; it is not
/// a proof that the operations fail to commute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExactCommutationState {
    Exact,
    NotProvenExact,
}

/// Exact-commutation lookup used by serial and parallel anchor scans.
///
/// Stable operation IDs, rather than mutable positions, identify operands so
/// retained source pairs can reuse results across fixpoint rounds.
trait ExactCommutationCache {
    fn exact_or_insert_with(
        &self,
        lhs_id: OperationId,
        rhs_id: OperationId,
        compute: impl FnOnce() -> bool,
    ) -> bool;
}

#[derive(Debug, Default)]
struct ExactCommutationShard {
    entries: HashMap<CommutationCacheKey, ExactCommutationState>,
}

/// Cross-round exact-commutation results for one rewrite block.
///
/// Stable operation IDs make entries independent of positional shifts. A
/// replacement always receives a fresh ID, which makes entries involving a
/// consumed source operation unreachable without an eager invalidation walk.
/// All entries are reclaimed together when the owning block cache is dropped.
#[derive(Debug)]
struct ExactCommutationMemo {
    shards: [Mutex<ExactCommutationShard>; COMMUTATION_CACHE_SHARDS],
    next_operation_id: AtomicU64,
}

impl ExactCommutationMemo {
    fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(ExactCommutationShard::default())),
            next_operation_id: AtomicU64::new(0),
        }
    }

    fn allocate_operation_id(&self) -> OperationId {
        self.next_operation_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl ExactCommutationCache for ExactCommutationMemo {
    fn exact_or_insert_with(
        &self,
        lhs_id: OperationId,
        rhs_id: OperationId,
        compute: impl FnOnce() -> bool,
    ) -> bool {
        let key = normalized_operation_pair(lhs_id, rhs_id);
        let shard_index = operation_pair_shard(key);
        let mut shard = self.shards[shard_index]
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = shard.entries.get(&key) {
            return *state == ExactCommutationState::Exact;
        }

        let exact = compute();
        shard.entries.insert(
            key,
            if exact {
                ExactCommutationState::Exact
            } else {
                ExactCommutationState::NotProvenExact
            },
        );
        exact
    }
}

/// Lock-free front cache for one serial scan or rayon worker.
///
/// The persistent memo is consulted only once per unique pair in a selection;
/// repeated rule and future-match queries then stay in this `RefCell` map.
struct LocalExactCommutationCache<'a> {
    entries: RefCell<HashMap<CommutationCacheKey, ExactCommutationState>>,
    memo: &'a ExactCommutationMemo,
}

impl<'a> LocalExactCommutationCache<'a> {
    fn new(memo: &'a ExactCommutationMemo) -> Self {
        Self {
            entries: RefCell::new(HashMap::new()),
            memo,
        }
    }
}

impl ExactCommutationCache for LocalExactCommutationCache<'_> {
    fn exact_or_insert_with(
        &self,
        lhs_id: OperationId,
        rhs_id: OperationId,
        compute: impl FnOnce() -> bool,
    ) -> bool {
        let key = normalized_operation_pair(lhs_id, rhs_id);
        if let Some(state) = self.entries.borrow().get(&key) {
            return *state == ExactCommutationState::Exact;
        }

        let exact = self.memo.exact_or_insert_with(lhs_id, rhs_id, compute);
        self.entries.borrow_mut().insert(
            key,
            if exact {
                ExactCommutationState::Exact
            } else {
                ExactCommutationState::NotProvenExact
            },
        );
        exact
    }
}

fn normalized_operation_pair(lhs: OperationId, rhs: OperationId) -> CommutationCacheKey {
    if lhs <= rhs { (lhs, rhs) } else { (rhs, lhs) }
}

fn operation_pair_shard((lhs, rhs): CommutationCacheKey) -> usize {
    let mixed = lhs.wrapping_mul(0x9e37_79b9_u64) ^ rhs.rotate_left(u64::BITS / 2);
    (mixed as usize) & (COMMUTATION_CACHE_SHARDS - 1)
}

/// Returns whether the instruction is a global-phase operation.
///
/// Global-phase operations never occupy a block position after a round: they
/// are absorbed into the owning sequence's phase delta instead of being
/// emitted. Every patch consumer and every dirty-range computation must
/// classify them identically, so this predicate is the single source of
/// truth shared by `rewriter.rs` and this module.
pub(super) fn is_gphase_instruction(instruction: &Instruction) -> bool {
    matches!(instruction, Instruction::Standard(StandardGate::GPhase))
}

/// A rewrite rule prepared for repeated matching.
struct CompiledRule {
    id: usize,
    kind: RuleKind,
    match_len: usize,
    qubit_count: usize,
    static_cost_delta: isize,
    preserves_two_qubit_connectivity: bool,
    source_keys: SmallVec<[RewriteInstructionKey; 8]>,
    match_keys: SmallVec<[RewriteInstructionKey; 4]>,
    rewrite_keys: SmallVec<[RewriteInstructionKey; 4]>,
    parameter_symbols: SmallVec<[String; 4]>,
    numeric_conditions: Option<CompiledConditions>,
    rule: Rule,
}

struct CompiledConditions {
    /// Rule-bound symbols first, followed by builtin constants used by the
    /// expression compiler.  Only the rule-bound prefix participates in the
    /// cache key.
    evaluator_parameters: Vec<String>,
    binding_parameter_count: usize,
    conditions: Vec<CompiledCondition>,
}

enum CompiledCondition {
    Eq {
        lhs: CompiledEvaluator,
        rhs: CompiledEvaluator,
    },
    EqMod {
        lhs: CompiledEvaluator,
        rhs: CompiledEvaluator,
        modulus: CompiledEvaluator,
    },
}

impl CompiledConditions {
    fn compile(conditions: &[Condition]) -> Option<Self> {
        if conditions.is_empty() {
            return Some(Self {
                evaluator_parameters: Vec::new(),
                binding_parameter_count: 0,
                conditions: Vec::new(),
            });
        }

        let mut binding_parameters = conditions
            .iter()
            .flat_map(Condition::symbols)
            // The rule DSL treats these names as mathematical constants, not
            // match bindings. Keep them in the evaluator's fixed suffix so
            // the parameter order and value vector remain exactly aligned.
            .filter(|symbol| !matches!(symbol.as_str(), "π" | "pi" | "e"))
            .collect::<Vec<_>>();
        binding_parameters.sort_unstable();
        binding_parameters.dedup();
        let binding_parameter_count = binding_parameters.len();
        let mut evaluator_parameters = binding_parameters;
        evaluator_parameters.extend(["π".to_string(), "pi".to_string(), "e".to_string()]);
        let parameter_order = evaluator_parameters
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let compile = |parameter: &Parameter| {
            CompiledEvaluator::compile(parameter.as_expr(), &parameter_order, None).ok()
        };

        let mut compiled = Vec::with_capacity(conditions.len());
        for condition in conditions {
            compiled.push(match condition {
                Condition::Eq(lhs, rhs) => CompiledCondition::Eq {
                    lhs: compile(lhs)?,
                    rhs: compile(rhs)?,
                },
                Condition::EqMod(lhs, rhs, modulus) => CompiledCondition::EqMod {
                    lhs: compile(lhs)?,
                    rhs: compile(rhs)?,
                    modulus: compile(modulus)?,
                },
            });
        }
        Some(Self {
            evaluator_parameters,
            binding_parameter_count,
            conditions: compiled,
        })
    }

    /// Returns `None` unless every referenced binding and expression can be
    /// evaluated to a finite number.  `None` deliberately means "use the
    /// symbolic implementation", never "condition failed".
    fn evaluate(&self, bindings: &MatchBindings) -> Option<bool> {
        let mut values = SmallVec::<[f64; 8]>::with_capacity(self.evaluator_parameters.len());
        for name in &self.evaluator_parameters[..self.binding_parameter_count] {
            let value = bindings.param(name)?.evaluate(&None).ok()?;
            if !value.is_finite() {
                return None;
            }
            values.push(value);
        }
        values.extend([
            std::f64::consts::PI,
            std::f64::consts::PI,
            std::f64::consts::E,
        ]);

        self.conditions
            .iter()
            .try_fold(true, |all_hold, condition| {
                if !all_hold {
                    return Some(false);
                }
                match condition {
                    CompiledCondition::Eq { lhs, rhs } => {
                        let lhs = lhs.evaluate(&values);
                        let rhs = rhs.evaluate(&values);
                        let diff = lhs - rhs;
                        (lhs.is_finite() && rhs.is_finite() && diff.is_finite())
                            .then_some(diff.abs() <= crate::compile::PARAMETER_EQ_TOLERANCE)
                    }
                    CompiledCondition::EqMod { lhs, rhs, modulus } => {
                        let lhs = lhs.evaluate(&values);
                        let rhs = rhs.evaluate(&values);
                        let modulus = modulus.evaluate(&values);
                        if !lhs.is_finite() || !rhs.is_finite() || !modulus.is_finite() {
                            return None;
                        }
                        let diff = lhs - rhs;
                        if !diff.is_finite() {
                            return None;
                        }
                        if diff.abs() <= crate::compile::PARAMETER_EQ_TOLERANCE {
                            return Some(true);
                        }
                        if modulus.abs() <= crate::compile::PARAMETER_EQ_TOLERANCE {
                            return Some(false);
                        }
                        let ratio = diff / modulus;
                        ratio.is_finite().then_some(
                            (ratio - ratio.round()).abs() <= crate::compile::PARAMETER_EQ_TOLERANCE,
                        )
                    }
                }
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RuleBindingCacheKey {
    rule_id: usize,
    parameters: SmallVec<[Parameter; 4]>,
}

#[derive(Debug, Clone)]
struct ReplacementTemplateItem {
    instruction: Instruction,
    rule_qubits: SmallVec<[u32; 3]>,
    params: SmallVec<[ParameterValue; 3]>,
    key: RewriteInstructionKey,
}

#[derive(Debug, Clone, Default)]
struct RuleEvaluationCacheEntry {
    condition: Option<bool>,
    replacement: Option<Arc<[ReplacementTemplateItem]>>,
}

#[derive(Debug, Default)]
struct RuleEvaluationCacheShard {
    entries: HashMap<RuleBindingCacheKey, RuleEvaluationCacheEntry>,
}

#[derive(Debug)]
struct RuleEvaluationCache {
    shards: [Mutex<RuleEvaluationCacheShard>; RULE_EVALUATION_CACHE_SHARDS],
    collect_diagnostics: bool,
    condition_cache_hits: AtomicUsize,
    condition_cache_misses: AtomicUsize,
    symbolic_fallbacks: AtomicUsize,
    dirty_anchors: AtomicUsize,
    full_scan_fallbacks: AtomicUsize,
}

impl RuleEvaluationCache {
    fn new(collect_diagnostics: bool) -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(RuleEvaluationCacheShard::default())),
            collect_diagnostics,
            condition_cache_hits: AtomicUsize::new(0),
            condition_cache_misses: AtomicUsize::new(0),
            symbolic_fallbacks: AtomicUsize::new(0),
            dirty_anchors: AtomicUsize::new(0),
            full_scan_fallbacks: AtomicUsize::new(0),
        }
    }

    fn condition_or_insert_with(
        &self,
        key: RuleBindingCacheKey,
        compute: impl FnOnce() -> bool,
    ) -> bool {
        let mut shard = self
            .shard(&key)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(result) = shard.entries.get(&key).and_then(|entry| entry.condition) {
            if self.collect_diagnostics {
                self.condition_cache_hits.fetch_add(1, Ordering::Relaxed);
            }
            return result;
        }

        if self.collect_diagnostics {
            self.condition_cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        let result = compute();
        if let Some(entry) = shard.entries.get_mut(&key) {
            entry.condition = Some(result);
        } else if shard.entries.len() < RULE_EVALUATION_CACHE_ENTRIES_PER_SHARD {
            shard.entries.insert(
                key,
                RuleEvaluationCacheEntry {
                    condition: Some(result),
                    replacement: None,
                },
            );
        }
        result
    }

    fn replacement(&self, key: &RuleBindingCacheKey) -> Option<Arc<[ReplacementTemplateItem]>> {
        self.shard(key)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entries
            .get(key)
            .and_then(|entry| entry.replacement.clone())
    }

    fn record_replacement(
        &self,
        key: RuleBindingCacheKey,
        replacement: Arc<[ReplacementTemplateItem]>,
    ) {
        self.record(key, |entry| entry.replacement = Some(replacement));
    }

    fn record(&self, key: RuleBindingCacheKey, update: impl FnOnce(&mut RuleEvaluationCacheEntry)) {
        let mut shard = self
            .shard(&key)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = shard.entries.get_mut(&key) {
            update(entry);
        } else if shard.entries.len() < RULE_EVALUATION_CACHE_ENTRIES_PER_SHARD {
            let mut entry = RuleEvaluationCacheEntry::default();
            update(&mut entry);
            shard.entries.insert(key, entry);
        }
    }

    fn shard(&self, key: &RuleBindingCacheKey) -> &Mutex<RuleEvaluationCacheShard> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        &self.shards[(hasher.finish() as usize) & (RULE_EVALUATION_CACHE_SHARDS - 1)]
    }

    fn record_symbolic_fallback(&self) {
        if self.collect_diagnostics {
            self.symbolic_fallbacks.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_incremental_scan(&self, dirty_anchors: usize, full_scan_fallback: bool) {
        if !self.collect_diagnostics {
            return;
        }
        self.dirty_anchors
            .fetch_add(dirty_anchors, Ordering::Relaxed);
        if full_scan_fallback {
            self.full_scan_fallbacks.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn diagnostics(&self) -> MatcherDiagnostics {
        if !self.collect_diagnostics {
            return MatcherDiagnostics::default();
        }
        MatcherDiagnostics {
            condition_cache_hits: self.condition_cache_hits.load(Ordering::Relaxed),
            condition_cache_misses: self.condition_cache_misses.load(Ordering::Relaxed),
            symbolic_fallbacks: self.symbolic_fallbacks.load(Ordering::Relaxed),
            dirty_anchors: self.dirty_anchors.load(Ordering::Relaxed),
            full_scan_fallbacks: self.full_scan_fallbacks.load(Ordering::Relaxed),
        }
    }
}

/// Compiled rule collection with a first-instruction candidate index.
pub(super) struct CompiledRuleSet {
    rules: Vec<CompiledRule>,
    first_key_map: HashMap<RewriteInstructionKey, SmallVec<[usize; 8]>>,
    commutation: CommutationChecker,
    qubit_bijection_invariant: bool,
}

/// Rules admitted by configuration and target constraints for one rewrite run.
///
/// The bitset is refined with block-local summaries before anchor scanning, so
/// the hot loop only performs one indexed bit test per first-key candidate.
pub(super) struct RunActiveRuleSet {
    bits: Vec<u64>,
}

struct BlockActiveRuleSet {
    bits: Vec<u64>,
}

impl RunActiveRuleSet {
    fn contains(&self, rule_index: usize) -> bool {
        bitset_contains(&self.bits, rule_index)
    }

    fn for_block(&self, rules: &CompiledRuleSet, block: &BlockContext<'_>) -> BlockActiveRuleSet {
        let mut bits = vec![0; self.bits.len()];
        for (rule_index, rule) in rules.rules.iter().enumerate() {
            if self.contains(rule_index) && rule_passes_block_filters(rule, block) {
                bitset_insert(&mut bits, rule_index);
            }
        }
        BlockActiveRuleSet { bits }
    }
}

impl BlockActiveRuleSet {
    fn contains(&self, rule_index: usize) -> bool {
        bitset_contains(&self.bits, rule_index)
    }
}

fn bitset_insert(bits: &mut [u64], index: usize) {
    bits[index / u64::BITS as usize] |= 1 << (index % u64::BITS as usize);
}

fn bitset_contains(bits: &[u64], index: usize) -> bool {
    bits.get(index / u64::BITS as usize)
        .is_some_and(|word| word & (1 << (index % u64::BITS as usize)) != 0)
}

/// One operation emitted by a rewrite target.
#[derive(Debug, Clone)]
pub(super) struct ReplacementItem {
    pub(super) instruction: Instruction,
    pub(super) qubits: SmallVec<[Qubit; 3]>,
    pub(super) params: SmallVec<[ParameterValue; 3]>,
    pub(super) key: RewriteInstructionKey,
}

/// A selected replacement for matched operation positions in one block.
#[derive(Debug, Clone)]
pub(super) struct RewritePatch {
    pub(super) rule_id: usize,
    static_cost_delta: isize,
    pub(super) first_position: usize,
    pub(super) last_position: usize,
    pub(super) matched_positions: Vec<usize>,
    pub(super) replacements: Vec<ReplacementItem>,
}

/// One step of the shared patch application plan for a block.
///
/// Applying a set of selected patches always means the same walk over the
/// block: emit each patch's replacements at its first matched position, drop
/// the remaining matched positions, and keep every other position in place.
/// All three patch consumers — the rebuilding emitter and the linear
/// workspace splice in `rewriter.rs`, and [`BlockMatchCache::into_rewritten`]
/// here — drive off this single plan so their application order cannot
/// diverge.
pub(super) enum PatchPlanStep<'a> {
    /// Emit the patch's replacements; occurs at the patch's first matched
    /// position, before the step for that position itself.
    Replacements(&'a RewritePatch),
    /// Drop the matched source operation at this position.
    DropMatched,
    /// Keep the source operation at this position unchanged.
    Keep(usize),
}

/// Builds the shared application plan for applying `patches` to a block of
/// `block_len` operations.
///
/// The plan is the linearization of "patches by first position plus matched
/// positions to skip" that every patch consumer previously computed by hand.
/// Steps are ordered by source position; a [`PatchPlanStep::Replacements`]
/// step immediately precedes the [`PatchPlanStep::DropMatched`] step of its
/// patch's first matched position. Errors if any patch references a matched
/// position outside the block, which would indicate a selector bug.
pub(super) fn patch_application_plan(
    block_len: usize,
    patches: &[RewritePatch],
) -> Result<Vec<PatchPlanStep<'_>>, CompilerError> {
    let mut patches_by_start = HashMap::with_capacity(patches.len());
    let mut matched = vec![false; block_len];
    for patch in patches {
        patches_by_start.insert(patch.first_position, patch);
        for &position in &patch.matched_positions {
            let Some(entry) = matched.get_mut(position) else {
                return Err(CompilerError::InvariantViolation(format!(
                    "rewrite patch matched position {position} outside block of length {block_len}"
                )));
            };
            *entry = true;
        }
    }

    let mut steps = Vec::with_capacity(block_len + patches.len());
    for (position, &is_matched) in matched.iter().enumerate() {
        if let Some(patch) = patches_by_start.remove(&position) {
            steps.push(PatchPlanStep::Replacements(patch));
        }
        if is_matched {
            steps.push(PatchPlanStep::DropMatched);
        } else {
            steps.push(PatchPlanStep::Keep(position));
        }
    }
    Ok(steps)
}

#[derive(Debug, Clone)]
struct CandidatePatch {
    patch: RewritePatch,
    before: LocalRewriteCost,
    after: LocalRewriteCost,
}

/// Per-block matching inputs cached across fixpoint rounds.
///
/// Rule matching probes the same operation many times across anchors, rules,
/// and commutation checks, so per-position instruction keys and resolved
/// symbolic parameters are computed once and reused. Stable operation IDs and
/// the shared commutation memo preserve source-pair proofs across incremental
/// rounds. `instruction_positions` and `qubit_count` feed static rule filters.
/// The ordered positions also skip unrelated keys during sparse window scans. A
/// transition does not by itself require a full rescan: a newly admitted local
/// match must observe the patch which introduced its missing instruction or
/// qubit.
#[derive(Debug, Clone)]
pub(super) struct BlockMatchCache {
    instruction_keys: Vec<RewriteInstructionKey>,
    resolved_params: Vec<SmallVec<[Parameter; 3]>>,
    operation_ids: Vec<OperationId>,
    commutation_memo: Arc<ExactCommutationMemo>,
    rule_evaluation_cache: Arc<RuleEvaluationCache>,
    instruction_positions: HashMap<RewriteInstructionKey, Vec<usize>>,
    qubit_count: usize,
}

impl BlockMatchCache {
    pub(super) fn new_with_diagnostics(
        circuit: &Circuit,
        operations: &[Operation],
        collect_diagnostics: bool,
    ) -> Result<Self, CompilerError> {
        let mut resolved_params = Vec::with_capacity(operations.len());
        let mut instruction_keys = Vec::with_capacity(operations.len());
        let mut touched_qubits = HashSet::new();
        let mut instruction_positions = HashMap::<RewriteInstructionKey, Vec<usize>>::new();
        let commutation_memo = Arc::new(ExactCommutationMemo::new());
        let rule_evaluation_cache = Arc::new(RuleEvaluationCache::new(collect_diagnostics));
        let mut operation_ids = Vec::with_capacity(operations.len());

        for operation in operations {
            let key = RewriteInstructionKey::from_instruction(&operation.instruction).ok_or_else(
                || {
                    CompilerError::InvariantViolation(format!(
                        "rewrite block contains unsupported instruction {:?}",
                        operation.instruction
                    ))
                },
            )?;
            let params = operation
                .params
                .iter()
                .map(|param| resolve_operation_param(circuit, param))
                .collect::<Result<SmallVec<[_; 3]>, _>>()?;

            touched_qubits.extend(operation.qubits.iter().copied());
            instruction_positions
                .entry(key.clone())
                .or_default()
                .push(instruction_keys.len());
            instruction_keys.push(key);
            resolved_params.push(params);
            operation_ids.push(commutation_memo.allocate_operation_id());
        }

        Ok(Self {
            instruction_keys,
            resolved_params,
            operation_ids,
            commutation_memo,
            rule_evaluation_cache,
            instruction_positions,
            qubit_count: touched_qubits.len(),
        })
    }

    pub(super) fn len(&self) -> usize {
        self.instruction_keys.len()
    }

    pub(super) fn params(&self, position: usize) -> &[Parameter] {
        &self.resolved_params[position]
    }

    pub(super) fn diagnostics(&self) -> MatcherDiagnostics {
        self.rule_evaluation_cache.diagnostics()
    }

    /// Advances cached per-operation match inputs through a verified linear
    /// edit script.  Clean entries retain resolved parameters and instruction
    /// keys; only replacement ranges are decoded from the new circuit.
    ///
    /// A routing qubit renaming invalidates the commutation memo and stable
    /// operation IDs because exact commutation depends on concrete qargs.  The
    /// parameter-only rule cache remains valid: its replacement entries use
    /// rule-local qubit labels and are remapped for every match.
    pub(super) fn after_linear_replacements(
        self,
        after: &Circuit,
        replacements: &[super::edit::OperationReplacement],
        qubits_renamed: bool,
    ) -> Option<Self> {
        let BlockMatchCache {
            instruction_keys,
            resolved_params,
            operation_ids,
            commutation_memo: old_commutation_memo,
            rule_evaluation_cache,
            instruction_positions: _,
            qubit_count: _,
        } = self;
        let old_len = instruction_keys.len();
        let new_len = after.operations().len();
        let mut old_entries = instruction_keys
            .into_iter()
            .zip(resolved_params)
            .zip(operation_ids);
        let commutation_memo = if qubits_renamed {
            Arc::new(ExactCommutationMemo::new())
        } else {
            old_commutation_memo
        };
        let mut new_keys = Vec::with_capacity(new_len);
        let mut new_params = Vec::with_capacity(new_len);
        let mut new_ids = Vec::with_capacity(new_len);
        let mut instruction_positions = HashMap::<RewriteInstructionKey, Vec<usize>>::new();
        let mut old_cursor = 0usize;

        let push_old =
            |entry: (
                (RewriteInstructionKey, SmallVec<[Parameter; 3]>),
                OperationId,
            ),
             keys: &mut Vec<RewriteInstructionKey>,
             params: &mut Vec<SmallVec<[Parameter; 3]>>,
             ids: &mut Vec<OperationId>,
             instruction_positions: &mut HashMap<RewriteInstructionKey, Vec<usize>>| {
                let ((key, resolved), old_id) = entry;
                instruction_positions
                    .entry(key.clone())
                    .or_default()
                    .push(keys.len());
                keys.push(key);
                params.push(resolved);
                ids.push(if qubits_renamed {
                    commutation_memo.allocate_operation_id()
                } else {
                    old_id
                });
            };

        for replacement in replacements {
            while old_cursor < replacement.old.start {
                push_old(
                    old_entries.next()?,
                    &mut new_keys,
                    &mut new_params,
                    &mut new_ids,
                    &mut instruction_positions,
                );
                old_cursor += 1;
            }
            while old_cursor < replacement.old.end {
                old_entries.next()?;
                old_cursor += 1;
            }
            for operation in &after.operations()[replacement.new.clone()] {
                let key = RewriteInstructionKey::from_instruction(&operation.instruction)?;
                let params = operation
                    .params
                    .iter()
                    .map(|parameter| resolve_operation_param(after, parameter))
                    .collect::<Result<SmallVec<[_; 3]>, _>>()
                    .ok()?;
                instruction_positions
                    .entry(key.clone())
                    .or_default()
                    .push(new_keys.len());
                new_keys.push(key);
                new_params.push(params);
                new_ids.push(commutation_memo.allocate_operation_id());
            }
        }
        while old_cursor < old_len {
            push_old(
                old_entries.next()?,
                &mut new_keys,
                &mut new_params,
                &mut new_ids,
                &mut instruction_positions,
            );
            old_cursor += 1;
        }
        if old_entries.next().is_some()
            || new_keys.len() != new_len
            || new_params.len() != new_len
            || new_ids.len() != new_len
        {
            return None;
        }

        Some(Self {
            instruction_keys: new_keys,
            resolved_params: new_params,
            operation_ids: new_ids,
            commutation_memo,
            rule_evaluation_cache,
            instruction_positions,
            // Using the declared circuit domain is a conservative
            // overestimate for the static width filter and avoids rescanning
            // every clean operation's qargs after an edit.
            qubit_count: after.qubits().len(),
        })
    }

    /// Like [`BlockMatchCache::into_rewritten`], but clones the cache first
    /// and discards the summary-change flag.
    pub(super) fn rewritten(
        &self,
        operations: &[Operation],
        patches: &[RewritePatch],
    ) -> Option<Self> {
        self.clone().into_rewritten(operations, patches)
    }

    /// Derives the cache for the block after applying `patches`, consuming
    /// the old cache so unchanged entries move instead of clone.
    ///
    /// Returns `None` when the cache cannot be derived incrementally: the
    /// operation slice no longer matches the cached length, the source block
    /// contains a global-phase operation (its absorption into the phase delta
    /// would shift content this incremental view cannot model), or a patch
    /// references a position outside the block. Callers must treat `None` as
    /// "rebuild from scratch or rescan fully", never as an error.
    ///
    /// Global-phase replacements never occupy a block position (the emitter
    /// folds them into the phase delta), so they are skipped here as well;
    /// every other replacement contributes its key, resolved parameters, and
    /// touched qubits.
    pub(super) fn into_rewritten(
        self,
        operations: &[Operation],
        patches: &[RewritePatch],
    ) -> Option<Self> {
        if operations.len() != self.len()
            || operations
                .iter()
                .any(|operation| is_gphase_instruction(&operation.instruction))
        {
            return None;
        }

        let steps = patch_application_plan(operations.len(), patches).ok()?;
        let expected_len = operations
            .len()
            .saturating_add(
                patches
                    .iter()
                    .map(|patch| {
                        patch
                            .replacements
                            .iter()
                            .filter(|replacement| !is_gphase_instruction(&replacement.instruction))
                            .count()
                    })
                    .sum::<usize>(),
            )
            .saturating_sub(
                patches
                    .iter()
                    .map(|patch| patch.matched_positions.len())
                    .sum::<usize>(),
            );
        let mut instruction_keys = Vec::with_capacity(expected_len);
        let mut resolved_params = Vec::with_capacity(expected_len);
        let mut operation_ids = Vec::with_capacity(expected_len);
        let mut instruction_positions = HashMap::<RewriteInstructionKey, Vec<usize>>::new();
        let mut touched_qubits = HashSet::new();
        let commutation_memo = self.commutation_memo;
        let rule_evaluation_cache = self.rule_evaluation_cache;
        let mut old_entries = self
            .instruction_keys
            .into_iter()
            .zip(self.resolved_params)
            .zip(self.operation_ids);

        for step in steps {
            match step {
                PatchPlanStep::Replacements(patch) => {
                    for replacement in &patch.replacements {
                        if is_gphase_instruction(&replacement.instruction) {
                            continue;
                        }
                        instruction_positions
                            .entry(replacement.key.clone())
                            .or_default()
                            .push(instruction_keys.len());
                        instruction_keys.push(replacement.key.clone());
                        operation_ids.push(commutation_memo.allocate_operation_id());
                        resolved_params.push(
                            replacement
                                .params
                                .iter()
                                .map(|value| match value {
                                    ParameterValue::Fixed(value) => Parameter::from(*value),
                                    ParameterValue::Param(parameter) => parameter.clone(),
                                })
                                .collect(),
                        );
                        touched_qubits.extend(replacement.qubits.iter().copied());
                    }
                }
                PatchPlanStep::DropMatched => {
                    old_entries.next()?;
                }
                PatchPlanStep::Keep(position) => {
                    let ((old_key, old_params), operation_id) = old_entries.next()?;
                    instruction_positions
                        .entry(old_key.clone())
                        .or_default()
                        .push(instruction_keys.len());
                    instruction_keys.push(old_key);
                    resolved_params.push(old_params);
                    operation_ids.push(operation_id);
                    touched_qubits.extend(operations[position].qubits.iter().copied());
                }
            }
        }

        if old_entries.next().is_some()
            || instruction_keys.len() != expected_len
            || operation_ids.len() != expected_len
        {
            return None;
        }
        let qubit_count = touched_qubits.len();
        Some(Self {
            instruction_keys,
            resolved_params,
            operation_ids,
            commutation_memo,
            rule_evaluation_cache,
            instruction_positions,
            qubit_count,
        })
    }
}

struct BlockContext<'a> {
    operations: &'a [Operation],
    cache: &'a BlockMatchCache,
}

impl<'a> BlockContext<'a> {
    /// Builds cached matching context for one linear operation block.
    ///
    /// Per-position instruction keys and resolved symbolic parameters are
    /// cached once because rule matching probes the same operation many times
    /// across anchors, rules, and commutation checks.
    fn new(operations: &'a [Operation], cache: &'a BlockMatchCache) -> Result<Self, CompilerError> {
        if operations.len() != cache.len() {
            return Err(CompilerError::InvariantViolation(format!(
                "rewrite block cache length {} does not match operation length {}",
                cache.len(),
                operations.len()
            )));
        }
        Ok(Self { operations, cache })
    }

    fn len(&self) -> usize {
        self.operations.len()
    }

    fn operation(&self, position: usize) -> &Operation {
        &self.operations[position]
    }

    fn key(&self, position: usize) -> &RewriteInstructionKey {
        &self.cache.instruction_keys[position]
    }

    fn params(&self, position: usize) -> &[Parameter] {
        &self.cache.resolved_params[position]
    }

    fn operation_id(&self, position: usize) -> OperationId {
        self.cache.operation_ids[position]
    }

    fn candidate_positions<'b>(
        &'b self,
        key: &RewriteInstructionKey,
        range: Range<usize>,
    ) -> CandidatePositions<'b> {
        let positions = self
            .cache
            .instruction_positions
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let window_len = range.end.saturating_sub(range.start);
        let use_index = window_len >= OCCURRENCE_INDEX_MIN_WINDOW
            && positions.len().saturating_mul(100)
                <= self
                    .len()
                    .saturating_mul(OCCURRENCE_INDEX_MAX_DENSITY_PERCENT);
        if !use_index {
            return CandidatePositions::Linear(range);
        }

        let start = positions.partition_point(|&position| position < range.start);
        let end = positions.partition_point(|&position| position < range.end);
        CandidatePositions::Indexed(positions[start..end].iter().copied())
    }
}

enum CandidatePositions<'a> {
    Linear(Range<usize>),
    Indexed(std::iter::Copied<std::slice::Iter<'a, usize>>),
}

impl Iterator for CandidatePositions<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Linear(positions) => positions.next(),
            Self::Indexed(positions) => positions.next(),
        }
    }
}

impl CompiledRuleSet {
    pub(super) fn from_library(library: &RuleLibrary) -> Result<Self, CompilerError> {
        let mut rules = Vec::with_capacity(library.len());
        let mut first_key_map: HashMap<RewriteInstructionKey, SmallVec<[usize; 8]>> =
            HashMap::new();
        let kind_by_id = build_kind_index(library);
        let commutation = CommutationChecker::from_library(library, rewrite_commutation_config());
        let qubit_bijection_invariant = library
            .rules()
            .iter()
            .all(rule_is_qubit_bijection_invariant);

        for (index, rule) in library.rules().iter().cloned().enumerate() {
            let kind = kind_by_id.get(&index).copied().unwrap_or(RuleKind::Other);
            push_compiled_rule(&mut rules, &mut first_key_map, index, kind, rule)?;
        }

        Ok(Self {
            rules,
            first_key_map,
            commutation,
            qubit_bijection_invariant,
        })
    }

    fn candidates_for_first_instruction(&self, key: &RewriteInstructionKey) -> &[usize] {
        self.first_key_map
            .get(key)
            .map(SmallVec::as_slice)
            .unwrap_or(&[])
    }

    fn get(&self, index: usize) -> &CompiledRule {
        &self.rules[index]
    }

    pub(super) fn active_rules(
        &self,
        config: &RewriteConfig,
        target_context: Option<&TargetContext>,
    ) -> RunActiveRuleSet {
        let mut bits = vec![0; self.rules.len().div_ceil(u64::BITS as usize)];
        for (rule_index, rule) in self.rules.iter().enumerate() {
            if rule_passes_run_filters(rule, config, target_context) {
                bitset_insert(&mut bits, rule_index);
            }
        }
        RunActiveRuleSet { bits }
    }

    pub(super) fn max_match_reach(&self, config: &RewriteConfig) -> usize {
        self.rules
            .iter()
            .filter(|rule| {
                rule.kind != RuleKind::Commute
                    && config.allows_kind(rule.kind)
                    && rule.match_len <= config.max_pattern_len()
                    && rule_allowed_by_connectivity_policy(rule, config)
            })
            .map(|rule| {
                rule.match_len
                    .saturating_sub(1)
                    .saturating_mul(config.max_window_ops())
            })
            .max()
            .unwrap_or(0)
    }

    pub(super) const fn is_qubit_bijection_invariant(&self) -> bool {
        self.qubit_bijection_invariant
    }

    pub(super) fn lowerable_rules(
        &self,
    ) -> impl Iterator<
        Item = (
            RuleKind,
            &[RewriteInstructionKey],
            &[RewriteInstructionKey],
            bool,
        ),
    > {
        self.rules.iter().map(|rule| {
            let has_conditions = rule
                .rule
                .conditions
                .as_ref()
                .is_some_and(|conditions| !conditions.is_empty());
            (
                rule.kind,
                rule.source_keys.as_slice(),
                rule.rewrite_keys.as_slice(),
                has_conditions,
            )
        })
    }
}

impl CompiledRule {
    fn binding_cache_key(&self, bindings: &MatchBindings) -> Option<RuleBindingCacheKey> {
        let parameters = self
            .parameter_symbols
            .iter()
            .map(|symbol| bindings.param(symbol).cloned())
            .collect::<Option<SmallVec<[_; 4]>>>()?;
        Some(RuleBindingCacheKey {
            rule_id: self.id,
            parameters,
        })
    }
}

fn rule_is_qubit_bijection_invariant(rule: &Rule) -> bool {
    rule.operations
        .iter()
        .chain(&rule.target)
        .all(|item| match &item.instruction {
            Instruction::Standard(_) | Instruction::McGate(_) => true,
            Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
            | Instruction::Directive(_)
            | Instruction::ClassicalData(_)
            | Instruction::ClassicalControl(_)
            | Instruction::Delay => false,
        })
        && rule
            .conditions
            .as_deref()
            .unwrap_or_default()
            .iter()
            .all(|condition| match condition {
                Condition::Eq(_, _) | Condition::EqMod(_, _, _) => true,
            })
}

fn rewrite_commutation_config() -> CommutationConfig {
    CommutationConfig {
        enable_rule_oracle: true,
        enable_matrix_fallback: false,
        max_matrix_qubits: 0,
    }
}

/// Adds one rule to the compiled rule set and first-key index.
///
/// The compiled representation keeps both full source order and unique
/// match/rewrite key sets. Source order drives the dependency-aware scan;
/// unique sets are used for cheap static filters before binding work starts.
fn push_compiled_rule(
    rules: &mut Vec<CompiledRule>,
    first_key_map: &mut HashMap<RewriteInstructionKey, SmallVec<[usize; 8]>>,
    id: usize,
    kind: RuleKind,
    rule: Rule,
) -> Result<(), CompilerError> {
    if rule.operations.is_empty() {
        return Err(CompilerError::InvariantViolation(
            "rewrite rule contains an empty match block".to_string(),
        ));
    }

    let match_len = rule.operations.len();
    let rewrite_len = rule.target.len();
    let mut rule_qubits = HashSet::new();
    for item in rule.operations.iter().chain(&rule.target) {
        rule_qubits.extend(item.qubits.iter().copied());
    }

    let mut source_keys = SmallVec::<[RewriteInstructionKey; 8]>::new();
    let mut match_keys = SmallVec::<[RewriteInstructionKey; 4]>::new();
    let mut rewrite_keys = SmallVec::<[RewriteInstructionKey; 4]>::new();
    let mut parameter_symbols = rule
        .operations
        .iter()
        .flat_map(|item| item.params.as_deref().unwrap_or_default())
        .filter_map(|parameter| match parameter {
            ParameterValue::Fixed(_) => None,
            ParameterValue::Param(parameter) => Some(parameter.get_symbols()),
        })
        .flatten()
        .collect::<SmallVec<[String; 4]>>();
    parameter_symbols.sort_unstable();
    parameter_symbols.dedup();
    let numeric_conditions = rule
        .conditions
        .as_deref()
        .and_then(CompiledConditions::compile);

    for item in &rule.operations {
        let key = RewriteInstructionKey::from_instruction(&item.instruction).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "rewrite rule contains unsupported instruction {:?}",
                item.instruction
            ))
        })?;
        if !match_keys.contains(&key) {
            match_keys.push(key.clone());
        }
        source_keys.push(key);
    }
    for item in &rule.target {
        let key = RewriteInstructionKey::from_instruction(&item.instruction).ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "rewrite rule contains unsupported instruction {:?}",
                item.instruction
            ))
        })?;
        if !rewrite_keys.contains(&key) {
            rewrite_keys.push(key.clone());
        }
    }

    first_key_map
        .entry(source_keys[0].clone())
        .or_default()
        .push(rules.len());
    rules.push(CompiledRule {
        id,
        kind,
        match_len,
        qubit_count: rule_qubits.len(),
        static_cost_delta: rewrite_len as isize - match_len as isize,
        preserves_two_qubit_connectivity: rule_preserves_two_qubit_connectivity(&rule),
        source_keys,
        match_keys,
        rewrite_keys,
        parameter_symbols,
        numeric_conditions,
        rule,
    });
    Ok(())
}

/// Computes whether a rule's replacement can only act on two-qubit pairs
/// already present in its match.
///
/// A routed circuit has every two-qubit operation on a device coupling edge.
/// A rule preserves that invariant when every exactly-two-qubit operation in
/// its rewrite uses an unordered qubit pair that some exactly-two-qubit
/// operation in its match already uses, and the rewrite emits no operations
/// on more than two qubits. Pair order is ignored: direction flips such as
/// `CX 0 1 -> CX 1 0` are legal because gate direction is resolved by device
/// lowering, not by routing.
///
/// Only exactly-two-qubit match operations count as evidence that a pair is
/// a valid physical edge: a three-qubit operation spanning qubits 0, 1, 2
/// does not prove that {0, 2} is routable. Rules may therefore not shrink a
/// multi-qubit match onto an arbitrary sub-pair.
///
/// Rewrites cannot introduce qubits absent from the match block; that is
/// already enforced by `Rule::validate` at library load time
/// (`UnboundRewriteQubit`), so this classification only inspects edge sets
/// and rewrite arity.
///
/// `pub(super)` so the rewrite test module can exercise the classification
/// directly on hand-written rules without routing a full circuit.
pub(super) fn rule_preserves_two_qubit_connectivity(rule: &Rule) -> bool {
    let mut source_pairs = HashSet::new();
    for item in &rule.operations {
        if item.qubits.len() == 2 {
            source_pairs.insert(normalized_qubit_pair(item.qubits[0], item.qubits[1]));
        }
    }

    for item in &rule.target {
        match item.qubits.len() {
            0 | 1 => {}
            2 => {
                if !source_pairs.contains(&normalized_qubit_pair(item.qubits[0], item.qubits[1])) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Returns the unordered qubit pair `(min, max)` for edge-set membership.
fn normalized_qubit_pair(a: u32, b: u32) -> (u32, u32) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Selects the rewrite patches for one block, optionally restricted to dirty
/// anchor ranges from the incremental workset.
///
/// `anchor_ranges` encodes the workset contract:
///
/// - `None`: full scan of every anchor. Used for the first round and whenever
///   global-phase involvement, an unstable scope, or failed mutation
///   reconciliation prevents the caller from preserving incremental state.
/// - `Some(ranges)`: scan only anchors inside `ranges`. The caller guarantees
///   that every anchor outside the ranges was proven candidate-free against
///   block content it can still observe, because matching at an anchor reads
///   at most `max_match_reach` operations ahead and all applied patches are
///   covered by the ranges.
/// - `Some(&[])`: the whole block was proven candidate-free in the previous
///   round and is unchanged, so the call short-circuits to an empty patch
///   set. This is the normal steady state of a converged block, not an error.
///
/// When the dirty ranges cover at least [`DIRTY_FULL_SCAN_PERCENT`] percent
/// of all block anchors, the restriction is dropped and the block is scanned
/// in full. Using raw density avoids a separate clean-region eligibility scan
/// before matching begins.
pub(super) fn select_rewrites_for_anchor_ranges(
    operations: &[Operation],
    cache: &BlockMatchCache,
    rules: &CompiledRuleSet,
    run_active_rules: &RunActiveRuleSet,
    config: &RewriteConfig,
    target_context: Option<&TargetContext>,
    anchor_ranges: Option<&[Range<usize>]>,
) -> Result<Vec<RewritePatch>, CompilerError> {
    let block = BlockContext::new(operations, cache)?;
    let active_rules = run_active_rules.for_block(rules, &block);
    let mut anchors = anchors_for_ranges(block.len(), anchor_ranges);
    let full_scan_fallback = anchor_ranges.is_some()
        && !block.operations.is_empty()
        && anchors.len().saturating_mul(100) >= block.len().saturating_mul(DIRTY_FULL_SCAN_PERCENT);
    if anchor_ranges.is_some() {
        cache
            .rule_evaluation_cache
            .record_incremental_scan(anchors.len(), full_scan_fallback);
    }
    if full_scan_fallback {
        // Use raw anchor density here.  Computing the exact eligible total
        // would itself scan every clean anchor, defeating incremental
        // execution before matching begins.
        anchors = (0..block.len()).collect();
    }
    let candidates = scan_anchors(
        &block,
        &anchors,
        rules,
        &active_rules,
        config,
        target_context,
        PARALLEL_ANCHOR_THRESHOLD,
    )?;

    select_candidate_patches(candidates, block.len(), target_context)
}

fn anchors_for_ranges(block_len: usize, ranges: Option<&[Range<usize>]>) -> Vec<usize> {
    let Some(ranges) = ranges else {
        return (0..block_len).collect();
    };
    let mut anchors = Vec::new();
    let mut previous_end = 0;
    for range in ranges {
        let start = range.start.min(block_len).max(previous_end);
        let end = range.end.min(block_len);
        if start < end {
            anchors.extend(start..end);
            previous_end = end;
        }
    }
    anchors
}

fn scan_anchors(
    block: &BlockContext<'_>,
    anchors: &[usize],
    rules: &CompiledRuleSet,
    active_rules: &BlockActiveRuleSet,
    config: &RewriteConfig,
    target_context: Option<&TargetContext>,
    parallel_threshold: usize,
) -> Result<Vec<CandidatePatch>, CompilerError> {
    let scanner = AnchorScanner {
        block,
        rules,
        active_rules,
        config,
        target_context,
    };
    if anchors.len() < parallel_threshold {
        let commutation_cache =
            LocalExactCommutationCache::new(block.cache.commutation_memo.as_ref());
        let mut candidates = Vec::new();
        for &anchor in anchors {
            scanner.scan_anchor_into(anchor, &commutation_cache, &mut candidates)?;
        }
        return Ok(candidates);
    }

    use rayon::prelude::*;
    let memo = block.cache.commutation_memo.as_ref();
    let state = anchors
        .par_iter()
        .fold(
            || AnchorScanWorker::new(memo),
            |mut worker, &anchor| {
                if worker.error.is_none()
                    && let Err(error) = scanner.scan_anchor_into(
                        anchor,
                        &worker.commutation_cache,
                        &mut worker.candidates,
                    )
                {
                    worker.error = Some((anchor, error));
                }
                worker
            },
        )
        .reduce(
            || AnchorScanWorker::new(memo),
            |mut left, mut right| {
                left.candidates.append(&mut right.candidates);
                left.error = earliest_scan_error(left.error, right.error);
                left
            },
        );
    match state.error {
        Some((_, error)) => Err(error),
        None => Ok(state.candidates),
    }
}

struct AnchorScanWorker<'a> {
    candidates: Vec<CandidatePatch>,
    error: Option<(usize, CompilerError)>,
    commutation_cache: LocalExactCommutationCache<'a>,
}

impl<'a> AnchorScanWorker<'a> {
    fn new(memo: &'a ExactCommutationMemo) -> Self {
        Self {
            candidates: Vec::new(),
            error: None,
            commutation_cache: LocalExactCommutationCache::new(memo),
        }
    }
}

fn earliest_scan_error(
    left: Option<(usize, CompilerError)>,
    right: Option<(usize, CompilerError)>,
) -> Option<(usize, CompilerError)> {
    match (left, right) {
        (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
        (left @ Some(_), None) => left,
        (None, right) => right,
    }
}

struct AnchorScanner<'a> {
    block: &'a BlockContext<'a>,
    rules: &'a CompiledRuleSet,
    active_rules: &'a BlockActiveRuleSet,
    config: &'a RewriteConfig,
    target_context: Option<&'a TargetContext>,
}

impl AnchorScanner<'_> {
    fn scan_anchor_into<C: ExactCommutationCache>(
        &self,
        anchor: usize,
        commutation_cache: &C,
        candidates: &mut Vec<CandidatePatch>,
    ) -> Result<(), CompilerError> {
        let operation = self.block.operation(anchor);
        if self.config.skips_labeled_ops() && operation.label.is_some() {
            return Ok(());
        }
        let first_key = self.block.key(anchor);

        for &rule_index in self.rules.candidates_for_first_instruction(first_key) {
            if !self.active_rules.contains(rule_index) {
                continue;
            }
            let compiled = self.rules.get(rule_index);
            if let Some(candidate) = try_match_rule(
                self.block,
                anchor,
                compiled,
                &self.rules.commutation,
                self.config,
                self.target_context,
                commutation_cache,
            )? {
                candidates.push(candidate);
            }
        }

        Ok(())
    }
}

fn select_candidate_patches(
    mut candidates: Vec<CandidatePatch>,
    block_len: usize,
    target_context: Option<&TargetContext>,
) -> Result<Vec<RewritePatch>, CompilerError> {
    // Rank candidate patches by the local objective before taking any patch.
    // With an explicit target, prefer the patch that legalizes more unsupported
    // operations so a cheap single-gate rewrite cannot occupy part of a better
    // multi-operation lowering. The remaining keys make the choice deterministic.
    candidates.sort_by(|lhs, rhs| {
        let lhs_unsupported_reduction = lhs
            .before
            .unsupported_ops
            .saturating_sub(lhs.after.unsupported_ops);
        let rhs_unsupported_reduction = rhs
            .before
            .unsupported_ops
            .saturating_sub(rhs.after.unsupported_ops);
        let target_reduction_order = if target_context.is_some() {
            rhs_unsupported_reduction.cmp(&lhs_unsupported_reduction)
        } else {
            std::cmp::Ordering::Equal
        };

        target_reduction_order
            .then_with(|| lhs.after.cmp(&rhs.after))
            .then_with(|| lhs.before.cmp(&rhs.before).reverse())
            .then_with(|| {
                rhs.patch
                    .matched_positions
                    .len()
                    .cmp(&lhs.patch.matched_positions.len())
            })
            .then_with(|| {
                lhs.patch
                    .static_cost_delta
                    .cmp(&rhs.patch.static_cost_delta)
            })
            .then_with(|| lhs.patch.first_position.cmp(&rhs.patch.first_position))
            .then_with(|| lhs.patch.rule_id.cmp(&rhs.patch.rule_id))
    });

    let mut occupied_spans = vec![false; block_len];
    let mut patches = Vec::new();
    for candidate in candidates {
        let first_position = candidate.patch.first_position;
        let last_position = candidate.patch.last_position;
        if first_position > last_position || last_position >= occupied_spans.len() {
            return Err(CompilerError::InvariantViolation(format!(
                "rewrite candidate span {first_position}..={last_position} is outside block of length {}",
                block_len
            )));
        }
        if occupied_spans[first_position..=last_position]
            .iter()
            .any(|occupied| *occupied)
        {
            continue;
        }

        occupied_spans[first_position..=last_position].fill(true);
        patches.push(candidate.patch);
    }

    patches.sort_by_key(|patch| patch.first_position);
    Ok(patches)
}

fn build_kind_index(library: &RuleLibrary) -> HashMap<usize, RuleKind> {
    let mut index = HashMap::new();
    for kind in [
        RuleKind::Simplify,
        RuleKind::Cancel,
        RuleKind::Merge,
        RuleKind::Commute,
        RuleKind::Decompose,
        RuleKind::Canonicalize,
        RuleKind::HardwareNative,
        RuleKind::Other,
    ] {
        for id in library.rules_by_kind(kind) {
            index.insert(id.as_usize(), kind);
        }
    }
    index
}

/// Applies configuration and target filters once per rewrite run.
fn rule_passes_run_filters(
    rule: &CompiledRule,
    config: &RewriteConfig,
    target_context: Option<&TargetContext>,
) -> bool {
    rule.kind != RuleKind::Commute
        && config.allows_kind(rule.kind)
        && rule.match_len <= config.max_pattern_len()
        && rule_allowed_by_connectivity_policy(rule, config)
        && rule_passes_target_filter(rule, target_context)
}

/// Applies block summary filters once before scanning its anchors.
fn rule_passes_block_filters(rule: &CompiledRule, block: &BlockContext<'_>) -> bool {
    rule.qubit_count <= block.cache.qubit_count
        && rule
            .match_keys
            .iter()
            .all(|key| block.cache.instruction_positions.contains_key(key))
}

/// Returns whether `rule` is admissible under the configured two-qubit
/// connectivity policy.
///
/// The policy is only restrictive when explicitly enabled (post-routing
/// phases); otherwise every rule passes. This predicate is shared by the
/// static filter, the match-reach estimate, and anchor eligibility so the
/// incremental matcher never budgets scan windows for rules that the filter
/// would reject anyway.
fn rule_allowed_by_connectivity_policy(rule: &CompiledRule, config: &RewriteConfig) -> bool {
    !config.preserve_two_qubit_connectivity() || rule.preserves_two_qubit_connectivity
}

/// Checks whether a rule is legal for an optional target-basis context.
///
/// Target lowering rules must preserve already-physical source operations and
/// emit only keys legal for the target context. Block key presence is handled
/// separately by [`rule_passes_block_filters`].
fn rule_passes_target_filter(rule: &CompiledRule, target_context: Option<&TargetContext>) -> bool {
    let Some(target_context) = target_context else {
        return true;
    };

    if rule_rewrites_physical_source_through_non_physical_target(rule, target_context) {
        return false;
    }

    rule.rewrite_keys
        .iter()
        .all(|key| target_context.allows_rewrite_key(key))
}

fn rule_rewrites_physical_source_through_non_physical_target(
    rule: &CompiledRule,
    target_context: &TargetContext,
) -> bool {
    !rule.source_keys.is_empty()
        && rule
            .source_keys
            .iter()
            .all(|key| is_implicit_target_key(key) || target_context.physically_supports(key))
        && rule
            .rewrite_keys
            .iter()
            .any(|key| !is_implicit_target_key(key) && !target_context.physically_supports(key))
}

fn is_implicit_target_key(key: &RewriteInstructionKey) -> bool {
    matches!(key, RewriteInstructionKey::Standard(StandardGate::GPhase))
}

/// Tries to match one compiled rule at one anchor position.
///
/// The anchor must match the first rule item exactly. Later items may be found
/// after commuting past unrelated operations, but the first item owns the
/// candidate's source position and first-key index lookup.
fn try_match_rule<C: ExactCommutationCache>(
    block: &BlockContext<'_>,
    anchor: usize,
    compiled: &CompiledRule,
    commutation: &CommutationChecker,
    config: &RewriteConfig,
    target_context: Option<&TargetContext>,
    commutation_cache: &C,
) -> Result<Option<CandidatePatch>, CompilerError> {
    let rule = &compiled.rule;
    let mut bindings = MatchBindings::new();

    if !match_item(
        block,
        anchor,
        &rule.operations[0],
        &compiled.source_keys[0],
        &mut bindings,
        config,
    )? {
        return Ok(None);
    }

    let mut matched_positions = SmallVec::<[usize; 8]>::from_slice(&[anchor]);
    let mut skipped_positions = SmallVec::<[usize; 8]>::new();
    let mut cursor = anchor + 1;
    for (item, item_key) in rule.operations.iter().zip(&compiled.source_keys).skip(1) {
        let mut found = None;
        let limit = block.len().min(cursor + config.max_window_ops());

        for position in block.candidate_positions(item_key, cursor..limit) {
            if block.key(position) != item_key {
                continue;
            }
            // Matching is non-contiguous only when every skipped operation can
            // commute with the already matched prefix and the candidate item.
            // This preserves the observable order of non-commuting operations.
            if !can_skip_between(
                block,
                cursor..position,
                &matched_positions,
                position,
                commutation,
                config,
                commutation_cache,
            )? {
                continue;
            }

            if match_item(block, position, item, item_key, &mut bindings, config)? {
                found = Some(position);
                break;
            }
        }

        let Some(position) = found else {
            return Ok(None);
        };
        skipped_positions.extend(cursor..position);
        matched_positions.push(position);
        cursor = position + 1;
    }

    let binding_cache_key = compiled.binding_cache_key(&bindings);
    let evaluate_conditions = || {
        compiled
            .numeric_conditions
            .as_ref()
            .and_then(|conditions| conditions.evaluate(&bindings))
            .unwrap_or_else(|| {
                block.cache.rule_evaluation_cache.record_symbolic_fallback();
                knowledge_conditions_hold(rule.conditions.as_deref(), &bindings)
            })
    };
    let conditions_hold = match binding_cache_key.as_ref() {
        Some(key) => block
            .cache
            .rule_evaluation_cache
            .condition_or_insert_with(key.clone(), evaluate_conditions),
        None => evaluate_conditions(),
    };
    if !conditions_hold {
        return Ok(None);
    }
    // A skipped operation may sit before a later matched operation. Verify the
    // complete match after all positions are known, including future matches
    // that were not available to `can_skip_between` earlier in the scan.
    if !skipped_sources_commute_with_future_matches(
        block,
        &skipped_positions,
        &matched_positions,
        commutation,
        config,
        commutation_cache,
    ) {
        return Ok(None);
    }

    let replacements = if let Some(template) = binding_cache_key
        .as_ref()
        .and_then(|key| block.cache.rule_evaluation_cache.replacement(key))
    {
        instantiate_replacement_template(&template, &bindings)?
    } else {
        let instantiated = knowledge_instantiate_target(&rule.target, &bindings)
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
        let template = instantiated
            .iter()
            .zip(&rule.target)
            .map(|(item, source)| ReplacementTemplateItem {
                instruction: item.instruction.clone(),
                rule_qubits: source.qubits.clone(),
                params: item.params.clone(),
                key: item.key.clone(),
            })
            .collect::<Arc<[_]>>();
        if let Some(key) = binding_cache_key {
            block
                .cache
                .rule_evaluation_cache
                .record_replacement(key, template);
        }
        instantiated
            .into_iter()
            .map(|item| ReplacementItem {
                instruction: item.instruction,
                qubits: item.qubits,
                params: item.params,
                key: item.key,
            })
            .collect::<Vec<_>>()
    };

    if !replacements_commute_with_skipped(block, &skipped_positions, &replacements, commutation)? {
        return Ok(None);
    }

    // Cost is computed on matched source positions only. Skipped operations
    // remain in place around the replacement and therefore are not part of the
    // before/after objective.
    let before = cost_for_operation_positions(block, &matched_positions, target_context);
    let after = cost_for_replacements(&replacements, target_context);
    if !config.allows_rewrite(compiled.kind, before, after) {
        return Ok(None);
    }

    let first_position = matched_positions[0];
    let last_position = matched_positions.last().copied().unwrap_or(first_position);
    Ok(Some(CandidatePatch {
        before,
        after,
        patch: RewritePatch {
            rule_id: compiled.id,
            static_cost_delta: compiled.static_cost_delta,
            first_position,
            last_position,
            matched_positions: matched_positions.into_vec(),
            replacements,
        },
    }))
}

fn instantiate_replacement_template(
    template: &[ReplacementTemplateItem],
    bindings: &MatchBindings,
) -> Result<Vec<ReplacementItem>, CompilerError> {
    template
        .iter()
        .map(|item| {
            let qubits = item
                .rule_qubits
                .iter()
                .map(|rule_qubit| {
                    bindings.qubit(*rule_qubit).ok_or_else(|| {
                        CompilerError::InvariantViolation(format!(
                            "rewrite qubit {rule_qubit} is not bound by the cached match"
                        ))
                    })
                })
                .collect::<Result<SmallVec<[_; 3]>, _>>()?;
            Ok(ReplacementItem {
                instruction: item.instruction.clone(),
                qubits,
                params: item.params.clone(),
                key: item.key.clone(),
            })
        })
        .collect()
}

/// Checks whether operations between a matched prefix and candidate can be skipped.
///
/// Only skipped operations touching the match's active qubits can constrain the
/// rewrite. Operations on disjoint qubits are independent in this local block
/// model and do not need oracle calls.
fn can_skip_between<C: ExactCommutationCache>(
    block: &BlockContext<'_>,
    skipped: Range<usize>,
    matched_positions: &[usize],
    candidate_position: usize,
    commutation: &CommutationChecker,
    config: &RewriteConfig,
    commutation_cache: &C,
) -> Result<bool, CompilerError> {
    if skipped.is_empty() {
        return Ok(true);
    }

    let mut relevant = SmallVec::<[Qubit; 4]>::new();
    for &position in matched_positions {
        for &qubit in &block.operation(position).qubits {
            if !relevant.contains(&qubit) {
                relevant.push(qubit);
            }
        }
    }
    for &qubit in &block.operation(candidate_position).qubits {
        if !relevant.contains(&qubit) {
            relevant.push(qubit);
        }
    }

    for skipped_position in skipped {
        let skipped_operation = block.operation(skipped_position);
        if config.skips_labeled_ops() && skipped_operation.label.is_some() {
            return Ok(false);
        }
        if !skipped_operation
            .qubits
            .iter()
            .any(|qubit| relevant.contains(qubit))
        {
            continue;
        }

        for &matched_position in matched_positions {
            if !operations_commute(
                block,
                skipped_position,
                matched_position,
                commutation,
                commutation_cache,
            ) {
                return Ok(false);
            }
        }
        if !operations_commute(
            block,
            skipped_position,
            candidate_position,
            commutation,
            commutation_cache,
        ) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Verifies skipped operations commute with all future matched positions.
///
/// This second pass catches skipped operations that were seen before all later
/// match positions were known.
fn skipped_sources_commute_with_future_matches<C: ExactCommutationCache>(
    block: &BlockContext<'_>,
    skipped_positions: &[usize],
    matched_positions: &[usize],
    commutation: &CommutationChecker,
    config: &RewriteConfig,
    commutation_cache: &C,
) -> bool {
    for &skipped_position in skipped_positions {
        let skipped_operation = block.operation(skipped_position);
        if config.skips_labeled_ops() && skipped_operation.label.is_some() {
            return false;
        }
        for &matched_position in matched_positions {
            if matched_position <= skipped_position
                || !skipped_operation
                    .qubits
                    .iter()
                    .any(|qubit| block.operation(matched_position).qubits.contains(qubit))
            {
                continue;
            }
            if !operations_commute(
                block,
                skipped_position,
                matched_position,
                commutation,
                commutation_cache,
            ) {
                return false;
            }
        }
    }

    true
}

/// Verifies replacements can be emitted without crossing skipped operations.
///
/// The replacement will be emitted at the match site while skipped operations
/// remain around it. Every replacement touching a skipped operation's qubits
/// must commute with that skipped operation.
fn replacements_commute_with_skipped(
    block: &BlockContext<'_>,
    skipped_positions: &[usize],
    replacements: &[ReplacementItem],
    commutation: &CommutationChecker,
) -> Result<bool, CompilerError> {
    if skipped_positions.is_empty() || replacements.is_empty() {
        return Ok(true);
    }

    for &skipped_position in skipped_positions {
        for replacement in replacements {
            let skipped_qubits = &block.operation(skipped_position).qubits;
            if !skipped_qubits
                .iter()
                .any(|qubit| replacement.qubits.contains(qubit))
            {
                continue;
            }
            if !operation_commutes_with_replacement(
                block,
                skipped_position,
                replacement,
                commutation,
            ) {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn operations_commute<C: ExactCommutationCache>(
    block: &BlockContext<'_>,
    mut lhs_position: usize,
    mut rhs_position: usize,
    commutation: &CommutationChecker,
    commutation_cache: &C,
) -> bool {
    let mut lhs = block.operation(lhs_position);
    let mut rhs = block.operation(rhs_position);
    if !lhs.qubits.iter().any(|qubit| rhs.qubits.contains(qubit)) {
        return true;
    }

    let mut lhs_id = block.operation_id(lhs_position);
    let mut rhs_id = block.operation_id(rhs_position);
    if lhs_id > rhs_id {
        std::mem::swap(&mut lhs_position, &mut rhs_position);
        std::mem::swap(&mut lhs_id, &mut rhs_id);
        lhs = block.operation(lhs_position);
        rhs = block.operation(rhs_position);
    }

    commutation_cache.exact_or_insert_with(lhs_id, rhs_id, || {
        commutation
            .check(
                &lhs.instruction,
                &lhs.qubits,
                block.params(lhs_position),
                &rhs.instruction,
                &rhs.qubits,
                block.params(rhs_position),
            )
            .is_some_and(|result| result.is_exact())
    })
}

fn operation_commutes_with_replacement(
    block: &BlockContext<'_>,
    operation_position: usize,
    replacement: &ReplacementItem,
    commutation: &CommutationChecker,
) -> bool {
    let operation = block.operation(operation_position);
    let replacement_params = replacement
        .params
        .iter()
        .map(|value| match value {
            ParameterValue::Fixed(value) => Parameter::from(*value),
            ParameterValue::Param(parameter) => parameter.clone(),
        })
        .collect::<SmallVec<[_; 3]>>();

    commutation
        .check(
            &operation.instruction,
            &operation.qubits,
            block.params(operation_position),
            &replacement.instruction,
            &replacement.qubits,
            &replacement_params,
        )
        .is_some_and(|result| result.is_exact())
}

fn match_item(
    block: &BlockContext<'_>,
    position: usize,
    item: &RuleItem,
    item_key: &RewriteInstructionKey,
    bindings: &mut MatchBindings,
    config: &RewriteConfig,
) -> Result<bool, CompilerError> {
    let operation = block.operation(position);
    if config.skips_labeled_ops() && operation.label.is_some() {
        return Ok(false);
    }
    if block.key(position) != item_key {
        return Ok(false);
    }

    knowledge_match_rule_item_with_keys(
        item,
        item_key,
        block.key(position),
        ConcreteOperationView {
            instruction: &operation.instruction,
            qubits: &operation.qubits,
            params: block.params(position),
        },
        bindings,
    )
    .map_err(|error| CompilerError::InvariantViolation(error.to_string()))
}

pub(super) fn resolve_operation_param(
    circuit: &Circuit,
    param: &CircuitParam,
) -> Result<Parameter, CompilerError> {
    match param {
        CircuitParam::Fixed(value) => Ok(Parameter::from(*value)),
        CircuitParam::Index(index) => circuit
            .parameters()
            .get_index(*index as usize)
            .cloned()
            .ok_or_else(|| {
                CompilerError::InvalidInput(format!("invalid rewrite parameter index {}", index))
            }),
    }
}

/// Computes the local rewrite cost for matched source operations.
///
/// The selector compares local alternatives; full-circuit scheduling is outside
/// the rewrite pass.
fn cost_for_operation_positions(
    block: &BlockContext<'_>,
    positions: &[usize],
    target_context: Option<&TargetContext>,
) -> LocalRewriteCost {
    let mut cost = LocalRewriteCost::default();
    let mut depths = HashMap::new();

    for &position in positions {
        let operation = block.operation(position);
        add_instruction_cost(
            &mut cost,
            &mut depths,
            block.key(position),
            &operation.qubits,
            operation.params.len(),
            GPhaseCost::ExplicitOperation,
            target_context,
        );
    }
    cost
}

fn cost_for_replacements(
    replacements: &[ReplacementItem],
    target_context: Option<&TargetContext>,
) -> LocalRewriteCost {
    let mut cost = LocalRewriteCost::default();
    let mut depths = HashMap::new();

    for replacement in replacements {
        add_instruction_cost(
            &mut cost,
            &mut depths,
            &replacement.key,
            &replacement.qubits,
            replacement.params.len(),
            GPhaseCost::ImplicitReplacement,
            target_context,
        );
    }
    cost
}

fn add_instruction_cost(
    cost: &mut LocalRewriteCost,
    depths: &mut HashMap<Qubit, usize>,
    key: &RewriteInstructionKey,
    qubits: &[Qubit],
    param_count: usize,
    gphase_cost: GPhaseCost,
    target_context: Option<&TargetContext>,
) {
    let target_supported = match target_context {
        Some(target_context) => target_context.physically_supports(key),
        None => true,
    };
    let standard_gate = match key {
        RewriteInstructionKey::Standard(gate) => Some(*gate),
        RewriteInstructionKey::McGate(_) => None,
    };
    let counted = cost.add_gate_like(
        standard_gate,
        target_supported,
        qubits.len(),
        param_count,
        gphase_cost,
    );
    if counted {
        if let Some(target_context) = target_context {
            cost.lowering_distance = cost
                .lowering_distance
                .saturating_add(target_context.lowering_distance(key));
        }
        update_depth_estimate(cost, depths, qubits);
    }
}

/// Updates the local ASAP-style depth estimate for one operation.
///
/// Repeated qubits within one operation are treated as a single dependency
/// edge. The estimate is deterministic and suitable for local ranking, but it
/// is not a substitute for a backend scheduler.
fn update_depth_estimate(
    cost: &mut LocalRewriteCost,
    depths: &mut HashMap<Qubit, usize>,
    qubits: &[Qubit],
) {
    if qubits.is_empty() {
        return;
    }

    let mut unique = SmallVec::<[Qubit; 3]>::new();
    for &qubit in qubits {
        if !unique.contains(&qubit) {
            unique.push(qubit);
        }
    }

    let next_depth = unique
        .iter()
        .filter_map(|qubit| depths.get(qubit))
        .max()
        .copied()
        .unwrap_or(0)
        + 1;
    for qubit in unique {
        depths.insert(qubit, next_depth);
    }
    cost.depth_estimate = cost.depth_estimate.max(next_depth);
}

#[cfg(test)]
#[path = "./matcher_test.rs"]
mod matcher_test;
