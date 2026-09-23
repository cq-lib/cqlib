# Layout

`cqlib.compile.transform.layout` provides initial layout algorithms and layout analysis tools. The layout algorithms map logical qubits to physical qubits and provide a starting point for the subsequent routing stage.

## Import

```python
from cqlib.compile.transform.layout import (
    trivial_layout,
    greedy_layout,
    vf2_perfect_layout,
    sabre_layout,
    analyze_circuit_for_layout,
    prepare_sabre_circuit,
    prepare_sabre_device_target,
    trivial_layout_prepared,
    greedy_layout_prepared,
    vf2_perfect_layout_prepared,
    sabre_layout_prepared,
    LayoutObjective,
    LayoutScore,
    LayoutDiagnostics,
    LayoutResult,
    Vf2EdgeRequirement,
    Vf2LayoutConfig,
    Interaction,
    InteractionGraph,
    CircuitLayoutAnalysis,
    DistanceTable,
    PhysicalLayoutGraph,
    PreparedSabreCircuit,
    PreparedSabreTarget,
)
```

---

## Layout functions

### trivial_layout(circuit, device, objective=None)

Trivial layout: map logical qubits to physical qubits in index order.

### greedy_layout(circuit, device, objective=None)

Greedy layout: place logical qubits on physical qubits in order of interaction strength.

### vf2_perfect_layout(circuit, device, objective=None, config=None)

VF2 exact layout: produce a perfect layout when the circuit interaction graph can be embedded exactly into the device topology.

Parameters:

- `config` (`Vf2LayoutConfig | None`): the VF2 search configuration.

### sabre_layout(circuit, device, objective=None, config=None)

SABRE layout: refine a candidate layout using the SABRE heuristic.

Parameters:

- `config` (`SabreConfig | None`): the SABRE configuration, see [SABRE](5_sabre.md).

Common parameters of the functions above:

- `circuit` (`Circuit`): the circuit to lay out.
- `device` (`Device`): the target device.
- `objective` (`LayoutObjective | None`): the layout objective weights; defaults to `LayoutObjective.topology_only()`.

Returns:

- `LayoutResult`

### Precomputed entry points

For scenarios that lay out repeatedly, the circuit analysis or device preparation can be done once and the intermediate results reused:

- `analyze_circuit_for_layout(circuit) -> CircuitLayoutAnalysis`: circuit interaction graph analysis.
- `prepare_sabre_circuit(circuit) -> PreparedSabreCircuit`: SABRE circuit preprocessing.
- `prepare_sabre_device_target(device) -> PreparedSabreTarget`: device physical graph preprocessing.
- `trivial_layout_prepared(analysis, physical, objective=None) -> LayoutResult`
- `greedy_layout_prepared(analysis, physical, objective=None) -> LayoutResult`
- `vf2_perfect_layout_prepared(analysis, physical, objective=None, config=None) -> LayoutResult`
- `sabre_layout_prepared(prepared, prepared_target, objective=None, config=None) -> LayoutResult`

