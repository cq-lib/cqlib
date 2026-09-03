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

//! Quantum Device Hardware Characteristics and Topology.
//!
//! This module defines the core data structures used to represent a target quantum backend
//! within the CQLib compiler. These structures (`Device`, `QubitProp`, `EdgeProp`, `InstructionProp`)
//! encapsulate all the physical constraints and fidelity data necessary for noise-aware compilation,
//! mapping, routing, and circuit scheduling.

use crate::circuit::{
    Circuit, ClassicalControlOp, Directive, Instruction, Operation, Qubit, StandardGate,
    ValueClassicalControlOp, ValueInstruction, ValueOperation,
};
use crate::device::topology::Topology;
use crate::device::{DeviceError, DeviceValidationError, PhysicalQubit};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::RangeInclusive;
use time::OffsetDateTime;

/// Checks that an instruction can be represented at a native-capability location.
///
/// The current device model resolves only atomic standard gates and represents
/// physical placement only for individual qubits and directed two-qubit edges.
fn validate_native_gate_arity(
    instruction: &Instruction,
    expected: RangeInclusive<usize>,
) -> Result<(), DeviceError> {
    let Instruction::Standard(gate) = instruction else {
        return Err(DeviceError::NonStandardNativeInstruction {
            instruction: instruction.to_string(),
        });
    };
    let actual = gate.num_qubits();
    if expected.contains(&actual) {
        Ok(())
    } else {
        Err(DeviceError::InvalidNativeInstructionArity {
            instruction: instruction.to_string(),
            expected,
            actual,
        })
    }
}

/// Represents the physical properties and execution characteristics of a quantum instruction (gate)
/// when applied to specific qubit(s).
///
/// This structure stores crucial calibration data such as the error rate (fidelity) and the
/// execution duration of the gate. This information is vital for noise-aware compilation,
/// gate scheduling, and duration-based dynamical decoupling.
#[derive(Debug, Clone)]
pub struct InstructionProp {
    /// The instruction (gate) being characterized.
    instruction: Instruction,
    /// Error rate for this instruction on the specific qubit(s), range [0.0, 1.0].
    error_rate: f64,
    /// Gate duration in nanoseconds.
    length: Option<f64>,
}

impl InstructionProp {
    /// Creates a new `InstructionProp`.
    pub fn new(instruction: Instruction, error_rate: f64) -> Self {
        Self {
            instruction,
            error_rate,
            length: None,
        }
    }

    /// Sets the gate duration (in nanoseconds) using the builder pattern.
    pub fn with_length(mut self, length: f64) -> Self {
        self.length = Some(length);
        self
    }

    pub fn set_length(&mut self, length: f64) {
        self.length = Some(length);
    }
    pub fn with_instruction(mut self, instruction: Instruction) -> Self {
        self.instruction = instruction;
        self
    }

    pub fn set_instruction(&mut self, instruction: Instruction) {
        self.instruction = instruction;
    }

    pub fn with_error_rate(mut self, error_rate: f64) -> Self {
        self.error_rate = error_rate;
        self
    }

    pub fn set_error_rate(&mut self, error_rate: f64) {
        self.error_rate = error_rate;
    }

    /// Gets a reference to the underlying instruction.
    pub fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// Gets the error rate of this instruction.
    pub fn error_rate(&self) -> f64 {
        self.error_rate
    }

    /// Gets the duration of this instruction in nanoseconds, if defined.
    pub fn length(&self) -> Option<f64> {
        self.length
    }

    fn validate(&self) -> Result<(), DeviceError> {
        if !(self.error_rate.is_finite() && (0.0..=1.0).contains(&self.error_rate)) {
            return Err(DeviceError::InvalidNativeInstructionErrorRate {
                instruction: self.instruction.to_string(),
                value: format!("{:?}", self.error_rate),
            });
        }
        if let Some(length) = self.length
            && !(length.is_finite() && length >= 0.0)
        {
            return Err(DeviceError::InvalidNativeInstructionDuration {
                instruction: self.instruction.to_string(),
                value: format!("{length:?}"),
            });
        }
        Ok(())
    }
}

/// Represents the physical and operational properties of a single quantum qubit.
///
/// This includes coherence metrics (T1 relaxation time, T2 dephasing time), operational frequency,
/// and measurement error rates. It also maintains a list of `InstructionProp` which defines
/// the specific native single-qubit instructions supported by this qubit along with their
/// calibrated fidelities and durations.
#[derive(Debug, Clone)]
pub struct QubitProp {
    /// Readout error rate, range [0.0, 1.0].
    readout_error: f64,
    /// Prob of measuring 0 given state was prepared in 1 (p0|1)
    prob_meas0_prep1: Option<f64>,
    /// Prob of measuring 1 given state was prepared in 0 (p1|0)
    prob_meas1_prep0: Option<f64>,

    /// T1 relaxation time in microseconds.
    t1: Option<f64>,
    /// T2 dephasing time in microseconds.
    t2: Option<f64>,
    /// Qubit frequency in GHz.
    frequency: Option<f64>,
    /// Native instructions supported on this qubit.
    ///
    /// An empty list inherits matching one-qubit instructions from the device
    /// defaults. A non-empty list completely overrides those defaults for this
    /// qubit.
    native_instructions: Vec<InstructionProp>,
}

impl QubitProp {
    /// Creates a new `QubitProp` with the specified readout error rate.
    pub fn new(readout_error: f64) -> Self {
        Self {
            readout_error,
            prob_meas0_prep1: None,
            prob_meas1_prep0: None,
            t1: None,
            t2: None,
            frequency: None,
            native_instructions: Vec::new(),
        }
    }

