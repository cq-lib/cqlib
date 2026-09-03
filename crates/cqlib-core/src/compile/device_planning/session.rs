// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::registry::{DevicePlanningKnowledge, shared_knowledge};
use super::swap_equivalence::prepare_equivalent_local_roots;
use super::{
    CalibrationEstimator, DeviceGateState, DevicePhysicalCost, DevicePlanner, DevicePlannerError,
    NativePlanLeaf, NativePlanSummary, PlanChoice, PlanId,
};
use crate::compile::CompilerError;
use crate::compile::error::DeviceLoweringFailure;
use crate::compile::knowledge::RuleLibrary;
use crate::device::Device;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::mem::size_of;
use std::sync::{Arc, Condvar, Mutex};

const SESSION_CACHE_MAX_ENTRIES: usize = 16;
const SESSION_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Result of planning one requested exact-device root.
#[derive(Debug, Clone)]
pub(crate) enum NativePlanAvailability {
    /// The root has a selected exact-qargs native lowering plan.
    Feasible(Arc<NativePlanSummary>),
    /// The root was requested, but no native lowering plan exists.
    Unsupported(Arc<DeviceLoweringFailure>),
}

/// One authoritative result for a state planned as an independent root.
///
/// Context-dependent Pareto nodes selected inside a parent recipe are owned by
/// that parent's tree, but are never inserted here unless the planner also
/// selects them independently through `selected_plan_for`.
#[derive(Debug)]
pub(super) enum CanonicalRootResult {
    Feasible(Arc<SelectedNativePlan>),
    Unsupported(Arc<DeviceLoweringFailure>),
}

impl CanonicalRootResult {
    fn availability(&self) -> NativePlanAvailability {
        match self {
            Self::Feasible(selected) => {
                NativePlanAvailability::Feasible(Arc::clone(&selected.summary))
            }
            Self::Unsupported(failure) => NativePlanAvailability::Unsupported(Arc::clone(failure)),
        }
    }

    fn selected_plan(&self) -> Option<Arc<SelectedNativePlan>> {
        match self {
            Self::Feasible(selected) => Some(Arc::clone(selected)),
            Self::Unsupported(_) => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct DevicePlanningSessionCache {
    roots: HashMap<DeviceGateState, Arc<CanonicalRootResult>>,
}

impl DevicePlanningSessionCache {
    pub(super) fn contains_key(&self, state: &DeviceGateState) -> bool {
        self.roots.contains_key(state)
    }

    pub(super) fn len(&self) -> usize {
        self.roots.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub(super) fn get(&self, state: &DeviceGateState) -> Option<Arc<CanonicalRootResult>> {
        self.roots.get(state).cloned()
    }

    pub(super) fn iter(
        &self,
    ) -> impl Iterator<Item = (&DeviceGateState, &Arc<CanonicalRootResult>)> {
        self.roots.iter()
    }

    pub(super) fn availability(&self, state: &DeviceGateState) -> Option<NativePlanAvailability> {
        self.roots.get(state).map(|result| result.availability())
    }

    pub(super) fn selected_plan(&self, state: &DeviceGateState) -> Option<Arc<SelectedNativePlan>> {
        self.roots
            .get(state)
            .and_then(|result| result.selected_plan())
    }

    pub(super) fn insert(
        &mut self,
        state: DeviceGateState,
        result: Arc<CanonicalRootResult>,
    ) -> Result<(), CompilerError> {
        if let Some(existing) = self.roots.get(&state) {
            if !canonical_results_equal(existing, &result) {
                return Err(CompilerError::InvariantViolation(format!(
                    "conflicting canonical device plans for {state:?}"
                )));
            }
            return Ok(());
        }
        self.roots.insert(state, result);
        Ok(())
    }

    pub(super) fn merge(&mut self, other: Self) -> Result<(), CompilerError> {
        let mut entries = other.roots.into_iter().collect::<Vec<_>>();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (state, result) in entries {
            self.insert(state, result)?;
        }
        Ok(())
    }

    pub(super) fn subset(&self, roots: &[DeviceGateState]) -> Result<Self, CompilerError> {
        let mut subset = Self::default();
        for root in roots {
            let result = self.roots.get(root).cloned().ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "device planning cache did not retain requested root {root:?}"
                ))
            })?;
            subset.insert(root.clone(), result)?;
        }
        Ok(subset)
    }

    pub(super) fn retain(&mut self, keep: impl Fn(&DeviceGateState) -> bool) {
        self.roots.retain(|state, _| keep(state));
    }

