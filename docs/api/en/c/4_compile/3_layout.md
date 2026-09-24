# Initial Layout (C)

This page covers the C ABI of the initial-layout algorithms and layout analysis tools: the four layout entry points (`trivial_layout` / `greedy_layout` / `vf2_perfect_layout` / `sabre_layout`), circuit interaction analysis and physical-layout-graph queries, the prepared SABRE objects and the prepared layout pipeline, and the read accessors of the layout result `CLayoutResult`. A layout maps logical qubits onto physical qubits and provides the starting point of the routing stage. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md); construction and access of the mapping handle `CLayout` are documented in [Layout](../2_device/3_layout.md).

---

## Constants

Layout objective tags (the `objective` parameter of every layout entry point and of `score_layout`):

| Constant | Value | Objective |
| --- | --- | --- |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | Topology only: weighted distance plus direction mismatch. |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | Fidelity aware: adds error-rate terms on top of the topology terms. |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | Auto-selected from the device's calibration data: fidelity-aware when the physical graph carries calibration data, otherwise topology only. |
| `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` | 3 | Fidelity required: like fidelity-aware, but errors when the physical graph carries no usable calibration data. |

VF2 edge-requirement tags (the `CVf2LayoutConfig.edge_requirement` field):

| Constant | Value | Requirement |
| --- | --- | --- |
| `VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS` | 0 | Only positive-weight interaction edges must match. |
| `VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS` | 1 | All interaction edges must match. |

---

## Data Structures

### CLayoutScore

Layout score snapshot, written by `score_layout` and `layout_result_score`:

```c
typedef struct CLayoutScore {
  double total;              /* Weighted total according to the objective. */
  double distance;           /* Raw weighted-distance component. */
  double direction;          /* Raw direction-mismatch component. */
  double two_qubit_error;    /* Unweighted effective two-qubit placement cost. */
  double readout_error;      /* Raw readout error component. */
  uint8_t used_fidelity;     /* 1 when the objective used fidelity terms. */
  uint8_t _reserved[7];      /* Reserved padding. */
} CLayoutScore;
```

| Field | Meaning |
| --- | --- |
| `total` | Weighted total according to the objective; lower is better. |
| `distance` | Raw weighted-distance component. |
| `direction` | Raw direction-mismatch component. |
| `two_qubit_error` | Unweighted effective two-qubit placement cost. |
| `readout_error` | Raw readout-error component. |
| `used_fidelity` | 1 when the objective used fidelity terms. |

### CInteraction

Snapshot of one logical-qubit interaction, written by `circuit_layout_analysis_interaction`:

| Field | Type | Meaning |
| --- | --- | --- |
| `left` | `uint32_t` | Lower-sorted logical qubit endpoint. |
| `right` | `uint32_t` | Higher-sorted logical qubit endpoint. |
| `weight` | `double` | Total interaction weight of this unordered pair. |
| `directed_weight_left_to_right` | `double` | Weight observed in operation order `left -> right`. |
| `directed_weight_right_to_left` | `double` | Weight observed in operation order `right -> left`. |
| `first_seen_order` | `uintptr_t` | First appearance index in operation order. |

### CVf2LayoutConfig

VF2 perfect-layout configuration:

```c
typedef struct CVf2LayoutConfig {
  uintptr_t candidate_limit;   /* Max complete perfect candidates to score. */
  uintptr_t call_limit;        /* Max partial mapping extensions ((uintptr_t)-1 = none). */
  uint8_t edge_requirement;    /* One of VF2_EDGE_REQUIREMENT_*. */
  uint8_t _reserved[7];        /* Reserved padding. */
} CVf2LayoutConfig;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `candidate_limit` | `uintptr_t` | Maximum number of complete perfect candidates to score. |
| `call_limit` | `uintptr_t` | Maximum number of partial mapping extensions attempted; `(uintptr_t)-1` means no explicit limit. |
| `edge_requirement` | `uint8_t` | One of `VF2_EDGE_REQUIREMENT_*`. |
| `_reserved[7]` | `uint8_t[7]` | Padding that keeps the struct layout stable; do not write non-zero values. |

### vf2_layout_config_default()

```c
struct CVf2LayoutConfig vf2_layout_config_default(void);
```

Returns the default VF2 configuration by value: `candidate_limit = 10`, `call_limit = (uintptr_t)-1` (no limit), `edge_requirement = VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS`.

---

## Layout Entry Points

None of the four entry points modifies the input circuit; free the returned `CLayoutResult*` with `layout_result_free`. `objective` is one of the `LAYOUT_OBJECTIVE_*` tags; `LAYOUT_OBJECTIVE_AUTO` resolves against the device's calibration data (the physical graph is derived from the device), and `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` makes every entry point return NULL when the target carries no usable calibration data.

### trivial_layout(circuit, device, objective)

```c
struct CLayoutResult *trivial_layout(const struct CCircuit *circuit,
                                     const struct CDevice *device,
                                     uint8_t objective);
