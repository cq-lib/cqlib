# Topology (C)

`CTopology` is an opaque handle to the hardware connectivity graph: physical qubits and the directed couplings between them. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md); every entry point on this page is declared in `cqlib_c.h`.

A `CTopology` handle comes from one of two places: `topology_line` builds a line topology from a qubit-ID array, and `device_topology` returns a clone of a device's topology (see [Device / Properties](2_properties_device.md)). Couplings are directed: a coupling on `(control, target)` does not imply the reverse direction.

---

## Construction and release

### topology_line(qubits, num_qubits)

Builds a directed line `qubits[0] -> qubits[1] -> ...` in the given order; coupling names are empty strings.

Parameters:

- `qubits` (`const uint32_t*`): array of physical qubit IDs.
- `num_qubits` (`uintptr_t`): array length.

Returns: a newly allocated `CTopology*` on success; NULL on a NULL pointer or construction failure.

### topology_free(ptr)

Frees a `CTopology`; NULL is allowed.

Parameters:

- `ptr` (`struct CTopology*`): handle to free.

### topology_num_qubits(ptr)

Returns the number of qubits in the topology; 0 for NULL.

Parameters:

- `ptr` (`const struct CTopology*`): topology handle.

### topology_num_couplings(ptr)

Returns the number of coupling edges in the topology; 0 for NULL.

Parameters:

- `ptr` (`const struct CTopology*`): topology handle.

---

## Editing qubits and couplings

### topology_add_qubits(ptr, qubits, len)

Adds a set of qubits to the topology.

Parameters:

- `ptr` (`struct CTopology*`): topology handle.
- `qubits` (`const uint32_t*`): array of qubit IDs.
- `len` (`uintptr_t`): array length.

Returns: 0 on success; -1 on NULL; -8 when a qubit already exists in the topology or is listed twice.

### topology_remove_qubits(ptr, qubits, len)

Removes a set of qubits and every coupling touching them.

Parameters:

- `ptr` (`struct CTopology*`): topology handle.
- `qubits` (`const uint32_t*`): array of qubit IDs.
- `len` (`uintptr_t`): array length.

Returns: 0 on success; -1 on NULL; -2 when a qubit is not in the topology; -8 when a qubit is listed twice.

### topology_add_couplings(ptr, edges, num_edges, names)

Adds directed coupling edges. `edges` points to `2 * num_edges` u32 values laid out as consecutive `(control, target)` pairs; `names` optionally points to `num_edges` C strings (NULL entries or a NULL array leave the couplings unnamed).

Parameters:

- `ptr` (`struct CTopology*`): topology handle.
- `edges` (`const uint32_t*`): coupling-edge array.
- `num_edges` (`uintptr_t`): number of edges.
- `names` (`const char *const*`): coupling-name array; may be NULL.

Returns: 0 on success; -1 on NULL; -4 for invalid UTF-8 in a name; -2 when an endpoint qubit is not in the topology; -8 for duplicate or self couplings.

### topology_remove_couplings(ptr, edges, num_edges)

Removes directed coupling edges. `edges` uses the same layout as `topology_add_couplings`.

Parameters:

- `ptr` (`struct CTopology*`): topology handle.
- `edges` (`const uint32_t*`): coupling-edge array.
- `num_edges` (`uintptr_t`): number of edges.

Returns: 0 on success; -1 on NULL; -2 when a qubit or coupling is not in the topology; -8 when a coupling is listed twice.

---

## Queries and traversal

### topology_contains_qubit(ptr, qubit)

Checks whether `qubit` is part of the topology.

Parameters:

- `ptr` (`const struct CTopology*`): topology handle.
- `qubit` (`uint32_t`): qubit ID.

Returns: 1 when present; 0 otherwise; -1 on NULL.

### topology_get_coupling_name(ptr, control, target)

Returns the name of the directed coupling `control -> target`; an existing but unnamed coupling yields an empty string.

Parameters:

- `ptr` (`const struct CTopology*`): topology handle.
- `control` (`uint32_t`): control qubit ID.
- `target` (`uint32_t`): target qubit ID.

