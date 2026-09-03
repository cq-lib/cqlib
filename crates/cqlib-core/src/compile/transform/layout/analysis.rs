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

//! Circuit interaction analysis used by layout methods.
//!
//! The analyzer extracts the information that initial-layout algorithms need:
//! logical qubits in circuit order and a deterministic weighted graph of
//! two-qubit interactions. Single-qubit operations, classical data operations,
//! directives, and delays do not constrain layout and are ignored.
//!
//! Structured classical-control operations are scanned recursively. This is a
//! static compiler analysis: a gate in a branch or loop body contributes once
//! to the layout model, regardless of whether that path is taken at runtime.

use crate::circuit::{Circuit, ClassicalControlOp, Instruction, Operation, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::device::LogicalQubit;
use std::collections::BTreeMap;

/// Layout-relevant summary of a circuit.
///
/// The analysis intentionally contains no physical-device information. It can
/// be reused across multiple target devices or layout algorithms.
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitLayoutAnalysis {
    /// Logical qubits in the same order as the source circuit.
    pub logical_qubits: Vec<LogicalQubit>,
    /// Weighted two-qubit interaction graph extracted from operations.
    pub interactions: InteractionGraph,
}

/// One weighted logical interaction between two logical qubits.
///
/// Endpoints are stored in sorted order so interaction lookup and algorithm
/// tie-breaks remain deterministic. The two directed weights preserve the
/// observed operation direction for later coupling-direction scoring.
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    /// Lower-sorted logical qubit endpoint.
    pub left: LogicalQubit,
    /// Higher-sorted logical qubit endpoint.
    pub right: LogicalQubit,
    /// Total interaction weight accumulated for this unordered pair.
    pub weight: f64,
    /// Weight observed in operation order `left -> right`.
    pub directed_weight_left_to_right: f64,
    /// Weight observed in operation order `right -> left`.
    pub directed_weight_right_to_left: f64,
    /// First operation order at which this unordered interaction was seen.
    ///
    /// Algorithms use this as a stable tie-breaker after interaction weight.
    pub first_seen_order: usize,
}

/// Weighted logical interaction graph.
///
/// The graph stores one edge per unordered logical-qubit pair. Edges are kept
/// sorted by endpoint, which makes downstream algorithms reproducible without
/// relying on hash-map iteration order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InteractionGraph {
    interactions: Vec<Interaction>,
    gate_contributions: Vec<Vec<GateInteraction>>,
    temporal_weights: Vec<[f64; 3]>,
}

/// Gate-specific weights for one unordered logical-qubit interaction.
///
/// This remains compiler-internal so the public [`Interaction`] summary keeps
/// its existing shape and direction-weight contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GateInteraction {
    pub(super) gate: StandardGate,
    pub(super) left_to_right_weight: f64,
    pub(super) right_to_left_weight: f64,
}

impl InteractionGraph {
    /// Creates an empty interaction graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all interactions in deterministic endpoint order.
    pub fn interactions(&self) -> &[Interaction] {
        &self.interactions
    }

    /// Returns gate-specific contributions for the interaction at `index`.
    pub(super) fn gate_contributions(&self, index: usize) -> &[GateInteraction] {
        debug_assert_eq!(self.interactions.len(), self.gate_contributions.len());
        self.gate_contributions
            .get(index)
            .expect("interaction and gate-contribution slots must remain aligned")
            .as_slice()
    }

    pub(super) fn temporal_weights(&self, index: usize) -> [f64; 3] {
        debug_assert_eq!(self.interactions.len(), self.temporal_weights.len());
        self.temporal_weights
            .get(index)
            .copied()
            .unwrap_or([0.0; 3])
    }

    #[cfg(test)]
    pub(super) fn contribution_slot_count(&self) -> usize {
        self.gate_contributions.len()
    }

    /// Returns whether no two-qubit interactions were observed.
    pub fn is_empty(&self) -> bool {
        self.interactions.is_empty()
    }

    /// Returns the number of logical interaction edges.
    pub fn len(&self) -> usize {
        self.interactions.len()
    }

