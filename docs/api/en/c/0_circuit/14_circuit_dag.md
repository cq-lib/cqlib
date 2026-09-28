# Circuit DAG

`CCircuitDag*` is the operation-dependency view of a circuit: nodes represent operations, and edges represent the ordering constraints between two operations that share a resource. Shared resources include qubits, mutable classical variables, immutable classical values, and the global order that carries operations with no concrete data resource; each wire corresponds to a pair of input / output sentinel nodes in the graph. The typical workflow is `circuit_dag_from_circuit` → inspect and validate → `circuit_dag_to_circuit`; the DAG can also be queried in depth (node identification, traversal, layering, wire timelines, gate runs, control flow) and edited in place (append, remove, substitute) before being lowered back. For the common conventions (error codes, handle release, two-step array output) see [Overview](../0_overview.md).

---

## Construction and release

### circuit_dag_from_circuit(ptr)

Builds a dependency DAG from an existing circuit, copying the circuit's qubits, parameter table, symbol names, classical variable and value type tables, and global phase; the DAG passes its internal validation by the time construction finishes.

- `ptr` (`const struct CCircuit*`): source circuit handle.

Returns: a newly allocated `CCircuitDag*` (free with `circuit_dag_free`), or NULL when `ptr` is NULL or the build fails.

### circuit_dag_from_operations(qubits, qubits_len, ops, ops_len)

Builds a dependency DAG from explicit qubits and an operation slice — the narrow entry point for local analysis: it takes no parameter table, classical declarations, or global phase. Each element of `qubits` is used directly as a qubit id, every operation must act on registered ids, and duplicate ids, out-of-set references, or non-self-contained operations (carrying parameter-table indices, classical variable / value references, or control-flow bodies) fail the build.

- `qubits` (`const uint32_t*`): array of qubit ids.
- `qubits_len` (`uintptr_t`): number of qubits.
- `ops` (`const struct COperation* const*`): array of operation handles (created with `operation_new`, see [Operations and Instructions](4_operation_instruction.md)).
- `ops_len` (`uintptr_t`): number of operations.

Returns: a newly allocated `CCircuitDag*` (free with `circuit_dag_free`), or NULL when an input array is NULL while its length is positive (`qubits` NULL with `qubits_len > 0`, `ops` NULL with `ops_len > 0`, a NULL entry inside the array) or the build fails.

### circuit_dag_free(ptr)

Frees a DAG handle.

- `ptr` (`struct CCircuitDag*`): handle to free; passing NULL is allowed.

---

## Conversion and validation

### circuit_dag_to_circuit(ptr)

Rebuilds the source-order circuit from the DAG, restoring the qubit set, classical type tables, and global phase; the source handle stays alive and unchanged.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: a newly allocated `CCircuit*` (free with `circuit_free`), or NULL when `ptr` is NULL or the lowering fails (a cyclic graph, an invalid parameter index, and so on).

### circuit_dag_validate(ptr)

Validates DAG consistency: whether the graph is cyclic, whether every wire's input and output sentinels are legal, whether the qubits and parameter indices referenced by operations are valid, whether the resources carried by edges are legal, and more.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: `0` on success; `-1` when `ptr` is NULL; `-3` on a cyclic or invalid graph.

---

## Metadata and statistics

### circuit_dag_num_qubits(ptr)

Returns the number of qubits.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the qubit count, or 0 when `ptr` is NULL.

### circuit_dag_num_ops(ptr)

Returns the number of operation nodes.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the operation-node count, or 0 when `ptr` is NULL.

### circuit_dag_is_empty(ptr)

Returns whether the DAG has no operation nodes.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: `1` when empty; `0` otherwise; `-1` when `ptr` is NULL.

### circuit_dag_qubits_len(ptr)

Returns the number of qubit ids in the DAG, used to size the output buffer.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the id count, or 0 when `ptr` is NULL.

### circuit_dag_qubits(ptr, buffer, len)

Copies the qubit ids in registration order into `buffer` (two-step: query the count with `circuit_dag_qubits_len` first).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while the id set is non-empty; `-8` when `len` is too small.

### circuit_dag_parameters_len(ptr)

Returns the number of registered parameters.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the parameter count, or 0 when `ptr` is NULL.

### circuit_dag_parameters(ptr, out, len)

Copies the parameter names in registration order into `out` (two-step: query the count with `circuit_dag_parameters_len`). Each written element is a freshly allocated C string freed separately with `cqlib_string_free`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `out` (`char**`): output array.
- `len` (`uintptr_t`): array capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `out` is NULL while parameters exist; `-8` when `len` is too small.

