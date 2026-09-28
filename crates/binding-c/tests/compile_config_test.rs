//! Integration tests for the compile-module configuration C ABI
//! (`CTransformConfig` in `compile::transform` and `CDecomposeUnitaryTarget`
//! in `compile::decompose`).

#![allow(dead_code)]

use binding_c::compile::*;
use binding_c::cqlib_string_free;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Builds an owned C string array from Rust string literals. The returned
/// `Vec<CString>` must outlive every use of the pointer array.
fn name_array(names: &[&str]) -> (Vec<*const c_char>, Vec<CString>) {
    let owned: Vec<CString> = names.iter().map(|n| CString::new(*n).unwrap()).collect();
    let ptrs: Vec<*const c_char> = owned.iter().map(|n| n.as_ptr()).collect();
    (ptrs, owned)
}

fn ptr_to_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null(), "expected a heap-allocated name");
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    cqlib_string_free(ptr);
    s
}

/// Reads a two-step name list and frees every entry.
fn read_name_list(total: usize, read: impl Fn(*mut *mut c_char, usize) -> usize) -> Vec<String> {
    assert_eq!(
        total,
        read(std::ptr::null_mut(), 0),
        "NULL-out query mismatch"
    );
    let mut buffer: Vec<*mut c_char> = vec![std::ptr::null_mut(); total];
    let written = read(buffer.as_mut_ptr(), total);
    assert_eq!(written, total, "two-step length mismatch");
    buffer.into_iter().map(ptr_to_string).collect()
}

// =====  CTransformConfig  =====

/// Reads the enabled-kind tags of a config via the two-step pattern.
fn read_enabled_kinds(config: *const CTransformConfig, expected: usize) -> Vec<u8> {
    assert_eq!(transform_config_enabled_kinds_len(config), expected);
    let mut buffer = vec![0u8; expected];
    let written = transform_config_enabled_kinds(config, buffer.as_mut_ptr(), expected);
    assert_eq!(written, expected);
    buffer
}

