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

//! # Device Error Types
//!
//! This module defines error types for quantum device operations.
//! It provides error handling for device topology validation,
//! layout mapping, and qubit connectivity issues.

use crate::device::{LogicalQubit, PhysicalQubit};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    InvalidOnlineQubit(PhysicalQubit),
    QubitNotInDevice(PhysicalQubit),
    QubitNotInTopology(PhysicalQubit),
    EdgeNotInTopology(PhysicalQubit, PhysicalQubit),
    InvalidTopology(TopologyError),
    /// A non-standard instruction was configured as an atomic native capability.
    NonStandardNativeInstruction {
        /// Human-readable instruction identity.
        instruction: String,
    },
    /// A standard gate's arity cannot be represented at the target location.
    InvalidNativeInstructionArity {
        /// Human-readable instruction identity.
        instruction: String,
        /// Inclusive arity range accepted by the writer.
        expected: std::ops::RangeInclusive<usize>,
        /// Number of qubits used by the standard gate.
        actual: usize,
    },
    /// A native instruction error rate is not a finite probability.
    InvalidNativeInstructionErrorRate {
        /// Human-readable instruction identity.
        instruction: String,
        /// Stable textual representation of the rejected value.
        value: String,
    },
    /// A native instruction duration is negative or non-finite.
    InvalidNativeInstructionDuration {
        /// Human-readable instruction identity.
        instruction: String,
        /// Stable textual representation of the rejected value.
        value: String,
    },
}

/// Errors raised when a physical circuit does not satisfy device capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceValidationError {
    /// An operation references a physical qubit that is absent or offline.
    UnusablePhysicalQubit {
        /// Device name used for diagnostics.
        device: String,
        /// Physical qubit that is absent or offline.
        qubit: PhysicalQubit,
    },
    /// A two-qubit instruction has no coupling in the requested direction.
    MissingDirectedCoupling {
        /// Device name used for diagnostics.
        device: String,
        /// Human-readable instruction identity.
        instruction: String,
        /// Requested control-side physical qubit.
        control: PhysicalQubit,
        /// Requested target-side physical qubit.
        target: PhysicalQubit,
    },
    /// The device does not support an instruction on the exact ordered qargs.
    UnsupportedInstruction {
        /// Device name used for diagnostics.
        device: String,
        /// Human-readable instruction identity.
        instruction: String,
        /// Ordered physical arguments requested by the operation.
        qargs: Vec<PhysicalQubit>,
    },
    /// A gate-like instruction remains above the device-native abstraction level.
    UndecomposedInstruction {
        /// Device name used for diagnostics.
        device: String,
        /// Human-readable instruction identity.
        instruction: String,
        /// Ordered physical arguments requested by the operation.
        qargs: Vec<PhysicalQubit>,
    },
}

impl fmt::Display for DeviceValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnusablePhysicalQubit { device, qubit } => write!(
                f,
                "device '{device}' cannot execute an operation on unusable physical qubit {qubit}"
            ),
            Self::UnsupportedInstruction {
                device,
                instruction,
                qargs,
            } => {
                let qargs = qargs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "device '{device}' does not support instruction {instruction} on ordered physical qargs [{qargs}]"
                )
            }
            Self::MissingDirectedCoupling {
                device,
                instruction,
                control,
                target,
            } => write!(
                f,
                "device '{device}' has no directed coupling {control} -> {target} required by instruction {instruction}"
            ),
            Self::UndecomposedInstruction {
                device,
                instruction,
                qargs,
            } => {
                let qargs = qargs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "device '{device}' cannot validate undecomposed instruction {instruction} on ordered physical qargs [{qargs}]"
                )
            }
        }
    }
}

impl std::error::Error for DeviceValidationError {}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOnlineQubit(q) => {
                write!(
                    f,
                    "Specified online qubit {} does not exist in the device topology",
                    q
                )
            }
            Self::QubitNotInDevice(q) => {
                write!(f, "Qubit {} is not registered with the device", q)
            }
            Self::QubitNotInTopology(q) => {
                write!(f, "Qubit {} is not in the device topology", q)
            }
            Self::EdgeNotInTopology(control, target) => {
                write!(
                    f,
                    "Edge ({}, {}) is not in the device topology",
                    control, target
                )
            }
            Self::InvalidTopology(error) => {
                write!(f, "Invalid device topology: {error}")
            }
            Self::NonStandardNativeInstruction { instruction } => write!(
                f,
                "instruction {instruction} cannot be stored as a native capability: native instructions must be standard gates"
            ),
            Self::InvalidNativeInstructionArity {
                instruction,
                expected,
                actual,
            } => {
                let expected = if expected.start() == expected.end() {
                    expected.start().to_string()
                } else {
                    format!("{}..={}", expected.start(), expected.end())
                };
                write!(
                    f,
                    "instruction {instruction} cannot be stored as a native capability: expected {expected} qubits, got {actual}"
                )
            }
            Self::InvalidNativeInstructionErrorRate { instruction, value } => write!(
                f,
                "instruction {instruction} has invalid native error rate {value}: expected a finite value in [0, 1]"
            ),
            Self::InvalidNativeInstructionDuration { instruction, value } => write!(
                f,
                "instruction {instruction} has invalid native duration {value}: expected a finite non-negative value"
            ),
        }
    }
}

