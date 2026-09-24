//! Integration tests for device module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{
    circuit_cx, circuit_free, circuit_h, circuit_index, circuit_new, circuit_t,
    value_operation_free,
};
use binding_c::cqlib_string_free;
use binding_c::device::*;
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

#[test]
fn test_device_line_and_properties() {
    let name = name_ptr("line-2");
    let device = device_line(name.as_ptr(), 2);
    assert!(!device.is_null());

    assert_eq!(device_num_qubits(device), 2);

    let got = device_name(device);
    assert_eq!(cstr_to_string(got), "line-2");
    cqlib_string_free(got);

    let topo = device_topology(device);
    assert!(!topo.is_null());
    assert_eq!(topology_num_qubits(topo), 2);
    assert_eq!(topology_num_couplings(topo), 1);
    topology_free(topo);

    device_free(device);
}

#[test]
fn test_device_topology_variants() {
    let name = name_ptr("d");

    let ring = device_ring(name.as_ptr(), 4);
    assert!(!ring.is_null());
    assert_eq!(device_num_qubits(ring), 4);
    let topo = device_topology(ring);
    assert_eq!(topology_num_couplings(topo), 8); // bidirectional ring
    topology_free(topo);
    device_free(ring);

    let grid = device_grid(name.as_ptr(), 2, 3);
    assert!(!grid.is_null());
    assert_eq!(device_num_qubits(grid), 6);
    device_free(grid);

    let star = device_star(name.as_ptr(), 5, 0);
    assert!(!star.is_null());
    assert_eq!(device_num_qubits(star), 5);
    device_free(star);

    let bidi = device_bidirectional_line(name.as_ptr(), 3);
    assert!(!bidi.is_null());
    device_free(bidi);
}

#[test]
fn test_device_new_null_name() {
    assert!(device_line(std::ptr::null(), 2).is_null());
    assert!(device_new(std::ptr::null(), 2).is_null());
}

#[test]
fn test_device_free_null() {
    device_free(std::ptr::null_mut());
}

#[test]
fn test_device_from_edges() {
    let name = name_ptr("custom");
    // Path 0-1-2 plus one reverse edge 2->1.
    let edges: [u32; 6] = [0, 1, 1, 2, 2, 1];
    let device = device_from_edges(name.as_ptr(), 3, edges.as_ptr(), 3);
    assert!(!device.is_null());
    assert_eq!(device_num_qubits(device), 3);
    let topo = device_topology(device);
    assert_eq!(topology_num_couplings(topo), 3);
    topology_free(topo);
    device_free(device);
}

#[test]
fn test_device_with_native_gates_and_validate() {
    let name = name_ptr("bidi");
    let device = device_bidirectional_line(name.as_ptr(), 2);
    assert!(!device.is_null());

    // Before setting gates, native gate list is empty -> NULL.
    assert!(device_native_gates(device).is_null());

    let gates = name_ptr("H,CX,RZ");
    assert_eq!(device_with_native_gates(device, gates.as_ptr()), 0);
    let got = device_native_gates(device);
    let names = cstr_to_string(got);
    cqlib_string_free(got);
    assert_eq!(names, "H,CX,RZ");

    // Unknown gate name -> -4.
    let bad = name_ptr("H,NOT_A_GATE");
    assert_eq!(device_with_native_gates(device, bad.as_ptr()), -4);

    // A circuit using only native gates validates.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    assert_eq!(device_validate_circuit(device, circuit), 0);
    circuit_free(circuit);

    device_free(device);
}

#[test]
fn test_layout_from_pairs_and_get() {
    let pairs: [u32; 4] = [0, 2, 1, 0]; // (0->2), (1->0)
    let layout = layout_from_pairs(pairs.as_ptr(), 2, 4);
    assert!(!layout.is_null());
    assert_eq!(layout_num_logical(layout), 2);
    assert_eq!(layout_num_physical(layout), 4);
    assert_eq!(layout_get(layout, 0), 2);
    assert_eq!(layout_get(layout, 1), 0);
    // Unmapped logical -> MAX
    assert_eq!(layout_get(layout, 9), u32::MAX);
    layout_free(layout);
}

#[test]
fn test_layout_new_ordered_mapping() {
    let logical: [u32; 2] = [0, 1];
    let physical: [u32; 2] = [5, 3];
    let layout = layout_new(logical.as_ptr(), 2, physical.as_ptr(), 2);
    assert!(!layout.is_null());
    assert_eq!(layout_get(layout, 0), 5);
    assert_eq!(layout_get(layout, 1), 3);
    layout_free(layout);
}

#[test]
fn test_layout_null_and_free() {
    layout_free(std::ptr::null_mut());
    assert!(layout_new(std::ptr::null(), 0, std::ptr::null(), 0).is_null());
    assert!(layout_from_pairs(std::ptr::null(), 0, 4).is_null());
    assert_eq!(layout_num_logical(std::ptr::null()), 0);
    assert_eq!(layout_num_physical(std::ptr::null()), 0);
    assert_eq!(layout_get(std::ptr::null(), 0), u32::MAX);
}

#[test]
fn test_execution_result_counts_and_probabilities() {
    let task = name_ptr("task-1");
    let bits = [name_ptr("00"), name_ptr("11")];
    let bit_ptrs: [*const c_char; 2] = [bits[0].as_ptr(), bits[1].as_ptr()];
    let counts: [u64; 2] = [30, 20];
    let result =
        execution_result_from_counts(task.as_ptr(), 2, 50, bit_ptrs.as_ptr(), counts.as_ptr(), 2);
    assert!(!result.is_null());

    assert_eq!(execution_result_shots(result), 50);
    assert_eq!(execution_result_num_qubits(result), 2);

    let list = execution_result_counts(result);
    assert!(!list.is_null());
    assert_eq!(counts_list_len(list), 2);
    let mut total = 0.0;
    for i in 0..counts_list_len(list) {
        let key = counts_list_get_key(list, i);
        let k = cstr_to_string(key);
        cqlib_string_free(key);
        assert!(k == "00" || k == "11");
        total += counts_list_get_value(list, i);
    }
    assert!((total - 50.0).abs() < 1e-9);
    counts_list_free(list);

    let probs = execution_result_probabilities(result);
    assert!(!probs.is_null());
    let mut sum = 0.0;
    for i in 0..counts_list_len(probs) {
        sum += counts_list_get_value(probs, i);
    }
    assert!((sum - 1.0).abs() < 1e-9);
    counts_list_free(probs);

    execution_result_free(result);
}

