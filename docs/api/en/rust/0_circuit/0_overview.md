# Quantum Circuit

`cqlib_core::circuit`

`cqlib_core::circuit` is the quantum circuit intermediate representation module in the Cqlib Rust core. It provides type-safe circuit construction, qubit management, parameter expressions, gate definitions, classical data, structured control flow, control flow graphs, matrix conversion and parameterized circuit templates.

## Overview

`cqlib_core::circuit` mainly provides the following capabilities:

- **Quantum circuit construction**: create a set of logical qubits through `Circuit`, and append standard gates, custom gates, composite gates, measurement, reset, barrier and other operations.
- **Parameterized expressions**: represent gate angles, global phases and variational circuit parameters through `Parameter`, with support for expression parsing, evaluation, simplification, substitution, differentiation and equivalence checking.
- **Operation-level IR**: represent instructions and operations in a circuit through `Instruction`, `Operation`, `ValueInstruction` and `ValueOperation`, with the construction-layer IR separated from the storage-layer IR.
- **Gate definition system**: provide gate definition types such as `StandardGate`, `UnitaryGate`, `MCGate`, `CircuitGate` and `FrozenCircuit`, covering standard gates, custom unitary gates, multi-controlled gates and sub-circuit composite gates.
- **Classical data and control flow**: describe measurement results, classical variables and structured control flow through types such as `ClassicalType`, `ClassicalExpr` and `ClassicalControlOp`.
- **Control flow graph analysis**: provide a control flow graph view for compiler passes through types such as `CircuitCFG`, `BasicBlock` and `Terminator`, and provide a directed acyclic graph view of the circuit through `CircuitDag`.
- **Matrix and symbolic matrix conversion**: convert small purely quantum circuits into numeric matrices, or into symbolic matrices that retain parameters, with support for global phase equivalence checking.
- **Parameterized circuit templates**: provide common templates such as VQE, QAOA, feature map and Hamiltonian evolution through the `ansatz` module.

---

## Common entry points

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

