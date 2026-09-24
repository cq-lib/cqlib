# Routing (C)

This page covers the transform-level routing entry points `route_with_layout` / `route_sabre` and every read accessor of the routing-result handles `CRoutedCircuit` / `CSabreRouteResult`: the routed circuit, initial and final layouts, the SWAP count, routing diagnostics, and layout metadata. Routing inserts SWAP operations on the device topology so that every two-qubit interaction lands on adjacent physical qubits; the routed circuit uses physical qubit identifiers. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md); the `SabreConfigC` configuration struct, the `CSabreRoutingDiagnostics` struct, and the low-level `sabre_route` are documented in [SABRE](5_sabre.md).

---

## Constants

Layout-objective tags (the `objective` parameter of `route_sabre`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | Topology-only scoring: distance and direction terms. |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | Adds fidelity terms on top of the topology score (two-qubit error weight 10.0, readout error weight 1.0). |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | Chooses automatically from device data: enables the fidelity terms the device carries (two-qubit gate / readout error data), otherwise falls back to topology-only. |
| `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` | 3 | Requires fidelity terms: like fidelity-aware, but fails when the device carries no usable calibration data. |

---

## Routing Entry Points

### route_with_layout(circuit, device, initial_layout, config)

Deterministic routing from a caller-supplied initial layout: no layout selection, refinement, or scoring is performed. Intended for callers that already hold a layout (for example from a layout search or a previous SABRE run).

```c
struct CRoutedCircuit *route_with_layout(const struct CCircuit *circuit,
                                         const struct CDevice *device,
                                         const struct CLayout *initial_layout,
                                         const struct SabreConfigC *config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `device` | `const CDevice*` | Target device (construction in [Device / Properties](../2_device/2_properties_device.md)). |
| `initial_layout` | `const CLayout*` | Initial logical-to-physical mapping (construction in [Layout](../2_device/3_layout.md)). |
| `config` | `const SabreConfigC*` | SABRE configuration; NULL selects the default (see [SABRE](5_sabre.md)). |

Returns: a new `CRoutedCircuit*` on success (free with `routed_circuit_free`); NULL when any handle is NULL, the configuration cannot be converted (for example `num_lookahead_weights` exceeds 8), or routing fails (invalid configuration, insufficient usable physical qubits, unreachable interactions, ...).

### route_sabre(circuit, device, objective, config)

SABRE routing with layout selection: first selects an initial layout under `objective`, then routes the original forward circuit from that layout. The selected layout is returned alongside the routing result.

```c
struct CSabreRouteResult *route_sabre(const struct CCircuit *circuit,
                                      const struct CDevice *device,
                                      uint8_t objective,
                                      const struct SabreConfigC *config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `device` | `const CDevice*` | Target device. |
| `objective` | `uint8_t` | Layout objective, one of `LAYOUT_OBJECTIVE_*`. |
| `config` | `const SabreConfigC*` | SABRE configuration; NULL selects the default. Equal seeds produce equal routing results for the same circuit and device. |

Returns: a new `CSabreRouteResult*` on success (free with `sabre_route_result_free`); NULL when any handle is NULL, `objective` is not a valid tag, the configuration cannot be converted, or layout selection / routing fails.

---

## RoutedCircuit Accessors

Result handle of `route_with_layout`.

### routed_circuit_free(ptr)

Frees a routing-result handle; NULL is allowed.

```c
void routed_circuit_free(struct CRoutedCircuit *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CRoutedCircuit*` | Handle to free; may be NULL. |

### routed_circuit_circuit(ptr)

Retrieves the routed physical circuit (a deep clone; the circuit qubits are physical qubit identifiers and every two-qubit operation is adjacent in the usable physical topology).

```c
struct CCircuit *routed_circuit_circuit(const struct CRoutedCircuit *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input.

### routed_circuit_initial_layout(ptr)

Retrieves the initial logical-to-physical layout used before routing (deep clone).

```c
struct CLayout *routed_circuit_initial_layout(const struct CRoutedCircuit *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### routed_circuit_final_layout(ptr)

Retrieves the final logical-to-physical layout after all routed operations (deep clone).

```c
struct CLayout *routed_circuit_final_layout(const struct CRoutedCircuit *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### routed_circuit_swap_count(ptr)

Returns the number of inserted SWAP operations.

```c
uintptr_t routed_circuit_swap_count(const struct CRoutedCircuit *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |

Returns: the SWAP count; 0 for NULL.

### routed_circuit_diagnostics(ptr, out)

Writes the routing-diagnostics snapshot to `*out` (field meanings in [SABRE](5_sabre.md), `CSabreRoutingDiagnostics`).

```c
int32_t routed_circuit_diagnostics(const struct CRoutedCircuit *ptr,
                                   struct CSabreRoutingDiagnostics *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |
| `out` | `struct CSabreRoutingDiagnostics*` | Diagnostics snapshot output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |

### routed_circuit_changed(ptr, original)

Tests whether routing observably changed `original`: changed when SWAP operations were inserted, the physical qubit set differs from the input, the global phase changed, the initial layout is not the identity mapping, or the operation stream differs structurally.

```c
int32_t routed_circuit_changed(const struct CRoutedCircuit *ptr, const struct CCircuit *original);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | Routing-result handle. |
| `original` | `const CCircuit*` | The original circuit before routing. |

Returns: 1 when changed; 0 otherwise.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `original` is NULL. |

---

## SabreRouteResult Accessors

Result handle of `route_sabre`; carries the routed data plus metadata about the selected layout.

### sabre_route_result_free(ptr)

Frees a routing-result handle; NULL is allowed.

```c
void sabre_route_result_free(struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CSabreRouteResult*` | Handle to free; may be NULL. |

### sabre_route_result_routed(ptr)

Retrieves the embedded routed circuit and routing metadata (deep clone).

```c
struct CRoutedCircuit *sabre_route_result_routed(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: a new `CRoutedCircuit*` on success (free with `routed_circuit_free`); NULL on NULL input.

### sabre_route_result_circuit(ptr)

Retrieves the routed physical circuit (deep clone); equivalent to reading the circuit of `routed`.

```c
struct CCircuit *sabre_route_result_circuit(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input.

### sabre_route_result_initial_layout(ptr)

Retrieves the initial logical-to-physical layout used for routing (deep clone).

```c
struct CLayout *sabre_route_result_initial_layout(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### sabre_route_result_final_layout(ptr)

Retrieves the final logical-to-physical layout after routing (deep clone).

```c
struct CLayout *sabre_route_result_final_layout(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### sabre_route_result_swap_count(ptr)

Returns the number of inserted SWAP operations.

```c
uintptr_t sabre_route_result_swap_count(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: the SWAP count; 0 for NULL.

### sabre_route_result_diagnostics(ptr, out)

Writes the routing-diagnostics snapshot to `*out` (field meanings in [SABRE](5_sabre.md), `CSabreRoutingDiagnostics`).

```c
int32_t sabre_route_result_diagnostics(const struct CSabreRouteResult *ptr,
                                       struct CSabreRoutingDiagnostics *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |
| `out` | `struct CSabreRoutingDiagnostics*` | Diagnostics snapshot output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |

### sabre_route_result_changed(ptr, original)

Tests whether routing observably changed `original`; the decision rule matches `routed_circuit_changed`.

```c
int32_t sabre_route_result_changed(const struct CSabreRouteResult *ptr,
                                   const struct CCircuit *original);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |
| `original` | `const CCircuit*` | The original circuit before routing. |

Returns: 1 when changed; 0 otherwise.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `original` is NULL. |

---

## Layout Metadata

Additional information about the layout selected by `route_sabre`.

### sabre_route_result_layout_score(ptr, out)

Writes the layout-score snapshot of the selected initial layout to `*out`. The score is diagnostic only; SABRE selects the winner by predicted native routing quality.

```c
int32_t sabre_route_result_layout_score(const struct CSabreRouteResult *ptr,
                                        struct CLayoutScore *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |
| `out` | `struct CLayoutScore*` | Layout-score snapshot output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |
| `-8` | InvalidParam | The result carries no layout score. |

`CLayoutScore` fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `total` | `double` | Weighted total according to the objective. |
| `distance` | `double` | Raw weighted-distance component. |
| `direction` | `double` | Raw direction-mismatch component. |
| `two_qubit_error` | `double` | Unweighted effective two-qubit placement cost. |
| `readout_error` | `double` | Raw readout error component. |
| `used_fidelity` | `uint8_t` | 1 when the objective used fidelity terms. |
| `_reserved[7]` | `uint8_t[7]` | Reserved padding that keeps the struct layout stable. |

### sabre_route_result_layout_is_perfect(ptr)

Tests whether the selected layout places every positive-weight interaction directly on an adjacent hardware edge (no SWAP needed at all).

```c
int32_t sabre_route_result_layout_is_perfect(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: 1 when perfect; 0 otherwise.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL. |

### sabre_route_result_layout_candidates_evaluated(ptr)

Returns the number of candidate layouts evaluated while selecting the initial layout.

```c
uintptr_t sabre_route_result_layout_candidates_evaluated(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: the candidate count; 0 for NULL.

### sabre_route_result_layout_used_fidelity(ptr)

Tests whether fidelity data contributed to the selected layout score.

```c
int32_t sabre_route_result_layout_used_fidelity(const struct CSabreRouteResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |

Returns: 1 when fidelity data contributed; 0 otherwise.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` is NULL. |

### sabre_route_result_layout_notes_len(ptr) / sabre_route_result_layout_note(ptr, index)

Two-step read of the layout diagnostic notes: `*_len` returns the number of notes (0 for NULL), and `layout_note` returns note `index`.

```c
uintptr_t sabre_route_result_layout_notes_len(const struct CSabreRouteResult *ptr);
```

```c
char *sabre_route_result_layout_note(const struct CSabreRouteResult *ptr, uintptr_t index);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | Routing-result handle. |
| `index` | `uintptr_t` | Note index (for `layout_note`). |

Returns: `layout_note` returns a heap-allocated C string; free with `cqlib_string_free`. NULL on NULL input or out-of-bounds index.

---

## Example

Routing `cx(0, 2)` on a line topology 0–1–2 and reading all metadata of both result kinds:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Line topology 0 - 1 - 2 and a non-adjacent cx(0, 2) */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* 2. Route from an explicit identity layout (logical i -> physical i) */
    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    struct CLayout *layout = layout_new(logical, 3, physical, 3);

    struct CRoutedCircuit *routed = route_with_layout(circuit, device, layout, NULL);
    if (routed != NULL) {
        /* 3. Read the routed circuit, layouts, and diagnostics */
        uintptr_t swaps = routed_circuit_swap_count(routed);       /* >= 1 */
        int32_t changed = routed_circuit_changed(routed, circuit); /* 1 */
        struct CCircuit *physical_circuit = routed_circuit_circuit(routed);
        struct CLayout *initial = routed_circuit_initial_layout(routed);
        struct CLayout *final_layout = routed_circuit_final_layout(routed);
        struct CSabreRoutingDiagnostics diagnostics;
        if (routed_circuit_diagnostics(routed, &diagnostics) == 0) {
            printf("trials=%lu swaps=%lu changed=%d\n",
                   (unsigned long)diagnostics.trials_evaluated,
                   (unsigned long)swaps, changed);
        }
        circuit_free(physical_circuit);
        layout_free(initial);
        layout_free(final_layout);
        routed_circuit_free(routed);
    }
    layout_free(layout);

    /* 4. route_sabre: select a layout and route in one call */
    struct CSabreRouteResult *result =
        route_sabre(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    if (result != NULL) {
        struct CLayoutScore score;
        if (sabre_route_result_layout_score(result, &score) == 0) {
            printf("layout score total=%f perfect=%d\n",
                   score.total, sabre_route_result_layout_is_perfect(result));
        }
        printf("candidates=%lu\n",
               (unsigned long)sabre_route_result_layout_candidates_evaluated(result));

        /* 5. Walk the layout diagnostic notes (two-step pattern) */
        uintptr_t notes = sabre_route_result_layout_notes_len(result);
        for (uintptr_t i = 0; i < notes; i++) {
            char *note = sabre_route_result_layout_note(result, i);
            if (note != NULL) {
                cqlib_string_free(note);
            }
        }

        struct CRoutedCircuit *inner = sabre_route_result_routed(result);
        if (inner != NULL) {
            routed_circuit_free(inner);
        }
        sabre_route_result_free(result);
    }

    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

Lowering a routed result to the device native gate set is done with `decompose_lower_to_device` in [Decompose / Resynthesis](6_decompose_resynthesis.md); the full compilation workflow (routing and lowering included) is shown in [Compiler](1_compiler.md).
