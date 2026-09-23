# Routing

`cqlib.compile.transform.routing` provides the transform-layer routing entry points. Routing inserts SWAPs on a restricted topology so that all two-body interactions in the circuit satisfy the device adjacency constraint.

## Import

```python
from cqlib.compile.transform.routing import (
    route_with_layout,
    route_sabre,
    RoutedCircuit,
    SabreRouteResult,
)
```

---

## Functions

### route_with_layout(circuit, device, initial_layout, config=None)

Deterministic routing under a known initial layout.

Parameters:

- `circuit` (`Circuit`): the circuit to route.
- `device` (`Device`): the target device.
- `initial_layout` (`Layout`): the logical-to-physical initial layout.
- `config` (`SabreConfig | None`): the SABRE configuration; defaults to `SabreConfig()`.

Returns:

- `RoutedCircuit`

### route_sabre(circuit, device, objective=None, config=None)

SABRE routing with layout selection: first select the initial layout according to `objective`, then perform routing.

Parameters:

- `circuit` (`Circuit`): the circuit to route.
- `device` (`Device`): the target device.
- `objective` (`LayoutObjective | None`): the layout objective; defaults to `LayoutObjective.topology_only()`.
- `config` (`SabreConfig | None`): the SABRE configuration; defaults to `SabreConfig()`.

Returns:

- `SabreRouteResult`

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform.routing import route_sabre
from cqlib.device import Device

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
result = route_sabre(circuit, device)
print("swap_count:", result.swap_count)
```

---

## RoutedCircuit

The routing result under a known initial layout.

### Attributes

- `circuit -> Circuit`: the routed circuit.
- `initial_layout -> Layout`: the layout before routing starts.
- `final_layout -> Layout`: the layout after routing ends.
- `swap_count -> int`: the number of inserted SWAPs.
- `diagnostics -> SabreRoutingDiagnostics`: routing diagnostic information, see [SABRE](5_sabre.md).
- `changed -> bool`: whether routing changed the circuit.

---

## SabreRouteResult

The routing result with layout selection; it carries layout scoring on top of `RoutedCircuit`.

### Attributes

- `routed -> RoutedCircuit`: the routing result.
- `layout_score -> LayoutScore | None`: the score of the selected layout.
- `layout_diagnostics -> LayoutDiagnostics`: diagnostic information of the layout process.
- `circuit`, `initial_layout`, `final_layout`, `swap_count`, `diagnostics`, `changed`: the same as `RoutedCircuit`.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform.routing import route_sabre
from cqlib.device import Device

circuit = Circuit(2)
circuit.cx(0, 1)

device = Device.line("line-3", 3)
result = route_sabre(circuit, device)

print("selected layout:", result.routed.initial_layout)
print("score:", result.layout_score.total if result.layout_score else None)
```

---

## Raises

- `CompilerConfigError`: the SABRE configuration is invalid, for example `routing_trials` is zero.
- `CompilerTransformError`: routing fails, for example the initial layout is missing or unreachable, or the device capacity is insufficient.

For details of the low-level `sabre_route` and `SabreConfig`, see [SABRE](5_sabre.md).
