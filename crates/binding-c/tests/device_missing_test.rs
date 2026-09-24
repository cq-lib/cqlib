//! Integration tests for the device-module FFI covering per-qubit property
//! setters/getters, layout-result helpers, execution-result status/probability
//! helpers, and noise-model queries.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_new};
use binding_c::compile::layout::{
    analyze_circuit_for_layout, circuit_layout_analysis_free, circuit_layout_analysis_num_logical,
    physical_layout_graph_free, physical_layout_graph_from_device, prepare_sabre_circuit,
    prepared_sabre_circuit_free,
};
use binding_c::cqlib_string_free;
use binding_c::device::*;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn name_ptr(name: &str) -> CString {
    CString::new(name).unwrap()
}

/// Builds a `CQubitPropInput` with only the readout error set.
fn bare_input(readout: f64) -> CQubitPropInput {
    CQubitPropInput {
        readout_error: readout,
        t1: f64::NAN,
        t2: f64::NAN,
        prob_meas0_prep1: f64::NAN,
        prob_meas1_prep0: f64::NAN,
        frequency: f64::NAN,
        native_gate_names: std::ptr::null(),
        native_error_rates: std::ptr::null(),
        native_lengths: std::ptr::null(),
        num_native_instructions: 0,
    }
}

#[test]
fn test_device_qubit_property_setters_and_getters() {
    let name = name_ptr("prop-line");
    let device = device_line(name.as_ptr(), 3);
    assert!(!device.is_null());

    // Record full properties for qubit 0: readout 0.01, t1 20, t2 30,
    // prob01 0.02, prob10 0.03, freq 5.0, native H 0.001/50, X 0.002/60.
    let gates = [name_ptr("H"), name_ptr("X")];
    let gate_ptrs = [gates[0].as_ptr(), gates[1].as_ptr()];
    let rates = [0.001, 0.002];
    let lengths = [50.0, 60.0];
    let input = CQubitPropInput {
        readout_error: 0.01,
        t1: 20.0,
        t2: 30.0,
        prob_meas0_prep1: 0.02,
        prob_meas1_prep0: 0.03,
        frequency: 5.0,
        native_gate_names: gate_ptrs.as_ptr(),
        native_error_rates: rates.as_ptr(),
        native_lengths: lengths.as_ptr(),
        num_native_instructions: 2,
    };
    assert_eq!(device_add_qubit_properties(device, 0, &input), 0);

    // Getters read back the stored values.
    let mut v = f64::NAN;
    assert_eq!(device_get_frequency(device, 0, &mut v), 0);
    assert_eq!(v, 5.0);
    assert_eq!(device_get_prob_meas0_prep1(device, 0, &mut v), 0);
    assert_eq!(v, 0.02);
    assert_eq!(device_get_prob_meas1_prep0(device, 0, &mut v), 0);
    assert_eq!(v, 0.03);
    assert_eq!(device_get_error_rate(device, 0, 1, &mut v), 0);
    assert_eq!(v, 0.002);
    assert_eq!(device_get_length(device, 0, 0, &mut v), 0);
    assert_eq!(v, 50.0);
    assert_eq!(device_get_t1(device, 0, &mut v), 0);
    assert_eq!(v, 20.0);

    // Getters fail for a qubit without recorded properties.
    assert_eq!(device_get_frequency(device, 1, &mut v), -8);
    assert_eq!(device_get_error_rate(device, 1, 0, &mut v), -8);

    // NULL handling.
    assert_eq!(device_get_frequency(std::ptr::null(), 0, &mut v), -1);
    assert_eq!(device_set_frequency(std::ptr::null_mut(), 0, 1.0), -1);

    // Setters update one field and preserve the others; all reads happen
    // after the last setter to verify the rebuild keeps every field.
    assert_eq!(device_set_t1(device, 0, 21.0), 0);
    assert_eq!(device_set_t2(device, 0, 31.0), 0);
    assert_eq!(device_set_frequency(device, 0, 5.5), 0);
    assert_eq!(device_set_prob_meas0_prep1(device, 0, 0.021), 0);
    assert_eq!(device_set_prob_meas1_prep0(device, 0, 0.031), 0);
    assert_eq!(device_get_t1(device, 0, &mut v), 0);
    assert_eq!(v, 21.0);
    assert_eq!(device_get_t2(device, 0, &mut v), 0);
    assert_eq!(v, 31.0);
    assert_eq!(device_get_frequency(device, 0, &mut v), 0);
    assert_eq!(v, 5.5);
    assert_eq!(device_get_prob_meas0_prep1(device, 0, &mut v), 0);
    assert_eq!(v, 0.021);
    assert_eq!(device_get_prob_meas1_prep0(device, 0, &mut v), 0);
    assert_eq!(v, 0.031);
    // Native instructions survive the property rebuilds.
    assert_eq!(device_get_error_rate(device, 0, 1, &mut v), 0);
    assert_eq!(v, 0.002);

    // Setters fail for a qubit without recorded properties.
    assert_eq!(device_set_t1(device, 2, 1.0), -8);
    assert_eq!(device_set_prob_meas0_prep1(device, 2, 0.1), -8);

    device_free(device);
}