    /// Returns per-logical-qubit activity weight from incident interactions.
    ///
    /// Activity is used to place busier logical qubits earlier, and to scale
    /// readout-error scoring when calibration data is available.
    pub fn logical_activity(&self) -> BTreeMap<LogicalQubit, f64> {
        let mut activity = BTreeMap::new();
        for interaction in &self.interactions {
            *activity.entry(interaction.left).or_insert(0.0) += interaction.weight;
            *activity.entry(interaction.right).or_insert(0.0) += interaction.weight;
        }
        activity
    }

    /// Adds one observed two-qubit operation to the deterministic graph.
    ///
    /// Endpoints are canonicalized into sorted order while preserving the
    /// operation direction in the directed weight fields.
    fn add_operation_interaction(
        &mut self,
        first: LogicalQubit,
        second: LogicalQubit,
        gate: Option<StandardGate>,
        weight: f64,
        order: usize,
        temporal_bucket: usize,
    ) {
        let (left, right, left_to_right) = if first <= second {
            (first, second, true)
        } else {
            (second, first, false)
        };

        match self
            .interactions
            .binary_search_by_key(&(left, right), |i| (i.left, i.right))
        {
            Ok(index) => {
                let interaction = &mut self.interactions[index];
                interaction.weight += weight;
                if left_to_right {
                    interaction.directed_weight_left_to_right += weight;
                } else {
                    interaction.directed_weight_right_to_left += weight;
                }
                interaction.first_seen_order = interaction.first_seen_order.min(order);
                if let Some(gate) = gate {
                    add_gate_contribution(
                        &mut self.gate_contributions[index],
                        gate,
                        left_to_right,
                        weight,
                    );
                }
                self.temporal_weights[index][temporal_bucket] += weight;
            }
            Err(index) => {
                self.interactions.insert(
                    index,
                    Interaction {
                        left,
                        right,
                        weight,
                        directed_weight_left_to_right: if left_to_right { weight } else { 0.0 },
                        directed_weight_right_to_left: if left_to_right { 0.0 } else { weight },
                        first_seen_order: order,
                    },
                );
                let mut contributions = Vec::new();
                if let Some(gate) = gate {
                    add_gate_contribution(&mut contributions, gate, left_to_right, weight);
                }
                self.gate_contributions.insert(index, contributions);
                let mut temporal = [0.0; 3];
                temporal[temporal_bucket] = weight;
                self.temporal_weights.insert(index, temporal);
            }
        }
        debug_assert_eq!(self.interactions.len(), self.gate_contributions.len());
        debug_assert_eq!(self.interactions.len(), self.temporal_weights.len());
    }
}

fn add_gate_contribution(
    contributions: &mut Vec<GateInteraction>,
    gate: StandardGate,
    left_to_right: bool,
    weight: f64,
) {
    if let Some(contribution) = contributions.iter_mut().find(|item| item.gate == gate) {
        if left_to_right {
            contribution.left_to_right_weight += weight;
        } else {
            contribution.right_to_left_weight += weight;
        }
        return;
    }

    contributions.push(GateInteraction {
        gate,
        left_to_right_weight: if left_to_right { weight } else { 0.0 },
        right_to_left_weight: if left_to_right { 0.0 } else { weight },
    });
}

/// Extracts layout-relevant interaction data from a circuit.
///
/// Scans the full operation tree. Structured classical-control bodies are
/// traversed recursively so mixed quantum/classical circuits still expose the
/// two-qubit interactions that drive initial-layout quality.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] when a unitary operation acts on more
/// than two qubits. Such operations must be decomposed before layout so the
/// interaction graph represents hardware-supported two-qubit constraints.
pub fn analyze_circuit_for_layout(
    circuit: &Circuit,
) -> Result<CircuitLayoutAnalysis, CompilerError> {
    let operation_count = count_layout_order_operations(circuit.operations());
    let mut analyzer = InteractionAnalyzer::new(circuit.qubits(), operation_count);
    analyzer.scan_operations(circuit.operations())?;
    Ok(analyzer.finish())
}

fn count_layout_order_operations(operations: &[Operation]) -> usize {
    operations
        .iter()
        .map(|operation| match &operation.instruction {
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    count_layout_order_operations(op.then_body().operations())
                        + op.else_body()
                            .map_or(0, |body| count_layout_order_operations(body.operations()))
                }
                ClassicalControlOp::While(op) => {
                    count_layout_order_operations(op.body().operations())
                }
                ClassicalControlOp::For(op) => {
                    count_layout_order_operations(op.body().operations())
                }
                ClassicalControlOp::Switch(op) => {
                    op.cases()
                        .iter()
                        .map(|case| count_layout_order_operations(case.body().operations()))
                        .sum::<usize>()
                        + op.default()
                            .map_or(0, |body| count_layout_order_operations(body.operations()))
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => 0,
            },
            Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => 0,
            _ => 1,
        })
        .sum()
}

