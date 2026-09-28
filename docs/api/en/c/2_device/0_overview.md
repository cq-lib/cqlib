# Device (C)

The C binding's device interface describes quantum hardware through opaque handles (`CTopology`, `CDevice`, `CLayout`, `CNoiseModel`, `CExecutionResult`) plus free functions: the coupling topology, calibration properties, the logical-to-physical mapping, the noise model, and task execution results. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

---

## Concept relationships

- A `CDevice` aggregates a `CTopology` and a set of property defaults: the topology describes the directed graph of physical qubits and couplings; the device layers a name, a native gate set, invalid qubits, a calibration time, and device-wide default properties on top of it. Qubit-property queries prefer local values and fall back to the defaults.
- A `CLayout` is the bidirectional mapping between logical and physical qubits used during routing; `layout_bind` / `layout_unbind` / `layout_swap_physical` are the basic operations for moving logical qubits.
- A `CNoiseModel` is built independently of any device and records noise channels and readout errors per gate + qubit combination.
- A `CExecutionResult` carries the outcome of one task: the state machine (queued → running → completed / failed / cancelled), measurement counts, and the probability distribution.

---

## Pages

| Page | Contents |
| --- | --- |
| [Topology](1_topology.md) | `CTopology`: construction, qubit and coupling editing, connectivity and degree queries. |
| [Device / Properties](2_properties_device.md) | `CDevice`: construction factories, the native gate set, validation, error properties, defaults, and the property structs. |
| [Layout](3_layout.md) | `CLayout`: building, binding, and swapping the logical↔physical mapping. |
| [NoiseModel](4_noise.md) | `CNoiseModel`: registering and querying single-qubit, two-qubit, and readout noise. |
| [ExecutionResult](5_result.md) | `CExecutionResult` and `CCountsList`: the task state machine, counts, and probabilities. |
| [Qubit identifiers](6_qubits.md) | `CLogicalQubit` / `CPhysicalQubit`: strongly typed device-side qubit identifiers and their conversions to/from `CQubit`. |

---

## Shared conventions

- Every handle is released with its matching `*_free` function, all of which accept NULL: `topology_free`, `device_free`, `layout_free`, `noise_model_free`, `execution_result_free`, `counts_list_free`.
- `device_topology` returns a clone of the topology; release it with `topology_free`.
- Functions returning `char*` (e.g. `device_name`, `counts_list_get_key`) are released with `cqlib_string_free`.
- Array outputs use the two-step pattern: call the `*_len` function for the total, allocate a buffer, then call the fill function (e.g. `device_usable_qubits_len` + `device_usable_qubits`); the fill function also returns the total count.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* line-topology device: 0 -> 1 -> 2 */
    struct CDevice *dev = device_line("demo", 3);
    if (dev == NULL) {
        return 1;
    }

    char *name = device_name(dev);              /* "demo" */
    uintptr_t usable = device_num_qubits(dev);  /* 3 */
    cqlib_string_free(name);

    /* cloned topology */
    struct CTopology *topo = device_topology(dev);
    int32_t directed = topology_supports_directed_coupling(topo, 0, 1);  /* 1 */
    int32_t reverse = topology_supports_directed_coupling(topo, 1, 0);  /* 0 */
    topology_free(topo);

    /* device-wide defaults and fallback query */
    device_set_default_t1(dev, 40.0);
    double t1 = 0.0;
    device_get_t1(dev, 0, &t1);  /* 0 (success), t1 == 40.0 */

    device_free(dev);
    return 0;
}
```

The compiler interface takes a `CDevice` and a `CLayout` as the device-target inputs; see [Compiler](../4_compile/1_compiler.md).
