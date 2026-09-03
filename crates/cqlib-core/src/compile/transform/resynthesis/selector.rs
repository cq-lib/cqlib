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

//! Candidate synthesis and selection for two-qubit resynthesis.
//!
//! Syntactically duplicate blocks are removed first. Bounded blocks are then
//! partitioned into overlap components, assigned admissible planning bounds,
//! and fully validated only while they can still outrank the component winner.
//! Finally, accepted patches are filtered so no two selected patches rewrite
//! the same source operation. Maximal unbounded runs retain the eager path.

mod planning_bound;

use self::planning_bound::{
    BlockPriorityBound, estimate_block_priority_bound, plan_block_priority_bound,
};
use super::collector::{
    BlockOrigin, TwoQubitNumericBlock, is_fixed_numeric_standard, is_hard_boundary,
};
use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use super::cost::{
    ResynthesisCost, cost_of_source_value_operations, value_operations_of_source_ops,
};
use super::synthesis_cache::{
    CachedBlockFacts, CachedBlockFactsHandle, CachedPlanView, TwoQubitSynthesisCache,
};
use crate::circuit::{Instruction, StandardGate, ValueInstruction, ValueOperation};
#[cfg(any(test, debug_assertions))]
use crate::circuit::{ParameterValue, Qubit, value_operations_to_matrix};
use crate::compile::transform::decompose::unitary::unitary_2q::{
    NumericTwoQubitCandidateValidation, TwoQubitMatrixOp,
    plan_cx_family_fallbacks_for_device_from_kak, plan_cx_family_fallbacks_from_kak,
    plan_numeric_2q_unitary_for_device, plan_numeric_2q_unitary_for_device_from_kak,
    plan_numeric_2q_unitary_from_kak, plan_pauli_fallback_for_device_from_kak,
    plan_pauli_fallback_from_kak, two_qubit_operation_matrix_product,
};
use crate::compile::transform::decompose::unitary::{
    DeviceContextCostFailure, DevicePhysicalCost, DeviceSynthesisPlacement,
    DeviceTwoQubitSynthesisContext, TwoQubitSynthesisRequest, TwoQubitUnitaryDecomposeBasis,
    plan_numeric_2q_unitary,
};
use crate::compile::transform::native_quality::{NativeQualityPolicy, NativeQualityVector};
use crate::compile::{CompilerError, compare_some_first_by};
use ndarray::Array2;
use num_complex::Complex64;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::OnceLock;

#[cfg(any(test, debug_assertions))]
const MAX_PATCH_VALIDATION_QUBITS: usize = 6;
#[cfg(any(test, debug_assertions))]
const PATCH_VALIDATION_TOLERANCE: f64 = 1e-8;

#[derive(Debug, Clone)]
pub(crate) struct BlockPatch {
    /// Source order where the synthesized replacement is emitted.
    pub first_order: usize,
    /// Source operations consumed by this patch.
    pub matched_orders: Vec<usize>,
    /// Source operations preserved in place after post-synthesis commutation
    /// verification.
    pub crossed_orders: Vec<usize>,
    pub replacement: Vec<ValueOperation>,
    pub before_cost: ResynthesisCost,
    pub after_cost: ResynthesisCost,
    pub device_after_cost: Option<DevicePhysicalCost>,
    device_after_quality: Option<NativeQualityVector>,
    quality_policy: NativeQualityPolicy,
    pub synthesis_phase: f64,
    selection_rank: usize,
}

#[derive(Clone, Copy)]
struct DeviceSelectionContext<'a> {
    synthesis: Option<&'a DeviceTwoQubitSynthesisContext>,
    quality_policy: NativeQualityPolicy,
}

/// A block whose extraction invariants were rechecked against the current
/// operation stream. Synthesis functions accept this wrapper rather than a raw
/// collector result so cached or malformed blocks cannot bypass validation.
struct ValidatedBlock(TwoQubitNumericBlock);