```

Trivial layout: maps logical qubits onto usable physical qubits in their existing order (logical i goes to the i-th usable physical qubit).

- `circuit` (`const CCircuit*`): input circuit.
- `device` (`const CDevice*`): target device (construction in [Device / Properties](../2_device/2_properties_device.md)).
- `objective` (`uint8_t`): layout objective tag.

Returns: a new `CLayoutResult*` on success; NULL when either argument is NULL, the `objective` tag is unknown, or the device has too few usable physical qubits.

### greedy_layout(circuit, device, objective)

```c
struct CLayoutResult *greedy_layout(const struct CCircuit *circuit,
                                    const struct CDevice *device,
                                    uint8_t objective);
```

Greedy layout: places logical qubits in interaction-strength order, prioritizing strongly interacting pairs.

Parameters and return value as for `trivial_layout`: a new `CLayoutResult*` on success; NULL on NULL arguments, an unknown `objective` tag, or execution failure.

### vf2_perfect_layout(circuit, device, objective, config)

```c
struct CLayoutResult *vf2_perfect_layout(const struct CCircuit *circuit,
                                         const struct CDevice *device,
                                         uint8_t objective,
                                         const struct CVf2LayoutConfig *config);
```

Searches for a perfect initial layout using non-induced VF2++ matching: when the circuit's interaction graph embeds exactly into the device topology, every positive-weight interaction lands on an adjacent hardware edge.

- `circuit` (`const CCircuit*`): input circuit.
- `device` (`const CDevice*`): target device.
- `objective` (`uint8_t`): layout objective tag.
- `config` (`const CVf2LayoutConfig*`): VF2 configuration; NULL selects the default.

Returns: a new `CLayoutResult*` on success; NULL when no perfect mapping exists, when a pointer argument is NULL, when the `objective` tag is unknown, or when `config->edge_requirement` is invalid.

### sabre_layout(circuit, device, objective, config)

```c
struct CLayoutResult *sabre_layout(const struct CCircuit *circuit,
                                   const struct CDevice *device,
                                   uint8_t objective,
                                   const struct SabreConfigC *config);
```

Selects an initial layout using SABRE layout refinement: several candidate layouts are refined forward/backward, scored, and the best one is returned.

- `circuit` (`const CCircuit*`): input circuit.
- `device` (`const CDevice*`): target device.
- `objective` (`uint8_t`): layout objective tag.
- `config` (`const SabreConfigC*`): SABRE configuration; NULL selects the default, field reference in [SABRE Configuration](5_sabre.md).

Returns: a new `CLayoutResult*` on success; NULL on NULL arguments, an unknown `objective` tag, or an invalid configuration.

---

## Circuit Layout Analysis

`analyze_circuit_for_layout` precomputes the weighted logical interactions of a circuit for reuse by the prepared layout entry points.

### analyze_circuit_for_layout(circuit)

```c
struct CCircuitLayoutAnalysis *analyze_circuit_for_layout(const struct CCircuit *circuit);
```

Analyzes the circuit for layout planning (the logical-qubit table and the weighted interaction graph).

- `circuit` (`const CCircuit*`): input circuit.

Returns: a new `CCircuitLayoutAnalysis*` on success (free with `circuit_layout_analysis_free`); NULL on NULL input or failure.

### circuit_layout_analysis_free(ptr)

```c
void circuit_layout_analysis_free(struct CCircuitLayoutAnalysis *ptr);
```

Frees a circuit layout analysis handle; NULL is allowed.

### circuit_layout_analysis_num_logical(ptr)

```c
uintptr_t circuit_layout_analysis_num_logical(const struct CCircuitLayoutAnalysis *ptr);
```

Returns the number of logical qubits in the analysis.

Returns: the logical-qubit count; 0 for NULL input.

### circuit_layout_analysis_logical_qubits(ptr, out, len)

```c
uintptr_t circuit_layout_analysis_logical_qubits(const struct CCircuitLayoutAnalysis *ptr,
                                                 uint32_t *out,
                                                 uintptr_t len);
