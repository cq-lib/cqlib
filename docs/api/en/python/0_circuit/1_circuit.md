# Circuit

`cqlib.circuit.Circuit`

```python
from cqlib import Circuit
```

`Circuit` is the most central circuit container in the Cqlib Python API, used to represent a complete quantum program. It is responsible for holding the set of qubits, the operation sequence in order, symbolic parameters, the global phase, classical variables and classical values, and structured control flow.

---

## Constructing a circuit

```python
Circuit(qubits: int | list[int] | list[Qubit])
```

`Circuit` supports several construction forms: the number of qubits can be given directly, or logical qubit indices can be specified explicitly, or existing `Qubit` objects can be passed in.

| Form | Meaning |
| --- | --- |
| `Circuit(3)` | Create a three-qubit circuit containing `Qubit(0)`, `Qubit(1)` and `Qubit(2)`. |
| `Circuit([0, 2, 4])` | Create a circuit with sparse logical qubit indices. |
| `Circuit([Qubit(5), Qubit(7)])` | Create a circuit with existing `Qubit` objects. |

Example:

```python
from cqlib import Circuit, Qubit

a = Circuit(3)
b = Circuit([0, 2, 4])
c = Circuit([Qubit(10), Qubit(11)])
```

---

## Low-level construction

```python
Circuit.from_operations(
    qubits: list[Qubit],
    operations: list[ValueOperation],
    classical_vars: list[ClassicalType] | None = None,
    classical_values: list[ClassicalType] | None = None,
) -> Circuit
```

`Circuit.from_operations()` rebuilds a circuit from an existing qubit list and operation sequence. This interface is a relatively low-level construction entry point, usually used for deserialization, IR import, restoring compiler pass output, or constructing an exact low-level circuit object in tests.

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation

op = ValueOperation.from_standard_gate(StandardGate.H(), [Qubit(0)])
circuit = Circuit.from_operations([Qubit(0)], [op])
```

---

## Common circuit attributes

| Attribute | Type | Description |
| --- | --- | --- |
| `id` | `CircuitId` | The unique identity of the current circuit, used for classical handles, measurement values and control flow scope management. |
| `num_qubits` | `int` | The number of qubits contained in the circuit. |
| `width` | `int` | An alias of `num_qubits`. |
| `qubits` | `list[Qubit]` | The qubit list kept in the circuit's internal order. |
| `parameters` | `list[Parameter]` | The list of parameter expressions recorded in the circuit. |
| `symbols` | `list[str]` | All free symbol names in the circuit. This list is a stable symbol registry and may contain symbols that are no longer referenced by the executable IR. |
| `used_symbols` | `list[str]` | The free symbol names actually referenced by the current executable IR. |
| `global_phase` | `Parameter` | The overall global phase of the circuit. |
| `classical_vars` | `list[ClassicalType]` | The list of allocated mutable classical variable types. |
| `classical_values` | `list[ClassicalType]` | The list of immutable classical value types produced by operations such as measurement. |
| `operations` | `list[ValueOperation]` | The self-contained operation list returned in append order. |

`operations` returns a list of `ValueOperation`s that can be inspected and rebuilt at the Python level. Each operation contains information such as the instruction type, the qubits acted on, the parameters and the label. This attribute is commonly used for circuit debugging, structural analysis, test assertions and comparison before and after a compiler pass.

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(circuit.num_qubits)
print(circuit.operations)
```

---

## Basic circuit operations

| Method | Description |
| --- | --- |
| `add_qubits(qubits)` | Append new qubits to the current circuit. |
| `append(operation)` | Append a self-contained `ValueOperation` to the circuit. |
| `operation(index)` | Return the `ValueOperation` at the given position; raises `IndexError` when the index is out of range. |
| `remove_operation(index)` | Remove the operation at the given position and return the removed `ValueOperation`; raises `IndexError` when the index is out of range. |
| `remove_operations(indices)` | Remove the operations at several positions in a batch and return the list of removed `ValueOperation`s; raises `IndexError` when an index is out of range. If a measurement result to be removed is still referenced by `store`, raises `CircuitError`. |
| `__len__()` | Return the number of operations in the circuit. |
| `__getitem__(index)` | Support reading operations with a forward or negative index. |
| `uses_symbol(symbol)` | Determine whether the current executable IR references the given symbol; returns `bool`. |
| `var(ty)` | Allocate a mutable classical variable owned by the current circuit, where `ty` is a `ClassicalType`. |
| `store(target, value)` | Write the classical expression `value` into the classical variable `target`. |
| `dag()` | Return the directed acyclic graph representation `CircuitDag` corresponding to the current circuit. |
| `validate()` | Validate the circuit structure, classical handles, scopes and control flow invariants. |
| `depth(recurse=False)` | Compute the circuit depth. |
| `set_global_phase(phase)` | Set the circuit global phase. |


