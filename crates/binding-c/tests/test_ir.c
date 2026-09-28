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

static const char* QASM2_SAMPLE =
    "OPENQASM 2.0;\n"
    "include \"qelib1.inc\";\n"
    "qreg q[2];\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

static const char* QCIS_SAMPLE = "H Q0\nCZ Q0 Q1\nM Q0 Q1";

static void test_qasm2_loads_and_dumps(void) {
    CCircuit* circuit = qasm2_loads(QASM2_SAMPLE);
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);

    char* dumped = qasm2_dumps(circuit);
    assert(dumped != NULL);
    assert(strstr(dumped, "h") != NULL);
    assert(strstr(dumped, "cx") != NULL);
    cqlib_string_free(dumped);
    circuit_free(circuit);
}

static void test_qasm2_errors(void) {
    assert(qasm2_loads(NULL) == NULL);
    assert(qasm2_loads("not valid qasm") == NULL);
    assert(qasm2_dumps(NULL) == NULL);
}

static void test_qasm2_file_roundtrip(void) {
    CCircuit* circuit = qasm2_loads(QASM2_SAMPLE);
    assert(circuit != NULL);

    const char* tmpdir = getenv("TEMP");
    if (tmpdir == NULL) {
        tmpdir = ".";
    }
    char path[512];
    snprintf(path, sizeof(path), "%s\\cqlib_c_test_qasm2_dump.qasm", tmpdir);

    assert(qasm2_dump(circuit, path) == 0);
    CCircuit* loaded = qasm2_load(path);
    assert(loaded != NULL);
    assert(circuit_num_qubits(loaded) == 2);
    circuit_free(loaded);
    remove(path);

    assert(qasm2_dump(NULL, path) == -1);
    assert(qasm2_dump(circuit, NULL) == -1);

    circuit_free(circuit);
}

static void test_qasm3_dumps(void) {
    CCircuit* circuit = qasm2_loads(QASM2_SAMPLE);
    assert(circuit != NULL);

    char* dumped = qasm3_dumps(circuit);
    assert(dumped != NULL);
    cqlib_string_free(dumped);
    circuit_free(circuit);
}

static void test_qcis_loads_and_dumps(void) {
    CCircuit* circuit = qcis_loads(QCIS_SAMPLE);
    assert(circuit != NULL);
    assert(circuit_num_qubits(circuit) == 2);

    char* dumped = qcis_dumps(circuit);
    assert(dumped != NULL);
    assert(strchr(dumped, 'H') != NULL || strchr(dumped, 'h') != NULL);
    cqlib_string_free(dumped);
    circuit_free(circuit);
}

static void test_qcis_errors(void) {
    assert(qcis_loads(NULL) == NULL);
    assert(qcis_loads("@@@INVALID_GARBAGE@@@") == NULL);
    assert(qcis_dumps(NULL) == NULL);
}

int main(void) {
    test_qasm2_loads_and_dumps();
    test_qasm2_errors();
    test_qasm2_file_roundtrip();
    test_qasm3_dumps();
    test_qcis_loads_and_dumps();
    test_qcis_errors();
    printf("binding-c ir tests passed\n");
    return 0;
}
