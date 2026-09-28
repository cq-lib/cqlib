// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Rust FFI tests for standalone circuit-operation handles
//! (`circuit/operations.rs`).

use binding_c::circuit::{
    COperation, OPERATION_PARAM_FIXED, OPERATION_PARAM_INDEX, operation_free,
    operation_is_standard_gate, operation_label, operation_matrix, operation_matrix_dims,
    operation_matrix_len, operation_name, operation_new, operation_num_qubits, operation_params,
    operation_params_len, operation_qubits, operation_qubits_len,
};
use binding_c::cqlib_string_free;
use cqlib_core::circuit::gate::instruction::Instruction;
use cqlib_core::circuit::gate::standard_gate::StandardGate;
use cqlib_core::circuit::{CircuitParam, Operation, Qubit};
use std::ffi::{CStr, CString};

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn fixed_param_operation_roundtrip() {
    // RX(0.5) acting on qubit 0.
    let name = cstr("RX");
    let qubits = [0u32];
    let params = [0.5f64];
    let op = operation_new(name.as_ptr(), qubits.as_ptr(), 1, params.as_ptr(), 1);
    assert!(!op.is_null());

    // Instruction name.
    let text = operation_name(op);
    assert!(!text.is_null());
    assert_eq!(unsafe { CStr::from_ptr(text) }.to_str().unwrap(), "RX");
    cqlib_string_free(text);

    // Standard gate, arity, and qubit list.
    assert_eq!(operation_is_standard_gate(op), 1);
    assert_eq!(operation_num_qubits(op), 1);
    assert_eq!(operation_qubits_len(op), 1);
    let mut ids = [9u32];
    assert_eq!(operation_qubits(op, ids.as_mut_ptr(), 1), 0);
    assert_eq!(ids, [0]);

    // Parameters: fixed value written inline.
    assert_eq!(operation_params_len(op), 1);
    let mut tags = [255u32];
    let mut values = [0.0f64];
    assert_eq!(
        operation_params(op, tags.as_mut_ptr(), values.as_mut_ptr(), 1),
        0
    );
    assert_eq!(tags, [OPERATION_PARAM_FIXED]);
    assert!((values[0] - 0.5).abs() < 1e-12);

    // Numeric unitary: 2x2 complex elements.
    assert_eq!(operation_matrix_len(op), 4);
    let mut rows = 0usize;
    let mut cols = 0usize;
    assert_eq!(operation_matrix_dims(op, &mut rows, &mut cols), 0);
    assert_eq!((rows, cols), (2, 2));
    let mut out = [0.0f64; 8];
    assert_eq!(operation_matrix(op, out.as_mut_ptr(), out.len()), 4);
    // RX(0.5) = [[cos(0.25), -i*sin(0.25)], [-i*sin(0.25), cos(0.25)]].
    let c = 0.25f64.cos();
    let s = 0.25f64.sin();
    assert!((out[0] - c).abs() < 1e-12);
    assert!(out[1].abs() < 1e-12);
    assert!(out[2].abs() < 1e-12);
    assert!((out[3] + s).abs() < 1e-12);
    assert!(out[4].abs() < 1e-12);
    assert!((out[5] + s).abs() < 1e-12);
    assert!((out[6] - c).abs() < 1e-12);
    assert!(out[7].abs() < 1e-12);

    // Operations built by `operation_new` never carry a label.
    assert!(operation_label(op).is_null());

    // Buffer too small is rejected everywhere.
    assert_eq!(operation_qubits(op, ids.as_mut_ptr(), 0), -8);
    assert_eq!(
        operation_params(op, tags.as_mut_ptr(), values.as_mut_ptr(), 0),
        -8
    );
    assert_eq!(operation_matrix(op, out.as_mut_ptr(), 7), 0);

    operation_free(op);
}

