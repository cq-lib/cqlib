# Circuit To Matrix

- `cqlib_core::circuit::circuit_to_matrix`
- `cqlib_core::circuit::Circuit::to_matrix`
- `cqlib_core::circuit::symbolic_matrix::circuit_to_symbolic_matrix`

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};
```

The interfaces related to `Circuit To Matrix` convert a quantum circuit into a matrix representation. The Rust core provides two categories of matrix conversion:

- Numeric matrix conversion: convert a circuit whose parameters are all bound into an `Array2<Complex64>`;
- Symbolic matrix conversion: retain the `Parameter` expressions in the circuit and generate a `SymbolicMatrix`.
  
---

## Numeric matrix conversion

```rust
pub fn circuit_to_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[usize]>,
) -> Result<Array2<Complex64>, CircuitError>
```

`circuit_to_matrix()` is the functional matrix conversion entry point, which converts a circuit into a numeric complex matrix. `Circuit::to_matrix()` is the corresponding method-style convenience interface, and the two have the same semantics.

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let matrix = circuit_to_matrix(&circuit, None)?;
assert_eq!(matrix.shape(), &[4, 4]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In the example above, the circuit contains 2 qubits, so the matrix dimension is `4 × 4`. In general, if a circuit contains `n` qubits, the returned matrix shape is:

```text
(2^n, 2^n)
```

---

## `Circuit::to_matrix()` method-style entry point

Besides the functional interface, `to_matrix()` can also be called directly on a circuit object:

```rust
let matrix = circuit.to_matrix(None)?;
# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Qubit order

`qubits_order` specifies the arrangement order of qubits in the matrix basis state indices. It must be a complete permutation of the qubit numbers in the circuit.

The rules are as follows:

- `None`: sort by qubit number by default;
- `Some(&[...])`: use the explicitly specified qubit order;
- The explicit order must match the qubit set of the circuit exactly, and must not omit, duplicate or contain unknown qubits;
- The first qubit in the order corresponds to the least-significant bit of the basis index, that is, the little-endian convention.

```rust
let matrix_default = circuit_to_matrix(&circuit, None)?;
let matrix_reordered = circuit_to_matrix(&circuit, Some(&[1, 0]))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Parameterized circuits

A numeric matrix requires all symbolic parameters to be bound to concrete values. If unbound symbolic parameters remain in the circuit, matrix conversion cannot produce a definite complex matrix.

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit, circuit_to_matrix};
use std::collections::HashMap;

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), Parameter::symbol("theta"))?;

let mut bindings = HashMap::new();
bindings.insert("theta", 0.5);

let bound = circuit.assign_parameters(&Some(bindings))?;
let matrix = circuit_to_matrix(&bound, None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Symbolic matrix conversion

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::circuit::symbolic_matrix::circuit_to_symbolic_matrix;

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), Parameter::symbol("theta"))?;

let symbolic = circuit_to_symbolic_matrix(&circuit, None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

Symbolic matrix conversion retains the `Parameter` expressions in the circuit and returns a `SymbolicMatrix`. It can then be evaluated repeatedly under different parameter values through `evaluate_symbolic_matrix()`.

---

## Global phase

Both numeric and symbolic matrix conversion include the global phase of the circuit. If the circuit global phase is `theta` and the circuit matrix without considering the global phase is `U`, the matrix conversion returns:

```text
exp(i * theta) * U
```

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0))?;
circuit.set_global_phase(std::f64::consts::PI.into());

let matrix = circuit_to_matrix(&circuit, None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In the example above, the operation part of the circuit corresponds to the `X` gate and the global phase is `π`, so the overall matrix is equivalent to `-X`.
