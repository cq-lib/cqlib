// This code is part of Cqlib.
//
// (C) Copyright China Telecom Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

// C smoke tests for symbolic complex values, element-level matrix
// construction and the in-place `apply_*` gate family.

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

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

/* Evaluates a fully numeric matrix with no bindings. */
static int eval_matrix(const struct CSymbolicMatrix* matrix, Complex64* buffer, uintptr_t len) {
    return symbolic_matrix_evaluate(matrix, NULL, buffer, len);
}

static int close_enough(double a, double b) { return fabs(a - b) < 1e-9; }

static void test_symbolic_complex_construction(void) {
    /* NULL tolerance and invalid-input handling. */
    CHECK(symbolic_complex_new(NULL, NULL) == NULL, "complex new null");
    CHECK(symbolic_complex_from_real(NULL) == NULL, "complex from_real null");
    CHECK(symbolic_complex_exp_i(NULL) == NULL, "complex exp_i null");
    CHECK(!isfinite(0.0 / 0.0) && symbolic_complex_from_complex(0.0 / 0.0, 0.0) == NULL,
          "complex from_complex nan");
    CHECK(symbolic_complex_re(NULL) == NULL, "complex re null");
    CHECK(symbolic_complex_im(NULL) == NULL, "complex im null");
    CHECK(symbolic_complex_is_zero_exact(NULL) == -1, "complex is_zero_exact null");
    CHECK(symbolic_complex_is_one_exact(NULL) == -1, "complex is_one_exact null");
    CHECK(symbolic_complex_simplifies_to_zero(NULL) == -1, "complex simplifies_to_zero null");
    CHECK(symbolic_complex_simplify(NULL) == NULL, "complex simplify null");
    symbolic_complex_free(NULL);
    CHECK(1, "complex free null");

    /* zero / one / i constants. */
    struct CSymbolicComplex* zero = symbolic_complex_zero();
    CHECK(symbolic_complex_is_zero_exact(zero) == 1, "zero is_zero_exact");
    CHECK(symbolic_complex_simplifies_to_zero(zero) == 1, "zero simplifies_to_zero");
    Complex64 out;
    CHECK(symbolic_complex_evaluate(zero, NULL, &out) == 0, "zero evaluate");
    CHECK(out.re == 0.0 && out.im == 0.0, "zero value");

    struct CSymbolicComplex* one = symbolic_complex_one();
    CHECK(symbolic_complex_is_one_exact(one) == 1, "one is_one_exact");
    CHECK(symbolic_complex_is_zero_exact(one) == 0, "one not zero");
    CHECK(symbolic_complex_evaluate(one, NULL, &out) == 0, "one evaluate");
    CHECK(out.re == 1.0 && out.im == 0.0, "one value");

    struct CSymbolicComplex* eye_unit = symbolic_complex_i();
    CHECK(symbolic_complex_is_zero_exact(eye_unit) == 0, "i not zero");
    CHECK(symbolic_complex_evaluate(eye_unit, NULL, &out) == 0, "i evaluate");
    CHECK(out.re == 0.0 && out.im == 1.0, "i value");
    CHECK(symbolic_complex_is_one_exact(eye_unit) == 0, "i not one");

    /* from_complex / from_real. */
    struct CSymbolicComplex* raw = symbolic_complex_from_complex(0.5, -0.25);
    CHECK(symbolic_complex_evaluate(raw, NULL, &out) == 0, "from_complex evaluate");
    CHECK(out.re == 0.5 && out.im == -0.25, "from_complex value");

    struct CParameter* theta = param_symbol("theta");
    struct CSymbolicComplex* real = symbolic_complex_from_real(theta);
    CHECK(symbolic_complex_evaluate(real, "theta:0.75", &out) == 0, "from_real evaluate");
    CHECK(out.re == 0.75 && out.im == 0.0, "from_real value");

    /* new(re, im) and the re/im accessors. */
    struct CParameter* two = param_from_double(2.0);
    struct CSymbolicComplex* value = symbolic_complex_new(theta, two);
    CHECK(value != NULL, "complex new");
    struct CParameter* re = symbolic_complex_re(value);
    struct CParameter* im = symbolic_complex_im(value);
    CHECK(re != NULL && im != NULL, "complex re/im accessors");
    CHECK(close_enough(param_evaluate(re, "theta:0.5"), 0.5), "re accessor value");
    CHECK(close_enough(param_evaluate(im, NULL), 2.0), "im accessor value");
    CHECK(symbolic_complex_evaluate(value, "theta:0.5", &out) == 0, "new evaluate");
    CHECK(out.re == 0.5 && out.im == 2.0, "new value");

    /* exp_i(theta) evaluates to cos(theta) + i*sin(theta). */
    struct CSymbolicComplex* phase = symbolic_complex_exp_i(theta);
    CHECK(symbolic_complex_evaluate(phase, "theta:0", &out) == 0, "exp_i at 0");
    CHECK(close_enough(out.re, 1.0) && close_enough(out.im, 0.0), "exp_i at 0 value");
    CHECK(symbolic_complex_evaluate(phase, "theta:3.141592653589793", &out) == 0, "exp_i at pi");
    CHECK(close_enough(out.re, -1.0) && close_enough(out.im, 0.0), "exp_i at pi value");

    param_free(im);
    param_free(re);
    symbolic_complex_free(phase);
    symbolic_complex_free(value);
    symbolic_complex_free(real);
    symbolic_complex_free(raw);
    symbolic_complex_free(eye_unit);
    symbolic_complex_free(one);
    symbolic_complex_free(zero);
    param_free(two);
    param_free(theta);
}

