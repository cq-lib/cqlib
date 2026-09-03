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

use super::{CircuitLayoutAnalysis, PhysicalLayoutGraph};
use crate::compile::CompilerError;
use crate::device::{Layout, LogicalQubit};
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy)]
struct DemandEdge {
    left: usize,
    right: usize,
    weight: f64,
}

struct DemandGraph {
    edges: Vec<DemandEdge>,
    adjacent: Vec<Vec<(usize, f64, usize)>>,
    incident: Vec<Vec<usize>>,
    activity: Vec<f64>,
}

impl DemandGraph {
    fn new(analysis: &CircuitLayoutAnalysis) -> Self {
        let logical_index = analysis
            .logical_qubits
            .iter()
            .copied()
            .enumerate()
            .map(|(index, logical)| (logical, index))
            .collect::<BTreeMap<_, _>>();
        let mut edges = Vec::new();
        let mut adjacent = vec![Vec::new(); analysis.logical_qubits.len()];
        let mut incident = vec![Vec::new(); analysis.logical_qubits.len()];
        let mut activity = vec![0.0; analysis.logical_qubits.len()];
        for (interaction_index, interaction) in analysis
            .interactions
            .interactions()
            .iter()
            .enumerate()
            .filter(|(_, interaction)| interaction.weight > 0.0)
        {
            let left = logical_index[&interaction.left];
            let right = logical_index[&interaction.right];
            let [early, middle, late] = analysis.interactions.temporal_weights(interaction_index);
            let weight = early + 0.65 * middle + 0.35 * late;
            let edge_index = edges.len();
            edges.push(DemandEdge {
                left,
                right,
                weight,
            });
            adjacent[left].push((right, weight, interaction.first_seen_order));
            adjacent[right].push((left, weight, interaction.first_seen_order));
            incident[left].push(edge_index);
            incident[right].push(edge_index);
            activity[left] += weight;
            activity[right] += weight;
        }
        for neighbors in &mut adjacent {
            neighbors.sort_by(|left, right| {
                left.2
                    .cmp(&right.2)
                    .then_with(|| right.1.total_cmp(&left.1))
                    .then_with(|| left.0.cmp(&right.0))
            });
        }
        Self {
            edges,
            adjacent,
            incident,
            activity,
        }
    }

    fn cost(&self, mapping: &[usize], physical: &PhysicalLayoutGraph) -> f64 {
        self.edges
            .iter()
            .map(|edge| demand_edge_cost(*edge, mapping[edge.left], mapping[edge.right], physical))
            .sum()
    }
}

/// Builds deterministic graph-embedding seeds without target-specific tables.
pub(super) fn interaction_aware_layouts(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    budget: usize,
) -> Result<Vec<Layout>, CompilerError> {
    if budget == 0 || analysis.logical_qubits.is_empty() {
        return Ok(Vec::new());
    }
    let demand = DemandGraph::new(analysis);
    if demand.edges.is_empty() {
        return Ok(Vec::new());
    }
    let physical_adjacency = physical_adjacency(physical);
    let central_roots = central_physical_order(physical, &physical_adjacency);
    let logical_root = (0..analysis.logical_qubits.len())
        .max_by(|left, right| {
            demand.activity[*left]
                .total_cmp(&demand.activity[*right])
                .then_with(|| {
                    demand.adjacent[*left]
                        .len()
                        .cmp(&demand.adjacent[*right].len())
                })
                .then_with(|| right.cmp(left))
        })
        .unwrap_or(0);

    let mut mappings = Vec::new();
    for root in central_roots.iter().copied().take(3).take(budget) {
        mappings.push(weighted_embedding(logical_root, root, &demand, physical));
    }

    if mappings.len() < budget
        && let Some((logical_path, is_cycle)) = logical_path_or_cycle(&demand)
        && let Some(physical_path) =
            physical_walk(&physical_adjacency, logical_path.len(), is_cycle, 500_000)
    {
        mappings.push(mapping_from_walk(
            analysis.logical_qubits.len(),
            &logical_path,
            &physical_path,
            physical.physical_qubits().len(),
        ));
        if mappings.len() < budget {
            let mut reversed = physical_path;
            reversed.reverse();
            mappings.push(mapping_from_walk(
                analysis.logical_qubits.len(),
                &logical_path,
                &reversed,
                physical.physical_qubits().len(),
            ));
        }
    }

    let mut layouts = Vec::new();
    for mut mapping in mappings.into_iter().take(budget) {
        improve_mapping(&mut mapping, &demand, physical);
        layouts.push(layout_from_mapping(analysis, physical, &mapping)?);
    }
    Ok(layouts)
}

