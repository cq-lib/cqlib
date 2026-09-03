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

//! Python bindings for cqlib-core DensityMatrix module.

use crate::circuit::{PyCircuit, PyMeasurement, PyStandardGate};
use crate::device::result::{PyExecutionResult, PyOutcome};
use crate::qis::qis_error_to_py_err;
use crate::qis::state::outcome_probabilities_to_py;
use cqlib_core::qis::state::density_matrix::DensityMatrix;
use num_complex::Complex64;
use numpy::{PyArray1, PyArray2, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyList};

/// Quantum density matrix representing mixed or pure quantum states.
///
/// A density matrix describes the statistical state of an N-qubit quantum system.
/// Unlike a statevector which can only represent pure states, a density matrix
/// can represent mixed states (ensembles of pure states).
///
/// # Memory Layout
/// The data uses contiguous memory layout representing a flattened 2^N x 2^N matrix.
/// To optimize simulation performance, the simulator employs a 2N-qubit isomorphism:
/// - The matrix is treated as a statevector of 2N qubits.
/// - The "ket" side (Left U) acts on the upper N qubits (indices N to 2N-1).
/// - The "bra" side (Right U†) acts on the lower N qubits (indices 0 to N-1).
///
/// # Example
/// ```python
/// from cqlib.qis import DensityMatrix
///
/// # Create a 1-qubit density matrix in state |0><0|
/// dm = DensityMatrix(1)
///
/// # Apply Hadamard gate -> |+><+|
/// dm.apply_h(0)
///
/// # Probabilities should be 0.5 for both |0> and |1>
/// probs = dm.probabilities()
/// print(probs)  # [0.5, 0.5]
/// ```
#[pyclass(
    name = "DensityMatrix",
    module = "cqlib.qis.state",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDensityMatrix {
    pub(crate) inner: DensityMatrix,
}

impl From<DensityMatrix> for PyDensityMatrix {
    fn from(inner: DensityMatrix) -> Self {
        Self { inner }
    }
}

impl From<PyDensityMatrix> for DensityMatrix {
    fn from(value: PyDensityMatrix) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyDensityMatrix {
    /// Creates a new density matrix initialized to the pure state |0...0><0...0|.
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///
    /// Returns:
    ///     A new DensityMatrix instance in the ground state
    ///
    /// Examples:
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> dm = DensityMatrix(2)  # |00><00| state
    #[new]
    fn new(num_qubits: usize) -> Self {
        Self {
            inner: DensityMatrix::new(num_qubits),
        }
    }

    /// Creates a density matrix from an initial statevector (pure state).
    ///
    /// Internally computes the outer product ρ = |ψ⟩⟨ψ|.
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///     initial_state: NumPy array of 2^N complex amplitudes, or a list of complex numbers
    ///
    /// Returns:
    ///     A new DensityMatrix instance
    ///
    /// Raises:
    ///     ValueError: If the state length doesn't match 2^num_qubits or state is not normalized
    ///
    /// Examples:
    ///     >>> import numpy as np
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> # Create |+> = (|0> + |1>)/√2
    ///     >>> amps = np.array([1/np.sqrt(2), 1/np.sqrt(2)], dtype=complex)
    ///     >>> dm = DensityMatrix.from_state(1, amps)
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

        let inner = DensityMatrix::from_state(num_qubits, data).map_err(qis_error_to_py_err)?;

        Ok(Self { inner })
    }

    /// Creates a density matrix directly from a flattened 2^N x 2^N matrix.
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///     dm_state: NumPy array of 4^N complex values representing the density matrix
    ///
    /// Returns:
    ///     A new DensityMatrix instance
    ///
    /// Raises:
    ///     ValueError: If the matrix length doesn't match 4^num_qubits or trace is not 1
    ///
    /// Examples:
    ///     >>> import numpy as np
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> # Create |0><0| density matrix for 1 qubit
    ///     >>> dm_flat = np.array([1, 0, 0, 0], dtype=complex)
    ///     >>> dm = DensityMatrix.from_density_matrix(1, dm_flat)
    #[staticmethod]
    fn from_density_matrix<'py>(num_qubits: usize, dm_state: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Try to extract as numpy array
        let data: Vec<Complex64> = if let Ok(array) = dm_state.cast::<PyArray1<Complex64>>() {
            array.to_vec()?
        } else if let Ok(list) = dm_state.cast::<PyList>() {
            let mut data = Vec::with_capacity(list.len());
            for item in list.iter() {
                if let Ok(py_c) = item.cast::<PyComplex>() {
                    data.push(Complex64::new(py_c.real(), py_c.imag()));
                } else if let Ok(val) = item.extract::<f64>() {
                    data.push(Complex64::new(val, 0.0));
                } else {
                    return Err(PyValueError::new_err(
                        "dm_state must contain complex numbers or floats",
                    ));
                }
            }
            data
        } else {
            return Err(PyValueError::new_err(
                "dm_state must be a numpy array or list of complex numbers",
            ));
        };

