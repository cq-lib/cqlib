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

use crate::device::{CCountsList, CExecutionResult, write_u32_list};
use crate::error::CqlibError;
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
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

// =====  Section 3.5: lifecycle, status, and metadata  =====

/// `execution_result_status` tag: task submitted and queued.
pub const EXECUTION_STATUS_QUEUED: u8 = 0;
/// `execution_result_status` tag: task currently running.
pub const EXECUTION_STATUS_RUNNING: u8 = 1;
/// `execution_result_status` tag: task completed successfully.
pub const EXECUTION_STATUS_COMPLETED: u8 = 2;
/// `execution_result_status` tag: task failed (query the error code and
/// message for details).
pub const EXECUTION_STATUS_FAILED: u8 = 3;
/// `execution_result_status` tag: task cancelled by the user.
pub const EXECUTION_STATUS_CANCELLED: u8 = 4;

/// Converts a Unix-timestamp nanosecond count to whole milliseconds.
fn unix_nanos_to_ms(nanos: i128) -> i64 {
    nanos.div_euclid(1_000_000) as i64
}

/// Creates an execution result in Queued status.
///
/// `qubits` points to `num_qubits` measured-qubit IDs (entry `i` corresponds
/// to bit `i` of the outcome keys); `backend` may be NULL. The creation
/// timestamp is the current time. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_new(
    task_id: *const c_char,
    qubits: *const u32,
    num_qubits: usize,
    shots: usize,
    backend: *const c_char,
) -> *mut CExecutionResult {
    if task_id.is_null() || (num_qubits > 0 && qubits.is_null()) {
        return std::ptr::null_mut();
    }
    let task_id = match unsafe { CStr::from_ptr(task_id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    let backend = if backend.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(backend) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => return std::ptr::null_mut(),
        }
    };
    let qubits: Vec<Qubit> = if num_qubits == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(qubits, num_qubits) }
            .iter()
            .map(|&q| Qubit::new(q))
            .collect()
    };
    let result = ExecutionResult::new(task_id, qubits, shots, num_qubits, backend, None);
    Box::into_raw(Box::new(CExecutionResult { inner: result }))
}

/// Marks the job as running and records the current time as the start
/// timestamp. Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_start(ptr: *mut CExecutionResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.start(None);
    CqlibError::Ok as i32
}

/// Marks the job as completed and replaces the measurement counts.
///
/// `bitstrings` and `counts` are parallel arrays of length `num_entries`;
/// repeated bitstrings accumulate. The finish timestamp is the current time.
/// Returns 0 on success, -1 on NULL, or -4 for an invalid bitstring.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_finish(
    ptr: *mut CExecutionResult,
    bitstrings: *const *const c_char,
    counts: *const u64,
    num_entries: usize,
) -> i32 {
    if ptr.is_null() || (num_entries > 0 && (bitstrings.is_null() || counts.is_null())) {
        return CqlibError::NullPtr as i32;
    }
    let mut map: HashMap<Outcome, usize> = HashMap::new();
    for i in 0..num_entries {
        let raw = unsafe { *bitstrings.add(i) };
        if raw.is_null() {
            return CqlibError::NullPtr as i32;
        }
        let bits = match unsafe { CStr::from_ptr(raw) }.to_str() {
            Ok(s) => s,
            Err(_) => return CqlibError::ParseError as i32,
        };
        let outcome = match Outcome::from_bitstring(bits) {
            Ok(o) => o,
            Err(_) => return CqlibError::ParseError as i32,
        };
        let count = unsafe { *counts.add(i) } as usize;
        *map.entry(outcome).or_insert(0) += count;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.finish(map, None);
    CqlibError::Ok as i32
}

/// Marks the job as failed with an error message and code.
///
/// Returns 0 on success, -1 on NULL, or -4 when the message is not valid
/// UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_fail(
    ptr: *mut CExecutionResult,
    msg: *const c_char,
    code: i32,
) -> i32 {
    if ptr.is_null() || msg.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let msg = match unsafe { CStr::from_ptr(msg) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CqlibError::ParseError as i32,
    };
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.fail(msg, code);
    CqlibError::Ok as i32
}

/// Marks the job as cancelled. Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_cancel(ptr: *mut CExecutionResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.cancel();
    CqlibError::Ok as i32
}

/// Writes the execution status tag to `*out_tag` (one of the
/// `EXECUTION_STATUS_*` constants).
///
/// When the status is failed, `*out_error_code` receives the stored error
/// code (0 otherwise); `out_error_code` may be NULL when not needed.
/// Returns 0 on success or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_status(
    ptr: *const CExecutionResult,
    out_tag: *mut u8,
    out_error_code: *mut i32,
) -> i32 {
    if ptr.is_null() || out_tag.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let status = unsafe { (*ptr).inner.status() };
    let (tag, code) = match status {
        Status::Queued => (EXECUTION_STATUS_QUEUED, 0),
        Status::Running => (EXECUTION_STATUS_RUNNING, 0),
        Status::Completed => (EXECUTION_STATUS_COMPLETED, 0),
        Status::Failed { error_code, .. } => (EXECUTION_STATUS_FAILED, *error_code),
        Status::Cancelled => (EXECUTION_STATUS_CANCELLED, 0),
    };
    unsafe {
        *out_tag = tag;
        if !out_error_code.is_null() {
            *out_error_code = code;
        }
    }
    CqlibError::Ok as i32
}

