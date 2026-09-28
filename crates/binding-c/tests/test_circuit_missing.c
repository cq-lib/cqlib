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

#include <assert.h>
#include <stdint.h>
#include <stdio.h>

#include "cqlib_c.h"

static void test_from_operations_and_append(void) {
    CCircuit* source = circuit_new(3);
    assert(source != NULL);
    assert(circuit_h(source, 0) == 0);
    assert(circuit_cx(source, 0, 1) == 0);
    CParameter* theta = param_parse("theta");
    assert(theta != NULL);
    assert(circuit_rx_param(source, 2, theta) == 0);
    assert(circuit_num_operations(source) == 3);
    assert(circuit_num_parameters(source) == 1);

    /* Snapshot every top-level operation. */
    CValueOperation* snapshots[3];
    for (uintptr_t i = 0; i < 3; i++) {
        snapshots[i] = circuit_index(source, i);
        assert(snapshots[i] != NULL);
    }
    /* Read-only view used by the value-level construction API. */
    const CValueOperation* ops[3] = {snapshots[0], snapshots[1], snapshots[2]};
    assert(circuit_index(source, 3) == NULL);
    assert(circuit_index(NULL, 0) == NULL);

    /* Rebuild an equivalent circuit from the snapshots. */
    const uint32_t qubits[3] = {0, 1, 2};
    CCircuit* rebuilt = circuit_from_operations(qubits, 3, ops, 3);
    assert(rebuilt != NULL);
    assert(circuit_num_qubits(rebuilt) == 3);
    assert(circuit_num_operations(rebuilt) == 3);
    assert(circuit_num_parameters(rebuilt) == 1);
    assert(circuit_operations_structurally_equal(source, rebuilt) == 1);
    assert(circuit_validate(rebuilt) == 0);

    /* Symbolic parameters survive the round trip. */
    CCircuit* assigned = circuit_assign_params(rebuilt, "theta:0.5");
    assert(assigned != NULL);
    assert(circuit_num_parameters(assigned) == 0);
    circuit_free(assigned);

    /* Appending a snapshot to a fresh circuit. */
    CCircuit* target = circuit_new(3);
    assert(circuit_append_value_operation(target, snapshots[0]) == 0);
    assert(circuit_num_operations(target) == 1);
    assert(circuit_operations_structurally_equal(target, source) == 0);

    /* Appending fails when the snapshot references an unknown qubit. */
    CCircuit* wide = circuit_new(10);
    assert(circuit_x(wide, 9) == 0);
    CValueOperation* far = circuit_index(wide, 0);
    assert(far != NULL);
    CCircuit* small = circuit_new(2);
    assert(circuit_append_value_operation(small, far) == -3);

    /* Error paths for from_operations. */
    CCircuit* empty = circuit_from_operations(NULL, 0, NULL, 0);
    assert(empty != NULL);
    assert(circuit_num_qubits(empty) == 0);
    assert(circuit_num_operations(empty) == 0);
    circuit_free(empty);

    assert(circuit_from_operations(NULL, 1, NULL, 0) == NULL);   /* qubits NULL */
    assert(circuit_from_operations(qubits, 3, NULL, 1) == NULL); /* ops NULL */
    const uint32_t duplicate[2] = {0, 0};
    assert(circuit_from_operations(duplicate, 2, NULL, 0) == NULL);
    const CValueOperation* with_hole[2] = {ops[0], NULL};
    assert(circuit_from_operations(qubits, 3, with_hole, 2) == NULL);
    const CValueOperation* out_of_domain[1] = {far};
    assert(circuit_from_operations(qubits, 3, out_of_domain, 1) == NULL);

    /* Error paths for append. */
    assert(circuit_append_value_operation(NULL, snapshots[0]) == -1);
    assert(circuit_append_value_operation(target, NULL) == -1);

    value_operation_free(far);
    for (uintptr_t i = 0; i < 3; i++) {
        value_operation_free(snapshots[i]);
    }
    circuit_free(small);
    circuit_free(wide);
    circuit_free(target);
    circuit_free(rebuilt);
    circuit_free(source);
    param_free(theta);
}