```

Two-step read: copies the logical qubit IDs in source-circuit order into `out`. First call with `len = 0` and `out = NULL` to query the total, then allocate a buffer and call again with `len` equal to the total.

- `ptr` (`const CCircuitLayoutAnalysis*`): analysis handle.
- `out` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: the total count; 0 for NULL input.

### circuit_layout_analysis_interactions_len(ptr)

```c
uintptr_t circuit_layout_analysis_interactions_len(const struct CCircuitLayoutAnalysis *ptr);
```

Returns the number of weighted logical interactions.

Returns: the interaction count; 0 for NULL input.

### circuit_layout_analysis_interaction(ptr, index, out)

```c
int32_t circuit_layout_analysis_interaction(const struct CCircuitLayoutAnalysis *ptr,
                                            uintptr_t index,
                                            struct CInteraction *out);
```

Writes the interaction at `index` to `*out` (fields in the `CInteraction` table above).

- `ptr` (`const CCircuitLayoutAnalysis*`): analysis handle.
- `index` (`uintptr_t`): interaction index, must be smaller than `circuit_layout_analysis_interactions_len`.
- `out` (`CInteraction*`): interaction snapshot output.

Returns: `0` on success; `-1` when either argument is NULL; `-8` when `index` is out of bounds.

---

## Physical Layout Graph

`CPhysicalLayoutGraph` is the compiler-local physical topology and calibration view of a device: usable physical qubits, shortest distances, coupling directions, and calibrated error rates.

### physical_layout_graph_from_device(device)

```c
struct CPhysicalLayoutGraph *physical_layout_graph_from_device(const struct CDevice *device);
```

Builds the physical layout graph of a device.

- `device` (`const CDevice*`): target device.

Returns: a new `CPhysicalLayoutGraph*` on success (free with `physical_layout_graph_free`); NULL on NULL input or failure.

### physical_layout_graph_free(ptr)

```c
void physical_layout_graph_free(struct CPhysicalLayoutGraph *ptr);
```

Frees a physical layout graph handle; NULL is allowed.

### physical_layout_graph_num_physical(ptr)

```c
uintptr_t physical_layout_graph_num_physical(const struct CPhysicalLayoutGraph *ptr);
```

Returns the number of usable physical qubits.

Returns: the usable-qubit count; 0 for NULL input.

### physical_layout_graph_physical_qubits(ptr, out, len)

```c
uintptr_t physical_layout_graph_physical_qubits(const struct CPhysicalLayoutGraph *ptr,
                                                uint32_t *out,
                                                uintptr_t len);
```

Two-step read: copies the usable physical qubit IDs in ascending order into `out`. Query the total first (`physical_layout_graph_num_physical`), then allocate and fill.

- `ptr` (`const CPhysicalLayoutGraph*`): graph handle.
- `out` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: the total count; 0 for NULL input.

### physical_layout_graph_distance(ptr, a, b)

```c
uint32_t physical_layout_graph_distance(const struct CPhysicalLayoutGraph *ptr,
                                        uint32_t a,
                                        uint32_t b);
```

Returns the undirected shortest-path distance between two physical qubits.

- `ptr` (`const CPhysicalLayoutGraph*`): graph handle.
- `a`, `b` (`uint32_t`): physical qubit IDs.

Returns: the shortest distance; `UINT32_MAX` when the pair is disconnected, unknown, or the handle is NULL.

### physical_layout_graph_is_adjacent_undirected(ptr, a, b)

```c
int32_t physical_layout_graph_is_adjacent_undirected(const struct CPhysicalLayoutGraph *ptr,
                                                     uint32_t a,
                                                     uint32_t b);
