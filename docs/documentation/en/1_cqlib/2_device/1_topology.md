# Topology modeling

Topology is a graph-theoretic abstraction of the physical qubits of a quantum chip and their coupling connectivity. It is the core input of all advanced compilation and routing algorithms: only when the hardware constraints of the chip are accurately known can the compiler find the optimal gate mapping scheme under a limited connectivity path.

The Topology of Cqlib adopts a **directed graph** model. This means that (0, 1, "CX") is not equivalent to (1, 0, "CX")—this edge only allows the CX gate to be executed when qubit 0 is the control and qubit 1 is the target. Directed modeling accurately reflects the direction dependence of two-qubit gates in real hardware.

---

## Constructing a topology

### 1. Explicit construction

Each edge must use a (control, target, name) triple:

```python
from cqlib.device import Topology

topo = Topology(
    [0, 1, 2],
    [
        (0, 1, "CX"),   # CX applied in the qubit 0 -> qubit 1 direction
        (1, 2, "CZ"),   # CZ applied in the qubit 1 -> qubit 2 direction
    ],
)

print("qubit count:", topo.num_qubits)
print("coupling count:", topo.num_couplings)
print("node list:", topo.qubits)
```

**Note**: the couplings parameter of the constructor requires (control, target, name) triples, and the two-element form is not supported.

### 2. Factory methods

Topology currently provides the line factory, used to create a line topology quickly:

```python
from cqlib.device import Topology

# Line topology: 0 -> 1 -> 2 -> 3 (a one-way chain)
line_topo = Topology.line([0, 1, 2, 3])
print("line coupling count:", line_topo.num_couplings)
```

**Notes**:
- Only the line factory method exists on Topology; there is no ring/star/grid/bidirectional_line
- For richer topology structures, the factory methods of Device can be used (see [Device modeling](2_device.md) for details), or bidirectional couplings can be added manually

### 3. Manually adding bidirectional coupling

```python
from cqlib.device import Topology

# Qubits 0 and 1 act as each other's control and target (bidirectional coupling)
topo = Topology([0, 1], [(0, 1, "CX"), (1, 0, "CX")])
print("supports 0->1:", topo.supports_directed_coupling(0, 1))   # True
print("supports 1->0:", topo.supports_directed_coupling(1, 0))   # True
```

---

## Topology queries and analysis

```python
from cqlib.device import Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# Connectivity checks
print("directed coupling 0->1:", topo.supports_directed_coupling(0, 1))  # True
print("directed coupling 1->0:", topo.supports_directed_coupling(1, 0))  # False
print("either direction present:", topo.supports_coupling_either_direction(0, 1))  # True
print("coupling name 0->1:", topo.get_coupling_name(0, 1))    # "CX"
print("coupling name 2->1 (reverse absent):", topo.get_coupling_name(2, 1))  # None

# Check whether a qubit exists
print("contains qubit 2:", topo.contains_qubit(2))    # True
print("contains qubit 9:", topo.contains_qubit(9))    # False

# Adjacency queries
print("successors of qubit 1 (out edges):", topo.successors(1))           # [PhysicalQubit(2)]
print("predecessors of qubit 1 (in edges):", topo.predecessors(1))         # [PhysicalQubit(0)]
print("undirected neighbours of qubit 1:", topo.neighbors_undirected(1))     # [PhysicalQubit(0), PhysicalQubit(2)]
print("out-degree of qubit 1:", topo.out_degree(1))                   # 1
print("in-degree of qubit 1:", topo.in_degree(1))                    # 1

# Undirected edge list (undirected_edges is a method and needs parentheses)
edges = topo.undirected_edges()
print("undirected edge list:", edges)
```

**Notes**:
- successors(q) returns the list of qubits directly reachable from q through directed edges
- predecessors(q) returns the list of qubits whose directed edges point to q
- 
neighbors_undirected(q) merges the two directions, and a repeated node appears only once
- contains_qubit(q) returns False for a non-existent qubit and does not raise an exception
- out_degree(q) and in_degree(q) return 0 for a non-existent qubit

---

## Dynamic topology modification

```python
from cqlib.device import Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# Add new qubits
topo.add_qubits([3, 4])
print("qubit count after adding:", topo.num_qubits)  # 5

# Add new couplings (note: new edges also need the triple form)
topo.add_couplings([(2, 3, "CX"), (3, 4, "CZ")])
print("coupling count after adding:", topo.num_couplings)  # 4

# Remove couplings (only the given direction is removed)
topo.remove_couplings([(2, 3)])
print("coupling count after removal:", topo.num_couplings)  # 3 (coupling (2,3) removed)

# Remove qubits (all associated couplings are removed with them)
topo.remove_qubits([4])
print("qubit count after removal:", topo.num_qubits)     # 4
print("coupling count after removal:", topo.num_couplings)  # 2 (coupling (3,4) removed)
```

**Edge cases**:
- add_qubits([...]): adding a qubit that already exists raises ValueError
- add_couplings([...]): if a coupling endpoint is not in the topology, or an existing coupling is added, ValueError is raised
- 
remove_qubits([...]): if the qubit does not exist, ValueError is raised
- 
remove_couplings([...]) takes (control, target) two-element tuples for the couplings parameter (**without names**)
- Removing a qubit automatically removes all its incoming and outgoing coupling edges

---

## Engineering meaning of directed coupling

In real quantum hardware, the directionality of two-qubit gates is an important physical constraint:

| Scenario | Directed semantics | Example |
|---|---|---|
| CX gate | Only a fixed direction is allowed | CX(control=0, target=1) |
| CZ gate | Usually no direction restriction | Both directions can be executed |
| Hardware calibration | Different directions may have different fidelities | (0→1) error rate 1%, (1→0) error rate 2% |

During the routing stage, the compiler uses query methods such as supports_directed_coupling() to determine whether a SWAP or a bridge gate needs to be inserted on the current coupling.

---

## Next steps

- [Device modeling](2_device.md): master the global default plus local override calibration strategy of Device
- [Layout mapping](3_layout.md): learn the bidirectional logical-physical qubit mapping and SWAP routing operations of Layout
- [Noise model](4_noise.md): understand the use of noise channels such as NoiseModel, SingleQubitNoise, TwoQubitNoise and ReadoutError
- [Execution result and status](5_result.md): become familiar with the complete lifecycle and error handling of Outcome, Status and ExecutionResult
