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

//! QCIS Parser Module
//!
//! This module provides functionality to parse QCIS (Quantum Circuit Intermediate Representation)
//! format into the internal `Circuit` representation.
//!
//! ## QCIS Format
//!
//! Each line in QCIS represents a quantum operation with the format:
//! ```text
//! GATE_NAME QUBIT_LIST [PARAMETER_LIST]
//! ```
//!
//! - **QUBIT_LIST**: Space-separated list of qubits in `Q<id>` format (e.g., `Q0`, `Q1`)
//! - **PARAMETER_LIST**: Optional space-separated parameters (numeric values or expressions)
//!   except QCIS delay `I Qn t`, whose `t` must be a fixed non-negative integer
//!   count of 0.5 ns ticks
//!
//! ## Examples
//!
//! ```rust
//! use cqlib_core::ir::qcis::loads;
//!
//! // Single-qubit gate
//! let circuit = loads("H Q0").unwrap();
//!
//! // Two-qubit gate
//! let circuit = loads("CZ Q0 Q1").unwrap();
//!
//! // Parametrized gate
//! let circuit = loads("RX Q0 1.57").unwrap();
//! ```
//!
//! ## Comments and Whitespace
//!
//! - Lines starting with `//` are treated as comments and ignored
//! - Empty lines are ignored
//! - Extra whitespace is normalized

use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::gate::Instruction;
use crate::circuit::{Circuit, Parameter, Qubit, StandardGate};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use thiserror::Error;

static QUBIT_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Q\d+$").unwrap());

/// Errors that can occur during QCIS parsing.
#[derive(Debug, Error)]
pub enum QcisParseError {
    /// File system or I/O error (file not found, permission denied, etc.)
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid qubit format (e.g., not "Q123" format)
    #[error("Invalid qubit format: '{0}' (expected format: Q<id>, e.g., Q0, Q1)")]
    InvalidQubitFormat(String),

    /// Invalid qubit ID (failed to parse the number after 'Q')
    #[error("Invalid qubit ID: '{0}'")]
    InvalidQubitId(String),

    /// Qubit count mismatch for a gate
    #[error("Qubit count mismatch for gate '{gate}': expected {expected}, got {actual}")]
    QubitCountMismatch {
        gate: String,
        expected: usize,
        actual: usize,
    },

    /// Parameter count mismatch for a gate
    #[error("Parameter count mismatch for gate '{gate}': expected {expected}, got {actual}")]
    ParameterCountMismatch {
        gate: String,
        expected: usize,
        actual: usize,
    },

    /// Missing required parameter for a gate
    #[error("Missing required parameter(s) for gate '{0}'")]
    MissingParameter(String),

    /// Failed to parse a parameter expression
    #[error("Failed to parse parameter '{param}' for gate '{gate}': {reason}")]
    InvalidParameter {
        gate: String,
        param: String,
        reason: String,
    },

    /// Unknown gate name
    #[error("Unknown gate: '{0}'")]
    UnknownGate(String),

    /// Empty line or no valid content
    #[error("Empty line or no valid content")]
    EmptyLine,
}

