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

use crate::circuit::{CCircuit, apply_three};
use cqlib_core::circuit::Circuit;

// =====  Section 2.1.5: Three-qubit gate  =====

#[unsafe(no_mangle)]
pub extern "C" fn circuit_ccx(
    ptr: *mut CCircuit,
    control1: u32,
    control2: u32,
    target: u32,
) -> i32 {
    apply_three(ptr, control1, control2, target, Circuit::ccx)
}
