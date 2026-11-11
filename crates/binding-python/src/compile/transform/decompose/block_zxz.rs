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

//! Python bindings for the Block-ZXZ decomposition of general n-qubit
//! unitaries (Krol & Al-Ars, arXiv:2403.13692), including the paper's
//! Section 5 A.1/A.2 CNOT-count optimizations.

use crate::circuit::PyCircuit;
use cqlib_core::circuit::Circuit;
use cqlib_core::compile::transform::decompose::block_zxz::{
    DecomposeOptions as CoreDecomposeOptions, decompose_block_zxz as core_decompose_block_zxz,
    decompose_block_zxz_with_options as core_decompose_block_zxz_with_options,
};
use num_complex::Complex64;
use numpy::ndarray::Array2;
use numpy::{PyArray2, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// The four factor matrices returned by `block_zxz_step1`.
type Step1Arrays<'py> = (
    Bound<'py, PyArray2<Complex64>>,
    Bound<'py, PyArray2<Complex64>>,
    Bound<'py, PyArray2<Complex64>>,
    Bound<'py, PyArray2<Complex64>>,
);

/// The three factor matrices returned by `block_zxz_demux_pair`.
type DemuxArrays<'py> = (
    Bound<'py, PyArray2<Complex64>>,
    Bound<'py, PyArray2<Complex64>>,
    Bound<'py, PyArray2<Complex64>>,
);

/// Optimization switches of the paper's Section 5 reductions.
///
/// ``opt_a1`` (paper 5.2 / Eq. 11) folds each recursion level's two UCRz
/// side CNOTs into the middle multiplexer (saves 2 CNOTs per level).
/// ``opt_a2`` (paper 5.1, Shende-Markov-Bullock) decomposes every two-qubit
/// leaf but the temporally last one into a 2-CNOT circuit plus a rolling
/// diagonal remainder (saves ``4^(n-2) - 1`` CNOTs). ``opt_a2_force``
/// re-enables A.2 on block-diagonal (controlled) inputs, where the paper
/// disables it by default.
#[pyclass(
    name = "BlockZxzDecomposeOptions",
    module = "cqlib.compile.transform.decompose.block_zxz",
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyBlockZxzDecomposeOptions {
    pub(crate) inner: CoreDecomposeOptions,
}

#[pymethods]
impl PyBlockZxzDecomposeOptions {
    #[new]
    #[pyo3(signature = (*, opt_a1=true, opt_a2=true, opt_a2_force=false))]
    fn new(opt_a1: bool, opt_a2: bool, opt_a2_force: bool) -> Self {
        Self {
            inner: CoreDecomposeOptions {
                opt_a1,
                opt_a2,
                opt_a2_force,
            },
        }
    }

    /// A.1: fold each recursion level's two UCRz side CNOTs into the
    /// middle multiplexer.
    #[getter]
    fn opt_a1(&self) -> bool {
        self.inner.opt_a1
    }

    /// A.2: two-qubit leaf 2-CNOT synthesis + rolling diagonal remainders.
    #[getter]
    fn opt_a2(&self) -> bool {
        self.inner.opt_a2
    }

    /// Force A.2 on block-diagonal (controlled) inputs.
    #[getter]
    fn opt_a2_force(&self) -> bool {
        self.inner.opt_a2_force
    }

