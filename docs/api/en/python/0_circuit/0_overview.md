# Quantum Circuit

`cqlib.circuit`

`cqlib.circuit` is the core module in Cqlib for constructing, representing and analyzing quantum circuits. It provides a unified interface ranging from basic quantum gates, parameterized circuits, composite gates, measurement and classical data to structured control flow and matrix conversion, and is the main entry point for quantum program development with Cqlib.

## Overview

`cqlib.circuit` mainly covers the following capabilities:

- **Basic circuit construction**: create a set of logical qubits and append operations such as quantum gates, measurement, reset and barrier in order.
- **Parameterized circuit modeling**: use `Parameter` to represent gate angles, global phase or other tunable parameters, suitable for scenarios such as VQE, QAOA, quantum machine learning and parameter sweeps.
- **Gate and instruction system**: describe standard gates, custom gates, multi-controlled gates and composite gates through objects such as `StandardGate`, `UnitaryGate`, `MCGate` and `CircuitGate`.
- **Operation-level intermediate representation**: use `Instruction`, `ValueInstruction` and `ValueOperation` to represent "what to do", "which qubits it acts on" and "which parameters it carries".
- **Classical data and control flow representation**: provide `ClassicalType`, `ClassicalVar`, `ClassicalExpr` and structured control flow interfaces for describing program structures such as conditional branches, loops and multi-way selection.
- **Matrix and symbolic matrix conversion**: support converting small-scale circuits made only of quantum gates into numeric matrices or symbolic matrices, for teaching, unit testing, gate validation and compiler rule checking.
- **Parameterized template circuits**: provide common circuit templates such as TwoLocal, feature map, QAOA and Pauli evolution through `cqlib.circuit.ansatz`.

---

## Common entry points

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.rz(1, theta)

