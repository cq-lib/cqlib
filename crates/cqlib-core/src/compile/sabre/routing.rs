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

use super::dag::{SabreControlFlow, SabreDag, SabreNodeKind};
use super::heuristic::{SabreConfig, SabreHeuristicConfig, SabreTrialObjective};
use super::layer::Layer;
use crate::circuit::value_instruction::storage_operation_to_value;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ControlBody, ForOp, IfOp, Instruction, Operation,
    Parameter, Qubit, StandardGate, SwitchCase, SwitchOp, WhileOp,
};
use crate::compile::CompilerError;
use crate::compile::physical_target::PhysicalLayoutGraph;
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit};
use indexmap::IndexSet;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use rustworkx_core::petgraph::Direction;
use rustworkx_core::petgraph::graph::{NodeIndex, UnGraph};
use rustworkx_core::petgraph::visit::EdgeRef;
use rustworkx_core::token_swapper::token_swapper;
use smallvec::{SmallVec, smallvec};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const CONTROL_FLOW_EPILOGUE_TRIALS: usize = 4;

/// Routed circuit and layout metadata produced by [`sabre_route`].
#[derive(Debug, Clone)]
pub struct SabreRoutingResult {
    /// Physical circuit with inserted SWAP operations.
    pub circuit: Circuit,
    /// Initial logical-to-physical layout used by the selected trial.
    pub initial_layout: Layout,
    /// Final logical-to-physical layout after all routed operations.
    pub final_layout: Layout,
    /// Number of inserted SWAP operations, including control-flow epilogues.
    pub swap_count: usize,
    /// Diagnostics describing routing search behavior.
    pub diagnostics: SabreRoutingDiagnostics,
}

/// Diagnostics emitted by SABRE routing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SabreRoutingDiagnostics {
    /// Number of routing trials evaluated.
    pub trials_evaluated: usize,
    /// Zero-based index of the selected routing trial.
    pub selected_trial_index: usize,
    /// Number of times the shortest-path fallback was used.
    pub fallback_count: usize,
    /// Number of recursively routed control-flow blocks.
    pub control_flow_blocks_routed: usize,
    /// ASAP two-qubit depth of the selected routed operation stream.
    pub two_qubit_depth: usize,
    /// Total number of operations in the selected routed operation stream.
    pub operation_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TrialResult {
    pub(crate) operations: Vec<Operation>,
    pub(crate) final_layout: Layout,
    pub(crate) swap_count: usize,
    pub(crate) fallback_count: usize,
    pub(crate) control_flow_blocks_routed: usize,
    pub(crate) quality: TrialQuality,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TrialQuality {
    pub(crate) swap_count: usize,
    pub(crate) two_qubit_depth: usize,
    pub(crate) operation_count: usize,
}

/// Routes `circuit` from `initial_layout` by inserting SWAP operations.
///
/// The returned circuit uses physical qubit identifiers as its circuit qubits.
/// Two-qubit operations in the routed circuit are adjacent in the usable
/// physical topology.  Control-flow bodies are routed recursively and restored
/// to their entry layout before leaving the block.
pub fn sabre_route(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<SabreRoutingResult, CompilerError> {
    validate_routing_config(config)?;
    // Build a dense, reusable view of the physical topology once. The routing
    // loop indexes into this structure heavily for adjacency, distance, and
    // deterministic candidate ordering.
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let target = RoutingTarget::from_physical(&physical)?;
    let logical_qubits = circuit
        .qubits()
        .into_iter()
        .map(LogicalQubit::from_qubit)
        .collect::<Vec<_>>();
    let initial_layout =
        normalize_initial_layout_for_target(&logical_qubits, &target, initial_layout)?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    validate_reachable_interactions_for_target(&sabre, &target, &initial_layout)?;

    // Trials share the normalized layout and DAG but use independent seeds for
    // tie-breaking. Selection stays deterministic for a configured seed because
    // result comparison falls back to the trial index.
    let trial_results = trial_seeds(config.seed, config.routing_trials)
        .into_par_iter()
        .enumerate()
        .map(|(index, seed)| {
            route_trial_unchecked(&sabre, &target, &initial_layout, &config.heuristic, seed)
                .map(|result| (index, result))
        })
        .collect::<Result<Vec<_>, CompilerError>>()?;
    let (best_index, best) = trial_results
        .into_iter()
        .min_by(|(left_index, left), (right_index, right)| {
            compare_trial_quality(
                config.trial_objective,
                left.quality,
                *left_index,
                right.quality,
                *right_index,
            )
        })
        .expect("routing_trials is validated to be non-zero");

    // Routing rewrites operation qubits but keeps symbolic parameters by index.
    // Rebuild the routed circuit's parameter table in first-use order, then
    // remap nested control-flow bodies to the new table.
    let mut parameter_order = IndexSet::<Parameter>::new();
    for operation in &best.operations {
        for param in &operation.params {
            if let CircuitParam::Index(index) = param {
                let parameter = circuit
                    .parameters()
                    .get_index(*index as usize)
                    .cloned()
                    .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index))?;
                parameter_order.insert(parameter);
            }
        }
    }
    for parameter in circuit.parameters() {
        parameter_order.insert(parameter.clone());
    }
    let parameter_indices = circuit
        .parameters()
        .iter()
        .map(|parameter| {
            parameter_order
                .get_index_of(parameter)
                .expect("source parameters are included in routed parameter order")
                as u32
        })
        .collect::<Vec<_>>();
    let mapped_operations = best
        .operations
        .iter()
        .map(|operation| remap_parameter_indices(operation, &parameter_indices))
        .collect::<Result<Vec<_>, _>>()?;
    let routed_operations = mapped_operations
        .iter()
        .map(|operation| {
            storage_operation_to_value(operation.clone(), &|param| match param {
                CircuitParam::Fixed(value) => Ok((*value).into()),
                CircuitParam::Index(index) => parameter_order
                    .get_index(*index as usize)
                    .cloned()
                    .map(Into::into)
                    .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index)),
            })
        })
        .collect::<Result<Vec<_>, crate::circuit::CircuitError>>()?;
    let mut routed = Circuit::from_operations(
        target
            .physical_qubits
            .iter()
            .copied()
            .map(PhysicalQubit::qubit)
            .collect(),
        routed_operations,
        Some(circuit.classical_vars().to_vec()),
        Some(circuit.classical_values().to_vec()),
    )?;
    for parameter in parameter_order {
        routed.add_parameter(parameter);
    }
    routed.set_global_phase(circuit.global_phase());

    Ok(SabreRoutingResult {
        circuit: routed,
        initial_layout,
        final_layout: best.final_layout,
        swap_count: best.swap_count,
        diagnostics: SabreRoutingDiagnostics {
            trials_evaluated: config.routing_trials,
            selected_trial_index: best_index,
            fallback_count: best.fallback_count,
            control_flow_blocks_routed: best.control_flow_blocks_routed,
            two_qubit_depth: best.quality.two_qubit_depth,
            operation_count: best.quality.operation_count,
        },
    })
}