    /// Sets the probability of measuring 0 given the state was prepared in 1.
    pub fn with_prob_meas0_prep1(mut self, prob: f64) -> Self {
        self.prob_meas0_prep1 = Some(prob);
        self
    }
    pub fn set_prob_meas0_prep1(&mut self, prob: f64) {
        self.prob_meas0_prep1 = Some(prob);
    }

    /// Sets the probability of measuring 1 given the state was prepared in 0.
    pub fn with_prob_meas1_prep0(mut self, prob: f64) -> Self {
        self.prob_meas1_prep0 = Some(prob);
        self
    }
    pub fn set_prob_meas1_prep0(&mut self, prob: f64) {
        self.prob_meas1_prep0 = Some(prob);
    }

    /// Sets the T1 relaxation time in microseconds.
    pub fn with_t1(mut self, t1: f64) -> Self {
        self.t1 = Some(t1);
        self
    }

    pub fn set_t1(&mut self, t1: f64) {
        self.t1 = Some(t1);
    }

    /// Sets the T2 dephasing time in microseconds.
    pub fn with_t2(mut self, t2: f64) -> Self {
        self.t2 = Some(t2);
        self
    }
    pub fn set_t2(&mut self, t2: f64) {
        self.t2 = Some(t2);
    }

    /// Sets the qubit frequency in GHz.
    pub fn with_frequency(mut self, frequency: f64) -> Self {
        self.frequency = Some(frequency);
        self
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = Some(frequency);
    }

    /// Adds a single-qubit standard gate to this qubit's native capabilities.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError`] if `prop` does not describe a single-qubit
    /// standard gate.
    pub fn with_native_instruction(mut self, prop: InstructionProp) -> Result<Self, DeviceError> {
        self.set_native_instruction(prop)?;
        Ok(self)
    }
    /// Appends a single-qubit standard gate, preserving the list on error.
    pub fn set_native_instruction(&mut self, prop: InstructionProp) -> Result<(), DeviceError> {
        validate_native_gate_arity(prop.instruction(), 1..=1)?;
        prop.validate()?;
        self.native_instructions.push(prop);
        Ok(())
    }

    /// Gets the readout error rate.
    pub fn readout_error(&self) -> f64 {
        self.readout_error
    }

    /// Gets the probability of measuring 0 given the state was prepared in 1 (p0|1).
    pub fn prob_meas0_prep1(&self) -> Option<f64> {
        self.prob_meas0_prep1
    }

    /// Gets the probability of measuring 1 given the state was prepared in 0 (p1|0).
    pub fn prob_meas1_prep0(&self) -> Option<f64> {
        self.prob_meas1_prep0
    }

    /// Gets the T1 relaxation time in microseconds, if defined.
    pub fn t1(&self) -> Option<f64> {
        self.t1
    }

    /// Gets the T2 dephasing time in microseconds, if defined.
    pub fn t2(&self) -> Option<f64> {
        self.t2
    }

    /// Gets the qubit frequency in GHz, if defined.
    pub fn frequency(&self) -> Option<f64> {
        self.frequency
    }

    /// Gets a slice of the native instructions supported on this qubit.
    pub fn native_instructions(&self) -> &[InstructionProp] {
        &self.native_instructions
    }
}

/// Represents the physical properties of a coupling edge between two qubits in the device topology.
///
/// This structure primarily tracks the native multi-qubit instructions (e.g., CX, CZ)
/// supported across this specific physical connection, including their directional
/// error rates and execution times.
#[derive(Debug, Clone)]
pub struct EdgeProp {
    /// Native instructions supported on this directed edge (typically 2-qubit gates).
    ///
    /// An empty list inherits matching two-qubit instructions from the device
    /// defaults. A non-empty list completely overrides those defaults for this
    /// ordered edge and does not grant support in the reverse direction.
    native_instructions: Vec<InstructionProp>,
}

/// Result of resolving one instruction against a local override and device defaults.
enum NativeInstructionSupport<'a> {
    /// The instruction is explicitly supported and has local calibration.
    Explicit(&'a InstructionProp),
    /// The instruction is inherited from the device-wide defaults.
    Inherited,
    /// Neither the local override nor the defaults support the instruction.
    Unsupported,
}

/// Optional calibration attached to one exact native instruction capability.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct NativeInstructionCalibration {
    pub(crate) error_rate: Option<f64>,
    pub(crate) duration: Option<f64>,
}

impl EdgeProp {
    /// Creates a new empty edge property.
    pub fn new() -> Self {
        Self {
            native_instructions: Vec::new(),
        }
    }

    /// Adds a two-qubit standard gate to this edge's native capabilities.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError`] if `prop` does not describe a two-qubit
    /// standard gate.
    pub fn with_native_instruction(mut self, prop: InstructionProp) -> Result<Self, DeviceError> {
        self.set_native_instruction(prop)?;
        Ok(self)
    }

    /// Appends a two-qubit standard gate, preserving the list on error.
    pub fn set_native_instruction(&mut self, prop: InstructionProp) -> Result<(), DeviceError> {
        validate_native_gate_arity(prop.instruction(), 2..=2)?;
        prop.validate()?;
        self.native_instructions.push(prop);
        Ok(())
    }

    /// Gets a slice of the native instructions supported on this edge.
    pub fn native_instructions(&self) -> &[InstructionProp] {
        &self.native_instructions
    }
}

