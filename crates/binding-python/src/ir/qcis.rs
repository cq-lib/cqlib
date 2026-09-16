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

//! Python bindings for QCIS format.
//!
//! QCIS (Quantum Circuit Intermediate Representation) is Telecom Quantum's
//! native quantum circuit format.
//!
//! # Format
//!
//! Each line in QCIS represents a quantum operation:
//! ```text
//! GATE_NAME QUBIT_LIST [PARAMETER_LIST]
//! ```
//!
//! All cqlib standard gates except identity and global phase are supported.
//! QCIS `I Qn t` represents a delay, not a standard identity gate.
//!
//! # Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | `loads` | Parse QCIS source string into a Circuit |
//! | `load` | Load and parse a QCIS file into a Circuit |
//! | `dumps` | Serialize a Circuit to QCIS string |
//! | `dump` | Serialize a Circuit to a QCIS file |
//!
//! # Example
//!
//! ```python
//! from cqlib.ir import qcis
//! from cqlib import Circuit, Qubit
//!
//! # Parse QCIS
//! qcis_code = """H Q0
//! CZ Q0 Q1
//! M Q0 Q1"""
//! circuit = qcis.loads(qcis_code)
//!
//! # Serialize back to QCIS
//! output = qcis.dumps(circuit)
//! ```

use crate::circuit::PyCircuit;
use cqlib_core::ir::qcis::dump::QcisDumpError;
use cqlib_core::ir::qcis::load::QcisParseError;
use cqlib_core::ir::{qcis_dump, qcis_dumps, qcis_load, qcis_loads};
use pyo3::prelude::*;

/// Parse QCIS source string into a Circuit.
///
/// QCIS (Quantum Circuit Intermediate Representation) is Telecom Quantum's
/// native quantum circuit format. Each line represents a gate operation:
/// `GATE_NAME QUBIT_LIST [PARAMETER_LIST]`
///
/// # Arguments
/// * `qcis` - QCIS source code string
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails (invalid qubit format, unknown gate, etc.).
///
/// # Example
/// ```python
/// from cqlib.ir import qcis
///
/// qcis_code = '''
/// H Q0
/// CZ Q0 Q1
/// RZ Q0 3.14159
/// M Q0 Q1
/// '''
/// circuit = qcis.loads(qcis_code)
/// ```
#[pyfunction(name = "loads")]
pub fn py_qcis_loads(qcis: &str) -> PyResult<PyCircuit> {
    match qcis_loads(qcis) {
        Ok(c) => Ok(PyCircuit::from(c)),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QCIS parse error: {}",
            e
        ))),
    }
}

/// Load and parse a QCIS file into a Circuit.
///
/// # Arguments
/// * `path` - Path to the QCIS file
///
/// # Returns
/// A `Circuit` object representing the parsed circuit.
///
/// # Errors
/// Returns `ValueError` if parsing fails or `OSError` if file cannot be read.
///
/// # Example
/// ```python
/// from cqlib.ir import qcis
///
/// circuit = qcis.load("/path/to/circuit.qcis")
/// ```
#[pyfunction(name = "load")]
pub fn py_qcis_load(path: &str) -> PyResult<PyCircuit> {
    match qcis_load(path) {
        Ok(c) => Ok(PyCircuit::from(c)),
        Err(QcisParseError::IoError(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
            format!("QCIS load error: {}", e),
        )),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "QCIS load error: {}",
            e
        ))),
    }
}

/// Serialize a Circuit to QCIS string.
///
/// All cqlib standard gates except identity and global phase can be serialized.
/// Custom, unitary, multi-controlled, and control-flow instructions are not
/// represented by QCIS.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
///
/// # Returns
/// QCIS source code string.
///
/// # Errors
/// Returns `ValueError` if the circuit contains instructions not represented by
/// QCIS (e.g., multi-controlled gates or custom unitary gates).
///
/// # Example
/// ```python
/// from cqlib.ir import qcis
/// from cqlib import Circuit, Qubit
///
/// circuit = Circuit(2)
/// circuit.h(Qubit(0))
/// circuit.cz(Qubit(0), Qubit(1))
///
/// qcis_str = qcis.dumps(circuit)
/// print(qcis_str)  # H Q0\nCZ Q0 Q1\n
/// ```
#[pyfunction(name = "dumps")]
pub fn py_qcis_dumps(circuit: &PyCircuit) -> PyResult<String> {
    qcis_dumps(&circuit.inner).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("QCIS dump error: {}", e))
    })
}

/// Serialize a Circuit to a QCIS file.
///
/// # Arguments
/// * `circuit` - The circuit to serialize
/// * `path` - Output file path
///
/// # Errors
/// Returns `ValueError` if the circuit contains unsupported gates,
/// or `OSError` if the file cannot be written.
///
/// # Example
/// ```python
/// from cqlib.ir import qcis
/// from cqlib import Circuit, Qubit
///
/// circuit = Circuit(2)
/// circuit.h(Qubit(0))
/// circuit.cz(Qubit(0), Qubit(1))
///
/// qcis.dump(circuit, "output.qcis")
/// ```
#[pyfunction(name = "dump")]
pub fn py_qcis_dump(circuit: &PyCircuit, path: &str) -> PyResult<()> {
    let path_buf = std::path::PathBuf::from(path);
    qcis_dump(&circuit.inner, &path_buf).map_err(|e| match e {
        QcisDumpError::IoError(_) => {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("QCIS dump error: {e}"))
        }
        _ => PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("QCIS dump error: {e}")),
    })
}
