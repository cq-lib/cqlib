# Symbolic Matrix

`cqlib_core::circuit::symbolic_matrix`

```rust
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
    circuit_to_symbolic_matrix,
    evaluate_symbolic_matrix,
    circuits_equivalent,
};
```

The `symbolic_matrix` module provides symbolic capability for circuit matrix conversion. Unlike the numeric matrix conversion function `circuit_to_matrix()`, symbolic matrix conversion retains the `Parameter` expressions in a circuit as far as possible, so that matrix elements can contain unbound symbolic parameters.

---

## Core concepts

| Type / Function | Description |
| --- | --- |
| `SymbolicComplex` | A symbolic complex number whose real and imaginary parts are both `Parameter` expressions. |
| `SymbolicMatrix` | A dense matrix composed of `SymbolicComplex`, usually understood as `Array2<SymbolicComplex>`. |
| `circuit_to_symbolic_matrix()` | Convert a purely quantum circuit into a matrix that retains symbolic parameters. |
| `evaluate_symbolic_matrix()` | Bind parameters for a symbolic matrix and evaluate it into a numeric complex matrix. |
| `symbolic_matrices_equivalent()` | Determine whether two symbolic matrices can be proven to differ only by a global phase. |
| `circuits_equivalent()` | Determine whether two circuits can be proven to be equivalent up to a global phase in the matrix sense. |

---

## `SymbolicComplex`

`SymbolicComplex` represents a symbolic complex number whose real and imaginary parts are both `Parameter` expressions. It can represent an ordinary complex constant, or a complex expression containing symbolic variables, such as `exp(iθ)`.

### Common constructors

| Method | Description |
| --- | --- |
| `new(re, im)` | Create a symbolic complex number from real and imaginary part expressions. |
| `zero()` | Create `0 + 0i`. |
| `one()` | Create `1 + 0i`. |
| `i()` | Create `0 + 1i`. |
| `from_real(value)` | Create a purely real symbolic complex number from a real number or a real expression. |
| `from_complex(value)` | Create a symbolic complex constant from a `Complex64`. |
| `exp_i(theta)` | Create `cos(theta) + i sin(theta)`, that is, `e^{i theta}`. |

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::SymbolicComplex;

let theta = Parameter::symbol("theta");

let zero = SymbolicComplex::zero();
let one = SymbolicComplex::one();
let imag = SymbolicComplex::i();
let phase = SymbolicComplex::exp_i(theta);
```

Among these, `exp_i(theta)` is commonly used to define matrix elements in phase gates, diagonal gates and symbolic quantum evolution.

### Common methods

| Method | Description |
| --- | --- |
| `evaluate(bindings)` | Bind symbolic parameters and evaluate into a `Complex64`. |
| `simplify()` | Simplify the real and imaginary part expressions. |
| `replace(symbol, value)` | Replace a symbol appearing in the complex number. |
| `is_zero_exact()` | Determine whether it is syntactically exactly zero. |
| `is_one_exact()` | Determine whether it is syntactically exactly one. |
| `simplifies_to_zero()` | Determine whether it is zero after simplification. |

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::SymbolicComplex;
use std::collections::HashMap;

let theta = Parameter::symbol("theta");
let value = SymbolicComplex::exp_i(theta);

let mut bindings = HashMap::new();
bindings.insert("theta", 0.0);

let numeric = value.evaluate(&Some(bindings))?;
assert_eq!(numeric, num_complex::Complex64::new(1.0, 0.0));

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## `SymbolicMatrix`

`SymbolicMatrix` is a dense matrix composed of `SymbolicComplex`. It can represent a quantum gate matrix or circuit matrix with symbolic parameters.

```rust
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
};

