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

//! Rust FFI tests for the circuit parameter-metadata and advanced-instruction
//! bindings (`circuit/advanced.rs`, checklist sections 1.4 and 1.5).

use binding_c::circuit::{
    CCircuit, CCircuitParam, CIRCUIT_PARAM_TAG_FIXED, CIRCUIT_PARAM_TAG_INDEX, CParameter,
    CParameterValue, PARAMETER_VALUE_TAG_FIXED, PARAMETER_VALUE_TAG_PARAM, circuit_add_parameter,
    circuit_assign_params, circuit_ccx, circuit_circuit_gate, circuit_cx, circuit_delay,
    circuit_delay_param, circuit_free, circuit_from_qubits, circuit_gate_free, circuit_gate_name,
    circuit_gate_num_params, circuit_gate_num_qubits, circuit_gate_signature_params,
    circuit_gate_signature_params_len, circuit_global_phase, circuit_h, circuit_id, circuit_index,
    circuit_map_param, circuit_multi_control, circuit_new, circuit_num_operations,
    circuit_num_parameters, circuit_num_qubits, circuit_parameter_value, circuit_parameters,
    circuit_parameters_len, circuit_phase, circuit_qubits, circuit_remove_operations,
    circuit_resolve_parameter, circuit_rx, circuit_rx_param, circuit_set_global_phase,
    circuit_set_global_phase_param, circuit_symbols, circuit_symbols_len, circuit_to_gate,
    circuit_to_matrix, circuit_to_matrix_len, circuit_unitary, circuit_unitary_with_params,
    circuit_used_symbols, circuit_used_symbols_len, circuit_uses_symbol, circuit_validate,
    circuit_x, param_evaluate, param_free, param_parse, value_operation_free,
    value_operation_instruction_type, value_operation_label, value_operation_name,
    value_operation_num_params, value_operation_num_qubits, value_operation_param,
    value_operation_qubits,
};
use binding_c::cqlib_string_free;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn matrix_of(circuit: *const CCircuit) -> Vec<f64> {
    let len = circuit_to_matrix_len(circuit, ptr::null(), 0);
    assert!(len > 0);
    let mut buf = vec![0.0f64; len * 2];
    assert_eq!(
        circuit_to_matrix(circuit, ptr::null(), 0, buf.as_mut_ptr(), buf.len()),
        len
    );
    buf
}

fn assert_matrices_close(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert!((x - y).abs() < 1e-12, "matrix mismatch: {x} vs {y}");
    }
}

#[test]
fn parameter_metadata_roundtrip() {
    let theta = cstr("theta");
    let theta_p = param_parse(theta.as_ptr());
    assert!(!theta_p.is_null());

    let circuit = circuit_new(1);

    // Interning a parameter registers it and its symbols, but does not mark
    // it as used by the executable IR.
    assert_eq!(circuit_add_parameter(circuit, theta_p), 0);
    assert_eq!(circuit_parameters_len(circuit), 1);
    assert_eq!(circuit_num_parameters(circuit), 1);
    assert_eq!(circuit_symbols_len(circuit), 1);
    assert_eq!(circuit_used_symbols_len(circuit), 0);
    assert_eq!(circuit_uses_symbol(circuit, theta.as_ptr()), 0);

    // Referencing it through a gate marks the symbol as used.
    assert_eq!(circuit_rx_param(circuit, 0, theta_p), 0);
    assert_eq!(circuit_uses_symbol(circuit, theta.as_ptr()), 1);
    assert_eq!(circuit_used_symbols_len(circuit), 1);

    // Symbol list contents (two-step, caller frees each string).
    let mut names: [*mut c_char; 1] = [ptr::null_mut(); 1];
    assert_eq!(circuit_symbols(circuit, names.as_mut_ptr(), 1), 1);
    assert_eq!(
        unsafe { CStr::from_ptr(names[0]) }.to_str().unwrap(),
        "theta"
    );
    cqlib_string_free(names[0]);
    assert_eq!(circuit_used_symbols(circuit, names.as_mut_ptr(), 1), 1);
    assert_eq!(
        unsafe { CStr::from_ptr(names[0]) }.to_str().unwrap(),
        "theta"
    );
    cqlib_string_free(names[0]);

    // Parameter list contents: cloned handles the caller frees.
    let mut params: [*mut binding_c::circuit::CParameter; 1] = [ptr::null_mut(); 1];
    assert_eq!(circuit_parameters(circuit, params.as_mut_ptr(), 1), 1);
    let bindings = cstr("theta:2");
    assert!((param_evaluate(params[0], bindings.as_ptr()) - 2.0).abs() < 1e-12);
    param_free(params[0]);

    // An unused symbol stays in the registry but not in the used set.
    let unused = cstr("unused");
    let unused_p = param_parse(unused.as_ptr());
    assert_eq!(circuit_add_parameter(circuit, unused_p), 0);
    assert_eq!(circuit_symbols_len(circuit), 2);
    assert_eq!(circuit_used_symbols_len(circuit), 1);
    assert_eq!(circuit_uses_symbol(circuit, unused.as_ptr()), 0);

    circuit_free(circuit);
    param_free(unused_p);
    param_free(theta_p);
}