static void test_symbolic_complex_evaluate_simplify_replace(void) {
    struct CParameter* theta = param_symbol("theta");
    struct CParameter* x = param_symbol("x");
    struct CParameter* diff = param_sub(x, theta);
    struct CParameter* zero_param = param_from_double(0.0);
    /* (x - theta, 0): after replacing x by theta both parts simplify to
     * exact zero. */
    struct CSymbolicComplex* value = symbolic_complex_new(diff, zero_param);
    CHECK(value != NULL, "replace fixture value");

    Complex64 out;
    /* Evaluation needs bound symbols: an unbound one yields -3 and a
     * malformed bindings string yields -4. */
    CHECK(symbolic_complex_evaluate(value, "theta:1", &out) == -3, "evaluate unbound symbol");
    CHECK(symbolic_complex_evaluate(value, "bogus;", &out) == -4, "evaluate malformed bindings");

    struct CSymbolicComplex* replaced = symbolic_complex_replace(value, "x", theta);
    CHECK(replaced != NULL, "complex replace");
    CHECK(symbolic_complex_simplifies_to_zero(replaced) == 1, "replaced simplifies to zero");

    struct CSymbolicComplex* simplified = symbolic_complex_simplify(replaced);
    CHECK(simplified != NULL, "complex simplify");
    CHECK(symbolic_complex_is_zero_exact(simplified) == 1, "simplified is_zero_exact");
    CHECK(symbolic_complex_evaluate(simplified, NULL, &out) == 0, "simplified evaluate");
    CHECK(out.re == 0.0 && out.im == 0.0, "simplified value");

    CHECK(symbolic_complex_replace(NULL, "x", theta) == NULL, "replace null value");
    CHECK(symbolic_complex_replace(value, NULL, theta) == NULL, "replace null symbol");
    CHECK(symbolic_complex_replace(value, "x", NULL) == NULL, "replace null replacement");

    /* Simplify works on composed arithmetic as well. */
    struct CParameter* sum = param_add(theta, theta);
    struct CSymbolicComplex* composed = symbolic_complex_from_real(sum);
    CHECK(symbolic_complex_simplify(composed) != NULL, "simplify composed");

    symbolic_complex_free(simplified);
    symbolic_complex_free(replaced);
    symbolic_complex_free(composed);
    symbolic_complex_free(value);
    param_free(zero_param);
    param_free(sum);
    param_free(diff);
    param_free(x);
    param_free(theta);
}