/// Returns the error message stored by the failed status as a
/// heap-allocated C string. Caller must free with `cqlib_string_free`.
/// Returns NULL on NULL input or when the status is not failed.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_error_message(ptr: *const CExecutionResult) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { (*ptr).inner.status() };
    let Status::Failed { error_msg, .. } = status else {
        return std::ptr::null_mut();
    };
    match CString::new(error_msg.as_str()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the task ID as a heap-allocated C string. Caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_task_id(ptr: *const CExecutionResult) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let task_id = unsafe { (*ptr).inner.task_id() }.to_string();
    match CString::new(task_id) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Returns the backend name as a heap-allocated C string. Caller must free
/// with `cqlib_string_free`. Returns NULL on error or when no backend is set.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_backend(ptr: *const CExecutionResult) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { (*ptr).inner.backend() } {
        Some(backend) => match CString::new(backend.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the task creation timestamp in milliseconds since the Unix epoch.
/// Returns 0 with the value in `*out_ms`, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_created_at(
    ptr: *const CExecutionResult,
    out_ms: *mut i64,
) -> i32 {
    if ptr.is_null() || out_ms.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let t = unsafe { (*ptr).inner.created_at() };
    unsafe { *out_ms = unix_nanos_to_ms(t.unix_timestamp_nanos()) };
    CqlibError::Ok as i32
}

/// Returns the execution start timestamp in milliseconds since the Unix
/// epoch. Returns 0 with the value in `*out_ms`, or -8 when the job has not
/// started.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_started_at(
    ptr: *const CExecutionResult,
    out_ms: *mut i64,
) -> i32 {
    if ptr.is_null() || out_ms.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.started_at() } {
        Some(t) => {
            unsafe { *out_ms = unix_nanos_to_ms(t.unix_timestamp_nanos()) };
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns the execution finish timestamp in milliseconds since the Unix
/// epoch. Returns 0 with the value in `*out_ms`, or -8 when the job has not
/// finished.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_finished_at(
    ptr: *const CExecutionResult,
    out_ms: *mut i64,
) -> i32 {
    if ptr.is_null() || out_ms.is_null() {
        return CqlibError::NullPtr as i32;
    }
    match unsafe { (*ptr).inner.finished_at() } {
        Some(t) => {
            unsafe { *out_ms = unix_nanos_to_ms(t.unix_timestamp_nanos()) };
            CqlibError::Ok as i32
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Returns the number of measured qubits, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_qubits_len(ptr: *const CExecutionResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits().len() }
}

/// Copies the measured qubit IDs into `out` (two-step pattern; pair with
/// `execution_result_qubits_len`). Entry `i` corresponds to bit `i` of the
/// outcome keys. Returns the total count.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_qubits(
    ptr: *const CExecutionResult,
    out: *mut u32,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let items: Vec<u32> = unsafe { (*ptr).inner.qubits() }
        .iter()
        .map(|q| q.id())
        .collect();
    write_u32_list(&items, out, len)
}

// =====  Section 3.6: status flags, bitstrings, and two-step probabilities  =====

/// Returns 1 when the execution completed successfully, 0 otherwise, or -1
/// on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_is_success(ptr: *const CExecutionResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    i32::from(unsafe { (*ptr).inner.status().is_success() })
}

/// Returns 1 when the execution reached a terminal state (completed, failed,
/// or cancelled), 0 otherwise, or -1 on NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_is_terminal(ptr: *const CExecutionResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    i32::from(unsafe { (*ptr).inner.status().is_terminal() })
}

/// Formats the raw outcome bitmask as a big-endian bitstring over the
/// result's measured-qubit width (bit `i` of `outcome` is measured qubit
/// `i`; high bits beyond the width are truncated).
///
/// Returns a heap-allocated C string freed with `cqlib_string_free`, or NULL
/// on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_to_bitstring(
    ptr: *const CExecutionResult,
    outcome: u64,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let n = unsafe { (*ptr).inner.num_qubits() };
    let indices = (0..u64::BITS as usize).filter(|&bit| outcome & (1u64 << bit) != 0);
    let bitstring = Outcome::from_indices(n, indices).to_bitstring(n);
    match CString::new(bitstring) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Computes the probability distribution and returns the number of entries,
/// or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_calc_probabilities_len(ptr: *mut CExecutionResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.calc_probabilities();
    wrapper
        .inner
        .probabilities()
        .as_ref()
        .map_or(0, |probs| probs.len())
}

/// Computes the probability distribution and copies the probabilities into
/// `out` (two-step pattern; pair with
/// `execution_result_calc_probabilities_len`).
///
/// Entries are sorted ascending by outcome value, where bit `i` of the
/// outcome value is measured qubit `i` (matching
/// `execution_result_to_bitstring`). Returns the total number of entries, or
/// 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn execution_result_calc_probabilities(
    ptr: *mut CExecutionResult,
    out: *mut f64,
    len: usize,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let wrapper = unsafe { &mut *ptr };
    wrapper.inner.calc_probabilities();
    let Some(probs) = wrapper.inner.probabilities() else {
        return 0;
    };
    // Sort keys ascending in big-endian chunk order (bit i = qubit i). Pad
    // all keys to a common chunk width so mismatched storage widths still
    // compare numerically.
    let mut entries: Vec<(Vec<u64>, f64)> = probs
        .iter()
        .map(|(outcome, probability)| {
            (
                outcome.0.iter().copied().collect::<Vec<u64>>(),
                *probability,
            )
        })
        .collect();
    let width = entries
        .iter()
        .map(|(chunks, _)| chunks.len())
        .max()
        .unwrap_or(0);
    for (chunks, _) in &mut entries {
        chunks.resize(width, 0);
        chunks.reverse();
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    if !out.is_null() {
        for (i, (_, probability)) in entries.iter().take(entries.len().min(len)).enumerate() {
            unsafe { *out.add(i) = *probability };
        }
    }
    entries.len()
}