impl std::ops::Deref for ValidatedBlock {
    type Target = TwoQubitNumericBlock;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
pub(crate) fn select_patches(
    blocks: Vec<TwoQubitNumericBlock>,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
) -> Result<Vec<BlockPatch>, CompilerError> {
    select_patches_with_device_policy(
        blocks,
        ops,
        commutation,
        config,
        None,
        &mut TwoQubitSynthesisCache::default(),
        NativeQualityPolicy::EntanglerFirst,
    )
}

pub(crate) fn select_patches_with_device_policy(
    blocks: Vec<TwoQubitNumericBlock>,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
    quality_policy: NativeQualityPolicy,
) -> Result<Vec<BlockPatch>, CompilerError> {
    let stats = synthesis_cache.selection_stats_mut();
    stats.input_blocks = stats.input_blocks.saturating_add(blocks.len());
    let mut seen = HashSet::new();
    let mut valid_blocks = Vec::new();
    for block in blocks {
        let Some(block) = validate_block(block, ops, commutation, config) else {
            let stats = synthesis_cache.selection_stats_mut();
            stats.invalid_blocks = stats.invalid_blocks.saturating_add(1);
            continue;
        };
        if !seen.insert(block.matched_orders.clone()) {
            let stats = synthesis_cache.selection_stats_mut();
            stats.duplicate_blocks = stats.duplicate_blocks.saturating_add(1);
            continue;
        }
        valid_blocks.push(block);
    }
    let mut blocks = valid_blocks;
    blocks.sort_by(|lhs, rhs| compare_blocks(lhs, rhs));

    let bounded = blocks
        .iter()
        .all(|block| matches!(block.origin(), BlockOrigin::DagDependencyClosed));
    if bounded && quality_policy == NativeQualityPolicy::EntanglerFirst {
        let stats = synthesis_cache.selection_stats_mut();
        stats.bounded_blocks = stats.bounded_blocks.saturating_add(blocks.len());
        return select_bounded_best_first(
            blocks,
            ops,
            commutation,
            config,
            device_context,
            synthesis_cache,
        );
    }

    let mut patches = Vec::new();
    for (selection_rank, block) in blocks.into_iter().enumerate() {
        if synthesis_cache.has_terminal_no_patch(&block, ops) {
            continue;
        }
        let patch = try_synthesize_block(
            &block,
            ops,
            commutation,
            config,
            DeviceSelectionContext {
                synthesis: device_context,
                quality_policy,
            },
            synthesis_cache,
            selection_rank,
        )?;
        if let Some(patch) = patch {
            let stats = synthesis_cache.selection_stats_mut();
            stats.patches_produced = stats.patches_produced.saturating_add(1);
            patches.push(patch);
        } else {
            synthesis_cache.record_terminal_no_patch(&block, ops);
        }
    }
    patches.sort_by(compare_patches);

    let mut covered = HashSet::new();
    let mut selected = Vec::new();
    for patch in patches {
        if patch
            .matched_orders
            .iter()
            .any(|order| covered.contains(order))
        {
            let stats = synthesis_cache.selection_stats_mut();
            stats.overlap_rejections = stats.overlap_rejections.saturating_add(1);
            continue;
        }
        for order in &patch.matched_orders {
            covered.insert(*order);
        }
        selected.push(patch);
    }
    selected.sort_by_key(|patch| patch.first_order);
    let stats = synthesis_cache.selection_stats_mut();
    stats.selected_patches = stats.selected_patches.saturating_add(selected.len());
    Ok(selected)
}

struct PreparedBlock {
    block: ValidatedBlock,
    facts: CachedBlockFactsHandle,
    selection_rank: usize,
}

#[derive(Clone, Copy, Default)]
struct SelectionAttemptCounts {
    plan_failures: usize,
    candidates_considered: usize,
    cost_rejections: usize,
    device_cost_rejections: usize,
    crossing_rejections: usize,
    replacement_rejections: usize,
}

fn record_attempt_counts(
    synthesis_cache: &mut TwoQubitSynthesisCache,
    counts: SelectionAttemptCounts,
) {
    let stats = synthesis_cache.selection_stats_mut();
    stats.plan_failures = stats.plan_failures.saturating_add(counts.plan_failures);
    stats.candidates_considered = stats
        .candidates_considered
        .saturating_add(counts.candidates_considered);
    stats.cost_rejections = stats.cost_rejections.saturating_add(counts.cost_rejections);
    stats.device_cost_rejections = stats
        .device_cost_rejections
        .saturating_add(counts.device_cost_rejections);
    stats.crossing_rejections = stats
        .crossing_rejections
        .saturating_add(counts.crossing_rejections);
    stats.replacement_rejections = stats
        .replacement_rejections
        .saturating_add(counts.replacement_rejections);
}

fn select_bounded_best_first(
    blocks: Vec<ValidatedBlock>,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<Vec<BlockPatch>, CompilerError> {
    let mut prepared = Vec::new();
    for (selection_rank, block) in blocks.into_iter().enumerate() {
        if synthesis_cache.has_terminal_no_patch(&block, ops) {
            continue;
        }
        let Some(facts) = load_block_facts(&block, ops, config, synthesis_cache)? else {
            synthesis_cache.record_terminal_no_patch(&block, ops);
            continue;
        };
        prepared.push(PreparedBlock {
            block,
            facts,
            selection_rank,
        });
    }

    let components = conflict_components_by_orders(prepared.len(), |index| {
        &prepared[index].block.matched_orders
    });
    let stats = synthesis_cache.selection_stats_mut();
    stats.conflict_components = stats.conflict_components.saturating_add(components.len());
    let mut selected = Vec::new();
    let mut states = vec![PreparedBlockState::Pending; prepared.len()];
    let mut bounds = vec![None; prepared.len()];
    let mut synthesized_patches = (0..prepared.len()).map(|_| None).collect::<Vec<_>>();
    for component in components {
        select_component_best_first(
            component,
            &prepared,
            &mut states,
            &mut bounds,
            &mut synthesized_patches,
            ops,
            commutation,
            config,
            device_context,
            synthesis_cache,
            &mut selected,
        )?;
    }
    selected.sort_by_key(|patch| patch.first_order);
    let stats = synthesis_cache.selection_stats_mut();
    stats.selected_patches = stats.selected_patches.saturating_add(selected.len());
    Ok(selected)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreparedBlockState {
    Pending,
    Estimated,
    Planned,
    Synthesized,
    Inactive,
}

#[allow(clippy::too_many_arguments)]
fn select_component_best_first(
    mut pending: Vec<usize>,
    prepared: &[PreparedBlock],
    states: &mut [PreparedBlockState],
    bounds: &mut [Option<PatchPriority>],
    patches: &mut [Option<BlockPatch>],
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device_context: Option<&DeviceTwoQubitSynthesisContext>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
    selected: &mut Vec<BlockPatch>,
) -> Result<(), CompilerError> {
    let mut pending_blocks = BinaryHeap::new();
    for &index in &pending {
        pending_blocks.push(Reverse((
            OptimisticPriority::Static(static_priority(&prepared[index])),
            index,
        )));
    }
    let mut candidates = BinaryHeap::new();
    let mut blocks_by_order = HashMap::<usize, Vec<usize>>::new();
    for index in pending.drain(..) {
        for &order in &prepared[index].block.matched_orders {
            blocks_by_order.entry(order).or_default().push(index);
        }
    }

    loop {
        while candidates
            .peek()
            .is_some_and(|entry: &Reverse<(PatchPriority, usize)>| {
                matches!(states[entry.0.1], PreparedBlockState::Inactive)
            })
        {
            candidates.pop();
        }
        while pending_blocks
            .peek()
            .is_some_and(|entry: &Reverse<(OptimisticPriority, usize)>| {
                let (priority, index) = entry.0;
                match states[index] {
                    PreparedBlockState::Pending => {
                        !matches!(priority, OptimisticPriority::Static(_))
                    }
                    PreparedBlockState::Estimated | PreparedBlockState::Planned => {
                        priority
                            != OptimisticPriority::Refined(
                                bounds[index],
                                static_priority(&prepared[index]),
                            )
                    }
                    PreparedBlockState::Synthesized | PreparedBlockState::Inactive => true,
                }
            })
        {
            pending_blocks.pop();
        }

        let best_index = candidates.peek().map(|entry| entry.0.1);
        let next_index = pending_blocks.peek().map(|entry| entry.0.1);
        let can_finalize = best_index.is_some_and(|best_index| {
            next_index.is_none_or(|next_index| {
                compare_patch_to_optimistic_bound(
                    patches[best_index]
                        .as_ref()
                        .expect("active candidate has a synthesized patch"),
                    &prepared[next_index],
                    bounds[next_index],
                ) != Ordering::Greater
            })
        });
        if can_finalize {
            let (_, winner_index) = candidates.pop().expect("finalized candidate exists").0;
            let winner = patches[winner_index]
                .take()
                .expect("finalized candidate has a patch");
            let mut pruned = 0usize;
            let mut rejected = 0usize;
            for order in &winner.matched_orders {
                let Some(conflicts) = blocks_by_order.get(order) else {
                    continue;
                };
                for &index in conflicts {
                    if index == winner_index
                        || matches!(states[index], PreparedBlockState::Inactive)
                    {
                        continue;
                    }
                    match states[index] {
                        PreparedBlockState::Pending
                        | PreparedBlockState::Estimated
                        | PreparedBlockState::Planned => pruned = pruned.saturating_add(1),
                        PreparedBlockState::Synthesized => rejected = rejected.saturating_add(1),
                        PreparedBlockState::Inactive => unreachable!(),
                    }
                    states[index] = PreparedBlockState::Inactive;
                }
            }
            states[winner_index] = PreparedBlockState::Inactive;
            let stats = synthesis_cache.selection_stats_mut();
            stats.lower_bound_pruned = stats.lower_bound_pruned.saturating_add(pruned);
            stats.overlap_rejections = stats.overlap_rejections.saturating_add(rejected);
            selected.push(winner);
            continue;
        }

        let Some(index) = next_index else {
            break;
        };
        pending_blocks.pop();
        let prepared_block = &prepared[index];
        if matches!(states[index], PreparedBlockState::Pending) {
            match estimate_block_priority_bound(
                prepared_block,
                ops,
                config,
                device_context,
                synthesis_cache,
            )? {
                BlockPriorityBound::Known(bound) => {
                    bounds[index] = Some(bound);
                    states[index] = PreparedBlockState::Estimated;
                    pending_blocks.push(Reverse((
                        OptimisticPriority::Refined(Some(bound), static_priority(prepared_block)),
                        index,
                    )));
                }
                BlockPriorityBound::Impossible => {
                    states[index] = PreparedBlockState::Inactive;
                    synthesis_cache.record_terminal_no_patch(&prepared_block.block, ops);
                }
                BlockPriorityBound::Unknown => {
                    states[index] = PreparedBlockState::Estimated;
                    pending_blocks.push(Reverse((
                        OptimisticPriority::Refined(None, static_priority(prepared_block)),
                        index,
                    )));
                }
            }
            continue;
        }
        if matches!(states[index], PreparedBlockState::Estimated) {
            match plan_block_priority_bound(
                prepared_block,
                ops,
                config,
                device_context,
                synthesis_cache,
            )? {
                BlockPriorityBound::Known(bound) => {
                    bounds[index] = Some(bound);
                    states[index] = PreparedBlockState::Planned;
                    pending_blocks.push(Reverse((
                        OptimisticPriority::Refined(Some(bound), static_priority(prepared_block)),
                        index,
                    )));
                }
                BlockPriorityBound::Impossible => {
                    states[index] = PreparedBlockState::Inactive;
                    synthesis_cache.record_terminal_no_patch(&prepared_block.block, ops);
                }
                BlockPriorityBound::Unknown => {
                    states[index] = PreparedBlockState::Planned;
                    pending_blocks.push(Reverse((
                        OptimisticPriority::Refined(None, static_priority(prepared_block)),
                        index,
                    )));
                }
            }
            continue;
        }
        debug_assert!(matches!(states[index], PreparedBlockState::Planned));
        let patch = try_synthesize_block_from_facts(
            &prepared_block.block,
            ops,
            commutation,
            config,
            DeviceSelectionContext {
                synthesis: device_context,
                quality_policy: NativeQualityPolicy::EntanglerFirst,
            },
            synthesis_cache,
            &prepared_block.facts,
            prepared_block.selection_rank,
        )?;
        if let Some(patch) = patch {
            if let Some(bound) = bounds[index] {
                debug_assert!(
                    patch_priority(&patch) >= bound,
                    "validated patch outranked its admissible planning bound"
                );
            }
            let stats = synthesis_cache.selection_stats_mut();
            stats.patches_produced = stats.patches_produced.saturating_add(1);
            candidates.push(Reverse((patch_priority(&patch), index)));
            patches[index] = Some(patch);
            states[index] = PreparedBlockState::Synthesized;
        } else {
            states[index] = PreparedBlockState::Inactive;
            synthesis_cache.record_terminal_no_patch(&prepared_block.block, ops);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PatchPriority {
    device_after_cost: Option<DevicePhysicalCost>,
    after_cost: ResynthesisCost,
    reduction: usize,
    matched_orders: usize,
    first_order: usize,
    selection_rank: usize,
}

impl Eq for PatchPriority {}

impl PartialOrd for PatchPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PatchPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_some_first_by(
            self.device_after_cost,
            other.device_after_cost,
            |left, right| left.compare(right),
        )
        .then_with(|| self.after_cost.cmp(&other.after_cost))
        .then_with(|| other.reduction.cmp(&self.reduction))
        .then_with(|| Reverse(self.matched_orders).cmp(&Reverse(other.matched_orders)))
        .then_with(|| self.first_order.cmp(&other.first_order))
        .then_with(|| self.selection_rank.cmp(&other.selection_rank))
    }
}

fn patch_priority(patch: &BlockPatch) -> PatchPriority {
    let reduction = patch
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(patch.after_cost.lowered_two_qubit_ops);
    PatchPriority {
        device_after_cost: patch.device_after_cost,
        after_cost: patch.after_cost,
        reduction,
        matched_orders: patch.matched_orders.len(),
        first_order: patch.first_order,
        selection_rank: patch.selection_rank,
    }
}

type StaticPriority = PatchPriority;

fn static_priority(block: &PreparedBlock) -> StaticPriority {
    // This universal bound is only the initial heap key. Every live block is
    // refined through KAK coordinates before it can reach full synthesis.
    PatchPriority {
        device_after_cost: None,
        after_cost: ResynthesisCost::default(),
        reduction: block.facts.source_cost.lowered_two_qubit_ops,
        matched_orders: block.block.matched_orders.len(),
        first_order: block.block.first_order(),
        selection_rank: block.selection_rank,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptimisticPriority {
    Static(StaticPriority),
    Refined(Option<PatchPriority>, StaticPriority),
}

impl OptimisticPriority {
    fn parts(self) -> (Option<PatchPriority>, StaticPriority) {
        match self {
            Self::Static(fallback) => (None, fallback),
            Self::Refined(bound, fallback) => (bound, fallback),
        }
    }
}

impl PartialOrd for OptimisticPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OptimisticPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        let (left, left_fallback) = self.parts();
        let (right, right_fallback) = other.parts();
        match (left, right) {
            (Some(left), Some(right)) => left.cmp(&right),
            // An unrefined or unknown bound retains the universal zero-cost
            // lower bound. A generic zero-cost plan can still prove it wins
            // against that exact static priority; physical plans cannot make
            // such a claim without a device-derived bound.
            (None, Some(right)) if right.device_after_cost.is_none() => left_fallback.cmp(&right),
            (Some(left), None) if left.device_after_cost.is_none() => left.cmp(&right_fallback),
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (None, None) => left_fallback.cmp(&right_fallback),
        }
    }
}

fn compare_patch_to_optimistic_bound(
    patch: &BlockPatch,
    block: &PreparedBlock,
    bound: Option<PatchPriority>,
) -> Ordering {
    if let Some(bound) = bound {
        return patch_priority(patch).cmp(&bound);
    }
    let patch_reduction = patch
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(patch.after_cost.lowered_two_qubit_ops);
    patch
        .after_cost
        .cmp(&ResynthesisCost::default())
        .then_with(|| {
            block
                .facts
                .source_cost
                .lowered_two_qubit_ops
                .cmp(&patch_reduction)
        })
        .then_with(|| {
            Reverse(patch.matched_orders.len()).cmp(&Reverse(block.block.matched_orders.len()))
        })
        .then_with(|| patch.first_order.cmp(&block.block.first_order()))
        .then_with(|| patch.selection_rank.cmp(&block.selection_rank))
}

fn conflict_components_by_orders<'a>(
    block_count: usize,
    matched_orders: impl Fn(usize) -> &'a [usize],
) -> Vec<Vec<usize>> {
    let mut parents = (0..block_count).collect::<Vec<_>>();
    let mut first_by_order = HashMap::<usize, usize>::new();
    for index in 0..block_count {
        for &order in matched_orders(index) {
            if let Some(&other) = first_by_order.get(&order) {
                union_components(&mut parents, index, other);
            } else {
                first_by_order.insert(order, index);
            }
        }
    }
    let mut by_root = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..block_count {
        let root = find_component(&mut parents, index);
        by_root.entry(root).or_default().push(index);
    }
    by_root.into_values().collect()
}

fn find_component(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

fn union_components(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find_component(parents, left);
    let right_root = find_component(parents, right);
    if left_root != right_root {
        parents[right_root] = left_root;
    }
}

fn compare_blocks(lhs: &TwoQubitNumericBlock, rhs: &TwoQubitNumericBlock) -> Ordering {
    // Synthesis is the expensive step. Prefer candidates most likely to improve:
    // more 2q gates, larger matched unitary, denser span, SWAP involvement, then
    // smaller source span and earlier deterministic position.
    let lhs_density = lhs.matched_orders.len() * rhs.span();
    let rhs_density = rhs.matched_orders.len() * lhs.span();
    Reverse(lhs.matched_2q_count)
        .cmp(&Reverse(rhs.matched_2q_count))
        .then_with(|| Reverse(lhs.matched_orders.len()).cmp(&Reverse(rhs.matched_orders.len())))
        .then_with(|| Reverse(lhs_density).cmp(&Reverse(rhs_density)))
        .then_with(|| Reverse(lhs.contains_swap).cmp(&Reverse(rhs.contains_swap)))
        .then_with(|| lhs.span().cmp(&rhs.span()))
        .then_with(|| lhs.first_order().cmp(&rhs.first_order()))
}

fn compare_patches(lhs: &BlockPatch, rhs: &BlockPatch) -> Ordering {
    let lhs_reduction = lhs
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(lhs.after_cost.lowered_two_qubit_ops);
    let rhs_reduction = rhs
        .before_cost
        .lowered_two_qubit_ops
        .saturating_sub(rhs.after_cost.lowered_two_qubit_ops);

    debug_assert_eq!(lhs.quality_policy, rhs.quality_policy);
    let physical = match (lhs.device_after_quality, rhs.device_after_quality) {
        (Some(left), Some(right)) => left.compare(right, lhs.quality_policy),
        _ => compare_some_first_by(
            lhs.device_after_cost,
            rhs.device_after_cost,
            |left, right| left.compare(right),
        ),
    };

    physical
        .then_with(|| lhs.after_cost.cmp(&rhs.after_cost))
        .then_with(|| rhs_reduction.cmp(&lhs_reduction))
        .then_with(|| Reverse(lhs.matched_orders.len()).cmp(&Reverse(rhs.matched_orders.len())))
        .then_with(|| lhs.first_order.cmp(&rhs.first_order))
        .then_with(|| lhs.selection_rank.cmp(&rhs.selection_rank))
}

fn try_synthesize_block(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device: DeviceSelectionContext<'_>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
    selection_rank: usize,
) -> Result<Option<BlockPatch>, CompilerError> {
    let Some(facts) = load_block_facts(block, ops, config, synthesis_cache)? else {
        return Ok(None);
    };
    try_synthesize_block_from_facts(
        block,
        ops,
        commutation,
        config,
        device,
        synthesis_cache,
        &facts,
        selection_rank,
    )
}

fn load_block_facts(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    config: &TwoQubitBlockResynthesisConfig,
    synthesis_cache: &mut TwoQubitSynthesisCache,
) -> Result<Option<CachedBlockFactsHandle>, CompilerError> {
    let matched = block
        .matched_orders
        .iter()
        .map(|&order| &ops[order])
        .collect::<Vec<_>>();
    let stats = synthesis_cache.selection_stats_mut();
    stats.source_fact_attempts = stats.source_fact_attempts.saturating_add(1);
    let facts = synthesis_cache.block_facts(block, ops, || {
        let source_operations = value_operations_of_source_ops(&matched)?;
        let source_cost =
            cost_of_source_value_operations(&source_operations, &config.two_qubit_target)?;
        Ok(CachedBlockFacts {
            matrix: OnceLock::new(),
            source_operations,
            source_cost,
        })
    })?;
    if facts.is_none() {
        let stats = synthesis_cache.selection_stats_mut();
        stats.source_fact_failures = stats.source_fact_failures.saturating_add(1);
    }
    Ok(facts)
}

#[allow(clippy::too_many_arguments)]
fn try_synthesize_block_from_facts(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
    device: DeviceSelectionContext<'_>,
    synthesis_cache: &mut TwoQubitSynthesisCache,
    facts: &CachedBlockFacts,
    selection_rank: usize,
) -> Result<Option<BlockPatch>, CompilerError> {
    if facts.matrix.get().is_none() {
        let stats = synthesis_cache.selection_stats_mut();
        stats.matrix_attempts = stats.matrix_attempts.saturating_add(1);
    }
    let Some(matrix) = facts
        .matrix
        .get_or_init(|| block_matrix(block, ops).ok())
        .as_ref()
    else {
        return Ok(None);
    };
    let before_cost = facts.source_cost;

    let crossed = block
        .crossed_orders
        .iter()
        .map(|&order| &ops[order])
        .collect::<Vec<_>>();
    let stats = synthesis_cache.selection_stats_mut();
    stats.synthesis_attempts = stats.synthesis_attempts.saturating_add(1);
    if let Some(device_context) = device.synthesis {
        return try_synthesize_device_block(
            block,
            ops,
            matrix,
            &facts.source_operations,
            &crossed,
            commutation,
            device_context,
            before_cost,
            synthesis_cache,
            selection_rank,
            device.quality_policy,
        );
    }
    let mut counts = SelectionAttemptCounts::default();
    let Some(mut candidates) = synthesis_cache.with_generic_plan(
        matrix,
        block.qubits,
        |cache| {
            if !cache.artifact_reuse_enabled() {
                return plan_numeric_2q_unitary(TwoQubitSynthesisRequest {
                    matrix,
                    qubits: block.qubits,
                    target: config.two_qubit_target.clone(),
                });
            }
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for resynthesis matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_from_kak(
                TwoQubitSynthesisRequest {
                    matrix,
                    qubits: block.qubits,
                    target: config.two_qubit_target.clone(),
                },
                &decomp,
            )
        },
        |plan| {
            let CachedPlanView::Candidates(candidates) = plan else {
                return None;
            };
            Some(candidates.to_vec())
        },
    )?
    else {
        counts.plan_failures = counts.plan_failures.saturating_add(1);
        record_attempt_counts(synthesis_cache, counts);
        return Ok(None);
    };
    let mut expanded_backends = HashSet::new();
    let mut pauli_generated = candidates
        .iter()
        .any(|candidate| candidate.backend == TwoQubitUnitaryDecomposeBasis::PauliRotations);
    let mut saw_exact_primary = false;
    let mut primary_unrankable = false;
    loop {
        candidates.sort_by(|left, right| {
            left.cost
                .cmp(&right.cost)
                .then_with(|| left.operations.len().cmp(&right.operations.len()))
        });
        let Some(candidate) = candidates.first().cloned() else {
            if !pauli_generated && !saw_exact_primary && !primary_unrankable {
                pauli_generated = true;
                let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
                    record_attempt_counts(synthesis_cache, counts);
                    return Ok(None);
                };
                candidates.extend(plan_pauli_fallback_from_kak(
                    TwoQubitSynthesisRequest {
                        matrix,
                        qubits: block.qubits,
                        target: config.two_qubit_target.clone(),
                    },
                    &decomp,
                )?);
                continue;
            }
            record_attempt_counts(synthesis_cache, counts);
            return Ok(None);
        };
        candidates.remove(0);
        counts.candidates_considered = counts.candidates_considered.saturating_add(1);
        if candidate.cost >= before_cost {
            counts.cost_rejections = counts.cost_rejections.saturating_add(1);
            primary_unrankable = true;
            continue;
        }
        match synthesis_cache.check_selected_candidate(matrix, block.qubits, &candidate)? {
            NumericTwoQubitCandidateValidation::Exact => {
                saw_exact_primary = true;
                if !commutation.replacements_commute_with_crossed(&crossed, &candidate.operations) {
                    counts.crossing_rejections = counts.crossing_rejections.saturating_add(1);
                    continue;
                }
                if !replacement_respects_block(block, &candidate.operations, candidate.global_phase)
                {
                    counts.replacement_rejections = counts.replacement_rejections.saturating_add(1);
                    continue;
                }
                record_attempt_counts(synthesis_cache, counts);
                validate_patch_with_differential_oracle(
                    block,
                    ops,
                    &candidate.operations,
                    candidate.global_phase,
                )?;
                return Ok(Some(BlockPatch {
                    first_order: block.first_order(),
                    matched_orders: block.matched_orders.clone(),
                    crossed_orders: block.crossed_orders.clone(),
                    replacement: candidate.operations,
                    before_cost,
                    after_cost: candidate.cost,
                    device_after_cost: None,
                    device_after_quality: None,
                    quality_policy: device.quality_policy,
                    synthesis_phase: candidate.global_phase,
                    selection_rank,
                }));
            }
            NumericTwoQubitCandidateValidation::Inexact { .. }
                if expanded_backends.insert(candidate.backend) =>
            {
                let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
                    record_attempt_counts(synthesis_cache, counts);
                    return Ok(None);
                };
                candidates.extend(plan_cx_family_fallbacks_from_kak(
                    TwoQubitSynthesisRequest {
                        matrix,
                        qubits: block.qubits,
                        target: config.two_qubit_target.clone(),
                    },
                    &decomp,
                    candidate.backend,
                )?);
            }
            NumericTwoQubitCandidateValidation::Inexact { .. } => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn try_synthesize_device_block(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    matrix: &Array2<Complex64>,
    source_operations: &[ValueOperation],
    crossed: &[&OperationView<'_>],
    commutation: &CachedCommutation,
    context: &DeviceTwoQubitSynthesisContext,
    before_cost: ResynthesisCost,
    synthesis_cache: &mut TwoQubitSynthesisCache,
    selection_rank: usize,
    quality_policy: NativeQualityPolicy,
) -> Result<Option<BlockPatch>, CompilerError> {
    let quality_policy = if context.placement() == DeviceSynthesisPlacement::ExactPhysical {
        quality_policy
    } else {
        NativeQualityPolicy::EntanglerFirst
    };
    let (device_before_cost, source_domain) = match context.placement() {
        DeviceSynthesisPlacement::PreLayoutEnvelope => {
            let Some(evaluation) = context.evaluate_pre_layout(source_operations, block.qubits)
            else {
                return Ok(None);
            };
            (evaluation.worst_cost, Some(evaluation.domain))
        }
        DeviceSynthesisPlacement::ExactPhysical => {
            match context.exact_cost_diagnostic(source_operations, block.qubits) {
                Ok(cost) => (cost, None),
                Err(DeviceContextCostFailure::Unsupported(_)) => return Ok(None),
                Err(DeviceContextCostFailure::Unprepared(state)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "device resynthesis context was not prepared for source state {state:?}"
                    )));
                }
                Err(DeviceContextCostFailure::WrongPlacement) => {
                    return Err(CompilerError::InvariantViolation(
                        "exact device resynthesis used a pre-layout context".to_string(),
                    ));
                }
                Err(DeviceContextCostFailure::InvalidOperation(reason)) => {
                    return Err(CompilerError::InvariantViolation(format!(
                        "invalid exact-device resynthesis source: {reason}"
                    )));
                }
            }
        }
    };
    let device_before_quality = if quality_policy == NativeQualityPolicy::BalancedDepth {
        let leaves = context
            .exact_native_leaves_diagnostic(source_operations, block.qubits)
            .map_err(|failure| {
                CompilerError::InvariantViolation(format!(
                    "failed to expand exact-device resynthesis source: {failure:?}"
                ))
            })?;
        NativeQualityVector::for_native_plan_leaves(device_before_cost, &leaves)
    } else {
        NativeQualityVector::for_value_operations(device_before_cost, source_operations)
    };
    let device_before_schedule = if quality_policy == NativeQualityPolicy::BalancedDepth {
        Some(
            context
                .exact_schedule_profile_diagnostic(source_operations, block.qubits)
                .map_err(|failure| {
                    CompilerError::InvariantViolation(format!(
                        "failed to profile exact-device resynthesis source schedule: {failure:?}"
                    ))
                })?,
        )
    } else {
        None
    };
    let mut counts = SelectionAttemptCounts::default();
    let Some(mut candidates) = synthesis_cache.with_device_plan(
        matrix,
        block.qubits,
        context.placement(),
        |cache| {
            if !cache.artifact_reuse_enabled() {
                return plan_numeric_2q_unitary_for_device(matrix, block.qubits, context);
            }
            let decomp = cache.kak_decomposition(matrix)?.ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "cached KAK decomposition failed for device resynthesis matrix".to_string(),
                )
            })?;
            plan_numeric_2q_unitary_for_device_from_kak(matrix, block.qubits, context, &decomp)
        },
        |plan| {
            let CachedPlanView::Candidates(candidates) = plan else {
                return None;
            };
            Some(candidates.to_vec())
        },
    )?
    else {
        counts.plan_failures = counts.plan_failures.saturating_add(1);
        record_attempt_counts(synthesis_cache, counts);
        return Ok(None);
    };
    let mut expanded_backends = HashSet::new();
    let mut pauli_generated = candidates.iter().any(|candidate| {
        candidate.candidate.backend
            == crate::compile::transform::decompose::unitary::TwoQubitUnitaryDecomposeBasis::PauliRotations
    });
    let mut saw_exact_primary = false;
    let mut primary_unrankable = false;
    loop {
        let mut ranked = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            let device_after_cost = match context.placement() {
                DeviceSynthesisPlacement::PreLayoutEnvelope => {
                    let Some(source_domain) = source_domain.as_ref() else {
                        counts.device_cost_rejections =
                            counts.device_cost_rejections.saturating_add(1);
                        primary_unrankable = true;
                        continue;
                    };
                    let Some(evaluation) = candidate.pre_layout.as_ref() else {
                        counts.device_cost_rejections =
                            counts.device_cost_rejections.saturating_add(1);
                        primary_unrankable = true;
                        continue;
                    };
                    if !source_domain.is_subset(&evaluation.domain) {
                        counts.device_cost_rejections =
                            counts.device_cost_rejections.saturating_add(1);
                        primary_unrankable = true;
                        continue;
                    }
                    let Some(cost) = evaluation.worst_cost_on_domain(source_domain) else {
                        counts.device_cost_rejections =
                            counts.device_cost_rejections.saturating_add(1);
                        primary_unrankable = true;
                        continue;
                    };
                    cost
                }
                DeviceSynthesisPlacement::ExactPhysical => candidate.physical_cost,
            };
            let device_after_quality = if quality_policy == NativeQualityPolicy::BalancedDepth {
                let leaves = context
                    .exact_native_leaves_diagnostic(&candidate.candidate.operations, block.qubits)
                    .map_err(|failure| {
                        CompilerError::InvariantViolation(format!(
                            "failed to expand exact-device resynthesis candidate: {failure:?}"
                        ))
                    })?;
                NativeQualityVector::for_native_plan_leaves(device_after_cost, &leaves)
            } else {
                NativeQualityVector::for_value_operations(
                    device_after_cost,
                    &candidate.candidate.operations,
                )
            };
            ranked.push((index, candidate, device_after_cost, device_after_quality));
        }
        ranked.sort_by(|(_, left, _, left_quality), (_, right, _, right_quality)| {
            left_quality
                .compare(*right_quality, quality_policy)
                .then_with(|| left.candidate.cost.cmp(&right.candidate.cost))
        });
        let Some((index, _, device_after_cost, device_after_quality)) = ranked.first().copied()
        else {
            if !pauli_generated && !saw_exact_primary && !primary_unrankable {
                pauli_generated = true;
                let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
                    record_attempt_counts(synthesis_cache, counts);
                    return Ok(None);
                };
                candidates.extend(plan_pauli_fallback_for_device_from_kak(
                    matrix,
                    block.qubits,
                    context,
                    &decomp,
                )?);
                continue;
            }
            record_attempt_counts(synthesis_cache, counts);
            return Ok(None);
        };
        let candidate = candidates.remove(index);
        counts.candidates_considered = counts.candidates_considered.saturating_add(1);
        match synthesis_cache.check_selected_candidate(
            matrix,
            block.qubits,
            &candidate.candidate,
        )? {
            NumericTwoQubitCandidateValidation::Exact => {
                saw_exact_primary = true;
                let schedule_is_no_worse = if let Some(before_schedule) = &device_before_schedule {
                    match context.exact_schedule_profile_diagnostic(
                        &candidate.candidate.operations,
                        block.qubits,
                    ) {
                        Ok(after_schedule) => after_schedule.dominance(before_schedule).is_some(),
                        Err(DeviceContextCostFailure::Unsupported(_)) => false,
                        Err(failure) => {
                            return Err(CompilerError::InvariantViolation(format!(
                                "failed to profile exact-device resynthesis candidate schedule: {failure:?}"
                            )));
                        }
                    }
                } else {
                    true
                };
                if !device_after_quality.admissible_against(device_before_quality, quality_policy)
                    || !device_after_quality
                        .compare(device_before_quality, quality_policy)
                        .is_lt()
                    || !schedule_is_no_worse
                {
                    counts.device_cost_rejections = counts.device_cost_rejections.saturating_add(1);
                    continue;
                }
                if !commutation
                    .replacements_commute_with_crossed(crossed, &candidate.candidate.operations)
                {
                    counts.crossing_rejections = counts.crossing_rejections.saturating_add(1);
                    continue;
                }
                if !replacement_respects_block(
                    block,
                    &candidate.candidate.operations,
                    candidate.candidate.global_phase,
                ) {
                    counts.replacement_rejections = counts.replacement_rejections.saturating_add(1);
                    continue;
                }
                record_attempt_counts(synthesis_cache, counts);
                validate_patch_with_differential_oracle(
                    block,
                    ops,
                    &candidate.candidate.operations,
                    candidate.candidate.global_phase,
                )?;
                return Ok(Some(BlockPatch {
                    first_order: block.first_order(),
                    matched_orders: block.matched_orders.clone(),
                    crossed_orders: block.crossed_orders.clone(),
                    replacement: candidate.candidate.operations,
                    before_cost,
                    after_cost: candidate.candidate.cost,
                    device_after_cost: Some(device_after_cost),
                    device_after_quality: Some(device_after_quality),
                    quality_policy,
                    synthesis_phase: candidate.candidate.global_phase,
                    selection_rank,
                }));
            }
            NumericTwoQubitCandidateValidation::Inexact { .. }
                if expanded_backends
                    .insert((candidate.candidate.backend, candidate.direction_order)) =>
            {
                let Some(decomp) = synthesis_cache.kak_decomposition(matrix)? else {
                    record_attempt_counts(synthesis_cache, counts);
                    return Ok(None);
                };
                candidates.extend(plan_cx_family_fallbacks_for_device_from_kak(
                    matrix,
                    block.qubits,
                    context,
                    &decomp,
                    candidate.candidate.backend,
                    candidate.direction_order,
                )?);
            }
            NumericTwoQubitCandidateValidation::Inexact { .. } => {}
        }
    }
}

