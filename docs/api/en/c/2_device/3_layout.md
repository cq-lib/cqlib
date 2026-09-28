# Layout (C)

`CLayout` is an opaque handle to the logical-to-physical qubit mapping used during routing; it maintains the bidirectional logical↔physical mapping, and a physical qubit carrying no logical qubit is vacant. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Construction and release

### layout_new(logical, num_logical, physical, num_physical)

Creates a layout mapping the logical qubits in the `logical` array, in order, onto the physical qubits in the `physical` array.

Parameters:

- `logical` (`const uint32_t*`): array of logical qubit IDs.
- `num_logical` (`uintptr_t`): number of logical qubits; must be non-zero.
- `physical` (`const uint32_t*`): array of physical qubit IDs.
- `num_physical` (`uintptr_t`): number of physical qubits; must equal `num_logical`.

Returns: a newly allocated `CLayout*` on success; NULL on error.

### layout_from_pairs(pairs, num_pairs, physical_count)

Creates a layout from `(logical, physical)` pairs. `pairs` points to `2 * num_pairs` u32 values laid out as consecutive `(logical, physical)` pairs; `physical_count` defines the total number of physical qubits (`0..physical_count`), and unreferenced ones stay vacant.

Parameters:

- `pairs` (`const uint32_t*`): pair array.
- `num_pairs` (`uintptr_t`): number of pairs; must be non-zero.
- `physical_count` (`uint32_t`): total number of physical qubits.

Returns: a newly allocated `CLayout*` on success; NULL on error.

### layout_free(ptr)

Frees a `CLayout`; NULL is allowed.

---

## Mapping queries

### layout_get(ptr, logical)

Returns the physical qubit mapped to `logical`.

Returns: the physical qubit ID when mapped; `UINT32_MAX` when unmapped or on error.

### layout_get_logical(ptr, physical)

Returns the logical qubit carried by `physical` (the reverse lookup of `layout_get`).

Returns: the logical qubit ID when occupied; `UINT32_MAX` when `physical` is vacant or unknown, which callers must treat as a sentinel.

### layout_is_physical_vacant(ptr, physical)

Checks whether `physical` belongs to the layout and is vacant.

Returns: 1 when vacant; 0 when occupied or unknown; -1 on NULL.

### layout_num_logical(ptr)

Returns the number of mapped logical qubits; 0 for NULL.

### layout_num_physical(ptr)

Returns the total number of physical qubits; 0 for NULL.

### layout_num_vacant_physical(ptr)

Returns the number of vacant physical qubits; 0 for NULL.

---

## Binding and swapping

### layout_bind(ptr, logical, physical)

Binds an unmapped `logical` qubit to a vacant `physical` qubit.

Returns: 0 on success; -1 on NULL; -2 when `physical` does not belong to the layout; -8 when either side already participates in a mapping.

### layout_unbind(ptr, logical, out_physical)

Removes the mapping for `logical` and writes the released physical qubit ID to `*out_physical`.

Returns: 0 on success; -1 on NULL; -8 when `logical` is not bound.

### layout_swap_physical(ptr, phys_a, phys_b)

Swaps the logical qubits carried by two physical qubits; either side may be vacant (a vacant swap moves the logical qubit). This is the basic operation for moving logical qubits during routing.

Returns: 0 on success; -1 on NULL; -2 when either physical qubit does not belong to the layout.

---

## Bulk reads

The following entry points all use the two-step pattern: fetch the length, then fill the buffer; the fill function returns the total count.

### layout_l2p_map_len(ptr) / layout_l2p_map(ptr, out, len)

The logical→physical mapping as `(logical, physical)` pairs sorted by logical ID; `out` must have room for `2 * len` u32 values.

### layout_p2l_map_len(ptr) / layout_p2l_map(ptr, out, len)

The physical→logical mapping as `(physical, logical)` pairs sorted by physical ID; `out` must have room for `2 * len` u32 values.

### layout_logical_qubits_len(ptr) / layout_logical_qubits(ptr, out, len)

The mapped logical qubit IDs, ascending.

### layout_physical_qubits_len(ptr) / layout_physical_qubits(ptr, out, len)

All physical qubit IDs available to the layout (including vacant ones), ascending.

### layout_vacant_physical_qubits_len(ptr) / layout_vacant_physical_qubits(ptr, out, len)

The vacant physical qubit IDs, ascending.

---

## Layout result helpers

These helpers read the SABRE layout artifacts: the all-pairs distance table of a physical layout graph and the per-logical-qubit activity weights of a circuit layout analysis. The handles come from the compile-side layout API ([Initial Layout](../4_compile/3_layout.md)): `CPhysicalLayoutGraph` from `physical_layout_graph_from_device`, `CCircuitLayoutAnalysis` from `analyze_circuit_for_layout` (or from `layout_result_analysis`), and `CPreparedSabreCircuit` from `prepare_sabre_circuit`.

### layout_result_distances(ptr, out, len)

