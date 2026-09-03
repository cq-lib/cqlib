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

use super::error::resource_error_to_py_err;
use super::{
    PyResourceLease, PyResourceLimits, PyResourcePlan, PyResourcePolicy, PyResourceRequest,
};
use crate::circuit::PyCircuit;
use cqlib_core::compile::resource::{ResourceLimits, ResourceManager, ResourcePolicy};
use pyo3::prelude::*;

/// Python wrapper around compiler-visible ancillary-resource bookkeeping.
#[pyclass(
    name = "ResourceManager",
    module = "cqlib.compile.resource",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyResourceManager {
    pub(crate) inner: ResourceManager,
}

impl From<ResourceManager> for PyResourceManager {
    fn from(inner: ResourceManager) -> Self {
        Self { inner }
    }
}

impl From<PyResourceManager> for ResourceManager {
    fn from(value: PyResourceManager) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyResourceManager {
    /// Creates a pre-layout manager synchronized with the supplied circuit.
    #[staticmethod]
    #[pyo3(signature = (circuit, *, policy=None, limits=None))]
    fn from_circuit(
        circuit: PyRef<'_, PyCircuit>,
        policy: Option<PyResourcePolicy>,
        limits: Option<PyResourceLimits>,
    ) -> PyResult<Self> {
        let inner = ResourceManager::from_circuit(
            &circuit.inner,
            policy.map_or_else(ResourcePolicy::default, |value| value.inner),
            limits.map_or_else(ResourceLimits::default, |value| value.inner),
        )
        .map_err(resource_error_to_py_err)?;
        Ok(Self { inner })
    }

    /// Returns a side-effect-free resource preview.
    fn preview(&self, request: PyRef<'_, PyResourceRequest>) -> PyResult<PyResourcePlan> {
        self.inner
            .preview(&request.inner)
            .map(PyResourcePlan::from)
            .map_err(resource_error_to_py_err)
    }

    /// Commits a preview and mutates the synchronized circuit when needed.
    fn commit(
        &mut self,
        mut circuit: PyRefMut<'_, PyCircuit>,
        plan: PyRef<'_, PyResourcePlan>,
    ) -> PyResult<PyResourceLease> {
        self.inner
            .commit(&mut circuit.inner, plan.inner.clone())
            .map(PyResourceLease::from)
            .map_err(resource_error_to_py_err)
    }

    /// Releases a lease after its consuming algorithm restored the contract.
    fn release(&mut self, lease: PyRef<'_, PyResourceLease>) -> PyResult<()> {
        self.inner
            .release(&lease.inner)
            .map_err(resource_error_to_py_err)
    }

    /// Enters the one-way post-layout resource phase.
    fn enter_post_layout(&mut self, circuit: PyRef<'_, PyCircuit>) -> PyResult<()> {
        self.inner
            .enter_post_layout(&circuit.inner)
            .map_err(resource_error_to_py_err)
    }

    /// Verifies structural agreement between the manager and circuit.
    fn verify_consistency(&self, circuit: PyRef<'_, PyCircuit>) -> PyResult<()> {
        self.inner
            .verify_consistency(&circuit.inner)
            .map_err(resource_error_to_py_err)
    }

    /// Verifies consistency and requires all leases to be released.
    fn verify_idle(&self, circuit: PyRef<'_, PyCircuit>) -> PyResult<()> {
        self.inner
            .verify_idle(&circuit.inner)
            .map_err(resource_error_to_py_err)
    }

    fn __repr__(&self) -> &'static str {
        "ResourceManager()"
    }
}
