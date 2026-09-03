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

//! Python binding for user-defined unitary gates.
//!
//! Numeric, symbolic, and frozen-circuit definitions delegate to the
//! corresponding core representations.

use crate::circuit::error::CircuitError as PyCircuitError;
use crate::circuit::gate::PyFrozenCircuit;
use crate::circuit::symbolic_matrix::PySymbolicMatrix;
use crate::utils::hash_value;
use cqlib_core::circuit::CircuitError;
use cqlib_core::circuit::gate::UnitaryGate;
use num_complex::Complex64;
use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::{PyResult, Python, pyclass, pymethods};
use std::sync::Arc;

/// User-defined unitary gate with stable definition identity.
#[pyclass(name = "UnitaryGate", module = "cqlib.circuit.gates", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyUnitaryGate {
    inner: UnitaryGate,
}

#[pymethods]
impl PyUnitaryGate {
    /// Creates a new unitary gate definition without a matrix.
    ///
    /// # Arguments
    ///
    /// * `label` - A descriptive name for the gate (e.g., "QFT", "Oracle").
    /// * `num_qubits` - The number of qubits the gate operates on.
    ///
    /// # Returns
    ///
    /// A new `UnitaryGate` with no matrix attached.
    #[new]
    #[pyo3(signature = (label, num_qubits, num_params=0))]
    pub fn new(label: String, num_qubits: u16, num_params: u16) -> PyResult<Self> {
        Ok(Self {
            inner: UnitaryGate::new(label.as_ref(), num_qubits, num_params),
        })
    }

    /// Attaches a unitary matrix to the gate.
    ///
    /// The matrix must be a 2D array of shape (2^n, 2^n) where n is num_qubits.
    /// Accepts numpy arrays, lists, or any array-like input.
    ///
    /// # Arguments
    ///
    /// * `matrix` - A 2D square matrix (numpy array or list of lists).
    ///
    /// # Returns
    ///
    /// A new gate with the matrix attached.
    #[pyo3(signature = (matrix))]
    pub fn with_matrix<'py>(&self, py: Python<'py>, matrix: Bound<'py, PyAny>) -> PyResult<Self> {
        let np = py.import("numpy")?;
        let array_obj = np.call_method1("array", (matrix, "complex128"))?;

