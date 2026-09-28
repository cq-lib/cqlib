# Routing

`cqlib_core::compile::transform::routing` provides transform-level routing entry points. Routing inserts SWAPs on a constrained topology so that all two-body interactions in the circuit satisfy the device adjacency constraint.

## Import

```rust
use cqlib_core::compile::transform::routing::{RoutedCircuit, SabreRouteResult, route_sabre, route_with_layout};
use cqlib_core::compile::transform::layout::LayoutObjective;
use cqlib_core::compile::sabre::SabreConfig;
```

---

## Functions

### `route_with_layout(circuit: &Circuit, device: &Device, initial_layout: &Layout, config: &SabreConfig) -> Result<RoutedCircuit, CompilerError>`

Deterministic routing given a known initial layout.

### `route_sabre(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &SabreConfig) -> Result<SabreRouteResult, CompilerError>`

SABRE routing with layout selection: first select the initial layout according to `objective`, then perform routing.

---

## RoutedCircuit

The routing result given a known initial layout; the fields are private:

- `circuit(&self) -> &Circuit`: the routed circuit.
- `into_circuit(self) -> Circuit`: consume and take out the circuit.
- `initial_layout(&self) -> &Layout`: the layout before routing starts.
- `final_layout(&self) -> &Layout`: the layout after routing ends.
- `swap_count(&self) -> usize`: the number of inserted SWAPs.
- `diagnostics(&self) -> &SabreRoutingDiagnostics`: routing diagnostic information; see [SABRE](5_sabre.md).

---

## SabreRouteResult

The routing result with layout selection; the fields are private:

- `routed(&self) -> &RoutedCircuit`: the routing result.
- `into_routed(self) -> RoutedCircuit`: consume and take out the routing result.
- `layout_score(&self) -> Option<&LayoutScore>`: the score of the selected layout. SABRE selects the winner by predicted native routing quality; this score is for diagnostics only and is not the routing selection key.
- `layout_diagnostics(&self) -> &LayoutDiagnostics`: diagnostic information of the layout process.

---

## Example

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::sabre::SabreConfig;
use cqlib_core::compile::transform::layout::LayoutObjective;
use cqlib_core::compile::transform::routing::route_sabre;
use cqlib_core::device::Device;

let mut circuit = Circuit::new(3);
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

let device = Device::line("line-3", 3).unwrap();
let result = route_sabre(
    &circuit,
    &device,
    &LayoutObjective::topology_only(),
    &SabreConfig::default(),
)
.unwrap();

println!("swap_count: {}", result.routed().swap_count());
```

---

## Errors

- `CompilerError::InvalidInput`: invalid SABRE configuration, for example `routing_trials` is zero.
- Other `CompilerError` variants: routing failure, for example a missing or unreachable initial layout, or insufficient device capacity.

For details of the lower-level `sabre_route` and `SabreConfig`, see [SABRE](5_sabre.md).
