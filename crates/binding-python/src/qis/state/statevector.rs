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

//! Python bindings for cqlib-core Statevector module.

use crate::circuit::{PyMeasurement, PyStandardGate, circuit_impl::PyCircuit};
use crate::device::result::{PyExecutionResult, PyOutcome};
use crate::qis::pauli::PyPauliString;
use crate::qis::qis_error_to_py_err;
use crate::qis::state::outcome_probabilities_to_py;
use cqlib_core::qis::state::statevector::Statevector;
use num_complex::Complex64;
use numpy::{PyArray1, PyArray2, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyList};

/// Quantum statevector representing a pure quantum state.
///
/// A statevector describes the quantum state of an N-qubit system as a vector
/// of 2^N complex amplitudes. The state |ψ⟩ = Σᵢ αᵢ|i⟩ is stored with αᵢ
/// as the amplitude for basis state |i⟩ (i in binary representation).
///
/// # Memory Layout
/// The data uses contiguous memory layout (compatible with C/NumPy),
/// where the amplitude at index `i` corresponds to basis state |i⟩ with
/// qubit indices mapping to bits from least significant (qubit 0) to most.
///
/// # Example
/// ```python
/// from cqlib.qis import Statevector
///
/// # Create a 2-qubit state in |00⟩
/// sv = Statevector(2)
///
/// # Apply gates to create Bell state
/// sv.apply_h(0)
/// sv.apply_cx(0, 1)
///
/// # Get probabilities
/// probs = sv.probabilities()
/// print(probs)  # [0.5, 0.0, 0.0, 0.5]
/// ```
#[pyclass(name = "Statevector", module = "cqlib.qis.state", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStatevector {
    pub(crate) inner: Statevector,
}

impl From<Statevector> for PyStatevector {
    fn from(inner: Statevector) -> Self {
        Self { inner }
    }
}

