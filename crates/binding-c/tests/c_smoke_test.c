/*
 * cqlib C binding smoke test (real C compile + run).
 *
 * Build (MSVC):
 *   cl /nologo /W3 /I crates\binding-c\include /Fe:zne-test\c_smoke_test.exe ^
 *      crates\binding-c\tests\c_smoke_test.c /link target\debug\binding_c.dll.lib
 * Run: target\debug\binding_c.dll must be on the search path.
 */

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "cqlib_c.h"

static int failures = 0;

#define CHECK(cond, msg)                                     \
    do {                                                     \
        if (cond) {                                          \
            printf("[C-PASS] %s\n", msg);                   \
        } else {                                             \
            printf("[C-FAIL] %s\n", msg);                   \
            failures++;                                      \
        }                                                    \
    } while (0)

int main(void) {
    /* 1. Lifecycle and structural queries */
    struct CCircuit *c3 = circuit_new(3);
    CHECK(c3 != NULL, "circuit_new(3) != NULL");
    CHECK(circuit_num_qubits(c3) == 3, "circuit_num_qubits == 3");
    CHECK(circuit_num_operations(c3) == 0, "initial num_operations == 0");
    CHECK(circuit_validate(c3) == 0, "empty circuit validate == 0");
    circuit_free(c3);

    /* 2. Gate append and error codes */
    struct CCircuit *bell = circuit_new(2);
    CHECK(circuit_h(bell, 0) == 0, "circuit_h(0) == 0");
    CHECK(circuit_cx(bell, 0, 1) == 0, "circuit_cx(0,1) == 0");
    CHECK(circuit_num_operations(bell) == 2, "num_operations == 2");
    CHECK(circuit_h(bell, 2) == -2, "out-of-range gate returns -2");
    CHECK(circuit_rx(bell, 0, NAN) == -3, "NaN parameter returns -3");

    /* 3. Statevector simulation: Bell probabilities */
    struct CStatevector *sv = statevector_from_circuit(bell);
    CHECK(sv != NULL, "statevector_from_circuit != NULL");
    uintptr_t plen = statevector_probabilities_len(sv);
    CHECK(plen == 4, "probabilities length == 4");
    double probs[4] = {0};
    CHECK(statevector_probabilities(sv, probs, plen) == 0, "statevector_probabilities == 0");
    CHECK(fabs(probs[0] - 0.5) < 1e-12 && fabs(probs[3] - 0.5) < 1e-12,
          "Bell probs |00>/|11> == 0.5");
    CHECK(probs[1] == 0.0 && probs[2] == 0.0, "Bell probs |01>/|10> == 0");

    /* 4. Shot sampling */
    struct COutcomeList *list = statevector_sample_shots(sv, 512);
    CHECK(list != NULL && outcome_list_len(list) == 512, "sampled 512 outcomes");
    int all_valid = 1;
    int n00 = 0, n11 = 0;
    for (uintptr_t i = 0; i < 512; ++i) {
        char *item = outcome_list_get(list, i);
        if (item == NULL) {
            all_valid = 0;
            break;
        }
        if (strcmp(item, "00") == 0) {
            n00++;
        } else if (strcmp(item, "11") == 0) {
            n11++;
        } else {
            all_valid = 0;
        }
        cqlib_string_free(item);
    }
    printf("  shots: \"00\"=%d, \"11\"=%d\n", n00, n11);
    CHECK(all_valid, "all sampled outcomes are 00 or 11");
    outcome_list_free(list);

    /* 5. Unitary matrix export */
    uintptr_t mlen = circuit_to_matrix_len(bell, NULL, 0);
    CHECK(mlen == 16, "matrix has 16 complex entries");
    double buf[32] = {0};
    CHECK(circuit_to_matrix(bell, NULL, 0, buf, 32) == 16, "to_matrix wrote 16 entries");
    CHECK(fabs(buf[0] - sqrt(0.5)) < 1e-12, "m[0][0].re == 1/sqrt(2)");
    CHECK(fabs(buf[24] - sqrt(0.5)) < 1e-12, "m[3][3].re == 1/sqrt(2)");

    /* 6. Compile pipeline and step report */
    struct CompileConfigC cfg;
    memset(&cfg, 0, sizeof(cfg));
    struct CCompileResult *res = compile(bell, cfg);
    CHECK(res != NULL, "compile != NULL");
    uintptr_t steps = compile_result_num_steps(res);
    printf("  compile steps=%llu changed=%d\n", (unsigned long long)steps,
           (int)compile_result_changed(res));
    CHECK(steps > 0, "compile produced steps > 0");
    char *name0 = compile_result_step_name(res, 0);
    CHECK(name0 != NULL && strlen(name0) > 0, "step 0 name non-empty");
    cqlib_string_free(name0);
    struct CCircuit *compiled = compile_result_circuit(res);
    CHECK(compiled != NULL && circuit_num_qubits(compiled) == 2, "compiled circuit 2 qubits");
    circuit_free(compiled);
    compile_result_free(res);

    /* 7. QASM2 round-trip and string free discipline */
    const char *src =
        "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\nh q[0];\ncx q[0],q[1];\n";
    struct CCircuit *qc = qasm2_loads(src);
    CHECK(qc != NULL && circuit_num_operations(qc) == 2, "qasm2 loads 2 operations");
    char *dump = qasm2_dumps(qc);
    CHECK(dump != NULL && strstr(dump, "h q[0]") != NULL, "qasm2 dump contains h");
    cqlib_string_free(dump);
    circuit_free(qc);
    CHECK(qasm2_loads("not qasm") == NULL, "invalid QASM returns NULL");

    /* 8. In-place apply and measurement */
    struct CCircuit *xc = circuit_new(2);
    circuit_x(xc, 0);
    struct CStatevector *xv = statevector_new(2);
    CHECK(statevector_apply_circuit(xv, xc) == 0, "apply_circuit == 0");
    CHECK(statevector_measure(xv, 0) == 1, "qubit0 measures 1");
    CHECK(statevector_measure(xv, 1) == 0, "qubit1 measures 0");
    char *all = statevector_measure_all(xv);
    CHECK(all != NULL && strcmp(all, "01") == 0, "measure_all == \"01\"");
    cqlib_string_free(all);
    statevector_free(xv);
    circuit_free(xc);

    /* 9. NULL-pointer safety */
    CHECK(circuit_num_qubits(NULL) == 0, "NULL query returns 0");
    CHECK(circuit_validate(NULL) == -1, "NULL validate returns -1");
    circuit_free(NULL);
    statevector_free(NULL);
    cqlib_string_free(NULL);

    statevector_free(sv);
    circuit_free(bell);

    if (failures == 0) {
        printf("\nC SMOKE TEST: ALL PASSED\n");
        return 0;
    }
    printf("\nC SMOKE TEST: %d FAILED\n", failures);
    return failures;
}