fn physical_adjacency(physical: &PhysicalLayoutGraph) -> Vec<Vec<usize>> {
    let mut adjacency = vec![Vec::new(); physical.physical_qubits().len()];
    for (left, right) in physical.undirected_edges_by_index() {
        adjacency[left].push(right);
        adjacency[right].push(left);
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
    }
    adjacency
}

fn central_physical_order(physical: &PhysicalLayoutGraph, adjacency: &[Vec<usize>]) -> Vec<usize> {
    let mut order = (0..physical.physical_qubits().len()).collect::<Vec<_>>();
    let physical_count = order.len();
    order.sort_by(|left, right| {
        let left_missing = (0..physical_count)
            .filter(|other| physical.distance_by_index(*left, *other).is_none())
            .count();
        let right_missing = (0..physical_count)
            .filter(|other| physical.distance_by_index(*right, *other).is_none())
            .count();
        let left_sum = (0..physical_count)
            .filter_map(|other| physical.distance_by_index(*left, other))
            .map(u64::from)
            .sum::<u64>();
        let right_sum = (0..physical_count)
            .filter_map(|other| physical.distance_by_index(*right, other))
            .map(u64::from)
            .sum::<u64>();
        left_missing
            .cmp(&right_missing)
            .then_with(|| left_sum.cmp(&right_sum))
            .then_with(|| adjacency[*right].len().cmp(&adjacency[*left].len()))
            .then_with(|| left.cmp(right))
    });
    order
}

fn weighted_embedding(
    logical_root: usize,
    physical_root: usize,
    demand: &DemandGraph,
    physical: &PhysicalLayoutGraph,
) -> Vec<usize> {
    let logical_count = demand.adjacent.len();
    let physical_count = physical.physical_qubits().len();
    let mut mapping = vec![usize::MAX; logical_count];
    let mut used = vec![false; physical_count];
    mapping[logical_root] = physical_root;
    used[physical_root] = true;

    while mapping.contains(&usize::MAX) {
        let logical = (0..logical_count)
            .filter(|logical| mapping[*logical] == usize::MAX)
            .max_by(|left, right| {
                let connected_weight = |logical: usize| {
                    demand.adjacent[logical]
                        .iter()
                        .filter(|(other, _, _)| mapping[*other] != usize::MAX)
                        .map(|(_, weight, _)| *weight)
                        .sum::<f64>()
                };
                connected_weight(*left)
                    .total_cmp(&connected_weight(*right))
                    .then_with(|| demand.activity[*left].total_cmp(&demand.activity[*right]))
                    .then_with(|| right.cmp(left))
            })
            .expect("an unmapped logical qubit exists");

        let selected = (0..physical_count)
            .filter(|physical| !used[*physical])
            .min_by(|left, right| {
                embedding_position_key(logical, *left, &mapping, demand, physical).cmp(
                    &embedding_position_key(logical, *right, &mapping, demand, physical),
                )
            })
            .expect("physical capacity is validated before layout generation");
        mapping[logical] = selected;
        used[selected] = true;
    }
    mapping
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PositionKey {
    disconnected: usize,
    distance: f64,
    centrality: u64,
    physical: usize,
}

impl Eq for PositionKey {}

impl Ord for PositionKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.disconnected
            .cmp(&other.disconnected)
            .then_with(|| self.distance.total_cmp(&other.distance))
            .then_with(|| self.centrality.cmp(&other.centrality))
            .then_with(|| self.physical.cmp(&other.physical))
    }
}