### circuit_dag_add_parameter(ptr, param, out_index, out_inserted)

Interns a copy of `param` into the DAG's parameter table: the parameter is inserted when absent, and all of its symbol names are registered.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `param` (`const struct CParameter*`): parameter handle, created with `param_parse` (see [Parameters](3_parameter.md)).
- `out_index` (`uintptr_t*`): optional output for the parameter's index in the table; NULL is allowed.
- `out_inserted` (`int32_t*`): optional output flag; `1` when the parameter was newly inserted, `0` when it already existed; NULL is allowed.

Returns: `0` on success; `-1` when `ptr` or `param` is NULL.

### circuit_dag_symbols_len(ptr)

Returns the number of free symbols referenced by the DAG.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the symbol count, or 0 when `ptr` is NULL.

### circuit_dag_symbols(ptr, out, len)

Copies the symbol names into `out` (two-step: query the count with `circuit_dag_symbols_len`). Each written element is a freshly allocated C string freed separately with `cqlib_string_free`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `out` (`char**`): output array.
- `len` (`uintptr_t`): array capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `out` is NULL while symbols exist; `-8` when `len` is too small.

---

## Wires

The resource carried by an edge is called a wire and doubles as the wire's identity. Qubit wires are all materialized at construction; classical variable and value wires are materialized only when referenced by an operation; the global-order wire is always present. A wire is identified by a `(tag, id)` pair:

| Constant | Value | Meaning |
| --- | --- | --- |
| `DAG_WIRE_QUBIT` | 0 | Qubit timeline, `id` is the qubit id |
| `DAG_WIRE_CLASSICAL_VAR` | 1 | Mutable classical storage, `id` is the variable id |
| `DAG_WIRE_CLASSICAL_VALUE` | 2 | Immutable classical value, `id` is the value index |
| `DAG_WIRE_GLOBAL_ORDER` | 3 | Global ordering resource with no payload (`id` is 0) |

### circuit_dag_wires_len(ptr)

Returns the number of wires materialized in the DAG.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the wire count, or 0 when `ptr` is NULL.

### circuit_dag_wires(ptr, tags, ids, len)

Copies the tags and payload ids of the materialized wires into the parallel arrays `tags` and `ids` (two-step: query the count with `circuit_dag_wires_len`). The global-order wire comes first, followed by the qubit wires; the `id` of a `DAG_WIRE_GLOBAL_ORDER` wire is 0.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tags` (`uint32_t*`): array of `DAG_WIRE_*` tags.
- `ids` (`uint32_t*`): array of payload ids.
- `len` (`uintptr_t`): array capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when either array is NULL while wires exist; `-8` when `len` is too small.

### circuit_dag_has_wire(ptr, tag, id)

Returns whether the DAG carries the given wire.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` (`id` is the qubit id) or `DAG_WIRE_GLOBAL_ORDER` (`id` is ignored).
- `id` (`uint32_t`): payload id of the wire.

Returns: `1` when present; `0` when not; `-1` when `ptr` is NULL; `-8` for other `tag` values.

---

## Node identification

Every graph node — operation node or wire sentinel — is identified by a `uint32_t` node id. These helpers tell the kinds apart and fetch stored content.

### circuit_dag_is_operation(ptr, node)

Returns whether `node` is an operation node.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: `1` for an operation node; `0` for a wire sentinel; `-1` when `ptr` is NULL; `-2` for an unknown node id.

### circuit_dag_node_kind(ptr, node)

Returns the kind of `node` as a freshly allocated C string: `"wire_in"`, `"wire_out"`, or `"operation"`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the kind string (free with `cqlib_string_free`), or NULL when `ptr` is NULL or the node id is unknown.

### circuit_dag_operation(ptr, node)

Returns a copy of the operation stored at an operation node.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: a newly allocated `COperation*` (free with `operation_free`, see [Operations and Instructions](4_operation_instruction.md)), or NULL when `ptr` is NULL or the node is not an operation node.

---

## Graph traversal

All traversal helpers use the two-step array output: query the count with the `_len` variant, then call the copy variant. The queries differ in what they list:

- `circuit_dag_op_nodes` / `circuit_dag_topological_op_nodes` list the operation nodes in source order and in deterministic topological order (ties are broken lexicographically, so the order is stable across runs).
- `circuit_dag_predecessors` / `circuit_dag_successors` return the raw graph neighbors of a node: wire sentinels included, a neighbor listed once per connecting edge (two nodes sharing several resources are connected by one edge per wire — parallel edges over the same wire are never created), and the order is unspecified. Sizing the buffer with the `_len` variant always suffices.
- The wire-filtered queries (`*_on_wire`, `quantum_*`, `classical_*`) return operation neighbors only, deduplicated and ordered by the deterministic topological key.

### circuit_dag_op_nodes_len(ptr)

Returns the number of operation nodes in source order.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the operation-node count, or 0 when `ptr` is NULL.

### circuit_dag_op_nodes(ptr, buffer, len)

Copies the operation node ids in source order into `buffer` (two-step: query the count with `circuit_dag_op_nodes_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while nodes exist; `-8` when `len` is too small.

### circuit_dag_topological_op_nodes_len(ptr)

Returns the number of operation nodes in deterministic topological order. A cyclic graph also reports 0, so pair this with `circuit_dag_validate` when the graph's health is unknown.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the operation-node count, or 0 when `ptr` is NULL or the graph is cyclic.

### circuit_dag_topological_op_nodes(ptr, buffer, len)

Copies the operation node ids in deterministic topological order into `buffer` (two-step: query the count with `circuit_dag_topological_op_nodes_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while nodes exist; `-3` on a cyclic graph; `-8` when `len` is too small.

### circuit_dag_predecessors_len(ptr, node)

Returns the number of raw direct-predecessor entries of `node` — sendinels included, one entry per connecting edge (see the section preamble).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the entry count, or 0 when `ptr` is NULL or `node` is unknown.

### circuit_dag_predecessors(ptr, node, buffer, len)

Copies the raw direct predecessors of `node` into `buffer` (two-step: query the count with `circuit_dag_predecessors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while entries exist; `-2` for an unknown node id; `-8` when `len` is too small.

### circuit_dag_successors_len(ptr, node)

Returns the number of raw direct-successor entries of `node` — sentinels included, one entry per connecting edge.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the entry count, or 0 when `ptr` is NULL or `node` is unknown.

### circuit_dag_successors(ptr, node, buffer, len)

Copies the raw direct successors of `node` into `buffer` (two-step: query the count with `circuit_dag_successors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while entries exist; `-2` for an unknown node id; `-8` when `len` is too small.

### circuit_dag_predecessors_on_wire_len(ptr, node, tag, id)

Returns the number of operation predecessors of `node` connected through the wire `(tag, id)`; results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` (`id` is the qubit id) or `DAG_WIRE_GLOBAL_ORDER` (`id` is ignored); classical tags carry circuit-scoped handles that C cannot reconstruct and are rejected with `-8`. This applies to every `(tag, id)` wire input below.
- `id` (`uint32_t`): payload id of the wire.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, the tag is unknown, or the wire/node combination is invalid.

### circuit_dag_predecessors_on_wire(ptr, node, tag, id, buffer, len)

Copies the operation predecessors of `node` connected through the wire `(tag, id)` into `buffer` (two-step: query the count with `circuit_dag_predecessors_on_wire_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while neighbors exist; `-2` for an unknown node id; `-3` for an invalid wire/node combination (for example a wire that is not part of this DAG); `-8` for an unknown wire tag or a too-small buffer.

### circuit_dag_successors_on_wire_len(ptr, node, tag, id)

Returns the number of operation successors of `node` connected through the wire `(tag, id)`; results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, the tag is unknown, or the wire/node combination is invalid.

### circuit_dag_successors_on_wire(ptr, node, tag, id, buffer, len)

Copies the operation successors of `node` connected through the wire `(tag, id)` into `buffer` (two-step: query the count with `circuit_dag_successors_on_wire_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while neighbors exist; `-2` for an unknown node id; `-3` for an invalid wire/node combination; `-8` for an unknown wire tag or a too-small buffer.

### circuit_dag_quantum_predecessors_len(ptr, node)

Returns the number of operation predecessors of `node` connected through any quantum wire; results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, or the query fails.

### circuit_dag_quantum_predecessors(ptr, node, buffer, len)

Copies the quantum-wire-connected operation predecessors of `node` into `buffer` (two-step: query the count with `circuit_dag_quantum_predecessors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` for an unknown node id; `-3` on failure; `-8` when `len` is too small.

### circuit_dag_quantum_successors_len(ptr, node)

Returns the number of operation successors of `node` connected through any quantum wire; results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, or the query fails.

### circuit_dag_quantum_successors(ptr, node, buffer, len)

Copies the quantum-wire-connected operation successors of `node` into `buffer` (two-step: query the count with `circuit_dag_quantum_successors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` for an unknown node id; `-3` on failure; `-8` when `len` is too small.

### circuit_dag_classical_predecessors_len(ptr, node)

Returns the number of operation predecessors of `node` connected through any classical wire (classical variables and classical values); results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, or the query fails.

### circuit_dag_classical_predecessors(ptr, node, buffer, len)

Copies the classical-wire-connected operation predecessors of `node` into `buffer` (two-step: query the count with `circuit_dag_classical_predecessors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` for an unknown node id; `-3` on failure; `-8` when `len` is too small.

### circuit_dag_classical_successors_len(ptr, node)

Returns the number of operation successors of `node` connected through any classical wire; results are deduplicated and ordered by the deterministic topological key.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.

Returns: the neighbor count, or 0 when `ptr` is NULL, `node` is unknown, or the query fails.

### circuit_dag_classical_successors(ptr, node, buffer, len)

Copies the classical-wire-connected operation successors of `node` into `buffer` (two-step: query the count with `circuit_dag_classical_successors_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` for an unknown node id; `-3` on failure; `-8` when `len` is too small.

---

## Layering and depth

Layering follows ASAP ("as soon as possible") semantics: every operation lands in the layer immediately after the latest layer among its operation predecessors, so layer 0 is the front layer and each operation appears in exactly one layer. Within a layer, nodes keep the deterministic topological order.

`circuit_dag_layers` and the run collectors below use a CSR-style group output: `nodes` receives all groups' node ids flattened back-to-back, and `offsets` receives `groups_len + 1` boundaries with `offsets[0] == 0`; group `k` occupies `nodes[offsets[k]..offsets[k+1]]`. Size `nodes_cap` to hold every node and `offsets_cap` to hold the group count plus one.

### circuit_dag_front_layer_len(ptr)

Returns the number of operation nodes without an operation predecessor — the immediately runnable set, in deterministic topological order.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the front-layer size, or 0 when `ptr` is NULL or the graph is cyclic.

### circuit_dag_front_layer(ptr, buffer, len)

Copies the front-layer operation node ids into `buffer` (two-step: query the count with `circuit_dag_front_layer_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while nodes exist; `-3` on a cyclic graph; `-8` when `len` is too small.

### circuit_dag_layers_len(ptr)

Returns the ASAP layer count, which equals the dependency depth. An empty DAG reports 0.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the layer count, or 0 when `ptr` is NULL or the graph is cyclic.

### circuit_dag_layers(ptr, nodes, nodes_cap, offsets, offsets_cap)

Copies the ASAP layers in CSR style: layer nodes flattened into `nodes` in layer order, boundaries into `offsets` (see the section preamble).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `nodes` (`uint32_t*`): flattened layer nodes; within each layer in deterministic topological order.
- `nodes_cap` (`uintptr_t`): capacity of `nodes`; must hold every operation node.
- `offsets` (`uintptr_t*`): layer boundaries, `offsets[0] == 0`.
- `offsets_cap` (`uintptr_t`): capacity of `offsets`; must hold the layer count plus one.

Returns: `0` on success; `-1` when `ptr` is NULL, or when a buffer is NULL while data exists; `-3` on a cyclic graph; `-8` when either capacity is too small.

### circuit_dag_node_layers_len(ptr)

Returns the number of per-node layer entries, which equals the operation-node count.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the entry count, or 0 when `ptr` is NULL or the graph is cyclic.

### circuit_dag_node_layers(ptr, nodes, layers, len)

Copies the ASAP layer index of every operation node into the parallel arrays `nodes` and `layers` as `(node, layer)` pairs in deterministic topological order (two-step: query the count with `circuit_dag_node_layers_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `nodes` (`uint32_t*`): node ids.
- `layers` (`uintptr_t*`): layer index of the node at the same position.
- `len` (`uintptr_t`): capacity of both arrays.

Returns: `0` on success; `-1` when `ptr` is NULL, or when either array is NULL while entries exist; `-3` on a cyclic graph; `-8` when `len` is too small.

### circuit_dag_depth(ptr)

Returns the ASAP dependency depth: the number of layers.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the depth as a signed value (`0` for an empty DAG); `-1` when `ptr` is NULL; `-3` on a cyclic graph.

---

## Wire timelines

Each wire is a timeline: `circuit_dag_wire_in` / `circuit_dag_wire_out` report its endpoints and `circuit_dag_nodes_on_wire` walks the operations in the order the wire threads them. Every `(tag, id)` input in this section accepts `DAG_WIRE_QUBIT` (`id` is the qubit id) and `DAG_WIRE_GLOBAL_ORDER` (`id` is ignored); classical tags are rejected with `-8` because classical wires carry circuit-scoped handles that C cannot reconstruct — for node-based classical connectivity use `circuit_dag_classical_predecessors` / `circuit_dag_classical_successors` from [Graph traversal](#graph-traversal).

### circuit_dag_is_wire_idle(ptr, tag, id)

Returns whether the wire carries no operation node.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.

Returns: `1` when idle; `0` when in use; `-1` when `ptr` is NULL; `-3` when the wire is not part of this DAG; `-8` for other `tag` values.

### circuit_dag_nodes_on_wire_len(ptr, tag, id)

Returns the number of operation nodes on the wire in wire order, or 0 when `ptr` is NULL, the tag is unknown, or the wire is not part of this DAG.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.

Returns: the node count, or 0 on the invalid states above.

### circuit_dag_nodes_on_wire(ptr, tag, id, buffer, len)

Copies the operation nodes on the wire in wire order into `buffer` (two-step: query the count with `circuit_dag_nodes_on_wire_len`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while nodes exist; `-3` when the wire is not part of this DAG; `-8` for an unknown wire tag or a too-small buffer.

### circuit_dag_wire_in(ptr, tag, id)

Returns the input sentinel node id of the wire.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.

Returns: the sentinel node id; `-1` when `ptr` is NULL; `-3` when the wire is not part of this DAG; `-8` for other `tag` values.

### circuit_dag_wire_out(ptr, tag, id)

Returns the output sentinel node id of the wire.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.

Returns: the sentinel node id; `-1` when `ptr` is NULL; `-3` when the wire is not part of this DAG; `-8` for other `tag` values.

---

## Control-flow and measurement flags

### circuit_dag_has_control_flow(ptr)

Returns whether any top-level operation is structured control flow (`if` / `while` / `for` / `switch` / `break` / `continue`).

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: `1` when true; `0` otherwise; `-1` when `ptr` is NULL.

### circuit_dag_has_nested_control_flow(ptr)

Returns whether any control-flow body itself contains structured control flow.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: `1` when true; `0` otherwise; `-1` when `ptr` is NULL.

### circuit_dag_has_measurement(ptr)

Returns whether any top-level operation measures qubits, directly or recursively inside control-flow bodies.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: `1` when true; `0` otherwise; `-1` when `ptr` is NULL.

---

## Operation counting

### circuit_dag_operation_count_by_name_len(ptr)

Returns the number of distinct instruction names among the top-level operations.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the distinct-name count, or 0 when `ptr` is NULL.

### circuit_dag_operation_count_by_name(ptr, names, counts, len)

Copies the top-level per-name operation counts into the parallel arrays `names` and `counts`, in first-appearance (insertion) order (two-step: query the count with `circuit_dag_operation_count_by_name_len`). Each written name is a freshly allocated C string freed separately with `cqlib_string_free`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `names` (`char**`): output array of instruction names.
- `counts` (`uintptr_t*`): output array of per-name counts.
- `len` (`uintptr_t`): capacity of both arrays.

Returns: `0` on success; `-1` when `ptr` is NULL, or when an output array is NULL while entries exist; `-8` when `len` is too small.

### circuit_dag_operation_count_by_name_recursive_len(ptr)

Returns the number of distinct instruction names among top-level operations and, recursively, operations inside control-flow bodies.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the distinct-name count, or 0 when `ptr` is NULL.

### circuit_dag_operation_count_by_name_recursive(ptr, names, counts, len)

Copies the top-level and nested per-name operation counts into the parallel arrays `names` and `counts`, in first-appearance (insertion) order (two-step: query the count with `circuit_dag_operation_count_by_name_recursive_len`). Each written name is a freshly allocated C string freed separately with `cqlib_string_free`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `names` (`char**`): output array of instruction names.
- `counts` (`uintptr_t*`): output array of per-name counts.
- `len` (`uintptr_t`): capacity of both arrays.

Returns: `0` on success; `-1` when `ptr` is NULL, or when an output array is NULL while entries exist; `-8` when `len` is too small.

---

## Contiguous run collection

A run is a maximal stretch of consecutive operation nodes along one wire's timeline in which every node satisfies the criterion; runs are reported along the wire in timeline order. The criteria:

| Constant | Value | Meaning |
| --- | --- | --- |
| `DAG_RUN_CRITERION_1Q_GATE` | 0 | One-qubit gate: the operation acts on exactly 1 qubit and its instruction is `Standard`, `McGate`, `UnitaryGate`, or `CircuitGate` |
| `DAG_RUN_CRITERION_2Q_GATE` | 1 | Two-qubit gate: the operation acts on exactly 2 qubits under the same instruction restrictions |

The copy variants use the CSR-style group output described in [Layering and depth](#layering-and-depth): run nodes flattened back-to-back into `nodes`, boundaries into `offsets` with `offsets[0] == 0`.

### circuit_dag_collect_1q_runs_len(ptr)

Returns the number of contiguous one-qubit-gate runs over all qubit wires, or 0 when `ptr` is NULL or the collection fails.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the run count, or 0 on the invalid states above.

### circuit_dag_collect_1q_runs(ptr, nodes, nodes_cap, offsets, offsets_cap)

Copies the one-qubit-gate runs in CSR style (see the section preamble).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `nodes` (`uint32_t*`): flattened run nodes.
- `nodes_cap` (`uintptr_t`): capacity of `nodes`; must hold every run node.
- `offsets` (`uintptr_t*`): run boundaries, `offsets[0] == 0`.
- `offsets_cap` (`uintptr_t`): capacity of `offsets`; must hold the run count plus one.

Returns: `0` on success; `-1` when `ptr` is NULL, or when a buffer is NULL while data exists; `-3` on a broken timeline; `-8` when either capacity is too small.

### circuit_dag_collect_2q_runs_len(ptr)

Returns the number of contiguous two-qubit-gate runs over all qubit wires, or 0 when `ptr` is NULL or the collection fails.

- `ptr` (`const struct CCircuitDag*`): DAG handle.

Returns: the run count, or 0 on the invalid states above.

### circuit_dag_collect_2q_runs(ptr, nodes, nodes_cap, offsets, offsets_cap)

Copies the two-qubit-gate runs in CSR style (see the section preamble). A two-qubit gate touches two qubit wires, so the same node may appear in the runs reported for both of its wires.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `nodes` (`uint32_t*`): flattened run nodes.
- `nodes_cap` (`uintptr_t`): capacity of `nodes`; must hold every run node.
- `offsets` (`uintptr_t*`): run boundaries, `offsets[0] == 0`.
- `offsets_cap` (`uintptr_t`): capacity of `offsets`; must hold the run count plus one.

Returns: `0` on success; `-1` when `ptr` is NULL, or when a buffer is NULL while data exists; `-3` on a broken timeline; `-8` when either capacity is too small.

### circuit_dag_collect_runs_on_wire_len(ptr, tag, id, criterion)

Returns the number of contiguous gate runs on one wire matching `criterion`, or 0 when `ptr` is NULL, the tag or criterion is unknown, or the collection fails.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.
- `criterion` (`uint32_t`): `DAG_RUN_CRITERION_1Q_GATE` or `DAG_RUN_CRITERION_2Q_GATE`.

Returns: the run count, or 0 on the invalid states above.

### circuit_dag_collect_runs_on_wire(ptr, tag, id, criterion, nodes, nodes_cap, offsets, offsets_cap)

Copies the contiguous gate runs on one wire matching `criterion` in CSR style (see the section preamble).

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `tag` (`uint32_t`): `DAG_WIRE_QUBIT` or `DAG_WIRE_GLOBAL_ORDER`.
- `id` (`uint32_t`): payload id of the wire.
- `criterion` (`uint32_t`): `DAG_RUN_CRITERION_1Q_GATE` or `DAG_RUN_CRITERION_2Q_GATE`.
- `nodes` (`uint32_t*`): flattened run nodes.
- `nodes_cap` (`uintptr_t`): capacity of `nodes`; must hold every run node.
- `offsets` (`uintptr_t*`): run boundaries, `offsets[0] == 0`.
- `offsets_cap` (`uintptr_t`): capacity of `offsets`; must hold the run count plus one.

Returns: `0` on success; `-1` when `ptr` is NULL, or when a buffer is NULL while data exists; `-3` when the wire is invalid or the timeline is broken; `-8` for an unknown wire tag or criterion, or undersized capacities.

---

## Mutating the DAG

These functions edit the DAG in place. Each mutation rebuilds the underlying graph, so node ids captured before a mutation may no longer identify the same nodes — re-query them (for example with `circuit_dag_op_nodes`) after every successful mutation. Operations passed in are copied (value-level operations are lowered first), so callers keep ownership of their handles.

### circuit_dag_apply_operation_back(ptr, operation, out_node)

Appends a copy of `operation` at the back of the DAG.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `operation` (`const struct COperation*`): operation to append.
- `out_node` (`uint32_t*`): optional output for the new node id; NULL is allowed.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-3` on failure (for example the operation references resources the DAG does not know).

### circuit_dag_apply_value_operation_back(ptr, operation, out_node)

Lowers the self-contained value-level operation `operation` (see [Operations and Instructions](4_operation_instruction.md)) and appends it at the back of the DAG.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `operation` (`const struct CValueOperation*`): value-level operation to append.
- `out_node` (`uint32_t*`): optional output for the new node id; NULL is allowed.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-3` on lowering or DAG failure.

### circuit_dag_apply_operation_front(ptr, operation, out_node)

Prepends a copy of `operation` at the front of the DAG.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `operation` (`const struct COperation*`): operation to prepend.
- `out_node` (`uint32_t*`): optional output for the new node id; NULL is allowed.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-3` on failure.

### circuit_dag_apply_value_operation_front(ptr, operation, out_node)

Lowers the self-contained value-level operation `operation` and prepends it at the front of the DAG.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `operation` (`const struct CValueOperation*`): value-level operation to prepend.
- `out_node` (`uint32_t*`): optional output for the new node id; NULL is allowed.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-3` on lowering or DAG failure.

### circuit_dag_remove_op_node(ptr, node)

Removes the operation node `node`; the wire timelines re-thread through the remaining operations.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id to remove.

Returns: the removed operation as a newly allocated `COperation*` (free with `operation_free`), or NULL when `ptr` is NULL or the node is missing or not an operation node.

### circuit_dag_substitute_node(ptr, node, operation)

Replaces the operation stored at `node` with a copy of `operation`.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id to replace.
- `operation` (`const struct COperation*`): replacement operation.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-2` when `node` is not an operation node; `-3` when the new operation does not fit the DAG (unknown qubits, unregistered parameters, invalid classical references, and so on).

### circuit_dag_substitute_value_node(ptr, node, operation)

Lowers the self-contained value-level operation `operation` and replaces the operation stored at `node` with it.

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id to replace.
- `operation` (`const struct CValueOperation*`): replacement value-level operation.

Returns: `0` on success; `-1` when `ptr` or `operation` is NULL; `-2` when `node` is not an operation node; `-3` on lowering or DAG failure.

### circuit_dag_substitute_node_with_dag(ptr, node, replacement)

Splices a copy of the `replacement` DAG in place of the operation node `node`. Two rules keep the schedule legal, and violations fail with `-3`:

- Resource footprint: every operation in `replacement` may only read wires the replaced node read or wrote, and may only write wires the replaced node wrote; the global-order resource is exempt.
- Control-flow shape: a control-flow operation can only be replaced by a replacement DAG holding exactly one operation of the same control-flow kind; ordinary operations can be replaced by multi-operation DAGs within the footprint rule.

The replacement must also pass DAG validation on its own (a cyclic or invalid replacement fails with `-3`).

- `ptr` (`struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id to replace.
- `replacement` (`const struct CCircuitDag*`): replacement DAG.

Returns: `0` on success; `-1` when `ptr` or `replacement` is NULL; `-2` when `node` is not an operation node; `-3` on validation failure.

---

## Control-flow inspection

`CDagControlFlow*` is a read-only snapshot of the (recursive) control-flow structure attached to an operation node. It is produced by `circuit_dag_control_flow` and freed with `dag_control_flow_free`. The structure's kind:

| Constant | Value | Meaning |
| --- | --- | --- |
| `DAG_CONTROL_FLOW_IF` | 0 | `if` / `else`: bodies are the `then` body first, the `else` body second when present |
| `DAG_CONTROL_FLOW_WHILE` | 1 | `while`: a single body |
| `DAG_CONTROL_FLOW_FOR` | 2 | `for`: a single body |
| `DAG_CONTROL_FLOW_SWITCH` | 3 | `switch`: one body per case, in case order; the optional default body is reported separately |
| `DAG_CONTROL_FLOW_BREAK` | 4 | `break`: no body |
| `DAG_CONTROL_FLOW_CONTINUE` | 5 | `continue`: no body |

Body DAGs returned here are independent deep copies (`CCircuitDag*` freed with `circuit_dag_free`); edits on a copy do not propagate back to the parent DAG.

### circuit_dag_control_flow(ptr, node, out)

Snapshots the control-flow structure attached to `node` into `out`.

- `ptr` (`const struct CCircuitDag*`): DAG handle.
- `node` (`uint32_t`): node id.
- `out` (`struct CDagControlFlow**`): output for the snapshot handle.

Returns: `1` on success (`*out` holds a freshly allocated `CDagControlFlow*`); `0` when the node is an ordinary operation (`*out` is set to NULL); `-1` when `ptr` or `out` is NULL; `-2` for an unknown node id or a non-operation node.

### dag_control_flow_kind(ptr)

Returns the kind tag of the snapshot.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.

Returns: a `DAG_CONTROL_FLOW_*` tag, or `-1` when `ptr` is NULL.

### dag_control_flow_len(ptr)

Returns the number of body DAGs: `if` carries its `then` plus an optional `else` body, `while` and `for` one body each, `switch` one per case, `break` and `continue` none.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.

Returns: the body count, or 0 when `ptr` is NULL.

### dag_control_flow_body(ptr, index)

Returns a copy of body `index`, in the order listed in the table above.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.
- `index` (`uintptr_t`): body index.

Returns: a newly allocated `CCircuitDag*` (free with `circuit_dag_free`), or NULL when `ptr` is NULL or `index` is out of range.

### dag_control_flow_has_default(ptr)

Returns whether a `switch` snapshot carries a default body.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.

Returns: `1` when present; `0` when absent or the structure is not a `switch`; `-1` when `ptr` is NULL.

### dag_control_flow_default_body(ptr)

Returns a copy of the `switch` default body.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.

Returns: a newly allocated `CCircuitDag*` (free with `circuit_dag_free`), or NULL when `ptr` is NULL, the structure is not a `switch`, or it has no default body.

### dag_control_flow_body_value(ptr, index, lo, hi)

Writes the 128-bit case value attached to `switch` body `index` into `lo` and `hi`.

- `ptr` (`const struct CDagControlFlow*`): snapshot handle.
- `index` (`uintptr_t`): case index, aligned with `dag_control_flow_body`.
- `lo` (`uint64_t*`): output for the low 64 bits of the case value.
- `hi` (`uint64_t*`): output for the high 64 bits of the case value.

Returns: `0` on success; `-1` when `ptr`, `lo`, or `hi` is NULL; `-8` when the structure is not a `switch` or `index` is out of range.

### dag_control_flow_free(ptr)

Frees a control-flow snapshot handle.

- `ptr` (`struct CDagControlFlow*`): snapshot handle; passing NULL is allowed.

---

## Example: circuit → DAG → statistics → back to circuit

```c
#include <stdint.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* Circuit → DAG */
    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);

    /* Validate and collect statistics */
    int32_t ok = circuit_dag_validate(dag);                  /* 0 */
    uintptr_t num_ops = circuit_dag_num_ops(dag);            /* 2 */
    int32_t empty = circuit_dag_is_empty(dag);               /* 0 */

    uint32_t ids[2] = {0, 0};
    circuit_dag_qubits(dag, ids, 2);                         /* ids = {0, 1} */

    /* Wire inventory: the global-order wire first, then the two qubit wires */
    uintptr_t wires = circuit_dag_wires_len(dag);            /* 3 */
    uint32_t tags[3] = {0, 0, 0};
    uint32_t wire_ids[3] = {0, 0, 0};
    circuit_dag_wires(dag, tags, wire_ids, 3);
    /* tags = {DAG_WIRE_GLOBAL_ORDER, DAG_WIRE_QUBIT, DAG_WIRE_QUBIT}
       wire_ids = {0, 0, 1} */

    int32_t has_q0 = circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0);   /* 1 */
    int32_t has_q5 = circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 5);   /* 0 */

    /* DAG → circuit: the rebuild matches the source shape
     * (2 qubits, 2 operations) */
    struct CCircuit* lowered = circuit_dag_to_circuit(dag);

    printf("validate=%d ops=%llu empty=%d wires=%llu lowered_ops=%llu\n",
           (int)ok, (unsigned long long)num_ops, (int)empty,
           (unsigned long long)wires,
           (unsigned long long)circuit_num_operations(lowered));

    circuit_free(lowered);
    circuit_dag_free(dag);
    circuit_free(circuit);
    return 0;
}
```

For the circuit-building entry points see [Circuit](1_circuit.md); for building operation handles see [Operations and Instructions](4_operation_instruction.md).