#[test]
fn test_execution_result_null_and_free() {
    execution_result_free(std::ptr::null_mut());
    assert!(
        execution_result_from_counts(
            std::ptr::null(),
            2,
            10,
            std::ptr::null(),
            std::ptr::null(),
            0
        )
        .is_null()
    );
    assert_eq!(execution_result_shots(std::ptr::null()), 0);
    assert_eq!(execution_result_num_qubits(std::ptr::null()), 0);
    assert!(execution_result_counts(std::ptr::null()).is_null());
    assert!(execution_result_probabilities(std::ptr::null_mut()).is_null());
    counts_list_free(std::ptr::null_mut());
    assert_eq!(counts_list_len(std::ptr::null()), 0);
    assert!(counts_list_get_key(std::ptr::null(), 0).is_null());
    assert!(counts_list_get_value(std::ptr::null(), 0).is_nan());
}

#[test]
fn test_noise_model_add_errors() {
    let model = noise_model_new();
    assert!(!model.is_null());

    let h = name_ptr("H");
    let cx = name_ptr("CX");

    // Valid single-qubit depolarizing noise.
    assert_eq!(
        noise_model_add_single_qubit(model, h.as_ptr(), 0, NOISE_DEPOLARIZING, 0.001),
        0
    );

    // Unknown gate -> -4.
    assert_eq!(
        noise_model_add_single_qubit(
            model,
            name_ptr("BOGUS").as_ptr(),
            0,
            NOISE_DEPOLARIZING,
            0.001
        ),
        -4
    );
    // Unknown noise tag -> -8.
    assert_eq!(
        noise_model_add_single_qubit(model, h.as_ptr(), 0, 99, 0.001),
        -8
    );
    // Invalid probability -> -8.
    assert_eq!(
        noise_model_add_single_qubit(model, h.as_ptr(), 0, NOISE_DEPOLARIZING, 1.5),
        -8
    );

    // Valid two-qubit noise.
    assert_eq!(
        noise_model_add_two_qubit(model, cx.as_ptr(), 0, 1, NOISE_TWO_DEPOLARIZING, 0.01),
        0
    );
    // Same qubit collision -> -8.
    assert_eq!(
        noise_model_add_two_qubit(model, cx.as_ptr(), 1, 1, NOISE_TWO_DEPOLARIZING, 0.01),
        -8
    );

    // Valid readout error.
    assert_eq!(noise_model_add_readout(model, 0, 0.02, 0.01), 0);
    // Independent probabilities are valid even when their sum exceeds 1.
    assert_eq!(noise_model_add_readout(model, 1, 0.5, 0.7), 0);
    // Invalid readout probabilities -> -8.
    assert_eq!(noise_model_add_readout(model, 2, 0.5, 1.7), -8);

    noise_model_free(model);
    noise_model_free(std::ptr::null_mut());
}

// =====  Section 3.1: device attribute queries and updates  =====

#[test]
fn test_device_defaults_and_attribute_queries() {
    let name = name_ptr("dev");
    let device = device_new(name.as_ptr(), 3);
    assert!(!device.is_null());

    // Defaults are unset -> -8 with NaN written to the out parameter.
    let mut v = 0.0f64;
    assert_eq!(device_default_t1(device, &mut v), -8);
    assert!(v.is_nan());
    assert_eq!(device_default_t2(device, &mut v), -8);
    assert_eq!(device_default_readout_error(device, &mut v), -8);
    assert_eq!(device_default_single_qubit_error(device, &mut v), -8);
    assert_eq!(device_default_two_qubit_error(device, &mut v), -8);

    // Set and read back the five device-wide defaults.
    assert_eq!(device_set_default_t1(device, 40.0), 0);
    assert_eq!(device_set_default_t2(device, 35.0), 0);
    assert_eq!(device_set_default_readout_error(device, 0.02), 0);
    assert_eq!(device_set_default_single_qubit_error(device, 0.005), 0);
    assert_eq!(device_set_default_two_qubit_error(device, 0.01), 0);
    assert_eq!(device_default_t1(device, &mut v), 0);
    assert_eq!(v, 40.0);
    assert_eq!(device_default_t2(device, &mut v), 0);
    assert_eq!(v, 35.0);
    assert_eq!(device_default_readout_error(device, &mut v), 0);
    assert_eq!(v, 0.02);
    assert_eq!(device_default_single_qubit_error(device, &mut v), 0);
    assert_eq!(v, 0.005);
    assert_eq!(device_default_two_qubit_error(device, &mut v), 0);
    assert_eq!(v, 0.01);

    // Per-qubit queries fall back to the defaults; registration is not checked.
    assert_eq!(device_get_t1(device, 0, &mut v), 0);
    assert_eq!(v, 40.0);
    assert_eq!(device_get_t2(device, 0, &mut v), 0);
    assert_eq!(v, 35.0);
    assert_eq!(device_get_readout_error(device, 2, &mut v), 0);
    assert_eq!(v, 0.02);
    assert_eq!(device_get_t1(device, 9, &mut v), 0);
    assert_eq!(v, 40.0);

    // Calibration time is unset -> -8.
    let mut ms = 0i64;
    assert_eq!(device_calibration_time(device, &mut ms), -8);

    // NULL guards.
    assert_eq!(device_get_t1(std::ptr::null(), 0, &mut v), -1);
    assert_eq!(device_get_readout_error(std::ptr::null(), 0, &mut v), -1);
    assert_eq!(device_set_default_t1(std::ptr::null_mut(), 1.0), -1);
    assert_eq!(device_calibration_time(std::ptr::null(), &mut ms), -1);

    device_free(device);
}