static void test_symbolic_matrix_new_and_element(void) {
    /* eye(2): build an element snapshot, then rebuild a matrix from it. */
    struct CSymbolicMatrix* eye = symbolic_eye(2);
    CHECK(eye != NULL, "new_and_element fixture eye");
    struct CSymbolicComplex* e00 = symbolic_matrix_element(eye, 0, 0);
    struct CSymbolicComplex* e01 = symbolic_matrix_element(eye, 0, 1);
    struct CSymbolicComplex* e10 = symbolic_matrix_element(eye, 1, 0);
    struct CSymbolicComplex* e11 = symbolic_matrix_element(eye, 1, 1);
    CHECK(e00 != NULL && e01 != NULL && e10 != NULL && e11 != NULL, "eye elements");
    CHECK(symbolic_complex_is_one_exact(e00) == 1, "eye e00 is one");
    CHECK(symbolic_complex_is_zero_exact(e01) == 1, "eye e01 is zero");

    const struct CSymbolicComplex* elements[4];
    elements[0] = e00;
    elements[1] = e01;
    elements[2] = e10;
    elements[3] = e11;
    struct CSymbolicMatrix* rebuilt = symbolic_matrix_new(2, 2, elements, 4);
    CHECK(rebuilt != NULL, "matrix_new from elements");
    CHECK(symbolic_matrix_rows(rebuilt) == 2, "rebuilt rows");
    CHECK(symbolic_matrices_equivalent(eye, rebuilt) == 1, "rebuilt equivalent");

    /* Out-of-bounds element access returns NULL. */
    CHECK(symbolic_matrix_element(eye, 2, 0) == NULL, "element row out of bounds");
    CHECK(symbolic_matrix_element(eye, 0, 2) == NULL, "element col out of bounds");
    CHECK(symbolic_matrix_element(NULL, 0, 0) == NULL, "element null matrix");

    /* Constructors reject shape/length mismatches and NULL handles. */
    CHECK(symbolic_matrix_new(2, 2, elements, 3) == NULL, "matrix_new length mismatch");
    CHECK(symbolic_matrix_new(0, 2, elements, 0) == NULL, "matrix_new zero rows");
    CHECK(symbolic_matrix_new(2, 0, elements, 0) == NULL, "matrix_new zero cols");
    CHECK(symbolic_matrix_new(2, 2, NULL, 4) == NULL, "matrix_new null elements");
    const struct CSymbolicComplex* bad[4];
    bad[0] = e00;
    bad[1] = NULL;
    bad[2] = e10;
    bad[3] = e11;
    CHECK(symbolic_matrix_new(2, 2, bad, 4) == NULL, "matrix_new null entry");

    symbolic_complex_free(e00);
    symbolic_complex_free(e01);
    symbolic_complex_free(e10);
    symbolic_complex_free(e11);
    symbolic_matrix_free(rebuilt);
    symbolic_matrix_free(eye);
}

