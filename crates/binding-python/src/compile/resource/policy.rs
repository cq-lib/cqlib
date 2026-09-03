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

use cqlib_core::compile::resource::{ResourceLimits, ResourcePolicy};
use pyo3::prelude::*;

/// Python wrapper for ancillary-resource permissions.
#[pyclass(
    name = "ResourcePolicy",
    module = "cqlib.compile.resource",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyResourcePolicy {
    pub(crate) inner: ResourcePolicy,
}

impl From<ResourcePolicy> for PyResourcePolicy {
    fn from(inner: ResourcePolicy) -> Self {
        Self { inner }
    }
}

impl From<PyResourcePolicy> for ResourcePolicy {
    fn from(value: PyResourcePolicy) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourcePolicy {
    #[new]
    #[pyo3(signature = (*, max_pre_layout_clean_ancillas=0, allow_dirty_borrowing=false))]
    fn new(max_pre_layout_clean_ancillas: usize, allow_dirty_borrowing: bool) -> Self {
        Self {
            inner: ResourcePolicy {
                max_pre_layout_clean_ancillas,
                allow_dirty_borrowing,
            },
        }
    }

    #[getter]
    fn max_pre_layout_clean_ancillas(&self) -> usize {
        self.inner.max_pre_layout_clean_ancillas
    }

    #[getter]
    fn allow_dirty_borrowing(&self) -> bool {
        self.inner.allow_dirty_borrowing
    }

    fn __repr__(&self) -> String {
        let allow_dirty_borrowing = if self.inner.allow_dirty_borrowing {
            "True"
        } else {
            "False"
        };
        format!(
            "ResourcePolicy(max_pre_layout_clean_ancillas={}, allow_dirty_borrowing={})",
            self.inner.max_pre_layout_clean_ancillas, allow_dirty_borrowing
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Python wrapper for hard target-derived resource limits.
#[pyclass(
    name = "ResourceLimits",
    module = "cqlib.compile.resource",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyResourceLimits {
    pub(crate) inner: ResourceLimits,
}

impl From<ResourceLimits> for PyResourceLimits {
    fn from(inner: ResourceLimits) -> Self {
        Self { inner }
    }
}

impl From<PyResourceLimits> for ResourceLimits {
    fn from(value: PyResourceLimits) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourceLimits {
    #[new]
    #[pyo3(signature = (*, max_total_qubits=None))]
    fn new(max_total_qubits: Option<usize>) -> Self {
        Self {
            inner: ResourceLimits { max_total_qubits },
        }
    }

    #[getter]
    fn max_total_qubits(&self) -> Option<usize> {
        self.inner.max_total_qubits
    }

    fn __repr__(&self) -> String {
        match self.inner.max_total_qubits {
            Some(limit) => format!("ResourceLimits(max_total_qubits={limit})"),
            None => "ResourceLimits(max_total_qubits=None)".to_string(),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}
