# Circuit analysis and transformation

After Circuit construction is complete, the circuit usually still needs to go through further analysis, validation and transformation processes. Depending on actual needs, the operation sequence can be inspected, gate types counted, symbolic parameters bound, composite gates expanded, an inverse circuit generated, the circuit converted into a matrix representation, or the circuit structure checked for consistency. These capabilities are an important foundation connecting circuit construction, algorithm validation, IR conversion, compilation and optimization, and backend execution.

This page gives a systematic introduction to the common interfaces related to circuit analysis and structure transformation in `cqlib.circuit`, and explains how a circuit is inspected, reused, transformed and validated after construction.

---

## Inspecting the operation sequence

`Circuit.operations` returns the ordered list of `ValueOperation` objects in a circuit. The list is kept in the order in which the operations were appended to the circuit.

Each `ValueOperation` can be further broken down into:
- `instruction` describes the gate, instruction or control flow structure corresponding to the operation;
- `qubits` denotes the qubits this operation acts on;
- `params` denotes the parameters carried by this operation;
- `label` records a debugging label or the transformation source.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rz(1, theta)

for index, op in enumerate(c.operations):
    if op.instruction.is_instruction:
        instruction = op.instruction.instruction
        name = instruction.name
    else:
        name = op.instruction.classical_control.kind

    qubits = [q.index for q in op.qubits]
    print(index, name, qubits, op.params, op.label)
```

Besides iterating over the complete operation list, a single operation can also be accessed by index. Both `circuit[i]` and `circuit.operation(i)` can be used to obtain the `ValueOperation` at a specified position.

```python
first = c[0]
second = c.operation(1)

print(first.instruction.instruction.name)   # H
print(second.instruction.instruction.name)  # CX
```

---

## Collecting circuit statistics

During circuit analysis and compilation, it is usually necessary to count the gate types, the number of operations or the occurrences of a particular instruction in a circuit. Such statistics can be used to assess the circuit size, compare the changes before and after optimization, or verify in tests whether a compilation pass produced the expected result.

In Cqlib, the required statistics logic can be implemented through `Circuit.operations`. Because `operations` keeps the complete operation sequence of a circuit, instruction names, acting qubits, parameters and control flow kinds can be extracted by iterating over the operation list.

```python
from collections import Counter
from cqlib import Circuit

c = Circuit(3)
c.h(0)
c.cx(0, 1)
c.cx(1, 2)
c.rzz(0, 2, 0.5)

names = []
for op in c.operations:
    if op.instruction.is_instruction:
        names.append(op.instruction.instruction.name)
    else:
        names.append(op.instruction.classical_control.kind)

print(Counter(names))
```

---

## Generating the inverse circuit

`inverse()` generates the inverse circuit corresponding to the current circuit. For a circuit that contains only invertible quantum gates, its inverse circuit represents the quantum evolution process opposite to that of the original circuit. Specifically, Cqlib reverses the operation order in the original circuit and replaces each operation with the corresponding inverse operation, thus obtaining a new `Circuit`.

```python
import numpy as np
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rz(1, 0.25)

inv = c.inverse()

product = inv.to_matrix() @ c.to_matrix()
print(np.allclose(product, np.eye(4), atol=1e-10))

print([op.instruction.instruction.name for op in c.operations])
print([op.instruction.instruction.name for op in inv.operations])
```

Note the following:

- `inverse()` does not modify the original circuit, but returns a new `Circuit`;
- `Barrier` does not change the quantum state and can be regarded as its own inverse, so it is kept;
- Non-invertible structures such as `Measure`, `Reset` and classical control flow cannot generate an ordinary inverse circuit.

```python
from cqlib import Circuit
from cqlib.circuit import CircuitError

c = Circuit(1)
c.measure(0)

try:
    c.inverse()
except CircuitError:
    print("measurement is not invertible")