#[test]
fn map_resolve_and_parameter_value() {
    let circuit = circuit_new(1);
    let theta = cstr("theta");
    let theta_p = param_parse(theta.as_ptr());
    assert_eq!(circuit_rx_param(circuit, 0, theta_p), 0);

    // Mapping a symbolic expression interns it and yields an index.
    let phi = cstr("phi * 2");
    let phi_p = param_parse(phi.as_ptr());
    let mut mapped = CCircuitParam {
        tag: 255,
        index: 0,
        value: 0.0,
    };
    assert_eq!(circuit_map_param(circuit, phi_p, &mut mapped), 0);
    assert_eq!(mapped.tag, CIRCUIT_PARAM_TAG_INDEX);
    assert_eq!(mapped.index, 1); // theta is index 0, phi*2 is index 1

    // Mapping a constant yields a fixed value.
    let half = cstr("0.5");
    let half_p = param_parse(half.as_ptr());
    let mut fixed = CCircuitParam {
        tag: 255,
        index: 0,
        value: 0.0,
    };
    assert_eq!(circuit_map_param(circuit, half_p, &mut fixed), 0);
    assert_eq!(fixed.tag, CIRCUIT_PARAM_TAG_FIXED);
    assert!((fixed.value - 0.5).abs() < 1e-12);

    // Resolve an index back into its expression and evaluate it.
    let resolved = circuit_resolve_parameter(circuit, &mapped);
    assert!(!resolved.is_null());
    let bindings = cstr("theta:0.25,phi:1.5");
    assert!((param_evaluate(resolved, bindings.as_ptr()) - 3.0).abs() < 1e-12);
    param_free(resolved);

    // Parameter values: index -> symbolic handle, fixed -> number.
    let mut value = CParameterValue {
        tag: 255,
        value: 0.0,
        param: ptr::null_mut(),
    };
    assert_eq!(circuit_parameter_value(circuit, &mapped, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_PARAM);
    assert!(!value.param.is_null());
    assert!((param_evaluate(value.param, bindings.as_ptr()) - 3.0).abs() < 1e-12);
    param_free(value.param);

    assert_eq!(circuit_parameter_value(circuit, &fixed, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_FIXED);
    assert!((value.value - 0.5).abs() < 1e-12);

    // Error paths: invalid index -> -3, invalid tag -> -8.
    let bad_index = CCircuitParam {
        tag: CIRCUIT_PARAM_TAG_INDEX,
        index: 99,
        value: 0.0,
    };
    assert_eq!(circuit_parameter_value(circuit, &bad_index, &mut value), -3);
    assert!(circuit_resolve_parameter(circuit, &bad_index).is_null());
    let bad_tag = CCircuitParam {
        tag: 7,
        index: 0,
        value: 1.0,
    };
    assert_eq!(circuit_parameter_value(circuit, &bad_tag, &mut value), -8);

    circuit_free(circuit);
    param_free(half_p);
    param_free(phi_p);
    param_free(theta_p);
}

#[test]
fn global_phase_metadata() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_set_global_phase(circuit, 0.25), 0);

    let mut value = CParameterValue {
        tag: 255,
        value: 0.0,
        param: ptr::null_mut(),
    };
    assert_eq!(circuit_global_phase(circuit, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_FIXED);
    assert!((value.value - 0.25).abs() < 1e-12);

    let alpha = cstr("alpha");
    let alpha_p = param_parse(alpha.as_ptr());
    assert_eq!(circuit_set_global_phase_param(circuit, alpha_p), 0);
    assert_eq!(circuit_global_phase(circuit, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_PARAM);
    assert!(!value.param.is_null());
    let bindings = cstr("alpha:1.5");
    assert!((param_evaluate(value.param, bindings.as_ptr()) - 1.5).abs() < 1e-12);
    param_free(value.param);

    assert_eq!(circuit_global_phase(ptr::null(), &mut value), -1);
    circuit_free(circuit);
    param_free(alpha_p);
}

#[test]
fn id_gate_alias_and_from_qubits() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_id(circuit, 0), 0);
    assert_eq!(circuit_id(circuit, 1), 0);
    assert_eq!(circuit_num_operations(circuit), 2);
    assert_eq!(circuit_id(circuit, 5), -2);
    assert_eq!(circuit_id(ptr::null_mut(), 0), -1);
    assert_eq!(circuit_validate(circuit), 0);
    circuit_free(circuit);

    let ids = [0u32, 2, 5];
    let sparse = circuit_from_qubits(ids.as_ptr(), 3);
    assert!(!sparse.is_null());
    assert_eq!(circuit_num_qubits(sparse), 3);
    let mut out = [0u32; 3];
    assert_eq!(circuit_qubits(sparse, out.as_mut_ptr(), 3), 3);
    assert_eq!(out, [0, 2, 5]);
    circuit_free(sparse);

    let duplicates = [1u32, 1];
    assert!(circuit_from_qubits(duplicates.as_ptr(), 2).is_null());
    assert!(circuit_from_qubits(ptr::null(), 1).is_null());
    let empty = circuit_from_qubits(ptr::null(), 0);
    assert!(!empty.is_null());
    circuit_free(empty);
}