static void test_membership_and_equality(void) {
    CCircuit* a = circuit_new(2);
    CCircuit* b = circuit_new(2);
    CCircuit* c = circuit_new(3);
    assert(a && b && c);
    assert(circuit_h(a, 0) == 0);
    assert(circuit_h(b, 0) == 0);

    assert(circuit_contains_qubit(a, 0) == 1);
    assert(circuit_contains_qubit(a, 1) == 1);
    assert(circuit_contains_qubit(a, 2) == 0);
    assert(circuit_contains_qubit(NULL, 0) == -1);

    assert(circuit_has_same_qubits(a, b) == 1);
    assert(circuit_has_same_qubits(a, c) == 0);
    assert(circuit_has_same_qubits(a, NULL) == -1);
    assert(circuit_has_same_qubits(NULL, b) == -1);

    assert(circuit_operations_structurally_equal(a, b) == 1);
    assert(circuit_operations_structurally_equal(a, c) == 0);
    assert(circuit_x(b, 0) == 0);
    assert(circuit_operations_structurally_equal(a, b) == 0);
    assert(circuit_operations_structurally_equal(NULL, b) == -1);
    assert(circuit_operations_structurally_equal(a, NULL) == -1);

    /* Different parameter values break structural equality. */
    CCircuit* p1 = circuit_new(1);
    CCircuit* p2 = circuit_new(1);
    assert(circuit_rx(p1, 0, 0.25) == 0);
    assert(circuit_rx(p2, 0, 0.5) == 0);
    assert(circuit_operations_structurally_equal(p1, p2) == 0);
    assert(circuit_operations_structurally_equal(p1, p1) == 1);

    circuit_free(p1);
    circuit_free(p2);
    circuit_free(c);
    circuit_free(b);
    circuit_free(a);
}

static void test_checkpoint_transactions(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);

    assert(circuit_checkpoint(NULL) == NULL);
    assert(circuit_begin(NULL) == NULL);

    /* Rollback discards everything captured after the checkpoint. */
    CCheckpoint* cp = circuit_checkpoint(circuit);
    assert(cp != NULL);
    assert(circuit_cx(circuit, 0, 1) == 0);
    assert(circuit_rx(circuit, 1, 0.25) == 0);
    assert(circuit_num_operations(circuit) == 3);
    assert(circuit_rollback_to(circuit, cp) == 0);
    assert(circuit_num_operations(circuit) == 1);
    assert(circuit_num_qubits(circuit) == 2);
    assert(circuit_validate(circuit) == 0);

    /* Commit keeps everything captured after the checkpoint. */
    cp = circuit_begin(circuit);
    assert(cp != NULL);
    assert(circuit_x(circuit, 1) == 0);
    assert(circuit_commit(circuit, cp) == 0);
    assert(circuit_num_operations(circuit) == 2);

    /* The control-body transaction variant also rolls back. */
    cp = circuit_checkpoint(circuit);
    assert(cp != NULL);
    assert(circuit_z(circuit, 1) == 0);
    assert(circuit_rollback_control_body_transaction(circuit, cp) == 0);
    assert(circuit_num_operations(circuit) == 2);

    /* NULL arguments fail without consuming the token. */
    cp = circuit_checkpoint(circuit);
    assert(cp != NULL);
    assert(circuit_commit(NULL, cp) == -1);
    assert(circuit_rollback_to(circuit, NULL) == -1);
    assert(circuit_commit(circuit, NULL) == -1);
    checkpoint_free(cp);
    checkpoint_free(NULL);

    circuit_free(circuit);
}

