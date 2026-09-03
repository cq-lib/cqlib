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

use crate::circuit::PyQubit;
use crate::circuit::bit::PyIntListOrQubitList;
use crate::utils::hash_value;
use cqlib_core::circuit::Qubit;
use cqlib_core::compile::resource::{
    AncillaRequirement, ResourceLease, ResourcePlan, ResourceRequest,
};
use pyo3::prelude::*;
use std::collections::BTreeSet;

/// State-restoration contract for ancillary qubits.
#[pyclass(
    name = "AncillaRequirement",
    module = "cqlib.compile.resource",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyAncillaRequirement {
    pub(crate) inner: AncillaRequirement,
}

impl From<AncillaRequirement> for PyAncillaRequirement {
    fn from(inner: AncillaRequirement) -> Self {
        Self { inner }
    }
}

impl From<PyAncillaRequirement> for AncillaRequirement {
    fn from(value: PyAncillaRequirement) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyAncillaRequirement {
    #[staticmethod]
    fn clean_zero() -> Self {
        AncillaRequirement::CleanZero.into()
    }

    #[staticmethod]
    fn dirty() -> Self {
        AncillaRequirement::Dirty.into()
    }

    fn __repr__(&self) -> &'static str {
        match self.inner {
            AncillaRequirement::CleanZero => "AncillaRequirement.clean_zero()",
            AncillaRequirement::Dirty => "AncillaRequirement.dirty()",
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            AncillaRequirement::CleanZero => "clean-zero",
            AncillaRequirement::Dirty => "dirty",
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Python value object describing an ancillary-resource request.
#[pyclass(
    name = "ResourceRequest",
    module = "cqlib.compile.resource",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyResourceRequest {
    pub(crate) inner: ResourceRequest,
}

impl From<ResourceRequest> for PyResourceRequest {
    fn from(inner: ResourceRequest) -> Self {
        Self { inner }
    }
}

impl From<PyResourceRequest> for ResourceRequest {
    fn from(value: PyResourceRequest) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourceRequest {
    #[new]
    #[pyo3(signature = (requirement, count, *, excluded=None))]
    fn new(
        requirement: PyAncillaRequirement,
        count: usize,
        excluded: Option<PyIntListOrQubitList>,
    ) -> Self {
        let excluded = excluded
            .map(Vec::<Qubit>::from)
            .unwrap_or_default()
            .into_iter()
            .collect::<BTreeSet<_>>();
        Self {
            inner: ResourceRequest {
                requirement: requirement.inner,
                count,
                excluded,
            },
        }
    }

    #[getter]
    fn requirement(&self) -> PyAncillaRequirement {
        self.inner.requirement.into()
    }

    #[getter]
    fn count(&self) -> usize {
        self.inner.count
    }

    #[getter]
    fn excluded(&self) -> Vec<PyQubit> {
        self.inner
            .excluded
            .iter()
            .copied()
            .map(PyQubit::from)
            .collect()
    }

    fn __repr__(&self) -> String {
        let excluded = self
            .inner
            .excluded
            .iter()
            .map(|qubit| qubit.id().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "ResourceRequest(requirement={}, count={}, excluded=[{}])",
            PyAncillaRequirement::from(self.inner.requirement).__repr__(),
            self.inner.count,
            excluded
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Side-effect-free, manager-specific allocation preview.
#[pyclass(
    name = "ResourcePlan",
    module = "cqlib.compile.resource",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyResourcePlan {
    pub(crate) inner: ResourcePlan,
}

impl From<ResourcePlan> for PyResourcePlan {
    fn from(inner: ResourcePlan) -> Self {
        Self { inner }
    }
}

impl From<PyResourcePlan> for ResourcePlan {
    fn from(value: PyResourcePlan) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourcePlan {
    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner
            .qubits()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn requirement(&self) -> PyAncillaRequirement {
        self.inner.requirement().into()
    }

    #[getter]
    fn num_new_qubits(&self) -> usize {
        self.inner.num_new_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourcePlan(requirement={}, qubits={:?}, num_new_qubits={})",
            PyAncillaRequirement::from(self.inner.requirement()).__repr__(),
            self.inner.qubits(),
            self.inner.num_new_qubits()
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Credential for an active ancillary-resource lease.
#[pyclass(
    name = "ResourceLease",
    module = "cqlib.compile.resource",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyResourceLease {
    pub(crate) inner: ResourceLease,
}

impl From<ResourceLease> for PyResourceLease {
    fn from(inner: ResourceLease) -> Self {
        Self { inner }
    }
}

impl From<PyResourceLease> for ResourceLease {
    fn from(value: PyResourceLease) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourceLease {
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id()
    }

    #[getter]
    fn qubits(&self) -> Vec<PyQubit> {
        self.inner
            .qubits()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn requirement(&self) -> PyAncillaRequirement {
        self.inner.requirement().into()
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceLease(id={}, requirement={}, qubits={:?})",
            self.inner.id(),
            PyAncillaRequirement::from(self.inner.requirement()).__repr__(),
            self.inner.qubits()
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}
