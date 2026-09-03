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

//! Python bindings for quantum device hardware characteristics.
//!
//! This module provides Python wrappers around the core device types:
//!
//! - [`PyInstructionProp`]: Calibration data for quantum gates (error rates, duration)
//! - [`PyQubitProp`]: Physical properties of individual qubits (T1, T2, readout errors)
//! - [`PyEdgeProp`]: Properties of coupling edges between qubits
//! - [`PyDevice`]: Complete hardware description including topology and calibration data
//!
//! # Example
//!
//! ```python
//! from cqlib.device import Device, Topology, QubitProp
//! from datetime import datetime, timezone
//!
//! # Create device topology
//! topology = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])
//!
//! # Initialize device
//! device = Device("superconducting_qpu", [0, 1, 2], topology)
//!
//! # Set calibration timestamp
//! device.set_calibration_time(datetime.now(timezone.utc))
//!
//! # Set default coherence times
//! device.default_t1 = 100.0
//! device.default_t2 = 50.0
//!
//! # Add qubit-specific properties
//! prop = QubitProp(readout_error=0.01)
//! prop.t1 = 120.0
//! device.add_qubit_properties(0, prop)
//! ```

use crate::circuit::{PyCircuit, PyInstruction, PyValueOperation};
use crate::device::qubit::{PyPhysicalQubit, PyPhysicalQubitLike, PyPhysicalQubitList};
use crate::device::topology::PyTopology;
use crate::device::{validate_nonnegative, validate_positive, validate_probability};
use chrono::{DateTime, TimeZone, Utc};
use cqlib_core::device::{Device, EdgeProp, InstructionProp, PhysicalQubit, QubitProp};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use time::OffsetDateTime;

/// Python wrapper for [`InstructionProp`].
///
/// Represents calibration data for a quantum gate executed on specific qubits,
/// including the gate's error rate (infidelity) and optionally its execution duration.
///
/// # Python Example
///
/// ```python
/// from cqlib.circuit import Instruction, StandardGate
/// from cqlib.device import InstructionProp
///
/// # Create properties for an H gate with 0.1% error rate
/// h = Instruction.from_standard_gate(StandardGate.H)
/// prop = InstructionProp(h, error_rate=0.001)
///
/// # Optionally set gate duration in nanoseconds
/// prop.length = 35.0  # 35 ns
/// ```
#[pyclass(name = "InstructionProp", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyInstructionProp {
    pub(crate) inner: InstructionProp,
}

impl From<InstructionProp> for PyInstructionProp {
    fn from(inner: InstructionProp) -> Self {
        Self { inner }
    }
}

impl From<PyInstructionProp> for InstructionProp {
    fn from(value: PyInstructionProp) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyInstructionProp {
    #[new]
    fn new(instruction: PyInstruction, error_rate: f64) -> PyResult<Self> {
        validate_probability(error_rate, "error_rate")?;
        Ok(Self {
            inner: InstructionProp::new(instruction.inner, error_rate),
        })
    }

    #[setter]
    fn set_length(&mut self, length: f64) -> PyResult<()> {
        validate_nonnegative(length, "length")?;
        self.inner.set_length(length);
        Ok(())
    }

    #[setter]
    fn set_instruction(&mut self, instruction: PyInstruction) {
        self.inner.set_instruction(instruction.inner);
    }

    #[setter]
    fn set_error_rate(&mut self, error_rate: f64) -> PyResult<()> {
        validate_probability(error_rate, "error_rate")?;
        self.inner.set_error_rate(error_rate);
        Ok(())
    }

    #[getter]
    fn instruction(&self) -> PyInstruction {
        PyInstruction::from(self.inner.instruction().clone())
    }

    #[getter]
    fn error_rate(&self) -> f64 {
        self.inner.error_rate()
    }

    #[getter]
    fn length(&self) -> Option<f64> {
        self.inner.length()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        let instruction_name = format!("{}", self.inner.instruction());
        match self.length() {
            Some(length) => format!(
                "InstructionProp(instruction={}, error_rate={}, length={})",
                instruction_name,
                self.error_rate(),
                length
            ),
            None => format!(
                "InstructionProp(instruction={}, error_rate={})",
                instruction_name,
                self.error_rate()
            ),
        }
    }
}

/// Python wrapper for [`QubitProp`].
///
/// Represents the physical properties and operational characteristics of a single
/// quantum qubit, including coherence times (T1, T2), readout errors, and supported
/// native gates.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import QubitProp
///
/// # Create qubit properties with 1% readout error
/// prop = QubitProp(readout_error=0.01)
///
/// # Set coherence times (in microseconds)
/// prop.t1 = 50.0
/// prop.t2 = 30.0
///
/// # Set measurement discrimination errors
/// prop.prob_meas0_prep1 = 0.02  # P(meas 0 | prep 1)
/// prop.prob_meas1_prep0 = 0.01  # P(meas 1 | prep 0)
/// ```
#[pyclass(name = "QubitProp", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyQubitProp {
    pub(crate) inner: QubitProp,
}