static void test_classical_literal_predicates(void) {
    CClassicalExpr* bit_true = classical_expr_bit_literal(true);
    CClassicalExpr* bit_false = classical_expr_bit_literal(false);
    CClassicalExpr* bool_true = classical_expr_bool_literal(true);
    CClassicalExpr* bool_false = classical_expr_bool_literal(false);
    assert(bit_true && bit_false && bool_true && bool_false);

    assert(classical_expr_is_bit_true(bit_true) == 1);
    assert(classical_expr_is_bit_false(bit_true) == 0);
    assert(classical_expr_is_bit_false(bit_false) == 1);
    assert(classical_expr_is_bit_true(bool_true) == 0);
    assert(classical_expr_is_bool_true(bool_true) == 1);
    assert(classical_expr_is_bool_false(bool_true) == 0);
    assert(classical_expr_is_bool_false(bool_false) == 1);
    assert(classical_expr_is_bool_true(bit_true) == 0);
    assert(classical_expr_is_bit_true(NULL) == -1);
    assert(classical_expr_is_bool_false(NULL) == -1);

    /* Zero and one literals of every classical type. */
    CClassicalExpr* zero_bit = classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_BIT, 0);
    CClassicalExpr* one_bool = classical_type_one_literal(CQLIB_CLASSICAL_TYPE_BOOL, 0);
    CClassicalExpr* zero_uint = classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_UINT, 8);
    CClassicalExpr* one_vec = classical_type_one_literal(CQLIB_CLASSICAL_TYPE_BIT_VEC, 4);
    assert(zero_bit && one_bool && zero_uint && one_vec);

    assert(classical_expr_is_bit_false(zero_bit) == 1);
    assert(classical_expr_is_bit_true(zero_bit) == 0);
    assert(classical_expr_is_bool_true(one_bool) == 1);
    assert(classical_expr_is_bool_false(one_bool) == 0);

    uint32_t tag = 99, width = 99, kind = 99;
    assert(classical_expr_ty(zero_uint, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_UINT && width == 8);
    assert(classical_expr_kind(zero_uint, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_UINT_LITERAL);
    assert(classical_expr_ty(one_vec, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 4);
    assert(classical_expr_kind(one_vec, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL);

    /* Invalid type parameters. */
    assert(classical_type_zero_literal(99, 8) == NULL);
    assert(classical_type_one_literal(99, 8) == NULL);
    assert(classical_type_zero_literal(CQLIB_CLASSICAL_TYPE_UINT, 0) == NULL);
    assert(classical_type_one_literal(CQLIB_CLASSICAL_TYPE_BIT_VEC, 0) == NULL);
    assert(classical_type_one_literal(CQLIB_CLASSICAL_TYPE_UINT, 129) == NULL);

    classical_expr_free(one_vec);
    classical_expr_free(zero_uint);
    classical_expr_free(one_bool);
    classical_expr_free(zero_bit);
    classical_expr_free(bool_false);
    classical_expr_free(bool_true);
    classical_expr_free(bit_false);
    classical_expr_free(bit_true);
}

static void test_remap_classical_ids(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    CClassicalVar* var0 = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    CClassicalVar* var1 = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    assert(var0 && var1);
    assert(classical_var_id(var0) == 0);
    assert(classical_var_id(var1) == 1);

    /* Remap a variable reference to a new circuit-local id. */
    CClassicalExpr* var_expr = classical_var_expr(var1);
    assert(var_expr != NULL);
    assert(classical_expr_vars_len(var_expr) == 1);
    const uint32_t old_ids[1] = {1};
    const uint32_t new_ids[1] = {5};
    CClassicalExpr* remapped =
        classical_expr_remap_classical_ids(var_expr, old_ids, new_ids, 1, NULL, NULL, 0);
    assert(remapped != NULL);
    assert(classical_expr_vars_len(remapped) == 1);
    CClassicalVar* collected[1];
    assert(classical_expr_vars(remapped, collected, 1) == 0);
    assert(classical_var_id(collected[0]) == 5);
    classical_var_free(collected[0]);

    /* Every variable read by the expression must be covered. */
    const uint32_t missing[1] = {2};
    assert(classical_expr_remap_classical_ids(var_expr, missing, new_ids, 1, NULL, NULL, 0) ==
           NULL);

    /* NULL arrays with non-zero length and NULL expressions are rejected. */
    assert(classical_expr_remap_classical_ids(var_expr, NULL, new_ids, 1, NULL, NULL, 0) == NULL);
    assert(classical_expr_remap_classical_ids(NULL, old_ids, new_ids, 1, NULL, NULL, 0) == NULL);

    /* Immutable value indices are remapped the same way. */
    uint32_t qubits[2] = {0, 1};
    CClassicalExpr* measured = circuit_measure_bits(circuit, qubits, 2);
    assert(measured != NULL);
    assert(classical_expr_values_len(measured) == 1);
    const uint32_t old_indices[1] = {0};
    const uint32_t new_indices[1] = {3};
    CClassicalExpr* remapped_value =
        classical_expr_remap_classical_ids(measured, NULL, NULL, 0, old_indices, new_indices, 1);
    assert(remapped_value != NULL);
    CClassicalValueInfo values[1];
    assert(classical_expr_values(remapped_value, values, 1) == 0);
    assert(values[0].index == 3 && values[0].tag == CQLIB_CLASSICAL_TYPE_BIT_VEC);

    classical_expr_free(remapped_value);
    classical_expr_free(remapped);
    classical_expr_free(measured);
    classical_expr_free(var_expr);
    classical_var_free(var0);
    classical_var_free(var1);
    circuit_free(circuit);
}

int main(void) {
    test_from_operations_and_append();
    test_membership_and_equality();
    test_checkpoint_transactions();
    test_classical_literal_predicates();
    test_remap_classical_ids();
    printf("binding-c circuit missing tests passed\n");
    return 0;
}