/// Validates a SABRE routing configuration.
///
/// This check intentionally ignores layout-refinement fields such as
/// [`SabreConfig::layout_trials`] and [`SabreConfig::layout_scoring_trials`].
/// Routing starts from a concrete initial layout and does not depend on those
/// layout-only knobs.
pub fn validate_config(config: &SabreConfig) -> Result<(), CompilerError> {
    validate_routing_config(config)
}

fn validate_routing_config(config: &SabreConfig) -> Result<(), CompilerError> {
    if config.routing_trials == 0 {
        return Err(CompilerError::InvalidInput(
            "sabre routing_trials must be greater than zero".to_string(),
        ));
    }
    config.heuristic.validate()
}

pub(crate) fn route_trial(
    sabre: &SabreDag,
    target: &RoutingTarget,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<TrialResult, CompilerError> {
    validate_reachable_interactions_for_target(sabre, target, initial_layout)?;
    route_trial_unchecked(sabre, target, initial_layout, heuristic, seed)
}

pub(crate) fn route_trial_unchecked(
    sabre: &SabreDag,
    target: &RoutingTarget,
    initial_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<TrialResult, CompilerError> {
    let mut output = TrialOutput::new(seed);
    let mut state = RoutingState::new(sabre, target, initial_layout.clone(), heuristic, seed);

    // Initial operations are dependency-free one-qubit or non-quantum work.
    // They can be emitted immediately under the starting layout.
    for operation in &sabre.initial {
        output
            .operations
            .push(map_operation(operation, &state.layout)?);
    }

    state.update_route(
        sabre,
        target,
        heuristic,
        &mut output,
        &sabre.first_layer,
        None,
    )?;
    state.populate_extended_set(sabre, target)?;

    let mut routable_nodes = Vec::with_capacity(2);
    let mut search_steps_since_decay_reset = 0usize;
    while !state.front_layer.is_empty() {
        let mut current_swaps = Vec::new();
        // Search accumulates speculative SWAPs until at least one front-layer
        // node becomes adjacent. Those SWAPs are emitted only when the routed
        // node is actually emitted, preserving a compact operation stream.
        while routable_nodes.is_empty() && current_swaps.len() <= heuristic.attempt_limit {
            let best_swap = state.choose_best_swap(target, heuristic)?;
            state.apply_swap(best_swap.physical, target)?;
            current_swaps.push(best_swap.physical);
            let adjacent = |left, right| target.are_adjacent_by_index(left, right);
            for candidate in best_swap
                .indices
                .into_iter()
                .filter_map(|index| state.front_layer.routable_node_on_index(index, &adjacent))
            {
                if !routable_nodes.contains(&candidate) {
                    routable_nodes.push(candidate);
                }
            }

            if let Some(increment) = heuristic.decay_increment {
                search_steps_since_decay_reset += 1;
                if search_steps_since_decay_reset >= heuristic.decay_reset {
                    for value in &mut state.decay {
                        *value = 1.0;
                    }
                    search_steps_since_decay_reset = 0;
                } else {
                    state.decay[best_swap.indices[0]] += increment;
                    state.decay[best_swap.indices[1]] += increment;
                }
            }
        }

        if routable_nodes.is_empty() {
            // The heuristic failed to make progress within its attempt budget.
            // Roll back speculative swaps, then force progress along a shortest
            // path so the router cannot livelock on a poor local score.
            for swap in current_swaps.drain(..).rev() {
                state.apply_swap(swap, target)?;
            }
            output.fallback_count += 1;
            let forced = state.force_enable_closest_node(target, &mut current_swaps)?;
            routable_nodes.extend(forced);
        }

        let distance = |left, right| target.distance_by_index(left, right);
        for node in &routable_nodes {
            state.front_layer.remove(*node, &distance)?;
        }
        state.update_route(
            sabre,
            target,
            heuristic,
            &mut output,
            &routable_nodes,
            Some(current_swaps),
        )?;
        state.lookahead_layers.iter_mut().for_each(Layer::clear);
        state.populate_extended_set(sabre, target)?;
        if heuristic.decay_increment.is_some() {
            for value in &mut state.decay {
                *value = 1.0;
            }
        }
        routable_nodes.clear();
    }

    let quality = trial_quality(&output.operations, output.swap_count);
    Ok(TrialResult {
        operations: output.operations,
        final_layout: state.layout,
        swap_count: output.swap_count,
        fallback_count: output.fallback_count,
        control_flow_blocks_routed: output.control_flow_blocks_routed,
        quality,
    })
}

/// Derives per-trial seeds from an optional workflow seed.
///
/// A configured seed seeds the seed generator, not every trial directly. This
/// gives reproducible but distinct tie-breaking streams per trial.
pub(crate) fn trial_seeds(seed: Option<u64>, count: usize) -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(seed.unwrap_or_else(rand::random));
    (0..count).map(|_| rng.random()).collect()
}

/// Normalizes an initial layout against a device's usable physical topology.
///
/// The returned layout contains the supplied logical qubits and every usable
/// physical qubit from `device`. Logical qubits must already be mapped by
/// `initial_layout`; extra usable physical qubits remain vacant.
pub fn normalize_initial_layout(
    logical_qubits: &[LogicalQubit],
    device: &Device,
    initial_layout: &Layout,
) -> Result<Layout, CompilerError> {
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let target = RoutingTarget::from_physical(&physical)?;
    normalize_initial_layout_for_target(logical_qubits, &target, initial_layout)
}

pub(crate) fn normalize_initial_layout_for_target(
    logical_qubits: &[LogicalQubit],
    target: &RoutingTarget,
    initial_layout: &Layout,
) -> Result<Layout, CompilerError> {
    let mut mapping = BTreeMap::new();
    for logical in logical_qubits {
        let physical = initial_layout.get_physical(*logical).ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "sabre initial layout does not map logical qubit {logical}"
            ))
        })?;
        if !target.physical_set.contains(&physical) {
            return Err(CompilerError::InvalidInput(format!(
                "sabre initial layout maps logical qubit {logical} to unusable physical qubit {physical}"
            )));
        }
        mapping.insert(*logical, physical);
    }
    Layout::new(
        logical_qubits.to_vec(),
        target.physical_qubits.clone(),
        Some(mapping),
    )
    .map_err(|error| {
        CompilerError::InvalidInput(format!("sabre initial layout is invalid: {error}"))
    })
}

