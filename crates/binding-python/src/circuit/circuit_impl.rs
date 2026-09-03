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

//! Python binding for the mutable quantum circuit container.
//!
//! The wrapper follows the two-layer core IR. Common gate and classical-data
//! methods build a circuit directly, while PyValueOperation is the generic
//! construction boundary for custom instructions and structured control flow.

use crate::circuit::bit::{PyIntListOrQubitList, PyIntOrQubit, PyIntQubitList, PyQubit};
use crate::circuit::error::{
    CircuitError as PyCircuitError, ParameterError as PyParameterError, circuit_error_to_py_err,
};
use crate::circuit::{
    PyCircuitDag, PyCircuitGate, PyCircuitId, PyClassicalControlOp, PyClassicalExpr,
    PyClassicalType, PyClassicalVar, PyMcGate, PyMeasurement, PyParameter, PyStandardGate,
    PySwitchBuilder, PySymbolicMatrix, PyUnitaryGate, PyValueOperation,
};
use cqlib_core::circuit::error::ParameterError;
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::circuit::symbolic_matrix::circuit_to_symbolic_matrix;
use cqlib_core::circuit::{
    Circuit, CircuitDag, ClassicalControlOp, ExternalControlScope, ForOp, IfOp, Parameter,
    ParameterValue, Qubit, SwitchOp, ValueInstruction, ValueOperation, WhileOp,
};
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Python gate parameter accepted as either a finite number or a Parameter.
#[derive(FromPyObject)]
enum PyParamLike {
    Float(f64),
    Parameter(PyParameter),
}

impl PyParamLike {
    /// Converts Python input without allowing non-finite fixed parameters.
    fn into_value(self) -> PyResult<ParameterValue> {
        match self {
            Self::Float(value) if value.is_finite() => Ok(ParameterValue::Fixed(value)),
            Self::Float(value) => Err(PyParameterError::new_err(
                ParameterError::DomainError(format!(
                    "numeric parameter must be finite, got {value}"
                ))
                .to_string(),
            )),
            Self::Parameter(parameter) => Ok(ParameterValue::Param(parameter.inner)),
        }
    }
}

/// Mutable quantum circuit with gate, parameter, and dynamic-control support.
#[pyclass(name = "Circuit", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyCircuit {
    pub(crate) inner: Circuit,
}

