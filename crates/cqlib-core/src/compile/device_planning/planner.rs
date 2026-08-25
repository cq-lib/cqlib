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

//! Finite exact-qargs hypergraph planning for device instruction lowering.

use super::DeviceGateState;
use super::cost::{
    CalibrationEstimator, DevicePhysicalCost, DeviceScheduleProfile, NativePlanLeaf,
    NativePlanSummary,
};
use super::templates::{self, DirectionTemplate};
use crate::circuit::{Instruction, ParameterValue, StandardGate};
use crate::compile::error::{
    DeviceLoweringCandidateFailure, DeviceLoweringDependency, DeviceLoweringFailure,
};
use crate::compile::knowledge::{KnowledgeInstructionKey, RuleId, RuleKind, RuleLibrary};
use crate::device::Device;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

const MAX_DIAGNOSTIC_CANDIDATES: usize = 16;
const MAX_FRONTIER_SIZE_PER_STATE: usize = 64;
const MAX_TOTAL_PLAN_NODES: usize = 100_000;
const MAX_TOTAL_GENERATED_CANDIDATES: usize = 1_000_000;

type StateId = usize;
#[derive(Debug, Clone, Copy)]
struct PlannerBudget {
    max_frontier_size_per_state: usize,
    max_total_plan_nodes: usize,
    max_total_generated_candidates: usize,
}

