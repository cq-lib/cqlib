# Circuit

`cqlib_core::circuit::Circuit`  

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
```

`Circuit` is the main quantum circuit container in the Rust core. It holds the core structural information of a quantum program, including the logical qubit set, the operation sequence, the parameter table, classical variables and classical values, control flow scopes and the global phase.

---

## Creating a circuit

### `Circuit::new`

```rust
pub fn new(num_qubits: usize) -> Self
```

`Circuit::new(num_qubits)` creates an empty circuit containing consecutive logical qubits. Qubit numbering starts at `0`, from `Qubit::new(0)` to `Qubit::new(num_qubits - 1)`.

```rust
use cqlib_core::circuit::Circuit;

let circuit = Circuit::new(3);
assert_eq!(circuit.num_qubits(), 3);
assert_eq!(circuit.width(), 3);
```

### `Circuit::from_qubits`

```rust
pub fn from_qubits(qubits: Vec<Qubit>) -> Result<Circuit, CircuitError>
```

`Circuit::from_qubits()` creates a circuit from an explicitly specified set of logical qubits. This interface allows sparse logical numbering, for example `Qubit::new(2)` and `Qubit::new(5)`.

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let circuit = Circuit::from_qubits(vec![Qubit::new(2), Qubit::new(5)])?;
assert_eq!(circuit.width(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### `Circuit::from_operations`

```rust
pub fn from_operations(
    qubits: Vec<Qubit>,
    operations: impl IntoIterator<Item = ValueOperation>,
    classical_vars: Option<Vec<ClassicalType>>,
    classical_values: Option<Vec<ClassicalType>>,
) -> Result<Self, CircuitError>
```

`Circuit::from_operations()` creates a circuit from the construction-layer IR.

---

## Basic properties

`Circuit` provides a set of read-only access methods for inspecting the basic structural information of a circuit.

| Method | Returns | Description |
| --- | --- | --- |
| `id()` | `CircuitId` | The classical handle identity of the current circuit, used to distinguish classical variables and values in different circuits. |
| `width()` | `usize` | Circuit width, that is, the number of qubits. |
| `num_qubits()` | `usize` | The number of qubits, with the same meaning as `width()`. |
| `qubits()` | `Vec<Qubit>` | Return the logical qubits in the circuit in insertion order. |
| `operations()` | `&[Operation]` | Return the internal storage-layer operation sequence. |
| `parameters()` | `&IndexSet<Parameter>` | Return the set of resident parameter expressions. |
| `symbols()` | `&IndexSet<String>` | Return the set of free symbol names appearing in the circuit. |
| `used_symbols()` | `IndexSet<String>` | Return the set of symbol names actually referenced by the executable IR. |
| `uses_symbol(symbol)` | `bool` | Determine whether the given symbol is referenced by the executable IR. |
| `classical_vars()` | `&[ClassicalType]` | Return the table of allocated mutable classical variable types. |
| `classical_values()` | `&[ClassicalType]` | Return the table of immutable classical value types, usually produced by measurement. |
| `global_phase()` | `Parameter` | Return the parameter expression of the circuit global phase. |
| `global_phase_param()` | `&CircuitParam` | Return the global phase parameter in its internal storage form. |

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);

let mut c = Circuit::new(2);
c.h(q0)?;
c.cx(q0, q1)?;

assert_eq!(c.operations().len(), 2);
assert_eq!(c.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Qubit and parameter management

`Circuit` maintains a qubit set and a parameter table internally. Qubits describe what an operation acts on, and the parameter table manages the symbolic parameters and parameter expressions in the circuit in a unified way.

| Method | Description |
| --- | --- |
| `add_qubits(new_qubits)` | Append new logical qubits to the circuit; return an error if a duplicate qubit is included. |
| `add_parameter(param)` | Insert a parameter into the circuit parameter table and return `(index, is_new)`. |
| `resolve_parameter(param)` | Restore an internal `CircuitParam` to a `Parameter`. |
| `parameter_value(param)` | Convert an internal `CircuitParam` into a construction-layer `ParameterValue`. |
| `map_param(param)` | Map a `Parameter` into the circuit parameter table and return the internal `CircuitParam`. |
| `set_global_phase(phase)` | Set the circuit global phase. |

---

## Appending operations

### `append`

```rust
pub fn append<Q, P>(
    &mut self,
    instruction: Instruction,
    qubits: Q,
    params: P,
    label: Option<&str>,
) -> Result<(), CircuitError>
where
    Q: IntoIterator,
    Q::Item: Into<Qubit>,
    P: IntoIterator<Item = ParameterValue>
