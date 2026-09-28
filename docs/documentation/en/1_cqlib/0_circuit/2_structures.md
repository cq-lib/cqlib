# Circuit structure and construction

`Circuit` is the core circuit container in the `cqlib.circuit` module, used to represent a complete quantum program. It records qubits and the circuit operations in order, and is also responsible for maintaining parameter expressions, the global phase, classical variables, measurement results and the classical handle namespace required by dynamic circuits. Therefore, `Circuit` is both the main entry point for constructing quantum circuits and an important data foundation for later processes such as IR conversion, compilation and optimization, device mapping and result analysis.

Structurally, a `Circuit` usually contains the following information:

* A qubit list, describing the logical qubits that can be operated on in the circuit;
* An operation list in append order, recording quantum gates, instructions and control flow structures;
* The free parameters in the circuit, supporting parameterized circuits, parameter binding and variational algorithms;
* The global phase, preserving the overall phase information of the circuit;
* The classical variables and classical values used in dynamic circuits, describing measurement results and classical control logic;
* The classical handle namespace identified by `CircuitId`, ensuring consistency of classical variables and measurement values within the circuit.

Together, these structures form the basic representation of a Cqlib quantum circuit, so that the circuit keeps a consistent data semantics across different stages such as construction, composition, parameter binding, matrix conversion, IR export, compilation and optimization and dynamic control flow analysis.


---

## Creating a circuit

`Circuit` supports several ways of creating a circuit. Either the number of qubits can be specified directly, or an explicit list of qubit indices or `Qubit` objects can be passed in. Different ways suit different modeling needs: when the qubit numbering is contiguous, passing an integer directly is the most concise; when a specific logical numbering needs to be preserved or aligned with the bit numbering in an external system, an integer index list or explicit `Qubit` objects can be used.

### 1. Using a qubit count

The most common way is to pass an integer `n` to `Circuit`. Cqlib then creates `n` logical qubits automatically and assigns indices in the order `0..n-1`.

```python
from cqlib import Circuit

c = Circuit(3)
print(c.num_qubits)             # 3
print([q.index for q in c.qubits])  # [0, 1, 2]
```

In the example above, `Circuit(3)` creates a circuit containing 3 qubits, with the corresponding logical qubit indices `0`, `1` and `2`. When gate operations are added later, these integer indices can be used directly to specify the operands of a gate, for example `c.h(0)` or `c.cx(0, 1)`.

In addition, `Circuit(0)` is also a legal construction, representing a zero-qubit circuit that contains no qubits and no operations. Zero-qubit circuits are usually used for boundary testing, recursive construction, as an initial placeholder when generating circuits programmatically, or to verify the behavior of an interface under empty input.

```python
from cqlib import Circuit

empty = Circuit(0)
print(empty.width)      # 0
print(len(empty))       # 0
print(empty.to_matrix())
```

### 2. Using an integer index list

When the logical qubit numbering of a circuit is not the contiguous `0..n-1`, or needs to stay consistent with the numbering in external data, a device mapping or an algorithm model, an integer index list can be passed directly to `Circuit`.

```python
from cqlib import Circuit

c = Circuit([2, 0, 5])
print([q.index for q in c.qubits])  # [2, 0, 5]

c.h(2)
c.cx(2, 5)
```

Note that Cqlib preserves the qubit order of the passed list. That order affects not only the value returned by `c.qubits`, but also the way qubits are arranged when a matrix is constructed by default. Unless `qubits_order` is specified explicitly during matrix conversion, Cqlib interprets each qubit in the order of `c.qubits` as stored inside the circuit.


### 3. Using `Qubit` objects

Besides integer indices, Cqlib also supports creating `Qubit` objects explicitly for circuit construction. `Qubit` is a lightweight handle representing a logical qubit in `Cqlib`, mainly used to wrap a non-negative integer index.

```python
from cqlib import Circuit, Qubit

q0 = Qubit(10)
q1 = Qubit(11)

c = Circuit([q0, q1])
c.h(q0)
c.cx(q0, q1)

print(q0.index)
print(q0 == Qubit(10))
```

---

## Adding qubits

In some scenarios, a smaller circuit may need to be constructed first, and the circuit width then extended according to later algorithm logic, sub-circuit composition or programmatic generation results. `add_qubits()` can then be used to add new qubits to the circuit.

`add_qubits()` adds new qubits to the current circuit while keeping the existing operation sequence. The new qubits do not affect the gate operations that already exist, but they extend the qubit list of the circuit and affect the scope of operations that can be added later as well as the dimension of the circuit matrix.

```python
from cqlib import Circuit

c = Circuit(1)
c.h(0)

c.add_qubits([2, 4])
print(c.num_qubits)                 # 3
print([q.index for q in c.qubits])  # [0, 2, 4]
print(len(c.operations))            # 1

c.cx(0, 2)
```

