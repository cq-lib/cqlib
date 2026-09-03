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

//! DAG-backed candidate collection for two-qubit block resynthesis.
//!
//! The DAG is a pass-local dependency view. It only improves candidate
//! discovery. Dependency closure and exact source crossings provide the inputs
//! for structural validation; the selector rechecks them before requiring
//! exact 4x4 synthesis, strict cost improvement, and replacement/crossed
//! commutation validation.

use super::collector::{
    BlockBuilder, TwoQubitNumericBlock, is_block_candidate, is_fixed_numeric_standard,
    is_hard_boundary,
};
use super::commutation::{CachedCommutation, OperationView};
use super::config::TwoQubitBlockResynthesisConfig;
use crate::circuit::{CircuitDag, CircuitParam, Directive, Instruction, Operation};
use crate::compile::CompilerError;
use indexmap::IndexSet;
use rustworkx_core::petgraph::prelude::NodeIndex;
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashSet, VecDeque};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AnchorDependencyTrace {
    pub(super) observed_orders: BTreeSet<usize>,
    pub(super) adjacency: BTreeSet<(usize, usize)>,
}

pub(super) struct DagCollectionContext {
    dag: CircuitDag,
}

impl DagCollectionContext {
    pub(super) fn build(ops: &[OperationView<'_>]) -> Result<Self, CompilerError> {
        let mut qubits = IndexSet::new();
        let operations = ops
            .iter()
            .map(|view| {
                qubits.extend(view.operation.qubits.iter().copied());

                if matches!(view.operation.instruction, Instruction::Standard(_)) {
                    Operation {
                        instruction: view.operation.instruction.clone(),
                        qubits: view.operation.qubits.clone(),
                        params: view
                            .params
                            .iter()
                            .map(|param| {
                                CircuitParam::Fixed(
                                    param
                                        .evaluate(&None)
                                        .ok()
                                        .filter(|value| value.is_finite())
                                        .unwrap_or(0.0),
                                )
                            })
                            .collect::<SmallVec<[_; 1]>>(),
                        label: view.operation.label.clone(),
                    }
                } else {
                    Operation {
                        instruction: Instruction::Directive(Directive::Barrier),
                        qubits: view.operation.qubits.clone(),
                        params: SmallVec::new(),
                        label: view.operation.label.clone(),
                    }
                }
            })
            .collect::<Vec<_>>();
        let dag =
            CircuitDag::from_operations(qubits, &operations).map_err(CompilerError::Circuit)?;
        Ok(Self { dag })
    }

    pub(super) fn collect_anchor(
        &self,
        ops: &[OperationView<'_>],
        anchor: usize,
        commutation: &mut CachedCommutation,
        config: &TwoQubitBlockResynthesisConfig,
    ) -> Result<(Option<TwoQubitNumericBlock>, AnchorDependencyTrace), CompilerError> {
        let mut trace = AnchorDependencyTrace::default();
        let Some(anchor_view) = ops.get(anchor) else {
            return Err(CompilerError::InvariantViolation(format!(
                "resynthesis anchor order {anchor} is out of bounds"
            )));
        };
        if !is_two_qubit_anchor(anchor_view, config) {
            return Ok((None, trace));
        }

        let qubits = [
            anchor_view.operation.qubits[0],
            anchor_view.operation.qubits[1],
        ];
        let Some(node) = self.dag.node_for_order(anchor) else {
            return Err(CompilerError::InvariantViolation(format!(
                "resynthesis DAG is missing operation order {anchor}"
            )));
        };
        trace.observed_orders.insert(anchor);
        let mut builder = BlockBuilder::new(qubits, anchor, ops);
        self.collect_direction(
            &mut builder,
            node,
            Direction::Left,
            commutation,
            config,
            &mut trace,
        );
        self.collect_direction(
            &mut builder,
            node,
            Direction::Right,
            commutation,
            config,
            &mut trace,
        );
        if let Some(adjacency) = self.adjacency_for_orders(trace.observed_orders.iter().copied()) {
            trace.adjacency = adjacency;
        }
        let block = builder.finish();
        Ok((block.is_promising().then_some(block), trace))
    }

    pub(super) fn adjacency_for_orders(
        &self,
        orders: impl IntoIterator<Item = usize>,
    ) -> Option<BTreeSet<(usize, usize)>> {
        let mut adjacency = BTreeSet::new();
        for order in orders {
            let node = self.dag.node_for_order(order)?;
            for neighbor in self.dag.predecessors(node).chain(self.dag.successors(node)) {
                if let Some(neighbor_order) = self.dag.operation_order(neighbor) {
                    let edge = if order <= neighbor_order {
                        (order, neighbor_order)
                    } else {
                        (neighbor_order, order)
                    };
                    adjacency.insert(edge);
                }
            }
        }
        Some(adjacency)
    }
}

pub(super) fn is_two_qubit_anchor(
    view: &OperationView<'_>,
    config: &TwoQubitBlockResynthesisConfig,
) -> bool {
    !is_hard_boundary(view, config)
        && is_fixed_numeric_standard(view)
        && matches!(view.operation.instruction, Instruction::Standard(_))
        && view.operation.qubits.len() == 2
}

pub(super) fn collect_two_qubit_blocks_dag(
    ops: &[OperationView<'_>],
    commutation: &mut CachedCommutation,
    config: &TwoQubitBlockResynthesisConfig,
) -> Result<Vec<TwoQubitNumericBlock>, CompilerError> {
    let context = DagCollectionContext::build(ops)?;

    let mut blocks = Vec::new();
    for anchor in 0..ops.len() {
        let (block, _) = context.collect_anchor(ops, anchor, commutation, config)?;
        if let Some(block) = block {
            blocks.push(block);
        }
    }
    Ok(blocks)
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Left,
    Right,
}

impl DagCollectionContext {
    fn collect_direction(
        &self,
        builder: &mut BlockBuilder<'_>,
        anchor: NodeIndex,
        direction: Direction,
        commutation: &mut CachedCommutation,
        config: &TwoQubitBlockResynthesisConfig,
        trace: &mut AnchorDependencyTrace,
    ) {
        let Some(anchor_order) = self.dag.operation_order(anchor) else {
            return;
        };
        let mut frontier = VecDeque::from(self.sorted_neighbors(anchor, direction));
        let mut seen = HashSet::new();
        let mut accepted = HashSet::from([anchor_order]);
        let mut visited = 0usize;

        while visited < config.max_scan_span {
            let Some(node) = frontier.pop_front() else {
                break;
            };
            if !seen.insert(node) {
                continue;
            }
            let Some(order) = self.dag.operation_order(node) else {
                continue;
            };
            trace.observed_orders.insert(order);
            visited += 1;

            if !self.dependencies_are_accepted(node, direction, anchor_order, &accepted) {
                if matches!(direction, Direction::Left) {
                    break;
                }
                continue;
            }

            let view = &builder.ops[order];
            if is_hard_boundary(view, config) {
                if matches!(direction, Direction::Left) {
                    break;
                }
                continue;
            }

            if is_block_candidate(view, builder.qubits) {
                if builder.matched_len() >= config.max_block_ops
                    || !builder.can_add_candidate(order, commutation)
                {
                    if matches!(direction, Direction::Left) {
                        break;
                    }
                    continue;
                }
                builder.add_matched(order);
                accepted.insert(order);
                frontier.extend(self.sorted_neighbors(node, direction));
                continue;
            }

            if builder.crossed_len() >= config.max_crossed_ops
                || !builder.can_cross(order, commutation)
            {
                if matches!(direction, Direction::Left) {
                    break;
                }
                continue;
            }
            builder.add_crossed(order);
            accepted.insert(order);
            frontier.extend(self.sorted_neighbors(node, direction));
        }
    }

    /// Ensures that intervening dependencies were accepted into the block.
    fn dependencies_are_accepted(
        &self,
        node: NodeIndex,
        direction: Direction,
        anchor_order: usize,
        accepted: &HashSet<usize>,
    ) -> bool {
        let dependencies = match direction {
            Direction::Left => self.dag.successors(node).collect::<Vec<_>>(),
            Direction::Right => self.dag.predecessors(node).collect::<Vec<_>>(),
        };
        dependencies.into_iter().all(|dependency| {
            let Some(order) = self.dag.operation_order(dependency) else {
                return true;
            };
            let between_anchor_and_node = match direction {
                Direction::Left => order <= anchor_order,
                Direction::Right => order >= anchor_order,
            };
            !between_anchor_and_node || accepted.contains(&order)
        })
    }

    /// Returns operation neighbors in deterministic source order.
    fn sorted_neighbors(&self, node: NodeIndex, direction: Direction) -> Vec<NodeIndex> {
        let mut neighbors = match direction {
            Direction::Left => self.dag.predecessors(node).collect::<Vec<_>>(),
            Direction::Right => self.dag.successors(node).collect::<Vec<_>>(),
        };
        neighbors.sort_by_key(|neighbor| {
            let order = self.dag.operation_order(*neighbor).unwrap_or(usize::MAX);
            match direction {
                Direction::Left => (usize::MAX - order, neighbor.index()),
                Direction::Right => (order, neighbor.index()),
            }
        });
        neighbors
    }
}

#[cfg(test)]
#[path = "dag_collector_test.rs"]
mod dag_collector_test;
