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

//! Python bindings for OpenQASM 2.0 format.
//!
//! This module provides functions to parse and serialize OpenQASM 2.0 programs.
//!
//! # Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | `loads` | Parse OpenQASM 2.0 source string into a Circuit |
//! | `load` | Load and parse an OpenQASM 2.0 file into a Circuit |
//! | `dumps` | Serialize a Circuit to OpenQASM 2.0 string |
//! | `dump` | Serialize a Circuit to an OpenQASM 2.0 file |
//!
//! # Example
//!
//! ```python
//! from cqlib.ir import qasm2
//! from cqlib import Circuit, Qubit
//!
//! # Parse OpenQASM 2.0
//! qasm_code = """OPENQASM 2.0;
//! include "qelib1.inc";
//! qreg q[2];
//! h q[0];
//! cx q[0], q[1];
//! """
//! circuit = qasm2.loads(qasm_code)
//!
//! # Serialize back to OpenQASM
//! output = qasm2.dumps(circuit)
//! ```

use crate::circuit::PyCircuit;
use cqlib_core::ir::qasm2::dump::QasmDumpError;
use cqlib_core::ir::qasm2::load::QasmParseError;
use cqlib_core::ir::{qasm2_dump, qasm2_dumps, qasm2_load, qasm2_loads};
use pyo3::prelude::*;

/// Parse OpenQASM 2.0 source string into a Circuit.
///
/// # Arguments
/// * `qasm` - OpenQASM 2.0 source code string
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails (syntax error, unknown gate, etc.).
///
/// # Example
/// ```python
/// from cqlib.ir import qasm2
///
/// circuit = qasm2.loads('''
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[1];
///     h q[0];
/// ''')
/// ```
#[pyfunction(name = "loads")]
pub fn py_qasm2_loads(qasm: &str) -> PyResult<PyCircuit> {
    match qasm2_loads(qasm) {
        Ok(circuit) => Ok(PyCircuit::from(circuit)),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM parse error: {}",
            e
        ))),
    }
}

/// Load and parse an OpenQASM 2.0 file into a Circuit.
///
/// # Arguments
/// * `path` - Path to the OpenQASM 2.0 file
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails or `OSError` if file cannot be read.
///
/// # Example
/// ```python
/// from cqlib.ir import qasm2
///
/// circuit = qasm2.load("/path/to/circuit.qasm")
/// ```
#[pyfunction(name = "load")]
pub fn py_qasm2_load(path: &str) -> PyResult<PyCircuit> {
    match qasm2_load(path) {
        Ok(circuit) => Ok(PyCircuit::from(circuit)),
        Err(QasmParseError::IoError(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
            format!("QASM load error: {}", e),
        )),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM load error: {}",
            e
        ))),
    }
}

/// Serialize a Circuit to OpenQASM 2.0 string.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
///
/// # Returns
/// OpenQASM 2.0 source code string.
///
/// # Errors
/// Returns `ValueError` if the circuit contains gates that cannot be
/// represented in OpenQASM 2.0 (e.g., some custom gates).
///
/// # Example
/// ```python
/// from cqlib.ir import qasm2
/// from cqlib import Circuit, Qubit
///
/// circuit = Circuit(2)
/// circuit.h(Qubit(0))
/// circuit.cz(Qubit(0), Qubit(1))
///
/// qasm_str = qasm2.dumps(circuit)
/// print(qasm_str)
/// ```
#[pyfunction(name = "dumps")]
pub fn py_qasm2_dumps(circuit: &PyCircuit) -> PyResult<String> {
    match qasm2_dumps(&circuit.inner) {
        Ok(qasm) => Ok(qasm),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM dump error: {}",
            e
        ))),
    }
}

/// Serialize a Circuit to an OpenQASM 2.0 file.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
/// * `path` - Output file path
///
/// # Errors
/// Returns `ValueError` if serialization fails, or `OSError` if the file
/// cannot be written.
///
/// # Example
/// ```python
/// from cqlib.ir import qasm2
/// from cqlib import Circuit, Qubit
///
/// circuit = Circuit(2)
/// circuit.h(Qubit(0))
/// circuit.cx(Qubit(0), Qubit(1))
///
/// qasm2.dump(circuit, "output.qasm")
/// ```
#[pyfunction(name = "dump")]
pub fn py_qasm2_dump(circuit: &PyCircuit, path: &str) -> PyResult<()> {
    match qasm2_dump(&circuit.inner, path) {
        Ok(()) => Ok(()),
        Err(QasmDumpError::IoError(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
            format!("QASM dump error: {}", e),
        )),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QASM dump error: {}",
            e
        ))),
    }
}
