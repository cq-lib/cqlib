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

//! C ABI for gate-commutation proofs: shared builtin and algebraic-only
//! checks plus configurable reusable checkers.
//!
//! Gates are described by a standard-gate name (as accepted elsewhere by the
//! C ABI, e.g. `"H"`, `"RZ"`, `"CX"`), a qubit-id array, and a numeric
//! parameter array. Proof results are conservative: tag 0 only means the
//! configured proof sources could not establish commutation.

use crate::compile::knowledge::CKnowledgeLibrary;
use crate::device::standard_gate_from_name;
use crate::error::CqlibError;
use cqlib_core::circuit::{Instruction, Parameter, Qubit};
use cqlib_core::compile::commutation::{
    Commutation, CommutationChecker, CommutationConfig, CommutationResult, algebraic_commutation,
    check_commutation,
};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Commutation proof tag.
/// | Value | Proof                                          |
/// |-------|------------------------------------------------|
/// | 0     | Not provable with the configured proof sources |
/// | 1     | Exact commutation                              |
/// | 2     | Commutation up to a global phase               |
pub const COMMUTATION_RESULT_UNPROVEN: i32 = 0;
pub const COMMUTATION_RESULT_EXACT: i32 = 1;
pub const COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE: i32 = 2;

/// C form of [`CommutationConfig`].
#[repr(C)]
pub struct CCommutationConfig {
    /// 1 enables matching explicit knowledge-base commutation rules.
    pub enable_rule_oracle: u8,
    /// 1 enables small local matrix comparison as a fallback.
    pub enable_matrix_fallback: u8,
    /// Reserved padding to keep the struct layout stable.
    pub _pad: [u8; 6],
    /// Maximum union-support size for the matrix fallback.
    pub max_matrix_qubits: usize,
    /// Reserved padding to keep the struct layout stable.
    pub _reserved: [u64; 2],
}

/// Opaque handle around a [`CommutationChecker`].
pub struct CCommutationChecker {
    pub inner: CommutationChecker,
}

// =====  Internal helpers  =====

/// A parsed gate application (instruction, qubits, numeric parameters).
struct GateApplication {
    instruction: Instruction,
    qubits: Vec<Qubit>,
    params: Vec<Parameter>,
}

/// Parses one gate application from its C arguments.
///
/// Returns -1 for NULL input, -4 for invalid UTF-8, and -8 for an unknown
/// standard-gate name.
fn parse_gate_application(
    gate_name: *const c_char,
    qubits: *const u32,
    qubits_len: usize,
    params: *const f64,
    params_len: usize,
) -> Result<GateApplication, i32> {
    if gate_name.is_null() {
        return Err(CqlibError::NullPtr as i32);
    }
    let name = match unsafe { CStr::from_ptr(gate_name) }.to_str() {
        Ok(name) => name,
        Err(_) => return Err(CqlibError::ParseError as i32),
    };
    let gate = match standard_gate_from_name(name) {
        Some(gate) => gate,
        None => return Err(CqlibError::InvalidParam as i32),
    };
    let mut qubit_list = Vec::with_capacity(qubits_len);
    if qubits_len > 0 {
        if qubits.is_null() {
            return Err(CqlibError::NullPtr as i32);
        }
        for &id in unsafe { std::slice::from_raw_parts(qubits, qubits_len) } {
            qubit_list.push(Qubit::new(id));
        }
    }
    let mut param_list = Vec::with_capacity(params_len);
    if params_len > 0 {
        if params.is_null() {
            return Err(CqlibError::NullPtr as i32);
        }
        for &value in unsafe { std::slice::from_raw_parts(params, params_len) } {
            param_list.push(Parameter::from(value));
        }
    }
    Ok(GateApplication {
        instruction: Instruction::Standard(gate),
        qubits: qubit_list,
        params: param_list,
    })
}

/// Writes `value` to `phase` when it is non-NULL.
fn write_phase(phase: *mut f64, value: f64) {
    if !phase.is_null() {
        unsafe { *phase = value };
    }
}

/// Converts a commutation proof into its `COMMUTATION_RESULT_*` tag and,
/// when `phase` is non-NULL, writes the proof's global phase (0.0 for exact
/// proofs). A symbolic phase that cannot be evaluated numerically surfaces as
/// NaN.
fn commutation_to_tag(result: CommutationResult, phase: *mut f64) -> i32 {
    match result {
        None => COMMUTATION_RESULT_UNPROVEN,
        Some(Commutation::Exact) => {
            write_phase(phase, 0.0);
            COMMUTATION_RESULT_EXACT
        }
        Some(Commutation::UpToGlobalPhase(parameter)) => {
            let value = parameter.evaluate(&None).unwrap_or(f64::NAN);
            write_phase(phase, value);
            COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE
        }
    }
}

/// Converts a C commutation configuration into the core form.
fn commutation_config_from_c(raw: &CCommutationConfig) -> CommutationConfig {
    CommutationConfig {
        enable_rule_oracle: raw.enable_rule_oracle != 0,
        enable_matrix_fallback: raw.enable_matrix_fallback != 0,
        max_matrix_qubits: raw.max_matrix_qubits,
    }
}

// =====  One-shot checks  =====