`depth()` estimates circuit depth. In a linear quantum circuit, depth usually means the number of operation layers on the longest qubit path after scheduling as soon as possible (ASAP). Ordinary gates and ordinary instructions normally contribute one layer; `barrier` constrains reordering on the qubits it covers; a `CircuitGate` can be treated as an opaque operation by default. When a circuit contains control flow structures such as `if`, `while`, `for`, `switch`, `break` or `continue`, depth computation needs extra handling of branch and loop semantics: the default `recurse=False` does not expand these structures, and in that case `depth()` raises `CircuitError` if control flow exists in the circuit; passing `recurse=True` recursively expands branches and loop bodies and returns an estimated depth.


```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

assert len(circuit) == 2
first = circuit[0]
last = circuit[-1]
depth = circuit.depth()
```

---

## Standard gate convenience methods

`Circuit` provides convenience methods for a set of commonly used quantum gates:

### 1. Single-qubit fixed gates

| Method | Gate |
| --- | --- |
| `i(qubit)` | Identity |
| `h(qubit)` | Hadamard |
| `x(qubit)` | Pauli-X |
| `y(qubit)` | Pauli-Y |
| `z(qubit)` | Pauli-Z |
| `s(qubit)` | S |
| `sdg(qubit)` | S dagger |
| `t(qubit)` | T |
| `tdg(qubit)` | T dagger |
| `x2p(qubit)` | X half-pi positive |
| `x2m(qubit)` | X half-pi negative |
| `y2p(qubit)` | Y half-pi positive |
| `y2m(qubit)` | Y half-pi negative |

### 2. Parameterized single-qubit gates

| Method | Parameter | Description |
| --- | --- | --- |
| `rx(qubit, theta)` | `float \| Parameter` | Rotation around the X axis. |
| `ry(qubit, theta)` | `float \| Parameter` | Rotation around the Y axis. |
| `rz(qubit, theta)` | `float \| Parameter` | Rotation around the Z axis. |
| `phase(qubit, lambda)` | `float \| Parameter` | The phase gate `P(lambda)`. |
| `u(qubit, theta, phi, lambda)` | `float \| Parameter` | General single-qubit gate. |
| `xy(qubit, theta)` | `float \| Parameter` | The XY interaction family. |
| `xy2p(qubit, theta)` | `float \| Parameter` | Positive half-angle XY gate. |
| `xy2m(qubit, theta)` | `float \| Parameter` | Negative half-angle XY gate. |
| `rxy(qubit, theta, phi)` | `float \| Parameter` | Rotation around an arbitrary axis in the XY plane. |

### 3. Multi-qubit gates

| Method | Description |
| --- | --- |
| `cx(control, target)` | CNOT gate. |
| `cy(control, target)` | controlled-Y gate. |
| `cz(control, target)` | controlled-Z gate. |
| `swap(a, b)` | Exchange the states of two qubits. |
| `ccx(control1, control2, target)` | Toffoli gate. |
| `rxx(a, b, theta)` | `exp(-i theta XX / 2)`. |
| `ryy(a, b, theta)` | `exp(-i theta YY / 2)`. |
| `rzz(a, b, theta)` | `exp(-i theta ZZ / 2)`. |
| `rzx(a, b, theta)` | `exp(-i theta ZX / 2)`. |
| `crx(control, target, theta)` | controlled-RX gate. |
| `cry(control, target, theta)` | controlled-RY gate. |
| `crz(control, target, theta)` | controlled-RZ gate. |
| `fsim(a, b, theta, phi)` | fSim(theta, phi) gate. |

Example:

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 1)
circuit.rzz(1, 2, theta)
circuit.crx(0, 2, 0.25)
```

---

## Adding operation interfaces

Besides the convenience gate methods, `Circuit` also provides more general interfaces for adding operations, used to add explicitly constructed gate objects, composite gates or low-level operation objects.

| Method | Description |
| --- | --- |
| `append_gate(gate, qubits, label=None)` | Append a `StandardGate`. |
| `append_mc_gate(gate, qubits, label=None)` | Append an `MCGate`. |
| `append_unitary_gate(gate, qubits, params=None)` | Append a `UnitaryGate`. |
| `append_circuit_gate(gate, qubits, params=None)` | Append a `CircuitGate`. |
| `append(operation)` | Append a low-level `ValueOperation`. |

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import StandardGate

theta = Parameter("theta")

circuit = Circuit(1)
circuit.append_gate(StandardGate.RX(theta), [0], label="first-rx")
```