static void test_apply_standard_gate_matches_circuit(void) {
    /* Reference: a 2-qubit circuit H(0); CX(0, 1). */
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit != NULL, "standard fixture circuit");
    CHECK(circuit_h(circuit, 0) == 0, "standard fixture h");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "standard fixture cx");
    struct CSymbolicMatrix* reference = circuit_to_symbolic_matrix(circuit, NULL, 0);
    CHECK(reference != NULL, "standard fixture reference matrix");

    /* Same unitary assembled in place from the identity matrix. */
    struct CSymbolicMatrix* matrix = symbolic_eye(4);
    const uintptr_t bits[1] = {0};
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "H", bits, 1, NULL, 0) == 0,
          "apply standard H");
    /* `apply_gate_to_matrix` expects the target bits in the system's
     * Little-Endian order, and `circuit_to_symbolic_matrix` reverses the
     * circuit's [control, target] pair before applying, so
     * CX(control=0, target=1) maps to bits [1, 0]. */
    const uintptr_t two_bits[2] = {1, 0};
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "CX", two_bits, 2, NULL, 0) == 0,
          "apply standard CX");
    CHECK(symbolic_matrices_equivalent(reference, matrix) == 1, "assembled equals reference");

    /* Error paths: NULL handles, unknown gate, bad bit arrays. */
    CHECK(symbolic_matrix_apply_standard_gate(NULL, "H", bits, 1, NULL, 0) == -1,
          "apply standard null matrix");
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "NOPE", bits, 1, NULL, 0) == -8,
          "apply standard unknown gate");
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "H", NULL, 1, NULL, 0) == -8,
          "apply standard null bits");
    const uintptr_t out_of_range[1] = {9};
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "H", out_of_range, 1, NULL, 0) == -8,
          "apply standard bit out of range");
    const uintptr_t duplicated[2] = {1, 1};
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "CX", duplicated, 2, NULL, 0) == -8,
          "apply standard duplicated bits");

    /* RX with one parameter works and NULL parameter handles are rejected. */
    struct CSymbolicMatrix* rebuilt = symbolic_eye(4);
    struct CParameter* half = param_from_double(0.5);
    const struct CParameter* params[1];
    params[0] = half;
    CHECK(symbolic_matrix_apply_standard_gate(rebuilt, "RX", bits, 1, params, 1) == 0,
          "apply standard RX");
    const struct CParameter* null_params[1];
    null_params[0] = NULL;
    CHECK(symbolic_matrix_apply_standard_gate(rebuilt, "RX", bits, 1, null_params, 1) == -1,
          "apply standard null param entry");

    param_free(half);
    symbolic_matrix_free(rebuilt);
    symbolic_matrix_free(matrix);
    symbolic_matrix_free(reference);
    circuit_free(circuit);
}