    fn ensure_compatible(&self, other: &Self) -> Result<(), CompilerError> {
        let (smaller, larger) = if self.roots.len() <= other.roots.len() {
            (&self.roots, &other.roots)
        } else {
            (&other.roots, &self.roots)
        };
        for (state, result) in smaller {
            if let Some(existing) = larger.get(state)
                && !canonical_results_equal(existing, result)
            {
                return Err(CompilerError::InvariantViolation(format!(
                    "conflicting canonical device plans for {state:?}"
                )));
            }
        }
        Ok(())
    }

    pub(super) fn estimated_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>().saturating_add(
            self.roots
                .len()
                .saturating_mul(size_of::<(DeviceGateState, Arc<CanonicalRootResult>)>() + 128),
        );
        let mut visited = HashSet::new();
        for result in self.roots.values() {
            if let CanonicalRootResult::Feasible(selected) = result.as_ref() {
                bytes = bytes.saturating_add(estimate_selected_plan_bytes(selected, &mut visited));
            }
        }
        bytes
    }
}

/// Owned selected recipe tree reusable after a planner batch is dropped.
#[derive(Debug)]
pub(crate) struct SelectedNativePlan {
    pub(crate) state: DeviceGateState,
    pub(crate) choice: PlanChoice,
    pub(crate) children: Arc<[Arc<SelectedNativePlan>]>,
    pub(crate) physical_cost: DevicePhysicalCost,
    pub(crate) summary: Arc<NativePlanSummary>,
}

/// Immutable plans pinned for one planning request.
///
/// The session may evict this batch from its LRU while a compile is running;
/// the caller's `Arc` keeps every selected recipe reachable until that compile
/// has finished consuming it.
#[derive(Debug)]
pub(crate) struct DevicePlanSnapshot {
    estimator: Arc<CalibrationEstimator>,
    roots: Arc<DevicePlanningSessionCache>,
}

impl DevicePlanSnapshot {
    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<NativePlanAvailability> {
        self.roots.availability(state)
    }

    pub(crate) fn selected_plan(&self, state: &DeviceGateState) -> Option<Arc<SelectedNativePlan>> {
        self.roots.selected_plan(state)
    }

    pub(crate) fn leaves_physical_cost(&self, leaves: &[NativePlanLeaf]) -> DevicePhysicalCost {
        self.estimator.physical_cost(leaves)
    }
}

#[derive(Debug)]
struct CachedPlanningBatch {
    roots: Arc<DevicePlanningSessionCache>,
    estimated_bytes: usize,
    last_used: u64,
}

#[derive(Debug, Default)]
struct DevicePlanningSessionState {
    batches: HashMap<Vec<DeviceGateState>, CachedPlanningBatch>,
    flights: HashSet<Vec<DeviceGateState>>,
    estimated_bytes: usize,
    clock: u64,
}

impl DevicePlanningSessionState {
    fn cached(&mut self, roots: &[DeviceGateState]) -> Option<Arc<DevicePlanningSessionCache>> {
        let cached_roots = if self.batches.contains_key(roots) {
            roots.to_vec()
        } else {
            self.batches
                .iter()
                .filter(|(_, batch)| roots.iter().all(|root| batch.roots.contains_key(root)))
                .min_by(|(left_roots, left), (right_roots, right)| {
                    left.roots
                        .len()
                        .cmp(&right.roots.len())
                        .then_with(|| left_roots.cmp(right_roots))
                })
                .map(|(cached_roots, _)| cached_roots.clone())?
        };
        self.clock = self.clock.wrapping_add(1);
        let last_used = self.clock;
        let batch = self.batches.get_mut(&cached_roots)?;
        batch.last_used = last_used;
        Some(Arc::clone(&batch.roots))
    }

    fn reusable_roots(&mut self, roots: &[DeviceGateState]) -> DevicePlanningSessionCache {
        let mut batches = self
            .batches
            .iter()
            .filter(|(_, batch)| roots.iter().any(|root| batch.roots.contains_key(root)))
            .map(|(cached_roots, batch)| (cached_roots.clone(), batch.last_used))
            .collect::<Vec<_>>();
        batches.sort_by_key(|(_, last_used)| std::cmp::Reverse(*last_used));

        let mut reusable = DevicePlanningSessionCache::default();
        for (cached_roots, _) in batches {
            let Some(batch) = self.batches.get(&cached_roots) else {
                continue;
            };
            for root in roots {
                if reusable.contains_key(root) {
                    continue;
                }
                if let Some(result) = batch.roots.get(root) {
                    // Both entries are canonical roots, so a conflict would be
                    // an internal planner invariant violation. Existing
                    // entries win here; the checked merge after planning
                    // validates newly produced overlaps.
                    reusable.roots.insert(root.clone(), result);
                }
            }
        }
        reusable
    }

