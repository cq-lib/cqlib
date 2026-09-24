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
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static void test_construction(void) {
    CParameter* theta = param_symbol("theta");
    assert(theta != NULL);

    char* name = param_as_symbol(theta);
    assert(name != NULL);
    assert(strcmp(name, "theta") == 0);
    cqlib_string_free(name);

    CParameter* pi = param_pi();
    assert(fabs(param_evaluate(pi, NULL) - M_PI) < 1e-12);
    assert(param_is_constant(pi) == 1);

    CParameter* e = param_e();
    assert(fabs(param_evaluate(e, NULL) - exp(1.0)) < 1e-12);

    CParameter* half = param_from_double(0.5);
    assert(fabs(param_evaluate(half, NULL) - 0.5) < 1e-12);

    assert(param_from_double(NAN) == NULL);
    assert(param_from_double(INFINITY) == NULL);
    assert(param_symbol(NULL) == NULL);

    param_free(theta);
    param_free(pi);
    param_free(e);
    param_free(half);
}

static void test_arithmetic_and_math(void) {
    CParameter* theta = param_symbol("theta");
    CParameter* phi = param_symbol("phi");

    CParameter* sum = param_add(theta, phi);
    assert(fabs(param_evaluate(sum, "theta:0.5,phi:1.5") - 2.0) < 1e-12);

    CParameter* neg = param_neg(theta);
    assert(fabs(param_evaluate(neg, "theta:0.5") + 0.5) < 1e-12);

    CParameter* x = param_from_double(2.0);
    CParameter* cube = param_pow(x, param_from_double(3.0));
    assert(fabs(param_evaluate(cube, NULL) - 8.0) < 1e-12);

    CParameter* log2_8 = param_log(param_from_double(8.0), param_from_double(2.0));
    assert(fabs(param_evaluate(log2_8, NULL) - 3.0) < 1e-12);

    CParameter* rounded = param_round(param_from_double(1.75));
    assert(fabs(param_evaluate(rounded, NULL) - 2.0) < 1e-12);

    CParameter* sinus = param_sin(param_from_double(0.5));
    assert(fabs(param_evaluate(sinus, NULL) - sin(0.5)) < 1e-12);

    assert(param_add(NULL, theta) == NULL);
    assert(param_sin(NULL) == NULL);

    param_free(theta);
    param_free(phi);
    param_free(sum);
    param_free(neg);
    param_free(x);
    param_free(cube);
    param_free(log2_8);
    param_free(rounded);
    param_free(sinus);
}

static void test_symbol_queries(void) {
    CParameter* expr = param_parse("phi + theta * 2");
    assert(expr != NULL);

    uintptr_t len = param_symbols_len(expr);
    assert(len == 2);

    char** names = malloc(len * sizeof(char*));
    assert(names != NULL);
    assert(param_symbols(expr, names, len) == 0);
    assert(strcmp(names[0], "phi") == 0);
    assert(strcmp(names[1], "theta") == 0);
    for (uintptr_t i = 0; i < len; ++i) {
        cqlib_string_free(names[i]);
    }
    free(names);

    assert(param_symbols(expr, NULL, 1) == -8);
    assert(param_symbols(NULL, NULL, 0) == -1);
    assert(param_as_symbol(expr) == NULL);
    assert(param_is_constant(expr) == 0);

    CParameter* zero = param_from_double(0.0);
    assert(param_is_zero(zero) == 1);
    assert(param_is_one(zero) == 0);
    assert(param_is_exact_zero(zero) == 1);
    assert(param_is_constant(zero) == 1);
    assert(param_is_zero(NULL) == -1);

    param_free(expr);
    param_free(zero);
}

static void test_simplify_substitute_derivative(void) {
    CParameter* theta = param_symbol("theta");
    CParameter* expr = param_add(theta, param_from_double(0.0));

    CParameter* simplified = param_simplify(expr);
    assert(simplified != NULL);
    assert(param_provably_equal(simplified, theta, 1e-12) == 1);

    CParameter* canonical = param_canonicalized(param_parse("pi / 2"));
    assert(canonical != NULL);
    assert(fabs(param_evaluate(canonical, NULL) - M_PI / 2.0) < 1e-12);

    CParameter* replaced = param_replace(param_parse("x + 2.0"), "x", param_from_double(1.0));
    assert(fabs(param_evaluate(replaced, NULL) - 3.0) < 1e-12);

    const char* names[2] = {"x", "y"};
    const CParameter* values[2] = {param_from_double(2.0), param_from_double(3.0)};
    CParameter* sum = param_substitute_many(param_parse("x + y"), names, values, 2);
    assert(fabs(param_evaluate(sum, NULL) - 5.0) < 1e-12);

    CParameter* square = param_mul(theta, theta);
    CParameter* deriv = param_derivative(square, "theta");
    assert(deriv != NULL);
    assert(fabs(param_evaluate(deriv, "theta:3.0") - 6.0) < 1e-12);

    CParameter* two_pi = param_from_double(2.0 * M_PI);
    assert(param_provably_equal_modulo(param_from_double(0.5), param_from_double(0.5 + 2.0 * M_PI),
                                       two_pi, 1e-12) == 1);
    assert(param_provably_equal(param_from_double(0.5), param_from_double(0.6), 1e-12) == 0);
    assert(param_provably_equal(NULL, two_pi, 1e-12) == -1);

    param_free(theta);
    param_free(expr);
    param_free(simplified);
    param_free(canonical);
    param_free(replaced);
    param_free(sum);
    param_free(square);
    param_free(deriv);
    param_free(two_pi);
    param_free((CParameter*)values[0]);
    param_free((CParameter*)values[1]);
}

int main(void) {
    test_construction();
    test_arithmetic_and_math();
    test_symbol_queries();
    test_simplify_substitute_derivative();
    printf("test_parameter passed\n");
    return 0;
}