impl Default for EdgeProp {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a quantum device's hardware characteristics and topology.
///
/// The `Device` struct is a fundamental component for compiler optimization, mapping,
/// routing, and noise-aware scheduling. It encapsulates:
/// - The physical connectivity (`Topology`) between qubits.
/// - Available and faulty qubits.
/// - Device-wide default parameters (e.g., T1, T2, gate error rates).
/// - Specific physical properties and supported instructions for individual qubits
///   and coupling edges.
///
/// This structure provides the necessary physical constraints and fidelity data
/// required to simulate noise models or compile quantum circuits onto realistic
/// backend hardware.
///
/// # Example
///
/// ```rust
/// use std::collections::HashSet;
/// use cqlib_core::device::{Device, PhysicalQubit, QubitProp, Topology};
///
/// // Create a 2-qubit topology
/// let q0 = PhysicalQubit::new(0);
/// let q1 = PhysicalQubit::new(1);
/// let topo = Topology::new(vec![q0, q1], vec![(q0, q1, "G1".to_string())]).unwrap();
///
/// // Initialize a device with defaults
/// let mut device = Device::new("mock_device", HashSet::from_iter([q0, q1]), topo).unwrap()
///     .with_default_t1(50.0)
///     .with_default_t2(25.0)
///     .with_default_readout_error(0.01);
///
/// // Add specific properties for Qubit 0
/// let q0_prop = QubitProp::new(0.05).with_t1(40.0);
/// device.add_qubit_properties(q0, q0_prop).unwrap();
///
/// // Query T1, using specific properties if available, else fallback to defaults
/// assert_eq!(device.get_t1(q0), Some(40.0));
/// assert_eq!(device.get_t1(q1), Some(50.0));
/// ```
#[derive(Debug, Clone)]
pub struct Device {
    name: String,
    /// Physical qubits registered with the device.
    qubits: BTreeSet<PhysicalQubit>,
    /// Offline or faulty qubits.
    invalid_qubits: BTreeSet<PhysicalQubit>,
    /// Connectivity topology.
    topology: Topology,
    /// Device-wide default native gates.
    ///
    /// Defaults apply by instruction arity when a qubit or directed edge has
    /// no non-empty local native-instruction override. Local capabilities are
    /// not synchronized into this list.
    native_gates: Vec<Instruction>,

    /// System calibration timestamp.
    calibration_time: Option<OffsetDateTime>,
    /// Default T1 time (μs) for qubits without specific data.
    default_t1: Option<f64>,
    /// Default T2 time (μs) for qubits without specific data.
    default_t2: Option<f64>,
    /// Default readout error for qubits without specific data.
    default_readout_error: Option<f64>,
    /// Default single-qubit gate error.
    default_single_qubit_error: Option<f64>,
    /// Default two-qubit gate error.
    default_two_qubit_error: Option<f64>,

    /// Per-qubit properties (T1, T2, readout error, native gates).
    qubit_properties: HashMap<PhysicalQubit, QubitProp>,
    /// Per-edge properties (gate fidelities on specific couplings).
    edge_properties: HashMap<(PhysicalQubit, PhysicalQubit), EdgeProp>,
}

impl Device {
    /// Creates a new `Device` with the specified name and topology.
    pub fn new(
        name: impl Into<String>,
        qubits: HashSet<PhysicalQubit>,
        topology: Topology,
    ) -> Result<Self, DeviceError> {
        for q in topology.qubits() {
            if !qubits.contains(&q) {
                return Err(DeviceError::InvalidOnlineQubit(q));
            }
        }

        Ok(Self {
            name: name.into(),
            qubits: qubits.into_iter().collect(),
            invalid_qubits: BTreeSet::new(),
            topology,
            native_gates: Vec::new(),
            calibration_time: None,
            default_t1: None,
            default_t2: None,
            default_readout_error: None,
            default_single_qubit_error: None,
            default_two_qubit_error: None,
            qubit_properties: HashMap::new(),
            edge_properties: HashMap::new(),
        })
    }

    /// Creates a device with physical qubits connected as a directed line.
    ///
    /// The device contains physical qubits `0..num_qubits`, all of which are
    /// online. Native gates and calibration data are left unset and may be
    /// configured with the builder-style setters.
    pub fn line(name: impl Into<String>, num_qubits: u32) -> Result<Self, DeviceError> {
        let physical_qubits = (0..num_qubits).map(PhysicalQubit::new).collect::<Vec<_>>();
        Self::line_from_qubits(name, physical_qubits)
    }

    /// Creates a device with the supplied physical qubits connected as a directed line.
    ///
    /// The device contains every supplied physical qubit, all of which are
    /// online. Couplings follow the supplied order: `qubits[i] -> qubits[i + 1]`.
    pub fn line_from_qubits(
        name: impl Into<String>,
        physical_qubits: Vec<PhysicalQubit>,
    ) -> Result<Self, DeviceError> {
        let qubits = physical_qubits.iter().copied().collect::<HashSet<_>>();
        let topology = Topology::line(physical_qubits).map_err(DeviceError::InvalidTopology)?;
        Self::new(name, qubits, topology)
    }

    /// Creates a device with physical qubits connected as a bidirectional line.
    ///
    /// The device contains physical qubits `0..num_qubits`, all of which are
    /// online. Adjacent qubits are connected in both directions.
    pub fn bidirectional_line(
        name: impl Into<String>,
        num_qubits: u32,
    ) -> Result<Self, DeviceError> {
        let edges = (0..num_qubits.saturating_sub(1))
            .flat_map(|index| [(index, index + 1), (index + 1, index)])
            .collect::<Vec<_>>();
        Self::from_u32_edges(name, num_qubits, &edges)
    }

