# CircuitGate / FrozenCircuit

- `cqlib_core::circuit::CircuitGate`
- `cqlib_core::circuit::gate::FrozenCircuit`

```rust
use cqlib_core::circuit::{Circuit, CircuitGate};
use cqlib_core::circuit::gate::FrozenCircuit;
```

`FrozenCircuit` and `CircuitGate` are used in the Rust core to express "a composite gate defined by a circuit". `FrozenCircuit` is an immutable circuit definition, holding the structure and operation sequence of a sub-circuit; `CircuitGate` is a reusable gate object built from that definition, and can be appended to other circuits like an ordinary gate.

---

## `FrozenCircuit`

`FrozenCircuit` is an immutable circuit definition, used to hold a `Circuit` that has already been constructed. After creation, the inner circuit is used as a gate definition and should no longer be modified externally.

```rust
pub fn new(circuit: Circuit) -> Self
pub fn circuit(&self) -> &Circuit
pub fn used_symbols(&self) -> &IndexSet<String>
pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError>
```

### Creating an immutable circuit definition

`FrozenCircuit::new(circuit)` moves the passed-in `Circuit` and stores it as an immutable definition. Because the passed-in circuit is moved, the caller can no longer modify this circuit through the original variable, which prevents the composite gate definition from changing while it is reused.

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let mut inner = Circuit::new(2);
inner.h(Qubit::new(0))?;
inner.cx(Qubit::new(0), Qubit::new(1))?;

let frozen = FrozenCircuit::new(inner);

