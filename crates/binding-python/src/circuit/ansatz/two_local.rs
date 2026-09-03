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

//! Python bindings for `EntanglementTopology` and `TwoLocal` ansatz.

use cqlib_core::circuit::ansatz::traits::Ansatz;
use cqlib_core::circuit::ansatz::two_local::{EntanglementTopology, TwoLocal};
use cqlib_core::circuit::gate::StandardGate;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::circuit::circuit_impl::PyCircuit;
use crate::circuit::gate::PyStandardGate;

/// Defines the connectivity topology of the entanglement layer in an ansatz.
///
/// Use factory methods to create topology instances:
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import EntanglementTopology
///     >>> t = EntanglementTopology.linear()
///     >>> t = EntanglementTopology.full()
///     >>> t = EntanglementTopology.custom([(0, 1), (1, 2)])
#[pyclass(
    name = "EntanglementTopology",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEntanglementTopology {
    pub(crate) inner: EntanglementTopology,
}

impl From<EntanglementTopology> for PyEntanglementTopology {
    fn from(inner: EntanglementTopology) -> Self {
        Self { inner }
    }
}

impl From<PyEntanglementTopology> for EntanglementTopology {
    fn from(value: PyEntanglementTopology) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyEntanglementTopology {
    /// Creates a linear nearest-neighbor topology: (0,1), (1,2), ..., (n-2, n-1).
    #[staticmethod]
    fn linear() -> Self {
        EntanglementTopology::Linear.into()
    }

    /// Creates a circular topology (linear + wrap-around edge (n-1, 0)).
    #[staticmethod]
    fn circular() -> Self {
        EntanglementTopology::Circular.into()
    }

    /// Creates a full all-to-all topology connecting every pair of qubits.
    #[staticmethod]
    fn full() -> Self {
        EntanglementTopology::Full.into()
    }

    /// Creates a custom topology with the specified qubit pairs.
    ///
    /// Args:
    ///     pairs: List of (control, target) qubit index pairs.
    ///
    /// Examples:
    ///     >>> t = EntanglementTopology.custom([(0, 1), (2, 3)])
    #[staticmethod]
    fn custom(pairs: Vec<(usize, usize)>) -> Self {
        EntanglementTopology::Custom(pairs).into()
    }

    /// Returns the list of qubit pairs for this topology given `num_qubits`.
    ///
    /// Args:
    ///     num_qubits: Total number of qubits.
    ///
    /// Returns:
    ///     List of (control, target) index pairs.
    fn generate_pairs(&self, num_qubits: usize) -> Vec<(usize, usize)> {
        self.inner.generate_pairs(num_qubits)
    }

    /// Returns all k-tuples of qubit indices for this topology.
    ///
    /// Args:
    ///     k: Locality of the interaction.
    ///     num_qubits: Total number of qubits.
    ///
    /// Returns:
    ///     List of k-tuples represented as lists of qubit indices.
    fn generate_k_tuples(&self, k: usize, num_qubits: usize) -> Vec<Vec<usize>> {
        self.inner.generate_k_tuples(k, num_qubits)
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            EntanglementTopology::Linear => "EntanglementTopology.linear()".to_string(),
            EntanglementTopology::Circular => "EntanglementTopology.circular()".to_string(),
            EntanglementTopology::Full => "EntanglementTopology.full()".to_string(),
            EntanglementTopology::Custom(pairs) => {
                format!("EntanglementTopology.custom({pairs:?})")
            }
        }
    }

    fn __str__(&self) -> String {
        match &self.inner {
            EntanglementTopology::Linear => "linear".to_string(),
            EntanglementTopology::Circular => "circular".to_string(),
            EntanglementTopology::Full => "full".to_string(),
            EntanglementTopology::Custom(_) => "custom".to_string(),
        }
    }

    fn __eq__(&self, other: &PyEntanglementTopology) -> bool {
        self.inner == other.inner
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }
}