static void test_apply_gate_and_single_qubit(void) {
    struct CSymbolicMatrix* x_gate = standard_gate_symbolic_matrix("X", NULL, 0);
    CHECK(x_gate != NULL, "gate fixture X");

    /* symbolic apply_gate mirrors the numeric X application. */
    struct CSymbolicMatrix* symbolic = symbolic_eye(4);
    const uintptr_t bit[1] = {1};
    CHECK(symbolic_matrix_apply_gate(NULL, x_gate, bit, 1) == -1, "apply gate null matrix");
    CHECK(symbolic_matrix_apply_gate(symbolic, NULL, bit, 1) == -1, "apply gate null gate");
    CHECK(symbolic_matrix_apply_gate(symbolic, x_gate, NULL, 1) == -8, "apply gate null bits");
    CHECK(symbolic_matrix_apply_gate(symbolic, x_gate, bit, 1) == 0, "apply gate X");

    const Complex64 x_num[4] = {{0.0, 0.0}, {1.0, 0.0}, {1.0, 0.0}, {0.0, 0.0}};
    struct CSymbolicMatrix* numeric = symbolic_eye(4);
    CHECK(symbolic_matrix_apply_gate_num(numeric, x_num, 2, bit, 1) == 0, "apply gate_num X");
    /* gate_dim must equal 1 << bits_len. */
    CHECK(symbolic_matrix_apply_gate_num(numeric, x_num, 4, bit, 1) == -8,
          "apply gate_num dim mismatch");
    CHECK(symbolic_matrix_apply_gate_num(NULL, x_num, 2, bit, 1) == -1,
          "apply gate_num null matrix");

    /* The two application paths agree numerically. Applying X on bit 1
     * swaps matrix rows r <-> r ^ 2, so the identity becomes the
     * row-permutation matrix with values[4*r + c] = 1 iff c == r ^ 2. */
    Complex64 sym_values[16];
    Complex64 num_values[16];
    CHECK(eval_matrix(symbolic, sym_values, 16) == 0, "symbolic X evaluate");
    CHECK(eval_matrix(numeric, num_values, 16) == 0, "numeric X evaluate");
    int all_equal = 1;
    for (int i = 0; i < 16; ++i) {
        if (!close_enough(sym_values[i].re, num_values[i].re) ||
            !close_enough(sym_values[i].im, num_values[i].im)) {
            all_equal = 0;
        }
    }
    CHECK(all_equal, "apply gate X paths agree");
    CHECK(num_values[0 * 4 + 2].re == 1.0, "X permutation 0/2");
    CHECK(num_values[2 * 4 + 0].re == 1.0, "X permutation 2/0");
    CHECK(num_values[1 * 4 + 3].re == 1.0, "X permutation 1/3");
    CHECK(num_values[3 * 4 + 1].re == 1.0, "X permutation 3/1");
    CHECK(num_values[0 * 4 + 0].re == 0.0, "X double-swaps itself");

    /* Applying X a second time restores the identity: X * X = I. */
    CHECK(symbolic_matrix_apply_gate(symbolic, x_gate, bit, 1) == 0, "apply gate X twice");
    CHECK(eval_matrix(symbolic, sym_values, 16) == 0, "symbolic X twice evaluate");
    int identity_after_twice = 1;
    for (int i = 0; i < 16; ++i) {
        int row = i / 4;
        int col = i % 4;
        double expect = (row == col) ? 1.0 : 0.0;
        if (!close_enough(sym_values[i].re, expect) || !close_enough(sym_values[i].im, 0.0)) {
            identity_after_twice = 0;
        }
    }
    CHECK(identity_after_twice, "X twice restores identity");

    /* single-qubit variants. A non-2x2 gate payload is rejected. */
    struct CSymbolicMatrix* swapped = symbolic_eye(2);
    struct CSymbolicMatrix* cx_gate = standard_gate_symbolic_matrix("CX", NULL, 0);
    CHECK(symbolic_matrix_apply_single_qubit_gate(swapped, cx_gate, 0) == -8,
          "single_qubit rejects 4x4 gate");
    CHECK(symbolic_matrix_apply_single_qubit_gate(swapped, x_gate, 0) == 0, "single_qubit X");
    CHECK(symbolic_matrix_apply_single_qubit_gate(swapped, x_gate, 5) == -8,
          "single_qubit bit out of range");

    struct CSymbolicMatrix* swapped_num = symbolic_eye(2);
    CHECK(symbolic_matrix_apply_single_qubit_gate_num(swapped_num, x_num, 0) == 0,
          "single_qubit_num X");
    CHECK(symbolic_matrix_apply_single_qubit_gate_num(swapped_num, x_num, 2) == -8,
          "single_qubit_num bit out of range");
    Complex64 swapped_values[4];
    Complex64 swapped_num_values[4];
    CHECK(eval_matrix(swapped, swapped_values, 4) == 0, "single_qubit evaluate");
    CHECK(eval_matrix(swapped_num, swapped_num_values, 4) == 0, "single_qubit_num evaluate");
    CHECK(close_enough(swapped_values[0].re, swapped_num_values[0].re) &&
              close_enough(swapped_values[1].re, swapped_num_values[1].re),
          "single_qubit paths agree");

    symbolic_matrix_free(swapped_num);
    symbolic_matrix_free(swapped);
    symbolic_matrix_free(cx_gate);
    symbolic_matrix_free(numeric);
    symbolic_matrix_free(symbolic);
    symbolic_matrix_free(x_gate);
}