#[test]
fn test_transform_config_default_getters() {
    let config = transform_config_default();
    assert!(!config.is_null());

    assert_eq!(transform_config_mode(config), REWRITE_MODE_OPTIMIZE as i32);
    assert_eq!(transform_config_max_rounds(config), 8);
    assert_eq!(transform_config_max_window_ops(config), 16);
    assert_eq!(transform_config_max_pattern_len(config), 8);
    assert_eq!(transform_config_recurse_control_flow(config), 1);
    assert_eq!(transform_config_skip_labeled_ops(config), 1);
    assert_eq!(transform_config_enabled_kinds_len(config), 4);
    assert_eq!(transform_config_target_instruction_basis_len(config), 0);
    assert_eq!(
        transform_config_target_instruction_basis(config, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(
        read_enabled_kinds(config, 4),
        vec![
            REWRITE_KIND_SIMPLIFY,
            REWRITE_KIND_CANCEL,
            REWRITE_KIND_MERGE,
            REWRITE_KIND_CANONICALIZE,
        ]
    );

    transform_config_free(config);
}

#[test]
fn test_transform_config_lowering_getters() {
    let config = transform_config_lowering();
    assert!(!config.is_null());

    assert_eq!(transform_config_mode(config), REWRITE_MODE_LOWERING as i32);
    assert_eq!(transform_config_enabled_kinds_len(config), 6);
    let kinds = read_enabled_kinds(config, 6);
    assert!(kinds.contains(&REWRITE_KIND_DECOMPOSE));
    assert!(kinds.contains(&REWRITE_KIND_HARDWARE_NATIVE));

    transform_config_free(config);
}

#[test]
fn test_transform_config_with_mode() {
    let config = transform_config_default();
    assert_eq!(transform_config_with_mode(config, REWRITE_MODE_LOWERING), 0);
    assert_eq!(transform_config_mode(config), REWRITE_MODE_LOWERING as i32);
    assert_eq!(transform_config_with_mode(config, REWRITE_MODE_OPTIMIZE), 0);
    assert_eq!(transform_config_mode(config), REWRITE_MODE_OPTIMIZE as i32);
    // An unknown mode tag is rejected and leaves the mode unchanged.
    assert_eq!(transform_config_with_mode(config, 99), -8);
    assert_eq!(transform_config_mode(config), REWRITE_MODE_OPTIMIZE as i32);
    transform_config_free(config);

    assert_eq!(transform_config_with_mode(std::ptr::null_mut(), 0), -1);
    assert_eq!(transform_config_mode(std::ptr::null()), -1);
}

#[test]
fn test_transform_config_scalar_setters() {
    let config = transform_config_default();

    assert_eq!(transform_config_with_max_rounds(config, 3), 0);
    assert_eq!(transform_config_max_rounds(config), 3);

    assert_eq!(transform_config_with_max_window_ops(config, 32), 0);
    assert_eq!(transform_config_max_window_ops(config), 32);

    assert_eq!(transform_config_with_max_pattern_len(config, 5), 0);
    assert_eq!(transform_config_max_pattern_len(config), 5);

    transform_config_free(config);

    assert_eq!(
        transform_config_with_max_rounds(std::ptr::null_mut(), 1),
        -1
    );
    assert_eq!(
        transform_config_with_max_window_ops(std::ptr::null_mut(), 1),
        -1
    );
    assert_eq!(
        transform_config_with_max_pattern_len(std::ptr::null_mut(), 1),
        -1
    );
    assert_eq!(transform_config_max_rounds(std::ptr::null()), 0);
    assert_eq!(transform_config_max_window_ops(std::ptr::null()), 0);
    assert_eq!(transform_config_max_pattern_len(std::ptr::null()), 0);
    assert_eq!(transform_config_recurse_control_flow(std::ptr::null()), -1);
    assert_eq!(transform_config_skip_labeled_ops(std::ptr::null()), -1);
}

#[test]
fn test_transform_config_with_enabled_kinds() {
    let config = transform_config_default();

    let tags = [REWRITE_KIND_CANCEL, REWRITE_KIND_MERGE];
    assert_eq!(
        transform_config_with_enabled_kinds(config, tags.as_ptr(), tags.len()),
        0
    );
    assert_eq!(read_enabled_kinds(config, 2), tags.to_vec());

    // Unknown tags are rejected and leave the configuration unchanged.
    let bad = [REWRITE_KIND_CANCEL, 42];
    assert_eq!(
        transform_config_with_enabled_kinds(config, bad.as_ptr(), bad.len()),
        -8
    );
    assert_eq!(read_enabled_kinds(config, 2), tags.to_vec());

    // An empty list disables every category.
    assert_eq!(
        transform_config_with_enabled_kinds(config, std::ptr::null(), 0),
        0
    );
    assert_eq!(transform_config_enabled_kinds_len(config), 0);

    // A NULL array with a positive length is rejected.
    assert_eq!(
        transform_config_with_enabled_kinds(config, std::ptr::null(), 1),
        -1
    );

    transform_config_free(config);
    assert_eq!(
        transform_config_with_enabled_kinds(std::ptr::null_mut(), std::ptr::null(), 0),
        -1
    );
}

#[test]
fn test_transform_config_enabled_kinds_two_step_consistency() {
    let config = transform_config_lowering();
    let total = transform_config_enabled_kinds_len(config);
    assert_eq!(total, 6);

    // A truncated buffer only receives the first entries; the returned
    // total is unaffected.
    let mut small = [0u8; 2];
    assert_eq!(
        transform_config_enabled_kinds(config, small.as_mut_ptr(), small.len()),
        total
    );
    assert_eq!(small[0], REWRITE_KIND_SIMPLIFY);
    assert_eq!(small[1], REWRITE_KIND_CANCEL);

    // A NULL out pointer only queries the total.
    assert_eq!(
        transform_config_enabled_kinds(config, std::ptr::null_mut(), total),
        total
    );
    assert_eq!(
        transform_config_enabled_kinds(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );

    transform_config_free(config);
}

#[test]
fn test_transform_config_with_target_instructions() {
    let config = transform_config_default();

    let (ptrs, _owned) = name_array(&["H", "RZ", "CX"]);
    assert_eq!(
        transform_config_with_target_instructions(config, ptrs.as_ptr(), 3),
        0
    );
    assert_eq!(transform_config_target_instruction_basis_len(config), 3);
    let names = read_name_list(3, |out, len| {
        transform_config_target_instruction_basis(config, out, len)
    });
    assert_eq!(names, vec!["H", "RZ", "CX"]);

    // Unknown gate names map to -4 and leave the basis unchanged.
    let (bad_ptrs, _bad_owned) = name_array(&["H", "NOPE"]);
    assert_eq!(
        transform_config_with_target_instructions(config, bad_ptrs.as_ptr(), 2),
        -4
    );
    assert_eq!(transform_config_target_instruction_basis_len(config), 3);

    // An empty basis is rejected (-8).
    assert_eq!(
        transform_config_with_target_instructions(config, std::ptr::null(), 0),
        -8
    );

    // A NULL name entry is rejected (-1).
    let with_null = [std::ptr::null()];
    assert_eq!(
        transform_config_with_target_instructions(config, with_null.as_ptr(), 1),
        -1
    );

    transform_config_free(config);
    assert_eq!(
        transform_config_with_target_instructions(std::ptr::null_mut(), std::ptr::null(), 0),
        -1
    );
}

#[test]
fn test_transform_config_with_target_instructions_invalid_utf8() {
    let config = transform_config_default();
    let invalid = unsafe { CString::from_vec_unchecked(vec![0xFF, 0xFE]) };
    let ptrs = [invalid.as_ptr()];
    assert_eq!(
        transform_config_with_target_instructions(config, ptrs.as_ptr(), 1),
        -4
    );
    transform_config_free(config);
}

#[test]
fn test_transform_config_target_basis_two_step_consistency() {
    let config = transform_config_default();
    assert_eq!(transform_config_target_instruction_basis_len(config), 0);
    assert_eq!(
        transform_config_target_instruction_basis(config, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(
        transform_config_target_instruction_basis(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );

    let (ptrs, _owned) = name_array(&["H", "CX"]);
    assert_eq!(
        transform_config_with_target_instructions(config, ptrs.as_ptr(), 2),
        0
    );
    assert_eq!(transform_config_target_instruction_basis_len(config), 2);

    // A truncated buffer only receives the first entry; the total is
    // unaffected.
    let mut small: Vec<*mut c_char> = vec![std::ptr::null_mut(); 1];
    assert_eq!(
        transform_config_target_instruction_basis(config, small.as_mut_ptr(), small.len()),
        2
    );
    assert_eq!(ptr_to_string(small[0]), "H");

    transform_config_free(config);
}

#[test]
fn test_transform_config_free_null() {
    transform_config_free(std::ptr::null_mut());
}

// =====  CDecomposeUnitaryTarget  =====

#[test]
fn test_decompose_unitary_target_unconstrained() {
    let target = decompose_unitary_target_unconstrained();
    assert!(!target.is_null());

    assert_eq!(decompose_unitary_target_native_1q_len(target), 0);
    assert_eq!(decompose_unitary_target_native_2q_len(target), 0);
    assert_eq!(decompose_unitary_target_basis_len(target), 0);
    assert_eq!(decompose_unitary_target_fallback_pauli(target), 1);
    assert_eq!(
        decompose_unitary_target_basis(target, std::ptr::null_mut(), 0),
        0
    );

    // Rebuilding the fallback flag needs a non-empty native basis.
    assert_eq!(decompose_unitary_target_set_fallback_pauli(target, 0), -8);
    assert_eq!(decompose_unitary_target_fallback_pauli(target), 1);

    // An unconstrained target has no basis to cost against.
    let mut cost = CTargetBasisCost {
        two_qubit_ops: 1,
        depth: 1,
        total_ops: 1,
        parameterized_ops: 1,
    };
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut cost
        ),
        -8
    );

    decompose_unitary_target_free(target);
    decompose_unitary_target_free(std::ptr::null_mut());
    assert_eq!(
        decompose_unitary_target_fallback_pauli(std::ptr::null()),
        -1
    );
    assert_eq!(decompose_unitary_target_native_1q_len(std::ptr::null()), 0);
    assert_eq!(decompose_unitary_target_native_2q_len(std::ptr::null()), 0);
    assert_eq!(decompose_unitary_target_basis_len(std::ptr::null()), 0);
}

#[test]
fn test_decompose_unitary_target_from_standard_gates() {
    let (oneq, _oneq_owned) = name_array(&["H", "RZ"]);
    let (twoq, _twoq_owned) = name_array(&["CX", "RZZ"]);
    let target =
        decompose_unitary_target_from_standard_gates(oneq.as_ptr(), 2, twoq.as_ptr(), 2, 1);
    assert!(!target.is_null());

    assert_eq!(decompose_unitary_target_fallback_pauli(target), 1);
    assert_eq!(decompose_unitary_target_native_1q_len(target), 2);
    assert_eq!(decompose_unitary_target_native_2q_len(target), 2);
    assert_eq!(decompose_unitary_target_basis_len(target), 4);

    let mut oneq_names = read_name_list(2, |out, len| {
        decompose_unitary_target_native_1q(target, out, len)
    });
    oneq_names.sort();
    assert_eq!(oneq_names, vec!["H", "RZ"]);

    let mut twoq_names = read_name_list(2, |out, len| {
        decompose_unitary_target_native_2q(target, out, len)
    });
    twoq_names.sort();
    assert_eq!(twoq_names, vec!["CX", "RZZ"]);

    // The combined basis contains every native gate.
    let basis = read_name_list(4, |out, len| {
        decompose_unitary_target_basis(target, out, len)
    });
    assert!(basis.contains(&"H".to_string()));
    assert!(basis.contains(&"RZ".to_string()));
    assert!(basis.contains(&"CX".to_string()));
    assert!(basis.contains(&"RZZ".to_string()));

    // Toggling the fallback flag rebuilds the target in place.
    assert_eq!(decompose_unitary_target_set_fallback_pauli(target, 0), 0);
    assert_eq!(decompose_unitary_target_fallback_pauli(target), 0);
    assert_eq!(decompose_unitary_target_set_fallback_pauli(target, 1), 0);
    assert_eq!(decompose_unitary_target_fallback_pauli(target), 1);
    assert_eq!(
        decompose_unitary_target_set_fallback_pauli(std::ptr::null_mut(), 1),
        -1
    );

    // Wrong arity is rejected by the core.
    assert!(
        decompose_unitary_target_from_standard_gates(twoq.as_ptr(), 1, twoq.as_ptr(), 1, 1)
            .is_null()
    );
    // Unknown gate names are rejected.
    let (bad, _bad_owned) = name_array(&["NOT_A_GATE"]);
    assert!(
        decompose_unitary_target_from_standard_gates(bad.as_ptr(), 1, twoq.as_ptr(), 1, 1)
            .is_null()
    );

    decompose_unitary_target_free(target);
}

#[test]
fn test_decompose_unitary_target_from_instructions() {
    let (ptrs, _owned) = name_array(&["H", "CX"]);
    let target = decompose_unitary_target_from_instructions(ptrs.as_ptr(), 2);
    assert!(!target.is_null());

    assert_eq!(decompose_unitary_target_native_1q_len(target), 1);
    assert_eq!(decompose_unitary_target_native_2q_len(target), 1);
    assert_eq!(decompose_unitary_target_fallback_pauli(target), 1);

    decompose_unitary_target_free(target);

    // An empty basis cannot build a cost model.
    assert!(decompose_unitary_target_from_instructions(std::ptr::null(), 0).is_null());
    // Unknown gate names are rejected.
    let (bad, _bad_owned) = name_array(&["NOPE"]);
    assert!(decompose_unitary_target_from_instructions(bad.as_ptr(), 1).is_null());
}

#[test]
fn test_decompose_unitary_target_basis_two_step_consistency() {
    let (oneq, _oneq_owned) = name_array(&["H"]);
    let (twoq, _twoq_owned) = name_array(&["CX"]);
    let target =
        decompose_unitary_target_from_standard_gates(oneq.as_ptr(), 1, twoq.as_ptr(), 1, 1);
    assert!(!target.is_null());

    let total = decompose_unitary_target_basis_len(target);
    assert_eq!(total, 2);

    // A truncated buffer only receives the first entry; the total is
    // unaffected.
    let mut small: Vec<*mut c_char> = vec![std::ptr::null_mut(); 1];
    assert_eq!(
        decompose_unitary_target_basis(target, small.as_mut_ptr(), small.len()),
        total
    );
    assert_eq!(ptr_to_string(small[0]), "H");

    // A NULL out pointer only queries the total.
    assert_eq!(
        decompose_unitary_target_basis(target, std::ptr::null_mut(), total),
        total
    );

    decompose_unitary_target_free(target);
}

#[test]
fn test_decompose_unitary_target_cost_of_fixed_operations() {
    let (oneq, _oneq_owned) = name_array(&["H", "RZ"]);
    let (twoq, _twoq_owned) = name_array(&["CX"]);
    let target =
        decompose_unitary_target_from_standard_gates(oneq.as_ptr(), 2, twoq.as_ptr(), 1, 1);
    assert!(!target.is_null());

    // Ops: H(q0); CX(q0, q1); H(q1) -> already native, so the lowered cost
    // is 3 ops, depth 3, one two-qubit op, no parameters.
    let (ops, _ops_owned) = name_array(&["H", "CX", "H"]);
    let qubit_ids = [0u32, 0, 1, 1];
    let qubit_counts = [1u32, 2, 1];
    let mut cost = CTargetBasisCost {
        two_qubit_ops: 0,
        depth: 0,
        total_ops: 0,
        parameterized_ops: 0,
    };
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            ops.as_ptr(),
            qubit_ids.as_ptr(),
            qubit_counts.as_ptr(),
            3,
            &mut cost
        ),
        0
    );
    assert_eq!(cost.total_ops, 3);
    assert_eq!(cost.two_qubit_ops, 1);
    assert_eq!(cost.parameterized_ops, 0);
    assert_eq!(cost.depth, 3);

    // Parameterized gates are counted as such.
    let (rz_ops, _rz_owned) = name_array(&["RZ"]);
    let rz_counts = [1u32];
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            rz_ops.as_ptr(),
            qubit_ids.as_ptr(),
            rz_counts.as_ptr(),
            1,
            &mut cost
        ),
        0
    );
    assert_eq!(cost.total_ops, 1);
    assert_eq!(cost.parameterized_ops, 1);

    // An empty sequence costs nothing.
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut cost
        ),
        0
    );
    assert_eq!(cost.total_ops, 0);

    // Error paths: NULL out, unknown gate name, NULL config.
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            ops.as_ptr(),
            qubit_ids.as_ptr(),
            qubit_counts.as_ptr(),
            3,
            std::ptr::null_mut()
        ),
        -1
    );
    let (bad_ops, _bad_owned) = name_array(&["NOT_A_GATE"]);
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            target,
            bad_ops.as_ptr(),
            qubit_ids.as_ptr(),
            qubit_counts.as_ptr(),
            1,
            &mut cost
        ),
        -4
    );
    assert_eq!(
        decompose_unitary_target_cost_of_fixed_operations(
            std::ptr::null(),
            ops.as_ptr(),
            qubit_ids.as_ptr(),
            qubit_counts.as_ptr(),
            3,
            &mut cost
        ),
        -1
    );

    decompose_unitary_target_free(target);
}

