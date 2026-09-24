# Operation / Instruction

`COperation*` and `CValueOperation*` are two read-only views of circuit operations. In the storage-layer view `COperation`, a parameter may be recorded as an index into the owning circuit's parameter table and can only be resolved with circuit context; `circuit_index` returns a `CValueOperation`, a resolved self-contained snapshot whose parameters are `CParameterValue` values (a fixed number or a full symbolic expression), independent of later circuit mutations. Delay instructions are covered at the end of this page. Gate functions and the gate-name list are covered in [Standard Gates](5_gate_standard.md); the memory rules of `CParameterValue` are described in [Parameter](3_parameter.md); error codes and memory conventions are described in [Overview](../0_overview.md).

---

## Storage-Layer Operations

A `COperation*` represents one concrete application of "instruction + acting qubits + parameters + optional label". `operation_new` creates a standard-gate operation from a gate name with fixed double parameters.

### operation_new(gate_name, qubits, qubits_len, params, params_len)

Creates an operation from a standard gate name (e.g. `"H"`, `"CX"`, `"RZZ"`); `params` holds fixed double values.

- `gate_name` (`const char*`): standard gate name.
- `qubits` (`const uint32_t*`): array of acting qubit ids.
- `qubits_len` (`uintptr_t`): number of qubits.
- `params` (`const double*`): array of fixed parameter values.
- `params_len` (`uintptr_t`): number of parameters.

Returns: a newly allocated `COperation*`, freed with `operation_free`; `NULL` when `gate_name` is NULL or names an unknown gate; also `NULL` when `qubits` is NULL while `qubits_len > 0`, or `params` is NULL while `params_len > 0`.

### operation_free(ptr)

Frees an operation handle. Passing NULL is allowed.

- `ptr` (`COperation*`): operation handle.

### operation_name(ptr)

Returns the instruction name (e.g. `"H"`, `"RZZ"`).

- `ptr` (`const COperation*`): operation handle.

Returns: a newly allocated C string, freed with `cqlib_string_free`; `NULL` on NULL input.

### operation_is_standard_gate(ptr)

Returns whether the operation is driven by a standard-gate instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_num_qubits(ptr)

Returns the number of qubits the operation acts on.

- `ptr` (`const COperation*`): operation handle.

Returns: the qubit count; 0 for NULL.

### operation_qubits_len(ptr)

Returns the number of qubits the operation acts on, used to size the buffer for `operation_qubits`.

- `ptr` (`const COperation*`): operation handle.

Returns: the qubit count; 0 for NULL.

### operation_qubits(ptr, buffer, len)

Copies the qubit ids into `buffer` in application order.

- `ptr` (`const COperation*`): operation handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer length.

Returns: `0` on success; `-1` for NULL; `-8` when `len` is smaller than the qubit count.

### operation_params_len(ptr)

Returns the number of parameters carried by the operation.

- `ptr` (`const COperation*`): operation handle.

Returns: the parameter count; 0 for NULL.

### operation_params(ptr, tags, values, len)

Copies the parameters into the parallel arrays `tags` and `values`. `tags[i]` is one of the `OPERATION_PARAM_*` constants; `values[i]` is the fixed value, or — under the `OPERATION_PARAM_INDEX` tag — the parameter-table index (as a double).

- `ptr` (`const COperation*`): operation handle.
- `tags` (`uint32_t*`): output array of parameter tags.
- `values` (`double*`): output array of parameter values.
- `len` (`uintptr_t`): buffer length.

Returns: `0` on success; `-1` for NULL; `-8` when `len` is smaller than the parameter count.

Parameter tags:

| Constant | Value | Meaning |
| --- | --- | --- |
| `OPERATION_PARAM_FIXED` | 0 | Fixed value; `values[i]` is the value itself. |
| `OPERATION_PARAM_INDEX` | 1 | Index into the owning circuit's parameter table; `values[i]` is the index value and must be resolved against the circuit's table. |

### operation_label(ptr)

Returns the operation's optional metadata label.

- `ptr` (`const COperation*`): operation handle.

Returns: a newly allocated C string, freed with `cqlib_string_free`; `NULL` when the operation has no label or the handle is NULL.

```c
uint32_t qubits[2] = {0, 1};
double params[1] = {0.7};
COperation *op = operation_new("RZZ", qubits, 2, params, 1);
if (op != NULL) {
    char *name = operation_name(op);        /* "RZZ" */
    cqlib_string_free(name);

    uintptr_t n = operation_qubits_len(op);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    operation_qubits(op, ids, n);            /* ids == {0, 1} */
    free(ids);

    operation_free(op);
}
```

---

## Instruction Kind Predicates

Each predicate inspects which kind of instruction the operation carries; they all return `1` when yes, `0` when no, and `-1` for a NULL handle.

