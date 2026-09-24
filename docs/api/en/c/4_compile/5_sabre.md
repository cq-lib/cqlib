# SABRE (C)

`CSabreRoutingResult` is the result handle of the low-level SABRE routing entry `sabre_route`. This page covers construction and validation of the SABRE configuration struct `SabreConfigC`, the routing entry with a known initial layout, the routing-result readers, plus the layout normalizer `normalize_initial_layout` and the reachability checker `validate_reachable_interactions`. A fixed random seed makes the tie-breaking and choices of the randomized trials reproducible. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md); the layout-selecting routing entries `route_sabre` / `route_with_layout` are documented in [Routing](4_routing.md).

---

## Constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `SABRE_MAX_LOOKAHEAD_WEIGHTS` | 8 | Length of the `SabreHeuristicConfigC.lookahead_weights` array; the configuration cannot be converted (-8) when `num_lookahead_weights` exceeds this value. |

---

## Configuration Construction and Validation

### sabre_config_default()

Returns the core default SABRE configuration (by value): `layout_trials=10`, `layout_assignment_budget=1000000`, VF2 prepass enabled (`candidate_limit=10`, `call_limit=1000000`), `refinement_iterations=1`, `routing_trials=1`, no seed; the heuristic uses `basic_weight=1.0`, `lookahead_weights=[0.5, 0.25, 0.125, 0.0625, 0.03125]`, congestion control enabled (`decay_increment=0.002`, `decay_reset=10`), `attempt_limit=1000`, and `best_epsilon=1e-10`.

```c
struct SabreConfigC sabre_config_default(void);
```

Returns: the default configuration struct (by value, nothing to free).

### sabre_config_deterministic_seeded(seed)

Returns a compact deterministic configuration suited to reproducible runs: shrunken trial sizes (`layout_trials=2`, `layout_assignment_budget=100000`, VF2 prepass `candidate_limit=10`, `call_limit=100000`, `refinement_iterations=1`, `routing_trials=1`), the random seed fixed to `seed` (`has_seed=1`), and `attempt_limit` tightened to 20; the remaining fields match the default configuration.

```c
struct SabreConfigC sabre_config_deterministic_seeded(uint64_t seed);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `seed` | `uint64_t` | Deterministic random seed. The same seed, circuit, and device produce the same routing result. |

Returns: the deterministic configuration struct (by value, nothing to free).

### sabre_config_validate(config)

Validates the routing-relevant fields of a configuration: `routing_trials` must be greater than 0; the weights (`basic_weight`, each `lookahead_weights` entry, `decay_increment`) must be finite and non-negative; when congestion control is enabled, `decay_reset` must be greater than 0; `best_epsilon` must be finite and non-negative. Layout-refinement fields (such as `layout_trials`) are not validated.

```c
int32_t sabre_config_validate(const struct SabreConfigC *config);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `config` | `const SabreConfigC*` | Configuration to validate. |

Returns: 0 when valid.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `config` is NULL. |
| `-6` | CompilerError | Core validation rejects: `routing_trials` is 0, a weight is invalid, congestion control is enabled but `decay_reset` is 0, or `best_epsilon` is invalid. |
| `-8` | InvalidParam | `num_lookahead_weights` exceeds `SABRE_MAX_LOOKAHEAD_WEIGHTS`, so the configuration cannot be converted. |

---

## Routing Entry

### sabre_route(circuit, device, initial_layout, config)

Low-level SABRE routing with a known initial layout: inserts SWAPs so that every two-qubit interaction becomes adjacent on the device's available physical topology; control-flow blocks are routed recursively and restore their entry layout before leaving the block. The returned circuit uses physical qubit identifiers.

