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

static void test_symbolic_eye_shape(void) {
    /* NULL tolerance for the shape readers and the free routine. */
    CHECK(symbolic_matrix_rows(NULL) == 0, "rows(null)");
    CHECK(symbolic_matrix_cols(NULL) == 0, "cols(null)");
    CHECK(symbolic_matrix_evaluate_len(NULL) == 0, "evaluate_len(null)");
    CHECK(symbolic_matrix_element_str(NULL, 0, 0) == NULL, "element_str(null)");
    CHECK(symbolic_matrix_simplify(NULL) == NULL, "simplify(null)");
    symbolic_matrix_free(NULL);
    CHECK(1, "free(null)");

    struct CSymbolicMatrix* eye = symbolic_eye(2);
    CHECK(eye != NULL, "symbolic_eye");
    CHECK(symbolic_matrix_rows(eye) == 2, "eye rows");
    CHECK(symbolic_matrix_cols(eye) == 2, "eye cols");
    CHECK(symbolic_matrix_evaluate_len(eye) == 4, "eye evaluate_len");

    /* Element strings: both diagonal entries match, both off-diagonal
     * entries match, and the two groups differ. */
    char* d00 = symbolic_matrix_element_str(eye, 0, 0);
    char* d11 = symbolic_matrix_element_str(eye, 1, 1);
    CHECK(d00 != NULL && d11 != NULL, "eye diagonal strings");
    char* o01 = symbolic_matrix_element_str(eye, 0, 1);
    char* o10 = symbolic_matrix_element_str(eye, 1, 0);
    CHECK(o01 != NULL && o10 != NULL, "eye off-diagonal strings");
    CHECK(strcmp(d00, d11) == 0, "eye diagonal match");
    CHECK(strcmp(o01, o10) == 0, "eye off-diagonal match");
    CHECK(strcmp(d00, o01) != 0, "eye diagonal vs off-diagonal");
    CHECK(strcmp(d11, o10) != 0, "eye other diagonal vs off-diagonal");
    cqlib_string_free(d00);
    cqlib_string_free(d11);
    cqlib_string_free(o01);
    cqlib_string_free(o10);

    /* Out-of-bounds indices return NULL. */
    CHECK(symbolic_matrix_element_str(eye, 2, 0) == NULL, "eye element_str row out of bounds");
    CHECK(symbolic_matrix_element_str(eye, 0, 2) == NULL, "eye element_str col out of bounds");

    /* Simplification keeps the shape. */
    struct CSymbolicMatrix* simplified = symbolic_matrix_simplify(eye);
    CHECK(simplified != NULL, "eye simplify");
    CHECK(symbolic_matrix_rows(simplified) == 2, "simplified rows");
    CHECK(symbolic_matrix_cols(simplified) == 2, "simplified cols");

    /* A constant matrix evaluates with no bindings at all. */
    Complex64 buf[4];
    CHECK(symbolic_matrix_evaluate(eye, NULL, buf, 4) == 0, "eye evaluate");
    CHECK(buf[0].re == 1.0, "eye evaluate buf[0].re");
    CHECK(buf[0].im == 0.0, "eye evaluate buf[0].im");
    CHECK(buf[1].re == 0.0, "eye evaluate buf[1].re");

    symbolic_matrix_free(simplified);
    symbolic_matrix_free(eye);
}