---

## Non-unitary instructions

A quantum circuit can contain not only ordinary quantum gates, but also instructions that are not ordinary unitary gates, such as measurement, reset, barrier and delay.

| Method | Description |
| --- | --- |
| `barrier(qubits)` | Insert a barrier, used to prevent the compiler from reordering operations on the related qubits across this boundary. |
| `reset(qubit)` | Reset a qubit. |
| `delay(qubit, duration)` | Insert idle waiting time on the given qubit. |
| `measure(qubit)` | Measure a single qubit and return a `Measurement`. |
| `measure_bits(qubits)` | Measure several qubits and return a multi-bit result. |
| `measure_into(qubit, target)` | Measure a single qubit and write the result into an existing classical variable. |
| `measure_bits_into(qubits, target)` | Measure several qubits and write the result into an existing classical variable. |

---

## Parameter binding

```python
assign_parameters(bindings: dict[str, float] | None = None) -> Circuit
```

`assign_parameters()` binds the symbolic parameters in a circuit to concrete numeric values and returns a new bound circuit. The method does not modify the original circuit, so it suits using one parameterized circuit as a template and repeatedly generating concrete circuits under different parameter values.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

bound = circuit.assign_parameters({"theta": 3.141592653589793})
assert bound.symbols == []
```

Note that bound values must be finite numbers. If a bound value is `nan`, `inf` or another invalid number, a `ParameterError` is usually raised. If only some symbols are bound, the unbound symbols remain in the returned new circuit.

---

## Inverse, decomposition and composition

| Method | Description |
| --- | --- |
| `inverse()` | Return a new circuit: the operation order is reversed and every invertible operation is inverted. |
| `decompose()` | Return a new circuit, recursively expanding composite gates represented by `CircuitGate`. |
| `to_gate(name)` | Wrap the current circuit as a reusable `CircuitGate`. |
| `compose(other, qubits=None)` | Append another circuit to the current circuit, with an optional qubit mapping. |

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

larger = Circuit(4)
larger.append_circuit_gate(bell_gate, [0, 1])
larger.append_circuit_gate(bell_gate, [2, 3])
```

The behavior of `compose(other, qubits=None)` is as follows:

- `qubits=None`: merge `other` into the current circuit by its logical qubit indices, adding new qubits to the current circuit when necessary;
- `qubits=[...]`: map `other.qubits` in order to the target qubits in the current circuit.

Unlike methods returning a new circuit such as `inverse()` and `decompose()`, `compose()` modifies the current circuit in place and returns `None`.

It is worth distinguishing that `compose()` appends the operations of another circuit directly to the current circuit, whereas `to_gate()` preserves the module boundary of the sub-circuit.

---

## Matrix conversion

| Method | Description |
| --- | --- |
| `to_matrix(qubits_order=None)` | Return a numeric matrix, usually of type `numpy.ndarray[np.complex128]`. |
| `to_symbolic_matrix(qubits_order=None)` | Return a `SymbolicMatrix`, preserving symbolic parameters. |

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()
matrix_reversed = circuit.to_matrix([1, 0])
```
---

## Structured control flow entry points

`Circuit` provides two kinds of control flow construction:

- `if_()`, `if_else()`, `while_()`, `for_uint()`, `switch()`;
- building a `ClassicalControlOp` by hand and then calling `append_control()`.

In the callback bodies of `while_()` and `for_uint()`, `break_loop()` can be called to leave the current loop or `switch` branch early, or `continue_loop()` to skip the rest of the current iteration. These two may only appear inside the corresponding control flow body, otherwise `CircuitError` is raised.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(2)
circuit.h(0)

circuit.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(1),
)
```

---

## Complete example: parameterized sub-circuit reuse

The following example shows how to first construct a parameterized sub-circuit, wrap it as a `CircuitGate`, then reuse it several times in a larger circuit and bind different parameter values each time.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

layer = Circuit(2)
layer.rx(0, theta)
layer.cx(0, 1)
layer.rz(1, theta / 2)

layer_gate = layer.to_gate("ParamLayer")

model = Circuit(4)
model.append_circuit_gate(layer_gate, [0, 1], params=[Parameter("a")])
model.append_circuit_gate(layer_gate, [2, 3], params=[Parameter("b")])

bound = model.assign_parameters({"a": 0.1, "b": 0.2})
```

This example reflects a typical reuse pattern of `Circuit`: first construct a parameterizable circuit module, then wrap it as a composite gate through `to_gate()`, and finally call it several times in the main circuit and bind parameters separately.
