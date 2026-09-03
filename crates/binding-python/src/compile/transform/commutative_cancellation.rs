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

//! Python bindings for global commutation-set cancellation.

use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::transform::PyTransformResult;
use cqlib_core::compile::transform::{CommutativeCancellation, Transformer};
use pyo3::prelude::*;

/// Cancels self-inverse gate pairs over unbounded commutation sets.
#[pyclass(
    name = "CommutativeCancellation",
    module = "cqlib.compile.transform.commutative_cancellation",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommutativeCancellation {
    inner: CommutativeCancellation,
}

#[pymethods]
impl PyCommutativeCancellation {
    #[new]
    fn new() -> Self {
        Self {
            inner: CommutativeCancellation::new(),
        }
    }

    /// Cancels self-inverse pairs without modifying the input circuit.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let pass = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || {
            let outcome = pass.transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        "CommutativeCancellation()".to_string()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

pub(crate) fn register_commutative_cancellation_module(
    parent: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "commutative_cancellation")?;
    module.add_class::<PyCommutativeCancellation>()?;
    parent.add_submodule(&module)?;
    parent.py().import("sys")?.getattr("modules")?.set_item(
        "cqlib._native.compile.transform.commutative_cancellation",
        &module,
    )?;
    Ok(())
}
