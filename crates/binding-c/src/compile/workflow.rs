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

//! C ABI for the reusable compiler workflow and device compilation metadata.

use crate::circuit::CCircuit;
use crate::compile::{
    CCompileResult, COMPILE_MODE_ENHANCED, COMPILE_MODE_NORMAL, COMPILE_TARGET_LOGICAL,
    CompileConfigC,
};
use crate::device::CLayout;
use crate::error::CqlibError;
use cqlib_core::compile::resource::ResourcePolicy;
use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, CompilerWorkflow};

/// Opaque handle around a [`CompilerWorkflow`].
pub struct CCompilerWorkflow {
    pub inner: CompilerWorkflow,
}

/// Create a reusable compiler workflow from a C configuration.
///
/// `config.mode` selects `Normal` or `Enhanced`. `config.target` only
/// supports `COMPILE_TARGET_LOGICAL`; other targets fall back to NULL
/// because the C ABI does not parse `Instruction` lists or `Device`
/// objects. Target preparation is validated eagerly (the mirror of
/// `CompilerWorkflow::try_new`), so an invalid configuration returns NULL.
#[unsafe(no_mangle)]
pub extern "C" fn compiler_workflow_new(config: CompileConfigC) -> *mut CCompilerWorkflow {
    let mode = match config.mode {
        COMPILE_MODE_NORMAL => CompileMode::Normal,
        COMPILE_MODE_ENHANCED => CompileMode::Enhanced,
        _ => return std::ptr::null_mut(),
    };
    let target = match config.target {
        COMPILE_TARGET_LOGICAL => CompileTarget::Logical,
        _ => return std::ptr::null_mut(),
    };
    let cfg = CompileConfig {
        mode,
        target,
        resource_policy: ResourcePolicy::default(),
    };
    match CompilerWorkflow::try_new(cfg) {
        Ok(workflow) => Box::into_raw(Box::new(CCompilerWorkflow { inner: workflow })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a compiler workflow. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn compiler_workflow_free(ptr: *mut CCompilerWorkflow) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Run the workflow over `circuit` and return a heap-allocated compile
/// result.
///
/// The workflow can be run repeatedly on different circuits; the input
/// circuit is never modified. Returns a `CCompileResult*` (free with
/// `compile_result_free`), or NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn compiler_workflow_run(
    workflow: *const CCompilerWorkflow,
    circuit: *const CCircuit,
) -> *mut CCompileResult {
    if workflow.is_null() || circuit.is_null() {
        return std::ptr::null_mut();
    }
    let workflow = unsafe { &(*workflow).inner };
    let circuit = unsafe { &(*circuit).inner };
    match workflow.run(circuit) {
        Ok(result) => Box::into_raw(Box::new(CCompileResult { inner: result })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return 1 when the result carries device compilation metadata (a
/// device-topology compile), 0 when it does not (for example after a
/// logical-target compile), or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_has_device_metadata(ptr: *const CCompileResult) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    if result.inner.device_metadata.is_some() {
        1
    } else {
        0
    }
}

/// Return the initial logical-to-physical layout recorded by a
/// device-target compilation, as an owned `CLayout*` (free with
/// `layout_free`).
///
/// Returns NULL when the result carries no device metadata (for example
/// after a logical-target compile) or on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_device_metadata_initial_layout(
    ptr: *const CCompileResult,
) -> *mut CLayout {
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
pub extern "C" fn compile_result_device_metadata_final_layout(
    ptr: *const CCompileResult,
) -> *mut CLayout {
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

/// Return the number of entries in the recorded virtual output permutation
/// (original output qubit -> rewritten output qubit), or 0 for NULL input
/// or a result without device metadata.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_device_metadata_permutation_len(
    ptr: *const CCompileResult,
) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let result = unsafe { &*ptr };
    match &result.inner.device_metadata {
        Some(metadata) => metadata
            .virtual_permutation
            .original_output_to_rewritten_output()
            .len(),
        None => 0,
    }
}

/// Copy the virtual output permutation into the `original` and `rewritten`
/// buffers as `uint32_t` qubit IDs, in ascending original-output order.
///
/// Each index `i` records that the state of original output
/// `original[i]` is carried by rewritten output `rewritten[i]`. `len` must
/// equal `compile_result_device_metadata_permutation_len(ptr)`.
///
/// Returns 0 on success (including a no-metadata result with len 0), -1 on
/// NULL arguments, or -8 on a length mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn compile_result_device_metadata_permutation(
    ptr: *const CCompileResult,
    original: *mut u32,
    rewritten: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let result = unsafe { &*ptr };
    let permutation = match &result.inner.device_metadata {
        Some(metadata) => metadata
            .virtual_permutation
            .original_output_to_rewritten_output(),
        None => {
            return if len == 0 {
                0
            } else {
                CqlibError::InvalidParam as i32
            };
        }
    };
    if permutation.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    if len > 0 && (original.is_null() || rewritten.is_null()) {
        return CqlibError::NullPtr as i32;
    }
    for (i, (from, to)) in permutation.iter().enumerate() {
        unsafe {
            *original.add(i) = from.id();
            *rewritten.add(i) = to.id();
        }
    }
    0
}
