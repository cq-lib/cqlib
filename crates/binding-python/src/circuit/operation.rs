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

//! Python bindings for circuit operations.
//!
//! [`PyValueOperation`] is the public, self-contained construction boundary
//! and the only operation type registered on `cqlib.circuit`.
//! [`PyOperation`] wraps circuit-local storage IR and is used internally for
//! testing; it exposes the indexed-parameter form used by `Circuit` storage.
//! New operations should be built via [`PyValueOperation`].

use crate::circuit::bit::PyQubit;
use crate::circuit::control_flow::PyClassicalControlOp;
use crate::circuit::error::{CircuitError as PyCircuitError, ParameterError as PyParameterError};
use crate::circuit::gate::{PyMcGate, PyStandardGate};
use crate::circuit::instruction::{PyInstruction, PyValueInstruction};
use crate::circuit::parameter::PyParameter;
use cqlib_core::circuit::circuit_param::{CircuitParam, ParameterValue};
use cqlib_core::circuit::error::ParameterError;
use cqlib_core::circuit::operation::{Operation, ValueOperation};
use numpy::ToPyArray;
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;

pub(crate) fn extract_parameter_value(value: &Bound<'_, PyAny>) -> PyResult<ParameterValue> {
    if let Ok(param) = value.extract::<PyParameter>() {
        return Ok(ParameterValue::Param(param.inner));
    }
    if let Ok(value) = value.extract::<f64>() {
        if value.is_finite() {
            return Ok(ParameterValue::Fixed(value));
        }
        return Err(PyParameterError::new_err(
            ParameterError::DomainError(format!("operation parameter must be finite, got {value}"))
                .to_string(),
        ));
    }
    Err(PyTypeError::new_err(
        "operation parameter must be a finite float or Parameter",
    ))
}

/// Internal Python wrapper for circuit-local storage IR.
///
/// # Examples
///
/// ```python
/// from cqlib import Circuit
///
/// circuit = Circuit(2)
/// circuit.h(0)
/// circuit.cx(0, 1)
/// circuit.rx(0, 0.5)
///
/// for op in circuit.operations():
///     print(f"Gate: {op.name}, Qubits: {op.num_qubits}")
/// ```
#[pyclass(name = "Operation", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyOperation {
    pub(crate) operation: Operation,
}

impl From<Operation> for PyOperation {
    fn from(operation: Operation) -> Self {
        Self { operation }
    }
}

impl From<PyOperation> for Operation {
    fn from(py: PyOperation) -> Self {
        py.operation
    }
}

#[pymethods]
impl PyOperation {
    /// Returns the instruction (gate type) of this operation.
    ///
    /// The instruction defines what type of gate or operation to apply.
    #[getter]
    fn instruction(&self) -> PyInstruction {
        PyInstruction::from(self.operation.instruction.clone())
    }

