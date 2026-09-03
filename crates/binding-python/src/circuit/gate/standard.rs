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

//! Python binding for the native standard-gate instruction set.
//!
//! Gate definitions are exposed as class attributes such as `StandardGate.H`.
//! Calling a parametric definition binds immutable [`Parameter`] expressions to
//! that gate instance.

use crate::circuit::error::{CircuitError as PyCircuitError, ParameterError as PyParameterError};
use crate::circuit::gate::PyMcGate;
use crate::circuit::parameter::PyParameter;
use crate::utils::hash_value;
use cqlib_core::circuit::CircuitError as CoreCircuitError;
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::error::ParameterError;
use cqlib_core::circuit::gate::{MCGate, StandardGate};
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::sync::RwLock;

/// Native standard gate with optional bound symbolic parameters.
#[pyclass(name = "StandardGate", module = "cqlib.circuit.gates", from_py_object)]
#[derive(Debug)]
pub struct PyStandardGate {
    pub inner: StandardGate,
    pub params: Vec<Parameter>,
    hash: RwLock<Option<u64>>,
}

impl Clone for PyStandardGate {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            params: self.params.clone(),
            hash: RwLock::new(None),
        }
    }
}

#[pymethods]
impl PyStandardGate {
    /// Disallows direct instantiation.
    ///
    /// Use static attributes like `StandardGate.H` or `StandardGate.RX(theta)`.
    #[new]
    fn new() -> PyResult<Self> {
        Err(PyTypeError::new_err(
            "StandardGate cannot be instantiated directly. Use static attributes like StandardGate.H or StandardGate.RX",
        ))
    }

    /// Returns all standard gate definitions in enum order.
    ///
    /// The returned gates are unbound definitions. Parametric gates can be
    /// called to bind concrete or symbolic parameters.
    #[staticmethod]
    fn all() -> Vec<Self> {
        StandardGate::all()
            .iter()
            .copied()
            .map(|gate| PyStandardGate::from(gate, Vec::new()))
            .collect()
    }

    /// Resolves a standard-gate name to its unbound definition.
    ///
    /// Matching is case-insensitive against the canonical gate names, so
    /// `StandardGate.from_name('cx')` returns `StandardGate.CX`.
    ///
    /// # Arguments
    ///
    /// * `name` - Canonical gate name (e.g. `'H'`, `'X2P'`, `'Phase'`).
    ///
    /// # Raises
    ///
    /// * `ValueError` - If `name` does not match any standard gate.
    #[staticmethod]
    fn from_name(name: &str) -> PyResult<Self> {
        standard_gate_from_name(name).map_or_else(
            || {
                Err(PyValueError::new_err(format!(
                    "unknown standard gate name: {name:?}"
                )))
            },
            |gate| Ok(Self::from(gate, Vec::new())),
        )
    }

    /// Binds parameters to the gate.
    ///
    /// Enables callable syntax: `StandardGate.RX(3.14)` returns a new gate instance
    /// with the specified parameter value.
    ///
    /// # Arguments
    ///
    /// * `args` - Positional arguments for gate parameters (float or `Parameter`).
    ///
    /// # Returns
    ///
    /// A new `PyStandardGate` with bound parameters.
    #[pyo3(signature = (*args))]
    fn __call__(&self, args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let expected_params = self.inner.num_params();

        // If gate requires no parameters (e.g., H, X), calling it returns a clone
        if expected_params == 0 {
            if !args.is_empty() {
                return Err(PyCircuitError::new_err(
                    CoreCircuitError::ParameterCountMismatch {
                        expected: 0,
                        actual: args.len(),
                    }
                    .to_string(),
                ));
            }
            return Ok(self.clone());
        }

        if args.len() != expected_params {
            return Err(PyCircuitError::new_err(
                CoreCircuitError::ParameterCountMismatch {
                    expected: expected_params,
                    actual: args.len(),
                }
                .to_string(),
            ));
        }

        let mut new_params = Vec::with_capacity(expected_params);
        for arg in args {
            if let Ok(py_param) = arg.extract::<PyParameter>() {
                new_params.push(py_param.inner);
            } else if let Ok(val) = arg.extract::<f64>() {
                if !val.is_finite() {
                    return Err(PyParameterError::new_err(
                        ParameterError::DomainError(format!(
                            "numeric parameter must be finite, got {val}"
                        ))
                        .to_string(),
                    ));
                }
                new_params.push(Parameter::from(val));
            } else {
                return Err(PyTypeError::new_err(format!(
                    "Parameter argument must be a float or Parameter, got {:?}",
                    arg
                )));
            }
        }

        Ok(PyStandardGate {
            inner: self.inner,
            params: new_params,
            hash: RwLock::new(None),
        })
    }

    fn __repr__(&self) -> String {
        if self.params.is_empty() {
            format!("StandardGate.{}", self.inner.name())
        } else {
            let params_str: Vec<String> = self.params.iter().map(|p| p.to_string()).collect();
            format!(
                "StandardGate.{}({})",
                self.inner.name(),
                params_str.join(", ")
            )
        }
    }