```c
struct CSabreRoutingResult *sabre_route(const struct CCircuit *circuit,
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
| `config` | `const SabreConfigC*` | SABRE configuration; NULL selects the default. |

Returns: a new `CSabreRoutingResult*` on success (free with `sabre_routing_result_free`); NULL when any handle is NULL, the configuration cannot be converted or fails validation (for example `routing_trials` is 0), or routing fails.

---

## Result Readers

### sabre_routing_result_free(ptr)

Frees a routing-result handle; NULL is allowed.

```c
void sabre_routing_result_free(struct CSabreRoutingResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `struct CSabreRoutingResult*` | Handle to free; may be NULL. |

### sabre_routing_result_circuit(ptr)

Extracts the physical circuit with SWAPs inserted (deep copy; the circuit's qubits are physical qubit identifiers).

```c
struct CCircuit *sabre_routing_result_circuit(const struct CSabreRoutingResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | Routing-result handle. |

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input.

### sabre_routing_result_initial_layout(ptr)

Extracts the initial logical-to-physical layout used by the selected trial (deep copy).

```c
struct CLayout *sabre_routing_result_initial_layout(const struct CSabreRoutingResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### sabre_routing_result_final_layout(ptr)

Extracts the final logical-to-physical layout after all routing operations (including control-flow epilogues) have executed (deep copy).

```c
struct CLayout *sabre_routing_result_final_layout(const struct CSabreRoutingResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | Routing-result handle. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### sabre_routing_result_swap_count(ptr)

Returns the number of inserted SWAP operations (including control-flow epilogues).

```c
uintptr_t sabre_routing_result_swap_count(const struct CSabreRoutingResult *ptr);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | Routing-result handle. |

Returns: the SWAP count; 0 on NULL input.

### sabre_routing_result_diagnostics(ptr, out)

Writes a diagnostics snapshot of the routing run into `*out`.

```c
int32_t sabre_routing_result_diagnostics(const struct CSabreRoutingResult *ptr,
                                         struct CSabreRoutingDiagnostics *out);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | Routing-result handle. |
| `out` | `struct CSabreRoutingDiagnostics*` | Diagnostics snapshot output. |

Returns: 0 on success.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` or `out` is NULL. |

---

## Layout Utilities

### normalize_initial_layout(logical_qubits, num_logical, device, initial_layout)

Normalizes a complete logical-to-physical layout onto the device's usable physical qubits: every logical qubit in `logical_qubits` must already be mapped by `initial_layout`, and each mapping target must be a usable physical qubit of the device; the returned layout's physical-qubit set is the full set of the device's usable physical qubits.

```c
struct CLayout *normalize_initial_layout(const uint32_t *logical_qubits,
                                         uintptr_t num_logical,
                                         const struct CDevice *device,
                                         const struct CLayout *initial_layout);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `logical_qubits` | `const uint32_t*` | Array of logical qubit IDs; may be NULL when `num_logical` is 0. |
| `num_logical` | `uintptr_t` | Number of logical qubits. |
| `device` | `const CDevice*` | Target device. |
| `initial_layout` | `const CLayout*` | Layout to normalize. |

Returns: a new `CLayout*` on success (free with `layout_free`); NULL when `device` or `initial_layout` is NULL, `num_logical` is greater than 0 but `logical_qubits` is NULL, or normalization fails (a logical qubit is unmapped, or mapped to an unusable physical qubit).

### validate_reachable_interactions(circuit, device, initial_layout)

Without performing routing, validates that starting from `initial_layout` every interaction in the circuit is reachable (both sides of the interaction lie in the same connected component of the device topology, so SWAPs can bring them together). The layout is first normalized onto the device's usable physical qubits before validation.

```c
int32_t validate_reachable_interactions(const struct CCircuit *circuit,
                                        const struct CDevice *device,
                                        const struct CLayout *initial_layout);
```

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | Input circuit. |
| `device` | `const CDevice*` | Target device. |
| `initial_layout` | `const CLayout*` | Initial logical-to-physical mapping. |

Returns: 0 when every interaction is reachable.

| Error code | Name | Scenario |
| --- | --- | --- |
| `-1` | NullPtr | Any handle is NULL. |
| `-6` | CompilerError | An unreachable interaction exists (for example one end of an interaction lands on an uncoupled physical qubit), or the layout cannot be normalized. |

---

## SabreConfigC

Configuration shared by SABRE layout refinement and routing, passed by value:

```c
typedef struct SabreConfigC {
  uintptr_t layout_trials;             /* Randomized starting layouts. */
  uintptr_t layout_assignment_budget;  /* Max layout assignment states. */
  uint8_t vf2_prepass_enabled;         /* 1 enables the bounded VF2 prepass. */
  struct SabreVf2PrepassConfigC vf2_prepass;
  uintptr_t refinement_iterations;     /* Refinement rounds per layout trial. */
  uintptr_t routing_trials;            /* Complete routing trials per layout. */
  uint64_t seed;                       /* Used when has_seed != 0. */
  uint8_t has_seed;                    /* 1 uses seed. */
  struct SabreHeuristicConfigC heuristic;
  uint64_t _reserved[4];               /* Reserved padding. */
} SabreConfigC;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `layout_trials` | `uintptr_t` | Number of randomized starting layouts added on top of the deterministic candidates. |
| `layout_assignment_budget` | `uintptr_t` | Maximum number of layout-component assignment states to explore; when exhausted, a budget exhaustion is reported, distinct from proven infeasibility. |
| `vf2_prepass_enabled` | `uint8_t` | 1 enables the bounded VF2 prepass, 0 disables it (`vf2_prepass` is then ignored). |
| `vf2_prepass` | `struct SabreVf2PrepassConfigC` | VF2 prepass limits (used when `vf2_prepass_enabled != 0`). |
| `refinement_iterations` | `uintptr_t` | Forward/backward refinement rounds per layout trial. |
| `routing_trials` | `uintptr_t` | Complete routing trials executed per fully refined layout; must be greater than 0. |
| `seed` | `uint64_t` | Deterministic seed (used when `has_seed != 0`). |
| `has_seed` | `uint8_t` | 1 uses `seed`, 0 leaves the seed unset. |
| `heuristic` | `struct SabreHeuristicConfigC` | Swap-selection heuristic configuration. |
| `_reserved[4]` | `uint64_t[4]` | Padding that keeps the struct layout stable; do not write non-zero values. |

### SabreVf2PrepassConfigC

```c
typedef struct SabreVf2PrepassConfigC {
  uintptr_t candidate_limit;  /* Complete perfect mappings scored. */
  uintptr_t call_limit;       /* Partial mapping extensions attempted. */
  uint64_t _reserved[2];      /* Reserved padding. */
} SabreVf2PrepassConfigC;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `candidate_limit` | `uintptr_t` | Upper bound on complete perfect mappings scored by VF2. |
| `call_limit` | `uintptr_t` | Upper bound on partial mapping extensions attempted by VF2. |
| `_reserved[2]` | `uint64_t[2]` | Padding that keeps the struct layout stable. |

### SabreHeuristicConfigC

SABRE swap-selection heuristic configuration: the main score combines the current front-layer total distance, the lookahead layers scaled by active layer, and the multiplicative congestion; when native solutions exist, candidates are adjudicated within a narrow structural window by the exact native two-qubit cost.

```c
typedef struct SabreHeuristicConfigC {
  double basic_weight;     /* Front-layer total distance weight. */
  double lookahead_weights[SABRE_MAX_LOOKAHEAD_WEIGHTS];
  uintptr_t num_lookahead_weights;  /* Leading entries in use. */
  double decay_increment;  /* Congestion multiplier increment. */
  uint8_t decay_enabled;   /* 1 enables congestion control. */
  uintptr_t decay_reset;   /* Attempts before decay resets. */
  uintptr_t attempt_limit; /* Heuristic SWAPs before fallback. */
  double best_epsilon;     /* Tie tolerance for candidate scores. */
  uint8_t _reserved[8];    /* Reserved padding. */
} SabreHeuristicConfigC;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `basic_weight` | `double` | Weight of the current front-layer total distance; must be finite and non-negative. |
| `lookahead_weights` | `double[8]` | Weights scaling the lookahead-layer totals per active layer; only the leading `num_lookahead_weights` entries take effect, the rest are padding. |
| `num_lookahead_weights` | `uintptr_t` | Number of leading valid entries in `lookahead_weights`; must not exceed `SABRE_MAX_LOOKAHEAD_WEIGHTS`. |
| `decay_increment` | `double` | Increment applied to a physical qubit's congestion multiplier after a heuristic SWAP uses it. |
| `decay_enabled` | `uint8_t` | 1 enables congestion control (uses `decay_increment`), 0 disables it. |
| `decay_reset` | `uintptr_t` | Number of heuristic SWAP attempts before the decay values reset; must be greater than 0 when congestion control is enabled. |
| `attempt_limit` | `uintptr_t` | Number of consecutive heuristic SWAPs allowed without routing a front-layer node before falling back to shortest-path escape. |
| `best_epsilon` | `double` | Floating-point tolerance within which candidate SWAP scores count as tied; must be finite and non-negative. |
| `_reserved[8]` | `uint8_t[8]` | Padding that keeps the struct layout stable. |

---

## CSabreRoutingDiagnostics

Diagnostics snapshot of a routing run (the output of `sabre_routing_result_diagnostics` and the diagnostics interfaces of [Routing](4_routing.md)):

| Field | Type | Meaning |
| --- | --- | --- |
| `trials_evaluated` | `uintptr_t` | Number of routing trials evaluated. |
| `selected_trial_index` | `uintptr_t` | Zero-based index of the selected routing trial. |
| `fallback_count` | `uintptr_t` | Number of shortest-path fallbacks used. |
| `control_flow_blocks_routed` | `uintptr_t` | Number of control-flow blocks routed recursively. |
| `two_qubit_depth` | `uintptr_t` | ASAP two-qubit depth of the selected routed operation stream. |
| `operation_count` | `uintptr_t` | Total number of operations in the selected routed stream. |
| `native_two_qubit_count` | `uintptr_t` | Predicted native two-qubit operation count after device lowering. |
| `native_two_qubit_depth` | `uintptr_t` | ASAP two-qubit depth using the selected exact-qargs native solutions. |
| `native_total_depth` | `uintptr_t` | ASAP total depth using the selected exact-qargs native solutions. |
| `native_operation_count` | `uintptr_t` | Predicted total native operation count after device lowering. |
| `unknown_loop_count` | `uintptr_t` | Number of dynamic `for`/`while` loops whose total execution cost is unknown. |
| `requirement_signature_count` | `uintptr_t` | Number of distinct single-qubit / qubit-pair requirement signatures prepared. |
| `eager_pair_state_count` | `uintptr_t` | Number of qubit-pair placement lower-bound states the target eagerly retains. |
| `lazy_pair_l1_lookup_count` | `uintptr_t` | Number of lazy qubit-pair state lower-bound probes issued by the selected trial. |
| `lazy_pair_l1_hit_count` | `uintptr_t` | Number of probes served from the selected trial's local L1 cache. |
| `lazy_pair_l1_cached_count` | `uintptr_t` | Number of qubit-pair state entries retained by the selected trial's bounded L1 cache. |

---

## Example

On the line topology 0-1-2, run low-level SABRE routing from the identity layout, then normalize the layout and validate reachability:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Line topology 0 - 1 - 2 with a non-adjacent cx(0, 2) */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* 2. Deterministic seeded config: same seed -> same routing result */
    struct SabreConfigC config = sabre_config_deterministic_seeded(42);
    if (sabre_config_validate(&config) != 0) {
        circuit_free(circuit);
        device_free(device);
        return 1;
    }

    /* 3. Low-level SABRE route from the identity layout */
    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    struct CLayout *layout = layout_new(logical, 3, physical, 3);

    struct CSabreRoutingResult *result = sabre_route(circuit, device, layout, &config);
    if (result != NULL) {
        /* 4. Routed circuit, swap count, layouts, and diagnostics */
        struct CCircuit *routed = sabre_routing_result_circuit(result);
        uintptr_t swaps = sabre_routing_result_swap_count(result);  /* >= 1 */
        struct CLayout *final_layout = sabre_routing_result_final_layout(result);
        struct CSabreRoutingDiagnostics diagnostics;
        if (sabre_routing_result_diagnostics(result, &diagnostics) == 0) {
            printf("trials=%lu swaps=%lu\n",
                   (unsigned long)diagnostics.trials_evaluated,
                   (unsigned long)swaps);
        }
        circuit_free(routed);
        layout_free(final_layout);
        sabre_routing_result_free(result);
    }

    /* 5. Normalize the layout onto usable qubits */
    struct CLayout *normalized = normalize_initial_layout(logical, 3, device, layout);
    if (normalized != NULL) {
        layout_free(normalized);
    }

    /* 6. Reachability check: qubits 0 and 2 share one connected component */
    int32_t reachable = validate_reachable_interactions(circuit, device, layout);  /* 0 */

    layout_free(layout);
    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

Use `route_sabre` when the initial layout should be selected automatically; see [Routing](4_routing.md).
