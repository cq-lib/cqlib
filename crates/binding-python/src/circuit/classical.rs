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

//! Python bindings for runtime classical types and circuit-local handles.
//!
//! These wrappers preserve the ownership and static-type model of `cqlib-core`:
//! variables and immutable values carry a [`CircuitId`], while a
//! [`Measurement`] combines an immutable result handle with measured qubit order.

use crate::circuit::bit::PyQubit;
use crate::circuit::classical_expr::PyClassicalExpr;
use crate::circuit::error::CircuitError as PyCircuitError;
use crate::device::result::PyOutcome;
use crate::utils::hash_value;
use cqlib_core::circuit::{
    CircuitError, CircuitId, ClassicalType, ClassicalValue, ClassicalVar, Measurement,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use smallvec::SmallVec;

/// Process-local identity shared by classical handles owned by one circuit.
#[pyclass(name = "CircuitId", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyCircuitId {
    pub(crate) inner: CircuitId,
}

#[pymethods]
impl PyCircuitId {
    /// Allocates a fresh process-local circuit identity.
    #[new]
    fn new() -> Self {
        Self {
            inner: CircuitId::new(),
        }
    }

    /// Returns a representation containing the allocated identity.
    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    /// Returns the same stable display form as `repr`.
    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}

impl From<CircuitId> for PyCircuitId {
    fn from(inner: CircuitId) -> Self {
        Self { inner }
    }
}

/// Static type of a runtime classical expression or storage location.
#[pyclass(name = "ClassicalType", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyClassicalType {
    pub(crate) inner: ClassicalType,
}

#[pymethods]
impl PyClassicalType {
    /// Returns the one-bit measurement type.
    #[staticmethod]
    fn bit() -> Self {
        Self {
            inner: ClassicalType::Bit,
        }
    }

    /// Returns the logical boolean type used by control-flow predicates.
    #[staticmethod]
    fn bool() -> Self {
        Self {
            inner: ClassicalType::Bool,
        }
    }

    /// Creates a fixed-width unsigned integer type.
    #[staticmethod]
    fn uint(width: u32) -> PyResult<Self> {
        ClassicalType::uint(width)
            .map(|inner| Self { inner })
            .ok_or_else(|| {
                PyCircuitError::new_err(
                    CircuitError::InvalidOperation("UInt width must be non-zero".to_string())
                        .to_string(),
                )
            })
    }

    /// Creates a fixed-width ordered bit-vector type.
    #[staticmethod]
    fn bit_vec(width: u32) -> PyResult<Self> {
        ClassicalType::bit_vec(width)
            .map(|inner| Self { inner })
            .ok_or_else(|| {
                PyCircuitError::new_err(
                    CircuitError::InvalidOperation("BitVec width must be non-zero".to_string())
                        .to_string(),
                )
            })
    }