assert_eq!(circuit.qubits().len(), 2);
assert_eq!(circuit.operations().len(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## API navigation

| Category | Page | Main objects | Description |
| --- | --- | --- | --- |
| Circuit container | [Circuit](1_circuit.md) | `Circuit` | Circuit construction, gate methods, parameter binding, inversion, decomposition, composition and matrix conversion. |
| Qubit | [Qubit](2_qubit.md) | `Qubit`, `QubitError` | `u32` logical qubit handle and checked integer conversion. |
| Parameter system | [Parameter](3_parameter.md) | `Parameter`, `ParameterError`, `EvalError` | Symbolic expressions, evaluation, differentiation, substitution and simplification. |
| Operation IR | [Operation / Instruction](4_operation_instruction.md) | `Instruction`, `Operation`, `ValueInstruction`, `ValueOperation` | Storage-layer IR and construction-layer IR. |
| Standard gates | [Standard Gates](5_gate_standard.md) | `StandardGate` | Native gate enum, gate metadata, matrices and inverse gates. |
| Custom unitary gates | [Unitary Gates](6_gate_unitary.md) | `UnitaryGate`, `UnitaryMatrix` | Numeric matrix gates, symbolic matrix gates and circuit-backed gates. |
| Multi-controlled gates | [Multi-Controlled Gates](7_gate_mc_gate.md) | `MCGate` | Add control qubits in front of a standard gate to express multi-controlled gate semantics. |
| Sub-circuit gates | [Circuit Gates](8_gate_circuit_gate.md) | `FrozenCircuit`, `CircuitGate` | Freeze a circuit and wrap it as a reusable composite gate. |
| Classical data and control flow | [Classical / Control Flow](9_classical_control_flow.md) | `ClassicalType`, `ClassicalExpr`, `ClassicalControlOp` | Measurement, classical data, expressions and structured control flow. |
| Symbolic matrix | [Symbolic Matrix](10_symbolic_matrix.md) | `SymbolicComplex`, `SymbolicMatrix` | Dense symbolic matrices that retain `Parameter`, and equivalence checking. |
| Ansatz | [Ansatz](11_ansatz.md) | `Ansatz`, `TwoLocal`, `QAOAAnsatz`, `PauliEvolutionAnsatz` | Parameterized circuit templates. |
| Control flow graph | [CFG](12_cfg.md) | `CircuitCFG`, `BasicBlock`, `Terminator` | Circuit control flow graph view, analysis and reconstruction. |
| Circuit DAG | [Circuit DAG](14_circuit_dag.md) | `CircuitDag`, `DagNode`, `DagWire` | Circuit directed acyclic graph view, dependency queries and graph structure reconstruction. |
| Matrix conversion | [Circuit To Matrix](13_circuit_to_matrix.md) | `circuit_to_matrix`, `Circuit::to_matrix` | Numeric matrix conversion, qubit ordering and global phase. |

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Abstract circuit** | A quantum circuit oriented to algorithm description; it represents a quantum program with logical qubits and high-level operations, and is not bound directly to a specific hardware topology or native gate set. |
| **Physical circuit** | A circuit that has completed layout, routing and gate set conversion according to the target constraints, and usually contains only the physical qubit connections and native operations supported by the backend. |
| **Qubit** | The basic logical unit of quantum information, represented by `Qubit`, with a `u32` handle internally. |
| **Instruction** | The type of an operation in a circuit, represented by `Instruction`; it can be a quantum gate, or `Barrier`, `Measure`, `Reset`, `Delay`, a classical data operation or a control flow structure. |
| **Operation** | A concrete application of an instruction in a circuit, including the instruction type, the acted-on qubits, the parameter list and an optional label. |
| **Gate definition** | A gate description separated from a concrete application, represented by types such as `StandardGate`, `UnitaryGate`, `MCGate` and `CircuitGate`. |
| **Parameter** | A construction-time symbolic expression, represented by `Parameter`, which can later be bound to a numeric value through `assign_parameters()`. |
| **Global phase** | The overall phase factor of a circuit. It usually does not affect individual measurement probabilities, but it still matters in circuit composition, controlled operations, matrix equivalence checking and compilation rewriting. |
| **Ansatz** | A parameterized circuit template provided by the `ansatz` module, commonly used in variational algorithms and quantum machine learning. |

### Comparison of the two IR layers

The circuit IR in the Rust core is divided into a construction layer and a storage layer. Understanding this distinction matters when writing importers, serialization tools and compiler passes.

| Layer | Operation type | Instruction type | Parameter type | Applicable scenarios |
| --- | --- | --- | --- | --- |
| Construction-layer IR | `ValueOperation` | `ValueInstruction` | `ParameterValue` | External construction, serialization, importers and compiler pass output. |
| Storage-layer IR | `Operation` | `Instruction` | `CircuitParam` | Compact internal storage inside `Circuit`, parameter residency and runtime validation. |

The construction-layer IR is self-contained: parameters are kept as `ParameterValue` and can be passed across circuits or written to external formats directly. The storage-layer IR is more compact, and a parameter may reference the internal parameter table of the owning `Circuit` in the form of `CircuitParam::Index`, so it must be interpreted together with the concrete circuit context.

The typical conversion relationships are as follows:

```text
ValueOperation
    └── append / from_operations
          └── Operation stored in Circuit

Operation
    └── Circuit::index(i)
          └── ValueOperation
```

---

## Minimal example: static circuit

The following example creates a two-qubit Bell circuit and adds an `H` gate and a `CX` gate in turn.

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

assert_eq!(circuit.operations().len(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`Circuit::new(2)` creates the logical qubits `Qubit(0)` and `Qubit(1)`. Most high-level gate methods return `Result<(), CircuitError>`, so that errors such as a non-existent qubit, a parameter count mismatch or a gate arity mismatch are exposed as early as possible when the operation is appended.

---

## Minimal example: parameterized circuit

A parameterized circuit allows gate angles to be expressed as symbolic expressions. After construction, concrete values can be bound through `assign_parameters()`, and matrix conversion or subsequent compilation can continue.

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), theta)?;

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::FRAC_PI_2);

let bound = circuit.assign_parameters(&Some(bindings))?;
let matrix = bound.to_matrix(None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`assign_parameters()` returns a new circuit object, and the original parameterized circuit can still be reused as a template. For VQE, QAOA and parameter sweep tasks, keep the template circuit and generate a bound circuit for each parameter set.

---

## Minimal example: classical data and control flow

The Rust core supports representing measurement, classical expressions and structured control flow in a circuit. The following example measures qubit `0` and conditionally applies an `X` gate to qubit `1` according to the measurement result.

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

let m = circuit.measure(Qubit::new(0))?;

circuit.if_(m.expr().to_bool()?, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Such circuits contain runtime classical data and control flow structures, and usually no longer have a single fixed unconditional unitary matrix representation. Therefore `to_matrix()` should not be called directly on a circuit that contains measurement or control flow, unless a purely quantum sub-circuit has been extracted from it.

Whether a specific backend supports such control flow structures must be confirmed against the target compiler, IR and execution backend.

---

## Minimal example: creating a circuit from the construction-layer IR

When a circuit comes from an external format, serialized data or compiler pass output, first build a list of `ValueOperation` and then produce the circuit through `Circuit::from_operations()`.

```rust
use cqlib_core::circuit::{
    Circuit,
    ParameterValue,
    Qubit,
    StandardGate,
    ValueOperation,
};

let ops = vec![
    ValueOperation::from_standard(StandardGate::H, [Qubit::new(0)], []),
    ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(0)],
        [ParameterValue::from("theta")],
    ),
];

let circuit = Circuit::from_operations(
    vec![Qubit::new(0)],
    ops,
    None,
    None,
)?;

assert!(circuit.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`Circuit::from_operations()` makes the construction-layer parameters resident in the circuit's internal parameter table, and validates qubits, parameter counts, classical handles and control flow structures.

---

## Types and parameters

| Name | Rust representation | Where it is used | Description |
| --- | --- | --- | --- |
| Logical qubit | `Qubit` | Circuits, gate operations, matrix ordering | Stores a `u32` number internally and is a logical identifier. |
| Parameter expression | `Parameter` | Gate angles, global phase, symbolic matrices | A construction-time symbolic expression. |
| Construction-layer parameter | `ParameterValue` | `ValueOperation` | Either a fixed numeric value or a complete `Parameter`. |
| Storage-layer parameter | `CircuitParam` | `Operation` | Either a fixed numeric value or an index into the circuit parameter table. |
| Classical type | `ClassicalType` | Classical variables, measurement values, expressions | Includes `Bit`, `Bool`, `UInt` and `BitVec`. |
| Classical expression | `ClassicalExpr` | Control flow conditions, comparisons, selections | A runtime classical-side expression AST. |

Controlled gates usually follow the order "control qubits first, target qubit last". For example, when a multi-controlled X gate is applied to `[q0, q1, q2]`, `q0` and `q1` are usually the control qubits and `q2` is the target qubit.

---

## Errors and return values

The Rust API does not use exceptions. Fallible interfaces usually return `Result<_, CircuitError>`, `Result<_, ParameterError>` or a related error type. Callers should propagate errors with `?`, or handle them according to the concrete error type.

| Error type | Description |
| --- | --- |
| `CircuitError` | Circuit construction, validation, matrix conversion, inversion, control flow and gate definition errors. |
| `ParameterError` | Errors in parameter expression parsing, simplification, substitution, differentiation or symbolic evaluation. |
| `EvalError` | Errors in numeric evaluation of parameters. |
| `QubitError` | A negative number or an out-of-range value when converting an integer to `Qubit`. |

Common triggering scenarios include:

- An operation references a qubit that does not exist in the circuit;
- The same qubit is used more than once in a single operation;
- The number of qubits a gate acts on does not match the gate definition;
- The parameter count does not match the gate definition;
- A fixed parameter is `NaN` or infinite;
- A numeric matrix is requested for a circuit that contains unbound parameters;
- A matrix or inversion is requested for measurement, reset, delay or control flow;
- A classical value in control flow is used outside its scope.

For circuits that are externally imported, generated automatically by a program or produced by compiler passes, call the following before entering subsequent flows:

```rust
circuit.validate()?;
```
