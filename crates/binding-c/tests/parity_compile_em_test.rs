// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Parity tests for the compile / error-mitigation C ABI additions:
//! `PauliEvolutionAnsatz` evolution info, `VirtualDistillation` copy-swap
//! circuit building, knowledge-rule symbol queries, and compile step skip
//! reasons.

use binding_c::circuit::{
    CPauliEvolutionInfo, TROTTER_FIRST_ORDER, TROTTER_MODE_NONE, circuit_cx, circuit_free,
    circuit_h, circuit_new, circuit_num_operations, circuit_num_qubits,
    pauli_evolution_ansatz_evolution_info, pauli_evolution_ansatz_free, pauli_evolution_ansatz_new,
};
use binding_c::compile::{
    CKnowledgeRule, COMPILE_MODE_NORMAL, COMPILE_TARGET_LOGICAL, CompileConfigC, compile,
    compile_result_free, compile_result_num_steps, compile_result_step_name,
    compile_result_step_reason, knowledge_rule_condition_symbols,
    knowledge_rule_condition_symbols_len, knowledge_rule_free, knowledge_rule_from_dsl,
    knowledge_rule_operation_symbols, knowledge_rule_operation_symbols_len,
    knowledge_rule_target_symbols, knowledge_rule_target_symbols_len,
};
use binding_c::cqlib_string_free;
use binding_c::error_mitigation::{
    virtual_distillation_build_copy_swap_circuit, virtual_distillation_free,
    virtual_distillation_new,
};
use binding_c::qis::{
    CHamiltonian, hamiltonian_add_term, hamiltonian_free, hamiltonian_new, pauli_string_free,
    pauli_string_parse,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

const MERGE_DSL: &str = "rule merge_rz {\n    match {\n        RZ(a) 0\n        RZ(b) 0\n    }\n    rewrite {\n        RZ(a + b) 0\n    }\n}";
const CANCEL_H_DSL: &str =
    "rule cancel_h {\n    match {\n        H 0\n        H 0\n    }\n    rewrite {}\n}";
const CONDITION_DSL: &str = "rule cancel_rz_inverse {\n    match { RZ(a) 0, RZ(b) 0 }\n    require { a + b == 0 mod 4*π }\n    rewrite {}\n}";

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn read_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null());
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    cqlib_string_free(ptr);
    text
}

fn make_rule(dsl: &str) -> *mut CKnowledgeRule {
    let source = cstr(dsl);
    let mut rule: *mut CKnowledgeRule = std::ptr::null_mut();
    assert_eq!(knowledge_rule_from_dsl(source.as_ptr(), &mut rule), 0);
    assert!(!rule.is_null());
    rule
}

/// Builds a real-coefficient Hamiltonian from `(pauli text, coefficient)`
/// terms.
fn hamiltonian_with_terms(width: usize, terms: &[(&str, f64)]) -> *mut CHamiltonian {
    let h = hamiltonian_new(width);
    assert!(!h.is_null());
    for (text, coeff) in terms {
        let pauli = pauli_string_parse(cstr(text).as_ptr());
        assert!(!pauli.is_null());
        assert_eq!(hamiltonian_add_term(h, pauli, *coeff, 0.0), 0);
        pauli_string_free(pauli);
    }
    h
}

// =====  PauliEvolutionAnsatz::evolution_info  =====

#[test]
fn test_evolution_info_commuting_exact() {
    // ZZ and ZI mutually commute: Auto takes the exact single-pass path.
    let h = hamiltonian_with_terms(2, &[("ZZ", 0.5), ("ZI", 0.3)]);
    let ansatz = pauli_evolution_ansatz_new(h);
    assert!(!ansatz.is_null());

    let mut info: CPauliEvolutionInfo = unsafe { std::mem::zeroed() };
    assert_eq!(pauli_evolution_ansatz_evolution_info(ansatz, &mut info), 0);
    assert_eq!(info.is_exact, 1);
    assert_eq!(info.all_terms_commute, 1);
    assert_eq!(info.steps, 1);
    assert_eq!(info.num_terms, 2);
    assert_eq!(info.trotter_mode, TROTTER_MODE_NONE);
    assert_eq!(info.trotter_seed, 0);

    // NULL guards.
    let mut sink: CPauliEvolutionInfo = unsafe { std::mem::zeroed() };
    assert_eq!(
        pauli_evolution_ansatz_evolution_info(std::ptr::null(), &mut sink),
        -1
    );
    assert_eq!(
        pauli_evolution_ansatz_evolution_info(ansatz, std::ptr::null_mut()),
        -1
    );

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(h);
}

#[test]
fn test_evolution_info_non_commuting_trotter() {
    // X and Z do not commute: Auto falls back to a first-order Trotter pass.
    let h = hamiltonian_with_terms(1, &[("X", 1.0), ("Z", 1.0)]);
    let ansatz = pauli_evolution_ansatz_new(h);
    assert!(!ansatz.is_null());

    let mut info: CPauliEvolutionInfo = unsafe { std::mem::zeroed() };
    assert_eq!(pauli_evolution_ansatz_evolution_info(ansatz, &mut info), 0);
    assert_eq!(info.is_exact, 0);
    assert_eq!(info.all_terms_commute, 0);
    assert_eq!(info.steps, 1);
    assert_eq!(info.num_terms, 2);
    assert_eq!(info.trotter_mode, TROTTER_FIRST_ORDER);
    assert_eq!(info.trotter_seed, 0);

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(h);
}

