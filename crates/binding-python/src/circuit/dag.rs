// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of the LICENSE.txt file in the root directory of this source
// tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Python bindings for the circuit dependency DAG analysis view.

use crate::circuit::PyCircuit;
use crate::circuit::bit::{PyIntOrQubit, PyQubit};
use crate::circuit::classical::{PyClassicalType, PyClassicalValue, PyClassicalVar};
use crate::circuit::error::CircuitError as PyCircuitError;
use crate::circuit::operation::PyValueOperation;
use crate::circuit::parameter::PyParameter;
use crate::utils::hash_value;
use cqlib_core::circuit::circuit_param::{CircuitParam, ParameterValue};
use cqlib_core::circuit::value_instruction::storage_operation_to_value;
use cqlib_core::circuit::{CircuitDag, CircuitError, DagControlFlow, DagSwitchCase, DagWire};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rustworkx_core::petgraph::prelude::NodeIndex;

fn py_error(error: CircuitError) -> PyErr {
    PyCircuitError::new_err(error.to_string())
}

fn node_index(node: usize) -> NodeIndex {
    NodeIndex::new(node)
}

fn node_id(node: NodeIndex) -> usize {
    node.index()
}

fn node_ids(nodes: Vec<NodeIndex>) -> Vec<usize> {
    nodes.into_iter().map(node_id).collect()
}

/// Resource carried by a circuit DAG edge.
#[pyclass(name = "DagWire", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyDagWire {
    pub(crate) inner: DagWire,
}

impl From<DagWire> for PyDagWire {
    fn from(inner: DagWire) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDagWire {
    /// Creates a qubit wire.
    #[staticmethod]
    fn qubit(qubit: PyIntOrQubit) -> Self {
        Self {
            inner: DagWire::Qubit(qubit.into()),
        }
    }

    /// Creates a mutable classical variable wire.
    #[staticmethod]
    fn classical_var(var: PyClassicalVar) -> Self {
        Self {
            inner: DagWire::ClassicalVar(var.inner),
        }
    }

    /// Creates an immutable classical value wire.
    #[staticmethod]
    fn classical_value(value: PyClassicalValue) -> Self {
        Self {
            inner: DagWire::ClassicalValue(value.inner),
        }
    }

    /// Creates the stable ordering wire for operations without concrete data resources.
    #[staticmethod]
    fn global_order() -> Self {
        Self {
            inner: DagWire::GlobalOrder,
        }
    }

    /// Returns the wire kind.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            DagWire::Qubit(_) => "qubit",
            DagWire::ClassicalVar(_) => "classical_var",
            DagWire::ClassicalValue(_) => "classical_value",
            DagWire::GlobalOrder => "global_order",
        }
    }

    /// Returns the qubit carried by this wire, if any.
    #[getter]
    fn qubit_value(&self) -> Option<PyQubit> {
        match self.inner {
            DagWire::Qubit(qubit) => Some(PyQubit::from(qubit)),
            _ => None,
        }
    }

    /// Returns the classical variable carried by this wire, if any.
    #[getter]
    fn classical_var_value(&self) -> Option<PyClassicalVar> {
        match self.inner {
            DagWire::ClassicalVar(var) => Some(PyClassicalVar::from(var)),
            _ => None,
        }
    }

    /// Returns the classical value carried by this wire, if any.
    #[getter]
    fn classical_value_value(&self) -> Option<PyClassicalValue> {
        match self.inner {
            DagWire::ClassicalValue(value) => Some(PyClassicalValue::from(value)),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            DagWire::Qubit(qubit) => format!("DagWire.qubit({})", qubit.index()),
            DagWire::ClassicalVar(var) => format!("DagWire.classical_var({var:?})"),
            DagWire::ClassicalValue(value) => format!("DagWire.classical_value({value:?})"),
            DagWire::GlobalOrder => "DagWire.global_order()".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyDagWire>() {
            return Ok(false);
        }
        let other = other.extract::<PyDagWire>()?;
        Ok(self.inner == other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}

/// Recursive DAG payload attached to a structured control-flow operation.
#[pyclass(name = "DagControlFlow", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyDagControlFlow {
    inner: DagControlFlow,
}

impl From<DagControlFlow> for PyDagControlFlow {
    fn from(inner: DagControlFlow) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDagControlFlow {
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            DagControlFlow::If { .. } => "if",
            DagControlFlow::While { .. } => "while",
            DagControlFlow::For { .. } => "for",
            DagControlFlow::Switch { .. } => "switch",
            DagControlFlow::Break => "break",
            DagControlFlow::Continue => "continue",
        }
    }

    #[getter]
    fn then_body(&self) -> Option<PyCircuitDag> {
        match &self.inner {
            DagControlFlow::If { then_body, .. } => Some(PyCircuitDag::from((**then_body).clone())),
            _ => None,
        }
    }

    #[getter]
    fn else_body(&self) -> Option<PyCircuitDag> {
        match &self.inner {
            DagControlFlow::If { else_body, .. } => else_body
                .as_ref()
                .map(|body| PyCircuitDag::from((**body).clone())),
            _ => None,
        }
    }

    #[getter]
    fn body(&self) -> Option<PyCircuitDag> {
        match &self.inner {
            DagControlFlow::While { body } | DagControlFlow::For { body } => {
                Some(PyCircuitDag::from((**body).clone()))
            }
            _ => None,
        }
    }

    #[getter]
    fn cases(&self) -> Vec<PyDagSwitchCase> {
        match &self.inner {
            DagControlFlow::Switch { cases, .. } => {
                cases.iter().cloned().map(PyDagSwitchCase::from).collect()
            }
            _ => Vec::new(),
        }
    }

    #[getter]
    fn default_body(&self) -> Option<PyCircuitDag> {
        match &self.inner {
            DagControlFlow::Switch { default, .. } => default
                .as_ref()
                .map(|body| PyCircuitDag::from((**body).clone())),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("DagControlFlow(kind='{}')", self.kind())
    }
}

/// One switch-case body in a structured control-flow DAG payload.
#[pyclass(name = "DagSwitchCase", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyDagSwitchCase {
    inner: DagSwitchCase,
}