### operation_is_unitary(ptr)

Returns whether the operation uses a user-defined unitary instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_mcgate(ptr)

Returns whether the operation uses a multi-controlled-gate instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_circuit_gate(ptr)

Returns whether the operation uses a circuit-backed gate instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_directive(ptr)

Returns whether the operation uses a non-unitary directive instruction (barrier, measure, reset).

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_delay(ptr)

Returns whether the operation uses a delay instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_classical_control(ptr)

Returns whether the operation uses a structured classical-control-flow instruction (`if` / `while` / `for` / `switch` / `break` / `continue`).

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_classical_data(ptr)

Returns whether the operation uses a classical-data instruction (store, measure-into-value).

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_quantum_gate(ptr)

Returns whether the operation is a unitary quantum gate (standard, multi-controlled, user-defined unitary, or circuit-backed gate).

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### operation_is_instruction(ptr)

Returns whether the operation carries a plain instruction rather than structured classical control flow.

- `ptr` (`const COperation*`): operation handle.

Returns: `1` when yes; `0` for classical-control operations; `-1` for NULL.

---

## Instruction Introspection

### operation_gate_arity(ptr, qubits, params)

Writes the fixed `(qubit_count, parameter_count)` intrinsic to the instruction.

- `ptr` (`const COperation*`): operation handle.
- `qubits` (`uintptr_t*`): written qubit count.
- `params` (`uintptr_t*`): written parameter count.

Returns: `0` on success; `-1` for NULL; `-3` when the arity is variable (barrier, multi-qubit measurement, classical control).

### operation_as_instruction(ptr)

Returns an owned clone of the operation when it carries a plain (non-control-flow) instruction.

- `ptr` (`const COperation*`): operation handle.

Returns: a newly allocated `COperation*`, freed with `operation_free`; `NULL` on NULL input or when the operation is a classical-control instruction.

### operation_directive(ptr)

Returns the directive carried by the operation as one of the `OPERATION_DIRECTIVE_*` tags.

- `ptr` (`const COperation*`): operation handle.

Returns: the directive tag; `OPERATION_DIRECTIVE_NONE` (0) when the operation is not a directive; `-1` for NULL.

Directive tags:

| Constant | Value | Meaning |
| --- | --- | --- |
| `OPERATION_DIRECTIVE_NONE` | 0 | The operation carries no directive instruction. |
| `OPERATION_DIRECTIVE_BARRIER` | 1 | Barrier directive. |
| `OPERATION_DIRECTIVE_MEASURE` | 2 | Measure directive. |
| `OPERATION_DIRECTIVE_RESET` | 3 | Reset directive. |

### operation_result(ptr, out)

Writes a snapshot of the immutable classical value produced by a measurement operation into `out`: the circuit-local value table index plus the (tag, width) type parts (for the `CClassicalValueInfo` layout see [Classical Data and Control Flow](9_classical_control_flow.md)).

- `ptr` (`const COperation*`): operation handle.
- `out` (`CClassicalValueInfo*`): output struct.

Returns: `0` on success; `-1` for NULL; `-3` when the operation is not a measurement (no classical result).

### operation_reads_value(ptr, expr)

Returns whether the operation directly or recursively reads the immutable classical value referenced by `expr`.

- `ptr` (`const COperation*`): operation handle.
- `expr` (`const CClassicalExpr*`): expression handle referencing a classical value.

Returns: `1` when yes; `0` when no; `-1` for NULL; `-8` when `expr` is not a plain value expression.

---

## Operation Matrix

The three functions work together: query the length first, then fetch the dimensions and the data. Non-unitary instructions and operations with unresolved symbolic parameters have no numeric matrix.

### operation_matrix_len(ptr)

Returns the total number of complex elements (rows × cols) in the unitary matrix.

- `ptr` (`const COperation*`): operation handle.

Returns: the complex-element count; 0 on NULL input or when the operation has no numeric matrix (non-unitary instruction or unresolved symbolic parameter).

### operation_matrix_dims(ptr, rows, cols)

Writes the matrix shape into `rows` and `cols`.

- `ptr` (`const COperation*`): operation handle.
- `rows` (`uintptr_t*`): written row count.
- `cols` (`uintptr_t*`): written column count.

Returns: `0` on success; `-1` for NULL; `-3` when the operation has no numeric matrix.

### operation_matrix(ptr, out, buffer_len)

Copies the unitary matrix into `out` as interleaved `(re, im)` doubles in row-major order; the buffer must hold at least `2 × operation_matrix_len` doubles.

- `ptr` (`const COperation*`): operation handle.
- `out` (`double*`): output buffer.
- `buffer_len` (`uintptr_t`): buffer length (number of doubles).

Returns: the number of complex elements written; 0 on error.