    /// Creates a device with physical qubits connected as a bidirectional ring.
    ///
    /// The device contains physical qubits `0..num_qubits`, all of which are
    /// online. For two or more qubits, each qubit is connected to its successor
    /// modulo `num_qubits` in both directions.
    pub fn ring(name: impl Into<String>, num_qubits: u32) -> Result<Self, DeviceError> {
        let mut edges = BTreeSet::new();
        if num_qubits >= 2 {
            for index in 0..num_qubits {
                let next = (index + 1) % num_qubits;
                edges.insert((index, next));
                edges.insert((next, index));
            }
        }
        let edges = edges.into_iter().collect::<Vec<_>>();
        Self::from_u32_edges(name, num_qubits, &edges)
    }

    /// Creates a device with physical qubits connected as a bidirectional star.
    ///
    /// The device contains physical qubits `0..num_qubits`, all of which are
    /// online. Every non-center qubit is connected to `center` in both
    /// directions.
    pub fn star(
        name: impl Into<String>,
        num_qubits: u32,
        center: u32,
    ) -> Result<Self, DeviceError> {
        let edges = (0..num_qubits)
            .filter(|&index| index != center)
            .flat_map(|index| [(center, index), (index, center)])
            .collect::<Vec<_>>();
        Self::from_u32_edges(name, num_qubits, &edges)
    }

