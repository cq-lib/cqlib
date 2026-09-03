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

//! Python bindings for `QAOAAnsatz`.

use cqlib_core::circuit::ansatz::qaoa::QAOAAnsatz;
use cqlib_core::circuit::ansatz::traits::Ansatz;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::circuit::circuit_impl::PyCircuit;
use crate::qis::hamiltonian::PyHamiltonian;

use super::hamiltonian_evolution::PyEvolutionStrategy;

/// The Quantum Approximate Optimization Algorithm (QAOA) ansatz.
///
/// QAOA alternates between cost and mixer Hamiltonian evolutions:
///     U(β, γ) = [e^{-i β_p H_M} e^{-i γ_p H_C}] ... [e^{-i β_1 H_M} e^{-i γ_1 H_C}]
///
/// where H_C is the cost Hamiltonian and H_M is the mixer (default: X on each qubit).
///
/// Builder methods return a new QAOAAnsatz (immutable builder pattern).
///
/// Examples:
///     >>> from cqlib.circuit.ansatz import QAOAAnsatz
///     >>> from cqlib import Hamiltonian, PauliString
///     >>> h_c = Hamiltonian(2)
///     >>> h_c.add_term(PauliString.from_str("ZZ"), 0.5)
///     >>> ansatz = QAOAAnsatz(h_c).reps(3)
///     >>> circuit = ansatz.build_circuit("p")
///     >>> ansatz.num_parameters()
///     6
#[pyclass(
    name = "QAOAAnsatz",
    module = "cqlib.circuit.ansatz",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyQAOAAnsatz {
    pub(crate) inner: QAOAAnsatz,
}

impl From<QAOAAnsatz> for PyQAOAAnsatz {
    fn from(inner: QAOAAnsatz) -> Self {
        Self { inner }
    }
}

impl From<PyQAOAAnsatz> for QAOAAnsatz {
    fn from(value: PyQAOAAnsatz) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyQAOAAnsatz {
    /// Creates a new QAOAAnsatz from a cost Hamiltonian.
    ///
    /// The default mixer is a sum of X operators on each qubit: H_M = Σ X_i.
    ///
    /// Args:
    ///     cost_operator: The cost Hamiltonian H_C (must have at least 1 qubit).
    ///
    /// Raises:
    ///     ValueError: If the cost operator is invalid (e.g. 0 qubits).
    #[new]
    fn new(cost_operator: PyRef<'_, PyHamiltonian>) -> PyResult<Self> {
        QAOAAnsatz::new(cost_operator.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Sets the number of QAOA layers (p).
    ///
    /// Each layer adds one pair of (cost evolution, mixer evolution) parameters.
    /// Total parameters = 2 * reps.
    ///
    /// Args:
    ///     n: Number of QAOA layers p ≥ 1.
    ///
    /// Returns:
    ///     A new QAOAAnsatz with the updated setting.
    fn reps(&self, n: usize) -> Self {
        Self {
            inner: self.inner.clone().reps(n),
        }
    }

    /// Overrides the default X-mixer with a custom mixer Hamiltonian.
    ///
    /// The mixer must act on the same number of qubits as the cost operator.
    ///
    /// Args:
    ///     mixer_operator: A Hamiltonian to use as the mixer H_M.
    ///
    /// Returns:
    ///     A new QAOAAnsatz with the updated mixer.
    ///
    /// Raises:
    ///     ValueError: If the mixer has a different number of qubits than the cost operator.
    fn mixer(&self, mixer_operator: PyRef<'_, PyHamiltonian>) -> PyResult<Self> {
        self.inner
            .clone()
            .mixer(mixer_operator.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Sets the initial state circuit prepended before the QAOA layers.
    ///
    /// Defaults to the uniform superposition state (H on all qubits).
    ///
    /// Args:
    ///     circuit: A Circuit acting on the same number of qubits.
    ///
    /// Returns:
    ///     A new QAOAAnsatz with the updated initial state.
    ///
    /// Raises:
    ///     ValueError: If the circuit has a different number of qubits.
    fn initial_state(&self, circuit: PyRef<'_, PyCircuit>) -> PyResult<Self> {
        self.inner
            .clone()
            .initial_state(circuit.inner.clone())
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Sets the Hamiltonian evolution strategy for both cost and mixer layers.
    ///
    /// Args:
    ///     strategy: The EvolutionStrategy used to compile cost and mixer
    ///         Hamiltonian evolutions.
    ///
    /// Returns:
    ///     A new QAOAAnsatz with the updated evolution strategy.
    fn evolution_strategy(&self, strategy: PyRef<'_, PyEvolutionStrategy>) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .evolution_strategy(strategy.inner.clone()),
        }
    }

    /// Validates the ansatz configuration.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Builds the QAOA circuit.
    ///
    /// Parameters are named `{prefix}_gamma_0`, `{prefix}_beta_0`,
    /// `{prefix}_gamma_1`, `{prefix}_beta_1`, ... for each QAOA layer.
    ///
    /// Args:
    ///     prefix: Prefix for parameter names (e.g. "p").
    ///
    /// Returns:
    ///     A Circuit with 2 * reps symbolic parameters.
    ///
    /// Raises:
    ///     ValueError: If the configuration is invalid.
    fn build_circuit(&self, prefix: &str) -> PyResult<PyCircuit> {
        self.inner
            .build_circuit(prefix)
            .map(|c| PyCircuit { inner: c })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the total number of parameters (= 2 * reps).
    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }

    /// Returns the number of qubits.
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    fn __repr__(&self) -> String {
        format!(
            "QAOAAnsatz(num_qubits={}, num_parameters={})",
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
