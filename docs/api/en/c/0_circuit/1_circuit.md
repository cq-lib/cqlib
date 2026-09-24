# Circuit

The `CCircuit*` handle is the main container for a quantum circuit, holding the set of logical qubits, the operation sequence, the parameter table, classical variables and values, control-flow scopes, and the global phase. This page covers circuit construction and release, property queries, structural transformations, checkpoint transactions, the global phase, and parameter binding; gate functions are covered in [Standard Gates](5_gate_standard.md) and symbolic parameter details in [Symbolic Parameters](3_parameter.md). Error codes and memory conventions are described in [Overview](../0_overview.md).

---

## Construction and Release

### circuit_new(num_qubits)

Creates an empty circuit with contiguous logical qubits whose ids run from `0` to `num_qubits - 1`.

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns: a newly allocated `CCircuit*`; free it with `circuit_free`.

### circuit_from_qubits(qubits, len)

Creates a circuit from an explicit list of qubit ids, allowing sparse numbering (e.g. `{2, 5}`). When `len` is 0 an empty circuit is created and `qubits` may be NULL.

- `qubits` (`const uint32_t*`): array of qubit ids.
- `len` (`uintptr_t`): array length.

Returns: a newly allocated `CCircuit*`; when `len > 0`, returns `NULL` if `qubits` is NULL or an id is duplicated.

### circuit_from_operations(qubits, qubits_len, operations, operations_len)

Creates a circuit from qubits plus a batch of resolved value-level operations (as produced by `circuit_index`, see [Operation / Instruction](4_operation_instruction.md)). Operations are appended in order through value-level lowering, so qubit membership is validated and symbolic parameters are interned into the new circuit's parameter table.

- `qubits` (`const uint32_t*`): array of qubit ids.
- `qubits_len` (`uintptr_t`): array length; when 0 an empty qubit set is created and `qubits` may be NULL.
- `operations` (`const CValueOperation* const*`): array of resolved operation snapshots.
- `operations_len` (`uintptr_t`): number of operations.

Returns: a newly allocated `CCircuit*`, freed with `circuit_free`; `NULL` on NULL input (including NULL entries inside `operations`), duplicate qubits, or any operation-level failure.

### circuit_append_value_operation(ptr, op)

Appends one resolved value-level operation (as produced by `circuit_index`) to the circuit.

- `ptr` (`CCircuit*`): circuit handle.
- `op` (`const CValueOperation*`): resolved operation snapshot.

Returns: `0` on success; `-1` for NULL; `-3` on application failure (e.g. the operation references a qubit outside the circuit).

### circuit_free(ptr)

Frees a circuit handle. Passing NULL is allowed.

- `ptr` (`CCircuit*`): circuit handle.

```c
/* Sparse numbering: qubit ids 2 and 5 */
uint32_t ids[2] = {2, 5};
CCircuit *qc = circuit_from_qubits(ids, 2);
if (qc != NULL) {
    /* circuit_width(qc) == 2 */
    circuit_free(qc);
}
```

---

## Property Queries

### circuit_num_qubits(ptr)

Returns the number of qubits.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the qubit count; 0 for NULL.

### circuit_width(ptr)

Returns the circuit width, i.e. the number of qubits; identical in semantics to `circuit_num_qubits`.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the qubit count; 0 for NULL.

### circuit_num_operations(ptr)

Returns the number of top-level operations.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the operation count; 0 for NULL.

### circuit_num_parameters(ptr)

Returns the number of symbolic parameters interned in the circuit's parameter table; identical to `circuit_parameters_len`.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the parameter count; 0 for NULL.

### circuit_qubits_len(ptr)

Returns the qubit count, used to size the buffer for `circuit_qubits`.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the qubit count; 0 for NULL.

### circuit_qubits(ptr, out, len)

Copies the qubit ids into `out` in insertion order. When `out` is NULL or `len` is too small, it only returns the count without copying.

- `ptr` (`const CCircuit*`): circuit handle.
- `out` (`uint32_t*`): output buffer; may be NULL (count query only).
- `len` (`uintptr_t`): buffer length.

Returns: the total number of qubits.

### circuit_depth(ptr, recurse)

Computes the circuit depth. With `recurse = true`, operations inside control-flow bodies are expanded recursively.

- `ptr` (`const CCircuit*`): circuit handle.
- `recurse` (`bool`): whether to recursively count operations inside control flow.

Returns: the circuit depth; `-1` for a NULL handle, `-3` on failure (e.g. the circuit contains control flow but was not traversed recursively).

### circuit_validate(ptr)

Validates circuit consistency: classical handle ownership, control-flow scopes, data dependencies, and structural invariants. Circuits imported from external sources or generated automatically should be validated before further processing.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: `0` on success; `-1` for NULL; `-3` when validation fails.

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

if (circuit_validate(qc) == 0) {
    /* num_qubits=2, num_operations=2, depth=2 */
    uintptr_t n = circuit_qubits_len(qc);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    circuit_qubits(qc, ids, n);      /* ids == {0, 1} */
    free(ids);
}
circuit_free(qc);
```

### circuit_contains_qubit(ptr, qubit)

Returns whether `qubit` belongs to the circuit.

- `ptr` (`const CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): qubit id.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### circuit_has_same_qubits(a, b)