impl From<DagSwitchCase> for PyDagSwitchCase {
    fn from(inner: DagSwitchCase) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDagSwitchCase {
    #[getter]
    fn value(&self) -> u128 {
        self.inner.value
    }

    #[getter]
    fn body(&self) -> PyCircuitDag {
        PyCircuitDag::from((*self.inner.body).clone())
    }

    fn __repr__(&self) -> String {
        format!("DagSwitchCase(value={})", self.inner.value)
    }
}

/// Operation-level dependency DAG analysis view for a circuit.
#[pyclass(name = "CircuitDag", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyCircuitDag {
    pub(crate) inner: CircuitDag,
}

impl From<CircuitDag> for PyCircuitDag {
    fn from(inner: CircuitDag) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCircuitDag {
    /// Builds a dependency DAG from a circuit.
    #[staticmethod]
    fn from_circuit(circuit: &PyCircuit) -> PyResult<Self> {
        CircuitDag::from_circuit(&circuit.inner)
            .map(Self::from)
            .map_err(py_error)
    }

    /// Reconstructs a circuit from this DAG.
    fn to_circuit(&self) -> PyResult<PyCircuit> {
        self.inner
            .to_circuit()
            .map(PyCircuit::from)
            .map_err(py_error)
    }

    /// Validates DAG invariants.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(py_error)
    }

    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    #[getter]
    fn num_ops(&self) -> usize {
        self.inner.num_ops()
    }

