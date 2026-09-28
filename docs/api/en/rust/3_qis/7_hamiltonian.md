# Hamiltonian and Observable

`cqlib_core::qis::hamiltonian`  
`cqlib_core::qis::observable`

This page covers the observable `Hamiltonian`, formed by Pauli strings and complex coefficients, and the unified interface `Observable` trait for computing expectation values.

`Hamiltonian` is a sparse representation of a $2^N \times 2^N$ matrix, of the form $H = \sum_k c_k P_k$, where $c_k$ is a complex coefficient and $P_k$ is an $N$-qubit Pauli string. The same structure serves both as the Hamiltonian of a physical interpretation and as an arbitrary observable whose expectation value is needed.

## Import

```rust
use cqlib_core::qis::hamiltonian::Hamiltonian;
use cqlib_core::qis::observable::Observable;
```

Both types are also re-exported at the top level of `cqlib_core::qis`.

---

## Hamiltonian

```rust
pub struct Hamiltonian {
    pub num_qubits: usize,
    pub terms: Vec<(PauliString, Complex64)>,
}
```

Fields:

| Field | Type | Description |
| --- | --- | --- |
| `num_qubits` | `usize` | The number of qubits the observable acts on. |
| `terms` | `Vec<(PauliString, Complex64)>` | The term list; each term consists of a Pauli string and a complex coefficient. |

### Construction

- `fn new(num_qubits: usize) -> Self`: create the zero operator for the given number of qubits, with an empty term list.
- `fn from_pauli(pauli: PauliString) -> Self`: construct from a single Pauli string with coefficient `1.0`. Equivalent to the `Into` conversion of `PauliString`.
- `fn from_list(ops: Vec<(PauliString, Complex64)>) -> Result<Self, QisError>`: construct from a term list. An empty list returns the zero operator with `num_qubits` equal to 0; when the terms have inconsistent qubit counts, `QisError::QubitMismatch` is returned. Equivalent to `TryFrom<Vec<(PauliString, Complex64)>>`.
- `fn add_term(&mut self, op: PauliString, coeff: Complex64) -> Result<(), QisError>`: append one term. When the qubit count of `op` is inconsistent with `num_qubits`, `QisError::QubitMismatch` is returned.

### Simplification and scaling

- `fn simplify(&mut self)`: simplify the term list. The operation has two steps: absorb the internal phase of each `PauliString` into the coefficient, normalizing all Pauli strings to `Phase::Plus`; then group by the X and Z component vectors and merge the coefficients of identical operators. Coefficients whose absolute value is smaller than $10^{-10}$ are discarded.
- `fn scale(&mut self, factor: Complex64)`: multiply the coefficients of all terms by the same complex factor.

### Queries and conversion

- `fn all_terms_commute(&self) -> bool`: determine whether all Pauli terms commute pairwise. When true, $e^{-iHt}$ can be decomposed term by term into an exact evolution.
- `fn to_matrix(&self) -> Result<Array2<Complex64>, QisError>`: return a dense $2^N \times 2^N$ matrix. When the qubit count of any term is inconsistent with `num_qubits`, `QisError::QubitMismatch` is returned. An empty operator returns the zero matrix of the corresponding dimension.
- `fn to_trotter_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`: see [Trotter Evolution](8_evolution.md).
- `fn to_evolution_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`: see [Trotter Evolution](8_evolution.md).

### Other behavior

- `Display`: outputs `(coefficient) * Pauli string` term by term, with the coefficient in the display form of a complex number and terms joined by ` + `. An empty term list outputs an empty string.
- `Add`: concatenates the term lists of two Hamiltonians directly, without merging; panics when the two sides have different qubit counts. When identical terms must be merged, call `simplify()` after the addition.
- `From<PauliString>`, `TryFrom<Vec<(PauliString, Complex64)>>`: correspond to `from_pauli()` and `from_list()` respectively.
- Implements `Observable`, see below.
- Supports `Debug`, `Clone`, `PartialEq`.

