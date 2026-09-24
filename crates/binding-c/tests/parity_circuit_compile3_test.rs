// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Rust FFI parity tests for the third circuit/compile binding batch:
//! `unitary_gate_matrix_for_params`, the by-value
//! `rewrite_config_with_target_instructions` setter, and the
//! `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` tag.

use binding_c::circuit::{
    circuit_cx, circuit_free, circuit_new, circuit_num_operations, param_free, param_parse,
    symbolic_complex_free, symbolic_complex_new, symbolic_matrix_free, symbolic_matrix_new,
    unitary_gate_matrix_for_params, unitary_gate_matrix_for_params_len,
};
use binding_c::compile::{
    LAYOUT_OBJECTIVE_FIDELITY_REQUIRED, REWRITE_MODE_LOWERING, analyze_circuit_for_layout,
    circuit_layout_analysis_free, knowledge_rewrite_result_changed,
    knowledge_rewrite_result_circuit, knowledge_rewrite_result_free, layout_result_free,
    physical_layout_graph_free, physical_layout_graph_from_device,
    physical_layout_graph_has_fidelity_data, rewrite_circuit, rewrite_config_default,
    rewrite_config_lowering, rewrite_config_with_target_instructions, score_layout,
    transform_knowledge_rewrite, trivial_layout, trivial_layout_prepared,
};
use binding_c::device::{
    device_free, device_from_edges, device_set_default_readout_error, layout_free, layout_new,
};
use num_complex::Complex64;
use std::ffi::CString;
use std::os::raw::c_char;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Builds a `CSymbolicComplex` from real/imaginary expression strings. The
/// returned handle (and the parsed parameter handles) are owned by the
/// caller and freed after `symbolic_matrix_new` clones them.
fn sym_complex(
    re: &str,
    im: &str,
) -> (
    *const binding_c::circuit::CSymbolicComplex,
    *mut binding_c::circuit::CParameter,
    *mut binding_c::circuit::CParameter,
) {
    let re_param = param_parse(cstr(re).as_ptr());
    assert!(!re_param.is_null());
    let im_param = param_parse(cstr(im).as_ptr());
    assert!(!im_param.is_null());
    let complex = symbolic_complex_new(re_param, im_param);
    assert!(!complex.is_null());
    (complex, re_param, im_param)
}

fn free_sym_complex(
    parts: (
        *const binding_c::circuit::CSymbolicComplex,
        *mut binding_c::circuit::CParameter,
        *mut binding_c::circuit::CParameter,
    ),
) {
    symbolic_complex_free(parts.0 as *mut binding_c::circuit::CSymbolicComplex);
    param_free(parts.1);
    param_free(parts.2);
}

fn close(actual: Complex64, expected: Complex64) -> bool {
    (actual - expected).norm() < 1e-9
}

#[test]
fn unitary_gate_matrix_for_params_evaluates_symbolic_matrix() {
    // U(theta) = cos(theta) * I + i sin(theta) * X, a unitary matrix whose
    // expressions need no unary-minus parsing.
    let e00 = sym_complex("cos(theta)", "0");
    let e01 = sym_complex("0", "sin(theta)");
    let e10 = sym_complex("0", "sin(theta)");
    let e11 = sym_complex("cos(theta)", "0");
    let elements = [e00.0, e01.0, e10.0, e11.0];
    let matrix = symbolic_matrix_new(2, 2, elements.as_ptr(), 4);
    assert!(!matrix.is_null());
    assert_eq!(unitary_gate_matrix_for_params_len(matrix), 4);

    let theta = cstr("theta");
    let names = [theta.as_ptr()];

    // theta = 0 -> identity.
    let mut buffer = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            [0.0f64].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        0
    );
    assert!(close(buffer[0], Complex64::new(1.0, 0.0)));
    assert!(close(buffer[1], Complex64::new(0.0, 0.0)));
    assert!(close(buffer[2], Complex64::new(0.0, 0.0)));
    assert!(close(buffer[3], Complex64::new(1.0, 0.0)));

    // theta = pi/2 -> iX.
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            [std::f64::consts::FRAC_PI_2].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        0
    );
    assert!(close(buffer[0], Complex64::new(0.0, 0.0)));
    assert!(close(buffer[1], Complex64::new(0.0, 1.0)));
    assert!(close(buffer[2], Complex64::new(0.0, 1.0)));
    assert!(close(buffer[3], Complex64::new(0.0, 0.0)));

    symbolic_matrix_free(matrix);
    for element in [e00, e01, e10, e11] {
        free_sym_complex(element);
    }
}