    /// Creates a device with physical qubits connected as a bidirectional grid.
    ///
    /// Qubit IDs are assigned in row-major order. Horizontal and vertical
    /// nearest-neighbor couplings are added in both directions.
    pub fn grid(name: impl Into<String>, rows: u32, cols: u32) -> Result<Self, DeviceError> {
        let num_qubits = rows.saturating_mul(cols);
        let mut edges = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let current = row * cols + col;
                if col + 1 < cols {
                    edges.push((current, current + 1));
                    edges.push((current + 1, current));
                }
                if row + 1 < rows {
                    edges.push((current, current + cols));
                    edges.push((current + cols, current));
                }
            }
        }
        Self::from_u32_edges(name, num_qubits, &edges)
    }

    /// Creates a device with physical qubits `0..num_qubits` and explicit directed edges.
    ///
    /// Each `(control, target)` pair in `edges` becomes one directed coupling.
    pub fn from_edges(
        name: impl Into<String>,
        num_qubits: u32,
        edges: &[(u32, u32)],
    ) -> Result<Self, DeviceError> {
        Self::from_u32_edges(name, num_qubits, edges)
    }

    fn from_u32_edges(
        name: impl Into<String>,
        num_qubits: u32,
        edges: &[(u32, u32)],
    ) -> Result<Self, DeviceError> {
        let physical_qubits = (0..num_qubits).map(PhysicalQubit::new).collect::<Vec<_>>();
        let qubits = physical_qubits.iter().copied().collect::<HashSet<_>>();
        let coupling_map = edges
            .iter()
            .enumerate()
            .map(|(index, &(control, target))| {
                (
                    PhysicalQubit::new(control),
                    PhysicalQubit::new(target),
                    format!("e{index}"),
                )
            })
            .collect::<Vec<_>>();
        let topology =
            Topology::new(physical_qubits, coupling_map).map_err(DeviceError::InvalidTopology)?;
        Self::new(name, qubits, topology)
    }

    /// Sets the offline or faulty physical qubits using the builder pattern.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::QubitNotInDevice`] if any invalid qubit is not
    /// registered with the device.
    pub fn with_invalid_qubits(
        mut self,
        invalid_qubits: HashSet<PhysicalQubit>,
    ) -> Result<Self, DeviceError> {
        self.set_invalid_qubits(invalid_qubits)?;
        Ok(self)
    }

    /// Sets the offline or faulty physical qubits.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::QubitNotInDevice`] if any invalid qubit is not
    /// registered with the device. The existing set is preserved on error.
    pub fn set_invalid_qubits(
        &mut self,
        invalid_qubits: HashSet<PhysicalQubit>,
    ) -> Result<(), DeviceError> {
        for &qubit in &invalid_qubits {
            if !self.qubits.contains(&qubit) {
                return Err(DeviceError::QubitNotInDevice(qubit));
            }
        }
        self.invalid_qubits = invalid_qubits.into_iter().collect();
        Ok(())
    }

    /// Sets the device-wide default native gates.
    ///
    /// This does not add, remove, or otherwise synchronize native instructions
    /// stored on individual qubits or directed edges.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError`] if any entry is not a standard gate or acts on
    /// more than two qubits.
    pub fn with_native_gates(mut self, gates: Vec<Instruction>) -> Result<Self, DeviceError> {
        self.set_native_gates(gates)?;
        Ok(self)
    }

    /// Replaces the device-wide native defaults after validating the full list.
    ///
    /// The existing defaults are preserved when any entry is invalid.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError`] if any entry is not a standard gate or acts on
    /// more than two qubits.
    pub fn set_native_gates(&mut self, gates: Vec<Instruction>) -> Result<(), DeviceError> {
        for gate in &gates {
            validate_native_gate_arity(gate, 0..=2)?;
        }
        self.native_gates = gates;
        Ok(())
    }

    /// Sets the system calibration timestamp.
    pub fn with_calibration_time(mut self, time: OffsetDateTime) -> Self {
        self.calibration_time = Some(time);
        self
    }

    pub fn set_calibration_time(&mut self, time: OffsetDateTime) {
        self.calibration_time = Some(time);
    }

    /// Sets the default T1 time (μs).
    pub fn with_default_t1(mut self, t1: f64) -> Self {
        self.default_t1 = Some(t1);
        self
    }

    pub fn set_default_t1(&mut self, t1: f64) {
        self.default_t1 = Some(t1);
    }

    /// Sets the default T2 time (μs).
    pub fn with_default_t2(mut self, t2: f64) -> Self {
        self.default_t2 = Some(t2);
        self
    }

    pub fn set_default_t2(&mut self, t2: f64) {
        self.default_t2 = Some(t2);
    }

    /// Sets the default readout error rate.
    pub fn with_default_readout_error(mut self, error: f64) -> Self {
        self.default_readout_error = Some(error);
        self
    }

    pub fn set_default_readout_error(&mut self, error: f64) {
        self.default_readout_error = Some(error);
    }

    /// Sets the default single-qubit gate error rate.
    pub fn with_default_single_qubit_error(mut self, error: f64) -> Self {
        self.default_single_qubit_error = Some(error);
        self
    }

    pub fn set_default_single_qubit_error(&mut self, error: f64) {
        self.default_single_qubit_error = Some(error);
    }

    /// Sets the default two-qubit gate error rate.
    pub fn with_default_two_qubit_error(mut self, error: f64) -> Self {
        self.default_two_qubit_error = Some(error);
        self
    }

    pub fn set_default_two_qubit_error(&mut self, error: f64) {
        self.default_two_qubit_error = Some(error);
    }

    /// Adds properties for a specific qubit.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::QubitNotInDevice`] if the qubit is not registered
    /// with the device, or [`DeviceError::QubitNotInTopology`] if it is
    /// registered but absent from the topology. Properties may be retained for
    /// a registered qubit while it is marked invalid/offline.
    pub fn add_qubit_properties(
        &mut self,
        qubit: PhysicalQubit,
        props: QubitProp,
    ) -> Result<(), DeviceError> {
        if !self.qubits.contains(&qubit) {
            return Err(DeviceError::QubitNotInDevice(qubit));
        }
        if !self.topology.contains_qubit(&qubit) {
            return Err(DeviceError::QubitNotInTopology(qubit));
        }
        self.qubit_properties.insert(qubit, props);
        Ok(())
    }

    /// Adds properties for a specific coupling edge.
    ///
    /// # Errors
    ///
    /// Returns `DeviceError::EdgeNotInTopology` if the edge is not in the device's topology.
    pub fn add_edge_properties(
        &mut self,
        control: PhysicalQubit,
        target: PhysicalQubit,
        props: EdgeProp,
    ) -> Result<(), DeviceError> {
        if !self.topology.supports_directed_coupling(control, target) {
            return Err(DeviceError::EdgeNotInTopology(control, target));
        }
        self.edge_properties.insert((control, target), props);
        Ok(())
    }

    /// Compares instruction identities supported by the current device model.
    ///
    /// Device-native capabilities are currently limited to standard gates;
    /// composite and runtime instructions are handled by circuit validation.
    fn instruction_matches(stored: &Instruction, requested: &Instruction) -> bool {
        match (stored, requested) {
            (Instruction::Standard(stored), Instruction::Standard(requested)) => {
                stored == requested
            }
            _ => false,
        }
    }

    /// Resolves the shared local-override/default-inheritance capability contract.
    ///
    /// A non-empty local list is authoritative. An absent or empty local list
    /// inherits the device defaults. Calibration consumers retain the explicit
    /// property through the result instead of repeating capability resolution.
    fn resolve_native_instruction<'a>(
        &'a self,
        local: Option<&'a [InstructionProp]>,
        instruction: &Instruction,
    ) -> NativeInstructionSupport<'a> {
        if let Some(local) = local.filter(|instructions| !instructions.is_empty()) {
            return local
                .iter()
                .find(|prop| Self::instruction_matches(prop.instruction(), instruction))
                .map_or(
                    NativeInstructionSupport::Unsupported,
                    NativeInstructionSupport::Explicit,
                );
        }

        if self
            .native_gates
            .iter()
            .any(|stored| Self::instruction_matches(stored, instruction))
        {
            NativeInstructionSupport::Inherited
        } else {
            NativeInstructionSupport::Unsupported
        }
    }

    /// Gets the name of the device.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets an iterator over the physical qubits registered with the device.
    pub fn qubits(&self) -> impl Iterator<Item = PhysicalQubit> + '_ {
        self.qubits.iter().copied()
    }

    /// Gets an iterator over the invalid (offline/faulty) qubits.
    pub fn invalid_qubits(&self) -> impl Iterator<Item = PhysicalQubit> + '_ {
        self.invalid_qubits.iter().copied()
    }

    /// Returns whether a physical qubit is registered and not marked invalid.
    pub fn is_usable_qubit(&self, qubit: PhysicalQubit) -> bool {
        self.qubits.contains(&qubit) && !self.invalid_qubits.contains(&qubit)
    }

    /// Gets an iterator over registered physical qubits that are not invalid.
    pub fn usable_qubits(&self) -> impl Iterator<Item = PhysicalQubit> + '_ {
        self.qubits.difference(&self.invalid_qubits).copied()
    }

    /// Returns the number of registered physical qubits that are not invalid.
    pub fn num_usable_qubits(&self) -> usize {
        self.qubits.len() - self.invalid_qubits.len()
    }

    /// Gets a reference to the device's connectivity topology.
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Gets the default native gates supported by the device.
    pub fn native_gates(&self) -> &[Instruction] {
        &self.native_gates
    }

    /// Gets the properties of a specific qubit.
    pub fn qubit_properties(&self, qubit: PhysicalQubit) -> Option<&QubitProp> {
        self.qubit_properties.get(&qubit)
    }

    /// Gets the properties of a specific coupling edge.
    pub fn edge_properties(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
    ) -> Option<&EdgeProp> {
        self.edge_properties.get(&(control, target))
    }

    /// Returns whether a native hardware instruction can execute on the exact
    /// ordered physical qargs.
    ///
    /// This query evaluates capability only; calibration error rates and
    /// durations do not affect the result. Standard one- and two-qubit gates
    /// follow the device default/local-override contract, and two-qubit qargs
    /// are directional. Structured control flow, runtime operations, and
    /// composite gates are not atomic device capabilities; validate them with
    /// [`Self::validate_operation`] or [`Self::validate_circuit`] instead.
    pub fn supports_native_instruction(
        &self,
        instruction: &Instruction,
        qargs: &[PhysicalQubit],
    ) -> bool {
        if qargs.iter().any(|qubit| !self.is_usable_qubit(*qubit)) {
            return false;
        }

        match instruction {
            Instruction::Standard(StandardGate::GPhase) if qargs.is_empty() => !matches!(
                self.resolve_native_instruction(None, instruction),
                NativeInstructionSupport::Unsupported
            ),
            Instruction::Standard(gate) if gate.num_qubits() == qargs.len() => match qargs {
                [qubit] => !matches!(
                    self.resolve_native_instruction(
                        self.qubit_properties
                            .get(qubit)
                            .map(QubitProp::native_instructions),
                        instruction,
                    ),
                    NativeInstructionSupport::Unsupported
                ),
                [control, target]
                    if self.topology.supports_directed_coupling(*control, *target) =>
                {
                    !matches!(
                        self.resolve_native_instruction(
                            self.edge_properties(*control, *target)
                                .map(EdgeProp::native_instructions),
                            instruction,
                        ),
                        NativeInstructionSupport::Unsupported
                    )
                }
                _ => false,
            },
            Instruction::Standard(_)
            | Instruction::McGate(_)
            | Instruction::UnitaryGate(_)
            | Instruction::CircuitGate(_)
            | Instruction::Directive(_)
            | Instruction::ClassicalData(_)
            | Instruction::ClassicalControl(_)
            | Instruction::Delay => false,
        }
    }

    /// Validates one operation interpreted in the physical-qubit ID space.
    ///
    /// Structured control-flow bodies are checked recursively. This method is
    /// read-only and never performs layout, routing, lowering, or direction
    /// correction.
    pub fn validate_operation(&self, operation: &Operation) -> Result<(), DeviceValidationError> {
        let qargs = self.validate_qargs(&operation.qubits)?;
        match &operation.instruction {
            Instruction::ClassicalControl(control) => match control {
                ClassicalControlOp::If(op) => {
                    self.validate_control_body(op.then_body().operations())?;
                    if let Some(body) = op.else_body() {
                        self.validate_control_body(body.operations())?;
                    }
                }
                ClassicalControlOp::While(op) => {
                    self.validate_control_body(op.body().operations())?
                }
                ClassicalControlOp::For(op) => {
                    self.validate_control_body(op.body().operations())?
                }
                ClassicalControlOp::Switch(op) => {
                    for case in op.cases() {
                        self.validate_control_body(case.body().operations())?;
                    }
                    if let Some(body) = op.default() {
                        self.validate_control_body(body.operations())?;
                    }
                }
                ClassicalControlOp::Break | ClassicalControlOp::Continue => {}
            },
            instruction => self.validate_atomic_instruction(instruction, qargs)?,
        }
        Ok(())
    }

    /// Validates one value-level operation interpreted in the physical-qubit ID space.
    ///
    /// This is the construction-IR counterpart of [`Self::validate_operation`].
    /// Structured control-flow bodies are checked recursively without first
    /// converting the operation into a [`Circuit`], so circuit-owned classical
    /// variables and values are not required solely for device validation.
    pub fn validate_value_operation(
        &self,
        operation: &ValueOperation,
    ) -> Result<(), DeviceValidationError> {
        let qargs = self.validate_qargs(&operation.qubits)?;
        match &operation.instruction {
            ValueInstruction::Instruction(instruction) => {
                self.validate_atomic_instruction(instruction, qargs)?
            }
            ValueInstruction::ClassicalControl(control) => match control {
                ValueClassicalControlOp::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.validate_value_control_body(then_body.operations())?;
                    if let Some(body) = else_body {
                        self.validate_value_control_body(body.operations())?;
                    }
                }
                ValueClassicalControlOp::While { body, .. }
                | ValueClassicalControlOp::For { body, .. } => {
                    self.validate_value_control_body(body.operations())?
                }
                ValueClassicalControlOp::Switch { cases, default, .. } => {
                    for case in cases {
                        self.validate_value_control_body(case.body.operations())?;
                    }
                    if let Some(body) = default {
                        self.validate_value_control_body(body.operations())?;
                    }
                }
                ValueClassicalControlOp::Break | ValueClassicalControlOp::Continue => {}
            },
        }
        Ok(())
    }

    fn validate_qargs(
        &self,
        qubits: &[Qubit],
    ) -> Result<Vec<PhysicalQubit>, DeviceValidationError> {
        let qargs = qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect::<Vec<_>>();
        // Diagnose unusable qargs before the coarser capability query so
        // callers receive the concrete physical-qubit failure.
        if let Some(qubit) = qargs
            .iter()
            .copied()
            .find(|qubit| !self.is_usable_qubit(*qubit))
        {
            return Err(DeviceValidationError::UnusablePhysicalQubit {
                device: self.name.clone(),
                qubit,
            });
        }
        Ok(qargs)
    }

    fn validate_atomic_instruction(
        &self,
        instruction: &Instruction,
        qargs: Vec<PhysicalQubit>,
    ) -> Result<(), DeviceValidationError> {
        match instruction {
            Instruction::Directive(Directive::Measure | Directive::Reset) if qargs.len() == 1 => {}
            Instruction::Directive(Directive::Barrier) => {}
            Instruction::ClassicalData(op) => match op {
                crate::circuit::ClassicalDataOp::Store { .. } if qargs.is_empty() => {}
                crate::circuit::ClassicalDataOp::MeasureBit { .. } if qargs.len() == 1 => {}
                crate::circuit::ClassicalDataOp::MeasureBits { .. } if !qargs.is_empty() => {}
                _ => return Err(self.unsupported_instruction(instruction, qargs)),
            },
            Instruction::Delay if qargs.len() == 1 => {}
            Instruction::McGate(_) | Instruction::UnitaryGate(_) | Instruction::CircuitGate(_) => {
                return Err(DeviceValidationError::UndecomposedInstruction {
                    device: self.name.clone(),
                    instruction: instruction.to_string(),
                    qargs,
                });
            }
            Instruction::Standard(gate) if gate.num_qubits() > 2 => {
                return Err(DeviceValidationError::UndecomposedInstruction {
                    device: self.name.clone(),
                    instruction: instruction.to_string(),
                    qargs,
                });
            }
            Instruction::Standard(gate) if gate.num_qubits() == 2 => {
                let [control, target] = qargs.as_slice() else {
                    return Err(self.unsupported_instruction(instruction, qargs));
                };
                if !self.topology.supports_directed_coupling(*control, *target) {
                    return Err(DeviceValidationError::MissingDirectedCoupling {
                        device: self.name.clone(),
                        instruction: instruction.to_string(),
                        control: *control,
                        target: *target,
                    });
                }
                if !self.supports_native_instruction(instruction, &qargs) {
                    return Err(self.unsupported_instruction(instruction, qargs));
                }
            }
            Instruction::Standard(_) if self.supports_native_instruction(instruction, &qargs) => {}
            _ => return Err(self.unsupported_instruction(instruction, qargs)),
        }
        Ok(())
    }

    /// Validates a circuit interpreted in the physical-qubit ID space.
    ///
    /// Validation stops at the first unsupported operation. The circuit is not
    /// modified and must already have completed logical-to-physical mapping and
    /// hardware lowering.
    pub fn validate_circuit(&self, circuit: &Circuit) -> Result<(), DeviceValidationError> {
        self.validate_operations(circuit.operations())
    }

    fn validate_operations(&self, operations: &[Operation]) -> Result<(), DeviceValidationError> {
        // Preserve source order and stop at the first failure so diagnostics
        // are deterministic for top-level and nested control-flow bodies.
        for operation in operations {
            self.validate_operation(operation)?;
        }
        Ok(())
    }

    /// Validates one canonical control-flow body.
    ///
    /// Until control bodies carry their own phase metadata, canonicalization
    /// represents a body-local global phase as one leading zero-qubit GPhase
    /// marker. It is semantic IR metadata rather than a hardware instruction,
    /// so it does not require a device capability. No other GPhase position is
    /// granted this exception.
    fn validate_control_body(&self, operations: &[Operation]) -> Result<(), DeviceValidationError> {
        let operations = if operations.first().is_some_and(|operation| {
            matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::GPhase)
            ) && operation.qubits.is_empty()
        }) {
            &operations[1..]
        } else {
            operations
        };
        self.validate_operations(operations)
    }

    fn validate_value_control_body(
        &self,
        operations: &[ValueOperation],
    ) -> Result<(), DeviceValidationError> {
        let operations = if operations.first().is_some_and(|operation| {
            matches!(
                operation.instruction,
                ValueInstruction::Instruction(Instruction::Standard(StandardGate::GPhase))
            ) && operation.qubits.is_empty()
        }) {
            &operations[1..]
        } else {
            operations
        };
        for operation in operations {
            self.validate_value_operation(operation)?;
        }
        Ok(())
    }

    /// Builds the common diagnostic for a well-formed but unsupported atomic instruction.
    fn unsupported_instruction(
        &self,
        instruction: &Instruction,
        qargs: Vec<PhysicalQubit>,
    ) -> DeviceValidationError {
        DeviceValidationError::UnsupportedInstruction {
            device: self.name.clone(),
            instruction: instruction.to_string(),
            qargs,
        }
    }

    /// Gets the error rate for `instruction` on a single physical qubit.
    ///
    /// Returns `None` if the qubit is not usable or does not support the
    /// instruction. A non-empty local native-instruction list is a complete
    /// capability override; otherwise device-wide one-qubit defaults apply.
    /// The default error rate is used only for an inherited supported gate.
    pub fn single_qubit_error(
        &self,
        qubit: PhysicalQubit,
        instruction: &Instruction,
    ) -> Option<f64> {
        if !self.is_usable_qubit(qubit) {
            return None;
        }

        if !matches!(instruction, Instruction::Standard(gate) if gate.num_qubits() == 1) {
            return None;
        }
        match self.resolve_native_instruction(
            self.qubit_properties
                .get(&qubit)
                .map(QubitProp::native_instructions),
            instruction,
        ) {
            NativeInstructionSupport::Explicit(prop) => Some(prop.error_rate()),
            NativeInstructionSupport::Inherited => self.default_single_qubit_error,
            NativeInstructionSupport::Unsupported => None,
        }
    }

    /// Gets the error rate for `instruction` on a directed coupling.
    ///
    /// Returns `None` if either endpoint is not usable, the exact directed
    /// coupling does not exist, or that edge does not support the instruction.
    /// A non-empty local native-instruction list is a complete capability
    /// override; otherwise device-wide two-qubit defaults apply. The default
    /// error rate is used only for an inherited supported gate.
    pub fn two_qubit_error(
        &self,
        control: PhysicalQubit,
        target: PhysicalQubit,
        instruction: &Instruction,
    ) -> Option<f64> {
        if !self.is_usable_qubit(control)
            || !self.is_usable_qubit(target)
            || !self.topology.supports_directed_coupling(control, target)
        {
            return None;
        }

        if !matches!(instruction, Instruction::Standard(gate) if gate.num_qubits() == 2) {
            return None;
        }
        match self.resolve_native_instruction(
            self.edge_properties(control, target)
                .map(EdgeProp::native_instructions),
            instruction,
        ) {
            NativeInstructionSupport::Explicit(prop) => Some(prop.error_rate()),
            NativeInstructionSupport::Inherited => self.default_two_qubit_error,
            NativeInstructionSupport::Unsupported => None,
        }
    }

    /// Returns calibration for a supported instruction on exact ordered qargs.
    ///
    /// Capability and calibration are deliberately separate: `None` means the
    /// instruction is unsupported, while `Some(default)` means it is supported
    /// but has no explicit or inherited calibration data.
    pub(crate) fn native_instruction_calibration(
        &self,
        instruction: &Instruction,
        qargs: &[PhysicalQubit],
    ) -> Option<NativeInstructionCalibration> {
        if !self.supports_native_instruction(instruction, qargs) {
            return None;
        }

        let support = match qargs {
            [qubit] => self.resolve_native_instruction(
                self.qubit_properties
                    .get(qubit)
                    .map(QubitProp::native_instructions),
                instruction,
            ),
            [control, target] => self.resolve_native_instruction(
                self.edge_properties(*control, *target)
                    .map(EdgeProp::native_instructions),
                instruction,
            ),
            _ => NativeInstructionSupport::Inherited,
        };

        match support {
            NativeInstructionSupport::Explicit(prop) => Some(NativeInstructionCalibration {
                error_rate: Some(prop.error_rate()),
                duration: prop.length(),
            }),
            NativeInstructionSupport::Inherited => Some(NativeInstructionCalibration {
                error_rate: match qargs.len() {
                    1 => self.default_single_qubit_error,
                    2 => self.default_two_qubit_error,
                    _ => None,
                },
                duration: None,
            }),
            NativeInstructionSupport::Unsupported => None,
        }
    }

    /// Gets a direction-specific coupling error suitable for routing costs.
    ///
    /// Returns the best available native two-qubit instruction error on the
    /// edge, or the default two-qubit error if no per-edge calibration exists.
    ///
    /// This instruction-agnostic calibration query does not prove that any
    /// two-qubit instruction is supported, even when it returns `Some`. It
    /// must not be used as a capability predicate. Use
    /// [`Self::supports_native_instruction`] or [`Self::two_qubit_error`] when
    /// a concrete instruction capability matters.
    pub fn edge_error(&self, control: PhysicalQubit, target: PhysicalQubit) -> Option<f64> {
        if !self.is_usable_qubit(control)
            || !self.is_usable_qubit(target)
            || !self.topology.supports_directed_coupling(control, target)
        {
            return None;
        }

        self.edge_properties(control, target)
            .and_then(|props| {
                props
                    .native_instructions()
                    .iter()
                    .map(InstructionProp::error_rate)
                    .min_by(|a, b| a.total_cmp(b))
            })
            .or(self.default_two_qubit_error)
    }

    /// Gets the T1 relaxation time for a qubit (μs).
    ///
    /// Falls back to the default T1 time if not specified for the qubit.
    pub fn get_t1(&self, qubit: PhysicalQubit) -> Option<f64> {
        self.qubit_properties
            .get(&qubit)
            .and_then(|p| p.t1)
            .or(self.default_t1)
    }

    /// Gets the T2 dephasing time for a qubit (μs).
    ///
    /// Falls back to the default T2 time if not specified for the qubit.
    pub fn get_t2(&self, qubit: PhysicalQubit) -> Option<f64> {
        self.qubit_properties
            .get(&qubit)
            .and_then(|p| p.t2)
            .or(self.default_t2)
    }

    /// Gets the readout error rate for a qubit.
    ///
    /// Falls back to the default readout error if not specified for the qubit.
    pub fn get_readout_error(&self, qubit: PhysicalQubit) -> Option<f64> {
        self.qubit_properties
            .get(&qubit)
            .map(|p| p.readout_error)
            .or(self.default_readout_error)
    }

    /// Gets the default single-qubit gate error rate.
    pub fn default_single_qubit_error(&self) -> Option<f64> {
        self.default_single_qubit_error
    }

    /// Gets the default two-qubit gate error rate.
    pub fn default_two_qubit_error(&self) -> Option<f64> {
        self.default_two_qubit_error
    }

    /// Gets the default T1 relaxation time.
    pub fn default_t1(&self) -> Option<f64> {
        self.default_t1
    }

    /// Gets the default T2 dephasing time.
    pub fn default_t2(&self) -> Option<f64> {
        self.default_t2
    }

    /// Gets the default readout error rate.
    pub fn default_readout_error(&self) -> Option<f64> {
        self.default_readout_error
    }

    /// Gets the system calibration timestamp.
    pub fn calibration_time(&self) -> Option<OffsetDateTime> {
        self.calibration_time
    }
}

#[cfg(test)]
#[path = "./device_test.rs"]
mod device_test;
