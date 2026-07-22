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

//! Structured control-flow graph representation for circuits.
//!
//! `CircuitCFG` is a graph view of the expression-based dynamic-circuit IR.
//! It is intentionally structured: every conditional, loop, and switch header
//! has an accompanying [`ControlFlowRegion`] that records the body entry blocks,
//! merge/exit block, and outer operation metadata needed for exact round-trip
//! conversion back to [`Circuit`].
//!
//! The CFG does not accept arbitrary irreducible control flow. Non-structured
//! cycles, missing region metadata, unreachable blocks, and mismatched branch
//! edges are rejected by [`CircuitCFG::validate`]. This keeps the graph aligned
//! with [`ClassicalControlOp`] rather than becoming a separate unstructured IR.

use crate::circuit::circuit_param::{CircuitParam, ParameterValue};
use crate::circuit::gate::instruction::Instruction;
use crate::circuit::value_instruction::storage_operation_to_value;
use crate::circuit::{
    Circuit, CircuitError, ClassicalControlOp, ClassicalDataOp, ClassicalExpr, ClassicalType,
    ClassicalVar, ControlBody, ForOp, IfOp, Operation, Parameter, Qubit, SwitchCase, SwitchOp,
    ValueOperation, WhileOp,
};
use indexmap::IndexSet;
use rustworkx_core::petgraph::prelude::{EdgeIndex, NodeIndex, StableDiGraph};
use rustworkx_core::petgraph::visit::EdgeRef;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Semantic kind of an edge between two circuit basic blocks.
pub enum FlowEdge {
    /// True edge out of `if`, `while`, or `for` headers.
    TrueBranch,
    /// False edge out of `if`, `while`, or `for` headers.
    FalseBranch,
    /// Fallthrough edge for sequential jumps and structured merges.
    Unconditional,
    /// Exact-value switch case edge.
    Case(u128),
    /// Switch default edge. Present even when the source switch has no default
    /// body, so the CFG always has an explicit non-matching path.
    DefaultCase,
    /// Edge taken by a structured `break`.
    Break,
    /// Edge taken by a structured `continue`.
    Continue,
}

#[derive(Debug, Clone)]
/// Control transfer that ends a [`BasicBlock`].
///
/// Every block in a valid [`CircuitCFG`] has exactly one terminator, and its
/// outgoing edges must agree with that terminator.
pub enum Terminator {
    /// Boolean condition for `if` and `while` headers.
    Branch(ClassicalExpr),
    /// Runtime unsigned range loop header. This is kept distinct from
    /// `Branch` because `ForOp` has no lossless lowering to the current
    /// expression language.
    ForLoop {
        /// Loop variable updated for each iteration.
        var: ClassicalVar,
        /// Inclusive initial value.
        start: ClassicalExpr,
        /// Exclusive upper bound.
        stop: ClassicalExpr,
        /// Non-zero iteration increment.
        step: ClassicalExpr,
    },
    /// Unsigned exact-value multi-way branch.
    Switch(ClassicalExpr),
    /// Unconditional structured fallthrough.
    Jump(NodeIndex),
    /// Exit the nearest loop or switch region.
    Break(NodeIndex),
    /// Advance the nearest loop region.
    Continue(NodeIndex),
    /// End of circuit execution.
    Return,
}

/// Operation fields that live on the outer `Instruction::ClassicalControl`
/// operation, not inside the expanded body blocks.
#[derive(Debug, Clone)]
pub struct OperationMetadata {
    /// Qubits attached to the outer control-flow operation.
    pub qubits: SmallVec<[Qubit; 3]>,
    /// Parameters attached to the outer control-flow operation.
    pub params: SmallVec<[CircuitParam; 1]>,
    /// Optional diagnostic label of the outer operation.
    pub label: Option<Box<str>>,
}

impl OperationMetadata {
    fn from_operation(operation: &Operation) -> Self {
        Self {
            qubits: operation.qubits.clone(),
            params: operation.params.clone(),
            label: operation.label.clone(),
        }
    }
}

#[derive(Debug, Clone)]
/// A switch match value and the entry block of its body.
pub struct SwitchRegionCase {
    /// Exact unsigned value matched by this case.
    pub value: u128,
    /// First block executed when the case matches.
    pub entry: NodeIndex,
}

/// Structured region owned by a control-flow header block.
///
/// Labels are deliberately not part of this metadata. Labels are diagnostic
/// text only; region metadata and graph edges define the control-flow shape.
#[derive(Debug, Clone)]
pub enum ControlFlowRegion {
    /// Structured conditional region.
    If {
        /// Entry block of the then body.
        then_entry: NodeIndex,
        /// Entry block of the else path, or the merge block when absent.
        else_entry: NodeIndex,
        /// Block where both branches rejoin.
        merge_block: NodeIndex,
        /// Whether an explicit else body existed in the source circuit.
        has_else: bool,
        /// Metadata from the outer control-flow operation.
        outer: OperationMetadata,
    },
    /// Structured while-loop region.
    While {
        /// Entry block of the loop body.
        body_entry: NodeIndex,
        /// First block executed after the loop.
        exit_block: NodeIndex,
        /// Metadata from the outer control-flow operation.
        outer: OperationMetadata,
    },
    /// Structured unsigned range-loop region.
    For {
        /// Entry block of the loop body.
        body_entry: NodeIndex,
        /// First block executed after the loop.
        exit_block: NodeIndex,
        /// Metadata from the outer control-flow operation.
        outer: OperationMetadata,
    },
    /// Structured exact-value switch region.
    Switch {
        /// Explicit cases in source order.
        cases: Vec<SwitchRegionCase>,
        /// Entry block of the default path, or the merge block when absent.
        default_entry: NodeIndex,
        /// Block where all switch paths rejoin.
        merge_block: NodeIndex,
        /// Whether an explicit default body existed in the source circuit.
        has_default: bool,
        /// Metadata from the outer control-flow operation.
        outer: OperationMetadata,
    },
}