In the example above, the circuit initially contains only qubit `0`, and one `H` gate has already been added. After `add_qubits([2, 4])` is called, qubits with indices `2` and `4` are added to the circuit, the original `H` gate is still kept, and operations can continue to be added on the new qubits later.

Note the following when using `add_qubits()`:

- A new qubit must not duplicate an existing qubit in the circuit, otherwise a circuit structure error is raised;
- Existing operations are not rewritten or remapped; new qubits only affect later operations;
- The qubit order of a circuit affects the bit arrangement used when a matrix is constructed by default;
- New qubits expand the circuit width, so the matrix dimension grows accordingly during circuit matrix conversion.

---

## Querying circuit information

When constructing or debugging a quantum circuit, Cqlib supports inspecting basic structural information such as the number of qubits, the operations already added, the free parameters and the classical handle namespace. `Circuit` provides a set of common attributes that help to understand the current state of a circuit quickly and provide necessary information for later parameter binding, circuit composition, matrix conversion or IR export.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
c = Circuit(2)
c.rx(0, theta)
c.cx(0, 1)

print(c.id)              # the CircuitId handle identifier of the current circuit
print(c.num_qubits)      # 2
print(c.width)           # 2
print(c.qubits)          # [Qubit(0), Qubit(1)]
print(c.parameters)      # [theta]
print(c.symbols)         # ['theta']
print(c.operations)      # [ValueOperation(...), ...]
print(len(c))            # 2
```

Here, both `num_qubits` and `width` denote the number of qubits the circuit contains; `qubits` returns the list of qubits registered in the circuit; `operations` returns the operation list in append order; and `len(c)` returns the number of operations in the current circuit. For scenarios that involve dynamic circuit structures, `id` denotes the classical handle namespace of the current circuit, used to distinguish classical variables and measurement results in different circuits.

Note that `parameters` and `symbols` do not have exactly the same meaning. `parameters` returns the parameter expression objects registered in the circuit, while `symbols` returns the free symbol names contained in those expressions.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(1)
c.rz(0, 2 * theta + phi)

print([str(p) for p in c.parameters])  # ['phi + 2*theta']
print(c.symbols)                       # ['theta', 'phi']
```

---

## Operation sequence and indexing

`Circuit` maintains an ordered operation sequence in the order in which operations are appended to the circuit. The complete operation list can be viewed through the `operations` attribute, and the operation at a given position can be accessed with `operation(index)` or `circuit[index]`. This mechanism applies to circuit inspection, debugging, converter development and structural comparison before and after compilation.

```python
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)

first = c[0]
second = c.operation(1)

print(first.instruction.instruction.name)   # H
print(second.instruction.instruction.name)  # CX
print([q.index for q in second.qubits])     # [0, 1]
```

Each operation in a `Circuit` is usually represented as a `ValueOperation`, which is the complete operation object in a circuit and mainly contains the following fields:
- `instruction`: the `ValueInstruction` corresponding to the operation, which can represent an ordinary quantum gate, a non-unitary instruction or a classical control flow structure;
- `qubits`: the list of qubits this operation acts on;
- `params`: the parameter list carried by this operation, commonly used for parameterized rotation gates, custom gates or composite gates;
- `label`: optional label information, usually used for debugging, marking circuit layers, recording the transformation source or assisting later analysis.

---

## Adding operations

After a circuit is created, quantum gates, composite gates, non-unitary instructions or low-level operation objects can be appended to `Circuit` continuously. Cqlib provides several ways of appending operations to suit different usage needs.

### 1. Using convenience gate methods

Using the convenience gate methods provided by `Circuit` is the most common way of constructing a circuit. Such methods take qubit indices and necessary parameters as input directly, with clear semantics and concise code, and are suitable for algorithm prototypes and everyday quantum circuit development.

```python
from cqlib import Circuit

c = Circuit(2)
c.h(0)
c.cx(0, 1)
c.rzz(0, 1, 0.25)
```

### 2. Using explicit gate objects

When gate objects need to be stored, gate attributes checked, labels added, or gate operations generated in batches through program logic, a `gate` object can be constructed explicitly first and then appended to the circuit through the corresponding `append_*` method.

```python
from cqlib import Circuit
from cqlib.circuit import MCGate, StandardGate

c = Circuit(3)
c.append_gate(StandardGate.H(), [0])
c.append_gate(StandardGate.RZ(0.25), [1], label="rz-calibrated")

mcx = MCGate(2, StandardGate.X())
c.append_mc_gate(mcx, [0, 1, 2])
```

The commonly used explicit append methods are as follows:

| Method | Purpose |
|---|---|
| `append_gate(gate, qubits, label=None)` | Appends a `StandardGate` |
| `append_mc_gate(gate, qubits, label=None)` | Appends an `MCGate` |
| `append_unitary_gate(gate, qubits, params=None)` | Appends a `UnitaryGate` |
| `append_circuit_gate(gate, qubits, params=None)` | Appends a `CircuitGate` |

### 3. Using a low-level `ValueOperation`

In lower-level development scenarios, a complete `ValueOperation` can also be constructed and appended directly.

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation

operation = ValueOperation.from_standard_gate(
    StandardGate.RX(0.5),
    [Qubit(0)],
    label="manual-rx",
)

c = Circuit(1)
c.append(operation)
print(c[0].label)
```

---

## Rebuilding a circuit from operations

Besides the usual step-by-step construction, Cqlib also provides the `Circuit.from_operations()` interface, which rebuilds a circuit from an existing qubit list and operation sequence. This interface is a relatively low-level circuit construction entry point, usually used to restore a complete circuit from serialized data, IR conversion results, compiler intermediate results or test cases.

Unlike calling methods such as `h()`, `cx()` and `append_gate()` to append operations step by step, `from_operations()` applies to scenarios where the qubits required by the circuit and the `ValueOperation` operation list are already prepared. Cqlib regenerates a `Circuit` object from this information and preserves the original operation order.

```python
from cqlib import Circuit

source = Circuit(2)
source.h(0)
source.cx(0, 1)

restored = Circuit.from_operations(source.qubits, source.operations)

print(restored.num_qubits)
print([op.instruction.instruction.name for op in restored.operations])
```
---

## Composing circuits

When constructing a complex quantum program, several smaller circuits usually need to be composed into one complete circuit. `compose()` appends the operations of another circuit to the end of the current circuit, achieving sequential composition between circuit modules.

By default, `compose()` composes according to the qubit numbering in the two circuits; if a sub-circuit needs to be mapped to specific qubit positions in the main circuit, the position remapping relation can be specified explicitly through the `qubits` parameter.

```python
from cqlib import Circuit

main = Circuit(3)
main.h(0)

sub = Circuit(2)
sub.cx(0, 1)

main.compose(sub, [1, 2])

print([op.instruction.instruction.name for op in main.operations])
print([q.index for q in main[1].qubits])  # [1, 2]
```

Note that the following rules need to be followed when using `compose()`:

- If the qubits parameter is passed, its length must match other.num_qubits;
- The i-th qubit in other is mapped to the main-circuit qubit specified by qubits[i];
- The operation order after composition is "the operations already in the current circuit first, the operations of other afterwards";
  
---

## Sub-circuit encapsulation

When constructing complex quantum algorithms, many circuit fragments are reused multiple times, such as rotation layers, entanglement layers, oracles, ansatz blocks or specific algorithm submodules. For such reusable structures, Cqlib supports using `to_gate(name)` to encapsulate the current circuit as a `CircuitGate`, and then appending the resulting composite gate to other circuits.

The composite gate can be invoked multiple times at different qubit positions of the main circuit, and can be expanded back into the original circuit operations through `decompose()` when needed.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(1)
block.rx(0, theta)
block.rz(0, theta / 2)

gate = block.to_gate("RotationBlock")

main = Circuit(2)
main.append_circuit_gate(gate, [0], [0.2])
main.append_circuit_gate(gate, [1], [0.4])

flat = main.decompose()
print([op.instruction.instruction.name for op in flat.operations])
```

---

## Global phase

Every `Circuit` contains a `global_phase` attribute, used to record the overall global phase of the circuit. The global phase does not change the measurement probability distribution of the computational basis states, but it is part of the mathematical representation of a quantum circuit and is of great significance in matrix comparison, circuit equivalence determination, compilation rewriting and certain phase-sensitive algorithm analyses.

In Cqlib, `global_phase` is represented with `Parameter`, so it can be set to an ordinary numeric value, a symbolic parameter or a parameter expression.

```python
from cqlib import Circuit, Parameter

c = Circuit(1)
print(c.global_phase.is_zero())

c.set_global_phase(0.25)
print(c.global_phase.evaluate({}))  # 0.25

alpha = Parameter("alpha")
c.set_global_phase(alpha)
print(c.global_phase)
print(c.symbols)
```

---

## Next steps

- [Parameter system](3_parameters.md): learn about parameter expressions, parameter binding, expression simplification, symbolic differentiation and symbolic matrices.
- [Circuit analysis and transformation](4_circuit_analysis.md): use tools such as inversion, decomposition, matrix conversion and operation inspection.
- [Control flow](5_control_flow.md): use measurement results, classical variables and structured control flow to construct dynamic circuits.