impl From<Circuit> for PyCircuit {
    fn from(inner: Circuit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCircuit {
    /// Creates a circuit from a qubit count, integer IDs, or Qubit objects.
    #[new]
    fn new(qubits: PyIntQubitList) -> PyResult<Self> {
        Circuit::from_qubits(qubits.into())
            .map(Self::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Builds a circuit from self-contained construction-IR operations.
    #[staticmethod]
    #[pyo3(signature = (qubits, operations, classical_vars=None, classical_values=None))]
    fn from_operations(
        qubits: Vec<PyQubit>,
        operations: Vec<PyValueOperation>,
        classical_vars: Option<Vec<PyClassicalType>>,
        classical_values: Option<Vec<PyClassicalType>>,
    ) -> PyResult<Self> {
        Circuit::from_operations(
            qubits.into_iter().map(|qubit| qubit.inner).collect(),
            operations.into_iter().map(|operation| operation.inner),
            classical_vars.map(|types| types.into_iter().map(|ty| ty.inner).collect()),
            classical_values.map(|types| types.into_iter().map(|ty| ty.inner).collect()),
        )
        .map(Self::from)
        .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns the identity owning this circuit's classical handles.
    #[getter]
    fn id(&self) -> PyCircuitId {
        self.inner.id().into()
    }

    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    #[getter]
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Returns qubits in insertion order.
    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner.qubits().into_iter().map(PyQubit::from).collect()
    }

    /// Returns interned parameters in insertion order.
    #[getter]
    fn parameters(&self) -> Vec<PyParameter> {
        self.inner
            .parameters()
            .iter()
            .cloned()
            .map(PyParameter::from)
            .collect()
    }

    /// Returns interned symbol names in insertion order.
    ///
    /// This registry is stable and may contain symbols no longer referenced by
    /// executable IR. Use `used_symbols` or `uses_symbol` for live dependencies.
    #[getter]
    fn symbols(&self) -> Vec<String> {
        self.inner.symbols().iter().cloned().collect()
    }

    /// Returns symbol names actually referenced by executable IR.
    #[getter]
    fn used_symbols(&self) -> Vec<String> {
        self.inner.used_symbols().into_iter().collect()
    }

    /// Returns whether executable IR currently references `symbol`.
    fn uses_symbol(&self, symbol: &str) -> bool {
        self.inner.uses_symbol(symbol)
    }

    #[getter]
    fn global_phase(&self) -> PyParameter {
        self.inner.global_phase().into()
    }

    /// Replaces the circuit global phase.
    fn set_global_phase(&mut self, phase: PyParamLike) -> PyResult<()> {
        let phase = match phase.into_value()? {
            ParameterValue::Fixed(value) => Parameter::from(value),
            ParameterValue::Param(parameter) => parameter,
        };
        self.inner.set_global_phase(phase);
        Ok(())
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

    /// Adds qubits while preserving existing circuit data.
    fn add_qubits(&mut self, qubits: PyIntListOrQubitList) -> PyResult<()> {
        self.inner
            .add_qubits(qubits.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends any self-contained construction-IR operation.
    fn append(&mut self, operation: PyValueOperation) -> PyResult<()> {
        self.inner
            .append_value_operation(operation.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a self-contained classical control-flow operation.
    fn append_control(&mut self, control: PyClassicalControlOp) -> PyResult<()> {
        let qubits = control.inner.used_qubits().into_iter().collect();
        self.inner
            .append_value_operation(ValueOperation {
                instruction: ValueInstruction::ClassicalControl(control.inner),
                qubits,
                params: Default::default(),
                label: None,
            })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Builds and appends an if body through a Python callback.
    fn if_<'py>(
        mut slf: PyRefMut<'py, Self>,
        condition: PyClassicalExpr,
        body: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.inner
            .validate_classical_expr(&condition.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        let transaction = slf.inner.begin_control_body_transaction();
        slf.inner
            .enter_external_control_body(&transaction, ExternalControlScope::Branch);
        let py = slf.py();
        let circuit: Py<Self> = slf.into();

        if let Err(error) = body.call1((circuit.bind(py),)) {
            circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }

        let mut circuit = circuit.bind(py).borrow_mut();
        let then_body = circuit.inner.finish_external_control_body(&transaction);
        let op = match IfOp::new(condition.inner, then_body, None) {
            Ok(op) => ClassicalControlOp::If(op),
            Err(error) => {
                circuit.inner.rollback_control_body_transaction(transaction);
                return Err(PyCircuitError::new_err(error.to_string()));
            }
        };
        if let Err(error) = circuit.inner.append_control(op) {
            circuit.inner.rollback_control_body_transaction(transaction);
            return Err(PyCircuitError::new_err(error.to_string()));
        }
        circuit.inner.commit_control_body_transaction(transaction);
        Ok(())
    }

    /// Builds and appends if/else bodies through Python callbacks.
    fn if_else<'py>(
        mut slf: PyRefMut<'py, Self>,
        condition: PyClassicalExpr,
        then_body: &Bound<'py, PyAny>,
        else_body: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.inner
            .validate_classical_expr(&condition.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        let transaction = slf.inner.begin_control_body_transaction();
        slf.inner
            .enter_external_control_body(&transaction, ExternalControlScope::Branch);
        let py = slf.py();
        let circuit: Py<Self> = slf.into();

        if let Err(error) = then_body.call1((circuit.bind(py),)) {
            circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }
        let then_body = {
            let mut inner = circuit.bind(py).borrow_mut();
            let body = inner.inner.finish_external_control_body(&transaction);
            inner
                .inner
                .enter_external_control_body(&transaction, ExternalControlScope::Branch);
            body
        };
        if let Err(error) = else_body.call1((circuit.bind(py),)) {
            circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }

        let mut circuit = circuit.bind(py).borrow_mut();
        let else_body = circuit.inner.finish_external_control_body(&transaction);
        let op = match IfOp::new(condition.inner, then_body, Some(else_body)) {
            Ok(op) => ClassicalControlOp::If(op),
            Err(error) => {
                circuit.inner.rollback_control_body_transaction(transaction);
                return Err(PyCircuitError::new_err(error.to_string()));
            }
        };
        if let Err(error) = circuit.inner.append_control(op) {
            circuit.inner.rollback_control_body_transaction(transaction);
            return Err(PyCircuitError::new_err(error.to_string()));
        }
        circuit.inner.commit_control_body_transaction(transaction);
        Ok(())
    }

    /// Builds and appends a while body through a Python callback.
    fn while_<'py>(
        mut slf: PyRefMut<'py, Self>,
        condition: PyClassicalExpr,
        body: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.inner
            .validate_classical_expr(&condition.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        let transaction = slf.inner.begin_control_body_transaction();
        slf.inner
            .enter_external_control_body(&transaction, ExternalControlScope::Loop);
        let py = slf.py();
        let circuit: Py<Self> = slf.into();

        if let Err(error) = body.call1((circuit.bind(py),)) {
            circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }

        let mut circuit = circuit.bind(py).borrow_mut();
        let body = circuit.inner.finish_external_control_body(&transaction);
        let op = match WhileOp::new(condition.inner, body) {
            Ok(op) => ClassicalControlOp::While(op),
            Err(error) => {
                circuit.inner.rollback_control_body_transaction(transaction);
                return Err(PyCircuitError::new_err(error.to_string()));
            }
        };
        if let Err(error) = circuit.inner.append_control(op) {
            circuit.inner.rollback_control_body_transaction(transaction);
            return Err(PyCircuitError::new_err(error.to_string()));
        }
        circuit.inner.commit_control_body_transaction(transaction);
        Ok(())
    }

    /// Builds and appends an unsigned range loop through a Python callback.
    fn for_uint<'py>(
        mut slf: PyRefMut<'py, Self>,
        var: PyClassicalVar,
        start: PyClassicalExpr,
        stop: PyClassicalExpr,
        step: PyClassicalExpr,
        body: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.inner
            .validate_classical_var(var.inner)
            .and_then(|_| slf.inner.validate_classical_expr(&start.inner))
            .and_then(|_| slf.inner.validate_classical_expr(&stop.inner))
            .and_then(|_| slf.inner.validate_classical_expr(&step.inner))
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        let loop_expr = var.inner.expr();
        let transaction = slf.inner.begin_control_body_transaction();
        slf.inner
            .enter_external_control_body(&transaction, ExternalControlScope::Loop);
        let py = slf.py();
        let circuit: Py<Self> = slf.into();

        if let Err(error) = body.call1((circuit.bind(py), PyClassicalExpr::from(loop_expr))) {
            circuit
                .bind(py)
                .borrow_mut()
                .inner
                .rollback_control_body_transaction(transaction);
            return Err(error);
        }

        let mut circuit = circuit.bind(py).borrow_mut();
        let body = circuit.inner.finish_external_control_body(&transaction);
        let op = match ForOp::new(var.inner, start.inner, stop.inner, step.inner, body) {
            Ok(op) => ClassicalControlOp::For(op),
            Err(error) => {
                circuit.inner.rollback_control_body_transaction(transaction);
                return Err(PyCircuitError::new_err(error.to_string()));
            }
        };
        if let Err(error) = circuit.inner.append_control(op) {
            circuit.inner.rollback_control_body_transaction(transaction);
            return Err(PyCircuitError::new_err(error.to_string()));
        }
        circuit.inner.commit_control_body_transaction(transaction);
        Ok(())
    }

    /// Builds and appends a switch through a temporary Python case builder.
    fn switch<'py>(
        slf: PyRefMut<'py, Self>,
        target: PyClassicalExpr,
        build: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.inner
            .validate_classical_expr(&target.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        let transaction = slf.inner.begin_control_body_transaction();
        let py = slf.py();
        let circuit: Py<Self> = slf.into();
        let builder = Py::new(py, PySwitchBuilder::new(circuit.clone_ref(py), transaction))?;

        if let Err(error) = build.call1((builder.bind(py),)) {
            let mut builder = builder.bind(py).borrow_mut();
            builder.closed = true;
            if let Some(transaction) = builder.transaction.take() {
                builder
                    .circuit
                    .bind(py)
                    .borrow_mut()
                    .inner
                    .rollback_control_body_transaction(transaction);
            }
            return Err(error);
        }

        let (transaction, cases, default) = {
            let mut builder = builder.bind(py).borrow_mut();
            builder.closed = true;
            (
                builder.transaction.take().ok_or_else(|| {
                    PyCircuitError::new_err("switch builder transaction is unavailable")
                })?,
                std::mem::take(&mut builder.cases),
                builder.default.take(),
            )
        };
        let mut circuit = circuit.bind(py).borrow_mut();
        let op = match SwitchOp::new(target.inner, cases, default) {
            Ok(op) => ClassicalControlOp::Switch(op),
            Err(error) => {
                circuit.inner.rollback_control_body_transaction(transaction);
                return Err(PyCircuitError::new_err(error.to_string()));
            }
        };
        if let Err(error) = circuit.inner.append_control(op) {
            circuit.inner.rollback_control_body_transaction(transaction);
            return Err(PyCircuitError::new_err(error.to_string()));
        }
        circuit.inner.commit_control_body_transaction(transaction);
        Ok(())
    }

    /// Appends a break to the nearest enclosing loop or switch callback body.
    fn break_loop(&mut self) -> PyResult<()> {
        self.inner
            .break_loop()
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a continue to the nearest enclosing loop callback body.
    fn continue_loop(&mut self) -> PyResult<()> {
        self.inner
            .continue_loop()
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns one operation with circuit-local parameters resolved.
    ///
    /// Raises `IndexError` when `index` is out of bounds.
    fn operation(&self, index: usize) -> PyResult<PyValueOperation> {
        self.inner
            .index(index)
            .map(PyValueOperation::from)
            .map_err(circuit_error_to_py_err)
    }

    /// Removes one top-level operation and returns it.
    fn remove_operation(&mut self, index: usize) -> PyResult<PyValueOperation> {
        let removed = self
            .inner
            .index(index)
            .map(PyValueOperation::from)
            .map_err(circuit_error_to_py_err)?;
        self.inner
            .remove_operation(index)
            .map_err(circuit_error_to_py_err)?;
        Ok(removed)
    }

    /// Removes multiple top-level operations and returns them in original-index order.
    fn remove_operations(&mut self, indices: Vec<usize>) -> PyResult<Vec<PyValueOperation>> {
        let mut indices = indices;
        indices.sort_unstable();
        indices.dedup();

        let removed = indices
            .iter()
            .copied()
            .map(|index| {
                self.inner
                    .index(index)
                    .map(PyValueOperation::from)
                    .map_err(circuit_error_to_py_err)
            })
            .collect::<PyResult<Vec<_>>>()?;

        self.inner
            .remove_operations(indices)
            .map_err(circuit_error_to_py_err)?;
        Ok(removed)
    }

    #[getter]
    fn operations(&self) -> PyResult<Vec<PyValueOperation>> {
        (0..self.inner.operations().len())
            .map(|index| {
                self.inner
                    .index(index)
                    .map(PyValueOperation::from)
                    .map_err(|error| PyCircuitError::new_err(error.to_string()))
            })
            .collect()
    }

    /// Builds the operation dependency DAG analysis view for this circuit.
    fn dag(&self) -> PyResult<PyCircuitDag> {
        CircuitDag::from_circuit(&self.inner)
            .map(PyCircuitDag::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns the circuit depth (longest ASAP path over qubit wires).
    ///
    /// Every instruction node contributes one layer on the qubits it touches.
    /// Barriers serialize their listed qubits (an empty-qubit barrier is
    /// global). ``CircuitGate`` counts as depth 1 (opaque; call :meth:`decompose`
    /// first to recurse).
    ///
    /// With ``recurse=False`` (default) raises :class:`CircuitError` if the
    /// circuit contains any control-flow operation (``if``/``while``/``for``/
    /// ``switch``/``break``/``continue``). With ``recurse=True`` returns an
    /// estimated depth: ``if``/``switch`` take the max branch, ``while`` counts
    /// its body once, ``for`` with a statically-known unsigned-literal range is
    /// fully unrolled (else counted once).
    #[pyo3(signature = (recurse=false))]
    fn depth(&self, recurse: bool) -> PyResult<usize> {
        self.inner
            .depth(recurse)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a standard gate using its bound parameters.
    #[pyo3(signature = (gate, qubits, label=None))]
    fn append_gate(
        &mut self,
        gate: PyStandardGate,
        qubits: PyIntListOrQubitList,
        label: Option<String>,
    ) -> PyResult<()> {
        self.inner
            .append(
                Instruction::Standard(gate.inner),
                Vec::<Qubit>::from(qubits),
                gate.params.into_iter().map(ParameterValue::from),
                label.as_deref(),
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a multi-controlled gate using its bound parameters.
    #[pyo3(signature = (gate, qubits, label=None))]
    fn append_mc_gate(
        &mut self,
        gate: PyMcGate,
        qubits: PyIntListOrQubitList,
        label: Option<String>,
    ) -> PyResult<()> {
        self.inner
            .append(
                Instruction::McGate(Box::new(gate.inner)),
                Vec::<Qubit>::from(qubits),
                gate.params.into_iter().map(ParameterValue::from),
                label.as_deref(),
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a custom unitary gate with positional parameter bindings.
    #[pyo3(signature = (gate, qubits, params=None))]
    fn append_unitary_gate(
        &mut self,
        gate: PyUnitaryGate,
        qubits: PyIntListOrQubitList,
        params: Option<Vec<PyParamLike>>,
    ) -> PyResult<()> {
        let params = params
            .unwrap_or_default()
            .into_iter()
            .map(PyParamLike::into_value)
            .collect::<PyResult<Vec<_>>>()?;
        self.inner
            .unitary_with_params(gate.into(), qubits.into(), params)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a circuit-defined gate with positional parameter bindings.
    #[pyo3(signature = (gate, qubits, params=None))]
    fn append_circuit_gate(
        &mut self,
        gate: PyCircuitGate,
        qubits: PyIntListOrQubitList,
        params: Option<Vec<PyParamLike>>,
    ) -> PyResult<()> {
        let params = params
            .unwrap_or_default()
            .into_iter()
            .map(PyParamLike::into_value)
            .collect::<PyResult<Vec<_>>>()?;
        self.inner
            .circuit_gate(gate.inner, qubits.into(), params)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends an identity gate.
    fn i(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .i(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a Hadamard gate.
    fn h(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .h(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn x(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .x(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn y(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .y(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn z(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .z(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn x2p(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .x2p(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn x2m(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .x2m(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn y2p(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .y2p(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn y2m(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .y2m(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn xy(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .xy(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn xy2p(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .xy2p(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn xy2m(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .xy2m(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn s(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .s(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn sdg(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .sdg(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn t(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .t(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn tdg(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .tdg(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends an RX gate.
    fn rx(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .rx(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn ry(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .ry(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn rz(&mut self, qubit: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .rz(qubit.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn phase(&mut self, qubit: PyIntOrQubit, lambda: PyParamLike) -> PyResult<()> {
        self.inner
            .phase(qubit.into(), lambda.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends a general single-qubit U gate.
    fn u(
        &mut self,
        qubit: PyIntOrQubit,
        theta: PyParamLike,
        phi: PyParamLike,
        lambda: PyParamLike,
    ) -> PyResult<()> {
        self.inner
            .u(
                qubit.into(),
                theta.into_value()?,
                phi.into_value()?,
                lambda.into_value()?,
            )
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn cx(&mut self, control: PyIntOrQubit, target: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .cx(control.into(), target.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn cy(&mut self, control: PyIntOrQubit, target: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .cy(control.into(), target.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn cz(&mut self, control: PyIntOrQubit, target: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .cz(control.into(), target.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn swap(&mut self, a: PyIntOrQubit, b: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .swap(a.into(), b.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn ccx(
        &mut self,
        control1: PyIntOrQubit,
        control2: PyIntOrQubit,
        target: PyIntOrQubit,
    ) -> PyResult<()> {
        self.inner
            .ccx(control1.into(), control2.into(), target.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn rxx(&mut self, a: PyIntOrQubit, b: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .rxx(a.into(), b.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn ryy(&mut self, a: PyIntOrQubit, b: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .ryy(a.into(), b.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn rzz(&mut self, a: PyIntOrQubit, b: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .rzz(a.into(), b.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn rzx(&mut self, a: PyIntOrQubit, b: PyIntOrQubit, theta: PyParamLike) -> PyResult<()> {
        self.inner
            .rzx(a.into(), b.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn crx(
        &mut self,
        control: PyIntOrQubit,
        target: PyIntOrQubit,
        theta: PyParamLike,
    ) -> PyResult<()> {
        self.inner
            .crx(control.into(), target.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn cry(
        &mut self,
        control: PyIntOrQubit,
        target: PyIntOrQubit,
        theta: PyParamLike,
    ) -> PyResult<()> {
        self.inner
            .cry(control.into(), target.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn crz(
        &mut self,
        control: PyIntOrQubit,
        target: PyIntOrQubit,
        theta: PyParamLike,
    ) -> PyResult<()> {
        self.inner
            .crz(control.into(), target.into(), theta.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn fsim(
        &mut self,
        a: PyIntOrQubit,
        b: PyIntOrQubit,
        theta: PyParamLike,
        phi: PyParamLike,
    ) -> PyResult<()> {
        self.inner
            .fsim(a.into(), b.into(), theta.into_value()?, phi.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn rxy(&mut self, qubit: PyIntOrQubit, theta: PyParamLike, phi: PyParamLike) -> PyResult<()> {
        self.inner
            .rxy(qubit.into(), theta.into_value()?, phi.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Inserts a compiler barrier.
    ///
    /// A non-empty list constrains optimization and reordering on exactly those qubits. An empty
    /// list creates a global barrier over every qubit in this circuit. Barriers have no physical
    /// effect on the quantum state.
    fn barrier(&mut self, qubits: PyIntListOrQubitList) -> PyResult<()> {
        self.inner
            .barrier(qubits.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn reset(&mut self, qubit: PyIntOrQubit) -> PyResult<()> {
        self.inner
            .reset(qubit.into())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn delay(&mut self, qubit: PyIntOrQubit, duration: PyParamLike) -> PyResult<()> {
        let qubit: Qubit = qubit.into();
        self.inner
            .delay(qubit, duration.into_value()?)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Allocates a mutable classical variable owned by this circuit.
    fn var(&mut self, ty: PyClassicalType) -> PyClassicalVar {
        self.inner.var(ty.inner).into()
    }

    fn store(&mut self, target: PyClassicalVar, value: PyClassicalExpr) -> PyResult<()> {
        self.inner
            .store(target.inner, value.inner)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn measure(&mut self, qubit: PyIntOrQubit) -> PyResult<PyMeasurement> {
        self.inner
            .measure(qubit.into())
            .map(|inner| PyMeasurement { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn measure_bits(&mut self, qubits: PyIntListOrQubitList) -> PyResult<PyMeasurement> {
        self.inner
            .measure_bits(Vec::from(qubits))
            .map(|inner| PyMeasurement { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn measure_into(
        &mut self,
        qubit: PyIntOrQubit,
        target: PyClassicalVar,
    ) -> PyResult<PyMeasurement> {
        self.inner
            .measure_into(qubit.into(), target.inner)
            .map(|inner| PyMeasurement { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn measure_bits_into(
        &mut self,
        qubits: PyIntListOrQubitList,
        target: PyClassicalVar,
    ) -> PyResult<PyMeasurement> {
        self.inner
            .measure_bits_into(Vec::from(qubits), target.inner)
            .map(|inner| PyMeasurement { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns an inverse circuit when every operation is reversible.
    fn inverse(&self, py: Python<'_>) -> PyResult<Self> {
        py.detach(|| self.inner.inverse())
            .map(Self::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Recursively expands circuit-defined gates.
    fn decompose(&self, py: Python<'_>) -> PyResult<Self> {
        py.detach(|| self.inner.decompose())
            .map(Self::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns a reusable circuit-defined gate.
    fn to_gate(&self, name: String) -> PyResult<PyCircuitGate> {
        match self
            .inner
            .clone()
            .to_gate(name)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?
        {
            Instruction::CircuitGate(gate) => Ok(PyCircuitGate::from(*gate)),
            _ => unreachable!("Circuit::to_gate always returns a circuit gate"),
        }
    }

    /// Returns a new circuit with supplied symbols numerically bound.
    #[pyo3(signature = (bindings=None))]
    fn assign_parameters(&self, bindings: Option<HashMap<String, f64>>) -> PyResult<Self> {
        if let Some(value) = bindings
            .as_ref()
            .and_then(|bindings| bindings.values().find(|value| !value.is_finite()))
        {
            return Err(PyParameterError::new_err(
                ParameterError::DomainError(format!(
                    "parameter binding must be finite, got {value}"
                ))
                .to_string(),
            ));
        }
        let bindings = bindings.as_ref().map(|bindings| {
            bindings
                .iter()
                .map(|(name, value)| (name.as_str(), *value))
                .collect::<HashMap<_, _>>()
        });
        self.inner
            .assign_parameters(&bindings)
            .map(Self::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Appends another circuit, optionally remapping its qubits.
    #[pyo3(signature = (other, qubits=None))]
    fn compose(&mut self, other: &PyCircuit, qubits: Option<PyIntListOrQubitList>) -> PyResult<()> {
        let qubits = qubits.map(Vec::from);
        self.inner
            .compose(&other.inner, qubits.as_deref())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Computes the dense numeric unitary matrix.
    #[pyo3(signature = (qubits_order=None))]
    fn to_matrix<'py>(
        &self,
        py: Python<'py>,
        qubits_order: Option<Vec<usize>>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let matrix = py
            .detach(|| self.inner.to_matrix(qubits_order.as_deref()))
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        Ok(matrix.to_pyarray(py))
    }

    /// Computes a dense unitary matrix while preserving symbolic parameters.
    #[pyo3(signature = (qubits_order=None))]
    fn to_symbolic_matrix(
        &self,
        py: Python<'_>,
        qubits_order: Option<Vec<usize>>,
    ) -> PyResult<PySymbolicMatrix> {
        py.detach(|| circuit_to_symbolic_matrix(&self.inner, qubits_order.as_deref()))
            .map(PySymbolicMatrix::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Validates classical ownership, dominance, and control-flow invariants.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Compares two circuits by structural equality.
    ///
    /// Circuit identity, the order of the interned parameter table, and other
    /// process-local classical handles are ignored; qubits, resolved
    /// parameters, and the ordered operation sequence must match. Cost is
    /// linear in the circuit's IR size. Circuits are unhashable.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyCircuit>() {
            return Ok(false);
        }
        let other = other.extract::<PyRef<'_, PyCircuit>>()?;
        Ok(self.inner == other.inner)
    }

    fn __len__(&self) -> usize {
        self.inner.operations().len()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __getitem__(&self, index: isize) -> PyResult<PyValueOperation> {
        let len = self.inner.operations().len();
        let resolved_index = if index < 0 {
            len.checked_add_signed(index)
        } else {
            usize::try_from(index).ok()
        };
        match resolved_index {
            Some(index) if index < len => self.operation(index),
            _ => Err(PyIndexError::new_err(format!(
                "operation index {index} out of bounds for circuit with {len} operations"
            ))),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Circuit(id={}, qubits={}, operations={})",
            self.inner.id(),
            self.inner.num_qubits(),
            self.inner.operations().len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::{
        ClassicalExpr, Qubit, StandardGate, UnitaryGate, ValueClassicalControlOp, ValueControlBody,
    };

    #[test]
    fn repr_contains_circuit_identity_and_shape() {
        let circuit = PyCircuit::from(Circuit::new(2));
        let repr = circuit.__repr__();
        assert!(repr.contains(&circuit.inner.id().to_string()));
        assert!(repr.contains("qubits=2"));
        assert!(repr.contains("operations=0"));
    }

    #[test]
    fn operation_returns_value_level_parameters() {
        let mut circuit = Circuit::new(1);
        circuit
            .rx(Qubit::new(0), Parameter::symbol("theta"))
            .unwrap();
        let operation = PyCircuit::from(circuit).operation(0).unwrap();
        assert!(matches!(
            operation.inner.params.first(),
            Some(ParameterValue::Param(parameter)) if parameter.as_symbol().as_deref() == Some("theta")
        ));
    }

    #[test]
    fn xy_family_matches_core_builders() {
        let mut circuit = PyCircuit::from(Circuit::new(1));
        circuit
            .xy(PyIntOrQubit::Int(0), PyParamLike::Float(0.1))
            .unwrap();
        circuit
            .xy2p(PyIntOrQubit::Int(0), PyParamLike::Float(0.2))
            .unwrap();
        circuit
            .xy2m(PyIntOrQubit::Int(0), PyParamLike::Float(0.3))
            .unwrap();

        let operations = circuit.inner.operations();
        assert!(matches!(
            operations[0].instruction,
            Instruction::Standard(StandardGate::XY)
        ));
        assert!(matches!(
            operations[1].instruction,
            Instruction::Standard(StandardGate::XY2P)
        ));
        assert!(matches!(
            operations[2].instruction,
            Instruction::Standard(StandardGate::XY2M)
        ));
        assert_eq!(circuit.inner.parameters().len(), 0);
    }

    #[test]
    fn append_unitary_gate_preserves_parameter_contract() {
        let mut circuit = PyCircuit::from(Circuit::new(1));
        let gate = PyUnitaryGate::from(UnitaryGate::new("custom", 1, 1));
        circuit
            .append_unitary_gate(
                gate,
                PyIntListOrQubitList::IntList(vec![0]),
                Some(vec![PyParamLike::Parameter(PyParameter::from(
                    Parameter::symbol("theta"),
                ))]),
            )
            .unwrap();

        let operation = circuit.operation(0).unwrap();
        assert!(matches!(
            operation.inner.params.first(),
            Some(ParameterValue::Param(parameter)) if parameter.as_symbol().as_deref() == Some("theta")
        ));
    }

    #[test]
    fn append_control_uses_value_level_control_flow() {
        let mut circuit = PyCircuit::from(Circuit::new(1));
        circuit
            .append_control(PyClassicalControlOp::from(ValueClassicalControlOp::If {
                condition: ClassicalExpr::bool_literal(true),
                then_body: ValueControlBody::new(vec![]),
                else_body: None,
            }))
            .unwrap();

        assert_eq!(circuit.inner.operations().len(), 1);
        assert!(
            circuit
                .operation(0)
                .unwrap()
                .inner
                .instruction
                .is_classical_control()
        );
    }

    #[test]
    fn symbolic_matrix_preserves_unbound_parameters() {
        Python::initialize();
        let mut circuit = Circuit::new(1);
        circuit
            .rx(Qubit::new(0), Parameter::symbol("theta"))
            .unwrap();

        let matrix = Python::attach(|py| {
            PyCircuit::from(circuit)
                .to_symbolic_matrix(py, None)
                .unwrap()
        });

        assert!(matrix.inner.iter().any(|value| {
            value.re.get_symbols().contains("theta") || value.im.get_symbols().contains("theta")
        }));
        assert_eq!(matrix.inner.dim(), (2, 2));
    }
}