impl From<QubitProp> for PyQubitProp {
    fn from(inner: QubitProp) -> Self {
        Self { inner }
    }
}

impl From<PyQubitProp> for QubitProp {
    fn from(value: PyQubitProp) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyQubitProp {
    #[new]
    fn new(readout_error: f64) -> PyResult<Self> {
        validate_probability(readout_error, "readout_error")?;
        Ok(Self {
            inner: QubitProp::new(readout_error),
        })
    }

    #[setter]
    fn set_prob_meas0_prep1(&mut self, prob: f64) -> PyResult<()> {
        validate_probability(prob, "prob_meas0_prep1")?;
        self.inner.set_prob_meas0_prep1(prob);
        Ok(())
    }

    #[setter]
    fn set_prob_meas1_prep0(&mut self, prob: f64) -> PyResult<()> {
        validate_probability(prob, "prob_meas1_prep0")?;
        self.inner.set_prob_meas1_prep0(prob);
        Ok(())
    }

    #[setter]
    fn set_t1(&mut self, t1: f64) -> PyResult<()> {
        validate_positive(t1, "t1")?;
        self.inner.set_t1(t1);
        Ok(())
    }

    #[setter]
    fn set_t2(&mut self, t2: f64) -> PyResult<()> {
        validate_positive(t2, "t2")?;
        self.inner.set_t2(t2);
        Ok(())
    }

    #[setter]
    fn set_frequency(&mut self, frequency: f64) -> PyResult<()> {
        validate_positive(frequency, "frequency")?;
        self.inner.set_frequency(frequency);
        Ok(())
    }

    /// Adds a native instruction property to this qubit.
    ///
    /// Appends to the existing list of native instructions.
    fn add_native_instruction(&mut self, prop: PyInstructionProp) -> PyResult<()> {
        self.inner
            .set_native_instruction(prop.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn readout_error(&self) -> f64 {
        self.inner.readout_error()
    }

    #[getter]
    fn prob_meas0_prep1(&self) -> Option<f64> {
        self.inner.prob_meas0_prep1()
    }

    #[getter]
    fn prob_meas1_prep0(&self) -> Option<f64> {
        self.inner.prob_meas1_prep0()
    }

    #[getter]
    fn t1(&self) -> Option<f64> {
        self.inner.t1()
    }

    #[getter]
    fn t2(&self) -> Option<f64> {
        self.inner.t2()
    }

    #[getter]
    fn frequency(&self) -> Option<f64> {
        self.inner.frequency()
    }

    #[getter]
    fn native_instructions(&self) -> Vec<PyInstructionProp> {
        self.inner
            .native_instructions()
            .iter()
            .cloned()
            .map(PyInstructionProp::from)
            .collect()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "QubitProp(readout_error={}, t1={:?}, t2={:?}, frequency={:?}, native_instructions={})",
            self.readout_error(),
            self.t1(),
            self.t2(),
            self.frequency(),
            self.native_instructions().len()
        )
    }
}

#[pyclass(name = "EdgeProp", module = "cqlib.device", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyEdgeProp {
    pub(crate) inner: EdgeProp,
}

impl From<EdgeProp> for PyEdgeProp {
    fn from(inner: EdgeProp) -> Self {
        Self { inner }
    }
}

impl From<PyEdgeProp> for EdgeProp {
    fn from(value: PyEdgeProp) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyEdgeProp {
    #[new]
    fn new() -> Self {
        Self {
            inner: EdgeProp::new(),
        }
    }

    /// Adds a native instruction property to this edge.
    ///
    /// Appends to the existing list of native instructions.
    fn add_native_instruction(&mut self, prop: PyInstructionProp) -> PyResult<()> {
        self.inner
            .set_native_instruction(prop.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn native_instructions(&self) -> Vec<PyInstructionProp> {
        self.inner
            .native_instructions()
            .iter()
            .cloned()
            .map(PyInstructionProp::from)
            .collect()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EdgeProp(native_instructions={})",
            self.native_instructions().len()
        )
    }
}

/// Python wrapper for [`Device`].
///
/// Represents a complete quantum device characterization, including topology,
/// qubit properties, coupling edge properties, and default calibration values.
///
/// This is the primary interface for describing quantum hardware to the compiler
/// and simulator.
///
/// # Python Example
///
/// ```python
/// from cqlib.device import Device, Topology, QubitProp
/// from datetime import datetime, timezone
///
/// # Create device topology
/// topology = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])
///
/// # Initialize device
/// device = Device("superconducting_qpu", [0, 1, 2], topology)
///
/// # Set calibration timestamp
/// device.set_calibration_time(datetime.now(timezone.utc))
///
/// # Set default coherence times
/// device.default_t1 = 100.0
/// device.default_t2 = 50.0
///
/// # Add qubit-specific properties
/// prop = QubitProp(readout_error=0.01)
/// prop.t1 = 120.0
/// device.add_qubit_properties(0, prop)
/// ```
#[pyclass(name = "Device", module = "cqlib.device", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyDevice {
    pub(crate) inner: Device,
}

impl From<Device> for PyDevice {
    fn from(inner: Device) -> Self {
        Self { inner }
    }
}

impl From<PyDevice> for Device {
    fn from(value: PyDevice) -> Self {
        value.inner
    }
}

#[pymethods]
impl PyDevice {
    #[new]
    fn new(name: String, qubits: PyPhysicalQubitList, topology: PyTopology) -> PyResult<Self> {
        let qubits = Vec::<PhysicalQubit>::from(qubits).into_iter().collect();
        let inner = Device::new(name, qubits, topology.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    #[setter]
    pub fn set_native_gates(&mut self, gates: Vec<PyInstruction>) -> PyResult<()> {
        let gates = gates.into_iter().map(|g| g.inner).collect();
        self.inner
            .set_native_gates(gates)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[setter]
    fn set_default_t1(&mut self, t1: f64) -> PyResult<()> {
        validate_positive(t1, "default_t1")?;
        self.inner.set_default_t1(t1);
        Ok(())
    }

    #[setter]
    fn set_default_t2(&mut self, t2: f64) -> PyResult<()> {
        validate_positive(t2, "default_t2")?;
        self.inner.set_default_t2(t2);
        Ok(())
    }

    #[setter]
    fn set_default_readout_error(&mut self, error: f64) -> PyResult<()> {
        validate_probability(error, "default_readout_error")?;
        self.inner.set_default_readout_error(error);
        Ok(())
    }

    #[setter]
    fn set_default_single_qubit_error(&mut self, error: f64) -> PyResult<()> {
        validate_probability(error, "default_single_qubit_error")?;
        self.inner.set_default_single_qubit_error(error);
        Ok(())
    }

    #[setter]
    fn set_default_two_qubit_error(&mut self, error: f64) -> PyResult<()> {
        validate_probability(error, "default_two_qubit_error")?;
        self.inner.set_default_two_qubit_error(error);
        Ok(())
    }

    /// Sets the calibration timestamp with nanosecond precision.
    fn set_calibration_time(&mut self, datetime: DateTime<Utc>) -> PyResult<()> {
        let secs = datetime.timestamp();
        let subsec_nanos = datetime.timestamp_subsec_nanos();
        let total_nanos = secs as i128 * 1_000_000_000 + subsec_nanos as i128;
        let time = OffsetDateTime::from_unix_timestamp_nanos(total_nanos)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.inner.set_calibration_time(time);
        Ok(())
    }

    /// Returns the calibration timestamp with nanosecond precision.
    #[getter]
    fn calibration_time(&self) -> Option<DateTime<Utc>> {
        self.inner.calibration_time().map(|t| {
            // Pre-epoch timestamps have negative total nanos; truncating
            // division would yield a negative remainder, which is an invalid
            // nanos value.
            let nanos = t.unix_timestamp_nanos();
            let secs = nanos.div_euclid(1_000_000_000) as i64;
            let nsecs = nanos.rem_euclid(1_000_000_000) as u32;
            Utc.timestamp_opt(secs, nsecs)
                .single()
                .expect("calibration time within chrono representable range")
        })
    }

    fn add_qubit_properties(
        &mut self,
        qubit: PyPhysicalQubitLike,
        props: PyQubitProp,
    ) -> PyResult<()> {
        self.inner
            .add_qubit_properties(qubit.into(), props.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn add_edge_properties(
        &mut self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
        props: PyEdgeProp,
    ) -> PyResult<()> {
        self.inner
            .add_edge_properties(control.into(), target.into(), props.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    #[getter]
    fn qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner.qubits().map(Into::into).collect()
    }

    #[getter]
    fn invalid_qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner.invalid_qubits().map(Into::into).collect()
    }

    #[setter]
    fn set_invalid_qubits(&mut self, qubits: PyPhysicalQubitList) -> PyResult<()> {
        let qubits = Vec::<PhysicalQubit>::from(qubits).into_iter().collect();
        self.inner
            .set_invalid_qubits(qubits)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn topology(&self) -> PyTopology {
        PyTopology {
            inner: self.inner.topology().clone(),
        }
    }

    #[getter]
    fn native_gates(&self) -> Vec<PyInstruction> {
        self.inner
            .native_gates()
            .iter()
            .cloned()
            .map(PyInstruction::from)
            .collect()
    }

    fn qubit_properties(&self, qubit: PyPhysicalQubitLike) -> Option<PyQubitProp> {
        self.inner
            .qubit_properties(qubit.into())
            .cloned()
            .map(PyQubitProp::from)
    }

    fn edge_properties(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
    ) -> Option<PyEdgeProp> {
        self.inner
            .edge_properties(control.into(), target.into())
            .cloned()
            .map(PyEdgeProp::from)
    }

    fn get_t1(&self, qubit: PyPhysicalQubitLike) -> Option<f64> {
        self.inner.get_t1(qubit.into())
    }

    fn get_t2(&self, qubit: PyPhysicalQubitLike) -> Option<f64> {
        self.inner.get_t2(qubit.into())
    }

    fn get_readout_error(&self, qubit: PyPhysicalQubitLike) -> Option<f64> {
        self.inner.get_readout_error(qubit.into())
    }

    #[getter]
    fn default_single_qubit_error(&self) -> Option<f64> {
        self.inner.default_single_qubit_error()
    }

    #[getter]
    fn default_t1(&self) -> Option<f64> {
        self.inner.default_t1()
    }

    #[getter]
    fn default_t2(&self) -> Option<f64> {
        self.inner.default_t2()
    }

    #[getter]
    fn default_readout_error(&self) -> Option<f64> {
        self.inner.default_readout_error()
    }

    #[getter]
    fn default_two_qubit_error(&self) -> Option<f64> {
        self.inner.default_two_qubit_error()
    }

    /// Creates a device with qubits connected as a directed line.
    ///
    /// The device contains physical qubits `0..num_qubits`, all online.
    /// Couplings are directed: `q[i] -> q[i+1]`.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `num_qubits` - Number of qubits in the line.
    ///
    /// # Returns
    ///
    /// A new `Device` with a directed line topology.
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if topology construction fails.
    #[staticmethod]
    pub fn line(name: String, num_qubits: u32) -> PyResult<Self> {
        let inner =
            Device::line(name, num_qubits).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with the supplied qubits connected as a directed
    /// line.
    ///
    /// Couplings follow the supplied order: `qubits[i] -> qubits[i+1]`.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `physical_qubits` - List of qubit IDs in line order.
    ///
    /// # Returns
    ///
    /// A new `Device` with a directed line topology following the given
    /// qubit order.
    #[staticmethod]
    pub fn line_from_qubits(name: String, physical_qubits: PyPhysicalQubitList) -> PyResult<Self> {
        let qubits = Vec::<PhysicalQubit>::from(physical_qubits);
        let inner = Device::line_from_qubits(name, qubits)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with qubits connected as a bidirectional line.
    ///
    /// The device contains physical qubits `0..num_qubits`, all online.
    /// Adjacent qubits are connected in both directions.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `num_qubits` - Number of qubits.
    #[staticmethod]
    pub fn bidirectional_line(name: String, num_qubits: u32) -> PyResult<Self> {
        let inner = Device::bidirectional_line(name, num_qubits)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with qubits connected as a bidirectional ring.
    ///
    /// The device contains physical qubits `0..num_qubits`, all online.
    /// For two or more qubits, each qubit is connected to its successor
    /// (modulo `num_qubits`) in both directions.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `num_qubits` - Number of qubits (minimum 2 for a non-trivial
    ///   ring).
    #[staticmethod]
    pub fn ring(name: String, num_qubits: u32) -> PyResult<Self> {
        let inner =
            Device::ring(name, num_qubits).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with qubits connected as a bidirectional star.
    ///
    /// The device contains physical qubits `0..num_qubits`, all online.
    /// Every non-center qubit is connected to the center qubit in both
    /// directions.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `num_qubits` - Total number of qubits.
    /// * `center` - The center qubit ID (must be `< num_qubits`).
    #[staticmethod]
    pub fn star(name: String, num_qubits: u32, center: u32) -> PyResult<Self> {
        let inner = Device::star(name, num_qubits, center)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with qubits connected as a bidirectional grid.
    ///
    /// Qubit IDs are assigned in row-major order. Horizontal and vertical
    /// nearest-neighbor couplings are added in both directions.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `rows` - Number of rows in the grid.
    /// * `cols` - Number of columns in the grid.
    #[staticmethod]
    pub fn grid(name: String, rows: u32, cols: u32) -> PyResult<Self> {
        let inner =
            Device::grid(name, rows, cols).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a device with explicit directed edges.
    ///
    /// The device contains physical qubits `0..num_qubits`, all online.
    /// Each `(control, target)` pair in `edges` becomes one directed
    /// coupling.
    ///
    /// # Arguments
    ///
    /// * `name` - The device name.
    /// * `num_qubits` - Number of physical qubits.
    /// * `edges` - List of `(control, target)` pairs defining directed
    ///   couplings.
    #[staticmethod]
    pub fn from_edges(name: String, num_qubits: u32, edges: Vec<(u32, u32)>) -> PyResult<Self> {
        let inner = Device::from_edges(name, num_qubits, &edges)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the error rate for a given instruction on a single qubit.
    ///
    /// Falls back to the per-qubit native instruction error if available,
    /// otherwise to the default single-qubit error rate.
    ///
    /// # Arguments
    ///
    /// * `qubit` - The qubit to query.
    /// * `instruction` - The instruction whose error rate is requested.
    ///
    /// # Returns
    ///
    /// The error rate as `float`, or `None` if the qubit is unusable.
    pub fn single_qubit_error(
        &self,
        qubit: PyPhysicalQubitLike,
        instruction: PyInstruction,
    ) -> Option<f64> {
        self.inner
            .single_qubit_error(qubit.into(), &instruction.inner)
    }

    /// Returns the error rate for a given instruction on a directed
    /// coupling.
    ///
    /// Falls back to the per-edge native instruction error if available,
    /// otherwise to the default two-qubit error rate.
    ///
    /// # Arguments
    ///
    /// * `control` - The source qubit of the coupling.
    /// * `target` - The destination qubit of the coupling.
    /// * `instruction` - The instruction whose error rate is requested.
    ///
    /// # Returns
    ///
    /// The error rate as `float`, or `None` if either qubit is unusable
    /// or the coupling does not exist.
    pub fn two_qubit_error(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
        instruction: PyInstruction,
    ) -> Option<f64> {
        self.inner
            .two_qubit_error(control.into(), target.into(), &instruction.inner)
    }
    /// Returns the best available two-qubit error on a directed coupling.
    ///
    /// Scans all native instructions on the edge and returns the minimum
    /// error rate. Useful for routing cost estimation.
    ///
    /// # Arguments
    ///
    /// * `control` - The source qubit.
    /// * `target` - The destination qubit.
    ///
    /// # Returns
    ///
    /// The minimum error rate, or `None` if the coupling is unusable.
    pub fn edge_error(
        &self,
        control: PyPhysicalQubitLike,
        target: PyPhysicalQubitLike,
    ) -> Option<f64> {
        self.inner.edge_error(control.into(), target.into())
    }

    /// Checks whether a physical qubit is registered and not marked
    /// invalid.
    ///
    /// # Arguments
    ///
    /// * `qubit` - The qubit to check.
    ///
    /// # Returns
    ///
    /// `True` if the qubit is online and usable.
    pub fn is_usable_qubit(&self, qubit: PyPhysicalQubitLike) -> bool {
        self.inner.is_usable_qubit(qubit.into())
    }

    /// Returns whether an instruction can execute natively on the exact
    /// ordered physical qubits.
    fn supports_native_instruction(
        &self,
        instruction: PyInstruction,
        qargs: PyPhysicalQubitList,
    ) -> bool {
        let qargs = Vec::<PhysicalQubit>::from(qargs);
        self.inner
            .supports_native_instruction(&instruction.inner, &qargs)
    }

    /// Validates one value-level operation in the physical-qubit ID space.
    ///
    /// Classical control-flow bodies are checked recursively without requiring
    /// the operation's owning circuit or classical resource tables.
    fn validate_operation(&self, operation: &PyValueOperation) -> PyResult<()> {
        self.inner
            .validate_value_operation(&operation.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Validates every operation in a circuit in the physical-qubit ID space.
    fn validate_circuit(&self, circuit: &PyCircuit) -> PyResult<()> {
        self.inner
            .validate_circuit(&circuit.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Returns a list of all usable physical qubits.
    ///
    /// Usable qubits are those registered with the device and not marked
    /// as invalid (offline or faulty).
    ///
    /// # Returns
    ///
    /// A list of `PhysicalQubit` objects representing usable qubits.
    #[getter]
    pub fn usable_qubits(&self) -> Vec<PyPhysicalQubit> {
        self.inner.usable_qubits().map(Into::into).collect()
    }

    /// Returns the number of usable (registered and not invalid) physical
    /// qubits.
    #[getter]
    pub fn num_usable_qubits(&self) -> usize {
        self.inner.num_usable_qubits()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __repr__(&self) -> String {
        format!("Device(name='{}')", self.name(),)
    }
}
