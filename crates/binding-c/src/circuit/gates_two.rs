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

use crate::circuit::{
    CCircuit, CParameter, apply_two, apply_two_qubit_param, apply_two_qubit_two_params,
};
use cqlib_core::circuit::Circuit;

#[unsafe(no_mangle)]
pub extern "C" fn circuit_cx(ptr: *mut CCircuit, control: u32, target: u32) -> i32 {
    apply_two(ptr, control, target, Circuit::cx)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_cz(ptr: *mut CCircuit, control: u32, target: u32) -> i32 {
    apply_two(ptr, control, target, Circuit::cz)
}

// =====  Section 2.1.3: Two-qubit gates (no parameters)  =====

#[unsafe(no_mangle)]
pub extern "C" fn circuit_cy(ptr: *mut CCircuit, control: u32, target: u32) -> i32 {
    apply_two(ptr, control, target, Circuit::cy)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_swap(ptr: *mut CCircuit, a: u32, b: u32) -> i32 {
    apply_two(ptr, a, b, Circuit::swap)
}

// =====  Section 2.1.4: Two-qubit gates (with parameters)  =====

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rxx(ptr: *mut CCircuit, a: u32, b: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, a, b, |c, a, b| c.rxx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_ryy(ptr: *mut CCircuit, a: u32, b: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, a, b, |c, a, b| c.ryy(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rzz(ptr: *mut CCircuit, a: u32, b: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, a, b, |c, a, b| c.rzz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rzx(ptr: *mut CCircuit, a: u32, b: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, a, b, |c, a, b| c.rzx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_crx(ptr: *mut CCircuit, control: u32, target: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, control, target, |c, ctl, tgt| c.crx(ctl, tgt, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_cry(ptr: *mut CCircuit, control: u32, target: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, control, target, |c, ctl, tgt| c.cry(ctl, tgt, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_crz(ptr: *mut CCircuit, control: u32, target: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_two(ptr, control, target, |c, ctl, tgt| c.crz(ctl, tgt, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_fsim(ptr: *mut CCircuit, a: u32, b: u32, theta: f64, phi: f64) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return -3;
    }
    apply_two(ptr, a, b, |c, a, b| c.fsim(a, b, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rxx_param(
    ptr: *mut CCircuit,
    a: u32,
    b: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, a, b, param, Circuit::rxx)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_ryy_param(
    ptr: *mut CCircuit,
    a: u32,
    b: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, a, b, param, Circuit::ryy)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rzz_param(
    ptr: *mut CCircuit,
    a: u32,
    b: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, a, b, param, Circuit::rzz)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rzx_param(
    ptr: *mut CCircuit,
    a: u32,
    b: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, a, b, param, Circuit::rzx)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_crx_param(
    ptr: *mut CCircuit,
    control: u32,
    target: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, control, target, param, Circuit::crx)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_cry_param(
    ptr: *mut CCircuit,
    control: u32,
    target: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, control, target, param, Circuit::cry)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_crz_param(
    ptr: *mut CCircuit,
    control: u32,
    target: u32,
    param: *const CParameter,
) -> i32 {
    apply_two_qubit_param(ptr, control, target, param, Circuit::crz)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_fsim_param(
    ptr: *mut CCircuit,
    a: u32,
    b: u32,
    theta: *const CParameter,
    phi: *const CParameter,
) -> i32 {
    apply_two_qubit_two_params(ptr, a, b, theta, phi, Circuit::fsim)
}
