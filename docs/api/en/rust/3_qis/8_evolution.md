# Pauli Evolution and Trotter Decomposition

`cqlib_core::qis::evolution`

This page covers the interfaces that convert Pauli strings and Hamiltonians into evolution circuits: the `PauliEvolution` trait provides the rotation gate of a single Pauli string, two entry points on `Hamiltonian` extend it into a full time-evolution circuit, and `TrotterMode` determines the approximation used for multiple non-commuting terms.

## Import

```rust
use cqlib_core::qis::evolution::{PauliEvolution, TrotterMode};
```

`PauliEvolution` and `TrotterMode` are also re-exported at the top level of `cqlib_core::qis`.

---

## TrotterMode

The Trotter-Suzuki decomposition mode for the time evolution $U(t) = e^{-iHt}$ of a Hamiltonian.

```rust
pub enum TrotterMode {
    FirstOrder,
    SecondOrder,
    Randomized(u64),
}
```

Variants:

| Variant | Payload | Approximation form | Error order |
| --- | --- | --- | --- |
| `TrotterMode::FirstOrder` | None | $U(t) \approx \left[\prod_k e^{-i c_k t/n \cdot P_k}\right]^n$ | $O(t^2/n)$ |
| `TrotterMode::SecondOrder` | None | Each step applies a half-angle evolution in forward order and then in reverse order | $O(t^3/n^2)$ |
| `TrotterMode::Randomized(u64)` | Random seed | Same as first order, but the order of terms within each step is shuffled randomly | Same order as first order |

The seed carried by `Randomized` determines the permutation of the terms within each step; the same seed gives the same circuit, which makes experiments reproducible.

### Other behavior

- Supports `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.

Example:

```rust
use cqlib_core::qis::evolution::TrotterMode;

// 同一种子的模式相等，可重复使用
assert_eq!(TrotterMode::Randomized(42), TrotterMode::Randomized(42));
assert_ne!(TrotterMode::Randomized(42), TrotterMode::Randomized(43));
assert_ne!(TrotterMode::FirstOrder, TrotterMode::SecondOrder);
```

---

## PauliEvolution

The extension trait that appends Pauli rotation gates to a circuit.

```rust
pub trait PauliEvolution {
    fn pauli_evolution(
        &mut self,
        pauli: &PauliString,
        angle: impl Into<ParameterValue>,
        qubits: &[Qubit],
    ) -> Result<(), CircuitError>;
}
```

### Methods

- `fn pauli_evolution(&mut self, pauli: &PauliString, angle: impl Into<ParameterValue>, qubits: &[Qubit]) -> Result<(), CircuitError>`: append the evolution operator $e^{-i\theta/2 \cdot P}$.

Parameters:

- `pauli` (`&PauliString`): the Pauli string $P$ to exponentiate; its phase must be $\pm 1$ (Hermitian).
- `angle` (`impl Into<ParameterValue>`): the rotation angle $\theta$, which can be a fixed floating-point number or a symbolic parameter.
- `qubits` (`&[Qubit]`): the positions acted on; the length must match `pauli.num_qubits`.

Note the angle convention here: $\theta$ is the angle corresponding to the half-factor of the rotation, and $\theta$ in $e^{-i\theta/2 \cdot P}$ must be $2ct$ to be equivalent to $e^{-ictP}$. When a Hamiltonian is passed to an evolution entry point, no manual conversion is needed, because the entry point already handles this convention internally.

### Implementors

| Type | Description |
| --- | --- |
| `Circuit` | Appends the rotation gate directly to the end of the target circuit. |

### Implementation steps

First, phase check. The internal phase of the Pauli string must be $\pm 1$; only Hermitian operators can be generators of unitary evolution, so a phase of $\pm i$ raises an error directly.

Second, phase absorption. Absorb $\pm 1$ into the rotation angle to obtain the effective angle $\theta_{\text{eff}} = \theta \cdot \text{phase}$.

Third, basis change. Apply to each non-identity qubit the gate that rotates its component into the Z basis: `X` uses `H`, `Y` uses $S^\dagger$ followed by `H`, and `Z` needs no transformation.

Fourth, CNOT chain. Apply `CNOT` in increasing order of the non-identity qubit indices, accumulating the parity level by level.

Fifth, core rotation. Apply `RZ(θ_eff)` on the last non-identity qubit.

Sixth, the CNOT chain in reverse order and the inverse basis change, restoring the qubits to their original basis.

When the Pauli string is all identity operators, the evolution degenerates into a global phase $e^{-i\theta/2}$, produces no gates at all, and the phase is recorded in the global phase of the circuit.

### Errors

- `CircuitError::QubitCountMismatch`: the length of `qubits` is inconsistent with `pauli.num_qubits`.
- `CircuitError::InvalidOperation`: the phase of the Pauli string is $\pm i$ and it cannot serve as a generator of unitary evolution.

### Example

Single-qubit rotation:

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(1);
let qubits = circuit.qubits();

// Z 分量不需要基变换，只产生一个 RZ 门
let pauli: PauliString = "Z".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
    .unwrap();

assert_eq!(circuit.operations().len(), 1);
```

Multi-qubit rotation:

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

// ZZ 链：CNOT(0->1) - RZ(1) - CNOT(0->1)
let pauli: PauliString = "ZZ".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits)
    .unwrap();

let ops = circuit.operations();
assert_eq!(ops.len(), 3);
assert_eq!(ops[0].qubits[0], qubits[0]);
assert_eq!(ops[0].qubits[1], qubits[1]);
assert_eq!(ops[1].qubits[0], qubits[1]);
```

An all-identity string contributes only a global phase:

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

let pauli: PauliString = "II".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
    .unwrap();

assert_eq!(circuit.operations().len(), 0);
```