#[test]
fn test_device_native_instruction_editors() {
    let name = name_ptr("gate-line");
    let device = device_line(name.as_ptr(), 2);
    assert!(!device.is_null());

    let gates = [name_ptr("H"), name_ptr("X")];
    let gate_ptrs = [gates[0].as_ptr(), gates[1].as_ptr()];
    let rates = [0.001, 0.002];
    let lengths = [50.0, 60.0];
    let input = CQubitPropInput {
        readout_error: 0.01,
        t1: f64::NAN,
        t2: f64::NAN,
        prob_meas0_prep1: f64::NAN,
        prob_meas1_prep0: f64::NAN,
        frequency: f64::NAN,
        native_gate_names: gate_ptrs.as_ptr(),
        native_error_rates: rates.as_ptr(),
        native_lengths: lengths.as_ptr(),
        num_native_instructions: 2,
    };
    assert_eq!(device_add_qubit_properties(device, 0, &input), 0);

    let mut v = f64::NAN;
    // Edit error rate and duration in place.
    assert_eq!(device_set_error_rate(device, 0, 1, 0.009), 0);
    assert_eq!(device_set_length(device, 0, 0, 70.0), 0);
    assert_eq!(device_get_error_rate(device, 0, 1, &mut v), 0);
    assert_eq!(v, 0.009);
    assert_eq!(device_get_length(device, 0, 0, &mut v), 0);
    assert_eq!(v, 70.0);
    // Untouched entries keep their values.
    assert_eq!(device_get_error_rate(device, 0, 0, &mut v), 0);
    assert_eq!(v, 0.001);
    assert_eq!(device_get_length(device, 0, 1, &mut v), 0);
    assert_eq!(v, 60.0);

    // Out-of-bounds index handling.
    assert_eq!(device_set_error_rate(device, 0, 5, 0.1), -8);
    assert_eq!(device_set_length(device, 0, 5, 1.0), -8);
    assert_eq!(device_get_error_rate(device, 0, 5, &mut v), -8);
    assert_eq!(device_set_error_rate(device, 1, 0, 0.1), -8);

    // Replace the carried gate of entry 0 (H -> X) and verify the snapshot.
    let x = name_ptr("X");
    assert_eq!(device_set_instruction(device, 0, 0, x.as_ptr()), 0);
    let mut inst = CNativeInstruction {
        name: std::ptr::null_mut(),
        error_rate: 0.0,
        length: 0.0,
    };
    assert_eq!(
        device_qubit_prop_native_instruction(device, 0, 0, &mut inst),
        0
    );
    assert_eq!(cstr_to_string(inst.name), "X");
    cqlib_string_free(inst.name);
    assert_eq!(inst.error_rate, 0.001);
    assert_eq!(inst.length, 70.0);

    // Append a new native instruction.
    let z = name_ptr("Z");
    assert_eq!(
        device_set_native_instruction(device, 0, z.as_ptr(), 0.003, 80.0),
        0
    );
    let mut snap = CQubitProp {
        readout_error: 0.0,
        t1: 0.0,
        t2: 0.0,
        prob_meas0_prep1: 0.0,
        prob_meas1_prep0: 0.0,
        frequency: 0.0,
        num_native_instructions: 0,
    };
    assert_eq!(device_qubit_properties(device, 0, &mut snap), 0);
    assert_eq!(snap.num_native_instructions, 3);
    let mut third = CNativeInstruction {
        name: std::ptr::null_mut(),
        error_rate: 0.0,
        length: 0.0,
    };
    assert_eq!(
        device_qubit_prop_native_instruction(device, 0, 2, &mut third),
        0
    );
    assert_eq!(cstr_to_string(third.name), "Z");
    cqlib_string_free(third.name);
    assert_eq!(third.error_rate, 0.003);
    assert_eq!(third.length, 80.0);

    // A two-qubit gate is rejected as a qubit-native instruction (arity).
    let cx = name_ptr("CX");
    assert_eq!(
        device_set_native_instruction(device, 0, cx.as_ptr(), 0.01, 300.0),
        -8
    );

    // Unknown gate names.
    let bogus = name_ptr("nope");
    assert_eq!(device_set_instruction(device, 0, 0, bogus.as_ptr()), -4);
    assert_eq!(
        device_set_native_instruction(device, 0, bogus.as_ptr(), 0.01, 1.0),
        -4
    );

    // NULL handling.
    assert_eq!(device_set_instruction(device, 0, 0, std::ptr::null()), -1);
    assert_eq!(
        device_set_native_instruction(device, 0, std::ptr::null(), 0.01, 1.0),
        -1
    );

    device_free(device);
}