Returns: a heap-allocated C string on success (free with `cqlib_string_free`); NULL on NULL input or when the coupling does not exist.

### topology_qubits_len(ptr) / topology_qubits(ptr, out, len)

Two-step output of all qubit IDs in the topology.

- `topology_qubits_len(ptr)` returns the total count; 0 for NULL.
- `topology_qubits(ptr, out, len)` copies the qubit IDs into `out` and returns the total count.

### topology_neighbors_undirected_len(ptr, qubit) / topology_neighbors_undirected(ptr, qubit, out, len)

Two-step output of the undirected neighbors of `qubit` (successors and predecessors, deduplicated, ascending). The length is 0 for NULL or an unknown qubit.

Parameters:

- `qubit` (`uint32_t`): qubit ID.

### topology_predecessors_len(ptr, qubit) / topology_predecessors(ptr, qubit, out, len)

Two-step output of the predecessors of `qubit` (qubits with a directed coupling into it, ascending).

### topology_successors_len(ptr, qubit) / topology_successors(ptr, qubit, out, len)

Two-step output of the successors of `qubit` (qubits reachable through outgoing couplings, ascending).

### topology_undirected_edges_len(ptr) / topology_undirected_edges(ptr, out, len)

Two-step output of the unique undirected coupling pairs: bidirectional couplings collapse to one pair, sorted ascending as `(low, high)`. `out` must have room for `2 * len` u32 values; the fill function returns the total number of pairs.

---

## Degrees and support

### topology_in_degree(ptr, qubit)

Returns the number of incoming couplings to `qubit`; 0 for NULL or an unknown qubit.

### topology_out_degree(ptr, qubit)

Returns the number of outgoing couplings from `qubit`; 0 for NULL or an unknown qubit.

### topology_supports_directed_coupling(ptr, control, target)

Checks whether the directed coupling `control -> target` exists.

Returns: 1 when it exists; 0 otherwise; -1 on NULL.

### topology_supports_coupling_either_direction(ptr, a, b)

Checks whether a coupling between the two qubits exists in either direction.

Returns: 1 when it exists; 0 otherwise; -1 on NULL.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t ids[3] = {0, 1, 2};
    struct CTopology *topo = topology_line(ids, 3);
    if (topo == NULL) {
        return 1;
    }

    /* 3 qubits, 2 directed couplings: 0->1, 1->2 */
    printf("qubits=%llu couplings=%llu\n",
           (unsigned long long)topology_num_qubits(topo),
           (unsigned long long)topology_num_couplings(topo));

    /* connectivity */
    int32_t fwd = topology_supports_directed_coupling(topo, 1, 2);              /* 1 */
    int32_t bwd = topology_supports_directed_coupling(topo, 2, 1);              /* 0 */
    int32_t either = topology_supports_coupling_either_direction(topo, 2, 1);  /* 1 */

    /* neighbors and degrees of qubit 1 */
    uintptr_t succ_len = topology_successors_len(topo, 1);  /* 1 */
    uint32_t succ[1];
    topology_successors(topo, 1, succ, succ_len);          /* {2} */

    uintptr_t nb_len = topology_neighbors_undirected_len(topo, 1);  /* 2 */
    uint32_t nb[2];
    topology_neighbors_undirected(topo, 1, nb, nb_len);             /* {0, 2} */

    uintptr_t in_deg = topology_in_degree(topo, 1);    /* 1 */
    uintptr_t out_deg = topology_out_degree(topo, 1);  /* 1 */

    /* add and remove qubits and couplings */
    uint32_t extra[1] = {3};
    if (topology_add_qubits(topo, extra, 1) == 0) {
        uint32_t edge[2] = {2, 3};
        const char *names[1] = {"CX"};
        topology_add_couplings(topo, edge, 1, names);
        topology_remove_couplings(topo, edge, 1);
        topology_remove_qubits(topo, extra, 1);
    }

    /* undirected edges */
    uintptr_t pairs = topology_undirected_edges_len(topo);  /* 2 */
    uint32_t buf[4];
    topology_undirected_edges(topo, buf, pairs);  /* {0,1, 1,2} */

    topology_free(topo);
    return 0;
}
```
