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

//! Python wrapper for runtime classical data operations.

use crate::circuit::{PyClassicalExpr, PyClassicalValue, PyClassicalVar};
use cqlib_core::circuit::gate::ClassicalDataOp;
use pyo3::prelude::*;

/// Runtime classical data operation: a store into a mutable classical
/// variable, or a measurement producing an immutable classical value.
///
/// Instances are obtained from `Instruction.classical_data` /
/// `ValueInstruction.classical_data`; they are not constructed directly.
#[pyclass(name = "ClassicalDataOp", module = "cqlib.circuit", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyClassicalDataOp {
    pub(crate) inner: ClassicalDataOp,
}

impl From<ClassicalDataOp> for PyClassicalDataOp {
    fn from(inner: ClassicalDataOp) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyClassicalDataOp {
    /// Returns the operation kind: `'store'`, `'measure_bit'`, or
    /// `'measure_bits'`.
    #[getter]
    pub fn kind(&self) -> &'static str {
        self.inner.name()
    }

    /// Returns the store target variable if this is a store operation,
    /// None otherwise.
    #[getter]
    pub fn target(&self) -> Option<PyClassicalVar> {
        self.inner.target().map(PyClassicalVar::from)
    }

    /// Returns the stored expression if this is a store operation,
    /// None otherwise.
    #[getter]
    pub fn value(&self) -> Option<PyClassicalExpr> {
        self.inner.value().cloned().map(PyClassicalExpr::from)
    }

    /// Returns the immutable value receiving the result if this is a
    /// measurement operation, None otherwise.
    #[getter]
    pub fn result(&self) -> Option<PyClassicalValue> {
        self.inner.result().map(PyClassicalValue::from)
    }

    fn __repr__(&self) -> String {
        format!("ClassicalDataOp({})", self.inner.name())
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyClassicalDataOp>() {
            return Ok(false);
        }
        let other = other.extract::<PyClassicalDataOp>()?;
        Ok(self.inner == other.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}