print(circuit.used_symbols)          # ['theta']
bound = circuit.assign_parameters({"theta": 0.5})
print(bound.used_symbols)            # []
```

---

## Core concepts and terms

This page gives a brief description of terms commonly used in the `cqlib.circuit` documentation. For more detailed interface signatures, parameter constraints and examples, see the following API pages.

| Term | Description |
| --- | --- |
| **abstract circuit** | A quantum circuit oriented toward algorithm description, representing a quantum program with logical qubits and high-level operations, not directly bound to a specific hardware topology or native gate set. |
| **physical circuit** | A circuit that has completed layout, routing and gate set conversion according to target device constraints, usually containing only the physical qubit connections and native operations supported by the backend. |
| **qubit** | The basic logical unit of quantum information. Represented by `Qubit` in the Cqlib Python API, and may also be abbreviated as an integer index in most interfaces. |
| **quantum gate** | A reversible quantum operation that usually has a unitary matrix representation. Standard gates are represented by `StandardGate`; custom gates may be represented by `UnitaryGate`, `MCGate` or `CircuitGate`. |
| **instruction** | The type of an operation in a circuit; it may be a quantum gate, or `barrier`, `measure`, `reset`, `delay`, a classical data operation or a control flow structure. |
| **operation** | A concrete application of an instruction in a circuit, containing the instruction type, the qubits acted on, the parameter list and an optional label. The Python construction layer usually represents it with `ValueOperation`. |
| **parameter** | A construction-time symbolic expression, commonly used for gate angles, global phase and variational circuit parameters. Represented by `Parameter`, and can later be bound to a numeric value through `assign_parameters()`. |
| **measurement** | A computational basis measurement operation, which produces a classical result. Measurement is a non-unitary operation and cannot directly participate in ordinary unitary matrix conversion. |
| **classical data** | Classical-side objects used in a circuit to express conditions, variables and measurement results, such as `ClassicalType`, `ClassicalVar`, `ClassicalValue` and `ClassicalExpr`. |
| **control flow** | The mechanism for organizing circuit structure based on classical expressions, such as `if_`, `while_`, `for_uint` and `switch`. Whether a specific backend supports the related semantics needs to be confirmed together with the compilation and execution flow. |
| **global phase** | The overall phase factor of a circuit. It usually does not affect individual measurement probabilities, but is still meaningful in circuit composition, controlled operations, matrix equivalence checking and compiler rewriting. |
| **Ansatz** | A parameterized circuit template, commonly used in variational algorithms and quantum machine learning. `cqlib.circuit.ansatz` provides several built-in templates. |

---

## `cqlib.circuit` API Overview

### 1. Core circuit container

| Object | Page | Description |
| --- | --- | --- |
| `Circuit` | [Circuit](1_circuit.md) | The main quantum circuit container, used to create a set of qubits, append quantum gates and non-unitary instructions, bind parameters, compose circuits, decompose composite gates, construct control flow and generate matrices. |

### 2. Qubits and parameters

| Object | Page | Description |
| --- | --- | --- |
| `Qubit` | [Qubit](2_qubit.md) | A lightweight logical qubit handle that holds a non-negative integer index and is comparable and hashable. |
| `Parameter` | [Parameter](3_parameter.md) | A symbolic or numeric parameter expression, supporting parsing, evaluation, simplification, substitution, differentiation and conservative equivalence checks. |

### 3. Operations, instructions and gates

| Object | Page | Description |
| --- | --- | --- |
| `Instruction` | [Operation / Instruction](4_operation_instruction.md) | A storage-layer instruction, used to represent operation types such as standard gates, multi-controlled gates, custom unitary gates, sub-circuit gates, directives and delay. |
| `ValueInstruction` | [Operation / Instruction](4_operation_instruction.md) | A construction-layer instruction, which can wrap an ordinary `Instruction` or a control flow operation. |
| `ValueOperation` | [Operation / Instruction](4_operation_instruction.md) | A self-contained operation, containing an instruction, qubits, parameters and an optional label; suitable for serialization, importers and compiler output. |
| `Directive` | [Operation / Instruction](4_operation_instruction.md) | A non-unitary directive, including `barrier`, `measure` and `reset`. |

### 4. Gate definitions

| Object | Page | Description |
| --- | --- | --- |
| `StandardGate` | [StandardGate](5_gates_standard.md) | The set of native standard gates in Cqlib, including Pauli, Clifford, rotation gates, controlled gates, two-body interaction gates and fSim gates. |
| `UnitaryGate` | [UnitaryGate](6_gates_unitary.md) | A user-defined unitary gate, which can be defined by a numeric matrix, a symbolic matrix or a frozen circuit. |
| `MCGate` | [MCGate](7_gates_mc_gate.md) | A multi-controlled standard gate, which can add any number of control qubits on top of a `StandardGate`. |
| `FrozenCircuit` | [CircuitGate / FrozenCircuit](8_gates_circuit_gate.md) | An immutable circuit snapshot, usually used for composite gate definitions. |
| `CircuitGate` | [CircuitGate / FrozenCircuit](8_gates_circuit_gate.md) | A composite gate defined by a `FrozenCircuit`, which can be reused as a single gate in other circuits. |

### 5. Built-in special instructions

| Instruction | Construction | Description |
| --- | --- | --- |
| `barrier` | `Circuit.barrier(qubits)` or `Directive.barrier()` | Insert a compilation boundary that prevents operations on the related qubits from being reordered across it. |
| `measure` | `Circuit.measure(qubit)`, `Circuit.measure_bits(qubits)` | Perform a computational basis measurement, producing a `Measurement` receipt and a classical result. |
| `reset` | `Circuit.reset(qubit)` or `Directive.reset()` | Reset a qubit to `\|0>`, which usually destroys the existing coherence. |
| `delay` | `Circuit.delay(qubit, duration)` | Insert idle time or a scheduling delay on the given qubit. |
| `store` | `Circuit.store(target, value)` | Write a classical expression into a classical variable. |

### 6. Classical data and control flow

| Object | Page | Description |
| --- | --- | --- |
| `CircuitId` | [Classical / Control Flow](9_classical_control_flow.md) | The identity of a circuit-local classical handle, used to prevent classical variables and values from being misused across circuits. |
| `ClassicalType` | [Classical / Control Flow](9_classical_control_flow.md) | Classical data types, including bit, bool, uint and bit vector. |
| `ClassicalVar` | [Classical / Control Flow](9_classical_control_flow.md) | A mutable classical storage handle, created by `Circuit.var()`. |
| `ClassicalValue` | [Classical / Control Flow](9_classical_control_flow.md) | An immutable classical value, usually produced by measurement. |
| `Measurement` | [Classical / Control Flow](9_classical_control_flow.md) | A measurement receipt, containing the measured values and the order of the measured qubits. |
| `ClassicalExpr` | [Classical / Control Flow](9_classical_control_flow.md) | A typed classical expression AST, used for conditions, comparisons, bit extraction and expression composition. |
| `ClassicalControlOp` | [Classical / Control Flow](9_classical_control_flow.md) | The structured control flow IR, including `if`, `while`, `for`, `switch`, `break` and `continue`. |
| `ValueControlBody` | [Classical / Control Flow](9_classical_control_flow.md) | A construction-layer control flow body, containing several `ValueOperation`s. |
| `ValueSwitchCase` | [Classical / Control Flow](9_classical_control_flow.md) | A construction-layer `switch` case, containing a match value and a branch body. |

### 7. Matrix and symbolic matrix utilities

| Object | Page | Description |
| --- | --- | --- |
| `circuit_to_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | A functional interface for computing the dense numeric matrix of a circuit. |
| `Circuit.to_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | A method-style interface, equivalent to `circuit_to_matrix()`. |
| `Circuit.to_symbolic_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | Generate a symbolic matrix that preserves `Parameter` expressions. |
| `SymbolicComplex` | [SymbolicMatrix](10_symbolic_matrix.md) | A symbolic complex number whose real and imaginary parts are both `Parameter`. |
| `SymbolicMatrix` | [SymbolicMatrix](10_symbolic_matrix.md) | A dense symbolic matrix, suitable for small-scale circuit analysis and custom symbolic gates. |