#[test]
fn bell_matrix_known_values() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let matrix = matrix_of(circuit);
    // Little-endian basis (qubit 0 = LSB), U[row][col] = <row|U|col>.
    // Columns are the Bell images of the basis states: |00> -> (|00>+|11>)/sqrt2,
    // |01> -> (|00>-|11>)/sqrt2, |10> -> (|01>+|10>)/sqrt2, |11> -> (-|01>+|10>)/sqrt2.
    let s = 1.0 / 2.0f64.sqrt();
    let expected = [
        [s, s, 0.0, 0.0],
        [0.0, 0.0, s, -s],
        [0.0, 0.0, s, s],
        [s, -s, 0.0, 0.0],
    ];
    for row in 0..4 {
        for col in 0..4 {
            let re = matrix[(row * 4 + col) * 2];
            let im = matrix[(row * 4 + col) * 2 + 1];
            assert!(
                (re - expected[row][col]).abs() < 1e-12,
                "({row},{col}) re = {re}"
            );
            assert!(im.abs() < 1e-12, "({row},{col}) im = {im}");
        }
    }
    circuit_free(circuit);
}

#[test]
fn unitary_matches_standard_gate_matrix() {
    // Pauli-X as a custom 1-qubit unitary, row-major Complex64.
    let x_matrix = [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
    ];
    let label = cstr("my_x");
    let targets = [0u32];

    let circuit = circuit_new(2);
    assert_eq!(
        circuit_unitary(
            circuit,
            label.as_ptr(),
            1,
            x_matrix.as_ptr(),
            targets.as_ptr(),
            1
        ),
        0
    );

    let reference = circuit_new(2);
    assert_eq!(circuit_x(reference, 0), 0);
    assert_matrices_close(&matrix_of(circuit), &matrix_of(reference));

    // Error paths.
    let oob = [5u32];
    assert_eq!(
        circuit_unitary(
            circuit,
            label.as_ptr(),
            1,
            x_matrix.as_ptr(),
            oob.as_ptr(),
            1
        ),
        -2
    );
    assert_eq!(
        circuit_unitary(
            circuit,
            label.as_ptr(),
            21,
            x_matrix.as_ptr(),
            targets.as_ptr(),
            1
        ),
        -8
    );
    assert_eq!(
        circuit_unitary(
            ptr::null_mut(),
            label.as_ptr(),
            1,
            x_matrix.as_ptr(),
            targets.as_ptr(),
            1
        ),
        -1
    );

    circuit_free(reference);
    circuit_free(circuit);
}

