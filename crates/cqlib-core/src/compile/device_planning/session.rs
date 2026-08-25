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
use std::sync::{Arc, Mutex};

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

/// Shared immutable device knowledge plus a workflow-local planning overlay.
///
/// Common topology SWAP recipes live in a bounded process registry. Roots
/// introduced by a particular circuit remain local, so arbitrary workloads
/// cannot grow the process catalog without bound.
#[derive(Debug)]
pub(crate) struct DevicePlanningSession {
    knowledge: Arc<DevicePlanningKnowledge>,
    local_cache: Mutex<DevicePlanningSessionCache>,
}

impl DevicePlanningSession {
    pub(crate) fn new(device: &Device) -> Self {
        Self {
            knowledge: shared_knowledge(device),
            local_cache: Mutex::new(DevicePlanningSessionCache::default()),
        }
    }

    pub(crate) fn estimator(&self) -> Arc<CalibrationEstimator> {
        Arc::clone(&self.knowledge.estimator)
    }

    pub(crate) fn prepare(
        &self,
        roots: impl IntoIterator<Item = DeviceGateState>,
    ) -> Result<(), CompilerError> {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        let (common, local): (Vec<_>, Vec<_>) = roots
            .into_iter()
            .partition(|root| self.knowledge.is_common_swap(root));
        self.knowledge.prepare_common(common)?;
        self.prepare_local(local)
    }

    fn prepare_local(&self, mut roots: Vec<DeviceGateState>) -> Result<(), CompilerError> {
        let mut cache = self
            .local_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        roots.retain(|root| !cache.availability.contains_key(root));
        if roots.is_empty() {
            return Ok(());
        }
        let library = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
        prepare_cache_baseline(
            &self.knowledge.device,
            library,
            roots,
            Arc::clone(&self.knowledge.estimator),
            &mut cache,
        )
    }

    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<NativePlanAvailability> {
        self.local_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .availability
            .get(state)
            .cloned()
            .or_else(|| self.knowledge.availability(state))
    }

    pub(crate) fn selected_plan(&self, state: &DeviceGateState) -> Option<Arc<SelectedNativePlan>> {
        self.local_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .selected_plans
            .get(state)
            .cloned()
            .or_else(|| self.knowledge.selected_plan(state))
    }

    pub(crate) fn leaves_physical_cost(&self, leaves: &[NativePlanLeaf]) -> DevicePhysicalCost {
        self.knowledge.estimator.physical_cost(leaves)
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
