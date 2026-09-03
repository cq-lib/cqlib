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

//! Python bindings for structural compiler analysis.

use crate::circuit::PyCircuit;
use cqlib_core::compile::transform::CircuitAnalysis;
use pyo3::prelude::*;

/// Structural facts about a circuit used by compiler transforms.
#[pyclass(
    name = "CircuitAnalysis",
    module = "cqlib.compile.transform.analysis",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCircuitAnalysis {
    inner: CircuitAnalysis,
}

impl From<CircuitAnalysis> for PyCircuitAnalysis {
    fn from(inner: CircuitAnalysis) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCircuitAnalysis {
    /// Computes structural facts without modifying the circuit.
    #[staticmethod]
    fn analyze(py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> Self {
        let circuit = circuit.inner.clone();
        py.detach(move || CircuitAnalysis::analyze(&circuit)).into()
    }

    #[getter]
    fn has_classical_data(&self) -> bool {
        self.inner.has_classical_data
    }

    #[getter]
    fn has_classical_control(&self) -> bool {
        self.inner.has_classical_control
    }

    #[getter]
    fn has_measurement(&self) -> bool {
        self.inner.has_measurement
    }

    #[getter]
    fn has_classical_values(&self) -> bool {
        self.inner.has_classical_values
    }

    #[getter]
    fn has_classical_vars(&self) -> bool {
        self.inner.has_classical_vars
    }

    #[getter]
    fn has_runtime_classical(&self) -> bool {
        self.inner.has_runtime_classical
    }

    #[getter]
    fn needs_classical_handle_preservation(&self) -> bool {
        self.inner.needs_classical_handle_preservation
    }

    #[getter]
    fn has_circuit_gate_definitions(&self) -> bool {
        self.inner.has_circuit_gate_definitions
    }

    #[getter]
    fn has_unitary_circuit_definitions(&self) -> bool {
        self.inner.has_unitary_circuit_definitions
    }

    #[getter]
    fn has_unitary_gates(&self) -> bool {
        self.inner.has_unitary_gates
    }

    #[getter]
    fn has_mc_gates(&self) -> bool {
        self.inner.has_mc_gates
    }

    fn __repr__(&self) -> String {
        format!(
            "CircuitAnalysis(has_runtime_classical={}, has_unitary_gates={}, has_mc_gates={})",
            self.inner.has_runtime_classical, self.inner.has_unitary_gates, self.inner.has_mc_gates
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

pub(crate) fn register_analysis_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "analysis")?;
    module.add_class::<PyCircuitAnalysis>()?;
    parent.add_submodule(&module)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.analysis", &module)?;
    Ok(())
}