#[test]
fn test_device_invalid_and_usable_qubits() {
    let name = name_ptr("dev");
    let device = device_new(name.as_ptr(), 4);
    assert!(!device.is_null());

    assert_eq!(device_invalid_qubits_len(device), 0);
    let invalid: [u32; 2] = [1, 3];
    assert_eq!(device_set_invalid_qubits(device, invalid.as_ptr(), 2), 0);
    assert_eq!(device_invalid_qubits_len(device), 2);
    let mut got = [0u32; 2];
    assert_eq!(device_invalid_qubits(device, got.as_mut_ptr(), 2), 2);
    let mut sorted: Vec<u32> = got.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![1, 3]);

    assert_eq!(device_is_usable_qubit(device, 0), 1);
    assert_eq!(device_is_usable_qubit(device, 1), 0);
    assert_eq!(device_is_usable_qubit(device, 3), 0);
    assert_eq!(device_is_usable_qubit(device, 9), 0); // unregistered
    assert_eq!(device_num_usable_qubits(device), 2);

    assert_eq!(device_usable_qubits_len(device), 2);
    let mut usable = [0u32; 2];
    assert_eq!(device_usable_qubits(device, usable.as_mut_ptr(), 2), 2);
    let mut sorted: Vec<u32> = usable.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 2]);

    assert_eq!(device_qubits_len(device), 4);
    let mut all = [0u32; 4];
    assert_eq!(device_qubits(device, all.as_mut_ptr(), 4), 4);
    let mut sorted: Vec<u32> = all.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1, 2, 3]);

    // Unregistered qubits are rejected and the existing set is preserved.
    let bad: [u32; 1] = [9];
    assert_eq!(device_set_invalid_qubits(device, bad.as_ptr(), 1), -2);
    assert_eq!(device_invalid_qubits_len(device), 2);

    // An invalid qubit hides its single-qubit error even when inherited.
    let gates = name_ptr("H");
    assert_eq!(device_with_native_gates(device, gates.as_ptr()), 0);
    assert_eq!(device_set_default_single_qubit_error(device, 0.005), 0);
    let h = name_ptr("H");
    let mut v = 0.0f64;
    assert_eq!(device_single_qubit_error(device, h.as_ptr(), 0, &mut v), 0);
    assert_eq!(v, 0.005);
    assert_eq!(device_single_qubit_error(device, h.as_ptr(), 1, &mut v), -8);

    // Clearing the invalid set restores full usability.
    assert_eq!(device_set_invalid_qubits(device, std::ptr::null(), 0), 0);
    assert_eq!(device_num_usable_qubits(device), 4);

    // NULL guards.
    assert_eq!(
        device_set_invalid_qubits(std::ptr::null_mut(), invalid.as_ptr(), 2),
        -1
    );
    assert_eq!(device_set_invalid_qubits(device, std::ptr::null(), 2), -1);
    assert_eq!(device_is_usable_qubit(std::ptr::null(), 0), -1);
    assert_eq!(device_num_usable_qubits(std::ptr::null()), 0);
    assert_eq!(device_usable_qubits_len(std::ptr::null()), 0);
    assert_eq!(device_invalid_qubits_len(std::ptr::null()), 0);
    assert_eq!(device_qubits_len(std::ptr::null()), 0);

    device_free(device);
}

