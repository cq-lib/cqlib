// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating that
// they have been altered from the originals.

// Smoke tests for the knowledge matching-engine C ABI: rule listing and
// lookup, transactional matching, condition checks, equivalence, candidate
// and key filtering, free symbols, rule extension, and target instantiation.

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

// DSL sources for the rules used below. `\xcf\x80` is the UTF-8 encoding of
// the Greek letter pi.
static const char MERGE_DSL[] =
    "rule merge_rz {\n"
    "    match {\n"
    "        RZ(a) 0\n"
    "        RZ(b) 0\n"
    "    }\n"
    "    rewrite {\n"
    "        RZ(a + b) 0\n"
    "    }\n"
    "}";
static const char CANCEL_H_DSL[] =
    "rule cancel_h {\n"
    "    match {\n"
    "        H 0\n"
    "        H 0\n"
    "    }\n"
    "    rewrite {}\n"
    "}";
static const char CANCEL_X_DSL[] =
    "rule cancel_x {\n"
    "    match {\n"
    "        X 0\n"
    "        X 0\n"
    "    }\n"
    "    rewrite {}\n"
    "}";
static const char CONDITION_DSL[] =
    "rule cancel_rz_inverse {\n"
    "    match { RZ(a) 0, RZ(b) 0 }\n"
    "    require { a + b == 0 mod 4*\xcf\x80 }\n"
    "    rewrite {}\n"
    "}";
static const char SHIFT_DSL[] =
    "rule shift_rz {\n"
    "    match { RZ(a) 0, RZ(b) 1 }\n"
    "    rewrite { RZ(a + b) 0, RZ(a + b) 1 }\n"
    "}";

static struct CKnowledgeRule* make_rule(const char* dsl) {
    struct CKnowledgeRule* rule = NULL;
    assert(knowledge_rule_from_dsl(dsl, &rule) == 0);
    assert(rule != NULL);
    return rule;
}

static struct COperation* make_op(const char* gate, const uint32_t* qubits, uintptr_t qubits_len,
                                  const double* params, uintptr_t params_len) {
    struct COperation* op = operation_new(gate, qubits, qubits_len, params, params_len);
    assert(op != NULL);
    return op;
}

static struct COperation* make_gate_op(const char* gate, uint32_t qubit) {
    return make_op(gate, &qubit, 1u, NULL, 0u);
}

static struct COperation* make_rz_op(uint32_t qubit, double angle) {
    return make_op("RZ", &qubit, 1u, &angle, 1u);
}

static int string_equals(char* ptr, const char* expected) {
    if (ptr == NULL) {
        return 0;
    }
    int equal = strcmp(ptr, expected) == 0;
    cqlib_string_free(ptr);
    return equal;
}

static void test_knowledge_bindings_lifecycle(void) {
    struct CKnowledgeBindings* bindings = knowledge_bindings_new();
    assert(bindings != NULL);
    knowledge_bindings_free(bindings);
    knowledge_bindings_free(NULL);
}

static void test_knowledge_rules_listing(void) {
    char source[512];
    snprintf(source, sizeof(source), "%s\n%s", CANCEL_H_DSL, CANCEL_X_DSL);

    struct CKnowledgeLibrary* library = NULL;
    assert(knowledge_library_from_dsl_str(source, RULE_KIND_CANCEL, &library) == 0);
    assert(library != NULL);

    assert(knowledge_rules_len(library) == 2u);
    uint32_t ids[2] = {99u, 99u};
    assert(knowledge_rules(library, ids, 2u) == 2u);
    assert(ids[0] == 0u && ids[1] == 1u);

    // Truncated buffer reports the full total but only writes the prefix.
    uint32_t truncated[1] = {99u};
    assert(knowledge_rules(library, truncated, 1u) == 2u);
    assert(truncated[0] == 0u);

    // NULL out buffer still reports the total; NULL library reports none.
    assert(knowledge_rules(library, NULL, 2u) == 2u);
    assert(knowledge_rules_len(NULL) == 0u);
    assert(knowledge_rules(NULL, ids, 2u) == 0u);

    assert(knowledge_library_len(library) == 2u);
    assert(knowledge_library_is_empty(library) == 0);
    assert(knowledge_library_contains(library, "cancel_h") == 1);
    assert(knowledge_library_contains(library, "missing") == 0);
    assert(knowledge_library_contains(NULL, "cancel_h") == -1);

    knowledge_library_free(library);
    knowledge_library_free(NULL);
}