#[test]
fn parameterless_gate_operation() {
    // CX with an empty parameter list.
    let name = cstr("CX");
    let qubits = [0u32, 1];
    let op = operation_new(name.as_ptr(), qubits.as_ptr(), 2, std::ptr::null(), 0);
    assert!(!op.is_null());

    assert_eq!(operation_qubits_len(op), 2);
    let mut ids = [0u32; 2];
    assert_eq!(operation_qubits(op, ids.as_mut_ptr(), 2), 0);
    assert_eq!(ids, [0, 1]);

    // Zero parameters: `operation_params` succeeds with len = 0.
    assert_eq!(operation_params_len(op), 0);
    assert_eq!(
        operation_params(op, std::ptr::null_mut(), std::ptr::null_mut(), 0),
        0
    );

    // The 4x4 matrix is returned as 16 interleaved doubles.
    assert_eq!(operation_matrix_len(op), 16);
    let mut out = [0.0f64; 32];
    assert_eq!(operation_matrix(op, out.as_mut_ptr(), out.len()), 16);

    operation_free(op);
}

#[test]
fn index_params_report_parameter_table_index() {
    // An operation whose parameter refers to a parameter-table index (as
    // produced by `circuit_rx_param`) reports the INDEX tag and carries the
    // index as the payload value. Such handles only originate from
    // expanded circuit bodies; construct one directly here.
    let op = Box::into_raw(Box::new(COperation {
        inner: Operation {
            instruction: Instruction::Standard(StandardGate::RX),
            qubits: vec![Qubit::new(0)].into(),
            params: vec![CircuitParam::Index(3)].into(),
            label: None,
        },
    }));
    assert_eq!(operation_is_standard_gate(op), 1);

    let mut tags = [255u32];
    let mut values = [0.0f64];
    assert_eq!(operation_params_len(op), 1);
    assert_eq!(
        operation_params(op, tags.as_mut_ptr(), values.as_mut_ptr(), 1),
        0
    );
    assert_eq!(tags, [OPERATION_PARAM_INDEX]);
    assert_eq!(values[0], 3.0);

    // An unresolved index parameter has no numeric matrix.
    assert_eq!(operation_matrix_len(op), 0);
    let mut rows = 1usize;
    let mut cols = 1usize;
    assert_eq!(operation_matrix_dims(op, &mut rows, &mut cols), -3);

    operation_free(op);
}

#[test]
fn operation_new_error_codes() {
    // NULL gate name.
    let qubits = [0u32];
    assert!(operation_new(std::ptr::null(), qubits.as_ptr(), 1, std::ptr::null(), 0).is_null());

    // Unknown gate name.
    let bogus = cstr("NOT_A_GATE");
    assert!(operation_new(bogus.as_ptr(), std::ptr::null(), 0, std::ptr::null(), 0).is_null());

    // Non-null qubit length with a NULL buffer.
    let h = cstr("H");
    assert!(operation_new(h.as_ptr(), std::ptr::null(), 1, std::ptr::null(), 0).is_null());

    // Non-null parameter length with a NULL buffer.
    assert!(operation_new(h.as_ptr(), qubits.as_ptr(), 1, std::ptr::null(), 1).is_null());
}

#[test]
fn accessors_tolerate_null_handles() {
    assert!(operation_name(std::ptr::null()).is_null());
    assert_eq!(operation_is_standard_gate(std::ptr::null()), -1);
    assert_eq!(operation_num_qubits(std::ptr::null()), 0);
    assert_eq!(operation_qubits_len(std::ptr::null()), 0);
    assert_eq!(
        operation_qubits(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(operation_params_len(std::ptr::null()), 0);
    assert_eq!(
        operation_params(
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ),
        -1
    );
    assert!(operation_label(std::ptr::null()).is_null());
    assert_eq!(operation_matrix_len(std::ptr::null()), 0);
    assert_eq!(
        operation_matrix_dims(std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut()),
        -1
    );
    assert_eq!(
        operation_matrix(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );

    // Buffer arguments may not be NULL on success paths.
    let name = cstr("H");
    let qubits = [0u32];
    let op = operation_new(name.as_ptr(), qubits.as_ptr(), 1, std::ptr::null(), 0);
    assert!(!op.is_null());
    assert_eq!(operation_qubits(op, std::ptr::null_mut(), 1), -1);
    let mut rows = 0usize;
    let mut cols = 0usize;
    assert_eq!(
        operation_matrix_dims(op, std::ptr::null_mut(), &mut cols),
        -1
    );
    assert_eq!(
        operation_matrix_dims(op, &mut rows, std::ptr::null_mut()),
        -1
    );
    assert_eq!(operation_matrix(op, std::ptr::null_mut(), 8), 0);

    operation_free(op);
    operation_free(std::ptr::null_mut());
}
