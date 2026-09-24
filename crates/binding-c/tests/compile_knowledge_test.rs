//! Integration tests for the knowledge matching engine C ABI: rule listing
//! and lookup, transactional matching, condition checks, equivalence,
//! candidate/key filtering, free symbols, rule extension, and target
//! instantiation.

use binding_c::circuit::{COperation, operation_free, operation_new};
use binding_c::compile::*;
use binding_c::cqlib_string_free;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

const MERGE_DSL: &str = "rule merge_rz {\n    match {\n        RZ(a) 0\n        RZ(b) 0\n    }\n    rewrite {\n        RZ(a + b) 0\n    }\n}";
const CANCEL_H_DSL: &str =
    "rule cancel_h {\n    match {\n        H 0\n        H 0\n    }\n    rewrite {}\n}";
const CANCEL_X_DSL: &str =
    "rule cancel_x {\n    match {\n        X 0\n        X 0\n    }\n    rewrite {}\n}";
const CONDITION_DSL: &str = "rule cancel_rz_inverse {\n    match { RZ(a) 0, RZ(b) 0 }\n    require { a + b == 0 mod 4*π }\n    rewrite {}\n}";
const SHIFT_DSL: &str =
    "rule shift_rz {\n    match { RZ(a) 0, RZ(b) 1 }\n    rewrite { RZ(a + b) 0, RZ(a + b) 1 }\n}";

fn name(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn make_rule(dsl: &str) -> *mut CKnowledgeRule {
    let source = name(dsl);
    let mut rule: *mut CKnowledgeRule = std::ptr::null_mut();
    assert_eq!(knowledge_rule_from_dsl(source.as_ptr(), &mut rule), 0);
    assert!(!rule.is_null());
    rule
}

fn make_op(gate: &str, qubits: &[u32], params: &[f64]) -> *mut COperation {
    let gate = name(gate);
    let op = operation_new(
        gate.as_ptr(),
        if qubits.is_empty() {
            std::ptr::null()
        } else {
            qubits.as_ptr()
        },
        qubits.len(),
        if params.is_empty() {
            std::ptr::null()
        } else {
            params.as_ptr()
        },
        params.len(),
    );
    assert!(!op.is_null());
    op
}

fn read_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null());
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    cqlib_string_free(ptr);
    text
}

// =====  Bindings lifecycle  =====

#[test]
fn test_knowledge_bindings_lifecycle() {
    let bindings = knowledge_bindings_new();
    assert!(!bindings.is_null());
    knowledge_bindings_free(bindings);
    knowledge_bindings_free(std::ptr::null_mut());
}

// =====  Rule listing and lookup  =====

#[test]
fn test_knowledge_rules_listing() {
    let source = name(&format!("{CANCEL_H_DSL}\n{CANCEL_X_DSL}"));
    let mut library: *mut CKnowledgeLibrary = std::ptr::null_mut();
    assert_eq!(
        knowledge_library_from_dsl_str(source.as_ptr(), RULE_KIND_CANCEL, &mut library),
        0
    );

    assert_eq!(knowledge_rules_len(library), 2);
    let mut ids = [99u32; 2];
    assert_eq!(knowledge_rules(library, ids.as_mut_ptr(), 2), 2);
    assert_eq!(ids, [0, 1]);

    // Truncated buffer reports the full total but only writes the prefix.
    let mut truncated = [99u32; 1];
    assert_eq!(knowledge_rules(library, truncated.as_mut_ptr(), 1), 2);
    assert_eq!(truncated[0], 0);

    assert_eq!(knowledge_rules(library, std::ptr::null_mut(), 2), 2);
    assert_eq!(knowledge_rules_len(std::ptr::null()), 0);
    assert_eq!(knowledge_rules(std::ptr::null(), ids.as_mut_ptr(), 2), 0);

    knowledge_library_free(library);
}

