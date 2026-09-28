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

//! C ABI for IR module (QASM2/QASM3/QCIS load and dump).

pub mod qasm2;
pub mod qasm3;
pub mod qcis;

pub use qasm2::{qasm2_dump, qasm2_dumps, qasm2_load, qasm2_loads};
pub use qasm3::{qasm3_dump, qasm3_dumps, qasm3_load, qasm3_loads};
pub use qcis::{qcis_dump, qcis_dumps, qcis_load, qcis_loads};