let matrix = SymbolicMatrix::from_shape_vec(
    (2, 2),
    vec![
        SymbolicComplex::zero(),
        SymbolicComplex::i(),
        SymbolicComplex::i(),
        SymbolicComplex::zero(),
    ],
).expect("valid 2x2 symbolic matrix");
```

When using a symbolic matrix, ensure that the matrix dimension matches the number of qubits of the target gate or circuit. For example, a single-qubit gate matrix should be `2 × 2`, and a two-qubit gate matrix should be `4 × 4`.

---

## Matrix utility functions

The `symbolic_matrix` module provides a set of common utility functions for construction, substitution, evaluation, conversion and equivalence checking.

| Function | Description |
| --- | --- |
| `symbolic_eye(dim)` | Create a `dim × dim` symbolic identity matrix. |
| `substitute_symbolic_matrix(matrix, replacements)` | Perform simultaneous substitution of the symbols in a matrix. |
| `evaluate_symbolic_matrix(matrix, bindings)` | Evaluate a symbolic matrix into an `Array2<Complex64>`. |
| `standard_gate_symbolic_matrix(gate, params)` | Generate the symbolic matrix of a standard gate. |
| `circuit_to_symbolic_matrix(circuit, qubits_order)` | Convert a circuit into a symbolic unitary matrix. |
| `symbolic_matrices_equivalent(lhs, rhs)` | Determine whether two symbolic matrices can be proven to differ only by a global phase. |
| `circuits_equivalent(lhs, rhs, qubits_order)` | Determine whether two circuits can be proven to be equivalent up to a global phase. |

Besides the functions above, the module also exports a set of low-level functions that write gate matrices into a symbolic matrix in place, mainly used for block-wise matrix construction and custom gate implementations.

| Function | Description |
| --- | --- |
| `apply_standard_gate_to_matrix(matrix, gate, bits, params)` | Apply a standard gate to the given bits in place. |
| `apply_gate_to_matrix(matrix, gate, bits)` | Apply a symbolic gate matrix to the given bits in place. |
| `apply_gate_to_matrix_num(matrix, gate, bits)` | Apply a numeric gate matrix to the given bits in place. |
| `apply_single_qubit_gate(matrix, gate, bit)` / `apply_single_qubit_gate_num(matrix, gate, bit)` | Apply a single-qubit gate in place; the two functions take a symbolic gate matrix and a numeric gate matrix respectively. |
| `apply_two_qubit_gate(matrix, gate, b0, b1)` / `apply_two_qubit_gate_num(matrix, gate, b0, b1)` | Apply a two-qubit gate in place. |
| `apply_general_gate(matrix, gate, bits)` / `apply_general_gate_num(matrix, gate, bits)` | Apply a gate on an arbitrary number of bits in place. |
| `apply_symbolic_permutation_gate(matrix, permutation, bits)` / `apply_numeric_permutation_gate(matrix, permutation, bits)` | Apply a permutation-type gate in place. |
| `apply_symbolic_diagonal_gate(matrix, diagonal, bits)` / `apply_numeric_diagonal_gate(matrix, diagonal, bits)` | Apply a diagonal-type gate in place. |

---

## Circuit symbolic matrix conversion

```rust
pub fn circuit_to_symbolic_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[usize]>,
) -> Result<SymbolicMatrix, CircuitError>
```

`circuit_to_symbolic_matrix()` converts a quantum circuit into a symbolic matrix that retains `Parameter` expressions.

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::circuit::symbolic_matrix::{
    circuit_to_symbolic_matrix,
    evaluate_symbolic_matrix,
};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), theta)?;

let symbolic = circuit_to_symbolic_matrix(&circuit, None)?;

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::FRAC_PI_2);

let numeric = evaluate_symbolic_matrix(&symbolic, &Some(bindings))?;
assert_eq!(numeric.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

In the example above, the parameter of `RX(theta)` is not evaluated to a number during symbolic matrix construction, but is kept as a symbolic expression. Later, `evaluate_symbolic_matrix()` can be used to bind a concrete value for `theta` and obtain a numeric matrix.

---

## Qubit order convention

`circuit_to_symbolic_matrix(circuit, qubits_order)` needs to determine the correspondence between matrix basis state indices and circuit qubits. The order convention is as follows:

- `qubits_order = None`: sort by qubit number by default;
- `qubits_order = Some(order)`: use the order explicitly given by the caller;
- The explicit order must match the qubit set in the circuit exactly, and must not omit, duplicate or contain unknown qubits;
- The first qubit in the order corresponds to the least-significant bit of the basis index.

For example, if the qubit numbers in the circuit are `0` and `2`, the explicit order can be written as `[0, 2]` or `[2, 0]`, and the two give matrix representations under different axis orders.

---

## Symbolic matrix evaluation

```rust
pub fn evaluate_symbolic_matrix(
    matrix: &SymbolicMatrix,
    bindings: &Option<HashMap<&str, f64>>,
) -> Result<Array2<Complex64>, ParameterError>
```

`evaluate_symbolic_matrix()` converts a symbolic matrix into a numeric complex matrix. It performs parameter binding and evaluation on each `SymbolicComplex` element separately.

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
    evaluate_symbolic_matrix,
};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let matrix = SymbolicMatrix::from_shape_vec(
    (2, 2),
    vec![
        SymbolicComplex::one(),
        SymbolicComplex::zero(),
        SymbolicComplex::zero(),
        SymbolicComplex::exp_i(theta),
    ],
).expect("valid symbolic matrix");

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::PI);

let numeric = evaluate_symbolic_matrix(&matrix, &Some(bindings))?;
assert_eq!(numeric.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## Symbolic substitution

`substitute_symbolic_matrix(matrix, replacements)` performs simultaneous substitution of the symbols in a matrix. It suits scenarios such as parameter renaming, composite gate expansion, template parameter substitution and custom symbolic gate binding.

Typical scenarios include:

- Replacing an inner parameter `theta` of a sub-circuit with an outer parameter `alpha`;
- Renaming parameters in a compiler pass to avoid symbol conflicts;
- Instantiating a generic symbolic matrix as another parameterized gate definition.

---

## Equivalence checking

The `symbolic_matrix` module provides equivalence checking interfaces in the sense of global phase.

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::circuit::symbolic_matrix::circuits_equivalent;

let mut lhs = Circuit::new(1);
lhs.x(Qubit::new(0))?;
lhs.set_global_phase(std::f64::consts::PI.into());

let mut rhs = Circuit::new(1);
rhs.x(Qubit::new(0))?;

assert!(circuits_equivalent(&lhs, &rhs, None)?);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```