    /// Returns the number of bits represented by this type.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Returns the zero literal for this type.
    fn zero_literal(&self) -> PyResult<PyClassicalExpr> {
        self.inner
            .zero_literal()
            .map(|inner| PyClassicalExpr { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns the one literal for this type.
    fn one_literal(&self) -> PyResult<PyClassicalExpr> {
        self.inner
            .one_literal()
            .map(|inner| PyClassicalExpr { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    fn __repr__(&self) -> String {
        match self.inner {
            ClassicalType::Bit => "ClassicalType.bit()".to_string(),
            ClassicalType::Bool => "ClassicalType.bool()".to_string(),
            ClassicalType::UInt(width) => format!("ClassicalType.uint({})", width.get()),
            ClassicalType::BitVec(width) => format!("ClassicalType.bit_vec({})", width.get()),
        }
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}

impl From<ClassicalType> for PyClassicalType {
    fn from(inner: ClassicalType) -> Self {
        Self { inner }
    }
}

/// Circuit-local handle to mutable runtime classical storage.
#[pyclass(name = "ClassicalVar", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyClassicalVar {
    pub(crate) inner: ClassicalVar,
}

#[pymethods]
impl PyClassicalVar {
    /// Creates an explicit handle for low-level IR construction.
    ///
    /// Related handles must use the same `circuit_id`.
    #[new]
    fn new(circuit_id: PyCircuitId, index: u32, ty: PyClassicalType) -> Self {
        Self {
            inner: ClassicalVar::new(circuit_id.inner, index, ty.inner),
        }
    }

    #[getter]
    fn id(&self) -> u32 {
        self.inner.id()
    }

    #[getter]
    fn index(&self) -> u32 {
        self.inner.index()
    }

    #[getter]
    fn circuit_id(&self) -> PyCircuitId {
        self.inner.circuit_id().into()
    }

    #[getter]
    fn ty(&self) -> PyClassicalType {
        self.inner.ty().into()
    }

    /// Returns an expression that reads this variable.
    fn expr(&self) -> PyClassicalExpr {
        PyClassicalExpr {
            inner: self.inner.expr(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ClassicalVar({}, {}, {:?})",
            self.inner.circuit_id(),
            self.inner.index(),
            self.inner.ty()
        )
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}

impl From<ClassicalVar> for PyClassicalVar {
    fn from(inner: ClassicalVar) -> Self {
        Self { inner }
    }
}

/// Circuit-local handle to an immutable runtime classical value.
#[pyclass(name = "ClassicalValue", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyClassicalValue {
    pub(crate) inner: ClassicalValue,
}

#[pymethods]
impl PyClassicalValue {
    /// Creates an explicit immutable value handle for low-level IR construction.
    #[new]
    fn new(circuit_id: PyCircuitId, index: u32, ty: PyClassicalType) -> Self {
        Self {
            inner: ClassicalValue::new(circuit_id.inner, index, ty.inner),
        }
    }

    #[getter]
    fn index(&self) -> u32 {
        self.inner.index()
    }

    #[getter]
    fn circuit_id(&self) -> PyCircuitId {
        self.inner.circuit_id().into()
    }

    #[getter]
    fn ty(&self) -> PyClassicalType {
        self.inner.ty().into()
    }

    /// Returns an expression that reads this immutable value.
    fn expr(&self) -> PyClassicalExpr {
        PyClassicalExpr {
            inner: self.inner.expr(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ClassicalValue({}, {}, {:?})",
            self.inner.circuit_id(),
            self.inner.index(),
            self.inner.ty()
        )
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }
}

impl From<ClassicalValue> for PyClassicalValue {
    fn from(inner: ClassicalValue) -> Self {
        Self { inner }
    }
}

/// Measurement receipt containing its immutable result and measured qubit order.
#[pyclass(name = "Measurement", module = "cqlib.circuit", skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyMeasurement {
    pub(crate) inner: Measurement,
}

#[pymethods]
impl PyMeasurement {
    /// Creates a receipt from an existing result handle and measured qubits.
    #[new]
    fn new(value: PyClassicalValue, qubits: Vec<PyQubit>) -> Self {
        let qubits: SmallVec<[cqlib_core::circuit::Qubit; 3]> =
            qubits.into_iter().map(|q| q.inner).collect();
        Self {
            inner: Measurement::new(value.inner, qubits),
        }
    }

    /// Returns the immutable measurement result.
    #[getter]
    fn value(&self) -> PyClassicalValue {
        self.inner.value().into()
    }

    /// Returns measured qubits in result-bit order.
    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner
            .qubits()
            .iter()
            .copied()
            .map(PyQubit::from)
            .collect()
    }

    /// Returns the result's static classical type.
    #[getter]
    fn ty(&self) -> PyClassicalType {
        self.inner.ty().into()
    }

    /// Returns the number of measured bits.
    #[getter]
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Returns an expression that reads the measurement result.
    fn expr(&self) -> PyClassicalExpr {
        PyClassicalExpr {
            inner: self.inner.expr(),
        }
    }

    /// Checks that all measured qubits are valid for a state with `num_qubits`.
    fn check_qubits(&self, num_qubits: usize) -> PyResult<()> {
        self.inner
            .check_qubits(num_qubits)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    /// Projects a full-register outcome onto this measurement's qubit order.
    fn project(&self, full: PyOutcome) -> PyOutcome {
        self.inner.project(&full.inner).into()
    }

    /// Projects a computational-basis index onto this measurement's qubit order.
    fn project_basis(&self, basis: usize) -> PyOutcome {
        self.inner.project_basis(basis).into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Measurement(value={:?}, qubits={})",
            self.inner.value(),
            self.inner.qubits().len()
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[cfg(test)]
mod tests {
    use super::{PyCircuitId, PyClassicalType, PyClassicalValue, PyClassicalVar, PyMeasurement};
    use crate::circuit::bit::PyQubit;
    use crate::device::result::PyOutcome;
    use cqlib_core::circuit::Qubit;
    use cqlib_core::device::Outcome;

    #[test]
    fn circuit_id_repr_contains_the_allocated_identity() {
        let id = PyCircuitId::new();

        assert_eq!(id.__repr__(), id.inner.to_string());
        assert_ne!(id.__repr__(), "CircuitId()");
    }

    #[test]
    fn related_handles_keep_the_supplied_circuit_identity() {
        let circuit_id = PyCircuitId::new();
        let ty = PyClassicalType::bit();
        let var = PyClassicalVar::new(circuit_id, 0, ty);
        let value = PyClassicalValue::new(circuit_id, 0, ty);

        assert_eq!(var.inner.circuit_id(), circuit_id.inner);
        assert_eq!(value.inner.circuit_id(), circuit_id.inner);
        assert!(var.__repr__().contains(&circuit_id.inner.to_string()));
        assert!(value.__repr__().contains(&circuit_id.inner.to_string()));
    }

    #[test]
    fn measurement_projects_outcomes_through_python_wrapper() {
        let circuit_id = PyCircuitId::new();
        let value = PyClassicalValue::new(circuit_id, 0, PyClassicalType::uint(2).unwrap());
        let measurement = PyMeasurement::new(
            value,
            vec![PyQubit::from(Qubit::new(1)), PyQubit::from(Qubit::new(0))],
        );
        let full = PyOutcome::from(Outcome::from_indices(2, [0]));

        assert_eq!(measurement.project(full).inner.to_bitstring(2), "10");
        assert_eq!(measurement.project_basis(0b10).inner.to_bitstring(2), "01");
        assert!(measurement.check_qubits(2).is_ok());
        assert!(measurement.check_qubits(1).is_err());
    }
}