// Matrix construction uses the same convention as `circuit_to_matrix`: source
// operations are multiplied as `gate_n * ... * gate_0`, where `gate_0` is the
// earliest source operation. `block.qubits[0]` is the first tensor factor.
fn block_matrix(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
) -> Result<Array2<Complex64>, CompilerError> {
    let mut orders = block.matched_orders.clone();
    orders.sort_unstable();
    let mut resolved = Vec::with_capacity(orders.len());
    for order in orders {
        let view = &ops[order];
        let Instruction::Standard(gate) = view.operation.instruction else {
            return Err(CompilerError::InvariantViolation(
                "resynthesis matrix requested for non-standard operation".to_string(),
            ));
        };
        let params = view
            .params
            .iter()
            .map(|param| param.evaluate(&None))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                CompilerError::InvariantViolation(
                    "resynthesis matrix requested for symbolic operation".to_string(),
                )
            })?;
        resolved.push(TwoQubitMatrixOp {
            gate,
            qubits: view.operation.qubits.iter().copied().collect(),
            params,
        });
    }
    two_qubit_operation_matrix_product(
        &resolved,
        0.0,
        block.qubits,
        "resynthesis block contains operation outside canonical qubits",
    )
}

fn validate_block(
    block: TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    commutation: &CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
) -> Option<ValidatedBlock> {
    if block.qubits[0] == block.qubits[1]
        || block.matched_orders.is_empty()
        || !strictly_sorted_unique(&block.matched_orders)
        || !strictly_sorted_unique(&block.crossed_orders)
        || block
            .matched_orders
            .iter()
            .any(|order| block.crossed_orders.binary_search(order).is_ok())
    {
        return None;
    }
    match block.origin() {
        BlockOrigin::MaximalClosedRun if !block.crossed_orders.is_empty() => return None,
        BlockOrigin::MaximalClosedRun | BlockOrigin::DagDependencyClosed => {}
    }

    let span_start = block
        .matched_orders
        .first()
        .copied()
        .into_iter()
        .chain(block.crossed_orders.first().copied())
        .min()?;
    let span_end = block
        .matched_orders
        .last()
        .copied()
        .into_iter()
        .chain(block.crossed_orders.last().copied())
        .max()?;
    if span_end >= ops.len() {
        return None;
    }

    // A collector proof is closed only when every source operation inside its
    // extraction span that touches the pair is explicitly matched or crossed.
    // The two sorted cursors keep this check linear and allocation-free.
    let mut matched_cursor = 0;
    let mut crossed_cursor = 0;
    for (order, view) in ops.iter().enumerate().take(span_end + 1).skip(span_start) {
        if view.order != order {
            return None;
        }
        let matched = block.matched_orders.get(matched_cursor) == Some(&order);
        let crossed = block.crossed_orders.get(crossed_cursor) == Some(&order);
        if matched {
            matched_cursor += 1;
        }
        if crossed {
            crossed_cursor += 1;
        }
        if !matched
            && !crossed
            && view
                .operation
                .qubits
                .iter()
                .any(|qubit| block.qubits.contains(qubit))
        {
            return None;
        }
    }

    let mut one_qubit = 0usize;
    let mut two_qubit = 0usize;
    let mut contains_swap = false;
    for &order in &block.matched_orders {
        let view = ops.get(order)?;
        if !is_fixed_numeric_standard(view)
            || view
                .operation
                .qubits
                .iter()
                .any(|qubit| !block.qubits.contains(qubit))
        {
            return None;
        }
        match view.operation.qubits.len() {
            1 => one_qubit += 1,
            2 => two_qubit += 1,
            _ => return None,
        }
        contains_swap |= matches!(
            view.operation.instruction,
            Instruction::Standard(StandardGate::SWAP)
        );
    }
    if one_qubit != block.matched_1q_count
        || two_qubit != block.matched_2q_count
        || contains_swap != block.contains_swap
    {
        return None;
    }

    for &crossed_order in &block.crossed_orders {
        let crossed = ops.get(crossed_order)?;
        if is_hard_boundary(crossed, config) {
            return None;
        }
        for &matched_order in &block.matched_orders {
            let matched = &ops[matched_order];
            if matched
                .operation
                .qubits
                .iter()
                .any(|qubit| crossed.operation.qubits.contains(qubit))
                && !commutation.commute_ops_rechecked(matched, crossed)
            {
                return None;
            }
        }
    }

    Some(ValidatedBlock(block))
}