### 8. Circuit templates and ansatz

| Object | Page | Description |
| --- | --- | --- |
| `EntanglementTopology` | [Ansatz](11_ansatz.md) | Entanglement topology, including linear, circular, full and custom. |
| `TwoLocal` | [Ansatz](11_ansatz.md) | A hardware-friendly ansatz alternating rotation layers and entanglement layers. |
| `AngleEncoding`, `BasisEncoding` | [Ansatz](11_ansatz.md) | Basic data encoding circuits. |
| `ZFeatureMap`, `IQPFeatureMap`, `ZZFeatureMap`, `PauliFeatureMap` | [Ansatz](11_ansatz.md) | Quantum machine learning feature map templates. |
| `BasicEntanglerLayers`, `StronglyEntanglingLayers` | [Ansatz](11_ansatz.md) | Common layered trainable circuit templates. |
| `QAOAAnsatz` | [Ansatz](11_ansatz.md) | The alternating QAOA cost/mixer structure. |
| `EvolutionStrategy`, `EvolutionInfo`, `PauliEvolutionAnsatz` | [Ansatz](11_ansatz.md) | Hamiltonian time evolution circuit templates. |
| `real_amplitudes`, `efficient_su2`, `zz_feature_map`, `pauli_feature_map` | [Ansatz](11_ansatz.md) | Convenience constructors for common templates. |

---

## Circuit representation

`Circuit` internally records the set of qubits, parameter information, classical data and the operation sequence at the same time.

```text
Circuit
├── id: CircuitId
├── qubits: list[Qubit]
├── parameters: list[Parameter]
├── symbols: list[str]
├── global_phase: Parameter
├── classical_vars: list[ClassicalType]
├── classical_values: list[ClassicalType]
└── operations: list[ValueOperation]
    ├── instruction: ValueInstruction
    │   ├── Instruction(...)
    │   └── ClassicalControlOp(...)
    ├── qubits: list[Qubit]
    ├── params: list[float | Parameter]
    └── label: str | None
```

