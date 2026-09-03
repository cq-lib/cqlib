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

//! Python bindings for cqlib-core DensityMatrixNoise module.

use crate::circuit::{PyCircuit, PyMeasurement, PyStandardGate};
use crate::device::noise::PyNoiseModel;
use crate::device::result::{PyExecutionResult, PyOutcome};
use crate::qis::qis_error_to_py_err;
use crate::qis::state::outcome_probabilities_to_py;
use cqlib_core::qis::state::density_matrix_noise::DensityMatrixNoise;
use numpy::{PyArray2, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// A density matrix quantum simulator with noise modeling capabilities.
///
/// This simulator wraps the `DensityMatrix` kernel and automatically applies
/// Kraus operator noise after each quantum gate based on a configurable
/// `NoiseModel`. It supports both interactive gate-by-gate simulation and
/// batch circuit execution.
///
/// # Example
/// ```python
/// from cqlib.qis.state import DensityMatrixNoise
/// from cqlib.device import NoiseModel, SingleQubitNoise
/// from cqlib.circuit import StandardGate
///
/// # Create noise model with bit-flip noise on X gates
/// noise_model = NoiseModel()
/// noise = SingleQubitNoise.bit_flip(p=0.01)
/// noise_model.add_single_qubit_error(StandardGate.X, 0, noise)
///
/// # Create simulator and apply noisy gate
/// sim = DensityMatrixNoise(1, noise_model)
/// sim.apply_x(0)
///
/// # Get probabilities (P(|1>) ~ 0.99 due to 1% bit-flip noise)
/// probs = sim.probabilities()
/// ```
#[pyclass(
    name = "DensityMatrixNoise",
    module = "cqlib.qis.state",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDensityMatrixNoise {
    pub(crate) inner: DensityMatrixNoise,
}

impl From<DensityMatrixNoise> for PyDensityMatrixNoise {
    fn from(inner: DensityMatrixNoise) -> Self {
        Self { inner }
    }
}

impl From<PyDensityMatrixNoise> for DensityMatrixNoise {
    fn from(value: PyDensityMatrixNoise) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyDensityMatrixNoise {
    /// Creates a new simulator with the specified number of qubits and optional noise model.
    ///
    /// Args:
    ///     num_qubits: The number of qubits in the quantum system
    ///     noise_model: Optional NoiseModel defining gate and readout errors
    ///
    /// Returns:
    ///     A new DensityMatrixNoise instance
    ///
    /// Examples:
    ///     >>> from cqlib.qis.state import DensityMatrixNoise
    ///     >>> # Simulator without noise (ideal simulation)
    ///     >>> sim = DensityMatrixNoise(3, None)
    ///     >>> # Simulator with empty noise model
    ///     >>> from cqlib.device import NoiseModel
    ///     >>> sim = DensityMatrixNoise(2, NoiseModel())
    #[new]
    #[pyo3(signature = (num_qubits, noise_model=None))]
    fn new(num_qubits: usize, noise_model: Option<PyNoiseModel>) -> Self {
        let model = noise_model.map(|m| m.inner);
        Self {
            inner: DensityMatrixNoise::new(num_qubits, model),
        }
    }

    /// Simulates a circuit, applying noise after each operation.
    ///
    /// The circuit is decomposed into basis gates before execution. Noise is
    /// applied according to the noise model immediately following each gate.
    ///
    /// Args:
    ///     circuit: The quantum circuit to simulate
    ///     noise_model: Optional NoiseModel for noise injection
    ///
    /// Returns:
    ///     A new DensityMatrixNoise instance after circuit execution
    ///
    /// Raises:
    ///     ValueError: If the circuit contains unsupported operations
    #[staticmethod]
    #[pyo3(signature = (circuit, noise_model=None))]
    fn from_circuit(
        py: Python<'_>,
        circuit: &PyCircuit,
        noise_model: Option<PyNoiseModel>,
    ) -> PyResult<Self> {
        let model = noise_model.map(|m| m.inner);
        let inner = py
            .detach(|| DensityMatrixNoise::from_circuit(&circuit.inner, model))
            .map_err(qis_error_to_py_err)?;
        Ok(Self { inner })
    }

    /// Applies a circuit to this simulator in place.
    fn apply_circuit(&mut self, py: Python<'_>, circuit: &PyCircuit) -> PyResult<()> {
        py.detach(|| self.inner.apply_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)
    }

    /// Returns the number of qubits in the simulator.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.state.num_qubits
    }

    /// Returns the underlying density matrix state as a 2D NumPy array.
    ///
    /// Returns:
    ///     A 2D NumPy array of complex numbers with shape (2^num_qubits, 2^num_qubits).
    #[getter]
    fn state<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyArray2<num_complex::Complex64>>> {
        let dim = 1 << self.inner.state.num_qubits;
        let data = self.inner.state.data();
        let mut matrix = Vec::with_capacity(data.len());
        for i in 0..dim {
            for j in 0..dim {
                matrix.push(data[i * dim + j]);
            }
        }
        let array = numpy::ndarray::Array2::from_shape_vec((dim, dim), matrix)
            .map_err(|e| PyValueError::new_err(format!("Failed to create array: {}", e)))?;
        Ok(PyArray2::from_owned_array(py, array))
    }

    /// Returns the ideal measurement probabilities without readout noise.
    fn probabilities(&self, py: Python<'_>) -> Vec<f64> {
        py.detach(|| self.inner.probabilities())
    }

    /// Computes measurement probabilities with readout error modeling.
    ///
    /// Args:
    ///     qubits: Indices of qubits to measure
    ///
    /// Returns:
    ///     A vector of probabilities for all 2^n computational basis states.
    fn probabilities_with_readout(&self, py: Python<'_>, qubits: Vec<usize>) -> PyResult<Vec<f64>> {
        py.detach(|| self.inner.probabilities_with_readout(&qubits))
            .map_err(qis_error_to_py_err)
    }

    /// Applies a standard gate to the density matrix, automatically applying any associated noise.
    ///
    /// Args:
    ///     gate: The standard gate to apply.
    ///     qubits: List of target qubit indices.
    ///     params: List of parameters for parameterized gates.
    #[pyo3(signature = (gate, qubits, params=None))]
    fn apply_standard_gate_noise(
        &mut self,
        py: Python<'_>,
        gate: &PyStandardGate,
        qubits: Vec<usize>,
        params: Option<Vec<f64>>,
    ) -> PyResult<()> {
        let p = params.unwrap_or_default();
        py.detach(|| {
            self.inner
                .apply_standard_gate_noise(gate.inner, &qubits, &p)
        })
        .map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-X gate with optional noise.
    fn apply_x(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_x(q).map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-Y gate with optional noise.
    fn apply_y(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_y(q).map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-Z gate with optional noise.
    fn apply_z(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_z(q).map_err(qis_error_to_py_err)
    }

    /// Applies the Hadamard gate with optional noise.
    fn apply_h(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_h(q).map_err(qis_error_to_py_err)
    }

    /// Applies the S gate with optional noise.
    fn apply_s(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_s(q).map_err(qis_error_to_py_err)
    }

    /// Applies the S dagger gate with optional noise.
    fn apply_sdg(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_sdg(q).map_err(qis_error_to_py_err)
    }

    /// Applies the T gate with optional noise.
    fn apply_t(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_t(q).map_err(qis_error_to_py_err)
    }

    /// Applies the T dagger gate with optional noise.
    fn apply_tdg(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_tdg(q).map_err(qis_error_to_py_err)
    }

    /// Applies a rotation around the X-axis with optional noise.
    fn apply_rx(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_rx(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies a rotation around the Y-axis with optional noise.
    fn apply_ry(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_ry(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies a rotation around the Z-axis with optional noise.
    fn apply_rz(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_rz(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies the phase gate with optional noise.
    fn apply_phase(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_phase(q, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the global phase gate with optional noise.
    fn apply_gphase(&mut self, phi: f64) -> PyResult<()> {
        self.inner.apply_gphase(phi).map_err(qis_error_to_py_err)
    }

    /// Applies the X2P gate with optional noise.
    fn apply_x2p(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_x2p(q).map_err(qis_error_to_py_err)
    }

    /// Applies the X2M gate with optional noise.
    fn apply_x2m(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_x2m(q).map_err(qis_error_to_py_err)
    }

    /// Applies the Y2P gate with optional noise.
    fn apply_y2p(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_y2p(q).map_err(qis_error_to_py_err)
    }

    /// Applies the Y2M gate with optional noise.
    fn apply_y2m(&mut self, q: usize) -> PyResult<()> {
        self.inner.apply_y2m(q).map_err(qis_error_to_py_err)
    }

    /// Applies an arbitrary rotation on the Bloch sphere with optional noise.
    fn apply_rxy(&mut self, q: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_rxy(q, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY gate with optional noise.
    fn apply_xy(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_xy(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies the XY2P gate with optional noise.
    fn apply_xy2p(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_xy2p(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies the XY2M gate with optional noise.
    fn apply_xy2m(&mut self, q: usize, theta: f64) -> PyResult<()> {
        self.inner.apply_xy2m(q, theta).map_err(qis_error_to_py_err)
    }

    /// Applies a general single-qubit U gate with optional noise.
    fn apply_u(&mut self, q: usize, theta: f64, phi: f64, lambda_: f64) -> PyResult<()> {
        self.inner
            .apply_u(q, theta, phi, lambda_)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-X gate with optional noise.
    fn apply_cx(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cx(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-Y gate with optional noise.
    fn apply_cy(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cy(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-Z gate with optional noise.
    fn apply_cz(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_cz(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Applies the SWAP gate with optional noise.
    fn apply_swap(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_swap(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Applies the RXX gate with optional noise.
    fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rxx(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RYY gate with optional noise.
    fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_ryy(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZZ gate with optional noise.
    fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzz(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZX gate with optional noise.
    fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzx(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-RX gate with optional noise.
    fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_crx(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-RY gate with optional noise.
    fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_cry(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Controlled-RZ gate with optional noise.
    fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_crz(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the fSim gate with optional noise.
    fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_fsim(q0, q1, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Toffoli gate with optional noise.
    fn apply_ccx(&mut self, c1: usize, c2: usize, t: usize) -> PyResult<()> {
        self.inner.apply_ccx(c1, c2, t).map_err(qis_error_to_py_err)
    }

    /// Applies an arbitrary unitary gate to the state.
    ///
    /// Note: No noise is applied for generic unitary gates.
    fn apply_unitary_gate<'py>(
        &mut self,
        py: Python<'_>,
        qubits: Vec<usize>,
        matrix: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let array = matrix
            .cast::<PyArray2<num_complex::Complex64>>()
            .map_err(|_| PyValueError::new_err("matrix must be a numpy array"))?;

        let readonly = array
            .try_readonly()
            .map_err(|e| PyValueError::new_err(format!("Failed to get readonly view: {}", e)))?;

        let expected_dim = 1 << qubits.len();
        let shape = readonly.shape();
        if shape != [expected_dim, expected_dim] {
            return Err(PyValueError::new_err(format!(
                "Matrix shape {:?} doesn't match qubit count {} (expected {}x{})",
                shape,
                qubits.len(),
                expected_dim,
                expected_dim
            )));
        }

        let flat: numpy::ndarray::Array2<num_complex::Complex64> = readonly.as_array().to_owned();
        py.detach(|| self.inner.apply_unitary_gate(&qubits, &flat))
            .map_err(qis_error_to_py_err)
    }

    /// Computes the expectation value of an observable.
    fn expectation(&self, py: Python<'_>, observable: &Bound<'_, PyAny>) -> PyResult<f64> {
        if let Ok(h) = observable.extract::<crate::qis::hamiltonian::PyHamiltonian>() {
            py.detach(|| self.inner.expectation(&h.inner))
                .map_err(qis_error_to_py_err)
        } else if let Ok(ps) = observable.extract::<crate::qis::pauli::PyPauliString>() {
            py.detach(|| self.inner.expectation(&ps.inner))
                .map_err(qis_error_to_py_err)
        } else {
            Err(PyValueError::new_err(
                "Observable must be a Hamiltonian or a PauliString",
            ))
        }
    }

    /// Returns a string representation of the simulator.
    fn __repr__(&self) -> String {
        let dim = 1 << self.inner.state.num_qubits;
        format!(
            "DensityMatrixNoise(num_qubits={}, state_shape=({}, {}))",
            self.inner.state.num_qubits, dim, dim
        )
    }

    /// Returns a copy of this simulator.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
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

    /// Samples measurement outcomes with readout noise, without mutating this state.
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

    /// Returns ideal marginal probabilities according to a circuit measurement receipt.
    fn probs(
        &self,
        py: Python<'_>,
        measurement: &PyMeasurement,
    ) -> PyResult<std::collections::HashMap<PyOutcome, f64>> {
        py.detach(|| self.inner.probs(&measurement.inner))
            .map(outcome_probabilities_to_py)
            .map_err(qis_error_to_py_err)
    }

    /// Returns marginal probabilities with readout noise applied.
    fn probs_with_readout(
        &self,
        py: Python<'_>,
        measurement: &PyMeasurement,
    ) -> PyResult<std::collections::HashMap<PyOutcome, f64>> {
        py.detach(|| self.inner.probs_with_readout(&measurement.inner))
            .map(outcome_probabilities_to_py)
            .map_err(qis_error_to_py_err)
    }
}
