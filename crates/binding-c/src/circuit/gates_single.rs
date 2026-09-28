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
    CCircuit, CParameter, apply_param, apply_single, apply_three_params, apply_two_params,
};
use cqlib_core::circuit::Circuit;

#[unsafe(no_mangle)]
pub extern "C" fn circuit_h(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::h)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_x(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::x)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_y(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::y)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_z(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::z)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rx(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |circuit, qubit| circuit.rx(qubit, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_ry(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |circuit, qubit| circuit.ry(qubit, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rz(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |circuit, qubit| circuit.rz(qubit, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_measure(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, |circuit, qubit| {
        circuit.measure(qubit).map(|_| ())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_reset(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::reset)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rx_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param_ptr: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param_ptr, Circuit::rx)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_ry_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param_ptr: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param_ptr, Circuit::ry)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rz_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param_ptr: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param_ptr, Circuit::rz)
}

// =====  Section 2.1.1: Single-qubit gates (no parameters)  =====

#[unsafe(no_mangle)]
pub extern "C" fn circuit_i(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::i)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_s(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::s)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_sdg(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::sdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_t(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::t)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_tdg(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::tdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_x2p(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::x2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_x2m(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::x2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_y2p(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::y2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_y2m(ptr: *mut CCircuit, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Circuit::y2m)
}

// =====  Section 2.1.2: Single-qubit gates (with parameters)  =====

#[unsafe(no_mangle)]
pub extern "C" fn circuit_phase(ptr: *mut CCircuit, qubit: u32, lambda: f64) -> i32 {
    if !lambda.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.phase(q, lambda))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_u(
    ptr: *mut CCircuit,
    qubit: u32,
    theta: f64,
    phi: f64,
    lambda: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() || !lambda.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.u(q, theta, phi, lambda))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.xy(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy2p(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.xy2p(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy2m(ptr: *mut CCircuit, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.xy2m(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rxy(ptr: *mut CCircuit, qubit: u32, theta: f64, phi: f64) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return -3;
    }
    apply_single(ptr, qubit, |c, q| c.rxy(q, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_phase_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param, Circuit::phase)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param, Circuit::xy)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy2p_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param, Circuit::xy2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_xy2m_param(
    ptr: *mut CCircuit,
    qubit: u32,
    param: *const CParameter,
) -> i32 {
    apply_param(ptr, qubit, param, Circuit::xy2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_u_param(
    ptr: *mut CCircuit,
    qubit: u32,
    theta: *const CParameter,
    phi: *const CParameter,
    lambda: *const CParameter,
) -> i32 {
    apply_three_params(ptr, qubit, theta, phi, lambda, Circuit::u)
}

#[unsafe(no_mangle)]
pub extern "C" fn circuit_rxy_param(
    ptr: *mut CCircuit,
    qubit: u32,
    theta: *const CParameter,
    phi: *const CParameter,
) -> i32 {
    apply_two_params(ptr, qubit, theta, phi, Circuit::rxy)
}
