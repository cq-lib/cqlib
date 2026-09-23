# Initial Layout (Layout)

Initial layout is responsible only for choosing the initial mapping from logical qubits to physical qubits, and does not insert any SWAP gates.

---

## Python entry points

```python
from cqlib import Circuit
from cqlib.compile.transform.layout import (
    LayoutObjective,
    Vf2LayoutConfig,
    vf2_perfect_layout,
    greedy_layout,
    sabre_layout,
    trivial_layout,
)
from cqlib.device import Device
```

---

## Quick examples: VF2 perfect embedding

```python
circuit = Circuit(3)
circuit.cx(0, 1)
circuit.cx(1, 2)

device = Device.line("line-5", 5)
objective = LayoutObjective.topology_only()

result = vf2_perfect_layout(circuit, device, objective)
print(result.layout)                    # logical -> physical mapping
print(result.diagnostics.is_perfect)    # whether all interactions are adjacent
```

If there is no perfect embedding, `vf2_perfect_layout` raises a `ValueError`; in that case switch to `greedy_layout` or `sabre_layout`, and then hand over to `route_sabre` or `route_with_layout`.

---

## Trivial layout (identity mapping)

Logical qubit `i → physical i`, used for baseline comparison or for an already aligned topology:

```python
result = trivial_layout(circuit, device, objective)
print(result.layout.l2p_map)
```

---

## Greedy layout

```python
objective = LayoutObjective.fidelity_aware()
result = greedy_layout(circuit, device, objective)
print("is_perfect:", result.diagnostics.is_perfect)
print("score:", result.score.total if result.score else None)
```

Characteristics: deterministic and fast, suitable as a seed for SABRE layout; on long chains or with large fan-out, `is_perfect=False` is possible.

---

## SABRE initial layout

```python
from cqlib.compile.sabre import SabreConfig

config = SabreConfig.deterministic_seeded(42)
result = sabre_layout(circuit, device, objective, config)
```

`sabre_layout` generates multiple candidate groups and refines them through forward/backward trial runs, and still does not insert SWAP gates into the output.

---

## LayoutObjective (layout scoring)

| Constructor | Behavior |
|----------|------|
| `LayoutObjective.topology_only()` | Topological distance and direction mismatch only |
| `LayoutObjective.fidelity_aware()` | Default fidelity weights |
| `LayoutObjective.fidelity_required(device)` | Requires the device to have usable calibration |
| `LayoutObjective.auto_from_device(device)` | fidelity if calibration is available, topology otherwise |

In enhanced mode, `compile(..., device=...)` uses the `fidelity_required` logic when the device has calibration.

---

## VF2 configuration

```python
from cqlib.compile.transform.layout import Vf2EdgeRequirement

config = Vf2LayoutConfig(
    candidate_limit=10,
    edge_requirement=Vf2EdgeRequirement.positive_interactions(),
)
result = vf2_perfect_layout(circuit, device, objective, config)
```

---

## Passing layout results to routing

```python
from cqlib.compile.transform.routing import route_with_layout
from cqlib.compile.sabre import SabreConfig

layout_result = vf2_perfect_layout(circuit, device, objective)
routed = route_with_layout(
    circuit,
    device,
    layout_result.layout,
    SabreConfig.deterministic_seeded(42),
)
print("swaps:", routed.swap_count)
```

---

## Notes

- The input is a **logical** `Circuit` plus a `Device`; the output `LayoutResult.layout` is a `cqlib.device.Layout`.
- The layout stage **does not change gate semantics** and **does not insert SWAP gates**.
- Device calibration participates in fidelity scoring through the readout / two-qubit error fields on `Device`.

---

## Next steps

- [SABRE routing mapping](2_sabre_mapping.md): pass the layout result to `route_sabre` or `route_with_layout` to complete physical routing.
- [Compilation and optimization](0_overview.md): review the `compile()` workflow and how the compilation stages relate to each other.