#[test]
fn test_layout_result_distances_and_activity() {
    let name = name_ptr("dist-line");
    let device = device_line(name.as_ptr(), 3);
    assert!(!device.is_null());

    let graph = physical_layout_graph_from_device(device);
    assert!(!graph.is_null());

    // Two-step pattern: length query, then a full copy of the 3x3 table.
    assert_eq!(layout_result_distances(graph, std::ptr::null_mut(), 0), 9);
    let mut buf = [0u32; 9];
    assert_eq!(layout_result_distances(graph, buf.as_mut_ptr(), 9), 9);
    assert_eq!(buf, [0, 1, 2, 1, 0, 1, 2, 1, 0]);

    // A partial copy only touches the requested prefix.
    let mut part = [7u32; 9];
    assert_eq!(layout_result_distances(graph, part.as_mut_ptr(), 3), 9);
    assert_eq!(&part[..3], &[0, 1, 2]);
    assert_eq!(&part[3..], &[7; 6]);

    assert_eq!(
        layout_result_distances(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );
    physical_layout_graph_free(graph);

    // Activity from interactions cx(0,1) x2 and cx(1,2) x1.
    let circuit = circuit_new(3);
    assert!(!circuit.is_null());
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(circuit_cx(circuit, 1, 2), 0);

    let analysis = analyze_circuit_for_layout(circuit);
    assert!(!analysis.is_null());
    assert_eq!(
        layout_result_logical_activity(analysis, std::ptr::null_mut(), std::ptr::null_mut(), 0),
        3
    );
    let mut qubits = [0u32; 3];
    let mut act = [0.0f64; 3];
    assert_eq!(
        layout_result_logical_activity(analysis, qubits.as_mut_ptr(), act.as_mut_ptr(), 3),
        3
    );
    assert_eq!(qubits, [0, 1, 2]);
    let expected = [2.0, 3.0, 1.0];
    for (got, want) in act.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-12);
    }
    circuit_layout_analysis_free(analysis);

    // layout_result_analysis reuses the CCircuitLayoutAnalysis handle type.
    let prepared = prepare_sabre_circuit(circuit);
    assert!(!prepared.is_null());
    let reused = layout_result_analysis(prepared);
    assert!(!reused.is_null());
    assert_eq!(circuit_layout_analysis_num_logical(reused), 3);
    circuit_layout_analysis_free(reused);
    prepared_sabre_circuit_free(prepared);
    assert!(layout_result_analysis(std::ptr::null()).is_null());

    circuit_free(circuit);
    device_free(device);
}