    #[getter]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner.qubits().into_iter().map(PyQubit::from).collect()
    }

    #[getter]
    fn parameters(&self) -> Vec<PyParameter> {
        self.inner
            .parameters()
            .iter()
            .cloned()
            .map(PyParameter::from)
            .collect()
    }

    #[getter]
    fn symbols(&self) -> Vec<String> {
        self.inner.symbols().iter().cloned().collect()
    }

    #[getter]
    fn classical_vars(&self) -> Vec<PyClassicalType> {
        self.inner
            .classical_vars()
            .iter()
            .copied()
            .map(PyClassicalType::from)
            .collect()
    }

    #[getter]
    fn classical_values(&self) -> Vec<PyClassicalType> {
        self.inner
            .classical_values()
            .iter()
            .copied()
            .map(PyClassicalType::from)
            .collect()
    }

    fn wires(&self) -> Vec<PyDagWire> {
        self.inner.wires().map(PyDagWire::from).collect()
    }

    fn has_wire(&self, wire: PyDagWire) -> bool {
        self.inner.has_wire(wire.inner)
    }

    fn is_wire_idle(&self, wire: PyDagWire) -> PyResult<bool> {
        self.inner.is_wire_idle(wire.inner).map_err(py_error)
    }

    fn nodes_on_wire(&self, wire: PyDagWire) -> PyResult<Vec<usize>> {
        self.inner
            .nodes_on_wire(wire.inner)
            .map(node_ids)
            .map_err(py_error)
    }

    fn wire_in(&self, wire: PyDagWire) -> Option<usize> {
        self.inner.wire_in(wire.inner).map(node_id)
    }

    fn wire_out(&self, wire: PyDagWire) -> Option<usize> {
        self.inner.wire_out(wire.inner).map(node_id)
    }

    fn is_operation(&self, node: usize) -> bool {
        self.inner.is_operation(node_index(node))
    }

    fn node_kind(&self, node: usize) -> Option<&'static str> {
        self.inner.node_kind(node_index(node))
    }

    fn op_nodes(&self) -> Vec<usize> {
        self.inner.op_nodes().map(node_id).collect()
    }

    fn topological_op_nodes(&self) -> PyResult<Vec<usize>> {
        self.inner
            .topological_op_nodes()
            .map(node_ids)
            .map_err(py_error)
    }

    fn front_layer(&self) -> PyResult<Vec<usize>> {
        self.inner.front_layer().map(node_ids).map_err(py_error)
    }

    fn layers(&self) -> PyResult<Vec<Vec<usize>>> {
        self.inner
            .layers()
            .map(|layers| layers.into_iter().map(node_ids).collect())
            .map_err(py_error)
    }

    fn node_layers<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (node, layer) in self.inner.node_layers().map_err(py_error)? {
            dict.set_item(node_id(node), layer)?;
        }
        Ok(dict.unbind())
    }

    fn predecessors(&self, node: usize) -> Vec<usize> {
        self.inner
            .predecessors(node_index(node))
            .map(node_id)
            .collect()
    }

    fn successors(&self, node: usize) -> Vec<usize> {
        self.inner
            .successors(node_index(node))
            .map(node_id)
            .collect()
    }

    fn predecessors_on_wire(&self, node: usize, wire: PyDagWire) -> PyResult<Vec<usize>> {
        self.inner
            .predecessors_on_wire(node_index(node), wire.inner)
            .map(node_ids)
            .map_err(py_error)
    }

    fn successors_on_wire(&self, node: usize, wire: PyDagWire) -> PyResult<Vec<usize>> {
        self.inner
            .successors_on_wire(node_index(node), wire.inner)
            .map(node_ids)
            .map_err(py_error)
    }

    fn quantum_predecessors(&self, node: usize) -> PyResult<Vec<usize>> {
        self.inner
            .quantum_predecessors(node_index(node))
            .map(node_ids)
            .map_err(py_error)
    }

    fn quantum_successors(&self, node: usize) -> PyResult<Vec<usize>> {
        self.inner
            .quantum_successors(node_index(node))
            .map(node_ids)
            .map_err(py_error)
    }

    fn classical_predecessors(&self, node: usize) -> PyResult<Vec<usize>> {
        self.inner
            .classical_predecessors(node_index(node))
            .map(node_ids)
            .map_err(py_error)
    }

    fn classical_successors(&self, node: usize) -> PyResult<Vec<usize>> {
        self.inner
            .classical_successors(node_index(node))
            .map(node_ids)
            .map_err(py_error)
    }

    fn operation(&self, node: usize) -> PyResult<Option<PyValueOperation>> {
        let Some(operation) = self.inner.operation(node_index(node)) else {
            return Ok(None);
        };
        storage_operation_to_value(operation.clone(), &|param| self.parameter_value(param))
            .map(PyValueOperation::from)
            .map(Some)
            .map_err(py_error)
    }

    fn control_flow(&self, node: usize) -> Option<PyDagControlFlow> {
        self.inner
            .control_flow(node_index(node))
            .cloned()
            .map(PyDagControlFlow::from)
    }

    fn depth(&self) -> PyResult<usize> {
        self.inner.depth().map_err(py_error)
    }

    fn has_control_flow(&self) -> bool {
        self.inner.has_control_flow()
    }

    fn has_nested_control_flow(&self) -> bool {
        self.inner.has_nested_control_flow()
    }

    fn has_measurement(&self) -> bool {
        self.inner.has_measurement()
    }

    fn operation_count_by_name<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (name, count) in self.inner.operation_count_by_name() {
            dict.set_item(name, count)?;
        }
        Ok(dict.unbind())
    }

    fn operation_count_by_name_recursive<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (name, count) in self.inner.operation_count_by_name_recursive() {
            dict.set_item(name, count)?;
        }
        Ok(dict.unbind())
    }

    fn collect_1q_runs(&self) -> PyResult<Vec<Vec<usize>>> {
        self.inner
            .collect_1q_runs()
            .map(|runs| runs.into_iter().map(node_ids).collect())
            .map_err(py_error)
    }

    fn collect_2q_runs(&self) -> PyResult<Vec<Vec<usize>>> {
        self.inner
            .collect_2q_runs()
            .map(|runs| runs.into_iter().map(node_ids).collect())
            .map_err(py_error)
    }

    fn add_parameter(&mut self, parameter: PyParameter) -> (usize, bool) {
        self.inner.add_parameter(parameter.inner)
    }

    fn apply_operation_back(&mut self, operation: PyValueOperation) -> PyResult<usize> {
        self.inner
            .apply_value_operation_back(operation.inner)
            .map(node_id)
            .map_err(py_error)
    }

    fn apply_operation_front(&mut self, operation: PyValueOperation) -> PyResult<usize> {
        self.inner
            .apply_value_operation_front(operation.inner)
            .map(node_id)
            .map_err(py_error)
    }

    fn remove_op_node(&mut self, node: usize) -> PyResult<PyValueOperation> {
        let operation = self
            .inner
            .remove_op_node(node_index(node))
            .map_err(py_error)?;
        storage_operation_to_value(operation, &|param| self.parameter_value(param))
            .map(PyValueOperation::from)
            .map_err(py_error)
    }

    fn substitute_node(&mut self, node: usize, operation: PyValueOperation) -> PyResult<()> {
        self.inner
            .substitute_value_node(node_index(node), operation.inner)
            .map_err(py_error)
    }

    fn substitute_node_with_dag(
        &mut self,
        node: usize,
        replacement: &PyCircuitDag,
    ) -> PyResult<()> {
        self.inner
            .substitute_node_with_dag(node_index(node), replacement.inner.clone())
            .map_err(py_error)
    }

    fn __repr__(&self) -> String {
        format!(
            "CircuitDag(num_qubits={}, num_ops={})",
            self.inner.num_qubits(),
            self.inner.num_ops()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_ops()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

impl PyCircuitDag {
    fn parameter_value(&self, parameter: &CircuitParam) -> Result<ParameterValue, CircuitError> {
        match parameter {
            CircuitParam::Fixed(value) => Ok(ParameterValue::Fixed(*value)),
            CircuitParam::Index(index) => self
                .inner
                .parameters()
                .get_index(*index as usize)
                .cloned()
                .map(ParameterValue::Param)
                .ok_or(CircuitError::InvalidParameterIndex(*index)),
        }
    }
}