Inconsistent qubit counts:

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

let pauli: PauliString = "XXX".parse().unwrap();
assert!(circuit.pauli_evolution(&pauli, 1.0, &qubits).is_err());
```

---

## Evolution entry points on Hamiltonian

Both entry points accept the same parameters:

- `time` (`f64`): the total evolution time $t$.
- `steps` (`usize`): the number of Trotter steps $n$, which must be greater than 0.
- `mode` (`TrotterMode`): the decomposition mode.

Both first copy the Hamiltonian and simplify it, then check the imaginary part of the coefficients: when the absolute value of the imaginary part exceeds $10^{-10}$, `QisError::NotHermitian` is returned, because only Hermitian operators can generate unitary evolution.

### `fn to_trotter_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`

Generate an approximate evolution circuit in the given mode; the qubit count of the output circuit matches `num_qubits`.

For a term $P_k$ with coefficient $c_k$, the time slice is $\Delta t = t/n$ and the rotation angle within the term is $2 c_k \Delta t$. The first-order mode applies all terms slice by slice in forward order; the second-order mode applies a half-angle evolution of $\Delta t/2$ in forward order and then once in reverse order within each slice; the randomized mode builds on first order and randomly permutes the order of the terms within each slice, the permutation being determined by the seed carried by `TrotterMode::Randomized`.

Example:

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// H = 0.5 * ZZ
let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

// 2 步，每步 3 个门
let circuit = h.to_trotter_circuit(1.0, 2, TrotterMode::FirstOrder).unwrap();
assert_eq!(circuit.operations().len(), 6);

// 二阶模式每片再重复一遍半角演化，1 步共 6 个门
let circuit = h.to_trotter_circuit(1.0, 1, TrotterMode::SecondOrder).unwrap();
assert_eq!(circuit.operations().len(), 6);
```

### `fn to_evolution_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`

Automatically chooses between exact decomposition and Trotter approximation. It first determines whether all terms commute pairwise:

- When they commute, the exact path is taken, applying each term once according to $U(t) = \prod_k e^{-i c_k t P_k}$; in this case `steps` and `mode` have no effect.
- When they do not commute, it falls back to the same logic as `to_trotter_circuit`, and `steps` and `mode` take effect.

Example:

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// ZZ 与 ZI 对易，直接走精确路径
let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
h.add_term("ZI".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();

let circuit = h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder).unwrap();
assert_eq!(circuit.num_qubits(), 2);
```

In the commuting case, the exact path and first-order Trotter give the same circuit matrix:

```rust
use cqlib_core::circuit::circuit_to_matrix;
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

let mut h = Hamiltonian::new(1);
h.add_term("Z".parse::<PauliString>().unwrap(), 0.7.into()).unwrap();

let time = 0.4_f64;
let exact = h.to_evolution_circuit(time, 1, TrotterMode::FirstOrder).unwrap();
let trotter = h.to_trotter_circuit(time, 1, TrotterMode::FirstOrder).unwrap();

let m_exact = circuit_to_matrix(&exact, None).unwrap();
let m_trotter = circuit_to_matrix(&trotter, None).unwrap();
for (a, b) in m_exact.iter().zip(m_trotter.iter()) {
    assert!((a - b).norm() < 1e-12);
}
```

Matrix form of the exact path:

```rust
use cqlib_core::circuit::circuit_to_matrix;
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// H = 0.5*Z，t = π，U = e^{-i*0.5*π*Z} = diag(-i, i)
let mut h = Hamiltonian::new(1);
h.add_term("Z".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

let circuit = h
    .to_evolution_circuit(std::f64::consts::PI, 1, TrotterMode::FirstOrder)
    .unwrap();
let matrix = circuit_to_matrix(&circuit, None).unwrap();

assert!((matrix[[0, 0]] - num_complex::Complex64::new(0.0, -1.0)).norm() < 1e-12);
assert!((matrix[[1, 1]] - num_complex::Complex64::new(0.0, 1.0)).norm() < 1e-12);
```

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::InvalidParameterValue` | `steps` is 0, or the term list of the Hamiltonian is empty. |
| `QisError::NotHermitian` | A coefficient whose imaginary part still exceeds the tolerance remains after simplification. |
| `QisError::UnsupportedOperation` | An error occurred during circuit construction, for example a Pauli string with phase $\pm i$ is present among the terms. |
| `CircuitError::QubitCountMismatch` | The length of `qubits` in `pauli_evolution()` is inconsistent with the qubit count of the Pauli string. |
| `CircuitError::InvalidOperation` | The phase of the Pauli string in `pauli_evolution()` is $\pm i$. |

The checks on `steps` and on an empty Hamiltonian happen first, so `to_evolution_circuit()` still requires `steps` to be greater than 0 even when it will take the exact path.

Example:

```rust
use cqlib_core::qis::Hamiltonian;
use cqlib_core::qis::evolution::TrotterMode;

let h = Hamiltonian::new(2);
assert!(h.to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder).is_err());

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
assert!(h.to_trotter_circuit(1.0, 0, TrotterMode::FirstOrder).is_err());
assert!(h.to_evolution_circuit(1.0, 0, TrotterMode::FirstOrder).is_err());
```

---

## Related pages

- [QIS Overview](0_overview.md): module overview and terminology.
- [Hamiltonian and Observable](7_hamiltonian.md): construction of terms, simplification, commutation checks and expectation values.
- [Pauli Operators and Pauli Strings](6_pauli.md): phase values and string parsing conventions.
