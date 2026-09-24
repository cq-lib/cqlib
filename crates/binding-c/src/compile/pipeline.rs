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

//! C ABI entry point for the compiler pipeline.

use crate::circuit::CCircuit;
use crate::compile::{CCompileResult, COMPILE_MODE_ENHANCED, COMPILE_MODE_NORMAL, CompileConfigC};
use crate::device::{CDevice, CLayout};
use crate::error::CqlibError;
use cqlib_core::compile::resource::ResourcePolicy;
use cqlib_core::compile::{
    CompileConfig, CompileMode, CompileTarget, DeviceCompileTarget, compile as cqlib_compile,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Run the configured compiler workflow over `circuit`.
///
/// `config.mode` selects `Normal` or `Enhanced`. `config.target` selects
/// `Logical`; other targets fall back to logical compilation (the C ABI
/// does not parse `Instruction` lists or `Device` objects).
///
/// Returns a heap-allocated `CCompileResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn compile(circuit: *const CCircuit, config: CompileConfigC) -> *mut CCompileResult {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let mode = match config.mode {
        0 => CompileMode::Normal,
        1 => CompileMode::Enhanced,
        _ => return std::ptr::null_mut(),
    };
    let target = match config.target {
        0 => Some(CompileTarget::Logical),
        _ => None,
    };
    let target = target.unwrap_or(CompileTarget::Logical);
    let cfg = CompileConfig {
        mode,
        target,
        resource_policy: ResourcePolicy::default(),
    };
    let circuit = unsafe { &(*circuit).inner };
    match cqlib_compile(circuit, cfg) {
        Ok(result) => Box::into_raw(Box::new(CCompileResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a compile result. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_free(ptr: *mut CCompileResult) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Wrap a `Circuit*` around the optimized circuit stored in `result`.
///
/// The returned circuit is a deep clone of the result's circuit, so the
/// caller can free `result` independently. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_circuit(ptr: *const CCompileResult) -> *mut CCircuit {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    Box::into_raw(Box::new(CCircuit {
        inner: result.inner.circuit.clone(),
    }))
}

/// Return 1 if the workflow changed the circuit, 0 otherwise, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_changed(ptr: *const CCompileResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    if result.inner.changed { 1 } else { 0 }
}

/// Return the compile mode (0 = Normal, 1 = Enhanced), or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_mode(ptr: *const CCompileResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    match result.inner.mode {
        CompileMode::Normal => 0,
        CompileMode::Enhanced => 1,
    }
}

/// Return the number of step reports in the result, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_num_steps(ptr: *const CCompileResult) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let result = unsafe { &*ptr };
    result.inner.steps.len()
}

/// Return the name of step `index` as a heap-allocated C string.
/// Caller must free with `cqlib_string_free`. Returns NULL on out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_step_name(
    ptr: *const CCompileResult,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match result.inner.steps.get(index) {
        Some(step) => match CString::new(step.name) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Return 1 if step `index` changed the circuit, 0 otherwise, or -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_step_changed(ptr: *const CCompileResult, index: usize) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    match result.inner.steps.get(index) {
        Some(step) => {
            if step.changed && !step.skipped {
                1
            } else {
                0
            }
        }
        None => CqlibError::InvalidParam as i32,
    }
}

/// Return the optional skip or configuration note of step `index` as a
/// heap-allocated C string (free with `cqlib_string_free`).
///
/// Returns NULL when the step record carries no reason, on NULL input, or on
/// an out-of-bounds index.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_step_reason(
    ptr: *const CCompileResult,
    index: usize,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match result
        .inner
        .steps
        .get(index)
        .and_then(|step| step.reason.as_deref())
    {
        Some(reason) => match CString::new(reason) {
            Ok(text) => text.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Find the first workflow step report named `name` (the C mirror of
/// `CompileResult::step`).
///
/// Writes the step's raw `changed` and `skipped` flags to the out pointers.
/// Returns 1 when a matching step exists, 0 when no step matches, or -1 on
/// NULL arguments or invalid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_step(
    ptr: *const CCompileResult,
    name: *const c_char,
    out_changed: *mut i32,
    out_skipped: *mut i32,
) -> i32 {
    if ptr.is_null() || name.is_null() || out_changed.is_null() || out_skipped.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let result = unsafe { &*ptr };
    match result.inner.step(name) {
        Some(step) => {
            unsafe {
                *out_changed = step.changed as i32;
                *out_skipped = step.skipped as i32;
            }
            1
        }
        None => 0,
    }
}

/// Run the configured compiler workflow for a concrete device target.
///
/// This is the device-target counterpart of [`compile`]: the circuit is
/// routed on the device topology and lowered to the device's ordered native
/// capabilities, so the returned circuit is always accepted by `device`.
///
/// - `mode` is one of the `COMPILE_MODE_*` tags.
/// - `initial_layout` optionally pins the initial logical-to-physical
///   mapping (NULL lets the workflow choose one).
/// - `seed` negative means "no seed" (heuristic default); otherwise it is
///   used as the deterministic device layout/routing seed.
///
/// Returns a heap-allocated `CCompileResult*`, or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn compile_with_device(
    circuit: *const CCircuit,
    mode: u8,
    device: *const CDevice,
    initial_layout: *const CLayout,
    seed: i64,
) -> *mut CCompileResult {
    if circuit.is_null() || device.is_null() {
        return std::ptr::null_mut();
    }
    let mode = match mode {
        COMPILE_MODE_NORMAL => CompileMode::Normal,
        COMPILE_MODE_ENHANCED => CompileMode::Enhanced,
        _ => return std::ptr::null_mut(),
    };
    let initial_layout = if initial_layout.is_null() {
        None
    } else {
        Some(unsafe { (*initial_layout).inner.clone() })
    };
    let target = CompileTarget::Device(DeviceCompileTarget {
        device: unsafe { (*device).inner.clone() },
        initial_layout,
        seed: u32::try_from(seed).ok(),
    });
    let cfg = CompileConfig {
        mode,
        target,
        resource_policy: ResourcePolicy::default(),
    };
    let circuit = unsafe { &(*circuit).inner };
    match cqlib_compile(circuit, cfg) {
        Ok(result) => Box::into_raw(Box::new(CCompileResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return the initial logical-to-physical layout recorded by a device-target
/// compilation, as an owned `CLayout*` (free with `layout_free`).
///
/// Returns NULL when the result carries no device metadata (for example
/// after a logical-target compile) or on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_initial_layout(ptr: *const CCompileResult) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match &result.inner.device_metadata {
        Some(metadata) => Box::into_raw(Box::new(CLayout {
            inner: metadata.initial_layout.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

/// Return the final logical-to-physical layout recorded by a device-target
/// compilation, as an owned `CLayout*` (free with `layout_free`).
///
/// Returns NULL when the result carries no device metadata (for example
/// after a logical-target compile) or on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_final_layout(ptr: *const CCompileResult) -> *mut CLayout {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let result = unsafe { &*ptr };
    match &result.inner.device_metadata {
        Some(metadata) => Box::into_raw(Box::new(CLayout {
            inner: metadata.final_layout.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}
