# Layout

`Layout` maintains the mapping from logical qubits to physical qubits.

## Import

```python
from cqlib.device import Layout
```

---

## Constructor

### `Layout(logical, physical, init_map=None)`

Parameters:

- `logical` (`list[int | Qubit | LogicalQubit]`): the list of logical qubits.
- `physical` (`list[int | Qubit | PhysicalQubit]`): the list of physical qubits, which may be more than the logical qubits.
- `init_map` (`dict[LogicalQubit, PhysicalQubit] | None`): an optional initial mapping (logical -> physical); keys and values may also be `int` or `Qubit`.

Raises:

- `ValueError`: there are more logical qubits than physical qubits, a logical or physical qubit is duplicated, a qubit in the mapping does not belong to the layout, or the same logical or physical qubit is mapped more than once.

### `Layout.from_pairs(pairs, physical_count)`

Static method that constructs a layout from `(logical id, physical id)` pairs and the total number of physical qubits. The physical qubits are `0..physical_count`, and a physical qubit not referenced by `pairs` is a vacant physical qubit.

Raises:

- `ValueError`: a logical id is duplicated, a physical id is duplicated, or a physical id is outside `0..physical_count`.

## Attributes

- `num_logical -> int`
- `num_physical -> int`
- `num_vacant_physical -> int`
- `logical_qubits -> list[LogicalQubit]`
- `physical_qubits -> list[PhysicalQubit]`
- `vacant_physical_qubits -> list[PhysicalQubit]`
- `l2p_map -> dict[LogicalQubit, PhysicalQubit]`
- `p2l_map -> dict[PhysicalQubit, LogicalQubit]`

## Methods

### `get_physical(logical_id) -> PhysicalQubit | None`

Look up the physical qubit by logical qubit; returns `None` when it is not bound.

### `get_logical(physical_id) -> LogicalQubit | None`

Look up the logical qubit by physical qubit; returns `None` when the physical qubit is vacant.

### `is_physical_vacant(physical_id) -> bool`

Whether the physical qubit is vacant.

### `bind(logical_id, physical_id) -> None`

Bind a logical qubit to a vacant physical qubit.

Raises:

- `ValueError`: the physical qubit does not belong to the layout, the logical qubit is already bound, or the physical qubit is already occupied.

### `unbind(logical_id) -> PhysicalQubit`

Unbind a logical qubit and return the physical qubit that is released.

Raises:

- `ValueError`: the logical qubit is not bound.

### `swap_physical(phys_a, phys_b) -> None`

Swap the logical qubits carried on two physical qubits; this is the basic operation for moving logical qubits during routing. Either physical qubit may be vacant: swapping with a vacant physical qubit moves the logical qubit to the vacant position.

Raises:

- `ValueError`: `phys_a` or `phys_b` is not in the layout's set of physical qubits.

## Other behavior

- Supports `copy` and `deepcopy`.
- `repr(layout) == "Layout(num_logical=2, num_vacant_physical=1, num_physical=3)"`.

## Example

```python
from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit

layout = Layout(logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)})

print(layout.num_logical)          # 2
print(layout.num_physical)         # 3
print(layout.num_vacant_physical)  # 1

assert layout.get_physical(0) == PhysicalQubit(11)
assert layout.get_logical(11) == LogicalQubit(0)
assert layout.get_logical(12) is None
assert layout.is_physical_vacant(12) is True

before_11 = layout.get_logical(11)
before_12 = layout.get_logical(12)
layout.swap_physical(11, 12)
assert layout.get_logical(12) == before_11
assert layout.get_logical(11) == before_12

released = layout.unbind(0)
assert released == PhysicalQubit(12)
assert layout.is_physical_vacant(12) is True
```