```c
uint32_t q0 = 0;
COperation *h = operation_new("H", &q0, 1, NULL, 0);
if (h != NULL) {
    uintptr_t n = operation_matrix_len(h);              /* 4 (2x2 complex elements) */
    double *buf = malloc(2 * n * sizeof(double));
    uintptr_t written = operation_matrix(h, buf, 2 * n); /* written == 4 */
    free(buf);
    operation_free(h);
}
```

---

## Resolved Snapshots

`circuit_index` returns a resolved snapshot `CValueOperation*` of a top-level operation: read-only and self-contained, with parameters as `CParameterValue` values (a fixed number or a full symbolic expression; free the `param` field of the symbolic case with `param_free`, see [Parameter](3_parameter.md)), unaffected by later circuit mutations.

### circuit_index(ptr, index)

Returns a resolved snapshot of the top-level operation at `index`.

- `ptr` (`const CCircuit*`): circuit handle.
- `index` (`uintptr_t`): operation index.

Returns: a newly allocated `CValueOperation*`, freed with `value_operation_free`; `NULL` on NULL input or an out-of-bounds index.

### value_operation_free(ptr)

Frees a snapshot handle. Passing NULL is allowed.

- `ptr` (`CValueOperation*`): snapshot handle.

### value_operation_name(ptr)

Returns the operation's instruction name (e.g. `"H"`, `"delay"`).

- `ptr` (`const CValueOperation*`): snapshot handle.

Returns: a newly allocated C string, freed with `cqlib_string_free`; `NULL` on NULL input.

### value_operation_instruction_type(ptr)

Returns the operation's instruction category (e.g. `"standard"`, `"delay"`).

- `ptr` (`const CValueOperation*`): snapshot handle.

Returns: a newly allocated C string, freed with `cqlib_string_free`; `NULL` on NULL input.

### value_operation_label(ptr)

Returns the operation's optional metadata label.

- `ptr` (`const CValueOperation*`): snapshot handle.

Returns: a newly allocated C string, freed with `cqlib_string_free`; `NULL` when the operation has no label or the handle is NULL.

### value_operation_num_qubits(ptr)

Returns the number of qubits the operation acts on, used to size the buffer for `value_operation_qubits`.

- `ptr` (`const CValueOperation*`): snapshot handle.

Returns: the qubit count; 0 for NULL.

### value_operation_qubits(ptr, out, len)

Two-step read of the acting qubit ids: call `value_operation_num_qubits` first to size the buffer, then this copies the ids into `out`.

- `ptr` (`const CValueOperation*`): snapshot handle.
- `out` (`uint32_t*`): output buffer, may be NULL (count query only).
- `len` (`uintptr_t`): buffer length.

Returns: the total qubit count.

### value_operation_num_params(ptr)

Returns the number of parameters carried by the operation.

- `ptr` (`const CValueOperation*`): snapshot handle.

Returns: the parameter count; 0 for NULL.

### value_operation_param(ptr, index, out)

Reads the parameter at `index`, writing a `CParameterValue` to `out`.

- `ptr` (`const CValueOperation*`): snapshot handle.
- `index` (`uintptr_t`): parameter index.
- `out` (`CParameterValue*`): resolved value written out; free the `param` field of the symbolic case with `param_free`.

Returns: `0` on success; `-1` for NULL; `-3` when the index is out of bounds.

```c
CCircuit *qc = circuit_new(1);
circuit_rx(qc, 0, 0.7);

CValueOperation *op = circuit_index(qc, 0);
if (op != NULL) {
    char *type = value_operation_instruction_type(op);   /* "standard" */
    cqlib_string_free(type);

    CParameterValue v;
    if (value_operation_param(op, 0, &v) == 0) {
        /* v.tag == PARAMETER_VALUE_TAG_FIXED, v.value == 0.7 */
    }
    value_operation_free(op);
}
circuit_free(qc);
```

---

## Delay Instructions

### circuit_delay(ptr, qubit, duration)

Appends a delay (idle period) with a numeric duration on `qubit`.

- `ptr` (`CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): qubit id.
- `duration` (`double`): duration.

Returns: `0` on success; `-1` for NULL; `-2` when the qubit is out of bounds; `-3` when the duration is not finite.

### circuit_delay_param(ptr, qubit, duration)

Appends a delay with a symbolic duration on `qubit` (the duration parameter is cloned).

- `ptr` (`CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): qubit id.
- `duration` (`const CParameter*`): symbolic duration handle.

Returns: `0` on success; `-1` for NULL; `-2` when the qubit is out of bounds; `-3` on failure.

```c
CCircuit *qc = circuit_new(2);
circuit_delay(qc, 0, 100.0);          /* numeric duration */

CParameter *tau = param_parse("tau");
circuit_delay_param(qc, 1, tau);      /* symbolic duration */
param_free(tau);

circuit_free(qc);
```
