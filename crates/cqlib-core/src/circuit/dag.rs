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

//! Operation-level dependency DAG for circuits.
//!
//! `CircuitDag` is an algorithm view of [`Circuit`]. It keeps the circuit's
//! storage IR unchanged while materializing operation dependencies induced by
//! qubits and runtime classical resources. The DAG is intentionally not a
//! commutation graph: edges mean "these operations are ordered by a shared
//! resource in the source program", not "these operations cannot commute".

use crate::circuit::circuit_param::{CircuitParam, ParameterValue};
use crate::circuit::gate::{ClassicalDataOp, Directive, Instruction};
use crate::circuit::value_instruction::storage_operation_to_value;
use crate::circuit::{
    Circuit, CircuitError, ClassicalControlOp, ClassicalExpr, ClassicalType, ClassicalValue,
    ClassicalVar, ControlBody, ForOp, IfOp, Operation, Parameter, Qubit, SwitchCase, SwitchOp,
    ValueClassicalControlOp, ValueControlBody, ValueInstruction, ValueOperation, WhileOp,
};
use indexmap::{IndexMap, IndexSet};
use rustworkx_core::dag_algo::lexicographical_topological_sort;
use rustworkx_core::petgraph::Direction;
use rustworkx_core::petgraph::algo::toposort;
use rustworkx_core::petgraph::prelude::{NodeIndex, StableDiGraph};
use rustworkx_core::petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::OnceLock;

/// Resource carried by an edge in a [`CircuitDag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DagWire {
    /// A quantum bit timeline.
    Qubit(Qubit),
    /// A mutable classical storage location.
    ClassicalVar(ClassicalVar),
    /// An immutable runtime classical value, typically a measurement result.
    ClassicalValue(ClassicalValue),
    /// Stable ordering resource for operations without a concrete data wire.
    GlobalOrder,
}

/// Node in a [`CircuitDag`].
#[derive(Debug, Clone)]
pub enum DagNode {
    /// Input sentinel for a resource timeline.
    WireIn(DagWire),
    /// Output sentinel for a resource timeline.
    WireOut(DagWire),
    /// Circuit operation node.
    Operation {
        /// Storage IR operation.
        operation: Operation,
        /// Source-order position used for deterministic traversal.
        order: usize,
    },
}

/// Recursive DAG payload for a structured control-flow operation.
#[derive(Debug, Clone)]
pub enum DagControlFlow {
    /// If/else bodies.
    If {
        then_body: Box<CircuitDag>,
        else_body: Option<Box<CircuitDag>>,
    },
    /// While body.
    While { body: Box<CircuitDag> },
    /// For body.
    For { body: Box<CircuitDag> },
    /// Switch cases and optional default body.
    Switch {
        cases: Vec<DagSwitchCase>,
        default: Option<Box<CircuitDag>>,
    },
    /// Structured break.
    Break,
    /// Structured continue.
    Continue,
}

/// One switch case body in a [`DagControlFlow::Switch`].
#[derive(Debug, Clone)]
pub struct DagSwitchCase {
    pub value: u128,
    pub body: Box<CircuitDag>,
}

/// Operation-level dependency DAG for a circuit.
#[derive(Debug, Clone)]
pub struct CircuitDag {
    qubits: IndexSet<Qubit>,
    symbols: IndexSet<String>,
    parameters: IndexSet<Parameter>,
    classical_vars: Vec<ClassicalType>,
    classical_values: Vec<ClassicalType>,
    global_phase: CircuitParam,
    graph: StableDiGraph<DagNode, DagWire>,
    wire_io: IndexMap<DagWire, [NodeIndex; 2]>,
    control_flow: HashMap<NodeIndex, DagControlFlow>,
    order_to_node: Vec<NodeIndex>,
    topological_op_nodes_cache: OnceLock<Vec<NodeIndex>>,
    node_layers_cache: OnceLock<IndexMap<NodeIndex, usize>>,
}

impl CircuitDag {
    /// Builds a dependency DAG from a circuit.
    pub fn from_circuit(circuit: &Circuit) -> Result<Self, CircuitError> {
        let mut dag = Self {
            qubits: circuit.qubits().into_iter().collect(),
            symbols: circuit.symbols().clone(),
            parameters: circuit.parameters().clone(),
            classical_vars: circuit.classical_vars().to_vec(),
            classical_values: circuit.classical_values().to_vec(),
            global_phase: circuit.global_phase_param().clone(),
            graph: StableDiGraph::new(),
            wire_io: IndexMap::new(),
            control_flow: HashMap::new(),
            order_to_node: Vec::with_capacity(circuit.operations().len()),
            topological_op_nodes_cache: OnceLock::new(),
            node_layers_cache: OnceLock::new(),
        };
        dag.initialize_wires();
        dag.add_operations(circuit.operations())?;
        dag.validate()?;
        Ok(dag)
    }

    /// Builds a dependency DAG from a self-contained operation slice.
    ///
    /// This is the narrow constructor for pass-local analysis when the caller
    /// already owns an operation stream and only needs quantum dependency
    /// structure. It deliberately does not accept parameter tables, classical
    /// declarations, or a global phase: every operation must be valid against
    /// exactly the supplied qubit set and must be independent of external
    /// circuit metadata.
    ///
    /// Use [`CircuitDag::from_circuit`] when the operation stream contains
    /// indexed parameters, classical variables/values, control-flow bodies, or
    /// any other resource that belongs to a full [`Circuit`]. Analysis passes
    /// that intentionally ignore those resources should normalize them before
    /// calling this constructor, for example by resolving parameters and
    /// replacing non-quantum operations with dependency barriers.
    ///
    /// # Errors
    ///
    /// Returns an error when `qubits` contains duplicates, an operation
    /// references a qubit outside that set, or the operation slice is not
    /// self-contained.
    pub fn from_operations(
        qubits: impl IntoIterator<Item = Qubit>,
        operations: &[Operation],
    ) -> Result<Self, CircuitError> {
        let mut qubit_set = IndexSet::new();
        for qubit in qubits {
            if !qubit_set.insert(qubit) {
                return Err(CircuitError::DuplicateQubits);
            }
        }
        let mut dag = Self {
            qubits: qubit_set,
            symbols: IndexSet::new(),
            parameters: IndexSet::new(),
            classical_vars: Vec::new(),
            classical_values: Vec::new(),
            global_phase: CircuitParam::Fixed(0.0),
            graph: StableDiGraph::new(),
            wire_io: IndexMap::new(),
            control_flow: HashMap::new(),
            order_to_node: Vec::with_capacity(operations.len()),
            topological_op_nodes_cache: OnceLock::new(),
            node_layers_cache: OnceLock::new(),
        };
        dag.initialize_wires();
        dag.add_operations(operations)?;
        dag.validate()?;
        Ok(dag)
    }

