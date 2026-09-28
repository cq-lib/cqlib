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

/*
 * Smoke test for the classical variable / expression C ABI (checklist 1.1 + 1.2).
 *
 * Compile (msvc):  cl /I ../../crates/binding-c/include test_circuit_classical.c /link cqlib_c.lib
 * Compile (gcc):   gcc -I ../../crates/binding-c/include test_circuit_classical.c -L <target>/debug
 * -lcqlib_c -o test_circuit_classical
 */

#include <assert.h>
#include <stdio.h>

#include "cqlib_c.h"

static void test_classical_vars_and_measure(void) {
    CCircuit* circuit = circuit_new(3);
    assert(circuit != NULL);

    /* Allocate classical variables of every type. */
    CClassicalVar* bit = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    CClassicalVar* flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    CClassicalVar* counter = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 8);
    CClassicalVar* register_var = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2);
    assert(bit && flag && counter && register_var);

    assert(classical_var_id(bit) == 0);
    assert(classical_var_index(register_var) == 3);

    uint32_t tag = 99, width = 99;
    assert(classical_var_ty(counter, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_UINT && width == 8);

    /* Invalid type parameters are rejected. */
    assert(circuit_var(circuit, 99, 4) == NULL);
    assert(circuit_var(circuit, CQLIB_CLASSICAL_TYPE_UINT, 0) == NULL);
    assert(circuit_var(NULL, CQLIB_CLASSICAL_TYPE_BIT, 0) == NULL);

    /* Type table snapshot (two-step). */
    assert(circuit_classical_vars_len(circuit) == 4);
    CClassicalType types[4];
    assert(circuit_classical_vars(circuit, types, 4) == 0);
    assert(types[0].tag == CQLIB_CLASSICAL_TYPE_BIT && types[0].width == 1);
    assert(types[2].tag == CQLIB_CLASSICAL_TYPE_UINT && types[2].width == 8);
    assert(types[3].tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && types[3].width == 2);
    assert(circuit_classical_vars(circuit, types, 3) == -8);

    /* Multi-qubit measurement returns a BitVec value expression. */
    uint32_t qubits[2] = {0, 1};
    CClassicalExpr* measured = circuit_measure_bits(circuit, qubits, 2);
    assert(measured != NULL);
    assert(classical_expr_ty(measured, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 2);
    assert(classical_expr_values_len(measured) == 1);
    assert(classical_expr_vars_len(measured) == 0);
    CClassicalValueInfo values[1];
    assert(classical_expr_values(measured, values, 1) == 0);
    assert(values[0].index == 0 && values[0].tag == CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert(circuit_validate_classical_expr(circuit, measured) == 0);

    /* Measure into a Bit variable: appends measure + store. */
    CClassicalExpr* stored_bit = circuit_measure_into(circuit, 2, bit);
    assert(stored_bit != NULL);
    assert(circuit_num_operations(circuit) == 3); /* measure_bits + measure + store */

    /* Measure bits into a matching-width BitVec variable. */
    CClassicalExpr* stored_vec = circuit_measure_bits_into(circuit, qubits, 2, register_var);
    assert(stored_vec != NULL);
    assert(circuit_num_operations(circuit) == 5);

    /* Error paths. */
    assert(circuit_measure_bits(circuit, NULL, 0) == NULL);
    uint32_t oob[1] = {9};
    assert(circuit_measure_bits(circuit, oob, 1) == NULL);
    assert(circuit_measure_into(circuit, 0, flag) == NULL); /* Bool target */

    /* Validation. */
    assert(circuit_validate_classical_var(circuit, bit) == 0);
    assert(circuit_validate_classical_var(NULL, bit) == -1);
    assert(circuit_validate(circuit) == 0);

    classical_expr_free(stored_bit);
    classical_expr_free(stored_vec);
    classical_expr_free(measured);
    classical_var_free(bit);
    classical_var_free(flag);
    classical_var_free(counter);
    classical_var_free(register_var);
    circuit_free(circuit);
}

static void test_expressions_and_store(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);

    /* Literals. */
    CClassicalExpr* bit_true = classical_expr_bit_literal(true);
    CClassicalExpr* bool_true = classical_expr_bool_literal(true);
    CClassicalExpr* bool_false = classical_expr_bool_literal(false);
    CClassicalExpr* ten = classical_expr_uint_literal(8, 10, 0);
    CClassicalExpr* twenty = classical_expr_uint_literal(8, 20, 0);
    assert(bit_true && bool_true && bool_false && ten && twenty);

    uint32_t kind = 99, tag = 99, width = 99;
    assert(classical_expr_kind(bool_true, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_BOOL_LITERAL);
    assert(classical_expr_ty(ten, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_UINT && width == 8);
    assert(classical_expr_uint_literal(8, 256, 0) == NULL); /* value overflow */
    assert(classical_expr_uint_literal(0, 0, 0) == NULL);   /* zero width */

    /* Bit <-> Bool conversion and logic. */
    CClassicalExpr* cond = classical_expr_bit_to_bool(bit_true);
    assert(cond != NULL);
    CClassicalExpr* not_cond = classical_expr_not(cond);
    assert(not_cond != NULL);
    CClassicalExpr* both = classical_expr_and(bool_true, bool_false);
    assert(both != NULL);
    assert(classical_expr_and(bool_true, bit_true) == NULL); /* type mismatch */
    assert(classical_expr_or(bool_true, NULL) == NULL);

    /* Comparisons on UInt operands produce Bool. */
    CClassicalExpr* lt = classical_expr_lt(ten, twenty);
    assert(lt != NULL);
    assert(classical_expr_ty(lt, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BOOL);
    assert(classical_expr_gt(bool_true, bool_false) == NULL); /* Bool operands */

    /* Select / extract / pack / concat / cast. */
    CClassicalExpr* chosen = classical_expr_select(bool_true, ten, twenty);
    assert(chosen != NULL);
    CClassicalExpr* lsb = classical_expr_extract_bit(ten, 0);
    assert(lsb != NULL);
    assert(classical_expr_extract_bit(ten, 8) == NULL); /* out of bounds */
    CClassicalExpr* nibble = classical_expr_extract_bits(ten, 4, 4);
    assert(nibble != NULL);

    const CClassicalExpr* bits[2] = {bit_true, bit_true};
    CClassicalExpr* packed = classical_expr_pack_bits(bits, 2);
    assert(packed != NULL);
    assert(classical_expr_ty(packed, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 2);

    const CClassicalExpr* parts[2] = {packed, bit_true};
    CClassicalExpr* concatenated = classical_expr_concat(parts, 2);
    assert(concatenated != NULL);
    assert(classical_expr_ty(concatenated, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 3);

    CClassicalExpr* as_uint = classical_expr_bit_vec_to_uint(packed);
    assert(as_uint != NULL);

    /* Simplification: and(b, true) folds to b. */
    CClassicalExpr* simplified = classical_expr_simplified(both);
    assert(simplified != NULL);
    assert(classical_expr_kind(simplified, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_BOOL_LITERAL);

    /* Store: Bool literal into a Bool variable. */
    CClassicalVar* flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert(flag != NULL);
    assert(circuit_store(circuit, flag, bool_true) == 0);
    assert(circuit_num_operations(circuit) == 1);

    /* Store a measured bit converted to Bool. */
    CClassicalVar* bit_target = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BIT, 0);
    CClassicalExpr* measured = circuit_measure_into(circuit, 0, bit_target);
    assert(measured != NULL);
    CClassicalExpr* condition = classical_expr_to_bool(measured);
    assert(condition != NULL);
    assert(circuit_store(circuit, flag, condition) == 0);
    assert(circuit_num_operations(circuit) == 4);
    assert(circuit_validate(circuit) == 0);

    /* Store error paths. */
    assert(circuit_store(NULL, flag, bool_true) == -1);
    assert(circuit_store(circuit, NULL, bool_true) == -1);
    assert(circuit_store(circuit, flag, NULL) == -1);
    assert(circuit_store(circuit, bit_target, bool_true) == -3); /* type mismatch */

    /* vars collection returns owned handles. */
    CClassicalExpr* flag_expr = classical_var_expr(flag);
    assert(flag_expr != NULL);
    assert(classical_expr_vars_len(flag_expr) == 1);
    CClassicalVar* collected[1];
    assert(classical_expr_vars(flag_expr, collected, 1) == 0);
    assert(classical_var_id(collected[0]) == 0);
    assert(circuit_validate_classical_var(circuit, collected[0]) == 0);

    classical_var_free(collected[0]);
    classical_expr_free(flag_expr);
    classical_expr_free(condition);
    classical_expr_free(measured);
    classical_expr_free(simplified);
    classical_expr_free(as_uint);
    classical_expr_free(concatenated);
    classical_expr_free(packed);
    classical_expr_free(nibble);
    classical_expr_free(lsb);
    classical_expr_free(chosen);
    classical_expr_free(lt);
    classical_expr_free(both);
    classical_expr_free(not_cond);
    classical_expr_free(cond);
    classical_expr_free(bit_true);
    classical_expr_free(bool_true);
    classical_expr_free(bool_false);
    classical_expr_free(ten);
    classical_expr_free(twenty);
    classical_var_free(flag);
    classical_var_free(bit_target);
    circuit_free(circuit);
}

int main(void) {
    test_classical_vars_and_measure();
    test_expressions_and_store();
    printf("binding-c circuit classical tests passed\n");
    return 0;
}