#[test]
fn test_knowledge_get_by_name() {
    let source = name(&format!("{CANCEL_H_DSL}\n{CANCEL_X_DSL}"));
    let mut library: *mut CKnowledgeLibrary = std::ptr::null_mut();
    assert_eq!(
        knowledge_library_from_dsl_str(source.as_ptr(), RULE_KIND_CANCEL, &mut library),
        0
    );

    let cancel_h = name("cancel_h");
    let rule = knowledge_get_by_name(library, cancel_h.as_ptr());
    assert!(!rule.is_null());
    let rule_name = knowledge_rule_name(rule);
    assert_eq!(read_string(rule_name), "cancel_h");
    knowledge_rule_free(rule);

    let absent = name("no_such_rule");
    assert!(knowledge_get_by_name(library, absent.as_ptr()).is_null());
    assert!(knowledge_get_by_name(std::ptr::null(), cancel_h.as_ptr()).is_null());
    assert!(knowledge_get_by_name(library, std::ptr::null()).is_null());

    knowledge_library_free(library);
}

// =====  Matching  =====

#[test]
fn test_knowledge_match_rule_item() {
    let rule = make_rule(MERGE_DSL);
    let op_a = make_op("RZ", &[7], &[0.25]);
    let op_b = make_op("RZ", &[7], &[0.5]);
    let op_conflict = make_op("RZ", &[9], &[0.5]);
    let op_h = make_op("H", &[7], &[]);

    let bindings = knowledge_bindings_new();

    // Both pattern items bind against the same window, transactionally.
    assert_eq!(knowledge_match_rule_item(rule, 0, op_a, bindings), 1);
    assert_eq!(knowledge_match_rule_item(rule, 1, op_b, bindings), 1);

    // A conflicting qubit fails without corrupting existing bindings.
    assert_eq!(knowledge_match_rule_item(rule, 0, op_conflict, bindings), 0);
    // A different gate never matches the RZ pattern item.
    assert_eq!(knowledge_match_rule_item(rule, 0, op_h, bindings), 0);

    // The rolled-back item can be re-matched on the original qubit.
    assert_eq!(knowledge_match_rule_item(rule, 1, op_b, bindings), 1);

    // Error paths.
    assert_eq!(knowledge_match_rule_item(rule, 5, op_a, bindings), -2);
    assert_eq!(
        knowledge_match_rule_item(std::ptr::null(), 0, op_a, bindings),
        -1
    );
    assert_eq!(
        knowledge_match_rule_item(rule, 0, std::ptr::null(), bindings),
        -1
    );
    assert_eq!(
        knowledge_match_rule_item(rule, 0, op_a, std::ptr::null_mut()),
        -1
    );

    knowledge_bindings_free(bindings);
    operation_free(op_h);
    operation_free(op_conflict);
    operation_free(op_b);
    operation_free(op_a);
    knowledge_rule_free(rule);
}

#[test]
fn test_knowledge_rule_matches_operations() {
    let rule = make_rule(CANCEL_H_DSL);
    let op_h1 = make_op("H", &[7], &[]);
    let op_h2 = make_op("H", &[7], &[]);
    let op_x = make_op("X", &[7], &[]);

    let adjacent = [op_h1 as *const COperation, op_h2 as *const COperation];
    let interrupted = [op_h1 as *const COperation, op_x as *const COperation];

    let mut bindings: *mut CKnowledgeBindings = std::ptr::null_mut();
    assert_eq!(
        knowledge_rule_matches_operations(rule, adjacent.as_ptr(), 2, &mut bindings),
        1
    );
    assert!(!bindings.is_null());
    knowledge_bindings_free(bindings);

    let mut no_match: *mut CKnowledgeBindings = std::ptr::null_mut();
    assert_eq!(
        knowledge_rule_matches_operations(rule, interrupted.as_ptr(), 2, &mut no_match),
        0
    );
    assert!(no_match.is_null());

    // The operation count must equal the pattern length.
    assert_eq!(
        knowledge_rule_matches_operations(rule, adjacent.as_ptr(), 1, &mut bindings),
        -8
    );
    assert_eq!(
        knowledge_rule_matches_operations(std::ptr::null(), adjacent.as_ptr(), 2, &mut bindings),
        -1
    );
    assert_eq!(
        knowledge_rule_matches_operations(rule, std::ptr::null(), 2, &mut bindings),
        -1
    );
    assert_eq!(
        knowledge_rule_matches_operations(rule, adjacent.as_ptr(), 2, std::ptr::null_mut()),
        -1
    );

    operation_free(op_x);
    operation_free(op_h2);
    operation_free(op_h1);
    knowledge_rule_free(rule);
}