/// Straight-line sequence of operations followed by one control transfer.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub(crate) operations: Vec<Operation>,
    pub(crate) terminator: Option<Terminator>,
    pub(crate) label: Option<String>,
}

impl BasicBlock {
    /// Creates an empty, unlabeled block without a terminator.
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            terminator: None,
            label: None,
        }
    }

    /// Sets a diagnostic label and returns the updated block.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Appends one non-control-flow storage operation.
    pub fn push_operation(&mut self, op: Operation) {
        self.operations.push(op);
    }

    /// Appends a sequence of non-control-flow storage operations.
    pub fn extend_operations(&mut self, ops: impl IntoIterator<Item = Operation>) {
        self.operations.extend(ops);
    }

    /// Replaces the block terminator.
    pub fn set_terminator(&mut self, terminator: Terminator) {
        self.terminator = Some(terminator);
    }

    /// Returns `true` when the block has neither operations nor a terminator.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty() && self.terminator.is_none()
    }

    /// Returns `true` when a terminator has been assigned.
    pub fn has_terminator(&self) -> bool {
        self.terminator.is_some()
    }

    /// Returns the number of ordinary operations, excluding the terminator.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Returns the optional diagnostic label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the block terminator, if assigned.
    pub fn terminator(&self) -> Option<&Terminator> {
        self.terminator.as_ref()
    }

    /// Returns the ordinary operations in execution order.
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Structured control-flow graph representation of a [`Circuit`].
///
/// This is not a general unstructured IR. Conditional, loop, and switch
/// headers require matching [`ControlFlowRegion`] metadata so the graph can
/// round-trip back to circuit control-flow operations.
#[derive(Debug)]
pub struct CircuitCFG {
    pub(crate) qubits: IndexSet<Qubit>,
    pub(crate) symbols: IndexSet<String>,
    pub(crate) parameters: IndexSet<Parameter>,
    pub(crate) classical_vars: Vec<ClassicalType>,
    pub(crate) classical_values: Vec<ClassicalType>,
    pub(crate) global_phase: CircuitParam,
    pub(crate) data: StableDiGraph<BasicBlock, FlowEdge>,
    pub(crate) entry_block: Option<NodeIndex>,
    pub(crate) control_flow_regions: HashMap<NodeIndex, ControlFlowRegion>,
}

impl CircuitCFG {
    /// Creates an empty CFG with densely numbered qubits `0..num_qubits`.
    pub fn new(num_qubits: usize) -> Self {
        let qubits = (0..num_qubits).map(|i| Qubit::new(i as u32)).collect();
        Self {
            qubits,
            symbols: IndexSet::new(),
            parameters: IndexSet::new(),
            classical_vars: vec![],
            classical_values: vec![],
            global_phase: CircuitParam::Fixed(0.0),
            data: StableDiGraph::new(),
            entry_block: None,
            control_flow_regions: HashMap::new(),
        }
    }

    /// Creates an empty CFG with the supplied logical qubits in insertion order.
    pub fn from_qubits(qubits: Vec<Qubit>) -> Self {
        Self {
            qubits: qubits.into_iter().collect(),
            symbols: IndexSet::new(),
            parameters: IndexSet::new(),
            classical_vars: vec![],
            classical_values: vec![],
            global_phase: CircuitParam::Fixed(0.0),
            data: StableDiGraph::new(),
            entry_block: None,
            control_flow_regions: HashMap::new(),
        }
    }

    /// Adds a block and returns its stable graph index.
    pub fn add_block(&mut self, block: BasicBlock) -> NodeIndex {
        self.data.add_node(block)
    }

    /// Adds a directed semantic edge between existing blocks.
    ///
    /// Returns `None` without mutation when either endpoint is unknown.
    pub fn add_edge(
        &mut self,
        source: NodeIndex,
        target: NodeIndex,
        flow: FlowEdge,
    ) -> Option<EdgeIndex> {
        if self.data.node_weight(source).is_none() || self.data.node_weight(target).is_none() {
            return None;
        }
        Some(self.data.add_edge(source, target, flow))
    }

    /// Returns the designated entry block.
    pub fn entry_block(&self) -> Option<NodeIndex> {
        self.entry_block
    }

    /// Sets the graph entry block.
    ///
    /// The index is checked by [`CircuitCFG::validate`], not by this setter.
    pub fn set_entry_block(&mut self, index: NodeIndex) {
        self.entry_block = Some(index);
    }

    /// Associates structured-region metadata with its header block.
    pub fn set_control_flow_region(&mut self, branch_block: NodeIndex, region: ControlFlowRegion) {
        self.control_flow_regions.insert(branch_block, region);
    }

    /// Returns structured-region metadata owned by `branch_block`.
    pub fn control_flow_region(&self, branch_block: NodeIndex) -> Option<&ControlFlowRegion> {
        self.control_flow_regions.get(&branch_block)
    }

