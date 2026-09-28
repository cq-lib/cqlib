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

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static int g_failures = 0;

#define CHECK(cond, name)                 \
    do {                                  \
        if (cond) {                       \
            printf("ok: %s\n", (name));   \
        } else {                          \
            printf("FAIL: %s\n", (name)); \
            g_failures++;                 \
        }                                 \
    } while (0)

/* Loop-body callback appending one gate; `user_data` is ignored. */
static int32_t add_h(struct CCircuit* circuit, void* user_data) {
    (void)user_data;
    return circuit_h(circuit, 0);
}

static void test_cfg_from_circuit_roundtrip(void) {
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit != NULL, "cfg fixture circuit_new");
    CHECK(circuit_h(circuit, 0) == 0, "cfg fixture circuit_h");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "cfg fixture circuit_cx");
    CHECK(circuit_num_operations(circuit) == 2, "cfg fixture two ops");

    /* A straight-line circuit expands into a single entry block. */
    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    CHECK(cfg != NULL, "circuit_cfg_from_circuit");
    CHECK(circuit_cfg_validate(cfg) == 0, "circuit_cfg_validate");
    CHECK(circuit_cfg_num_qubits(cfg) == 2, "circuit_cfg_num_qubits");
    CHECK(circuit_cfg_num_blocks(cfg) == 1, "circuit_cfg_num_blocks");
    CHECK(circuit_cfg_entry_block(cfg) == 0, "circuit_cfg_entry_block");

    uint32_t ids[2] = {9, 9};
    CHECK(circuit_cfg_qubits(cfg, ids, 2) == 0, "circuit_cfg_qubits");
    CHECK(ids[0] == 0 && ids[1] == 1, "circuit_cfg_qubits values");

    char* entry_label = circuit_cfg_block_label(cfg, 0);
    CHECK(entry_label != NULL, "circuit_cfg_block_label");
    CHECK(entry_label != NULL && strcmp(entry_label, "entry") == 0, "entry block label value");
    cqlib_string_free(entry_label);

    /* Entry block: two ordinary operations plus a `Return` terminator. */
    CHECK(circuit_cfg_block_is_empty(cfg, 0) == 0, "entry block not empty");
    CHECK(circuit_cfg_block_operations_len(cfg, 0) == 2, "entry ops len");
    CHECK(circuit_cfg_block_has_terminator(cfg, 0) == 1, "entry has terminator");
    CHECK(circuit_cfg_block_terminator_tag(cfg, 0) == (int32_t)CFG_TERMINATOR_RETURN,
          "entry terminator tag");
    CHECK(circuit_cfg_terminator_target(cfg, 0) == UINT32_MAX, "return has no target");
    CHECK(circuit_cfg_terminator_condition(cfg, 0) == NULL, "return has no condition");

    /* Cloned operation handles are independent of the cfg and of each other. */
    struct COperation* cloned[2] = {NULL, NULL};
    CHECK(circuit_cfg_block_operations(cfg, 0, cloned, 2) == 0, "block operations copy");
    CHECK(cloned[0] != NULL && cloned[1] != NULL, "cloned operations non-null");
    operation_free(cloned[0]);
    operation_free(cloned[1]);

    /* An undersized buffer is rejected before any write. */
    CHECK(circuit_cfg_block_operations(cfg, 0, cloned, 1) == -8, "block operations short buffer");

    /* Straight-line circuits carry no classical state. */
    CHECK(circuit_cfg_classical_vars_len(cfg) == 0, "no classical vars");
    CHECK(circuit_cfg_classical_values_len(cfg) == 0, "no classical values");

    /* Round trip: qubit count and operation count are preserved. The source
     * circuit stays alive and unchanged. */
    struct CCircuit* lowered = circuit_cfg_to_circuit(cfg);
    CHECK(lowered != NULL, "circuit_cfg_to_circuit");
    CHECK(circuit_num_qubits(lowered) == 2, "lowered num_qubits");
    CHECK(circuit_num_operations(lowered) == 2, "lowered num_operations");
    CHECK(circuit_num_operations(circuit) == 2, "source circuit unchanged");

    circuit_free(lowered);
    circuit_cfg_free(cfg);
    circuit_free(circuit);
}

