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

//! Python bindings for pre-layout virtual-permutation elision.

use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use cqlib_core::circuit::Circuit;
use cqlib_core::compile::transform::{VirtualPermutationElisionStatus, elide_virtual_permutations};
use pyo3::prelude::*;
use std::collections::HashMap;

/// Resolved result of pre-layout logical SWAP elision.
#[pyclass(
    name = "VirtualPermutationElisionResult",
    module = "cqlib.compile.transform.virtual_permutation",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVirtualPermutationElisionResult {
    circuit: Circuit,
    changed: bool,
    status: &'static str,
    elided_swap_count: usize,
    virtual_permutation: HashMap<u32, u32>,
}

#[pymethods]
impl PyVirtualPermutationElisionResult {
    #[getter]
    fn circuit(&self) -> PyCircuit {
        self.circuit.clone().into()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.changed
    }

    #[getter]
    fn status(&self) -> &'static str {
        self.status
    }

    #[getter]
    fn elided_swap_count(&self) -> usize {
        self.elided_swap_count
    }

    #[getter]
    fn virtual_permutation(&self) -> HashMap<u32, u32> {
        self.virtual_permutation.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "VirtualPermutationElisionResult(changed={}, status={:?}, elided_swap_count={})",
            if self.changed { "True" } else { "False" },
            self.status,
            self.elided_swap_count,
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Removes eligible logical SWAPs without modifying `circuit`.
#[pyfunction(name = "elide_virtual_permutations")]
fn py_elide_virtual_permutations(
    py: Python<'_>,
    circuit: PyRef<'_, PyCircuit>,
) -> PyResult<PyVirtualPermutationElisionResult> {
    let original = circuit.inner.clone();
    py.detach(move || {
        let result = elide_virtual_permutations(&original)?;
        let status = match result.status() {
            VirtualPermutationElisionStatus::Unchanged => "unchanged",
            VirtualPermutationElisionStatus::Changed => "changed",
            VirtualPermutationElisionStatus::SkippedControlFlow => "skipped_control_flow",
        };
        let changed = result.status() == VirtualPermutationElisionStatus::Changed;
        let elided_swap_count = result.elided_swap_count();
        let virtual_permutation = result
            .virtual_permutation()
            .original_output_to_rewritten_output()
            .iter()
            .map(|(original, rewritten)| (original.id(), rewritten.id()))
            .collect();
        let circuit = result.into_changed_circuit().unwrap_or(original);
        Ok(PyVirtualPermutationElisionResult {
            circuit,
            changed,
            status,
            elided_swap_count,
            virtual_permutation,
        })
    })
    .map_err(compiler_error_to_py_err)
}

pub(crate) fn register_virtual_permutation_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "virtual_permutation")?;
    module.add_class::<PyVirtualPermutationElisionResult>()?;
    let elide = pyo3::wrap_pyfunction!(py_elide_virtual_permutations, &module)?;
    elide.setattr("__module__", "cqlib.compile.transform.virtual_permutation")?;
    module.add_function(elide)?;
    parent.add_submodule(&module)?;
    parent.py().import("sys")?.getattr("modules")?.set_item(
        "cqlib._native.compile.transform.virtual_permutation",
        &module,
    )?;
    Ok(())
}