// =====  VirtualDistillation::build_copy_swap_circuit  =====

#[test]
fn test_virtual_distillation_build_copy_swap_circuit() {
    let circuit = circuit_new(1);
    assert_eq!(circuit_h(circuit, 0), 0);

    let vd = virtual_distillation_new(circuit, 2);
    assert!(!vd.is_null());

    let copy_swap = virtual_distillation_build_copy_swap_circuit(vd);
    assert!(!copy_swap.is_null());
    // The copy-swap width is copies (2) times the base width (1).
    assert_eq!(circuit_num_qubits(copy_swap), 2);
    assert!(
        circuit_num_operations(copy_swap) > 0,
        "copy-swap circuit should contain operations"
    );
    circuit_free(copy_swap);

    // NULL input yields NULL.
    assert!(virtual_distillation_build_copy_swap_circuit(std::ptr::null()).is_null());

    virtual_distillation_free(vd);
    circuit_free(circuit);
}

// =====  KnowledgeRule symbol queries  =====

#[test]
fn test_knowledge_rule_symbol_families() {
    let merge = make_rule(MERGE_DSL);
    let cancel = make_rule(CANCEL_H_DSL);
    let condition = make_rule(CONDITION_DSL);

    // Match-block symbols: both merge branches bind a and b (sorted).
    assert_eq!(knowledge_rule_operation_symbols_len(merge), 2);
    let mut ops: [*mut c_char; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(
        knowledge_rule_operation_symbols(merge, ops.as_mut_ptr(), 2),
        0
    );
    assert_eq!(read_string(ops[0]), "a");
    assert_eq!(read_string(ops[1]), "b");

    // Rewrite-target symbols: the merge target references a and b; the
    // cancel target is empty.
    assert_eq!(knowledge_rule_target_symbols_len(merge), 2);
    let mut targets: [*mut c_char; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(
        knowledge_rule_target_symbols(merge, targets.as_mut_ptr(), 2),
        0
    );
    assert_eq!(read_string(targets[0]), "a");
    assert_eq!(read_string(targets[1]), "b");
    assert_eq!(knowledge_rule_target_symbols_len(cancel), 0);
    assert_eq!(
        knowledge_rule_target_symbols(cancel, std::ptr::null_mut(), 0),
        0
    );

    // Condition symbols: only the condition rule carries a require block.
    assert_eq!(knowledge_rule_condition_symbols_len(condition), 2);
    let mut conds: [*mut c_char; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(
        knowledge_rule_condition_symbols(condition, conds.as_mut_ptr(), 2),
        0
    );
    assert_eq!(read_string(conds[0]), "a");
    assert_eq!(read_string(conds[1]), "b");
    assert_eq!(knowledge_rule_condition_symbols_len(merge), 0);
    assert_eq!(knowledge_rule_condition_symbols_len(cancel), 0);

    // A wrong buffer length is rejected before any string is written.
    let mut short: [*mut c_char; 1] = [std::ptr::null_mut()];
    assert_eq!(
        knowledge_rule_operation_symbols(merge, short.as_mut_ptr(), 1),
        -8
    );
    assert!(short[0].is_null());
    // A NULL buffer with a positive length is rejected.
    assert_eq!(
        knowledge_rule_operation_symbols(merge, std::ptr::null_mut(), 2),
        -1
    );

    // NULL guards.
    assert_eq!(knowledge_rule_operation_symbols_len(std::ptr::null()), 0);
    assert_eq!(knowledge_rule_target_symbols_len(std::ptr::null()), 0);
    assert_eq!(knowledge_rule_condition_symbols_len(std::ptr::null()), 0);
    assert_eq!(
        knowledge_rule_operation_symbols(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        knowledge_rule_target_symbols(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert_eq!(
        knowledge_rule_condition_symbols(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );

    knowledge_rule_free(condition);
    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

// =====  Compile step skip reasons  =====

#[test]
fn test_compile_result_step_reason() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let config = CompileConfigC {
        mode: COMPILE_MODE_NORMAL,
        target: COMPILE_TARGET_LOGICAL,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0; 4],
    };
    let result = compile(circuit, config);
    assert!(!result.is_null(), "compile should succeed");

    let n = compile_result_num_steps(result);
    assert!(n > 0);

    // Every step has a name; steps with a reason report non-empty text. A
    // logical-target compile skips the target-basis steps with documented
    // reasons, so at least one reason must be present.
    let mut reasons = 0;
    for i in 0..n {
        let name_ptr = compile_result_step_name(result, i);
        assert!(!name_ptr.is_null(), "step {i} should have a name");
        cqlib_string_free(name_ptr);

        let reason_ptr = compile_result_step_reason(result, i);
        if !reason_ptr.is_null() {
            let reason = read_string(reason_ptr);
            assert!(!reason.is_empty(), "step {i} reason should be non-empty");
            reasons += 1;
        }
    }
    assert!(reasons > 0, "expected at least one step with a skip reason");

    // Out-of-bounds indices and NULL input yield NULL.
    assert!(compile_result_step_reason(result, n + 10).is_null());
    assert!(compile_result_step_reason(std::ptr::null(), 0).is_null());

    compile_result_free(result);
    circuit_free(circuit);
}
