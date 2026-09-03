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

//! Python bindings for cqlib-core StabilizerState module.

use crate::circuit::{PyMeasurement, circuit_impl::PyCircuit};
use crate::device::result::{PyExecutionResult, PyOutcome};
use crate::qis::pauli::PyPauliString;
use crate::qis::qis_error_to_py_err;
use crate::qis::state::classical::PyClassicalState;
use crate::qis::state::outcome_probabilities_to_py;
use cqlib_core::qis::state::stabilizer::{CircuitExecutionResult, StabilizerState};
use pyo3::prelude::*;

/// Result of executing a Clifford circuit with a stabilizer simulator.
#[pyclass(
    name = "StabilizerCircuitResult",
    module = "cqlib.qis.state",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyStabilizerCircuitResult {
    inner: CircuitExecutionResult,
}

#[pymethods]
impl PyStabilizerCircuitResult {
    /// Final stabilizer state after circuit execution.
    #[getter]
    fn state(&self) -> PyStabilizerState {
        PyStabilizerState::from(self.inner.state.clone())
    }

    /// Runtime classical values and variables produced during circuit execution.
    #[getter]
    fn classical(&self) -> PyClassicalState {
        PyClassicalState::from(self.inner.classical.clone())
    }

    /// Returns a string representation of the circuit execution result.
    fn __repr__(&self) -> String {
        format!(
            "StabilizerCircuitResult(num_qubits={})",
            self.inner.state.num_qubits()
        )
    }
}

/// Stabilizer state simulator for Clifford circuits.
#[pyclass(
    name = "StabilizerState",
    module = "cqlib.qis.state",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStabilizerState {
    pub(crate) inner: StabilizerState,
}

impl From<StabilizerState> for PyStabilizerState {
    fn from(inner: StabilizerState) -> Self {
        Self { inner }
    }
}

impl From<PyStabilizerState> for StabilizerState {
    fn from(value: PyStabilizerState) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyStabilizerState {
    /// Creates a new stabilizer state initialized to |0...0>.
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: StabilizerState::new(num_qubits),
        }
    }

    /// Creates a stabilizer state by simulating a Clifford circuit.
    #[staticmethod]
    fn from_circuit(py: Python<'_>, circuit: &PyCircuit) -> PyResult<Self> {
        py.detach(|| StabilizerState::from_circuit(&circuit.inner))
            .map(Self::from)
            .map_err(qis_error_to_py_err)
    }

    /// Executes a Clifford circuit and returns final state plus runtime classical data.
    #[staticmethod]
    fn run_circuit(py: Python<'_>, circuit: &PyCircuit) -> PyResult<PyStabilizerCircuitResult> {
        py.detach(|| StabilizerState::run_circuit(&circuit.inner))
            .map(|inner| PyStabilizerCircuitResult { inner })
            .map_err(qis_error_to_py_err)
    }

    /// Applies a Clifford circuit to this state in place.
    fn apply_circuit(&mut self, py: Python<'_>, circuit: &PyCircuit) -> PyResult<()> {
        py.detach(|| self.inner.apply_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)
    }

    /// Returns the number of qubits in the state.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits()
    }

    /// Applies a Hadamard gate.
    fn apply_h(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_h(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies an S gate.
    fn apply_s(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_s(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies an S-dagger gate.
    fn apply_sdg(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_sdg(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies an X gate.
    fn apply_x(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a Y gate.
    fn apply_y(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a Z gate.
    fn apply_z(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_z(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies an X/2 plus Clifford gate.
    fn apply_x2p(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x2p(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies an X/2 minus Clifford gate.
    fn apply_x2m(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x2m(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a Y/2 plus Clifford gate.
    fn apply_y2p(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y2p(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a Y/2 minus Clifford gate.
    fn apply_y2m(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y2m(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a controlled-X gate.
    fn apply_cx(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cx(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a controlled-Y gate.
    fn apply_cy(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cy(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a controlled-Z gate.
    fn apply_cz(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_cz(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Applies a SWAP gate.
    fn apply_swap(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_swap(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Measures one qubit and collapses the state.
    fn measure(&mut self, py: Python<'_>, qubit: usize) -> PyResult<bool> {
        py.detach(|| self.inner.measure(qubit))
            .map_err(qis_error_to_py_err)
    }

    /// Measures all qubits and collapses the state.
    fn measure_all(&mut self, py: Python<'_>) -> PyOutcome {
        PyOutcome::from(py.detach(|| self.inner.measure_all()))
    }

    /// Resets one qubit to |0>.
    fn reset(&mut self, py: Python<'_>, qubit: usize) -> PyResult<()> {
        py.detach(|| self.inner.reset(qubit))
            .map_err(qis_error_to_py_err)
    }

    /// Returns the probability of a computational basis bitstring.
    fn probability_of(&self, py: Python<'_>, bits: Vec<bool>) -> PyResult<f64> {
        py.detach(|| self.inner.probability_of(&bits))
            .map_err(qis_error_to_py_err)
    }

    /// Returns the full computational-basis probability distribution.
    fn probabilities(&self, py: Python<'_>) -> PyResult<Vec<f64>> {
        py.detach(|| self.inner.probabilities())
            .map_err(qis_error_to_py_err)
    }

    /// Samples measurement outcomes without mutating this state.
    fn sample_shots(&self, py: Python<'_>, shots: usize) -> Vec<PyOutcome> {
        py.detach(|| self.inner.sample_shots(shots))
            .into_iter()
            .map(PyOutcome::from)
            .collect()
    }

    /// Samples measurement outcomes according to a circuit measurement receipt.
    fn sample(
        &self,
        py: Python<'_>,
        measurement: &PyMeasurement,
        shots: usize,
    ) -> PyResult<PyExecutionResult> {
        py.detach(|| self.inner.sample(&measurement.inner, shots))
            .map(PyExecutionResult::from)
            .map_err(qis_error_to_py_err)
    }

    /// Returns marginal probabilities according to a circuit measurement receipt.
    fn probs(
        &self,
        py: Python<'_>,
        measurement: &PyMeasurement,
    ) -> PyResult<std::collections::HashMap<PyOutcome, f64>> {
        py.detach(|| self.inner.probs(&measurement.inner))
            .map(outcome_probabilities_to_py)
            .map_err(qis_error_to_py_err)
    }

    /// Returns the stabilizer generators.
    fn get_stabilizers(&self) -> Vec<PyPauliString> {
        self.inner
            .get_stabilizers()
            .into_iter()
            .map(PyPauliString::from)
            .collect()
    }

    /// Returns the destabilizer generators.
    fn get_destabilizers(&self) -> Vec<PyPauliString> {
        self.inner
            .get_destabilizers()
            .into_iter()
            .map(PyPauliString::from)
            .collect()
    }

    /// Returns the expectation value of a Pauli string: -1, 0, or 1.
    fn pauli_expectation(&self, pauli: &PyPauliString) -> PyResult<i32> {
        self.inner
            .pauli_expectation(&pauli.inner)
            .map_err(qis_error_to_py_err)
    }

    /// Returns a Stim-like tableau representation.
    fn to_stim_format(&self) -> String {
        self.inner.to_stim_format()
    }

    /// Returns a copy of this stabilizer state.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Returns a string representation of the stabilizer state.
    fn __repr__(&self) -> String {
        format!("StabilizerState(num_qubits={})", self.inner.num_qubits())
    }
}