#[test]
fn unitary_with_params_evaluates_symbolic_matrix() {
    // diag(1, exp(i*theta)) as a symbolic matrix; bind theta = 0.5.
    let re = [cstr("1"), cstr("0"), cstr("0"), cstr("cos(theta)")];
    let im = [cstr("0"), cstr("0"), cstr("0"), cstr("sin(theta)")];
    let names = [cstr("theta")];
    let re_ptrs: [*const c_char; 4] = [
        re[0].as_ptr(),
        re[1].as_ptr(),
        re[2].as_ptr(),
        re[3].as_ptr(),
    ];
    let im_ptrs: [*const c_char; 4] = [
        im[0].as_ptr(),
        im[1].as_ptr(),
        im[2].as_ptr(),
        im[3].as_ptr(),
    ];
    let name_ptrs: [*const c_char; 1] = [names[0].as_ptr()];

    let label = cstr("phase_like");
    let arg = cstr("0.5");
    let arg_p = param_parse(arg.as_ptr());
    let args: [*const CParameter; 1] = [arg_p];
    let targets = [0u32];

    let circuit = circuit_new(1);
    assert_eq!(
        circuit_unitary_with_params(
            circuit,
            label.as_ptr(),
            1,
            re_ptrs.as_ptr(),
            im_ptrs.as_ptr(),
            name_ptrs.as_ptr(),
            1,
            targets.as_ptr(),
            1,
            args.as_ptr(),
            1
        ),
        0
    );

    // The numeric matrix path only accepts fixed parameter values; the
    // constant argument is interned as an index, so fold it first (NULL
    // bindings keep true symbols symbolic but constant-fold expressions).
    let resolved = circuit_assign_params(circuit, ptr::null());
    assert!(!resolved.is_null());
    let matrix = matrix_of(resolved);
    assert!((matrix[0] - 1.0).abs() < 1e-12); // [0][0].re
    assert!(matrix[1].abs() < 1e-12); // [0][0].im
    assert!((matrix[6] - 0.5f64.cos()).abs() < 1e-12); // [1][1].re
    assert!((matrix[7] - 0.5f64.sin()).abs() < 1e-12); // [1][1].im

    // A NULL imaginary-part array is treated as all zeros: the matrix
    // becomes diag(1, cos(theta)) and binding theta = 0 yields the identity.
    let zero = cstr("0");
    let zero_p = param_parse(zero.as_ptr());
    let zero_args: [*const CParameter; 1] = [zero_p];
    let plain = circuit_new(1);
    assert_eq!(
        circuit_unitary_with_params(
            plain,
            label.as_ptr(),
            1,
            re_ptrs.as_ptr(),
            ptr::null(),
            name_ptrs.as_ptr(),
            1,
            targets.as_ptr(),
            1,
            zero_args.as_ptr(),
            1
        ),
        0
    );
    let resolved_plain = circuit_assign_params(plain, ptr::null());
    assert!(!resolved_plain.is_null());
    let identity = matrix_of(resolved_plain);
    assert!((identity[0] - 1.0).abs() < 1e-12);
    assert!((identity[6] - 1.0).abs() < 1e-12); // [1][1].re, cos(0) = 1
    assert!(identity[7].abs() < 1e-12); // [1][1].im

    // Wrong parameter count -> -3.
    assert_eq!(
        circuit_unitary_with_params(
            circuit,
            label.as_ptr(),
            1,
            re_ptrs.as_ptr(),
            im_ptrs.as_ptr(),
            name_ptrs.as_ptr(),
            1,
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -3
    );

    circuit_free(resolved_plain);
    circuit_free(plain);
    circuit_free(resolved);
    circuit_free(circuit);
    param_free(zero_p);
    param_free(arg_p);
}

#[test]
fn to_gate_circuit_gate_bell_roundtrip() {
    let inner = circuit_new(2);
    assert_eq!(circuit_h(inner, 0), 0);
    assert_eq!(circuit_cx(inner, 0, 1), 0);

    let name = cstr("bell");
    let gate = circuit_to_gate(inner, name.as_ptr());
    assert!(!gate.is_null());

    let gate_name = circuit_gate_name(gate);
    assert_eq!(
        unsafe { CStr::from_ptr(gate_name) }.to_str().unwrap(),
        "bell"
    );
    cqlib_string_free(gate_name);
    assert_eq!(circuit_gate_num_qubits(gate), 2);
    assert_eq!(circuit_gate_num_params(gate), 0);
    assert_eq!(circuit_gate_signature_params_len(gate), 0);

    let outer = circuit_new(2);
    let qubits = [0u32, 1];
    assert_eq!(
        circuit_circuit_gate(outer, gate, qubits.as_ptr(), 2, ptr::null(), 0),
        0
    );
    // Applying the composite gate reproduces the original Bell circuit.
    assert_matrices_close(&matrix_of(outer), &matrix_of(inner));

    // Wrong qubit arity -> -3; out-of-bounds qubit -> -2.
    let single = [0u32];
    assert_eq!(
        circuit_circuit_gate(outer, gate, single.as_ptr(), 1, ptr::null(), 0),
        -3
    );
    let oob = [0u32, 9];
    assert_eq!(
        circuit_circuit_gate(outer, gate, oob.as_ptr(), 2, ptr::null(), 0),
        -2
    );

    circuit_free(outer);
    circuit_gate_free(gate);
    circuit_free(inner);
}

#[test]
fn to_gate_circuit_gate_parametrized_roundtrip() {
    // Template: RX(theta) on one qubit.
    let template = circuit_new(1);
    let theta = cstr("theta");
    let theta_p = param_parse(theta.as_ptr());
    assert_eq!(circuit_rx_param(template, 0, theta_p), 0);

    let name = cstr("rx_theta");
    let gate = circuit_to_gate(template, name.as_ptr());
    assert!(!gate.is_null());
    assert_eq!(circuit_gate_num_params(gate), 1);
    assert_eq!(circuit_gate_signature_params_len(gate), 1);
    let mut names: [*mut c_char; 1] = [ptr::null_mut(); 1];
    assert_eq!(
        circuit_gate_signature_params(gate, names.as_mut_ptr(), 1),
        1
    );
    assert_eq!(
        unsafe { CStr::from_ptr(names[0]) }.to_str().unwrap(),
        "theta"
    );
    cqlib_string_free(names[0]);

    // Apply with theta = 0.5 -> equivalent to RX(0.5).
    let arg = cstr("0.5");
    let arg_p = param_parse(arg.as_ptr());
    let args: [*const CParameter; 1] = [arg_p];
    let qubits = [0u32];
    let outer = circuit_new(1);
    assert_eq!(
        circuit_circuit_gate(outer, gate, qubits.as_ptr(), 1, args.as_ptr(), 1),
        0
    );

    let reference = circuit_new(1);
    assert_eq!(circuit_rx(reference, 0, 0.5), 0);
    // Fold the constant argument into a fixed parameter so the numeric
    // matrix path can evaluate the gate.
    let resolved = circuit_assign_params(outer, ptr::null());
    assert!(!resolved.is_null());
    assert_matrices_close(&matrix_of(resolved), &matrix_of(reference));

    // Wrong parameter count -> -3.
    assert_eq!(
        circuit_circuit_gate(outer, gate, qubits.as_ptr(), 1, ptr::null(), 0),
        -3
    );

    circuit_free(reference);
    circuit_free(resolved);
    circuit_free(outer);
    param_free(arg_p);
    circuit_gate_free(gate);
    circuit_free(template);
    param_free(theta_p);
}

#[test]
fn multi_control_matches_ccx() {
    let circuit = circuit_new(3);
    let gate = cstr("X");
    let controls = [0u32, 1];
    let targets = [2u32];
    assert_eq!(
        circuit_multi_control(
            circuit,
            gate.as_ptr(),
            controls.as_ptr(),
            2,
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        0
    );

    let reference = circuit_new(3);
    assert_eq!(circuit_ccx(reference, 0, 1, 2), 0);
    assert_matrices_close(&matrix_of(circuit), &matrix_of(reference));

    // Unknown gate name -> -8; arity mismatch -> -3; OOB control -> -2.
    let bogus = cstr("NOT_A_GATE");
    assert_eq!(
        circuit_multi_control(
            circuit,
            bogus.as_ptr(),
            controls.as_ptr(),
            2,
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );
    let hadamard = cstr("H");
    assert_eq!(
        circuit_multi_control(
            circuit,
            hadamard.as_ptr(),
            controls.as_ptr(),
            2,
            ptr::null(),
            0,
            ptr::null(),
            0
        ),
        -3
    );
    let oob = [0u32, 9];
    assert_eq!(
        circuit_multi_control(
            circuit,
            gate.as_ptr(),
            oob.as_ptr(),
            2,
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -2
    );

    circuit_free(reference);
    circuit_free(circuit);
}

#[test]
fn delay_and_index_snapshot() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_delay(circuit, 1, 100.0), 0);
    assert_eq!(circuit_delay(circuit, 5, 1.0), -2);
    assert_eq!(circuit_delay(circuit, 0, f64::NAN), -3);
    assert_eq!(circuit_delay(ptr::null_mut(), 0, 1.0), -1);

    let dt = cstr("dt");
    let dt_p = param_parse(dt.as_ptr());
    assert_eq!(circuit_delay_param(circuit, 0, dt_p), 0);
    assert_eq!(circuit_delay_param(circuit, 0, ptr::null()), -1);
    assert_eq!(circuit_num_operations(circuit), 2);
    assert_eq!(circuit_num_parameters(circuit), 1);

    // Snapshot of the numeric delay.
    let op = circuit_index(circuit, 0);
    assert!(!op.is_null());
    let name = value_operation_name(op);
    assert_eq!(unsafe { CStr::from_ptr(name) }.to_str().unwrap(), "delay");
    cqlib_string_free(name);
    let ty = value_operation_instruction_type(op);
    assert_eq!(unsafe { CStr::from_ptr(ty) }.to_str().unwrap(), "delay");
    cqlib_string_free(ty);
    assert!(value_operation_label(op).is_null());
    assert_eq!(value_operation_num_qubits(op), 1);
    let mut qubits = [0u32; 1];
    assert_eq!(value_operation_qubits(op, qubits.as_mut_ptr(), 1), 1);
    assert_eq!(qubits[0], 1);
    assert_eq!(value_operation_num_params(op), 1);

    let mut value = CParameterValue {
        tag: 255,
        value: 0.0,
        param: ptr::null_mut(),
    };
    assert_eq!(value_operation_param(op, 0, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_FIXED);
    assert!((value.value - 100.0).abs() < 1e-12);
    assert_eq!(value_operation_param(op, 1, &mut value), -3);
    value_operation_free(op);

    // Snapshot of the symbolic delay.
    let symbolic = circuit_index(circuit, 1);
    assert!(!symbolic.is_null());
    assert_eq!(value_operation_param(symbolic, 0, &mut value), 0);
    assert_eq!(value.tag, PARAMETER_VALUE_TAG_PARAM);
    assert!(!value.param.is_null());
    let bindings = cstr("dt:7");
    assert!((param_evaluate(value.param, bindings.as_ptr()) - 7.0).abs() < 1e-12);
    param_free(value.param);
    value_operation_free(symbolic);

    assert!(circuit_index(circuit, 99).is_null());
    assert!(circuit_index(ptr::null(), 0).is_null());

    circuit_free(circuit);
    param_free(dt_p);
}

#[test]
fn remove_operations_batch() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_x(circuit, 0), 0);
    assert_eq!(circuit_phase(circuit, 0, 0.5), 0);

    let indices = [0usize, 2];
    assert_eq!(circuit_remove_operations(circuit, indices.as_ptr(), 2), 0);
    assert_eq!(circuit_num_operations(circuit), 1);

    // The surviving operation is X.
    let op = circuit_index(circuit, 0);
    assert!(!op.is_null());
    let name = value_operation_name(op);
    assert_eq!(unsafe { CStr::from_ptr(name) }.to_str().unwrap(), "X");
    cqlib_string_free(name);
    value_operation_free(op);

    // Empty index list is a successful no-op.
    assert_eq!(circuit_remove_operations(circuit, ptr::null(), 0), 0);
    assert_eq!(circuit_num_operations(circuit), 1);

    // Out-of-bounds index -> -3; NULL circuit -> -1.
    let bad = [0usize, 9];
    assert_eq!(circuit_remove_operations(circuit, bad.as_ptr(), 2), -3);
    assert_eq!(
        circuit_remove_operations(ptr::null_mut(), ptr::null(), 0),
        -1
    );

    circuit_free(circuit);
}

#[test]
fn advanced_bindings_null_checks() {
    assert_eq!(circuit_add_parameter(ptr::null_mut(), ptr::null()), -1);
    assert_eq!(circuit_parameters_len(ptr::null()), 0);
    assert_eq!(circuit_parameters(ptr::null(), ptr::null_mut(), 0), 0);
    assert_eq!(circuit_symbols_len(ptr::null()), 0);
    assert_eq!(circuit_symbols(ptr::null(), ptr::null_mut(), 0), 0);
    assert_eq!(circuit_used_symbols_len(ptr::null()), 0);
    assert_eq!(circuit_used_symbols(ptr::null(), ptr::null_mut(), 0), 0);
    assert_eq!(circuit_uses_symbol(ptr::null(), ptr::null()), -1);
    assert_eq!(circuit_global_phase(ptr::null(), ptr::null_mut()), -1);
    assert_eq!(
        circuit_map_param(ptr::null_mut(), ptr::null(), ptr::null_mut()),
        -1
    );
    assert!(circuit_resolve_parameter(ptr::null(), ptr::null()).is_null());
    assert_eq!(
        circuit_parameter_value(ptr::null(), ptr::null(), ptr::null_mut()),
        -1
    );

    assert_eq!(
        circuit_unitary(ptr::null_mut(), ptr::null(), 0, ptr::null(), ptr::null(), 0),
        -1
    );
    assert_eq!(
        circuit_unitary_with_params(
            ptr::null_mut(),
            ptr::null(),
            0,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            ptr::null(),
            0
        ),
        -1
    );
    assert!(circuit_to_gate(ptr::null(), ptr::null()).is_null());
    assert_eq!(
        circuit_circuit_gate(ptr::null_mut(), ptr::null(), ptr::null(), 0, ptr::null(), 0),
        -1
    );
    assert_eq!(
        circuit_multi_control(
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            0,
            ptr::null(),
            0,
            ptr::null(),
            0
        ),
        -1
    );
    assert_eq!(circuit_delay(ptr::null_mut(), 0, 1.0), -1);
    assert_eq!(circuit_delay_param(ptr::null_mut(), 0, ptr::null()), -1);
    assert!(circuit_index(ptr::null(), 0).is_null());
    assert_eq!(
        circuit_remove_operations(ptr::null_mut(), ptr::null(), 0),
        -1
    );

    // Handle accessors tolerate NULL.
    assert!(circuit_gate_name(ptr::null()).is_null());
    assert_eq!(circuit_gate_num_qubits(ptr::null()), 0);
    assert_eq!(circuit_gate_num_params(ptr::null()), 0);
    assert_eq!(circuit_gate_signature_params_len(ptr::null()), 0);
    assert_eq!(
        circuit_gate_signature_params(ptr::null(), ptr::null_mut(), 0),
        0
    );
    assert!(value_operation_name(ptr::null()).is_null());
    assert!(value_operation_instruction_type(ptr::null()).is_null());
    assert!(value_operation_label(ptr::null()).is_null());
    assert_eq!(value_operation_num_qubits(ptr::null()), 0);
    assert_eq!(value_operation_qubits(ptr::null(), ptr::null_mut(), 0), 0);
    assert_eq!(value_operation_num_params(ptr::null()), 0);
    assert_eq!(value_operation_param(ptr::null(), 0, ptr::null_mut()), -1);
    circuit_gate_free(ptr::null_mut());
    value_operation_free(ptr::null_mut());
}
