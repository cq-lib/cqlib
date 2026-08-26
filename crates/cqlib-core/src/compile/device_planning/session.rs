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
use super::{
    CalibrationEstimator, DeviceGateState, DevicePhysicalCost, DevicePlanner, DevicePlannerError,
    NativePlanLeaf, NativePlanSummary, PlanChoice, PlanId,
};
use crate::compile::CompilerError;
use crate::compile::error::DeviceLoweringFailure;
use crate::compile::knowledge::RuleLibrary;
use crate::device::Device;
use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::{Arc, Condvar, Mutex};

const SESSION_CACHE_MAX_ENTRIES: usize = 16;
const SESSION_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Result of planning one requested exact-device root.
#[derive(Debug, Clone)]
pub(crate) enum NativePlanAvailability {
    /// The root has a selected exact-qargs native lowering plan.
    Feasible(NativePlanSummary),
    /// The root was requested, but no native lowering plan exists.
    Unsupported(DeviceLoweringFailure),
}

#[derive(Debug, Default)]
pub(super) struct DevicePlanningSessionCache {
    pub(super) availability: HashMap<DeviceGateState, NativePlanAvailability>,
    pub(super) selected_plans: HashMap<DeviceGateState, Arc<SelectedNativePlan>>,
}

impl DevicePlanningSessionCache {
    pub(super) fn merge(&mut self, other: Self) {
        self.availability.extend(other.availability);
        self.selected_plans.extend(other.selected_plans);
    }

    pub(super) fn estimated_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>()
            .saturating_add(
                self.availability
                    .len()
                    .saturating_mul(size_of::<(DeviceGateState, NativePlanAvailability)>() + 128),
            )
            .saturating_add(
                self.selected_plans
                    .len()
                    .saturating_mul(size_of::<(DeviceGateState, Arc<SelectedNativePlan>)>()),
            );
        let mut visited = HashSet::new();
        for selected in self.selected_plans.values() {
            bytes = bytes.saturating_add(estimate_selected_plan_bytes(selected, &mut visited));
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
    pub(crate) summary: NativePlanSummary,
}

/// Immutable plans pinned for one planning request.
///
/// The session may evict this batch from its LRU while a compile is running;
/// the caller's `Arc` keeps every selected recipe reachable until that compile
/// has finished consuming it.
#[derive(Debug)]
pub(crate) struct DevicePlanSnapshot {
    knowledge: Arc<DevicePlanningKnowledge>,
    local: Arc<DevicePlanningSessionCache>,
}

impl DevicePlanSnapshot {
    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<NativePlanAvailability> {
        self.local
            .availability
            .get(state)
            .cloned()
            .or_else(|| self.knowledge.availability(state))
    }

    pub(crate) fn selected_plan(&self, state: &DeviceGateState) -> Option<Arc<SelectedNativePlan>> {
        self.local
            .selected_plans
            .get(state)
            .cloned()
            .or_else(|| self.knowledge.selected_plan(state))
    }

    pub(crate) fn leaves_physical_cost(&self, leaves: &[NativePlanLeaf]) -> DevicePhysicalCost {
        self.knowledge.estimator.physical_cost(leaves)
    }
}

#[derive(Debug)]
struct CachedPlanningBatch {
    snapshot: Arc<DevicePlanSnapshot>,
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
    fn cached(&mut self, roots: &[DeviceGateState]) -> Option<Arc<DevicePlanSnapshot>> {
        let cached_roots = if self.batches.contains_key(roots) {
            roots.to_vec()
        } else {
            self.batches
                .iter()
                .filter(|(_, batch)| {
                    roots
                        .iter()
                        .all(|root| batch.snapshot.local.availability.contains_key(root))
                })
                .min_by(|(left_roots, left), (right_roots, right)| {
                    left.snapshot
                        .local
                        .availability
                        .len()
                        .cmp(&right.snapshot.local.availability.len())
                        .then_with(|| left_roots.cmp(right_roots))
                })
                .map(|(cached_roots, _)| cached_roots.clone())?
        };
        self.clock = self.clock.wrapping_add(1);
        let last_used = self.clock;
        let batch = self.batches.get_mut(&cached_roots)?;
        batch.last_used = last_used;
        Some(Arc::clone(&batch.snapshot))
    }