/// Checks whether two concrete gate applications commute using the shared
/// builtin checker (knowledge rules and matrix fallback enabled).
///
/// Each side is described by a standard-gate name, a qubit-id array, and a
/// numeric parameter array. When the proof is `UpToGlobalPhase`, the phase is
/// written to `phase` (may be NULL). Returns one of
/// `COMMUTATION_RESULT_*`, -1 on NULL input, -4 on invalid UTF-8, and -8 on
/// an unknown gate name.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub extern "C" fn commutation_check(
    lhs_gate: *const c_char,
    lhs_qubits: *const u32,
    lhs_qubits_len: usize,
    lhs_params: *const f64,
    lhs_params_len: usize,
    rhs_gate: *const c_char,
    rhs_qubits: *const u32,
    rhs_qubits_len: usize,
    rhs_params: *const f64,
    rhs_params_len: usize,
    phase: *mut f64,
) -> i32 {
    let lhs = match parse_gate_application(
        lhs_gate,
        lhs_qubits,
        lhs_qubits_len,
        lhs_params,
        lhs_params_len,
    ) {
        Ok(lhs) => lhs,
        Err(code) => return code,
    };
    let rhs = match parse_gate_application(
        rhs_gate,
        rhs_qubits,
        rhs_qubits_len,
        rhs_params,
        rhs_params_len,
    ) {
        Ok(rhs) => rhs,
        Err(code) => return code,
    };
    commutation_to_tag(
        check_commutation(
            &lhs.instruction,
            &lhs.qubits,
            &lhs.params,
            &rhs.instruction,
            &rhs.qubits,
            &rhs.params,
        ),
        phase,
    )
}

/// Checks commutation using only the symbolic algebraic proof sources
/// (structural facts, diagonal/axis algebra, controlled-axis and symmetric
/// two-qubit families).
///
/// Arguments and return values match [`commutation_check`]; tag 0 keeps the
/// result conservative so rule or matrix fallbacks can still run.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub extern "C" fn commutation_check_algebraic(
    lhs_gate: *const c_char,
    lhs_qubits: *const u32,
    lhs_qubits_len: usize,
    lhs_params: *const f64,
    lhs_params_len: usize,
    rhs_gate: *const c_char,
    rhs_qubits: *const u32,
    rhs_qubits_len: usize,
    rhs_params: *const f64,
    rhs_params_len: usize,
    phase: *mut f64,
) -> i32 {
    let lhs = match parse_gate_application(
        lhs_gate,
        lhs_qubits,
        lhs_qubits_len,
        lhs_params,
        lhs_params_len,
    ) {
        Ok(lhs) => lhs,
        Err(code) => return code,
    };
    let rhs = match parse_gate_application(
        rhs_gate,
        rhs_qubits,
        rhs_qubits_len,
        rhs_params,
        rhs_params_len,
    ) {
        Ok(rhs) => rhs,
        Err(code) => return code,
    };
    commutation_to_tag(
        algebraic_commutation(
            &lhs.instruction,
            &lhs.qubits,
            &lhs.params,
            &rhs.instruction,
            &rhs.qubits,
            &rhs.params,
        ),
        phase,
    )
}

// =====  Reusable checker handles  =====

/// Returns the default commutation configuration by value.
#[unsafe(no_mangle)]
pub extern "C" fn commutation_config_default() -> CCommutationConfig {
    let config = CommutationConfig::default();
    CCommutationConfig {
        enable_rule_oracle: config.enable_rule_oracle as u8,
        enable_matrix_fallback: config.enable_matrix_fallback as u8,
        _pad: [0; 6],
        max_matrix_qubits: config.max_matrix_qubits,
        _reserved: [0; 2],
    }
}

/// Creates a reusable checker with builtin commutation rules and an explicit
/// configuration. Free with `commutation_checker_free`.
#[unsafe(no_mangle)]
pub extern "C" fn commutation_checker_new(config: CCommutationConfig) -> *mut CCommutationChecker {
    let inner = CommutationChecker::with_config(commutation_config_from_c(&config));
    Box::into_raw(Box::new(CCommutationChecker { inner }))
}

/// Creates a reusable checker that draws commutation rules from an already
/// loaded knowledge library.
///
/// Returns a heap-allocated `CCommutationChecker*` (free with
/// `commutation_checker_free`), or NULL on NULL input.
#[unsafe(no_mangle)]
pub extern "C" fn commutation_checker_from_library(
    library: *const CKnowledgeLibrary,
    config: CCommutationConfig,
) -> *mut CCommutationChecker {
    if library.is_null() {
        return std::ptr::null_mut();
    }
    let library = unsafe { &(*library).inner };
    let inner = CommutationChecker::from_library(library, commutation_config_from_c(&config));
    Box::into_raw(Box::new(CCommutationChecker { inner }))
}

/// Frees a commutation checker. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn commutation_checker_free(ptr: *mut CCommutationChecker) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Checks whether two concrete gate applications commute using this checker.
///
/// Arguments and return values match [`commutation_check`]; the checker's
/// configuration selects which proof sources participate.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub extern "C" fn commutation_checker_check(
    ptr: *const CCommutationChecker,
    lhs_gate: *const c_char,
    lhs_qubits: *const u32,
    lhs_qubits_len: usize,
    lhs_params: *const f64,
    lhs_params_len: usize,
    rhs_gate: *const c_char,
    rhs_qubits: *const u32,
    rhs_qubits_len: usize,
    rhs_params: *const f64,
    rhs_params_len: usize,
    phase: *mut f64,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let lhs = match parse_gate_application(
        lhs_gate,
        lhs_qubits,
        lhs_qubits_len,
        lhs_params,
        lhs_params_len,
    ) {
        Ok(lhs) => lhs,
        Err(code) => return code,
    };
    let rhs = match parse_gate_application(
        rhs_gate,
        rhs_qubits,
        rhs_qubits_len,
        rhs_params,
        rhs_params_len,
    ) {
        Ok(rhs) => rhs,
        Err(code) => return code,
    };
    let checker = unsafe { &(*ptr).inner };
    commutation_to_tag(
        checker.check(
            &lhs.instruction,
            &lhs.qubits,
            &lhs.params,
            &rhs.instruction,
            &rhs.qubits,
            &rhs.params,
        ),
        phase,
    )
}
