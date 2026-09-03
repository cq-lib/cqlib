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

//! Python bindings for quantum noise models.
//!
//! This module provides types for modeling quantum noise in simulations and
//! error-aware compilation. It includes representations for:
//!
//! - **Single-qubit noise channels**: Bit-flip, phase-flip, depolarizing, etc.
//! - **Two-qubit noise channels**: Depolarizing, correlated Pauli errors
//! - **Readout errors**: Asymmetric measurement errors
//! - **Noise models**: Complete device noise characterization
//!
//! # Noise Channels
//!
//! Each noise channel can be converted to its Kraus operator representation
//! via `to_kraus()`, which returns a list of NumPy arrays suitable for
//! density matrix simulations.
//!
//! # Example
//!
//! ```python
//! from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise, ReadoutError
//! from cqlib.circuit import StandardGate
//!
//! # Create a noise model
//! model = NoiseModel()
//!
//! # Add single-qubit depolarizing noise to H gates on qubit 0
//! noise = SingleQubitNoise.depolarizing(p=0.001)
//! model.add_single_qubit_error(StandardGate.H, 0, noise)
//!
//! # Add two-qubit depolarizing noise to CX gates
//! cx_noise = TwoQubitNoise.depolarizing(p=0.005)
//! model.add_two_qubit_error(StandardGate.CX, 0, 1, cx_noise)
//!
//! # Add readout error
//! readout = ReadoutError(p_0_given_1=0.02, p_1_given_0=0.01)
//! model.add_readout_error(0, readout)
//! ```

use crate::circuit::PyStandardGate;
use crate::device::qubit::PyPhysicalQubitLike;
use crate::device::validate_probability;
use crate::qis::pauli::PyPauli;
use crate::utils::hash_value;
use cqlib_core::circuit::{Parameter, Qubit};
use cqlib_core::device::{NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise};
use num_complex::Complex64;
use numpy::{PyArray2, ToPyArray};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Single-qubit quantum noise channel.
///
/// Represents various types of single-qubit noise that can occur in quantum
/// systems, including bit-flip, phase-flip, depolarizing, and amplitude/phase
/// damping channels.
///
/// # Noise Types
///
/// - **Bit-flip**: Flips |0⟩ ↔ |1⟩ with probability p
/// - **Phase-flip**: Applies Z with probability p
/// - **Pauli**: General Pauli noise with independent X, Y, Z probabilities
/// - **Depolarizing**: Uniform mixture of all Pauli errors
/// - **Amplitude damping**: Energy relaxation (T1) process
/// - **Phase damping**: Pure dephasing (T2) process
///
/// # Example
///
/// ```python
/// from cqlib.device import SingleQubitNoise
/// import numpy as np
///
/// # Create depolarizing noise with 0.1% error probability
/// noise = SingleQubitNoise.depolarizing(p=0.001)
///
/// # Get Kraus operators for simulation
/// kraus_ops = noise.to_kraus()  # List of 2x2 NumPy arrays
///
/// # Validate noise parameters
/// assert noise.is_valid()  # True if probabilities are in [0, 1]
/// ```
#[pyclass(name = "SingleQubitNoise", module = "cqlib.device", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PySingleQubitNoise {
    pub(crate) inner: SingleQubitNoise,
}

impl From<SingleQubitNoise> for PySingleQubitNoise {
    fn from(inner: SingleQubitNoise) -> Self {
        Self { inner }
    }
}

impl From<PySingleQubitNoise> for SingleQubitNoise {
    fn from(value: PySingleQubitNoise) -> Self {
        value.inner
    }
}

#[pymethods]
impl PySingleQubitNoise {
    /// Creates a bit-flip noise channel.
    ///
    /// Kraus operators: E₀ = √(1-p) I, E₁ = √p X
    ///
    /// # Arguments
    ///
    /// * `p` - Bit-flip probability in range [0.0, 1.0]
    #[staticmethod]
    fn bit_flip(p: f64) -> PyResult<Self> {
        validate_probability(p, "p")?;
        Ok(Self {
            inner: SingleQubitNoise::BitFlip(p),
        })
    }

