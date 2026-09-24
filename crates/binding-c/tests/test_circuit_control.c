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
 * Smoke tests for the callback-based classical control-flow builders.
 */

#include <assert.h>
#include <stdio.h>

#include "cqlib_c.h"

typedef struct {
    int calls;
} BodyCtx;

static int add_x_body(CCircuit* circuit, void* user_data) {
    BodyCtx* ctx = (BodyCtx*)user_data;
    ctx->calls += 1;
    return circuit_x(circuit, 1);
}

static int measure_then_break_body(CCircuit* circuit, void* user_data) {
    BodyCtx* ctx = (BodyCtx*)user_data;
    ctx->calls += 1;

    int rc = circuit_measure(circuit, 0);
    if (rc != 0) {
        return rc;
    }
    rc = circuit_x(circuit, 1);
    if (rc != 0) {
        return rc;
    }
    return circuit_break_loop(circuit);
}

static void test_if_with_callback(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);

    CClassicalExpr* cond = classical_expr_bool_literal(true);
    assert(cond != NULL);

    BodyCtx ctx = {0};
    assert(circuit_if(circuit, cond, add_x_body, &ctx) == 0);
    assert(ctx.calls == 1);
    /* Only the control operation itself is visible at the top level. */
    assert(circuit_num_operations(circuit) == 1);
    /* Recursive depth: control op boundary + body x(1) -> 1 + 1 = 2. */
    assert(circuit_depth(circuit, true) == 2);
    assert(circuit_validate(circuit) == 0);

    classical_expr_free(cond);
    circuit_free(circuit);
}

static void test_while_with_loop_variable(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);

    CClassicalVar* flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    assert(flag != NULL);
    CClassicalExpr* cond = classical_var_expr(flag);
    assert(cond != NULL);

    BodyCtx ctx = {0};
    assert(circuit_while(circuit, cond, measure_then_break_body, &ctx) == 0);
    assert(ctx.calls == 1);
    assert(circuit_num_operations(circuit) == 1);
    assert(circuit_validate(circuit) == 0);

    classical_expr_free(cond);
    classical_var_free(flag);
    circuit_free(circuit);
}

static void test_null_and_scope_errors(void) {
    assert(circuit_if(NULL, NULL, NULL, NULL) == -1);
    assert(circuit_break_loop(NULL) == -1);
    assert(circuit_continue_loop(NULL) == -1);
    assert(circuit_append_control(NULL, NULL, 0) == -1);

    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);
    /* break/continue outside any loop body are rejected */
    assert(circuit_break_loop(circuit) == -3);
    assert(circuit_continue_loop(circuit) == -3);
    circuit_free(circuit);
}

int main(void) {
    test_if_with_callback();
    test_while_with_loop_variable();
    test_null_and_scope_errors();
    printf("binding-c circuit control-flow tests passed\n");
    return 0;
}