impl Default for PlannerBudget {
    fn default() -> Self {
        Self {
            max_frontier_size_per_state: MAX_FRONTIER_SIZE_PER_STATE,
            max_total_plan_nodes: MAX_TOTAL_PLAN_NODES,
            max_total_generated_candidates: MAX_TOTAL_GENERATED_CANDIDATES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DevicePlannerError {
    Invariant(String),
    ComplexityExceeded {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
}

impl fmt::Display for DevicePlannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invariant(message) => f.write_str(message),
            Self::ComplexityExceeded {
                resource,
                limit,
                observed,
            } => write!(
                f,
                "device planning exceeded {resource} budget {limit} at {observed}"
            ),
        }
    }
}

impl std::error::Error for DevicePlannerError {}

impl DevicePlannerError {
    pub(crate) fn into_compiler_error(self) -> crate::compile::CompilerError {
        match self {
            Self::Invariant(message) => crate::compile::CompilerError::InvariantViolation(message),
            error @ Self::ComplexityExceeded { .. } => {
                crate::compile::CompilerError::TransformFailed {
                    name: "device_planning",
                    reason: error.to_string(),
                }
            }
        }
    }
}

/// Stable identifier for one exact plan tree in the planner arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlanId(usize);

/// The physical cost minimized by device lowering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DevicePlanCost {
    pub(crate) two_qubit_ops: u32,
    pub(crate) total_ops: u32,
    // A well-founded tertiary metric makes equal-physical-cost one-child
    // equivalences settle after their dependencies and prevents cyclic plans.
    derivation_steps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanTemplate {
    Rule(RuleId),
    Direction(DirectionTemplate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanChoice {
    Native,
    Template(PlanTemplate),
}

#[derive(Debug, Clone)]
struct HyperEdge {
    parent: StateId,
    children: Vec<StateId>,
    template: PlanTemplate,
    stable_name: String,
}

#[derive(Debug, Clone)]
struct DevicePlanCandidate {
    state: StateId,
    choice: PlanChoice,
    children: Vec<PlanId>,
    leaves: Vec<NativePlanLeaf>,
    physical_cost: DevicePhysicalCost,
    schedule_profile: DeviceScheduleProfile,
    derivation_steps: u32,
    stable_key: String,
    ancestry: HashSet<StateId>,
}

impl DevicePlanCandidate {
    fn dominates(&self, other: &Self) -> bool {
        let Some(schedule_strict) = self.schedule_profile.dominance(&other.schedule_profile) else {
            return false;
        };
        let left = self.physical_cost;
        let right = other.physical_cost;
        let comparisons = [
            left.native_two_qubit_ops.cmp(&right.native_two_qubit_ops),
            left.error
                .compare_by(right.error, |left, right| left.compare(right)),
            left.native_total_ops.cmp(&right.native_total_ops),
            left.duration
                .compare_by(right.duration, |left, right| left.compare(right)),
        ];
        comparisons.iter().all(|ordering| !ordering.is_gt())
            && (schedule_strict || comparisons.iter().any(|ordering| ordering.is_lt()))
    }
}

/// One solved plan table for the finite closure reachable from circuit roots.
pub(crate) struct DevicePlanner<'a> {
    device: &'a Device,
    states: Vec<DeviceGateState>,
    state_ids: HashMap<DeviceGateState, StateId>,
    edges: Vec<HyperEdge>,
    nodes: Vec<DevicePlanCandidate>,
    frontiers: Vec<Vec<PlanId>>,
    selected: Vec<Option<PlanId>>,
    plans: Vec<Option<PlanChoice>>,
    costs: Vec<Option<DevicePlanCost>>,
    estimator: Arc<CalibrationEstimator>,
    budget: PlannerBudget,
    generated_candidates: usize,
}

impl<'a> DevicePlanner<'a> {
    /// Builds a planner against a compile-scoped immutable calibration model.
    ///
    /// Planning batches remain independent, but missing-calibration estimates
    /// are derived once per device snapshot rather than rescanning the entire
    /// device for every pass.
    pub(crate) fn build_with_estimator(
        device: &'a Device,
        library: &RuleLibrary,
        roots: impl IntoIterator<Item = DeviceGateState>,
        estimator: Arc<CalibrationEstimator>,
    ) -> Result<Self, DevicePlannerError> {
        Self::build_with_estimator_and_budget(
            device,
            library,
            roots,
            PlannerBudget::default(),
            estimator,
        )
    }

    fn build_with_estimator_and_budget(
        device: &'a Device,
        library: &RuleLibrary,
        roots: impl IntoIterator<Item = DeviceGateState>,
        budget: PlannerBudget,
        estimator: Arc<CalibrationEstimator>,
    ) -> Result<Self, DevicePlannerError> {
        let mut builder = GraphBuilder::new(library);
        for root in roots {
            builder.intern(root);
        }
        builder.expand().map_err(DevicePlannerError::Invariant)?;

        let state_count = builder.states.len();
        let mut planner = Self {
            device,
            plans: vec![None; state_count],
            costs: vec![None; state_count],
            nodes: Vec::new(),
            frontiers: vec![Vec::new(); state_count],
            selected: vec![None; state_count],
            states: builder.states,
            state_ids: builder.state_ids,
            edges: builder.edges,
            estimator,
            budget,
            generated_candidates: 0,
        };
        planner.solve()?;
        Ok(planner)
    }

    pub(crate) fn selected_plan_for(&self, state: &DeviceGateState) -> Option<PlanId> {
        self.state_ids.get(state).and_then(|id| self.selected[*id])
    }

    pub(crate) fn choice_for_plan(&self, plan: PlanId) -> Option<PlanChoice> {
        self.nodes.get(plan.0).map(|node| node.choice)
    }

    /// Returns the exact physical cost the estimator assigned to one plan.
    pub(crate) fn cost_for_plan(&self, plan: PlanId) -> Option<DevicePhysicalCost> {
        self.nodes.get(plan.0).map(|node| node.physical_cost)
    }

    pub(crate) fn children_for_plan(&self, plan: PlanId) -> Option<&[PlanId]> {
        self.nodes.get(plan.0).map(|node| node.children.as_slice())
    }

    pub(crate) fn state_for_plan(&self, plan: PlanId) -> Option<&DeviceGateState> {
        self.nodes.get(plan.0).map(|node| &self.states[node.state])
    }

    pub(crate) fn summary_for_plan(
        &self,
        plan: PlanId,
    ) -> Result<NativePlanSummary, DevicePlannerError> {
        let node = self.nodes.get(plan.0).ok_or_else(|| {
            DevicePlannerError::Invariant(format!("unknown device plan id {plan:?}"))
        })?;
        let leaves = node.leaves.clone();
        let native_two_qubit_ops = leaves
            .iter()
            .filter(|leaf| leaf.ordered_qargs.len() == 2)
            .count() as u32;
        Ok(NativePlanSummary {
            native_two_qubit_ops,
            native_total_ops: leaves.len() as u32,
            leaves,
        })
    }

    pub(crate) fn failure_for(&self, state: &DeviceGateState) -> DeviceLoweringFailure {
        let attempted_candidates = self
            .state_ids
            .get(state)
            .into_iter()
            .flat_map(|state_id| {
                self.edges
                    .iter()
                    .filter(move |edge| edge.parent == *state_id)
            })
            .take(MAX_DIAGNOSTIC_CANDIDATES)
            .map(|edge| {
                let mut seen = HashSet::new();
                let unsatisfied_dependencies = edge
                    .children
                    .iter()
                    .copied()
                    .filter(|child| self.plans[*child].is_none())
                    .filter(|child| seen.insert(*child))
                    .map(|child| DeviceLoweringDependency {
                        instruction: instruction_from_key(&self.states[child].instruction),
                        qargs: self.states[child].ordered_qargs.to_vec(),
                    })
                    .collect();
                DeviceLoweringCandidateFailure {
                    template: edge.stable_name.clone(),
                    unsatisfied_dependencies,
                }
            })
            .collect();

        DeviceLoweringFailure {
            instruction: instruction_from_key(&state.instruction),
            qargs: state.ordered_qargs.to_vec(),
            attempted_candidates,
        }
    }

    fn solve(&mut self) -> Result<(), DevicePlannerError> {
        for state_id in 0..self.states.len() {
            let state = self.states[state_id].clone();
            let instruction = instruction_from_key(&state.instruction);
            if self
                .device
                .supports_native_instruction(&instruction, &state.ordered_qargs)
            {
                let calibration = self
                    .device
                    .native_instruction_calibration(&instruction, &state.ordered_qargs)
                    .ok_or_else(|| {
                        DevicePlannerError::Invariant(format!(
                            "supported native instruction {instruction} on {:?} has no calibration record",
                            state.ordered_qargs
                        ))
                    })?;
                validate_leaf_calibration(
                    &instruction,
                    calibration.error_rate,
                    calibration.duration,
                )
                .map_err(DevicePlannerError::Invariant)?;
                let leaf = NativePlanLeaf {
                    instruction,
                    ordered_qargs: state.ordered_qargs.clone(),
                    error_rate: calibration.error_rate,
                    duration: calibration.duration,
                };
                let leaves = vec![leaf];
                let mut ancestry = HashSet::new();
                ancestry.insert(state_id);
                self.insert_candidate(DevicePlanCandidate {
                    state: state_id,
                    choice: PlanChoice::Native,
                    children: Vec::new(),
                    physical_cost: self.estimator.physical_cost(&leaves),
                    schedule_profile: self
                        .estimator
                        .schedule_profile(&leaves, &state.ordered_qargs)
                        .map_err(DevicePlannerError::Invariant)?,
                    leaves,
                    derivation_steps: 0,
                    stable_key: "native".to_string(),
                    ancestry,
                })?;
            }
        }

        let mut explored = vec![HashSet::<Vec<PlanId>>::new(); self.edges.len()];
        loop {
            let mut changed = false;
            let edges = self.edges.clone();
            for (edge, explored_set) in edges.iter().zip(explored.iter_mut()) {
                let edge = edge.clone();
                let child_frontiers = edge
                    .children
                    .iter()
                    .map(|child| self.frontiers[*child].clone())
                    .collect::<Vec<_>>();
                if child_frontiers.iter().any(Vec::is_empty) {
                    continue;
                }
                for children in PlanCombinations::new(&child_frontiers) {
                    if !explored_set.insert(children.clone()) {
                        continue;
                    }
                    self.generated_candidates += 1;
                    if self.generated_candidates > self.budget.max_total_generated_candidates {
                        return Err(DevicePlannerError::ComplexityExceeded {
                            resource: "total generated candidates",
                            limit: self.budget.max_total_generated_candidates,
                            observed: self.generated_candidates,
                        });
                    }

                    let mut ancestry = HashSet::new();
                    let mut leaves = Vec::new();
                    let mut derivation_steps = 1_u32;
                    let mut child_keys = Vec::with_capacity(children.len());
                    let mut cyclic = false;
                    for child in &children {
                        let node = &self.nodes[child.0];
                        if node.ancestry.contains(&edge.parent) {
                            cyclic = true;
                            break;
                        }
                        ancestry.extend(node.ancestry.iter().copied());
                        leaves.extend(node.leaves.iter().cloned());
                        derivation_steps = derivation_steps.saturating_add(node.derivation_steps);
                        child_keys.push(node.stable_key.as_str());
                    }
                    if cyclic {
                        continue;
                    }
                    ancestry.insert(edge.parent);
                    let stable_key = format!("{}({})", edge.stable_name, child_keys.join(","));
                    let physical_cost = self.estimator.physical_cost(&leaves);
                    let schedule_profile = self
                        .estimator
                        .schedule_profile(&leaves, &self.states[edge.parent].ordered_qargs)
                        .map_err(DevicePlannerError::Invariant)?;
                    changed |= self.insert_candidate(DevicePlanCandidate {
                        state: edge.parent,
                        choice: PlanChoice::Template(edge.template),
                        children,
                        leaves,
                        physical_cost,
                        schedule_profile,
                        derivation_steps,
                        stable_key,
                        ancestry,
                    })?;
                }
            }
            if !changed {
                break;
            }
        }

        for state in 0..self.states.len() {
            let selected = self.frontiers[state].iter().copied().min_by(|left, right| {
                let left = &self.nodes[left.0];
                let right = &self.nodes[right.0];
                left.physical_cost
                    .compare(right.physical_cost)
                    .then_with(|| left.derivation_steps.cmp(&right.derivation_steps))
                    .then_with(|| left.stable_key.cmp(&right.stable_key))
            });
            self.selected[state] = selected;
            if let Some(plan) = selected {
                let node = &self.nodes[plan.0];
                self.plans[state] = Some(node.choice);
                self.costs[state] = Some(DevicePlanCost {
                    two_qubit_ops: node.physical_cost.native_two_qubit_ops,
                    total_ops: node.physical_cost.native_total_ops,
                    derivation_steps: node.derivation_steps,
                });
            }
        }
        Ok(())
    }

    fn insert_candidate(
        &mut self,
        candidate: DevicePlanCandidate,
    ) -> Result<bool, DevicePlannerError> {
        let frontier = &self.frontiers[candidate.state];
        for existing in frontier.iter().copied() {
            let existing = &self.nodes[existing.0];
            if existing.dominates(&candidate)
                || (existing.physical_cost == candidate.physical_cost
                    && existing.schedule_profile == candidate.schedule_profile
                    && (existing.derivation_steps, &existing.stable_key)
                        <= (candidate.derivation_steps, &candidate.stable_key))
            {
                return Ok(false);
            }
        }

        let retained = frontier
            .iter()
            .filter(|existing| {
                let existing = &self.nodes[existing.0];
                !(candidate.dominates(existing)
                    || (candidate.physical_cost == existing.physical_cost
                        && candidate.schedule_profile == existing.schedule_profile
                        && (candidate.derivation_steps, &candidate.stable_key)
                            < (existing.derivation_steps, &existing.stable_key)))
            })
            .copied()
            .collect::<Vec<_>>();
        if retained.len() + 1 > self.budget.max_frontier_size_per_state {
            return Err(DevicePlannerError::ComplexityExceeded {
                resource: "frontier size per state",
                limit: self.budget.max_frontier_size_per_state,
                observed: retained.len() + 1,
            });
        }
        if self.nodes.len() + 1 > self.budget.max_total_plan_nodes {
            return Err(DevicePlannerError::ComplexityExceeded {
                resource: "total plan nodes",
                limit: self.budget.max_total_plan_nodes,
                observed: self.nodes.len() + 1,
            });
        }

        let state = candidate.state;
        let id = PlanId(self.nodes.len());
        self.nodes.push(candidate);
        self.frontiers[state] = retained;
        self.frontiers[state].push(id);
        self.frontiers[state].sort_by(|left, right| {
            self.nodes[left.0]
                .stable_key
                .cmp(&self.nodes[right.0].stable_key)
        });
        Ok(true)
    }
}

struct PlanCombinations<'a> {
    frontiers: &'a [Vec<PlanId>],
    indices: Vec<usize>,
    done: bool,
}

impl<'a> PlanCombinations<'a> {
    fn new(frontiers: &'a [Vec<PlanId>]) -> Self {
        Self {
            frontiers,
            indices: vec![0; frontiers.len()],
            done: frontiers.iter().any(Vec::is_empty),
        }
    }
}

impl Iterator for PlanCombinations<'_> {
    type Item = Vec<PlanId>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let combination = self
            .frontiers
            .iter()
            .zip(&self.indices)
            .map(|(frontier, index)| frontier[*index])
            .collect();