#[test]
fn test_device_qubit_and_edge_properties() {
    let name = name_ptr("line");
    let device = device_line(name.as_ptr(), 3);
    assert!(!device.is_null());

    // No properties recorded yet.
    let mut qprop = CQubitProp {
        readout_error: 0.0,
        t1: 0.0,
        t2: 0.0,
        prob_meas0_prep1: 0.0,
        prob_meas1_prep0: 0.0,
        frequency: 0.0,
        num_native_instructions: 0,
    };
    assert_eq!(device_qubit_properties(device, 0, &mut qprop), -8);
    let mut eprop = CEdgeProp {
        num_native_instructions: 0,
    };
    assert_eq!(device_edge_properties(device, 0, 1, &mut eprop), -8);

    let h = name_ptr("H");
    let cx = name_ptr("CX");
    let mut v = 0.0f64;
    // H is not in the device-wide native gates -> unsupported -> -8.
    assert_eq!(device_single_qubit_error(device, h.as_ptr(), 0, &mut v), -8);

    // Record qubit-0 properties with one native H instruction.
    let gate_names: [*const c_char; 1] = [h.as_ptr()];
    let error_rates: [f64; 1] = [0.001];
    let lengths: [f64; 1] = [50.0];
    let input = CQubitPropInput {
        readout_error: 0.01,
        t1: 20.0,
        t2: 30.0,
        prob_meas0_prep1: 0.02,
        prob_meas1_prep0: 0.03,
        frequency: 5.0,
        native_gate_names: gate_names.as_ptr(),
        native_error_rates: error_rates.as_ptr(),
        native_lengths: lengths.as_ptr(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_qubit_properties(device, 0, &input), 0);

    assert_eq!(device_qubit_properties(device, 0, &mut qprop), 0);
    assert_eq!(qprop.readout_error, 0.01);
    assert_eq!(qprop.t1, 20.0);
    assert_eq!(qprop.t2, 30.0);
    assert_eq!(qprop.prob_meas0_prep1, 0.02);
    assert_eq!(qprop.prob_meas1_prep0, 0.03);
    assert_eq!(qprop.frequency, 5.0);
    assert_eq!(qprop.num_native_instructions, 1);

    let mut native = CNativeInstruction {
        name: std::ptr::null_mut(),
        error_rate: 0.0,
        length: 0.0,
    };
    assert_eq!(
        device_qubit_prop_native_instruction(device, 0, 0, &mut native),
        0
    );
    assert_eq!(cstr_to_string(native.name), "H");
    cqlib_string_free(native.name);
    assert_eq!(native.error_rate, 0.001);
    assert_eq!(native.length, 50.0);
    assert_eq!(
        device_qubit_prop_native_instruction(device, 0, 1, &mut native),
        -8
    );

    // Per-qubit values now win over the (unset) defaults.
    assert_eq!(device_get_t1(device, 0, &mut v), 0);
    assert_eq!(v, 20.0);
    assert_eq!(device_get_t2(device, 0, &mut v), 0);
    assert_eq!(v, 30.0);
    assert_eq!(device_get_readout_error(device, 0, &mut v), 0);
    assert_eq!(v, 0.01);

    // A non-empty local list is authoritative: H is explicit, X unsupported.
    assert_eq!(device_single_qubit_error(device, h.as_ptr(), 0, &mut v), 0);
    assert_eq!(v, 0.001);
    let x = name_ptr("X");
    assert_eq!(device_single_qubit_error(device, x.as_ptr(), 0, &mut v), -8);
    // Two-qubit gates never resolve on single qubits.
    assert_eq!(
        device_single_qubit_error(device, cx.as_ptr(), 0, &mut v),
        -8
    );
    // Unknown gate name -> -4.
    assert_eq!(
        device_single_qubit_error(device, name_ptr("BOGUS").as_ptr(), 0, &mut v),
        -4
    );

    // Record edge (0 -> 1) properties with one native CX instruction.
    let edge_names: [*const c_char; 1] = [cx.as_ptr()];
    let edge_rates: [f64; 1] = [0.02];
    let edge_lengths: [f64; 1] = [100.0];
    let edge_input = CEdgePropInput {
        native_gate_names: edge_names.as_ptr(),
        native_error_rates: edge_rates.as_ptr(),
        native_lengths: edge_lengths.as_ptr(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_edge_properties(device, 0, 1, &edge_input), 0);
    assert_eq!(device_edge_properties(device, 0, 1, &mut eprop), 0);
    assert_eq!(eprop.num_native_instructions, 1);
    assert_eq!(
        device_edge_prop_native_instruction(device, 0, 1, 0, &mut native),
        0
    );
    assert_eq!(cstr_to_string(native.name), "CX");
    cqlib_string_free(native.name);
    assert_eq!(native.error_rate, 0.02);
    assert_eq!(native.length, 100.0);
    assert_eq!(
        device_edge_prop_native_instruction(device, 0, 1, 1, &mut native),
        -8
    );

    assert_eq!(device_two_qubit_error(device, cx.as_ptr(), 0, 1, &mut v), 0);
    assert_eq!(v, 0.02);
    // The directed line has no reverse coupling 1 -> 0 and no coupling 0 -> 2.
    assert_eq!(
        device_two_qubit_error(device, cx.as_ptr(), 1, 0, &mut v),
        -8
    );
    assert_eq!(
        device_two_qubit_error(device, cx.as_ptr(), 0, 2, &mut v),
        -8
    );
    // One-qubit gates never resolve on couplings.
    assert_eq!(device_two_qubit_error(device, h.as_ptr(), 0, 1, &mut v), -8);
    assert_eq!(
        device_two_qubit_error(device, name_ptr("BOGUS").as_ptr(), 0, 1, &mut v),
        -4
    );

    // edge_error returns the best calibrated edge instruction error.
    assert_eq!(device_edge_error(device, 0, 1, &mut v), 0);
    assert_eq!(v, 0.02);
    assert_eq!(device_edge_error(device, 1, 0, &mut v), -8);
    // 1 -> 2 exists but has no calibration and no default -> -8.
    assert_eq!(device_edge_error(device, 1, 2, &mut v), -8);
    assert_eq!(device_edge_error(std::ptr::null(), 0, 1, &mut v), -1);

    // Unregistered qubits and unknown couplings are rejected.
    assert_eq!(device_add_qubit_properties(device, 9, &input), -2);
    assert_eq!(device_add_edge_properties(device, 2, 1, &edge_input), -2);
    assert_eq!(device_add_qubit_properties(device, 1, std::ptr::null()), -1);
    assert_eq!(
        device_add_edge_properties(device, 0, 1, std::ptr::null()),
        -1
    );

    // Native-list validation: wrong arity -> -8, unknown gate -> -4, bad rate -> -8.
    let h_names: [*const c_char; 1] = [h.as_ptr()];
    let cx_names: [*const c_char; 1] = [cx.as_ptr()];
    let bogus = name_ptr("BOGUS");
    let bogus_names: [*const c_char; 1] = [bogus.as_ptr()];
    let bad_rate: [f64; 1] = [1.5];
    let bad_edge = CEdgePropInput {
        native_gate_names: h_names.as_ptr(),
        native_error_rates: edge_rates.as_ptr(),
        native_lengths: std::ptr::null(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_edge_properties(device, 0, 1, &bad_edge), -8);
    let bad_qubit = CQubitPropInput {
        readout_error: 0.01,
        t1: f64::NAN,
        t2: f64::NAN,
        prob_meas0_prep1: f64::NAN,
        prob_meas1_prep0: f64::NAN,
        frequency: f64::NAN,
        native_gate_names: cx_names.as_ptr(),
        native_error_rates: edge_rates.as_ptr(),
        native_lengths: std::ptr::null(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_qubit_properties(device, 1, &bad_qubit), -8);
    let bogus_input = CQubitPropInput {
        readout_error: 0.01,
        t1: f64::NAN,
        t2: f64::NAN,
        prob_meas0_prep1: f64::NAN,
        prob_meas1_prep0: f64::NAN,
        frequency: f64::NAN,
        native_gate_names: bogus_names.as_ptr(),
        native_error_rates: edge_rates.as_ptr(),
        native_lengths: std::ptr::null(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_qubit_properties(device, 1, &bogus_input), -4);
    let bad_rate_input = CQubitPropInput {
        readout_error: 0.01,
        t1: f64::NAN,
        t2: f64::NAN,
        prob_meas0_prep1: f64::NAN,
        prob_meas1_prep0: f64::NAN,
        frequency: f64::NAN,
        native_gate_names: h_names.as_ptr(),
        native_error_rates: bad_rate.as_ptr(),
        native_lengths: std::ptr::null(),
        num_native_instructions: 1,
    };
    assert_eq!(device_add_qubit_properties(device, 1, &bad_rate_input), -8);

    // supports_native_instruction capability queries.
    let qargs0: [u32; 1] = [0];
    let qargs1: [u32; 1] = [1];
    let pair01: [u32; 2] = [0, 1];
    let pair10: [u32; 2] = [1, 0];
    assert_eq!(
        device_supports_native_instruction(device, h.as_ptr(), qargs0.as_ptr(), 1),
        1
    );
    assert_eq!(
        device_supports_native_instruction(device, h.as_ptr(), qargs1.as_ptr(), 1),
        0
    );
    assert_eq!(
        device_supports_native_instruction(device, cx.as_ptr(), pair01.as_ptr(), 2),
        1
    );
    assert_eq!(
        device_supports_native_instruction(device, cx.as_ptr(), pair10.as_ptr(), 2),
        0
    );
    // Arity mismatch is not a capability.
    assert_eq!(
        device_supports_native_instruction(device, cx.as_ptr(), qargs0.as_ptr(), 1),
        0
    );
    assert_eq!(
        device_supports_native_instruction(device, name_ptr("BOGUS").as_ptr(), qargs0.as_ptr(), 1),
        -4
    );
    assert_eq!(
        device_supports_native_instruction(device, h.as_ptr(), std::ptr::null(), 1),
        -1
    );
    // GPhase carries no qargs; it is not in the device native gates here.
    let gphase = name_ptr("GPhase");
    assert_eq!(
        device_supports_native_instruction(device, gphase.as_ptr(), std::ptr::null(), 0),
        0
    );

    device_free(device);
}

#[test]
fn test_device_line_from_qubits_and_validate_operation() {
    let name = name_ptr("line7");
    let qubits: [u32; 3] = [5, 6, 7];
    let device = device_line_from_qubits(name.as_ptr(), qubits.as_ptr(), 3);
    assert!(!device.is_null());
    assert_eq!(device_qubits_len(device), 3);
    let mut got = [0u32; 3];
    assert_eq!(device_qubits(device, got.as_mut_ptr(), 3), 3);
    assert_eq!(got, [5, 6, 7]);
    assert_eq!(device_num_usable_qubits(device), 3);
    device_free(device);

    // NULL guards.
    assert!(device_line_from_qubits(std::ptr::null(), qubits.as_ptr(), 3).is_null());
    assert!(device_line_from_qubits(name.as_ptr(), std::ptr::null(), 3).is_null());

    // validate_operation consumes a CValueOperation snapshot from circuit_index.
    let dev_name = name_ptr("dev");
    let dev = device_line(dev_name.as_ptr(), 2);
    let gates = name_ptr("H,CX");
    assert_eq!(device_with_native_gates(dev, gates.as_ptr()), 0);

    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    let op = circuit_index(circuit, 0);
    assert!(!op.is_null());
    assert_eq!(device_validate_operation(dev, op), 0);
    assert_eq!(device_validate_value_operation(dev, op), 0);
    value_operation_free(op);

    // T is not a native gate -> the device rejects the operation.
    assert_eq!(circuit_t(circuit, 0), 0);
    let bad_op = circuit_index(circuit, 2);
    assert!(!bad_op.is_null());
    assert_eq!(device_validate_operation(dev, bad_op), -3);
    assert_eq!(device_validate_value_operation(dev, bad_op), -3);
    value_operation_free(bad_op);
    assert_eq!(device_validate_operation(dev, std::ptr::null()), -1);
    assert_eq!(device_validate_value_operation(dev, std::ptr::null()), -1);

    circuit_free(circuit);
    device_free(dev);
}

// =====  Section 3.2: layout binding and map introspection  =====

#[test]
fn test_layout_binding_and_maps() {
    let pairs: [u32; 4] = [0, 2, 1, 0]; // L0->P2, L1->P0
    let layout = layout_from_pairs(pairs.as_ptr(), 2, 4);
    assert!(!layout.is_null());

    // Reverse lookup distinguishes vacant and unknown physical qubits.
    assert_eq!(layout_get_logical(layout, 2), 0);
    assert_eq!(layout_get_logical(layout, 0), 1);
    assert_eq!(layout_get_logical(layout, 1), u32::MAX);
    assert_eq!(layout_get_logical(layout, 3), u32::MAX);
    assert_eq!(layout_is_physical_vacant(layout, 1), 1);
    assert_eq!(layout_is_physical_vacant(layout, 3), 1);
    assert_eq!(layout_is_physical_vacant(layout, 0), 0);
    assert_eq!(layout_is_physical_vacant(layout, 2), 0);
    assert_eq!(layout_is_physical_vacant(layout, 9), 0);

    // Bind a new logical qubit onto a vacant physical qubit.
    assert_eq!(layout_bind(layout, 2, 1), 0);
    assert_eq!(layout_get(layout, 2), 1);
    assert_eq!(layout_get_logical(layout, 1), 2);
    // Either side already mapped -> -8.
    assert_eq!(layout_bind(layout, 2, 3), -8);
    assert_eq!(layout_bind(layout, 3, 1), -8);
    // Physical qubit outside the layout -> -2.
    assert_eq!(layout_bind(layout, 3, 9), -2);
    // Unbinding an unmapped logical qubit -> -8.
    let mut released = 0u32;
    assert_eq!(layout_unbind(layout, 9, &mut released), -8);

    // Full map introspection.
    assert_eq!(layout_l2p_map_len(layout), 3);
    let mut l2p = [0u32; 6];
    assert_eq!(layout_l2p_map(layout, l2p.as_mut_ptr(), 3), 3);
    assert_eq!(l2p, [0, 2, 1, 0, 2, 1]);
    assert_eq!(layout_p2l_map_len(layout), 3);
    let mut p2l = [0u32; 6];
    assert_eq!(layout_p2l_map(layout, p2l.as_mut_ptr(), 3), 3);
    assert_eq!(p2l, [0, 1, 1, 2, 2, 0]);
    assert_eq!(layout_logical_qubits_len(layout), 3);
    let mut logical = [0u32; 3];
    assert_eq!(layout_logical_qubits(layout, logical.as_mut_ptr(), 3), 3);
    assert_eq!(logical, [0, 1, 2]);
    assert_eq!(layout_physical_qubits_len(layout), 4);
    let mut physical = [0u32; 4];
    assert_eq!(layout_physical_qubits(layout, physical.as_mut_ptr(), 4), 4);
    assert_eq!(physical, [0, 1, 2, 3]);
    assert_eq!(layout_num_vacant_physical(layout), 1);
    assert_eq!(layout_vacant_physical_qubits_len(layout), 1);
    let mut vacant = [0u32; 1];
    assert_eq!(
        layout_vacant_physical_qubits(layout, vacant.as_mut_ptr(), 1),
        1
    );
    assert_eq!(vacant, [3]);

    // Unbind releases the physical qubit.
    assert_eq!(layout_unbind(layout, 0, &mut released), 0);
    assert_eq!(released, 2);
    assert_eq!(layout_get(layout, 0), u32::MAX);
    assert_eq!(layout_is_physical_vacant(layout, 2), 1);
    assert_eq!(layout_num_vacant_physical(layout), 2);

    // Swapping with a vacant physical qubit moves the logical qubit.
    assert_eq!(layout_swap_physical(layout, 1, 3), 0);
    assert_eq!(layout_get(layout, 2), 3);
    assert_eq!(layout_get_logical(layout, 1), u32::MAX);
    assert_eq!(layout_get_logical(layout, 3), 2);
    // Unknown physical qubits -> -2.
    assert_eq!(layout_swap_physical(layout, 1, 99), -2);

    // NULL guards.
    assert_eq!(layout_bind(std::ptr::null_mut(), 0, 0), -1);
    assert_eq!(layout_unbind(std::ptr::null_mut(), 0, &mut released), -1);
    assert_eq!(layout_get_logical(std::ptr::null(), 0), u32::MAX);
    assert_eq!(layout_is_physical_vacant(std::ptr::null(), 0), -1);
    assert_eq!(layout_l2p_map_len(std::ptr::null()), 0);
    assert_eq!(layout_p2l_map_len(std::ptr::null()), 0);
    assert_eq!(layout_logical_qubits_len(std::ptr::null()), 0);
    assert_eq!(layout_physical_qubits_len(std::ptr::null()), 0);
    assert_eq!(layout_num_vacant_physical(std::ptr::null()), 0);
    assert_eq!(layout_vacant_physical_qubits_len(std::ptr::null()), 0);
    assert_eq!(layout_swap_physical(std::ptr::null_mut(), 0, 1), -1);

    layout_free(layout);
}

// =====  Section 3.3: topology graph queries and updates  =====

#[test]
fn test_topology_graph_queries_and_updates() {
    let qubits: [u32; 3] = [0, 1, 2];
    let topo = topology_line(qubits.as_ptr(), 3);
    assert!(!topo.is_null());
    assert_eq!(topology_num_qubits(topo), 3);
    assert_eq!(topology_num_couplings(topo), 2);

    assert_eq!(topology_contains_qubit(topo, 0), 1);
    assert_eq!(topology_contains_qubit(topo, 9), 0);
    assert_eq!(topology_contains_qubit(std::ptr::null(), 0), -1);

    // Qubit listing (two-step pattern).
    assert_eq!(topology_qubits_len(topo), 3);
    let mut got = [0u32; 3];
    assert_eq!(topology_qubits(topo, got.as_mut_ptr(), 3), 3);
    let mut sorted: Vec<u32> = got.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1, 2]);

    // Directed line: 0 -> 1 -> 2.
    let mut neighbors = [0u32; 2];
    assert_eq!(topology_neighbors_undirected_len(topo, 1), 2);
    assert_eq!(
        topology_neighbors_undirected(topo, 1, neighbors.as_mut_ptr(), 2),
        2
    );
    assert_eq!(neighbors, [0, 2]);
    let mut preds = [0u32; 1];
    assert_eq!(topology_predecessors_len(topo, 1), 1);
    assert_eq!(topology_predecessors(topo, 1, preds.as_mut_ptr(), 1), 1);
    assert_eq!(preds, [0]);
    let mut succs = [0u32; 1];
    assert_eq!(topology_successors_len(topo, 1), 1);
    assert_eq!(topology_successors(topo, 1, succs.as_mut_ptr(), 1), 1);
    assert_eq!(succs, [2]);
    assert_eq!(topology_in_degree(topo, 1), 1);
    assert_eq!(topology_out_degree(topo, 1), 1);
    // Unknown qubits yield empty results.
    assert_eq!(topology_neighbors_undirected_len(topo, 9), 0);
    assert_eq!(topology_predecessors_len(topo, 9), 0);
    assert_eq!(topology_successors_len(topo, 9), 0);
    assert_eq!(topology_in_degree(topo, 9), 0);
    assert_eq!(topology_out_degree(topo, 9), 0);

    assert_eq!(topology_supports_directed_coupling(topo, 0, 1), 1);
    assert_eq!(topology_supports_directed_coupling(topo, 1, 0), 0);
    assert_eq!(topology_supports_coupling_either_direction(topo, 1, 0), 1);
    assert_eq!(topology_supports_coupling_either_direction(topo, 0, 2), 0);
    assert_eq!(
        topology_supports_directed_coupling(std::ptr::null(), 0, 1),
        -1
    );
    assert_eq!(
        topology_supports_coupling_either_direction(std::ptr::null(), 0, 1),
        -1
    );

    // Undirected edge listing collapses direction.
    assert_eq!(topology_undirected_edges_len(topo), 2);
    let mut undirected = [0u32; 4];
    assert_eq!(
        topology_undirected_edges(topo, undirected.as_mut_ptr(), 2),
        2
    );
    assert_eq!(undirected, [0, 1, 1, 2]);
    assert_eq!(topology_undirected_edges_len(std::ptr::null()), 0);

    // Add a qubit and a named coupling.
    let five: [u32; 1] = [5];
    assert_eq!(topology_add_qubits(topo, five.as_ptr(), 1), 0);
    assert_eq!(topology_qubits_len(topo), 4);
    assert_eq!(topology_contains_qubit(topo, 5), 1);
    // Duplicate qubits -> -8; removal of unknown qubits -> -2.
    assert_eq!(topology_add_qubits(topo, five.as_ptr(), 1), -8);
    let nine: [u32; 1] = [9];
    assert_eq!(topology_remove_qubits(topo, nine.as_ptr(), 1), -2);

    let edge: [u32; 2] = [1, 5];
    let coupling_name = name_ptr("c15");
    let names: [*const c_char; 1] = [coupling_name.as_ptr()];
    assert_eq!(
        topology_add_couplings(topo, edge.as_ptr(), 1, names.as_ptr()),
        0
    );
    assert_eq!(topology_num_couplings(topo), 3);
    let got_name = topology_get_coupling_name(topo, 1, 5);
    assert_eq!(cstr_to_string(got_name), "c15");
    cqlib_string_free(got_name);
    // Existing but unnamed coupling -> empty string; missing coupling -> NULL.
    let unnamed = topology_get_coupling_name(topo, 0, 1);
    assert_eq!(cstr_to_string(unnamed), "");
    cqlib_string_free(unnamed);
    assert!(topology_get_coupling_name(topo, 5, 1).is_null());
    assert!(topology_get_coupling_name(std::ptr::null(), 0, 1).is_null());

    assert_eq!(topology_supports_directed_coupling(topo, 1, 5), 1);
    assert_eq!(topology_supports_coupling_either_direction(topo, 5, 1), 1);
    // The new edge extends qubit 1's neighborhood and out-degree.
    let mut neighbors2 = [0u32; 3];
    assert_eq!(topology_neighbors_undirected_len(topo, 1), 3);
    assert_eq!(
        topology_neighbors_undirected(topo, 1, neighbors2.as_mut_ptr(), 3),
        3
    );
    assert_eq!(neighbors2, [0, 2, 5]);
    let mut succs2 = [0u32; 2];
    assert_eq!(topology_successors_len(topo, 1), 2);
    assert_eq!(topology_successors(topo, 1, succs2.as_mut_ptr(), 2), 2);
    assert_eq!(succs2, [2, 5]);
    assert_eq!(topology_in_degree(topo, 1), 1);
    assert_eq!(topology_out_degree(topo, 1), 2);

    // Coupling updates.
    assert_eq!(topology_remove_couplings(topo, edge.as_ptr(), 1), 0);
    assert_eq!(topology_supports_directed_coupling(topo, 1, 5), 0);
    // Removing a missing coupling -> -2; duplicate add -> -8.
    assert_eq!(topology_remove_couplings(topo, edge.as_ptr(), 1), -2);
    assert_eq!(
        topology_add_couplings(topo, edge.as_ptr(), 1, names.as_ptr()),
        0
    );
    assert_eq!(
        topology_add_couplings(topo, edge.as_ptr(), 1, names.as_ptr()),
        -8
    );
    // Couplings to unknown qubits -> -2; self couplings -> -8.
    let bad_edge: [u32; 2] = [0, 9];
    assert_eq!(
        topology_add_couplings(topo, bad_edge.as_ptr(), 1, std::ptr::null()),
        -2
    );
    let self_edge: [u32; 2] = [0, 0];
    assert_eq!(
        topology_add_couplings(topo, self_edge.as_ptr(), 1, std::ptr::null()),
        -8
    );
    // NULL guards.
    assert_eq!(
        topology_add_qubits(std::ptr::null_mut(), five.as_ptr(), 1),
        -1
    );
    assert_eq!(
        topology_add_couplings(std::ptr::null_mut(), edge.as_ptr(), 1, std::ptr::null()),
        -1
    );
    assert_eq!(
        topology_remove_couplings(std::ptr::null_mut(), edge.as_ptr(), 1),
        -1
    );
    assert_eq!(topology_add_qubits(topo, std::ptr::null(), 1), -1);
    assert_eq!(topology_qubits_len(std::ptr::null()), 0);
    assert_eq!(topology_neighbors_undirected_len(std::ptr::null(), 0), 0);
    assert_eq!(topology_predecessors_len(std::ptr::null(), 0), 0);
    assert_eq!(topology_successors_len(std::ptr::null(), 0), 0);

    // Removing a qubit drops every coupling touching it.
    assert_eq!(topology_remove_qubits(topo, five.as_ptr(), 1), 0);
    assert_eq!(topology_qubits_len(topo), 3);
    assert_eq!(topology_contains_qubit(topo, 5), 0);
    assert_eq!(topology_num_couplings(topo), 2);

    topology_free(topo);
    assert!(topology_line(std::ptr::null(), 3).is_null());
}

// =====  Section 3.4: Pauli channels and noise queries  =====

#[test]
fn test_noise_model_pauli_and_queries() {
    let model = noise_model_new();
    assert!(!model.is_null());

    let h = name_ptr("H");
    let cx = name_ptr("CX");

    // Pauli channel add + query.
    assert_eq!(
        noise_model_add_single_qubit_pauli(model, h.as_ptr(), 0, 0.01, 0.02, 0.03),
        0
    );
    assert_eq!(
        noise_model_add_single_qubit_pauli(model, name_ptr("BOGUS").as_ptr(), 0, 0.01, 0.0, 0.0),
        -4
    );
    // Probabilities must stay within [0, 1] and sum to at most 1.
    assert_eq!(
        noise_model_add_single_qubit_pauli(model, h.as_ptr(), 0, 0.5, 0.5, 0.5),
        -8
    );
    assert_eq!(
        noise_model_add_single_qubit_pauli(std::ptr::null_mut(), h.as_ptr(), 0, 0.01, 0.0, 0.0),
        -1
    );

    assert_eq!(
        noise_model_get_single_qubit_errors_len(model, h.as_ptr(), 0),
        1
    );
    let mut channels = [CSingleQubitNoise {
        tag: 0,
        p: 0.0,
        px: 0.0,
        py: 0.0,
        pz: 0.0,
    }];
    assert_eq!(
        noise_model_get_single_qubit_errors(model, h.as_ptr(), 0, channels.as_mut_ptr(), 1),
        1
    );
    assert_eq!(channels[0].tag, NOISE_PAULI);
    assert_eq!(channels[0].px, 0.01);
    assert_eq!(channels[0].py, 0.02);
    assert_eq!(channels[0].pz, 0.03);

    // A second channel on the same key.
    assert_eq!(
        noise_model_add_single_qubit(model, h.as_ptr(), 0, NOISE_DEPOLARIZING, 0.001),
        0
    );
    assert_eq!(
        noise_model_get_single_qubit_errors_len(model, h.as_ptr(), 0),
        2
    );
    let mut two = [
        CSingleQubitNoise {
            tag: 0,
            p: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        },
        CSingleQubitNoise {
            tag: 0,
            p: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        },
    ];
    assert_eq!(
        noise_model_get_single_qubit_errors(model, h.as_ptr(), 0, two.as_mut_ptr(), 2),
        2
    );
    assert!(
        two.iter()
            .any(|c| c.tag == NOISE_DEPOLARIZING && (c.p - 0.001).abs() < 1e-12)
    );
    assert!(two.iter().any(|c| c.tag == NOISE_PAULI));

    // Absent keys and unknown gates yield empty lists.
    assert_eq!(
        noise_model_get_single_qubit_errors_len(model, h.as_ptr(), 1),
        0
    );
    assert_eq!(
        noise_model_get_single_qubit_errors_len(model, name_ptr("BOGUS").as_ptr(), 0),
        0
    );
    assert_eq!(
        noise_model_get_single_qubit_errors_len(std::ptr::null(), h.as_ptr(), 0),
        0
    );

    // Two-qubit queries.
    assert_eq!(
        noise_model_add_two_qubit(model, cx.as_ptr(), 0, 1, NOISE_TWO_DEPOLARIZING, 0.01),
        0
    );
    assert_eq!(
        noise_model_get_two_qubit_errors_len(model, cx.as_ptr(), 0, 1),
        1
    );
    let make_single = || CSingleQubitNoise {
        tag: 0,
        p: 0.0,
        px: 0.0,
        py: 0.0,
        pz: 0.0,
    };
    let mut two_q = [CTwoQubitNoise {
        tag: 0,
        p: 0.0,
        q0_noise: make_single(),
        q1_noise: make_single(),
        op_q0: 0,
        op_q1: 0,
    }];
    assert_eq!(
        noise_model_get_two_qubit_errors(model, cx.as_ptr(), 0, 1, two_q.as_mut_ptr(), 1),
        1
    );
    assert_eq!(two_q[0].tag, NOISE_TWO_DEPOLARIZING);
    assert_eq!(two_q[0].p, 0.01);
    // Direction, collisions, and unknown gates yield empty lists.
    assert_eq!(
        noise_model_get_two_qubit_errors_len(model, cx.as_ptr(), 1, 0),
        0
    );
    assert_eq!(
        noise_model_get_two_qubit_errors_len(model, cx.as_ptr(), 1, 1),
        0
    );
    assert_eq!(
        noise_model_get_two_qubit_errors_len(model, name_ptr("BOGUS").as_ptr(), 0, 1),
        0
    );
    assert_eq!(
        noise_model_get_two_qubit_errors_len(std::ptr::null(), cx.as_ptr(), 0, 1),
        0
    );

    // Readout queries.
    assert_eq!(noise_model_add_readout(model, 0, 0.02, 0.03), 0);
    let (mut p01, mut p10) = (0.0f64, 0.0f64);
    assert_eq!(
        noise_model_get_readout_error(model, 0, &mut p01, &mut p10),
        0
    );
    assert_eq!(p01, 0.02);
    assert_eq!(p10, 0.03);
    assert_eq!(
        noise_model_get_readout_error(model, 5, &mut p01, &mut p10),
        -8
    );
    assert_eq!(
        noise_model_get_readout_error(std::ptr::null(), 0, &mut p01, &mut p10),
        -1
    );

    noise_model_free(model);
}

// =====  Section 3.5: execution result lifecycle and metadata  =====

#[test]
fn test_execution_result_lifecycle() {
    let task = name_ptr("task-2");
    let qubits: [u32; 2] = [0, 1];
    let result = execution_result_new(task.as_ptr(), qubits.as_ptr(), 2, 100, std::ptr::null());
    assert!(!result.is_null());

    // Freshly created: queued, no backend, only the creation timestamp exists.
    let (mut tag, mut code) = (0u8, 0i32);
    assert_eq!(execution_result_status(result, &mut tag, &mut code), 0);
    assert_eq!(tag, EXECUTION_STATUS_QUEUED);
    assert_eq!(code, 0);
    let mut ms = 0i64;
    assert_eq!(execution_result_created_at(result, &mut ms), 0);
    assert!(ms > 1_600_000_000_000);
    assert_eq!(execution_result_started_at(result, &mut ms), -8);
    assert_eq!(execution_result_finished_at(result, &mut ms), -8);
    assert_eq!(execution_result_qubits_len(result), 2);
    let mut got = [0u32; 2];
    assert_eq!(execution_result_qubits(result, got.as_mut_ptr(), 2), 2);
    assert_eq!(got, [0, 1]);
    assert_eq!(execution_result_shots(result), 100);
    assert_eq!(execution_result_num_qubits(result), 2);
    let id = execution_result_task_id(result);
    assert_eq!(cstr_to_string(id), "task-2");
    cqlib_string_free(id);
    assert!(execution_result_backend(result).is_null());
    assert!(execution_result_error_message(result).is_null());

    // start -> running.
    assert_eq!(execution_result_start(result), 0);
    assert_eq!(execution_result_status(result, &mut tag, &mut code), 0);
    assert_eq!(tag, EXECUTION_STATUS_RUNNING);
    assert_eq!(execution_result_started_at(result, &mut ms), 0);
    assert!(ms > 1_600_000_000_000);

    // finish -> completed with accumulated counts.
    let bits = [name_ptr("00"), name_ptr("11"), name_ptr("00")];
    let bit_ptrs: [*const c_char; 3] = [bits[0].as_ptr(), bits[1].as_ptr(), bits[2].as_ptr()];
    let counts: [u64; 3] = [600, 400, 50];
    assert_eq!(
        execution_result_finish(result, bit_ptrs.as_ptr(), counts.as_ptr(), 3),
        0
    );
    assert_eq!(execution_result_status(result, &mut tag, &mut code), 0);
    assert_eq!(tag, EXECUTION_STATUS_COMPLETED);
    assert_eq!(execution_result_finished_at(result, &mut ms), 0);
    let list = execution_result_counts(result);
    assert_eq!(counts_list_len(list), 2);
    let mut total = 0.0;
    for i in 0..counts_list_len(list) {
        total += counts_list_get_value(list, i);
    }
    assert_eq!(total, 1050.0);
    counts_list_free(list);
    assert!(execution_result_error_message(result).is_null());

    // Invalid bitstrings are rejected.
    let bad_bits = [name_ptr("0x")];
    let bad_ptrs: [*const c_char; 1] = [bad_bits[0].as_ptr()];
    let one: [u64; 1] = [1];
    assert_eq!(
        execution_result_finish(result, bad_ptrs.as_ptr(), one.as_ptr(), 1),
        -4
    );

    // fail -> failed with error code and message.
    let msg = name_ptr("device offline");
    assert_eq!(execution_result_fail(result, msg.as_ptr(), 42), 0);
    assert_eq!(execution_result_status(result, &mut tag, &mut code), 0);
    assert_eq!(tag, EXECUTION_STATUS_FAILED);
    assert_eq!(code, 42);
    let err = execution_result_error_message(result);
    assert_eq!(cstr_to_string(err), "device offline");
    cqlib_string_free(err);
    assert_eq!(execution_result_fail(result, std::ptr::null(), 1), -1);

    // cancel -> cancelled.
    assert_eq!(execution_result_cancel(result), 0);
    assert_eq!(execution_result_status(result, &mut tag, &mut code), 0);
    assert_eq!(tag, EXECUTION_STATUS_CANCELLED);

    execution_result_free(result);

    // A backend string is echoed back.
    let backend = name_ptr("sim-ctx");
    let with_backend =
        execution_result_new(task.as_ptr(), qubits.as_ptr(), 2, 10, backend.as_ptr());
    assert!(!with_backend.is_null());
    let got_backend = execution_result_backend(with_backend);
    assert_eq!(cstr_to_string(got_backend), "sim-ctx");
    cqlib_string_free(got_backend);
    execution_result_free(with_backend);

    // NULL guards.
    assert!(
        execution_result_new(std::ptr::null(), qubits.as_ptr(), 2, 10, std::ptr::null()).is_null()
    );
    assert!(
        execution_result_new(task.as_ptr(), std::ptr::null(), 2, 10, std::ptr::null()).is_null()
    );
    assert_eq!(execution_result_start(std::ptr::null_mut()), -1);
    assert_eq!(
        execution_result_finish(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), 0),
        -1
    );
    assert_eq!(execution_result_cancel(std::ptr::null_mut()), -1);
    assert_eq!(
        execution_result_status(std::ptr::null(), &mut tag, &mut code),
        -1
    );
    assert!(execution_result_error_message(std::ptr::null()).is_null());
    assert!(execution_result_task_id(std::ptr::null()).is_null());
    assert!(execution_result_backend(std::ptr::null()).is_null());
    assert_eq!(execution_result_created_at(std::ptr::null(), &mut ms), -1);
    assert_eq!(execution_result_started_at(std::ptr::null(), &mut ms), -1);
    assert_eq!(execution_result_finished_at(std::ptr::null(), &mut ms), -1);
    assert_eq!(execution_result_qubits_len(std::ptr::null()), 0);
}