/// Validates that every two-qubit interaction in `circuit` is reachable.
///
/// The check uses the usable physical topology of `device` and the logical to
/// physical mapping in `initial_layout`. It validates reachability, not current
/// adjacency; non-adjacent but connected interactions can still be routed by
/// SABRE.
pub fn validate_reachable_interactions(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
) -> Result<(), CompilerError> {
    let physical = PhysicalLayoutGraph::from_device(device)?;
    let target = RoutingTarget::from_physical(&physical)?;
    let logical_qubits = circuit
        .qubits()
        .into_iter()
        .map(LogicalQubit::from_qubit)
        .collect::<Vec<_>>();
    let initial_layout =
        normalize_initial_layout_for_target(&logical_qubits, &target, initial_layout)?;
    let sabre = SabreDag::from_operations(circuit.operations())?;
    validate_reachable_interactions_for_target(&sabre, &target, &initial_layout)
}

#[derive(Debug, Clone)]
pub(crate) struct RoutingTarget {
    pub(crate) physical_qubits: Vec<PhysicalQubit>,
    physical_set: BTreeSet<PhysicalQubit>,
    physical_index: BTreeMap<PhysicalQubit, usize>,
    physical_order_indices: Vec<usize>,
    neighbors_by_index: Vec<Vec<usize>>,
    adjacent_by_index: Vec<Vec<bool>>,
    distances: Vec<Vec<Option<f64>>>,
    graph: UnGraph<(), ()>,
    graph_index: BTreeMap<PhysicalQubit, NodeIndex>,
    physical_by_index: Vec<PhysicalQubit>,
}

impl RoutingTarget {
    /// Builds the dense routing view used by SABRE scoring.
    ///
    /// The target keeps both semantic physical-qubit ids and dense indices.
    /// Dense indices make layer scoring cheap; semantic ids keep diagnostics
    /// and emitted SWAP operations stable.
    pub(crate) fn from_physical(physical: &PhysicalLayoutGraph) -> Result<Self, CompilerError> {
        let physical_qubits = physical.physical_qubits().to_vec();
        let physical_set = physical_qubits.iter().copied().collect::<BTreeSet<_>>();
        let mut graph = UnGraph::with_capacity(physical_qubits.len(), 0);
        let mut graph_index = BTreeMap::new();
        let mut physical_index = BTreeMap::new();
        let mut physical_by_index = Vec::with_capacity(physical_qubits.len());

        for (dense_index, physical) in physical_qubits.iter().copied().enumerate() {
            let graph_node = graph.add_node(());
            graph_index.insert(physical, graph_node);
            physical_index.insert(physical, dense_index);
            physical_by_index.push(physical);
        }
        let mut distances = vec![vec![None; physical_qubits.len()]; physical_qubits.len()];
        for (index, row) in distances.iter_mut().enumerate() {
            row[index] = Some(0.0);
        }
        let mut neighbors_by_index = vec![Vec::new(); physical_qubits.len()];
        let mut adjacent_by_index = vec![vec![false; physical_qubits.len()]; physical_qubits.len()];
        let mut physical_order_indices = (0..physical_qubits.len()).collect::<Vec<_>>();
        physical_order_indices.sort_unstable_by_key(|index| physical_qubits[*index]);

        for (left_index, left) in physical_qubits.iter().copied().enumerate() {
            for (right_index, right) in physical_qubits
                .iter()
                .copied()
                .enumerate()
                .skip(left_index + 1)
            {
                if let Some(distance) = physical.distance(left, right) {
                    distances[left_index][right_index] = Some(f64::from(distance));
                    distances[right_index][left_index] = Some(f64::from(distance));
                }
                if physical.is_adjacent_undirected(left, right) {
                    neighbors_by_index[left_index].push(right_index);
                    neighbors_by_index[right_index].push(left_index);
                    adjacent_by_index[left_index][right_index] = true;
                    adjacent_by_index[right_index][left_index] = true;
                    graph.add_edge(graph_index[&left], graph_index[&right], ());
                }
            }
        }
        for items in &mut neighbors_by_index {
            items.sort_unstable_by_key(|index| physical_qubits[*index]);
        }

        Ok(Self {
            physical_qubits,
            physical_set,
            physical_index,
            physical_order_indices,
            neighbors_by_index,
            adjacent_by_index,
            distances,
            graph,
            graph_index,
            physical_by_index,
        })
    }

    fn distance(&self, left: PhysicalQubit, right: PhysicalQubit) -> Result<f64, CompilerError> {
        let left_index = self.physical_index(left)?;
        let right_index = self.physical_index(right)?;
        self.distance_by_index(left_index, right_index)
    }

