//! Integration tests for the compiler workflow C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new, circuit_num_qubits};
use binding_c::compile::*;
use binding_c::device::layout_free;

fn logical_config(mode: u8) -> CompileConfigC {
    CompileConfigC {
        mode,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    }
}

fn build_bell_circuit() -> *mut binding_c::circuit::CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    circuit
}

#[test]
fn test_workflow_new_and_free() {
    let workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert!(
        !workflow.is_null(),
        "logical config should build a workflow"
    );
    compiler_workflow_free(workflow);

    // Enhanced mode is also accepted.
    let workflow = compiler_workflow_new(logical_config(COMPILE_MODE_ENHANCED));
    assert!(!workflow.is_null());
    compiler_workflow_free(workflow);

    // Invalid mode and unsupported target tags return NULL.
    assert!(compiler_workflow_new(logical_config(42)).is_null());

    let mut config = logical_config(COMPILE_MODE_NORMAL);
    config.target = COMPILE_TARGET_DEVICE;
    assert!(compiler_workflow_new(config).is_null());

    // Freeing NULL is allowed.
    compiler_workflow_free(std::ptr::null_mut());
}

#[test]
fn test_workflow_run_logical() {
    let circuit = build_bell_circuit();
    let workflow = compiler_workflow_new(logical_config(COMPILE_MODE_ENHANCED));
    assert!(!workflow.is_null());

    let result = compiler_workflow_run(workflow, circuit);
    assert!(!result.is_null(), "workflow run should succeed");
    assert_eq!(compile_result_mode(result), COMPILE_MODE_ENHANCED as i32);
    assert!(compile_result_num_steps(result) > 0);

    let optimized = compile_result_circuit(result);
    assert!(!optimized.is_null());
    assert_eq!(circuit_num_qubits(optimized), 2);
    circuit_free(optimized);

    // The workflow is reusable: a second run on the same circuit works.
    let second = compiler_workflow_run(workflow, circuit);
    assert!(!second.is_null());
    compile_result_free(second);

    compile_result_free(result);
    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

#[test]
fn test_workflow_run_null_arguments() {
    let circuit = build_bell_circuit();
    let workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert!(!workflow.is_null());

    assert!(compiler_workflow_run(std::ptr::null(), circuit).is_null());
    assert!(compiler_workflow_run(workflow, std::ptr::null()).is_null());

    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

#[test]
fn test_device_metadata_accessors_logical_compile_is_empty() {
    let circuit = build_bell_circuit();
    let workflow = compiler_workflow_new(logical_config(COMPILE_MODE_NORMAL));
    assert!(!workflow.is_null());
    let result = compiler_workflow_run(workflow, circuit);
    assert!(!result.is_null());

    // A logical-target compile carries no device metadata: numeric
    // accessors return 0, handle accessors return NULL, and query
    // functions report absence.
    assert_eq!(compile_result_has_device_metadata(result), 0);
    assert!(compile_result_device_metadata_initial_layout(result).is_null());
    assert!(compile_result_device_metadata_final_layout(result).is_null());
    assert_eq!(compile_result_device_metadata_permutation_len(result), 0);
    assert_eq!(
        compile_result_device_metadata_permutation(
            result,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ),
        0
    );
    // A positive length with no metadata is a mismatch.
    let mut original: u32 = 0;
    let mut rewritten: u32 = 0;
    assert_eq!(
        compile_result_device_metadata_permutation(result, &mut original, &mut rewritten, 1),
        -8
    );

    compile_result_free(result);
    compiler_workflow_free(workflow);
    circuit_free(circuit);
}

#[test]
fn test_device_metadata_accessors_null_result() {
    assert_eq!(compile_result_has_device_metadata(std::ptr::null()), -1);
    assert!(compile_result_device_metadata_initial_layout(std::ptr::null()).is_null());
    assert!(compile_result_device_metadata_final_layout(std::ptr::null()).is_null());
    assert_eq!(
        compile_result_device_metadata_permutation_len(std::ptr::null()),
        0
    );
    assert_eq!(
        compile_result_device_metadata_permutation(
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ),
        -1
    );
}

#[test]
fn test_device_metadata_layouts_present_after_device_compile() {
    use binding_c::device::{
        device_bidirectional_line, device_free, device_with_native_gates, layout_num_logical,
    };

    let circuit = build_bell_circuit();
    let device = device_bidirectional_line(c"line-2".as_ptr(), 2);
    assert!(!device.is_null());
    // Device-target compilation requires declared native capabilities.
    assert_eq!(device_with_native_gates(device, c"H,CX".as_ptr()), 0);

    let result = compile_with_device(circuit, COMPILE_MODE_NORMAL, device, std::ptr::null(), 7);
    assert!(!result.is_null());

    // A device-target compile records metadata for every accessor.
    assert_eq!(compile_result_has_device_metadata(result), 1);

    let initial = compile_result_device_metadata_initial_layout(result);
    assert!(!initial.is_null());
    assert_eq!(layout_num_logical(initial), 2);

    let final_layout = compile_result_device_metadata_final_layout(result);
    assert!(!final_layout.is_null());

    // The permutation is reported with the two-step array protocol.
    let len = compile_result_device_metadata_permutation_len(result);
    assert_eq!(len, 2);
    let mut original = [0u32; 2];
    let mut rewritten = [0u32; 2];
    assert_eq!(
        compile_result_device_metadata_permutation(
            result,
            original.as_mut_ptr(),
            rewritten.as_mut_ptr(),
            len
        ),
        0
    );
    for i in 0..len {
        assert!(
            original[i] < 2 && rewritten[i] < 2,
            "permutation ids must be in range"
        );
    }

    // Length mismatch is rejected.
    assert_eq!(
        compile_result_device_metadata_permutation(
            result,
            original.as_mut_ptr(),
            rewritten.as_mut_ptr(),
            len + 1
        ),
        -8
    );
    assert_eq!(
        compile_result_device_metadata_permutation(
            result,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            len
        ),
        -1
    );

    layout_free(initial);
    layout_free(final_layout);
    compile_result_free(result);
    device_free(device);
    circuit_free(circuit);
}