fn strictly_sorted_unique(values: &[usize]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn replacement_respects_block(
    block: &TwoQubitNumericBlock,
    replacement: &[ValueOperation],
    synthesis_phase: f64,
) -> bool {
    synthesis_phase.is_finite()
        && replacement.iter().all(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(_))
            ) && operation
                .qubits
                .iter()
                .all(|qubit| block.qubits.contains(qubit))
                && operation.params.iter().all(|param| {
                    matches!(param, crate::circuit::ParameterValue::Fixed(value) if value.is_finite())
                })
        })
}

#[cfg(any(test, debug_assertions))]
fn validate_patch_with_differential_oracle(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    replacement: &[ValueOperation],
    synthesis_phase: f64,
) -> Result<(), CompilerError> {
    #[cfg(not(test))]
    {
        // Stable sampling makes failures reproducible without introducing RNG
        // state into the compiler. Tests validate every accepted candidate.
        let fingerprint = block.matched_orders.iter().fold(
            block.qubits[0].index() ^ block.qubits[1].index().rotate_left(7),
            |hash, order| hash.rotate_left(5) ^ order,
        );
        if fingerprint & 63 != 0 {
            return Ok(());
        }
    }

    match relevant_span_oracle(block, ops, replacement, synthesis_phase, &mut None)? {
        SpanOracleOutcome::Mismatch => Err(CompilerError::InvariantViolation(format!(
            "resynthesis structural proof disagreed with span oracle at source order {}",
            block.first_order()
        ))),
        SpanOracleOutcome::Equivalent | SpanOracleOutcome::Unavailable => Ok(()),
    }
}