static void test_knowledge_get_by_name(void) {
    char source[512];
    snprintf(source, sizeof(source), "%s\n%s", CANCEL_H_DSL, CANCEL_X_DSL);

    struct CKnowledgeLibrary* library = NULL;
    assert(knowledge_library_from_dsl_str(source, RULE_KIND_CANCEL, &library) == 0);

    struct CKnowledgeRule* rule = knowledge_get_by_name(library, "cancel_h");
    assert(rule != NULL);
    char* name = knowledge_rule_name(rule);
    int ok = name != NULL && strcmp(name, "cancel_h") == 0;
    cqlib_string_free(name);
    assert(ok);
    knowledge_rule_free(rule);

    assert(knowledge_get_by_name(library, "no_such_rule") == NULL);
    assert(knowledge_get_by_name(NULL, "cancel_h") == NULL);
    assert(knowledge_get_by_name(library, NULL) == NULL);

    knowledge_library_free(library);
}

static void test_knowledge_match_rule_item(void) {
    struct CKnowledgeRule* rule = make_rule(MERGE_DSL);
    struct COperation* op_a = make_rz_op(7u, 0.25);
    struct COperation* op_b = make_rz_op(7u, 0.5);
    struct COperation* op_conflict = make_rz_op(9u, 0.5);
    struct COperation* op_h = make_gate_op("H", 7u);

    struct CKnowledgeBindings* bindings = knowledge_bindings_new();
    assert(bindings != NULL);

    // Both pattern items bind against the same window, transactionally.
    assert(knowledge_match_rule_item(rule, 0u, op_a, bindings) == 1);
    assert(knowledge_match_rule_item(rule, 1u, op_b, bindings) == 1);

    // A conflicting qubit fails without corrupting existing bindings.
    assert(knowledge_match_rule_item(rule, 0u, op_conflict, bindings) == 0);
    // A different gate never matches the RZ pattern item.
    assert(knowledge_match_rule_item(rule, 0u, op_h, bindings) == 0);

    // The rolled-back item can be re-matched on the original qubit.
    assert(knowledge_match_rule_item(rule, 1u, op_b, bindings) == 1);

    // Error paths.
    assert(knowledge_match_rule_item(rule, 5u, op_a, bindings) == -2);
    assert(knowledge_match_rule_item(NULL, 0u, op_a, bindings) == -1);
    assert(knowledge_match_rule_item(rule, 0u, NULL, bindings) == -1);
    assert(knowledge_match_rule_item(rule, 0u, op_a, NULL) == -1);

    knowledge_bindings_free(bindings);
    operation_free(op_h);
    operation_free(op_conflict);
    operation_free(op_b);
    operation_free(op_a);
    knowledge_rule_free(rule);
}

static void test_knowledge_rule_matches_operations(void) {
    struct CKnowledgeRule* rule = make_rule(CANCEL_H_DSL);
    struct COperation* op_h1 = make_gate_op("H", 7u);
    struct COperation* op_h2 = make_gate_op("H", 7u);
    struct COperation* op_x = make_gate_op("X", 7u);

    const struct COperation* const adjacent[2] = {op_h1, op_h2};
    const struct COperation* const interrupted[2] = {op_h1, op_x};

    struct CKnowledgeBindings* bindings = NULL;
    assert(knowledge_rule_matches_operations(rule, adjacent, 2u, &bindings) == 1);
    assert(bindings != NULL);
    knowledge_bindings_free(bindings);

    struct CKnowledgeBindings* no_match = NULL;
    assert(knowledge_rule_matches_operations(rule, interrupted, 2u, &no_match) == 0);
    assert(no_match == NULL);

    // The operation count must equal the pattern length.
    assert(knowledge_rule_matches_operations(rule, adjacent, 1u, &bindings) == -8);
    assert(knowledge_rule_matches_operations(NULL, adjacent, 2u, &bindings) == -1);
    assert(knowledge_rule_matches_operations(rule, NULL, 2u, &bindings) == -1);
    assert(knowledge_rule_matches_operations(rule, adjacent, 2u, NULL) == -1);

    operation_free(op_x);
    operation_free(op_h2);
    operation_free(op_h1);
    knowledge_rule_free(rule);
}

