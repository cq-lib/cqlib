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

//! Non-induced VF2++ search used by perfect layout.
//!
//! This module uses `rustworkx-core`/`petgraph` as the graph substrate. Its
//! search state combines ordered graph copies, bidirectional mappings, frontier
//! state, adjacency matrices, lookahead feasibility checks, and an extension
//! call limit.
//!
//! The engine deliberately returns only node-index mappings. Layout-specific
//! concepts such as logical qubit IDs, physical qubit IDs, direction penalties,
//! and calibration scoring stay in the public `vf2` adapter.

use rustworkx_core::petgraph::graph::{NodeIndex, UnGraph};
use rustworkx_core::petgraph::visit::EdgeRef;
use std::cmp::Reverse;

pub(super) type Vf2Graph = UnGraph<(), ()>;

/// Search limits for the internal non-induced VF2 matcher.
#[derive(Debug, Clone, Copy)]
pub(super) struct Vf2SearchConfig {
    /// Maximum number of complete mappings to collect.
    pub candidate_limit: usize,
    /// Maximum number of candidate node-pair extensions to try.
    pub call_limit: Option<usize>,
}

/// Search counters reported back to the layout adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Vf2SearchStats {
    /// Number of complete mappings emitted by the search.
    pub candidates_evaluated: usize,
    /// Number of candidate node-pair extensions attempted.
    pub calls: usize,
    /// Whether search stopped because [`Vf2SearchConfig::call_limit`] was hit.
    pub stopped_by_call_limit: bool,
}

/// Finds non-induced embeddings of `needle` in `haystack`.
///
/// Each returned vector maps `needle` node index to `haystack` node index.
/// Non-induced semantics require every needle edge to exist in the haystack but
/// allow extra haystack edges between mapped nodes.
pub(super) fn find_non_induced_mappings(
    needle: &Vf2Graph,
    haystack: &Vf2Graph,
    config: Vf2SearchConfig,
) -> (Vec<Vec<usize>>, Vf2SearchStats) {
    if needle.node_count() > haystack.node_count() || needle.edge_count() > haystack.edge_count() {
        return (Vec::new(), Vf2SearchStats::default());
    }

    let needle_order = vf2pp_order(needle);
    let haystack_order = vf2pp_order(haystack);
    let mut search = Search {
        needle: GraphState::new(needle),
        haystack: GraphState::new(haystack),
        needle_order,
        haystack_order,
        config,
        mappings: Vec::new(),
        stats: Vf2SearchStats::default(),
    };
    search.search();
    (search.mappings, search.stats)
}

/// Computes the VF2++ node visitation order for one graph.
///
/// High-degree roots are visited first, and nodes adjacent to the growing order
/// are preferred within each breadth level. The returned order is deterministic
/// for a fixed graph node order.
fn vf2pp_order(graph: &Vf2Graph) -> Vec<usize> {
    // VF2++ starts with high-degree roots and then prioritizes nodes already
    // connected to the growing order. This exposes constraints early and
    // reduces backtracking for layout interaction graphs.
    let node_count = graph.node_count();
    let degrees = (0..node_count)
        .map(|node| graph.neighbors(NodeIndex::new(node)).count())
        .collect::<Vec<_>>();
    let mut connected_to_order = vec![0usize; node_count];
    let mut seen = vec![false; node_count];
    let mut order = Vec::with_capacity(node_count);

    let mut roots = (0..node_count).collect::<Vec<_>>();
    roots.sort_by_key(|node| Reverse((degrees[*node], Reverse(*node))));

    for root in roots {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut next_level = vec![root];
        while !next_level.is_empty() {
            for index in 0..next_level.len() {
                let best_offset = next_level[index..]
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, node)| {
                        (connected_to_order[**node], degrees[**node], Reverse(**node))
                    })
                    .map(|(offset, _)| offset)
                    .expect("level slice is non-empty");
                next_level.swap(index, index + best_offset);
                let node = next_level[index];
                order.push(node);
                for neighbor in graph.neighbors(NodeIndex::new(node)) {
                    connected_to_order[neighbor.index()] += 1;
                }
            }

            let this_level = next_level;
            next_level = Vec::new();
            for node in this_level {
                for neighbor in graph.neighbors(NodeIndex::new(node)) {
                    let neighbor = neighbor.index();
                    if !seen[neighbor] {
                        seen[neighbor] = true;
                        next_level.push(neighbor);
                    }
                }
            }
        }
    }
    order
}

