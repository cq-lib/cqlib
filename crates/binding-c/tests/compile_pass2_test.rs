//! Integration tests for the compile pass C ABI: decomposition/resynthesis,
//! knowledge rules, ancillary resources, and commutation checks.

#![allow(dead_code)]

use binding_c::circuit::{
    circuit_circuit_gate, circuit_cx, circuit_free, circuit_gate_free, circuit_h,
    circuit_multi_control, circuit_new, circuit_num_operations, circuit_num_qubits, circuit_t,
    circuit_to_gate, circuit_unitary,
};
use binding_c::compile::*;
use binding_c::cqlib_string_free;
use binding_c::device::{device_free, device_from_edges, device_with_native_gates};
use num_complex::Complex64;
use std::ffi::{CStr, CString};

fn name(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn zeroed<T>() -> T {
    unsafe { std::mem::zeroed() }
}

// =====  Decomposition and resynthesis  =====

#[test]
fn test_decompose_expand_definitions() {
    let inner = circuit_new(1);
    assert_eq!(circuit_h(inner, 0), 0);
    let gate_name = name("my_h");
    let gate = circuit_to_gate(inner, gate_name.as_ptr());
    assert!(!gate.is_null());

    let outer = circuit_new(2);
    let qubits = [0u32];
    assert_eq!(
        circuit_circuit_gate(outer, gate, qubits.as_ptr(), 1, std::ptr::null(), 0),
        0
    );
    assert_eq!(circuit_num_operations(outer), 1);

    // The circuit-gate definition is expanded back into plain operations.
    let expanded = decompose_expand_definitions(outer);
    assert!(!expanded.is_null());
    assert_eq!(circuit_num_operations(expanded), 1);
    circuit_free(expanded);

    circuit_gate_free(gate);
    circuit_free(outer);
    circuit_free(inner);
}

#[test]
fn test_decompose_unitaries() {
    let h_matrix = [
        Complex64::new(1.0 / 2.0f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0f64.sqrt(), 0.0),
        Complex64::new(-1.0 / 2.0f64.sqrt(), 0.0),
    ];
    let label = name("my_h");
    let targets = [0u32];
    let circuit = circuit_new(2);
    assert_eq!(
        circuit_unitary(
            circuit,
            label.as_ptr(),
            1,
            h_matrix.as_ptr(),
            targets.as_ptr(),
            1
        ),
        0
    );

    // The matrix-backed unitary is synthesized into standard operations.
    let out = decompose_unitaries(circuit, decompose_unitary_config_default());
    assert!(!out.is_null());
    assert!(circuit_num_operations(out) >= 1);
    circuit_free(out);

    // The diagnostic form additionally reports rule-cache statistics.
    let mut stats: CDecompositionRuleStats = zeroed();
    let out = decompose_unitaries_with_rule_stats(
        circuit,
        decompose_unitary_config_default(),
        &mut stats,
    );
    assert!(!out.is_null());
    circuit_free(out);

    circuit_free(circuit);
}

#[test]
fn test_decompose_mc_gates() {
    let gate = name("X");
    let controls = [0u32, 1];
    let targets = [2u32];
    let circuit = circuit_new(3);
    assert_eq!(
        circuit_multi_control(
            circuit,
            gate.as_ptr(),
            controls.as_ptr(),
            2,
            targets.as_ptr(),
            1,
            std::ptr::null(),
            0
        ),
        0
    );

    let out = decompose_mc_gates(circuit, mc_gate_config_default());
    assert!(!out.is_null());
    assert!(circuit_num_operations(out) >= 1);
    circuit_free(out);

    let mut stats: CDecompositionRuleStats = zeroed();
    let out = decompose_mc_gates_with_rule_stats(circuit, mc_gate_config_default(), &mut stats);
    assert!(!out.is_null());
    circuit_free(out);

    // Device-bounded form caps the decomposition at the usable width.
    let dev_name = name("line-3");
    let edges = [0u32, 1, 1, 2];
    let device = device_from_edges(dev_name.as_ptr(), 3, edges.as_ptr(), 2);
    assert!(!device.is_null());
    let out = decompose_mc_gates_for_device(circuit, device, std::ptr::null());
    assert!(!out.is_null());
    circuit_free(out);
    device_free(device);

    circuit_free(circuit);
}

#[test]
fn test_resynthesize_two_qubit_blocks() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let out = resynthesize_two_qubit_blocks(circuit, resynthesis_config_normal());
    assert!(!out.is_null());
    assert!(circuit_num_operations(out) >= 1);
    circuit_free(out);

    let out = resynthesize_two_qubit_blocks(circuit, resynthesis_config_enhanced());
    assert!(!out.is_null());
    circuit_free(out);

    circuit_free(circuit);
}

