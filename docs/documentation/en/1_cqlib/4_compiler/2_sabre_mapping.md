# SABRE Routing Mapping

SABRE routes logical two-qubit gates onto the physical couplings allowed by the device by means of heuristic SWAPs.

---

## Two entry points

| API | Purpose |
|-----|------|
| `route_sabre(circuit, device, objective, config)` | Automatic `sabre_layout` + routing |
| `route_with_layout(circuit, device, initial_layout, config)` | Routing only, skipping the layout search |

```python
from cqlib import Circuit
from cqlib.circuit import Instruction, StandardGate
from cqlib.compile.transform.layout import LayoutObjective
from cqlib.compile.transform.routing import route_sabre, route_with_layout
from cqlib.compile.sabre import SabreConfig
from cqlib.device import Device, Layout

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
device.native_gates = [
    Instruction.from_standard_gate(StandardGate.H),
    Instruction.from_standard_gate(StandardGate.CX),
]
objective = LayoutObjective.topology_only()
config = SabreConfig.deterministic_seeded(42)

result = route_sabre(circuit, device, objective, config)
print("swap_count:", result.swap_count)
print("ops:", len(result.circuit.operations))
```

---

## Integration with the compile workflow

The `device` below must already be configured with executable native instruction capabilities; using `device` alone as the target selects strict device compilation semantics.

```python
from cqlib.compile import CompileMode, compile

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    seed=42,
)

for step in result.steps:
    if step.name in {
        "route.sabre",
        "validate.device",
        "select.sabre_pareto_beam",
    }:
        print(step.stage, step.name, step.changed, step.reason)
```

When `initial_layout` is already provided, the workflow skips the automatic layout and still uses the same SABRE router.

### Pareto beam for enhanced strict devices

For `CompileMode.enhanced()` and a strict `Device` target, the complete compilation result of the first `route.sabre` is candidate 0. Only after candidate 0 has completed post-routing cleanup, native instruction lowering, native fixed-point optimization and `validate.device` does the compiler start the bounded Pareto beam. Each explored candidate starts from the same pre-routing prefix and re-executes the same suffix up to device validation.

`select.sabre_pareto_beam` accepts only validated candidates that do not degrade the exact native quality in any control-flow scope and that strictly reduce the native two-qubit gate count or depth; when no such candidate exists, candidate 0 is kept. This selection is the last orchestration and diagnostic step and does not transform the circuit further after validation.

`result.steps` records the finally kept path, not every route that was attempted. The number of candidates, the discard situation, whether a winner was accepted, and the quality summaries of candidate 0 and the winner can be read from `select.sabre_pareto_beam.reason`. A `TopologyBasis` target does not run this beam even when SABRE routing is performed; for example, passing both `device` and `target_basis` to `compile()` selects `TopologyBasis` semantics.

---

## Example: routing only (skipping the layout search)

```python
from cqlib.device import Layout

initial = Layout.from_pairs([(0, 0), (1, 2), (2, 1)], physical_count=3)
routed = route_with_layout(circuit, device, initial, config)
print("swaps:", routed.swap_count)
```

---

## Example: passing initial_layout to compile

```python
from cqlib.compile import compile

result = compile(
    circuit,
    device=device,
    initial_layout=initial,
    seed=42,
)
```

---

## SabreConfig

| Field | Meaning |
|------|------|
| `layout_trials` | Number of random initial layouts; interaction-aware, greedy and VF2 candidates are added in addition |
| `layout_assignment_budget` | Search limit for movement component assignment on disconnected devices |
| `vf2_prepass` | Bounded VF2 pre-check used when an exact embedding is promising |
| `refinement_iterations` | Number of forward + backward refinement rounds per candidate |
| `routing_trials` | Total number of full routes allocated across lightweight refinement checkpoints per initial candidate |
| `seed` | Deterministic seed |
| `heuristic` | `SabreHeuristicConfig`: a lookahead that is single-layer by default, scaled by device width and stopped early for highly repetitive circuits; multiplicative congestion suppresses consecutive reuse of a physical qubit; exactly tied candidates are chosen by trial seed |

```python
from cqlib.compile.sabre import SabreConfig

config = SabreConfig(
    seed=42,
    layout_trials=24,
    refinement_iterations=2,
    routing_trials=2,
)
```

Shorthand constructor: `SabreConfig.deterministic_seeded(42)`.

---

## Example: a reproducible record with a fixed seed

```python
compile_record = {
    "seed": 123,
    "layout_trials": 24,
    "refinement_iterations": 2,
    "routing_trials": 2,
}
```

Routing involves randomness, so a formal experiment must fix and record the `seed`.

Automatic layout + routing uses a fused search: each candidate keeps the lightweight checkpoints produced by its initial, forward and backward refinements, and `routing_trials` is allocated to checkpoints that are cheaper and map differently; full routes take part directly in a global streaming reduction, so routing is not repeated after layout scoring. The final ordering is fixed by predicted native 2Q count, native 2Q depth, native total depth and stable candidate index.

This describes the in-routing selection of a single `route_sabre()` and of workflow candidate 0. The Pareto beam that enhanced strict device compilation runs afterwards continues to lower, optimize and validate the candidates, and then decides whether to replace candidate 0 based on the final exact native quality.

---

## Output semantics

- `result.circuit`: the circuit on physical qubit numbering, including the inserted `SWAP` gates;
- `swap_count`: should match the number of SWAP gates in the circuit;
- `layout_score`: the diagnostic score of the winning layout under the requested objective, not the basis on which SABRE makes its final choice;
- `diagnostics.native_two_qubit_count`, `native_two_qubit_depth`, `native_total_depth`: structural native quality estimates of the winning route;
- Routing guarantees undirected physical adjacency.

---

## Next steps

- [Template matching and knowledge rule optimization](3_template_optimization.md): learn how knowledge rules clean up and rewrite the circuit further after routing.
- [Initial layout (Layout)](1_layout.md): review the initial layout algorithms and how `LayoutObjective` scores them.