    /// Returns options with every optimization disabled (the paper's
    /// unoptimized pipeline).
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: CoreDecomposeOptions::none(),
        }
    }

    fn __repr__(&self) -> String {
        self.repr_string()
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

impl PyBlockZxzDecomposeOptions {
    /// Formats the options for Python reprs.
    pub(crate) fn repr_string(&self) -> String {
        let opt = |value: bool| if value { "True" } else { "False" };
        format!(
            "BlockZxzDecomposeOptions(opt_a1={}, opt_a2={}, opt_a2_force={})",
            opt(self.inner.opt_a1),
            opt(self.inner.opt_a2),
            opt(self.inner.opt_a2_force)
        )
    }
}

/// Converts the engine's `String` error into a Python `ValueError` (the
/// original Python project raises `ValueError` on invalid inputs).
fn string_error_to_py_err(message: String) -> pyo3::PyErr {
    PyValueError::new_err(message)
}

/// Converts a Python matrix argument into a complex128 ndarray.
fn extract_complex_matrix(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
) -> pyo3::PyResult<Array2<Complex64>> {
    let numpy = py.import("numpy")?;
    let array = numpy.call_method1("array", (matrix, "complex128"))?;
    let array = array.cast_into::<PyArray2<Complex64>>().map_err(|_| {
        PyValueError::new_err("matrix must be convertible to a two-dimensional complex128 array")
    })?;
    Ok(array.to_owned_array())
}

/// Wraps an owned core circuit into the public `Circuit` wrapper.
fn circuit_into_py(circuit: Circuit) -> PyCircuit {
    PyCircuit::from(circuit)
}

/// Decomposes a general n-qubit unitary into a `Circuit` of single-qubit
/// rotations and CX gates via the recursive Block-ZXZ method.
///
/// Both paper Section 5 optimizations (A.1 + A.2) are enabled by default;
/// the resulting CNOT count meets the paper bound
/// ``22/48 * 4^n - 3/2 * 2^n + 5/3`` (n=3 -> 19, n=4 -> 95).
///
/// Numerically degenerate inputs can exceed that bound by at most two CX:
/// when folding the accumulated A.2 diagonal remainder into a two-qubit leaf
/// is not numerically feasible, the remainder is emitted as an explicit
/// diagonal gate instead of failing the decomposition.
///
/// Raises:
///     ValueError: If ``matrix`` is not a square power-of-two unitary (a
///         NaN or infinite entry is rejected up front), if ``atol`` is
///         not a finite non-negative number, or if a decomposition
///         invariant fails.
#[pyfunction(name = "decompose_block_zxz")]
#[pyo3(signature = (matrix, atol=1e-10))]
pub fn py_decompose_block_zxz(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
) -> PyResult<PyCircuit> {
    let matrix = extract_complex_matrix(py, matrix)?;
    py.detach(move || core_decompose_block_zxz(&matrix, atol))
        .map(|result| circuit_into_py(result.circuit))
        .map_err(string_error_to_py_err)
}

/// Decomposes a general n-qubit unitary with explicit optimization
/// switches (see `BlockZxzDecomposeOptions`).
///
/// A.2 is auto-disabled on block-diagonal (controlled-gate) inputs unless
/// ``options.opt_a2_force`` is set (paper Sec. 6.2 constraint).
///
/// Raises:
///     ValueError: If ``matrix`` is invalid (including NaN or infinite
///         entries), if ``atol`` is not a finite non-negative number, or
///         if a decomposition invariant fails.
#[pyfunction(name = "decompose_block_zxz_with_options")]
#[pyo3(signature = (matrix, atol=1e-10, *, options=None))]
pub fn py_decompose_block_zxz_with_options(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
    options: Option<PyBlockZxzDecomposeOptions>,
) -> PyResult<PyCircuit> {
    let matrix = extract_complex_matrix(py, matrix)?;
    let options = options.map_or_else(CoreDecomposeOptions::default, |value| value.inner);
    py.detach(move || core_decompose_block_zxz_with_options(&matrix, atol, &options))
        .map(|result| circuit_into_py(result.circuit))
        .map_err(string_error_to_py_err)
}

/// Runs Block-ZXZ step 1 and returns the four sub-block factors
/// ``(A1, A2, B, C)`` as complex128 arrays.
///
/// Raises:
///     ValueError: If ``matrix`` is not a valid unitary of at least two
///         qubits.
#[pyfunction(name = "block_zxz_step1")]
#[pyo3(signature = (matrix, atol=1e-10))]
pub fn py_block_zxz_step1<'py>(
    py: Python<'py>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
) -> PyResult<Step1Arrays<'py>> {
    let matrix = extract_complex_matrix(py, matrix)?;
    let (a1, a2, b, c) = py
        .detach(move || {
            cqlib_core::compile::transform::decompose::block_zxz::block_zxz_step1(&matrix, atol)
        })
        .map_err(string_error_to_py_err)?;
    Ok((
        PyArray2::from_owned_array(py, a1),
        PyArray2::from_owned_array(py, a2),
        PyArray2::from_owned_array(py, b),
        PyArray2::from_owned_array(py, c),
    ))
}

