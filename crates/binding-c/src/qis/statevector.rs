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

//! C ABI for `Statevector` simulator.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::qis::{COutcomeList, CStatevector, check_qubit, qis_err_code};
use cqlib_core::qis::{Observable, Statevector};
use std::ffi::CString;
use std::os::raw::c_char;

#[inline]
fn map_err(err: cqlib_core::qis::QisError) -> i32 {
    qis_err_code(&err)
}

/// Apply a single-qubit method that takes only a qubit index.
fn apply_single(
    ptr: *mut CStatevector,
    qubit: u32,
    f: impl FnOnce(&mut Statevector, usize) -> Result<(), cqlib_core::qis::QisError>,
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

/// Apply a two-qubit method.
fn apply_two(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    f: impl FnOnce(&mut Statevector, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
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

/// Apply a three-qubit method.
fn apply_three(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    q2: u32,
    f: impl FnOnce(&mut Statevector, usize, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
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

/// Create a new statevector with `num_qubits` initialised to |0...0>.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_new(num_qubits: usize) -> *mut CStatevector {
    Box::into_raw(Box::new(CStatevector {
        inner: Statevector::new(num_qubits),
    }))
}

/// Free a statevector. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_free(ptr: *mut CStatevector) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Build a statevector by simulating `circuit`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_from_circuit(circuit: *const CCircuit) -> *mut CStatevector {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match Statevector::from_circuit(circuit) {
        Ok(sv) => Box::into_raw(Box::new(CStatevector { inner: sv })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Apply `circuit` to the statevector in place. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_circuit(
    ptr: *mut CStatevector,
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
pub extern "C" fn statevector_apply_h(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_h)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_x(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_x)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_y(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_y)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_z(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_z)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_s(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_sdg(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_sdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_t(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_t)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_tdg(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_tdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_x2p(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_x2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_x2m(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_x2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_y2p(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_y2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_y2m(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::apply_y2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rx(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_rx(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_ry(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_ry(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rz(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_rz(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_phase(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_phase(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_u(
    ptr: *mut CStatevector,
    qubit: u32,
    theta: f64,
    phi: f64,
    lambda: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() || !lambda.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_u(q, theta, phi, lambda))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_xy(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_xy(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_xy2p(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_xy2p(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_xy2m(ptr: *mut CStatevector, qubit: u32, theta: f64) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_xy2m(q, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rxy(
    ptr: *mut CStatevector,
    qubit: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_single(ptr, qubit, |sv, q| sv.apply_rxy(q, theta, phi))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_gphase(ptr: *mut CStatevector, phi: f64) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    if !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper
        .inner
        .apply_gphase(phi)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_cx(ptr: *mut CStatevector, control: u32, target: u32) -> i32 {
    apply_two(ptr, control, target, Statevector::apply_cx)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_cy(ptr: *mut CStatevector, control: u32, target: u32) -> i32 {
    apply_two(ptr, control, target, Statevector::apply_cy)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_cz(ptr: *mut CStatevector, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, Statevector::apply_cz)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_swap(ptr: *mut CStatevector, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, Statevector::apply_swap)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_ccx(
    ptr: *mut CStatevector,
    c0: u32,
    c1: u32,
    target: u32,
) -> i32 {
    apply_three(ptr, c0, c1, target, Statevector::apply_ccx)
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rxx(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sv, a, b| sv.apply_rxx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_ryy(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sv, a, b| sv.apply_ryy(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rzz(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sv, a, b| sv.apply_rzz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_rzx(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sv, a, b| sv.apply_rzx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_crx(
    ptr: *mut CStatevector,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sv, a, b| sv.apply_crx(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_cry(
    ptr: *mut CStatevector,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sv, a, b| sv.apply_cry(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_crz(
    ptr: *mut CStatevector,
    control: u32,
    target: u32,
    theta: f64,
) -> i32 {
    if !theta.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, control, target, |sv, a, b| sv.apply_crz(a, b, theta))
}

#[unsafe(no_mangle)]
pub extern "C" fn statevector_apply_fsim(
    ptr: *mut CStatevector,
    q0: u32,
    q1: u32,
    theta: f64,
    phi: f64,
) -> i32 {
    if !theta.is_finite() || !phi.is_finite() {
        return CqlibError::InvalidParam as i32;
    }
    apply_two(ptr, q0, q1, |sv, a, b| sv.apply_fsim(a, b, theta, phi))
}

/// Return the number of basis-state amplitudes (2^N), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_probabilities_len(ptr: *const CStatevector) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    1usize << wrapper.inner.num_qubits
}

/// Copy the probability distribution into `buffer`.
/// Returns 0 on success, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_probabilities(
    ptr: *const CStatevector,
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

/// Compute the expectation value `<H>` of an observable.
/// Writes the value to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_expectation(
    ptr: *const CStatevector,
    observable: *const crate::qis::CHamiltonian,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || observable.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let h = unsafe { &(*observable).inner };
    let result = wrapper.inner.expectation(h as &dyn Observable);
    match result {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Measure `qubit` in the Z basis, collapsing the state.
/// Returns 0 for outcome |0>, 1 for outcome |1>, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_measure(ptr: *mut CStatevector, qubit: u32) -> i32 {
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

/// Measure all qubits and return the outcome as a heap-allocated big-endian
/// bitstring (MSB = qubit N-1, LSB = qubit 0). Caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_measure_all(ptr: *mut CStatevector) -> *mut c_char {
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

/// Sample `shots` independent measurement outcomes.
/// Returns a heap-allocated `COutcomeList*` (caller must free with
/// `outcome_list_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_sample_shots(
    ptr: *const CStatevector,
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

/// Reset `qubit` to |0>. Returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_reset(ptr: *mut CStatevector, qubit: u32) -> i32 {
    apply_single(ptr, qubit, Statevector::reset)
}

/// Return the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn statevector_num_qubits(ptr: *const CStatevector) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits }
}

/// Free an `COutcomeList` allocated by `*_sample_shots`. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn outcome_list_free(ptr: *mut COutcomeList) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Return the number of outcomes in the list, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn outcome_list_len(ptr: *const COutcomeList) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).items.len() }
}

/// Return the bitstring at `index` as a heap-allocated C string.
/// Caller must free with `cqlib_string_free`. Returns NULL on out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn outcome_list_get(ptr: *const COutcomeList, index: usize) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let list = unsafe { &*ptr };
    match list.items.get(index) {
        Some(s) => match CString::new(s.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}
