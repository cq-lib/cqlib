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
use crate::compile::{CCompileResult, CompileConfigC};
use crate::error::CqlibError;
use cqlib_core::compile::resource::ResourcePolicy;
use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile as cqlib_compile};
use std::ffi::CString;
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
