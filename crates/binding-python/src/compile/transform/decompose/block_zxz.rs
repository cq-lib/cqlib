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

use super::unitary::extract_complex_matrix;
use crate::circuit::PyCircuit;
use crate::compile::error::compiler_error_to_py_err;
use crate::utils::hash_value;
use cqlib_core::compile::transform::decompose::{
    BlockZxzDecomposeOptions, decompose_block_zxz_with_options,
};
use pyo3::prelude::*;

/// CX reductions for recursive Block-ZXZ synthesis.
#[pyclass(
    name = "BlockZxzDecomposeOptions",
    module = "cqlib.compile.transform.decompose.block_zxz",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub(super) struct Options(pub(super) BlockZxzDecomposeOptions);

#[pymethods]
impl Options {
    #[new]
    #[pyo3(signature = (*, opt_a1=true, opt_a2=true, opt_a2_force=false))]
    fn new(opt_a1: bool, opt_a2: bool, opt_a2_force: bool) -> Self {
        Self(BlockZxzDecomposeOptions {
            opt_a1,
            opt_a2,
            opt_a2_force,
        })
    }
    /// Absorb the outer UCRZ boundary CX gates (Section 5.2).
    #[getter]
    fn opt_a1(&self) -> bool {
        self.0.opt_a1
    }
    /// Transfer diagonal remainders between two-qubit leaves (Section 5.1).
    #[getter]
    fn opt_a2(&self) -> bool {
        self.0.opt_a2
    }
    /// Permit an enabled A.2 reduction on block-diagonal inputs.
    #[getter]
    fn opt_a2_force(&self) -> bool {
        self.0.opt_a2_force
    }
    /// Return options for unoptimized synthesis.
    #[staticmethod]
    fn none() -> Self {
        Self::new(false, false, false)
    }

    fn __repr__(&self) -> String {
        self.repr_value()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.0)
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

impl Options {
    pub(super) fn repr_value(&self) -> String {
        let py_bool = |value: bool| if value { "True" } else { "False" };
        format!(
            "BlockZxzDecomposeOptions(opt_a1={}, opt_a2={}, opt_a2_force={})",
            py_bool(self.0.opt_a1),
            py_bool(self.0.opt_a2),
            py_bool(self.0.opt_a2_force),
        )
    }
}

/// Return a standard-gate circuit for a finite unitary, including its scalar phase.
#[pyfunction(name = "decompose_block_zxz", signature = (matrix, atol=1e-10))]
fn decompose(py: Python<'_>, matrix: Bound<'_, PyAny>, atol: f64) -> PyResult<PyCircuit> {
    with_options(py, matrix, atol, None)
}

/// Set the A.1/A.2 reductions. A.2 is disabled on block diagonals unless forced.
#[pyfunction(name = "decompose_block_zxz_with_options", signature = (matrix, atol=1e-10, *, options=None))]
fn with_options(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
    options: Option<Options>,
) -> PyResult<PyCircuit> {
    let matrix = extract_complex_matrix(py, matrix)?;
    let options = options.map(|o| o.0).unwrap_or_default();
    py.detach(move || decompose_block_zxz_with_options(&matrix, atol, &options))
        .map(Into::into)
        .map_err(compiler_error_to_py_err)
}

pub(super) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "block_zxz")?;
    m.add_class::<Options>()?;
    m.add_function(wrap_pyfunction!(decompose, &m)?)?;
    m.add_function(wrap_pyfunction!(with_options, &m)?)?;
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.decompose.block_zxz", &m)
}
