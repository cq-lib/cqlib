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

//! OpenQASM 3 import and export support.
//!
//! This module converts between OpenQASM 3 source and Cqlib's [`Circuit`] IR.
//! Loading is implemented by parsing source with Qiskit's `oq3_semantics`
//! front-end and lowering the semantic graph into Cqlib operations. Dumping
//! generates normalized OpenQASM 3 text from a [`Circuit`]; it does not try to
//! preserve original source formatting.
//!
//! # Entry Points
//!
//! - [`load::load`] / [`load::from_path`] reads an OpenQASM 3 file and returns
//!   [`load::Qasm3ParseError`] on failure.
//! - [`load::loads`] / [`load::from_str`] reads OpenQASM 3 source from a string
//!   and returns [`load::Qasm3ParseError`] on failure.
//! - [`dump::dump`] / [`dump::to_path`] writes a circuit to an OpenQASM 3 file
//!   and returns [`dump::Qasm3DumpError`] on failure.
//! - [`dump::dumps`] / [`dump::to_string`] serializes a circuit to an OpenQASM 3
//!   string and returns [`dump::Qasm3DumpError`] on failure.
//!
//! The crate root also re-exports these as `qasm3_load`, `qasm3_loads`,
//! `qasm3_dump`, and `qasm3_dumps`.
//!
//! # Loading Support
//!
//! The loader accepts `OPENQASM 3;` and `OPENQASM 3.0;`, including
//! `stdgates.inc`. `stdgates.inc` is the OpenQASM 3 standard-library include;
//! Cqlib does not vendor a local copy of this file. The loader relies on
//! `oq3_semantics` to make the standard library available, and the dumper emits
//! `include "stdgates.inc";` for OpenQASM 3 consumers that implement the
//! standard library. See the OpenQASM standard-library documentation:
//! <https://openqasm.com/language/standard_library.html>.
//!
//! Supported constructs include:
//!
//! - quantum declarations for scalar qubits and one-dimensional qubit registers
//! - classical `bit`, `bit[n]`, `bool`, and fixed-width `uint[n]` declarations
//! - standard gates that map to [`StandardGate`], including Cqlib extension names
//!   such as `x2p`, `x2m`, `y2p`, `y2m`, `xy2p`, `xy2m`, `rxx`, `ryy`, `rzz`,
//!   `rzx`, and `fsim`
//! - circuit-backed custom `gate` definitions
//! - measurements, reset, barrier, global phase, and gate-level `if`/`else`
//! - statically unrollable `for` loops and supported exact-value `switch` cases
//!
//! Unsupported OpenQASM 3 features return explicit errors instead of being
//! partially lowered. Examples include calibration, pulse timing, subroutines,
//! extern calls, hardware qubits, aliases, complex lvalue slicing, unsupported
//! gate modifiers, and general runtime arithmetic that cannot be represented by
//! the current circuit IR.
//!
//! # Dumping Support
//!
//! The dumper emits a normalized file in this order:
//!
//! 1. `OPENQASM 3.0;`
//! 2. `include "stdgates.inc";`
//! 3. generated gate definitions for Cqlib extension gates used by the circuit
//! 4. custom circuit-gate definitions
//! 5. quantum and classical declarations
//! 6. main circuit operations
//!
//! Gates already provided by `stdgates.inc` are called directly. Cqlib gates not
//! provided by `stdgates.inc`, such as `x2p`, `xy2p`, `rxx`, and `rzx`, are
//! defined once before the circuit body and are still called by their Cqlib
//! names in the main circuit.
//!
//! The top-level qubit collection is emitted as `qubit[n] q`, including the
//! single-qubit case. Classical variables are emitted as `c0`, `c1`, ... and
//! immutable classical measurement values as `v0`, `v1`, .... Measurement
//! followed immediately by a compatible store is folded to OpenQASM 3 assignment
//! form, for example `c0 = measure q;`. Standalone measurements are assigned to
//! a generated `bit[k] meas` register instead of being discarded, so downstream
//! hardware-oriented consumers get explicit classical destinations.
//!
//! The dumper is intentionally conservative. It rejects constructs that would
//! lose semantics in generated OpenQASM 3, including delay, matrix-only unitary
//! gates, general classical stores, measurements inside gate definitions, and
//! unsupported control-flow bodies.
//!
//! # Loading Example
//!
//! ```rust
//! use cqlib_core::circuit::{Instruction, StandardGate};
//! use cqlib_core::ir::qasm3_loads;
//!
//! let qasm = r#"
//!     OPENQASM 3;
//!     include "stdgates.inc";
//!     qubit[2] q;
//!     h q[0];
//!     cx q[0], q[1];
//! "#;
//!
//! let circuit = qasm3_loads(qasm).unwrap();
//! assert_eq!(circuit.num_qubits(), 2);
//! assert!(matches!(
//!     circuit.operations()[0].instruction,
//!     Instruction::Standard(StandardGate::H)
//! ));
//! ```
//!
//! # Dumping Example
//!
//! ```rust
//! use cqlib_core::circuit::{Circuit, Qubit};
//! use cqlib_core::ir::{qasm3_dumps, qasm3_loads};
//!
//! let q0 = Qubit::new(0);
//! let q1 = Qubit::new(1);
//! let mut circuit = Circuit::new(2);
//! circuit.x2p(q0).unwrap();
//! circuit.rzz(q0, q1, 0.25).unwrap();
//!
//! let qasm = qasm3_dumps(&circuit).unwrap();
//! assert_eq!(
//!     qasm,
//!     r#"OPENQASM 3.0;
//! include "stdgates.inc";
//!
//! gate x2p q { rx(pi/2) q; }
//!
//! gate rzz(theta) a,b { cx a,b; rz(theta) b; cx a,b; }
//!
//! qubit[2] q;
//!
//! x2p q[0];
//! rzz(0.25) q[0],q[1];
//! "#
//! );
//!
//! let round_trip = qasm3_loads(&qasm).unwrap();
//! assert_eq!(round_trip.operations().len(), 2);
//! ```
//!
//! [`Circuit`]: crate::circuit::Circuit
//! [`StandardGate`]: crate::circuit::StandardGate

pub mod dump;
pub mod load;

pub use dump::{dump, dumps, to_path, to_string};
pub use load::{from_path, from_str, load, loads};