Example:

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
h.add_term("XX".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();

assert_eq!(h.terms.len(), 2);
// ZZ 与 XX 在每个量子比特上都同时反交换，整体相乘后相互抵消，因此两者对易
assert!(h.all_terms_commute());
```

Switching to terms whose commutation differs on each qubit makes the result false:

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};

let mut h = Hamiltonian::new(2);
h.add_term("XI".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
h.add_term("ZI".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();

// 同一量子比特上的 X 与 Z 反交换
assert!(!h.all_terms_commute());
```

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};
use num_complex::Complex64;

// 同一 Pauli 串的两项会被合并，系数相加
let mut h = Hamiltonian::from_list(vec![
    ("ZZ".parse::<PauliString>().unwrap(), 0.5.into()),
    ("ZZ".parse::<PauliString>().unwrap(), 0.3.into()),
])
.unwrap();

h.simplify();

assert_eq!(h.terms.len(), 1);
assert!((h.terms[0].1 - Complex64::new(0.8, 0.0)).norm() < 1e-10);
```

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};

// ZZ 与 IZ 对易，演化可以按项精确分解
let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse().unwrap(), 1.0.into()).unwrap();
h.add_term("IZ".parse().unwrap(), 0.5.into()).unwrap();

assert!(h.all_terms_commute());
```

Dense matrix:

```rust
use cqlib_core::qis::{Hamiltonian, PauliString, Phase};
use num_complex::Complex64;

let mut x: PauliString = "X".parse().unwrap();
x.phase = Phase::Minus;
let z: PauliString = "Z".parse().unwrap();

let hamiltonian = Hamiltonian::from_list(vec![
    (x, Complex64::new(2.0, 0.0)),
    (z, Complex64::new(0.0, 1.0)),
])
.unwrap();

assert_eq!(
    hamiltonian.to_matrix().unwrap(),
    ndarray::arr2(&[
        [Complex64::new(0.0, 1.0), Complex64::new(-2.0, 0.0)],
        [Complex64::new(-2.0, 0.0), Complex64::new(0.0, -1.0)],
    ])
);
```

---

## Observable

The unified interface for computing the expectation value of an observable, used to evaluate the same observable on different quantum state representations.

```rust
pub trait Observable {
    fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError>;
    fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError>;
    fn expectation_probs(
        &self,
        measurements: &[(PauliString, HashMap<String, f64>)],
    ) -> Result<f64, QisError>;
    fn num_qubits(&self) -> usize;
    fn variance_statevector(&self, _sv: &Statevector) -> Result<f64, QisError> {
        Err(QisError::UnsupportedOperation(
            "variance_statevector not implemented for this observable type".into(),
        ))
    }
}
```

### Required methods

- `fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError>`: compute the pure-state expectation value $\langle \psi | O | \psi \rangle$. When the qubit count of the state is inconsistent with `num_qubits()`, `QisError::QubitMismatch` is returned.
- `fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError>`: compute the mixed-state expectation value $\text{Tr}(\rho O)$. When the qubit counts are inconsistent, `QisError::QubitMismatch` is returned.
- `fn expectation_probs(&self, measurements: &[(PauliString, HashMap<String, f64>)]) -> Result<f64, QisError>`: compute the expectation value from measurement probabilities. Each element is a measurement basis together with a map from state string to probability. State strings are written in little-endian, with the rightmost character corresponding to qubit 0.
- `fn num_qubits(&self) -> usize`: return the number of qubits the observable acts on, used for the dimension check before computing an expectation value.

### Default methods

- `fn variance_statevector(&self, sv: &Statevector) -> Result<f64, QisError>`: compute the pure-state variance $\text{Var}(O) = \langle O^2 \rangle - \langle O \rangle^2$. The default implementation in the trait returns `QisError::UnsupportedOperation`; an observable that supports variance computation must override this method.

### Implementors

| Type | Description |
| --- | --- |
| `Hamiltonian` | Computes term by term and accumulates by coefficient, for a multi-term Hamiltonian. |
| `PauliString` | A single-term Pauli string, for direct measurement of a single observable. |

### Measurement basis matching of `expectation_probs`

For every non-identity term, the interface looks for a compatible measurement basis in the given measurement list: a measurement basis is considered compatible if its X and Z components at the non-identity positions of the term agree with those of the term. Identity terms contribute their coefficient and global phase directly and need no measurement basis.

