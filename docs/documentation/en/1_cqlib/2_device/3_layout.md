# Layout mapping

Layout manages the bidirectional mapping from logical qubits to physical qubits, and is the core data structure of the compilation and routing stage.

---

## Core concepts

- **Logical qubit (LogicalQubit)**: a virtual qubit defined in an algorithm/circuit
- **Physical qubit (PhysicalQubit)**: the index of a real physical qubit on a hardware chip
- **Vacant physical qubit (Vacant PhysicalQubit)**: a physical position not occupied by any logical qubit, which can be used for subsequent binding

---

## Constructing a mapping

Layout supports several construction forms:

```python
from cqlib.device import Layout

# option 1: automatic sequential mapping
# logical qubits [0, 1] are mapped to physical qubits [10, 11] automatically
# physical qubit 12 stays vacant
layout = Layout(logical=[0, 1], physical=[10, 11, 12])
print("number of logical qubits:", layout.num_logical)       # 2
print("number of physical qubits:", layout.num_physical)      # 3
print("number of vacant physical qubits:", layout.num_vacant_physical)  # 1

# option 2: specify an initial mapping with from_pairs
# (logical, physical) pairs set the mapping explicitly; the other physical qubits stay vacant
layout2 = Layout.from_pairs([(0, 2), (1, 0)], physical_count=4)
print("from_pairs vacant count:", layout2.num_vacant_physical)  # 2
```

**Note**:
- The init_map parameter of Layout.__init__ requires the dict[Qubit, Qubit] type; passing {0: 11} directly raises TypeError due to the type mismatch
- If an initial mapping needs to be specified, Layout.from_pairs() is recommended instead
- The lengths of the logical and physical lists should satisfy len(logical) <= len(physical), otherwise ValueError is raised

---

## Querying a mapping

```python
from cqlib.device import Layout

layout = Layout.from_pairs([(0, 11), (1, 10)], physical_count=13)

print("logical qubit list:", layout.logical_qubits)
print("physical qubit list:", layout.physical_qubits)
print("vacant physical qubits:", layout.vacant_physical_qubits)

# forward lookup: logical -> physical
print("logical 0 maps to physical:", layout.get_physical(0))   # P11

# reverse lookup: physical -> logical
print("physical 11 maps to logical:", layout.get_logical(11))  # L0

# check whether a physical qubit is vacant
print("is physical 10 vacant:", layout.is_physical_vacant(10))  # False (occupied by logical 1)
print("is physical 12 vacant:", layout.is_physical_vacant(12))  # True

# get the full mapping dict
print("logical -> physical mapping:", layout.l2p_map)
print("physical -> logical mapping:", layout.p2l_map)
```

**Notes**:
- get_physical(logical_id) returns None for an unbound logical qubit
- get_logical(physical_id) returns None for a vacant physical qubit
- p2l_map contains only the physical qubits already occupied by logical qubits; vacant qubits do not appear in it

---

## Updating a mapping

```python
from cqlib.device import Layout

# initial mapping: logical 0 -> physical 11, logical 1 -> physical 12
layout = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)
print("initial vacant:", layout.num_vacant_physical)  # 11

# bind: bind a new logical qubit to a vacant physical qubit
layout.bind(2, 10)
print("vacant after bind:", layout.num_vacant_physical)  # 10

# unbind: unbind a logical qubit and release its physical qubit
released = layout.unbind(0)
print("released after unbind:", released)                    # P11
print("vacant after unbind:", layout.num_vacant_physical)  # 11

# swap_physical: swap the logical qubits carried by two physical qubits (the core routing operation)
layout3 = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)
layout3.swap_physical(11, 12)
print("physical 11 -> logical after SWAP:", layout3.get_logical(11))  # L1
print("physical 12 -> logical after SWAP:", layout3.get_logical(12))  # L0
```

**Edge cases**:
- bind(logical, physical): if physical is already occupied, or logical is already bound, ValueError is raised
- unbind(logical): if logical is not bound, ValueError is raised
- swap_physical(a, b): if  or  is not in the layout, ValueError is raised. One of them is allowed to be a vacant qubit (equivalent to moving a logical qubit)

---

## Robustness guarantees

```python
from cqlib.device import Layout

layout = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)

try:
    layout.swap_physical(11, 99)  # 99 is not in the layout
except ValueError as e:
    print("invalid physical qubit:", e)

try:
    layout.bind(0, 11)  # physical 11 is already occupied
except ValueError as e:
    print("bind an already occupied physical qubit:", e)

try:
    layout.swap_physical(11, 12)  # normal operation, no exception is raised
    print("SWAP succeeded")
    print("physical 11 -> logical:", layout.get_logical(11))
    print("physical 12 -> logical:", layout.get_logical(12))
except ValueError as e:
    print("SWAP failed:", e)
```

---

## Next steps

- [Noise model](4_noise.md): understand the use of noise channels such as NoiseModel, SingleQubitNoise, TwoQubitNoise and ReadoutError
- [Execution result and status](5_result.md): become familiar with the complete lifecycle and error handling of Outcome, Status and ExecutionResult