#[test]
fn test_execution_result_status_flags_and_bitstring() {
    let task = name_ptr("task-1");
    let qubits = [0u32, 1, 2];
    let result = execution_result_new(task.as_ptr(), qubits.as_ptr(), 3, 100, std::ptr::null());
    assert!(!result.is_null());

    // Queued: neither success nor terminal.
    assert_eq!(execution_result_is_success(result), 0);
    assert_eq!(execution_result_is_terminal(result), 0);

    // Running is still not terminal.
    assert_eq!(execution_result_start(result), 0);
    assert_eq!(execution_result_is_terminal(result), 0);

    // Complete with counts.
    let keys = [name_ptr("000"), name_ptr("111"), name_ptr("101")];
    let key_ptrs = [keys[0].as_ptr(), keys[1].as_ptr(), keys[2].as_ptr()];
    let counts = [50u64, 30, 20];
    assert_eq!(
        execution_result_finish(result, key_ptrs.as_ptr(), counts.as_ptr(), 3),
        0
    );
    assert_eq!(execution_result_is_success(result), 1);
    assert_eq!(execution_result_is_terminal(result), 1);

    // Bitstring formatting: bit i of the mask is measured qubit i.
    for (mask, want) in [
        (0b001u64, "001"),
        (0b101, "101"),
        (0, "000"),
        (u64::MAX, "111"),
    ] {
        let s = execution_result_to_bitstring(result, mask);
        assert_eq!(cstr_to_string(s), want);
        cqlib_string_free(s);
    }
    assert!(execution_result_to_bitstring(std::ptr::null(), 0).is_null());

    // Failure and cancellation are terminal but not successful.
    let boom = name_ptr("boom");
    assert_eq!(execution_result_fail(result, boom.as_ptr(), 7), 0);
    assert_eq!(execution_result_is_success(result), 0);
    assert_eq!(execution_result_is_terminal(result), 1);
    assert_eq!(execution_result_cancel(result), 0);
    assert_eq!(execution_result_is_terminal(result), 1);

    assert_eq!(execution_result_is_success(std::ptr::null()), -1);
    assert_eq!(execution_result_is_terminal(std::ptr::null()), -1);

    execution_result_free(result);
}

#[test]
fn test_execution_result_calc_probabilities_two_step() {
    let task = name_ptr("task-2");
    let qubits = [0u32, 1];
    let result = execution_result_new(task.as_ptr(), qubits.as_ptr(), 2, 50, std::ptr::null());
    assert!(!result.is_null());
    let keys = [name_ptr("00"), name_ptr("11")];
    let key_ptrs = [keys[0].as_ptr(), keys[1].as_ptr()];
    let counts = [30u64, 20];
    assert_eq!(
        execution_result_finish(result, key_ptrs.as_ptr(), counts.as_ptr(), 2),
        0
    );

    // Two-step pattern with ascending outcome order ("00" then "11").
    assert_eq!(execution_result_calc_probabilities_len(result), 2);
    let mut probs = [0.0f64; 2];
    assert_eq!(
        execution_result_calc_probabilities(result, probs.as_mut_ptr(), 2),
        2
    );
    assert!((probs[0] - 0.6).abs() < 1e-12);
    assert!((probs[1] - 0.4).abs() < 1e-12);

    // A partial copy only touches the requested prefix.
    let mut part = [f64::NAN; 2];
    assert_eq!(
        execution_result_calc_probabilities(result, part.as_mut_ptr(), 1),
        2
    );
    assert!((part[0] - 0.6).abs() < 1e-12);
    assert!(part[1].is_nan());

    assert_eq!(
        execution_result_calc_probabilities_len(std::ptr::null_mut()),
        0
    );
    assert_eq!(
        execution_result_calc_probabilities(std::ptr::null_mut(), std::ptr::null_mut(), 0),
        0
    );

    execution_result_free(result);
}