    fn insert(
        &mut self,
        roots: Vec<DeviceGateState>,
        snapshot: Arc<DevicePlanSnapshot>,
        estimated_bytes: usize,
    ) {
        let estimated_bytes = estimated_bytes
            .saturating_add(size_of::<Vec<DeviceGateState>>())
            .saturating_add(
                roots
                    .capacity()
                    .saturating_mul(size_of::<DeviceGateState>()),
            );
        if estimated_bytes > SESSION_CACHE_MAX_BYTES {
            return;
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
                snapshot,
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
    empty_snapshot: Arc<DevicePlanSnapshot>,
    state: Mutex<DevicePlanningSessionState>,
    ready: Condvar,
}

impl DevicePlanningSession {
    pub(crate) fn new(device: &Device) -> Self {
        let knowledge = shared_knowledge(device);
        let empty_snapshot = Arc::new(DevicePlanSnapshot {
            knowledge: Arc::clone(&knowledge),
            local: Arc::new(DevicePlanningSessionCache::default()),
        });
        Self {
            knowledge,
            empty_snapshot,
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
        self.knowledge.prepare_common(common)?;
        self.prepare_local(local)
    }

    fn prepare_local(
        &self,
        roots: Vec<DeviceGateState>,
    ) -> Result<Arc<DevicePlanSnapshot>, CompilerError> {
        if roots.is_empty() {
            return Ok(Arc::clone(&self.empty_snapshot));
        }
        loop {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(snapshot) = state.cached(&roots) {
                return Ok(snapshot);
            }
            if state.flights.insert(roots.clone()) {
                break;
            }
            drop(
                self.ready
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            );
        }
        let mut local = DevicePlanningSessionCache::default();
        let result = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))
            .and_then(|library| {
                prepare_cache_baseline(
                    &self.knowledge.device,
                    library,
                    roots.iter().cloned(),
                    Arc::clone(&self.knowledge.estimator),
                    &mut local,
                )
            });
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.flights.remove(&roots);
        let prepared = result.map(|()| {
            let estimated_bytes = local.estimated_bytes();
            let snapshot = Arc::new(DevicePlanSnapshot {
                knowledge: Arc::clone(&self.knowledge),
                local: Arc::new(local),
            });
            state.insert(roots, Arc::clone(&snapshot), estimated_bytes);
            snapshot
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
    let roots = roots.into_iter().collect::<Vec<_>>();
    if roots.is_empty() {
        return Ok(());
    }
    let planner =
        DevicePlanner::build_with_estimator(device, library, roots.iter().cloned(), estimator)
            .map_err(DevicePlannerError::into_compiler_error)?;
    let mut plan_memo = HashMap::new();
    for root in roots {
        let planned = if let Some(plan) = planner.selected_plan_for(&root) {
            let selected = own_selected_plan(&planner, plan, &mut plan_memo)?;
            let summary = selected.summary.clone();
            cache.selected_plans.insert(root.clone(), selected);
            NativePlanAvailability::Feasible(summary)
        } else {
            NativePlanAvailability::Unsupported(planner.failure_for(&root))
        };
        cache.availability.insert(root, planned);
    }
    retain_selected_dependencies(cache, plan_memo.into_values());
    Ok(())
}

pub(super) fn retain_selected_dependencies(
    cache: &mut DevicePlanningSessionCache,
    selected: impl IntoIterator<Item = Arc<SelectedNativePlan>>,
) {
    for selected in selected {
        cache
            .availability
            .entry(selected.state.clone())
            .or_insert_with(|| NativePlanAvailability::Feasible(selected.summary.clone()));
        cache
            .selected_plans
            .entry(selected.state.clone())
            .or_insert(selected);
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
    let summary = planner
        .summary_for_plan(plan)
        .map_err(DevicePlannerError::into_compiler_error)?;
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