static void test_knowledge_conditions_hold(void) {
    struct CKnowledgeRule* rule = make_rule(CONDITION_DSL);
    struct COperation* op_hold_a = make_rz_op(3u, 0.0);
    struct COperation* op_hold_b = make_rz_op(3u, 0.0);
    struct COperation* op_fail_a = make_rz_op(3u, 0.1);
    struct COperation* op_fail_b = make_rz_op(3u, 0.2);

    const struct COperation* const holding[2] = {op_hold_a, op_hold_b};
    const struct COperation* const failing[2] = {op_fail_a, op_fail_b};

    struct CKnowledgeBindings* bindings = NULL;
    assert(knowledge_rule_matches_operations(rule, holding, 2u, &bindings) == 1);
    assert(knowledge_conditions_hold(rule, bindings) == 1);
    knowledge_bindings_free(bindings);

    struct CKnowledgeBindings* failing_bindings = NULL;
    assert(knowledge_rule_matches_operations(rule, failing, 2u, &failing_bindings) == 1);
    assert(knowledge_conditions_hold(rule, failing_bindings) == 0);
    knowledge_bindings_free(failing_bindings);

    assert(knowledge_conditions_hold(NULL, NULL) == -1);

    operation_free(op_fail_b);
    operation_free(op_fail_a);
    operation_free(op_hold_b);
    operation_free(op_hold_a);
    knowledge_rule_free(rule);
}

static void test_knowledge_equivalent_to(void) {
    struct CKnowledgeRule* cancel_h_1 = make_rule(CANCEL_H_DSL);
    struct CKnowledgeRule* cancel_h_2 = make_rule(CANCEL_H_DSL);
    struct CKnowledgeRule* cancel_x = make_rule(CANCEL_X_DSL);
    struct CKnowledgeRule* merge = make_rule(MERGE_DSL);
    struct CKnowledgeRule* condition = make_rule(CONDITION_DSL);

    // Identical rules have equivalent items.
    assert(knowledge_equivalent_to(cancel_h_1, 0u, cancel_h_2, 0u) == 1);
    assert(knowledge_equivalent_to(cancel_h_1, 1u, cancel_h_2, 1u) == 1);
    // Different gates are not equivalent.
    assert(knowledge_equivalent_to(cancel_h_1, 0u, cancel_x, 0u) == 0);
    // RZ(a) in the merge rule matches RZ(a) in the condition rule.
    assert(knowledge_equivalent_to(merge, 0u, condition, 0u) == 1);

    // Out-of-bounds item indices and NULL guards.
    assert(knowledge_equivalent_to(cancel_h_1, 7u, cancel_h_2, 0u) == -2);
    assert(knowledge_equivalent_to(cancel_h_1, 0u, cancel_h_2, 7u) == -2);
    assert(knowledge_equivalent_to(NULL, 0u, cancel_h_2, 0u) == -1);

    knowledge_rule_free(condition);
    knowledge_rule_free(merge);
    knowledge_rule_free(cancel_x);
    knowledge_rule_free(cancel_h_2);
    knowledge_rule_free(cancel_h_1);
}

static void test_knowledge_candidates_for_first_instruction(void) {
    struct CKnowledgeLibrary* library = knowledge_library_builtin();
    assert(library != NULL);

    const uintptr_t total = knowledge_candidates_for_first_instruction_len(library, "H");
    assert(total >= 1u);

    uint32_t ids[64];
    assert(total <= 64u);
    assert(knowledge_candidates_for_first_instruction(library, "H", ids, total) == total);

    // cancel_h is a builtin rule starting with H.
    const int64_t cancel_h_id = knowledge_library_id_by_name(library, "cancel_h");
    assert(cancel_h_id >= 0);
    int found = 0;
    for (uintptr_t i = 0; i < total; i++) {
        if (ids[i] == (uint32_t)cancel_h_id) {
            found = 1;
        }
    }
    assert(found == 1);

    // Truncated copies still report the full total.
    uint32_t truncated[1] = {0u};
    assert(knowledge_candidates_for_first_instruction(library, "H", truncated, 1u) == total);
    assert(truncated[0] == ids[0]);

    // Unknown gate names and NULL inputs report no candidates.
    assert(knowledge_candidates_for_first_instruction_len(library, "NOPE") == 0u);
    assert(knowledge_candidates_for_first_instruction_len(NULL, "H") == 0u);
    assert(knowledge_candidates_for_first_instruction(library, NULL, NULL, 0u) == 0u);

    knowledge_library_free(library);
}

