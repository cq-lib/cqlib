// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use crate::compile::CompilerError;
use rustworkx_core::petgraph::prelude::NodeIndex;

/// Physical placement of one ready routing requirement.
///
/// Unary requirements deliberately use a distinct representation. Encoding a
/// unary operation as `[q, q]` would corrupt endpoint occupancy, pair-state
/// reachability, and candidate-SWAP generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequirementPlacement {
    Unary(usize),
    Pair([usize; 2]),
}

impl RequirementPlacement {
    fn endpoints(self) -> impl Iterator<Item = usize> {
        let (endpoints, length) = match self {
            Self::Unary(physical) => ([physical, physical], 1),
            Self::Pair(pair) => (pair, 2),
        };
        endpoints.into_iter().take(length)
    }

    pub(crate) fn after_swap(self, swap: [usize; 2]) -> Self {
        let moved = |physical| {
            if physical == swap[0] {
                swap[1]
            } else if physical == swap[1] {
                swap[0]
            } else {
                physical
            }
        };
        match self {
            Self::Unary(physical) => Self::Unary(moved(physical)),
            Self::Pair(pair) => Self::Pair(pair.map(moved)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LayerNode {
    requirement: usize,
    placement: RequirementPlacement,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Layer {
    nodes: Vec<Option<LayerNode>>,
    occupied_node_indices: Vec<usize>,
    occupied_positions: Vec<usize>,
    active: Vec<Option<NodeIndex>>,
    total_score: f64,
}

const VACANT_POSITION: usize = usize::MAX;

impl Layer {
    pub(crate) fn new(node_count: usize, physical_count: usize) -> Self {
        Self {
            nodes: vec![None; node_count],
            occupied_node_indices: Vec::new(),
            occupied_positions: vec![VACANT_POSITION; node_count],
            active: vec![None; physical_count],
            total_score: 0.0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.occupied_node_indices.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.occupied_node_indices.len()
    }

    pub(crate) fn insert<F>(
        &mut self,
        node: NodeIndex,
        requirement: usize,
        placement: RequirementPlacement,
        distances: &F,
    ) -> Result<(), CompilerError>
    where
        F: Fn(usize, RequirementPlacement) -> Result<f64, CompilerError>,
    {
        if node.index() >= self.nodes.len() {
            self.nodes.resize(node.index() + 1, None);
            self.occupied_positions
                .resize(node.index() + 1, VACANT_POSITION);
        }
        self.ensure_active_entries_available(node, placement)?;
        let replacement = LayerNode {
            requirement,
            placement,
        };
        if let Some(previous) = self.nodes[node.index()].replace(replacement) {
            self.total_score -= distances(previous.requirement, previous.placement)?;
            self.remove_active_entry(node, previous.placement);
        } else {
            self.insert_occupied_node_index(node.index());
        }
        self.total_score += distances(requirement, placement)?;
        self.insert_active_entry(node, placement);
        Ok(())
    }

    pub(crate) fn remove<F>(&mut self, node: NodeIndex, distances: &F) -> Result<(), CompilerError>
    where
        F: Fn(usize, RequirementPlacement) -> Result<f64, CompilerError>,
    {
        if node.index() >= self.nodes.len() {
            return Ok(());
        }
        if let Some(entry) = self.nodes[node.index()].take() {
            self.total_score -= distances(entry.requirement, entry.placement)?;
            self.remove_active_entry(node, entry.placement);
            self.remove_occupied_node_index(node.index());
        }
        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        for index in self.occupied_node_indices.drain(..) {
            if let Some(entry) = self.nodes[index].take() {
                for physical in entry.placement.endpoints() {
                    self.active[physical] = None;
                }
            }
            self.occupied_positions[index] = VACANT_POSITION;
        }
        self.total_score = 0.0;
    }

    pub(crate) fn apply_swap<F>(
        &mut self,
        swap: [usize; 2],
        distances: &F,
    ) -> Result<(), CompilerError>
    where
        F: Fn(usize, RequirementPlacement) -> Result<f64, CompilerError>,
    {
        let affected = self.swap_affected_nodes(swap);
        let mut updates = Vec::with_capacity(2);
        for node in affected.into_iter().flatten() {
            let entry = self.nodes[node.index()].ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "sabre layer active node {} has no node entry",
                    node.index()
                ))
            })?;
            let before = entry.placement;
            let after = before.after_swap(swap);
            let delta =
                distances(entry.requirement, after)? - distances(entry.requirement, before)?;
            updates.push((node, entry.requirement, before, after, delta));
        }

        for (node, requirement, before, after, delta) in updates {
            self.total_score += delta;
            self.remove_active_entry(node, before);
            self.nodes[node.index()] = Some(LayerNode {
                requirement,
                placement: after,
            });
            self.insert_active_entry(node, after);
        }
        Ok(())
    }

    /// Applies a SWAP using a dense row-major topology-distance matrix.
    ///
    /// This is the common topology-only path. Keeping the matrix lookup inside
    /// the layer avoids an indirect distance callback for every affected node.
    pub(crate) fn apply_topology_swap(
        &mut self,
        swap: [usize; 2],
        distances: &[u32],
        width: usize,
    ) -> Result<(), CompilerError> {
        let affected = self.swap_affected_nodes(swap);
        let mut updates = [None, None];
        for (slot, node) in affected.into_iter().flatten().enumerate() {
            let entry = self.nodes[node.index()].ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "sabre layer active node {} has no node entry",
                    node.index()
                ))
            })?;
            let before = entry.placement;
            let after = before.after_swap(swap);
            let delta = topology_distance(distances, width, after)
                - topology_distance(distances, width, before);
            updates[slot] = Some((node, entry.requirement, before, after, delta));
        }

        for (node, requirement, before, after, delta) in updates.into_iter().flatten() {
            self.total_score += delta;
            self.remove_active_entry(node, before);
            self.nodes[node.index()] = Some(LayerNode {
                requirement,
                placement: after,
            });
            self.insert_active_entry(node, after);
        }
        Ok(())
    }

    pub(crate) fn routable_node_on_index(
        &self,
        physical: usize,
        executable: &impl Fn(usize, RequirementPlacement) -> bool,
    ) -> Option<NodeIndex> {
        let node = self.active.get(physical).copied().flatten()?;
        let entry = self.nodes[node.index()]?;
        executable(entry.requirement, entry.placement).then_some(node)
    }

    pub(crate) fn active_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.occupied_node_indices
            .iter()
            .copied()
            .filter_map(|index| self.nodes[index])
            .flat_map(|entry| entry.placement.endpoints())
    }

    pub(crate) fn iter_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.occupied_node_indices
            .iter()
            .copied()
            .map(NodeIndex::new)
    }

    pub(crate) fn iter(
        &self,
    ) -> impl Iterator<Item = (NodeIndex, usize, RequirementPlacement)> + '_ {
        self.occupied_node_indices
            .iter()
            .copied()
            .filter_map(|index| {
                self.nodes[index]
                    .map(|entry| (NodeIndex::new(index), entry.requirement, entry.placement))
            })
    }

    pub(crate) fn placements_after_swap(
        &self,
        swap: [usize; 2],
    ) -> impl Iterator<Item = (usize, RequirementPlacement)> + '_ {
        self.occupied_node_indices
            .iter()
            .copied()
            .filter_map(move |index| {
                self.nodes[index].map(|entry| (entry.requirement, entry.placement.after_swap(swap)))
            })
    }

    /// Returns the layer's total distance after applying a candidate SWAP.
    #[inline]
    pub(crate) fn total_score_after_swap<F>(
        &self,
        swap: [usize; 2],
        distances: &F,
    ) -> Result<f64, CompilerError>
    where
        F: Fn(usize, RequirementPlacement) -> Result<f64, CompilerError>,
    {
        if self.occupied_node_indices.is_empty() {
            return Ok(0.0);
        }
        let mut delta = 0.0;
        for node in self.swap_affected_nodes(swap).into_iter().flatten() {
            let entry = self.nodes[node.index()].ok_or_else(|| {
                CompilerError::InvariantViolation(format!(
                    "sabre layer active node {} has no node entry",
                    node.index()
                ))
            })?;
            let after = entry.placement.after_swap(swap);
            delta += distances(entry.requirement, after)?
                - distances(entry.requirement, entry.placement)?;
        }
        Ok(self.total_score + delta)
    }

    /// Returns the layer score after a candidate SWAP using the compact
    /// topology-distance matrix directly.
    #[inline(always)]
    pub(crate) fn total_topology_score_after_swap(
        &self,
        swap: [usize; 2],
        distances: &[u32],
        width: usize,
    ) -> f64 {
        let mut delta = 0.0;
        for node in self.swap_affected_nodes(swap).into_iter().flatten() {
            let entry = self.nodes[node.index()]
                .expect("sabre active topology node must have a layer entry");
            let after = entry.placement.after_swap(swap);
            delta += topology_distance(distances, width, after)
                - topology_distance(distances, width, entry.placement);
        }
        self.total_score + delta
    }

    fn insert_occupied_node_index(&mut self, index: usize) {
        if self.occupied_positions[index] == VACANT_POSITION {
            self.occupied_positions[index] = self.occupied_node_indices.len();
            self.occupied_node_indices.push(index);
        }
    }

    fn remove_occupied_node_index(&mut self, index: usize) {
        let position = self.occupied_positions[index];
        if position == VACANT_POSITION {
            return;
        }
        let removed = self.occupied_node_indices.swap_remove(position);
        debug_assert_eq!(removed, index);
        self.occupied_positions[index] = VACANT_POSITION;
        if position < self.occupied_node_indices.len() {
            let replacement = self.occupied_node_indices[position];
            self.occupied_positions[replacement] = position;
        }
    }

    fn insert_active_entry(&mut self, node: NodeIndex, placement: RequirementPlacement) {
        for physical in placement.endpoints() {
            self.active[physical] = Some(node);
        }
    }

    fn ensure_active_entries_available(
        &self,
        node: NodeIndex,
        placement: RequirementPlacement,
    ) -> Result<(), CompilerError> {
        for physical in placement.endpoints() {
            if let Some(active) = self.active.get(physical).copied().flatten()
                && active != node
            {
                return Err(CompilerError::InvariantViolation(format!(
                    "sabre layer nodes {} and {} share physical endpoint {physical}",
                    active.index(),
                    node.index()
                )));
            }
        }
        Ok(())
    }

    fn remove_active_entry(&mut self, node: NodeIndex, placement: RequirementPlacement) {
        for physical in placement.endpoints() {
            if self.active[physical] == Some(node) {
                self.active[physical] = None;
            }
        }
    }

    fn swap_affected_nodes(&self, swap: [usize; 2]) -> [Option<NodeIndex>; 2] {
        let first = self.active[swap[0]];
        let second = self.active[swap[1]].filter(|node| Some(*node) != first);
        [first, second]
    }
}

#[inline(always)]
fn topology_distance(distances: &[u32], width: usize, placement: RequirementPlacement) -> f64 {
    let RequirementPlacement::Pair([left, right]) = placement else {
        return 1.0;
    };
    let index = left * width + right;
    let distance = distances[index];
    debug_assert_ne!(
        distance,
        u32::MAX,
        "prepared topology route crossed disconnected components"
    );
    f64::from(distance)
}

#[cfg(test)]
#[path = "layer_test.rs"]
mod layer_test;
