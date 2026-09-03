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

//! Python bindings for runtime classical state produced by QIS simulators.

use crate::circuit::classical::{PyClassicalType, PyClassicalValue, PyClassicalVar};
use crate::device::result::PyOutcome;
use crate::utils::hash_value;
use cqlib_core::qis::state::{ClassicalState, RuntimeValue};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

/// A typed runtime classical value produced during circuit execution.
#[pyclass(name = "RuntimeValue", module = "cqlib.qis.state", skip_from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyRuntimeValue {
    pub(crate) inner: RuntimeValue,
}

impl From<RuntimeValue> for PyRuntimeValue {
    fn from(inner: RuntimeValue) -> Self {
        Self { inner }
    }
}

impl From<PyRuntimeValue> for RuntimeValue {
    fn from(value: PyRuntimeValue) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyRuntimeValue {
    /// Returns the runtime value kind.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            RuntimeValue::Bit(_) => "bit",
            RuntimeValue::Bool(_) => "bool",
            RuntimeValue::UInt { .. } => "uint",
            RuntimeValue::BitVec { .. } => "bit_vec",
        }
    }

    /// Returns the static classical type represented by this runtime value.
    #[getter]
    fn ty(&self) -> PyClassicalType {
        self.inner.ty().into()
    }

    /// Returns a bit string for Bit and BitVec values.
    fn to_bitstring(&self) -> Option<String> {
        self.inner.to_bitstring()
    }

    /// Returns this value as a bit.
    fn as_bit(&self) -> PyResult<bool> {
        match &self.inner {
            RuntimeValue::Bit(value) => Ok(*value),
            _ => Err(PyTypeError::new_err("runtime value is not a Bit")),
        }
    }

    /// Returns this value as a logical boolean.
    fn as_bool(&self) -> PyResult<bool> {
        match &self.inner {
            RuntimeValue::Bool(value) => Ok(*value),
            _ => Err(PyTypeError::new_err("runtime value is not a Bool")),
        }
    }

    /// Returns this value as an unsigned integer.
    fn as_uint(&self) -> PyResult<u128> {
        match &self.inner {
            RuntimeValue::UInt { value, .. } => Ok(*value),
            _ => Err(PyTypeError::new_err("runtime value is not a UInt")),
        }
    }

    /// Returns this value as a bit-vector outcome.
    fn as_bitvec_outcome(&self) -> PyResult<PyOutcome> {
        match &self.inner {
            RuntimeValue::BitVec { bits, .. } => Ok(bits.clone().into()),
            _ => Err(PyTypeError::new_err("runtime value is not a BitVec")),
        }
    }

    /// Compares two runtime values by value: same kind, width, and payload.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyRuntimeValue>() {
            return Ok(false);
        }
        let other = other.extract::<PyRef<'_, PyRuntimeValue>>()?;
        Ok(self.inner == other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RuntimeValue::Bit(value) => format!("RuntimeValue.bit({})", value),
            RuntimeValue::Bool(value) => format!("RuntimeValue.bool({})", value),
            RuntimeValue::UInt { width, value } => {
                format!("RuntimeValue.uint(width={}, value={})", width, value)
            }
            RuntimeValue::BitVec { width, bits } => format!(
                "RuntimeValue.bit_vec(width={}, bits='{}')",
                width,
                bits.to_bitstring(*width as usize)
            ),
        }
    }
}

/// Runtime classical state produced while executing a circuit.
#[pyclass(
    name = "ClassicalState",
    module = "cqlib.qis.state",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyClassicalState {
    pub(crate) inner: ClassicalState,
}

impl From<ClassicalState> for PyClassicalState {
    fn from(inner: ClassicalState) -> Self {
        Self { inner }
    }
}

impl From<PyClassicalState> for ClassicalState {
    fn from(value: PyClassicalState) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyClassicalState {
    /// Returns the runtime value produced for an immutable circuit value.
    fn value(&self, value: PyClassicalValue) -> Option<PyRuntimeValue> {
        self.inner
            .value(value.inner)
            .cloned()
            .map(PyRuntimeValue::from)
    }

    /// Returns the current runtime value of a mutable classical variable.
    fn var(&self, var: PyClassicalVar) -> Option<PyRuntimeValue> {
        self.inner.var(var.inner).cloned().map(PyRuntimeValue::from)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        "ClassicalState()".to_string()
    }
}