    fn __eq__(&self, other: &PyStandardGate) -> bool {
        self.inner == other.inner && self.params == other.params
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __hash__(&self) -> u64 {
        match self.hash.read() {
            Ok(guard) if guard.is_some() => return guard.unwrap(),
            _ => {}
        }
        let mut guard = self.hash.write().expect("Hash cache lock poisoned");
        if let Some(hash) = *guard {
            return hash;
        }
        let hash = hash_value(&(&self.inner, &self.params));

        *guard = Some(hash);
        hash
    }

    /// Customizes attribute access to only show relevant attributes.
    ///
    /// This prevents pollution from other gates in the same class.
    /// For example, `StandardGate.H` will only show its own methods/properties,
    /// not `StandardGate.X`, `StandardGate.RX`, etc.
    fn __dir__(&self) -> Vec<&'static str> {
        let mut attrs = vec![
            // Magic methods
            "__class__",
            "__repr__",
            "__eq__",
            "__hash__",
            // Properties
            "num_qubits",
            "num_ctrl_qubits",
            "num_params",
            "params",
            // Methods
            "matrix",
            "control",
            "inverse",
        ];
        // Only add __call__ for parametric gates
        if self.inner.num_params() > 0 {
            attrs.push("__call__");
        }
        attrs
    }

    /// Returns the total number of qubits this gate acts on.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    /// Returns the number of control qubits.
    #[getter]
    fn num_ctrl_qubits(&self) -> usize {
        self.inner.num_ctrl_qubits()
    }

    /// Returns the number of parameters this gate accepts.
    #[getter]
    fn num_params(&self) -> usize {
        self.inner.num_params()
    }

    /// Returns the bound parameters for this gate.
    #[getter]
    fn params(&self) -> Vec<PyParameter> {
        self.params
            .iter()
            .map(|p| PyParameter { inner: p.clone() })
            .collect()
    }

    /// Returns the unitary matrix representation as a NumPy array.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional concrete parameter values. Required for parametric gates
    ///   that have symbolic parameters.
    ///
    /// # Returns
    ///
    /// A 2D NumPy array (dtype=complex128) representing the gate matrix.
    #[pyo3(signature = (params=None))]
    fn matrix<'py>(
        &self,
        py: Python<'py>,
        params: Option<Vec<f64>>,
    ) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let eval_params: Vec<f64>;

        // Priority: use provided params, then try to evaluate internal params
        if let Some(p) = params {
            if p.len() != self.inner.num_params() {
                return Err(PyCircuitError::new_err(
                    CoreCircuitError::ParameterCountMismatch {
                        expected: self.inner.num_params(),
                        actual: p.len(),
                    }
                    .to_string(),
                ));
            }
            eval_params = p;
        } else if !self.params.is_empty() {
            // Try to evaluate internal params (must be constant)
            let mut calculated = Vec::with_capacity(self.params.len());
            for parameter in &self.params {
                calculated.push(
                    parameter
                        .evaluate(&None)
                        .map_err(|error| PyParameterError::new_err(error.to_string()))?,
                );
            }
            eval_params = calculated;
        } else if self.inner.num_params() == 0 {
            eval_params = vec![];
        } else {
            return Err(PyCircuitError::new_err(format!(
                "Gate {:?} expects {} parameters, but none were provided.",
                self.inner,
                self.inner.num_params()
            )));
        }

        // StandardGate::matrix returns Result<Cow<Array2>, CircuitError>
        // Use rust-numpy to_pyarray for efficient conversion
        let mat_cow = self
            .inner
            .matrix(&eval_params)
            .map_err(|error| PyCircuitError::new_err(error.to_string()))?;
        Ok(mat_cow.to_pyarray(py))
    }

    /// Returns a multi-controlled form of this standard gate.
    fn control(&self, num_controls: u8) -> PyMcGate {
        PyMcGate {
            inner: MCGate::new(num_controls, self.inner),
            params: self.params.clone(),
        }
    }

    /// Returns the inverse (Hermitian conjugate) of this gate.
    ///
    /// # Returns
    ///
    /// A new gate representing the inverse operation.
    fn inverse(&self) -> PyResult<Self> {
        if self.params.len() != self.inner.num_params() {
            return Err(PyCircuitError::new_err(format!(
                "Gate {} requires {} bound parameters before inversion",
                self.inner,
                self.inner.num_params()
            )));
        }

        match self.inner.inverse(&self.params) {
            Some((inv_gate, inv_params)) => Ok(PyStandardGate {
                inner: inv_gate,
                params: inv_params.into_vec(),
                hash: RwLock::new(None),
            }),
            None => Err(PyRuntimeError::new_err(format!(
                "Gate {:?} is not invertible",
                self.inner
            ))),
        }
    }
}