static void test_standard_gate_matrix_evaluation(void) {
    /* RX(theta) as a symbolic standard-gate matrix. */
    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "gate evaluate fixture param_parse");
    const struct CParameter* params[1];
    params[0] = theta;
    struct CSymbolicMatrix* rx_mat = standard_gate_symbolic_matrix("RX", params, 1);
    CHECK(rx_mat != NULL, "standard_gate_symbolic_matrix RX");
    CHECK(symbolic_matrix_rows(rx_mat) == 2, "RX matrix rows");
    CHECK(symbolic_matrix_cols(rx_mat) == 2, "RX matrix cols");
    CHECK(symbolic_matrix_evaluate_len(rx_mat) == 4, "RX evaluate_len");

    /* The diagonal entry renders as a cos(theta/2) expression. */
    char* diag = symbolic_matrix_element_str(rx_mat, 0, 0);
    CHECK(diag != NULL && strstr(diag, "cos") != NULL, "RX diagonal contains cos");
    cqlib_string_free(diag);

    /* Bound evaluation: RX(0.5) = [[cos(0.25), -i*sin(0.25)],
     *                               [-i*sin(0.25), cos(0.25)]] in row-major
     * order. */
    Complex64 buf[4];
    CHECK(symbolic_matrix_evaluate(rx_mat, "theta:0.5", buf, 4) == 0, "RX bound evaluate");
    CHECK(fabs(buf[0].re - cos(0.25)) < 1e-12, "RX buf[0].re");
    CHECK(buf[0].im == 0.0, "RX buf[0].im");
    CHECK(buf[1].re == 0.0, "RX buf[1].re");
    CHECK(fabs(buf[1].im + sin(0.25)) < 1e-12, "RX buf[1].im");
    CHECK(fabs(buf[2].re - buf[1].re) < 1e-12 && fabs(buf[2].im - buf[1].im) < 1e-12,
          "RX buf[2] mirrors buf[1]");
    CHECK(fabs(buf[3].re - buf[0].re) < 1e-12 && fabs(buf[3].im - buf[0].im) < 1e-12,
          "RX buf[3] mirrors buf[0]");

    /* Cross-check against the matrix of a numeric RX(0.5) circuit. */
    struct CCircuit* circuit = circuit_new(1);
    CHECK(circuit != NULL, "gate evaluate fixture circuit");
    CHECK(circuit_rx(circuit, 0, 0.5) == 0, "gate evaluate fixture circuit_rx");
    struct CSymbolicMatrix* circ_mat = circuit_to_symbolic_matrix(circuit, NULL, 0);
    CHECK(circ_mat != NULL, "gate evaluate circuit matrix");
    Complex64 buf2[4];
    CHECK(symbolic_matrix_evaluate(circ_mat, NULL, buf2, 4) == 0, "circuit matrix evaluate");
    int match = 1;
    for (int i = 0; i < 4; i++) {
        if (fabs(buf[i].re - buf2[i].re) >= 1e-12 || fabs(buf[i].im - buf2[i].im) >= 1e-12) {
            match = 0;
        }
    }
    CHECK(match, "RX matrix matches circuit matrix");

    symbolic_matrix_free(circ_mat);
    symbolic_matrix_free(rx_mat);
    circuit_free(circuit);
    param_free(theta);
}

static void test_standard_gate_matrix_invalid_inputs(void) {
    /* The parameterless X gate builds with no params buffer at all. */
    struct CSymbolicMatrix* x_mat = standard_gate_symbolic_matrix("X", NULL, 0);
    CHECK(x_mat != NULL, "gate matrix X without params");
    symbolic_matrix_free(x_mat);

    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "gate matrix fixture param_parse");
    const struct CParameter* params[1];
    params[0] = theta;

    /* NULL gate name. */
    CHECK(standard_gate_symbolic_matrix(NULL, NULL, 0) == NULL, "gate matrix null name");

    /* Unknown gate name. */
    CHECK(standard_gate_symbolic_matrix("NOT_A_GATE", NULL, 0) == NULL, "gate matrix unknown name");

    /* RX with a NULL params buffer but a non-zero length. */
    CHECK(standard_gate_symbolic_matrix("RX", NULL, 1) == NULL, "gate matrix null params");

    /* RX with zero parameters. */
    CHECK(standard_gate_symbolic_matrix("RX", NULL, 0) == NULL, "gate matrix RX without params");

    /* X with one parameter too many (parameter-count mismatch). */
    CHECK(standard_gate_symbolic_matrix("X", params, 1) == NULL, "gate matrix X with extra param");

    /* A NULL entry inside the params array. */
    const struct CParameter* null_params[1];
    null_params[0] = NULL;
    CHECK(standard_gate_symbolic_matrix("RX", null_params, 1) == NULL,
          "gate matrix null param entry");

    param_free(theta);
}

