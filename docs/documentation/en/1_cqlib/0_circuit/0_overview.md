# Quantum Circuit

`cqlib.circuit` is the basic module for describing quantum programs in Cqlib. It is responsible for expressing qubits, gate operations, parameter expressions, composite circuits, non-unitary instructions, measurement results and dynamic control flow driven by classical expressions. The later [IR conversion](../1_ir/0_overview.md), [QIS simulation](../3_qis/0_overview.md), [compilation and optimization](../4_compiler/0_overview.md), [device mapping](../2_device/0_overview.md) and [visualization](../5_visualization/0_overview.md) modules usually take `Circuit` as input or as an intermediate representation.

---

## Core abstractions

`cqlib.circuit` builds around `Circuit` a set of cooperating objects for describing the complete circuit structure, from basic quantum gates to dynamic circuit control flow.

| Abstraction | Role | Common entry points |
|---|---|---|
| `Qubit` | Qubit identifier, storing only a non-negative index | `Qubit(0)`|
| `Circuit` | Quantum circuit container, storing qubits, parameters, classical values and the operation sequence | `Circuit(...)` |
| `StandardGate` | Set of standard gates built into Cqlib, including Pauli, Clifford, rotation and two-qubit gates | `StandardGate.H`, `StandardGate.RX(theta)` |
| `MCGate` | Multi-controlled gate, promoting a standard gate into a gate with multiple control qubits | `MCGate(2, StandardGate.X())` |
| `UnitaryGate` | User-defined unitary gate, which can be defined by a numeric matrix, a symbolic matrix or an immutable sub-circuit | `UnitaryGate("Oracle", 2)` |
| `CircuitGate` | Composite gate obtained by converting a sub-circuit | `sub.to_gate("Block")` |
| `Directive` | Non-unitary instructions, such as barrier, measure and reset | `circuit.measure(0)` |
| `Parameter` | Symbolic parameter expression | `Parameter("theta")` |
| `ClassicalType` / `ClassicalExpr` | Classical types and classical expressions in dynamic circuits | `ClassicalType.bit()`, `m.expr()` |
| `ValueOperation` | Complete operation representation: instruction + qubits + parameters + optional label | `circuit.operations[0]` |

---

## Core functionality

### 1. Static circuits

A static circuit is the most basic way of using the module. When the circuit width is fixed, quantum gates and instructions are appended one after another in execution order.

```python
from cqlib import Circuit

c = Circuit(3)
c.h(0)
c.cx(0, 1)
c.rzz(1, 2, 0.25)
c.barrier([0, 1, 2])
c.reset(2)
```

Static circuits apply to scenarios such as algorithm prototype validation, compilation and optimization, QIS simulation and IR export. For a purely unitary circuit that contains no measurement, reset or control flow, the circuit can be further converted into a matrix representation for small-scale circuit validation.

### 2. Parameterized circuits

Parameterized circuits use `Parameter` as an angle or expression placeholder, and are commonly used in scenarios such as VQE, QAOA, quantum machine learning, parameter sweeps and gradient computation.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(2)
c.rx(0, theta)
c.ry(1, phi)
c.cx(0, 1)

print(c.symbols)  # ['theta', 'phi']

bound = c.assign_parameters({"theta": 0.1, "phi": 0.2})
print(bound.to_matrix())
```

Note that `assign_parameters()` returns a new bound circuit and does not modify the original parameterized circuit template. The same parameterized circuit can therefore be reused many times, for sweeps and optimization at different parameter points.

### 3. Sub-circuits and composite gates

Real quantum algorithms are usually composed of several reusable circuit modules. Cqlib supports converting a `Circuit` into a `CircuitGate`, which can then be appended to other circuits like an ordinary quantum gate.

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

main = Circuit(4)
main.append_circuit_gate(bell_gate, [0, 1])
main.append_circuit_gate(bell_gate, [2, 3])

flat = main.decompose()
```

`CircuitGate` is suitable for expressing reusable modules in an algorithm, such as entanglement blocks, feature-mapping blocks and oracle subroutines. In addition, `decompose()` can expand a composite gate into basic operations, which facilitates later matrix validation, compilation and optimization or IR export.

### 4. Custom gates and multi-controlled gates

When the built-in gate set is not enough to describe a certain algorithm unit, `UnitaryGate` can be used to define a custom unitary gate, and `MCGate` can be used to construct a multi-controlled gate.

```python
import numpy as np
from cqlib import Circuit
from cqlib.circuit import MCGate, StandardGate, UnitaryGate

c = Circuit(3)

controlled_h = MCGate(2, StandardGate.H())
c.append_mc_gate(controlled_h, [0, 1, 2])

custom_x = UnitaryGate("CustomX", 1).with_matrix([[0, 1], [1, 0]])
c.append_unitary_gate(custom_x, [2])
```

Multi-controlled gates and custom gates are commonly used in algorithm libraries, oracle construction, controlled subroutines and hardware-specific instruction modeling. Note that when a custom unitary gate is used, the matrix dimension must match the number of qubits it acts on, and the matrix must satisfy the unitarity requirement.

### 5. Dynamic circuits

Dynamic circuits allow mid-circuit measurement during circuit execution and control subsequent operations according to measurement results or classical variables. Here, Cqlib describes such circuits through `ClassicalType`, `ClassicalExpr` and structured control flow interfaces.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)
measurement = c.measure(0)

condition = measurement.expr().to_bool()
c.if_(condition, lambda body: body.x(0))

flag = c.var(ClassicalType.bool())
c.store(flag, ClassicalExpr.bool_literal(True))
c.while_(flag.expr(), lambda body: body.break_loop())

c.validate()
```

Note that a dynamic circuit usually cannot be represented as a single unitary matrix, so `to_matrix()` cannot be used directly for whole-circuit matrix conversion. For dynamic circuits, structured validation, IR conversion or a backend flow that supports dynamic execution semantics should be preferred.

### 6. Circuit analysis and transformation

After the circuit has been constructed, operations such as parameter binding, inversion, decomposition, numeric matrix conversion or symbolic matrix conversion can be performed further.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
c = Circuit(1)
c.rx(0, theta)

inverse = c.inverse()
symbolic = c.to_symbolic_matrix()
numeric = c.assign_parameters({"theta": 0.3}).to_matrix()
```
---

## Next steps

- [Quantum gates and instructions](1_gates.md): learn about built-in gates, custom gates, composite gates and non-unitary instructions.
- [Circuit structure and construction](2_structures.md): understand the lifecycle, indexing, composition and operation representation of `Circuit`.
- [Parameter system](3_parameters.md): learn about parameter expressions, parameter binding, expression simplification, symbolic differentiation and symbolic matrices.
