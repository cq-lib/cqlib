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

//! # Quantum Circuit Intermediate Representation (IR)
//!
//! This module provides parsers and serializers for various quantum circuit formats,
//! enabling interoperability between Cqlib and other quantum computing tools.
//!
//! ## Supported Formats
//!
//! | Format | Description | Load Module | Dump Module |
//! |--------|-------------|--------------|-------------|
//! | OpenQASM 2.0 | IBM's quantum assembly language | [`qasm2::load`] | [`qasm2::dump`] |
//! | OpenQASM 3.0 | OpenQASM 3 circuit language | [`qasm3::load`] | [`qasm3::dump`] |
//! | QCIS | Telecom Quantum's intermediate format | [`qcis::load`] | [`qcis::dump`] |
//!
//! ## Error Types
//!
//! Each format keeps concrete error types so callers can match format-specific
//! failures without downcasting:
//!
//! | Format | Load Error | Dump Error |
//! |--------|------------|------------|
//! | OpenQASM 2.0 | [`qasm2::load::QasmParseError`] | [`qasm2::dump::QasmDumpError`] |
//! | OpenQASM 3.0 | [`qasm3::load::Qasm3ParseError`] | [`qasm3::dump::Qasm3DumpError`] |
//! | QCIS | [`qcis::load::QcisParseError`] | [`qcis::dump::QcisDumpError`] |
//!
//! File-based entry points preserve the original I/O error through
//! [`std::error::Error::source`]. The crate does not currently expose a shared
//! `Format` trait or umbrella IR error enum; use the explicit per-format modules
//! or crate-root aliases below.
//!
//! ## Quick Start
//!
//! ### Loading from OpenQASM
//!
//! ```rust
//! use cqlib_core::ir::qasm2_loads;
//!
//! let qasm = r#"
//!     OPENQASM 2.0;
//!     include "qelib1.inc";
//!     qreg q[2];
//!     h q[0];
//!     cx q[0], q[1];
//! "#;
//!
//! let circuit = qasm2_loads(qasm).unwrap();
//! ```
//!
//! ### Loading from QCIS
//!
//! ```rust
//! use cqlib_core::ir::qcis_loads;
//!
//! let qcis = r#"
//! H Q0
//! CZ Q0 Q1
//! M Q0 Q1
//! "#;
//!
//! let circuit = qcis_loads(qcis).unwrap();
//! ```
//!
//! ### Dumping to Format
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::ir::{qasm2_dumps, qasm3_dumps, qcis_dumps};
//!
//! let mut circuit = Circuit::new(2);
//! circuit.h(Qubit::new(0)).unwrap();
//! circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
//!
//! let qasm = qasm2_dumps(&circuit).unwrap();
//! let qasm3 = qasm3_dumps(&circuit).unwrap();
//! let qcis = qcis_dumps(&circuit).unwrap();
//! ```

pub mod qasm2;
pub mod qasm3;
pub mod qcis;

pub use qasm2::dump::{dump as qasm2_dump, dumps as qasm2_dumps};
pub use qasm2::load::{load as qasm2_load, loads as qasm2_loads};
pub use qasm3::dump::{dump as qasm3_dump, dumps as qasm3_dumps};
pub use qasm3::load::{load as qasm3_load, loads as qasm3_loads};
pub use qcis::dump::{dump as qcis_dump, dumps as qcis_dumps};
pub use qcis::load::{load as qcis_load, loads as qcis_loads};

pub(crate) fn io_errors_equivalent(lhs: &std::io::Error, rhs: &std::io::Error) -> bool {
    lhs.kind() == rhs.kind() && lhs.to_string() == rhs.to_string()
}
