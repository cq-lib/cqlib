# StandardGate

`cqlib.circuit.gates.StandardGate`  

```python
from cqlib import Parameter
from cqlib.circuit.gates import StandardGate
```

`StandardGate` represents the set of standard quantum gates built into Cqlib, used to describe common single-qubit gates, multi-qubit gates, parameterized rotation gates, controlled gates, two-body Pauli interaction gates and some hardware-related gates. It is one of the basic gate types in `Circuit` construction, gate-level analysis, matrix computation, compiler transformation and IR export.

---

## Basic operations

A standard gate can be appended explicitly to a circuit through `Circuit.append_gate()`.

```python
from cqlib import Circuit, Parameter
from cqlib.circuit.gates import StandardGate

theta = Parameter("theta")

circuit = Circuit(2)
circuit.append_gate(StandardGate.H, [0])
circuit.append_gate(StandardGate.RX(theta), [0])
circuit.append_gate(StandardGate.CX, [0, 1])
```

In the example above, `StandardGate.H` and `StandardGate.CX` are fixed gates without parameters; `StandardGate.RX(theta)` is a parameterized gate instance carrying the parameter `theta`. When appending a gate, the `qubits` argument specifies the qubits the gate acts on, and the number of qubits must agree with the gate's `num_qubits`.

---

## Standard gate categories

The standard gates built into Cqlib can be categorized by the number of qubits they act on and by their parameter form:

### 1. Single-qubit fixed gates

Single-qubit fixed gates act on one qubit and require no angle parameter when used.

| Gate | Description |
| --- | --- |
| `StandardGate.I` | Identity gate, which leaves the quantum state unchanged. |
| `StandardGate.H` | Hadamard gate, used to construct superposition states. |
| `StandardGate.X` | Pauli-X gate, equivalent to a bit flip. |
| `StandardGate.Y` | Pauli-Y gate. |
| `StandardGate.Z` | Pauli-Z gate, used for a phase flip. |
| `StandardGate.S` | S phase gate, equivalent to `sqrt(Z)`. |
| `StandardGate.SDG` | The inverse gate of `S`. |
| `StandardGate.T` | T phase gate, corresponding to a `pi/4` phase. |
| `StandardGate.TDG` | The inverse gate of `T`. |
| `StandardGate.X2P` | Positive half-angle gate in the X direction. |
| `StandardGate.X2M` | Negative half-angle gate in the X direction. |
| `StandardGate.Y2P` | Positive half-angle gate in the Y direction. |
| `StandardGate.Y2M` | Negative half-angle gate in the Y direction. |

### 2. Parameterized single-qubit gates

Parameterized single-qubit gates control their effect through one or more angle parameters.

| Gate | Number of parameters | Description |
| --- | --- | --- |
| `StandardGate.RX(theta)` | 1 | X-axis rotation gate, usually expressed as `exp(-i theta X / 2)`. |
| `StandardGate.RY(theta)` | 1 | Y-axis rotation gate, usually expressed as `exp(-i theta Y / 2)`. |
| `StandardGate.RZ(theta)` | 1 | Z-axis rotation gate, usually expressed as `exp(-i theta Z / 2)`. |
| `StandardGate.RXY(theta, phi)` | 2 | Rotation gate around an arbitrary axis in the XY plane. |
| `StandardGate.U(theta, phi, lambda_)` | 3 | General single-qubit gate. |
| `StandardGate.Phase(lambda_)` | 1 | The phase gate `P(lambda)`. |
| `StandardGate.XY(theta)` | 1 | The gate family related to XY interaction. |
| `StandardGate.XY2P(theta)` | 1 | Positive half-angle XY gate. |
| `StandardGate.XY2M(theta)` | 1 | Negative half-angle XY gate. |

A parameter can be an ordinary number or a `Parameter` expression. When a symbolic parameter is used, the gate can be part of a parameterized circuit and bound later through `Circuit.assign_parameters()`.

### 3. Multi-qubit gates

Multi-qubit gates describe interactions between qubits and are an important foundation for constructing entangled states, controlled logic and the core circuits of quantum algorithms.

| Gate | Number of parameters | Description |
| --- | --- | --- |
| `StandardGate.CX` | 0 | controlled-X, that is, the CNOT gate. |
| `StandardGate.CY` | 0 | controlled-Y gate. |
| `StandardGate.CZ` | 0 | controlled-Z gate. |
| `StandardGate.SWAP` | 0 | Exchange the states of two qubits. |
| `StandardGate.CCX` | 0 | Toffoli gate, that is, the doubly controlled X gate. |
| `StandardGate.RXX(theta)` | 1 | XX rotation gate. |
| `StandardGate.RYY(theta)` | 1 | YY rotation gate. |
| `StandardGate.RZZ(theta)` | 1 | ZZ rotation gate. |
| `StandardGate.RZX(theta)` | 1 | ZX rotation gate. |
| `StandardGate.CRX(theta)` | 1 | controlled-RX gate. |
| `StandardGate.CRY(theta)` | 1 | controlled-RY gate. |
| `StandardGate.CRZ(theta)` | 1 | controlled-RZ gate. |
| `StandardGate.FSIM(theta, phi)` | 2 | fSim gate, common in some superconducting quantum computing models. |

For controlled gates, calling `append_gate()` usually passes qubits in the order of control qubit first and target qubit last. For example, `StandardGate.CX` should act on `[control, target]`.

### 4. Global phase gate

| Gate | Number of parameters | Description |
| --- | --- | --- |
| `StandardGate.GPhase(lambda_)` | 1 | Zero-qubit global phase marker. |

---

## Attributes

