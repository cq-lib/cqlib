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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cqlib_c.h"

static CCircuit* build_bell_circuit(void) {
    CCircuit* circuit = circuit_new(2);
    assert(circuit != NULL);
    assert(circuit_h(circuit, 0) == 0);
    assert(circuit_cx(circuit, 0, 1) == 0);
    return circuit;
}

static void test_circuit_to_text(void) {
    CCircuit* circuit = build_bell_circuit();

    char* text = circuit_to_text(circuit, NULL);
    assert(text != NULL);
    assert(strchr(text, 'H') != NULL);
    /* CX renders as a control dot with an "X" target label. */
    assert(strchr(text, 'X') != NULL);
    cqlib_string_free(text);

    TextDrawerOptionsC options;
    memset(&options, 0, sizeof(options));
    options.show_params = 1;
    options.line_width = -1;
    options.initial_state = 1;
    text = circuit_to_text(circuit, &options);
    assert(text != NULL);
    cqlib_string_free(text);

    assert(circuit_to_text(NULL, NULL) == NULL);
    circuit_free(circuit);
}

static void test_circuit_to_figure(void) {
    CCircuit* circuit = build_bell_circuit();

    char* svg = circuit_to_figure(circuit, NULL);
    assert(svg != NULL);
    assert(strstr(svg, "<svg") != NULL);
    cqlib_string_free(svg);

    FigureDrawerOptionsC options;
    memset(&options, 0, sizeof(options));
    options.show_params = 1;
    options.width_per_column = 1.5;
    options.height_per_qubit = 1.0;
    options.dpi = 160;
    options.fold = -1;
    svg = circuit_to_figure(circuit, &options);
    assert(svg != NULL);
    cqlib_string_free(svg);

    assert(circuit_to_figure(NULL, NULL) == NULL);
    circuit_free(circuit);
}

static void test_render_figure_to_file(void) {
    CCircuit* circuit = build_bell_circuit();

    const char* tmpdir = getenv("TEMP");
    if (tmpdir == NULL) {
        tmpdir = ".";
    }
    char path[512];
    snprintf(path, sizeof(path), "%s\\cqlib_c_test_circuit.svg", tmpdir);

    assert(render_figure_to_file(circuit, path, NULL) == 0);
    FILE* fp = fopen(path, "rb");
    assert(fp != NULL);
    fclose(fp);
    remove(path);

    /* NULL path and NULL circuit must fail. */
    assert(render_figure_to_file(circuit, NULL, NULL) == -1);
    assert(render_figure_to_file(NULL, path, NULL) == -1);

    circuit_free(circuit);
}

int main(void) {
    test_circuit_to_text();
    test_circuit_to_figure();
    test_render_figure_to_file();
    printf("binding-c visualization tests passed\n");
    return 0;
}