        if self.frontiers.is_empty() {
            self.done = true;
            return Some(combination);
        }
        for position in (0..self.indices.len()).rev() {
            self.indices[position] += 1;
            if self.indices[position] < self.frontiers[position].len() {
                return Some(combination);
            }
            self.indices[position] = 0;
        }
        self.done = true;
        Some(combination)
    }
}

fn validate_leaf_calibration(
    instruction: &Instruction,
    error_rate: Option<f64>,
    duration: Option<f64>,
) -> Result<(), String> {
    if let Some(error_rate) = error_rate
        && !(error_rate.is_finite() && (0.0..=1.0).contains(&error_rate))
    {
        return Err(format!(
            "native instruction {instruction} has invalid error rate {error_rate:?}"
        ));
    }
    if let Some(duration) = duration
        && !(duration.is_finite() && duration >= 0.0)
    {
        return Err(format!(
            "native instruction {instruction} has invalid duration {duration:?}"
        ));
    }
    Ok(())
}

struct GraphBuilder<'a> {
    library: &'a RuleLibrary,
    states: Vec<DeviceGateState>,
    state_ids: HashMap<DeviceGateState, StateId>,
    edges: Vec<HyperEdge>,
}

impl<'a> GraphBuilder<'a> {
    fn new(library: &'a RuleLibrary) -> Self {
        Self {
            library,
            states: Vec::new(),
            state_ids: HashMap::new(),
            edges: Vec::new(),
        }
    }