impl PartialOrd for PositionKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn embedding_position_key(
    logical: usize,
    candidate: usize,
    mapping: &[usize],
    demand: &DemandGraph,
    physical: &PhysicalLayoutGraph,
) -> PositionKey {
    let mut disconnected = 0;
    let mut distance = 0.0;
    for &(other, weight, _) in &demand.adjacent[logical] {
        if mapping[other] == usize::MAX {
            continue;
        }
        if let Some(steps) = physical.distance_by_index(candidate, mapping[other]) {
            distance += weight * f64::from(steps.saturating_sub(1));
        } else {
            disconnected += 1;
        }
    }
    let centrality = (0..physical.physical_qubits().len())
        .filter_map(|other| physical.distance_by_index(candidate, other))
        .map(u64::from)
        .sum();
    PositionKey {
        disconnected,
        distance,
        centrality,
        physical: candidate,
    }
}

fn logical_path_or_cycle(demand: &DemandGraph) -> Option<(Vec<usize>, bool)> {
    let active = demand
        .adjacent
        .iter()
        .enumerate()
        .filter(|(_, adjacent)| !adjacent.is_empty())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if active.is_empty()
        || active
            .iter()
            .any(|logical| demand.adjacent[*logical].len() > 2)
    {
        return None;
    }
    let endpoints = active
        .iter()
        .copied()
        .filter(|logical| demand.adjacent[*logical].len() == 1)
        .collect::<Vec<_>>();
    let is_cycle = endpoints.is_empty();
    if !is_cycle && endpoints.len() != 2 {
        return None;
    }
    let start = if is_cycle {
        *active.iter().min()?
    } else {
        *endpoints.iter().min_by_key(|logical| {
            (
                demand.adjacent[**logical][0].2,
                demand.adjacent[**logical][0].0,
            )
        })?
    };
    let mut path = Vec::with_capacity(active.len());
    let mut previous = None;
    let mut current = start;
    loop {
        path.push(current);
        let next = demand.adjacent[current]
            .iter()
            .filter(|(neighbor, _, _)| Some(*neighbor) != previous)
            .map(|(neighbor, _, _)| *neighbor)
            .find(|neighbor| *neighbor != start || path.len() == active.len());
        let Some(next) = next else {
            break;
        };
        if next == start {
            break;
        }
        previous = Some(current);
        current = next;
        if path.len() > active.len() {
            return None;
        }
    }
    (path.len() == active.len()).then_some((path, is_cycle))
}

fn physical_walk(
    adjacency: &[Vec<usize>],
    desired: usize,
    prefer_cycle: bool,
    mut budget: usize,
) -> Option<Vec<usize>> {
    if desired == 0 || desired > adjacency.len() {
        return None;
    }
    let mut starts = (0..adjacency.len()).collect::<Vec<_>>();
    starts.sort_by_key(|start| (adjacency[*start].len(), *start));
    for start in starts {
        let mut used = vec![false; adjacency.len()];
        let mut path = vec![start];
        used[start] = true;
        if walk_dfs(
            start,
            desired,
            prefer_cycle,
            adjacency,
            &mut used,
            &mut path,
            &mut budget,
        ) {
            return Some(path);
        }
        if budget == 0 {
            break;
        }
    }
    if prefer_cycle {
        return physical_walk(adjacency, desired, false, 250_000);
    }
    None
}

fn walk_dfs(
    start: usize,
    desired: usize,
    prefer_cycle: bool,
    adjacency: &[Vec<usize>],
    used: &mut [bool],
    path: &mut Vec<usize>,
    budget: &mut usize,
) -> bool {
    if path.len() == desired {
        return !prefer_cycle
            || adjacency[*path.last().expect("path is non-empty")].contains(&start);
    }
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    let current = *path.last().expect("path is non-empty");
    let mut next = adjacency[current]
        .iter()
        .copied()
        .filter(|candidate| !used[*candidate])
        .collect::<Vec<_>>();
    next.sort_by_key(|candidate| {
        let onward = adjacency[*candidate]
            .iter()
            .filter(|neighbor| !used[**neighbor])
            .count();
        let closes =
            prefer_cycle && path.len() + 1 == desired && adjacency[*candidate].contains(&start);
        (!closes, onward, *candidate)
    });
    for candidate in next {
        used[candidate] = true;
        path.push(candidate);
        if walk_dfs(start, desired, prefer_cycle, adjacency, used, path, budget) {
            return true;
        }
        path.pop();
        used[candidate] = false;
    }
    false
}