impl PartialEq for QcisParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(lhs), Self::IoError(rhs)) => {
                lhs.kind() == rhs.kind() && lhs.to_string() == rhs.to_string()
            }
            (Self::InvalidQubitFormat(lhs), Self::InvalidQubitFormat(rhs)) => lhs == rhs,
            (Self::InvalidQubitId(lhs), Self::InvalidQubitId(rhs)) => lhs == rhs,
            (
                Self::QubitCountMismatch {
                    gate: lhs_gate,
                    expected: lhs_expected,
                    actual: lhs_actual,
                },
                Self::QubitCountMismatch {
                    gate: rhs_gate,
                    expected: rhs_expected,
                    actual: rhs_actual,
                },
            ) => lhs_gate == rhs_gate && lhs_expected == rhs_expected && lhs_actual == rhs_actual,
            (
                Self::ParameterCountMismatch {
                    gate: lhs_gate,
                    expected: lhs_expected,
                    actual: lhs_actual,
                },
                Self::ParameterCountMismatch {
                    gate: rhs_gate,
                    expected: rhs_expected,
                    actual: rhs_actual,
                },
            ) => lhs_gate == rhs_gate && lhs_expected == rhs_expected && lhs_actual == rhs_actual,
            (Self::MissingParameter(lhs), Self::MissingParameter(rhs)) => lhs == rhs,
            (
                Self::InvalidParameter {
                    gate: lhs_gate,
                    param: lhs_param,
                    reason: lhs_reason,
                },
                Self::InvalidParameter {
                    gate: rhs_gate,
                    param: rhs_param,
                    reason: rhs_reason,
                },
            ) => lhs_gate == rhs_gate && lhs_param == rhs_param && lhs_reason == rhs_reason,
            (Self::UnknownGate(lhs), Self::UnknownGate(rhs)) => lhs == rhs,
            (Self::EmptyLine, Self::EmptyLine) => true,
            _ => false,
        }
    }
}

/// Result type for QCIS parsing operations.
pub type Result<T> = std::result::Result<T, QcisParseError>;

/// Gate specification defining required qubit and parameter counts.
///
/// Used to validate gate invocations against the QCIS specification.
#[derive(Debug, Clone, Copy)]
struct GateSpec {
    min_qubits: usize,
    max_qubits: usize,
    min_params: usize,
    max_params: usize,
}

impl GateSpec {
    const fn new(
        min_qubits: usize,
        max_qubits: usize,
        min_params: usize,
        max_params: usize,
    ) -> Self {
        Self {
            min_qubits,
            max_qubits,
            min_params,
            max_params,
        }
    }

    /// Expects exact qubit and parameter counts.
    const fn exact(qubits: usize, params: usize) -> Self {
        Self::new(qubits, qubits, params, params)
    }

    /// Variable qubit count (at least min), exact parameters.
    const fn min_qubits(min_qubits: usize, params: usize) -> Self {
        Self::new(min_qubits, usize::MAX, params, params)
    }
}

/// Get the gate specification for a given gate name.
fn get_gate_spec(gate_name: &str) -> Option<GateSpec> {
    if let Some(gate) = standard_gate_from_qcis(gate_name) {
        return Some(GateSpec::exact(gate.num_qubits(), gate.num_params()));
    }

    match gate_name {
        // Delay gate with time parameter
        "I" => Some(GateSpec::exact(1, 1)),
        "B" | "Barrier" => Some(GateSpec::min_qubits(1, 0)),
        "M" => Some(GateSpec::min_qubits(1, 0)),
        _ => None,
    }
}

/// Map a QCIS opcode to a cqlib standard gate.
///
/// QCIS `I` is reserved for delay, and `GPHASE` is intentionally not part of
/// the supported QCIS circuit format.
fn standard_gate_from_qcis(gate_name: &str) -> Option<StandardGate> {
    Some(match gate_name {
        "H" => StandardGate::H,
        "RX" => StandardGate::RX,
        "RXX" => StandardGate::RXX,
        "RXY" => StandardGate::RXY,
        "RY" => StandardGate::RY,
        "RYY" => StandardGate::RYY,
        "RZ" => StandardGate::RZ,
        "RZX" => StandardGate::RZX,
        "RZZ" => StandardGate::RZZ,
        "S" => StandardGate::S,
        "SD" | "SDG" => StandardGate::SDG,
        "SWAP" => StandardGate::SWAP,
        "T" => StandardGate::T,
        "TD" | "TDG" => StandardGate::TDG,
        "U" => StandardGate::U,
        "X" => StandardGate::X,
        "XY" => StandardGate::XY,
        "X2P" => StandardGate::X2P,
        "X2M" => StandardGate::X2M,
        "XY2P" => StandardGate::XY2P,
        "XY2M" => StandardGate::XY2M,
        "Y" => StandardGate::Y,
        "Y2P" => StandardGate::Y2P,
        "Y2M" => StandardGate::Y2M,
        "Z" => StandardGate::Z,
        "PHASE" => StandardGate::Phase,
        "CX" => StandardGate::CX,
        "CCX" => StandardGate::CCX,
        "CY" => StandardGate::CY,
        "CZ" => StandardGate::CZ,
        "CRX" => StandardGate::CRX,
        "CRY" => StandardGate::CRY,
        "CRZ" => StandardGate::CRZ,
        "FSIM" => StandardGate::FSIM,
        _ => return None,
    })
}