#[test]
fn test_noise_model_validity_and_kraus() {
    let model = noise_model_new();
    assert!(!model.is_null());

    let h = name_ptr("H");
    assert_eq!(
        noise_model_add_single_qubit(model, h.as_ptr(), 0, NOISE_DEPOLARIZING, 0.1),
        0
    );
    assert_eq!(noise_model_is_valid(model, h.as_ptr(), 0, 0), 1);

    // Kraus output for depolarizing p=0.1: I*sqrt(0.9), X/Y/Z*sqrt(p/3),
    // four 2x2 operators written back to back in row-major order.
    assert_eq!(noise_model_to_kraus_len(model, h.as_ptr(), 0, 0), 16);
    let mut buf = vec![Complex64::new(0.0, 0.0); 16];
    assert_eq!(
        noise_model_to_kraus(model, h.as_ptr(), 0, 0, buf.as_mut_ptr(), 16),
        16
    );
    assert!((buf[0].re - 0.9f64.sqrt()).abs() < 1e-12);
    assert!(buf[0].im.abs() < 1e-12);
    assert!(buf[1].re.abs() < 1e-12);
    assert!((buf[3].re - 0.9f64.sqrt()).abs() < 1e-12);
    let p_other = (0.1f64 / 3.0).sqrt();
    assert!(buf[4].re.abs() < 1e-12);
    assert!((buf[5].re - p_other).abs() < 1e-12);
    assert!((buf[6].re - p_other).abs() < 1e-12);
    assert!(buf[7].re.abs() < 1e-12);

    // A second channel under a different gate key (X@0, its own index 0).
    let x = name_ptr("X");
    assert_eq!(
        noise_model_add_single_qubit(model, x.as_ptr(), 0, NOISE_AMPLITUDE_DAMPING, 0.25),
        0
    );
    assert_eq!(noise_model_is_valid(model, x.as_ptr(), 0, 0), 1);
    assert_eq!(noise_model_to_kraus_len(model, x.as_ptr(), 0, 0), 8);
    let mut buf2 = vec![Complex64::new(0.0, 0.0); 8];
    assert_eq!(
        noise_model_to_kraus(model, x.as_ptr(), 0, 0, buf2.as_mut_ptr(), 8),
        8
    );
    assert_eq!(buf2[0].re, 1.0);
    assert!((buf2[3].re - 0.75f64.sqrt()).abs() < 1e-12);
    assert!(buf2[4].re.abs() < 1e-12 && buf2[4].im.abs() < 1e-12);
    assert!((buf2[5].re - 0.5).abs() < 1e-12);
    assert!(buf2[7].re.abs() < 1e-12);

    // A partial copy only touches the requested prefix.
    let mut part = vec![Complex64::new(f64::NAN, f64::NAN); 8];
    assert_eq!(
        noise_model_to_kraus(model, x.as_ptr(), 0, 0, part.as_mut_ptr(), 2),
        8
    );
    assert!((part[0].re - 1.0).abs() < 1e-12);
    assert!(part[3].re.is_nan());

    // Errors: absent key, out-of-bounds index, unknown gate, NULL.
    assert_eq!(noise_model_is_valid(model, h.as_ptr(), 1, 0), -8);
    assert_eq!(noise_model_to_kraus_len(model, h.as_ptr(), 1, 0), 0);
    assert_eq!(noise_model_is_valid(model, h.as_ptr(), 0, 5), -8);
    let bogus = name_ptr("nope");
    assert_eq!(noise_model_is_valid(model, bogus.as_ptr(), 0, 0), -4);
    assert_eq!(noise_model_to_kraus_len(model, bogus.as_ptr(), 0, 0), 0);
    assert_eq!(noise_model_is_valid(std::ptr::null(), h.as_ptr(), 0, 0), -1);
    assert_eq!(
        noise_model_to_kraus_len(std::ptr::null(), h.as_ptr(), 0, 0),
        0
    );

    noise_model_free(model);
}