    /// Reconstructs a circuit from this DAG's deterministic topological order.
    ///
    /// # Errors
    ///
    /// Returns an error if the DAG contains a cycle, references an invalid
    /// parameter, or the reconstructed operation stream is rejected by
    /// [`Circuit::from_operations`].
    pub fn to_circuit(&self) -> Result<Circuit, CircuitError> {
        let value_ops = self
            .topological_operations()?
            .into_iter()
            .map(|operation| {
                storage_operation_to_value(operation, &|param| self.parameter_value(param))
            })
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

    /// Validates graph and metadata invariants.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph is cyclic, has invalid wire endpoints,
    /// references resources outside the circuit metadata, or has inconsistent
    /// control-flow payloads.
    pub fn validate(&self) -> Result<(), CircuitError> {
        toposort(&self.graph, None).map_err(|cycle| {
            CircuitError::InvalidDag(format!(
                "DAG contains a cycle at node {:?}",
                cycle.node_id()
            ))
        })?;
        self.validate_wire_io()?;

        let mut produced_values = HashSet::new();
        for node in self.graph.node_indices() {
            match &self.graph[node] {
                DagNode::WireIn(wire) | DagNode::WireOut(wire) => self.validate_wire(*wire)?,
                DagNode::Operation { operation, order } => {
                    if self.node_for_order(*order) != Some(node) {
                        return Err(CircuitError::InvalidDag(format!(
                            "operation order {order} is not unique"
                        )));
                    }
                    if self.order_to_node.get(*order).copied() != Some(node) {
                        return Err(CircuitError::InvalidDag(format!(
                            "operation order {order} is missing from the order index"
                        )));
                    }
                    self.validate_operation(operation)?;
                    if let Instruction::ClassicalData(op) = &operation.instruction
                        && let Some(value) = op.result()
                        && !produced_values.insert(value)
                    {
                        return Err(CircuitError::DuplicateClassicalValueDefinition {
                            index: value.index(),
                            first: "CircuitDag".to_string(),
                            second: "CircuitDag".to_string(),
                        });
                    }
                    if let Some(control) = self.control_flow.get(&node) {
                        self.validate_control_flow_payload(operation, control)?;
                    }
                }
            }
        }

        for edge in self.graph.edge_references() {
            self.validate_wire(*edge.weight())?;
        }
        Ok(())
    }

    /// Returns the number of qubits.
    pub fn num_qubits(&self) -> usize {
        self.qubits.len()
    }

    /// Returns the number of operation nodes.
    pub fn num_ops(&self) -> usize {
        self.op_nodes().count()
    }

    /// Returns true if there are no operation nodes.
    pub fn is_empty(&self) -> bool {
        self.num_ops() == 0
    }

    /// Returns circuit qubits in insertion order.
    pub fn qubits(&self) -> Vec<Qubit> {
        self.qubits.iter().copied().collect()
    }

    /// Returns the parameter table.
    pub fn parameters(&self) -> &IndexSet<Parameter> {
        &self.parameters
    }

    /// Interns a parameter into this DAG's parameter table.
    ///
    /// Mirrors [`Circuit::add_parameter`]: inserts `param` into the table when
    /// absent, records its symbolic names, and returns the stable table index
    /// plus whether the parameter was newly inserted.
    ///
    /// [`Circuit::add_parameter`]: crate::circuit::Circuit::add_parameter
    pub fn add_parameter(&mut self, param: Parameter) -> (usize, bool) {
        let (index, is_new) = self.parameters.insert_full(param.clone());
        if is_new {
            for sym in param.get_symbols() {
                self.symbols.insert(sym);
            }
        }
        (index, is_new)
    }

    /// Returns symbolic parameter names referenced by this DAG.
    pub fn symbols(&self) -> &IndexSet<String> {
        &self.symbols
    }

    /// Returns classical variable types.
    pub fn classical_vars(&self) -> &[ClassicalType] {
        &self.classical_vars
    }

    /// Returns classical value types.
    pub fn classical_values(&self) -> &[ClassicalType] {
        &self.classical_values
    }

    /// Iterates all materialized DAG wires.
    pub fn wires(&self) -> impl Iterator<Item = DagWire> + '_ {
        self.wire_io.keys().copied()
    }

    /// Returns whether `wire` has materialized endpoints in this DAG.
    pub fn has_wire(&self, wire: DagWire) -> bool {
        self.wire_io.contains_key(&wire)
    }

    /// Returns whether `wire` has no operation on its timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if `wire` is not a valid resource for this DAG or its
    /// materialized timeline is broken.
    pub fn is_wire_idle(&self, wire: DagWire) -> Result<bool, CircuitError> {
        Ok(self.nodes_on_wire(wire)?.is_empty())
    }

    /// Returns the input sentinel for `wire`.
    pub fn wire_in(&self, wire: DagWire) -> Option<NodeIndex> {
        self.wire_io.get(&wire).map(|pair| pair[0])
    }

    /// Returns the output sentinel for `wire`.
    pub fn wire_out(&self, wire: DagWire) -> Option<NodeIndex> {
        self.wire_io.get(&wire).map(|pair| pair[1])
    }

    /// Returns an operation by node index.
    pub fn operation(&self, node: NodeIndex) -> Option<&Operation> {
        match self.graph.node_weight(node) {
            Some(DagNode::Operation { operation, .. }) => Some(operation),
            _ => None,
        }
    }

    /// Returns true if `node` is an operation node.
    pub fn is_operation(&self, node: NodeIndex) -> bool {
        self.operation(node).is_some()
    }