```

`append()` is the generic entry point for appending storage-layer instructions, used to append an arbitrary `Instruction` to a circuit. The call must explicitly provide the instruction, the acted-on qubits, the parameter list and an optional label.

The method performs several checks, including:

- whether the number of qubits the instruction acts on matches;
- whether the parameter count agrees with the instruction definition;
- whether the qubits belong to the current circuit;
- whether the same qubit is referenced more than once in a single operation;
- whether a fixed numeric parameter is finite, that is, not `NaN` or infinite;
- whether the parameter expressions can be mapped correctly into the circuit parameter table.

### `append_value_operation`

```rust
pub fn append_value_operation(&mut self, operation: ValueOperation) -> Result<(), CircuitError>
```

`append_value_operation()` appends a self-contained construction-layer operation. A `ValueOperation` already contains the instruction, qubits, parameters and label information, so this interface is particularly suitable for IR import, deserialization and compiler pass output.

### `index`

```rust
pub fn index(&self, i: usize) -> Result<ValueOperation, CircuitError>
```

`index(i)` reads the `i`-th operation and restores the internal storage-layer parameter representation to a construction-layer `ParameterValue`.

---

## Standard gate convenience methods

`Circuit` provides a set of convenience methods for common standard gates. All convenience methods modify the current circuit and return `Result<(), CircuitError>`. Users can handle possible errors with `?` or an explicit `match`.

### Single-qubit fixed gates

| Method | Standard gate |
| --- | --- |
| `i(qubit)` | `StandardGate::I` |
| `h(qubit)` | `StandardGate::H` |
| `x(qubit)` | `StandardGate::X` |
| `y(qubit)` | `StandardGate::Y` |
| `z(qubit)` | `StandardGate::Z` |
| `s(qubit)` | `StandardGate::S` |
| `sdg(qubit)` | `StandardGate::SDG` |
| `t(qubit)` | `StandardGate::T` |
| `tdg(qubit)` | `StandardGate::TDG` |
| `x2p(qubit)` | `StandardGate::X2P` |
| `x2m(qubit)` | `StandardGate::X2M` |
| `y2p(qubit)` | `StandardGate::Y2P` |
| `y2m(qubit)` | `StandardGate::Y2M` |

### Parameterized single-qubit gates

| Method | Description |
| --- | --- |
| `rx(qubit, theta)` | Rotation around the X axis. |
| `ry(qubit, theta)` | Rotation around the Y axis. |
| `rz(qubit, theta)` | Rotation around the Z axis. |
| `phase(qubit, lambda)` | Phase gate. |
| `u(qubit, theta, phi, lambda)` | Generic single-qubit gate. |
| `xy(qubit, theta)` | Gate of the XY interaction family. |
| `xy2p(qubit, theta)` | Positive half-angle XY gate. |
| `xy2m(qubit, theta)` | Negative half-angle XY gate. |
| `rxy(qubit, theta, phi)` | Rotation around an arbitrary axis in the XY plane. |

### Multi-qubit gates

| Method | Description |
| --- | --- |
| `cx(control, target)` | controlled-X, also known as CNOT. |
| `cy(control, target)` | controlled-Y. |
| `cz(control, target)` | controlled-Z. |
| `swap(a, b)` | Exchange the states of two qubits. |
| `ccx(control1, control2, target)` | Toffoli gate. |
| `rxx(a, b, theta)` | XX rotation. |
| `ryy(a, b, theta)` | YY rotation. |
| `rzz(a, b, theta)` | ZZ rotation. |
| `rzx(a, b, theta)` | ZX rotation. |
| `crx(control, target, theta)` | controlled-RX. |
| `cry(control, target, theta)` | controlled-RY. |
| `crz(control, target, theta)` | controlled-RZ. |
| `fsim(a, b, theta, phi)` | fSim gate. |

Example:

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let theta = Parameter::symbol("theta");

let mut c = Circuit::new(2);
c.h(q0)?;
c.cx(q0, q1)?;
c.rzz(q0, q1, theta)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Other quantum operations

Besides the standard gate convenience methods, `Circuit` also provides entry points for barrier, reset, delay, multi-controlled gates, custom unitary gates and composite gates.

| Method | Description |
| --- | --- |
| `barrier(qubits)` | Insert a barrier, which constrains the compiler or scheduler not to reorder operations on the affected qubits across this boundary. |
| `reset(qubit)` | Reset the qubit to `\|0>`. |
| `delay(qubit, delay)` | Insert idle time on the given qubit. |
| `multi_control(instruction, controls, targets, params)` | Construct a multi-controlled standard gate. |
| `unitary(gate, qubits)` | Append a `UnitaryGate` without extra positional parameters. |
| `unitary_with_params(gate, qubits, params)` | Append a `UnitaryGate` with positional parameters. |
| `circuit_gate(gate, qubits, params)` | Append a `CircuitGate`. |

---

## Classical data and control flow entry points

`Circuit` also provides interfaces related to classical data and structured control flow. They are used to construct program structures such as conditional branches, loops and multi-way selections.

| Method | Description |
| --- | --- |
| `var(ty)` | Allocate a classical variable. |
| `store(target, value)` | Write a classical expression into a classical variable. |
| `measure(qubit)` | Measure one qubit. |
| `measure_bits(qubits)` | Measure multiple qubits. |
| `measure_into(qubit, target)` | Measure and write into an existing classical variable. |
| `measure_bits_into(qubits, target)` | Measure multiple qubits and write into an existing classical variable. |
| `if_()` / `if_else()` | Conditional branch. |
| `while_()` | `while` loop. |
| `for_uint()` | Half-open interval loop based on `UInt`. |
| `switch()` | Multi-way selection. |
| `append_control(op)` | Append a low-level control flow object. |
| `break_loop()` / `continue_loop()` | Control flow jumps. |

---

## Analysis and transformation

`Circuit` provides various circuit analysis and structural transformation methods.

| Method | Description |
| --- | --- |
| `depth(recurse)` | Compute the circuit depth. If the circuit contains control flow and `recurse = false`, `ControlFlowPresent` is usually returned. |
| `remove_operation(index)` | Remove a single top-level operation and return the removed operation. |
| `remove_operations(indices)` | Remove multiple top-level operations and return the removed operations in ascending order of their original indices. |
| `validate()` | Validate classical handles, control flow scopes, data dependencies and circuit structure invariants. |
| `inverse()` | Return the inverse of the current circuit. Return an error if it contains non-invertible operations. |
| `decompose()` | Expand composite gates defined by circuits. |
| `to_gate(name)` | Convert the circuit into an `Instruction::CircuitGate` for reuse as a composite gate. |
| `to_matrix(qubits_order)` | Return the dense numeric matrix of a small purely unitary circuit. |
| `assign_parameters(bindings)` | Bind symbolic parameters and return a new circuit. |
| `compose(other, qubits_map)` | Append another circuit to the current circuit, with an optional qubit mapping. |

---

## Parameter binding

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut c = Circuit::new(1);
c.rx(Qubit::new(0), theta)?;

let mut bindings = HashMap::new();
bindings.insert("theta", 0.5);

let bound = c.assign_parameters(&Some(bindings))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`assign_parameters()` replaces the symbolic parameters in a circuit with concrete numeric values.

---

## Circuit composition

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut lhs = Circuit::new(3);
let mut rhs = Circuit::new(2);

rhs.cx(Qubit::new(0), Qubit::new(1))?;

lhs.compose(&rhs, Some(&[Qubit::new(1), Qubit::new(2)]))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`compose(other, qubits_map)` appends another circuit to the end of the current circuit. When `qubits_map` is passed, the qubits of the right-hand circuit are mapped, in the order of `other.qubits()`, to the target qubits in the current circuit.