static void test_cfg_layout_and_blocks(void) {
    const uint32_t zero[1] = {0};
    struct COperation* op = operation_new("H", zero, 1, NULL, 0);
    CHECK(op != NULL, "cfg layout fixture");

    /* A fresh CFG is empty and has no designated entry. */
    struct CCircuitCFG* cfg = circuit_cfg_new(2);
    CHECK(cfg != NULL, "circuit_cfg_new");
    CHECK(circuit_cfg_num_qubits(cfg) == 2, "fresh num_qubits");
    CHECK(circuit_cfg_num_blocks(cfg) == 0, "fresh num_blocks");
    CHECK(circuit_cfg_entry_block(cfg) == UINT32_MAX, "fresh entry sentinel");
    CHECK(circuit_cfg_blocks(cfg, NULL, 0) == 0, "fresh blocks copy");

    /* Blocks are appended with stable sequential indices. */
    uint32_t b0 = circuit_cfg_add_block(cfg);
    uint32_t b1 = circuit_cfg_add_block(cfg);
    CHECK(b0 == 0 && b1 == 1, "add_block indices");
    CHECK(circuit_cfg_num_blocks(cfg) == 2, "num_blocks 2");
    uint32_t b2 = circuit_cfg_add_block(cfg);
    CHECK(b2 == 2, "third block index");
    CHECK(circuit_cfg_num_blocks(cfg) == 3, "num_blocks 3");

    /* Block enumeration into the caller-supplied buffer. */
    uint32_t nodes[3] = {9, 9, 9};
    CHECK(circuit_cfg_blocks(cfg, nodes, 3) == 0, "blocks copy");
    CHECK(nodes[0] == 0 && nodes[1] == 1 && nodes[2] == 2, "blocks values");
    CHECK(circuit_cfg_blocks(cfg, nodes, 2) == -8, "blocks short buffer");
    CHECK(circuit_cfg_blocks(cfg, NULL, 3) == -1, "blocks null buffer");

    /* Entry switching accepts any non-sentinel index (validate checks it). */
    CHECK(circuit_cfg_set_entry_block(cfg, b1) == 0, "set_entry_block");
    CHECK(circuit_cfg_entry_block(cfg) == b1, "entry_block readback");
    CHECK(circuit_cfg_set_entry_block(cfg, UINT32_MAX) == -2, "set_entry_block sentinel");

    /* Fresh blocks are empty and unlabeled. */
    CHECK(circuit_cfg_block_is_empty(cfg, b2) == 1, "fresh block empty");
    CHECK(circuit_cfg_block_label(cfg, b2) == NULL, "fresh block unlabeled");
    CHECK(circuit_cfg_block_operations_len(cfg, b2) == 0, "fresh block ops len");
    CHECK(circuit_cfg_block_operations(cfg, b2, NULL, 0) == 0, "fresh block operations copy");

    /* Semantic edges between existing blocks; flow tags are checked first. */
    CHECK(circuit_cfg_add_edge(cfg, b0, b1, CFG_FLOW_UNCONDITIONAL, 0, 0) == 0, "add_edge");
    CHECK(circuit_cfg_add_edge(cfg, b0, 3, CFG_FLOW_UNCONDITIONAL, 0, 0) == -2,
          "add_edge missing target");
    CHECK(circuit_cfg_add_edge(cfg, UINT32_MAX, b1, CFG_FLOW_UNCONDITIONAL, 0, 0) == -2,
          "add_edge sentinel source");
    CHECK(circuit_cfg_add_edge(cfg, b0, 3, 99, 0, 0) == -8, "add_edge bad flow");
    CHECK(circuit_cfg_add_edge(cfg, b0, b1, 99, 0, 0) == -8, "add_edge bad flow 2");

    uint32_t targets[1] = {9};
    uint32_t tags[1] = {9};
    uint64_t lo[1] = {9};
    uint64_t hi[1] = {9};
    CHECK(circuit_cfg_outgoing_edges_len(cfg, b0) == 1, "outgoing_edges_len");
    CHECK(circuit_cfg_outgoing_edges(cfg, b0, targets, tags, lo, hi, 1) == 0, "outgoing_edges");
    CHECK(targets[0] == b1, "edge target");
    CHECK(tags[0] == CFG_FLOW_UNCONDITIONAL, "edge flow tag");
    CHECK(lo[0] == 0 && hi[0] == 0, "edge case halves");
    /* Undersized then NULL parallel buffers. */
    CHECK(circuit_cfg_outgoing_edges(cfg, b0, targets, tags, lo, hi, 0) == -8,
          "outgoing_edges short buffer");
    CHECK(circuit_cfg_outgoing_edges(cfg, b0, NULL, tags, lo, hi, 1) == -1,
          "outgoing_edges null buffer");

    /* Pushing one operation flips `block_is_empty` and grows the count. */
    CHECK(circuit_cfg_push_operation(cfg, b2, op) == 0, "push_operation");
    CHECK(circuit_cfg_block_is_empty(cfg, b2) == 0, "block no longer empty");
    CHECK(circuit_cfg_block_operations_len(cfg, b2) == 1, "block ops len 1");
    struct COperation* single[1] = {NULL};
    CHECK(circuit_cfg_block_operations(cfg, b2, single, 1) == 0, "block operations clone");
    CHECK(single[0] != NULL, "cloned op non-null");
    operation_free(single[0]);
    CHECK(circuit_cfg_block_operations(cfg, b2, single, 0) == -8, "block operations short clone");
    CHECK(circuit_cfg_block_operations(cfg, b2, NULL, 1) == -1, "block operations null clone");

    /* Unknown blocks are rejected by mutation entry points. */
    CHECK(circuit_cfg_push_operation(cfg, 7, op) == -2, "push unknown block");
    CHECK(circuit_cfg_push_operation(cfg, UINT32_MAX, op) == -2, "push sentinel block");
    CHECK(circuit_cfg_push_operation(cfg, b2, NULL) == -1, "push null op");

    circuit_cfg_free(cfg);
    operation_free(op);
}

