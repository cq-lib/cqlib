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

//! Bit-exact caching for two-qubit synthesis planners.
//!
//! The cache stores planner candidates rather than selected patches. A caller
//! must therefore repeat source-cost, structural, commutation, and overlap checks for
//! every block. Keys preserve the exact complex floating-point representation
//! exactly. Generic candidates are stored on canonical tensor roles and
//! remapped to requested qubits at consumption. Pre-layout device candidates
//! use the same canonical logical tensor roles, while exact-physical candidates
//! retain their ordered physical pair. Numerically close or phase-equivalent
//! matrices are deliberately unrelated.
//!
//! Capacity is bounded with an admission-only policy. Existing entries remain
//! available after the budget is reached, while new keys are planned normally
//! and consumed without insertion. Planner failures and empty candidate lists
//! are both cached so deterministic negative results do not repeat expensive
//! work.

use crate::circuit::Qubit;
use crate::circuit::{Instruction, ValueOperation};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::unitary_2q::DeviceTwoQubitSynthesisCandidate;
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, DeviceTwoQubitSynthesisContext,
};
use crate::compile::transform::decompose::unitary::{
    KakDecomposition, TwoQubitSynthesisCandidate, kak_decompose,
};
use crate::compile::transform::resynthesis::TwoQubitBlockResynthesisConfig;
use ndarray::Array2;
use num_complex::Complex64;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use super::collector::TwoQubitNumericBlock;
use super::commutation::OperationView;
use super::cost::ResynthesisCost;

pub(super) const RESYNTHESIS_SYNTHESIS_CACHE_BUDGET: usize = 4096;
const SYNTHESIS_ALGORITHM_REVISION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SynthesisCacheNamespace {
    config: TwoQubitBlockResynthesisConfig,
    placement: Option<DeviceSynthesisPlacement>,
    device_context_generation: Option<u64>,
    algorithm_revision: u32,
}