fn mapping_from_walk(
    logical_count: usize,
    logical_path: &[usize],
    physical_path: &[usize],
    physical_count: usize,
) -> Vec<usize> {
    let mut mapping = vec![usize::MAX; logical_count];
    let mut used = vec![false; physical_count];
    for (&logical, &physical) in logical_path.iter().zip(physical_path) {
        mapping[logical] = physical;
        used[physical] = true;
    }
    let mut remaining = (0..physical_count).filter(|physical| !used[*physical]);
    for physical in &mut mapping {
        if *physical == usize::MAX {
            *physical = remaining
                .next()
                .expect("physical capacity is validated before layout generation");
        }
    }
    mapping
}

fn improve_mapping(mapping: &mut [usize], demand: &DemandGraph, physical: &PhysicalLayoutGraph) {
    let mut hot = (0..mapping.len()).collect::<Vec<_>>();
    hot.sort_by(|left, right| {
        demand.activity[*right]
            .total_cmp(&demand.activity[*left])
            .then_with(|| left.cmp(right))
    });
    hot.truncate(24);
    let adjacency = physical_adjacency(physical);
    let central_order = central_physical_order(physical, &adjacency);
    for _ in 0..16 {
        let mut occupied = vec![None; physical.physical_qubits().len()];
        for (logical, physical) in mapping.iter().copied().enumerate() {
            occupied[physical] = Some(logical);
        }
        let mut vacant = Vec::new();
        for logical in hot.iter().copied() {
            for candidate in adjacency[mapping[logical]].iter().copied() {
                if occupied[candidate].is_none() && !vacant.contains(&candidate) {
                    vacant.push(candidate);
                }
            }
        }
        for candidate in central_order.iter().copied() {
            if occupied[candidate].is_none() && !vacant.contains(&candidate) {
                vacant.push(candidate);
            }
            if vacant.len() >= 16 {
                break;
            }
        }
        vacant.truncate(16);

        let mut best = None::<MappingImprovement>;
        for (position, left) in hot.iter().copied().enumerate() {
            for right in hot.iter().copied().skip(position + 1) {
                consider_improvement(
                    &mut best,
                    MappingImprovement::new(
                        mapping_delta(
                            mapping,
                            &[(left, mapping[right]), (right, mapping[left])],
                            demand,
                            physical,
                        ),
                        vec![(left, mapping[right]), (right, mapping[left])],
                    ),
                );
            }
            for target in vacant.iter().copied() {
                consider_improvement(
                    &mut best,
                    MappingImprovement::new(
                        mapping_delta(mapping, &[(left, target)], demand, physical),
                        vec![(left, target)],
                    ),
                );
                if let Some(path) = shortest_path_limited(mapping[left], target, &adjacency, 4)
                    && path.len() > 2
                {
                    let mut changes = vec![(left, target)];
                    for index in 1..path.len() - 1 {
                        if let Some(logical) = occupied[path[index]] {
                            changes.push((logical, path[index - 1]));
                        }
                    }
                    changes.sort_unstable();
                    consider_improvement(
                        &mut best,
                        MappingImprovement::new(
                            mapping_delta(mapping, &changes, demand, physical),
                            changes,
                        ),
                    );
                }
            }
        }
        let Some(best) = best.filter(|candidate| candidate.delta < -1e-12) else {
            break;
        };
        for (logical, physical) in best.changes {
            mapping[logical] = physical;
        }
    }
}

#[derive(Debug)]
struct MappingImprovement {
    delta: f64,
    changes: Vec<(usize, usize)>,
}

impl MappingImprovement {
    fn new(delta: f64, changes: Vec<(usize, usize)>) -> Self {
        Self { delta, changes }
    }
}