        let inner = DensityMatrix::from_density_matrix_state(num_qubits, data)
            .map_err(qis_error_to_py_err)?;

        Ok(Self { inner })
    }

    /// Creates a density matrix by simulating a quantum circuit.
    ///
    /// Executes the circuit gates sequentially to evolve the initial |0...0⟩⟨0...0| state.
    ///
    /// Args:
    ///     circuit: The quantum circuit to simulate
    ///
    /// Returns:
    ///     A new DensityMatrix instance after circuit execution
    ///
    /// Raises:
    ///     ValueError: If the circuit contains unsupported operations
    ///
    /// Examples:
    ///     >>> from cqlib import Circuit
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> circuit = Circuit(2)
    ///     >>> circuit.h(0)
    ///     >>> circuit.cx(0, 1)
    ///     >>> dm = DensityMatrix.from_circuit(circuit)
    #[staticmethod]
    fn from_circuit(py: Python<'_>, circuit: &PyCircuit) -> PyResult<Self> {
        let inner = py
            .detach(|| DensityMatrix::from_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)?;
        Ok(Self { inner })
    }

    /// Creates the maximally mixed state I / 2^N.
    ///
    /// The result is a diagonal matrix with entries 1/2^N on the diagonal,
    /// representing a uniform classical mixture over all computational basis
    /// states. For zero qubits this yields the 1x1 matrix [1].
    ///
    /// Args:
    ///     num_qubits: Number of qubits in the system
    ///
    /// Returns:
    ///     A new DensityMatrix instance in the maximally mixed state
    ///
    /// Examples:
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> dm = DensityMatrix.maximally_mixed(1)
    ///     >>> dm.probabilities()
    ///     [0.5, 0.5]
    #[staticmethod]
    fn maximally_mixed(num_qubits: usize) -> Self {
        Self {
            inner: DensityMatrix::maximally_mixed(num_qubits),
        }
    }

    /// Applies a circuit to this density matrix in place.
    fn apply_circuit(&mut self, py: Python<'_>, circuit: &PyCircuit) -> PyResult<()> {
        py.detach(|| self.inner.apply_circuit(&circuit.inner))
            .map_err(qis_error_to_py_err)
    }

    /// Returns the number of qubits in the density matrix.
    #[getter]
    fn num_qubits(&self) -> usize {
        self.inner.num_qubits
    }

    /// Returns the density matrix data as a 2D NumPy array.
    ///
    /// Returns:
    ///     A 2D NumPy array of complex numbers with shape (2^num_qubits, 2^num_qubits).
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<Complex64>>> {
        let dim = 1 << self.inner.num_qubits;
        let data = self.inner.data();
        // Reshape the flat data into 2D matrix
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

    /// Returns the measurement probabilities for all computational basis states.
    ///
    /// Extracts the diagonal elements of the density matrix, which represent
    /// the probabilities P(|i⟩) = ρ_ii.
    ///
    /// Returns:
    ///     A list of probabilities (floats) with length 2^num_qubits.
    fn probabilities(&self, py: Python<'_>) -> Vec<f64> {
        py.detach(|| self.inner.probabilities())
    }

    /// Computes the trace of the density matrix.
    ///
    /// For any valid physical state, the trace must equal 1.0.
    ///
    /// Returns:
    ///     The trace (sum of diagonal elements) as a real number.
    fn trace(&self) -> f64 {
        self.inner.trace().re
    }

    /// Applies a standard gate to the density matrix.
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

    /// Applies a global phase (has no observable effect on a density matrix).
    fn apply_gphase(&mut self, phi: f64) -> PyResult<()> {
        // Global phase has no effect on density matrix
        let _ = phi;
        Ok(())
    }

    /// Applies the XY gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle
    fn apply_xy(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY2P gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle
    fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy2p(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the XY2M gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle
    fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_xy2m(qubit, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a parameterized RXY rotation gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     theta: Rotation angle θ
    ///     phi: Rotation angle φ
    fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_rxy(qubit, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the SWAP gate between two qubits.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
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
    ///
    /// Args:
    ///     control: Control qubit index
    ///     target: Target qubit index
    ///     theta: Rotation angle in radians
    fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_crx(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-RY gate.
    ///
    /// Args:
    ///     control: Control qubit index
    ///     target: Target qubit index
    ///     theta: Rotation angle in radians
    fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_cry(control, target, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the controlled-RZ gate.
    ///
    /// Args:
    ///     control: Control qubit index
    ///     target: Target qubit index
    ///     theta: Rotation angle in radians
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
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     theta: Rotation angle
    fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_ryy(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZZ (Ising ZZ) gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     theta: Rotation angle
    fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzz(q0, q1, theta)
            .map_err(qis_error_to_py_err)
    }

    /// Applies the RZX gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     theta: Rotation angle
    fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> PyResult<()> {
        self.inner
            .apply_rzx(q0, q1, theta)
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

    /// Applies the Fermionic Simulation (FSIM) gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     theta: iSWAP angle
    ///     phi: Controlled-phase angle
    fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> PyResult<()> {
        self.inner
            .apply_fsim(q0, q1, theta, phi)
            .map_err(qis_error_to_py_err)
    }

    /// Applies a custom single-qubit gate.
    ///
    /// Args:
    ///     qubit: Target qubit index
    ///     matrix: 2x2 complex matrix as a NumPy 2D array
    ///
    /// Raises:
    ///     ValueError: If the matrix is not 2x2
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
            .map_err(qis_error_to_py_err)
    }

    /// Applies a custom two-qubit gate.
    ///
    /// Args:
    ///     q0: First qubit index
    ///     q1: Second qubit index
    ///     matrix: 4x4 complex matrix as a NumPy array
    ///
    /// Raises:
    ///     ValueError: If the matrix is not 4x4
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
            .map_err(qis_error_to_py_err)
    }

    /// Applies an arbitrary n-qubit unitary gate.
    ///
    /// The evolution is given by ρ → U ρ U†.
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

    /// Applies a general quantum channel specified by Kraus operators.
    ///
    /// The evolution of the density matrix is given by ρ → Σ_k K_k ρ K_k†,
    /// where Σ_k K_k† K_k = I for a trace-preserving channel.
    ///
    /// Args:
    ///     qubits: List of qubit indices the channel acts upon
    ///     ops: A list of Kraus operators, where each operator is a flattened NumPy array of Complex64
    ///
    /// Raises:
    ///     ValueError: If the Kraus operators are invalid
    ///
    /// Examples:
    ///     >>> import numpy as np
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> # Depolarizing channel with p=0.1
    ///     >>> p = 0.1
    ///     >>> K0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
    ///     >>> K1 = np.sqrt(p/3) * np.array([[0, 1], [1, 0]], dtype=complex)
    ///     >>> K2 = np.sqrt(p/3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
    ///     >>> K3 = np.sqrt(p/3) * np.array([[1, 0], [0, -1]], dtype=complex)
    ///     >>> dm = DensityMatrix(1)
    ///     >>> dm.apply_kraus([0], [K0.flatten(), K1.flatten(), K2.flatten(), K3.flatten()])
    fn apply_kraus<'py>(
        &mut self,
        py: Python<'_>,
        qubits: Vec<usize>,
        ops: &Bound<'py, PyList>,
    ) -> PyResult<()> {
        let mut kraus_ops: Vec<Vec<Complex64>> = Vec::with_capacity(ops.len());

        for op in ops.iter() {
            let data: Vec<Complex64> = if let Ok(array) = op.cast::<PyArray1<Complex64>>() {
                array.to_vec().map_err(|e| {
                    PyValueError::new_err(format!("Failed to convert array to vec: {}", e))
                })?
            } else if let Ok(list) = op.cast::<PyList>() {
                let mut data = Vec::with_capacity(list.len());
                for item in list.iter() {
                    if let Ok(py_c) = item.cast::<PyComplex>() {
                        data.push(Complex64::new(py_c.real(), py_c.imag()));
                    } else if let Ok(val) = item.extract::<f64>() {
                        data.push(Complex64::new(val, 0.0));
                    } else {
                        return Err(PyValueError::new_err(
                            "Kraus operators must contain complex numbers or floats",
                        ));
                    }
                }
                data
            } else {
                return Err(PyValueError::new_err(
                    "Kraus operators must be a list of numpy arrays or lists",
                ));
            };
            kraus_ops.push(data);
        }

        py.detach(|| self.inner.apply_kraus(&kraus_ops, &qubits))
            .map_err(qis_error_to_py_err)
    }

    /// Computes the partial trace over a set of qubits.
    ///
    /// Reduces the N-qubit system to a smaller subsystem containing only the specified qubits
    /// by tracing out all other qubits.
    ///
    /// Args:
    ///     keep: List of qubit indices to keep in the resulting reduced density matrix
    ///
    /// Returns:
    ///     A new DensityMatrix representing the subsystem
    ///
    /// Raises:
    ///     ValueError: If any qubit index is out of bounds
    fn partial_trace(&self, py: Python<'_>, keep: Vec<usize>) -> PyResult<Self> {
        let inner = py
            .detach(|| self.inner.partial_trace(&keep))
            .map_err(qis_error_to_py_err)?;
        Ok(Self { inner })
    }

    /// Computes the expectation value of an observable.
    ///
    /// Calculates Tr(ρ * O) for the current density matrix ρ and a given observable O.
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

    /// Returns a string representation of the density matrix.
    fn __repr__(&self) -> String {
        let dim = 1 << self.inner.num_qubits;
        format!(
            "DensityMatrix(num_qubits={}, shape=({}, {}))",
            self.inner.num_qubits, dim, dim
        )
    }

    /// Returns a copy of this density matrix.
    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Checks if the density matrix is Hermitian (self-adjoint) within a tolerance.
    ///
    /// A valid density matrix must satisfy ρ = ρ†, i.e., ρ_ij = ρ_ji*.
    ///
    /// Args:
    ///     tol: Tolerance for floating-point comparison (default: 1e-10)
    ///
    /// Returns:
    ///     True if the matrix is Hermitian within the specified tolerance.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> dm = DensityMatrix(1)
    ///     >>> dm.apply_h(0)
    ///     >>> dm.is_hermitian()
    ///     True
    #[pyo3(signature = (tol=1e-10))]
    fn is_hermitian(&self, tol: f64) -> bool {
        self.inner.is_hermitian(tol)
    }

    /// Checks if the density matrix is positive semidefinite.
    ///
    /// Computes eigenvalues via a self-adjoint eigenvalue decomposition and
    /// checks that every eigenvalue satisfies λᵢ ≥ -tol. For Hermitian matrices
    /// this check is definitive; non-Hermitian inputs (or matrices containing
    /// NaN/Inf) return `false`.
    ///
    /// Args:
    ///     tol: Tolerance for floating-point comparison (default: 1e-10)
    ///
    /// Returns:
    ///     True if the matrix satisfies the positive semidefinite condition.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> dm = DensityMatrix(1)
    ///     >>> dm.is_positive_semidefinite()
    ///     True
    #[pyo3(signature = (tol=1e-10))]
    fn is_positive_semidefinite(&self, tol: f64) -> bool {
        self.inner.is_positive_semidefinite_approx(tol)
    }

    /// Validates all physical constraints of the density matrix.
    ///
    /// Checks:
    /// 1. Hermiticity: ρ = ρ†
    /// 2. Positive semidefiniteness: All eigenvalues >= 0
    /// 3. Unit trace: Tr(ρ) = 1
    ///
    /// Args:
    ///     tol: Tolerance for floating-point comparisons (default: 1e-10)
    ///
    /// Raises:
    ///     ValueError: If any physical constraint is violated.
    ///
    /// Examples:
    ///     >>> from cqlib.qis import DensityMatrix
    ///     >>> dm = DensityMatrix(1)
    ///     >>> dm.apply_h(0)
    ///     >>> dm.validate_physical()  # Should pass for valid states
    #[pyo3(signature = (tol=1e-10))]
    fn validate_physical(&self, py: Python<'_>, tol: f64) -> PyResult<()> {
        py.detach(|| self.inner.validate_physical(tol))
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
}