#[test]
fn test_knowledge_conditions_hold() {
    let rule = make_rule(CONDITION_DSL);
    let op_hold_a = make_op("RZ", &[3], &[0.0]);
    let op_hold_b = make_op("RZ", &[3], &[0.0]);
    let op_fail_a = make_op("RZ", &[3], &[0.1]);
    let op_fail_b = make_op("RZ", &[3], &[0.2]);

    let holding = [
        op_hold_a as *const COperation,
        op_hold_b as *const COperation,
    ];
    let failing = [
        op_fail_a as *const COperation,
        op_fail_b as *const COperation,
    ];

    let mut bindings: *mut CKnowledgeBindings = std::ptr::null_mut();
    assert_eq!(
        knowledge_rule_matches_operations(rule, holding.as_ptr(), 2, &mut bindings),
        1
    );
    assert_eq!(knowledge_conditions_hold(rule, bindings), 1);
    knowledge_bindings_free(bindings);

    let mut failing_bindings: *mut CKnowledgeBindings = std::ptr::null_mut();
    assert_eq!(
        knowledge_rule_matches_operations(rule, failing.as_ptr(), 2, &mut failing_bindings),
        1
    );
    assert_eq!(knowledge_conditions_hold(rule, failing_bindings), 0);
    knowledge_bindings_free(failing_bindings);

    assert_eq!(
        knowledge_conditions_hold(std::ptr::null(), std::ptr::null()),
        -1
    );

    operation_free(op_fail_b);
    operation_free(op_fail_a);
    operation_free(op_hold_b);
    operation_free(op_hold_a);
    knowledge_rule_free(rule);
}

#[test]
fn test_knowledge_equivalent_to() {
    let cancel_h_1 = make_rule(CANCEL_H_DSL);
    let cancel_h_2 = make_rule(CANCEL_H_DSL);
    let cancel_x = make_rule(CANCEL_X_DSL);
    let merge = make_rule(MERGE_DSL);
    let condition = make_rule(CONDITION_DSL);

    // Identical rules have equivalent items.
    assert_eq!(knowledge_equivalent_to(cancel_h_1, 0, cancel_h_2, 0), 1);
    assert_eq!(knowledge_equivalent_to(cancel_h_1, 1, cancel_h_2, 1), 1);
    // Different gates are not equivalent.
    assert_eq!(knowledge_equivalent_to(cancel_h_1, 0, cancel_x, 0), 0);
    // RZ(a) in the merge rule matches RZ(a) in the condition rule.
    assert_eq!(knowledge_equivalent_to(merge, 0, condition, 0), 1);

    // Out-of-bounds item indices and NULL guards.
    assert_eq!(knowledge_equivalent_to(cancel_h_1, 7, cancel_h_2, 0), -2);
    assert_eq!(knowledge_equivalent_to(cancel_h_1, 0, cancel_h_2, 7), -2);
    assert_eq!(
        knowledge_equivalent_to(std::ptr::null(), 0, cancel_h_2, 0),
        -1
    );

    knowledge_rule_free(condition);
    knowledge_rule_free(merge);
    knowledge_rule_free(cancel_x);
    knowledge_rule_free(cancel_h_2);
    knowledge_rule_free(cancel_h_1);
}