static void test_apply_two_qubit_and_general(void) {
    struct CSymbolicMatrix* cx_gate = standard_gate_symbolic_matrix("CX", NULL, 0);
    CHECK(cx_gate != NULL, "two_qubit fixture CX");

    /* A 2-qubit symbolic gate must be 4x4 (CX is) and the bits distinct. */
    struct CSymbolicMatrix* symbolic = symbolic_eye(4);
    CHECK(symbolic_matrix_apply_two_qubit_gate(NULL, cx_gate, 0, 1) == -1, "two_qubit null matrix");
    CHECK(symbolic_matrix_apply_two_qubit_gate(symbolic, NULL, 0, 1) == -1, "two_qubit null gate");
    CHECK(symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 0) == -8,
          "two_qubit duplicated bits");
    CHECK(symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 7) == -8,
          "two_qubit bit out of range");
    CHECK(symbolic_matrix_apply_two_qubit_gate(symbolic, cx_gate, 0, 1) == 0, "two_qubit CX");

    /* The numeric payload is read back from the standard CX gate so both
     * paths use the exact same matrix. */
    Complex64 cx_num[16];
    CHECK(eval_matrix(cx_gate, cx_num, 16) == 0, "CX numeric payload");

    struct CSymbolicMatrix* numeric = symbolic_eye(4);
    CHECK(symbolic_matrix_apply_two_qubit_gate_num(numeric, NULL, 0, 1) == -1,
          "two_qubit_num null gate");
    CHECK(symbolic_matrix_apply_two_qubit_gate_num(numeric, cx_num, 1, 1) == -8,
          "two_qubit_num duplicated bits");
    CHECK(symbolic_matrix_apply_two_qubit_gate_num(NULL, cx_num, 0, 1) == -1,
          "two_qubit_num null matrix");
    CHECK(symbolic_matrix_apply_two_qubit_gate_num(numeric, cx_num, 0, 1) == 0, "two_qubit_num CX");

    /* Cross-check the two-qubit path against apply_standard_gate on a
     * freshly built identity: both produce the same CX permutation. */
    struct CSymbolicMatrix* via_standard = symbolic_eye(4);
    const uintptr_t std_bits[2] = {0, 1};
    CHECK(symbolic_matrix_apply_standard_gate(via_standard, "CX", std_bits, 2, NULL, 0) == 0,
          "standard CX cross-check");

    /* general-gate variants with a 2-bit target. */
    struct CSymbolicMatrix* general = symbolic_eye(4);
    const uintptr_t bits[2] = {0, 1};
    CHECK(symbolic_matrix_apply_general_gate(general, cx_gate, bits, 2) == 0, "general CX");
    /* Shape mismatch on the general variant: a 2x2 gate with 2 bits. */
    struct CSymbolicMatrix* x_gate = standard_gate_symbolic_matrix("X", NULL, 0);
    struct CSymbolicMatrix* bad_shape = symbolic_eye(4);
    CHECK(symbolic_matrix_apply_general_gate(bad_shape, x_gate, bits, 2) == -8,
          "general gate shape mismatch");

    /* General numeric application equals the symbolic one. */
    struct CSymbolicMatrix* general_num = symbolic_eye(4);
    CHECK(symbolic_matrix_apply_general_gate_num(general_num, cx_num, bits, 2) == 0,
          "general_num CX");
    CHECK(symbolic_matrix_apply_general_gate_num(NULL, cx_num, bits, 2) == -1,
          "general_num null matrix");

    /* All four application paths produce the same matrix. */
    Complex64 sym_values[16];
    Complex64 num_values[16];
    Complex64 std_values[16];
    Complex64 gen_values[16];
    Complex64 gen_num_values[16];
    CHECK(eval_matrix(symbolic, sym_values, 16) == 0, "two_qubit evaluate");
    CHECK(eval_matrix(numeric, num_values, 16) == 0, "two_qubit_num evaluate");
    CHECK(eval_matrix(via_standard, std_values, 16) == 0, "standard CX evaluate");
    CHECK(eval_matrix(general, gen_values, 16) == 0, "general evaluate");
    CHECK(eval_matrix(general_num, gen_num_values, 16) == 0, "general_num evaluate");
    int all_equal = 1;
    for (int i = 0; i < 16; ++i) {
        if (!close_enough(sym_values[i].re, num_values[i].re) ||
            !close_enough(sym_values[i].re, std_values[i].re) ||
            !close_enough(sym_values[i].re, gen_values[i].re) ||
            !close_enough(sym_values[i].re, gen_num_values[i].re)) {
            all_equal = 0;
        }
    }
    CHECK(all_equal, "all CX paths agree");

    symbolic_matrix_free(general_num);
    symbolic_matrix_free(bad_shape);
    symbolic_matrix_free(x_gate);
    symbolic_matrix_free(general);
    symbolic_matrix_free(via_standard);
    symbolic_matrix_free(numeric);
    symbolic_matrix_free(symbolic);
    symbolic_matrix_free(cx_gate);
}

