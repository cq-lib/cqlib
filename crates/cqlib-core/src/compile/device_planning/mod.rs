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

//! Shared exact-qargs device planning used by routing and final lowering.

pub(crate) mod cost;
mod planner;
mod templates;

use crate::circuit::{Instruction, StandardGate};
use crate::compile::CompilerError;
use crate::compile::error::DeviceLoweringFailure;
use crate::compile::knowledge::{KnowledgeInstructionKey, RuleLibrary};
use crate::device::{Device, PhysicalQubit};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) use cost::{
    CalibrationEstimator, DevicePhysicalCost, NativePlanCost, NativePlanLeaf, NativePlanSummary,
};
pub(crate) use planner::{DevicePlanner, DevicePlannerError, PlanChoice, PlanId, PlanTemplate};
pub(crate) use templates::DirectionTemplate;

/// A parameter-independent gate state on exact ordered physical qargs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DeviceGateState {
    pub(crate) instruction: KnowledgeInstructionKey,
    pub(crate) ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
}

#[cfg(test)]
#[path = "device_planning_test.rs"]
mod device_planning_test;

impl DeviceGateState {
    pub(crate) fn standard(
        gate: StandardGate,
        ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    ) -> Self {
        Self {
            instruction: KnowledgeInstructionKey::Standard(gate),
            ordered_qargs,
        }
    }

    pub(crate) fn from_instruction(
        instruction: &Instruction,
        ordered_qargs: SmallVec<[PhysicalQubit; 2]>,
    ) -> Option<Self> {
        Some(Self {
            instruction: KnowledgeInstructionKey::from_instruction(instruction)?,
            ordered_qargs,
        })
    }
}

/// Result of planning one requested exact-device root.
#[derive(Debug, Clone)]
pub(crate) enum NativePlanAvailability {
    /// The root has a selected exact-qargs native lowering plan.
    Feasible(NativePlanSummary),
    /// The root was requested, but no native lowering plan exists.
    Unsupported(DeviceLoweringFailure),
}

#[derive(Debug, Default)]
struct DevicePlanningSessionCache {
    availability: HashMap<DeviceGateState, NativePlanAvailability>,
    selected_plans: HashMap<DeviceGateState, Arc<SelectedNativePlan>>,
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

/// Immutable device snapshot plus incremental exact-qargs planning results.
///
/// A workflow owns one session. Passes ask it to prepare only the states they
/// actually need; repeated states reuse both the selected plan summary and the
/// device-wide calibration estimator. The mutex intentionally serializes a
/// missing-root solve so concurrent callers cannot duplicate the same batch.
#[derive(Debug)]
pub(crate) struct DevicePlanningSession {
    device: Arc<Device>,
    estimator: Arc<CalibrationEstimator>,
    cache: Mutex<DevicePlanningSessionCache>,
}

impl DevicePlanningSession {
    pub(crate) fn new(device: &Device) -> Self {
        let device = Arc::new(device.clone());
        let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
        let estimator = Arc::new(CalibrationEstimator::from_device(&device, &physical_qubits));
        Self {
            device,
            estimator,
            cache: Mutex::new(DevicePlanningSessionCache::default()),
        }
    }

    pub(crate) fn estimator(&self) -> Arc<CalibrationEstimator> {
        Arc::clone(&self.estimator)
    }

    pub(crate) fn prepare(
        &self,
        roots: impl IntoIterator<Item = DeviceGateState>,
    ) -> Result<(), CompilerError> {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort();
        roots.dedup();

        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        roots.retain(|root| !cache.availability.contains_key(root));
        if roots.is_empty() {
            return Ok(());
        }

        let library = RuleLibrary::builtin_rules()
            .map_err(|error| CompilerError::InvariantViolation(error.to_string()))?;
        let planner = DevicePlanner::build_with_estimator(
            &self.device,
            library,
            roots.iter().cloned(),
            Arc::clone(&self.estimator),
        )
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
        // Retain selected dependency recipes too. A later pass may request a
        // state that was previously only a child of another root; it should
        // not rebuild the same sub-plan merely because its role changed.
        for selected in plan_memo.into_values() {
            cache
                .availability
                .entry(selected.state.clone())
                .or_insert_with(|| NativePlanAvailability::Feasible(selected.summary.clone()));
            cache
                .selected_plans
                .entry(selected.state.clone())
                .or_insert(selected);
        }
        Ok(())
    }

    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<NativePlanAvailability> {
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .availability
            .get(state)
            .cloned()
    }

    pub(crate) fn selected_plan(&self, state: &DeviceGateState) -> Option<Arc<SelectedNativePlan>> {
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .selected_plans
            .get(state)
            .cloned()
    }

    pub(crate) fn leaves_physical_cost(&self, leaves: &[NativePlanLeaf]) -> DevicePhysicalCost {
        self.estimator.physical_cost(leaves)
    }
}

fn own_selected_plan(
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

/// Immutable exact-device planning results used by routing cost preparation.
///
/// Every requested root is retained. A missing map entry therefore means the
/// caller failed to prepare that state, rather than that planning proved it
/// unsupported.
#[derive(Debug, Clone)]
pub(crate) struct NativePlanCatalog {
    availability: HashMap<DeviceGateState, NativePlanAvailability>,
}

impl NativePlanCatalog {
    pub(crate) fn build_with_session(
        session: &DevicePlanningSession,
        roots: impl IntoIterator<Item = DeviceGateState>,
    ) -> Result<Self, CompilerError> {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        session.prepare(roots.iter().cloned())?;
        let mut availability = HashMap::with_capacity(roots.len());
        for root in roots {
            let planned = session.availability(&root).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "device planning session did not retain requested root {root:?}"
                ))
            })?;
            availability.insert(root, planned);
        }
        Ok(Self { availability })
    }

    pub(crate) fn availability(&self, state: &DeviceGateState) -> Option<&NativePlanAvailability> {
        self.availability.get(state)
    }

    pub(crate) fn summary(&self, state: &DeviceGateState) -> Option<&NativePlanSummary> {
        match self.availability(state) {
            Some(NativePlanAvailability::Feasible(summary)) => Some(summary),
            Some(NativePlanAvailability::Unsupported(_)) | None => None,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&DeviceGateState, &NativePlanSummary)> {
        self.availability
            .iter()
            .filter_map(|(state, availability)| {
                let NativePlanAvailability::Feasible(summary) = availability else {
                    return None;
                };
                Some((state, summary))
            })
    }

    pub(crate) fn iter_availability(
        &self,
    ) -> impl Iterator<Item = (&DeviceGateState, &NativePlanAvailability)> {
        self.availability.iter()
    }
}