#[test]
fn test_numeric_unitary_synthesis() {
    // 1q synthesis of the identity yields U(0, 0, 0) with zero global phase.
    let identity2 = [
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
    ];
    let mut one_q: COneQubitUnitaryDecomposition = zeroed();
    assert_eq!(
        synthesize_numeric_1q_unitary(identity2.as_ptr(), &mut one_q),
        0
    );
    assert!(one_q.theta.abs() < 1e-10);
    assert!(one_q.phi.abs() < 1e-10);
    assert!(one_q.lambda.abs() < 1e-10);
    assert!(one_q.global_phase.abs() < 1e-10);

    // 2q synthesis of the identity succeeds and exposes its operations.
    let mut identity4 = [Complex64::new(0.0, 0.0); 16];
    for (i, slot) in identity4.iter_mut().enumerate() {
        if i % 5 == 0 {
            *slot = Complex64::new(1.0, 0.0);
        }
    }
    let synthesis = synthesize_numeric_2q_unitary(identity4.as_ptr(), 0, 1, TWO_QUBIT_BASIS_CX);
    assert!(!synthesis.is_null());
    assert!(two_qubit_synthesis_global_phase(synthesis).is_finite());
    for index in 0..two_qubit_synthesis_num_operations(synthesis) {
        let op_name = two_qubit_synthesis_operation_name(synthesis, index);
        assert!(!op_name.is_null());
        cqlib_string_free(op_name);
        let qubits_len = two_qubit_synthesis_operation_qubits_len(synthesis, index);
        assert!(qubits_len >= 1);
        let mut qubits = vec![0u32; qubits_len];
        assert_eq!(
            two_qubit_synthesis_operation_qubits(synthesis, index, qubits.as_mut_ptr(), qubits_len),
            qubits_len
        );
    }
    two_qubit_synthesis_free(synthesis);

    // KAK decomposition of the identity has zero interaction coordinates.
    let kak = kak_decompose(identity4.as_ptr());
    assert!(!kak.is_null());
    let (mut a, mut b, mut c) = (0.0f64, 0.0f64, 0.0f64);
    assert_eq!(
        kak_decomposition_coordinates(kak, &mut a, &mut b, &mut c),
        0
    );
    assert!(a.abs() < 1e-6);
    assert!(b.abs() < 1e-6);
    assert!(c.abs() < 1e-6);
    assert!(kak_decomposition_global_phase(kak).abs() < 1e-6);
    let mut factor = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        kak_decomposition_local_factor(kak, KAK_FACTOR_K1L, factor.as_mut_ptr()),
        0
    );
    // Unknown factor tags are rejected.
    assert_eq!(
        kak_decomposition_local_factor(kak, 99, factor.as_mut_ptr()),
        -8
    );
    kak_decomposition_free(kak);
}

#[test]
fn test_target_basis_lowering() {
    let names = [name("H"), name("RZ"), name("CX")];
    let name_ptrs: Vec<*const std::os::raw::c_char> = names.iter().map(|s| s.as_ptr()).collect();
    let lowerer = target_basis_lowerer_new(name_ptrs.as_ptr(), 3);
    assert!(!lowerer.is_null());
    assert_eq!(target_basis_lowerer_num_gates(lowerer), 3);

    // T is outside {H, RZ, CX} and requires lowering.
    let circuit = circuit_new(1);
    assert_eq!(circuit_t(circuit, 0), 0);
    assert_eq!(target_basis_lowerer_requires_lowering(lowerer, circuit), 1);
    let lowered = target_basis_lowerer_apply(lowerer, circuit);
    assert!(!lowered.is_null());
    assert!(circuit_num_operations(lowered) >= 1);
    circuit_free(lowered);

    // A circuit already in the basis needs no lowering.
    let basis_circuit = circuit_new(2);
    assert_eq!(circuit_h(basis_circuit, 0), 0);
    assert_eq!(circuit_cx(basis_circuit, 0, 1), 0);
    assert_eq!(
        target_basis_lowerer_requires_lowering(lowerer, basis_circuit),
        0
    );
    circuit_free(basis_circuit);

    circuit_free(circuit);
    target_basis_lowerer_free(lowerer);

    // Unknown gate names and empty bases are rejected.
    let bad = [name("NOPE")];
    let bad_ptrs = [bad[0].as_ptr()];
    assert!(target_basis_lowerer_new(bad_ptrs.as_ptr(), 1).is_null());
    assert!(target_basis_lowerer_new(std::ptr::null(), 0).is_null());
}

