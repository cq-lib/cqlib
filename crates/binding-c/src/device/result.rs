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

//! C ABI for `ExecutionResult` (measurement counts and probabilities).

use crate::device::{CCountsList, CExecutionResult};
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn bitstring_key(outcome: &Outcome, num_qubits: usize) -> String {
    outcome.to_bitstring(num_qubits)
}

/// Creates a completed execution result from measurement counts.
///
/// `bitstrings` and `counts` are parallel arrays of length `num_entries`;
/// each bitstring must have exactly `num_qubits` characters. Returns NULL
/// on error.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_from_counts(
    task_id: *const c_char,
    num_qubits: usize,
    shots: usize,
    bitstrings: *const *const c_char,
    counts: *const u64,
    num_entries: usize,
) -> *mut CExecutionResult {
    if task_id.is_null() || (num_entries > 0 && (bitstrings.is_null() || counts.is_null())) {
        return std::ptr::null_mut();
    }
    let task_id = match unsafe { CStr::from_ptr(task_id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    let mut map: HashMap<Outcome, usize> = HashMap::new();
    for i in 0..num_entries {
        let raw = unsafe { *bitstrings.add(i) };
        if raw.is_null() {
            return std::ptr::null_mut();
        }
        let bits = match unsafe { CStr::from_ptr(raw) }.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let outcome = match Outcome::from_bitstring(bits) {
            Ok(o) => o,
            Err(_) => return std::ptr::null_mut(),
        };
        let count = unsafe { *counts.add(i) } as usize;
        *map.entry(outcome).or_insert(0) += count;
    }
    let qubits: Vec<Qubit> = (0..num_qubits as u32).map(Qubit::new).collect();
    let result = ExecutionResult::from_counts(task_id, qubits, shots, num_qubits, None, map);
    Box::into_raw(Box::new(CExecutionResult { inner: result }))
}

/// Frees an execution result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_free(ptr: *mut CExecutionResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Returns the number of shots, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_shots(ptr: *const CExecutionResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.shots() }
}

/// Returns the number of qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_num_qubits(ptr: *const CExecutionResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the measurement counts as an owned `CCountsList*`
/// (bitstring keys, integer weights as f64). Caller must free with
/// `counts_list_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_counts(ptr: *const CExecutionResult) -> *mut CCountsList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    let n = result.inner.num_qubits();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for (outcome, count) in result.inner.counts() {
        keys.push(bitstring_key(outcome, n));
        values.push(*count as f64);
    }
    Box::into_raw(Box::new(CCountsList { keys, values }))
}

/// Computes (if needed) and returns the probability distribution as an
/// owned `CCountsList*` (bitstring keys, f64 probabilities).
/// Caller must free with `counts_list_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_probabilities(ptr: *mut CExecutionResult) -> *mut CCountsList {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &mut *ptr };
    result.inner.calc_probabilities();
    let n = result.inner.num_qubits();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    if let Some(probs) = result.inner.probabilities() {
        for (outcome, probability) in probs {
            keys.push(bitstring_key(outcome, n));
            values.push(*probability);
        }
    }
    Box::into_raw(Box::new(CCountsList { keys, values }))
}

/// Returns the number of entries in the list, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn counts_list_len(ptr: *const CCountsList) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let list = unsafe { &*ptr };
    list.keys.len()
}

/// Returns the bitstring key at `index` as a heap-allocated C string.
/// Caller must free with `cqlib_string_free`. Returns NULL on out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn counts_list_get_key(ptr: *const CCountsList, index: usize) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let list = unsafe { &*ptr };
    match list.keys.get(index) {
        Some(key) => match CString::new(key.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the weight (count or probability) at `index`, or NaN on error.
#[unsafe(no_mangle)]
pub extern "C" fn counts_list_get_value(ptr: *const CCountsList, index: usize) -> f64 {
    if ptr.is_null() {
        return f64::NAN;
    }
    let list = unsafe { &*ptr };
    list.values.get(index).copied().unwrap_or(f64::NAN)
}

/// Frees a counts list. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn counts_list_free(ptr: *mut CCountsList) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
