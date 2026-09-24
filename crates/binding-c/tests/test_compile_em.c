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
#include <string.h>

#include "cqlib_c.h"

static CCircuit* build_bell_circuit(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    return circuit;
}

/* Demo estimator: reports the folded circuit's operation count as the
 * "expectation value", so the sequence mirrors the folding shape. */
static void counting_estimator(const CCircuit* circuit, const CHamiltonian* hamiltonian,
                               uintptr_t shots, double* expectation, double* variance) {
    (void)hamiltonian;
    (void)shots;
    *expectation = (double)circuit_num_operations(circuit);
    *variance = 0.0;
}

/* Static estimator for virtual distillation: numerator invocations
 * (Hamiltonian present) report (4.0, 0.1); denominator invocations
 * (Hamiltonian NULL) report (2.0, 0.05). */
static void static_vd_estimator(const CCircuit* circuit, const CHamiltonian* hamiltonian,
                                uintptr_t shots, double* expectation, double* variance) {
    (void)circuit;
    (void)shots;
    if (hamiltonian != NULL) {
        *expectation = 4.0;
        *variance = 0.1;
    } else {
        *expectation = 2.0;
        *variance = 0.05;
    }
}

/* ===== Compile: step query by name + device-target compilation ===== */

static void test_compile_result_step_by_name(void) {
    CCircuit* circuit = build_bell_circuit();

    CompileConfigC config;
    memset(&config, 0, sizeof(config));
    config.mode = COMPILE_MODE_ENHANCED;
    config.target = COMPILE_TARGET_LOGICAL;

    CCompileResult* result = compile(circuit, config);
    assert(result != NULL);

    /* A logical-target workflow reports route.sabre as skipped. */
    int32_t changed = -1;
    int32_t skipped = -1;
    assert(compile_result_step(result, "route.sabre", &changed, &skipped) == 1);
    assert(changed == 0 || changed == 1);
    assert(skipped == 1);

    /* Every reported step name must be findable by name. */
    uintptr_t steps = compile_result_num_steps(result);
    assert(steps > 0);
    for (uintptr_t i = 0; i < steps; i++) {
        char* name = compile_result_step_name(result, i);
        assert(name != NULL);
        assert(compile_result_step(result, name, &changed, &skipped) == 1);
        cqlib_string_free(name);
    }

    /* Unknown names report "not found"; NULL arguments report -1. */
    assert(compile_result_step(result, "no.such_step", &changed, &skipped) == 0);
    assert(compile_result_step(NULL, "route.sabre", &changed, &skipped) == -1);
    assert(compile_result_step(result, NULL, &changed, &skipped) == -1);

    compile_result_free(result);
    circuit_free(circuit);
}

static void test_compile_with_device_layouts(void) {
    CCircuit* circuit = build_bell_circuit();

    CDevice* device = device_bidirectional_line("line-2", 2);
    assert(device != NULL);
    assert(device_with_native_gates(device, "H,CX") == 0);

    /* Device-target compilation routes on the topology and records layouts. */
    CCompileResult* result = compile_with_device(circuit, COMPILE_MODE_NORMAL, device, NULL, 7);
    assert(result != NULL);

    CLayout* initial = compile_result_initial_layout(result);
    assert(initial != NULL);
    assert(layout_num_logical(initial) == 2);
    layout_free(initial);

    CLayout* final_layout = compile_result_final_layout(result);
    assert(final_layout != NULL);
    layout_free(final_layout);

    /* Logical-target compilation carries no device metadata. */
    CompileConfigC config;
    memset(&config, 0, sizeof(config));
    config.mode = COMPILE_MODE_NORMAL;
    config.target = COMPILE_TARGET_LOGICAL;
    CCompileResult* logical = compile(circuit, config);
    assert(logical != NULL);
    assert(compile_result_initial_layout(logical) == NULL);
    assert(compile_result_final_layout(logical) == NULL);

    /* NULL and invalid-argument handling. */
    assert(compile_with_device(NULL, COMPILE_MODE_NORMAL, device, NULL, 7) == NULL);
    assert(compile_with_device(circuit, COMPILE_MODE_NORMAL, NULL, NULL, 7) == NULL);
    assert(compile_with_device(circuit, 42, device, NULL, 7) == NULL);
    assert(compile_result_initial_layout(NULL) == NULL);
    assert(compile_result_final_layout(NULL) == NULL);

    compile_result_free(result);
    compile_result_free(logical);
    device_free(device);
    circuit_free(circuit);
}

/* ===== ZNE: fine-grained folding / extrapolation / sampling ===== */

static CZneMitigation* build_zne(CCircuit* circuit) {
    const int32_t levels[] = {0, 1, 2};
    CZneMitigation* zne = zne_mitigation_new(circuit, levels, 3);
    assert(zne != NULL);
    return zne;
}

