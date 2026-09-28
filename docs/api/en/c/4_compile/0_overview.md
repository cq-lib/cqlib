# Compile (C)

The compile module provides optimization workflows through free functions plus an opaque handle (`CCompileResult`): run logical compilation driven by a `CompileConfigC`, or run routing and native-gate lowering against a `CDevice` target, recording the optimized circuit, change flags, and per-step reports in the compile result. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Concepts

- `compile` takes `CompileConfigC` by value: `mode` selects the Normal or Enhanced workflow, and `target` selects the compilation target; non-`Logical` target values are handled as logical compilation.
- `compile_with_device` is the device-target entry point: the circuit is routed on the device topology and lowered to the device's ordered native capabilities, so the resulting circuit is always accepted by the device; an optional initial layout and seed control the mapping process.
- `CCompileResult` carries the optimized circuit (retrieved as a deep clone), the overall change flag, the compile mode, and per-step reports; device-target compilations additionally record the initial and final layouts (`CLayout`, see [Layout](../2_device/3_layout.md)).

---

## Page Navigation

| Page | Content |
| --- | --- |
| [Compiler](1_compiler.md) | `compile` / `compile_with_device` and `CCompileResult`: construction, result reading, step reports, and layout access. |
| [Transform](2_transform.md) | Standalone transform passes: structural analysis with `circuit_analyze`, canonicalization with `canonicalize_circuit`, knowledge rewriting with `rewrite_circuit`, and the `transform_*` wrappers. |
| [Initial Layout](3_layout.md) | The layout entry points `trivial_layout` / `greedy_layout` / `vf2_perfect_layout` / `sabre_layout`, circuit interaction analysis, the physical layout graph, prepared SABRE objects, and layout result reading. |
| [Routing](4_routing.md) | The routing entry points `route_with_layout` / `route_sabre`, the `CRoutedCircuit` routing result, and layout reachability validation. |
| [SABRE Configuration](5_sabre.md) | The `SabreConfigC` configuration family and `sabre_route` routing from a known initial layout. |
| [Decompose / Resynthesis](6_decompose_resynthesis.md) | Definition expansion, unitary synthesis, multi-controlled-gate decomposition, and two-qubit-block resynthesis. |
| [Knowledge](7_knowledge.md) | Construction, validation, and structural matching of knowledge rules. |
| [Resource](8_resource.md) | Ancilla resource policies, limits, and lease management. |
| [Commutation](9_commutation.md) | Conservative commutation checks between operations. |

---

## CompileConfigC

The input configuration of `compile`, passed by value:

```c
typedef struct CompileConfigC {
  uint8_t mode;                 /* One of COMPILE_MODE_*. */
  uint8_t target;               /* One of COMPILE_TARGET_*. */
  uint8_t allow_dirty_ancilla;  /* Reserved for future use. */
  uint8_t allow_clean_ancilla;  /* Reserved for future use. */
  uint8_t _reserved[4];         /* Reserved padding. */
} CompileConfigC;
```

| Field | Type | Meaning |
| --- | --- | --- |
| `mode` | `uint8_t` | Compile mode, one of `COMPILE_MODE_*`. |
| `target` | `uint8_t` | Compile target, one of `COMPILE_TARGET_*`; only `mode` is consulted for `Logical` targets. |
| `allow_dirty_ancilla` | `uint8_t` | Reserved field (resource policy flags); not used by the workflow today. |
| `allow_clean_ancilla` | `uint8_t` | Reserved field (resource policy flags); not used by the workflow today. |
| `_reserved[4]` | `uint8_t[4]` | Padding that keeps the struct layout stable; do not write non-zero values. |

For `Logical` targets only `mode` is consulted; other target values are handled as logical compilation. All fields are kept in the struct so it can be extended without breaking the ABI.

---

## Constants

Compile mode tags (the `CompileConfigC.mode` field and the `mode` parameter of `compile_with_device`):

| Constant | Value | Mode |
| --- | --- | --- |
| `COMPILE_MODE_NORMAL` | 0 | Normal compilation mode. |
| `COMPILE_MODE_ENHANCED` | 1 | Enhanced compilation mode. |

Compile target tags (the `CompileConfigC.target` field):

| Constant | Value | Target |
| --- | --- | --- |
| `COMPILE_TARGET_LOGICAL` | 0 | Logical-qubit target. |
| `COMPILE_TARGET_BASIS` | 1 | Basis-gate-set target. |
| `COMPILE_TARGET_DEVICE` | 2 | Device target. |
| `COMPILE_TARGET_TOPOLOGY_BASIS` | 3 | Topology basis-gate-set target. |

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* Enhanced-mode logical compilation */
    struct CompileConfigC config = {0};
    config.mode = COMPILE_MODE_ENHANCED;
    config.target = COMPILE_TARGET_LOGICAL;

    struct CCompileResult *result = compile(qc, config);
    if (result == NULL) {
        circuit_free(qc);
        return 1;
    }

    int32_t changed = compile_result_changed(result);   /* 1: the workflow changed the circuit */
    struct CCircuit *optimized = compile_result_circuit(result);
    if (optimized != NULL) {
        circuit_free(optimized);  /* deep clone; independent of result */
    }

    compile_result_free(result);
    circuit_free(qc);
    return 0;
}
```

See [Compiler](1_compiler.md) for the full device-target flow, including routing and layout access.
