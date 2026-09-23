# Topology

`cqlib.device.Topology` describes the relationship between hardware physical qubits and coupling edges.  
The `cqlib` top level also re-exports the same type, so it can be imported directly from `cqlib`.

## Import

```python
from cqlib.device import Topology
```

---

## Construction

### `Topology(qubits, couplings)`

Parameters:

- `qubits` (`list[int | Qubit | PhysicalQubit]`): the list of physical qubits.
- `couplings` (`list[tuple[control, target, name]]`): the list of directed couplings, each entry a `(control, target, name)` triple.
  - `control -> target`: the coupling direction.
  - `name`: the name label of the coupling, used for identification only and not involved in connectivity or duplicate determination.

Description:

- The topology is stored with directed edge semantics; `(control, target)` and `(target, control)` are two different couplings.
- Only one coupling is allowed for the same ordered pair of qubits, and a different name still counts as a duplicate; duplicate qubits and self-coupling are both rejected.

### `Topology.line(qubits)`

Static method that constructs a directed straight-line topology `qubits[0] -> qubits[1] -> ...` in the given order, with `len(qubits) - 1` couplings whose names are empty strings.

Raises:

- `ValueError`: the qubit list contains duplicate qubits.

## Attributes

- `num_qubits -> int`
- `num_couplings -> int`
- `qubits -> list[PhysicalQubit]`

## Modification methods

### `add_qubits(qubits)`

Add physical qubits.

Raises:

- `ValueError`: a qubit in the list to add is duplicated, or duplicates a qubit already in the topology.

### `add_couplings(couplings)`

Add directed couplings; the parameter is a list of `(control, target, name)` triples.

Raises:

- `ValueError`: an endpoint qubit is not in the topology, the coupling already exists, or a self-coupling occurs.

### `remove_qubits(qubits)`

Remove physical qubits (the associated coupling edges are removed as well).

Raises:

- `ValueError`: a qubit to remove does not exist or appears more than once.

### `remove_couplings(couplings)`

Remove directed couplings. The parameter format is `list[tuple[control, target]]`; only the given direction is removed, and the reverse coupling is unaffected.

Raises:

- `ValueError`: a coupling to remove does not exist or appears more than once.

## Query methods

- `supports_directed_coupling(control, target) -> bool`: whether a coupling exists in the `control -> target` direction.
- `supports_coupling_either_direction(a, b) -> bool`: whether a coupling exists between the two qubits in either direction.
- `successors(qubit) -> list[PhysicalQubit]`: outgoing neighbors.
- `predecessors(qubit) -> list[PhysicalQubit]`: incoming neighbors.
- `neighbors_undirected(qubit) -> list[PhysicalQubit]`: undirected neighbors, each appearing only once across both directions.
- `undirected_edges() -> list[tuple[PhysicalQubit, PhysicalQubit]]`: the list of undirected edges, each pair sorted by value, with two couplings in opposite directions folded into one.
- `get_coupling_name(control, target) -> str | None`: the coupling name, or `None` when the coupling does not exist.
- `contains_qubit(qubit) -> bool`: whether the qubit is in the topology.
- `out_degree(qubit) -> int`: the number of outgoing edges.
- `in_degree(qubit) -> int`: the number of incoming edges.

Description:

- `successors` / `out_degree` are counted over outgoing edges, and `predecessors` / `in_degree` over incoming edges.
- When a qubit is not in the topology, neighbor queries return an empty list and degree queries return `0`, without raising an exception.

## Other behavior

- Supports `copy` and `deepcopy`.
- `repr(topology) == "Topology(num_qubits=3, num_couplings=2)"`.

## Example

```python
from cqlib.device import PhysicalQubit, Topology

topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])

assert topo.num_qubits == 3
assert topo.num_couplings == 2
assert topo.supports_directed_coupling(1, 2) is True
assert topo.supports_directed_coupling(2, 1) is False
assert topo.supports_coupling_either_direction(2, 1) is True
assert topo.get_coupling_name(1, 2) == "G2"
assert set(topo.successors(1)) == {PhysicalQubit(2)}
assert set(topo.neighbors_undirected(1)) == {PhysicalQubit(0), PhysicalQubit(2)}
assert topo.out_degree(1) == 1
assert topo.in_degree(1) == 1

topo.add_qubits([3])
topo.add_couplings([(2, 3, "G2")])
assert topo.supports_coupling_either_direction(2, 3) is True
topo.remove_couplings([(2, 3)])
topo.remove_qubits([3])
assert topo.num_qubits == 3
```