static void test_circuit_matrix_conversion_and_order(void) {
    /* A 1q parametric circuit yields a 2x2 matrix with a free symbol. */
    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "circuit matrix fixture param_parse");
    struct CCircuit* rx_circuit = circuit_new(1);
    CHECK(rx_circuit != NULL, "circuit matrix fixture rx circuit");
    CHECK(circuit_rx_param(rx_circuit, 0, theta) == 0, "circuit_rx_param");
    struct CSymbolicMatrix* rx_mat = circuit_to_symbolic_matrix(rx_circuit, NULL, 0);
    CHECK(rx_mat != NULL, "circuit matrix from parametric circuit");
    CHECK(symbolic_matrix_rows(rx_mat) == 2, "parametric matrix rows");
    CHECK(symbolic_matrix_cols(rx_mat) == 2, "parametric matrix cols");

    /* The free symbol blocks evaluation without bindings. */
    Complex64 buf[4];
    CHECK(symbolic_matrix_evaluate(rx_mat, NULL, buf, 4) == -3, "parametric matrix needs bindings");

    /* A constant 2q circuit yields a 4x4 matrix of 16 elements. */
    struct CCircuit* circuit = circuit_new(2);
    CHECK(circuit != NULL, "circuit matrix fixture circuit");
    CHECK(circuit_h(circuit, 0) == 0, "circuit matrix fixture h");
    CHECK(circuit_cx(circuit, 0, 1) == 0, "circuit matrix fixture cx");
    struct CSymbolicMatrix* mat = circuit_to_symbolic_matrix(circuit, NULL, 0);
    CHECK(mat != NULL, "circuit matrix from constant circuit");
    CHECK(symbolic_matrix_rows(mat) == 4, "circuit matrix rows");
    CHECK(symbolic_matrix_cols(mat) == 4, "circuit matrix cols");
    CHECK(symbolic_matrix_evaluate_len(mat) == 16, "circuit matrix eval_len");
    Complex64 buf16[16];
    CHECK(symbolic_matrix_evaluate(mat, NULL, buf16, 16) == 0, "circuit matrix constant evaluate");

    /* Order handling: a NULL order buffer with a non-zero length is
     * rejected. */
    CHECK(circuit_to_symbolic_matrix(circuit, NULL, 1) == NULL, "circuit matrix null order");

    /* A full order for the 2q qubit set works. */
    const uint32_t order[2] = {0, 1};
    struct CSymbolicMatrix* ordered = circuit_to_symbolic_matrix(circuit, order, 2);
    CHECK(ordered != NULL, "circuit matrix full order");

    /* A one-entry order misses qubit 1 of the 2q set. */
    const uint32_t partial[1] = {0};
    CHECK(circuit_to_symbolic_matrix(circuit, partial, 1) == NULL, "circuit matrix partial order");

    /* A two-entry order references a qubit the 1q circuit does not
     * have. */
    CHECK(circuit_to_symbolic_matrix(rx_circuit, order, 2) == NULL,
          "circuit matrix order beyond qubit set");

    symbolic_matrix_free(ordered);
    symbolic_matrix_free(mat);
    symbolic_matrix_free(rx_mat);
    circuit_free(circuit);
    circuit_free(rx_circuit);
    param_free(theta);
}

static void test_matrix_evaluate_error_codes(void) {
    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "evaluate errors fixture param_parse");
    const struct CParameter* params[1];
    params[0] = theta;
    struct CSymbolicMatrix* rx_mat = standard_gate_symbolic_matrix("RX", params, 1);
    CHECK(rx_mat != NULL, "evaluate errors fixture rx matrix");

    Complex64 buf[4];

    /* NULL matrix handle. */
    CHECK(symbolic_matrix_evaluate(NULL, "theta:0.5", buf, 4) == -1, "evaluate null matrix");

    /* NULL output buffer with valid bindings and sufficient capacity. */
    CHECK(symbolic_matrix_evaluate(rx_mat, "theta:0.5", NULL, 4) == -1, "evaluate null buffer");

    /* Undersized buffer. */
    CHECK(symbolic_matrix_evaluate(rx_mat, "theta:0.5", buf, 3) == -8, "evaluate short buffer");

    /* Malformed bindings value. */
    CHECK(symbolic_matrix_evaluate(rx_mat, "theta:abc", buf, 4) == -4,
          "evaluate malformed bindings");

    /* Bindings for an unrelated symbol leave theta unevaluable. */
    CHECK(symbolic_matrix_evaluate(rx_mat, "ghost:1.0", buf, 4) == -3,
          "evaluate unrelated bindings");

    /* No bindings at all. */
    CHECK(symbolic_matrix_evaluate(rx_mat, NULL, buf, 4) == -3, "evaluate without bindings");

    /* Constant matrices evaluate without any bindings. */
    struct CSymbolicMatrix* eye = symbolic_eye(2);
    CHECK(eye != NULL, "evaluate errors fixture eye");
    CHECK(symbolic_matrix_evaluate(eye, NULL, buf, 4) == 0, "evaluate constant eye");

    symbolic_matrix_free(eye);
    symbolic_matrix_free(rx_mat);
    param_free(theta);
}