impl SynthesisCacheNamespace {
    fn new(
        config: &TwoQubitBlockResynthesisConfig,
        device_context: Option<&DeviceTwoQubitSynthesisContext>,
    ) -> Self {
        Self {
            config: config.clone(),
            placement: device_context.map(DeviceTwoQubitSynthesisContext::placement),
            device_context_generation: device_context
                .map(DeviceTwoQubitSynthesisContext::generation),
            algorithm_revision: SYNTHESIS_ALGORITHM_REVISION,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactUnitaryFingerprint {
    matrix_bits: [(u64, u64); 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactKakCacheKey {
    unitary: ExactUnitaryFingerprint,
    algorithm_revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactTwoQubitSynthesisKey {
    unitary: ExactUnitaryFingerprint,
    ordered_qargs: [Qubit; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactStandardOperationKey {
    gate: u8,
    qargs: Vec<Qubit>,
    parameter_bits: Vec<u64>,
    label: Option<Box<str>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactBlockFactsKey {
    ordered_qargs: [Qubit; 2],
    operations: Vec<ExactStandardOperationKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExactTerminalDecisionKey {
    block: ExactBlockFactsKey,
    matched_orders: Vec<usize>,
    crossed_orders: Vec<usize>,
    exact_span: Vec<ExactStandardOperationKey>,
}

impl ExactTerminalDecisionKey {
    fn new(block: &TwoQubitNumericBlock, ops: &[OperationView<'_>]) -> Option<Self> {
        let block_key = ExactBlockFactsKey::new(block, ops).ok()?;
        let span_start = block
            .matched_orders
            .iter()
            .chain(&block.crossed_orders)
            .min()
            .copied()?;
        let span_end = block
            .matched_orders
            .iter()
            .chain(&block.crossed_orders)
            .max()
            .copied()?;
        let exact_span = (span_start..=span_end)
            .map(|order| ExactStandardOperationKey::new(&ops[order]))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            block: block_key,
            matched_orders: block
                .matched_orders
                .iter()
                .map(|order| order.saturating_sub(span_start))
                .collect(),
            crossed_orders: block
                .crossed_orders
                .iter()
                .map(|order| order.saturating_sub(span_start))
                .collect(),
            exact_span,
        })
    }
}

impl ExactStandardOperationKey {
    fn new(view: &OperationView<'_>) -> Option<Self> {
        let Instruction::Standard(gate) = view.operation.instruction else {
            return None;
        };
        let parameter_bits = view
            .params
            .iter()
            .map(|parameter| parameter.evaluate(&None).ok().map(f64::to_bits))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            gate: gate as u8,
            qargs: view.operation.qubits.iter().copied().collect(),
            parameter_bits,
            label: view.operation.label.clone(),
        })
    }
}

impl ExactBlockFactsKey {
    fn new(block: &TwoQubitNumericBlock, ops: &[OperationView<'_>]) -> Result<Self, CompilerError> {
        let operations = block
            .matched_orders
            .iter()
            .map(|&order| {
                let view = &ops[order];
                ExactStandardOperationKey::new(view).ok_or_else(|| {
                    CompilerError::InvariantViolation(
                        "block facts require fixed standard operations".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<_>, CompilerError>>()?;
        Ok(Self {
            ordered_qargs: block.qubits,
            operations,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct CachedBlockFacts {
    /// Matrix construction is deferred until a block survives cheap overlap
    /// bounds. `None` memoizes a deterministic matrix-construction failure.
    pub(super) matrix: OnceLock<Option<Array2<Complex64>>>,
    pub(super) source_operations: Vec<ValueOperation>,
    pub(super) source_cost: ResynthesisCost,
}

/// Keeps the ordinary one-pass path allocation-free while allowing native
/// fixed-point rounds to share retained facts without copying matrices or
/// source operations.
pub(super) enum CachedBlockFactsHandle {
    Owned(CachedBlockFacts),
    Shared(Arc<CachedBlockFacts>),
}

impl Deref for CachedBlockFactsHandle {
    type Target = CachedBlockFacts;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(facts) => facts,
            Self::Shared(facts) => facts,
        }
    }
}

impl ExactUnitaryFingerprint {
    fn new(matrix: &Array2<Complex64>) -> Result<Self, CompilerError> {
        if matrix.dim() != (4, 4) {
            return Err(CompilerError::InvariantViolation(format!(
                "2q synthesis cache requires a 4x4 matrix, got {}x{}",
                matrix.nrows(),
                matrix.ncols()
            )));
        }
        // Index by logical row and column so the key is independent of ndarray
        // storage layout while preserving every floating-point bit exactly.
        let matrix_bits = std::array::from_fn(|index| {
            let value = matrix[(index / 4, index % 4)];
            (value.re.to_bits(), value.im.to_bits())
        });
        Ok(Self { matrix_bits })
    }
}

impl ExactTwoQubitSynthesisKey {
    fn new(matrix: &Array2<Complex64>, ordered_qargs: [Qubit; 2]) -> Result<Self, CompilerError> {
        Ok(Self {
            unitary: ExactUnitaryFingerprint::new(matrix)?,
            ordered_qargs,
        })
    }
}

#[derive(Debug, Clone)]
enum CachedPlan<T> {
    Candidates(Vec<T>),
    Failed,
}

#[derive(Debug, Clone)]
enum CachedKakDecomposition {
    Decomposition(Arc<KakDecomposition>),
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CachedPlanView<'a, T> {
    Candidates(&'a [T]),
    Failed,
}

impl<T> CachedPlan<T> {
    fn view(&self) -> CachedPlanView<'_, T> {
        match self {
            Self::Candidates(candidates) => CachedPlanView::Candidates(candidates),
            Self::Failed => CachedPlanView::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ResynthesisSelectionStats {
    pub(crate) input_blocks: usize,
    pub(crate) bounded_blocks: usize,
    pub(crate) invalid_blocks: usize,
    pub(crate) duplicate_blocks: usize,
    pub(crate) terminal_lookups: usize,
    pub(crate) terminal_hits: usize,
    pub(crate) terminal_misses: usize,
    pub(crate) source_fact_attempts: usize,
    pub(crate) source_fact_failures: usize,
    pub(crate) matrix_attempts: usize,
    pub(crate) synthesis_attempts: usize,
    pub(crate) plan_failures: usize,
    pub(crate) candidates_considered: usize,
    pub(crate) cost_rejections: usize,
    pub(crate) device_cost_rejections: usize,
    pub(crate) crossing_rejections: usize,
    pub(crate) replacement_rejections: usize,
    pub(crate) patches_produced: usize,
    pub(crate) selected_patches: usize,
    pub(crate) overlap_rejections: usize,
    pub(crate) lower_bound_pruned: usize,
    pub(crate) conflict_components: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TwoQubitSynthesisCacheStats {
    pub(crate) generic_lookups: usize,
    pub(crate) generic_hits: usize,
    pub(crate) generic_misses: usize,
    pub(crate) generic_entries: usize,
    pub(crate) device_lookups: usize,
    pub(crate) device_hits: usize,
    pub(crate) device_misses: usize,
    pub(crate) device_entries: usize,
    pub(crate) failed_plan_hits: usize,
    pub(crate) capacity_rejections: usize,
    pub(crate) namespace_invalidations: usize,
    pub(crate) kak_lookups: usize,
    pub(crate) kak_hits: usize,
    pub(crate) kak_misses: usize,
    pub(crate) kak_entries: usize,
    pub(crate) kak_failed_hits: usize,
    pub(crate) block_fact_lookups: usize,
    pub(crate) block_fact_hits: usize,
    pub(crate) block_fact_misses: usize,
    pub(crate) block_fact_entries: usize,
    pub(crate) terminal_decision_hits: usize,
    pub(crate) terminal_decision_entries: usize,
    pub(crate) selection: ResynthesisSelectionStats,
}

/// Candidate, block-fact, and terminal-decision entries are valid only inside
/// one exact synthesis namespace. The namespace contains the complete
/// resynthesis configuration plus placement and the immutable device-context
/// generation. KAK entries are keyed by exact matrix bits and the synthesis
/// algorithm revision because the decomposition is independent of the target,
/// placement, and device namespace inputs.
#[derive(Debug)]
pub(super) struct TwoQubitSynthesisCache {
    generic: HashMap<ExactTwoQubitSynthesisKey, CachedPlan<TwoQubitSynthesisCandidate>>,
    device: HashMap<ExactTwoQubitSynthesisKey, CachedPlan<DeviceTwoQubitSynthesisCandidate>>,
    kak: HashMap<ExactKakCacheKey, CachedKakDecomposition>,
    block_facts: HashMap<ExactBlockFactsKey, CachedPlan<Arc<CachedBlockFacts>>>,
    terminal_no_patch: HashSet<ExactTerminalDecisionKey>,
    stats: TwoQubitSynthesisCacheStats,
    budget: usize,
    namespace: Option<SynthesisCacheNamespace>,
    artifact_reuse: bool,
    native_round_reuse: bool,
}

impl Default for TwoQubitSynthesisCache {
    fn default() -> Self {
        Self::new(RESYNTHESIS_SYNTHESIS_CACHE_BUDGET)
    }
}

impl TwoQubitSynthesisCache {
    /// Starts a compiler-pass-local selection measurement. Cache counters stay
    /// cumulative so reuse across fixed-point rounds remains observable.
    pub(super) fn begin_selection_pass(&mut self) {
        self.stats.selection = ResynthesisSelectionStats::default();
    }

    pub(super) fn selection_stats_mut(&mut self) -> &mut ResynthesisSelectionStats {
        &mut self.stats.selection
    }

    pub(super) fn new(budget: usize) -> Self {
        Self {
            generic: HashMap::new(),
            device: HashMap::new(),
            kak: HashMap::new(),
            block_facts: HashMap::new(),
            terminal_no_patch: HashSet::new(),
            stats: TwoQubitSynthesisCacheStats::default(),
            budget,
            namespace: None,
            artifact_reuse: false,
            native_round_reuse: false,
        }
    }

    pub(super) fn new_native_session() -> Self {
        Self {
            artifact_reuse: true,
            native_round_reuse: true,
            ..Self::default()
        }
    }

    pub(super) fn new_workflow_session() -> Self {
        Self {
            artifact_reuse: true,
            ..Self::default()
        }
    }

    pub(super) const fn artifact_reuse_enabled(&self) -> bool {
        self.artifact_reuse
    }

    pub(super) fn ensure_namespace(
        &mut self,
        config: &TwoQubitBlockResynthesisConfig,
        device_context: Option<&DeviceTwoQubitSynthesisContext>,
    ) {
        let namespace = SynthesisCacheNamespace::new(config, device_context);
        if self
            .namespace
            .as_ref()
            .is_some_and(|current| current != &namespace)
        {
            self.generic.clear();
            self.device.clear();
            self.block_facts.clear();
            self.terminal_no_patch.clear();
            self.stats.generic_entries = 0;
            self.stats.device_entries = 0;
            self.stats.block_fact_entries = 0;
            self.stats.terminal_decision_entries = 0;
            self.stats.namespace_invalidations =
                self.stats.namespace_invalidations.saturating_add(1);
        }
        self.namespace = Some(namespace);
    }

    pub(super) const fn stats(&self) -> TwoQubitSynthesisCacheStats {
        self.stats
    }

    /// Returns the KAK decomposition for this exact matrix. These entries are
    /// independent of target, placement, qubit identity, and device revision,
    /// so they remain valid when the candidate namespace changes.
    pub(super) fn kak_decomposition(
        &mut self,
        matrix: &Array2<Complex64>,
    ) -> Result<Option<Arc<KakDecomposition>>, CompilerError> {
        let key = ExactKakCacheKey {
            unitary: ExactUnitaryFingerprint::new(matrix)?,
            algorithm_revision: SYNTHESIS_ALGORITHM_REVISION,
        };
        self.stats.kak_lookups = self.stats.kak_lookups.saturating_add(1);
        if let Some(decomp) = self.kak.get(&key) {
            self.stats.kak_hits = self.stats.kak_hits.saturating_add(1);
            return Ok(match decomp {
                CachedKakDecomposition::Decomposition(decomp) => Some(decomp.clone()),
                CachedKakDecomposition::Failed => {
                    self.stats.kak_failed_hits = self.stats.kak_failed_hits.saturating_add(1);
                    None
                }
            });
        }

        self.stats.kak_misses = self.stats.kak_misses.saturating_add(1);
        let decomp = match kak_decompose(matrix) {
            Ok(decomp) => CachedKakDecomposition::Decomposition(Arc::new(decomp)),
            Err(_) => CachedKakDecomposition::Failed,
        };
        let result = match &decomp {
            CachedKakDecomposition::Decomposition(decomp) => Some(decomp.clone()),
            CachedKakDecomposition::Failed => None,
        };
        if self.kak.len() < self.budget {
            self.kak.insert(key, decomp);
            self.stats.kak_entries = self.kak.len();
        } else {
            self.stats.capacity_rejections = self.stats.capacity_rejections.saturating_add(1);
        }
        Ok(result)
    }

    pub(super) fn block_facts(
        &mut self,
        block: &TwoQubitNumericBlock,
        ops: &[OperationView<'_>],
        compute: impl FnOnce() -> Result<CachedBlockFacts, CompilerError>,
    ) -> Result<Option<CachedBlockFactsHandle>, CompilerError> {
        // The key is intentionally richer than the matrix cache key because it
        // protects source cost and exact source operations. Constructing and
        // hashing it is only repaid across native fixed-point rounds; within a
        // single ordinary pass it regresses unique-block workloads and the
        // existing matrix-plan cache already captures repeated synthesis.
        if !self.native_round_reuse {
            return Ok(compute().ok().map(CachedBlockFactsHandle::Owned));
        }
        let key = ExactBlockFactsKey::new(block, ops)?;
        self.stats.block_fact_lookups = self.stats.block_fact_lookups.saturating_add(1);
        if let Some(facts) = self.block_facts.get(&key) {
            self.stats.block_fact_hits = self.stats.block_fact_hits.saturating_add(1);
            return Ok(match facts {
                CachedPlan::Candidates(facts) => {
                    facts.first().cloned().map(CachedBlockFactsHandle::Shared)
                }
                CachedPlan::Failed => None,
            });
        }

        self.stats.block_fact_misses = self.stats.block_fact_misses.saturating_add(1);
        let facts = match compute() {
            Ok(facts) => CachedPlan::Candidates(vec![Arc::new(facts)]),
            Err(_) => CachedPlan::Failed,
        };
        let result = match &facts {
            CachedPlan::Candidates(facts) => {
                facts.first().cloned().map(CachedBlockFactsHandle::Shared)
            }
            CachedPlan::Failed => None,
        };
        if self.block_facts.len() < self.budget {
            self.block_facts.insert(key, facts);
            self.stats.block_fact_entries = self.block_facts.len();
        } else {
            self.stats.capacity_rejections = self.stats.capacity_rejections.saturating_add(1);
        }
        Ok(result)
    }

    pub(super) fn has_terminal_no_patch(
        &mut self,
        block: &TwoQubitNumericBlock,
        ops: &[OperationView<'_>],
    ) -> bool {
        self.stats.selection.terminal_lookups =
            self.stats.selection.terminal_lookups.saturating_add(1);
        if !self.native_round_reuse {
            self.stats.selection.terminal_misses =
                self.stats.selection.terminal_misses.saturating_add(1);
            return false;
        }
        let Some(key) = ExactTerminalDecisionKey::new(block, ops) else {
            self.stats.selection.terminal_misses =
                self.stats.selection.terminal_misses.saturating_add(1);
            return false;
        };
        let hit = self.terminal_no_patch.contains(&key);
        if hit {
            self.stats.terminal_decision_hits = self.stats.terminal_decision_hits.saturating_add(1);
            self.stats.selection.terminal_hits =
                self.stats.selection.terminal_hits.saturating_add(1);
        } else {
            self.stats.selection.terminal_misses =
                self.stats.selection.terminal_misses.saturating_add(1);
        }
        hit
    }

    pub(super) fn record_terminal_no_patch(
        &mut self,
        block: &TwoQubitNumericBlock,
        ops: &[OperationView<'_>],
    ) {
        if !self.native_round_reuse {
            return;
        }
        if self.terminal_no_patch.len() >= self.budget {
            self.stats.capacity_rejections = self.stats.capacity_rejections.saturating_add(1);
            return;
        }
        if let Some(key) = ExactTerminalDecisionKey::new(block, ops) {
            self.terminal_no_patch.insert(key);
            self.stats.terminal_decision_entries = self.terminal_no_patch.len();
        }
    }

    pub(super) fn with_generic_plan<R>(
        &mut self,
        matrix: &Array2<Complex64>,
        ordered_qargs: [Qubit; 2],
        planner: impl FnOnce(&mut Self) -> Result<Vec<TwoQubitSynthesisCandidate>, CompilerError>,
        consume: impl FnOnce(CachedPlanView<'_, TwoQubitSynthesisCandidate>) -> R,
    ) -> Result<R, CompilerError> {
        // Generic synthesis is physical-location independent. Store templates
        // on canonical tensor roles and remap only at the consumption edge.
        let canonical_qargs = [Qubit::new(0), Qubit::new(1)];
        let key = ExactTwoQubitSynthesisKey::new(matrix, canonical_qargs)?;
        self.stats.generic_lookups = self.stats.generic_lookups.saturating_add(1);
        if let Some(plan) = self.generic.get(&key) {
            self.stats.generic_hits = self.stats.generic_hits.saturating_add(1);
            if matches!(plan, CachedPlan::Failed) {
                self.stats.failed_plan_hits = self.stats.failed_plan_hits.saturating_add(1);
            }
            return Ok(consume_generic_plan(
                plan,
                canonical_qargs,
                ordered_qargs,
                consume,
            ));
        }

        self.stats.generic_misses = self.stats.generic_misses.saturating_add(1);
        let plan = match planner(self).and_then(|candidates| {
            if ordered_qargs == canonical_qargs {
                Ok(candidates)
            } else {
                remap_generic_candidates(candidates, ordered_qargs, canonical_qargs)
            }
        }) {
            Ok(candidates) => CachedPlan::Candidates(candidates),
            Err(_) => CachedPlan::Failed,
        };
        if self.generic.len() < self.budget {
            self.generic.insert(key.clone(), plan);
            self.stats.generic_entries = self.generic.len();
            return Ok(consume_generic_plan(
                self.generic
                    .get(&key)
                    .expect("newly inserted generic synthesis plan"),
                canonical_qargs,
                ordered_qargs,
                consume,
            ));
        }

        self.stats.capacity_rejections = self.stats.capacity_rejections.saturating_add(1);
        Ok(consume_generic_plan(
            &plan,
            canonical_qargs,
            ordered_qargs,
            consume,
        ))
    }

    pub(super) fn with_device_plan<R>(
        &mut self,
        matrix: &Array2<Complex64>,
        ordered_qargs: [Qubit; 2],
        placement: DeviceSynthesisPlacement,
        planner: impl FnOnce(&mut Self) -> Result<Vec<DeviceTwoQubitSynthesisCandidate>, CompilerError>,
        consume: impl FnOnce(CachedPlanView<'_, DeviceTwoQubitSynthesisCandidate>) -> R,
    ) -> Result<R, CompilerError> {
        let canonical_qargs = [Qubit::new(0), Qubit::new(1)];
        let cache_qargs = match placement {
            DeviceSynthesisPlacement::PreLayoutEnvelope => canonical_qargs,
            DeviceSynthesisPlacement::ExactPhysical => ordered_qargs,
        };
        let key = ExactTwoQubitSynthesisKey::new(matrix, cache_qargs)?;
        self.stats.device_lookups = self.stats.device_lookups.saturating_add(1);
        if let Some(plan) = self.device.get(&key) {
            self.stats.device_hits = self.stats.device_hits.saturating_add(1);
            if matches!(plan, CachedPlan::Failed) {
                self.stats.failed_plan_hits = self.stats.failed_plan_hits.saturating_add(1);
            }
            return Ok(consume_device_plan(
                plan,
                cache_qargs,
                ordered_qargs,
                consume,
            ));
        }

        self.stats.device_misses = self.stats.device_misses.saturating_add(1);
        let plan = match planner(self).and_then(|candidates| {
            if cache_qargs == ordered_qargs {
                Ok(candidates)
            } else {
                remap_device_candidates(candidates, ordered_qargs, cache_qargs)
            }
        }) {
            Ok(candidates) => CachedPlan::Candidates(candidates),
            Err(_) => CachedPlan::Failed,
        };
        if self.device.len() < self.budget {
            self.device.insert(key.clone(), plan);
            self.stats.device_entries = self.device.len();
            return Ok(consume_device_plan(
                self.device
                    .get(&key)
                    .expect("newly inserted device synthesis plan"),
                cache_qargs,
                ordered_qargs,
                consume,
            ));
        }

        self.stats.capacity_rejections = self.stats.capacity_rejections.saturating_add(1);
        Ok(consume_device_plan(
            &plan,
            cache_qargs,
            ordered_qargs,
            consume,
        ))
    }
}

fn consume_device_plan<R>(
    plan: &CachedPlan<DeviceTwoQubitSynthesisCandidate>,
    cached_qargs: [Qubit; 2],
    ordered_qargs: [Qubit; 2],
    consume: impl FnOnce(CachedPlanView<'_, DeviceTwoQubitSynthesisCandidate>) -> R,
) -> R {
    if cached_qargs == ordered_qargs {
        return consume(plan.view());
    }
    match plan {
        CachedPlan::Failed => consume(CachedPlanView::Failed),
        CachedPlan::Candidates(candidates) => {
            let remapped = remap_device_candidates(candidates.clone(), cached_qargs, ordered_qargs)
                .expect("cached device synthesis template only references canonical qargs");
            consume(CachedPlanView::Candidates(&remapped))
        }
    }
}

fn consume_generic_plan<R>(
    plan: &CachedPlan<TwoQubitSynthesisCandidate>,
    canonical_qargs: [Qubit; 2],
    ordered_qargs: [Qubit; 2],
    consume: impl FnOnce(CachedPlanView<'_, TwoQubitSynthesisCandidate>) -> R,
) -> R {
    if canonical_qargs == ordered_qargs {
        return consume(plan.view());
    }
    match plan {
        CachedPlan::Failed => consume(CachedPlanView::Failed),
        CachedPlan::Candidates(candidates) => {
            let remapped =
                remap_generic_candidates(candidates.clone(), canonical_qargs, ordered_qargs)
                    .expect("cached generic synthesis template only references canonical qargs");
            consume(CachedPlanView::Candidates(&remapped))
        }
    }
}

fn remap_generic_candidates(
    mut candidates: Vec<TwoQubitSynthesisCandidate>,
    source: [Qubit; 2],
    target: [Qubit; 2],
) -> Result<Vec<TwoQubitSynthesisCandidate>, CompilerError> {
    for candidate in &mut candidates {
        for operation in &mut candidate.operations {
            for qubit in &mut operation.qubits {
                *qubit = if *qubit == source[0] {
                    target[0]
                } else if *qubit == source[1] {
                    target[1]
                } else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "generic 2q synthesis candidate references unexpected qubit {qubit}"
                    )));
                };
            }
        }
    }
    Ok(candidates)
}

fn remap_device_candidates(
    mut candidates: Vec<DeviceTwoQubitSynthesisCandidate>,
    source: [Qubit; 2],
    target: [Qubit; 2],
) -> Result<Vec<DeviceTwoQubitSynthesisCandidate>, CompilerError> {
    for candidate in &mut candidates {
        for operation in &mut candidate.candidate.operations {
            for qubit in &mut operation.qubits {
                *qubit = if *qubit == source[0] {
                    target[0]
                } else if *qubit == source[1] {
                    target[1]
                } else {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device 2q synthesis candidate references unexpected qubit {qubit}"
                    )));
                };
            }
        }
    }
    Ok(candidates)
}

#[cfg(test)]
#[path = "synthesis_cache_test.rs"]
mod synthesis_cache_test;