/// Demultiplexes a block pair ``(U0, U1)`` into the factors
/// ``(V, D, W)`` with ``U0 = V @ D @ W`` and ``U1 = V @ D.conj().T @ W``.
///
/// Raises:
///     ValueError: If the inputs are invalid or the reconstruction check
///         fails.
#[pyfunction(name = "block_zxz_demux_pair")]
#[pyo3(signature = (u0, u1, atol=1e-10))]
pub fn py_block_zxz_demux_pair<'py>(
    py: Python<'py>,
    u0: Bound<'_, PyAny>,
    u1: Bound<'_, PyAny>,
    atol: f64,
) -> PyResult<DemuxArrays<'py>> {
    let u0 = extract_complex_matrix(py, u0)?;
    let u1 = extract_complex_matrix(py, u1)?;
    let (v, d, w) = py
        .detach(move || {
            cqlib_core::compile::transform::decompose::block_zxz::step2::demux_pair(&u0, &u1, atol)
        })
        .map_err(string_error_to_py_err)?;
    Ok((
        PyArray2::from_owned_array(py, v),
        PyArray2::from_owned_array(py, d),
        PyArray2::from_owned_array(py, w),
    ))
}

/// Expands a diagonal unitary ``D`` into its uniformly controlled Rz
/// circuit, enacting ``diag(D, D†)``.
///
/// Raises:
///     ValueError: If ``d_matrix`` is not a diagonal unitary.
#[pyfunction(name = "block_zxz_step3_ucrz_to_circuit")]
#[pyo3(signature = (d_matrix, atol=1e-10))]
pub fn py_block_zxz_step3_ucrz_to_circuit(
    py: Python<'_>,
    d_matrix: Bound<'_, PyAny>,
    atol: f64,
) -> PyResult<PyCircuit> {
    let d_matrix = extract_complex_matrix(py, d_matrix)?;
    let circuit = py
        .detach(move || {
            cqlib_core::compile::transform::decompose::block_zxz::ucrz::step3_ucrz_to_circuit(
                &d_matrix, atol,
            )
        })
        .map_err(string_error_to_py_err)?
        .circuit;
    Ok(circuit_into_py(circuit))
}

/// Returns whether ``matrix`` is block-diagonal (a controlled-gate shape)
/// within ``atol``.
#[pyfunction(name = "block_zxz_is_block_diagonal")]
#[pyo3(signature = (matrix, atol=1e-10))]
pub fn py_block_zxz_is_block_diagonal(
    py: Python<'_>,
    matrix: Bound<'_, PyAny>,
    atol: f64,
) -> PyResult<bool> {
    let matrix = extract_complex_matrix(py, matrix)?;
    Ok(
        cqlib_core::compile::transform::decompose::block_zxz::engine::is_block_diagonal(
            &matrix, atol,
        ),
    )
}

/// Counts the CX gates of ``circuit`` (including nested composite-gate
/// blocks).
#[pyfunction(name = "block_zxz_cx_count")]
pub fn py_block_zxz_cx_count(circuit: &PyCircuit) -> usize {
    cqlib_core::compile::transform::decompose::block_zxz::vw::count_cx(&circuit.inner)
}

/// Registers Block-ZXZ bindings as
/// `_native.compile.transform.decompose.block_zxz`.
pub(crate) fn register_block_zxz_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "block_zxz")?;

    m.add_class::<PyBlockZxzDecomposeOptions>()?;
    m.add_function(pyo3::wrap_pyfunction!(py_decompose_block_zxz, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_decompose_block_zxz_with_options,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_block_zxz_step1, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_block_zxz_demux_pair, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_block_zxz_step3_ucrz_to_circuit,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_block_zxz_is_block_diagonal, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_block_zxz_cx_count, &m)?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.compile.transform.decompose.block_zxz", &m)?;

    Ok(())
}