#[derive(Debug, Clone)]
struct GraphState {
    /// Dense adjacency matrix for constant-time edge checks.
    adjacency: Vec<Vec<bool>>,
    /// Sorted-by-construction neighbor lists from the petgraph node order.
    neighbors: Vec<Vec<usize>>,
    /// Degree cache used by feasibility pruning.
    degrees: Vec<usize>,
    /// Mapping from this graph's nodes to the opposite graph's nodes.
    mapping: Vec<Option<usize>>,
    /// Generation in which each node first became adjacent to the mapping.
    neighbor_since: Vec<Option<usize>>,
    /// Number of unmapped nodes adjacent to the current partial mapping.
    unmapped_frontier_count: usize,
    /// Current search depth, also used to undo frontier updates precisely.
    generation: usize,
}

impl GraphState {
    /// Builds dense search state from a sparse graph.
    ///
    /// The adjacency matrix supports constant-time feasibility checks, while
    /// neighbor lists and degree caches drive frontier and lookahead pruning.
    fn new(graph: &Vf2Graph) -> Self {
        let node_count = graph.node_count();
        let mut adjacency = vec![vec![false; node_count]; node_count];
        let mut neighbors = vec![Vec::new(); node_count];
        for edge in graph.edge_references() {
            let source = edge.source().index();
            let target = edge.target().index();
            if !adjacency[source][target] {
                adjacency[source][target] = true;
                adjacency[target][source] = true;
                neighbors[source].push(target);
                neighbors[target].push(source);
            }
        }
        let degrees = neighbors.iter().map(Vec::len).collect();

        Self {
            adjacency,
            neighbors,
            degrees,
            mapping: vec![None; node_count],
            neighbor_since: vec![None; node_count],
            unmapped_frontier_count: 0,
            generation: 0,
        }
    }

    /// Adds one mapping pair and updates frontier bookkeeping for this depth.
    ///
    /// `ours` is a node in this graph and `theirs` is the corresponding node in
    /// the opposite graph. The mapping is assumed to have passed feasibility
    /// checks before this method is called.
    fn push_mapping(&mut self, ours: usize, theirs: usize) {
        // Frontier bookkeeping is generation-scoped: pop_mapping can remove
        // exactly the neighbor marks introduced at this depth.
        self.generation += 1;
        debug_assert!(self.mapping[ours].is_none());
        if self.neighbor_since[ours].is_some() {
            self.unmapped_frontier_count -= 1;
        }
        self.mapping[ours] = Some(theirs);
        for neighbor in self.neighbors[ours].iter().copied() {
            if self.neighbor_since[neighbor].is_none() {
                self.neighbor_since[neighbor] = Some(self.generation);
                if self.mapping[neighbor].is_none() {
                    self.unmapped_frontier_count += 1;
                }
            }
        }
    }

    /// Removes the most recently pushed mapping for `ours`.
    ///
    /// Frontier marks created at the current generation are rolled back, while
    /// marks inherited from earlier search depths are preserved.
    fn pop_mapping(&mut self, ours: usize) {
        // Undo only marks created by the matching push_mapping call. Older
        // frontier marks must survive because they belong to shallower choices.
        for neighbor in self.neighbors[ours].iter().copied() {
            if self.neighbor_since[neighbor] == Some(self.generation) {
                self.neighbor_since[neighbor] = None;
                if self.mapping[neighbor].is_none() {
                    self.unmapped_frontier_count -= 1;
                }
            }
        }
        self.mapping[ours] = None;
        if self.neighbor_since[ours].is_some() {
            self.unmapped_frontier_count += 1;
        }
        self.generation -= 1;
    }
}

struct Search {
    needle: GraphState,
    haystack: GraphState,
    needle_order: Vec<usize>,
    haystack_order: Vec<usize>,
    config: Vf2SearchConfig,
    mappings: Vec<Vec<usize>>,
    stats: Vf2SearchStats,
}

