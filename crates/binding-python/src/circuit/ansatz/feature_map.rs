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

//! Python bindings for feature map ansatze.

use cqlib_core::circuit::ansatz::feature_map::{
    AngleEncoding, BasisEncoding, IQPFeatureMap, PauliFeatureMap, ZFeatureMap, ZZFeatureMap,
};
use cqlib_core::circuit::ansatz::traits::Ansatz;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::circuit::circuit_impl::PyCircuit;
use crate::circuit::gate::PyStandardGate;
use crate::qis::pauli::PyPauliString;

use super::two_local::PyEntanglementTopology;

/// A data encoding circuit using a single parameterized rotation gate per qubit.
///
/// Each qubit receives one rotation gate (e.g. RX, RY, or RZ) parameterized by
/// the corresponding input feature. This is the simplest data encoding strategy.
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import AngleEncoding
///     >>> from cqlib import StandardGate
///     >>> ae = AngleEncoding(4, StandardGate.RX)
///     >>> circuit = ae.build_circuit("x")
///     >>> ae.num_parameters()
///     4
#[pyclass(
    name = "AngleEncoding",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAngleEncoding {
    pub(crate) inner: AngleEncoding,
}

impl From<AngleEncoding> for PyAngleEncoding {
    fn from(inner: AngleEncoding) -> Self {
        Self { inner }
    }
}