/// Errors that can occur when creating or operating on a [`crate::device::Layout`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The number of logical qubits exceeds the number of physical qubits.
    TooManyLogicalQubits { logical: usize, physical: usize },
    /// A logical qubit appears more than once in the logical qubit list.
    DuplicateLogicalQubit(LogicalQubit),
    /// A physical qubit appears more than once in the physical qubit list.
    DuplicatePhysicalQubit(PhysicalQubit),
    /// A logical qubit in `init_map` is not present in the logical qubit list.
    InvalidLogicalQubit(LogicalQubit),
    /// A physical qubit is not available to the layout.
    InvalidPhysicalQubit(PhysicalQubit),
    /// Tried to bind a logical qubit that is already mapped.
    LogicalQubitAlreadyBound(LogicalQubit),
    /// Tried to bind a physical qubit that already carries a logical qubit.
    PhysicalQubitAlreadyOccupied(PhysicalQubit),
    /// Tried to unbind a logical qubit that is not mapped.
    LogicalQubitNotBound(LogicalQubit),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyLogicalQubits { logical, physical } => {
                write!(
                    f,
                    "Logical qubits ({}) exceed physical qubits ({})",
                    logical, physical
                )
            }
            Self::DuplicateLogicalQubit(q) => {
                write!(f, "Logical qubit {} appears more than once", q)
            }
            Self::DuplicatePhysicalQubit(q) => {
                write!(f, "Physical qubit {} appears more than once", q)
            }
            Self::InvalidLogicalQubit(q) => {
                write!(f, "Logical qubit {} not in logical qubit list", q)
            }
            Self::InvalidPhysicalQubit(q) => {
                write!(f, "Physical qubit {} not in physical qubit list", q)
            }
            Self::LogicalQubitAlreadyBound(q) => {
                write!(f, "Logical qubit {} is already bound", q)
            }
            Self::PhysicalQubitAlreadyOccupied(q) => {
                write!(f, "Physical qubit {} is already occupied", q)
            }
            Self::LogicalQubitNotBound(q) => write!(f, "Logical qubit {} is not bound", q),
        }
    }
}

/// Errors that can occur when operating on a Topology.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// Tried to operate on a qubit that doesn't exist in the topology.
    QubitNotFound(PhysicalQubit),
    /// Tried to operate on a coupling edge that doesn't exist.
    CouplingNotFound {
        control: PhysicalQubit,
        target: PhysicalQubit,
    },
    /// Tried to add a duplicate qubit.
    QubitAlreadyExists(PhysicalQubit),
    /// Tried to add a duplicate coupling.
    CouplingAlreadyExists {
        control: PhysicalQubit,
        target: PhysicalQubit,
    },
    /// Tried to add a coupling from a qubit to itself.
    SelfCoupling { qubit: PhysicalQubit },
    /// Tried to remove the same qubit more than once in one operation.
    DuplicateQubitRemoval(PhysicalQubit),
    /// Tried to remove the same coupling more than once in one operation.
    DuplicateCouplingRemoval {
        control: PhysicalQubit,
        target: PhysicalQubit,
    },
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyError::QubitNotFound(q) => write!(f, "Qubit {:?} not found", q),
            TopologyError::CouplingNotFound { control, target } => {
                write!(f, "Coupling ({:?} -> {:?}) not found", control, target)
            }
            TopologyError::QubitAlreadyExists(q) => write!(f, "Qubit {:?} already exists", q),
            TopologyError::CouplingAlreadyExists { control, target } => {
                write!(f, "Coupling ({:?} -> {:?}) already exists", control, target)
            }
            TopologyError::SelfCoupling { qubit } => {
                write!(
                    f,
                    "Self coupling ({:?} -> {:?}) is not allowed",
                    qubit, qubit
                )
            }
            TopologyError::DuplicateQubitRemoval(q) => {
                write!(f, "Qubit {:?} appears more than once in removal request", q)
            }
            TopologyError::DuplicateCouplingRemoval { control, target } => {
                write!(
                    f,
                    "Coupling ({:?} -> {:?}) appears more than once in removal request",
                    control, target
                )
            }
        }
    }
}