When no compatible measurement basis is found, `QisError::UnsupportedOperation` is returned. The implementations of `Hamiltonian` and `PauliString` both match by this rule, with slight differences in the error branches related to state strings:

| Case | `Hamiltonian` | `PauliString` |
| --- | --- | --- |
| State string length inconsistent with the qubit count | `QisError::DimensionMismatch` | `QisError::QubitMismatch` |
| State string contains a character other than `0` and `1` | `QisError::PauliStringParseError` | `QisError::UnsupportedOperation` |
| No compatible measurement basis found | `QisError::UnsupportedOperation` | `QisError::UnsupportedOperation` |

The implementation of `Hamiltonian` ignores the phase introduced by Y components in a term and counts only the global phase and the coefficient.

### Example

Pure-state expectation value on a Bell state:

```rust
use cqlib_core::qis::{Hamiltonian, Observable, PauliString, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

let ps: PauliString = "ZZ".parse().unwrap();
let h = Hamiltonian::from_pauli(ps);

assert!((h.expectation_statevector(&sv).unwrap() - 1.0).abs() < 1e-10);
assert_eq!(h.num_qubits(), 2);
```

Computing an expectation value from measurement probabilities. All three Z terms are compatible with the same ZZ measurement basis:

```rust
use cqlib_core::qis::{Hamiltonian, Observable, PauliString};
use std::collections::HashMap;

let mut h = Hamiltonian::new(2);
let mut z1 = PauliString::new(2);
z1.set_pauli(1, cqlib_core::qis::pauli::Pauli::Z);
h.add_term(z1, 2.0.into()).unwrap();

let mut z0 = PauliString::new(2);
z0.set_pauli(0, cqlib_core::qis::pauli::Pauli::Z);
h.add_term(z0, 3.0.into()).unwrap();

let mut zz = PauliString::new(2);
zz.set_pauli(0, cqlib_core::qis::pauli::Pauli::Z);
zz.set_pauli(1, cqlib_core::qis::pauli::Pauli::Z);
h.add_term(zz.clone(), 4.0.into()).unwrap();

let mut probs = HashMap::new();
probs.insert("10".to_string(), 1.0);

let measurements = vec![(zz, probs)];
assert!((h.expectation_probs(&measurements).unwrap() - (-3.0)).abs() < 1e-10);
```

Variance:

```rust
use cqlib_core::qis::{Hamiltonian, Observable, PauliString, Statevector};

// |+> 态上 Z 的方差为 1，系数 2.0 的平方给出 4.0
let mut sv = Statevector::new(1);
sv.apply_h(0).unwrap();

let mut z = PauliString::new(1);
z.set_pauli(0, cqlib_core::qis::pauli::Pauli::Z);

let mut h = Hamiltonian::new(1);
h.add_term(z, 2.0.into()).unwrap();

assert!((h.variance_statevector(&sv).unwrap() - 4.0).abs() < 1e-10);
```

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::QubitMismatch` | When constructing or appending a term, the qubit count of the Pauli string is inconsistent with `num_qubits`; when computing an expectation value or variance, the qubit count of the state is inconsistent with that of the observable. |
| `QisError::NotHermitian` | The observable is not Hermitian when computing a variance, for example the phase of a `PauliString` is `±i`, or a coefficient whose imaginary part still exceeds the tolerance remains after simplification. |
| `QisError::UnsupportedOperation` | An observable that does not override `variance_statevector()` calls that method; `expectation_probs()` finds no compatible measurement basis. |
| `QisError::DimensionMismatch` | In `Hamiltonian::expectation_probs()`, the state string length is inconsistent with the qubit count. |
| `QisError::PauliStringParseError` | In `Hamiltonian::expectation_probs()`, a state string contains a character other than `0` and `1`. |

`Add` panics when the qubit counts of the two sides are inconsistent, rather than returning an error.

Variance computation tolerates a certain amount of numerical noise: a small negative value near zero is truncated to 0, while a negative value beyond the tolerance returns `QisError::InvalidParameterValue` instead.

---

## Related pages

- [QIS Overview](0_overview.md): module overview and terminology.
- [Pauli Operators and Pauli Strings](6_pauli.md): construction of terms, commutation and symplectic encoding.
- [Trotter Evolution](8_evolution.md): convert a Hamiltonian into an evolution circuit.