/// A hardware-efficient ansatz with alternating rotation and entanglement layers.
///
/// TwoLocal consists of:
///   1. Rotation layers: single-qubit parameterized gates (e.g. RY, RZ).
///   2. Entanglement layers: two-qubit gates (e.g. CX) determined by the topology.
///
/// The pattern is: [Rotation] → [Entanglement] → [Rotation] → ... → [Final Rotation]
///
/// Builder methods return a new `TwoLocal` instance (immutable builder pattern).
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import TwoLocal, EntanglementTopology
///     >>> from cqlib import StandardGate
///     >>> ansatz = (TwoLocal(3)
///     ...     .reps(2)
///     ...     .rotation_gates([StandardGate.RY, StandardGate.RZ])
///     ...     .entanglement(EntanglementTopology.linear()))
///     >>> circuit = ansatz.build_circuit("theta")
///     >>> ansatz.num_parameters()
///     18
#[pyclass(
    name = "TwoLocal",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTwoLocal {
    pub(crate) inner: TwoLocal,
}

impl From<TwoLocal> for PyTwoLocal {
    fn from(inner: TwoLocal) -> Self {
        Self { inner }
    }
}

impl From<PyTwoLocal> for TwoLocal {
    fn from(value: PyTwoLocal) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyTwoLocal {
    /// Creates a new TwoLocal ansatz.
    ///
    /// Args:
    ///     num_qubits: Number of qubits (must be ≥ 1).
    ///
    /// Defaults:
    ///     - 3 repetition layers
    ///     - RY rotation gate
    ///     - CX entanglement gate
    ///     - Linear entanglement topology
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: TwoLocal::new(num_qubits),
        }
    }

    /// Sets the number of repetition layers.
    ///
    /// Args:
    ///     n: Number of [Rotation + Entanglement] repetitions. The final rotation
    ///        layer is always appended unless `skip_final_rotation_layer` is set.
    ///
    /// Returns:
    ///     A new TwoLocal with the updated setting.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Sets the rotation gates applied in each rotation layer.
    ///
    /// Each gate in the list is applied to every qubit in order.
    ///
    /// Args:
    ///     gates: List of single-qubit parameterized gates (e.g. [StandardGate.RY]).
    ///
    /// Returns:
    ///     A new TwoLocal with the updated setting.
    fn rotation_gates(&self, gates: Vec<PyRef<'_, PyStandardGate>>) -> Self {
        let rust_gates: Vec<StandardGate> = gates.iter().map(|g| g.inner).collect();
        Self {
            inner: self.inner.clone().rotation_gates(rust_gates),
        }
    }

    /// Sets the two-qubit entanglement gate.
    ///
    /// Args:
    ///     gate: A two-qubit gate (e.g. StandardGate.CX, StandardGate.CZ).
    ///
    /// Returns:
    ///     A new TwoLocal with the updated setting.
    fn entanglement_gate(&self, gate: PyRef<'_, PyStandardGate>) -> Self {
        Self {
            inner: self.inner.clone().entanglement_gate(gate.inner),
        }
    }

    /// Sets the entanglement topology.
    ///
    /// Args:
    ///     topology: An EntanglementTopology instance.
    ///
    /// Returns:
    ///     A new TwoLocal with the updated setting.
    fn entanglement(&self, topology: PyRef<'_, PyEntanglementTopology>) -> Self {
        Self {
            inner: self.inner.clone().entanglement(topology.inner.clone()),
        }
    }

    /// Controls whether the final rotation layer is included.
    ///
    /// Args:
    ///     skip: If True, omit the last rotation layer after the final entanglement.
    ///
    /// Returns:
    ///     A new TwoLocal with the updated setting.
    fn skip_final_rotation_layer(&self, skip: bool) -> Self {
        Self {
            inner: self.inner.clone().skip_final_rotation_layer(skip),
        }
    }

    /// Validates the ansatz configuration.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid (e.g. num_qubits == 0).
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the parameterized quantum circuit.
    ///
    /// Parameters are named `{prefix}_0`, `{prefix}_1`, etc.
    ///
    /// Args:
    ///     prefix: Prefix for parameter names (e.g. "theta").
    ///
    /// Returns:
    ///     A Circuit with `num_parameters()` symbolic parameters.
    ///
    /// Raises:
    ///     ValueError: If the ansatz configuration is invalid.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the total number of parameters in the ansatz.
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits in the ansatz.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "TwoLocal(num_qubits={}, num_parameters={})",
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