Copies the all-pairs undirected shortest-path distance table of the physical layout graph into `out` (two-step pattern). The table is row-major over the graph's physical qubits in ascending ID order (`n * n` entries for `n` qubits; `physical_layout_graph_physical_qubits` gives the axis order). Unreachable pairs are written as `UINT32_MAX`.

Parameters:

- `ptr` (`const struct CPhysicalLayoutGraph*`): physical layout graph.
- `out` (`uint32_t*`): receives the table; NULL only queries the length.
- `len` (`uintptr_t`): buffer capacity in entries; a smaller value copies only the first `len` entries.

Returns: the total number of entries (`n * n`); 0 for NULL.

### layout_result_logical_activity(ptr, out_qubits, out_activity, len)

Copies the per-logical-qubit activity weights (the sum of incident interaction weights) into parallel arrays sorted by logical ID (two-step pattern): `out_qubits[i]` receives the logical qubit ID and `out_activity[i]` its activity. Either array may be NULL only when both are NULL (length query).

Parameters:

- `ptr` (`const struct CCircuitLayoutAnalysis*`): circuit layout analysis.
- `out_qubits` (`uint32_t*`): receives the logical qubit IDs.
- `out_activity` (`double*`): receives the activity weights.
- `len` (`uintptr_t`): buffer capacity in entries; a smaller value copies only the first `len` entries.

Returns: the total number of entries; 0 for NULL.

### layout_result_analysis(ptr)

Extracts the reusable circuit layout analysis captured by the prepared SABRE circuit as an owned `CCircuitLayoutAnalysis*` — the same handle type used by `analyze_circuit_for_layout`; free it with `circuit_layout_analysis_free`.

Parameters:

- `ptr` (`const struct CPreparedSabreCircuit*`): prepared SABRE circuit.

Returns: a newly allocated `CCircuitLayoutAnalysis*` on success; NULL on NULL input.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* logical {0, 1} mapped in order onto physical {10, 11}; 3 physical total (12 vacant) */
    uint32_t logical[2] = {0, 1};
    uint32_t physical[3] = {10, 11, 12};
    struct CLayout *layout = layout_new(logical, 2, physical, 3);
    if (layout == NULL) {
        return 1;
    }

    uintptr_t n_logical = layout_num_logical(layout);          /* 2 */
    uintptr_t n_physical = layout_num_physical(layout);       /* 3 */
    uintptr_t n_vacant = layout_num_vacant_physical(layout);  /* 1 */

    uint32_t p = layout_get(layout, 0);                 /* 10 */
    uint32_t l = layout_get_logical(layout, 10);         /* 0 */
    uint32_t vacant_l = layout_get_logical(layout, 12);  /* UINT32_MAX (vacant) */
    int32_t is_vacant = layout_is_physical_vacant(layout, 12);  /* 1 */

    /* bulk read */
    uintptr_t pairs_len = layout_l2p_map_len(layout);  /* 2 */
    uint32_t pairs[4];
    layout_l2p_map(layout, pairs, pairs_len);         /* {0,10, 1,11} */

    /* swap physical 11 and 12: logical 1 moves to 12 */
    layout_swap_physical(layout, 11, 12);

    /* unbind logical 0: physical 10 is released */
    uint32_t freed = 0;
    layout_unbind(layout, 0, &freed);  /* 0, freed == 10 */

    /* rebind onto the vacant qubit 11 */
    int32_t rc = layout_bind(layout, 0, 11);  /* 0 */

    /* layout result helpers */
    struct CDevice *dev = device_line("mock_backend", 3);
    struct CPhysicalLayoutGraph *graph = physical_layout_graph_from_device(dev);
    uintptr_t n_dist = layout_result_distances(graph, NULL, 0);  /* 9 (3x3) */
    uint32_t dist[9];
    layout_result_distances(graph, dist, n_dist);
    /* dist == {0,1,2, 1,0,1, 2,1,0} row-major */

    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 1);
    circuit_cx(circuit, 0, 1);
    circuit_cx(circuit, 1, 2);

    struct CCircuitLayoutAnalysis *analysis = analyze_circuit_for_layout(circuit);
    uintptr_t n_act = layout_result_logical_activity(analysis, NULL, NULL, 0);  /* 3 */
    uint32_t lq[3];
    double act[3];
    layout_result_logical_activity(analysis, lq, act, n_act);
    /* lq == {0,1,2}, act == {2.0, 3.0, 1.0} */
    circuit_layout_analysis_free(analysis);

    /* reuse the analysis captured by the prepared SABRE circuit */
    struct CPreparedSabreCircuit *prepared = prepare_sabre_circuit(circuit);
    struct CCircuitLayoutAnalysis *reused = layout_result_analysis(prepared);
    circuit_layout_analysis_free(reused);
    prepared_sabre_circuit_free(prepared);

    circuit_free(circuit);
    physical_layout_graph_free(graph);
    device_free(dev);

    layout_free(layout);
    return 0;
}
```