```

Tests whether two physical qubits are adjacent (directly coupled, undirected).

Returns: `1` when adjacent; `0` otherwise; `-1` for NULL.

### physical_layout_graph_readout_error(ptr, qubit, out)

```c
int32_t physical_layout_graph_readout_error(const struct CPhysicalLayoutGraph *ptr,
                                            uint32_t qubit,
                                            double *out);
```

Reads the readout error rate of one physical qubit.

- `ptr` (`const CPhysicalLayoutGraph*`): graph handle.
- `qubit` (`uint32_t`): physical qubit ID.
- `out` (`double*`): error-rate output.

Returns: `0` on success (value written to `*out`); `-1` when either argument is NULL; `-8` when no readout data is recorded for the qubit.

### physical_layout_graph_supports_directed_coupling(ptr, control, target)

```c
int32_t physical_layout_graph_supports_directed_coupling(const struct CPhysicalLayoutGraph *ptr,
                                                         uint32_t control,
                                                         uint32_t target);
```

Tests whether the directed coupling `control -> target` exists.

- `control`, `target` (`uint32_t`): physical qubit IDs.

Returns: `1` when the directed coupling exists; `0` when it does not; `-1` for NULL.

### physical_layout_graph_supports_two_qubit_gate_directed(ptr, gate_name, control, target)

```c
int32_t physical_layout_graph_supports_two_qubit_gate_directed(const struct CPhysicalLayoutGraph *ptr,
                                                               const char *gate_name,
                                                               uint32_t control,
                                                               uint32_t target);
```

Tests whether `gate_name` is a native capability on the directed edge `control -> target`.

- `gate_name` (`const char*`): standard gate name (for example `"CX"`, `"SWAP"`).
- `control`, `target` (`uint32_t`): physical qubit IDs.

Returns: `1` when supported; `0` when not; `-1` for NULL; `-4` for an unknown gate name.

### physical_layout_graph_two_qubit_gate_error_directed(ptr, gate_name, control, target, out)

```c
int32_t physical_layout_graph_two_qubit_gate_error_directed(const struct CPhysicalLayoutGraph *ptr,
                                                            const char *gate_name,
                                                            uint32_t control,
                                                            uint32_t target,
                                                            double *out);
```

Reads the calibrated error rate of `gate_name` on the directed edge `control -> target`.

- `gate_name` (`const char*`): standard gate name.
- `control`, `target` (`uint32_t`): physical qubit IDs.
- `out` (`double*`): error-rate output.

Returns: `0` on success (value written to `*out`); `-1` for NULL; `-4` for an unknown gate name; `-8` when the gate is unsupported or uncalibrated on the edge.

### physical_layout_graph_has_fidelity_data(ptr) / has_readout_error_data / has_two_qubit_error_data

```c
int32_t physical_layout_graph_has_fidelity_data(const struct CPhysicalLayoutGraph *ptr);
int32_t physical_layout_graph_has_readout_error_data(const struct CPhysicalLayoutGraph *ptr);
int32_t physical_layout_graph_has_two_qubit_error_data(const struct CPhysicalLayoutGraph *ptr);
```

Query calibration-data availability: any readout or two-qubit calibration data / readout-error data / two-qubit error data. `LAYOUT_OBJECTIVE_AUTO` uses these flags to pick the fidelity-aware or topology-only objective.

Returns: `1` when the data is available; `0` otherwise; `-1` for NULL.

---

## Prepared SABRE Objects

The prepared pipeline builds the circuit-side and device-side SABRE data once and reuses it: `prepare_sabre_circuit` prepares the circuit analysis, `prepare_sabre_device_target` the exact device-side data.

### prepare_sabre_circuit(circuit)

```c
struct CPreparedSabreCircuit *prepare_sabre_circuit(const struct CCircuit *circuit);
```

Prepares the circuit-side analysis and dependency models used by SABRE.

- `circuit` (`const CCircuit*`): input circuit.

Returns: a new `CPreparedSabreCircuit*` on success (free with `prepared_sabre_circuit_free`); NULL on NULL input or failure.

### prepared_sabre_circuit_free(ptr)

```c
void prepared_sabre_circuit_free(struct CPreparedSabreCircuit *ptr);
```

Frees a prepared SABRE circuit handle; NULL is allowed.

### prepared_sabre_circuit_logical_qubits_len(ptr)

```c
uintptr_t prepared_sabre_circuit_logical_qubits_len(const struct CPreparedSabreCircuit *ptr);
```

Returns the number of logical qubits of the prepared circuit.

Returns: the logical-qubit count; 0 for NULL input.

### prepared_sabre_circuit_logical_qubits(ptr, out, len)

```c
uintptr_t prepared_sabre_circuit_logical_qubits(const struct CPreparedSabreCircuit *ptr,
                                                uint32_t *out,
                                                uintptr_t len);