    fn intern(&mut self, state: DeviceGateState) -> StateId {
        if let Some(id) = self.state_ids.get(&state) {
            return *id;
        }
        let id = self.states.len();
        self.states.push(state.clone());
        self.state_ids.insert(state, id);
        id
    }

    fn expand(&mut self) -> Result<(), String> {
        let mut cursor = 0;
        while cursor < self.states.len() {
            let parent_state = self.states[cursor].clone();
            let mut candidates = self.rule_candidates(&parent_state)?;
            candidates.extend(
                templates::candidates(&parent_state)
                    .into_iter()
                    .map(|template| {
                        (
                            template.stable_name(),
                            PlanTemplate::Direction(template),
                            template.child_states(&parent_state),
                        )
                    }),
            );
            candidates.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

            for (stable_name, template, child_states) in candidates {
                let children = child_states
                    .into_iter()
                    .map(|child| self.intern(child))
                    .collect();
                self.edges.push(HyperEdge {
                    parent: cursor,
                    children,
                    template,
                    stable_name,
                });
            }
            cursor += 1;
        }
        Ok(())
    }

    fn rule_candidates(
        &self,
        parent: &DeviceGateState,
    ) -> Result<Vec<(String, PlanTemplate, Vec<DeviceGateState>)>, String> {
        let instruction = instruction_from_key(&parent.instruction);
        let rule_ids = self
            .library
            .candidates_for_first_instruction(&instruction)
            .map_err(|error| error.to_string())?;
        let mut candidates = Vec::new();

        for &rule_id in rule_ids {
            let Some(metadata) = self.library.metadata(rule_id) else {
                return Err(format!("missing device-lowering rule metadata {rule_id:?}"));
            };
            if !matches!(
                metadata.kind,
                RuleKind::Decompose | RuleKind::HardwareNative
            ) {
                continue;
            }
            let Some(rule) = self.library.get(rule_id) else {
                return Err(format!("missing device-lowering rule {rule_id:?}"));
            };
            if rule.operations.len() != 1
                || rule
                    .conditions
                    .as_ref()
                    .is_some_and(|conditions| !conditions.is_empty())
                || !source_params_are_generic_and_distinct(&rule.operations[0].params)
            {
                continue;
            }
            let source = &rule.operations[0];
            if source.qubits.len() != parent.ordered_qargs.len() {
                continue;
            }
            let qarg_bindings = source
                .qubits
                .iter()
                .copied()
                .zip(parent.ordered_qargs.iter().copied())
                .collect::<HashMap<_, _>>();
            let mut children = Vec::new();
            let mut valid = true;
            for item in &rule.target {
                let Some(key) = KnowledgeInstructionKey::from_instruction(&item.instruction) else {
                    valid = false;
                    break;
                };
                if matches!(key, KnowledgeInstructionKey::Standard(StandardGate::GPhase)) {
                    continue;
                }
                let ordered_qargs = item
                    .qubits
                    .iter()
                    .map(|qubit| qarg_bindings.get(qubit).copied())
                    .collect::<Option<SmallVec<[_; 2]>>>();
                let Some(ordered_qargs) = ordered_qargs else {
                    valid = false;
                    break;
                };
                children.push(DeviceGateState {
                    instruction: key,
                    ordered_qargs,
                });
            }
            if valid {
                candidates.push((
                    format!("rule:{}", rule.name),
                    PlanTemplate::Rule(rule_id),
                    children,
                ));
            }
        }
        Ok(candidates)
    }
}

/// Returns whether a decomposition source can be planned independently of
/// concrete parameter values and bound positionally during emission.
fn source_params_are_generic_and_distinct(params: &Option<SmallVec<[ParameterValue; 1]>>) -> bool {
    let mut symbols = HashSet::new();
    params.as_deref().unwrap_or(&[]).iter().all(|param| {
        let ParameterValue::Param(parameter) = param else {
            return false;
        };
        parameter
            .as_symbol()
            .is_some_and(|symbol| symbols.insert(symbol))
    })
}

pub(crate) fn instruction_from_key(key: &KnowledgeInstructionKey) -> Instruction {
    match key {
        KnowledgeInstructionKey::Standard(gate) => Instruction::Standard(*gate),
        KnowledgeInstructionKey::McGate(gate) => Instruction::McGate(Box::new(gate.clone())),
    }
}

#[cfg(test)]
#[path = "planner_test.rs"]
mod planner_test;
