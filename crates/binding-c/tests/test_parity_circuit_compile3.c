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

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

static int close_enough(double a, double b) { return fabs(a - b) < 1e-9; }

static void assert_complex(Complex64 actual, double re, double im) {
    assert(close_enough(actual.re, re));
    assert(close_enough(actual.im, im));
}

/* Builds one symbolic complex from real/imaginary expression strings and
 * returns the parsed parameter handles so the caller can free them. */
static struct CSymbolicComplex* make_complex(const char* re_expr, const char* im_expr,
                                             struct CParameter** re_out,
                                             struct CParameter** im_out) {
    *re_out = param_parse(re_expr);
    *im_out = param_parse(im_expr);
    assert(*re_out != NULL);
    assert(*im_out != NULL);
    struct CSymbolicComplex* complex = symbolic_complex_new(*re_out, *im_out);
    assert(complex != NULL);
    return complex;
}

static void test_unitary_gate_matrix_for_params(void) {
    /* U(theta) = cos(theta) * I + i sin(theta) * X. */
    struct CParameter* re00 = NULL;
    struct CParameter* im00 = NULL;
    struct CParameter* re01 = NULL;
    struct CParameter* im01 = NULL;
    struct CParameter* re10 = NULL;
    struct CParameter* im10 = NULL;
    struct CParameter* re11 = NULL;
    struct CParameter* im11 = NULL;
    struct CSymbolicComplex* e00 = make_complex("cos(theta)", "0", &re00, &im00);
    struct CSymbolicComplex* e01 = make_complex("0", "sin(theta)", &re01, &im01);
    struct CSymbolicComplex* e10 = make_complex("0", "sin(theta)", &re10, &im10);
    struct CSymbolicComplex* e11 = make_complex("cos(theta)", "0", &re11, &im11);
    const struct CSymbolicComplex* elements[4] = {e00, e01, e10, e11};
    struct CSymbolicMatrix* matrix = symbolic_matrix_new(2, 2, elements, 4);
    assert(matrix != NULL);
    assert(unitary_gate_matrix_for_params_len(matrix) == 4u);

    const char* names[1] = {"theta"};
    Complex64 buffer[4] = {{0.0, 0.0}};

    /* theta = 0 evaluates to the identity. */
    double zero[1] = {0.0};
    assert(unitary_gate_matrix_for_params(matrix, names, 1, zero, 1, buffer, 4) == 0);
    assert_complex(buffer[0], 1.0, 0.0);
    assert_complex(buffer[1], 0.0, 0.0);
    assert_complex(buffer[2], 0.0, 0.0);
    assert_complex(buffer[3], 1.0, 0.0);

    /* theta = pi/2 evaluates to iX. */
    double half_pi[1] = {M_PI / 2.0};
    assert(unitary_gate_matrix_for_params(matrix, names, 1, half_pi, 1, buffer, 4) == 0);
    assert_complex(buffer[0], 0.0, 0.0);
    assert_complex(buffer[1], 0.0, 1.0);
    assert_complex(buffer[2], 0.0, 1.0);
    assert_complex(buffer[3], 0.0, 0.0);

    /* Error paths. */
    assert(unitary_gate_matrix_for_params(NULL, names, 1, zero, 1, buffer, 4) == -1);
    assert(unitary_gate_matrix_for_params(matrix, names, 1, zero, 1, NULL, 4) == -1);
    assert(unitary_gate_matrix_for_params(matrix, NULL, 1, zero, 1, buffer, 4) == -1);
    assert(unitary_gate_matrix_for_params(matrix, names, 1, NULL, 1, buffer, 4) == -1);

    const char* null_names[1] = {NULL};
    assert(unitary_gate_matrix_for_params(matrix, null_names, 1, zero, 1, buffer, 4) == -1);

    /* Parameter-count mismatch and non-finite values map to -3. */
    assert(unitary_gate_matrix_for_params(matrix, names, 1, NULL, 0, buffer, 4) == -3);
    double nan_value[1] = {NAN};
    assert(unitary_gate_matrix_for_params(matrix, names, 1, nan_value, 1, buffer, 4) == -3);

    /* A too-small buffer maps to -8. */
    assert(unitary_gate_matrix_for_params(matrix, names, 1, zero, 1, buffer, 3) == -8);

    symbolic_matrix_free(matrix);
    symbolic_complex_free(e00);
    symbolic_complex_free(e01);
    symbolic_complex_free(e10);
    symbolic_complex_free(e11);
    param_free(re00);
    param_free(im00);
    param_free(re01);
    param_free(im01);
    param_free(re10);
    param_free(im10);
    param_free(re11);
    param_free(im11);

    /* A parameterless constant Pauli-X matrix evaluates without names. */
    struct CSymbolicComplex* zero_c = symbolic_complex_zero();
    struct CSymbolicComplex* one_c = symbolic_complex_one();
    assert(zero_c != NULL && one_c != NULL);
    const struct CSymbolicComplex* x_elements[4] = {zero_c, one_c, one_c, zero_c};
    struct CSymbolicMatrix* x_matrix = symbolic_matrix_new(2, 2, x_elements, 4);
    assert(x_matrix != NULL);
    Complex64 x_buffer[4] = {{9.0, 9.0}, {9.0, 9.0}, {9.0, 9.0}, {9.0, 9.0}};
    assert(unitary_gate_matrix_for_params(x_matrix, NULL, 0, NULL, 0, x_buffer, 4) == 0);
    assert_complex(x_buffer[0], 0.0, 0.0);
    assert_complex(x_buffer[1], 1.0, 0.0);
    assert_complex(x_buffer[2], 1.0, 0.0);
    assert_complex(x_buffer[3], 0.0, 0.0);

    /* A singular constant matrix fails the core unitarity check (-3). */
    const struct CSymbolicComplex* singular_elements[4] = {one_c, zero_c, zero_c, zero_c};
    struct CSymbolicMatrix* singular = symbolic_matrix_new(2, 2, singular_elements, 4);
    assert(singular != NULL);
    assert(unitary_gate_matrix_for_params(singular, NULL, 0, NULL, 0, x_buffer, 4) == -3);

    symbolic_matrix_free(singular);
    symbolic_matrix_free(x_matrix);
    symbolic_complex_free(zero_c);
    symbolic_complex_free(one_c);

    /* NULL/unsupported length queries return 0. */
    assert(unitary_gate_matrix_for_params_len(NULL) == 0u);
}

