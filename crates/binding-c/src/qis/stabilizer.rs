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

//! C ABI for `StabilizerState` (Clifford-circuit simulator).

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::qis::{COutcomeList, CPauliString, CStabilizerState, check_qubit, qis_err_code};
use cqlib_core::qis::StabilizerState;
use std::ffi::CString;
use std::os::raw::c_char;

#[inline]
fn map_err(err: cqlib_core::qis::QisError) -> i32 {
    qis_err_code(&err)
}

fn apply_single(
    ptr: *mut CStabilizerState,
    qubit: u32,
    f: impl FnOnce(&mut StabilizerState, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let n = wrapper.inner.num_qubits();
    let q = match check_qubit(n, qubit) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, q)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

fn apply_two(
    ptr: *mut CStabilizerState,
    q0: u32,
    q1: u32,
    f: impl FnOnce(&mut StabilizerState, usize, usize) -> Result<(), cqlib_core::qis::QisError>,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let n = wrapper.inner.num_qubits();
    let a = match check_qubit(n, q0) {
        Ok(q) => q,
        Err(code) => return code,
    };
    let b = match check_qubit(n, q1) {
        Ok(q) => q,
        Err(code) => return code,
    };
    f(&mut wrapper.inner, a, b)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_new(num_qubits: usize) -> *mut CStabilizerState {
    if num_qubits == 0 {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(CStabilizerState {
        inner: StabilizerState::new(num_qubits),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_free(ptr: *mut CStabilizerState) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_from_circuit(circuit: *const CCircuit) -> *mut CStabilizerState {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match StabilizerState::from_circuit(circuit) {
        Ok(ss) => Box::into_raw(Box::new(CStabilizerState { inner: ss })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_circuit(
    ptr: *mut CStabilizerState,
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
pub extern "C" fn stabilizer_apply_h(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_h)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_x(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_x)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_y(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_y)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_z(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_z)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_s(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_sdg(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_sdg)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_x2p(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_x2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_x2m(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_x2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_y2p(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_y2p)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_y2m(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::apply_y2m)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_cx(
    ptr: *mut CStabilizerState,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, StabilizerState::apply_cx)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_cy(
    ptr: *mut CStabilizerState,
    control: u32,
    target: u32,
) -> i32 {
    apply_two(ptr, control, target, StabilizerState::apply_cy)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_cz(ptr: *mut CStabilizerState, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, StabilizerState::apply_cz)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_swap(ptr: *mut CStabilizerState, q0: u32, q1: u32) -> i32 {
    apply_two(ptr, q0, q1, StabilizerState::apply_swap)
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_measure(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    let n = wrapper.inner.num_qubits();
    let q = match check_qubit(n, qubit) {
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
pub extern "C" fn stabilizer_measure_all(ptr: *mut CStabilizerState) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &mut *ptr };
    let n = wrapper.inner.num_qubits();
    let outcome = wrapper.inner.measure_all();
    let bitstring = outcome.to_bitstring(n);
    match CString::new(bitstring) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_reset(ptr: *mut CStabilizerState, qubit: u32) -> i32 {
    apply_single(ptr, qubit, StabilizerState::reset)
}

/// Return the number of basis-state probabilities (2^N), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_probabilities_len(ptr: *const CStabilizerState) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    1usize << wrapper.inner.num_qubits()
}

/// Copy the probability distribution into `buffer`.
/// Returns 0 on success, or a negative error code.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_probabilities(
    ptr: *const CStabilizerState,
    buffer: *mut f64,
    len: usize,
) -> i32 {
    if ptr.is_null() || buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let probs = match wrapper.inner.probabilities() {
        Ok(p) => p,
        Err(err) => return qis_err_code(&err),
    };
    if probs.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(probs.as_ptr(), buffer, len);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_sample_shots(
    ptr: *const CStabilizerState,
    shots: usize,
) -> *mut COutcomeList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let n = wrapper.inner.num_qubits();
    let outcomes = wrapper.inner.sample_shots(shots);
    let items = outcomes.into_iter().map(|o| o.to_bitstring(n)).collect();
    Box::into_raw(Box::new(COutcomeList { items }))
}

#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_num_qubits(ptr: *const CStabilizerState) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Compute the Pauli expectation value <P> for the given Pauli string.
/// Writes the value (-1, 0, or +1) to `out` and returns 0 on success.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_pauli_expectation(
    ptr: *const CStabilizerState,
    pauli: *const CPauliString,
    out: *mut i32,
) -> i32 {
    if ptr.is_null() || pauli.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let p = unsafe { &(*pauli).inner };
    match wrapper.inner.pauli_expectation(p) {
        Ok(value) => {
            unsafe { *out = value };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Return the number of stabilizer generators (always N), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_get_stabilizers_len(ptr: *const CStabilizerState) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    wrapper.inner.num_qubits()
}

/// Return the stabilizer generator at `index` as a heap-allocated bitstring.
/// Caller must free with `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_get_stabilizer(
    ptr: *const CStabilizerState,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let stabilizers = wrapper.inner.get_stabilizers();
    match stabilizers.get(index) {
        Some(pauli) => {
            // Display uses the same convention as a bitstring: +XYZ format
            // where Y counts as a stabilizer label.
            let label = pauli.to_string();
            match CString::new(label) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        None => std::ptr::null_mut(),
    }
}

// =====  Simulator supplement (checklist 2.5)  =====

use std::ffi::CStr;

use crate::device::standard_gate_from_name;

/// Resolves a NUL-terminated standard-gate name (e.g. "H", "CX", "SWAP").
/// Returns -1 for NULL input and -8 for an unknown name or invalid UTF-8.
fn parse_standard_gate(name: *const c_char) -> Result<cqlib_core::circuit::StandardGate, i32> {
    if name.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let name = unsafe { CStr::from_ptr(name) };
    let name = match name.to_str() {
        Ok(s) => s,
        Err(_) => return Err(CqlibError::InvalidParam as i32),
    };
    standard_gate_from_name(name).ok_or(CqlibError::InvalidParam as i32)
}

/// Collects and bounds-checks a qubit-index array for this simulator.
fn resolve_qubits(num_qubits: usize, qubits: *const u32, len: usize) -> Result<Vec<usize>, i32> {
    let slice = unsafe { std::slice::from_raw_parts(qubits, len) };
    slice.iter().map(|&q| check_qubit(num_qubits, q)).collect()
}

/// Builds the gate-parameter slice, validating that every value is finite.
/// A NULL pointer is allowed when `num_params` is zero.
fn resolve_params(params: *const f64, num_params: usize) -> Result<Vec<f64>, i32> {
    if num_params == 0 {
        return Ok(Vec::new());
    }
    if params.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let slice = unsafe { std::slice::from_raw_parts(params, num_params) };
    if slice.iter().any(|v| !v.is_finite()) {
        return Err(CqlibError::InvalidParam as i32);
    }
    Ok(slice.to_vec())
}

/// Apply a Clifford standard gate identified by name (e.g. "H", "CX", "SWAP").
/// Non-Clifford gates are rejected by the simulator.
///
/// `qubits` holds `num_target_qubits` target indices and `params` holds
/// `num_params` gate parameters (NULL is allowed when `num_params` is 0).
/// Returns 0 on success, -1 for NULL pointers, -2 for an out-of-bounds qubit,
/// -8 for an unknown gate name or invalid argument counts, -7 for a
/// non-Clifford gate.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_apply_standard_gate(
    ptr: *mut CStabilizerState,
    gate_name: *const c_char,
    qubits: *const u32,
    num_target_qubits: usize,
    params: *const f64,
    num_params: usize,
) -> i32 {
    if ptr.is_null() || qubits.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let gate = match parse_standard_gate(gate_name) {
        Ok(gate) => gate,
        Err(code) => return code,
    };
    let wrapper = unsafe { &mut *ptr };
    let targets = match resolve_qubits(wrapper.inner.num_qubits(), qubits, num_target_qubits) {
        Ok(targets) => targets,
        Err(code) => return code,
    };
    let params = match resolve_params(params, num_params) {
        Ok(params) => params,
        Err(code) => return code,
    };
    wrapper
        .inner
        .apply_standard_gate(gate, &targets, &params)
        .map_err(map_err)
        .map_or_else(|e| e, |_| 0)
}

/// Return the number of destabilizer generators (always N), or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_get_destabilizers_len(ptr: *const CStabilizerState) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &*ptr };
    wrapper.inner.num_qubits()
}

/// Return the destabilizer generator at `index` as a heap-allocated Pauli
/// label (same "+XYZ" format as `stabilizer_get_stabilizer`). Caller must free
/// with `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_get_destabilizer(
    ptr: *const CStabilizerState,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    let destabilizers = wrapper.inner.get_destabilizers();
    match destabilizers.get(index) {
        Some(pauli) => match CString::new(pauli.to_string()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Return the probability of measuring a specific computational-basis
/// outcome. `bits` is an array of `len` booleans where `bits[0]` is qubit 0;
/// `len` must equal the number of qubits. Writes the probability to `out` and
/// returns 0 on success, -1 on NULL pointers, -8 on a length mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_probability_of(
    ptr: *const CStabilizerState,
    bits: *const bool,
    len: usize,
    out: *mut f64,
) -> i32 {
    if ptr.is_null() || bits.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &*ptr };
    let bits = unsafe { std::slice::from_raw_parts(bits, len) };
    match wrapper.inner.probability_of(bits) {
        Ok(probability) => {
            unsafe { *out = probability };
            0
        }
        Err(err) => qis_err_code(&err),
    }
}

/// Execute a Clifford circuit from |0...0> and return the final state.
///
/// Unlike `stabilizer_from_circuit` / `stabilizer_apply_circuit`, which ignore
/// terminal measurement declarations, this entry point executes `MeasureBit`
/// and `MeasureBits` operations, so the returned state is already collapsed.
/// Runtime classical results are not yet exposed through the C ABI.
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_run_circuit(circuit: *const CCircuit) -> *mut CStabilizerState {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    match StabilizerState::run_circuit(circuit) {
        Ok(result) => Box::into_raw(Box::new(CStabilizerState {
            inner: result.state,
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Export the stabilizer generators in Stim-compatible text format (one
/// "+XYZ" Pauli label per line). Caller must free with `cqlib_string_free`.
/// Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn stabilizer_to_stim_format(ptr: *const CStabilizerState) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let wrapper = unsafe { &*ptr };
    match CString::new(wrapper.inner.to_stim_format()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
