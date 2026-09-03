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

use super::config::PyTwoQubitUnitaryDecomposeBasis;
use crate::circuit::PyValueOperation;
use crate::circuit::bit::PyIntOrQubit;
use crate::compile::error::compiler_error_to_py_err;
use cqlib_core::compile::transform::decompose::unitary::{
    KakDecomposition, OneQubitUnitaryDecomposition, TwoQubitUnitaryDecomposeBasis,
    TwoQubitUnitarySynthesisResult, kak_decompose, synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary,
};
use num_complex::Complex64;
use numpy::ndarray::Array2;
use numpy::{PyArray2, PyArrayMethods};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

/// Numeric decomposition of a one-qubit unitary matrix.
#[pyclass(
    name = "OneQubitUnitaryDecomposition",
    module = "cqlib.compile.transform.decompose.unitary",
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyOneQubitUnitaryDecomposition {
    inner: OneQubitUnitaryDecomposition,
}

#[pymethods]
impl PyOneQubitUnitaryDecomposition {
    #[getter]
    fn theta(&self) -> f64 {
        self.inner.theta
    }

    #[getter]
    fn phi(&self) -> f64 {
        self.inner.phi
    }

    #[getter(lambda_)]
    fn lambda_(&self) -> f64 {
        self.inner.lambda
    }

    #[getter]
    fn global_phase(&self) -> f64 {
        self.inner.global_phase
    }

    fn __repr__(&self) -> String {
        format!(
            "OneQubitUnitaryDecomposition(theta={}, phi={}, lambda_={}, global_phase={})",
            self.inner.theta, self.inner.phi, self.inner.lambda, self.inner.global_phase
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

/// Standard-gate synthesis of a two-qubit unitary matrix.
#[pyclass(
    name = "TwoQubitUnitarySynthesisResult",
    module = "cqlib.compile.transform.decompose.unitary",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTwoQubitUnitarySynthesisResult {
    inner: TwoQubitUnitarySynthesisResult,
}

#[pymethods]
impl PyTwoQubitUnitarySynthesisResult {
    #[getter]
    fn operations(&self) -> Vec<PyValueOperation> {
        self.inner
            .operations
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    #[getter]
    fn global_phase(&self) -> f64 {
        self.inner.global_phase
    }

    fn __repr__(&self) -> String {
        format!(
            "TwoQubitUnitarySynthesisResult(operations={}, global_phase={})",
            self.inner.operations.len(),
            self.inner.global_phase
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Canonical two-qubit KAK decomposition.
#[pyclass(
    name = "KakDecomposition",
    module = "cqlib.compile.transform.decompose.unitary",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyKakDecomposition {
    inner: KakDecomposition,
}

#[pymethods]
impl PyKakDecomposition {
    #[getter]
    fn global_phase(&self) -> f64 {
        self.inner.global_phase
    }

    #[getter]
    fn k1l<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<Complex64>> {
        PyArray2::from_owned_array(py, self.inner.k1l.clone())
    }

    #[getter]
    fn k1r<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<Complex64>> {
        PyArray2::from_owned_array(py, self.inner.k1r.clone())
    }

    #[getter]
    fn k2l<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<Complex64>> {
        PyArray2::from_owned_array(py, self.inner.k2l.clone())
    }

    #[getter]
    fn k2r<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<Complex64>> {
        PyArray2::from_owned_array(py, self.inner.k2r.clone())
    }

    #[getter]
    fn a(&self) -> f64 {
        self.inner.a
    }

    #[getter]
    fn b(&self) -> f64 {
        self.inner.b
    }

    #[getter]
    fn c(&self) -> f64 {
        self.inner.c
    }

    fn __repr__(&self) -> String {
        format!(
            "KakDecomposition(a={}, b={}, c={}, global_phase={})",
            self.inner.a, self.inner.b, self.inner.c, self.inner.global_phase
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Decomposes a numeric 2x2 unitary into U-gate angles and global phase.
#[pyfunction(name = "synthesize_numeric_1q_unitary")]
pub fn py_synthesize_numeric_1q_unitary(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
) -> PyResult<PyOneQubitUnitaryDecomposition> {
    let matrix = extract_complex_matrix(py, matrix)?;
    py.detach(move || synthesize_numeric_1q_unitary(&matrix))
        .map(|inner| PyOneQubitUnitaryDecomposition { inner })
        .map_err(compiler_error_to_py_err)
}

/// Synthesizes a numeric 4x4 unitary into standard-gate operations.
#[pyfunction(name = "synthesize_numeric_2q_unitary")]
#[pyo3(signature = (matrix, first, second, basis=None))]
pub fn py_synthesize_numeric_2q_unitary(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    first: PyIntOrQubit,
    second: PyIntOrQubit,
    basis: Option<PyTwoQubitUnitaryDecomposeBasis>,
) -> PyResult<PyTwoQubitUnitarySynthesisResult> {
    let matrix = extract_complex_matrix(py, matrix)?;
    let qubits = [first.into(), second.into()];
    let basis = basis.map_or(TwoQubitUnitaryDecomposeBasis::PauliRotations, |value| {
        value.inner
    });
    py.detach(move || synthesize_numeric_2q_unitary(&matrix, qubits, basis))
        .map(|inner| PyTwoQubitUnitarySynthesisResult { inner })
        .map_err(compiler_error_to_py_err)
}

/// Computes the canonical KAK decomposition of a numeric 4x4 unitary.
#[pyfunction(name = "kak_decompose")]
pub fn py_kak_decompose(py: Python<'_>, matrix: Bound<'_, PyAny>) -> PyResult<PyKakDecomposition> {
    let matrix = extract_complex_matrix(py, matrix)?;
    py.detach(move || kak_decompose(&matrix))
        .map(|inner| PyKakDecomposition { inner })
        .map_err(compiler_error_to_py_err)
}

fn extract_complex_matrix(py: Python<'_>, matrix: Bound<'_, PyAny>) -> PyResult<Array2<Complex64>> {
    let numpy = py.import("numpy")?;
    let array = numpy.call_method1("array", (matrix, "complex128"))?;
    let array = array.cast_into::<PyArray2<Complex64>>().map_err(|_| {
        PyTypeError::new_err("matrix must be convertible to a two-dimensional complex128 array")
    })?;
    Ok(array.to_owned_array())
}

/// Registers numeric unitary synthesis as `_native.compile.transform.decompose.unitary`.
pub(crate) fn register_unitary_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "unitary")?;

    m.add_class::<PyOneQubitUnitaryDecomposition>()?;
    m.add_class::<PyTwoQubitUnitarySynthesisResult>()?;
    m.add_class::<PyKakDecomposition>()?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_synthesize_numeric_1q_unitary,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_synthesize_numeric_2q_unitary,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_kak_decompose, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.decompose.unitary", &m)?;

    Ok(())
}