// =====  Candidate and key filtering  =====

#[test]
fn test_knowledge_candidates_for_first_instruction() {
    let library = knowledge_library_builtin();
    assert!(!library.is_null());

    let h = name("H");
    let total = knowledge_candidates_for_first_instruction_len(library, h.as_ptr());
    assert!(total >= 1);
    let mut ids = vec![0u32; total];
    assert_eq!(
        knowledge_candidates_for_first_instruction(library, h.as_ptr(), ids.as_mut_ptr(), total),
        total
    );
    // cancel_h is a builtin rule starting with H.
    assert!(
        ids.contains(&(knowledge_library_id_by_name(library, name("cancel_h").as_ptr()) as u32))
    );

    // Truncated copies still report the full total.
    let mut truncated = [0u32; 1];
    assert_eq!(
        knowledge_candidates_for_first_instruction(library, h.as_ptr(), truncated.as_mut_ptr(), 1),
        total
    );
    assert_eq!(truncated[0], ids[0]);

    // Unknown gate names and NULL inputs report no candidates.
    let unknown = name("NOPE");
    assert_eq!(
        knowledge_candidates_for_first_instruction_len(library, unknown.as_ptr()),
        0
    );
    assert_eq!(
        knowledge_candidates_for_first_instruction_len(std::ptr::null(), h.as_ptr()),
        0
    );
    assert_eq!(
        knowledge_candidates_for_first_instruction(
            library,
            std::ptr::null(),
            std::ptr::null_mut(),
            0
        ),
        0
    );

    knowledge_library_free(library);
}

