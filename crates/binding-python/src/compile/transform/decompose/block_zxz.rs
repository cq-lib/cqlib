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

//! Block-ZXZ synthesis of arbitrary unitary matrices (paper arXiv:2403.13692,
//! with the Section 5 A.1/A.2 CNOT optimizations).

use crate::circuit::circuit_impl::PyCircuit;
use cqlib_core::compile::transform::decompose::block_zxz::engine::{
    decompose_block_zxz_with_options as core_decompose_block_zxz, DecomposeOptions,
};
use cqlib_core::compile::transform::decompose::block_zxz::DecomposeResult;
use num_complex::Complex64;
use numpy::ndarray::Array2;
use numpy::{PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Optimization switches for the Block-ZXZ paper's Section 5 reductions.
#[pyclass(
    name = "BlockZxzOptions",
    module = "cqlib.compile.transform.decompose.block_zxz",
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyBlockZxzOptions {
    inner: DecomposeOptions,
}

#[pymethods]
impl PyBlockZxzOptions {
    #[new]
    #[pyo3(signature = (*, opt_a1=true, opt_a2=true, opt_a2_force=false))]
    fn new(opt_a1: bool, opt_a2: bool, opt_a2_force: bool) -> Self {
        PyBlockZxzOptions {
            inner: DecomposeOptions {
                opt_a1,
                opt_a2,
                opt_a2_force,
            },
        }
    }

    /// All optimizations off (matches the unoptimized paper baseline).
    #[staticmethod]
    fn none() -> Self {
        PyBlockZxzOptions {
            inner: DecomposeOptions::none(),
        }
    }

    #[getter]
    fn opt_a1(&self) -> bool {
        self.inner.opt_a1
    }

    #[getter]
    fn opt_a2(&self) -> bool {
        self.inner.opt_a2
    }

    #[getter]
    fn opt_a2_force(&self) -> bool {
        self.inner.opt_a2_force
    }

    fn __repr__(&self) -> String {
        format!(
            "BlockZxzOptions(opt_a1={}, opt_a2={}, opt_a2_force={})",
            if self.inner.opt_a1 { "True" } else { "False" },
            if self.inner.opt_a2 { "True" } else { "False" },
            if self.inner.opt_a2_force { "True" } else { "False" }
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.opt_a1 == other.inner.opt_a1
            && self.inner.opt_a2 == other.inner.opt_a2
            && self.inner.opt_a2_force == other.inner.opt_a2_force
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }
}

/// Result of a Block-ZXZ decomposition.
#[pyclass(
    name = "BlockZxzResult",
    module = "cqlib.compile.transform.decompose.block_zxz",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyBlockZxzResult {
    inner: DecomposeResult,
    cx_count: usize,
}

#[pymethods]
impl PyBlockZxzResult {
    /// The assembled circuit implementing the input unitary (up to global phase).
    #[getter]
    fn circuit(&self) -> PyCircuit {
        PyCircuit::from(self.inner.circuit.clone())
    }

    /// Number of qubits of the synthesized circuit.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.circuit.num_qubits()
    }

    /// Number of CX gates in the assembled circuit.
    #[getter]
    fn cx_count(&self) -> usize {
        self.cx_count
    }

    fn __repr__(&self) -> String {
        format!(
            "BlockZxzResult(num_qubits={}, cx_count={})",
            self.inner.circuit.num_qubits(),
            self.cx_count
        )
    }
}

/// Synthesizes an arbitrary `2^n x 2^n` unitary matrix into a standard-gate
/// circuit via Block-ZXZ decomposition with the paper's A.1/A.2 CNOT
/// optimizations (fully optimized by default).
///
/// Example (CNOT counts for generic random unitaries): n=3 -> 19,
/// n=4 -> 95 (paper Table 1); the unoptimized baseline is 24 / 120.
#[pyfunction(name = "decompose_block_zxz")]
#[pyo3(signature = (matrix, atol=1e-10, options=None))]
pub fn py_decompose_block_zxz(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
    options: Option<&PyBlockZxzOptions>,
) -> PyResult<PyBlockZxzResult> {
    if !atol.is_finite() || atol < 0.0 {
        return Err(PyValueError::new_err("atol must be a finite, non-negative float"));
    }
    let matrix = extract_complex_matrix(py, matrix)?;
    let options = options.map_or(DecomposeOptions::default(), |value| value.inner);
    let inner = py
        .detach(move || core_decompose_block_zxz(&matrix, atol, &options))
        .map_err(PyValueError::new_err)?;
    let cx_count = count_cx(&inner.circuit);
    Ok(PyBlockZxzResult { inner, cx_count })
}

fn extract_complex_matrix(py: Python<'_>, matrix: Bound<'_, PyAny>) -> PyResult<Array2<Complex64>> {
    let numpy = py.import("numpy")?;
    let array = numpy.call_method1("array", (matrix, "complex128"))?;
    let array = array.cast_into::<PyArray2<Complex64>>().map_err(|_| {
        PyValueError::new_err(
            "matrix must be convertible to a two-dimensional complex128 array",
        )
    })?;
    Ok(array.to_owned_array())
}

fn count_cx(circuit: &cqlib_core::circuit::Circuit) -> usize {
    use cqlib_core::circuit::Instruction;
    circuit
        .operations()
        .iter()
        .filter(|op| {
            matches!(
                &op.instruction,
                Instruction::Standard(gate) if gate.name() == "CX"
            )
        })
        .count()
}

/// Registers Block-ZXZ synthesis as `_native.compile.transform.decompose.block_zxz`.
pub(crate) fn register_block_zxz_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "block_zxz")?;

    m.add_class::<PyBlockZxzOptions>()?;
    m.add_class::<PyBlockZxzResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_decompose_block_zxz, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item(
            "cqlib._native.compile.transform.decompose.block_zxz",
            &m,
        )?;

    Ok(())
}
