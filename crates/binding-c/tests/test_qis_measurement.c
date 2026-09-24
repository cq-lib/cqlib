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
 * Smoke test for the QIS simulator supplement and Measurement C ABI
 * (checklist 2.5 + 2.6).
 *
 * Compile (msvc):  cl /I ../../crates/binding-c/include test_qis_measurement.c /link cqlib_c.lib
 * Compile (gcc):   gcc -I ../../crates/binding-c/include test_qis_measurement.c -L <target>/debug
 * -lcqlib_c -o test_qis_measurement
 */

#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

static int near(double a, double b) { return fabs(a - b) < 1e-10; }

static void test_statevector_supplement(void) {
    const double inv = 1.0 / sqrt(2.0);

    /* Bell state built with named gates, read back through data(). */
    CStatevector* sv = statevector_new(2);
    assert(sv != NULL);
    uint32_t targets[1] = {0};
    uint32_t pair[2] = {0, 1};
    assert(statevector_apply_standard_gate(sv, "H", targets, 1, NULL, 0) == 0);
    assert(statevector_apply_standard_gate(sv, "CX", pair, 2, NULL, 0) == 0);

    assert(statevector_data_len(sv) == 4);
    Complex64 amps[4];
    assert(statevector_data(sv, amps, 4) == 0);
    assert(near(amps[0].re, inv) && near(amps[0].im, 0.0));
    assert(near(amps[1].re, 0.0) && near(amps[1].im, 0.0));
    assert(near(amps[2].re, 0.0) && near(amps[2].im, 0.0));
    assert(near(amps[3].re, inv) && near(amps[3].im, 0.0));

    /* Named "H" matches the dedicated apply_h entry point. */
    CStatevector* a = statevector_new(1);
    CStatevector* b = statevector_new(1);
    assert(statevector_apply_h(a, 0) == 0);
    assert(statevector_apply_standard_gate(b, "H", targets, 1, NULL, 0) == 0);
    Complex64 ca[2], cb[2];
    assert(statevector_data(a, ca, 2) == 0);
    assert(statevector_data(b, cb, 2) == 0);
    assert(near(ca[0].re, cb[0].re) && near(ca[1].re, cb[1].re));
    statevector_free(a);
    statevector_free(b);

    /* RX(pi) maps |0> to -i|1>. */
    double pi = acos(-1.0);
    assert(statevector_apply_standard_gate(sv, "RX", targets, 1, &pi, 1) == 0);

    /* Pauli rotation exp(-i*pi/2*X) on a fresh qubit maps |0> to -i|1>. */
    CPauliString* pauli = pauli_string_new(1);
    assert(pauli != NULL);
    assert(pauli_string_set_pauli(pauli, 0, PAULI_X) == 0);
    CStatevector* sv1 = statevector_new(1);
    assert(statevector_apply_pauli_rotation(sv1, pauli, pi) == 0);
    assert(statevector_data(sv1, amps, 2) == 0);
    assert(near(amps[0].re, 0.0) && near(amps[1].im, -1.0));
    statevector_free(sv1);
    pauli_string_free(pauli);

    /* Arbitrary unitary: X matrix flips |0> to |1>. */
    Complex64 x_matrix[4] = {{0.0, 0.0}, {1.0, 0.0}, {1.0, 0.0}, {0.0, 0.0}};
    CStatevector* svx = statevector_new(1);
    assert(statevector_apply_unitary_gate(svx, targets, 1, x_matrix, 2) == 0);
    assert(statevector_data(svx, amps, 2) == 0);
    assert(near(amps[1].re, 1.0));
    statevector_free(svx);

    /* Rebuild the Bell state from amplitudes via from_state. */
    Complex64 bell[4] = {{inv, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {inv, 0.0}};
    CStatevector* rebuilt = statevector_from_state(2, bell, 4);
    assert(rebuilt != NULL);
    assert(statevector_data(rebuilt, amps, 4) == 0);
    assert(near(amps[0].re, inv) && near(amps[3].re, inv));
    statevector_free(rebuilt);
    assert(statevector_from_state(2, bell, 3) == NULL); /* wrong dimension */

    /* Error paths. */
    assert(statevector_data(sv, amps, 3) == -8);
    assert(statevector_data(sv, NULL, 4) == -1);
    assert(statevector_apply_standard_gate(sv, "NOPE", targets, 1, NULL, 0) == -8);
    assert(statevector_apply_standard_gate(sv, NULL, targets, 1, NULL, 0) == -1);
    uint32_t oob[1] = {9};
    assert(statevector_apply_standard_gate(sv, "H", oob, 1, NULL, 0) == -2);
    statevector_free(sv);
}

static void test_density_matrix_supplement(void) {
    const double inv = 1.0 / sqrt(2.0);

    /* rho = |+><+| has all entries 0.5 and trace 1. */
    Complex64 plus[2] = {{inv, 0.0}, {inv, 0.0}};
    CDensityMatrix* dm = density_matrix_from_state(1, plus, 2);
    assert(dm != NULL);
    assert(density_matrix_data_len(dm) == 4);
    Complex64 data[16];
    assert(density_matrix_data(dm, data, 4) == 0);
    for (int i = 0; i < 4; i++) {
        assert(near(data[i].re, 0.5) && near(data[i].im, 0.0));
    }
    double re = 0.0, im = 0.0;
    assert(density_matrix_trace(dm, &re, &im) == 0);
    assert(near(re, 1.0) && near(im, 0.0));
    bool hermitian = false;
    assert(density_matrix_is_hermitian(dm, 1e-10, &hermitian) == 0);
    assert(hermitian);

    /* Named "X" flips |0><0| to |1><1|. */
    Complex64 ket0[2] = {{1.0, 0.0}, {0.0, 0.0}};
    CDensityMatrix* flipped = density_matrix_from_state(1, ket0, 2);
    uint32_t targets[1] = {0};
    assert(density_matrix_apply_standard_gate(flipped, "X", targets, 1, NULL, 0) == 0);
    assert(density_matrix_data(flipped, data, 4) == 0);
    assert(near(data[0].re, 0.0) && near(data[3].re, 1.0));
    assert(density_matrix_apply_standard_gate(flipped, "NOPE", targets, 1, NULL, 0) == -8);
    density_matrix_free(flipped);
    density_matrix_free(dm);

    /* Maximally mixed 2-qubit state: diagonal 0.25, trace 1. */
    CDensityMatrix* mm = density_matrix_maximally_mixed(2);
    assert(mm != NULL);
    assert(density_matrix_data_len(mm) == 16);
    assert(density_matrix_data(mm, data, 16) == 0);
    for (int i = 0; i < 4; i++) {
        assert(near(data[i * 4 + i].re, 0.25));
    }
    assert(density_matrix_trace(mm, &re, &im) == 0);
    assert(near(re, 1.0) && near(im, 0.0));
    density_matrix_free(mm);

    assert(density_matrix_trace(NULL, &re, &im) == -1);
    assert(density_matrix_data_len(NULL) == 0);
}

static void test_stabilizer_supplement(void) {
    /* Fresh 1-qubit state: destabilizer +X, stim export "+Z\n". */
    CStabilizerState* s = stabilizer_new(1);
    assert(s != NULL);
    assert(stabilizer_get_destabilizers_len(s) == 1);
    char* destab = stabilizer_get_destabilizer(s, 0);
    assert(destab != NULL && strcmp(destab, "+X") == 0);
    cqlib_string_free(destab);
    assert(stabilizer_get_destabilizer(s, 5) == NULL);
    char* stim = stabilizer_to_stim_format(s);
    assert(stim != NULL && strcmp(stim, "+Z\n") == 0);
    cqlib_string_free(stim);
    stabilizer_free(s);

    /* Bell state outcome probabilities. */
    CStabilizerState* bell = stabilizer_new(2);
    uint32_t pair[2] = {0, 1};
    assert(stabilizer_apply_standard_gate(bell, "H", pair, 1, NULL, 0) == 0);
    assert(stabilizer_apply_standard_gate(bell, "CX", pair, 2, NULL, 0) == 0);
    double p = 0.0;
    bool bits00[2] = {false, false};
    bool bits11[2] = {true, true};
    bool bits01[2] = {false, true};
    assert(stabilizer_probability_of(bell, bits00, 2, &p) == 0 && near(p, 0.5));
    assert(stabilizer_probability_of(bell, bits11, 2, &p) == 0 && near(p, 0.5));
    assert(stabilizer_probability_of(bell, bits01, 2, &p) == 0 && p < 1e-12);
    bool one_bit[1] = {false};
    assert(stabilizer_probability_of(bell, one_bit, 1, &p) == -8);
    stabilizer_free(bell);

    /* Non-Clifford "T" is rejected with -7. */
    CStabilizerState* t = stabilizer_new(1);
    uint32_t targets[1] = {0};
    assert(stabilizer_apply_standard_gate(t, "T", targets, 1, NULL, 0) == -7);
    assert(stabilizer_apply_standard_gate(t, "NOPE", targets, 1, NULL, 0) == -8);
    stabilizer_free(t);

    /* run_circuit collapses terminal measurements. */
    CCircuit* circuit = circuit_new(2);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    uint32_t measured[2] = {0, 1};
    CClassicalExpr* expr = circuit_measure_bits(circuit, measured, 2);
    assert(expr != NULL);
    classical_expr_free(expr);

    CStabilizerState* collapsed = stabilizer_run_circuit(circuit);
    assert(collapsed != NULL);
    circuit_free(circuit);
    double p00 = 0.0, p11 = 0.0, p01 = 0.0;
    assert(stabilizer_probability_of(collapsed, bits00, 2, &p00) == 0);
    assert(stabilizer_probability_of(collapsed, bits11, 2, &p11) == 0);
    assert(stabilizer_probability_of(collapsed, bits01, 2, &p01) == 0);
    assert(near(p00 + p11, 1.0) && p01 < 1e-12);
    stabilizer_free(collapsed);

    assert(stabilizer_run_circuit(NULL) == NULL);
    assert(stabilizer_to_stim_format(NULL) == NULL);
}

static void test_measurement_credentials(void) {
    /* Standalone descriptor reading qubits [2, 0] (result bit order). */
    uint32_t qubits[2] = {2, 0};
    CMeasurement* m = measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits, 2);
    assert(m != NULL);

    assert(measurement_qubits_len(m) == 2);
    assert(measurement_width(m) == 2);
    uint32_t buffer[2] = {99, 99};
    assert(measurement_qubits(m, buffer, 2) == 0);
    assert(buffer[0] == 2 && buffer[1] == 0);
    assert(measurement_qubits(m, buffer, 1) == -8);

    uint32_t tag = 99, width = 99;
    assert(measurement_ty(m, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 2);

    /* Value / expression chain. */
    CClassicalValue* value = measurement_value(m);
    assert(value != NULL);
    assert(classical_value_index(value) == 0);
    assert(classical_value_ty(value, &tag, &width) == 0);
    assert(tag == CQLIB_CLASSICAL_TYPE_BIT_VEC && width == 2);
    CClassicalExpr* value_expr = classical_value_expr(value);
    assert(value_expr != NULL);
    uint32_t kind = 99;
    assert(classical_expr_kind(value_expr, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_VALUE);
    classical_expr_free(value_expr);
    classical_value_free(value);

    CClassicalExpr* expr = measurement_expr(m);
    assert(expr != NULL);
    assert(classical_expr_kind(expr, &kind) == 0);
    assert(kind == CQLIB_CLASSICAL_EXPR_VALUE);
    classical_expr_free(expr);

    /* Qubit-order validation and projection. */
    assert(measurement_check_qubits(m, 3) == 0);
    assert(measurement_check_qubits(m, 2) == -2);
    char* projected = measurement_project(m, "101");
    assert(projected != NULL && strcmp(projected, "11") == 0);
    cqlib_string_free(projected);
    projected = measurement_project_basis(m, 5 /* 0b101 */);
    assert(projected != NULL && strcmp(projected, "11") == 0);
    cqlib_string_free(projected);
    assert(measurement_project(m, "10a") == NULL);
    measurement_free(m);

    /* Constructor error paths. */
    assert(measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, NULL, 2) == NULL);
    assert(measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits, 0) == NULL);
    assert(measurement_new(0, 99, 2, qubits, 2) == NULL);

    /* NULL handling. */
    measurement_free(NULL);
    classical_value_free(NULL);
    assert(measurement_value(NULL) == NULL);
    assert(measurement_qubits_len(NULL) == 0);
    assert(measurement_width(NULL) == 0);
    assert(measurement_ty(NULL, &tag, &width) == -1);
    assert(classical_value_index(NULL) == UINT32_MAX);
    assert(classical_value_expr(NULL) == NULL);
}

int main(void) {
    test_statevector_supplement();
    test_density_matrix_supplement();
    test_stabilizer_supplement();
    test_measurement_credentials();
    printf("binding-c qis measurement tests passed\n");
    return 0;
}