/// Registers static gate attributes on the `StandardGate` class.
///
/// Adds all standard gates (H, X, RX, CX, etc.) as class attributes
/// to the Python `StandardGate` module.
pub fn register_gates(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let cls = module.getattr("StandardGate")?;
    let py = module.py();

    // Helper to create gate instances
    // Since we don't expose new publicly, we use Py::new from Rust side

    let add_gate = |name: &str, gate: StandardGate| -> PyResult<()> {
        let instance = Py::new(
            py,
            PyStandardGate {
                inner: gate,
                params: Vec::new(),
                hash: RwLock::new(None),
            },
        )?;
        cls.setattr(name, instance)?;
        Ok(())
    };

    // Single Qubit
    add_gate("I", StandardGate::I)?;
    add_gate("H", StandardGate::H)?;
    add_gate("X", StandardGate::X)?;
    add_gate("Y", StandardGate::Y)?;
    add_gate("Z", StandardGate::Z)?;
    add_gate("S", StandardGate::S)?;
    add_gate("SDG", StandardGate::SDG)?;
    add_gate("T", StandardGate::T)?;
    add_gate("TDG", StandardGate::TDG)?;

    // Parametric
    add_gate("RX", StandardGate::RX)?;
    add_gate("RY", StandardGate::RY)?;
    add_gate("RZ", StandardGate::RZ)?;
    add_gate("U", StandardGate::U)?;
    add_gate("Phase", StandardGate::Phase)?;
    add_gate("GPhase", StandardGate::GPhase)?;

    // Two Qubit
    add_gate("CX", StandardGate::CX)?;
    add_gate("CY", StandardGate::CY)?;
    add_gate("CZ", StandardGate::CZ)?;
    add_gate("CCX", StandardGate::CCX)?; // 3-qubit
    add_gate("SWAP", StandardGate::SWAP)?;

    add_gate("RXX", StandardGate::RXX)?;
    add_gate("RXY", StandardGate::RXY)?;
    add_gate("RYY", StandardGate::RYY)?;
    add_gate("RZX", StandardGate::RZX)?;
    add_gate("RZZ", StandardGate::RZZ)?;

    add_gate("CRX", StandardGate::CRX)?;
    add_gate("CRY", StandardGate::CRY)?;
    add_gate("CRZ", StandardGate::CRZ)?;

    add_gate("XY", StandardGate::XY)?;
    add_gate("X2P", StandardGate::X2P)?;
    add_gate("X2M", StandardGate::X2M)?;
    add_gate("XY2P", StandardGate::XY2P)?;
    add_gate("XY2M", StandardGate::XY2M)?;
    add_gate("Y2P", StandardGate::Y2P)?;
    add_gate("Y2M", StandardGate::Y2M)?;

    add_gate("FSIM", StandardGate::FSIM)?;

    Ok(())
}

impl PyStandardGate {
    /// Constructs a `PyStandardGate` from a core `StandardGate` and parameters.
    pub fn from(gate: StandardGate, params: Vec<Parameter>) -> Self {
        Self {
            inner: gate,
            params,
            hash: RwLock::new(None),
        }
    }
}

/// Resolves a case-insensitive standard-gate name to its gate definition.
///
/// Canonical names come from [`StandardGate::name`] (e.g. `"H"`, `"Phase"`),
/// so `"h"` and `"PHASE"` resolve identically. This is the single shared
/// lookup used by `StandardGate.from_name` and every Python API that accepts
/// basis entries as strings.
pub(crate) fn standard_gate_from_name(name: &str) -> Option<StandardGate> {
    StandardGate::all()
        .iter()
        .copied()
        .find(|gate| gate.name().eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::PyStandardGate;
    use cqlib_core::circuit::{Parameter, StandardGate};

    #[test]
    fn control_preserves_bound_parameters() {
        let theta = Parameter::symbol("theta");
        let gate = PyStandardGate::from(StandardGate::RX, vec![theta.clone()]);
        let controlled = gate.control(2);

        assert_eq!(controlled.inner.num_ctrl_qubits(), 2);
        assert_eq!(controlled.params, vec![theta]);
    }

    #[test]
    fn inverse_uses_exact_bound_parameter_count() {
        let theta = Parameter::symbol("theta");
        let gate = PyStandardGate::from(StandardGate::RX, vec![theta.clone()]);
        let inverse = gate.inverse().unwrap();

        assert_eq!(inverse.inner, StandardGate::RX);
        assert_eq!(inverse.params, vec![-theta]);
    }

    #[test]
    fn all_mirrors_core_standard_gate_order() {
        let gates = PyStandardGate::all();

        assert_eq!(gates.len(), StandardGate::all().len());
        assert!(
            gates
                .iter()
                .zip(StandardGate::all())
                .all(
                    |(py_gate, core_gate)| py_gate.inner == *core_gate && py_gate.params.is_empty()
                )
        );
    }
}