impl Search {
    /// Recursively extends the partial mapping until limits or completion.
    ///
    /// Complete mappings are appended to `self.mappings`. Search stops when the
    /// candidate limit is met or when the optional call limit is reached.
    fn search(&mut self) {
        if self.mappings.len() >= self.config.candidate_limit || self.stats.stopped_by_call_limit {
            return;
        }

        if self.needle.generation == self.needle.mapping.len() {
            self.stats.candidates_evaluated += 1;
            self.mappings.push(
                self.needle
                    .mapping
                    .iter()
                    .map(|mapped| mapped.expect("complete mappings have no holes"))
                    .collect(),
            );
            return;
        }

        let Some((needle_node, needle_kind)) = self.next_needle_candidate() else {
            return;
        };

        for haystack_index in 0..self.haystack_order.len() {
            let haystack_node = self.haystack_order[haystack_index];
            if self.haystack.mapping[haystack_node].is_some()
                || (needle_kind == NeighborKind::Frontier
                    && self.haystack.neighbor_since[haystack_node].is_none())
            {
                continue;
            }
            if self.reached_call_limit() {
                return;
            }
            self.stats.calls += 1;

            if !self.is_feasible(needle_node, haystack_node) {
                continue;
            }

            // Extend both directions of the mapping. The frontier-count check
            // is the VF2 lookahead condition that prevents the needle frontier
            // from outgrowing the available haystack frontier.
            self.needle.push_mapping(needle_node, haystack_node);
            self.haystack.push_mapping(haystack_node, needle_node);
            if self.needle.unmapped_frontier_count <= self.haystack.unmapped_frontier_count {
                self.search();
            }
            self.haystack.pop_mapping(haystack_node);
            self.needle.pop_mapping(needle_node);

            if self.mappings.len() >= self.config.candidate_limit
                || self.stats.stopped_by_call_limit
            {
                return;
            }
        }
    }

    /// Records and reports whether another extension would exceed call limit.
    fn reached_call_limit(&mut self) -> bool {
        if let Some(limit) = self.config.call_limit
            && self.stats.calls >= limit
        {
            self.stats.stopped_by_call_limit = true;
            return true;
        }
        false
    }

    /// Chooses the next unmapped needle node.
    ///
    /// Frontier nodes are preferred because their compatibility is constrained
    /// by already-mapped neighbors. If no frontier node exists, the next
    /// isolated or disconnected component root is returned.
    fn next_needle_candidate(&self) -> Option<(usize, NeighborKind)> {
        // Prefer frontier nodes because they are constrained by the existing
        // partial mapping. Isolated/unreached nodes are delayed until no
        // frontier nodes remain.
        let mut isolated = None;
        for node in self.needle_order.iter().copied() {
            if self.needle.mapping[node].is_some() {
                continue;
            }
            if self.needle.neighbor_since[node].is_some() {
                return Some((node, NeighborKind::Frontier));
            }
            isolated.get_or_insert(node);
        }
        isolated.map(|node| (node, NeighborKind::Neither))
    }

    /// Checks whether one candidate node pair can extend the mapping.
    ///
    /// This combines degree pruning, already-mapped edge consistency, and VF2
    /// lookahead for unmapped frontier neighbors.
    fn is_feasible(&self, needle_node: usize, haystack_node: usize) -> bool {
        // A haystack node with lower degree cannot realize all required needle
        // edges in a non-induced embedding.
        if self.haystack.degrees[haystack_node] < self.needle.degrees[needle_node] {
            return false;
        }

        if !self.mapped_edges_count_match(needle_node, haystack_node) {
            return false;
        }

        if !self.unmapped_existing_neighbors_feasible(needle_node, haystack_node) {
            return false;
        }

        true
    }

    /// Verifies required edges to already-mapped needle neighbors.
    ///
    /// Because the search is non-induced, extra haystack edges are allowed and
    /// do not appear in this check.
    fn mapped_edges_count_match(&self, needle_node: usize, haystack_node: usize) -> bool {
        // Every already-mapped needle neighbor must correspond to an existing
        // haystack edge. Extra haystack edges are allowed by non-induced
        // matching and therefore are not checked here.
        for needle_neighbor in self.needle.neighbors[needle_node].iter().copied() {
            let Some(haystack_neighbor) = self.needle.mapping[needle_neighbor] else {
                continue;
            };
            if !self.haystack.adjacency[haystack_node][haystack_neighbor] {
                return false;
            }
        }
        true
    }

    /// Applies the VF2 lookahead condition for unmapped frontier neighbors.
    ///
    /// The candidate haystack node must expose at least as many unmapped
    /// frontier neighbors as the needle node still needs. This avoids exploring
    /// branches that cannot satisfy future required edges.
    fn unmapped_existing_neighbors_feasible(
        &self,
        needle_node: usize,
        haystack_node: usize,
    ) -> bool {
        let needle_frontier_neighbors = self.needle.neighbors[needle_node]
            .iter()
            .filter(|neighbor| {
                self.needle.mapping[**neighbor].is_none()
                    && self.needle.neighbor_since[**neighbor].is_some()
            })
            .count();
        let haystack_frontier_neighbors = self.haystack.neighbors[haystack_node]
            .iter()
            .filter(|neighbor| {
                self.haystack.mapping[**neighbor].is_none()
                    && self.haystack.neighbor_since[**neighbor].is_some()
            })
            .count();

        needle_frontier_neighbors <= haystack_frontier_neighbors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NeighborKind {
    Neither,
    Frontier,
}