```

Two-step read: copies the prepared circuit's logical qubit IDs in source-circuit order into `out`. Query the total first, then allocate and fill.

Returns: the total count; 0 for NULL input.

### prepare_sabre_device_target(prepared, device)

```c
struct CPreparedSabreTarget *prepare_sabre_device_target(const struct CPreparedSabreCircuit *prepared,
                                                         const struct CDevice *device);
```

Prepares exact device-native SABRE data for a prepared circuit: native operation feasibility, direction, and implementation costs.

**Prerequisite**: the device must declare routing capability — one of the following must hold: the device-level native gate set is non-empty (for example set via `device_with_native_gates`, see [Device / Properties](../2_device/2_properties_device.md)); at least one usable physical qubit records native instructions in its properties; or at least one usable coupling edge records native instructions in its properties. A device without declared routing capability makes this call return NULL.

- `prepared` (`const CPreparedSabreCircuit*`): prepared circuit handle (result of `prepare_sabre_circuit`).
- `device` (`const CDevice*`): target device.

Returns: a new `CPreparedSabreTarget*` on success (free with `prepared_sabre_target_free`); NULL when either argument is NULL, when the device declares no routing capability, or on failure.

### prepared_sabre_target_free(ptr)

```c
void prepared_sabre_target_free(struct CPreparedSabreTarget *ptr);
```

Frees a prepared SABRE target handle; NULL is allowed.

### prepared_sabre_target_physical(ptr)

```c
struct CPhysicalLayoutGraph *prepared_sabre_target_physical(const struct CPreparedSabreTarget *ptr);
```

Retrieves the physical layout graph used by layout objective scoring (a deep clone).

Returns: a new `CPhysicalLayoutGraph*` on success (free with `physical_layout_graph_free`); NULL on NULL input.

---

## Prepared Layout Algorithms

The prepared entry points accept the analysis from `analyze_circuit_for_layout` and a `CPhysicalLayoutGraph` (or the SABRE prepared objects), skipping the repeated analysis.

### trivial_layout_prepared(analysis, physical, objective)

```c
struct CLayoutResult *trivial_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                              const struct CPhysicalLayoutGraph *physical,
                                              uint8_t objective);
```

Trivial layout over a prepared analysis and physical graph. `LAYOUT_OBJECTIVE_AUTO` resolves against the passed physical graph's calibration data.

- `analysis` (`const CCircuitLayoutAnalysis*`): circuit layout analysis.
- `physical` (`const CPhysicalLayoutGraph*`): physical layout graph.
- `objective` (`uint8_t`): layout objective tag.

Returns: a new `CLayoutResult*` on success; NULL on NULL arguments, an unknown `objective` tag, or execution failure.

### greedy_layout_prepared(analysis, physical, objective)

```c
struct CLayoutResult *greedy_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                             const struct CPhysicalLayoutGraph *physical,
                                             uint8_t objective);
```

Greedy layout over a prepared analysis and physical graph. Parameters and return value as for `trivial_layout_prepared`.

### vf2_perfect_layout_prepared(analysis, physical, objective, config)

```c
struct CLayoutResult *vf2_perfect_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                                  const struct CPhysicalLayoutGraph *physical,
                                                  uint8_t objective,
                                                  const struct CVf2LayoutConfig *config);