static void test_zne_circuit_levels_factors(void) {
    CCircuit* circuit = build_bell_circuit();
    CZneMitigation* zne = build_zne(circuit);

    /* zne_circuit returns the original (unfolded) circuit. */
    CCircuit* original = zne_circuit(zne);
    assert(original != NULL);
    assert(circuit_num_operations(original) == 2);
    circuit_free(original);

    /* Fold levels and noise factors roundtrip through the two-step pattern. */
    int32_t levels[3] = {0, 0, 0};
    assert(zne_fold_levels_len(zne) == 3);
    assert(zne_fold_levels(zne, levels, 3) == 0);
    assert(levels[0] == 0 && levels[1] == 1 && levels[2] == 2);
    assert(zne_fold_levels(zne, levels, 2) == -8);

    int32_t factors[3] = {0, 0, 0};
    assert(zne_noise_factors_len(zne) == 3);
    assert(zne_noise_factors(zne, factors, 3) == 0);
    assert(factors[0] == 1 && factors[1] == 3 && factors[2] == 5);
    assert(zne_noise_factors(zne, factors, 4) == -8);

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

static void test_zne_fold_circuits_two_step(void) {
    CCircuit* circuit = build_bell_circuit();
    CZneMitigation* zne = build_zne(circuit);

    /* Global folding scales the circuit to (2 * level + 1) copies: the
     * 2-operation Bell circuit becomes 2 / 6 / 10 operations. */
    CCircuit* folded[3] = {NULL, NULL, NULL};
    assert(zne_fold_circuits_len(zne, NULL) == 3);
    assert(zne_fold_circuits(zne, NULL, folded, 3) == 0);
    assert(circuit_num_operations(folded[0]) == 2);
    assert(circuit_num_operations(folded[1]) == 6);
    assert(circuit_num_operations(folded[2]) == 10);
    for (int i = 0; i < 3; i++) {
        circuit_free(folded[i]);
    }

    /* Selective folding on H only keeps CX untouched (4 / 6 operations). */
    assert(zne_fold_circuits_len(zne, "H") == 3);
    assert(zne_fold_circuits(zne, "H", folded, 3) == 0);
    assert(circuit_num_operations(folded[1]) == 4);
    assert(circuit_num_operations(folded[2]) == 6);
    for (int i = 0; i < 3; i++) {
        circuit_free(folded[i]);
    }

    /* Length mismatch and unknown gate names must fail. */
    CCircuit* sink = NULL;
    assert(zne_fold_circuits(zne, NULL, &sink, 2) == -8);
    assert(zne_fold_circuits(zne, "NOT_A_GATE", &sink, 3) == -4);
    assert(zne_fold_circuits_len(zne, "NOT_A_GATE") == 0);

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

static void test_zne_extrapolate_known_values(void) {
    CCircuit* circuit = build_bell_circuit();
    CZneMitigation* zne = build_zne(circuit);

    /* Linear data y = 2x + 1 over noise factors x = {1, 3, 5}: the degree-1
     * fit extrapolates exactly to y(0) = 1. */
    const double linear[3] = {3.0, 7.0, 11.0};
    double value = NAN;
    assert(zne_poly_extrapolate(zne, linear, 3, 1, &value) == 0);
    assert(fabs(value - 1.0) < 1e-9);

    /* The unified entry point with the method tag must agree. */
    assert(zne_extrapolate(zne, linear, 3, ZNE_EXTRAPOLATE_POLYNOMIAL, 1, &value) == 0);
    assert(fabs(value - 1.0) < 1e-9);

    /* Exponential decay y = exp(-x) extrapolates to A = 1 at x = 0. */
    const double decay[3] = {exp(-1.0), exp(-3.0), exp(-5.0)};
    assert(zne_exp_extrapolate(zne, decay, 3, &value) == 0);
    assert(fabs(value - 1.0) < 1e-9);
    assert(zne_extrapolate(zne, decay, 3, ZNE_EXTRAPOLATE_EXPONENTIAL, 0, &value) == 0);
    assert(fabs(value - 1.0) < 1e-9);

    /* Invalid inputs must fail: degree too large, wrong length, non-positive
     * data, unknown method tag. */
    assert(zne_poly_extrapolate(zne, linear, 3, 3, &value) != 0);
    assert(zne_poly_extrapolate(zne, linear, 2, 1, &value) != 0);
    const double bad[3] = {0.0, 1.0, 2.0};
    assert(zne_exp_extrapolate(zne, bad, 3, &value) != 0);
    assert(zne_extrapolate(zne, linear, 3, 42, 1, &value) != 0);

    zne_mitigation_free(zne);
    circuit_free(circuit);
}

static void test_zne_run_em_sequence(void) {
    CCircuit* circuit = build_bell_circuit();
    CZneMitigation* zne = build_zne(circuit);

    CPauliString* pauli = pauli_string_parse("ZZ");
    assert(pauli != NULL);
    CHamiltonian* ham = hamiltonian_from_pauli(pauli); /* takes ownership */
    assert(ham != NULL);

    /* The counting estimator reports the folded circuit lengths. */
    double sequence[3] = {NAN, NAN, NAN};
    assert(zne_run_em_sequence_len(zne) == 3);
    assert(zne_run_em_sequence(zne, NULL, ham, counting_estimator, sequence, 3) == 0);
    assert(sequence[0] == 2.0 && sequence[1] == 6.0 && sequence[2] == 10.0);

    /* The with-shots variant supports selective folding. */
    assert(zne_run_em_sequence_with_shots_len(zne) == 3);
    assert(zne_run_em_sequence_with_shots(zne, "H", ham, 128, counting_estimator, sequence, 3) ==
           0);
    assert(sequence[0] == 2.0 && sequence[1] == 4.0 && sequence[2] == 6.0);

    /* Buffer mismatch and unknown gate names must fail. */
    double sink[2] = {0.0, 0.0};
    assert(zne_run_em_sequence(zne, NULL, ham, counting_estimator, sink, 2) == -8);
    assert(zne_run_em_sequence(zne, "NOT_A_GATE", ham, counting_estimator, sequence, 3) == -4);
    assert(zne_run_em_sequence(NULL, NULL, ham, counting_estimator, sequence, 3) == -1);

    hamiltonian_free(ham);
    zne_mitigation_free(zne);
    circuit_free(circuit);
}

/* ===== Virtual distillation: copies / sampling ===== */

static void test_virtual_distillation_fine_grained(void) {
    CCircuit* circuit = circuit_new(1);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);

    CVirtualDistillation* vd = virtual_distillation_new(circuit, 2);
    assert(vd != NULL);

    /* Copies roundtrip; values below 2 are rejected. */
    assert(virtual_distillation_copies(vd) == 2);
    assert(virtual_distillation_set_copies(vd, 3) == 0);
    assert(virtual_distillation_copies(vd) == 3);
    assert(virtual_distillation_set_copies(vd, 1) == -8);
    assert(virtual_distillation_copies(vd) == 3);
    assert(virtual_distillation_set_copies(vd, 2) == 0);

    /* The copy-swap circuit is non-empty. */
    CCircuit* protocol = virtual_distillation_build_circuit(vd);
    assert(protocol != NULL);
    assert(circuit_num_operations(protocol) > 0);
    circuit_free(protocol);

    /* run_vd normalizes the numerator by the denominator: 4 / 2 = 2 with
     * var = 0.1 / 4 + 16 * 0.05 / 16 = 0.075. */
    CPauliString* z_pauli = pauli_string_parse("Z");
    assert(z_pauli != NULL);
    CHamiltonian* ham = hamiltonian_from_pauli(z_pauli); /* takes ownership */
    assert(ham != NULL);

    double mean = NAN;
    double variance = NAN;
    assert(virtual_distillation_run_vd(vd, ham, 100, 100, static_vd_estimator, &mean, &variance) ==
           0);
    assert(fabs(mean - 2.0) < 1e-12);
    assert(fabs(variance - 0.075) < 1e-12);

    /* A Hamiltonian wider than the circuit must fail. */
    CPauliString* wide_pauli = pauli_string_parse("ZZ");
    assert(wide_pauli != NULL);
    CHamiltonian* wide = hamiltonian_from_pauli(wide_pauli); /* takes ownership */
    assert(wide != NULL);
    assert(virtual_distillation_run_vd(vd, wide, 100, 100, static_vd_estimator, &mean, &variance) ==
           -8);
    hamiltonian_free(wide);

    /* NULL handling. */
    assert(virtual_distillation_copies(NULL) == 0);
    assert(virtual_distillation_set_copies(NULL, 3) == -1);
    assert(virtual_distillation_run_vd(NULL, ham, 100, 100, static_vd_estimator, &mean,
                                       &variance) == -1);

    hamiltonian_free(ham);
    virtual_distillation_free(vd);
    circuit_free(circuit);
}

int main(void) {
    test_compile_result_step_by_name();
    test_compile_with_device_layouts();
    test_zne_circuit_levels_factors();
    test_zne_fold_circuits_two_step();
    test_zne_extrapolate_known_values();
    test_zne_run_em_sequence();
    test_virtual_distillation_fine_grained();
    printf("binding-c compile + error mitigation fine-grained tests passed\n");
    return 0;
}