#[test]
fn unitary_gate_matrix_for_params_accepts_parameterless_constant_matrix() {
    // The constant Pauli-X matrix with zero formal parameters.
    let e00 = sym_complex("0", "0");
    let e01 = sym_complex("1", "0");
    let e10 = sym_complex("1", "0");
    let e11 = sym_complex("0", "0");
    let elements = [e00.0, e01.0, e10.0, e11.0];
    let matrix = symbolic_matrix_new(2, 2, elements.as_ptr(), 4);
    assert!(!matrix.is_null());

    let mut buffer = [Complex64::new(9.0, 9.0); 4];
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            4
        ),
        0
    );
    assert!(close(buffer[0], Complex64::new(0.0, 0.0)));
    assert!(close(buffer[1], Complex64::new(1.0, 0.0)));
    assert!(close(buffer[2], Complex64::new(1.0, 0.0)));
    assert!(close(buffer[3], Complex64::new(0.0, 0.0)));

    symbolic_matrix_free(matrix);
    for element in [e00, e01, e10, e11] {
        free_sym_complex(element);
    }
}

#[test]
fn unitary_gate_matrix_for_params_reports_errors() {
    let e00 = sym_complex("cos(theta)", "0");
    let e01 = sym_complex("0", "sin(theta)");
    let e10 = sym_complex("0", "sin(theta)");
    let e11 = sym_complex("cos(theta)", "0");
    let elements = [e00.0, e01.0, e10.0, e11.0];
    let matrix = symbolic_matrix_new(2, 2, elements.as_ptr(), 4);
    assert!(!matrix.is_null());

    let theta = cstr("theta");
    let names = [theta.as_ptr()];
    let mut buffer = [Complex64::new(0.0, 0.0); 4];

    // NULL inputs.
    assert_eq!(
        unitary_gate_matrix_for_params(
            std::ptr::null(),
            names.as_ptr(),
            1,
            [0.0].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -1
    );
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            [0.0].as_ptr(),
            1,
            std::ptr::null_mut(),
            4
        ),
        -1
    );
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            std::ptr::null(),
            1,
            [0.0].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -1
    );
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            std::ptr::null(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -1
    );

    // A NULL name entry and invalid UTF-8 names are rejected.
    let null_names: [*const c_char; 1] = [std::ptr::null()];
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            null_names.as_ptr(),
            1,
            [0.0].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -1
    );
    let invalid = unsafe { CString::from_vec_unchecked(vec![0xFF, 0xFE]) };
    let bad_names = [invalid.as_ptr()];
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            bad_names.as_ptr(),
            1,
            [0.0].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -4
    );

    // Parameter-count mismatch and non-finite values map to -3.
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            4
        ),
        -3
    );
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            [f64::NAN].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            4
        ),
        -3
    );

    // A too-small buffer maps to -8.
    assert_eq!(
        unitary_gate_matrix_for_params(
            matrix,
            names.as_ptr(),
            1,
            [0.0].as_ptr(),
            1,
            buffer.as_mut_ptr(),
            3
        ),
        -8
    );

    symbolic_matrix_free(matrix);
    for element in [e00, e01, e10, e11] {
        free_sym_complex(element);
    }

    // A singular constant matrix fails the core unitarity check (-3).
    let z00 = sym_complex("1", "0");
    let z01 = sym_complex("0", "0");
    let z10 = sym_complex("0", "0");
    let z11 = sym_complex("0", "0");
    let zero_elements = [z00.0, z01.0, z10.0, z11.0];
    let singular = symbolic_matrix_new(2, 2, zero_elements.as_ptr(), 4);
    assert!(!singular.is_null());
    assert_eq!(
        unitary_gate_matrix_for_params(
            singular,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            4
        ),
        -3
    );
    symbolic_matrix_free(singular);
    for element in [z00, z01, z10, z11] {
        free_sym_complex(element);
    }

    // NULL/non-square length queries return 0.
    assert_eq!(unitary_gate_matrix_for_params_len(std::ptr::null()), 0);
}

#[test]
fn rewrite_config_target_instructions_setter_validates_inputs() {
    let mut config = rewrite_config_default();
    assert_eq!(config.mode, 0);
    assert!(config.target_gate_names.is_null());
    assert_eq!(config.target_gate_names_len, 0);

    // NULL config, empty basis and NULL arrays with a non-zero length.
    assert_eq!(
        rewrite_config_with_target_instructions(std::ptr::null_mut(), std::ptr::null(), 2),
        -1
    );
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, std::ptr::null(), 0),
        -8
    );
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, std::ptr::null(), 2),
        -1
    );

    // Unknown gate name -> -4 and the field stays unset.
    let bad = cstr("NOPE");
    let bad_ptrs = [bad.as_ptr()];
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, bad_ptrs.as_ptr(), 1),
        -4
    );
    assert!(config.target_gate_names.is_null());

    // A NULL name entry -> -1.
    let null_entry: [*const c_char; 1] = [std::ptr::null()];
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, null_entry.as_ptr(), 1),
        -1
    );

    // Invalid UTF-8 -> -4.
    let invalid = unsafe { CString::from_vec_unchecked(vec![0xFF, 0xFE]) };
    let invalid_ptrs = [invalid.as_ptr()];
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, invalid_ptrs.as_ptr(), 1),
        -4
    );

    // Valid names are stored on the by-value config.
    let h = cstr("H");
    let cz = cstr("CZ");
    let gates = [h.as_ptr(), cz.as_ptr()];
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, gates.as_ptr(), 2),
        0
    );
    assert_eq!(config.target_gate_names_len, 2);
}