#[test]
fn test_decompose_lower_to_device() {
    let dev_name = name("line-2");
    let edges = [0u32, 1];
    let device = device_from_edges(dev_name.as_ptr(), 2, edges.as_ptr(), 1);
    assert!(!device.is_null());
    let native = name("H,RZ,CX");
    assert_eq!(device_with_native_gates(device, native.as_ptr()), 0);

    // A circuit already native to the device passes through unchanged.
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    let lowered = decompose_lower_to_device(circuit, device);
    assert!(!lowered.is_null());
    assert_eq!(circuit_num_operations(lowered), 2);
    circuit_free(lowered);

    circuit_free(circuit);
    device_free(device);
}

// =====  Ancillary resources  =====

#[test]
fn test_resource_manager_lifecycle() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);

    let mut policy = resource_policy_default();
    policy.max_pre_layout_clean_ancillas = 2;

    let manager = resource_manager_from_circuit(circuit, &policy, std::ptr::null());
    assert!(!manager.is_null());
    assert_eq!(resource_manager_verify_consistency(manager), 0);
    assert_eq!(resource_manager_verify_idle(manager), 0);

    // Invalid requirement tags and oversized requests are rejected.
    assert!(resource_manager_preview(manager, 99, 1, std::ptr::null(), 0).is_null());
    assert!(
        resource_manager_preview(
            manager,
            RESOURCE_REQUIREMENT_CLEAN_ZERO,
            5,
            std::ptr::null(),
            0
        )
        .is_null()
    );

    // Preview plans one fresh clean ancilla beyond the input width.
    let plan = resource_manager_preview(
        manager,
        RESOURCE_REQUIREMENT_CLEAN_ZERO,
        1,
        std::ptr::null(),
        0,
    );
    assert!(!plan.is_null());
    assert_eq!(resource_plan_qubits_len(plan), 1);
    assert_eq!(resource_plan_num_new_qubits(plan), 1);
    assert_eq!(
        resource_plan_requirement(plan),
        RESOURCE_REQUIREMENT_CLEAN_ZERO as i32
    );
    let mut planned = [7u32];
    assert_eq!(resource_plan_qubits(plan, planned.as_mut_ptr(), 1), 1);
    assert_eq!(planned[0], 2);

    // Commit mutates the manager's working circuit and reserves the qubit.
    let lease = resource_manager_commit(manager, plan);
    assert!(!lease.is_null());
    assert_eq!(resource_lease_qubits_len(lease), 1);
    assert_eq!(
        resource_lease_requirement(lease),
        RESOURCE_REQUIREMENT_CLEAN_ZERO as i32
    );
    let working = resource_manager_circuit(manager);
    assert!(!working.is_null());
    assert_eq!(circuit_num_qubits(working), 3);
    circuit_free(working);

    // While the lease is active the manager is consistent but not idle.
    assert_eq!(resource_manager_verify_consistency(manager), 0);
    assert_eq!(resource_manager_verify_idle(manager), -6);

    // Release frees the lease; a second release is rejected.
    assert_eq!(resource_manager_release(manager, lease), 0);
    assert_eq!(resource_manager_verify_idle(manager), 0);
    assert_eq!(resource_manager_release(manager, lease), -6);

    resource_lease_free(lease);
    resource_manager_free(manager);
    circuit_free(circuit);
}

// =====  Knowledge rules  =====