        let array: Bound<'py, PyArray2<Complex64>> = array_obj.cast_into().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(
                "Input could not be converted to a 2D complex numpy array.",
            )
        })?;

        let array = array.to_owned();
        let new_inner = self
            .inner
            .clone()
            .with_matrix(array.to_owned_array())
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        Ok(Self { inner: new_inner })
    }

    /// Attaches a symbolic matrix and its positional parameter names.
    fn with_symbolic_matrix(
        &self,
        matrix: PySymbolicMatrix,
        params: Vec<String>,
    ) -> PyResult<Self> {
        self.inner
            .clone()
            .with_symbolic_matrix(params, matrix.inner)
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Attaches an immutable circuit definition.
    fn with_circuit(&self, circuit: PyFrozenCircuit) -> PyResult<Self> {
        self.inner
            .clone()
            .with_circuit(Arc::new(circuit.inner))
            .map(|inner| Self { inner })
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Returns the label of the gate.
    #[getter]
    pub fn label(&self) -> String {
        self.inner.label().to_string()
    }

    /// Returns the number of qubits this gate acts on.
    #[getter]
    pub fn num_qubits(&self) -> u16 {
        self.inner.num_qubits()
    }

    #[getter]
    /// Returns the number of parameters required by each gate application.
    pub fn num_params(&self) -> u16 {
        self.inner.num_params()
    }

    /// Returns the symbolic matrix definition when present.
    #[getter]
    fn symbolic_matrix(&self) -> Option<PySymbolicMatrix> {
        self.inner.symbolic_matrix().cloned().map(Into::into)
    }

    /// Returns positional symbolic-matrix parameter names when present.
    #[getter]
    fn matrix_params(&self) -> Option<Vec<String>> {
        self.inner.matrix_params().map(ToOwned::to_owned)
    }

    /// Returns the frozen circuit definition when present.
    #[getter]
    fn circuit(&self) -> Option<PyFrozenCircuit> {
        self.inner
            .circuit()
            .as_ref()
            .map(|circuit| PyFrozenCircuit {
                inner: circuit.as_ref().clone(),
            })
    }

    /// Returns the unitary matrix as a NumPy array.
    ///
    /// # Returns
    ///
    /// A 2D numpy array (dtype=complex128).
    ///
    /// # Raises
    ///
    /// ValueError if no matrix was attached to the gate.
    pub fn matrix<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        match self.inner.matrix() {
            Some(mat) => Ok(PyArray2::from_array(py, mat)),
            None => Err(PyCircuitError::new_err(
                CircuitError::NoMatrixRepresentation.to_string(),
            )),
        }
    }

    /// Evaluates a numeric or symbolic matrix definition for concrete parameters.
    fn matrix_for_params<'py>(
        &self,
        py: Python<'py>,
        params: Vec<f64>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        self.inner
            .matrix_for_params(&params)
            .map(|matrix| PyArray2::from_array(py, matrix.as_ref()))
            .map_err(|error| PyCircuitError::new_err(error.to_string()))
    }

    /// Implements the numpy array protocol for numpy 2.0+ compatibility.
    ///
    /// Allows direct conversion to numpy array: `np.array(gate)` or `gate.matrix`.
    /// Supports dtype and copy keyword arguments as required by NumPy 2.0.
    #[pyo3(signature = (dtype=None, copy=None))]
    pub fn __array__<'py>(
        &self,
        py: Python<'py>,
        dtype: Option<Bound<'py, PyAny>>,
        copy: Option<bool>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let mat = match self.inner.matrix() {
            Some(m) => m,
            None => {
                return Err(PyCircuitError::new_err(
                    CircuitError::NoMatrixRepresentation.to_string(),
                ));
            }
        };

        let array = PyArray2::from_array(py, mat);

        // Handle dtype conversion if specified
        if let Some(dtype) = dtype {
            let astype_result = array.call_method("astype", (dtype,), None)?;
            return Ok(astype_result.extract()?);
        }

        if copy == Some(true) {
            let copy_result = array.call_method("copy", (), None)?;
            return Ok(copy_result.extract()?);
        }

        Ok(array)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "UnitaryGate({:?}, {}, {})",
            self.inner.label(),
            self.inner.num_qubits(),
            self.inner.num_params()
        )
    }
}

impl From<UnitaryGate> for PyUnitaryGate {
    fn from(inner: UnitaryGate) -> Self {
        Self { inner }
    }
}

impl From<PyUnitaryGate> for UnitaryGate {
    fn from(py: PyUnitaryGate) -> Self {
        py.inner
    }
}

#[cfg(test)]
mod tests {
    use super::PyUnitaryGate;
    use crate::circuit::symbolic_matrix::PySymbolicMatrix;
    use cqlib_core::circuit::Parameter;
    use cqlib_core::circuit::symbolic_matrix::{SymbolicComplex, SymbolicMatrix};

    #[test]
    fn constructor_preserves_gate_metadata() {
        let gate = PyUnitaryGate::new("oracle".to_string(), 3, 2).unwrap();

        assert_eq!(gate.label(), "oracle");
        assert_eq!(gate.num_qubits(), 3);
        assert_eq!(gate.num_params(), 2);
        assert_eq!(gate.__repr__(), "UnitaryGate(\"oracle\", 3, 2)");
    }

    #[test]
    fn symbolic_definition_preserves_parameter_order() {
        let matrix = SymbolicMatrix::from_shape_vec(
            (2, 2),
            vec![
                SymbolicComplex::one(),
                SymbolicComplex::zero(),
                SymbolicComplex::zero(),
                SymbolicComplex::from_real(Parameter::symbol("theta")),
            ],
        )
        .unwrap();
        let gate = PyUnitaryGate::new("symbolic".to_string(), 1, 1)
            .unwrap()
            .with_symbolic_matrix(PySymbolicMatrix::from(matrix), vec!["theta".to_string()])
            .unwrap();

        assert_eq!(gate.matrix_params(), Some(vec!["theta".to_string()]));
        assert!(gate.symbolic_matrix().is_some());
    }
}