impl From<PyStatevector> for Statevector {
    fn from(value: PyStatevector) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyStatevector {
    /// Creates a new statevector initialized to the |0...0⟩ state.
    ///
    /// The statevector represents the quantum state as a vector of 2^N complex amplitudes,
    /// where N is the number of qubits. All amplitudes are initialized to zero except
    /// the first element (|0...0⟩) which is set to 1.0.
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///
    /// Returns:
    ///     A new Statevector instance in the ground state
    ///
    /// Examples:
    ///     >>> from cqlib.qis import Statevector
    ///     >>> sv = Statevector(2)  # |00⟩ state
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: Statevector::new(num_qubits),
        }
    }

    /// Creates a statevector from initial amplitudes.
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///     initial_state: NumPy array of 2^N complex amplitudes, or a list of complex numbers
    ///
    /// Returns:
    ///     A new Statevector instance
    ///
    /// Raises:
    ///     ValueError: If the state length doesn't match 2^num_qubits or state is not normalized
    ///
    /// Examples:
    ///     >>> import numpy as np
    ///     >>> from cqlib.qis import Statevector
    ///     >>> # Create |+0⟩ = (|00⟩ + |10⟩)/√2
    ///     >>> amps = np.array([1/np.sqrt(2), 0, 1/np.sqrt(2), 0], dtype=complex)
    ///     >>> sv = Statevector.from_state(2, amps)
    #[staticmethod]
    fn from_state<'py>(num_qubits: usize, initial_state: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Try to extract as numpy array
        let data: Vec<Complex64> = if let Ok(array) = initial_state.cast::<PyArray1<Complex64>>() {
            array.to_vec()?
        } else if let Ok(list) = initial_state.cast::<PyList>() {
            // Extract from Python list
            let mut data = Vec::with_capacity(list.len());
            for item in list.iter() {
                if let Ok(py_c) = item.cast::<PyComplex>() {
                    data.push(Complex64::new(py_c.real(), py_c.imag()));
                } else if let Ok(val) = item.extract::<f64>() {
                    data.push(Complex64::new(val, 0.0));
                } else {
                    return Err(PyValueError::new_err(
                        "initial_state must contain complex numbers or floats",
                    ));
                }
            }
            data
        } else {
            return Err(PyValueError::new_err(
                "initial_state must be a numpy array or list of complex numbers",
            ));
        };

        let inner = Statevector::from_state(num_qubits, data).map_err(qis_error_to_py_err)?;

        Ok(Self { inner })
    }

    /// Creates a statevector by simulating a quantum circuit.
    ///
    /// Executes the circuit gates sequentially to evolve the initial |0...0⟩ state.
    ///
    /// Args:
    ///     circuit: The quantum circuit to simulate
    ///
    /// Returns:
    ///     A new Statevector instance after circuit execution
    ///
    /// Raises:
    ///     ValueError: If the circuit contains non-unitary operations
    ///
    /// Examples:
    ///     >>> from cqlib import Circuit
    ///     >>> from cqlib.qis.state import Statevector
    ///     >>> circuit = Circuit(2)
    ///     >>> circuit.h(0)
    ///     >>> circuit.cx(0, 1)
    ///     >>> sv = Statevector.from_circuit(circuit)
    #[staticmethod]
    fn from_circuit(py: Python<'_>, circuit: &PyCircuit) -> PyResult<Self> {
        let inner = py
            .detach(|| Statevector::from_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)?;
        Ok(Self { inner })
    }

    /// Applies a circuit to this statevector in place.
    ///
    /// Args:
    ///     circuit: The quantum circuit to apply
    ///
    /// Raises:
    ///     ValueError: If the circuit qubit count does not match this state
    ///         or contains unsupported operations
    fn apply_circuit(&mut self, py: Python<'_>, circuit: &PyCircuit) -> PyResult<()> {
        py.detach(|| self.inner.apply_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)
    }

    /// Computes the expectation value of an observable.
    ///
    /// Calculates ⟨ψ|O|ψ⟩ for the current state |ψ⟩ and a given observable O.
    ///
    /// Args:
    ///     observable: The observable (Hamiltonian or PauliString)
    ///
    /// Returns:
    ///     The expectation value as a real number
    ///
    /// Raises:
    ///     ValueError: If the qubit counts don't match or the observable type is invalid
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

    /// Returns the number of qubits in the statevector.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits
    }

    /// Returns the statevector amplitudes as a NumPy array.
    ///
    /// Returns:
    ///     A 1D NumPy array of complex amplitudes with length 2^num_qubits.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray1<Complex64>>> {
        Ok(PyArray1::from_vec(py, self.inner.data().to_vec()))
    }

    /// Returns the measurement probabilities for all basis states.
    ///
    /// Computes p(i) = |αᵢ|² for each basis state |i⟩.
    ///
    /// Returns:
    ///     A list of probabilities (floats) with length 2^num_qubits.
    fn probabilities(&self, py: Python<'_>) -> Vec<f64> {
        py.detach(|| self.inner.probabilities())
    }

    /// Applies a standard gate to the statevector.
    ///
    /// Args:
    ///     gate: The standard gate to apply.
    ///     qubits: List of target qubit indices.
    ///     params: List of parameters for parameterized gates.
    #[pyo3(signature = (gate, qubits, params=None))]
    fn apply_standard_gate(
        &mut self,
        py: Python<'_>,
        gate: &PyStandardGate,
        qubits: Vec<usize>,
        params: Option<Vec<f64>>,
    ) -> PyResult<()> {
        let p = params.unwrap_or_default();
        py.detach(|| self.inner.apply_standard_gate(gate.inner, &qubits, &p))
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-X (NOT) gate to the specified qubit.
    fn apply_x(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-Y gate to the specified qubit.
    fn apply_y(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the Pauli-Z gate to the specified qubit.
    fn apply_z(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_z(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the Hadamard gate to the specified qubit.
    fn apply_h(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_h(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the S (phase) gate to the specified qubit.
    fn apply_s(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_s(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the S† (S-dagger) gate to the specified qubit.
    fn apply_sdg(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_sdg(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the T gate to the specified qubit.
    fn apply_t(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_t(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the T† (T-dagger) gate to the specified qubit.
    fn apply_tdg(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_tdg(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a parameterized RX (X-rotation) gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle in radians
    fn apply_rx(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rx(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a parameterized RY (Y-rotation) gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle in radians
    fn apply_ry(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_ry(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a parameterized RZ (Z-rotation) gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle in radians
    fn apply_rz(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rz(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a parameterized phase (P) gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Phase angle in radians
    fn apply_phase(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_phase(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the √X (X/2 plus) gate to the specified qubit.
    fn apply_x2p(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x2p(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the √X† (X/2 minus) gate to the specified qubit.
    fn apply_x2m(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_x2m(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the √Y (Y/2 plus) gate to the specified qubit.
    fn apply_y2p(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y2p(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies the √Y† (Y/2 minus) gate to the specified qubit.
    fn apply_y2m(&mut self, qubit: usize) -> PyResult<()> {
        self.inner.apply_y2m(qubit).map_err(qis_error_to_py_err)
    }

    /// Applies a general single-qubit U gate.
    ///
    /// The U gate is defined as:
    /// U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle θ
    ///     phi: Rotation angle φ
    ///     lambda_: Rotation angle λ
    fn apply_u(&mut self, qubit: usize, theta: f64, phi: f64, lambda_: f64) -> PyResult<()> {
        self.inner
            .apply_u(qubit, theta, phi, lambda_)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a global phase to the statevector.
    fn apply_gphase(&mut self, phi: f64) -> PyResult<()> {
        self.inner.apply_gphase(phi).map_err(qis_error_to_py_err)
    }

    /// Applies the native Pauli-string rotation exp(-i * theta / 2 * P).
    ///
    /// Args:
    ///     pauli: Hermitian Pauli string acting on all statevector qubits.
    ///     theta: Rotation angle in radians.
    fn apply_pauli_rotation(&mut self, pauli: &PyPauliString, theta: f64) -> PyResult<()> {
        self.inner
            .apply_pauli_rotation(&pauli.inner, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the SWAP gate between two qubits.
    fn apply_swap(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_swap(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-X (CNOT) gate.
    ///
    /// Args:
    ///     control: Control qubit index
    ///     target: Target qubit index
    fn apply_cx(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cx(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-Y gate.
    ///
    /// Args:
    ///     control: Control qubit index
    ///     target: Target qubit index
    fn apply_cy(&mut self, control: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_cy(control, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-Z gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    fn apply_cz(&mut self, q0: usize, q1: usize) -> PyResult<()> {
        self.inner.apply_cz(q0, q1).map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-RX gate.
    fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_crx(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-RY gate.
    fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_cry(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-RZ gate.
    fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_crz(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RXX (Ising XX) gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     theta: Rotation angle
    fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rxx(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RYY (Ising YY) gate.
    fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_ryy(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZZ (Ising ZZ) gate.
    fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzz(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZX gate.
    fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzx(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY gate.
    fn apply_xy(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY2P gate: Rz(θ - π/2) Ry(π/2) Rz(π/2 - θ).
    fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy2p(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY2M gate: Rz(-θ + π/2) Ry(-π/2) Rz(-π/2 + θ).
    fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy2m(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RXY gate.
    fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_rxy(qubit, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the Fermionic Simulation (fSim) gate.
    fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_fsim(q0, q1, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the CCX (Toffoli) gate.
    ///
    /// Args:
    ///     c0: First control qubit index
    ///     c1: Second control qubit index
    ///     target: Target qubit index
    fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> PyResult<()> {
        self.inner
            .apply_ccx(c0, c1, target)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a custom single-qubit gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     matrix: 2x2 complex matrix as a NumPy 2D array
    fn apply_single_qubit_gate<'py>(
        &mut self,
        py: Python<'_>,
        qubit: usize,
        matrix: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let mat: [[Complex64; 2]; 2] = if let Ok(array) = matrix.cast::<PyArray2<Complex64>>() {
            let readonly = array.try_readonly().map_err(|e| {
                PyValueError::new_err(format!("Failed to get readonly view: {}", e))
            })?;
            let shape = readonly.shape();
            if shape != [2, 2] {
                return Err(PyValueError::new_err(
                    "Single-qubit gate matrix must be 2x2",
                ));
            }
            let array = readonly.as_array();
            [
                [array[[0, 0]], array[[0, 1]]],
                [array[[1, 0]], array[[1, 1]]],
            ]
        } else {
            return Err(PyValueError::new_err("matrix must be a 2x2 numpy array"));
        };

        py.detach(|| self.inner.apply_single_qubit_gate(qubit, mat))
            .map_err(qis_error_to_py_err)?;
        Ok(())
    }

    /// Applies a custom two-qubit gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     matrix: 4x4 complex matrix as a NumPy array
    fn apply_double_qubits_gate<'py>(
        &mut self,
        py: Python<'_>,
        q0: usize,
        q1: usize,
        matrix: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let mat: [[Complex64; 4]; 4] = if let Ok(array) = matrix.cast::<PyArray2<Complex64>>() {
            let readonly = array.try_readonly().map_err(|e| {
                PyValueError::new_err(format!("Failed to get readonly view: {}", e))
            })?;
            let shape = readonly.shape();
            if shape != [4, 4] {
                return Err(PyValueError::new_err("Two-qubit gate matrix must be 4x4"));
            }
            let array = readonly.as_array();
            [
                [array[[0, 0]], array[[0, 1]], array[[0, 2]], array[[0, 3]]],
                [array[[1, 0]], array[[1, 1]], array[[1, 2]], array[[1, 3]]],
                [array[[2, 0]], array[[2, 1]], array[[2, 2]], array[[2, 3]]],
                [array[[3, 0]], array[[3, 1]], array[[3, 2]], array[[3, 3]]],
            ]
        } else {
            return Err(PyValueError::new_err("matrix must be a 4x4 numpy array"));
        };

        py.detach(|| self.inner.apply_two_qubit_gate(q0, q1, mat))
            .map_err(qis_error_to_py_err)?;
        Ok(())
    }

    /// Applies an arbitrary n-qubit unitary gate.
    ///
    /// Args:
    ///     qubits: List of qubit indices the gate acts on
    ///     matrix: The unitary matrix as a 2^n x 2^n NumPy array
    ///
    /// Raises:
    ///     ValueError: If the matrix dimensions don't match qubit count
    fn apply_unitary_gate<'py>(
        &mut self,
        py: Python<'_>,
        qubits: Vec<usize>,
        matrix: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let array = matrix
            .cast::<PyArray2<Complex64>>()
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

        let flat: numpy::ndarray::Array2<Complex64> = readonly.as_array().to_owned();
        py.detach(|| self.inner.apply_unitary_gate(&qubits, &flat))
            .map_err(qis_error_to_py_err)
    }

    /// Resets the specified qubit to the |0⟩ state by measuring and flipping if 1.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///
    /// Raises:
    ///     IndexError: If qubit index is out of bounds.
    fn reset(&mut self, py: Python<'_>, qubit: usize) -> PyResult<()> {
        py.detach(|| self.inner.reset(qubit))
            .map_err(qis_error_to_py_err)
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

    /// Returns a string representation of the statevector.
    fn __repr__(&self) -> String {
        format!(
            "Statevector(num_qubits={}, amplitudes={})",
            self.inner.num_qubits,
            self.inner.data().len()
        )
    }

    /// Returns a copy of this statevector.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