// =====  CCanonicalizeConfig  =====

#[test]
fn test_canonicalize_config_defaults_and_setters() {
    let config = canonicalize_config_default();
    assert!(!config.is_null());
    assert_eq!(canonicalize_config_round_limit(config), 8);
    assert_eq!(canonicalize_config_recurse_control_flow(config), 1);
    assert_eq!(canonicalize_config_fold_gphase(config), 1);
    assert_eq!(canonicalize_config_canonicalize_instruction_form(config), 1);
    assert_eq!(canonicalize_config_drop_noops(config), 1);
    assert_eq!(canonicalize_config_canonicalize_barriers(config), 1);

    // Setters chain on the current config instead of resetting to defaults.
    assert_eq!(canonicalize_config_with_round_limit(config, 2), 0);
    assert_eq!(canonicalize_config_with_drop_noops(config, 0), 0);
    assert_eq!(canonicalize_config_with_recurse_control_flow(config, 0), 0);
    assert_eq!(canonicalize_config_with_fold_gphase(config, 0), 0);
    assert_eq!(
        canonicalize_config_with_canonicalize_instruction_form(config, 0),
        0
    );
    assert_eq!(canonicalize_config_with_canonicalize_barriers(config, 0), 0);
    assert_eq!(canonicalize_config_round_limit(config), 2);
    assert_eq!(canonicalize_config_recurse_control_flow(config), 0);
    assert_eq!(canonicalize_config_fold_gphase(config), 0);
    assert_eq!(canonicalize_config_canonicalize_instruction_form(config), 0);
    assert_eq!(canonicalize_config_drop_noops(config), 0);
    assert_eq!(canonicalize_config_canonicalize_barriers(config), 0);

    assert_eq!(
        canonicalize_config_with_round_limit(std::ptr::null_mut(), 1),
        -1
    );
    assert_eq!(canonicalize_config_round_limit(std::ptr::null_mut()), 0);
    assert_eq!(canonicalize_config_drop_noops(std::ptr::null_mut()), -1);

    canonicalize_config_free(config);
    canonicalize_config_free(std::ptr::null_mut());
}

#[test]
fn test_canonicalize_circuit_with_config() {
    use binding_c::circuit::{circuit_free, circuit_h, circuit_new};

    let qc = circuit_new(2);
    assert_eq!(circuit_h(qc, 0), 0);

    let config = canonicalize_config_default();
    let result = canonicalize_circuit_with_config(qc, config);
    assert!(!result.is_null());
    assert!(canonicalize_result_rounds(result) >= 1);
    canonicalize_result_free(result);

    // NULL inputs return NULL; the config stays owned by the caller.
    assert!(canonicalize_circuit_with_config(std::ptr::null(), config).is_null());

    canonicalize_config_free(config);
    circuit_free(qc);
}