    fn overlaps_flight(&self, roots: &[DeviceGateState]) -> bool {
        self.flights
            .iter()
            .any(|flight| roots.iter().any(|root| flight.binary_search(root).is_ok()))
    }

    fn insert(
        &mut self,
        roots: Vec<DeviceGateState>,
        cache: Arc<DevicePlanningSessionCache>,
        estimated_bytes: usize,
    ) -> Result<(), CompilerError> {
        for batch in self.batches.values() {
            batch.roots.ensure_compatible(&cache)?;
        }
        let estimated_bytes = estimated_bytes
            .saturating_add(size_of::<Vec<DeviceGateState>>())
            .saturating_add(
                roots
                    .capacity()
                    .saturating_mul(size_of::<DeviceGateState>()),
            );
        if estimated_bytes > SESSION_CACHE_MAX_BYTES {
            return Ok(());
        }
        self.clock = self.clock.wrapping_add(1);
        if let Some(replaced) = self.batches.remove(&roots) {
            self.estimated_bytes = self
                .estimated_bytes
                .saturating_sub(replaced.estimated_bytes);
        }
        self.estimated_bytes = self.estimated_bytes.saturating_add(estimated_bytes);
        self.batches.insert(
            roots,
            CachedPlanningBatch {
                roots: cache,
                estimated_bytes,
                last_used: self.clock,
            },
        );
        while self.batches.len() > SESSION_CACHE_MAX_ENTRIES
            || self.estimated_bytes > SESSION_CACHE_MAX_BYTES
        {
            let Some(oldest) = self
                .batches
                .iter()
                .min_by_key(|(_, batch)| batch.last_used)
                .map(|(roots, _)| roots.clone())
            else {
                break;
            };
            if let Some(removed) = self.batches.remove(&oldest) {
                self.estimated_bytes = self.estimated_bytes.saturating_sub(removed.estimated_bytes);
            }
        }
        Ok(())
    }
}

/// Shared immutable device knowledge plus a workflow-local planning overlay.
///
/// Common topology SWAP recipes live in a bounded process registry. Roots
/// introduced by a particular circuit remain local, so arbitrary workloads
/// cannot grow the process catalog without bound.
#[derive(Debug)]
pub(crate) struct DevicePlanningSession {
    knowledge: Arc<DevicePlanningKnowledge>,
    empty_cache: Arc<DevicePlanningSessionCache>,
    state: Mutex<DevicePlanningSessionState>,
    ready: Condvar,
}

impl DevicePlanningSession {
    pub(crate) fn new(device: &Device) -> Self {
        let knowledge = shared_knowledge(device);
        Self {
            knowledge,
            empty_cache: Arc::new(DevicePlanningSessionCache::default()),
            state: Mutex::new(DevicePlanningSessionState::default()),
            ready: Condvar::new(),
        }
    }

    pub(crate) fn estimator(&self) -> Arc<CalibrationEstimator> {
        Arc::clone(&self.knowledge.estimator)
    }

    pub(crate) fn prepare(
        &self,
        roots: impl IntoIterator<Item = DeviceGateState>,
    ) -> Result<Arc<DevicePlanSnapshot>, CompilerError> {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        let (common, local): (Vec<_>, Vec<_>) = roots
            .into_iter()
            .partition(|root| self.knowledge.is_common_swap(root));
        let common = self.knowledge.prepare_common(common)?;
        let local = self.prepare_local(local)?;
        let roots = if common.is_empty() {
            local
        } else if local.is_empty() {
            common
        } else {
            let mut combined = common.as_ref().clone();
            combined.merge(local.as_ref().clone())?;
            Arc::new(combined)
        };
        Ok(Arc::new(DevicePlanSnapshot {
            estimator: Arc::clone(&self.knowledge.estimator),
            roots,
        }))
    }