`StandardGate` provides a set of attributes for querying basic structural information about a gate. These attributes are commonly used for circuit validation, compiler rule matching, gate set coverage checks and document generation.

| Attribute | Type | Description |
| --- | --- | --- |
| `num_qubits` | `int` | The total number of qubits the gate acts on. |
| `num_ctrl_qubits` | `int` | The number of control qubits built into the gate. |
| `num_params` | `int` | The number of parameters the gate requires. |
| `params` | `list[Parameter]` | The parameter list already bound on the current gate instance. |

```python
from cqlib.circuit.gates import StandardGate

assert StandardGate.H.num_qubits == 1
assert StandardGate.CX.num_ctrl_qubits == 1
assert StandardGate.RX.num_params == 1

gate = StandardGate.RZ(0.5)
assert len(gate.params) == 1
```

---

## Lookup and enumeration

```python
StandardGate.all() -> list[StandardGate]
StandardGate.from_name(name: str) -> StandardGate
```

`all()` returns all built-in standard gate definitions in the standard gate enumeration order; every item in the list is a gate definition with no bound parameters.

`from_name(name)` looks up a standard gate definition by name; matching is case-insensitive; raises `ValueError` when the name matches no standard gate.

```python
from cqlib.circuit.gates import StandardGate

assert len(StandardGate.all()) == 36

assert StandardGate.from_name("H") == StandardGate.H
assert StandardGate.from_name("cx") == StandardGate.CX
```

---

## Parameter binding

A parameterized standard gate can produce a gate instance with parameters by calling the gate factory and passing in the parameters.

```python
gate(*args: float | Parameter) -> StandardGate
```

```python
from cqlib import Parameter
from cqlib.circuit.gates import StandardGate

theta = Parameter("theta")

rx = StandardGate.RX(theta)
u = StandardGate.U(theta, 0.1, 0.2)
```

---

## Matrix computation

```python
matrix(params: list[float] | None = None) -> np.ndarray[np.complex128]
```

`matrix()` returns the local numeric unitary matrix of a standard gate, that is, the matrix of the gate itself on the space of the qubits it acts on.

For a fixed gate, `matrix()` can be called directly. For a parameterized gate, there are two common ways:

- if the gate instance has already bound evaluable constant parameters, `matrix()` can be called directly;
- if the gate instance or gate definition contains symbolic parameters, concrete numeric parameters need to be passed in `matrix(params)`, or parameter binding needs to be completed in a circuit before matrix conversion.

The length of the `params` passed in must agree with the gate's `num_params`, otherwise `CircuitError` is raised; if bound parameters cannot be evaluated to constants, `ParameterError` is raised. Calling `matrix()` directly on a parameterized gate with no parameters bound likewise raises `CircuitError`.

```python
import numpy as np
from cqlib.circuit.gates import StandardGate

h = StandardGate.H.matrix()
rx = StandardGate.RX.matrix([np.pi / 2])

assert h.shape == (2, 2)
assert rx.shape == (2, 2)
```

---

## Inverse gate

```python
inverse() -> StandardGate
```

`inverse()` returns the inverse gate of the current standard gate. For self-inverse gates, for example `H`, `X`, `Y`, `Z`, `CX` and `CZ`, the inverse gate is usually the gate itself; for phase gates, `S` and `SDG`, and `T` and `TDG`, are mutually inverse; for parameterized rotation gates, the inverse gate usually corresponds to negating the angle, or an equivalent inverse gate is returned according to the gate definition.

```python
from cqlib.circuit.gates import StandardGate

assert StandardGate.H.inverse() == StandardGate.H
assert StandardGate.S.inverse() == StandardGate.SDG
assert StandardGate.T.inverse() == StandardGate.TDG
```

When a gate instance carries parameters, `inverse()` handles the parameters according to the inverse transformation rule of that gate: for example, `StandardGate.RZ(0.5).inverse()` returns `StandardGate.RZ(-0.5)`. This capability is commonly used to construct inverse circuits, uncompute structures, gate-level tests and compiler optimization validation.

Calling `inverse()` on a parameterized gate that has not bound all parameters raises `CircuitError`; if the gate itself is not invertible, `RuntimeError` is raised.

---

## Generating multi-controlled gates

```python
control(num_controls: int) -> MCGate
```

`control(num_controls)` adds the given number of control qubits on top of an existing standard gate and returns an `MCGate`. This interface suits constructing multi-controlled `X` gates, multi-controlled phase gates, multi-controlled rotation gates and the controlled operations commonly found in `oracle`.

```python
from cqlib.circuit.gates import StandardGate

mcx = StandardGate.X.control(3)

assert mcx.num_ctrl_qubits == 3
assert mcx.base_gate == StandardGate.X
```

When a multi-controlled gate is applied, qubits are usually passed in the order of control qubits first and target qubit last. For example, a triply controlled `X` gate needs four qubits in total, with the first three as control qubits and the last as the target qubit.

```python
from cqlib import Circuit

circuit = Circuit(4)
circuit.append_mc_gate(mcx, [0, 1, 2, 3])  # controls: 0, 1, 2; target: 3
```

---

## `StandardGate` and `Circuit` 

`Circuit` provides a large number of convenience gate methods, which in essence append the corresponding `StandardGate` to the circuit:

| `Circuit` method | Corresponding standard gate |
| --- | --- |
| `h(q)` | `StandardGate.H` |
| `x(q)` | `StandardGate.X` |
| `rx(q, theta)` | `StandardGate.RX(theta)` |
| `cx(control, target)` | `StandardGate.CX` |
| `rzz(a, b, theta)` | `StandardGate.RZZ(theta)` |
| `fsim(a, b, theta, phi)` | `StandardGate.FSIM(theta, phi)` |