#[test]
fn test_knowledge_collect_free_symbols() {
    let merge = make_rule(MERGE_DSL);
    let cancel = make_rule(CANCEL_H_DSL);

    assert_eq!(knowledge_collect_free_symbols_len(merge), 2);
    let mut symbols: [*mut c_char; 2] = [std::ptr::null_mut(); 2];
    assert_eq!(
        knowledge_collect_free_symbols(merge, symbols.as_mut_ptr(), 2),
        2
    );
    assert_eq!(read_string(symbols[0]), "a");
    assert_eq!(read_string(symbols[1]), "b");

    assert_eq!(knowledge_collect_free_symbols_len(cancel), 0);
    assert_eq!(
        knowledge_collect_free_symbols(cancel, std::ptr::null_mut(), 0),
        0
    );

    assert_eq!(knowledge_collect_free_symbols_len(std::ptr::null()), 0);
    assert_eq!(
        knowledge_collect_free_symbols(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );

    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

#[test]
fn test_knowledge_filter_rule_ids_by_instruction_keys() {
    let source = name(MERGE_DSL);
    let mut library: *mut CKnowledgeLibrary = std::ptr::null_mut();
    assert_eq!(
        knowledge_library_from_dsl_str(source.as_ptr(), RULE_KIND_MERGE, &mut library),
        0
    );

    let rz = name("RZ");
    let op_names = [rz.as_ptr()];
    let target_names = [rz.as_ptr()];

    let total = knowledge_filter_rule_ids_by_instruction_keys_len(
        library,
        op_names.as_ptr(),
        1,
        target_names.as_ptr(),
        1,
    );
    assert_eq!(total, 1);
    let mut ids = [0u32; 1];
    assert_eq!(
        knowledge_filter_rule_ids_by_instruction_keys(
            library,
            op_names.as_ptr(),
            1,
            target_names.as_ptr(),
            1,
            ids.as_mut_ptr(),
            1
        ),
        1
    );
    assert_eq!(ids, [0]);

    // A match side that does not cover RZ filters everything out.
    let h = name("H");
    let h_names = [h.as_ptr()];
    assert_eq!(
        knowledge_filter_rule_ids_by_instruction_keys_len(
            library,
            h_names.as_ptr(),
            1,
            target_names.as_ptr(),
            1
        ),
        0
    );

    // Unknown gate names and NULL inputs report nothing.
    let unknown = name("NOPE");
    let unknown_names = [unknown.as_ptr()];
    assert_eq!(
        knowledge_filter_rule_ids_by_instruction_keys_len(
            library,
            unknown_names.as_ptr(),
            1,
            target_names.as_ptr(),
            1
        ),
        0
    );
    assert_eq!(
        knowledge_filter_rule_ids_by_instruction_keys_len(
            std::ptr::null(),
            op_names.as_ptr(),
            1,
            target_names.as_ptr(),
            1
        ),
        0
    );

    knowledge_library_free(library);
}

// =====  Rule extension and target queries  =====

#[test]
fn test_knowledge_extend_rules() {
    let cancel_h = make_rule(CANCEL_H_DSL);
    let cancel_x = make_rule(CANCEL_X_DSL);
    let rules = [
        cancel_h as *const CKnowledgeRule,
        cancel_x as *const CKnowledgeRule,
    ];

    let library = knowledge_library_new();
    let mut ids = [0u32; 2];
    assert_eq!(
        knowledge_extend_rules(
            library,
            rules.as_ptr(),
            2,
            RULE_KIND_CANCEL,
            ids.as_mut_ptr(),
            2
        ),
        0
    );
    assert_eq!(ids, [0, 1]);
    assert_eq!(knowledge_library_len(library), 2);
    assert_eq!(
        knowledge_library_rule_kind(library, 0),
        RULE_KIND_CANCEL as i32
    );

    // Duplicate names are rejected and the library stays unchanged.
    assert_eq!(
        knowledge_extend_rules(
            library,
            rules.as_ptr(),
            2,
            RULE_KIND_CANCEL,
            ids.as_mut_ptr(),
            2
        ),
        -4
    );
    assert_eq!(knowledge_library_len(library), 2);

    // Unknown kind tags and small buffers are rejected before insertion.
    assert_eq!(
        knowledge_extend_rules(library, rules.as_ptr(), 2, 99, ids.as_mut_ptr(), 2),
        -8
    );
    assert_eq!(
        knowledge_extend_rules(
            library,
            rules.as_ptr(),
            2,
            RULE_KIND_CANCEL,
            ids.as_mut_ptr(),
            1
        ),
        -8
    );

    // NULL guards.
    assert_eq!(
        knowledge_extend_rules(
            std::ptr::null_mut(),
            rules.as_ptr(),
            2,
            RULE_KIND_CANCEL,
            ids.as_mut_ptr(),
            2
        ),
        -1
    );
    assert_eq!(
        knowledge_extend_rules(
            library,
            std::ptr::null(),
            2,
            RULE_KIND_CANCEL,
            ids.as_mut_ptr(),
            2
        ),
        -1
    );

    knowledge_library_free(library);
    knowledge_rule_free(cancel_x);
    knowledge_rule_free(cancel_h);
}

#[test]
fn test_knowledge_target_qubits() {
    let merge = make_rule(MERGE_DSL);
    let cancel = make_rule(CANCEL_H_DSL);
    let shift = make_rule(SHIFT_DSL);

    assert_eq!(knowledge_target_qubits_len(merge), 1);
    let mut qubits = [0u32; 1];
    assert_eq!(knowledge_target_qubits(merge, qubits.as_mut_ptr(), 1), 1);
    assert_eq!(qubits, [0]);

    // The shift rule rewrites on rule-local labels 0 and 1.
    assert_eq!(knowledge_target_qubits_len(shift), 2);
    let mut shift_qubits = [0u32; 2];
    assert_eq!(
        knowledge_target_qubits(shift, shift_qubits.as_mut_ptr(), 2),
        2
    );
    assert_eq!(shift_qubits, [0, 1]);

    // An empty rewrite target has no qubits.
    assert_eq!(knowledge_target_qubits_len(cancel), 0);
    assert_eq!(knowledge_target_qubits(cancel, std::ptr::null_mut(), 0), 0);

    assert_eq!(knowledge_target_qubits_len(std::ptr::null()), 0);
    assert_eq!(
        knowledge_target_qubits(std::ptr::null(), std::ptr::null_mut(), 0),
        0
    );

    knowledge_rule_free(shift);
    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

// =====  Target instantiation  =====

#[test]
fn test_knowledge_instantiate_target() {
    let rule = make_rule(MERGE_DSL);
    let op_a = make_op("RZ", &[7], &[0.25]);
    let op_b = make_op("RZ", &[7], &[0.5]);

    // Match the full window, then instantiate the rewrite target.
    let adjacent = [op_a as *const COperation, op_b as *const COperation];
    let mut bindings: *mut CKnowledgeBindings = std::ptr::null_mut();
    assert_eq!(
        knowledge_rule_matches_operations(rule, adjacent.as_ptr(), 2, &mut bindings),
        1
    );

    let mut replacements: *mut CKnowledgeReplacements = std::ptr::null_mut();
    assert_eq!(
        knowledge_instantiate_target(rule, bindings, &mut replacements),
        0
    );
    assert!(!replacements.is_null());
    assert_eq!(knowledge_replacements_len(replacements), 1);

    assert_eq!(
        read_string(knowledge_replacement_instruction(replacements, 0)),
        "RZ"
    );

    assert_eq!(knowledge_replacement_qubits_len(replacements, 0), 1);
    let mut qubits = [0u32; 1];
    assert_eq!(
        knowledge_replacement_qubits(replacements, 0, qubits.as_mut_ptr(), 1),
        1
    );
    assert_eq!(qubits, [7]);

    assert_eq!(knowledge_replacement_params_len(replacements, 0), 1);
    let mut params = [0.0f64; 1];
    assert_eq!(
        knowledge_replacement_params(replacements, 0, params.as_mut_ptr(), 1),
        1
    );
    assert!((params[0] - 0.75).abs() < 1e-12);

    // Out-of-bounds replacement indices are benign.
    assert!(knowledge_replacement_instruction(replacements, 3).is_null());
    assert_eq!(knowledge_replacement_qubits_len(replacements, 3), 0);
    assert_eq!(
        knowledge_replacement_qubits(replacements, 3, std::ptr::null_mut(), 0),
        0
    );
    assert_eq!(knowledge_replacement_params_len(replacements, 3), 0);
    assert_eq!(
        knowledge_replacement_params(replacements, 3, std::ptr::null_mut(), 0),
        0
    );
    knowledge_replacements_free(replacements);

    // Instantiating against empty bindings fails on the unbound qubit.
    let empty = knowledge_bindings_new();
    let mut unbound: *mut CKnowledgeReplacements = std::ptr::null_mut();
    assert_eq!(knowledge_instantiate_target(rule, empty, &mut unbound), -2);
    knowledge_bindings_free(empty);

    // NULL guards.
    assert_eq!(
        knowledge_instantiate_target(std::ptr::null(), bindings, &mut unbound),
        -1
    );
    assert_eq!(
        knowledge_instantiate_target(rule, std::ptr::null(), &mut unbound),
        -1
    );
    assert_eq!(
        knowledge_instantiate_target(rule, bindings, std::ptr::null_mut()),
        -1
    );
    knowledge_bindings_free(bindings);

    // NULL-safe accessors and frees.
    assert_eq!(knowledge_replacements_len(std::ptr::null()), 0);
    assert!(knowledge_replacement_instruction(std::ptr::null(), 0).is_null());
    knowledge_replacements_free(std::ptr::null_mut());

    operation_free(op_b);
    operation_free(op_a);
    knowledge_rule_free(rule);
}