#[test]
fn test_knowledge_library_builtin() {
    let builtin = knowledge_library_builtin();
    assert!(!builtin.is_null());
    assert!(knowledge_library_len(builtin) > 0);
    assert_eq!(knowledge_library_is_empty(builtin), 0);

    // Builtin rules are registered under their compiler kinds.
    assert!(knowledge_library_rules_by_kind_len(builtin, RULE_KIND_DECOMPOSE) > 0);
    assert!(knowledge_library_rules_by_kind_len(builtin, RULE_KIND_CANCEL) > 0);
    let decompose_len = knowledge_library_rules_by_kind_len(builtin, RULE_KIND_DECOMPOSE);
    let mut ids = vec![0u32; decompose_len];
    assert_eq!(
        knowledge_library_rules_by_kind(
            builtin,
            RULE_KIND_DECOMPOSE,
            ids.as_mut_ptr(),
            decompose_len
        ),
        decompose_len
    );

    // Metadata accessors cover every builtin rule.
    for index in 0..knowledge_library_len(builtin) {
        let rule_name = knowledge_library_rule_name(builtin, index);
        assert!(!rule_name.is_null());
        cqlib_string_free(rule_name);
        let kind = knowledge_library_rule_kind(builtin, index);
        assert!(kind >= RULE_KIND_SIMPLIFY as i32);
        assert!(kind <= RULE_KIND_OTHER as i32);
        assert!(knowledge_library_rule_pattern_len(builtin, index) >= 1);
    }
    let first = knowledge_library_rule_first_instruction(builtin, 0);
    assert!(!first.is_null());
    cqlib_string_free(first);

    // Out-of-bounds indices are rejected.
    assert!(knowledge_library_rule_name(builtin, usize::MAX).is_null());
    assert_eq!(knowledge_library_rule_kind(builtin, usize::MAX), -8);

    knowledge_library_free(builtin);
}

#[test]
fn test_knowledge_library_from_dsl() {
    let source = name(
        "rule merge_rz {\n    match {\n        RZ(a) 0\n        RZ(b) 0\n    }\n    rewrite {\n        RZ(a + b) 0\n    }\n}",
    );
    let mut library: *mut CKnowledgeLibrary = std::ptr::null_mut();
    assert_eq!(
        knowledge_library_from_dsl_str(source.as_ptr(), RULE_KIND_MERGE, &mut library),
        0
    );
    assert!(!library.is_null());
    assert_eq!(knowledge_library_len(library), 1);

    let merge_name = name("merge_rz");
    assert_eq!(knowledge_library_contains(library, merge_name.as_ptr()), 1);
    assert_eq!(
        knowledge_library_id_by_name(library, merge_name.as_ptr()),
        0
    );
    assert_eq!(knowledge_library_rule_pattern_len(library, 0), 2);
    assert_eq!(knowledge_library_rule_rewrite_len(library, 0), 1);
    assert_eq!(knowledge_library_rule_qubit_count(library, 0), 1);
    assert_eq!(knowledge_library_rule_cost_delta(library, 0), -1);
    assert_eq!(knowledge_library_rule_has_conditions(library, 0), 0);
    assert_eq!(
        knowledge_library_rule_kind(library, 0),
        RULE_KIND_MERGE as i32
    );
    let rule_name = knowledge_library_rule_name(library, 0);
    assert!(!rule_name.is_null());
    unsafe {
        assert_eq!(CStr::from_ptr(rule_name).to_str().unwrap(), "merge_rz");
    }
    cqlib_string_free(rule_name);

    // Unknown names report absence, not errors.
    let absent = name("no_such_rule");
    assert_eq!(knowledge_library_contains(library, absent.as_ptr()), 0);
    assert_eq!(knowledge_library_id_by_name(library, absent.as_ptr()), -1);

    // Unknown kind tags are rejected without touching `out`.
    assert_eq!(
        knowledge_library_from_dsl_str(source.as_ptr(), 99, &mut library),
        -8
    );

    knowledge_library_free(library);

    // Malformed DSL text fails with the parse error code.
    let bad = name("this is not a rule");
    let mut bad_library: *mut CKnowledgeLibrary = std::ptr::null_mut();
    assert_eq!(
        knowledge_library_from_dsl_str(bad.as_ptr(), RULE_KIND_OTHER, &mut bad_library),
        -4
    );

    // Missing files fail with the I/O error code.
    let missing = name("no_such_rule_file.rule");
    assert_eq!(
        knowledge_library_from_dsl_file(missing.as_ptr(), RULE_KIND_OTHER, &mut bad_library),
        -5
    );

    // NULL guards.
    assert_eq!(
        knowledge_library_from_dsl_str(std::ptr::null(), RULE_KIND_OTHER, std::ptr::null_mut()),
        -1
    );
    assert_eq!(
        knowledge_library_from_dsl_file(std::ptr::null(), RULE_KIND_OTHER, std::ptr::null_mut()),
        -1
    );
}