impl From<PyAngleEncoding> for AngleEncoding {
    fn from(value: PyAngleEncoding) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyAngleEncoding {
    /// Creates a new AngleEncoding feature map.
    ///
    /// Args:
    ///     num_qubits: Number of qubits (= number of input features). Must be ≥ 1.
    ///     rotation_gate: Single-qubit rotation gate to use (RX, RY, or RZ).
    ///
    /// Raises:
    ///     ValueError: If num_qubits is 0 or the gate is not a valid rotation gate.
    #[new]
    fn new(num_qubits: usize, rotation_gate: PyRef<'_, PyStandardGate>) -> Self {
        Self {
            inner: AngleEncoding::new(num_qubits, rotation_gate.inner),
        }
    }

    /// Validates the configuration.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the encoding circuit.
    ///
    /// Parameters are named `{prefix}_0`, `{prefix}_1`, ..., `{prefix}_{n-1}`.
    ///
    /// Args:
    ///     prefix: Prefix for feature parameter names (default "x").
    ///
    /// Returns:
    ///     A Circuit with `num_qubits` symbolic parameters.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of parameters (= num_qubits).
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "AngleEncoding(num_qubits={}, num_parameters={})",
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

/// Computational basis encoding from a bitstring.
#[pyclass(
    name = "BasisEncoding",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBasisEncoding {
    pub(crate) inner: BasisEncoding,
}

impl From<BasisEncoding> for PyBasisEncoding {
    fn from(inner: BasisEncoding) -> Self {
        Self { inner }
    }
}

impl From<PyBasisEncoding> for BasisEncoding {
    fn from(value: PyBasisEncoding) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyBasisEncoding {
    /// Creates a new BasisEncoding from a boolean bitstring.
    #[new]
    fn new(bits: Vec<bool>) -> Self {
        Self {
            inner: BasisEncoding::new(bits),
        }
    }

    /// Validates the configuration.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the basis preparation circuit.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns zero because basis encoding has no symbolic parameters.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!("BasisEncoding(num_qubits={})", self.inner.num_qubits())
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

/// A first-order Z feature map without entanglement.
#[pyclass(
    name = "ZFeatureMap",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyZFeatureMap {
    pub(crate) inner: ZFeatureMap,
}

impl From<ZFeatureMap> for PyZFeatureMap {
    fn from(inner: ZFeatureMap) -> Self {
        Self { inner }
    }
}

impl From<PyZFeatureMap> for ZFeatureMap {
    fn from(value: PyZFeatureMap) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyZFeatureMap {
    /// Creates a new ZFeatureMap.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: ZFeatureMap::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Validates the configuration.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the encoding circuit.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of parameters.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "ZFeatureMap(num_qubits={}, num_parameters={})",
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

/// An IQP-style diagonal feature map.
#[pyclass(
    name = "IQPFeatureMap",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyIQPFeatureMap {
    pub(crate) inner: IQPFeatureMap,
}

impl From<IQPFeatureMap> for PyIQPFeatureMap {
    fn from(inner: IQPFeatureMap) -> Self {
        Self { inner }
    }
}

impl From<PyIQPFeatureMap> for IQPFeatureMap {
    fn from(value: PyIQPFeatureMap) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyIQPFeatureMap {
    /// Creates a new IQPFeatureMap.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: IQPFeatureMap::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the entanglement topology.
    fn entanglement(&self, topology: PyRef<'_, PyEntanglementTopology>) -> Self {
        Self {
            inner: self.inner.clone().entanglement(topology.inner.clone()),
        }
    }

    /// Validates the configuration.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the encoding circuit.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of parameters.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "IQPFeatureMap(num_qubits={}, num_parameters={})",
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

/// A second-order Pauli-Z feature map for quantum kernel methods.
///
/// Encodes classical data using single-qubit Z-rotations and two-qubit
/// ZZ-entanglement. Widely used for quantum kernel estimation.
///
/// For each repetition layer:
///   1. Hadamard on all qubits.
///   2. RZ(2·x_i) on each qubit i.
///   3. exp(-i·2·(π-x_i)(π-x_j)·ZZ) for each entangled pair (i, j).
///
/// Builder methods return a new ZZFeatureMap (immutable builder pattern).
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import ZZFeatureMap, EntanglementTopology
///     >>> fm = ZZFeatureMap(3).reps(2).entanglement(EntanglementTopology.full())
///     >>> circuit = fm.build_circuit("x")
///     >>> fm.num_parameters()
///     3
#[pyclass(
    name = "ZZFeatureMap",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyZZFeatureMap {
    pub(crate) inner: ZZFeatureMap,
}

impl From<ZZFeatureMap> for PyZZFeatureMap {
    fn from(inner: ZZFeatureMap) -> Self {
        Self { inner }
    }
}

impl From<PyZZFeatureMap> for ZZFeatureMap {
    fn from(value: PyZZFeatureMap) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyZZFeatureMap {
    /// Creates a new ZZFeatureMap.
    ///
    /// Args:
    ///     num_qubits: Number of qubits (= number of input features). Must be ≥ 1.
    ///
    /// Defaults:
    ///     - 2 repetition layers
    ///     - Full entanglement topology
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: ZZFeatureMap::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    ///
    /// Args:
    ///     n: Number of encoding repetitions. More reps → richer feature space.
    ///
    /// Returns:
    ///     A new ZZFeatureMap with the updated setting.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the entanglement topology.
    ///
    /// Args:
    ///     topology: An EntanglementTopology instance.
    ///
    /// Returns:
    ///     A new ZZFeatureMap with the updated setting.
    fn entanglement(&self, topology: PyRef<'_, PyEntanglementTopology>) -> Self {
        Self {
            inner: self.inner.clone().entanglement(topology.inner.clone()),
        }
    }

    /// Validates the configuration.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the encoding circuit.
    ///
    /// Args:
    ///     prefix: Prefix for feature parameter names (default "x").
    ///
    /// Returns:
    ///     A Circuit with `num_parameters()` symbolic parameters.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of parameters (= num_qubits).
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "ZZFeatureMap(num_qubits={}, num_parameters={})",
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

/// A general-purpose data encoding circuit using Pauli evolution gates.
///
/// Supports arbitrary Pauli strings (e.g. "Z", "ZZ", "XY", "ZZZ") and flexible
/// entanglement topologies. This is the most expressive feature map in the library.
///
/// For each repetition:
///   1. Hadamard on all qubits.
///   2. For each Pauli template and each k-tuple of qubit indices:
///      - k=1: apply exp(-i·x_i·P) (angle = 2·x_i).
///      - k≥2: apply exp(-i·2·∏(π-x_j)·P) (angle = 4·∏(π-x_j)).
///
/// Builder methods return a new PauliFeatureMap (immutable builder pattern).
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import PauliFeatureMap, EntanglementTopology
///     >>> from cqlib import PauliString
///     >>> fm = (PauliFeatureMap(3)
///     ...     .reps(2)
///     ...     .paulis([PauliString.from_str("Z"), PauliString.from_str("ZZ")])
///     ...     .entanglement(EntanglementTopology.full()))
///     >>> circuit = fm.build_circuit("x")
#[pyclass(
    name = "PauliFeatureMap",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPauliFeatureMap {
    pub(crate) inner: PauliFeatureMap,
}

impl From<PauliFeatureMap> for PyPauliFeatureMap {
    fn from(inner: PauliFeatureMap) -> Self {
        Self { inner }
    }
}

impl From<PyPauliFeatureMap> for PauliFeatureMap {
    fn from(value: PyPauliFeatureMap) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyPauliFeatureMap {
    /// Creates a new PauliFeatureMap with default configuration.
    ///
    /// Args:
    ///     num_qubits: Number of qubits (= number of input features). Must be ≥ 1.
    ///
    /// Defaults:
    ///     - 2 repetition layers
    ///     - Paulis: [Z, ZZ]
    ///     - Full entanglement topology
    ///     - Parameter prefix: "x"
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: PauliFeatureMap::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    ///
    /// Args:
    ///     n: Number of encoding repetitions.
    ///
    /// Returns:
    ///     A new PauliFeatureMap with the updated setting.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the Pauli string templates for the feature map.
    ///
    /// Each PauliString defines one type of interaction. The number of
    /// non-identity operators in each string determines its locality (k).
    ///
    /// Args:
    ///     paulis: List of PauliString instances (e.g. [PauliString.from_str("Z"), PauliString.from_str("ZZ")]).
    ///             Labels are auto-generated from the string representation.
    ///
    /// Returns:
    ///     A new PauliFeatureMap with the updated setting.
    fn paulis(&self, paulis: Vec<PyRef<'_, PyPauliString>>) -> Self {
        let rust_paulis: Vec<_> = paulis
            .iter()
            .map(|p| {
                let label = p.inner.to_string();
                (p.inner.clone(), label)
            })
            .collect();
        Self {
            inner: self.inner.clone().paulis(rust_paulis),
        }
    }

    /// Sets the entanglement topology for multi-qubit interactions.
    ///
    /// Args:
    ///     topology: An EntanglementTopology instance.
    ///
    /// Returns:
    ///     A new PauliFeatureMap with the updated setting.
    fn entanglement(&self, topology: PyRef<'_, PyEntanglementTopology>) -> Self {
        Self {
            inner: self.inner.clone().entanglement(topology.inner.clone()),
        }
    }

    /// Sets the parameter name prefix.
    ///
    /// Parameter names are generated as `{prefix}_0`, `{prefix}_1`, etc.
    ///
    /// Args:
    ///     prefix: The parameter name prefix (default "x").
    ///
    /// Returns:
    ///     A new PauliFeatureMap with the updated setting.
    fn parameter_prefix(&self, prefix: &str) -> Self {
        Self {
            inner: self.inner.clone().parameter_prefix(prefix),
        }
    }

    /// Validates the configuration.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the encoding circuit.
    ///
    /// Args:
    ///     prefix: Prefix for feature parameter names (overrides parameter_prefix if set).
    ///
    /// Returns:
    ///     A Circuit with `num_parameters()` symbolic parameters.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the number of parameters (= num_qubits).
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "PauliFeatureMap(num_qubits={}, num_parameters={})",
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