Here `analysis` is a `CircuitLayoutAnalysis`, `physical` is a `PhysicalLayoutGraph`, `prepared` is a `PreparedSabreCircuit`, and `prepared_target` is a `PreparedSabreTarget`.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform.layout import sabre_layout
from cqlib.device import Device

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
result = sabre_layout(circuit, device)
print(result.layout)
```

---

## LayoutObjective

Layout objective weights, used to score candidate layouts.

### LayoutObjective(*, distance_weight=1.0, direction_weight=1.0, two_qubit_error_weight=0.0, readout_error_weight=0.0)

Parameters:

- `distance_weight` (`float`): the weight of the shortest distance term.
- `direction_weight` (`float`): the weight of the direction term.
- `two_qubit_error_weight` (`float`): the weight of the two-qubit gate error rate term.
- `readout_error_weight` (`float`): the weight of the readout error rate term.

### Static methods

- `LayoutObjective.topology_only()`: pure topology objective.
- `LayoutObjective.fidelity_aware()`: the default fidelity-aware objective.
- `LayoutObjective.auto_from_device(device)`: choose the fidelity-aware objective when the device has usable calibration data, otherwise fall back to the topology objective.
- `LayoutObjective.auto_from_physical(physical)`: as above, acting on a prepared physical graph.
- `LayoutObjective.fidelity_required(device)`: the fidelity-aware objective; raises when the device has no usable calibration data.
- `LayoutObjective.fidelity_required_from_physical(physical)`: as above, acting on a prepared physical graph.

### Attributes

- `distance_weight`, `direction_weight`, `two_qubit_error_weight`, `readout_error_weight` (`float`), corresponding one-to-one with the constructor parameters.
- `uses_fidelity -> bool`: whether fidelity terms are used.

### Methods

- `score_layout(analysis, physical, layout) -> LayoutScore`: compute the score from a circuit analysis, a physical graph and a complete layout.

### Other behavior

- Supports `==`, `copy` and `deepcopy`.

---

## LayoutScore

Layout scoring result.

### Attributes

- `total -> float`: the total score.
- `distance -> float`: the distance term.
- `direction -> float`: the direction term.
- `two_qubit_error -> float`: the two-qubit error term.
- `readout_error -> float`: the readout error term.
- `used_fidelity -> bool`: whether fidelity terms were used.

---

## LayoutDiagnostics

Diagnostic information of the layout process.

### Attributes

- `is_perfect -> bool`: whether a perfect layout was found.
- `candidates_evaluated -> int`: the number of candidates evaluated.
- `used_fidelity -> bool`: whether fidelity terms were used.
- `notes -> list[str]`: diagnostic notes.

---

## LayoutResult

The object returned by the layout functions.

### Attributes

- `layout -> Layout`: the selected logical-to-physical layout.
- `score -> LayoutScore | None`: the scoring result, `None` when no scoring is performed.
- `diagnostics -> LayoutDiagnostics`: diagnostic information.

---

## Vf2EdgeRequirement

Edge requirements of the VF2 search.

### Static methods

- `Vf2EdgeRequirement.positive_interactions()`: require only positive interaction edges to match.
- `Vf2EdgeRequirement.all_interactions()`: require all interaction edges to match.

---

## Vf2LayoutConfig

VF2 search configuration.

### Vf2LayoutConfig(*, candidate_limit=10, call_limit=None, edge_requirement=None)

Parameters:

- `candidate_limit` (`int`): the upper limit on the number of candidate layouts.
- `call_limit` (`int | None`): the upper limit on the number of search calls.
- `edge_requirement` (`Vf2EdgeRequirement | None`): the edge matching requirement.

### Attributes

`candidate_limit`, `call_limit`, `edge_requirement`, corresponding one-to-one with the constructor parameters.

---

## Layout analysis objects

### Interaction

A logical qubit pair interaction.

Attributes:

- `left -> Qubit`, `right -> Qubit`: the logical qubits at the two ends of the interaction.
- `weight -> float`: the total interaction weight.
- `directed_weight_left_to_right -> float`: the directed weight from left to right.
- `directed_weight_right_to_left -> float`: the directed weight from right to left.
- `first_seen_order -> int`: the first-seen order.

### InteractionGraph

Circuit interaction graph.

Attributes and methods:

- `interactions -> list[Interaction]`: the interaction list.
- `is_empty() -> bool`: whether there are no interactions.
- `logical_activity() -> list[tuple[Qubit, float]]`: the total interaction weight of each logical qubit.

### CircuitLayoutAnalysis

Circuit layout analysis result, returned by `analyze_circuit_for_layout()`.

Attributes:

- `logical_qubits -> list[Qubit]`: the logical qubit list.
- `interactions -> InteractionGraph`: the interaction graph.

### DistanceTable

Shortest distance table of the physical graph.

Attributes and methods:

- `qubits -> list[Qubit]`: the physical qubit list.
- `distance(a, b) -> int | None`: the shortest distance between two physical qubits, `None` when unreachable.

### PhysicalLayoutGraph

Device physical layout graph, constructed by `PhysicalLayoutGraph.from_device(device)`.

Attributes and methods:

- `physical_qubits -> list[Qubit]`
- `distances -> DistanceTable`
- `has_fidelity_data -> bool`: whether fidelity data is present.
- `has_readout_error_data -> bool`: whether readout error rate data is present.
- `has_two_qubit_error_data -> bool`: whether two-qubit gate error rate data is present.
- `distance(a, b) -> int | None`
- `is_adjacent_undirected(a, b) -> bool`
- `readout_error(qubit) -> float | None`
- `supports_two_qubit_gate_directed(a, b) -> bool`
- `two_qubit_gate_error_directed(a, b) -> float | None`
- `supports_directed_coupling(a, b) -> bool`

### PreparedSabreCircuit

SABRE circuit preprocessing result, returned by `prepare_sabre_circuit()`.

Attributes:

- `analysis -> CircuitLayoutAnalysis`
- `logical_qubits -> list[Qubit]`

### PreparedSabreTarget

Device physical graph preprocessing result, returned by `prepare_sabre_device_target()`.

Attributes:

- `physical -> PhysicalLayoutGraph`

---

## Raises

- `CompilerConfigError`: the configuration is invalid, for example a fidelity objective used on a device without calibration data.
- `CompilerTransformError`: the layout cannot be completed, for example an interaction cannot be satisfied on the topology.