### 1. Quantum data

Quantum data is identified by `Qubit`. `Circuit(3)` creates the logical qubits `0`, `1` and `2`; `Circuit([0, 2, 4])` creates sparse logical indices.

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])
assert circuit.num_qubits == 2
```

### 2. Operations and instructions

A circuit operation is determined jointly by "what to do" and "where it acts". `Instruction` describes the operation type, for example an `H` gate, an `RZ` gate, a measurement or control flow; `ValueOperation` further binds the qubits acted on, the parameters and an optional label.

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import ValueOperation
from cqlib.circuit.gates import StandardGate

op = ValueOperation.from_standard_gate(StandardGate.H, [Qubit(0)])
circuit = Circuit.from_operations([Qubit(0)], [op])
```

This layered representation suits import and export, deserialization, compiler pass output and low-level testing. For ordinary users, directly using convenience methods such as `Circuit.h()` and `Circuit.cx()` is usually simpler.

### 3. Parameters

`Parameter` is used to construct parameterized circuits. A circuit automatically collects the free symbols that appear in it, for later numeric binding through `assign_parameters()`.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
circuit = Circuit(1)
circuit.rx(0, theta)

assert circuit.symbols == ["theta"]
bound = circuit.assign_parameters({"theta": 0.5})
```

### 4. Classical data and control flow

Classical data is represented by objects such as `ClassicalType`, `ClassicalVar`, `ClassicalValue` and `ClassicalExpr`.

```python
from cqlib import Circuit

circuit = Circuit(2)
measurement = circuit.measure(0)

circuit.if_(
    measurement.expr().to_bool(),
    lambda body: body.x(1),
)
```

---

## Quick examples

### 1. Bell state preparation

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()
```

This example creates a two-qubit Bell state circuit and computes its small-scale matrix representation.

### 2. Parameterized rotation layer

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

circuit = Circuit(1)
circuit.rx(0, theta)
circuit.rz(0, theta + phi / 2)

bound = circuit.assign_parameters({"theta": 0.1, "phi": 0.2})
```

This example shows how to use `Parameter` to construct a parameterized circuit template that can be bound repeatedly.

### 3. Reusable sub-circuit gate

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(2)
block.rx(0, theta)
block.cx(0, 1)

block_gate = block.to_gate("Block")

main = Circuit(4)
main.append_circuit_gate(block_gate, [0, 1], params=[Parameter("a")])
main.append_circuit_gate(block_gate, [2, 3], params=[Parameter("b")])
```

This example wraps a parameterized sub-circuit as a `CircuitGate` and reuses it in a larger circuit.

### 4. Conditional structure based on measurement results

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
result = circuit.measure(0)

circuit.if_(
    result.expr().to_bool(),
    lambda body: body.x(1),
)
```

This example shows how to construct a conditional branch based on a measurement result. Whether an actual backend supports such a structure needs to be confirmed together with the target device and the compilation flow.

---

## Validation and error handling

High-level `Circuit` methods check common errors as early as possible, for example a nonexistent qubit, duplicate qubits, a mismatch in the number of qubits a gate acts on, or a mismatch in the number of parameters. For circuits imported from an external IR, generated automatically by a program or assembled by hand, explicitly call:

```python
circuit.validate()
```

Common exceptions include:

| Exception | When it occurs |
| --- | --- |
| `CircuitError` | The circuit structure or an operation is invalid, for example a nonexistent qubit, a wrong gate target, computing a matrix for a non-unitary operation, or a control flow scope error. |
| `ParameterError` | Parsing, binding, simplification or evaluation of a parameter expression failed. |
| `QubitError` | A qubit index is invalid, for example negative, beyond the internal range, or of an incorrect input type. |
| `CqlibError` | The base class of Cqlib-specific exceptions, which can be used to catch Cqlib-related exceptions uniformly. |