    /// Returns `true` when the block owns a while or for region.
    pub fn is_loop_header(&self, block: NodeIndex) -> bool {
        matches!(
            self.control_flow_region(block),
            Some(ControlFlowRegion::While { .. } | ControlFlowRegion::For { .. })
        )
    }

    /// Iterates over all stable block indices and immutable blocks.
    pub fn blocks(&self) -> impl Iterator<Item = (NodeIndex, &BasicBlock)> {
        self.data.node_indices().map(|i| (i, &self.data[i]))
    }

    /// Returns a mutable block by stable graph index.
    pub fn block_mut(&mut self, index: NodeIndex) -> Option<&mut BasicBlock> {
        self.data.node_weight_mut(index)
    }

    /// Iterates over outgoing target blocks and semantic edge kinds.
    pub fn outgoing_edges(
        &self,
        source: NodeIndex,
    ) -> impl Iterator<Item = (NodeIndex, FlowEdge)> + '_ {
        self.data
            .edges(source)
            .map(|edge| (edge.target(), edge.weight().clone()))
    }

    /// Returns the number of blocks currently stored in the graph.
    pub fn num_blocks(&self) -> usize {
        self.data.node_indices().count()
    }

    /// Returns the number of logical qubits known to the CFG.
    pub fn num_qubits(&self) -> usize {
        self.qubits.len()
    }

    /// Returns logical qubits in insertion order.
    pub fn qubits(&self) -> Vec<Qubit> {
        self.qubits.iter().cloned().collect()
    }

    /// Returns the static type table for mutable classical variables.
    pub fn classical_vars(&self) -> &[ClassicalType] {
        &self.classical_vars
    }

    /// Returns the static type table for immutable classical values.
    pub fn classical_values(&self) -> &[ClassicalType] {
        &self.classical_values
    }

    /// Expands a structured circuit into a validated CFG.
    ///
    /// # Errors
    ///
    /// Returns an error if operations cannot be expanded into the supported
    /// structured representation or the generated graph fails validation.
    pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError> {
        let mut cfg = Self::from_qubits(circuit.qubits());
        cfg.symbols = circuit.symbols().clone();
        cfg.parameters = circuit.parameters().clone();
        cfg.classical_vars = circuit.classical_vars().to_vec();
        cfg.classical_values = circuit.classical_values().to_vec();
        cfg.global_phase = circuit.global_phase_param().clone();

        let entry = cfg.add_block(BasicBlock::new().with_label("entry"));
        cfg.set_entry_block(entry);

        match process_operations(
            circuit.operations(),
            &mut cfg,
            entry,
            ControlContext::default(),
        )? {
            ProcessExit::Reachable(block) => {
                if !cfg.data[block].has_terminator() {
                    cfg.data[block].set_terminator(Terminator::Return);
                }
            }
            ProcessExit::Terminated => {}
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// Checks graph, operation, parameter, edge, terminator, and region invariants.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InvalidControlFlow`] for malformed or
    /// unreachable graph structure and other [`CircuitError`] variants for
    /// invalid operations or parameter references.
    pub fn validate(&self) -> Result<(), CircuitError> {
        let entry = self.entry_block.ok_or_else(|| {
            CircuitError::InvalidControlFlow("CFG does not define an entry block".to_string())
        })?;
        self.require_block(entry, "entry block")?;
        self.validate_param(&self.global_phase, "global phase")?;

        for (node, block) in self.blocks() {
            for (index, operation) in block.operations.iter().enumerate() {
                self.validate_operation(
                    operation,
                    &format!("block {:?} operation {}", node, index),
                )?;
                if matches!(operation.instruction, Instruction::ClassicalControl(_)) {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "block {:?} contains an unexpanded classical control operation",
                        node
                    )));
                }
            }

            let terminator = block.terminator.as_ref().ok_or_else(|| {
                CircuitError::InvalidControlFlow(format!(
                    "Block '{}' (index {:?}) is missing a terminator",
                    block.label().unwrap_or("<unlabeled>"),
                    node
                ))
            })?;
            self.validate_terminator(node, block, terminator)?;
        }

        for region_node in self.control_flow_regions.keys() {
            self.require_block(*region_node, "structured region owner")?;
        }

        let mut visited = HashSet::new();
        self.parse_subgraph(entry, None, &mut visited)?;
        if visited.len() != self.num_blocks() {
            return Err(CircuitError::InvalidControlFlow(format!(
                "CFG contains {} unreachable or unconsumed block(s)",
                self.num_blocks() - visited.len()
            )));
        }

        Ok(())
    }

    /// Reconstructs a structured circuit from this CFG.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails or stored operations and
    /// parameters cannot be converted back to value-level circuit IR.
    pub fn to_circuit(&self) -> Result<Circuit, CircuitError> {
        self.validate()?;
        let entry = self.entry_block.expect("validated CFG must define entry");
        let mut visited = HashSet::new();
        let ops = self.parse_subgraph(entry, None, &mut visited)?;

        let value_ops = ops
            .into_iter()
            .map(|operation| self.value_operation(operation))
            .collect::<Result<Vec<_>, _>>()?;
        let mut circuit = Circuit::from_operations(
            self.qubits.iter().copied().collect(),
            value_ops,
            Some(self.classical_vars.clone()),
            Some(self.classical_values.clone()),
        )?;
        circuit.set_global_phase(self.global_phase_parameter()?);
        Ok(circuit)
    }

    fn validate_terminator(
        &self,
        node: NodeIndex,
        block: &BasicBlock,
        terminator: &Terminator,
    ) -> Result<(), CircuitError> {
        let outgoing: Vec<_> = self.data.edges(node).collect();
        match terminator {
            Terminator::Return => {
                if !outgoing.is_empty() {
                    return Err(self.invalid_block(
                        block,
                        node,
                        "has Return terminator with outgoing edges",
                    ));
                }
            }
            Terminator::Jump(target) => {
                self.require_block(*target, "jump target")?;
                if outgoing.len() != 1
                    || !matches!(outgoing[0].weight(), FlowEdge::Unconditional)
                    || outgoing[0].target() != *target
                {
                    return Err(self.invalid_block(block, node, "has invalid Jump edge"));
                }
            }
            Terminator::Break(target) => {
                self.require_block(*target, "break target")?;
                if outgoing.len() != 1
                    || !matches!(outgoing[0].weight(), FlowEdge::Break)
                    || outgoing[0].target() != *target
                {
                    return Err(self.invalid_block(block, node, "has invalid Break edge"));
                }
            }
            Terminator::Continue(target) => {
                self.require_block(*target, "continue target")?;
                if outgoing.len() != 1
                    || !matches!(outgoing[0].weight(), FlowEdge::Continue)
                    || outgoing[0].target() != *target
                {
                    return Err(self.invalid_block(block, node, "has invalid Continue edge"));
                }
            }
            Terminator::Branch(condition) => {
                if condition.ty() != ClassicalType::Bool {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "Branch condition in block {:?} must be Bool, got {:?}",
                        node,
                        condition.ty()
                    )));
                }
                self.validate_expr(condition, "branch condition")?;
                let (true_target, false_target) = self.branch_targets(node, block)?;
                match self.control_flow_region(node).ok_or_else(|| {
                    self.invalid_block(
                        block,
                        node,
                        "has Branch terminator but no structured region",
                    )
                })? {
                    ControlFlowRegion::If {
                        then_entry,
                        else_entry,
                        merge_block,
                        outer,
                        ..
                    } => {
                        if *then_entry != true_target || *else_entry != false_target {
                            return Err(self.invalid_block(
                                block,
                                node,
                                "has If region that does not match branch edges",
                            ));
                        }
                        self.require_block(*merge_block, "if merge block")?;
                        self.validate_outer_fields(outer, node)?;
                    }
                    ControlFlowRegion::While {
                        body_entry,
                        exit_block,
                        outer,
                    } => {
                        if *body_entry != true_target || *exit_block != false_target {
                            return Err(self.invalid_block(
                                block,
                                node,
                                "has While region that does not match branch edges",
                            ));
                        }
                        self.validate_outer_fields(outer, node)?;
                    }
                    _ => {
                        return Err(self.invalid_block(
                            block,
                            node,
                            "has Branch terminator with incompatible region",
                        ));
                    }
                }
            }
            Terminator::ForLoop {
                var,
                start,
                stop,
                step,
            } => {
                self.validate_classical_var(*var, "for loop variable")?;
                self.validate_expr(start, "for start")?;
                self.validate_expr(stop, "for stop")?;
                self.validate_expr(step, "for step")?;
                let (body_target, exit_target) = self.branch_targets(node, block)?;
                match self.control_flow_region(node).ok_or_else(|| {
                    self.invalid_block(block, node, "has For terminator but no structured region")
                })? {
                    ControlFlowRegion::For {
                        body_entry,
                        exit_block,
                        outer,
                    } => {
                        if *body_entry != body_target || *exit_block != exit_target {
                            return Err(self.invalid_block(
                                block,
                                node,
                                "has For region that does not match branch edges",
                            ));
                        }
                        self.validate_outer_fields(outer, node)?;
                    }
                    _ => {
                        return Err(self.invalid_block(
                            block,
                            node,
                            "has For terminator with incompatible region",
                        ));
                    }
                }
            }
            Terminator::Switch(target) => {
                if !matches!(target.ty(), ClassicalType::UInt(_)) {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "Switch target in block {:?} must be UInt, got {:?}",
                        node,
                        target.ty()
                    )));
                }
                self.validate_expr(target, "switch target")?;
                match self.control_flow_region(node).ok_or_else(|| {
                    self.invalid_block(
                        block,
                        node,
                        "has Switch terminator but no structured region",
                    )
                })? {
                    ControlFlowRegion::Switch {
                        cases,
                        default_entry,
                        merge_block,
                        outer,
                        ..
                    } => {
                        self.require_block(*default_entry, "switch default block")?;
                        self.require_block(*merge_block, "switch merge block")?;
                        self.validate_outer_fields(outer, node)?;
                        for case in cases {
                            self.require_block(case.entry, "switch case block")?;
                            let found = outgoing.iter().any(|edge| {
                                edge.target() == case.entry
                                    && matches!(edge.weight(), FlowEdge::Case(value) if *value == case.value)
                            });
                            if !found {
                                return Err(self.invalid_block(
                                    block,
                                    node,
                                    "is missing a switch case edge",
                                ));
                            }
                        }
                        let has_default = outgoing.iter().any(|edge| {
                            edge.target() == *default_entry
                                && matches!(edge.weight(), FlowEdge::DefaultCase)
                        });
                        if !has_default || outgoing.len() != cases.len() + 1 {
                            return Err(self.invalid_block(
                                block,
                                node,
                                "has invalid switch outgoing edges",
                            ));
                        }
                    }
                    _ => {
                        return Err(self.invalid_block(
                            block,
                            node,
                            "has Switch terminator with incompatible region",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_subgraph(
        &self,
        start_node: NodeIndex,
        stop_node: Option<NodeIndex>,
        visited: &mut HashSet<NodeIndex>,
    ) -> Result<Vec<Operation>, CircuitError> {
        let mut ops = Vec::new();
        let mut current = Some(start_node);

        while let Some(node) = current {
            if Some(node) == stop_node {
                return Ok(ops);
            }
            if !visited.insert(node) {
                return Err(CircuitError::InvalidControlFlow(format!(
                    "control flow visits block {:?} more than once outside a structured boundary",
                    node
                )));
            }

            let block = &self.data[node];
            ops.extend(block.operations.clone());
            match block.terminator.as_ref() {
                Some(Terminator::Return) => return Ok(ops),
                Some(Terminator::Jump(target)) => current = Some(*target),
                Some(Terminator::Break(_)) => {
                    ops.push(control_operation(ClassicalControlOp::Break));
                    return Ok(ops);
                }
                Some(Terminator::Continue(_)) => {
                    ops.push(control_operation(ClassicalControlOp::Continue));
                    return Ok(ops);
                }
                Some(Terminator::Branch(condition)) => match self
                    .control_flow_region(node)
                    .ok_or_else(|| {
                        CircuitError::InvalidControlFlow(format!(
                            "Branch block {:?} is missing structured region",
                            node
                        ))
                    })? {
                    ControlFlowRegion::If {
                        then_entry,
                        else_entry,
                        merge_block,
                        has_else,
                        outer,
                    } => {
                        let then_ops =
                            self.parse_subgraph(*then_entry, Some(*merge_block), visited)?;
                        let else_ops =
                            self.parse_subgraph(*else_entry, Some(*merge_block), visited)?;
                        let op = IfOp::new(
                            condition.clone(),
                            ControlBody::new(then_ops),
                            has_else.then_some(ControlBody::new(else_ops)),
                        )?;
                        ops.push(Operation {
                            instruction: Instruction::ClassicalControl(ClassicalControlOp::If(op)),
                            qubits: outer.qubits.clone(),
                            params: outer.params.clone(),
                            label: outer.label.clone(),
                        });
                        current = Some(*merge_block);
                    }
                    ControlFlowRegion::While {
                        body_entry,
                        exit_block,
                        outer,
                    } => {
                        let body_ops = self.parse_subgraph(*body_entry, Some(node), visited)?;
                        let op = WhileOp::new(condition.clone(), ControlBody::new(body_ops))?;
                        ops.push(Operation {
                            instruction: Instruction::ClassicalControl(ClassicalControlOp::While(
                                op,
                            )),
                            qubits: outer.qubits.clone(),
                            params: outer.params.clone(),
                            label: outer.label.clone(),
                        });
                        current = Some(*exit_block);
                    }
                    _ => unreachable!("validation rejects incompatible branch regions"),
                },
                Some(Terminator::ForLoop {
                    var,
                    start,
                    stop,
                    step,
                }) => match self.control_flow_region(node).ok_or_else(|| {
                    CircuitError::InvalidControlFlow(format!(
                        "For block {:?} is missing structured region",
                        node
                    ))
                })? {
                    ControlFlowRegion::For {
                        body_entry,
                        exit_block,
                        outer,
                    } => {
                        let body_ops = self.parse_subgraph(*body_entry, Some(node), visited)?;
                        let op = ForOp::new(
                            *var,
                            start.clone(),
                            stop.clone(),
                            step.clone(),
                            ControlBody::new(body_ops),
                        )?;
                        ops.push(Operation {
                            instruction: Instruction::ClassicalControl(ClassicalControlOp::For(op)),
                            qubits: outer.qubits.clone(),
                            params: outer.params.clone(),
                            label: outer.label.clone(),
                        });
                        current = Some(*exit_block);
                    }
                    _ => unreachable!("validation rejects incompatible for regions"),
                },
                Some(Terminator::Switch(target)) => {
                    match self.control_flow_region(node).ok_or_else(|| {
                        CircuitError::InvalidControlFlow(format!(
                            "Switch block {:?} is missing structured region",
                            node
                        ))
                    })? {
                        ControlFlowRegion::Switch {
                            cases,
                            default_entry,
                            merge_block,
                            has_default,
                            outer,
                        } => {
                            let mut rebuilt_cases = Vec::with_capacity(cases.len());
                            for case in cases {
                                let body_ops =
                                    self.parse_subgraph(case.entry, Some(*merge_block), visited)?;
                                rebuilt_cases
                                    .push(SwitchCase::new(case.value, ControlBody::new(body_ops)));
                            }
                            let default = if *has_default {
                                Some(ControlBody::new(self.parse_subgraph(
                                    *default_entry,
                                    Some(*merge_block),
                                    visited,
                                )?))
                            } else {
                                self.parse_subgraph(*default_entry, Some(*merge_block), visited)?;
                                None
                            };
                            let op = SwitchOp::new(target.clone(), rebuilt_cases, default)?;
                            ops.push(Operation {
                                instruction: Instruction::ClassicalControl(
                                    ClassicalControlOp::Switch(op),
                                ),
                                qubits: outer.qubits.clone(),
                                params: outer.params.clone(),
                                label: outer.label.clone(),
                            });
                            current = Some(*merge_block);
                        }
                        _ => unreachable!("validation rejects incompatible switch regions"),
                    }
                }
                None => unreachable!("validation rejects unterminated blocks"),
            }
        }

        Err(CircuitError::InvalidControlFlow(
            "structured CFG traversal ended without a terminator".to_string(),
        ))
    }

    fn value_operation(&self, operation: Operation) -> Result<ValueOperation, CircuitError> {
        storage_operation_to_value(operation, &|param| self.parameter_value(param))
    }

    fn parameter_value(&self, parameter: &CircuitParam) -> Result<ParameterValue, CircuitError> {
        match parameter {
            CircuitParam::Fixed(value) => Ok(ParameterValue::Fixed(*value)),
            CircuitParam::Index(index) => self
                .parameters
                .get_index(*index as usize)
                .cloned()
                .map(ParameterValue::Param)
                .ok_or(CircuitError::InvalidParameterIndex(*index)),
        }
    }

    fn global_phase_parameter(&self) -> Result<Parameter, CircuitError> {
        match self.global_phase {
            CircuitParam::Fixed(value) => Ok(Parameter::from(value)),
            CircuitParam::Index(index) => self
                .parameters
                .get_index(index as usize)
                .cloned()
                .ok_or(CircuitError::InvalidParameterIndex(index)),
        }
    }

    fn validate_operation(&self, operation: &Operation, context: &str) -> Result<(), CircuitError> {
        for qubit in &operation.qubits {
            if !self.qubits.contains(qubit) {
                return Err(CircuitError::InvalidControlFlow(format!(
                    "{} references unknown qubit {}",
                    context,
                    qubit.id()
                )));
            }
        }
        for parameter in &operation.params {
            self.validate_param(parameter, context)?;
        }
        if let Instruction::ClassicalData(op) = &operation.instruction {
            self.validate_classical_data_op(op, operation.qubits.len())?;
        }
        Ok(())
    }

    fn validate_classical_data_op(
        &self,
        op: &ClassicalDataOp,
        qubit_count: usize,
    ) -> Result<(), CircuitError> {
        match op {
            ClassicalDataOp::Store { target, value } => {
                self.validate_classical_var(*target, "store target")?;
                if qubit_count != 0 {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "store operation has {} qubits",
                        qubit_count
                    )));
                }
                self.validate_expr(value, "store value")?;
                if target.ty() != value.ty() {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "store target type {:?} does not match value type {:?}",
                        target.ty(),
                        value.ty()
                    )));
                }
            }
            ClassicalDataOp::MeasureBit { result } => {
                self.validate_classical_value(*result, "measure_bit result")?;
                if qubit_count != 1 {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "measure_bit operation has {} qubits",
                        qubit_count
                    )));
                }
            }
            ClassicalDataOp::MeasureBits { result } => {
                self.validate_classical_value(*result, "measure_bits result")?;
                if result.ty().measurement_width() != Some(qubit_count as u32) {
                    return Err(CircuitError::InvalidControlFlow(format!(
                        "measure_bits result type {:?} does not match {} qubits",
                        result.ty(),
                        qubit_count
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_outer_fields(
        &self,
        outer: &OperationMetadata,
        node: NodeIndex,
    ) -> Result<(), CircuitError> {
        for qubit in &outer.qubits {
            if !self.qubits.contains(qubit) {
                return Err(CircuitError::InvalidControlFlow(format!(
                    "control operation at block {:?} references unknown qubit {}",
                    node,
                    qubit.id()
                )));
            }
        }
        for param in &outer.params {
            self.validate_param(param, &format!("control operation at block {:?}", node))?;
        }
        Ok(())
    }

    fn validate_expr(&self, expr: &ClassicalExpr, context: &str) -> Result<(), CircuitError> {
        for var in expr.vars() {
            self.validate_classical_var(var, context)?;
        }
        for value in expr.values() {
            self.validate_classical_value(value, context)?;
        }
        Ok(())
    }

    fn validate_classical_var(&self, var: ClassicalVar, context: &str) -> Result<(), CircuitError> {
        match self.classical_vars.get(var.id() as usize) {
            Some(ty) if *ty == var.ty() => Ok(()),
            Some(ty) => Err(CircuitError::InvalidControlFlow(format!(
                "{} references classical var {} with type {:?}, got {:?}",
                context,
                var.id(),
                ty,
                var.ty()
            ))),
            None => Err(CircuitError::InvalidControlFlow(format!(
                "{} references unknown classical var {}",
                context,
                var.id()
            ))),
        }
    }

    fn validate_classical_value(
        &self,
        value: crate::circuit::ClassicalValue,
        context: &str,
    ) -> Result<(), CircuitError> {
        match self.classical_values.get(value.index() as usize) {
            Some(ty) if *ty == value.ty() => Ok(()),
            Some(ty) => Err(CircuitError::InvalidControlFlow(format!(
                "{} references classical value {} with type {:?}, got {:?}",
                context,
                value.index(),
                ty,
                value.ty()
            ))),
            None => Err(CircuitError::InvalidControlFlow(format!(
                "{} references unknown classical value {}",
                context,
                value.index()
            ))),
        }
    }

    fn validate_param(&self, parameter: &CircuitParam, context: &str) -> Result<(), CircuitError> {
        if let CircuitParam::Index(index) = parameter
            && self.parameters.get_index(*index as usize).is_none()
        {
            return Err(CircuitError::InvalidControlFlow(format!(
                "{} references missing parameter index {}",
                context, index
            )));
        }
        Ok(())
    }

    fn require_block(&self, node: NodeIndex, context: &str) -> Result<(), CircuitError> {
        if self.data.node_weight(node).is_none() {
            return Err(CircuitError::InvalidControlFlow(format!(
                "{} {:?} does not exist in the CFG",
                context, node
            )));
        }
        Ok(())
    }

    fn branch_targets(
        &self,
        node: NodeIndex,
        block: &BasicBlock,
    ) -> Result<(NodeIndex, NodeIndex), CircuitError> {
        let outgoing: Vec<_> = self.data.edges(node).collect();
        let true_targets: Vec<_> = outgoing
            .iter()
            .filter(|edge| matches!(edge.weight(), FlowEdge::TrueBranch))
            .map(|edge| edge.target())
            .collect();
        let false_targets: Vec<_> = outgoing
            .iter()
            .filter(|edge| matches!(edge.weight(), FlowEdge::FalseBranch))
            .map(|edge| edge.target())
            .collect();
        if true_targets.is_empty() {
            return Err(self.invalid_block(block, node, "is missing a TrueBranch edge"));
        }
        if false_targets.is_empty() {
            return Err(self.invalid_block(block, node, "is missing a FalseBranch edge"));
        }
        if true_targets.len() != 1 || false_targets.len() != 1 || outgoing.len() != 2 {
            return Err(self.invalid_block(
                block,
                node,
                "must have exactly one TrueBranch edge and one FalseBranch edge",
            ));
        }
        Ok((true_targets[0], false_targets[0]))
    }

    fn invalid_block(&self, block: &BasicBlock, node: NodeIndex, message: &str) -> CircuitError {
        CircuitError::InvalidControlFlow(format!(
            "Block '{}' (index {:?}) {}",
            block.label().unwrap_or("<unlabeled>"),
            node,
            message
        ))
    }
}

#[derive(Clone, Copy, Default)]
struct ControlContext {
    break_target: Option<NodeIndex>,
    continue_target: Option<NodeIndex>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessExit {
    Reachable(NodeIndex),
    Terminated,
}

fn process_operations(
    operations: &[Operation],
    cfg: &mut CircuitCFG,
    mut current_block: NodeIndex,
    context: ControlContext,
) -> Result<ProcessExit, CircuitError> {
    for (idx, op) in operations.iter().enumerate() {
        match &op.instruction {
            Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) => {
                let then_entry =
                    cfg.add_block(BasicBlock::new().with_label(format!("if_then_{idx}")));
                let else_entry =
                    cfg.add_block(BasicBlock::new().with_label(format!("if_else_{idx}")));
                let merge_block =
                    cfg.add_block(BasicBlock::new().with_label(format!("if_merge_{idx}")));

                cfg.data[current_block]
                    .set_terminator(Terminator::Branch(if_op.condition().clone()));
                cfg.add_edge(current_block, then_entry, FlowEdge::TrueBranch);
                cfg.add_edge(current_block, else_entry, FlowEdge::FalseBranch);

                let then_exit =
                    process_operations(if_op.then_body().operations(), cfg, then_entry, context)?;
                let else_exit = if let Some(else_body) = if_op.else_body() {
                    process_operations(else_body.operations(), cfg, else_entry, context)?
                } else {
                    ProcessExit::Reachable(else_entry)
                };

                connect_to_merge(cfg, then_exit, merge_block)?;
                connect_to_merge(cfg, else_exit, merge_block)?;
                cfg.set_control_flow_region(
                    current_block,
                    ControlFlowRegion::If {
                        then_entry,
                        else_entry,
                        merge_block,
                        has_else: if_op.else_body().is_some(),
                        outer: OperationMetadata::from_operation(op),
                    },
                );
                current_block = merge_block;
            }
            Instruction::ClassicalControl(ClassicalControlOp::While(while_op)) => {
                let header =
                    cfg.add_block(BasicBlock::new().with_label(format!("while_cond_{idx}")));
                cfg.data[current_block].set_terminator(Terminator::Jump(header));
                cfg.add_edge(current_block, header, FlowEdge::Unconditional);

                let body_entry =
                    cfg.add_block(BasicBlock::new().with_label(format!("while_body_{idx}")));
                let exit_block =
                    cfg.add_block(BasicBlock::new().with_label(format!("while_exit_{idx}")));
                cfg.data[header].set_terminator(Terminator::Branch(while_op.condition().clone()));
                cfg.add_edge(header, body_entry, FlowEdge::TrueBranch);
                cfg.add_edge(header, exit_block, FlowEdge::FalseBranch);

                let body_exit = process_operations(
                    while_op.body().operations(),
                    cfg,
                    body_entry,
                    ControlContext {
                        break_target: Some(exit_block),
                        continue_target: Some(header),
                    },
                )?;
                connect_to_merge(cfg, body_exit, header)?;
                cfg.set_control_flow_region(
                    header,
                    ControlFlowRegion::While {
                        body_entry,
                        exit_block,
                        outer: OperationMetadata::from_operation(op),
                    },
                );
                current_block = exit_block;
            }
            Instruction::ClassicalControl(ClassicalControlOp::For(for_op)) => {
                let header =
                    cfg.add_block(BasicBlock::new().with_label(format!("for_header_{idx}")));
                cfg.data[current_block].set_terminator(Terminator::Jump(header));
                cfg.add_edge(current_block, header, FlowEdge::Unconditional);

                let body_entry =
                    cfg.add_block(BasicBlock::new().with_label(format!("for_body_{idx}")));
                let exit_block =
                    cfg.add_block(BasicBlock::new().with_label(format!("for_exit_{idx}")));
                cfg.data[header].set_terminator(Terminator::ForLoop {
                    var: for_op.var(),
                    start: for_op.start().clone(),
                    stop: for_op.stop().clone(),
                    step: for_op.step().clone(),
                });
                cfg.add_edge(header, body_entry, FlowEdge::TrueBranch);
                cfg.add_edge(header, exit_block, FlowEdge::FalseBranch);

                let body_exit = process_operations(
                    for_op.body().operations(),
                    cfg,
                    body_entry,
                    ControlContext {
                        break_target: Some(exit_block),
                        continue_target: Some(header),
                    },
                )?;
                connect_to_merge(cfg, body_exit, header)?;
                cfg.set_control_flow_region(
                    header,
                    ControlFlowRegion::For {
                        body_entry,
                        exit_block,
                        outer: OperationMetadata::from_operation(op),
                    },
                );
                current_block = exit_block;
            }
            Instruction::ClassicalControl(ClassicalControlOp::Switch(switch_op)) => {
                let merge_block =
                    cfg.add_block(BasicBlock::new().with_label(format!("switch_merge_{idx}")));
                cfg.data[current_block]
                    .set_terminator(Terminator::Switch(switch_op.target().clone()));

                let mut cases = Vec::with_capacity(switch_op.cases().len());
                let switch_context = ControlContext {
                    break_target: Some(merge_block),
                    continue_target: context.continue_target,
                };
                for case in switch_op.cases() {
                    let entry = cfg.add_block(BasicBlock::new().with_label(format!(
                        "switch_case_{}_{}",
                        idx,
                        case.value()
                    )));
                    cfg.add_edge(current_block, entry, FlowEdge::Case(case.value()));
                    let case_exit =
                        process_operations(case.body().operations(), cfg, entry, switch_context)?;
                    connect_to_merge(cfg, case_exit, merge_block)?;
                    cases.push(SwitchRegionCase {
                        value: case.value(),
                        entry,
                    });
                }

                let default_entry =
                    cfg.add_block(BasicBlock::new().with_label(format!("switch_default_{idx}")));
                cfg.add_edge(current_block, default_entry, FlowEdge::DefaultCase);
                let has_default = switch_op.default().is_some();
                let default_exit = if let Some(default) = switch_op.default() {
                    process_operations(default.operations(), cfg, default_entry, switch_context)?
                } else {
                    ProcessExit::Reachable(default_entry)
                };
                connect_to_merge(cfg, default_exit, merge_block)?;
                cfg.set_control_flow_region(
                    current_block,
                    ControlFlowRegion::Switch {
                        cases,
                        default_entry,
                        merge_block,
                        has_default,
                        outer: OperationMetadata::from_operation(op),
                    },
                );
                current_block = merge_block;
            }
            Instruction::ClassicalControl(ClassicalControlOp::Break) => {
                let target = context.break_target.ok_or_else(|| {
                    CircuitError::InvalidControlFlow(
                        "break appears outside a loop or switch".to_string(),
                    )
                })?;
                cfg.data[current_block].set_terminator(Terminator::Break(target));
                cfg.add_edge(current_block, target, FlowEdge::Break);
                return require_terminal_control(idx, operations.len());
            }
            Instruction::ClassicalControl(ClassicalControlOp::Continue) => {
                let target = context.continue_target.ok_or_else(|| {
                    CircuitError::InvalidControlFlow("continue appears outside a loop".to_string())
                })?;
                cfg.data[current_block].set_terminator(Terminator::Continue(target));
                cfg.add_edge(current_block, target, FlowEdge::Continue);
                return require_terminal_control(idx, operations.len());
            }
            _ => cfg.data[current_block].push_operation(op.clone()),
        }
    }
    Ok(ProcessExit::Reachable(current_block))
}

fn connect_to_merge(
    cfg: &mut CircuitCFG,
    source: ProcessExit,
    target: NodeIndex,
) -> Result<(), CircuitError> {
    if let ProcessExit::Reachable(source) = source {
        cfg.data[source].set_terminator(Terminator::Jump(target));
        cfg.add_edge(source, target, FlowEdge::Unconditional);
    }
    Ok(())
}

fn require_terminal_control(
    index: usize,
    operation_count: usize,
) -> Result<ProcessExit, CircuitError> {
    if index + 1 != operation_count {
        return Err(CircuitError::InvalidControlFlow(
            "break/continue must be the final operation in its control-flow body".to_string(),
        ));
    }
    Ok(ProcessExit::Terminated)
}

fn control_operation(op: ClassicalControlOp) -> Operation {
    Operation {
        instruction: Instruction::ClassicalControl(op),
        qubits: SmallVec::new(),
        params: SmallVec::new(),
        label: None,
    }
}

#[cfg(test)]
#[path = "./cfg_test.rs"]
mod cfg_test;