static void test_cfg_terminators(void) {
    struct CCircuitCFG* cfg = circuit_cfg_new(1);
    uint32_t b0 = circuit_cfg_add_block(cfg);
    uint32_t b1 = circuit_cfg_add_block(cfg);
    uint32_t b2 = circuit_cfg_add_block(cfg);

    /* Setters succeed and the getter round-trips each terminator flavor. */
    CHECK(circuit_cfg_set_terminator_jump(cfg, b0, b1) == 0, "set jump");
    CHECK(circuit_cfg_set_terminator_break(cfg, b1, b0) == 0, "set break");
    CHECK(circuit_cfg_set_terminator_continue(cfg, b2, b0) == 0, "set continue");

    CHECK(circuit_cfg_block_has_terminator(cfg, b0) == 1, "b0 has terminator");
    CHECK(circuit_cfg_block_terminator_tag(cfg, b0) == (int32_t)CFG_TERMINATOR_JUMP, "b0 jump tag");
    CHECK(circuit_cfg_terminator_target(cfg, b0) == b1, "jump target");
    CHECK(circuit_cfg_block_terminator_tag(cfg, b1) == (int32_t)CFG_TERMINATOR_BREAK,
          "b1 break tag");
    CHECK(circuit_cfg_terminator_target(cfg, b1) == b0, "break target");
    CHECK(circuit_cfg_block_terminator_tag(cfg, b2) == (int32_t)CFG_TERMINATOR_CONTINUE,
          "b2 continue tag");
    CHECK(circuit_cfg_terminator_target(cfg, b2) == b0, "continue target");

    /* A terminator without operations still makes the block non-empty. */
    CHECK(circuit_cfg_block_is_empty(cfg, b0) == 0, "terminator makes block non-empty");
    CHECK(circuit_cfg_block_operations_len(cfg, b0) == 0, "terminator block no ops");

    /* Jump/Break/Continue carry no condition and no loop info. */
    CHECK(circuit_cfg_terminator_condition(cfg, b0) == NULL, "jump no condition");
    struct CClassicalVar* var = NULL;
    struct CClassicalExpr* start = NULL;
    struct CClassicalExpr* stop = NULL;
    struct CClassicalExpr* step = NULL;
    CHECK(circuit_cfg_terminator_for_info(cfg, b0, &var, &start, &stop, &step) == -8,
          "jump no for info");

    /* `Return` has no target and overwrites the previous terminator. */
    CHECK(circuit_cfg_set_terminator_return(cfg, b2) == 0, "set return");
    CHECK(circuit_cfg_block_terminator_tag(cfg, b2) == (int32_t)CFG_TERMINATOR_RETURN,
          "return tag");
    CHECK(circuit_cfg_terminator_target(cfg, b2) == UINT32_MAX, "return no target");

    /* NULL and unknown blocks. */
    CHECK(circuit_cfg_set_terminator_jump(NULL, 0, 0) == -1, "jump null cfg");
    CHECK(circuit_cfg_set_terminator_return(NULL, 0) == -1, "return null cfg");
    CHECK(circuit_cfg_set_terminator_break(NULL, 0, 0) == -1, "break null cfg");
    CHECK(circuit_cfg_set_terminator_continue(NULL, 0, 0) == -1, "continue null cfg");
    CHECK(circuit_cfg_set_terminator_jump(cfg, 7, 0) == -2, "jump unknown block");
    CHECK(circuit_cfg_set_terminator_jump(cfg, UINT32_MAX, 0) == -2, "jump sentinel block");

    circuit_cfg_free(cfg);
}