    /// Returns the qubits this operation acts on.
    ///
    /// For controlled gates, control qubits usually come first, followed by target qubits.
    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.operation
            .qubits
            .iter()
            .map(|&q| PyQubit::from(q))
            .collect()
    }

    /// Returns the number of qubits this operation acts on.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.operation.qubits.len()
    }

    /// Returns the parameters of this operation.
    ///
    /// Note: The return type is heterogeneous. Each element can be either:
    /// - A fixed float value (e.g., `0.5` for a concrete rotation angle)
    /// - A tuple `("param", index)` representing a reference to a symbolic parameter
    ///   stored in the parent circuit's parameter table. To resolve such references,
    ///   use `circuit.parameters[index]`.
    ///
    /// Example:
    ///     >>> for op in circuit.operations:
    ///     ...     for p in op.params:
    ///     ...         if isinstance(p, tuple) and p[0] == "param":
    ///     ...             param = circuit.parameters[p[1]]  # Resolve symbolic param
    ///     ...         else:
    ///     ...             value = p  # Fixed float value
    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let mut result = Vec::with_capacity(self.operation.params.len());
        for param in &self.operation.params {
            match param {
                CircuitParam::Fixed(val) => {
                    result.push(val.into_pyobject(py)?.into_any().unbind());
                }
                CircuitParam::Index(idx) => {
                    // Return the index as a tuple ("param", idx).
                    // The user can resolve this using circuit.parameters[idx].
                    let tuple = ("param", *idx).into_pyobject(py)?;
                    result.push(tuple.into_any().unbind());
                }
            }
        }
        Ok(result)
    }

    /// Returns the unitary matrix representation of this operation.
    ///
    /// # Returns
    ///
    /// A 2D numpy array (dtype=complex128) representing the unitary matrix.
    ///
    /// # Raises
    ///
    /// CircuitError if the operation is non-unitary (e.g., Measure, Barrier).
    fn matrix(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let matrix_cow = self
            .operation
            .matrix()
            .map_err(|e| PyCircuitError::new_err(e.to_string()))?;
        matrix_cow.to_pyarray(py).into_py_any(py)
    }

    /// Returns the number of parameters.
    #[getter]
    fn num_params(&self) -> usize {
        self.operation.params.len()
    }

    /// Returns the label of this operation, if any.
    #[getter]
    fn label(&self) -> Option<String> {
        self.operation.label.as_ref().map(|s| s.to_string())
    }

    /// Returns the name of the instruction.
    ///
    /// Examples: "h", "cx", "rx", "measure"
    #[getter]
    fn name(&self) -> String {
        format!("{}", self.operation.instruction)
    }

    fn __repr__(&self) -> String {
        format!(
            "Operation({}, qubits={:?}, params={})",
            self.name(),
            self.operation
                .qubits
                .iter()
                .map(|q| q.index())
                .collect::<Vec<_>>(),
            self.operation.params.len()
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.operation.instruction)
    }

    /// Compares two operations by storage-level structure.
    ///
    /// Classical handles embedded in classical-data/control instructions
    /// carry the owning circuit's process-local identity, so operations from
    /// different circuits are generally unequal.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyOperation>() {
            return Ok(false);
        }
        let other = other.extract::<PyRef<'_, PyOperation>>()?;
        Ok(self.operation == other.operation)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

#[pyclass(name = "ValueOperation", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyValueOperation {
    pub(crate) inner: ValueOperation,
}

impl From<ValueOperation> for PyValueOperation {
    fn from(inner: ValueOperation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValueOperation {
    #[new]
    #[pyo3(signature = (instruction, qubits, params=None, label=None))]
    fn new(
        instruction: PyValueInstruction,
        qubits: Vec<PyQubit>,
        params: Option<Vec<Bound<'_, PyAny>>>,
        label: Option<String>,
    ) -> PyResult<Self> {
        let params = params
            .unwrap_or_default()
            .iter()
            .map(extract_parameter_value)
            .collect::<PyResult<_>>()?;
        Ok(Self {
            inner: ValueOperation {
                instruction: instruction.inner,
                qubits: qubits.into_iter().map(|q| q.inner).collect(),
                params,
                label: label.map(Into::into),
            },
        })
    }

    #[staticmethod]
    #[pyo3(signature = (instruction, qubits, params=None, label=None))]
    fn from_instruction(
        instruction: PyInstruction,
        qubits: Vec<PyQubit>,
        params: Option<Vec<Bound<'_, PyAny>>>,
        label: Option<String>,
    ) -> PyResult<Self> {
        Self::new(
            PyValueInstruction::from(cqlib_core::circuit::ValueInstruction::from_instruction(
                instruction.inner,
            )),
            qubits,
            params,
            label,
        )
    }

    /// Creates a value operation while preserving parameters bound to a standard gate.
    #[staticmethod]
    #[pyo3(signature = (gate, qubits, label=None))]
    fn from_standard_gate(
        gate: PyStandardGate,
        qubits: Vec<PyQubit>,
        label: Option<String>,
    ) -> Self {
        Self {
            inner: ValueOperation {
                instruction: cqlib_core::circuit::ValueInstruction::from_instruction(
                    cqlib_core::circuit::Instruction::Standard(gate.inner),
                ),
                qubits: qubits.into_iter().map(|q| q.inner).collect(),
                params: gate.params.into_iter().map(ParameterValue::from).collect(),
                label: label.map(Into::into),
            },
        }
    }

    /// Creates a value operation while preserving parameters bound to an MC gate.
    #[staticmethod]
    #[pyo3(signature = (gate, qubits, label=None))]
    fn from_mc_gate(gate: PyMcGate, qubits: Vec<PyQubit>, label: Option<String>) -> Self {
        Self {
            inner: ValueOperation {
                instruction: cqlib_core::circuit::ValueInstruction::from_instruction(
                    cqlib_core::circuit::Instruction::McGate(Box::new(gate.inner)),
                ),
                qubits: qubits.into_iter().map(|q| q.inner).collect(),
                params: gate.params.into_iter().map(ParameterValue::from).collect(),
                label: label.map(Into::into),
            },
        }
    }

    /// Creates a value operation from construction-time classical control flow.
    #[staticmethod]
    fn from_classical_control(control: PyClassicalControlOp) -> Self {
        let qubits = control.inner.used_qubits().into_iter().collect();
        Self {
            inner: ValueOperation {
                instruction: cqlib_core::circuit::ValueInstruction::ClassicalControl(control.inner),
                qubits,
                params: Default::default(),
                label: None,
            },
        }
    }

    #[getter]
    fn instruction(&self) -> PyValueInstruction {
        PyValueInstruction::from(self.inner.instruction.clone())
    }

    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner
            .qubits
            .iter()
            .copied()
            .map(PyQubit::from)
            .collect()
    }

    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let mut result = Vec::with_capacity(self.inner.params.len());
        for param in &self.inner.params {
            match param {
                ParameterValue::Fixed(value) => {
                    result.push(value.into_pyobject(py)?.into_any().unbind());
                }
                ParameterValue::Param(param) => {
                    result.push(
                        PyParameter {
                            inner: param.clone(),
                        }
                        .into_py_any(py)?,
                    );
                }
            }
        }
        Ok(result)
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name()
    }

    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    #[getter]
    fn num_params(&self) -> usize {
        self.inner.num_params()
    }