pub fn load<P: AsRef<Path>>(path: P) -> Result<Circuit> {
    let content = std::fs::read_to_string(path)?;
    loads(&content)
}

/// Parse a QCIS file and convert it to a [`Circuit`].
///
/// Rust-style alias for [`load`]. The Python-style `load` name is retained for
/// compatibility with the rest of the IR module API.
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Circuit> {
    load(path)
}

pub fn loads(qcis: &str) -> Result<Circuit> {
    let mut c = Circuit::new(0);
    // Maintain a set of existing qubit IDs to avoid repeated circuit.qubits() calls
    let mut existing_qubits: HashSet<u32> = HashSet::new();

    for line in qcis.lines() {
        process_line(line, &mut c, &mut existing_qubits)?
    }

    Ok(c)
}

/// Parse a QCIS source string and convert it to a [`Circuit`].
///
/// Rust-style alias for [`loads`].
pub fn from_str(qcis: &str) -> Result<Circuit> {
    loads(qcis)
}

/// Parse a parameter string into a ParameterValue.
/// Supports numbers, pi, e, and basic arithmetic operations (+, -, *, /).
fn parse_param(param_str: &str) -> Result<ParameterValue> {
    let param_str = param_str.trim();
    if param_str.is_empty() {
        return Err(QcisParseError::MissingParameter("".to_string()));
    }
    let p = Parameter::try_from(param_str);
    if let Ok(p) = p {
        if let Ok(v) = p.evaluate(&None) {
            Ok(ParameterValue::Fixed(v))
        } else {
            Ok(p.into())
        }
    } else {
        Err(QcisParseError::MissingParameter("".to_string()))
    }
}

/// Ensure the circuit has enough qubits for the given qubit IDs.
///
/// Uses the provided `existing_qubits` set for O(1) membership testing
/// instead of querying the circuit repeatedly.
fn ensure_qubits(c: &mut Circuit, qubits: &[Qubit], existing_qubits: &mut HashSet<u32>) {
    let mut missing = Vec::new();

    for q in qubits {
        let id = q.id();
        if existing_qubits.insert(id) {
            // insert returns true if the value was not already present
            missing.push(*q);
        }
    }

    if !missing.is_empty() {
        // Sort to ensure consistent ordering (though HashSet iteration order is arbitrary,
        // the qubit IDs themselves are what matter)
        missing.sort_by_key(|q| q.id());
        let _ = c.add_qubits(missing);
    }
}

/// Parse a qubit string (e.g., "Q0", "Q123") into a Qubit.
fn parse_qubit(s: &str) -> Result<Qubit> {
    if s.len() > 1 && s.starts_with('Q') {
        s[1..]
            .parse::<u32>()
            .map(Qubit::new)
            .map_err(|_| QcisParseError::InvalidQubitId(s.to_string()))
    } else {
        Err(QcisParseError::InvalidQubitFormat(s.to_string()))
    }
}

/// Validate qubit and parameter counts against gate specification.
fn validate_gate_args(gate_name: &str, qubits: &[Qubit], params: &[ParameterValue]) -> Result<()> {
    let spec = match get_gate_spec(gate_name) {
        Some(s) => s,
        None => {
            // Unknown gates are allowed without validation
            return Ok(());
        }
    };

    // Validate qubit count
    let qubit_count = qubits.len();
    if qubit_count < spec.min_qubits || qubit_count > spec.max_qubits {
        return Err(QcisParseError::QubitCountMismatch {
            gate: gate_name.to_string(),
            expected: spec.min_qubits,
            actual: qubit_count,
        });
    }

    // Validate parameter count
    let param_count = params.len();
    if param_count < spec.min_params || param_count > spec.max_params {
        return Err(QcisParseError::ParameterCountMismatch {
            gate: gate_name.to_string(),
            expected: spec.min_params,
            actual: param_count,
        });
    }

    Ok(())
}