```

---

## Decomposing composite gates

`decompose()` expands the composite gates in a circuit and returns a new `Circuit`. When a circuit contains a composite gate represented by `CircuitGate`, `decompose()` replaces it with the original operation sequence defined inside the composite gate. This interface is commonly used in matrix validation, IR export, compilation and backend adaptation flows that do not support composite gates.

```python
from cqlib import Circuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)
bell = sub.to_gate("Bell")

main = Circuit(2)
main.append_circuit_gate(bell, [0, 1])

print(len(main))  # 1

flat = main.decompose()
print(len(flat))  # 2
print([op.instruction.instruction.name for op in flat.operations])
```

For a parameterized composite gate, `decompose()` binds and expands according to the positional parameters passed in when the composite gate was appended.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

sub = Circuit(1)
sub.rx(0, theta)
block = sub.to_gate("RxBlock")

main = Circuit(1)
main.append_circuit_gate(block, [0], [0.75])

flat = main.decompose()
print(flat[0].params)  # [0.75]
```

---

## Matrix of a single operation

Cqlib supports computing the matrix of a single `ValueOperation`. `ValueOperation.matrix()` obtains the matrix representation corresponding to one concrete operation, and is suitable for gate-level testing, compilation rule validation, gate decomposition result checking and local operation analysis.

```python
import numpy as np
from cqlib import Qubit
from cqlib.circuit import StandardGate, ValueOperation

op = ValueOperation.from_standard_gate(StandardGate.X(), [Qubit(0)])
matrix = op.matrix()

print(np.allclose(matrix, np.array([[0, 1], [1, 0]], dtype=complex)))
```

Note that this interface applies only to ordinary unitary operations.

---

## Converting to a gate

In quantum algorithm development, a circuit that has already been constructed usually needs to serve as a reusable module invoked multiple times in other circuits. `to_gate(name)` encapsulates the current circuit as a `CircuitGate`, thus converting a sub-circuit into a composite gate with a name.

This approach is suitable for expressing structured algorithm modules, such as state preparation modules, oracles, ansatz blocks, entanglement layers or reused circuit fragments. Unlike appending a circuit directly with `compose()`, `to_gate()` keeps the module boundary, making the main circuit clearer in structure and also facilitating later composite gate decomposition, parameter binding, IR export and compilation.

```python
from cqlib import Circuit

sub = Circuit(1)
sub.h(0)

gate = sub.to_gate("HadamardBlock")

main = Circuit(1)
main.append_circuit_gate(gate, [0])
```

When a later analysis flow needs to inspect the internal structure of a composite gate, `decompose()` can be called to expand the composite gate into the original operation sequence.

```python
flat = main.decompose()
print(flat.to_matrix())
```

---

## Circuit composition and remapping

`compose()` appends the operations of one circuit to another circuit. Through this interface, several circuit fragments can be concatenated in order into one complete circuit, and qubits can also be remapped during appending so that the logical qubits in a sub-circuit act on specified qubits in the main circuit.

```python
from cqlib import Circuit

prefix = Circuit(3)
prefix.h(0)

block = Circuit(2)
block.cx(0, 1)

prefix.compose(block, [1, 2])

print([op.instruction.instruction.name for op in prefix.operations])
print([q.index for q in prefix[1].qubits])
```

---

## Structure validation

`validate()` checks the internal consistency of a circuit object and helps to detect potential structural problems as early as possible before later analysis, transformation, compilation or execution:
- For an ordinary static circuit, validation usually focuses on whether qubit references, operation objects and parameter structures are consistent;
- For a dynamic circuit that contains measurement, classical variables and control flow, validation further checks whether classical handles, measurement result references and control flow structures meet the circuit semantic requirements.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)
c.measure(0)
c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

c.validate()
```

---



## Next steps

- [Control flow](5_control_flow.md): use measurement results, classical variables and structured control flow to construct dynamic circuits.
- [Intermediate Representation](../1_ir/0_overview.md): understand the bidirectional conversion flow between Circuit and IR.
- [QCIS support](../1_ir/1_qcis.md): export a Cqlib circuit as QCIS instructions or load a circuit from a QCIS file.