#[test]
fn test_knowledge_rule_add() {
    let source =
        name("rule cancel_h {\n    match {\n        H 0\n        H 0\n    }\n    rewrite {}\n}");
    let mut rule: *mut CKnowledgeRule = std::ptr::null_mut();
    assert_eq!(knowledge_rule_from_dsl(source.as_ptr(), &mut rule), 0);
    assert!(!rule.is_null());
    assert_eq!(knowledge_rule_validate(rule), 0);
    assert_eq!(knowledge_rule_pattern_len(rule), 2);
    assert_eq!(knowledge_rule_rewrite_len(rule), 0);
    assert_eq!(knowledge_rule_num_qubits(rule), 1);
    assert_eq!(knowledge_rule_has_conditions(rule), 0);
    let rule_name = knowledge_rule_name(rule);
    assert!(!rule_name.is_null());
    unsafe {
        assert_eq!(CStr::from_ptr(rule_name).to_str().unwrap(), "cancel_h");
    }
    cqlib_string_free(rule_name);

    // Insertion into a fresh library assigns id 0.
    let library = knowledge_library_new();
    assert!(!library.is_null());
    assert_eq!(knowledge_library_is_empty(library), 1);
    assert_eq!(
        knowledge_library_add_rule(library, rule, RULE_KIND_CANCEL),
        0
    );
    assert_eq!(knowledge_library_len(library), 1);
    assert_eq!(
        knowledge_library_rule_kind(library, 0),
        RULE_KIND_CANCEL as i32
    );

    // Duplicate names and unknown kind tags are rejected.
    assert_eq!(
        knowledge_library_add_rule(library, rule, RULE_KIND_CANCEL),
        -4
    );
    assert_eq!(knowledge_library_add_rule(library, rule, 99), -8);

    knowledge_library_free(library);
    knowledge_rule_free(rule);

    // Malformed DSL text fails with the parse error code.
    let bad = name("this is not a rule");
    let mut bad_rule: *mut CKnowledgeRule = std::ptr::null_mut();
    assert_eq!(knowledge_rule_from_dsl(bad.as_ptr(), &mut bad_rule), -4);
    assert_eq!(
        knowledge_rule_from_dsl(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
}

// =====  Commutation  =====

#[test]
fn test_commutation_checks() {
    let x = name("X");
    let z = name("Z");
    let rz = name("RZ");
    let q0 = [0u32];
    let q1 = [1u32];
    let angle_a = [0.3f64];
    let angle_b = [0.5f64];
    let mut phase = 0.0f64;

    // Disjoint supports commute exactly.
    assert_eq!(
        commutation_check(
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q1.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_EXACT
    );
    assert_eq!(phase, 0.0);

    // The same application commutes exactly.
    assert_eq!(
        commutation_check(
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_EXACT
    );

    // X and Z on the same qubit anti-commute, proving commutation up to pi.
    phase = 0.0;
    assert_eq!(
        commutation_check(
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            z.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE
    );
    assert!((phase - std::f64::consts::PI).abs() < 1e-9);

    // Diagonal rotations commute exactly with numeric parameters.
    assert_eq!(
        commutation_check(
            rz.as_ptr(),
            q0.as_ptr(),
            1,
            angle_a.as_ptr(),
            1,
            rz.as_ptr(),
            q0.as_ptr(),
            1,
            angle_b.as_ptr(),
            1,
            &mut phase,
        ),
        COMMUTATION_RESULT_EXACT
    );

    // The algebraic-only variant proves the same facts.
    assert_eq!(
        commutation_check_algebraic(
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q1.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_EXACT
    );
    phase = 0.0;
    assert_eq!(
        commutation_check_algebraic(
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            z.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE
    );
    assert!((phase - std::f64::consts::PI).abs() < 1e-9);

    // Unknown gate names and NULL inputs are rejected.
    let nope = name("NOPE");
    assert_eq!(
        commutation_check(
            nope.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        -8
    );
    assert_eq!(
        commutation_check(
            std::ptr::null(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        -1
    );
}

#[test]
fn test_commutation_checker() {
    let config = commutation_config_default();
    assert_eq!(config.enable_rule_oracle, 1);
    assert_eq!(config.enable_matrix_fallback, 1);
    assert_eq!(config.max_matrix_qubits, 4);

    let x = name("X");
    let q0 = [0u32];
    let q1 = [1u32];
    let mut phase = 0.0f64;

    let checker = commutation_checker_new(config);
    assert!(!checker.is_null());
    assert_eq!(
        commutation_checker_check(
            checker,
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q1.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        COMMUTATION_RESULT_EXACT
    );
    assert_eq!(
        commutation_checker_check(
            std::ptr::null(),
            x.as_ptr(),
            q0.as_ptr(),
            1,
            std::ptr::null(),
            0,
            x.as_ptr(),
            q1.as_ptr(),
            1,
            std::ptr::null(),
            0,
            &mut phase,
        ),
        -1
    );
    commutation_checker_free(checker);

    // A checker can draw its commutation rules from a knowledge library.
    let library = knowledge_library_builtin();
    assert!(!library.is_null());
    let from_library = commutation_checker_from_library(library, commutation_config_default());
    assert!(!from_library.is_null());
    assert!(
        commutation_checker_from_library(std::ptr::null(), commutation_config_default()).is_null()
    );
    commutation_checker_free(from_library);
    knowledge_library_free(library);
}

// =====  NULL guards  =====

#[test]
fn test_null_guards() {
    // Decompose entry points.
    assert!(decompose_expand_definitions(std::ptr::null()).is_null());
    assert!(decompose_unitaries(std::ptr::null(), decompose_unitary_config_default()).is_null());
    assert!(
        decompose_unitaries_with_rule_stats(
            std::ptr::null(),
            decompose_unitary_config_default(),
            std::ptr::null_mut()
        )
        .is_null()
    );
    assert!(decompose_mc_gates(std::ptr::null(), mc_gate_config_default()).is_null());
    assert!(
        decompose_mc_gates_with_rule_stats(
            std::ptr::null(),
            mc_gate_config_default(),
            std::ptr::null_mut()
        )
        .is_null()
    );
    assert!(
        decompose_mc_gates_for_device(std::ptr::null(), std::ptr::null(), std::ptr::null())
            .is_null()
    );
    assert!(resynthesize_two_qubit_blocks(std::ptr::null(), resynthesis_config_normal()).is_null());
    assert!(synthesize_numeric_2q_unitary(std::ptr::null(), 0, 1, TWO_QUBIT_BASIS_CX).is_null());
    assert!(kak_decompose(std::ptr::null()).is_null());
    assert_eq!(
        synthesize_numeric_1q_unitary(std::ptr::null(), std::ptr::null_mut()),
        -1
    );
    let (mut a, mut b, mut c) = (0.0f64, 0.0f64, 0.0f64);
    assert_eq!(
        kak_decomposition_coordinates(std::ptr::null(), &mut a, &mut b, &mut c),
        -1
    );
    let mut factor = [Complex64::new(0.0, 0.0); 4];
    assert_eq!(
        kak_decomposition_local_factor(std::ptr::null(), KAK_FACTOR_K1L, factor.as_mut_ptr()),
        -1
    );
    assert_eq!(two_qubit_synthesis_num_operations(std::ptr::null()), 0);
    assert_eq!(two_qubit_synthesis_global_phase(std::ptr::null()), 0.0);
    assert!(two_qubit_synthesis_operation_name(std::ptr::null(), 0).is_null());
    assert_eq!(
        two_qubit_synthesis_operation_qubits(std::ptr::null(), 0, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(
        two_qubit_synthesis_operation_qubits_len(std::ptr::null(), 0),
        0
    );
    assert_eq!(kak_decomposition_global_phase(std::ptr::null()), 0.0);
    assert!(target_basis_lowerer_apply(std::ptr::null(), std::ptr::null()).is_null());
    assert!(target_basis_lowerer_requires_lowering(std::ptr::null(), std::ptr::null()) == -1);
    assert_eq!(target_basis_lowerer_num_gates(std::ptr::null()), 0);
    assert!(decompose_lower_to_device(std::ptr::null(), std::ptr::null()).is_null());
    two_qubit_synthesis_free(std::ptr::null_mut());
    kak_decomposition_free(std::ptr::null_mut());
    target_basis_lowerer_free(std::ptr::null_mut());

    // Resource entry points.
    assert!(
        resource_manager_from_circuit(std::ptr::null(), std::ptr::null(), std::ptr::null())
            .is_null()
    );
    assert!(
        resource_manager_preview(
            std::ptr::null(),
            RESOURCE_REQUIREMENT_CLEAN_ZERO,
            1,
            std::ptr::null(),
            0
        )
        .is_null()
    );
    assert!(resource_manager_commit(std::ptr::null_mut(), std::ptr::null_mut()).is_null());
    assert!(resource_manager_circuit(std::ptr::null()).is_null());
    assert_eq!(
        resource_manager_release(std::ptr::null_mut(), std::ptr::null()),
        -1
    );
    assert_eq!(resource_manager_enter_post_layout(std::ptr::null_mut()), -1);
    assert_eq!(resource_manager_verify_consistency(std::ptr::null()), -1);
    assert_eq!(resource_manager_verify_idle(std::ptr::null()), -1);
    assert_eq!(resource_plan_qubits_len(std::ptr::null()), 0);
    assert_eq!(resource_plan_num_new_qubits(std::ptr::null()), 0);
    assert_eq!(resource_plan_requirement(std::ptr::null()), -1);
    assert_eq!(resource_lease_qubits_len(std::ptr::null()), 0);
    assert_eq!(resource_lease_requirement(std::ptr::null()), -1);
    resource_manager_free(std::ptr::null_mut());
    resource_plan_free(std::ptr::null_mut());
    resource_lease_free(std::ptr::null_mut());

    // Knowledge entry points.
    assert_eq!(knowledge_library_len(std::ptr::null()), 0);
    assert_eq!(knowledge_library_is_empty(std::ptr::null()), -1);
    assert_eq!(
        knowledge_library_contains(std::ptr::null(), std::ptr::null()),
        -1
    );
    assert_eq!(
        knowledge_library_id_by_name(std::ptr::null(), std::ptr::null()),
        -1
    );
    assert_eq!(
        knowledge_library_rules_by_kind_len(std::ptr::null(), RULE_KIND_OTHER),
        0
    );
    assert_eq!(
        knowledge_library_rules_by_kind(std::ptr::null(), RULE_KIND_OTHER, std::ptr::null_mut(), 0),
        0
    );
    assert!(knowledge_library_rule_name(std::ptr::null(), 0).is_null());
    assert_eq!(knowledge_library_rule_kind(std::ptr::null(), 0), -1);
    assert!(knowledge_library_rule_first_instruction(std::ptr::null(), 0).is_null());
    assert_eq!(knowledge_library_rule_pattern_len(std::ptr::null(), 0), 0);
    assert_eq!(knowledge_library_rule_rewrite_len(std::ptr::null(), 0), 0);
    assert_eq!(knowledge_library_rule_qubit_count(std::ptr::null(), 0), 0);
    assert_eq!(knowledge_library_rule_cost_delta(std::ptr::null(), 0), 0);
    assert_eq!(
        knowledge_library_rule_has_conditions(std::ptr::null(), 0),
        -1
    );
    assert_eq!(
        knowledge_library_add_rule(std::ptr::null_mut(), std::ptr::null(), RULE_KIND_OTHER),
        -1
    );
    assert!(knowledge_rule_name(std::ptr::null()).is_null());
    assert_eq!(knowledge_rule_num_qubits(std::ptr::null()), 0);
    assert_eq!(knowledge_rule_pattern_len(std::ptr::null()), 0);
    assert_eq!(knowledge_rule_rewrite_len(std::ptr::null()), 0);
    assert_eq!(knowledge_rule_has_conditions(std::ptr::null()), -1);
    assert_eq!(knowledge_rule_validate(std::ptr::null()), -1);
    knowledge_library_free(std::ptr::null_mut());
    knowledge_rule_free(std::ptr::null_mut());

    // Commutation entry points.
    assert_eq!(
        commutation_check(
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        ),
        -1
    );
    assert_eq!(
        commutation_check_algebraic(
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        ),
        -1
    );
    assert_eq!(
        commutation_checker_check(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        ),
        -1
    );
    commutation_checker_free(std::ptr::null_mut());
}