    #[getter]
    fn instruction_type(&self) -> &'static str {
        self.inner.instruction_type()
    }

    #[getter]
    fn is_standard(&self) -> bool {
        self.inner.is_standard()
    }

    #[getter]
    fn is_mcgate(&self) -> bool {
        self.inner.is_mcgate()
    }

    #[getter]
    fn is_unitary(&self) -> bool {
        self.inner.is_unitary()
    }

    #[getter]
    fn is_circuit_gate(&self) -> bool {
        self.inner.is_circuit_gate()
    }

    #[getter]
    fn is_directive(&self) -> bool {
        self.inner.is_directive()
    }

    #[getter]
    fn is_classical_data(&self) -> bool {
        self.inner.is_classical_data()
    }

    #[getter]
    fn is_classical_control(&self) -> bool {
        self.inner.is_classical_control()
    }

    #[getter]
    fn is_delay(&self) -> bool {
        self.inner.is_delay()
    }

    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label.as_ref().map(|s| s.to_string())
    }

    fn matrix(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let operation = Operation {
            instruction: self
                .inner
                .instruction
                .clone()
                .into_instruction()
                .ok_or_else(|| {
                    PyValueError::new_err(
                        "classical control value operations do not have a matrix",
                    )
                })?,
            qubits: self.inner.qubits.clone(),
            params: self
                .inner
                .params
                .iter()
                .map(|param| match param {
                    ParameterValue::Fixed(value) => Ok(CircuitParam::Fixed(*value)),
                    ParameterValue::Param(_) => Err(PyValueError::new_err(
                        "symbolic value operation parameters cannot be converted without a circuit parameter table",
                    )),
                })
                .collect::<PyResult<_>>()?,
            label: self.inner.label.clone(),
        };
        let matrix_cow = operation
            .matrix()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        matrix_cow.to_pyarray(py).into_py_any(py)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("ValueOperation(\"{}\")", self.inner)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyValueOperation>() {
            return Ok(false);
        }
        let other = other.extract::<PyValueOperation>()?;
        Ok(self.inner == other.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::{Parameter, StandardGate};

    #[test]
    fn standard_gate_factory_preserves_bound_parameters() {
        let operation = PyValueOperation::from_standard_gate(
            PyStandardGate::from(StandardGate::RX, vec![Parameter::symbol("theta")]),
            vec![PyQubit::from(cqlib_core::circuit::Qubit::new(0))],
            None,
        );

        assert!(matches!(
            operation.inner.params.first(),
            Some(ParameterValue::Param(parameter)) if parameter.as_symbol().as_deref() == Some("theta")
        ));
    }
}