```

VF2 perfect layout over a prepared analysis and physical graph; `config` NULL selects the default.

Returns: a new `CLayoutResult*` on success; NULL when no perfect mapping exists, on invalid arguments, or on execution failure.

### sabre_layout_prepared(prepared, prepared_target, objective, config)

```c
struct CLayoutResult *sabre_layout_prepared(const struct CPreparedSabreCircuit *prepared,
                                            const struct CPreparedSabreTarget *prepared_target,
                                            uint8_t objective,
                                            const struct SabreConfigC *config);
```

SABRE layout over a prepared circuit and prepared target; `LAYOUT_OBJECTIVE_AUTO` resolves against the prepared target's physical graph, and `config` NULL selects the default.

- `prepared` (`const CPreparedSabreCircuit*`): prepared circuit.
- `prepared_target` (`const CPreparedSabreTarget*`): prepared target (physical graph plus native costs).
- `objective` (`uint8_t`): layout objective tag.
- `config` (`const SabreConfigC*`): SABRE configuration, see [SABRE Configuration](5_sabre.md).

Returns: a new `CLayoutResult*` on success; NULL on NULL pointer arguments, an unknown `objective` tag, or an invalid configuration.

### score_layout(objective, analysis, physical, layout, out)

```c
int32_t score_layout(uint8_t objective,
                     const struct CCircuitLayoutAnalysis *analysis,
                     const struct CPhysicalLayoutGraph *physical,
                     const struct CLayout *layout,
                     struct CLayoutScore *out);
