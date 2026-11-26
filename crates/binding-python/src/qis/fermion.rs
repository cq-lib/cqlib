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

//! Structured fermionic operators and sparse mappings; algebra stays in Core.

use super::hamiltonian::PyHamiltonian;
use super::qis_error_to_py_err;
use cqlib_core::qis::{
    FermionToQubitMapper, FermionicLadder, FermionicOp, JordanWignerMapper, OccupationStateMapper,
};
use num_complex::Complex64;
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyList, PyTuple};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Respect Python's integer protocol, including type-level method lookup and
// validation of __index__ return values, rather than invoking the method directly.
fn integer_index<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // SAFETY: value is a live Python object under the GIL. PyNumber_Index returns
    // a new owned reference, or NULL with an exception; Bound handles both cases.
    unsafe { Bound::from_owned_ptr_or_err(value.py(), pyo3::ffi::PyNumber_Index(value.as_ptr())) }
}

pub(super) fn index(value: &Bound<'_, PyAny>, is_count: bool) -> PyResult<usize> {
    let integer = integer_index(value)?;
    if integer.rich_compare(0, CompareOp::Lt)?.is_truthy()? {
        return Err(if is_count {
            PyValueError::new_err("register size must be nonnegative")
        } else {
            PyIndexError::new_err("mode indices must be nonnegative")
        });
    }
    integer.extract()
}

fn indices(values: &Bound<'_, PyAny>) -> PyResult<Vec<usize>> {
    values.try_iter()?.map(|v| index(&v?, false)).collect()
}

fn ladders(term: &Bound<'_, PyAny>) -> PyResult<Vec<FermionicLadder>> {
    term.try_iter()?
        .map(|v| Ok(v?.extract::<PyRef<'_, PyFermionicLadder>>()?.inner))
        .collect()
}

/// An immutable creation or annihilation operator on a zero-based mode.
#[pyclass(
    name = "FermionicLadder",
    module = "cqlib.qis.fermion",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFermionicLadder {
    inner: FermionicLadder,
}

#[pymethods]
impl PyFermionicLadder {
    #[staticmethod]
    /// Create an immutable creation ladder on a zero-based mode.
    fn create(mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: FermionicLadder::Create(index(mode, false)?),
        })
    }

    #[staticmethod]
    /// Create an immutable annihilation ladder on a zero-based mode.
    fn annihilate(mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: FermionicLadder::Annihilate(index(mode, false)?),
        })
    }

    #[getter]
    /// Zero-based mode index; the containing operator validates its range.
    fn mode(&self) -> usize {
        self.inner.mode()
    }

    #[getter]
    /// Ladder kind, either "create" or "annihilate".
    fn kind(&self) -> &'static str {
        match self.inner {
            FermionicLadder::Create(_) => "create",
            FermionicLadder::Annihilate(_) => "annihilate",
        }
    }

    /// Return a ladder on the same mode with creation and annihilation exchanged.
    fn adjoint(&self) -> Self {
        Self {
            inner: self.inner.adjoint(),
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let py = other.py();
        match other.extract::<PyRef<'_, Self>>() {
            Ok(rhs) => Ok((self.inner == rhs.inner)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind()),
            Err(_) => Ok(py.NotImplemented()),
        }
    }

    fn __hash__(&self) -> isize {
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        let hash = hasher.finish() as isize;
        if hash == -1 { -2 } else { hash }
    }

    fn __repr__(&self) -> String {
        format!("FermionicLadder.{}({})", self.kind(), self.mode())
    }
}

/// A finite complex sum of ordered fermionic products on a declared mode space.
///
/// In A * B, B acts first. Products are not automatically normal ordered.
/// Only exact zero coefficients are removed; use chop for explicit truncation.
#[pyclass(
    name = "FermionicOp",
    module = "cqlib.qis.fermion",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFermionicOp {
    inner: FermionicOp,
}