    /// Creates a phase-flip noise channel.
    ///
    /// Kraus operators: E₀ = √(1-p) I, E₁ = √p Z
    ///
    /// # Arguments
    ///
    /// * `p` - Phase-flip probability in range [0.0, 1.0]
    #[staticmethod]
    fn phase_flip(p: f64) -> PyResult<Self> {
        validate_probability(p, "p")?;
        Ok(Self {
            inner: SingleQubitNoise::PhaseFlip(p),
        })
    }

    /// Creates a general Pauli noise channel.
    ///
    /// Kraus operators include √(1-px-py-pz) I, √px X, √py Y, √pz Z.
    ///
    /// # Arguments
    ///
    /// * `px` - Probability of X error
    /// * `py` - Probability of Y error
    /// * `pz` - Probability of Z error
    ///
    /// # Constraints
    ///
    /// Must satisfy px + py + pz ≤ 1.0
    #[staticmethod]
    fn pauli(px: f64, py: f64, pz: f64) -> PyResult<Self> {
        validate_probability(px, "px")?;
        validate_probability(py, "py")?;
        validate_probability(pz, "pz")?;
        if px + py + pz > 1.0 {
            return Err(PyValueError::new_err(format!(
                "px + py + pz must not exceed 1, got {}",
                px + py + pz
            )));
        }
        Ok(Self {
            inner: SingleQubitNoise::Pauli { px, py, pz },
        })
    }

    /// Creates a depolarizing noise channel.
    ///
    /// With probability p, applies a random Pauli error (X, Y, or Z).
    /// Each Pauli occurs with probability p/3.
    ///
    /// # Arguments
    ///
    /// * `p` - Total depolarization probability in range [0.0, 1.0]
    #[staticmethod]
    fn depolarizing(p: f64) -> PyResult<Self> {
        validate_probability(p, "p")?;
        Ok(Self {
            inner: SingleQubitNoise::Depolarizing(p),
        })
    }

    /// Creates an amplitude damping channel.
    ///
    /// Models energy relaxation (T1) where excited states decay to ground state.
    ///
    /// # Arguments
    ///
    /// * `gamma` - Damping parameter in range [0.0, 1.0]
    ///
    /// # Physical Interpretation
    ///
    /// After time t, γ = 1 - exp(-t/T1). For small t/T1, γ ≈ t/T1.
    #[staticmethod]
    fn amplitude_damping(gamma: f64) -> PyResult<Self> {
        validate_probability(gamma, "gamma")?;
        Ok(Self {
            inner: SingleQubitNoise::AmplitudeDamping(gamma),
        })
    }

    /// Creates a phase damping channel.
    ///
    /// Models pure dephasing (T2) without energy relaxation.
    ///
    /// # Arguments
    ///
    /// * `lambda_` - Phase damping parameter in range [0.0, 1.0]
    ///
    /// # Physical Interpretation
    ///
    /// After time t, λ = 1 - exp(-t/T2). For small t/T2, λ ≈ t/T2.
    #[staticmethod]
    fn phase_damping(lambda_: f64) -> PyResult<Self> {
        validate_probability(lambda_, "lambda_")?;
        Ok(Self {
            inner: SingleQubitNoise::PhaseDamping(lambda_),
        })
    }

