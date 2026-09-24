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
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static CStatevector* build_bell_state(void) {
    double inv = 0.7071067811865476;
    Complex64 amplitudes[4] = {
        {inv, 0.0},
        {0.0, 0.0},
        {0.0, 0.0},
        {inv, 0.0},
    };
    CStatevector* state = statevector_from_state(2, amplitudes, 4);
    assert(state != NULL);
    return state;
}

static CExecutionResult* build_bell_result(void) {
    const char* task_id = "task-visualization";
    uint32_t qubits[2] = {0, 1};
    CExecutionResult* result = execution_result_new(task_id, qubits, 2, 7, NULL);
    assert(result != NULL);
    const char* bitstrings[2] = {"00", "11"};
    uint64_t counts[2] = {2, 5};
    assert(execution_result_finish(result, bitstrings, counts, 2) == 0);
    return result;
}

static void assert_is_svg(const char* svg) {
    assert(svg != NULL);
    assert(strstr(svg, "<svg") == svg);
}

static void test_state_plots(void) {
    CStatevector* state = build_bell_state();

    char* svg = plot_bloch_multivector(state, NULL);
    assert_is_svg(svg);
    assert(strstr(svg, "q0") != NULL);
    assert(strstr(svg, "q1") != NULL);
    cqlib_string_free(svg);

    svg = plot_state_city(state, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    svg = plot_state_paulivec(state, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    /* NULL state must fail. */
    assert(plot_bloch_multivector(NULL, NULL) == NULL);
    assert(plot_state_city(NULL, NULL) == NULL);
    assert(plot_state_paulivec(NULL, NULL) == NULL);

    statevector_free(state);
}

static void test_plot_bloch_vector(void) {
    char* svg = plot_bloch_vector(0.0, 0.0, 1.0, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    /* Vectors longer than the unit sphere are clamped, not rejected. */
    svg = plot_bloch_vector(2.0, 0.0, 0.0, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);
}

static void test_state_plots_density_matrix(void) {
    double inv = 0.7071067811865476;
    Complex64 amplitudes[4] = {
        {inv, 0.0},
        {0.0, 0.0},
        {0.0, 0.0},
        {inv, 0.0},
    };
    CDensityMatrix* state = density_matrix_from_state(2, amplitudes, 4);
    assert(state != NULL);

    char* svg = plot_bloch_multivector_density_matrix(state, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    svg = plot_state_city_density_matrix(state, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    svg = plot_state_paulivec_density_matrix(state, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    density_matrix_free(state);
}

static void test_state_plot_options(void) {
    CStatevector* state = build_bell_state();

    CStatePlotOptions options;
    memset(&options, 0, sizeof(options));
    options.title = "Bell state";
    options.alpha = 0.75;
    options.reverse_bits = 1;
    options.has_figsize = 1;
    options.fig_width = 4.0;
    options.fig_height = 3.0;

    char* svg = plot_state_city(state, &options);
    assert_is_svg(svg);
    assert(strstr(svg, "Bell state") != NULL);
    cqlib_string_free(svg);

    statevector_free(state);
}

static void test_result_plots(void) {
    CExecutionResult* result = build_bell_result();

    char* svg = plot_histogram(result, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    svg = plot_distribution(result, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    CResultPlotOptions options;
    memset(&options, 0, sizeof(options));
    options.has_figsize = 1;
    options.fig_width = 3.2;
    options.fig_height = 2.4;
    options.number_to_keep = -1;
    options.bar_labels = 1;
    options.title = "Measurement counts";
    svg = plot_distribution(result, &options);
    assert_is_svg(svg);
    assert(strstr(svg, "Measurement counts") != NULL);
    cqlib_string_free(svg);

    /* NULL result must fail. */
    assert(plot_histogram(NULL, NULL) == NULL);
    assert(plot_distribution(NULL, NULL) == NULL);

    execution_result_free(result);
}

static void test_render_plots_to_file(void) {
    char* svg = plot_bloch_vector(0.0, 0.0, 1.0, NULL);
    assert(svg != NULL);

    const char* tmpdir = getenv("TEMP");
    if (tmpdir == NULL) {
        tmpdir = ".";
    }
    char path[512];
    snprintf(path, sizeof(path), "%s\\cqlib_c_test_state_plot.svg", tmpdir);

    assert(render_state_plot_to_file(svg, path) == 0);
    FILE* fp = fopen(path, "rb");
    assert(fp != NULL);
    fclose(fp);
    remove(path);

    /* NULL input and NULL path must fail. */
    assert(render_state_plot_to_file(NULL, path) == -1);
    assert(render_state_plot_to_file(svg, NULL) == -1);
    assert(render_result_plot_to_file(NULL, path) == -1);
    assert(render_result_plot_to_file(svg, NULL) == -1);
    cqlib_string_free(svg);
}

static void test_visual_ir(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);

    CVisualCircuit* visual = build_visual_circuit(circuit, NULL);
    assert(visual != NULL);
    assert(visual_circuit_num_qubits(visual) == 2);
    assert(visual_circuit_num_operations(visual) == 2);
    assert(visual_circuit_num_columns(visual) == 2);

    uint32_t qubits[2] = {99, 99};
    assert(visual_circuit_qubits_len(visual) == 2);
    assert(visual_circuit_qubits(visual, qubits, 2) == 0);
    assert(qubits[0] == 0 && qubits[1] == 1);

    char* label = visual_circuit_operation_label(visual, 0);
    assert(label != NULL);
    assert(strcmp(label, "H") == 0);
    cqlib_string_free(label);

    assert(visual_circuit_operation_column(visual, 0) == 0);
    assert(visual_circuit_operation_column(visual, 1) == 1);
    assert(visual_circuit_operation_style(visual, 0) == VISUAL_OP_STYLE_GATE);
    assert(visual_circuit_operation_style(visual, 1) == VISUAL_OP_STYLE_CONTROLLED);
    assert(visual_circuit_operation_num_controls(visual, 1) == 1);
    assert(visual_circuit_operation_is_span_box(visual, 0) == 0);

    uintptr_t lanes_len = visual_circuit_operation_lanes_len(visual, 1);
    assert(lanes_len == 2);
    uintptr_t lanes[2] = {99, 99};
    assert(visual_circuit_operation_lanes(visual, 1, lanes, 2) == 0);
    assert(lanes[0] == 0 && lanes[1] == 1);
    assert(visual_circuit_operation_covered_lanes_len(visual, 1) == 2);
    assert(visual_circuit_operation_covered_lanes(visual, 1, lanes, 2) == 0);

    char* text = draw_text_from_visual(visual, NULL);
    assert(text != NULL);
    assert(strchr(text, 'H') != NULL);
    cqlib_string_free(text);

    char* svg = draw_figure_from_visual(visual, NULL);
    assert_is_svg(svg);
    cqlib_string_free(svg);

    /* Explicit build options also work. */
    CVisualBuildOptions options;
    memset(&options, 0, sizeof(options));
    options.reserve_full_span_for_multi_qubit = 1;
    options.parameter_format.mode = PARAM_MODE_PI_FRACTION_PREFERRED;
    options.parameter_format.decimal_precision = 2;
    options.parameter_format.scientific_lower_bound = 1e-3;
    options.parameter_format.scientific_upper_bound = 1e4;
    options.parameter_format.pi_tolerance = 1e-3;
    options.parameter_format.pi_max_denominator = 16;
    CVisualCircuit* visual2 = build_visual_circuit(circuit, &options);
    assert(visual2 != NULL);
    assert(visual_circuit_num_operations(visual2) == 2);
    visual_circuit_free(visual2);

    /* Out-of-bounds access must fail gracefully. */
    assert(visual_circuit_operation_label(visual, 2) == NULL);
    assert(visual_circuit_operation_lanes_len(visual, 2) == 0);

    visual_circuit_free(visual);
    circuit_free(circuit);

    /* NULL input must fail. */
    assert(build_visual_circuit(NULL, NULL) == NULL);
    assert(visual_circuit_operation_label(NULL, 0) == NULL);
    assert(draw_text_from_visual(NULL, NULL) == NULL);
    assert(draw_figure_from_visual(NULL, NULL) == NULL);
    visual_circuit_free(NULL);
}

int main(void) {
    test_state_plots();
    test_plot_bloch_vector();
    test_state_plots_density_matrix();
    test_state_plot_options();
    test_result_plots();
    test_render_plots_to_file();
    test_visual_ir();
    printf("binding-c visualization plot tests passed\n");
    return 0;
}