impl From<FermionicOp> for PyFermionicOp {
    fn from(inner: FermionicOp) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFermionicOp {
    /// Construct the zero operator; num_modes is retained even for zero modes.
    #[new]
    fn new(num_modes: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(FermionicOp::new(index(num_modes, true)?).into())
    }

    #[staticmethod]
    /// Return the identity on the declared mode space, including zero modes.
    fn identity(num_modes: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(FermionicOp::identity(index(num_modes, true)?).into())
    }

    /// An empty product is identity, whereas an empty sum is zero.
    #[staticmethod]
    #[pyo3(signature = (num_modes, term, coefficient=Complex64::new(1.0, 0.0)), text_signature = "(num_modes, term, coefficient=1.0)")]
    fn from_term(
        num_modes: &Bound<'_, PyAny>,
        term: &Bound<'_, PyAny>,
        coefficient: Complex64,
    ) -> PyResult<Self> {
        FermionicOp::from_term(index(num_modes, true)?, ladders(term)?, coefficient)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    /// Accept an iterable of (ordered ladders, coefficient) pairs and merge duplicates.
    #[staticmethod]
    fn from_terms(num_modes: &Bound<'_, PyAny>, terms: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut result = FermionicOp::new(index(num_modes, true)?);
        for item in terms.try_iter()? {
            let item = item?;
            let pair = item.cast::<PyTuple>()?;
            if pair.len() != 2 {
                return Err(PyValueError::new_err(
                    "each term must be a pair (ladders, coefficient)",
                ));
            }
            result
                .add_term(ladders(&pair.get_item(0)?)?, pair.get_item(1)?.extract()?)
                .map_err(qis_error_to_py_err)?;
        }
        Ok(result.into())
    }

    #[staticmethod]
    /// Return a creation operator; invalid mode indices raise IndexError.
    fn creation(num_modes: &Bound<'_, PyAny>, mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        FermionicOp::creation(index(num_modes, true)?, index(mode, false)?)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    #[staticmethod]
    /// Return an annihilation operator; invalid mode indices raise IndexError.
    fn annihilation(num_modes: &Bound<'_, PyAny>, mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        FermionicOp::annihilation(index(num_modes, true)?, index(mode, false)?)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    #[staticmethod]
    /// Return the mode number operator a†_p a_p.
    fn number(num_modes: &Bound<'_, PyAny>, mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        FermionicOp::number(index(num_modes, true)?, index(mode, false)?)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    #[staticmethod]
    /// Return total particle number; zero modes yield the zero operator.
    fn total_number(num_modes: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(FermionicOp::total_number(index(num_modes, true)?).into())
    }

    /// G = i(T - T†), with target creators followed by reversed source annihilators.
    /// Input order is preserved; no spin-orbital ordering or factorial is imposed.
    #[staticmethod]
    fn excitation_generator(
        num_modes: &Bound<'_, PyAny>,
        sources: &Bound<'_, PyAny>,
        targets: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        FermionicOp::excitation_generator(
            index(num_modes, true)?,
            &indices(sources)?,
            &indices(targets)?,
        )
        .map(Into::into)
        .map_err(qis_error_to_py_err)
    }

    #[getter]
    /// Declared number of modes, retained even for zero and identity operators.
    fn num_modes(&self) -> usize {
        self.inner.num_modes()
    }

    #[getter]
    /// Number of stored ordered products, before algebraic normal ordering.
    fn num_terms(&self) -> usize {
        self.inner.num_terms()
    }

    /// Return an independent snapshot in deterministic structural order.
    #[getter]
    fn terms<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let result = PyList::empty(py);
        for (term, coefficient) in self.inner.terms() {
            let term = PyTuple::new(
                py,
                term.iter()
                    .map(|&inner| Py::new(py, PyFermionicLadder { inner }))
                    .collect::<PyResult<Vec<_>>>()?,
            )?;
            result.append((term, coefficient))?;
        }
        Ok(result)
    }

    /// Test whether storage is empty; use equiv for algebraic zero.
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    /// Add a term in place and return None; errors leave this operator unchanged.
    #[pyo3(signature = (term, coefficient=Complex64::new(1.0, 0.0)), text_signature = "($self, term, coefficient=1.0)")]
    fn add_term(&mut self, term: &Bound<'_, PyAny>, coefficient: Complex64) -> PyResult<()> {
        self.inner
            .add_term(ladders(term)?, coefficient)
            .map_err(qis_error_to_py_err)
    }

    /// Return a conjugate transpose, reversing products and exchanging ladder kinds.
    fn adjoint(&self) -> Self {
        self.inner.adjoint().into()
    }

    /// Return a normal-ordered expansion, including contractions and fermionic signs.
    /// Creators precede annihilators; each group uses ascending mode indices.
    fn normal_order(&self) -> PyResult<Self> {
        self.inner
            .normal_order()
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    /// Return a scaled copy; the factor and resulting coefficients must be finite.
    fn scaled(&self, factor: Complex64) -> PyResult<Self> {
        self.inner
            .scaled(factor)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    /// Explicitly remove coefficients whose complex magnitude is at most atol, in place.
    #[pyo3(signature = (*, atol=0.0))]
    fn chop(&mut self, atol: f64) -> PyResult<()> {
        self.inner.chop(atol).map_err(qis_error_to_py_err)
    }

    /// Compare algebraically after normal ordering the difference.
    #[pyo3(signature = (other, *, atol=0.0))]
    fn equiv(&self, other: &Self, atol: f64) -> PyResult<bool> {
        self.inner
            .equiv(&other.inner, atol)
            .map_err(qis_error_to_py_err)
    }

    #[pyo3(signature = (*, atol=0.0))]
    /// Compare algebraically with the adjoint; atol must be finite and nonnegative.
    fn is_hermitian(&self, atol: f64) -> PyResult<bool> {
        self.inner.is_hermitian(atol).map_err(qis_error_to_py_err)
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        match other.extract::<PyRef<'_, Self>>() {
            Ok(rhs) => Ok(Py::new(
                other.py(),
                Self::from(
                    self.inner
                        .try_add(&rhs.inner)
                        .map_err(qis_error_to_py_err)?,
                ),
            )?
            .into_any()),
            Err(_) => Ok(other.py().NotImplemented()),
        }
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        match other.extract::<PyRef<'_, Self>>() {
            Ok(rhs) => Ok(Py::new(
                other.py(),
                Self::from(
                    self.inner
                        .try_sub(&rhs.inner)
                        .map_err(qis_error_to_py_err)?,
                ),
            )?
            .into_any()),
            Err(_) => Ok(other.py().NotImplemented()),
        }
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let result = if let Ok(rhs) = other.extract::<PyRef<'_, Self>>() {
            self.inner.try_mul(&rhs.inner)
        } else if let Some(factor) = scalar_operand(other)? {
            self.inner.scaled(factor)
        } else {
            return Ok(other.py().NotImplemented());
        };
        Ok(Py::new(other.py(), Self::from(result.map_err(qis_error_to_py_err)?))?.into_any())
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        match scalar_operand(other)? {
            Some(factor) => Ok(Py::new(other.py(), self.scaled(factor)?)?.into_any()),
            None => Ok(other.py().NotImplemented()),
        }
    }

    fn __neg__(&self) -> PyResult<Self> {
        self.scaled(Complex64::new(-1.0, 0.0))
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        match other.extract::<PyRef<'_, Self>>() {
            Ok(rhs) => Ok((self.inner == rhs.inner)
                .into_pyobject(other.py())?
                .to_owned()
                .into_any()
                .unbind()),
            Err(_) => Ok(other.py().NotImplemented()),
        }
    }

    /// Return an independent copy of this mutable operator.
    fn copy(&self) -> Self {
        self.clone()
    }
    fn __copy__(&self) -> Self {
        self.copy()
    }
    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.copy()
    }

    fn __repr__(&self) -> String {
        format!(
            "FermionicOp(num_modes={}, num_terms={})",
            self.num_modes(),
            self.num_terms()
        )
    }

    fn __str__(&self) -> String {
        if self.is_zero() {
            return "0".into();
        }
        self.inner
            .terms()
            .map(|(term, coefficient)| {
                let product = if term.is_empty() {
                    "I".into()
                } else {
                    term.iter()
                        .map(|ladder| match ladder {
                            FermionicLadder::Create(mode) => format!("a†_{mode}"),
                            FermionicLadder::Annihilate(mode) => format!("a_{mode}"),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                format!("({coefficient}) {product}")
            })
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

// Decline foreign types for Python's reflected arithmetic, but propagate numeric errors.
pub(super) fn scalar_operand(value: &Bound<'_, PyAny>) -> PyResult<Option<Complex64>> {
    if value.is_instance_of::<PyComplex>()
        || value.hasattr("__complex__")?
        || value.hasattr("__float__")?
        || value.hasattr("__index__")?
    {
        value.extract().map(Some)
    } else {
        Ok(None)
    }
}

/// Jordan-Wigner encoding: mode p maps to qubit p, with Z factors on modes below p.
#[pyclass(name = "JordanWignerMapper", module = "cqlib.qis.fermion", frozen)]
pub struct PyJordanWignerMapper;

#[pymethods]
impl PyJordanWignerMapper {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Map to a sparse Hamiltonian; non-Hermitian inputs are valid here.
    fn map_operator(&self, op: &PyFermionicOp) -> PyResult<PyHamiltonian> {
        JordanWignerMapper
            .map_operator(&op.inner)
            .map(Into::into)
            .map_err(qis_error_to_py_err)
    }

    /// Return the register width; Jordan-Wigner uses one qubit per mode.
    fn num_qubits(&self, num_modes: &Bound<'_, PyAny>) -> PyResult<usize> {
        Ok(JordanWignerMapper.num_qubits(index(num_modes, true)?))
    }

    /// bits[p] addresses qubit p (qubit 0 is the least significant bit).
    /// Accept only integer or boolean zero/one occupations. Empty input is vacuum.
    fn map_occupation(&self, occupation: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let mut occupations = Vec::new();
        for value in occupation.try_iter()? {
            let value = value?;
            let integer = integer_index(&value)?;
            let bit = if integer.rich_compare(0, CompareOp::Eq)?.is_truthy()? {
                0
            } else if integer.rich_compare(1, CompareOp::Eq)?.is_truthy()? {
                1
            } else {
                return Err(PyValueError::new_err("occupations must be zero or one"));
            };
            occupations.push(bit);
        }
        JordanWignerMapper
            .map_occupation(&occupations)
            .map_err(qis_error_to_py_err)
    }

    fn __repr__(&self) -> &'static str {
        "JordanWignerMapper()"
    }
}

pub fn register_fermion_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(parent.py(), "fermion")?;
    module.add_class::<PyFermionicLadder>()?;
    module.add_class::<PyFermionicOp>()?;
    module.add_class::<PyJordanWignerMapper>()?;
    parent.add_submodule(&module)
}