Returns whether both circuits share the same ordered qubit domain.

- `a` (`const CCircuit*`): first circuit handle.
- `b` (`const CCircuit*`): second circuit handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

### circuit_operations_structurally_equal(a, b)

Returns whether two circuits are structurally equal: same ordered qubits, parameter tables, and operations (with parameter indices remapped across the two tables, and circuit-local classical handles compared by structure).

- `a` (`const CCircuit*`): first circuit handle.
- `b` (`const CCircuit*`): second circuit handle.

Returns: `1` when yes; `0` when no; `-1` for NULL.

---

## Qubit Management

### circuit_add_qubits(ptr, qubits, count)

Appends new logical qubits to the circuit.

- `ptr` (`CCircuit*`): circuit handle.
- `qubits` (`const uint32_t*`): array of new qubit ids.
- `count` (`uintptr_t`): number of qubits to append.

Returns: `0` on success; `-1` for NULL; `-3` on failure (e.g. an id duplicates an existing qubit).

---

## Structural Transformations

### circuit_compose(ptr, other, qubits_map, map_len)

Appends `other` to the end of the current circuit. `qubits_map` maps `other`'s qubits, in `other`'s qubit order, onto the current circuit's qubits; passing a NULL map (`map_len` 0) merges by identity of ids, and qubits of `other` that do not exist in the current circuit are absorbed into it.

- `ptr` (`CCircuit*`): target circuit (modified).
- `other` (`const CCircuit*`): circuit being composed in (not modified).
- `qubits_map` (`const uint32_t*`): qubit mapping array; may be NULL.
- `map_len` (`uintptr_t`): mapping length.

Returns: `0` on success; `-1` for NULL; `-3` on failure (invalid mapping or structure check failure).

```c
CCircuit *lhs = circuit_new(3);
CCircuit *rhs = circuit_new(2);
circuit_cx(rhs, 0, 1);

/* rhs qubits 0 and 1 map onto lhs qubits 1 and 2, in order */
uint32_t map[2] = {1, 2};
int32_t rc = circuit_compose(lhs, rhs, map, 2);   /* rc == 0 */

circuit_free(rhs);
circuit_free(lhs);
```

### circuit_inverse(ptr)

Returns the inverse of the current circuit: operations in reverse order with each one inverted, and the global phase negated. The original circuit is unchanged.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: a newly allocated `CCircuit*`, freed with `circuit_free`; returns `NULL` when the circuit contains non-invertible operations (measure, reset, ...) or on failure.

### circuit_decompose(ptr)

Returns a copy of the circuit with composite gates expanded: circuit-defined gates (composite gates appended via `circuit_circuit_gate`) are expanded into their underlying operations. The original circuit is unchanged.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: a newly allocated `CCircuit*`; `NULL` on failure.

### circuit_remove_operation(ptr, index)

Deletes the top-level operation at `index`.

- `ptr` (`CCircuit*`): circuit handle.
- `index` (`uintptr_t`): operation index.

Returns: `0` on success; `-1` for NULL; `-3` when the index is out of bounds or a post-deletion structure check fails.

### circuit_remove_operations(ptr, indices, len)

Deletes top-level operations by index in one batch. `indices` are interpreted against the operation list before deletion; duplicate indices are ignored; any failure rolls the whole batch back (atomic).

- `ptr` (`CCircuit*`): circuit handle.
- `indices` (`const uintptr_t*`): array of operation indices.
- `len` (`uintptr_t`): number of indices (may be 0).

Returns: `0` on success (including an empty index list); `-1` for NULL; `-3` when any index is out of bounds or a classical value is still referenced.

```c
CCircuit *qc = circuit_new(1);
circuit_rx(qc, 0, 0.7);

CCircuit *inv = circuit_inverse(qc);   /* inverse circuit: RX(-0.7) */
if (inv != NULL && circuit_validate(inv) == 0) {
    /* use inv ... */
}
circuit_free(inv);
circuit_free(qc);
```

---

## Checkpoint Transactions

The checkpoint family captures and restores circuit state (operation list, parameter and symbol tables, classical tables, control scopes) through an opaque `CCheckpoint*` token. A token is produced by `circuit_checkpoint` / `circuit_begin` and consumed exactly once by `circuit_commit`, `circuit_rollback_to`, or `circuit_rollback_control_body_transaction`; free an unconsumed token with `checkpoint_free`.

```c
CCircuit *qc = circuit_new(1);
CCheckpoint *cp = circuit_checkpoint(qc);
circuit_h(qc, 0);                        /* tentative operations */

if (circuit_validate(qc) != 0) {
    circuit_rollback_to(qc, cp);         /* undo everything since the checkpoint */
} else {
    circuit_commit(qc, cp);              /* keep the operations */
}

circuit_free(qc);
```

### circuit_checkpoint(ptr)

Captures a checkpoint of the circuit state. The returned token is consumed exactly once by `circuit_commit`, `circuit_rollback_to`, or `circuit_rollback_control_body_transaction`.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: a newly allocated `CCheckpoint*`; `NULL` on NULL input.