fn consider_improvement(best: &mut Option<MappingImprovement>, candidate: MappingImprovement) {
    if candidate.delta >= -1e-12 {
        return;
    }
    if best.as_ref().is_none_or(|current| {
        candidate.delta < current.delta - 1e-12
            || ((candidate.delta - current.delta).abs() <= 1e-12
                && candidate.changes < current.changes)
    }) {
        *best = Some(candidate);
    }
}

fn mapping_delta(
    mapping: &[usize],
    changes: &[(usize, usize)],
    demand: &DemandGraph,
    physical: &PhysicalLayoutGraph,
) -> f64 {
    let mut affected = Vec::new();
    for (logical, _) in changes {
        for edge in demand.incident[*logical].iter().copied() {
            if !affected.contains(&edge) {
                affected.push(edge);
            }
        }
    }
    let mapped = |logical: usize| {
        changes
            .iter()
            .find_map(|(changed, physical)| (*changed == logical).then_some(*physical))
            .unwrap_or(mapping[logical])
    };
    affected
        .into_iter()
        .map(|edge_index| {
            let edge = demand.edges[edge_index];
            let before = demand_edge_cost(edge, mapping[edge.left], mapping[edge.right], physical);
            let after = demand_edge_cost(edge, mapped(edge.left), mapped(edge.right), physical);
            after - before
        })
        .sum()
}

fn shortest_path_limited(
    start: usize,
    target: usize,
    adjacency: &[Vec<usize>],
    max_edges: usize,
) -> Option<Vec<usize>> {
    if start == target {
        return Some(vec![start]);
    }
    let mut predecessor = vec![usize::MAX; adjacency.len()];
    let mut depth = vec![usize::MAX; adjacency.len()];
    let mut queue = VecDeque::from([start]);
    depth[start] = 0;
    while let Some(current) = queue.pop_front() {
        if depth[current] >= max_edges {
            continue;
        }
        for next in adjacency[current].iter().copied() {
            if depth[next] != usize::MAX {
                continue;
            }
            predecessor[next] = current;
            depth[next] = depth[current] + 1;
            if next == target {
                let mut path = vec![target];
                let mut cursor = target;
                while cursor != start {
                    cursor = predecessor[cursor];
                    path.push(cursor);
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(next);
        }
    }
    None
}

fn demand_edge_cost(
    edge: DemandEdge,
    left: usize,
    right: usize,
    physical: &PhysicalLayoutGraph,
) -> f64 {
    // Keep disconnected penalties finite so a swap between two disconnected
    // placements still has a well-defined delta instead of producing
    // `infinity - infinity = NaN`.
    physical.distance_by_index(left, right).map_or_else(
        || edge.weight * (physical.physical_qubits().len().saturating_add(1) as f64),
        |distance| edge.weight * f64::from(distance.saturating_sub(1)),
    )
}

fn layout_from_mapping(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    mapping: &[usize],
) -> Result<Layout, CompilerError> {
    let mapping = analysis
        .logical_qubits
        .iter()
        .copied()
        .zip(mapping.iter().map(|index| {
            physical
                .physical_at(*index)
                .expect("interaction-aware mapping uses in-range physical indices")
        }))
        .collect::<BTreeMap<LogicalQubit, _>>();
    Layout::new(
        analysis.logical_qubits.clone(),
        physical.physical_qubits().to_vec(),
        Some(mapping),
    )
    .map_err(|error| {
        CompilerError::InvariantViolation(format!(
            "interaction-aware SABRE seed is not a valid layout: {error}"
        ))
    })
}

pub(super) fn interaction_layout_cost(
    analysis: &CircuitLayoutAnalysis,
    physical: &PhysicalLayoutGraph,
    layout: &Layout,
) -> Result<f64, CompilerError> {
    let mapping = analysis
        .logical_qubits
        .iter()
        .map(|logical| {
            let physical_qubit = layout.get_physical(*logical).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "interaction layout is missing logical qubit {logical}"
                ))
            })?;
            physical.physical_index(physical_qubit).ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "interaction layout uses unknown physical qubit {physical_qubit}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DemandGraph::new(analysis).cost(&mapping, physical))
}

#[cfg(test)]
#[path = "interaction_seed_test.rs"]
mod interaction_seed_test;