static void test_rewrite_config_target_instructions_setter(void) {
    CRewriteConfig config = rewrite_config_default();
    assert(config.mode == REWRITE_MODE_OPTIMIZE);
    assert(config.target_gate_names == NULL);
    assert(config.target_gate_names_len == 0u);

    /* NULL config, an empty basis and NULL arrays with a non-zero length. */
    assert(rewrite_config_with_target_instructions(NULL, NULL, 2) == -1);
    assert(rewrite_config_with_target_instructions(&config, NULL, 0) == -8);
    assert(rewrite_config_with_target_instructions(&config, NULL, 2) == -1);

    /* An unknown gate name maps to -4 and leaves the field unset. */
    const char* bad[1] = {"NOPE"};
    assert(rewrite_config_with_target_instructions(&config, bad, 1) == -4);
    assert(config.target_gate_names == NULL);
    assert(config.target_gate_names_len == 0u);

    /* A NULL name entry maps to -1. */
    const char* null_entry[1] = {NULL};
    assert(rewrite_config_with_target_instructions(&config, null_entry, 1) == -1);

    /* Valid names are stored on the by-value config. */
    const char* gates[2] = {"H", "CZ"};
    assert(rewrite_config_with_target_instructions(&config, gates, 2) == 0);
    assert(config.target_gate_names_len == 2u);
    assert(config.target_gate_names == gates);
}

static void test_rewrite_target_basis_lowering(void) {
    struct CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_cx(circuit, 0, 1) == 0);

    /* CX lowers to H-CZ-H against the {H, CZ} target basis. */
    const char* gates[2] = {"H", "CZ"};
    CRewriteConfig config = rewrite_config_lowering();
    assert(config.mode == REWRITE_MODE_LOWERING);
    assert(rewrite_config_with_target_instructions(&config, gates, 2) == 0);

    struct CKnowledgeRewriteResult* result = rewrite_circuit(circuit, config);
    assert(result != NULL);
    assert(knowledge_rewrite_result_changed(result) == 1);
    struct CCircuit* rewritten = knowledge_rewrite_result_circuit(result);
    assert(rewritten != NULL);
    assert(circuit_num_operations(rewritten) == 3u);
    circuit_free(rewritten);
    knowledge_rewrite_result_free(result);

    /* The standalone transform wrapper consumes the same config. */
    struct CCircuit* wrapped = transform_knowledge_rewrite(circuit, config);
    assert(wrapped != NULL);
    assert(circuit_num_operations(wrapped) == 3u);
    circuit_free(wrapped);

    /* A basis that cannot represent CX fails final-target validation. */
    const char* x_only[1] = {"X"};
    CRewriteConfig impossible = rewrite_config_lowering();
    assert(rewrite_config_with_target_instructions(&impossible, x_only, 1) == 0);
    assert(rewrite_circuit(circuit, impossible) == NULL);

    circuit_free(circuit);
}

