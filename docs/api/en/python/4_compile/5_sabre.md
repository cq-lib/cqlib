# SABRE

`cqlib.compile.sabre` provides the low-level entry point and configuration of SWAP routing. Use `sabre_route` when the initial layout is known and the circuit must satisfy the device topology constraint. Fixing the random seed makes the tie-breaking and selection of multiple trials reproducible.

## Import

```python
from cqlib.compile.sabre import (
    SabreConfig,
    SabreHeuristicConfig,
    SabreVf2PrepassConfig,
    SabreRoutingResult,
    SabreRoutingDiagnostics,
    sabre_route,
    normalize_initial_layout,
    validate_reachable_interactions,
)
```

---

## sabre_route(circuit, device, initial_layout, config=None)

SABRE routing under a known initial layout.

Parameters:

- `circuit` (`Circuit`): the circuit to route.
- `device` (`Device`): the target device.
- `initial_layout` (`Layout`): the logical-to-physical initial layout.
- `config` (`SabreConfig | None`): defaults to `SabreConfig()`.

Returns:

- `SabreRoutingResult`

Example:

```python
from cqlib import Circuit
from cqlib.compile.sabre import SabreConfig, sabre_route
from cqlib.device import Device, Layout

circuit = Circuit(2)
circuit.cx(0, 1)

result = sabre_route(
    circuit,
    Device.line("line-3", 3),
    Layout.from_pairs([(0, 0), (1, 2)], physical_count=3),
    SabreConfig(routing_trials=1, seed=7),
)
assert result.swap_count == 1
```

---

## Helper functions

### normalize_initial_layout(logical_qubits, device, initial_layout)

Normalize a complete logical-to-physical layout against the qubits available on the device.

Parameters:

- `logical_qubits` (`list[Qubit]`): the logical qubit list of the circuit.
- `device` (`Device`): the target device.
- `initial_layout` (`Layout`): the initial layout to normalize.

Returns:

- `Layout`

### validate_reachable_interactions(circuit, device, initial_layout)

Validate native movement reachability without performing routing.

Parameters:

- `circuit` (`Circuit`): the circuit to check.
- `device` (`Device`): the target device.
- `initial_layout` (`Layout`): the initial layout.

Returns:

- `None`; raises `CompilerError` when something is unreachable.

---

## SabreConfig

Configuration shared by SABRE layout refinement and routing.

### SabreConfig(*, layout_trials=10, layout_assignment_budget=1_000_000, vf2_prepass=SabreVf2PrepassConfig(candidate_limit=10, call_limit=1_000_000), refinement_iterations=1, routing_trials=1, seed=None, heuristic=None)

Parameters:

- `layout_trials` (`int`): the number of randomized initial layouts added beyond the deterministic and interaction-aware candidates.
- `layout_assignment_budget` (`int`): the maximum exploration of layout component assignment states; exhaustion reports budget exhaustion, which is distinct from proven infeasibility.
- `vf2_prepass` (`SabreVf2PrepassConfig | None`): the bounded VF2 prepass, used to add topology-perfect candidates; `None` disables the prepass.
- `refinement_iterations` (`int`): the number of forward/backward refinement rounds per layout trial.
- `routing_trials` (`int`): the number of complete routing trials performed per fully refined layout. The search returns the best route directly and does not perform routing for intermediate refinement states.
- `seed` (`int | None`): the deterministic random seed. The same seed produces the same cqlib result.
- `heuristic` (`SabreHeuristicConfig | None`): the swap selection heuristic configuration; defaults to `SabreHeuristicConfig()`.

### Static methods

- `SabreConfig.deterministic_seeded(seed)`: a compact deterministic configuration, suitable for tests and examples. All trial counts are small, the random seed is fixed, and swap attempts are bounded.

### Attributes

`layout_trials`, `layout_assignment_budget`, `vf2_prepass`, `refinement_iterations`, `routing_trials`, `seed`, `heuristic`, corresponding one-to-one with the constructor parameters.

### Raises

- `CompilerConfigError`: routing fields are invalid, for example `routing_trials` is zero (layout refinement fields do not affect routing validation).

---

## SabreHeuristicConfig

SABRE swap selection heuristic configuration. The main score combines the current front layer sum, the lookahead scaled by the active layer, and a multiplicative congestion term; when a native plan exists, candidates are decided within a narrow structural window using the exact native two-qubit cost.

### SabreHeuristicConfig(*, basic_weight=1.0, lookahead_weights=None, decay_increment=0.002, decay_reset=10, attempt_limit=1000, best_epsilon=1e-10)

Parameters:

- `basic_weight` (`float`): the weight of the total distance of the current front layer.
- `lookahead_weights` (`list[float] | None`): the weight that scales the lookahead layer sum for each active layer; defaults to `[0.5, 0.25, 0.125, 0.0625, 0.03125]`.
- `decay_increment` (`float | None`): the increment of the congestion multiplier of a physical qubit after the heuristic uses it for a SWAP; `None` disables congestion control.
- `decay_reset` (`int`): the number of heuristic SWAP attempts before the decay value is reset.
- `attempt_limit` (`int`): the number of consecutive heuristic SWAPs allowed while no frontier node has been routed, beyond which the search falls back to a shortest-path escape.
- `best_epsilon` (`float`): the floating-point tolerance within which candidate SWAP scores are considered tied.

### Attributes

`basic_weight`, `lookahead_weights`, `decay_increment`, `decay_reset`, `attempt_limit`, `best_epsilon`, corresponding one-to-one with the constructor parameters.

---

## SabreVf2PrepassConfig

The bounded VF2 prepass used to seed SABRE layout candidates.

### SabreVf2PrepassConfig(*, candidate_limit=10, call_limit=1_000_000)

Parameters:

- `candidate_limit` (`int`): the upper limit on the number of complete perfect mappings scored by VF2.
- `call_limit` (`int`): the upper limit on the number of partial mapping extensions attempted by VF2.

### Attributes

`candidate_limit`, `call_limit`, corresponding one-to-one with the constructor parameters.

---

## SabreRoutingResult

The object returned by `sabre_route`.

### Attributes

- `circuit -> Circuit`: the routed circuit.
- `initial_layout -> Layout`: the layout before routing starts.
- `final_layout -> Layout`: the layout after routing ends.
- `swap_count -> int`: the number of inserted SWAPs.
- `diagnostics -> SabreRoutingDiagnostics`: routing diagnostic information.

---

## SabreRoutingDiagnostics

Diagnostic information of the routing process.

### Attributes

- `trials_evaluated -> int`: the number of routing trials evaluated.
- `selected_trial_index -> int`: the index of the selected trial.
- `fallback_count -> int`: the number of fallbacks.
- `control_flow_blocks_routed -> int`: the number of control flow blocks routed.
- `two_qubit_depth -> int`: the two-qubit depth of the routed circuit.
- `operation_count -> int`: the total number of operations.
- `native_two_qubit_count -> int`: the number of native two-qubit gates.
- `native_two_qubit_depth -> int`: the native two-qubit depth.
- `native_total_depth -> int`: the native total depth.
- `native_operation_count -> int`: the total number of native operations.
- `unknown_loop_count -> int`: the number of unknown loops.
- `requirement_signature_count -> int`: the number of requirement signatures.
- `eager_pair_state_count -> int`: the number of eager pair states.
- `lazy_pair_l1_lookup_count -> int`: the number of lazy pair level-1 lookups.
- `lazy_pair_l1_hit_count -> int`: the number of lazy pair level-1 hits.
- `lazy_pair_l1_cached_count -> int`: the number of lazy pair level-1 cached entries.
