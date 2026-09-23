//! Integration tests for compile module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new, circuit_num_qubits};
use binding_c::compile::*;
use binding_c::cqlib_string_free;
use std::ffi::CStr;
use std::os::raw::c_char;

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn build_bell_circuit() -> *mut binding_c::circuit::CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    circuit
}

#[test]
fn test_compile_logical_normal() {
    let circuit = build_bell_circuit();
    let config = CompileConfigC {
        mode: COMPILE_MODE_NORMAL,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };

    let result = compile(circuit, config);
    assert!(!result.is_null(), "compile should succeed");

    assert!(compile_result_num_steps(result) > 0);

    let mode = compile_result_mode(result);
    assert_eq!(mode, 0);

    let changed = compile_result_changed(result);
    assert!(changed == 0 || changed == 1);

    let optimized = compile_result_circuit(result);
    assert!(!optimized.is_null());
    assert_eq!(circuit_num_qubits(optimized), 2);

    circuit_free(optimized);
    compile_result_free(result);
    circuit_free(circuit);
}

#[test]
fn test_compile_enhanced_mode() {
    let circuit = build_bell_circuit();
    let config = CompileConfigC {
        mode: COMPILE_MODE_ENHANCED,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };

    let result = compile(circuit, config);
    assert!(!result.is_null());

    assert_eq!(compile_result_mode(result), 1);
    compile_result_free(result);
    circuit_free(circuit);
}

#[test]
fn test_compile_null_circuit() {
    let config = CompileConfigC {
        mode: COMPILE_MODE_NORMAL,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };
    let result = compile(std::ptr::null(), config);
    assert!(result.is_null(), "NULL circuit should return NULL");
}

#[test]
fn test_compile_result_step_name() {
    let circuit = build_bell_circuit();
    let config = CompileConfigC {
        mode: COMPILE_MODE_NORMAL,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };
    let result = compile(circuit, config);
    assert!(!result.is_null());

    let n = compile_result_num_steps(result);
    assert!(n > 0);
    for i in 0..n {
        let ptr = compile_result_step_name(result, i);
        let name = cstr_to_string(ptr);
        cqlib_string_free(ptr);
        assert!(!name.is_empty(), "step {i} should have a non-empty name");
    }

    let bad = compile_result_step_name(result, n + 10);
    assert!(bad.is_null(), "out-of-range index should return NULL");

    compile_result_free(result);
    circuit_free(circuit);
}

#[test]
fn test_compile_result_step_changed() {
    let circuit = build_bell_circuit();
    let config = CompileConfigC {
        mode: COMPILE_MODE_NORMAL,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };
    let result = compile(circuit, config);
    assert!(!result.is_null());

    let n = compile_result_num_steps(result);
    for i in 0..n {
        let v = compile_result_step_changed(result, i);
        assert!(v == 0 || v == 1);
    }

    assert_eq!(
        compile_result_step_changed(result, n + 10),
        -8,
        "out-of-range index should return InvalidParam"
    );

    compile_result_free(result);
    circuit_free(circuit);
}

#[test]
fn test_compile_result_free_null() {
    compile_result_free(std::ptr::null_mut());
}

#[test]
fn test_compile_result_accessors_null() {
    assert_eq!(compile_result_changed(std::ptr::null()), -1);
    assert_eq!(compile_result_mode(std::ptr::null()), -1);
    assert_eq!(compile_result_num_steps(std::ptr::null()), 0);
    assert!(compile_result_step_name(std::ptr::null(), 0).is_null());
    assert_eq!(compile_result_step_changed(std::ptr::null(), 0), -1);
    assert!(compile_result_circuit(std::ptr::null()).is_null());
}