static void test_knowledge_collect_free_symbols(void) {
    struct CKnowledgeRule* merge = make_rule(MERGE_DSL);
    struct CKnowledgeRule* cancel = make_rule(CANCEL_H_DSL);

    assert(knowledge_collect_free_symbols_len(merge) == 2u);
    char* symbols[2] = {NULL, NULL};
    assert(knowledge_collect_free_symbols(merge, symbols, 2u) == 2u);
    int ok_a = string_equals(symbols[0], "a");
    int ok_b = string_equals(symbols[1], "b");
    assert(ok_a && ok_b);

    assert(knowledge_collect_free_symbols_len(cancel) == 0u);
    assert(knowledge_collect_free_symbols(cancel, NULL, 0u) == 0u);

    assert(knowledge_collect_free_symbols_len(NULL) == 0u);
    assert(knowledge_collect_free_symbols(NULL, NULL, 0u) == 0u);

    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

static void test_knowledge_filter_rule_ids_by_instruction_keys(void) {
    struct CKnowledgeLibrary* library = NULL;
    assert(knowledge_library_from_dsl_str(MERGE_DSL, RULE_KIND_MERGE, &library) == 0);

    const char* const op_names[1] = {"RZ"};
    const char* const target_names[1] = {"RZ"};

    assert(knowledge_filter_rule_ids_by_instruction_keys_len(library, op_names, 1u, target_names,
                                                             1u) == 1u);
    uint32_t ids[1] = {0u};
    assert(knowledge_filter_rule_ids_by_instruction_keys(library, op_names, 1u, target_names, 1u,
                                                         ids, 1u) == 1u);
    assert(ids[0] == 0u);

    // A match side that does not cover RZ filters everything out.
    const char* const h_names[1] = {"H"};
    assert(knowledge_filter_rule_ids_by_instruction_keys_len(library, h_names, 1u, target_names,
                                                             1u) == 0u);

    // Unknown gate names and NULL inputs report nothing.
    const char* const unknown_names[1] = {"NOPE"};
    assert(knowledge_filter_rule_ids_by_instruction_keys_len(library, unknown_names, 1u,
                                                             target_names, 1u) == 0u);
    assert(knowledge_filter_rule_ids_by_instruction_keys_len(NULL, op_names, 1u, target_names,
                                                             1u) == 0u);

    knowledge_library_free(library);
}

static void test_knowledge_extend_rules(void) {
    struct CKnowledgeRule* cancel_h = make_rule(CANCEL_H_DSL);
    struct CKnowledgeRule* cancel_x = make_rule(CANCEL_X_DSL);
    const struct CKnowledgeRule* const rules[2] = {cancel_h, cancel_x};

    struct CKnowledgeLibrary* library = knowledge_library_new();
    assert(library != NULL);

    uint32_t ids[2] = {0u, 0u};
    assert(knowledge_extend_rules(library, rules, 2u, RULE_KIND_CANCEL, ids, 2u) == 0);
    assert(ids[0] == 0u && ids[1] == 1u);
    assert(knowledge_library_len(library) == 2u);
    assert(knowledge_library_rule_kind(library, 0u) == (int32_t)RULE_KIND_CANCEL);

    // Duplicate names are rejected and the library stays unchanged.
    assert(knowledge_extend_rules(library, rules, 2u, RULE_KIND_CANCEL, ids, 2u) == -4);
    assert(knowledge_library_len(library) == 2u);

    // Unknown kind tags and small buffers are rejected before insertion.
    assert(knowledge_extend_rules(library, rules, 2u, 99u, ids, 2u) == -8);
    assert(knowledge_extend_rules(library, rules, 2u, RULE_KIND_CANCEL, ids, 1u) == -8);

    // NULL guards.
    assert(knowledge_extend_rules(NULL, rules, 2u, RULE_KIND_CANCEL, ids, 2u) == -1);
    assert(knowledge_extend_rules(library, NULL, 2u, RULE_KIND_CANCEL, ids, 2u) == -1);

    knowledge_library_free(library);
    knowledge_rule_free(cancel_x);
    knowledge_rule_free(cancel_h);
}

static void test_knowledge_target_qubits(void) {
    struct CKnowledgeRule* merge = make_rule(MERGE_DSL);
    struct CKnowledgeRule* cancel = make_rule(CANCEL_H_DSL);
    struct CKnowledgeRule* shift = make_rule(SHIFT_DSL);

    assert(knowledge_target_qubits_len(merge) == 1u);
    uint32_t qubits[1] = {0u};
    assert(knowledge_target_qubits(merge, qubits, 1u) == 1u);
    assert(qubits[0] == 0u);

    // The shift rule rewrites on rule-local labels 0 and 1.
    assert(knowledge_target_qubits_len(shift) == 2u);
    uint32_t shift_qubits[2] = {0u, 0u};
    assert(knowledge_target_qubits(shift, shift_qubits, 2u) == 2u);
    assert(shift_qubits[0] == 0u && shift_qubits[1] == 1u);

    // An empty rewrite target has no qubits.
    assert(knowledge_target_qubits_len(cancel) == 0u);
    assert(knowledge_target_qubits(cancel, NULL, 0u) == 0u);

    assert(knowledge_target_qubits_len(NULL) == 0u);
    assert(knowledge_target_qubits(NULL, NULL, 0u) == 0u);

    knowledge_rule_free(shift);
    knowledge_rule_free(cancel);
    knowledge_rule_free(merge);
}

static void test_knowledge_instantiate_target(void) {
    struct CKnowledgeRule* rule = make_rule(MERGE_DSL);
    struct COperation* op_a = make_rz_op(7u, 0.25);
    struct COperation* op_b = make_rz_op(7u, 0.5);

    // Match the full window, then instantiate the rewrite target.
    const struct COperation* const adjacent[2] = {op_a, op_b};
    struct CKnowledgeBindings* bindings = NULL;
    assert(knowledge_rule_matches_operations(rule, adjacent, 2u, &bindings) == 1);
    assert(bindings != NULL);

    struct CKnowledgeReplacements* replacements = NULL;
    assert(knowledge_instantiate_target(rule, bindings, &replacements) == 0);
    assert(replacements != NULL);
    assert(knowledge_replacements_len(replacements) == 1u);

    char* instruction = knowledge_replacement_instruction(replacements, 0u);
    int ok = instruction != NULL && strcmp(instruction, "RZ") == 0;
    cqlib_string_free(instruction);
    assert(ok);

    assert(knowledge_replacement_qubits_len(replacements, 0u) == 1u);
    uint32_t qubits[1] = {0u};
    assert(knowledge_replacement_qubits(replacements, 0u, qubits, 1u) == 1u);
    assert(qubits[0] == 7u);

    assert(knowledge_replacement_params_len(replacements, 0u) == 1u);
    double params[1] = {0.0};
    assert(knowledge_replacement_params(replacements, 0u, params, 1u) == 1u);
    const double diff = params[0] - 0.75;
    assert(diff > -1e-12 && diff < 1e-12);

    // Out-of-bounds replacement indices are benign.
    assert(knowledge_replacement_instruction(replacements, 3u) == NULL);
    assert(knowledge_replacement_qubits_len(replacements, 3u) == 0u);
    assert(knowledge_replacement_qubits(replacements, 3u, NULL, 0u) == 0u);
    assert(knowledge_replacement_params_len(replacements, 3u) == 0u);
    assert(knowledge_replacement_params(replacements, 3u, NULL, 0u) == 0u);
    knowledge_replacements_free(replacements);

    // Instantiating against empty bindings fails on the unbound qubit.
    struct CKnowledgeBindings* empty = knowledge_bindings_new();
    struct CKnowledgeReplacements* unbound = NULL;
    assert(knowledge_instantiate_target(rule, empty, &unbound) == -2);
    knowledge_bindings_free(empty);

    // NULL guards.
    assert(knowledge_instantiate_target(NULL, bindings, &unbound) == -1);
    assert(knowledge_instantiate_target(rule, NULL, &unbound) == -1);
    assert(knowledge_instantiate_target(rule, bindings, NULL) == -1);
    knowledge_bindings_free(bindings);

    // NULL-safe accessors and frees.
    assert(knowledge_replacements_len(NULL) == 0u);
    assert(knowledge_replacement_instruction(NULL, 0u) == NULL);
    knowledge_replacements_free(NULL);

    operation_free(op_b);
    operation_free(op_a);
    knowledge_rule_free(rule);
}

int main(void) {
    test_knowledge_bindings_lifecycle();
    test_knowledge_rules_listing();
    test_knowledge_get_by_name();
    test_knowledge_match_rule_item();
    test_knowledge_rule_matches_operations();
    test_knowledge_conditions_hold();
    test_knowledge_equivalent_to();
    test_knowledge_candidates_for_first_instruction();
    test_knowledge_collect_free_symbols();
    test_knowledge_filter_rule_ids_by_instruction_keys();
    test_knowledge_extend_rules();
    test_knowledge_target_qubits();
    test_knowledge_instantiate_target();
    printf("binding-c compile knowledge tests passed\n");
    return 0;
}