static void test_apply_permutation_and_diagonal(void) {
    /* X on bit 0 equals the permutation (i -> (1 - i, one)). */
    struct CSymbolicComplex* one_a = symbolic_complex_one();
    struct CSymbolicComplex* one_b = symbolic_complex_one();
    struct CSymbolicMatrix* symbolic = symbolic_eye(2);
    const uintptr_t bit[1] = {0};
    const uintptr_t indices[2] = {1, 0};
    const struct CSymbolicComplex* values[2];
    values[0] = one_a;
    values[1] = one_b;
    CHECK(symbolic_matrix_apply_permutation_gate(NULL, indices, values, 2, bit, 1) == -1,
          "permutation null matrix");
    CHECK(symbolic_matrix_apply_permutation_gate(symbolic, NULL, values, 2, bit, 1) == -8,
          "permutation null indices");
    /* len must equal 1 << bits_len. */
    CHECK(symbolic_matrix_apply_permutation_gate(symbolic, indices, values, 3, bit, 1) == -8,
          "permutation length mismatch");
    /* Source index beyond the gate dimension. */
    const uintptr_t bad_index[2] = {5, 0};
    CHECK(symbolic_matrix_apply_permutation_gate(symbolic, bad_index, values, 2, bit, 1) == -1,
          "permutation index beyond dim");
    /* NULL value handle. */
    const struct CSymbolicComplex* null_values[2];
    null_values[0] = one_a;
    null_values[1] = NULL;
    CHECK(symbolic_matrix_apply_permutation_gate(symbolic, indices, null_values, 2, bit, 1) == -1,
          "permutation null value");
    CHECK(symbolic_matrix_apply_permutation_gate(symbolic, indices, values, 2, bit, 1) == 0,
          "permutation X");
    /* The permutation swaps rows 0 <-> 1 of the identity, i.e. it is X. */
    Complex64 permuted[4];
    CHECK(eval_matrix(symbolic, permuted, 4) == 0, "permutation evaluate");
    CHECK(permuted[0].re == 0.0 && permuted[1].re == 1.0, "permutation row 0");
    CHECK(permuted[2].re == 1.0 && permuted[3].re == 0.0, "permutation row 1");

    /* Numeric permutation with factors (1, -1): output row 0 comes from
     * input row 1 unchanged and output row 1 is the negated input row 0. */
    struct CSymbolicMatrix* perm_num = symbolic_eye(2);
    const Complex64 factors[2] = {{1.0, 0.0}, {-1.0, 0.0}};
    CHECK(symbolic_matrix_apply_permutation_gate_num(perm_num, indices, factors, 2, bit, 1) == 0,
          "permutation_num");
    CHECK(symbolic_matrix_apply_permutation_gate_num(NULL, indices, factors, 2, bit, 1) == -1,
          "permutation_num null matrix");
    CHECK(symbolic_matrix_apply_permutation_gate_num(perm_num, bad_index, factors, 2, bit, 1) == -8,
          "permutation_num index beyond dim");
    Complex64 flipped[4];
    CHECK(eval_matrix(perm_num, flipped, 4) == 0, "permutation_num evaluate");
    CHECK(flipped[0].re == 0.0 && flipped[1].re == 1.0, "permutation_num row 0");
    CHECK(flipped[2].re == -1.0 && flipped[3].re == 0.0, "permutation_num row 1");

    /* Symbolic diagonal: diag(1, i) rotated as the S gate on bit 0. */
    struct CSymbolicComplex* eye_unit = symbolic_complex_i();
    struct CSymbolicComplex* diag_one = symbolic_complex_one();
    struct CSymbolicMatrix* diagonal = symbolic_eye(2);
    const struct CSymbolicComplex* diag_values[2];
    diag_values[0] = diag_one;
    diag_values[1] = eye_unit;
    CHECK(symbolic_matrix_apply_diagonal_gate(NULL, diag_values, 2, bit, 1) == -1,
          "diagonal null matrix");
    /* len/dim mismatch. */
    CHECK(symbolic_matrix_apply_diagonal_gate(diagonal, diag_values, 3, bit, 1) == -8,
          "diagonal length mismatch");
    CHECK(symbolic_matrix_apply_diagonal_gate(diagonal, NULL, 2, bit, 1) == -8,
          "diagonal null values");
    CHECK(symbolic_matrix_apply_diagonal_gate(diagonal, diag_values, 2, bit, 1) == 0,
          "diagonal S gate");
    Complex64 rotated[4];
    CHECK(eval_matrix(diagonal, rotated, 4) == 0, "diagonal evaluate");
    CHECK(rotated[0].re == 1.0 && rotated[0].im == 0.0, "diagonal row 0");
    CHECK(rotated[3].re == 0.0 && rotated[3].im == 1.0, "diagonal row 1");

    /* Numeric diagonal with the same payload. */
    struct CSymbolicMatrix* diagonal_num = symbolic_eye(2);
    const Complex64 raw_diag[2] = {{1.0, 0.0}, {0.0, 1.0}};
    CHECK(symbolic_matrix_apply_diagonal_gate_num(diagonal_num, raw_diag, 2, bit, 1) == 0,
          "diagonal_num");
    CHECK(symbolic_matrix_apply_diagonal_gate_num(NULL, raw_diag, 2, bit, 1) == -1,
          "diagonal_num null matrix");
    CHECK(symbolic_matrix_apply_diagonal_gate_num(diagonal_num, raw_diag, 5, bit, 1) == -8,
          "diagonal_num length mismatch");
    Complex64 rotated_num[4];
    CHECK(eval_matrix(diagonal_num, rotated_num, 4) == 0, "diagonal_num evaluate");
    CHECK(close_enough(rotated[3].re, rotated_num[3].re) &&
              close_enough(rotated[3].im, rotated_num[3].im),
          "diagonal_num paths agree");

    symbolic_matrix_free(diagonal_num);
    symbolic_matrix_free(diagonal);
    symbolic_complex_free(diag_one);
    symbolic_complex_free(eye_unit);
    symbolic_matrix_free(perm_num);
    symbolic_matrix_free(symbolic);
    symbolic_complex_free(one_b);
    symbolic_complex_free(one_a);
}

static void test_apply_keeps_shape(void) {
    /* Sanity: applying gates never changes the matrix shape. */
    struct CSymbolicMatrix* matrix = symbolic_eye(4);
    const uintptr_t bits[2] = {0, 1};
    CHECK(symbolic_matrix_apply_standard_gate(matrix, "CX", bits, 2, NULL, 0) == 0,
          "shape check CX");
    CHECK(symbolic_matrix_rows(matrix) == 4, "shape rows unchanged");
    CHECK(symbolic_matrix_evaluate_len(matrix) == 16, "shape evaluate_len unchanged");
    symbolic_matrix_free(matrix);
}

int main(void) {
    test_symbolic_complex_construction();
    test_symbolic_complex_evaluate_simplify_replace();
    test_symbolic_matrix_new_and_element();
    test_apply_standard_gate_matches_circuit();
    test_apply_gate_and_single_qubit();
    test_apply_two_qubit_and_general();
    test_apply_permutation_and_diagonal();
    test_apply_keeps_shape();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
