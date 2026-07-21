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

//! Python bindings for OpenQASM 3.0 format.
//!
//! This module provides functions to parse and serialize OpenQASM 3.0 programs.

use crate::circuit::PyCircuit;
use cqlib_core::ir::{
    Qasm3DumpOptions, qasm3_dump_with_options, qasm3_dumps_with_options, qasm3_load, qasm3_loads,
};
use cqlib_core::ir::{qasm3::dump::Qasm3DumpError, qasm3::load::Qasm3ParseError};
use pyo3::prelude::*;

fn dump_options(qubit_mode: &str) -> PyResult<Qasm3DumpOptions> {
    match qubit_mode {
        "auto" => Ok(Qasm3DumpOptions::auto()),
        "logical" => Ok(Qasm3DumpOptions::logical()),
        "physical" => Ok(Qasm3DumpOptions::physical()),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "qubit_mode must be 'auto', 'logical', or 'physical'",
        )),
    }
}

/// Parse OpenQASM 3.0 source string into a Circuit.
///
/// # Arguments
/// * `qasm` - OpenQASM 3.0 source code string
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails or the source uses unsupported
/// OpenQASM 3.0 features.
#[pyfunction(name = "loads")]
pub fn py_qasm3_loads(qasm: &str) -> PyResult<PyCircuit> {
    match qasm3_loads(qasm) {
        Ok(circuit) => Ok(PyCircuit { inner: circuit }),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM3 parse error: {}",
            e
        ))),
    }
}

/// Load and parse an OpenQASM 3.0 file into a Circuit.
///
/// # Arguments
/// * `path` - Path to the OpenQASM 3.0 file
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails or `OSError` if file cannot be read.
#[pyfunction(name = "load")]
pub fn py_qasm3_load(path: &str) -> PyResult<PyCircuit> {
    match qasm3_load(path) {
        Ok(circuit) => Ok(PyCircuit { inner: circuit }),
        Err(Qasm3ParseError::IoError(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
            format!("QASM3 load error: {}", e),
        )),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM3 load error: {}",
            e
        ))),
    }
}

/// Serialize a Circuit to OpenQASM 3.0 string.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
///
/// # Returns
/// OpenQASM 3.0 source code string.
///
/// # Errors
/// Returns `ValueError` if the circuit contains instructions that cannot be
/// represented in OpenQASM 3.0.
#[pyfunction(name = "dumps", signature = (circuit, *, qubit_mode = "auto"))]
pub fn py_qasm3_dumps(circuit: &PyCircuit, qubit_mode: &str) -> PyResult<String> {
    let options = dump_options(qubit_mode)?;
    match qasm3_dumps_with_options(&circuit.inner, options) {
        Ok(qasm) => Ok(qasm),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM3 dump error: {}",
            e
        ))),
    }
}

/// Serialize a Circuit to an OpenQASM 3.0 file.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
/// * `path` - Output file path
///
/// # Errors
/// Returns `ValueError` if serialization fails, or `OSError` if the file cannot
/// be written.
#[pyfunction(name = "dump", signature = (circuit, path, *, qubit_mode = "auto"))]
pub fn py_qasm3_dump(circuit: &PyCircuit, path: &str, qubit_mode: &str) -> PyResult<()> {
    let options = dump_options(qubit_mode)?;
    match qasm3_dump_with_options(&circuit.inner, path, options) {
        Ok(()) => Ok(()),
        Err(Qasm3DumpError::IoError(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
            format!("QASM3 dump error: {}", e),
        )),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM3 dump error: {}",
            e
        ))),
    }
}
