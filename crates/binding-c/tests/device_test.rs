//! Integration tests for device module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new};
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
