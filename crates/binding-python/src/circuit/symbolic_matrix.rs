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

//! Python bindings for dense symbolic complex matrices.
//!
//! Symbolic matrices preserve circuit parameters until explicit evaluation.
//! Their storage grows as O(4^n), so they are intended for small circuits,
//! compiler rewrites, and custom gate definitions rather than simulation.

use crate::circuit::PyParameter;
use crate::circuit::error::{CircuitError as PyCircuitError, ParameterError as PyParameterError};
use cqlib_core::circuit::error::ParameterError;
use cqlib_core::circuit::symbolic_matrix::matrix::simplify_matrix;
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex, SymbolicMatrix, evaluate_symbolic_matrix, substitute_symbolic_matrix,
};
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use std::collections::{BTreeSet, HashMap};

/// Complex scalar whose real and imaginary parts are Parameter expressions.
#[pyclass(name = "SymbolicComplex", module = "cqlib.circuit", from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PySymbolicComplex {
    pub(crate) inner: SymbolicComplex,
}

impl From<SymbolicComplex> for PySymbolicComplex {
    fn from(inner: SymbolicComplex) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySymbolicComplex {
    /// Creates a symbolic complex value from real and imaginary parameters.
    #[new]
    fn new(real: PyParameter, imag: PyParameter) -> Self {
        Self {
            inner: SymbolicComplex::new(real.inner, imag.inner),
        }
    }

    #[staticmethod]
    fn zero() -> Self {
        SymbolicComplex::zero().into()
    }

    #[staticmethod]
    fn one() -> Self {
        SymbolicComplex::one().into()
    }

    #[staticmethod]
    fn i() -> Self {
        SymbolicComplex::i().into()
    }

    #[staticmethod]
    fn from_real(value: PyParameter) -> Self {
        SymbolicComplex::from_real(value.inner).into()
    }

    #[staticmethod]
    fn exp_i(theta: PyParameter) -> Self {
        SymbolicComplex::exp_i(theta.inner).into()
    }

    /// Returns the symbolic real part.
    #[getter]
    fn real(&self) -> PyParameter {
        self.inner.re.clone().into()
    }

    /// Returns the symbolic imaginary part.
    #[getter]
    fn imag(&self) -> PyParameter {
        self.inner.im.clone().into()
    }

    /// Returns all free symbols in deterministic order.
    #[getter]
    fn symbols(&self) -> Vec<String> {
        self.inner
            .re
            .get_symbols()
            .into_iter()
            .chain(self.inner.im.get_symbols())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Simplifies both symbolic components.
    fn simplify(&self) -> PyResult<Self> {
        self.inner
            .simplify()
            .map(Self::from)
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    fn replace(&self, symbol: String, value: PyParameter) -> Self {
        self.inner.replace(&symbol, value.inner).into()
    }

    fn is_zero_exact(&self) -> bool {
        self.inner.is_zero_exact()
    }

    fn is_one_exact(&self) -> bool {
        self.inner.is_one_exact()
    }

    fn simplifies_to_zero(&self) -> PyResult<bool> {
        self.inner
            .simplifies_to_zero()
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Evaluates the scalar after binding every required symbol.
    #[pyo3(signature = (bindings=None))]
    fn evaluate(&self, bindings: Option<HashMap<String, f64>>) -> PyResult<Complex64> {
        if let Some(value) = bindings
            .as_ref()
            .and_then(|values| values.values().find(|value| !value.is_finite()))
        {
            return Err(PyParameterError::new_err(
                ParameterError::DomainError(format!("symbol binding must be finite, got {value}"))
                    .to_string(),
            ));
        }
        let bindings = bindings.as_ref().map(|values| {
            values
                .iter()
                .map(|(symbol, value)| (symbol.as_str(), *value))
                .collect()
        });
        self.inner
            .evaluate(&bindings)
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "SymbolicComplex(real={}, imag={})",
            self.inner.re, self.inner.im
        )
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Dense row-major matrix of symbolic complex values.
#[pyclass(name = "SymbolicMatrix", module = "cqlib.circuit", from_py_object)]
#[derive(Clone, Debug)]
pub struct PySymbolicMatrix {
    pub(crate) inner: SymbolicMatrix,
}

impl From<SymbolicMatrix> for PySymbolicMatrix {
    fn from(inner: SymbolicMatrix) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySymbolicMatrix {
    /// Creates a rectangular symbolic matrix from rows.
    #[new]
    fn new(rows: Vec<Vec<PySymbolicComplex>>) -> PyResult<Self> {
        let num_rows = rows.len();
        let num_cols = rows.first().map_or(0, Vec::len);
        if num_rows == 0 || num_cols == 0 {
            return Err(PyCircuitError::new_err(
                "symbolic matrix must contain at least one row and one column",
            ));
        }
        if rows.iter().any(|row| row.len() != num_cols) {
            return Err(PyCircuitError::new_err(
                "symbolic matrix rows must have equal length",
            ));
        }
        let values = rows
            .into_iter()
            .flatten()
            .map(|value| value.inner)
            .collect();
        Ok(Self {
            inner: SymbolicMatrix::from_shape_vec((num_rows, num_cols), values)
                .expect("validated rectangular symbolic matrix shape"),
        })
    }

    /// Returns the row and column count.
    #[getter]
    fn shape(&self) -> (usize, usize) {
        self.inner.dim()
    }

    /// Returns all free symbols in deterministic order.
    #[getter]
    fn symbols(&self) -> Vec<String> {
        self.inner
            .iter()
            .flat_map(|value| {
                value
                    .re
                    .get_symbols()
                    .into_iter()
                    .chain(value.im.get_symbols())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Returns the element at the supplied row and column.
    fn __getitem__(&self, index: (isize, isize)) -> PyResult<PySymbolicComplex> {
        let (rows, cols) = self.inner.dim();
        let row = if index.0 < 0 {
            rows.checked_add_signed(index.0)
        } else {
            usize::try_from(index.0).ok()
        };
        let col = if index.1 < 0 {
            cols.checked_add_signed(index.1)
        } else {
            usize::try_from(index.1).ok()
        };
        match (row, col) {
            (Some(row), Some(col)) if row < rows && col < cols => {
                Ok(self.inner[(row, col)].clone().into())
            }
            _ => Err(PyIndexError::new_err(format!(
                "symbolic matrix index ({}, {}) is out of bounds for shape ({rows}, {cols})",
                index.0, index.1
            ))),
        }
    }

    /// Returns a nested row representation.
    fn rows(&self) -> Vec<Vec<PySymbolicComplex>> {
        self.inner
            .rows()
            .into_iter()
            .map(|row| row.iter().cloned().map(PySymbolicComplex::from).collect())
            .collect()
    }

    /// Simplifies every matrix element.
    fn simplify(&self) -> PyResult<Self> {
        simplify_matrix(&self.inner)
            .map(Self::from)
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Simultaneously substitutes symbols with Parameter expressions.
    fn substitute(&self, replacements: HashMap<String, PyParameter>) -> PyResult<Self> {
        let replacements = replacements
            .into_iter()
            .map(|(symbol, value)| (symbol, value.inner))
            .collect();
        substitute_symbolic_matrix(self.inner.clone(), &replacements)
            .map(Self::from)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Evaluates the symbolic matrix as a NumPy complex128 array.
    #[pyo3(signature = (bindings=None))]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        bindings: Option<HashMap<String, f64>>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        if let Some(value) = bindings
            .as_ref()
            .and_then(|values| values.values().find(|value| !value.is_finite()))
        {
            return Err(PyParameterError::new_err(
                ParameterError::DomainError(format!("symbol binding must be finite, got {value}"))
                    .to_string(),
            ));
        }
        let bindings = bindings.as_ref().map(|values| {
            values
                .iter()
                .map(|(symbol, value)| (symbol.as_str(), *value))
                .collect()
        });
        evaluate_symbolic_matrix(&self.inner, &bindings)
            .map(|matrix| matrix.to_pyarray(py))
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    fn __len__(&self) -> usize {
        self.inner.nrows()
    }

    fn __repr__(&self) -> String {
        let (rows, cols) = self.inner.dim();
        format!("SymbolicMatrix(shape=({rows}, {cols}))")
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqlib_core::circuit::Parameter;

    #[test]
    fn negative_indices_follow_python_semantics() {
        let matrix = PySymbolicMatrix::from(
            SymbolicMatrix::from_shape_vec(
                (1, 2),
                vec![
                    SymbolicComplex::zero(),
                    SymbolicComplex::from_real(Parameter::symbol("theta")),
                ],
            )
            .unwrap(),
        );

        assert_eq!(
            matrix.__getitem__((-1, -1)).unwrap().symbols(),
            vec!["theta"]
        );
        assert!(matrix.__getitem__((-2, 0)).is_err());
    }

    #[test]
    fn symbols_are_sorted_and_deduplicated() {
        let matrix = PySymbolicMatrix::from(
            SymbolicMatrix::from_shape_vec(
                (1, 2),
                vec![
                    SymbolicComplex::from_real(Parameter::symbol("z")),
                    SymbolicComplex::new(Parameter::symbol("a"), Parameter::symbol("z")),
                ],
            )
            .unwrap(),
        );

        assert_eq!(matrix.symbols(), vec!["a", "z"]);
    }

    #[test]
    fn scalar_constructors_and_replace_delegate_to_core() {
        let value = PySymbolicComplex::exp_i(PyParameter::from(Parameter::symbol("theta")));
        assert_eq!(value.symbols(), vec!["theta"]);

        let replaced = value.replace("theta".to_string(), PyParameter::from(Parameter::from(0.0)));
        assert!(replaced.inner.im.is_zero());
        assert!(replaced.inner.re.is_one());
    }
}