static void test_cfg_classical_state(void) {
    /* Circuit carrying two variables plus one measured value. */
    struct CCircuit* circuit = circuit_new(2);
    struct CClassicalVar* bit_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    struct CClassicalVar* uint_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    CHECK(bit_var != NULL, "bit var");
    CHECK(uint_var != NULL, "uint var");

    const uint32_t measured[2] = {0, 1};
    struct CClassicalExpr* measured_expr = circuit_measure_bits(circuit, measured, 2);
    CHECK(measured_expr != NULL, "measure expr");
    CHECK(circuit_classical_vars_len(circuit) == 2, "circuit vars len");

    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    CHECK(cfg != NULL, "cfg from classical circuit");

    /* The CFG snapshots the circuit's static classical tables. */
    CHECK(circuit_cfg_classical_vars_len(cfg) == 2, "cfg vars len");
    CHECK(circuit_cfg_classical_values_len(cfg) == 1, "cfg values len");

    CClassicalType vars[2] = {{9, 9}, {9, 9}};
    CHECK(circuit_cfg_classical_vars(cfg, vars, 2) == 0, "vars copy");
    /* Compare per field: the repr(C) snapshot has no comparison support. */
    CHECK(vars[0].tag == CQLIB_CLASSICAL_TYPE_BIT && vars[0].width == 1, "bit var snapshot");
    CHECK(vars[1].tag == CQLIB_CLASSICAL_TYPE_UINT && vars[1].width == 8, "uint var snapshot");

    CClassicalType values[1] = {{9, 9}};
    CHECK(circuit_cfg_classical_values(cfg, values, 1) == 0, "values copy");
    CHECK(values[0].tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && values[0].width == 2, "value snapshot");

    /* Undersized then NULL buffers. */
    CHECK(circuit_cfg_classical_vars(cfg, vars, 1) == -8, "vars short buffer");
    CHECK(circuit_cfg_classical_vars(cfg, NULL, 2) == -1, "vars null buffer");
    CHECK(circuit_cfg_classical_values(cfg, values, 0) == -8, "values short buffer");
    CHECK(circuit_cfg_classical_values(cfg, NULL, 1) == -1, "values null buffer");

    classical_expr_free(measured_expr);
    classical_var_free(uint_var);
    classical_var_free(bit_var);
    circuit_cfg_free(cfg);

    /* A circuit without classical state produces empty CFG tables. */
    struct CCircuit* plain = circuit_new(1);
    struct CCircuitCFG* plain_cfg = circuit_cfg_from_circuit(plain);
    CHECK(plain_cfg != NULL, "plain cfg");
    CHECK(circuit_cfg_classical_vars_len(plain_cfg) == 0, "plain vars len");
    CHECK(circuit_cfg_classical_values_len(plain_cfg) == 0, "plain values len");
    CHECK(circuit_cfg_classical_vars(plain_cfg, NULL, 0) == 0, "plain vars copy");
    CHECK(circuit_cfg_classical_values(plain_cfg, NULL, 0) == 0, "plain values copy");
    circuit_cfg_free(plain_cfg);
    circuit_free(plain);

    /* A hand-built CFG starts with empty tables as well. */
    struct CCircuitCFG* empty = circuit_cfg_new(0);
    CHECK(circuit_cfg_classical_vars_len(empty) == 0, "hand-built vars len");
    CHECK(circuit_cfg_classical_values_len(empty) == 0, "hand-built values len");
    CHECK(circuit_cfg_classical_vars(empty, NULL, 0) == 0, "hand-built vars copy");
    CHECK(circuit_cfg_classical_values(empty, NULL, 0) == 0, "hand-built values copy");
    circuit_cfg_free(empty);

    circuit_free(circuit);
}

