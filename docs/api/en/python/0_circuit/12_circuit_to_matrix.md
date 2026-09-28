# Circuit To Matrix

- `cqlib.circuit.circuit_to_matrix`
- `cqlib.circuit.Circuit.to_matrix`
- `cqlib.circuit.Circuit.to_symbolic_matrix`

```python
from cqlib.circuit import circuit_to_matrix
```

The `Circuit To Matrix` interfaces convert a quantum circuit into a matrix representation. Cqlib provides both numeric matrix and symbolic matrix capabilities: numeric matrices give a definite complex unitary matrix, while symbolic matrices preserve the `Parameter` expressions in a circuit.

---

## Overview

Cqlib provides the following three common entry points:

| Interface | Returns | Description |
| --- | --- | --- |
| `Circuit.to_matrix(qubits_order=None)` | `numpy.ndarray[np.complex128]` | Convert the whole circuit into a numeric complex matrix. |
| `circuit_to_matrix(circuit, qubits_order=None)` | `numpy.ndarray[np.complex128]` | A functional entry equivalent to `Circuit.to_matrix()`. |
| `Circuit.to_symbolic_matrix(qubits_order=None)` | `SymbolicMatrix` | Convert the circuit into a symbolic matrix that preserves symbolic parameters. |

## Numeric matrix conversion

Numeric matrix conversion converts a circuit made only of quantum gates into a definite dense complex matrix. This matrix describes the overall linear transformation the circuit applies to a quantum state.

```python
Circuit.to_matrix(
    qubits_order: list[int] | None = None,
) -> np.ndarray[np.complex128]

circuit_to_matrix(
    circuit: Circuit,
    qubits_order: list[int] | None = None,
) -> np.ndarray[np.complex128]
```

This conversion applies only to circuits made only of quantum gates. If unbound symbolic parameters remain in the circuit, or the circuit contains measurement, classical data operations or control flow, both `Circuit.to_matrix()` and `circuit_to_matrix()` raise `CircuitError`.

The following example constructs a Bell circuit and computes the matrix through the object method and the functional entry respectively:

```python
from cqlib import Circuit
from cqlib.circuit import circuit_to_matrix

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

a = circuit.to_matrix()
b = circuit_to_matrix(circuit)

assert (a == b).all()
```

## Output shape

If a circuit contains `n` qubits, the shape of the full matrix is:

```text
(2**n, 2**n)
```

For example, a 3-qubit circuit corresponds to an `8 × 8` matrix:

```python
from cqlib import Circuit

circuit = Circuit(3)
matrix = circuit.to_matrix()

assert matrix.shape == (8, 8)
```

Note that the matrix dimension depends on the number of qubits in the circuit. For example, `Circuit([0, 10])` is still a two-qubit circuit and its matrix shape is `(4, 4)`.

## Qubit order

`qubits_order` specifies the qubit arrangement order in the matrix representation. By default, Cqlib generates the matrix in the qubit order kept internally by the circuit. For contiguously numbered circuits this usually matches user intuition; but for circuits with sparse logical indices or a manually specified qubit order, it is advisable to pass `qubits_order` explicitly to avoid ambiguity in matrix interpretation.

```python
from cqlib import Circuit

circuit = Circuit([0, 2])
circuit.cx(0, 2)

default = circuit.to_matrix()
reordered = circuit.to_matrix([2, 0])

print(default.shape)
print(reordered.shape)
```

In the example above, the circuit contains only the logical qubits `0` and `2`, so the matrix is still a two-qubit matrix. `qubits_order=[2, 0]` means the reversed logical qubit order is used for the tensor axes of the matrix.

Using `qubits_order` must satisfy the following requirements:

- all qubits passed in must belong to the current circuit;
- the list length should agree with the number of qubits in the circuit;
- duplicate qubits are not allowed;

## Parameterized circuits

A numeric matrix requires all symbolic parameters in the circuit to be bound to concrete values.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

bound = circuit.assign_parameters({"theta": 0.5})
matrix = bound.to_matrix()
```

## Symbolic matrix conversion

Symbolic matrix conversion preserves the `Parameter` expressions in a circuit. This interface returns a `SymbolicMatrix`, whose elements can contain symbolic complex numbers and parameter expressions.

```python
Circuit.to_symbolic_matrix(
    qubits_order: list[int] | None = None,
) -> SymbolicMatrix
```

Example:

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rz(0, theta)

symbolic = circuit.to_symbolic_matrix()
assert "theta" in symbolic.symbols

numeric = symbolic.evaluate({"theta": 0.25})
```

In the example above, `symbolic` preserves the parameter `theta`. Concrete parameter values can later be passed through `evaluate()` to evaluate the symbolic matrix into a numeric matrix.

## Typical application scenarios

### 1. Checking the matrix of a Bell circuit

```python
import numpy as np
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()

assert matrix.shape == (4, 4)
assert np.allclose(matrix.conj().T @ matrix, np.eye(4))
```

The code above checks whether the matrix is close to the identity matrix through `matrix.conj().T @ matrix`, verifying that the circuit as a whole is unitary.

### 2. Validating a custom gate matrix

```python
import numpy as np
from cqlib.circuit.gates import UnitaryGate

mat = np.array([[0, 1], [1, 0]], dtype=np.complex128)
gate = UnitaryGate("CustomX", 1).with_matrix(mat)

assert np.allclose(gate.matrix(), mat)
```

When a custom gate is defined through `UnitaryGate`, the gate matrix itself can be validated first and the gate then appended to a circuit for use.

### 3. Comparing the matrices of a composite gate before and after expansion

```python
from cqlib import Circuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

gate = sub.to_gate("Bell")

outer = Circuit(2)
outer.append_circuit_gate(gate, [0, 1])

assert outer.to_matrix().shape == sub.to_matrix().shape
```

For a `CircuitGate` obtained by wrapping a sub-circuit, matrix comparison can check whether the same quantum behavior is preserved before and after wrapping. For stricter validation, `np.allclose()` can be used further to compare the elements of the two matrices.