struct InteractionAnalyzer {
    logical_qubits: Vec<LogicalQubit>,
    interactions: InteractionGraph,
    /// Monotonic order for quantum operations that affect layout tie-breaks.
    operation_order: usize,
    total_operation_count: usize,
}

impl InteractionAnalyzer {
    /// Creates an analyzer seeded with the circuit's logical qubit order.
    fn new(qubits: Vec<Qubit>, total_operation_count: usize) -> Self {
        Self {
            logical_qubits: qubits.into_iter().map(LogicalQubit::from_qubit).collect(),
            interactions: InteractionGraph::new(),
            operation_order: 0,
            total_operation_count,
        }
    }

    /// Scans one operation and records layout-relevant quantum interactions.
    ///
    /// Structured classical-control operations delegate to
    /// [`InteractionAnalyzer::scan_control_flow`]. Non-control operations with
    /// arity greater than two are rejected because layout requires explicit
    /// two-qubit decomposition.
    fn scan_operation(&mut self, operation: &Operation) -> Result<(), CompilerError> {
        match &operation.instruction {
            Instruction::ClassicalControl(control) => self.scan_control_flow(control),
            Instruction::ClassicalData(_) | Instruction::Directive(_) | Instruction::Delay => {
                Ok(())
            }
            _ => {
                // Layout only needs two-qubit constraints. Larger unitary
                // operations must be lowered first so their hardware
                // interaction structure is explicit.
                match operation.qubits.len() {
                    0 | 1 => {}
                    2 => {
                        let first = LogicalQubit::from_qubit(operation.qubits[0]);
                        let second = LogicalQubit::from_qubit(operation.qubits[1]);
                        self.interactions.add_operation_interaction(
                            first,
                            second,
                            match &operation.instruction {
                                Instruction::Standard(gate) if gate.num_qubits() == 2 => {
                                    Some(*gate)
                                }
                                _ => None,
                            },
                            1.0,
                            self.operation_order,
                            (self.operation_order.saturating_mul(3)
                                / self.total_operation_count.max(1))
                            .min(2),
                        );
                    }
                    arity => {
                        return Err(CompilerError::InvalidInput(format!(
                            "layout requires unitary operations with more than two qubits to be decomposed before layout; found {}-qubit operation {}",
                            arity, operation.instruction
                        )));
                    }
                }
                self.operation_order += 1;
                Ok(())
            }
        }
    }

    /// Scans a sequence of operations in source order.
    fn scan_operations(&mut self, operations: &[Operation]) -> Result<(), CompilerError> {
        for operation in operations {
            self.scan_operation(operation)?;
        }
        Ok(())
    }

    /// Recursively scans every body of one classical-control operation.
    ///
    /// This is structural analysis: each branch, loop body, or switch case is
    /// included once, without runtime probability or loop-count weighting.
    fn scan_control_flow(&mut self, control: &ClassicalControlOp) -> Result<(), CompilerError> {
        // Control flow is scanned as a static operation tree. We include every
        // body once and leave runtime path weighting to future profile-guided
        // compilation work.
        match control {
            ClassicalControlOp::If(op) => {
                self.scan_operations(op.then_body().operations())?;
                if let Some(body) = op.else_body() {
                    self.scan_operations(body.operations())?;
                }
            }
            ClassicalControlOp::While(op) => self.scan_operations(op.body().operations())?,
            ClassicalControlOp::For(op) => self.scan_operations(op.body().operations())?,
            ClassicalControlOp::Switch(op) => {
                for case in op.cases() {
                    self.scan_operations(case.body().operations())?;
                }
                if let Some(body) = op.default() {
                    self.scan_operations(body.operations())?;
                }
            }
            ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
        }
        Ok(())
    }

    /// Consumes the analyzer and returns the immutable layout analysis.
    fn finish(self) -> CircuitLayoutAnalysis {
        CircuitLayoutAnalysis {
            logical_qubits: self.logical_qubits,
            interactions: self.interactions,
        }
    }
}