```

Scores a candidate layout against the layout objective and writes the snapshot to `*out` (fields in the `CLayoutScore` table above). `layout` must map the logical qubits of `analysis` onto the physical qubits of `physical`.

- `objective` (`uint8_t`): layout objective tag.
- `analysis` (`const CCircuitLayoutAnalysis*`): circuit layout analysis.
- `physical` (`const CPhysicalLayoutGraph*`): physical layout graph.
- `layout` (`const CLayout*`): candidate layout, see [Layout](../2_device/3_layout.md).
- `out` (`CLayoutScore*`): score snapshot output.

Returns: `0` on success; `-1` when any argument is NULL; `-8` for an unknown `objective` tag or an unresolvable objective (for example `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` without calibration data); `-6` when the core scoring fails.

---

## Layout Result Accessors

`CLayoutResult` carries the selected mapping, an optional score, and diagnostics.

### layout_result_free(ptr)

```c
void layout_result_free(struct CLayoutResult *ptr);
```

Frees a layout result handle; NULL is allowed.

### layout_result_layout(ptr)

```c
struct CLayout *layout_result_layout(const struct CLayoutResult *ptr);
```

Retrieves the selected logical-to-physical mapping (a deep clone).

Returns: a new `CLayout*` on success (free with `layout_free`); NULL on NULL input.

### layout_result_has_score(ptr)

```c
int32_t layout_result_has_score(const struct CLayoutResult *ptr);
```

Tests whether the result carries an observed score.

Returns: `1` when a score is present; `0` otherwise; `-1` for NULL.

### layout_result_score(ptr, out)

```c
int32_t layout_result_score(const struct CLayoutResult *ptr, struct CLayoutScore *out);
```

Writes the score snapshot to `*out`.

Returns: `0` on success; `-1` when either argument is NULL; `-8` when the result has no score.

### layout_result_is_perfect(ptr)

```c
int32_t layout_result_is_perfect(const struct CLayoutResult *ptr);
```

Tests whether the layout is perfect: every positive-weight interaction lands on an adjacent hardware edge.

Returns: `1` when perfect; `0` otherwise; `-1` for NULL.

### layout_result_candidates_evaluated(ptr)

```c
uintptr_t layout_result_candidates_evaluated(const struct CLayoutResult *ptr);
```

Returns the number of candidate layouts considered by the method.

Returns: the candidate count; 0 for NULL input.

### layout_result_used_fidelity(ptr)

```c
int32_t layout_result_used_fidelity(const struct CLayoutResult *ptr);
```

Tests whether fidelity data contributed to the selected score.

Returns: `1` when it contributed; `0` otherwise; `-1` for NULL.

### layout_result_notes_len(ptr) / layout_result_note(ptr, index)

```c
uintptr_t layout_result_notes_len(const struct CLayoutResult *ptr);
char *layout_result_note(const struct CLayoutResult *ptr, uintptr_t index);
```

Two-step read of diagnostic notes: `notes_len` returns the note count (0 for NULL), and `note` returns the note at `index` as a heap-allocated C string (free with `cqlib_string_free`); NULL on out-of-bounds.

---

## Example

The four layout entry points with result reading:

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Directed line 0 -> 1 -> 2 built from an edge list */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);   /* non-adjacent pair on the line */
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* Trivial layout maps logical i to physical i */
    struct CLayoutResult *trivial =
        trivial_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    if (trivial != NULL) {
        struct CLayout *mapping = layout_result_layout(trivial);
        uintptr_t n = layout_num_logical(mapping);   /* 3 */
        layout_free(mapping);
        layout_result_free(trivial);
    }

    /* Greedy and SABRE layouts run on the same circuit */
    struct CLayoutResult *greedy =
        greedy_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    if (greedy != NULL) {
        layout_result_free(greedy);
    }

    struct CLayoutResult *sabre =
        sabre_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    if (sabre != NULL) {
        uintptr_t evaluated = layout_result_candidates_evaluated(sabre);  /* >= 1 */
        layout_result_free(sabre);
    }

    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

The prepared pipeline: circuit analysis, physical graph, prepared SABRE objects, and scoring (the device declares its native gate set first):

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* Exact device-native SABRE requires declared routing capability */
    if (device_with_native_gates(device, "H,CX,SWAP") != 0) {
        circuit_free(circuit);
        device_free(device);
        return 1;
    }

    /* Circuit-side analysis: one cx(0, 2) collapses to one interaction pair */
    struct CCircuitLayoutAnalysis *analysis = analyze_circuit_for_layout(circuit);
    uintptr_t num_interactions = circuit_layout_analysis_interactions_len(analysis);
    for (uintptr_t i = 0; i < num_interactions; i++) {
        struct CInteraction interaction;
        if (circuit_layout_analysis_interaction(analysis, i, &interaction) == 0) {
            printf("interaction %u-%u weight %f\n",
                   interaction.left, interaction.right, interaction.weight);
        }
    }

    /* Physical graph view of the device */
    struct CPhysicalLayoutGraph *graph = physical_layout_graph_from_device(device);
    uintptr_t num_physical = physical_layout_graph_num_physical(graph);  /* 3 */
    uint32_t *physical_ids = malloc(num_physical * sizeof(uint32_t));
    physical_layout_graph_physical_qubits(graph, physical_ids, num_physical);
    uint32_t dist = physical_layout_graph_distance(graph, 0, 2);  /* 2 */
    int32_t adjacent =
        physical_layout_graph_is_adjacent_undirected(graph, 0, 2); /* 0 */
    free(physical_ids);

    /* Prepared SABRE pipeline */
    struct CPreparedSabreCircuit *prepared = prepare_sabre_circuit(circuit);
    struct CPreparedSabreTarget *target = prepare_sabre_device_target(prepared, device);
    if (prepared != NULL && target != NULL) {
        struct CPhysicalLayoutGraph *target_graph = prepared_sabre_target_physical(target);
        physical_layout_graph_free(target_graph);

        struct CLayoutResult *result = sabre_layout_prepared(
            prepared, target, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
        if (result != NULL) {
            struct CLayout *mapping = layout_result_layout(result);

            /* Score the selected mapping against the prepared analysis */
            struct CLayoutScore score;
            if (score_layout(LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
                             analysis, graph, mapping, &score) == 0) {
                printf("total=%f distance=%f\n", score.total, score.distance);
            }
            layout_free(mapping);
            layout_result_free(result);
        }
        prepared_sabre_target_free(target);
        prepared_sabre_circuit_free(prepared);
    }

    physical_layout_graph_free(graph);
    circuit_layout_analysis_free(analysis);
    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

Handing the selected layout to the routing stage for SWAP insertion is covered in [Routing](4_routing.md); the device-target compile entry point that performs layout and routing in one call is documented in [Compiler](1_compiler.md).