assert_eq!(frozen.circuit().num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Reading the inner circuit

```rust
pub fn circuit(&self) -> &Circuit
```

`circuit()` returns a reference to the inner immutable circuit. This interface is suitable for inspecting the number of qubits, the operation sequence or the symbolic parameters in the definition, or for performing compilation analysis.

### Symbolic matrix cache

```rust
pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError>
```

`symbolic_matrix()` computes and returns the symbolic matrix of the inner circuit. The method usually computes the symbolic matrix on the first call and caches the result; later calls reuse the cached result, so as to reduce repeated computation overhead.

---

## `CircuitGate::new`

```rust
pub fn new(name: impl Into<String>, circuit: FrozenCircuit) -> Result<Self, CircuitError>
```

`CircuitGate::new(name, circuit)` creates a composite gate definition from a `FrozenCircuit`. The method automatically uses the free symbols in the frozen circuit as the call signature.

```rust
use cqlib_core::circuit::{Circuit, CircuitGate, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let mut inner = Circuit::new(1);
inner.h(Qubit::new(0))?;

let gate = CircuitGate::new("HBlock", FrozenCircuit::new(inner))?;

assert_eq!(gate.name(), "HBlock");
assert_eq!(gate.num_qubits(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In the example above, `HBlock` is a composite gate defined by a single-qubit `H` gate circuit. Because the inner circuit has no symbolic parameters, the number of parameters of the composite gate is 0.

---

## `CircuitGate::with_signature`

```rust
pub fn with_signature(
    name: impl Into<String>,
    circuit: FrozenCircuit,
    params: impl IntoIterator<Item = String>,
) -> Result<Self, CircuitError>
```

`with_signature()` is used to declare the call parameter signature of a composite gate explicitly.

---

## Attributes and methods

`CircuitGate` provides the following common query interfaces:

| Method | Returns | Description |
| --- | --- | --- |
| `name()` | `&str` | Return the composite gate name. |
| `num_qubits()` | `usize` | Return the number of qubits required when the composite gate is applied. |
| `num_params()` | `usize` | Return the number of positional parameters in the call signature. |
| `signature_params()` | `&IndexSet<String>` | Return the parameter list in the composite gate call signature; the order carries binding semantics. |
| `used_symbols()` | `&IndexSet<String>` | Return the set of symbols actually referenced by the inner circuit. |
| `symbols()` | `IndexSet<String>` | Return a clone of `used_symbols()`. |
| `circuit()` | `Arc<FrozenCircuit>` | Return the frozen circuit definition of the composite gate. |
| `symbolic_matrix()` | `Result<Arc<SymbolicMatrix>, CircuitError>` | Return the symbolic matrix of the inner definition, usually reusing the cache. |

---

## Parameter signature and binding semantics

Parameter binding of `CircuitGate` uses positional parameter semantics. That is, the `i`-th parameter passed in when the composite gate is called replaces the inner symbol corresponding to `signature_params()[i]`.

For example, if the signature is:

```text
["theta", "phi"]
```

then the call passes in:

```text
[alpha, beta]
```

which means:

```text
theta -> alpha
phi   -> beta
```

This substitution is performed simultaneously, not step by step in order. Therefore, for swap scenarios such as `a -> b` and `b -> a`, no intermediate conflict arises from the substitution order.

---

## Parameterized composite gate example

The example below constructs a sub-circuit with symbolic parameters and wraps it as a `CircuitGate`. When the outer circuit calls this composite gate, new parameter expressions can be passed in to replace the inner symbols.

```rust
use cqlib_core::circuit::{Circuit, CircuitGate, Parameter, ParameterValue, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let theta = Parameter::symbol("theta");

let mut inner = Circuit::new(1);
inner.rx(Qubit::new(0), theta)?;

let gate = CircuitGate::new("RxBlock", FrozenCircuit::new(inner))?;

let mut outer = Circuit::new(1);
outer.circuit_gate(
    gate,
    vec![Qubit::new(0)],
    vec![ParameterValue::from("alpha")],
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In this example, the outer call parameter `alpha` replaces the inner symbol `theta` in the composite gate definition. If the inner circuit contains several symbols, the number and order of the parameters passed in when calling must agree with `signature_params()`.

---

## Inverse gate

```rust
pub fn inverse(&self) -> Result<Self, CircuitError>
```

`inverse()` returns a new `CircuitGate` whose underlying circuit is the result of inverting the original frozen circuit operation by operation and arranging the operations in reverse order. By default, the new gate name has the suffix `_dg` appended to the original name, and the original parameter signature is kept.

```rust
let inv = gate.inverse()?;
assert_eq!(inv.name(), "HBlock_dg");

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Appending to `Circuit`

A `CircuitGate` can be appended to an outer circuit through `Circuit::circuit_gate()`.

```rust
circuit.circuit_gate(gate, qubits, params)?;
```

Here:

- `gate` is the composite gate to apply;
- `qubits` is the list of qubits the gate actually acts on in the outer circuit;
- `params` is the list of positional parameters provided by the outer call.

```rust
use cqlib_core::circuit::{Circuit, CircuitGate, ParameterValue, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let mut inner = Circuit::new(1);
inner.h(Qubit::new(0))?;

let gate = CircuitGate::new("HBlock", FrozenCircuit::new(inner))?;

let mut outer = Circuit::new(1);

outer.circuit_gate(
    gate,
    vec![Qubit::new(0)],
    Vec::<ParameterValue>::new(),
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Decomposition and matrix analysis

Because `CircuitGate` keeps the inner circuit structure, it can be expanded when needed or used for matrix analysis.

A typical flow includes:

- Expand the composite gate into the inner base operations with `Circuit::decompose()`;
- Obtain the symbolic matrix of the inner definition with `CircuitGate::symbolic_matrix()`;
- Validate the overall matrix behavior in the outer circuit with `Circuit::to_matrix()`;
- Generate the corresponding inverse gate with `inverse()`.

Compared with a black-box matrix gate, the advantage of `CircuitGate` is that it keeps the operation structure of the sub-circuit, so it is better suited to compiler analysis, gate decomposition, resource counting and visualization.

---

## Difference from `UnitaryGate`

| Type | Suitable scenario | Characteristics |
| --- | --- | --- |
| `CircuitGate` | A sub-circuit is reused as a composite gate | Keeps the circuit structure; it can be decomposed and inverted, and its symbolic matrix can be cached. |
| `UnitaryGate::with_matrix()` | Only a black-box numeric matrix is of interest | Does not keep the inner circuit structure; suitable for a fixed custom matrix. |
| `UnitaryGate::with_symbolic_matrix()` | Only a black-box symbolic matrix is of interest | Suitable for a parameterized custom matrix gate. |
| `UnitaryGate::with_circuit()` | A circuit-backed definition is to be kept in the form of a custom unitary | Keeps the circuit-backed definition, but semantically it is closer to a custom unitary gate. |

---

## Signature and internal symbols

`signature_params()` and `used_symbols()` are used to distinguish the external call interface of a composite gate from its internal implementation details.

| Term | Description |
| --- | --- |
| `signature_params()` | The list of positional parameters that must be provided in order when calling. |
| `used_symbols()` | The set of free symbols actually referenced by the inner circuit. |
| `num_params()` | Equal to the number of signature parameters, not necessarily equal to the number of symbols actually used inside. |