    fn physical_index(&self, physical: PhysicalQubit) -> Result<usize, CompilerError> {
        self.physical_index.get(&physical).copied().ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "physical qubit {physical} is not usable in the target topology"
            ))
        })
    }

    fn physical_at(&self, index: usize) -> Result<PhysicalQubit, CompilerError> {
        self.physical_qubits.get(index).copied().ok_or_else(|| {
            CompilerError::InvariantViolation(format!(
                "physical index {index} is outside target topology of length {}",
                self.physical_qubits.len()
            ))
        })
    }

    fn distance_by_index(
        &self,
        left_index: usize,
        right_index: usize,
    ) -> Result<f64, CompilerError> {
        let left = self.physical_at(left_index)?;
        let right = self.physical_at(right_index)?;
        self.distances[left_index][right_index].ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "physical qubits {left} and {right} are disconnected in the usable topology"
            ))
        })
    }

    fn are_adjacent(
        &self,
        left: PhysicalQubit,
        right: PhysicalQubit,
    ) -> Result<bool, CompilerError> {
        let left_index = self.physical_index(left)?;
        let right_index = self.physical_index(right)?;
        Ok(self.adjacent_by_index[left_index][right_index])
    }

    fn are_adjacent_by_index(&self, left_index: usize, right_index: usize) -> bool {
        self.adjacent_by_index[left_index][right_index]
    }

    fn shortest_path(
        &self,
        start: PhysicalQubit,
        goal: PhysicalQubit,
    ) -> Option<Vec<PhysicalQubit>> {
        if start == goal {
            return Some(vec![start]);
        }

        let start_index = self.physical_index(start).ok()?;
        let goal_index = self.physical_index(goal).ok()?;
        let mut queue = VecDeque::new();
        let mut predecessor = vec![None; self.physical_qubits.len()];
        let mut seen = vec![false; self.physical_qubits.len()];
        queue.push_back(start_index);
        seen[start_index] = true;

        while let Some(current) = queue.pop_front() {
            for &neighbor in &self.neighbors_by_index[current] {
                if seen[neighbor] {
                    continue;
                }
                seen[neighbor] = true;
                predecessor[neighbor] = Some(current);
                if neighbor == goal_index {
                    let mut path = vec![goal];
                    let mut cursor = goal_index;
                    while cursor != start_index {
                        cursor = predecessor[cursor]?;
                        path.push(self.physical_qubits[cursor]);
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(neighbor);
            }
        }
        None
    }
}

#[derive(Debug)]
struct RoutingState {
    layout: Layout,
    front_layer: Layer,
    lookahead_layers: Vec<Layer>,
    required_predecessors: Vec<u32>,
    decay: Vec<f64>,
    rng: StdRng,
}

#[derive(Debug, Clone, Copy)]
struct SwapChoice {
    physical: [PhysicalQubit; 2],
    indices: [usize; 2],
}

impl RoutingState {
    /// Creates mutable state for one SABRE routing trial.
    ///
    /// `required_predecessors` is the mutable readiness counter for DAG
    /// scheduling. Lookahead temporarily edits the same counters and restores
    /// them before returning to the real routing loop.
    fn new(
        sabre: &SabreDag,
        target: &RoutingTarget,
        layout: Layout,
        heuristic: &SabreHeuristicConfig,
        seed: u64,
    ) -> Self {
        let mut required_predecessors = vec![0; sabre.graph.node_count()];
        for edge in sabre.graph.edge_references() {
            required_predecessors[edge.target().index()] += 1;
        }

        Self {
            layout,
            front_layer: Layer::new(sabre.graph.node_count(), target.physical_qubits.len()),
            lookahead_layers: vec![
                Layer::new(
                    sabre.graph.node_count(),
                    target.physical_qubits.len()
                );
                heuristic.lookahead_weights.len()
            ],
            required_predecessors,
            decay: vec![1.0; target.physical_qubits.len()],
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Applies a physical SWAP to the layout and all cached layer scores.
    ///
    /// The layout and every cached layer must move together; otherwise future
    /// SWAP deltas would be scored against stale physical positions.
    fn apply_swap(
        &mut self,
        swap: [PhysicalQubit; 2],
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        let swap_indices = [
            target.physical_index(swap[0])?,
            target.physical_index(swap[1])?,
        ];
        let distance = |left, right| target.distance_by_index(left, right);
        self.front_layer.apply_swap(swap_indices, &distance)?;
        for layer in &mut self.lookahead_layers {
            layer.apply_swap(swap_indices, &distance)?;
        }
        self.layout
            .swap_physical(swap[0], swap[1])
            .map_err(|error| {
                CompilerError::InvariantViolation(format!(
                    "sabre attempted an invalid physical swap {swap:?}: {error}"
                ))
            })
    }

    fn update_route(
        &mut self,
        sabre: &SabreDag,
        target: &RoutingTarget,
        heuristic: &SabreHeuristicConfig,
        output: &mut TrialOutput,
        nodes: &[NodeIndex],
        initial_swaps: Option<Vec<[PhysicalQubit; 2]>>,
    ) -> Result<(), CompilerError> {
        let mut to_visit = nodes.iter().copied().collect::<VecDeque<_>>();
        let mut pending_swaps = initial_swaps;

        while let Some(node_id) = to_visit.pop_front() {
            let node = &sabre.graph[node_id];
            match &node.kind {
                SabreNodeKind::TwoQ(pair) => {
                    // A two-qubit node that is still non-adjacent becomes part
                    // of the front layer. Adjacent nodes flush any pending
                    // SWAPs and are emitted under the current layout.
                    let physical = [
                        physical_for(&self.layout, pair[0])?,
                        physical_for(&self.layout, pair[1])?,
                    ];
                    if !target.are_adjacent(physical[0], physical[1])? {
                        let distance = |left, right| target.distance_by_index(left, right);
                        self.front_layer.insert(
                            node_id,
                            [
                                target.physical_index(physical[0])?,
                                target.physical_index(physical[1])?,
                            ],
                            &distance,
                        )?;
                        continue;
                    }
                    output.apply_pending_swaps(pending_swaps.take());
                    for operation in &node.operations {
                        output
                            .operations
                            .push(map_operation(operation, &self.layout)?);
                    }
                }
                SabreNodeKind::Synchronize => {
                    // Synchronize nodes preserve parent-level ordering around
                    // classical or barrier-like effects but do not add an
                    // adjacency constraint of their own.
                    output.apply_pending_swaps(pending_swaps.take());
                    for operation in &node.operations {
                        output
                            .operations
                            .push(map_operation(operation, &self.layout)?);
                    }
                }
                SabreNodeKind::ControlFlow(flow) => {
                    // Control-flow bodies are routed recursively from the
                    // current entry layout and restore that layout on exit, so
                    // parent routing can continue with a single layout state.
                    output.apply_pending_swaps(pending_swaps.take());
                    self.route_control_flow_node(
                        flow,
                        &node.operations,
                        target,
                        heuristic,
                        output,
                    )?;
                }
            }

            for edge in sabre.graph.edges_directed(node_id, Direction::Outgoing) {
                let successor = edge.target();
                self.required_predecessors[successor.index()] -= 1;
                if self.required_predecessors[successor.index()] == 0 {
                    to_visit.push_back(successor);
                }
            }
        }

        if pending_swaps.is_some() {
            return Err(CompilerError::InvariantViolation(
                "sabre selected swaps that did not route any front-layer node".to_string(),
            ));
        }
        Ok(())
    }

    fn route_control_flow_node(
        &mut self,
        flow: &SabreControlFlow,
        operations: &[Operation],
        target: &RoutingTarget,
        heuristic: &SabreHeuristicConfig,
        output: &mut TrialOutput,
    ) -> Result<(), CompilerError> {
        let Some((first, rest)) = operations.split_first() else {
            return Ok(());
        };
        // The SABRE DAG keeps the representative control-flow operation first
        // and may attach additional bookkeeping operations after it. Rebuild the
        // first operation with routed bodies, then map the remaining operations
        // through the unchanged parent layout.
        let routed = match flow {
            SabreControlFlow::If {
                condition,
                then_body,
                else_body,
            } => {
                let then_result = route_control_flow_body(
                    then_body,
                    target,
                    &self.layout,
                    heuristic,
                    output.next_nested_seed(),
                )?;
                let else_result = else_body
                    .as_ref()
                    .map(|body| {
                        route_control_flow_body(
                            body,
                            target,
                            &self.layout,
                            heuristic,
                            output.next_nested_seed(),
                        )
                    })
                    .transpose()?;
                output.merge_nested(&then_result);
                if let Some(result) = &else_result {
                    output.merge_nested(result);
                }
                let flow = ClassicalControlOp::If(IfOp::new(
                    condition.clone(),
                    ControlBody::new(then_result.operations),
                    else_result.map(|result| ControlBody::new(result.operations)),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::While { condition, body } => {
                let body_result = route_control_flow_body(
                    body,
                    target,
                    &self.layout,
                    heuristic,
                    output.next_nested_seed(),
                )?;
                output.merge_nested(&body_result);
                let flow = ClassicalControlOp::While(WhileOp::new(
                    condition.clone(),
                    ControlBody::new(body_result.operations),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::For {
                var,
                start,
                stop,
                step,
                body,
            } => {
                let body_result = route_control_flow_body(
                    body,
                    target,
                    &self.layout,
                    heuristic,
                    output.next_nested_seed(),
                )?;
                output.merge_nested(&body_result);
                let flow = ClassicalControlOp::For(ForOp::new(
                    *var,
                    start.clone(),
                    stop.clone(),
                    step.clone(),
                    ControlBody::new(body_result.operations),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
            SabreControlFlow::Switch {
                target: switch_target,
                cases,
                default,
            } => {
                let mut routed_cases = Vec::with_capacity(cases.len());
                for case in cases {
                    let result = route_control_flow_body(
                        &case.body,
                        target,
                        &self.layout,
                        heuristic,
                        output.next_nested_seed(),
                    )?;
                    output.merge_nested(&result);
                    routed_cases.push(SwitchCase::new(
                        case.value,
                        ControlBody::new(result.operations),
                    ));
                }
                let routed_default = default
                    .as_ref()
                    .map(|body| {
                        route_control_flow_body(
                            body,
                            target,
                            &self.layout,
                            heuristic,
                            output.next_nested_seed(),
                        )
                    })
                    .transpose()?;
                if let Some(result) = &routed_default {
                    output.merge_nested(result);
                }
                let flow = ClassicalControlOp::Switch(SwitchOp::new(
                    switch_target.clone(),
                    routed_cases,
                    routed_default.map(|result| ControlBody::new(result.operations)),
                )?);
                let qubits = flow.used_qubits().into_iter().collect();
                Operation {
                    instruction: Instruction::ClassicalControl(flow),
                    qubits,
                    params: SmallVec::new(),
                    label: first.label.clone(),
                }
            }
        };
        output.operations.push(routed);
        for operation in rest {
            output
                .operations
                .push(map_operation(operation, &self.layout)?);
        }
        Ok(())
    }

    fn populate_extended_set(
        &mut self,
        sabre: &SabreDag,
        target: &RoutingTarget,
    ) -> Result<(), CompilerError> {
        // Build fixed-depth lookahead layers from the current front layer. Synchronize
        // and control-flow nodes are transparent for depth counting because they do
        // not add a parent-level two-qubit adjacency constraint.
        let mut next_visit = self.front_layer.iter_nodes().collect::<Vec<_>>();
        let mut to_visit = Vec::new();
        let mut decremented = BTreeMap::<NodeIndex, u32>::new();

        for layer in &mut self.lookahead_layers {
            for node in next_visit.drain(..) {
                for edge in sabre.graph.edges_directed(node, Direction::Outgoing) {
                    let successor = edge.target();
                    *decremented.entry(successor).or_insert(0) += 1;
                    self.required_predecessors[successor.index()] -= 1;
                    if self.required_predecessors[successor.index()] == 0 {
                        to_visit.push(successor);
                    }
                }
            }

            let mut index = 0;
            while index < to_visit.len() {
                let node = to_visit[index];
                match &sabre.graph[node].kind {
                    SabreNodeKind::TwoQ(pair) => {
                        if let (Ok(left), Ok(right)) = (
                            physical_for(&self.layout, pair[0]),
                            physical_for(&self.layout, pair[1]),
                        ) {
                            let distance = |left, right| target.distance_by_index(left, right);
                            layer.insert(
                                node,
                                [target.physical_index(left)?, target.physical_index(right)?],
                                &distance,
                            )?;
                            next_visit.push(node);
                        }
                        // Missing physical mappings are ignored defensively here.
                        // Normal routing entrypoints normalize complete layouts before
                        // creating state, so this only affects future partial-layout use.
                    }
                    SabreNodeKind::Synchronize | SabreNodeKind::ControlFlow(_) => {
                        for edge in sabre.graph.edges_directed(node, Direction::Outgoing) {
                            let successor = edge.target();
                            *decremented.entry(successor).or_insert(0) += 1;
                            self.required_predecessors[successor.index()] -= 1;
                            if self.required_predecessors[successor.index()] == 0 {
                                to_visit.push(successor);
                            }
                        }
                    }
                }
                index += 1;
            }
            to_visit.clear();
        }

        // Lookahead exploration temporarily relaxes predecessor counts; restore
        // them before the real routing state advances.
        for (node, amount) in decremented {
            self.required_predecessors[node.index()] += amount;
        }
        Ok(())
    }

    fn choose_best_swap(
        &mut self,
        target: &RoutingTarget,
        heuristic: &SabreHeuristicConfig,
    ) -> Result<SwapChoice, CompilerError> {
        let mut candidates = Vec::new();
        for active_index in self
            .front_layer
            .active_indices_in_order(&target.physical_order_indices)
        {
            let active = target.physical_at(active_index)?;
            for &neighbor_index in &target.neighbors_by_index[active_index] {
                let neighbor = target.physical_at(neighbor_index)?;
                candidates.push(if active <= neighbor {
                    SwapChoice {
                        physical: [active, neighbor],
                        indices: [active_index, neighbor_index],
                    }
                } else {
                    SwapChoice {
                        physical: [neighbor, active],
                        indices: [neighbor_index, active_index],
                    }
                });
            }
        }
        candidates.sort_unstable_by_key(|candidate| candidate.physical);
        candidates.dedup_by(|left, right| left.physical == right.physical);
        if candidates.is_empty() {
            return Err(CompilerError::TransformFailed {
                name: "sabre_route",
                reason: "no candidate SWAP can affect the front layer".to_string(),
            });
        }

        // SABRE score = weighted front-layer distance + weighted lookahead
        // distance, with optional multiplicative decay on recently swapped
        // physical qubits.
        let distance = |left, right| target.distance_by_index(left, right);
        let mut absolute = heuristic.basic_weight * self.front_layer.total_score();
        for (layer, weight) in self
            .lookahead_layers
            .iter()
            .zip(heuristic.lookahead_weights.iter().copied())
        {
            absolute += weight * layer.total_score();
        }

        let mut best_score = f64::INFINITY;
        let mut best_swaps = Vec::new();
        for candidate in candidates {
            let mut score = absolute
                + heuristic.basic_weight
                    * self
                        .front_layer
                        .swap_delta_score(candidate.indices, &distance)?;
            for (layer, weight) in self
                .lookahead_layers
                .iter()
                .zip(heuristic.lookahead_weights.iter().copied())
            {
                score += weight * layer.swap_delta_score(candidate.indices, &distance)?;
            }
            if heuristic.decay_increment.is_some() {
                let decay = self.decay[candidate.indices[0]].max(self.decay[candidate.indices[1]]);
                score *= decay;
            }

            if score - best_score < -heuristic.best_epsilon {
                best_score = score;
                best_swaps.clear();
                best_swaps.push(candidate);
            } else if (score - best_score).abs() <= heuristic.best_epsilon {
                best_swaps.push(candidate);
            }
        }

        best_swaps.choose(&mut self.rng).copied().ok_or_else(|| {
            CompilerError::InvariantViolation("sabre found no best SWAP".to_string())
        })
    }

    fn force_enable_closest_node(
        &mut self,
        target: &RoutingTarget,
        current_swaps: &mut Vec<[PhysicalQubit; 2]>,
    ) -> Result<Vec<NodeIndex>, CompilerError> {
        // Fallback guarantees progress when the heuristic is stuck: choose the
        // closest front-layer interaction and walk one endpoint along a shortest
        // path until that interaction becomes adjacent.
        let (closest_node, qubits) = self
            .front_layer
            .iter()
            .min_by(|(_, a), (_, b)| {
                target
                    .distance_by_index(a[0], a[1])
                    .and_then(|ad| {
                        target
                            .distance_by_index(b[0], b[1])
                            .map(|bd| ad.total_cmp(&bd))
                    })
                    .unwrap_or(Ordering::Equal)
            })
            .ok_or_else(|| {
                CompilerError::InvariantViolation(
                    "sabre fallback called with an empty front layer".to_string(),
                )
            })?;
        let path_start = target.physical_at(qubits[0])?;
        let path_goal = target.physical_at(qubits[1])?;
        let path = target.shortest_path(path_start, path_goal).ok_or_else(|| {
            CompilerError::InvalidInput(format!(
                "physical qubits {path_start} and {path_goal} are disconnected in the usable topology"
            ))
        })?;
        if path.len() < 3 {
            return Ok(vec![closest_node]);
        }
        for window in path.windows(2).take(path.len() - 2) {
            let swap = [window[0], window[1]];
            self.apply_swap(swap, target)?;
            current_swaps.push(swap);
        }

        let mut routed = Vec::new();
        if self.front_layer.iter().any(|(node, current)| {
            node == closest_node && target.are_adjacent_by_index(current[0], current[1])
        }) {
            routed.push(closest_node);
        }

        let adjacent = |left, right| target.are_adjacent_by_index(left, right);
        for swap in current_swaps.iter().copied() {
            let swap_indices = [
                target.physical_index(swap[0])?,
                target.physical_index(swap[1])?,
            ];
            for candidate in swap_indices
                .into_iter()
                .filter_map(|index| self.front_layer.routable_node_on_index(index, &adjacent))
            {
                if !routed.contains(&candidate) {
                    routed.push(candidate);
                }
            }
        }

        if routed.is_empty() {
            routed.push(closest_node);
        }
        Ok(routed)
    }
}

#[derive(Debug, Default)]
struct TrialOutput {
    operations: Vec<Operation>,
    swap_count: usize,
    fallback_count: usize,
    control_flow_blocks_routed: usize,
    nested_seed_counter: u64,
}

impl TrialOutput {
    fn new(seed: u64) -> Self {
        Self {
            nested_seed_counter: seed,
            ..Self::default()
        }
    }

    fn apply_pending_swaps(&mut self, swaps: Option<Vec<[PhysicalQubit; 2]>>) {
        if let Some(swaps) = swaps {
            self.swap_count += swaps.len();
            self.operations
                .extend(swaps.into_iter().map(swap_operation));
        }
    }

    fn next_nested_seed(&mut self) -> u64 {
        let seed = self.nested_seed_counter;
        self.nested_seed_counter = self.nested_seed_counter.wrapping_add(1);
        seed
    }

    fn merge_nested(&mut self, nested: &TrialResult) {
        self.swap_count += nested.swap_count;
        self.fallback_count += nested.fallback_count;
        self.control_flow_blocks_routed += nested.control_flow_blocks_routed + 1;
    }
}

/// Routes a control-flow body and restores its entry layout before exit.
///
/// A control-flow body must be layout-neutral from the parent router's point of
/// view: whichever branch or iteration executes, the body exits with the same
/// logical-to-physical mapping it entered with.
fn route_control_flow_body(
    sabre: &SabreDag,
    target: &RoutingTarget,
    entry_layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<TrialResult, CompilerError> {
    let mut result = route_trial(sabre, target, entry_layout, heuristic, seed)?;
    let epilogue_swaps = restore_layout_swaps(target, &result.final_layout, entry_layout, seed)?;
    let mut layout = result.final_layout.clone();
    for swap in &epilogue_swaps {
        layout.swap_physical(swap[0], swap[1]).map_err(|error| {
            CompilerError::InvariantViolation(format!(
                "sabre control-flow epilogue generated an invalid SWAP: {error}"
            ))
        })?;
    }
    let control_transfer = matches!(
        result
            .operations
            .last()
            .map(|operation| &operation.instruction),
        Some(Instruction::ClassicalControl(
            ClassicalControlOp::Break | ClassicalControlOp::Continue
        ))
    )
    .then(|| result.operations.pop().expect("last operation exists"));
    result
        .operations
        .extend(epilogue_swaps.iter().copied().map(swap_operation));
    result.operations.extend(control_transfer);
    result.swap_count += epilogue_swaps.len();
    result.final_layout = layout;
    result.quality = trial_quality(&result.operations, result.swap_count);
    if result.final_layout.l2p_map() != entry_layout.l2p_map() {
        return Err(CompilerError::InvariantViolation(
            "sabre control-flow epilogue did not restore the entry layout".to_string(),
        ));
    }
    Ok(result)
}

/// Computes SWAPs that restore one layout to another on the target graph.
///
/// Token swapping computes an epilogue that returns every live logical qubit to
/// its entry physical location. Vacant physical qubits are irrelevant and are
/// omitted from the token mapping.
fn restore_layout_swaps(
    target: &RoutingTarget,
    current: &Layout,
    desired: &Layout,
    seed: u64,
) -> Result<Vec<[PhysicalQubit; 2]>, CompilerError> {
    let mapping = desired
        .physical_qubits()
        .filter_map(|physical| {
            let logical = current.get_logical(physical)?;
            let desired_physical = desired
                .get_physical(logical)
                .expect("desired layout maps logical qubits it reports");
            Some((
                target.graph_index[&physical],
                target.graph_index[&desired_physical],
            ))
        })
        .collect();

    let swaps = token_swapper(
        &target.graph,
        mapping,
        Some(CONTROL_FLOW_EPILOGUE_TRIALS),
        Some(seed),
        None,
    )
    .map_err(|error| CompilerError::TransformFailed {
        name: "sabre_route",
        reason: format!("failed to restore control-flow layout: {error}"),
    })?;
    swaps
        .into_iter()
        .map(|(left, right)| {
            Ok([
                target.physical_by_index[left.index()],
                target.physical_by_index[right.index()],
            ])
        })
        .collect()
}

fn map_operation(operation: &Operation, layout: &Layout) -> Result<Operation, CompilerError> {
    Ok(Operation {
        instruction: operation.instruction.clone(),
        qubits: operation
            .qubits
            .iter()
            .copied()
            .map(|qubit| {
                physical_for(layout, LogicalQubit::from_qubit(qubit)).map(PhysicalQubit::qubit)
            })
            .collect::<Result<SmallVec<[Qubit; 3]>, _>>()?,
        params: operation.params.clone(),
        label: operation.label.clone(),
    })
}

/// Remaps operation parameter indices into the routed circuit's parameter table.
///
/// Parameter indices are scoped to a circuit table, while routed nested
/// operations are rebuilt before the final table exists. This function walks
/// recursively so every body points at the reordered routed table.
fn remap_parameter_indices(
    operation: &Operation,
    parameter_indices: &[u32],
) -> Result<Operation, CompilerError> {
    let mut mapped = operation.clone();
    for param in &mut mapped.params {
        if let CircuitParam::Index(index) = param {
            *index = *parameter_indices
                .get(*index as usize)
                .ok_or(crate::circuit::CircuitError::InvalidParameterIndex(*index))?;
        }
    }
    mapped.instruction = match &operation.instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_body = op
                .then_body()
                .operations()
                .iter()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            let else_body = op
                .else_body()
                .map(|body| {
                    body.operations()
                        .iter()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()
                        .map(ControlBody::new)
                })
                .transpose()?;
            Instruction::ClassicalControl(ClassicalControlOp::If(IfOp::new(
                op.condition().clone(),
                ControlBody::new(then_body),
                else_body,
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            let body = op
                .body()
                .operations()
                .iter()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            Instruction::ClassicalControl(ClassicalControlOp::While(WhileOp::new(
                op.condition().clone(),
                ControlBody::new(body),
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            let body = op
                .body()
                .operations()
                .iter()
                .map(|operation| remap_parameter_indices(operation, parameter_indices))
                .collect::<Result<Vec<_>, _>>()?;
            Instruction::ClassicalControl(ClassicalControlOp::For(ForOp::new(
                op.var(),
                op.start().clone(),
                op.stop().clone(),
                op.step().clone(),
                ControlBody::new(body),
            )?))
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
            let cases = op
                .cases()
                .iter()
                .map(|case| {
                    let body = case
                        .body()
                        .operations()
                        .iter()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(SwitchCase::new(case.value(), ControlBody::new(body)))
                })
                .collect::<Result<Vec<_>, CompilerError>>()?;
            let default = op
                .default()
                .map(|body| {
                    body.operations()
                        .iter()
                        .map(|operation| remap_parameter_indices(operation, parameter_indices))
                        .collect::<Result<Vec<_>, _>>()
                        .map(ControlBody::new)
                })
                .transpose()?;
            Instruction::ClassicalControl(ClassicalControlOp::Switch(SwitchOp::new(
                op.target().clone(),
                cases,
                default,
            )?))
        }
        _ => operation.instruction.clone(),
    };
    Ok(mapped)
}

fn physical_for(layout: &Layout, logical: LogicalQubit) -> Result<PhysicalQubit, CompilerError> {
    layout.get_physical(logical).ok_or_else(|| {
        CompilerError::InvariantViolation(format!(
            "sabre layout does not map logical qubit {logical}"
        ))
    })
}

/// Validates that every interaction in a SABRE DAG is physically reachable.
///
/// Reachability is recursive because control-flow bodies are routed with the
/// same physical topology and entry-layout contract as parent operations.
pub(crate) fn validate_reachable_interactions_for_target(
    sabre: &SabreDag,
    target: &RoutingTarget,
    layout: &Layout,
) -> Result<(), CompilerError> {
    for node in sabre.graph.node_weights() {
        match &node.kind {
            SabreNodeKind::TwoQ(pair) => {
                let left = physical_for(layout, pair[0])?;
                let right = physical_for(layout, pair[1])?;
                target.distance(left, right)?;
            }
            SabreNodeKind::ControlFlow(SabreControlFlow::If {
                then_body,
                else_body,
                ..
            }) => {
                validate_reachable_interactions_for_target(then_body, target, layout)?;
                if let Some(else_body) = else_body {
                    validate_reachable_interactions_for_target(else_body, target, layout)?;
                }
            }
            SabreNodeKind::ControlFlow(
                SabreControlFlow::While { body, .. } | SabreControlFlow::For { body, .. },
            ) => {
                validate_reachable_interactions_for_target(body, target, layout)?;
            }
            SabreNodeKind::ControlFlow(SabreControlFlow::Switch { cases, default, .. }) => {
                for case in cases {
                    validate_reachable_interactions_for_target(&case.body, target, layout)?;
                }
                if let Some(default) = default {
                    validate_reachable_interactions_for_target(default, target, layout)?;
                }
            }
            SabreNodeKind::Synchronize => {}
        }
    }
    Ok(())
}

fn swap_operation(swap: [PhysicalQubit; 2]) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::SWAP),
        qubits: smallvec![swap[0].qubit(), swap[1].qubit()],
        params: SmallVec::new(),
        label: None,
    }
}

pub(crate) fn compare_trial_quality(
    objective: SabreTrialObjective,
    left: TrialQuality,
    left_index: usize,
    right: TrialQuality,
    right_index: usize,
) -> Ordering {
    match objective {
        SabreTrialObjective::SwapCount => left
            .swap_count
            .cmp(&right.swap_count)
            .then_with(|| left_index.cmp(&right_index)),
        SabreTrialObjective::Depth => left
            .two_qubit_depth
            .cmp(&right.two_qubit_depth)
            .then_with(|| left_index.cmp(&right_index)),
        SabreTrialObjective::SwapThenDepth => left
            .swap_count
            .cmp(&right.swap_count)
            .then_with(|| left.two_qubit_depth.cmp(&right.two_qubit_depth))
            .then_with(|| left.operation_count.cmp(&right.operation_count))
            .then_with(|| left_index.cmp(&right_index)),
        SabreTrialObjective::DepthThenSwap => left
            .two_qubit_depth
            .cmp(&right.two_qubit_depth)
            .then_with(|| left.swap_count.cmp(&right.swap_count))
            .then_with(|| left.operation_count.cmp(&right.operation_count))
            .then_with(|| left_index.cmp(&right_index)),
    }
}

fn trial_quality(operations: &[Operation], swap_count: usize) -> TrialQuality {
    TrialQuality {
        swap_count,
        two_qubit_depth: two_qubit_depth(operations),
        operation_count: operation_count(operations),
    }
}

/// Estimates ASAP two-qubit depth for trial ranking.
///
/// This is not a full scheduler. Control-flow contributes the maximum local
/// branch or body depth, and parent operations are chained by used qubits.
fn two_qubit_depth(operations: &[Operation]) -> usize {
    let mut qubit_depths = BTreeMap::<Qubit, usize>::new();
    let mut max_depth = 0usize;

    for operation in operations {
        let local_depth = operation_local_two_qubit_depth(operation);
        if local_depth == 0 {
            continue;
        }

        let base = operation
            .qubits
            .iter()
            .map(|qubit| qubit_depths.get(qubit).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let depth = base + local_depth;
        for qubit in &operation.qubits {
            qubit_depths.insert(*qubit, depth);
        }
        max_depth = max_depth.max(depth);
    }

    max_depth
}

fn operation_local_two_qubit_depth(operation: &Operation) -> usize {
    match &operation.instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let then_depth = two_qubit_depth(op.then_body().operations());
            let else_depth = op
                .else_body()
                .map(|body| two_qubit_depth(body.operations()))
                .unwrap_or(0);
            then_depth.max(else_depth)
        }
        Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
            two_qubit_depth(op.body().operations())
        }
        Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
            two_qubit_depth(op.body().operations())
        }
        Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => op
            .cases()
            .iter()
            .map(|case| two_qubit_depth(case.body().operations()))
            .chain(
                op.default()
                    .into_iter()
                    .map(|body| two_qubit_depth(body.operations())),
            )
            .max()
            .unwrap_or(0),
        _ if operation.qubits.len() == 2 => 1,
        _ => 0,
    }
}

fn operation_count(operations: &[Operation]) -> usize {
    operations
        .iter()
        .map(|operation| {
            1 + match &operation.instruction {
                Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
                    operation_count(op.then_body().operations())
                        + op.else_body()
                            .map(|body| operation_count(body.operations()))
                            .unwrap_or(0)
                }
                Instruction::ClassicalControl(ClassicalControlOp::While(op)) => {
                    operation_count(op.body().operations())
                }
                Instruction::ClassicalControl(ClassicalControlOp::For(op)) => {
                    operation_count(op.body().operations())
                }
                Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) => {
                    op.cases()
                        .iter()
                        .map(|case| operation_count(case.body().operations()))
                        .sum::<usize>()
                        + op.default()
                            .map(|body| operation_count(body.operations()))
                            .unwrap_or(0)
                }
                _ => 0,
            }
        })
        .sum()
}