    fn prepare_local(
        &self,
        roots: Vec<DeviceGateState>,
    ) -> Result<Arc<DevicePlanningSessionCache>, CompilerError> {
        if roots.is_empty() {
            return Ok(Arc::clone(&self.empty_cache));
        }
        let (mut local, missing) = loop {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(snapshot) = state.cached(&roots) {
                return Ok(snapshot);
            }
            let reusable = state.reusable_roots(&roots);
            let missing = roots
                .iter()
                .filter(|root| !reusable.contains_key(root))
                .cloned()
                .collect::<Vec<_>>();
            if missing.is_empty() {
                let estimated_bytes = reusable.estimated_bytes();
                let cache = Arc::new(reusable);
                state.insert(roots, Arc::clone(&cache), estimated_bytes)?;
                return Ok(cache);
            }
            if !state.overlaps_flight(&missing) {
                state.flights.insert(missing.clone());
                break (reusable, missing);
            }
            drop(
                self.ready
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            );
        };
        let result = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))
            .and_then(|library| {
                let mut staged = DevicePlanningSessionCache::default();
                match prepare_equivalent_local_roots(
                    &self.knowledge.device,
                    library,
                    &missing,
                    Arc::clone(&self.knowledge.estimator),
                    &mut staged,
                ) {
                    Ok(_) => {
                        local.merge(staged)?;
                        Ok(())
                    }
                    Err(_) => prepare_cache_baseline(
                        &self.knowledge.device,
                        library,
                        missing.iter().cloned(),
                        Arc::clone(&self.knowledge.estimator),
                        &mut local,
                    ),
                }
            });
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.flights.remove(&missing);
        let prepared = result.and_then(|()| {
            let estimated_bytes = local.estimated_bytes();
            let cache = Arc::new(local);
            state.insert(roots, Arc::clone(&cache), estimated_bytes)?;
            Ok(cache)
        });
        self.ready.notify_all();
        prepared
    }
}

pub(super) fn prepare_cache_baseline(
    device: &Device,
    library: &RuleLibrary,
    roots: impl IntoIterator<Item = DeviceGateState>,
    estimator: Arc<CalibrationEstimator>,
    cache: &mut DevicePlanningSessionCache,
) -> Result<(), CompilerError> {
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        return Ok(());
    }
    let planner =
        DevicePlanner::build_with_estimator(device, library, roots.iter().cloned(), estimator)
            .map_err(DevicePlannerError::into_compiler_error)?;
    let mut plan_memo = HashMap::new();
    populate_canonical_cache(&planner, roots, cache, &mut plan_memo)
}

/// Exports independent canonical roots from an already solved planner.
///
/// Every state reached through a selected tree is put back through
/// `selected_plan_for` before entering the root cache. This preserves reuse
/// without confusing a parent's context-dependent child with the child's
/// standalone canonical choice.
pub(super) fn populate_canonical_cache(
    planner: &DevicePlanner<'_>,
    roots: impl IntoIterator<Item = DeviceGateState>,
    cache: &mut DevicePlanningSessionCache,
    plan_memo: &mut HashMap<PlanId, Arc<SelectedNativePlan>>,
) -> Result<(), CompilerError> {
    let requested = roots.into_iter().collect::<BTreeSet<_>>();
    let mut pending = requested.clone();
    let mut processed = BTreeSet::new();
    while let Some(state) = pending.pop_first() {
        if !processed.insert(state.clone()) {
            continue;
        }
        let result = if let Some(plan) = planner.selected_plan_for(&state) {
            let selected = own_selected_plan(planner, plan, plan_memo)?;
            collect_selected_states(&selected, &mut pending, &processed);
            Arc::new(CanonicalRootResult::Feasible(selected))
        } else if requested.contains(&state) {
            Arc::new(CanonicalRootResult::Unsupported(Arc::new(
                planner.failure_for(&state),
            )))
        } else {
            return Err(CompilerError::InvariantViolation(format!(
                "selected device plan depends on state without a canonical plan {state:?}"
            )));
        };
        cache.insert(state, result)?;
    }
    Ok(())
}

fn collect_selected_states(
    selected: &Arc<SelectedNativePlan>,
    pending: &mut BTreeSet<DeviceGateState>,
    processed: &BTreeSet<DeviceGateState>,
) {
    let mut stack = vec![Arc::clone(selected)];
    let mut visited = HashSet::new();
    while let Some(selected) = stack.pop() {
        let pointer = Arc::as_ptr(&selected) as usize;
        if !visited.insert(pointer) {
            continue;
        }
        if !processed.contains(&selected.state) {
            pending.insert(selected.state.clone());
        }
        stack.extend(selected.children.iter().cloned());
    }
}