static void test_equivalence_checks(void) {
    /* Matrix-level equivalence: a NULL side is an error. */
    struct CSymbolicMatrix* x_gate = standard_gate_symbolic_matrix("X", NULL, 0);
    CHECK(x_gate != NULL, "equivalence fixture X gate matrix");
    CHECK(symbolic_matrices_equivalent(NULL, x_gate) == -1, "matrix equivalent null lhs");
    CHECK(symbolic_matrices_equivalent(x_gate, NULL) == -1, "matrix equivalent null rhs");

    struct CCircuit* x_circuit = circuit_new(1);
    CHECK(x_circuit != NULL, "equivalence fixture X circuit");
    CHECK(circuit_x(x_circuit, 0) == 0, "equivalence fixture circuit_x");
    struct CSymbolicMatrix* x_mat = circuit_to_symbolic_matrix(x_circuit, NULL, 0);
    CHECK(x_mat != NULL, "equivalence fixture X circuit matrix");
    struct CCircuit* h_circuit = circuit_new(1);
    CHECK(h_circuit != NULL, "equivalence fixture H circuit");
    CHECK(circuit_h(h_circuit, 0) == 0, "equivalence fixture circuit_h");
    struct CSymbolicMatrix* h_mat = circuit_to_symbolic_matrix(h_circuit, NULL, 0);
    CHECK(h_mat != NULL, "equivalence fixture H circuit matrix");

    /* The X gate matrix matches the X circuit matrix. */
    CHECK(symbolic_matrices_equivalent(x_gate, x_mat) == 1, "X gate equals X circuit");
    /* X and H differ by more than a global phase. */
    CHECK(symbolic_matrices_equivalent(x_mat, h_mat) == 0, "X and H circuits differ");

    /* Different shapes (2x2 identity vs 4x4 controlled-X) are not
     * equivalent. */
    struct CSymbolicMatrix* eye = symbolic_eye(2);
    CHECK(eye != NULL, "equivalence fixture eye");
    struct CSymbolicMatrix* cnot = symbolic_matrix_controlled(x_gate, 1);
    CHECK(cnot != NULL, "equivalence fixture controlled X");
    CHECK(symbolic_matrices_equivalent(eye, cnot) == 0, "different shapes not equivalent");

    /* Circuit-level equivalence: RX(pi) equals X up to a global phase. */
    struct CCircuit* rx_pi = circuit_new(1);
    CHECK(rx_pi != NULL, "equivalence fixture RX(pi) circuit");
    CHECK(circuit_rx(rx_pi, 0, M_PI) == 0, "equivalence fixture circuit_rx");
    CHECK(circuits_equivalent(x_circuit, rx_pi, NULL, 0) == 1, "X equals RX(pi)");
    CHECK(circuits_equivalent(x_circuit, x_circuit, NULL, 0) == 1, "X circuit self equivalence");
    CHECK(circuits_equivalent(x_circuit, h_circuit, NULL, 0) == 0,
          "X and H circuits not equivalent");

    /* NULL circuits. */
    CHECK(circuits_equivalent(NULL, x_circuit, NULL, 0) == -1, "circuit equivalent null lhs");
    CHECK(circuits_equivalent(x_circuit, NULL, NULL, 0) == -1, "circuit equivalent null rhs");

    /* A NULL order buffer with a non-zero length. */
    CHECK(circuits_equivalent(x_circuit, rx_pi, NULL, 1) == -8, "circuit equivalent null order");

    /* An order that does not cover the 2q qubit set fails matrix
     * construction. */
    struct CCircuit* empty_a = circuit_new(2);
    struct CCircuit* empty_b = circuit_new(2);
    CHECK(empty_a != NULL && empty_b != NULL, "equivalence fixture empty circuits");
    const uint32_t order[1] = {0};
    CHECK(circuits_equivalent(empty_a, empty_b, order, 1) == -3,
          "circuit equivalent order mismatch");

    symbolic_matrix_free(cnot);
    symbolic_matrix_free(eye);
    symbolic_matrix_free(h_mat);
    symbolic_matrix_free(x_mat);
    symbolic_matrix_free(x_gate);
    circuit_free(empty_b);
    circuit_free(empty_a);
    circuit_free(rx_pi);
    circuit_free(h_circuit);
    circuit_free(x_circuit);
}

