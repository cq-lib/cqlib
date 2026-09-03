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

//! Python binding for multi-controlled standard gates.
//!
//! A wrapper retains parameters already bound to the base standard gate so
//! control promotion does not discard symbolic gate arguments.

use crate::circuit::PyStandardGate;
use crate::circuit::error::{CircuitError as PyCircuitError, ParameterError as PyParameterError};
use crate::circuit::parameter::PyParameter;
use crate::utils::hash_value;
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::error::ParameterError;
use cqlib_core::circuit::gate::MCGate;
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::prelude::*;
use pyo3::{PyResult, pyclass, pymethods};
use std::fmt;

/// Multi-controlled standard gate with optional bound parameters.
#[pyclass(name = "MCGate", module = "cqlib.circuit.gates", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyMcGate {
    pub(crate) inner: MCGate,
    pub(crate) params: Vec<Parameter>,
}

#[pymethods]
impl PyMcGate {
    /// Creates a new multi-controlled gate.
    ///
    /// # Arguments
    ///
    /// * `num_controls` - The number of control qubits to add.
    /// * `gate` - The base `StandardGate` to control.
    ///
    /// # Examples
    ///
    /// ```python
    /// # Create a Toffoli-like gate (CCX) with 2 controls
    /// ccx = MCGate(2, StandardGate.X)
    ///
    /// # Create a multi-controlled Hadamard
    /// much = MCGate(3, StandardGate.H)
    /// ```
    #[new]
    pub fn new(num_controls: u8, gate: PyStandardGate) -> Self {
        Self {
            inner: MCGate::new(num_controls, gate.inner),
            params: gate.params,
        }
    }

    /// Returns the unitary matrix representation as a NumPy array.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional parameters for the base gate (if parametric).
    ///
    /// # Returns
    ///
    /// A 2D numpy array (dtype=complex128).
    #[pyo3(signature = (params=None))]
    pub fn matrix<'py>(
        &self,
        py: Python<'py>,
        params: Option<Vec<f64>>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let params = if let Some(params) = params {
            if params.len() != self.inner.num_params() {
                return Err(PyCircuitError::new_err(format!(
                    "Gate {} expects {} parameters, got {}",
                    self.inner,
                    self.inner.num_params(),
                    params.len()
                )));
            }
            if let Some(value) = params.iter().find(|value| !value.is_finite()) {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {value}"
                    ))
                    .to_string(),
                ));
            }
            params
        } else if self.params.len() == self.inner.num_params() {
            self.params
                .iter()
                .map(|parameter| {
                    parameter
                        .evaluate(&None)
                        .map_err(|error| PyParameterError::new_err(error.to_string()))
                })
                .collect::<PyResult<_>>()?
        } else {
            return Err(PyCircuitError::new_err(format!(
                "Gate {} requires {} bound parameters",
                self.inner,
                self.inner.num_params()
            )));
        };
        let mat = self
            .inner
            .matrix(&params)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        Ok(mat.to_pyarray(py))
    }

    /// Computes the inverse (Hermitian conjugate) of the gate.
    ///
    /// The inverse of a controlled gate C(U) is C(U†).
    ///
    /// # Returns
    ///
    /// A new multi-controlled gate with inverse parameters already bound.
    pub fn inverse(&self) -> PyResult<Self> {
        if self.params.len() != self.inner.num_params() {
            return Err(PyCircuitError::new_err(format!(
                "Gate {} requires {} bound parameters before inversion",
                self.inner,
                self.inner.num_params()
            )));
        }
        match self.inner.inverse(&self.params) {
            Some((inv_gate, inv_params)) => Ok(Self {
                inner: inv_gate,
                params: inv_params.into_vec(),
            }),
            None => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Gate {} is not invertible",
                self.inner
            ))),
        }
    }

    /// Returns the number of control qubits.
    #[getter]
    pub fn num_ctrl_qubits(&self) -> usize {
        self.inner.num_ctrl_qubits()
    }

    /// Returns the total number of qubits (controls + targets).
    #[getter]
    pub fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    /// Returns the number of parameters required by the gate.
    #[getter]
    pub fn num_params(&self) -> usize {
        self.inner.num_params()
    }

    /// Returns the base gate (without controls).
    #[getter]
    pub fn base_gate(&self) -> PyStandardGate {
        PyStandardGate::from(*self.inner.base_gate(), self.params.clone())
    }

    /// Returns parameters bound to the base gate.
    #[getter]
    pub fn params(&self) -> Vec<PyParameter> {
        self.params.iter().cloned().map(PyParameter::from).collect()
    }

    fn __repr__(&self) -> String {
        let base_name = self.inner.base_gate().name();
        if self.params.is_empty() {
            format!(
                "MCGate({}, StandardGate.{})",
                self.inner.num_ctrl_qubits(),
                base_name
            )
        } else {
            let params = self
                .params
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "MCGate({}, StandardGate.{}({}))",
                self.inner.num_ctrl_qubits(),
                base_name,
                params
            )
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner && self.params == other.params
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __hash__(&self) -> u64 {
        hash_value(&(&self.inner, &self.params))
    }
}

impl fmt::Display for PyMcGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
