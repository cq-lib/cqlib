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

//! Python bindings for logical and target-basis-aware one-qubit optimization.

use crate::circuit::{PyCircuit, PyInstruction};
use crate::compile::error::compiler_error_to_py_err;
use crate::compile::target_basis_item::PyTargetBasisItem;
use crate::compile::transform::PyTransformResult;
use crate::utils::python_string_literal;
use cqlib_core::circuit::Instruction;
use cqlib_core::compile::transform::{OptimizeOneQubitRuns, Transformer};
use pyo3::prelude::*;

/// Reusable exact one-qubit optimizer for logical or explicit-basis circuits.
#[pyclass(
    name = "OptimizeOneQubitRuns",
    module = "cqlib.compile.transform.one_qubit_optimization",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyOptimizeOneQubitRuns {
    inner: OptimizeOneQubitRuns,
    target_basis: Option<Vec<Instruction>>,
}

#[pymethods]
impl PyOptimizeOneQubitRuns {
    /// Creates a target-neutral optimizer using strict logical cost.
    #[staticmethod]
    fn logical() -> Self {
        Self {
            inner: OptimizeOneQubitRuns::logical(),
            target_basis: None,
        }
    }

    /// Creates an optimizer costed after exact lowering to `target_basis`.
    ///
    /// Basis entries are either case-insensitive standard-gate names (e.g.
    /// `'H'`, `'X2P'`) or `Instruction` objects for multi-controlled gates.
    #[staticmethod]
    fn basis(py: Python<'_>, target_basis: Vec<PyTargetBasisItem>) -> PyResult<Self> {
        let target_basis = target_basis
            .into_iter()
            .map(PyTargetBasisItem::into_instruction)
            .collect::<PyResult<Vec<_>>>()?;
        let optimizer_basis = target_basis.clone();
        py.detach(move || OptimizeOneQubitRuns::basis(optimizer_basis))
            .map(|inner| Self {
                inner,
                target_basis: Some(target_basis),
            })
            .map_err(compiler_error_to_py_err)
    }

    /// Returns `"logical"` or `"basis"` for the active acceptance policy.
    #[getter]
    fn policy(&self) -> &'static str {
        if self.target_basis.is_some() {
            "basis"
        } else {
            "logical"
        }
    }

    /// Returns the explicit target basis, or `None` for logical optimization.
    #[getter]
    fn target_basis(&self) -> Option<Vec<PyInstruction>> {
        self.target_basis
            .as_ref()
            .map(|basis| basis.iter().cloned().map(Into::into).collect())
    }

    /// Optimizes fixed numeric one-qubit runs without modifying the input.
    fn run(&self, py: Python<'_>, circuit: PyRef<'_, PyCircuit>) -> PyResult<PyTransformResult> {
        let optimizer = self.inner.clone();
        let circuit = circuit.inner.clone();
        py.detach(move || {
            let outcome = optimizer.transform(&circuit, None)?;
            Ok(PyTransformResult::from_outcome(circuit, outcome))
        })
        .map_err(compiler_error_to_py_err)
    }

    fn __repr__(&self) -> String {
        match &self.target_basis {
            None => "OptimizeOneQubitRuns.logical()".to_string(),
            Some(target_basis) => format!(
                "OptimizeOneQubitRuns.basis([{}])",
                target_basis
                    .iter()
                    .map(|instruction| python_string_literal(&instruction.name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        match (&self.target_basis, &other.target_basis) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| left.name() == right.name())
            }
            (None, Some(_)) | (Some(_), None) => false,
        }
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

pub(crate) fn register_one_qubit_optimization_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "one_qubit_optimization")?;
    module.add_class::<PyOptimizeOneQubitRuns>()?;
    parent.add_submodule(&module)?;
    parent.py().import("sys")?.getattr("modules")?.set_item(
        "cqlib._native.compile.transform.one_qubit_optimization",
        &module,
    )?;
    Ok(())
}