### circuit_begin(ptr)

Starts an externally driven construction transaction; equivalent to `circuit_checkpoint`.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: a newly allocated `CCheckpoint*`; `NULL` on NULL input.

### circuit_commit(ptr, checkpoint)

Commits the state allocated since the token was captured and consumes the token.

- `ptr` (`CCircuit*`): circuit handle.
- `checkpoint` (`CCheckpoint*`): token produced by `circuit_checkpoint` / `circuit_begin`.

Returns: `0` on success; `-1` for NULL.

### circuit_rollback_to(ptr, checkpoint)

Rolls back every state allocated since the token was captured and consumes the token.

- `ptr` (`CCircuit*`): circuit handle.
- `checkpoint` (`CCheckpoint*`): token produced by `circuit_checkpoint` / `circuit_begin`.

Returns: `0` on success; `-1` for NULL.

### circuit_rollback_control_body_transaction(ptr, checkpoint)

Rolls back every state allocated since the token was captured and consumes the token; same behavior as `circuit_rollback_to`, kept under the core transaction name.

- `ptr` (`CCircuit*`): circuit handle.
- `checkpoint` (`CCheckpoint*`): token produced by `circuit_checkpoint` / `circuit_begin`.

Returns: `0` on success; `-1` for NULL.

### checkpoint_free(ptr)

Frees an unconsumed checkpoint token. Passing NULL is allowed.

- `ptr` (`CCheckpoint*`): token to free.

---

## Global Phase

### circuit_global_phase(ptr, out)

Reads the circuit's global phase. The result is written into a `CParameterValue`: a fixed number, or — in the symbolic case — a newly allocated `CParameter*` (free it with `param_free`; see [Symbolic Parameters](3_parameter.md)).

- `ptr` (`const CCircuit*`): circuit handle.
- `out` (`CParameterValue*`): output struct.

Returns: `0` on success; `-1` for NULL.

### circuit_set_global_phase(ptr, phase)

Sets the global phase to a numeric value.

- `ptr` (`CCircuit*`): circuit handle.
- `phase` (`double`): global phase (radians).

Returns: `0` on success; `-1` for NULL; `-3` when the phase is not finite.

### circuit_set_global_phase_param(ptr, param)

Sets the global phase to a symbolic parameter (the parameter is cloned).

- `ptr` (`CCircuit*`): circuit handle.
- `param` (`const CParameter*`): symbolic parameter handle.

Returns: `0` on success; `-1` for NULL.

---

## Parameter Binding

### circuit_assign_params(circuit, bindings)

Returns a new circuit in which every interned parameter that can be evaluated from the binding string is replaced by its fixed numeric value, while the remaining parameters stay symbolic; the original circuit is unchanged and can be reused as a template.

- `circuit` (`const CCircuit*`): circuit handle.
- `bindings` (`const char*`): binding string of the form `"name:value,name2:value2"` (same as `param_evaluate`); NULL or a malformed string is treated as no bindings.

Returns: a newly allocated `CCircuit*`, freed with `circuit_free`; `NULL` when the circuit contains classical control flow or a substitution/evaluation failure occurs.

```c
CCircuit *tpl = circuit_new(1);
CParameter *theta = param_parse("theta");
circuit_rx_param(tpl, 0, theta);
param_free(theta);

CCircuit *bound = circuit_assign_params(tpl, "theta:0.5");
/* bound no longer references the symbol theta */

circuit_free(bound);
circuit_free(tpl);
```

---

## Non-Unitary Instructions and the Identity Gate

### circuit_measure(ptr, qubit)

Appends a computational-basis measurement on `qubit`, producing an immutable classical `Bit` value owned by the circuit. Variants that read measurement results or write into existing classical variables are covered in [Classical Data and Control Flow](9_classical_control_flow.md).

- `ptr` (`CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): the qubit being measured.

Returns: `0` on success; `-1` for NULL; `-2` when the qubit is out of bounds; `-3` on failure.

### circuit_reset(ptr, qubit)

Appends a reset on `qubit`, returning the qubit to `|0>`.

- `ptr` (`CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): qubit id.

Returns: `0` on success; `-1` for NULL; `-2` when the qubit is out of bounds; `-3` on failure.

### circuit_barrier(ptr, qubits, count)

Inserts a barrier over the given qubits, preventing related operations from being reordered across the boundary. Passing NULL `qubits` or `count` 0 inserts a global barrier over all circuit qubits.

- `ptr` (`CCircuit*`): circuit handle.
- `qubits` (`const uint32_t*`): array of qubit ids; may be NULL.
- `count` (`uintptr_t`): number of qubits.

Returns: `0` on success; `-1` for NULL; `-3` when a qubit does not exist or on failure.

### circuit_id(ptr, qubit)

Appends the identity gate on `qubit`; an alias of `circuit_i` kept for frontend naming compatibility.

- `ptr` (`CCircuit*`): circuit handle.
- `qubit` (`uint32_t`): qubit id.

Returns: `0` on success; `-1` for NULL; `-2` when the qubit is out of bounds; `-3` on failure.
