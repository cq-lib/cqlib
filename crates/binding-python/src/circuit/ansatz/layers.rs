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

//! Python bindings for layer-style ansatze.

use cqlib_core::circuit::ansatz::layers::{BasicEntanglerLayers, StronglyEntanglingLayers};
use cqlib_core::circuit::ansatz::traits::Ansatz;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::circuit::circuit_impl::PyCircuit;
use crate::circuit::gate::PyStandardGate;

/// Basic entangler layers with one rotation per qubit followed by ring entanglement.
#[pyclass(
    name = "BasicEntanglerLayers",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBasicEntanglerLayers {
    pub(crate) inner: BasicEntanglerLayers,
}

impl From<BasicEntanglerLayers> for PyBasicEntanglerLayers {
    fn from(inner: BasicEntanglerLayers) -> Self {
        Self { inner }
    }
}

impl From<PyBasicEntanglerLayers> for BasicEntanglerLayers {
    fn from(value: PyBasicEntanglerLayers) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyBasicEntanglerLayers {
    /// Creates a new BasicEntanglerLayers template.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: BasicEntanglerLayers::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the single-parameter rotation gate.
    fn rotation_gate(&self, gate: PyRef<'_, PyStandardGate>) -> Self {
        Self {
            inner: self.inner.clone().rotation_gate(gate.inner),
        }
    }

    /// Sets the two-qubit entanglement gate.
    fn entanglement_gate(&self, gate: PyRef<'_, PyStandardGate>) -> Self {
        Self {
            inner: self.inner.clone().entanglement_gate(gate.inner),
        }
    }

    /// Validates the configuration.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the parameterized circuit.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of symbolic parameters.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "BasicEntanglerLayers(num_qubits={}, num_parameters={})",
            self.inner.num_qubits(),
            self.inner.num_parameters()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// Strongly entangling layers with U rotations and range-based ring entanglement.
///
/// Uses CX by default because the range pattern is directed and has explicit
/// control-target semantics. Users can still choose CX, CY, or CZ manually to
/// match backend-native gates or experiment-specific circuit conventions.
#[pyclass(
    name = "StronglyEntanglingLayers",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyStronglyEntanglingLayers {
    pub(crate) inner: StronglyEntanglingLayers,
}

impl From<StronglyEntanglingLayers> for PyStronglyEntanglingLayers {
    fn from(inner: StronglyEntanglingLayers) -> Self {
        Self { inner }
    }
}

impl From<PyStronglyEntanglingLayers> for StronglyEntanglingLayers {
    fn from(value: PyStronglyEntanglingLayers) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyStronglyEntanglingLayers {
    /// Creates a new StronglyEntanglingLayers template.
    ///
    /// Defaults to CX as the entanglement gate. The gate remains configurable
    /// through `entanglement_gate`.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: StronglyEntanglingLayers::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the two-qubit entanglement gate.
    ///
    /// Valid choices are CX, CY, and CZ.
    fn entanglement_gate(&self, gate: PyRef<'_, PyStandardGate>) -> Self {
        Self {
            inner: self.inner.clone().entanglement_gate(gate.inner),
        }
    }

    /// Sets explicit entanglement ranges. Values are reused cyclically by layer.
    fn ranges(&self, ranges: Vec<usize>) -> Self {
        Self {
            inner: self.inner.clone().ranges(ranges),
        }
    }

    /// Validates the configuration.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the parameterized circuit.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of symbolic parameters.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "StronglyEntanglingLayers(num_qubits={}, num_parameters={})",
            self.inner.num_qubits(),
            self.inner.num_parameters()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}