    /// Returns the kind of `node`.
    pub fn node_kind(&self, node: NodeIndex) -> Option<&'static str> {
        match self.graph.node_weight(node) {
            Some(DagNode::WireIn(_)) => Some("wire_in"),
            Some(DagNode::WireOut(_)) => Some("wire_out"),
            Some(DagNode::Operation { .. }) => Some("operation"),
            None => None,
        }
    }

    /// Returns recursive control-flow DAGs attached to `node`, if any.
    pub fn control_flow(&self, node: NodeIndex) -> Option<&DagControlFlow> {
        self.control_flow.get(&node)
    }

    /// Iterates operation nodes in deterministic source order.
    pub fn op_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        let mut nodes = self
            .graph
            .node_indices()
            .filter(|node| matches!(self.graph[*node], DagNode::Operation { .. }))
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| self.operation_order(*node).unwrap_or(usize::MAX));
        nodes.into_iter()
    }

    /// Returns operation nodes in deterministic topological order.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph contains a cycle.
    pub fn topological_op_nodes(&self) -> Result<Vec<NodeIndex>, CircuitError> {
        Ok(self.topological_op_nodes_ref()?.clone())
    }

    /// Returns predecessor nodes.
    pub fn predecessors(&self, node: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph
            .edges_directed(node, rustworkx_core::petgraph::Incoming)
            .map(|edge| edge.source())
    }

    /// Returns successor nodes.
    pub fn successors(&self, node: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph
            .edges_directed(node, rustworkx_core::petgraph::Outgoing)
            .map(|edge| edge.target())
    }

    /// Returns operation predecessors of `node` connected through `wire`.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist or `wire` is not valid for
    /// this DAG.
    pub fn predecessors_on_wire(
        &self,
        node: NodeIndex,
        wire: DagWire,
    ) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_on_wire(node, wire, Direction::Incoming)
    }

    /// Returns operation successors of `node` connected through `wire`.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist or `wire` is not valid for
    /// this DAG.
    pub fn successors_on_wire(
        &self,
        node: NodeIndex,
        wire: DagWire,
    ) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_on_wire(node, wire, Direction::Outgoing)
    }

    /// Returns operation predecessors connected by quantum wires.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist.
    pub fn quantum_predecessors(&self, node: NodeIndex) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_matching(node, Direction::Incoming, |wire| {
            matches!(wire, DagWire::Qubit(_))
        })
    }

    /// Returns operation successors connected by quantum wires.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist.
    pub fn quantum_successors(&self, node: NodeIndex) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_matching(node, Direction::Outgoing, |wire| {
            matches!(wire, DagWire::Qubit(_))
        })
    }

    /// Returns operation predecessors connected by classical wires.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist.
    pub fn classical_predecessors(&self, node: NodeIndex) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_matching(node, Direction::Incoming, |wire| {
            matches!(wire, DagWire::ClassicalVar(_) | DagWire::ClassicalValue(_))
        })
    }

    /// Returns operation successors connected by classical wires.
    ///
    /// # Errors
    ///
    /// Returns an error if `node` does not exist.
    pub fn classical_successors(&self, node: NodeIndex) -> Result<Vec<NodeIndex>, CircuitError> {
        self.operation_neighbors_matching(node, Direction::Outgoing, |wire| {
            matches!(wire, DagWire::ClassicalVar(_) | DagWire::ClassicalValue(_))
        })
    }

    /// Returns operation nodes on `wire` in wire order.
    ///
    /// # Errors
    ///
    /// Returns an error if `wire` is invalid or the stored wire timeline is
    /// broken or branched.
    pub fn nodes_on_wire(&self, wire: DagWire) -> Result<Vec<NodeIndex>, CircuitError> {
        self.validate_wire(wire)?;
        let Some(input) = self.wire_in(wire) else {
            return Ok(Vec::new());
        };
        let output = self.wire_out(wire).ok_or_else(|| {
            CircuitError::InvalidDag(format!("wire {:?} has no output endpoint", wire))
        })?;

        let mut nodes = Vec::new();
        let mut seen = HashSet::new();
        let mut current = input;
        while current != output {
            if !seen.insert(current) {
                return Err(CircuitError::InvalidDag(format!(
                    "wire {:?} contains a cycle",
                    wire
                )));
            }
            let next = self.next_node_on_wire(current, wire)?.ok_or_else(|| {
                CircuitError::InvalidDag(format!("wire {:?} is not connected to its output", wire))
            })?;
            match self.graph.node_weight(next) {
                Some(DagNode::Operation { .. }) => nodes.push(next),
                Some(DagNode::WireOut(found)) if *found == wire => {}
                Some(_) => {
                    return Err(CircuitError::InvalidDag(format!(
                        "wire {:?} reaches an invalid node {:?}",
                        wire, next
                    )));
                }
                None => {
                    return Err(CircuitError::InvalidDag(format!(
                        "wire {:?} reaches missing node {:?}",
                        wire, next
                    )));
                }
            }
            current = next;
        }
        Ok(nodes)
    }

    /// Returns operation nodes with no operation predecessor.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph contains a cycle.
    pub fn front_layer(&self) -> Result<Vec<NodeIndex>, CircuitError> {
        Ok(self
            .topological_op_nodes_ref()?
            .iter()
            .copied()
            .filter(|node| {
                !self
                    .predecessors(*node)
                    .any(|pred| matches!(self.graph[pred], DagNode::Operation { .. }))
            })
            .collect())
    }

    /// Returns ASAP operation layers.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph contains a cycle or operation predecessor
    /// layers are inconsistent with the topological order.
    pub fn layers(&self) -> Result<Vec<Vec<NodeIndex>>, CircuitError> {
        let mut layers: Vec<Vec<NodeIndex>> = Vec::new();
        for (node, layer) in self.node_layers_ref()? {
            if layers.len() <= *layer {
                layers.resize_with(layer + 1, Vec::new);
            }
            layers[*layer].push(*node);
        }
        Ok(layers)
    }

    /// Returns the ASAP layer index for every operation node.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph contains a cycle or operation predecessor
    /// layers are inconsistent with the topological order.
    pub fn node_layers(&self) -> Result<IndexMap<NodeIndex, usize>, CircuitError> {
        Ok(self.node_layers_ref()?.clone())
    }

    /// Returns the ASAP dependency depth.
    pub fn depth(&self) -> Result<usize, CircuitError> {
        Ok(self
            .node_layers_ref()?
            .values()
            .copied()
            .max()
            .map_or(0, |layer| layer + 1))
    }

    /// Returns true if any top-level operation is structured control flow.
    pub fn has_control_flow(&self) -> bool {
        self.op_nodes().any(|node| {
            matches!(
                self.operation(node).map(|op| &op.instruction),
                Some(Instruction::ClassicalControl(_))
            )
        })
    }

    /// Returns true if a control-flow body contains structured control flow.
    pub fn has_nested_control_flow(&self) -> bool {
        self.control_flow.values().any(|control| {
            control
                .body_dags()
                .any(|body| body.has_control_flow() || body.has_nested_control_flow())
        })
    }

    /// Returns true if any top-level operation directly or recursively measures.
    pub fn has_measurement(&self) -> bool {
        self.op_nodes().any(|node| {
            self.operation(node)
                .is_some_and(|operation| operation.instruction.has_measurement())
        })
    }

    /// Counts top-level operations by instruction name.
    pub fn operation_count_by_name(&self) -> IndexMap<String, usize> {
        let mut counts = IndexMap::new();
        for node in self.op_nodes() {
            if let Some(operation) = self.operation(node) {
                let name = operation.instruction.name();
                *counts.entry(name).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Counts top-level and nested control-flow body operations by instruction name.
    pub fn operation_count_by_name_recursive(&self) -> IndexMap<String, usize> {
        let mut counts = IndexMap::new();
        self.add_operation_counts_recursive(&mut counts);
        counts
    }

    /// Collects contiguous runs on `wire` for operations accepted by `predicate`.
    ///
    /// Runs are collected in wire order. Operations for which `predicate`
    /// returns false break the current run. This method is intended for
    /// concrete qubit and classical wires. [`DagWire::GlobalOrder`] is accepted
    /// for completeness, but it only orders operations without concrete data
    /// resources and is usually not meaningful for optimization runs.
    ///
    /// # Errors
    ///
    /// Returns an error if `wire` is invalid or its materialized timeline is
    /// broken.
    pub fn collect_runs_on_wire<F>(
        &self,
        wire: DagWire,
        mut predicate: F,
    ) -> Result<Vec<Vec<NodeIndex>>, CircuitError>
    where
        F: FnMut(&Operation) -> bool,
    {
        let mut runs = Vec::new();
        let mut current = Vec::new();
        for node in self.nodes_on_wire(wire)? {
            let operation = self.operation(node).ok_or_else(|| {
                CircuitError::InvalidDag(format!("node {:?} is not an operation", node))
            })?;
            if predicate(operation) {
                current.push(node);
            } else if !current.is_empty() {
                runs.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }
        Ok(runs)
    }

    /// Collects contiguous one-qubit gate runs on every qubit wire.
    ///
    /// Non-gate operations, measurements, reset, barriers, and multi-qubit
    /// operations break a run.
    ///
    /// # Errors
    ///
    /// Returns an error if any materialized qubit timeline is broken.
    pub fn collect_1q_runs(&self) -> Result<Vec<Vec<NodeIndex>>, CircuitError> {
        let mut runs = Vec::new();
        for qubit in &self.qubits {
            runs.extend(self.collect_runs_on_wire(DagWire::Qubit(*qubit), is_one_qubit_gate)?);
        }
        Ok(runs)
    }

    /// Collects contiguous two-qubit gate runs on every qubit wire.
    ///
    /// The same two-qubit operation may appear in the runs for both of its
    /// qubit wires.
    ///
    /// # Errors
    ///
    /// Returns an error if any materialized qubit timeline is broken.
    pub fn collect_2q_runs(&self) -> Result<Vec<Vec<NodeIndex>>, CircuitError> {
        let mut runs = Vec::new();
        for qubit in &self.qubits {
            runs.extend(self.collect_runs_on_wire(DagWire::Qubit(*qubit), is_two_qubit_gate)?);
        }
        Ok(runs)
    }

    /// Appends an operation at the back of the DAG.
    pub fn apply_operation_back(
        &mut self,
        operation: Operation,
    ) -> Result<NodeIndex, CircuitError> {
        let mut operations = self.topological_operations()?;
        let order = operations.len();
        operations.push(operation);
        self.rebuild_from_operations(&operations)?;
        self.node_for_order(order)
            .ok_or_else(|| CircuitError::InvalidDag("appended operation node missing".to_string()))
    }

    /// Lowers and appends a self-contained value-level operation at the back of the DAG.
    ///
    /// # Errors
    ///
    /// Returns errors from value-instruction lowering or DAG rebuilding.
    pub fn apply_value_operation_back(
        &mut self,
        operation: ValueOperation,
    ) -> Result<NodeIndex, CircuitError> {
        let operation = self.lower_value_operation(operation)?;
        self.apply_operation_back(operation)
    }

    /// Prepends an operation at the front of the DAG.
    pub fn apply_operation_front(
        &mut self,
        operation: Operation,
    ) -> Result<NodeIndex, CircuitError> {
        let mut operations = Vec::with_capacity(self.num_ops() + 1);
        operations.push(operation);
        operations.extend(self.topological_operations()?);
        self.rebuild_from_operations(&operations)?;
        self.node_for_order(0)
            .ok_or_else(|| CircuitError::InvalidDag("prepended operation node missing".to_string()))
    }

    /// Lowers and prepends a self-contained value-level operation at the front of the DAG.
    ///
    /// # Errors
    ///
    /// Returns errors from value-instruction lowering or DAG rebuilding.
    pub fn apply_value_operation_front(
        &mut self,
        operation: ValueOperation,
    ) -> Result<NodeIndex, CircuitError> {
        let operation = self.lower_value_operation(operation)?;
        self.apply_operation_front(operation)
    }

    /// Removes one operation node.
    pub fn remove_op_node(&mut self, node: NodeIndex) -> Result<Operation, CircuitError> {
        let order = self.require_operation_order(node)?;
        let mut operations = self.topological_operations()?;
        let removed = operations.remove(order);
        self.rebuild_from_operations_unchecked(&operations)?;
        Ok(removed)
    }

    /// Replaces one operation node.
    pub fn substitute_node(
        &mut self,
        node: NodeIndex,
        operation: Operation,
    ) -> Result<(), CircuitError> {
        let order = self.require_operation_order(node)?;
        let mut operations = self.topological_operations()?;
        operations[order] = operation;
        self.rebuild_from_operations(&operations)
    }

    /// Lowers and replaces one operation node with a self-contained value-level operation.
    ///
    /// # Errors
    ///
    /// Returns errors from value-instruction lowering, node validation, or DAG rebuilding.
    pub fn substitute_value_node(
        &mut self,
        node: NodeIndex,
        operation: ValueOperation,
    ) -> Result<(), CircuitError> {
        let operation = self.lower_value_operation(operation)?;
        self.substitute_node(node, operation)
    }

    /// Replaces one operation node by another DAG.
    pub fn substitute_node_with_dag(
        &mut self,
        node: NodeIndex,
        replacement: CircuitDag,
    ) -> Result<(), CircuitError> {
        replacement.validate()?;
        let order = self.require_operation_order(node)?;
        let old = self.operation(node).ok_or_else(|| {
            CircuitError::InvalidDag(format!("node {:?} is not an operation", node))
        })?;
        let replacement_operations = self.validate_replacement(old, &replacement)?;
        let mut operations = self.topological_operations()?;
        operations.splice(order..=order, replacement_operations);
        self.rebuild_from_operations(&operations)
    }

    fn initialize_wires(&mut self) {
        let mut wires = Vec::new();
        wires.push(DagWire::GlobalOrder);
        wires.extend(self.qubits.iter().copied().map(DagWire::Qubit));
        for wire in wires {
            let input = self.graph.add_node(DagNode::WireIn(wire));
            let output = self.graph.add_node(DagNode::WireOut(wire));
            self.wire_io.insert(wire, [input, output]);
        }
    }

    fn add_operations(&mut self, operations: &[Operation]) -> Result<(), CircuitError> {
        let mut frontier = self
            .wire_io
            .iter()
            .map(|(wire, endpoints)| (*wire, endpoints[0]))
            .collect::<HashMap<_, _>>();

        for (order, operation) in operations.iter().cloned().enumerate() {
            self.validate_operation(&operation)?;
            let node = self.graph.add_node(DagNode::Operation {
                operation: operation.clone(),
                order,
            });
            self.order_to_node.push(node);
            if let Some(control) = self.build_control_flow_payload(&operation)? {
                self.control_flow.insert(node, control);
            }

            let resources = self.operation_resources(&operation);
            let mut deps = resources.reads.clone();
            deps.extend(resources.writes.iter().copied());
            if deps.is_empty() {
                deps.insert(DagWire::GlobalOrder);
            }

            for wire in deps {
                self.ensure_wire(wire)?;
                if let std::collections::hash_map::Entry::Vacant(entry) = frontier.entry(wire) {
                    let endpoints = self.wire_io.get(&wire).ok_or_else(|| {
                        CircuitError::InvalidDag(format!("missing endpoints for wire {:?}", wire))
                    })?;
                    entry.insert(endpoints[0]);
                }
                let source = frontier.get(&wire).copied().ok_or_else(|| {
                    CircuitError::InvalidDag(format!("missing frontier for wire {:?}", wire))
                })?;
                self.add_edge_once(source, node, wire);
                frontier.insert(wire, node);
            }
        }

        let output_edges = self
            .wire_io
            .iter()
            .map(|(wire, endpoints)| {
                (
                    frontier.get(wire).copied().unwrap_or(endpoints[0]),
                    endpoints[1],
                    *wire,
                )
            })
            .collect::<Vec<_>>();
        for (source, target, wire) in output_edges {
            self.add_edge_once(source, target, wire);
        }
        Ok(())
    }

    fn rebuild_from_operations(&mut self, operations: &[Operation]) -> Result<(), CircuitError> {
        self.rebuild_from_operations_with_validation(operations, true)
    }

    fn rebuild_from_operations_unchecked(
        &mut self,
        operations: &[Operation],
    ) -> Result<(), CircuitError> {
        self.rebuild_from_operations_with_validation(operations, false)
    }

    fn rebuild_from_operations_with_validation(
        &mut self,
        operations: &[Operation],
        validate: bool,
    ) -> Result<(), CircuitError> {
        let mut rebuilt = Self {
            qubits: self.qubits.clone(),
            symbols: self.symbols.clone(),
            parameters: self.parameters.clone(),
            classical_vars: self.classical_vars.clone(),
            classical_values: self.classical_values.clone(),
            global_phase: self.global_phase.clone(),
            graph: StableDiGraph::new(),
            wire_io: IndexMap::new(),
            control_flow: HashMap::new(),
            order_to_node: Vec::with_capacity(operations.len()),
            topological_op_nodes_cache: OnceLock::new(),
            node_layers_cache: OnceLock::new(),
        };
        rebuilt.initialize_wires();
        rebuilt.add_operations(operations)?;
        if validate {
            rebuilt.validate()?;
        }
        *self = rebuilt;
        Ok(())
    }

    fn build_control_flow_payload(
        &self,
        operation: &Operation,
    ) -> Result<Option<DagControlFlow>, CircuitError> {
        let Instruction::ClassicalControl(control) = &operation.instruction else {
            return Ok(None);
        };
        let build_body = |ops: &[Operation]| -> Result<Box<CircuitDag>, CircuitError> {
            let mut dag = Self {
                qubits: self.qubits.clone(),
                symbols: self.symbols.clone(),
                parameters: self.parameters.clone(),
                classical_vars: self.classical_vars.clone(),
                classical_values: self.classical_values.clone(),
                global_phase: self.global_phase.clone(),
                graph: StableDiGraph::new(),
                wire_io: IndexMap::new(),
                control_flow: HashMap::new(),
                order_to_node: Vec::with_capacity(ops.len()),
                topological_op_nodes_cache: OnceLock::new(),
                node_layers_cache: OnceLock::new(),
            };
            dag.initialize_wires();
            dag.add_operations(ops)?;
            dag.validate()?;
            Ok(Box::new(dag))
        };

        Ok(Some(match control {
            ClassicalControlOp::If(op) => DagControlFlow::If {
                then_body: build_body(op.then_body().operations())?,
                else_body: op
                    .else_body()
                    .map(|body| build_body(body.operations()))
                    .transpose()?,
            },
            ClassicalControlOp::While(op) => DagControlFlow::While {
                body: build_body(op.body().operations())?,
            },
            ClassicalControlOp::For(op) => DagControlFlow::For {
                body: build_body(op.body().operations())?,
            },
            ClassicalControlOp::Switch(op) => DagControlFlow::Switch {
                cases: op
                    .cases()
                    .iter()
                    .map(|case| {
                        Ok(DagSwitchCase {
                            value: case.value(),
                            body: build_body(case.body().operations())?,
                        })
                    })
                    .collect::<Result<_, CircuitError>>()?,
                default: op
                    .default()
                    .map(|body| build_body(body.operations()))
                    .transpose()?,
            },
            ClassicalControlOp::Break => DagControlFlow::Break,
            ClassicalControlOp::Continue => DagControlFlow::Continue,
        }))
    }

    fn add_edge_once(&mut self, source: NodeIndex, target: NodeIndex, wire: DagWire) {
        if self
            .graph
            .edges_connecting(source, target)
            .any(|edge| *edge.weight() == wire)
        {
            return;
        }
        self.graph.add_edge(source, target, wire);
    }

    fn operation_neighbors_on_wire(
        &self,
        node: NodeIndex,
        wire: DagWire,
        direction: Direction,
    ) -> Result<Vec<NodeIndex>, CircuitError> {
        if self.graph.node_weight(node).is_none() {
            return Err(CircuitError::InvalidDag(format!(
                "node {:?} is not in the DAG",
                node
            )));
        }
        self.validate_wire(wire)?;
        Ok(self.operation_neighbors_matching_unchecked(node, direction, |found| found == wire))
    }

    fn operation_neighbors_matching(
        &self,
        node: NodeIndex,
        direction: Direction,
        predicate: impl FnMut(DagWire) -> bool,
    ) -> Result<Vec<NodeIndex>, CircuitError> {
        if self.graph.node_weight(node).is_none() {
            return Err(CircuitError::InvalidDag(format!(
                "node {:?} is not in the DAG",
                node
            )));
        }
        Ok(self.operation_neighbors_matching_unchecked(node, direction, predicate))
    }

    fn operation_neighbors_matching_unchecked(
        &self,
        node: NodeIndex,
        direction: Direction,
        mut predicate: impl FnMut(DagWire) -> bool,
    ) -> Vec<NodeIndex> {
        let mut neighbors = IndexSet::new();
        for edge in self.graph.edges_directed(node, direction) {
            if !predicate(*edge.weight()) {
                continue;
            }
            let neighbor = match direction {
                Direction::Incoming => edge.source(),
                Direction::Outgoing => edge.target(),
            };
            if matches!(
                self.graph.node_weight(neighbor),
                Some(DagNode::Operation { .. })
            ) {
                neighbors.insert(neighbor);
            }
        }
        let mut neighbors = neighbors.into_iter().collect::<Vec<_>>();
        neighbors.sort_by_key(|node| self.topological_sort_key(*node));
        neighbors
    }

    fn next_node_on_wire(
        &self,
        node: NodeIndex,
        wire: DagWire,
    ) -> Result<Option<NodeIndex>, CircuitError> {
        let mut next = None;
        for edge in self
            .graph
            .edges_directed(node, rustworkx_core::petgraph::Outgoing)
            .filter(|edge| *edge.weight() == wire)
        {
            if next.replace(edge.target()).is_some() {
                return Err(CircuitError::InvalidDag(format!(
                    "wire {:?} branches at node {:?}",
                    wire, node
                )));
            }
        }
        Ok(next)
    }

    fn ensure_wire(&mut self, wire: DagWire) -> Result<(), CircuitError> {
        self.validate_wire(wire)?;
        if !self.wire_io.contains_key(&wire) {
            let input = self.graph.add_node(DagNode::WireIn(wire));
            let output = self.graph.add_node(DagNode::WireOut(wire));
            self.wire_io.insert(wire, [input, output]);
        }
        Ok(())
    }

    fn validate_wire_io(&self) -> Result<(), CircuitError> {
        for (wire, endpoints) in &self.wire_io {
            match self.graph.node_weight(endpoints[0]) {
                Some(DagNode::WireIn(found)) if found == wire => {}
                _ => {
                    return Err(CircuitError::InvalidDag(format!(
                        "wire {:?} has invalid input endpoint",
                        wire
                    )));
                }
            }
            match self.graph.node_weight(endpoints[1]) {
                Some(DagNode::WireOut(found)) if found == wire => {}
                _ => {
                    return Err(CircuitError::InvalidDag(format!(
                        "wire {:?} has invalid output endpoint",
                        wire
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_wire(&self, wire: DagWire) -> Result<(), CircuitError> {
        match wire {
            DagWire::Qubit(qubit) => {
                if !self.qubits.contains(&qubit) {
                    return Err(CircuitError::QubitNotFound(qubit.id()));
                }
            }
            DagWire::ClassicalVar(var) => {
                self.validate_classical_var(var)?;
            }
            DagWire::ClassicalValue(value) => {
                self.validate_classical_value(value)?;
            }
            DagWire::GlobalOrder => {}
        }
        Ok(())
    }

    fn validate_operation(&self, operation: &Operation) -> Result<(), CircuitError> {
        for qubit in &operation.qubits {
            if !self.qubits.contains(qubit) {
                return Err(CircuitError::QubitNotFound(qubit.id()));
            }
        }
        for param in &operation.params {
            if let CircuitParam::Index(index) = param
                && self.parameters.get_index(*index as usize).is_none()
            {
                return Err(CircuitError::InvalidParameterIndex(*index));
            }
        }
        self.validate_operation_classical_resources(operation)?;
        Ok(())
    }

    fn validate_classical_var(&self, var: ClassicalVar) -> Result<(), CircuitError> {
        match self.classical_vars.get(var.index() as usize) {
            Some(ty) if *ty == var.ty() => Ok(()),
            Some(_) => Err(CircuitError::InvalidDag(format!(
                "classical var {} has mismatched type",
                var.index()
            ))),
            None => Err(CircuitError::ForeignClassicalHandle {
                kind: "classical var",
                index: var.index(),
            }),
        }
    }

    fn validate_classical_value(&self, value: ClassicalValue) -> Result<(), CircuitError> {
        match self.classical_values.get(value.index() as usize) {
            Some(ty) if *ty == value.ty() => Ok(()),
            Some(_) => Err(CircuitError::InvalidDag(format!(
                "classical value {} has mismatched type",
                value.index()
            ))),
            None => Err(CircuitError::ForeignClassicalHandle {
                kind: "classical value",
                index: value.index(),
            }),
        }
    }

    fn validate_control_flow_payload(
        &self,
        operation: &Operation,
        control: &DagControlFlow,
    ) -> Result<(), CircuitError> {
        if !matches!(operation.instruction, Instruction::ClassicalControl(_)) {
            return Err(CircuitError::InvalidDag(
                "control-flow payload attached to non-control operation".to_string(),
            ));
        }
        match control {
            DagControlFlow::If {
                then_body,
                else_body,
            } => {
                then_body.validate()?;
                if let Some(body) = else_body {
                    body.validate()?;
                }
            }
            DagControlFlow::While { body } | DagControlFlow::For { body } => body.validate()?,
            DagControlFlow::Switch { cases, default } => {
                for case in cases {
                    case.body.validate()?;
                }
                if let Some(body) = default {
                    body.validate()?;
                }
            }
            DagControlFlow::Break | DagControlFlow::Continue => {}
        }
        Ok(())
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

    fn lower_value_operation(
        &mut self,
        operation: ValueOperation,
    ) -> Result<Operation, CircuitError> {
        let instruction = self.lower_value_instruction(operation.instruction)?;
        let params = operation
            .params
            .into_iter()
            .enumerate()
            .map(
                |(param_index, param)| -> Result<CircuitParam, CircuitError> {
                    match param {
                        ParameterValue::Param(param) => {
                            let (index, _) = self.add_parameter(param);
                            Ok(CircuitParam::Index(index as u32))
                        }
                        ParameterValue::Fixed(value) => {
                            if !value.is_finite() {
                                return Err(CircuitError::InvalidParameterValue(
                                    param_index,
                                    value,
                                ));
                            }
                            Ok(CircuitParam::Fixed(value))
                        }
                    }
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Operation {
            instruction,
            qubits: operation.qubits,
            params,
            label: operation.label,
        })
    }

    fn lower_value_instruction(
        &mut self,
        instruction: ValueInstruction,
    ) -> Result<Instruction, CircuitError> {
        let op = match instruction {
            ValueInstruction::Instruction(Instruction::ClassicalControl(_)) => {
                return Err(CircuitError::InvalidOperation(
                    "ValueInstruction::Instruction cannot wrap Instruction::ClassicalControl"
                        .to_string(),
                ));
            }
            ValueInstruction::Instruction(instruction) => return Ok(instruction),
            ValueInstruction::ClassicalControl(op) => op,
        };

        let op = match op {
            ValueClassicalControlOp::If {
                condition,
                then_body,
                else_body,
            } => {
                let then_body = self.lower_value_body(then_body)?;
                let else_body = else_body
                    .map(|body| self.lower_value_body(body))
                    .transpose()?;
                IfOp::new(condition, then_body, else_body).map(ClassicalControlOp::If)?
            }
            ValueClassicalControlOp::While { condition, body } => {
                let body = self.lower_value_body(body)?;
                WhileOp::new(condition, body).map(ClassicalControlOp::While)?
            }
            ValueClassicalControlOp::For {
                var,
                start,
                stop,
                step,
                body,
            } => {
                let body = self.lower_value_body(body)?;
                ForOp::new(var, start, stop, step, body).map(ClassicalControlOp::For)?
            }
            ValueClassicalControlOp::Switch {
                target,
                cases,
                default,
            } => {
                let cases = cases
                    .into_iter()
                    .map(|case| {
                        Ok(SwitchCase::new(
                            case.value,
                            self.lower_value_body(case.body)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, CircuitError>>()?;
                let default = default
                    .map(|body| self.lower_value_body(body))
                    .transpose()?;
                SwitchOp::new(target, cases, default).map(ClassicalControlOp::Switch)?
            }
            ValueClassicalControlOp::Break => ClassicalControlOp::Break,
            ValueClassicalControlOp::Continue => ClassicalControlOp::Continue,
        };
        Ok(Instruction::ClassicalControl(op))
    }

    fn lower_value_body(&mut self, body: ValueControlBody) -> Result<ControlBody, CircuitError> {
        body.operations()
            .iter()
            .cloned()
            .map(|operation| self.lower_value_operation(operation))
            .collect::<Result<Vec<_>, _>>()
            .map(ControlBody::new)
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

    fn topological_operations(&self) -> Result<Vec<Operation>, CircuitError> {
        Ok(self
            .topological_op_nodes_ref()?
            .iter()
            .copied()
            .map(|node| {
                self.operation(node)
                    .expect("topological_op_nodes returns only operation nodes")
                    .clone()
            })
            .collect())
    }

    fn topological_op_nodes_ref(&self) -> Result<&Vec<NodeIndex>, CircuitError> {
        if self.topological_op_nodes_cache.get().is_none() {
            let nodes = lexicographical_topological_sort(
                &self.graph,
                |node| Ok::<_, Infallible>(self.topological_sort_key(node)),
                false,
                None,
            )
            .map_err(|error| {
                CircuitError::InvalidDag(format!("DAG topological sort failed: {error}"))
            })?;
            let nodes = nodes
                .into_iter()
                .filter(|node| matches!(self.graph[*node], DagNode::Operation { .. }))
                .collect::<Vec<_>>();
            let _ = self.topological_op_nodes_cache.set(nodes);
        }
        self.topological_op_nodes_cache.get().ok_or_else(|| {
            CircuitError::InvalidDag("topological operation cache was not initialized".to_string())
        })
    }

    fn node_layers_ref(&self) -> Result<&IndexMap<NodeIndex, usize>, CircuitError> {
        if self.node_layers_cache.get().is_none() {
            let mut ordered_layers = IndexMap::new();
            for node in self.topological_op_nodes_ref()?.iter().copied() {
                let mut layer = 0usize;
                for pred in self
                    .predecessors(node)
                    .filter(|pred| matches!(self.graph[*pred], DagNode::Operation { .. }))
                {
                    let predecessor_layer =
                        ordered_layers.get(&pred).copied().ok_or_else(|| {
                            CircuitError::InvalidDag(format!(
                                "operation predecessor {:?} was not visited before {:?}",
                                pred, node
                            ))
                        })?;
                    layer = layer.max(predecessor_layer + 1);
                }
                ordered_layers.insert(node, layer);
            }
            let _ = self.node_layers_cache.set(ordered_layers);
        }
        self.node_layers_cache.get().ok_or_else(|| {
            CircuitError::InvalidDag("node layer cache was not initialized".to_string())
        })
    }

    pub(crate) fn operation_order(&self, node: NodeIndex) -> Option<usize> {
        match self.graph.node_weight(node) {
            Some(DagNode::Operation { order, .. }) => Some(*order),
            _ => None,
        }
    }

    fn topological_sort_key(&self, node: NodeIndex) -> (u8, usize, usize) {
        match self.graph.node_weight(node) {
            Some(DagNode::WireIn(_)) => (0, node.index(), 0),
            Some(DagNode::Operation { order, .. }) => (1, *order, node.index()),
            Some(DagNode::WireOut(_)) => (2, node.index(), 0),
            None => (3, node.index(), 0),
        }
    }

    fn require_operation_order(&self, node: NodeIndex) -> Result<usize, CircuitError> {
        self.operation_order(node)
            .ok_or_else(|| CircuitError::InvalidDag(format!("node {:?} is not an operation", node)))
    }

    pub(crate) fn node_for_order(&self, order: usize) -> Option<NodeIndex> {
        self.order_to_node.get(order).copied()
    }

    fn operation_resources(&self, operation: &Operation) -> OperationResources {
        let mut resources = OperationResources::from_operation(operation);
        if matches!(
            operation.instruction,
            Instruction::Directive(Directive::Barrier)
        ) && operation.qubits.is_empty()
        {
            resources.writes.clear();
            resources
                .writes
                .extend(self.qubits.iter().copied().map(DagWire::Qubit));
        }
        resources
    }

    fn validate_operation_classical_resources(
        &self,
        operation: &Operation,
    ) -> Result<(), CircuitError> {
        let resources = self.operation_resources(operation);
        for wire in resources.reads.iter().chain(resources.writes.iter()) {
            self.validate_wire(*wire)?;
        }
        Ok(())
    }

    fn validate_replacement(
        &self,
        old: &Operation,
        replacement: &CircuitDag,
    ) -> Result<Vec<Operation>, CircuitError> {
        let replacement_operations = replacement.topological_operations()?;
        self.validate_replacement_control_flow_shape(old, &replacement_operations)?;
        self.validate_replacement_footprint(old, replacement, &replacement_operations)?;
        Ok(replacement_operations)
    }

    fn validate_replacement_control_flow_shape(
        &self,
        old: &Operation,
        replacement_operations: &[Operation],
    ) -> Result<(), CircuitError> {
        if !matches!(old.instruction, Instruction::ClassicalControl(_)) {
            return Ok(());
        }

        let [replacement_op] = replacement_operations else {
            return Err(CircuitError::InvalidDag(
                "control-flow operation replacement must contain exactly one operation".to_string(),
            ));
        };

        if !same_control_flow_kind(&old.instruction, &replacement_op.instruction) {
            return Err(CircuitError::InvalidDag(
                "control-flow operation replacement must preserve the control-flow kind"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn validate_replacement_footprint(
        &self,
        old: &Operation,
        replacement: &CircuitDag,
        replacement_operations: &[Operation],
    ) -> Result<(), CircuitError> {
        let old_resources = self.operation_resources(old);
        let readable = old_resources
            .reads
            .union(&old_resources.writes)
            .copied()
            .collect::<IndexSet<_>>();

        for operation in replacement_operations {
            let resources = replacement.operation_resources(operation);
            for wire in &resources.writes {
                if *wire != DagWire::GlobalOrder && !old_resources.writes.contains(wire) {
                    return Err(CircuitError::InvalidDag(format!(
                        "replacement writes wire {:?} outside replaced node write footprint",
                        wire
                    )));
                }
            }
            for wire in &resources.reads {
                if *wire != DagWire::GlobalOrder && !readable.contains(wire) {
                    return Err(CircuitError::InvalidDag(format!(
                        "replacement reads wire {:?} outside replaced node footprint",
                        wire
                    )));
                }
            }
        }
        Ok(())
    }

    fn add_operation_counts_recursive(&self, counts: &mut IndexMap<String, usize>) {
        for node in self.op_nodes() {
            if let Some(operation) = self.operation(node) {
                let name = operation.instruction.name();
                *counts.entry(name).or_insert(0) += 1;
            }
            if let Some(control) = self.control_flow(node) {
                for body in control.body_dags() {
                    body.add_operation_counts_recursive(counts);
                }
            }
        }
    }
}

impl DagControlFlow {
    fn body_dags(&self) -> Box<dyn Iterator<Item = &CircuitDag> + '_> {
        match self {
            DagControlFlow::If {
                then_body,
                else_body,
            } => Box::new(std::iter::once(then_body.as_ref()).chain(else_body.as_deref())),
            DagControlFlow::While { body } | DagControlFlow::For { body } => {
                Box::new(std::iter::once(body.as_ref()))
            }
            DagControlFlow::Switch { cases, default } => Box::new(
                cases
                    .iter()
                    .map(|case| case.body.as_ref())
                    .chain(default.as_deref()),
            ),
            DagControlFlow::Break | DagControlFlow::Continue => Box::new(std::iter::empty()),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct OperationResources {
    reads: IndexSet<DagWire>,
    writes: IndexSet<DagWire>,
}

impl OperationResources {
    fn from_operation(operation: &Operation) -> Self {
        let mut resources = Self::default();
        resources
            .writes
            .extend(operation.qubits.iter().copied().map(DagWire::Qubit));

        match &operation.instruction {
            Instruction::ClassicalData(op) => match op {
                ClassicalDataOp::Store { target, value } => {
                    resources.add_expr_reads(value);
                    resources.writes.insert(DagWire::ClassicalVar(*target));
                }
                ClassicalDataOp::MeasureBit { result }
                | ClassicalDataOp::MeasureBits { result } => {
                    resources.writes.insert(DagWire::ClassicalValue(*result));
                }
            },
            Instruction::ClassicalControl(control) => resources.add_control_resources(control),
            _ => {}
        }
        resources
    }

    fn add_control_resources(&mut self, control: &ClassicalControlOp) {
        match control {
            ClassicalControlOp::If(op) => {
                self.add_expr_reads(op.condition());
                self.add_body_resources(op.then_body().operations());
                if let Some(body) = op.else_body() {
                    self.add_body_resources(body.operations());
                }
            }
            ClassicalControlOp::While(op) => {
                self.add_expr_reads(op.condition());
                self.add_body_resources(op.body().operations());
            }
            ClassicalControlOp::For(op) => {
                self.writes.insert(DagWire::ClassicalVar(op.var()));
                self.add_expr_reads(op.start());
                self.add_expr_reads(op.stop());
                self.add_expr_reads(op.step());
                self.add_body_resources(op.body().operations());
            }
            ClassicalControlOp::Switch(op) => {
                self.add_expr_reads(op.target());
                for case in op.cases() {
                    self.add_body_resources(case.body().operations());
                }
                if let Some(body) = op.default() {
                    self.add_body_resources(body.operations());
                }
            }
            ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
        }
    }

    fn add_body_resources(&mut self, operations: &[Operation]) {
        for operation in operations {
            let nested = Self::from_operation(operation);
            self.reads.extend(nested.reads);
            self.writes.extend(nested.writes);
        }
    }

    fn add_expr_reads(&mut self, expr: &ClassicalExpr) {
        self.reads
            .extend(expr.vars().into_iter().map(DagWire::ClassicalVar));
        self.reads
            .extend(expr.values().into_iter().map(DagWire::ClassicalValue));
    }
}

fn is_one_qubit_gate(operation: &Operation) -> bool {
    operation.qubits.len() == 1
        && matches!(
            operation.instruction,
            Instruction::Standard(_)
                | Instruction::McGate(_)
                | Instruction::UnitaryGate(_)
                | Instruction::CircuitGate(_)
        )
}

fn is_two_qubit_gate(operation: &Operation) -> bool {
    operation.qubits.len() == 2
        && matches!(
            operation.instruction,
            Instruction::Standard(_)
                | Instruction::McGate(_)
                | Instruction::UnitaryGate(_)
                | Instruction::CircuitGate(_)
        )
}

fn same_control_flow_kind(lhs: &Instruction, rhs: &Instruction) -> bool {
    matches!((lhs, rhs), (Instruction::ClassicalControl(lhs), Instruction::ClassicalControl(rhs)) if lhs.kind() == rhs.kind())
}

impl TryFrom<&Circuit> for CircuitDag {
    type Error = CircuitError;

    fn try_from(circuit: &Circuit) -> Result<Self, Self::Error> {
        Self::from_circuit(circuit)
    }
}

impl TryFrom<&CircuitDag> for Circuit {
    type Error = CircuitError;

    fn try_from(dag: &CircuitDag) -> Result<Self, Self::Error> {
        dag.to_circuit()
    }
}

#[cfg(test)]
#[path = "./dag_test.rs"]
mod dag_test;