static void test_cfg_loop_regions(void) {
    /* `while (flag) { h(0); }` expands into entry / cond / body / exit. */
    struct CCircuit* circuit = circuit_new(2);
    struct CClassicalVar* condition_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    CHECK(condition_var != NULL, "while condition var");
    struct CClassicalExpr* condition = classical_expr_var(condition_var);
    CHECK(condition != NULL, "while condition expr");
    CHECK(circuit_while(circuit, condition, add_h, NULL) == 0, "circuit_while");
    classical_expr_free(condition);
    classical_var_free(condition_var);

    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    CHECK(cfg != NULL, "while cfg");
    CHECK(circuit_cfg_validate(cfg) == 0, "while cfg validate");
    CHECK(circuit_cfg_num_blocks(cfg) == 4, "while blocks");
    CHECK(circuit_cfg_entry_block(cfg) == 0, "while entry");

    /* Entry jumps to the condition header, which owns the While region. */
    CHECK(circuit_cfg_block_terminator_tag(cfg, 0) == (int32_t)CFG_TERMINATOR_JUMP, "entry jump");
    CHECK(circuit_cfg_terminator_target(cfg, 0) == 1, "entry target");
    CHECK(circuit_cfg_region_tag(cfg, 1) == (int32_t)CFG_REGION_WHILE, "cond region tag");
    CHECK(circuit_cfg_is_loop_header(cfg, 1) == 1, "cond is loop header");
    CHECK(circuit_cfg_region_tag(cfg, 0) == 0, "entry no region");
    CHECK(circuit_cfg_is_loop_header(cfg, 0) == 0, "entry not loop header");
    CHECK(circuit_cfg_block_terminator_tag(cfg, 1) == (int32_t)CFG_TERMINATOR_BRANCH,
          "cond branch tag");
    CHECK(circuit_cfg_terminator_target(cfg, 1) == UINT32_MAX, "branch no target");

    /* Structured labels are attached for diagnostics. */
    char* body_label = circuit_cfg_block_label(cfg, 2);
    CHECK(body_label != NULL, "body label");
    CHECK(body_label != NULL && strncmp(body_label, "while_body_", 11) == 0, "body label prefix");
    cqlib_string_free(body_label);

    /* The Branch condition is exposed as a fresh expression handle. */
    struct CClassicalExpr* branch_condition = circuit_cfg_terminator_condition(cfg, 1);
    CHECK(branch_condition != NULL, "branch condition");
    classical_expr_free(branch_condition);

    /* The loop layout reads back body and exit blocks. */
    uint32_t body_block = 9;
    uint32_t exit_block = 9;
    CHECK(circuit_cfg_region_loop(cfg, 1, &body_block, &exit_block) == 0, "while region loop");
    CHECK(body_block == 2 && exit_block == 3, "while body/exit");
    /* Non-loop blocks have no region and are rejected. */
    CHECK(circuit_cfg_region_loop(cfg, 0, &body_block, &exit_block) == -8, "non-loop region_loop");
    CHECK(circuit_cfg_terminator_for_info(cfg, 1, NULL, NULL, NULL, NULL) == -8, "branch for_info");

    /* Header edges: TrueBranch into the body, FalseBranch into the exit. */
    uint32_t edge_targets[2] = {9, 9};
    uint32_t flow_tags[2] = {9, 9};
    uint64_t edge_lo[2] = {9, 9};
    uint64_t edge_hi[2] = {9, 9};
    CHECK(circuit_cfg_outgoing_edges_len(cfg, 1) == 2, "cond edges len");
    CHECK(circuit_cfg_outgoing_edges(cfg, 1, edge_targets, flow_tags, edge_lo, edge_hi, 2) == 0,
          "cond edges");
    CHECK(edge_targets[0] == 2 && edge_targets[1] == 3, "cond edge targets");
    CHECK(flow_tags[0] == CFG_FLOW_TRUE_BRANCH && flow_tags[1] == CFG_FLOW_FALSE_BRANCH,
          "cond flow tags");

    /* The body jumps back to the header; the exit returns. */
    CHECK(circuit_cfg_block_operations_len(cfg, 2) == 1, "body ops len");
    CHECK(circuit_cfg_terminator_target(cfg, 2) == 1, "body jumps back");
    CHECK(circuit_cfg_block_terminator_tag(cfg, 3) == (int32_t)CFG_TERMINATOR_RETURN,
          "exit return");

    circuit_cfg_free(cfg);
    circuit_free(circuit);

    /* `for` loops expose their loop variable through `terminator_for_info`. */
    struct CCircuit* for_circuit = circuit_new(1);
    struct CClassicalVar* counter = circuit_var(for_circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    CHECK(counter != NULL, "for counter var");
    struct CClassicalExpr* start = classical_expr_uint_literal(8, 0, 0);
    struct CClassicalExpr* stop = classical_expr_uint_literal(8, 4, 0);
    struct CClassicalExpr* step = classical_expr_uint_literal(8, 1, 0);
    CHECK(start != NULL && stop != NULL && step != NULL, "for literals");
    CHECK(circuit_for_uint(for_circuit, counter, start, stop, step, add_h, NULL) == 0,
          "circuit_for_uint");

    struct CCircuitCFG* for_cfg = circuit_cfg_from_circuit(for_circuit);
    CHECK(for_cfg != NULL, "for cfg");
    CHECK(circuit_cfg_num_blocks(for_cfg) == 4, "for blocks");
    CHECK(circuit_cfg_block_terminator_tag(for_cfg, 1) == (int32_t)CFG_TERMINATOR_FOR_LOOP,
          "for terminator");
    CHECK(circuit_cfg_region_tag(for_cfg, 1) == (int32_t)CFG_REGION_FOR, "for region tag");
    CHECK(circuit_cfg_is_loop_header(for_cfg, 1) == 1, "for is loop header");
    CHECK(circuit_cfg_region_loop(for_cfg, 1, &body_block, &exit_block) == 0, "for region loop");
    CHECK(body_block == 2 && exit_block == 3, "for body/exit");

    struct CClassicalVar* var = NULL;
    struct CClassicalExpr* for_start = NULL;
    struct CClassicalExpr* for_stop = NULL;
    struct CClassicalExpr* for_step = NULL;
    CHECK(circuit_cfg_terminator_for_info(for_cfg, 1, &var, &for_start, &for_stop, &for_step) == 0,
          "for_info");
    CHECK(var != NULL && for_start != NULL && for_stop != NULL && for_step != NULL,
          "for_info handles");
    classical_var_free(var);
    classical_expr_free(for_step);
    classical_expr_free(for_stop);
    classical_expr_free(for_start);

    /* NULL out-parameters on a present ForLoop header fail before writing. */
    var = NULL;
    for_start = NULL;
    for_stop = NULL;
    for_step = NULL;
    CHECK(circuit_cfg_terminator_for_info(for_cfg, 1, NULL, &for_start, &for_stop, &for_step) == -1,
          "for_info null out-param");
    /* Non-loop blocks and unknown blocks report their failure mode. */
    CHECK(circuit_cfg_terminator_for_info(for_cfg, 0, &var, &for_start, &for_stop, &for_step) == -8,
          "for_info non-loop");
    CHECK(circuit_cfg_terminator_for_info(for_cfg, UINT32_MAX, &var, &for_start, &for_stop,
                                          &for_step) == -2,
          "for_info unknown block");

    circuit_cfg_free(for_cfg);
    classical_expr_free(step);
    classical_expr_free(stop);
    classical_expr_free(start);
    classical_var_free(counter);
    circuit_free(for_circuit);

    /* Region setters: manual attachment and replacement on a bare CFG. */
    struct CCircuitCFG* manual = circuit_cfg_new(1);
    uint32_t header = circuit_cfg_add_block(manual);
    uint32_t body_b = circuit_cfg_add_block(manual);
    uint32_t exit_b = circuit_cfg_add_block(manual);
    uint32_t merge_b = circuit_cfg_add_block(manual);

    CHECK(circuit_cfg_set_while_region(manual, header, body_b, exit_b, NULL) == 0,
          "set_while_region");
    CHECK(circuit_cfg_region_tag(manual, header) == (int32_t)CFG_REGION_WHILE, "while region tag");
    CHECK(circuit_cfg_is_loop_header(manual, header) == 1, "while header");

    /* Replacing the region with a For region keeps it a loop header. */
    CHECK(circuit_cfg_set_for_region(manual, header, body_b, exit_b, NULL) == 0, "set_for_region");
    CHECK(circuit_cfg_region_tag(manual, header) == (int32_t)CFG_REGION_FOR, "for region tag");
    CHECK(circuit_cfg_is_loop_header(manual, header) == 1, "for header");
    CHECK(circuit_cfg_region_loop(manual, header, &body_block, &exit_block) == 0,
          "for region loop readback");
    CHECK(body_block == body_b && exit_block == exit_b, "for region values");

    /* An If region is not a loop header and carries the four-way layout. */
    CHECK(circuit_cfg_set_if_region(manual, header, body_b, exit_b, merge_b, 0, NULL) == 0,
          "set_if_region");
    CHECK(circuit_cfg_region_tag(manual, header) == (int32_t)CFG_REGION_IF, "if region tag");
    CHECK(circuit_cfg_is_loop_header(manual, header) == 0, "if not loop header");
    uint32_t then_entry = 9;
    uint32_t else_entry = 9;
    uint32_t merge_block = 9;
    int32_t has_else = 9;
    CHECK(circuit_cfg_region_if(manual, header, &then_entry, &else_entry, &merge_block,
                                &has_else) == 0,
          "region_if");
    CHECK(then_entry == body_b && else_entry == exit_b && merge_block == merge_b, "if layout");
    CHECK(has_else == 0, "if has_else");
    CHECK(circuit_cfg_region_loop(manual, header, &body_block, &exit_block) == -8,
          "if region not loop");

    /* A Switch region enumerates its match values and entries. */
    const uint64_t case_lo[1] = {7};
    const uint64_t case_hi[1] = {0};
    const uint32_t case_entries[1] = {body_b};
    CHECK(circuit_cfg_set_switch_region(manual, header, case_lo, case_hi, case_entries, 1, exit_b,
                                        merge_b, 1, NULL) == 0,
          "set_switch_region");
    CHECK(circuit_cfg_region_tag(manual, header) == (int32_t)CFG_REGION_SWITCH,
          "switch region tag");
    CHECK(circuit_cfg_is_loop_header(manual, header) == 0, "switch not header");
    CHECK(circuit_cfg_region_switch_cases_len(manual, header) == 1, "switch cases len");
    uint64_t out_lo[1] = {9};
    uint64_t out_hi[1] = {9};
    uint32_t out_entries[1] = {9};
    CHECK(circuit_cfg_region_switch(manual, header, out_lo, out_hi, out_entries, 1, &exit_block,
                                    &merge_block, &has_else) == 0,
          "region_switch");
    CHECK(out_lo[0] == 7 && out_hi[0] == 0, "switch case value halves");
    CHECK(out_entries[0] == body_b, "switch case entry");
    CHECK(exit_block == exit_b && merge_block == merge_b && has_else == 1,
          "switch default/merge/has_default");

    /* Setter failure modes: NULL handles, sentinel, and unknown headers. */
    CHECK(circuit_cfg_set_while_region(NULL, 0, 0, 0, NULL) == -1, "set_while null");
    CHECK(circuit_cfg_set_if_region(NULL, 0, 0, 0, 0, 0, NULL) == -1, "set_if null");
    CHECK(circuit_cfg_set_for_region(NULL, 0, 0, 0, NULL) == -1, "set_for null");
    CHECK(circuit_cfg_set_switch_region(NULL, 0, NULL, NULL, NULL, 0, 0, 0, 0, NULL) == -1,
          "set_switch null");
    CHECK(circuit_cfg_set_switch_region(manual, header, NULL, NULL, NULL, 1, exit_b, merge_b, 1,
                                        NULL) == -1,
          "set_switch null arrays");
    CHECK(circuit_cfg_set_while_region(manual, 7, 0, 0, NULL) == -2, "set_while unknown header");
    CHECK(circuit_cfg_set_while_region(manual, UINT32_MAX, 0, 0, NULL) == -2,
          "set_while sentinel header");

    /* Hand-attached regions point at real blocks everywhere. */
    const uint32_t region_targets[1] = {0};
    CHECK(circuit_cfg_set_switch_region(manual, header, case_lo, case_hi, region_targets, 1, exit_b,
                                        merge_b, 1, NULL) == 0,
          "set_switch existing targets");
    CHECK(circuit_cfg_region_switch_cases_len(manual, header) == 1,
          "switch cases len after reattach");

    circuit_cfg_free(manual);
}

static void test_cfg_null_and_error_paths(void) {
    const uint32_t zero[1] = {0};
    struct COperation* op = operation_new("H", zero, 1, NULL, 0);
    CHECK(op != NULL, "null-path fixture");
    struct CClassicalVar* var = NULL;
    struct CClassicalExpr* start = NULL;
    struct CClassicalExpr* stop = NULL;
    struct CClassicalExpr* step = NULL;
    uint32_t entry = 9;

    /* Every exported function tolerates a NULL handle. */
    CHECK(circuit_cfg_from_circuit(NULL) == NULL, "from_circuit null");
    CHECK(circuit_cfg_from_qubits(NULL, 1) == NULL, "from_qubits null buffer");
    CHECK(circuit_cfg_to_circuit(NULL) == NULL, "to_circuit null");
    CHECK(circuit_cfg_validate(NULL) == -1, "validate null");
    CHECK(circuit_cfg_num_qubits(NULL) == 0, "num_qubits null");
    CHECK(circuit_cfg_num_blocks(NULL) == 0, "num_blocks null");
    CHECK(circuit_cfg_qubits(NULL, NULL, 0) == -1, "qubits null");
    CHECK(circuit_cfg_classical_vars_len(NULL) == 0, "vars_len null");
    CHECK(circuit_cfg_classical_values_len(NULL) == 0, "values_len null");
    CHECK(circuit_cfg_classical_vars(NULL, NULL, 0) == -1, "vars null");
    CHECK(circuit_cfg_classical_values(NULL, NULL, 0) == -1, "values null");
    CHECK(circuit_cfg_entry_block(NULL) == UINT32_MAX, "entry_block null");
    CHECK(circuit_cfg_add_block(NULL) == UINT32_MAX, "add_block null");
    CHECK(circuit_cfg_add_edge(NULL, 0, 0, CFG_FLOW_UNCONDITIONAL, 0, 0) == -1, "add_edge null");
    CHECK(circuit_cfg_set_entry_block(NULL, 0) == -1, "set_entry null");
    CHECK(circuit_cfg_push_operation(NULL, 0, op) == -1, "push null cfg");
    CHECK(circuit_cfg_blocks(NULL, NULL, 0) == -1, "blocks null");
    CHECK(circuit_cfg_block_label(NULL, 0) == NULL, "block_label null");
    CHECK(circuit_cfg_block_is_empty(NULL, 0) == -1, "block_is_empty null");
    CHECK(circuit_cfg_block_operations_len(NULL, 0) == 0, "block_operations_len null");
    CHECK(circuit_cfg_block_operations(NULL, 0, NULL, 0) == -1, "block_operations null");
    CHECK(circuit_cfg_block_has_terminator(NULL, 0) == -1, "has_terminator null");
    CHECK(circuit_cfg_block_terminator_tag(NULL, 0) == -1, "terminator_tag null");
    CHECK(circuit_cfg_terminator_target(NULL, 0) == UINT32_MAX, "terminator_target null");
    CHECK(circuit_cfg_terminator_condition(NULL, 0) == NULL, "terminator_condition null");
    CHECK(circuit_cfg_terminator_for_info(NULL, 0, &var, &start, &stop, &step) == -1,
          "for_info null");
    CHECK(circuit_cfg_outgoing_edges_len(NULL, 0) == 0, "edges_len null");
    CHECK(circuit_cfg_outgoing_edges(NULL, 0, NULL, NULL, NULL, NULL, 0) == -1, "edges null");
    CHECK(circuit_cfg_region_tag(NULL, 0) == -1, "region_tag null");
    CHECK(circuit_cfg_is_loop_header(NULL, 0) == -1, "is_loop_header null");
    CHECK(circuit_cfg_region_if(NULL, 0, &entry, &entry, &entry, NULL) == -1, "region_if null");
    CHECK(circuit_cfg_region_loop(NULL, 0, &entry, &entry) == -1, "region_loop null");
    CHECK(circuit_cfg_region_switch_cases_len(NULL, 0) == 0, "cases_len null");
    CHECK(circuit_cfg_region_switch(NULL, 0, NULL, NULL, NULL, 0, &entry, &entry, NULL) == -1,
          "region_switch null");
    CHECK(circuit_cfg_set_terminator_jump(NULL, 0, 0) == -1, "set jump null");
    CHECK(circuit_cfg_set_terminator_return(NULL, 0) == -1, "set return null");
    circuit_cfg_free(NULL);
    CHECK(1, "cfg_free null");

    /* Out-of-range and sentinel node ids fail every block-level query. */
    struct CCircuitCFG* cfg = circuit_cfg_new(1);
    CHECK(circuit_cfg_add_block(cfg) == 0, "single block fixture");
    const uint32_t invalid[2] = {7, UINT32_MAX};
    for (int i = 0; i < 2; i++) {
        uint32_t bad = invalid[i];
        CHECK(circuit_cfg_block_is_empty(cfg, bad) == -2, "is_empty bad node");
        CHECK(circuit_cfg_block_label(cfg, bad) == NULL, "label bad node");
        CHECK(circuit_cfg_block_operations_len(cfg, bad) == 0, "ops len bad node");
        CHECK(circuit_cfg_block_has_terminator(cfg, bad) == -2, "has_terminator bad node");
        CHECK(circuit_cfg_block_terminator_tag(cfg, bad) == -2, "tag bad node");
        CHECK(circuit_cfg_terminator_target(cfg, bad) == UINT32_MAX, "target bad node");
        CHECK(circuit_cfg_terminator_condition(cfg, bad) == NULL, "condition bad node");
        CHECK(circuit_cfg_block_operations(cfg, bad, NULL, 1) == -2, "block ops bad node");
        CHECK(circuit_cfg_outgoing_edges_len(cfg, bad) == 0, "edges len bad node");
        CHECK(circuit_cfg_outgoing_edges(cfg, bad, NULL, NULL, NULL, NULL, 1) == -2,
              "edges bad node");
        CHECK(circuit_cfg_region_tag(cfg, bad) == -2, "region tag bad node");
        CHECK(circuit_cfg_is_loop_header(cfg, bad) == -2, "loop header bad node");
        CHECK(circuit_cfg_region_if(cfg, bad, &entry, &entry, &entry, NULL) == -2,
              "region_if bad node");
        CHECK(circuit_cfg_region_loop(cfg, bad, &entry, &entry) == -2, "region_loop bad node");
        CHECK(circuit_cfg_region_switch_cases_len(cfg, bad) == 0, "cases len bad node");
        CHECK(circuit_cfg_region_switch(cfg, bad, NULL, NULL, NULL, 0, &entry, &entry, NULL) == -2,
              "region_switch bad node");
        CHECK(circuit_cfg_push_operation(cfg, bad, op) == -2, "push bad node");
        CHECK(circuit_cfg_set_terminator_return(cfg, bad) == -2, "set return bad node");
        CHECK(circuit_cfg_set_while_region(cfg, bad, 0, 0, NULL) == -2, "set while bad node");
    }
    CHECK(circuit_cfg_set_entry_block(cfg, UINT32_MAX) == -2, "set_entry sentinel");

    circuit_cfg_free(cfg);
    operation_free(op);
}

int main(void) {
    test_cfg_from_circuit_roundtrip();
    test_cfg_layout_and_blocks();
    test_cfg_terminators();
    test_cfg_classical_state();
    test_cfg_loop_regions();
    test_cfg_null_and_error_paths();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