    /// Validates that noise parameters are physically valid.
    ///
    /// Returns `True` if all probabilities are in valid ranges:
    /// - Individual probabilities in [0.0, 1.0]
    /// - For Pauli noise: px + py + pz ≤ 1.0
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Returns the Kraus operators as NumPy arrays.
    ///
    /// # Returns
    ///
    /// List of 2x2 complex NumPy arrays representing the Kraus operators.
    #[allow(clippy::wrong_self_convention)]
    fn to_kraus<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyArray2<Complex64>>> {
        self.inner
            .to_kraus()
            .into_iter()
            .map(|k| k.to_pyarray(py))
            .collect()
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __repr__(&self) -> String {
        match self.inner {
            SingleQubitNoise::BitFlip(p) => format!("SingleQubitNoise.bit_flip({})", p),
            SingleQubitNoise::PhaseFlip(p) => format!("SingleQubitNoise.phase_flip({})", p),
            SingleQubitNoise::Pauli { px, py, pz } => {
                format!("SingleQubitNoise.pauli({}, {}, {})", px, py, pz)
            }
            SingleQubitNoise::Depolarizing(p) => format!("SingleQubitNoise.depolarizing({})", p),
            SingleQubitNoise::AmplitudeDamping(gamma) => {
                format!("SingleQubitNoise.amplitude_damping({})", gamma)
            }
            SingleQubitNoise::PhaseDamping(lambda_) => {
                format!("SingleQubitNoise.phase_damping({})", lambda_)
            }
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PySingleQubitNoise>() {
            return Ok(false);
        }
        let other = other.extract::<PySingleQubitNoise>()?;
        Ok(self.inner == other.inner)
    }
}

/// Two-qubit quantum noise channel.
///
/// Represents noise affecting pairs of qubits, including depolarizing noise
/// and correlated Pauli errors.
///
/// # Noise Types
///
/// - **Depolarizing**: Uniform mixture of all 15 non-identity Pauli operators
/// - **Independent**: Tensor product of single-qubit noise channels
/// - **Correlated Pauli**: Correlated error on both qubits (e.g., XX, ZZ)
///
/// # Example
///
/// ```python
/// from cqlib.device import SingleQubitNoise, TwoQubitNoise
///
/// # Depolarizing noise with 1% total error probability
/// noise = TwoQubitNoise.depolarizing(p=0.01)
///
/// # Independent noise on each qubit
/// q0_noise = SingleQubitNoise.depolarizing(0.001)
/// q1_noise = SingleQubitNoise.depolarizing(0.001)
/// independent = TwoQubitNoise.independent(q0_noise, q1_noise)
/// ```
#[pyclass(name = "TwoQubitNoise", module = "cqlib.device", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyTwoQubitNoise {
    pub(crate) inner: TwoQubitNoise,
}

impl From<TwoQubitNoise> for PyTwoQubitNoise {
    fn from(inner: TwoQubitNoise) -> Self {
        Self { inner }
    }
}

impl From<PyTwoQubitNoise> for TwoQubitNoise {
    fn from(value: PyTwoQubitNoise) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyTwoQubitNoise {
    /// Creates a two-qubit depolarizing noise channel.
    ///
    /// With probability p, applies a random Pauli error from the 15 non-identity
    /// Pauli operators (IX, IT, IZ, XI, XX, XY, ..., ZZ).
    ///
    /// # Arguments
    ///
    /// * `p` - Total depolarization probability in range [0.0, 1.0]
    #[staticmethod]
    fn depolarizing(p: f64) -> PyResult<Self> {
        validate_probability(p, "p")?;
        Ok(Self {
            inner: TwoQubitNoise::Depolarizing(p),
        })
    }

    /// Creates independent single-qubit noise on both qubits.
    ///
    /// The resulting channel is E = E₀ ⊗ E₁ where E₀ and E₁ are the
    /// single-qubit noise channels.
    ///
    /// # Arguments
    ///
    /// * `q0_noise` - Noise channel for the first qubit
    /// * `q1_noise` - Noise channel for the second qubit
    #[staticmethod]
    fn independent(q0_noise: PySingleQubitNoise, q1_noise: PySingleQubitNoise) -> Self {
        Self {
            inner: TwoQubitNoise::Independent {
                q0_noise: q0_noise.inner,
                q1_noise: q1_noise.inner,
            },
        }
    }

    /// Creates correlated Pauli noise.
    ///
    /// # Arguments
    ///
    /// * `op_q0` - Pauli operator for first qubit
    /// * `op_q1` - Pauli operator for second qubit
    /// * `p` - Correlation probability in range [0.0, 1.0]
    #[staticmethod]
    #[pyo3(signature = (op_q0, op_q1, p))]
    fn correlated_pauli(op_q0: PyPauli, op_q1: PyPauli, p: f64) -> PyResult<Self> {
        validate_probability(p, "p")?;
        Ok(Self {
            inner: TwoQubitNoise::CorrelatedPauli {
                op_q0: op_q0.inner,
                op_q1: op_q1.inner,
                p,
            },
        })
    }

    /// Validates that noise parameters are physically valid.
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Returns the Kraus operators as NumPy arrays.
    ///
    /// # Returns
    ///
    /// List of 4x4 complex NumPy arrays representing the Kraus operators.
    #[allow(clippy::wrong_self_convention)]
    fn to_kraus<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyArray2<Complex64>>> {
        self.inner
            .to_kraus()
            .into_iter()
            .map(|k| k.to_pyarray(py))
            .collect()
    }

    /// Returns the noise channel type.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            TwoQubitNoise::Depolarizing(_) => "depolarizing",
            TwoQubitNoise::Independent { .. } => "independent",
            TwoQubitNoise::CorrelatedPauli { .. } => "correlated_pauli",
        }
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __repr__(&self) -> String {
        match self.inner {
            TwoQubitNoise::Depolarizing(p) => format!("TwoQubitNoise.depolarizing({})", p),
            TwoQubitNoise::Independent { q0_noise, q1_noise } => format!(
                "TwoQubitNoise.independent({}, {})",
                PySingleQubitNoise::from(q0_noise).__repr__(),
                PySingleQubitNoise::from(q1_noise).__repr__()
            ),
            TwoQubitNoise::CorrelatedPauli { op_q0, op_q1, p } => format!(
                "TwoQubitNoise.correlated_pauli('{}', '{}', {})",
                op_q0, op_q1, p
            ),
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyTwoQubitNoise>() {
            return Ok(false);
        }
        let other = other.extract::<PyTwoQubitNoise>()?;
        Ok(self.inner == other.inner)
    }
}

/// Asymmetric readout error model.
///
/// Represents measurement errors where the probabilities of false 0 and false 1
/// may differ. This is common in superconducting qubits where |1⟩ has higher
/// readout error than |0⟩.
///
/// # State Discrimination
///
/// - `p_0_given_1`: Probability of measuring 0 when state was |1⟩ (false negative)
/// - `p_1_given_0`: Probability of measuring 1 when state was |0⟩ (false positive)
///
/// # Example
///
/// ```python
/// from cqlib.device import ReadoutError
///
/// # Typical superconducting qubit readout errors
/// error = ReadoutError(
///     p_0_given_1=0.02,  # 2% false negative
///     p_1_given_0=0.01   # 1% false positive
/// )
///
/// assert error.is_valid()  # Both probabilities in [0, 1]
/// ```
#[pyclass(name = "ReadoutError", module = "cqlib.device", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyReadoutError {
    pub(crate) inner: ReadoutError,
}

impl From<ReadoutError> for PyReadoutError {
    fn from(inner: ReadoutError) -> Self {
        Self { inner }
    }
}

impl From<PyReadoutError> for ReadoutError {
    fn from(value: PyReadoutError) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyReadoutError {
    /// Creates a new readout error model.
    ///
    /// # Arguments
    ///
    /// * `p_0_given_1` - Probability of measuring 0 given state was prepared in 1
    /// * `p_1_given_0` - Probability of measuring 1 given state was prepared in 0
    ///
    /// # Constraints
    ///
    /// Both probabilities must be in range [0.0, 1.0].
    #[new]
    fn new(p_0_given_1: f64, p_1_given_0: f64) -> PyResult<Self> {
        validate_probability(p_0_given_1, "p_0_given_1")?;
        validate_probability(p_1_given_0, "p_1_given_0")?;
        Ok(Self {
            inner: ReadoutError {
                p_0_given_1,
                p_1_given_0,
            },
        })
    }

    /// Returns P(meas 0 | prep 1), the false-negative probability.
    #[getter]
    fn p_0_given_1(&self) -> f64 {
        self.inner.p_0_given_1
    }

    /// Returns P(meas 1 | prep 0), the false-positive probability.
    #[getter]
    fn p_1_given_0(&self) -> f64 {
        self.inner.p_1_given_0
    }

    /// Validates that error probabilities are valid.
    ///
    /// Returns `True` if both probabilities are in [0.0, 1.0].
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __copy__(&self) -> Self {
        *self
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        *self
    }

    fn __repr__(&self) -> String {
        format!(
            "ReadoutError(p_0_given_1={}, p_1_given_0={})",
            self.p_0_given_1(),
            self.p_1_given_0()
        )
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyReadoutError>() {
            return Ok(false);
        }
        let other = other.extract::<PyReadoutError>()?;
        Ok(self.inner == other.inner)
    }
}

/// Key for looking up noise parameters in a noise model.
///
/// Uniquely identifies a gate operation on specific qubits for noise lookup.
/// The key includes the gate type and qubit indices but not gate parameters.
///
/// # Example
///
/// ```python
/// from cqlib.device import OperationKey
/// from cqlib.circuit import StandardGate
///
/// # Key for H gate on qubit 0
/// key = OperationKey.new_single(StandardGate.H, 0)
///
/// # Key for CX gate on qubits 0 (control) and 1 (target)
/// key = OperationKey.new_double(StandardGate.CX, 0, 1)
///
/// # Get gate and qubits
/// print(key.gate)    # StandardGate.H
/// print(key.qubits)  # [0]
/// ```
#[pyclass(name = "OperationKey", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyOperationKey {
    pub(crate) inner: OperationKey,
}

impl From<OperationKey> for PyOperationKey {
    fn from(inner: OperationKey) -> Self {
        Self { inner }
    }
}

impl From<PyOperationKey> for OperationKey {
    fn from(value: PyOperationKey) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyOperationKey {
    /// Creates a key for a single-qubit operation.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate
    /// * `q0` - The target qubit
    ///
    #[staticmethod]
    fn new_single(gate: PyStandardGate, q0: PyPhysicalQubitLike) -> Self {
        Self {
            inner: OperationKey::new_single(gate.inner, Qubit::from(q0)),
        }
    }

    /// Creates a key for a two-qubit operation.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate
    /// * `q0` - First qubit (typically control)
    /// * `q1` - Second qubit (typically target)
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if q0 and q1 are the same qubit.
    #[staticmethod]
    fn new_double(
        gate: PyStandardGate,
        q0: PyPhysicalQubitLike,
        q1: PyPhysicalQubitLike,
    ) -> PyResult<Self> {
        let q0 = Qubit::from(q0);
        let q1 = Qubit::from(q1);
        let inner = OperationKey::new_double(gate.inner, q0, q1)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a key for a three-qubit operation.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate
    /// * `q0` - First qubit
    /// * `q1` - Second qubit
    /// * `q2` - Third qubit
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if any qubits are duplicated.
    #[staticmethod]
    fn new_triple(
        gate: PyStandardGate,
        q0: PyPhysicalQubitLike,
        q1: PyPhysicalQubitLike,
        q2: PyPhysicalQubitLike,
    ) -> PyResult<Self> {
        let inner = OperationKey::new_triple(
            gate.inner,
            Qubit::from(q0),
            Qubit::from(q1),
            Qubit::from(q2),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the qubit indices involved in this operation.
    #[getter]
    fn qubits(&self) -> Vec<usize> {
        self.inner.qubits().to_vec()
    }

    /// Returns the gate type.
    ///
    /// Note: For parametric gates (e.g., RX, U), the returned gate has
    /// zero parameters since OperationKey only stores the gate type.
    #[getter]
    fn gate(&self) -> PyStandardGate {
        let gate = *self.inner.gate();
        // Fill with zeros for parametric gates (e.g., RX, RY, RZ, U)
        // This is necessary because OperationKey only stores gate type, not parameters.
        let num_params = gate.num_params();
        let params = if num_params > 0 {
            vec![Parameter::from(0.0); num_params]
        } else {
            Vec::new()
        };
        PyStandardGate::from(gate, params)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if !other.is_instance_of::<PyOperationKey>() {
            return Ok(false);
        }
        let other = other.extract::<PyOperationKey>()?;
        Ok(self.inner == other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationKey(gate={}, qubits={:?})",
            self.inner.gate(),
            self.qubits()
        )
    }
}

/// Complete noise model for a quantum device.
///
/// Aggregates all noise sources: readout errors, single-qubit gate errors,
/// and two-qubit gate errors. Used by noise-aware compilers and simulators.
///
/// # Example
///
/// ```python
/// from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise
/// from cqlib.circuit import StandardGate
///
/// model = NoiseModel()
///
/// # Add noise to all H gates on qubit 0
/// model.add_single_qubit_error(
///     StandardGate.H, 0,
///     SingleQubitNoise.depolarizing(0.001)
/// )
///
/// # Add noise to CX gates between qubits 0 and 1
/// model.add_two_qubit_error(
///     StandardGate.CX, 0, 1,
///     TwoQubitNoise.depolarizing(0.01)
/// )
///
/// # Retrieve errors
/// key = OperationKey.new_single(StandardGate.H, 0)
/// errors = model.get_single_qubit_errors(key)
/// ```
#[pyclass(name = "NoiseModel", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug, Default)]
pub struct PyNoiseModel {
    pub(crate) inner: NoiseModel,
}

impl From<NoiseModel> for PyNoiseModel {
    fn from(inner: NoiseModel) -> Self {
        Self { inner }
    }
}

impl From<PyNoiseModel> for NoiseModel {
    fn from(value: PyNoiseModel) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyNoiseModel {
    /// Creates an empty noise model.
    #[new]
    fn new() -> Self {
        Self {
            inner: NoiseModel::new(),
        }
    }

    /// Adds a readout error for a specific qubit.
    ///
    /// # Arguments
    ///
    /// * `qubit` - The qubit index
    /// * `error` - The readout error model
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the error probabilities are invalid.
    fn add_readout_error(
        &mut self,
        qubit: PyPhysicalQubitLike,
        error: PyReadoutError,
    ) -> PyResult<()> {
        self.inner
            .add_readout_error(Qubit::from(qubit), error.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Adds single-qubit noise to a gate on a specific qubit.
    ///
    /// Multiple noise channels can be added to the same gate/qubit pair.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate
    /// * `qubit` - The target qubit
    /// * `noise` - The noise channel
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the noise parameters are invalid.
    fn add_single_qubit_error(
        &mut self,
        gate: PyStandardGate,
        qubit: PyPhysicalQubitLike,
        noise: PySingleQubitNoise,
    ) -> PyResult<()> {
        self.inner
            .add_single_qubit_error(gate.inner, Qubit::from(qubit), noise.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Adds two-qubit noise to a gate on specific qubits.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate
    /// * `q0` - First qubit (typically control)
    /// * `q1` - Second qubit (typically target)
    /// * `noise` - The noise channel
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if the noise parameters are invalid or if q0 == q1.
    fn add_two_qubit_error(
        &mut self,
        gate: PyStandardGate,
        q0: PyPhysicalQubitLike,
        q1: PyPhysicalQubitLike,
        noise: PyTwoQubitNoise,
    ) -> PyResult<()> {
        self.inner
            .add_two_qubit_error(gate.inner, Qubit::from(q0), Qubit::from(q1), noise.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns the readout error for a qubit, if any.
    fn get_readout_error(&self, qubit: PyPhysicalQubitLike) -> Option<PyReadoutError> {
        self.inner
            .get_readout_error(&Qubit::from(qubit))
            .copied()
            .map(PyReadoutError::from)
    }

    /// Returns all single-qubit noise channels for an operation.
    ///
    /// Returns a list of noise channels (typically just one, but multiple
    /// can be added to the same operation).
    fn get_single_qubit_errors(&self, key: PyOperationKey) -> Option<Vec<PySingleQubitNoise>> {
        self.inner
            .get_single_qubit_errors(&key.inner)
            .cloned()
            .map(|v| v.into_iter().map(PySingleQubitNoise::from).collect())
    }

    /// Returns all two-qubit noise channels for an operation.
    fn get_two_qubit_errors(&self, key: PyOperationKey) -> Option<Vec<PyTwoQubitNoise>> {
        self.inner
            .get_two_qubit_errors(&key.inner)
            .cloned()
            .map(|v| v.into_iter().map(PyTwoQubitNoise::from).collect())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        "NoiseModel()".to_string()
    }
}