#[test]
fn rewrite_config_target_instructions_drives_knowledge_lowering() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    // CX lowers to H-CZ-H against the {H, CZ} target basis.
    let h = cstr("H");
    let cz = cstr("CZ");
    let gates = [h.as_ptr(), cz.as_ptr()];
    let mut config = rewrite_config_lowering();
    assert_eq!(config.mode, REWRITE_MODE_LOWERING);
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config, gates.as_ptr(), 2),
        0
    );
    let result = rewrite_circuit(circuit, config);
    assert!(!result.is_null());
    assert_eq!(knowledge_rewrite_result_changed(result), 1);
    let rewritten = knowledge_rewrite_result_circuit(result);
    assert!(!rewritten.is_null());
    assert_eq!(circuit_num_operations(rewritten), 3);
    circuit_free(rewritten);
    knowledge_rewrite_result_free(result);

    // The standalone transform wrapper consumes the same by-value config.
    let mut config2 = rewrite_config_lowering();
    assert_eq!(
        rewrite_config_with_target_instructions(&mut config2, gates.as_ptr(), 2),
        0
    );
    let wrapped = transform_knowledge_rewrite(circuit, config2);
    assert!(!wrapped.is_null());
    assert_eq!(circuit_num_operations(wrapped), 3);
    circuit_free(wrapped);

    // A basis that cannot represent CX fails final-target validation.
    let x = cstr("X");
    let x_only = [x.as_ptr()];
    let mut impossible = rewrite_config_lowering();
    assert_eq!(
        rewrite_config_with_target_instructions(&mut impossible, x_only.as_ptr(), 1),
        0
    );
    assert!(rewrite_circuit(circuit, impossible).is_null());

    circuit_free(circuit);
}

fn line3_device() -> *mut binding_c::device::CDevice {
    let edges: [u32; 4] = [0, 1, 1, 2];
    device_from_edges(cstr("line-3").as_ptr(), 3, edges.as_ptr(), 2)
}

#[test]
fn layout_objective_fidelity_required_tag_value() {
    // The C tag follows the core constructor ordering: 3.
    assert_eq!(LAYOUT_OBJECTIVE_FIDELITY_REQUIRED, 3);
}

#[test]
fn layout_objective_fidelity_required_without_calibration_errors() {
    let device = line3_device();
    let circuit = circuit_new(2);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let graph = physical_layout_graph_from_device(device);
    assert!(!graph.is_null());
    assert_eq!(physical_layout_graph_has_fidelity_data(graph), 0);

    // Pointer-returning entries surface the missing calibration as NULL.
    assert!(trivial_layout(circuit, device, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED).is_null());
    let first_analysis = analyze_circuit_for_layout(circuit);
    assert!(
        trivial_layout_prepared(first_analysis, graph, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED)
            .is_null()
    );
    circuit_layout_analysis_free(first_analysis);

    // The scoring entry reports the invalid configuration as -8.
    let analysis = analyze_circuit_for_layout(circuit);
    let qubits = [0u32, 1u32];
    let layout = layout_new(qubits.as_ptr(), 2, qubits.as_ptr(), 2);
    let mut score: binding_c::compile::CLayoutScore = unsafe { std::mem::zeroed() };
    assert_eq!(
        score_layout(
            LAYOUT_OBJECTIVE_FIDELITY_REQUIRED,
            analysis,
            graph,
            layout,
            &mut score
        ),
        -8
    );

    layout_free(layout);
    circuit_layout_analysis_free(analysis);
    physical_layout_graph_free(graph);
    circuit_free(circuit);
    device_free(device);
}

#[test]
fn layout_objective_fidelity_required_with_calibration_succeeds() {
    let device = line3_device();
    assert_eq!(device_set_default_readout_error(device, 0.01), 0);

    let circuit = circuit_new(2);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let graph = physical_layout_graph_from_device(device);
    assert!(!graph.is_null());
    assert_eq!(physical_layout_graph_has_fidelity_data(graph), 1);

    let result = trivial_layout(circuit, device, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED);
    assert!(!result.is_null());
    layout_result_free(result);

    let analysis = analyze_circuit_for_layout(circuit);
    let prepared = trivial_layout_prepared(analysis, graph, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED);
    assert!(!prepared.is_null());
    layout_result_free(prepared);

    // The resolved objective carries fidelity terms into scoring.
    let qubits = [0u32, 1u32];
    let layout = layout_new(qubits.as_ptr(), 2, qubits.as_ptr(), 2);
    let mut score: binding_c::compile::CLayoutScore = unsafe { std::mem::zeroed() };
    assert_eq!(
        score_layout(
            LAYOUT_OBJECTIVE_FIDELITY_REQUIRED,
            analysis,
            graph,
            layout,
            &mut score
        ),
        0
    );
    assert_eq!(score.used_fidelity, 1);

    layout_free(layout);
    circuit_layout_analysis_free(analysis);
    physical_layout_graph_free(graph);
    circuit_free(circuit);
    device_free(device);
}
