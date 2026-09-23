# Qubit Identifiers

`cqlib.device`

`LogicalQubit` and `PhysicalQubit` are the strongly typed qubit identifiers used by the device-side interfaces: the former identifies a logical qubit in a circuit, and the latter identifies a physical qubit on a device. Both types use a numeric id as their internal representation, but they are different types, so two identifiers with the same numeric value are not equal.

## Import

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit
```

---

## LogicalQubit

The logical qubit identifier, used for logical qubits in a circuit.

### LogicalQubit(id)

Parameters:

- `id` (`int`): the numeric id, a non-negative integer.

Raises:

- `TypeError`: `id` is not an integer.
- `OverflowError`: `id` is negative or outside the `u32` range.

### Attributes

- `id -> int`: the numeric id.
- `index -> int`: the index form of the numeric id.
- `qubit -> Qubit`: the corresponding circuit qubit.

### Other behavior

- Supports `==`, `<`, `<=`, `>`, `>=`, `hash`, `copy` and `deepcopy`.
- Comparison holds only within this type: comparison with a `Qubit` or a `PhysicalQubit` is judged unequal and does not raise an exception.
- `str(LogicalQubit(0)) == "L0"`; `repr(LogicalQubit(0)) == "LogicalQubit(0)"`.

Example:

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit

logical = LogicalQubit(0)
assert logical.id == 0
assert logical.index == 0
assert logical.qubit == Qubit(0)
assert logical != Qubit(0)
assert logical != PhysicalQubit(0)
assert str(logical) == "L0"
assert repr(logical) == "LogicalQubit(0)"

assert sorted([LogicalQubit(2), LogicalQubit(0)]) == [LogicalQubit(0), LogicalQubit(2)]
assert LogicalQubit(2) >= LogicalQubit(2)
```

---

## PhysicalQubit

The physical qubit identifier, used for qubit positions on a device.

### PhysicalQubit(id)

Parameters:

- `id` (`int`): the numeric id, a non-negative integer.

Raises:

- `TypeError`: `id` is not an integer.
- `OverflowError`: `id` is negative or outside the `u32` range.

### Attributes

- `id -> int`: the numeric id.
- `index -> int`: the index form of the numeric id.
- `qubit -> Qubit`: the corresponding circuit qubit.

### Other behavior

- Supports `==`, `<`, `<=`, `>`, `>=`, `hash`, `copy` and `deepcopy`.
- Comparison holds only within this type: comparison with a `Qubit` or a `LogicalQubit` is judged unequal and does not raise an exception.
- `str(PhysicalQubit(11)) == "P11"`; `repr(PhysicalQubit(11)) == "PhysicalQubit(11)"`.

Example:

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit

physical = PhysicalQubit(11)
assert physical.id == 11
assert physical.index == 11
assert physical.qubit == Qubit(11)
assert physical == PhysicalQubit(11)
assert physical != Qubit(11)
assert physical != LogicalQubit(11)
assert str(physical) == "P11"
assert repr(physical) == "PhysicalQubit(11)"

assert sorted([PhysicalQubit(2), PhysicalQubit(0)]) == [PhysicalQubit(0), PhysicalQubit(2)]
assert PhysicalQubit(1) < PhysicalQubit(2)
```

---

## Role in the layout

The layout maintains the mapping from logical qubits to physical qubits, which is the main scenario where the two types are used together: logical qubits come from the circuit, physical qubits come from the device, and the mapping result is recorded with strongly typed identifiers, avoiding a mix of the two kinds of numeric values.

Example:

```python
from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit

layout = Layout(logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)})

assert layout.num_logical == 2
assert layout.num_physical == 3
assert layout.num_vacant_physical == 1

assert set(layout.logical_qubits) == {LogicalQubit(0), LogicalQubit(1)}
assert set(layout.physical_qubits) == {PhysicalQubit(10), PhysicalQubit(11), PhysicalQubit(12)}

assert layout.get_physical(0) == PhysicalQubit(11)
assert layout.get_logical(11) == LogicalQubit(0)
assert layout.l2p_map == {LogicalQubit(0): PhysicalQubit(11), LogicalQubit(1): PhysicalQubit(10)}
assert layout.p2l_map == {PhysicalQubit(10): LogicalQubit(1), PhysicalQubit(11): LogicalQubit(0)}

before_11 = layout.get_logical(11)
before_12 = layout.get_logical(12)
layout.swap_physical(11, 12)
assert layout.get_logical(11) == before_12
assert layout.get_logical(12) == before_11

layout.unbind(0)
assert layout.get_physical(0) is None
```

---

## Interfaces that accept qubit parameters

The device-side interfaces accept an `int` or an identifier object for a qubit parameter, and accept `Qubit` as well, all parsed by numeric id; qubits are always returned as strongly typed identifier objects. A logical qubit parameter position accepts `int | LogicalQubit | Qubit`, and a physical qubit parameter position accepts `int | PhysicalQubit | Qubit`.

- The layout `Layout`: constructor parameters `logical`, `physical`, `init_map`, and `get_physical`, `get_logical`, `bind`, `unbind`, `is_physical_vacant`, `swap_physical`.
- The topology `Topology`: constructor parameters `qubits`, `couplings`, and `add_qubits`, `remove_qubits`, `add_couplings`, `remove_couplings`, `contains_qubit`, `successors`, `predecessors`, `neighbors_undirected`, `out_degree`, `in_degree`, `supports_directed_coupling`, `supports_coupling_either_direction`, `get_coupling_name`.
- The device `Device`: the constructor parameter `qubits` and the parameters of `line_from_qubits`, and `add_qubit_properties`, `add_edge_properties`, `qubit_properties`, `edge_properties`, `get_t1`, `get_t2`, `get_readout_error`, `is_usable_qubit`, `single_qubit_error`, `two_qubit_error`, `edge_error`, `supports_native_instruction`, `set_invalid_qubits`.
- Noise `NoiseModel`: `add_readout_error`, `add_single_qubit_error`, `add_two_qubit_error`, `get_readout_error`; and `new_single`, `new_double`, `new_triple` of `OperationKey`.
