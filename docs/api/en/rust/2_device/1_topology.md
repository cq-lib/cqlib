# Topology

`Topology` is the hardware connectivity graph structure in `cqlib_core::device`, using a directed graph to represent qubit coupling relationships.

## Import

```rust
use cqlib_core::device::{PhysicalQubit, Topology, TopologyError};
```

## Construction

### `Topology::new(qubits, coupling_map) -> Result<Topology, TopologyError>`

Parameters:

- `qubits: Vec<PhysicalQubit>`
- `coupling_map: Vec<(PhysicalQubit, PhysicalQubit, String)>`

Notes:

- Edges are treated with directed semantics; `(q0, q1)` and `(q1, q0)` are not equivalent.
- Only one coupling is allowed for the same ordered pair of qubits, and a different name still counts as a duplicate; duplicate qubits and self-coupling return an error.

### `Topology::line(qubits) -> Result<Topology, TopologyError>`

Parameters:

- `qubits: Vec<PhysicalQubit>`

Notes:

- Constructs a directed line `qubits[0] -> qubits[1] -> ...` in the given order, with `len(qubits) - 1` couplings and an empty string as the coupling name.

## Read-only interface

- `graph(&self) -> &StableGraph<PhysicalQubit, String>`
- `num_qubits(&self) -> usize`
- `num_couplings(&self) -> usize`
- `qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `supports_directed_coupling(&self, control: PhysicalQubit, target: PhysicalQubit) -> bool`
- `supports_coupling_either_direction(&self, a: PhysicalQubit, b: PhysicalQubit) -> bool`
- `successors(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `predecessors(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `neighbors_undirected(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `undirected_edges(&self) -> impl Iterator<Item = (PhysicalQubit, PhysicalQubit)>`
- `get_coupling_name(&self, control: PhysicalQubit, target: PhysicalQubit) -> Option<String>`
- `contains_qubit(&self, qubit: &PhysicalQubit) -> bool`
- `out_degree(&self, qubit: &PhysicalQubit) -> usize`
- `in_degree(&self, qubit: &PhysicalQubit) -> usize`

Notes:

- `successors` / `out_degree` count outgoing edges, while `predecessors` / `in_degree` count incoming edges.
- Each pair from `undirected_edges` is sorted by key value, and two couplings in opposite directions are folded into one.
- When a qubit is not in the topology, neighbor queries return an empty iterator and degree queries return `0`.

## Modification interface

- `add_qubits(&mut self, qubits: impl IntoIterator<Item = PhysicalQubit>) -> Result<(), TopologyError>`
- `add_couplings(&mut self, couplings: impl IntoIterator<Item = (PhysicalQubit, PhysicalQubit, String)>) -> Result<(), TopologyError>`
- `remove_qubits(&mut self, qubits: impl IntoIterator<Item = PhysicalQubit>) -> Result<(), TopologyError>`
- `remove_couplings(&mut self, couplings: impl IntoIterator<Item = (PhysicalQubit, PhysicalQubit)>) -> Result<(), TopologyError>`

Common errors:

- `TopologyError::QubitNotFound`
- `TopologyError::QubitAlreadyExists`
- `TopologyError::CouplingNotFound`
- `TopologyError::CouplingAlreadyExists`
- `TopologyError::SelfCoupling`
- `TopologyError::DuplicateQubitRemoval`
- `TopologyError::DuplicateCouplingRemoval`

## Example

```rust
use cqlib_core::device::{PhysicalQubit, Topology};

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let q2 = PhysicalQubit::new(2);

let mut topo = Topology::new(
    vec![q0, q1, q2],
    vec![(q0, q1, "CX".to_string()), (q1, q2, "CZ".to_string())],
)
.unwrap();

assert_eq!(topo.num_qubits(), 3);
assert_eq!(topo.num_couplings(), 2);
assert!(topo.supports_directed_coupling(q1, q2));
assert!(!topo.supports_directed_coupling(q2, q1));
assert!(topo.supports_coupling_either_direction(q2, q1));
assert_eq!(topo.get_coupling_name(q1, q2).as_deref(), Some("CZ"));
assert_eq!(topo.successors(q1).collect::<Vec<_>>(), vec![q2]);
assert_eq!(topo.neighbors_undirected(q1).collect::<Vec<_>>(), vec![q0, q2]);
assert_eq!(topo.out_degree(&q1), 1);
assert_eq!(topo.in_degree(&q1), 1);

topo.add_qubits([PhysicalQubit::new(3)]).unwrap();
topo.add_couplings([(q2, PhysicalQubit::new(3), "CX".to_string())])
    .unwrap();
topo.remove_couplings([(q2, PhysicalQubit::new(3))]).unwrap();
topo.remove_qubits([PhysicalQubit::new(3)]).unwrap();
assert_eq!(topo.num_qubits(), 3);
```
