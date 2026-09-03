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

//! Python bindings for the quantum device module.
//!
//! This module provides Python access to quantum hardware characterization data,
//! including device topology, qubit properties, noise models, and execution results.
//!
//! # Submodules
//!
//! - [`device_impl`]: Device definitions and hardware properties ([`PyDevice`], [`PyQubitProp`], etc.)
//! - [`topology`]: Qubit connectivity graphs ([`PyTopology`])
//! - [`layout`]: Logical-to-physical qubit mappings ([`PyLayout`])
//! - [`qubit`]: Strongly typed qubit identifiers ([`PyLogicalQubit`], [`PyPhysicalQubit`])
//! - [`noise`]: Noise models for quantum operations ([`PyNoiseModel`], [`PySingleQubitNoise`], etc.)
//! - [`result`]: Execution results and measurement outcomes ([`PyExecutionResult`], [`PyOutcome`])
//!
//! # Example
//!
//! ```python
//! from cqlib.device import Device, Topology, QubitProp
//!
//! # Create a device topology
//! topology = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])
//!
//! # Initialize a device with calibration data
//! device = Device("superconducting_qpu", [0, 1, 2], topology)
//! device.set_calibration_time(datetime.now(timezone.utc))
//!
//! # Set qubit properties
//! prop = QubitProp(readout_error=0.01)
//! prop.t1 = 50.0  # microseconds
//! prop.t2 = 25.0
//! device.add_qubit_properties(0, prop)
//! ```

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub mod device_impl;
pub mod layout;
pub mod noise;
pub mod qubit;
pub mod result;
pub mod topology;

pub(crate) fn validate_probability(value: f64, name: &str) -> PyResult<f64> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(PyValueError::new_err(format!(
            "{name} must be finite and in [0, 1], got {value}"
        )))
    }
}

pub(crate) fn validate_positive(value: f64, name: &str) -> PyResult<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(PyValueError::new_err(format!(
            "{name} must be finite and greater than 0, got {value}"
        )))
    }
}

pub(crate) fn validate_nonnegative(value: f64, name: &str) -> PyResult<f64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(PyValueError::new_err(format!(
            "{name} must be finite and non-negative, got {value}"
        )))
    }
}

/// Registers all device-related classes with the Python module.
///
/// This function adds the following classes to the `cqlib.device` submodule:
/// - [`PyLogicalQubit`]: Logical qubit identifier for circuit wires
/// - [`PyPhysicalQubit`]: Physical qubit identifier for hardware positions
/// - [`PyInstructionProp`]: Gate calibration data (error rates, duration)
/// - [`PyQubitProp`]: Single-qubit properties (T1, T2, readout error)
/// - [`PyEdgeProp`]: Coupling edge properties
/// - [`PyDevice`]: Complete device characterization
/// - [`PyLayout`]: Logical-to-physical qubit mapping
/// - [`PyTopology`]: Qubit connectivity graph
/// - [`PySingleQubitNoise`]: Single-qubit noise channels
/// - [`PyTwoQubitNoise`]: Two-qubit noise channels
/// - [`PyReadoutError`]: Measurement error probabilities
/// - [`PyOperationKey`]: Noise model lookup keys
/// - [`PyNoiseModel`]: Complete noise model
/// - [`PyOutcome`]: Measurement outcome bitstrings
/// - [`PyStatus`]: Job execution status
/// - [`PyExecutionResult`]: Full execution results
///
/// # Errors
///
/// Returns `PyResult::Err` if any class registration fails (e.g., due to
/// naming conflicts or Python interpreter issues).
///
/// # Example
///
/// ```rust,no_run
/// use pyo3::prelude::*;
/// use _native::device::register_device_module;
///
/// Python::try_attach(|py| {
///     let module = PyModule::new(py, "cqlib").unwrap();
///     register_device_module(&module).unwrap();
/// });
/// ```
pub fn register_device_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let device_module = PyModule::new(parent.py(), "device")?;

    // Strongly typed qubit identifiers
    device_module.add_class::<qubit::PyLogicalQubit>()?;
    device_module.add_class::<qubit::PyPhysicalQubit>()?;

    // Hardware characterization classes
    device_module.add_class::<device_impl::PyInstructionProp>()?;
    device_module.add_class::<device_impl::PyQubitProp>()?;
    device_module.add_class::<device_impl::PyEdgeProp>()?;
    device_module.add_class::<device_impl::PyDevice>()?;

    // Topology and layout classes
    device_module.add_class::<layout::PyLayout>()?;
    device_module.add_class::<topology::PyTopology>()?;

    // Noise model classes
    device_module.add_class::<noise::PySingleQubitNoise>()?;
    device_module.add_class::<noise::PyTwoQubitNoise>()?;
    device_module.add_class::<noise::PyReadoutError>()?;
    device_module.add_class::<noise::PyOperationKey>()?;
    device_module.add_class::<noise::PyNoiseModel>()?;

    // Execution result classes
    device_module.add_class::<result::PyOutcome>()?;
    device_module.add_class::<result::PyStatus>()?;
    device_module.add_class::<result::PyExecutionResult>()?;

    parent.add_submodule(&device_module)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("cqlib._native.device", &device_module)?;
    Ok(())
}