static void test_matrix_substitute_and_controlled(void) {
    /* Substitute theta -> 0.5 in the RX(theta) matrix. */
    struct CParameter* theta = param_parse("theta");
    CHECK(theta != NULL, "substitute fixture param_parse");
    const struct CParameter* params[1];
    params[0] = theta;
    struct CSymbolicMatrix* rx_mat = standard_gate_symbolic_matrix("RX", params, 1);
    CHECK(rx_mat != NULL, "substitute fixture RX(theta)");

    struct CParameter* half = param_parse("0.5");
    CHECK(half != NULL, "substitute fixture half");
    const char* names[1];
    names[0] = "theta";
    const struct CParameter* repl[1];
    repl[0] = half;
    struct CSymbolicMatrix* substituted = symbolic_matrix_substitute(rx_mat, names, repl, 1);
    CHECK(substituted != NULL, "symbolic_matrix_substitute");

    /* The substituted matrix evaluates without bindings... */
    Complex64 buf[4];
    CHECK(symbolic_matrix_evaluate(substituted, NULL, buf, 4) == 0, "substituted matrix evaluates");
    CHECK(fabs(buf[0].re - cos(0.25)) < 1e-12, "substituted matrix diagonal value");

    /* ...while the original matrix still carries the free symbol. */
    CHECK(symbolic_matrix_evaluate(rx_mat, NULL, buf, 4) == -3,
          "original matrix keeps free symbol");

    /* Controlled-X embeds the X matrix in the bottom-right of a 4x4
     * identity. */
    struct CSymbolicMatrix* x_mat = standard_gate_symbolic_matrix("X", NULL, 0);
    CHECK(x_mat != NULL, "controlled fixture X matrix");
    struct CSymbolicMatrix* cnot = symbolic_matrix_controlled(x_mat, 1);
    CHECK(cnot != NULL, "controlled X");
    CHECK(symbolic_matrix_rows(cnot) == 4, "controlled X rows");
    CHECK(symbolic_matrix_cols(cnot) == 4, "controlled X cols");
    Complex64 buf4[16];
    CHECK(symbolic_matrix_evaluate(cnot, NULL, buf4, 16) == 0, "controlled X evaluate");
    /* Top-left identity block. */
    CHECK(buf4[0].re == 1.0, "controlled identity block 00");
    CHECK(buf4[5].re == 1.0, "controlled identity block 11");
    /* Bottom-right X block (CNOT pattern) in row-major order. */
    CHECK(buf4[6].re == 0.0, "controlled X block 12 re");
    CHECK(buf4[7].re == 0.0, "controlled X block 13 re");
    CHECK(buf4[10].re == 0.0, "controlled X block 23 re");
    CHECK(buf4[11].re == 1.0, "controlled X block 24 re");
    CHECK(buf4[14].re == 1.0, "controlled X block 32 re");
    CHECK(buf4[15].re == 0.0, "controlled X block 33 re");

    /* One more control level doubles the dimension again. */
    struct CSymbolicMatrix* ccx = symbolic_matrix_controlled(cnot, 1);
    CHECK(ccx != NULL, "controlled cnot");
    CHECK(symbolic_matrix_rows(ccx) == 8, "controlled cnot rows");
    CHECK(symbolic_matrix_cols(ccx) == 8, "controlled cnot cols");

    /* Error paths. */
    CHECK(symbolic_matrix_controlled(NULL, 1) == NULL, "controlled null base");
    CHECK(symbolic_matrix_substitute(NULL, names, repl, 1) == NULL, "substitute null matrix");
    CHECK(symbolic_matrix_substitute(rx_mat, NULL, repl, 1) == NULL, "substitute null names");
    CHECK(symbolic_matrix_substitute(rx_mat, names, NULL, 1) == NULL, "substitute null params");
    /* A NULL entry inside the names array. */
    const char* null_names[1];
    null_names[0] = NULL;
    CHECK(symbolic_matrix_substitute(rx_mat, null_names, repl, 1) == NULL,
          "substitute null name entry");

    symbolic_matrix_free(ccx);
    symbolic_matrix_free(cnot);
    symbolic_matrix_free(x_mat);
    symbolic_matrix_free(substituted);
    symbolic_matrix_free(rx_mat);
    param_free(half);
    param_free(theta);
}

int main(void) {
    test_symbolic_eye_shape();
    test_standard_gate_matrix_evaluation();
    test_standard_gate_matrix_invalid_inputs();
    test_circuit_matrix_conversion_and_order();
    test_matrix_evaluate_error_codes();
    test_equivalence_checks();
    test_matrix_substitute_and_controlled();

    printf("\n%d failure(s)\n", g_failures);
    return (g_failures == 0) ? 0 : 1;
}