pub(super) fn own_selected_plan(
    planner: &DevicePlanner<'_>,
    plan: PlanId,
    memo: &mut HashMap<PlanId, Arc<SelectedNativePlan>>,
) -> Result<Arc<SelectedNativePlan>, CompilerError> {
    if let Some(selected) = memo.get(&plan) {
        return Ok(Arc::clone(selected));
    }
    let state = planner.state_for_plan(plan).cloned().ok_or_else(|| {
        CompilerError::InvariantViolation(format!("unknown selected device plan {plan:?}"))
    })?;
    let choice = planner.choice_for_plan(plan).ok_or_else(|| {
        CompilerError::InvariantViolation(format!("device plan {plan:?} has no choice"))
    })?;
    let children = planner.children_for_plan(plan).ok_or_else(|| {
        CompilerError::InvariantViolation(format!("device plan {plan:?} has no child list"))
    })?;
    let children = children
        .iter()
        .copied()
        .map(|child| own_selected_plan(planner, child, memo))
        .collect::<Result<Vec<_>, _>>()?;
    let physical_cost = planner.cost_for_plan(plan).ok_or_else(|| {
        CompilerError::InvariantViolation(format!("device plan {plan:?} has no physical cost"))
    })?;
    let summary = Arc::new(
        planner
            .summary_for_plan(plan)
            .map_err(DevicePlannerError::into_compiler_error)?,
    );
    let selected = Arc::new(SelectedNativePlan {
        state,
        choice,
        children: children.into(),
        physical_cost,
        summary,
    });
    memo.insert(plan, Arc::clone(&selected));
    Ok(selected)
}

fn estimate_selected_plan_bytes(
    selected: &Arc<SelectedNativePlan>,
    visited: &mut HashSet<usize>,
) -> usize {
    let pointer = Arc::as_ptr(selected) as usize;
    if !visited.insert(pointer) {
        return 0;
    }
    let mut bytes = size_of::<SelectedNativePlan>()
        .saturating_add(
            selected
                .children
                .len()
                .saturating_mul(size_of::<Arc<SelectedNativePlan>>()),
        )
        .saturating_add(
            selected
                .summary
                .leaves
                .capacity()
                .saturating_mul(size_of::<NativePlanLeaf>()),
        );
    for child in selected.children.iter() {
        bytes = bytes.saturating_add(estimate_selected_plan_bytes(child, visited));
    }
    bytes
}

fn canonical_results_equal(left: &CanonicalRootResult, right: &CanonicalRootResult) -> bool {
    match (left, right) {
        (CanonicalRootResult::Feasible(left), CanonicalRootResult::Feasible(right)) => {
            let mut visited = HashSet::new();
            selected_plans_equal(left, right, &mut visited)
        }
        (CanonicalRootResult::Unsupported(left), CanonicalRootResult::Unsupported(right)) => {
            lowering_failures_equal(left, right)
        }
        _ => false,
    }
}

fn selected_plans_equal(
    left: &Arc<SelectedNativePlan>,
    right: &Arc<SelectedNativePlan>,
    visited: &mut HashSet<(usize, usize)>,
) -> bool {
    if Arc::ptr_eq(left, right) {
        return true;
    }
    let pair = (Arc::as_ptr(left) as usize, Arc::as_ptr(right) as usize);
    if !visited.insert(pair) {
        return true;
    }
    left.state == right.state
        && left.choice == right.choice
        && left.physical_cost == right.physical_cost
        && summaries_equal(&left.summary, &right.summary)
        && left.children.len() == right.children.len()
        && left
            .children
            .iter()
            .zip(right.children.iter())
            .all(|(left, right)| selected_plans_equal(left, right, visited))
}

fn summaries_equal(left: &NativePlanSummary, right: &NativePlanSummary) -> bool {
    left.native_two_qubit_ops == right.native_two_qubit_ops
        && left.native_total_ops == right.native_total_ops
        && left.leaves.len() == right.leaves.len()
        && left
            .leaves
            .iter()
            .zip(right.leaves.iter())
            .all(|(left, right)| {
                left.instruction == right.instruction
                    && left.ordered_qargs == right.ordered_qargs
                    && left.error_rate.map(f64::to_bits) == right.error_rate.map(f64::to_bits)
                    && left.duration.map(f64::to_bits) == right.duration.map(f64::to_bits)
            })
}

fn lowering_failures_equal(left: &DeviceLoweringFailure, right: &DeviceLoweringFailure) -> bool {
    left.instruction == right.instruction
        && left.qargs == right.qargs
        && left.attempted_candidates.len() == right.attempted_candidates.len()
        && left
            .attempted_candidates
            .iter()
            .zip(right.attempted_candidates.iter())
            .all(|(left, right)| {
                left.template == right.template
                    && left.unsatisfied_dependencies.len() == right.unsatisfied_dependencies.len()
                    && left
                        .unsatisfied_dependencies
                        .iter()
                        .zip(right.unsatisfied_dependencies.iter())
                        .all(|(left, right)| {
                            left.instruction == right.instruction && left.qargs == right.qargs
                        })
            })
}