#[cfg(not(any(test, debug_assertions)))]
#[inline(always)]
fn validate_patch_with_differential_oracle(
    _block: &TwoQubitNumericBlock,
    _ops: &[OperationView<'_>],
    _replacement: &[ValueOperation],
    _synthesis_phase: f64,
) -> Result<(), CompilerError> {
    Ok(())
}

#[cfg(any(test, debug_assertions))]
struct RelevantSpanValidation {
    qubits: Vec<Qubit>,
    source_matrix: Array2<Complex64>,
    converted: Vec<Option<ValueOperation>>,
    included_orders: HashSet<usize>,
    matched_orders: HashSet<usize>,
    span_start: usize,
    span_end: usize,
}

#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpanOracleOutcome {
    Equivalent,
    Mismatch,
    Unavailable,
}

#[cfg(test)]
fn patch_preserves_relevant_span(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    replacement: &[ValueOperation],
    synthesis_phase: f64,
) -> Result<bool, CompilerError> {
    Ok(matches!(
        relevant_span_oracle(block, ops, replacement, synthesis_phase, &mut None)?,
        SpanOracleOutcome::Equivalent
    ))
}

#[cfg(any(test, debug_assertions))]
fn relevant_span_oracle(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
    replacement: &[ValueOperation],
    synthesis_phase: f64,
    prepared: &mut Option<RelevantSpanValidation>,
) -> Result<SpanOracleOutcome, CompilerError> {
    if prepared.is_none() {
        *prepared = prepare_relevant_span(block, ops)?;
    }
    let Some(prepared) = prepared else {
        return Ok(SpanOracleOutcome::Unavailable);
    };
    if replacement
        .iter()
        .flat_map(|operation| operation.qubits.iter())
        .any(|qubit| !prepared.qubits.contains(qubit))
    {
        return Ok(SpanOracleOutcome::Mismatch);
    }

    let mut replacement_ops = Vec::new();
    for (order, cached) in prepared
        .converted
        .iter()
        .enumerate()
        .take(prepared.span_end + 1)
        .skip(prepared.span_start)
    {
        if order == block.first_order() {
            replacement_ops.extend(replacement.iter().cloned());
        }
        if prepared.matched_orders.contains(&order) {
            continue;
        }
        if prepared.included_orders.contains(&order) {
            replacement_ops.push(
                cached
                    .as_ref()
                    .expect("included operation was converted")
                    .clone(),
            );
        }
    }
    let Ok(replacement_matrix) =
        value_operations_to_matrix(&prepared.qubits, &replacement_ops, synthesis_phase)
    else {
        return Ok(SpanOracleOutcome::Unavailable);
    };
    let equivalent = prepared.source_matrix.shape() == replacement_matrix.shape()
        && prepared
            .source_matrix
            .iter()
            .zip(replacement_matrix.iter())
            .all(|(source, replacement)| {
                (*source - *replacement).norm() <= PATCH_VALIDATION_TOLERANCE
            });
    Ok(if equivalent {
        SpanOracleOutcome::Equivalent
    } else {
        SpanOracleOutcome::Mismatch
    })
}

