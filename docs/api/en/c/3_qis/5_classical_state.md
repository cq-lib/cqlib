# Classical State (C)

This page covers three handle kinds: the measurement descriptor `CMeasurement` (binding measured qubits to the result bit order), the immutable classical value `CClassicalValue` produced by a measurement, and the sampling outcome list `COutcomeList`. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Measurement Descriptors

### measurement_new(value_index, ty_tag, ty_width, qubits, len)

Creates a standalone measurement descriptor from raw components: `qubits[0]` becomes result bit 0 (the least-significant bit), `qubits[1]` becomes result bit 1, and so on.

This constructor targets the self-contained qubit-order utilities (`measurement_project`, `measurement_project_basis`, `measurement_check_qubits`): its internal classical value is built with a fresh circuit identity and belongs to no circuit, so expressions derived from it are rejected by `circuit_validate_classical_expr`; build circuit-integrated measurements with `circuit_measure` / `circuit_measure_bits` instead (see [Classical Data and Control Flow](../0_circuit/9_classical_control_flow.md)).

Parameters:

- `value_index` (`uint32_t`): value-table index.
- `ty_tag` (`uint32_t`): result type tag, one of `CQLIB_CLASSICAL_TYPE_*` (`Bit` and `BitVec` are the direct measurement-target types).
- `ty_width` (`uint32_t`): result type width.
- `qubits` (`const uint32_t*`): measured qubit indices in result bit order.
- `len` (`uintptr_t`): number of measured qubits.

Returns: a new `CMeasurement*` on success; NULL for NULL `qubits`, an empty list, or an invalid type tag.

### measurement_free(ptr)

Frees a measurement handle; NULL is allowed.

### measurement_value(ptr)

Returns the immutable classical value produced by this measurement.

Returns: a new `CClassicalValue*` (free with `classical_value_free`); NULL on NULL input.

### measurement_expr(ptr)

Creates an expression that reads this measurement's immutable value.

Returns: a new `CClassicalExpr*` (free with `classical_expr_free`); NULL on NULL input.

### measurement_qubits_len(ptr) / measurement_qubits(ptr, buffer, len)

Two-step read of the measured qubit indices (in result bit order): `*_len` returns the count (0 for NULL), and the fill function copies the indices into `buffer`.

Returns (fill function): 0 on success; -1 for NULL pointers; -8 when `len` differs from `measurement_qubits_len`.

### measurement_width(ptr)

Returns the number of measured bits; 0 for NULL.

### measurement_ty(ptr, tag, width)

Writes the result type to the out-parameters.

Returns: 0 on success (`*tag` is one of `CQLIB_CLASSICAL_TYPE_*`, `*width` is the type width); -1 for NULL pointers.

### measurement_check_qubits(ptr, num_qubits)

Checks that all measured qubits are valid for a state of `num_qubits` qubits.

Returns: 0 on success; -1 for NULL input; -2 when a qubit index is out of bounds.

### measurement_project(ptr, bitstring)

Projects a full-register outcome onto this measurement's qubit order: `bitstring` is a big-endian `'0'`/`'1'` string over the full register (leftmost character is the highest qubit, matching `statevector_measure_all` output); if `qubits[i]` is one in the outcome, result bit `i` is set.

Returns: the projected outcome of `measurement_width` bits as a big-endian bitstring; free with `cqlib_string_free`. NULL on NULL input, invalid UTF-8, or an invalid bitstring.

### measurement_project_basis(ptr, basis)

Projects a computational-basis index onto this measurement's qubit order: if bit `qubits[i]` is one in `basis`, result bit `i` is set.

Returns: the projected outcome of `measurement_width` bits as a big-endian bitstring; free with `cqlib_string_free`. NULL on NULL input.

---

## Classical Values

### classical_value_free(ptr)

Frees a classical value handle; NULL is allowed.

### classical_value_index(ptr)

Returns the circuit-local value-table index; `UINT32_MAX` for NULL.

### classical_value_ty(ptr, tag, width)

Writes the value type to the out-parameters.

Returns: 0 on success (`*tag` is one of `CQLIB_CLASSICAL_TYPE_*`, `*width` is the type width); -1 for NULL pointers.

### classical_value_expr(ptr)

Creates an expression that reads this immutable runtime value.

Returns: a new `CClassicalExpr*` (free with `classical_expr_free`); NULL on NULL input.

---

## Sampling Outcome Lists

The `*_sample_shots` entry points (see [Statevector](1_statevector.md), [Stabilizer](4_stabilizer.md), etc.) return a `COutcomeList*`: each entry is a big-endian binary bitstring whose leftmost character corresponds to qubit N-1 and rightmost character to qubit 0.

### outcome_list_free(ptr)

Frees an outcome list; NULL is allowed.

### outcome_list_len(ptr)

Returns the number of bitstrings; 0 for NULL.

### outcome_list_get(ptr, index)

Returns the bitstring at `index`.

Returns: a heap-allocated C string; free with `cqlib_string_free`. NULL on out-of-bounds.

---

## Example

Full flow of declaring a measurement, sampling, and traversing the OutcomeList:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build the Bell circuit and declare a measurement */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);
    if (circuit_measure(qc, 0) != 0) {
        circuit_free(qc);
        return 1;
    }

    /* 2. Exact simulation, then 100 samples */
    struct CStatevector *sv = statevector_from_circuit(qc);
    struct COutcomeList *shots = statevector_sample_shots(sv, 100);

    /* 3. Traverse the OutcomeList (big-endian: leftmost char is qubit N-1) */
    uintptr_t n = outcome_list_len(shots);  /* 100 */
    for (uintptr_t i = 0; i < n; i++) {
        char *bs = outcome_list_get(shots, i);  /* "00" or "11" */
        printf("%s\n", bs);
        cqlib_string_free(bs);
    }

    outcome_list_free(shots);
    statevector_free(sv);
    circuit_free(qc);

    /* 4. Standalone measurement: qubits[0] is result bit 0, full-register projection */
    uint32_t qubits[2] = {1, 0};
    struct CMeasurement *m =
        measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits, 2);
    char *proj = measurement_project(m, "01");  /* "10" */
    cqlib_string_free(proj);
    measurement_free(m);
    return 0;
}
```