static struct CDevice* line3_device(void) {
    const uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice* device = device_from_edges("line-3", 3, edges, 2);
    assert(device != NULL);
    return device;
}

static void test_layout_objective_tag(void) {
    /* The C tag follows the core constructor ordering. */
    assert(LAYOUT_OBJECTIVE_FIDELITY_REQUIRED == 3);
}

static void test_layout_fidelity_required_without_calibration(void) {
    struct CDevice* device = line3_device();
    struct CCircuit* circuit = circuit_new(2);
    assert(circuit_cx(circuit, 0, 1) == 0);

    struct CPhysicalLayoutGraph* graph = physical_layout_graph_from_device(device);
    assert(graph != NULL);
    assert(physical_layout_graph_has_fidelity_data(graph) == 0);

    /* Pointer-returning entries surface missing calibration as NULL. */
    assert(trivial_layout(circuit, device, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED) == NULL);
    struct CCircuitLayoutAnalysis* analysis = analyze_circuit_for_layout(circuit);
    assert(analysis != NULL);
    assert(trivial_layout_prepared(analysis, graph, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED) == NULL);

    /* The scoring entry reports the invalid configuration as -8. */
    const uint32_t qubits[2] = {0, 1};
    struct CLayout* layout = layout_new(qubits, 2, qubits, 2);
    assert(layout != NULL);
    CLayoutScore score = {0};
    assert(score_layout(LAYOUT_OBJECTIVE_FIDELITY_REQUIRED, analysis, graph, layout, &score) == -8);

    /* An unknown objective tag is rejected the same way. */
    assert(trivial_layout(circuit, device, 99) == NULL);
    assert(score_layout(99, analysis, graph, layout, &score) == -8);

    layout_free(layout);
    circuit_layout_analysis_free(analysis);
    physical_layout_graph_free(graph);
    circuit_free(circuit);
    device_free(device);
}

static void test_layout_fidelity_required_with_calibration(void) {
    struct CDevice* device = line3_device();
    assert(device_set_default_readout_error(device, 0.01) == 0);
    struct CCircuit* circuit = circuit_new(2);
    assert(circuit_cx(circuit, 0, 1) == 0);

    struct CPhysicalLayoutGraph* graph = physical_layout_graph_from_device(device);
    assert(graph != NULL);
    assert(physical_layout_graph_has_fidelity_data(graph) == 1);

    struct CLayoutResult* result =
        trivial_layout(circuit, device, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED);
    assert(result != NULL);
    layout_result_free(result);

    struct CCircuitLayoutAnalysis* analysis = analyze_circuit_for_layout(circuit);
    assert(analysis != NULL);
    struct CLayoutResult* prepared =
        trivial_layout_prepared(analysis, graph, LAYOUT_OBJECTIVE_FIDELITY_REQUIRED);
    assert(prepared != NULL);
    layout_result_free(prepared);

    /* The resolved objective carries fidelity terms into scoring. */
    const uint32_t qubits[2] = {0, 1};
    struct CLayout* layout = layout_new(qubits, 2, qubits, 2);
    assert(layout != NULL);
    CLayoutScore score = {0};
    assert(score_layout(LAYOUT_OBJECTIVE_FIDELITY_REQUIRED, analysis, graph, layout, &score) == 0);
    assert(score.used_fidelity == 1u);

    layout_free(layout);
    circuit_layout_analysis_free(analysis);
    physical_layout_graph_free(graph);
    circuit_free(circuit);
    device_free(device);
}

int main(void) {
    test_unitary_gate_matrix_for_params();
    test_rewrite_config_target_instructions_setter();
    test_rewrite_target_basis_lowering();
    test_layout_objective_tag();
    test_layout_fidelity_required_without_calibration();
    test_layout_fidelity_required_with_calibration();
    printf("binding-c parity circuit/compile tests passed\n");
    return 0;
}