#[cfg(any(test, debug_assertions))]
fn prepare_relevant_span(
    block: &TwoQubitNumericBlock,
    ops: &[OperationView<'_>],
) -> Result<Option<RelevantSpanValidation>, CompilerError> {
    let mut relevant_qubits = HashSet::new();
    let mut included_orders = block
        .matched_orders
        .iter()
        .chain(&block.crossed_orders)
        .copied()
        .collect::<HashSet<_>>();
    for &order in &included_orders {
        relevant_qubits.extend(ops[order].operation.qubits.iter().copied());
    }
    if relevant_qubits.len() > MAX_PATCH_VALIDATION_QUBITS {
        return Ok(None);
    }

    let Some(span_start) = included_orders.iter().min().copied() else {
        return Ok(None);
    };
    let Some(span_end) = included_orders.iter().max().copied() else {
        return Ok(None);
    };

    let mut changed = true;
    let mut converted = vec![None; ops.len()];
    while changed {
        changed = false;
        for (order, view) in ops.iter().enumerate().take(span_end + 1).skip(span_start) {
            if included_orders.contains(&order) {
                continue;
            }
            if !view.operation.qubits.is_empty()
                && !view
                    .operation
                    .qubits
                    .iter()
                    .any(|qubit| relevant_qubits.contains(qubit))
            {
                continue;
            }
            let Some(operation) = operation_view_to_value(view) else {
                return Ok(None);
            };
            converted[order] = Some(operation);
            included_orders.insert(order);
            relevant_qubits.extend(view.operation.qubits.iter().copied());
            if relevant_qubits.len() > MAX_PATCH_VALIDATION_QUBITS {
                return Ok(None);
            }
            changed = true;
        }
    }

    for &order in &included_orders {
        if converted[order].is_none() {
            let Some(operation) = operation_view_to_value(&ops[order]) else {
                return Ok(None);
            };
            converted[order] = Some(operation);
        }
    }

    let mut source_ops = Vec::new();
    let matched_orders = block.matched_orders.iter().copied().collect::<HashSet<_>>();
    for (order, cached) in converted
        .iter()
        .enumerate()
        .take(span_end + 1)
        .skip(span_start)
    {
        if included_orders.contains(&order) {
            source_ops.push(
                cached
                    .as_ref()
                    .expect("included operation was converted")
                    .clone(),
            );
        }
    }

    let mut qubits = relevant_qubits.into_iter().collect::<Vec<_>>();
    qubits.sort_by_key(|qubit| qubit.index());

    let Ok(source_matrix) = value_operations_to_matrix(&qubits, &source_ops, 0.0) else {
        return Ok(None);
    };
    Ok(Some(RelevantSpanValidation {
        qubits,
        source_matrix,
        converted,
        included_orders,
        matched_orders,
        span_start,
        span_end,
    }))
}

#[cfg(any(test, debug_assertions))]
fn operation_view_to_value(view: &OperationView<'_>) -> Option<ValueOperation> {
    let Instruction::Standard(gate) = view.operation.instruction else {
        return None;
    };
    let params = view
        .params
        .iter()
        .map(|param| param.evaluate(&None))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if params.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some(ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(gate)),
        qubits: view.operation.qubits.clone(),
        params: params.into_iter().map(ParameterValue::Fixed).collect(),
        label: view.operation.label.clone(),
    })
}

#[cfg(test)]
#[path = "selector_test.rs"]
mod selector_test;