fn validate_delay_tick(param: &ParameterValue, param_text: &str) -> Result<()> {
    match param {
        ParameterValue::Fixed(value)
            if value.is_finite() && *value >= 0.0 && value.fract().abs() < 1e-10 =>
        {
            Ok(())
        }
        ParameterValue::Fixed(_) => Err(QcisParseError::InvalidParameter {
            gate: "I".to_string(),
            param: param_text.to_string(),
            reason: "QCIS delay time must be a non-negative integer number of 0.5 ns ticks"
                .to_string(),
        }),
        ParameterValue::Param(_) => Err(QcisParseError::InvalidParameter {
            gate: "I".to_string(),
            param: param_text.to_string(),
            reason: "QCIS delay time must be a fixed non-negative integer number of 0.5 ns ticks"
                .to_string(),
        }),
    }
}

fn process_line(line: &str, c: &mut Circuit, existing_qubits: &mut HashSet<u32>) -> Result<()> {
    let clean_line = line.split("//").next().unwrap_or("").trim();
    if clean_line.is_empty() {
        return Ok(());
    }

    let parts = clean_line.split_whitespace().collect::<Vec<_>>();

    if parts.is_empty() {
        return Ok(());
    }

    // let qubit_pattern = Regex::new(r"^Q\d+$").unwrap();

    let gate_name = parts[0];
    let args = &parts[1..];

    // Validate qubit arguments first
    for &token in args.iter() {
        if (token.starts_with('Q') || token.starts_with('q')) && !QUBIT_PATTERN.is_match(token) {
            return Err(QcisParseError::InvalidQubitFormat(token.to_string()));
        }
    }

    let split_index = args
        .iter()
        .position(|&token| !QUBIT_PATTERN.is_match(token))
        .unwrap_or(args.len());

    let (qubit_slice, param_slice) = args.split_at(split_index);

    // Parse qubits with explicit error handling
    let qubits: Vec<Qubit> = qubit_slice
        .iter()
        .map(|&s| parse_qubit(s))
        .collect::<Result<Vec<_>>>()?;

    // Parse parameters
    let params: Vec<ParameterValue> = param_slice
        .iter()
        .map(|&s| parse_param(s))
        .collect::<Result<Vec<_>>>()?;

    // Validate qubit and parameter counts
    validate_gate_args(gate_name, &qubits, &params)?;
    if gate_name == "I" {
        validate_delay_tick(&params[0], param_slice[0])?;
    }

    // Ensure circuit has enough qubits (using cached set for efficiency)
    ensure_qubits(c, &qubits, existing_qubits);

    if let Some(gate) = standard_gate_from_qcis(gate_name) {
        c.append(Instruction::Standard(gate), qubits, params, None)
            .map_err(|e| QcisParseError::InvalidQubitId(format!("{:?}", e)))?;
        return Ok(());
    }

    match gate_name {
        "I" => c
            .delay(qubits[0], params[0].clone())
            .map_err(|e| QcisParseError::InvalidQubitId(format!("{:?}", e)))?,

        // Barrier
        "B" | "Barrier" => {
            if !qubits.is_empty() {
                c.barrier(qubits)
                    .map_err(|e| QcisParseError::InvalidQubitId(format!("{:?}", e)))?;
            }
        }

        // Measurement - supports 1 or more qubits
        "M" => {
            for q in qubits.iter() {
                c.measure(*q)
                    .map_err(|e| QcisParseError::InvalidQubitId(format!("{:?}", e)))?;
            }
        }

        _ => {
            // Unknown gate
            return Err(QcisParseError::UnknownGate(gate_name.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "./load_test.rs"]
mod load_test;
