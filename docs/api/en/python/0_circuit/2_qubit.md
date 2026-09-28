# Qubit

`cqlib.circuit.Qubit`

```python
from cqlib import Qubit
```


`Qubit` is the basic type in Cqlib for representing logical qubits. It is usually used as a handle for qubits in a quantum circuit, identifying the logical qubit index with a non-negative integer. Note that `Qubit` only represents a logical index; it does not hold the quantum state itself, nor is it bound to a specific physical device location. It also does not record which `Circuit` it belongs to. Whether a certain `Qubit` can be used in a circuit depends on whether that qubit has already been registered in the current `Circuit`.

---

## Constructor

```python
Qubit(index: int)
```

`Qubit(index)` creates a logical qubit handle. `index` must be a non-negative integer and is stored internally as an unsigned integer. The acceptable range is `0` to `4294967295` (that is, `2**32 - 1`): passing a negative integer or an integer greater than that upper bound raises `QubitError`, while passing a non-integer (for example a float or a string) raises `TypeError`.

| Parameter | Type | Description |
| --- | --- | --- |
| `index` | `int` | The logical qubit index; the value must be between `0` and `4294967295`. |

Example:

```python
from cqlib import Qubit

q0 = Qubit(0)
q5 = Qubit(5)

print(q0.index)
print(q5.index)
```

---

## Attributes

`Qubit` provides the following common attributes:

| Attribute | Type | Description |
| --- | --- | --- |
| `index` | `int` | The logical qubit index. |
| `id` | `int` | The internal raw index, numerically equal to `index` by default. |

Usually, using `index` directly is sufficient. `id` is more for low-level debugging, internal representation, or alignment with other low-level data structures.

```python
from cqlib import Qubit

q = Qubit(3)

print(q.index)  # 3
print(q.id)     # 3
```

---

## Comparison, ordering and hashing

`Qubit` supports common comparison, ordering and hashing operations, including:

- `==` / `!=`
- `<` / `<=` / `>` / `>=`
- `hash()`
- `copy.copy()` / `copy.deepcopy()`
- `str()` / `repr()`

These operations are all judged based on the qubit index. That is, two `Qubit` objects are regarded as the same logical qubit as long as their indices are equal.

```python
from cqlib import Qubit

assert Qubit(0) == Qubit(0)
assert Qubit(0) < Qubit(1)

mapping = {Qubit(0): "ancilla"}
assert mapping[Qubit(0)] == "ancilla"
```

---

## `Circuit` and `Qubit`

Most `Circuit` gate methods accept both integer indices and `Qubit` objects. For simple circuits, using integer indices directly is usually more concise; for scenarios that need explicit management of logical qubit indices, using `Qubit` objects is clearer.

```python
from cqlib import Circuit, Qubit

circuit = Circuit(2)

circuit.h(0)
circuit.cx(Qubit(0), Qubit(1))
```

In the example above, both `h(0)` and `cx(Qubit(0), Qubit(1))` act on logical qubits already registered in the current circuit.

When constructing a circuit, a list of `Qubit` objects can also be passed in directly:

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])
circuit.cx(Qubit(10), Qubit(20))
```

---

## Sparse logical indices

The index of a `Qubit` is a logical identifier and is not required to be contiguous starting from `0`. For example, `Qubit(10)` and `Qubit(20)` can together form a two-qubit circuit.

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])

assert circuit.num_qubits == 2
assert [q.index for q in circuit.qubits] == [10, 20]
```

Note that a logical index is not necessarily the same as the axis order in a matrix or statevector. For `Circuit([Qubit(10), Qubit(20)])`, the circuit contains only two qubits internally and the matrix dimension is `4 × 4`. By default, matrix conversion interprets qubits according to the order of `qubits` saved in the circuit.

If the matrix qubit order needs to be controlled explicitly, pass `qubits_order` during matrix conversion:

```python
matrix = circuit.to_matrix([20, 10])
```

This means the qubit axes are interpreted in the order of logical qubits `20` and `10` when the matrix is constructed.

---

## Semantic boundary

`Qubit` is the "address" or "handle" of a logical qubit, not a quantum state object. `Qubit`s with the same index can appear in different circuits, but those circuits are independent of each other.

```python
from cqlib import Circuit, Qubit

q0 = Qubit(0)

left = Circuit([q0])
right = Circuit([q0])

left.x(q0)
right.h(q0)
```

In the example above, both `left` and `right` use `Qubit(0)`, but they are two different circuits and do not affect each other.

---

## Difference from physical qubits

`Qubit.index` represents a logical qubit index and is not the same as a physical qubit index on a real device. When executing against hardware, a logical qubit usually needs layout and mapping before it can correspond to a physical qubit on a specific device. For example, the logical qubit `Qubit(0)` may be mapped to the physical qubit `Q5` on one device, and may be mapped to another physical location on another device or in another compilation result.

---

## Complete example

The following example shows typical usage of `Qubit`, including constructing logical qubits, creating a circuit with sparse indices, adding gate operations and specifying the matrix qubit order.

```python
from cqlib import Circuit, Qubit

q10 = Qubit(10)
q20 = Qubit(20)

circuit = Circuit([q10, q20])
circuit.h(q10)
circuit.cx(q10, q20)

print(circuit.num_qubits)
print([q.index for q in circuit.qubits])

matrix_default = circuit.to_matrix()
matrix_reversed = circuit.to_matrix([20, 10])

print(matrix_default.shape)
print(matrix_reversed.shape)
```
