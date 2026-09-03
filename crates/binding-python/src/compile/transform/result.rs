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

use crate::circuit::PyCircuit;
use cqlib_core::circuit::Circuit;
use cqlib_core::compile::transform::TransformOutcome;
use cqlib_core::compile::transform::decompose::DecompositionRuleStats;
use pyo3::prelude::*;

/// Common result returned by circuit-to-circuit transforms.
#[pyclass(
    name = "TransformResult",
    module = "cqlib.compile.transform",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTransformResult {
    circuit: Circuit,
    changed: bool,
}

impl PyTransformResult {
    /// Resolves the core's borrowed-input outcome into Python's owned result.
    ///
    /// The input circuit has already been cloned at the Python boundary. Move
    /// that clone into the result when the transform is unchanged instead of
    /// cloning it a second time.
    pub(crate) fn from_outcome(original: Circuit, outcome: TransformOutcome) -> Self {
        match outcome {
            TransformOutcome::Unchanged => Self {
                circuit: original,
                changed: false,
            },
            TransformOutcome::Changed(circuit) => Self {
                circuit,
                changed: true,
            },
        }
    }
}

#[pymethods]
impl PyTransformResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.circuit.clone().into()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.changed
    }

    fn __repr__(&self) -> String {
        format!(
            "TransformResult(changed={})",
            if self.changed { "True" } else { "False" }
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.circuit == other.circuit && self.changed == other.changed
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Pass-local runtime decomposition-rule cache counters.
#[pyclass(
    name = "DecompositionRuleStats",
    module = "cqlib.compile.transform.decompose",
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyDecompositionRuleStats {
    pub(crate) inner: DecompositionRuleStats,
}

impl From<DecompositionRuleStats> for PyDecompositionRuleStats {
    fn from(inner: DecompositionRuleStats) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDecompositionRuleStats {
    #[getter]
    fn hits(&self) -> usize {
        self.inner.hits
    }

    #[getter]
    fn misses(&self) -> usize {
        self.inner.misses
    }

    #[getter]
    fn inserts(&self) -> usize {
        self.inner.inserts
    }

    fn __repr__(&self) -> String {
        format!(
            "DecompositionRuleStats(hits={}, misses={}, inserts={})",
            self.inner.hits, self.inner.misses, self.inner.inserts
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
