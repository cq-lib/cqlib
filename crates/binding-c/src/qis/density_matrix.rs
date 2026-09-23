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

//! C ABI for `DensityMatrix` simulator.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::qis::{CDensityMatrix, COutcomeList, check_qubit, qis_err_code};
use cqlib_core::qis::{DensityMatrix, Observable};
use num_complex::Complex64;
use std::ffi::CString;
use std::os::raw::c_char;

#[inline]
fn map_err(err: cqlib_core::qis::QisError) -> i32 {
    qis_err_code(&err)
}

fn apply_single(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    f: impl FnOnce(&mut DensityMatrix, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let q = match check_qubit(wrapper.inner.num_qubits, qubit) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, q)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

fn apply_two(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    f: impl FnOnce(&mut DensityMatrix, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let a = match check_qubit(wrapper.inner.num_qubits, q0) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let b = match check_qubit(wrapper.inner.num_qubits, q1) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, a, b)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

fn apply_three(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    q2: u32,
    f: impl FnOnce(&mut DensityMatrix, usize, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let a = match check_qubit(wrapper.inner.num_qubits, q0) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let b = match check_qubit(wrapper.inner.num_qubits, q1) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let c = match check_qubit(wrapper.inner.num_qubits, q2) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, a, b, c)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_new(num_qubits: usize) -> *mut CDensityMatrix {
    Box::into_raw(Box::new(CDensityMatrix {
        inner: DensityMatrix::new(num_qubits),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_free(ptr: *mut CDensityMatrix) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_from_circuit(circuit: *const CCircuit) -> *mut CDensityMatrix {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match DensityMatrix::from_circuit(circuit) {
        Ok(dm) => Box::into_raw(Box::new(CDensityMatrix { inner: dm })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_circuit(
    ptr: *mut CDensityMatrix,
    circuit: *const CCircuit,
) -> i32 {
    if ptr.is_null() || circuit.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let circuit = unsafe { &(*circuit).inner };
    wrapper
        .inner
        .apply_circuit(circuit)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_h(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_h)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_x(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_x)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_y(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_y)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_z(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_z)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_s(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_sdg(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_sdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_t(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_t)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_tdg(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_tdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_x2p(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_x2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_x2m(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_x2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_y2p(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_y2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_y2m(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::apply_y2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rx(ptr: *mut CDensityMatrix, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_rx(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_ry(ptr: *mut CDensityMatrix, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_ry(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rz(ptr: *mut CDensityMatrix, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_rz(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_phase(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_phase(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_u(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    theta: f64,
    phi: f64,
    lambda: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() || !lambda.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_u(q, theta, phi, lambda))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_xy(ptr: *mut CDensityMatrix, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_xy(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_xy2p(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_xy2p(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_xy2m(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_xy2m(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rxy(
    ptr: *mut CDensityMatrix,
    qubit: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |dm, q| dm.apply_rxy(q, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_gphase(ptr: *mut CDensityMatrix, phi: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    // Global phase has no effect on a density matrix; mirror the Rust API.
    wrapper.inner.apply_gphase(phi);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_cx(
    ptr: *mut CDensityMatrix,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, DensityMatrix::apply_cx)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_cy(
    ptr: *mut CDensityMatrix,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, DensityMatrix::apply_cy)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_cz(ptr: *mut CDensityMatrix, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, DensityMatrix::apply_cz)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_swap(ptr: *mut CDensityMatrix, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, DensityMatrix::apply_swap)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_ccx(
    ptr: *mut CDensityMatrix,
    c0: u32,
    c1: u32,
    target: u32,
) -> i32 {
    apply_three(ptr, c0, c1, target, DensityMatrix::apply_ccx)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rxx(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |dm, a, b| dm.apply_rxx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_ryy(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |dm, a, b| dm.apply_ryy(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rzz(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |dm, a, b| dm.apply_rzz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_rzx(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |dm, a, b| dm.apply_rzx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_crx(
    ptr: *mut CDensityMatrix,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |dm, a, b| dm.apply_crx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_cry(
    ptr: *mut CDensityMatrix,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |dm, a, b| dm.apply_cry(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_crz(
    ptr: *mut CDensityMatrix,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |dm, a, b| dm.apply_crz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_fsim(
    ptr: *mut CDensityMatrix,
    q0: u32,
    q1: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |dm, a, b| dm.apply_fsim(a, b, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_probabilities_len(ptr: *const CDensityMatrix) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    1usize << wrapper.inner.num_qubits
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_probabilities(
    ptr: *const CDensityMatrix,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let probs = wrapper.inner.probabilities();
    if probs.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(probs.as_ptr(), buffer, len);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_expectation(
    ptr: *const CDensityMatrix,
    observable: *const crate::qis::CHamiltonian,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || observable.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let h = unsafe { &(*observable).inner };
    match wrapper.inner.expectation(h as &dyn Observable) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_measure(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let q = match check_qubit(wrapper.inner.num_qubits, qubit) {
        Ok(q) => q,
        Err(code) => return code,
    };
    match wrapper.inner.measure(q) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(err) => qis_err_code(&err),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_measure_all(ptr: *mut CDensityMatrix) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let outcome = wrapper.inner.measure_all();
    let bitstring = outcome.to_bitstring(wrapper.inner.num_qubits);
    match CString::new(bitstring) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_sample_shots(
    ptr: *const CDensityMatrix,
    shots: usize,
) -> *mut COutcomeList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let n = wrapper.inner.num_qubits;
    let outcomes = wrapper.inner.sample_shots(shots);
    let items = outcomes.into_iter().map(|o| o.to_bitstring(n)).collect();
    Box::into_raw(Box::new(COutcomeList { items }))
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_reset(ptr: *mut CDensityMatrix, qubit: u32) -> i32 {
    apply_single(ptr, qubit, DensityMatrix::reset)
}

#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_num_qubits(ptr: *const CDensityMatrix) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits }
}

/// Apply a Kraus operator channel to the targeted qubits.
///
/// `ops` is a flattened array of `num_ops * op_dim * op_dim` Complex64 values
/// (row-major). Each operator has dimension `op_dim = 2^num_target_qubits`.
/// `qubits` is the array of target qubit indices of length `num_target_qubits`.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_apply_kraus(
    ptr: *mut CDensityMatrix,
    ops: *const Complex64,
    num_ops: usize,
    op_dim: usize,
    qubits: *const u32,
    num_target_qubits: usize,
) -> i32 {
    if ptr.is_null() || ops.is_null() || qubits.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if num_ops == 0 || op_dim == 0 || num_target_qubits == 0 {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let qs = unsafe { std::slice::from_raw_parts(qubits, num_target_qubits) };
    let targets: Result<Vec<usize>, i32> = qs
        .iter()
        .map(|&q| check_qubit(wrapper.inner.num_qubits, q))
        .collect();
    let targets = match targets {
        Ok(t) => t,
        Err(code) => return code,
    };
    let expected_len = op_dim * op_dim;
    let ops_slice = unsafe { std::slice::from_raw_parts(ops, num_ops * expected_len) };
    let mut operators: Vec<Vec<Complex64>> = Vec::with_capacity(num_ops);
    for chunk in ops_slice.chunks_exact(expected_len) {
        operators.push(chunk.to_vec());
    }
    wrapper
        .inner
        .apply_kraus(&operators, &targets)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Compute the partial trace, keeping `keep_qubits`. Returns a new
/// `CDensityMatrix*` or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn density_matrix_partial_trace(
    ptr: *const CDensityMatrix,
    keep_qubits: *const u32,
    num_keep: usize,
) -> *mut CDensityMatrix {
    if ptr.is_null() || (num_keep > 0 && keep_qubits.is_null()) {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let qs = if num_keep == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(keep_qubits, num_keep) }
            .iter()
            .map(|&q| q as usize)
            .collect()
    };
    match wrapper.inner.partial_trace(&qs) {
        Ok(dm) => Box::into_raw(Box::new(CDensityMatrix { inner: dm })),
        Err(_) => std::ptr::null_mut(),
    }
}
