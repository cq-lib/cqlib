# Metrics and Entropy

`cqlib_core::qis::metrics`  
`cqlib_core::qis::entropy`

This page covers the metrics and information measures between quantum states. `metrics` collects purity, fidelity, trace distance, von Neumann entropy and partial transpose related quantities; `entropy` collects linear entropy, Rényi entropy, and the measures aimed at entanglement. The functions of both modules take a statevector or a density matrix as input and return `Result<f64, QisError>` (except `density_matrix_to_faer`).

## Import

```rust
use cqlib_core::qis::metrics::{
    density_matrix_to_faer, entropy, logarithmic_negativity, partial_transpose, purity_mixed,
    purity_pure, state_fidelity_mixed, state_fidelity_pure, state_fidelity_pure_mixed,
    trace_distance_mixed, trace_distance_pure,
};
use cqlib_core::qis::entropy::{
    concurrence, entanglement_entropy_pure, entanglement_of_formation, linear_entropy, negativity,
    renyi_entropy,
};
```

Both modules are public under `cqlib_core::qis`; `metrics` and `entropy` themselves are not re-exported at the top level.

---

## Purity

### `purity_pure(sv: &Statevector) -> Result<f64, QisError>`

Return the squared norm $\langle \psi | \psi \rangle$ of the statevector. A normalized statevector gives `1.0`, so this function can also serve as a numerical check of normalization.

### `purity_mixed(dm: &DensityMatrix) -> Result<f64, QisError>`

Return the purity $\text{Tr}(\rho^2)$ of the density matrix. Since $\rho$ is a Hermitian matrix, the implementation sums the squared moduli of all elements. The value falls between $1/2^N$ (maximally mixed state) and `1.0` (pure state).

Example:

```rust
use cqlib_core::qis::state::{DensityMatrix, Statevector};
use cqlib_core::qis::metrics::{purity_mixed, purity_pure};

let sv = Statevector::new(2);
assert!((purity_pure(&sv).unwrap() - 1.0).abs() < 1e-10);

// 单比特完全混合态 Tr((I/2)^2) = 0.5
let dm = DensityMatrix::maximally_mixed(1);
assert!((purity_mixed(&dm).unwrap() - 0.5).abs() < 1e-10);
```

---

## Fidelity

### `state_fidelity_pure(sv1: &Statevector, sv2: &Statevector) -> Result<f64, QisError>`

The fidelity between two pure states, $F(\psi, \phi) = |\langle \psi | \phi \rangle|^2$, ranging from `0.0` (orthogonal) to `1.0` (the same state). When the qubit counts are inconsistent, `QisError::QubitMismatch` is returned.

### `state_fidelity_pure_mixed(sv: &Statevector, dm: &DensityMatrix) -> Result<f64, QisError>`

The fidelity between a pure state and a mixed state, $F(\psi, \rho) = \langle \psi | \rho | \psi \rangle$. When the qubit counts are inconsistent, `QisError::QubitMismatch` is returned.

### `state_fidelity_mixed(dm1: &DensityMatrix, dm2: &DensityMatrix) -> Result<f64, QisError>`

The fidelity between two mixed states, $F(\rho, \sigma) = \left(\text{Tr}\sqrt{\sqrt{\rho}\,\sigma\sqrt{\rho}}\right)^2$. When the qubit counts are inconsistent, `QisError::QubitMismatch` is returned; when the eigendecomposition fails, `QisError::UnsupportedOperation` is returned.

Example:

```rust
use cqlib_core::qis::state::{DensityMatrix, Statevector};
use cqlib_core::qis::metrics::{state_fidelity_pure, state_fidelity_pure_mixed};

let sv0 = Statevector::new(1);

// |1> 态
let mut sv1 = Statevector::new(1);
sv1.data_mut()[0] = num_complex::Complex64::new(0.0, 0.0);
sv1.data_mut()[1] = num_complex::Complex64::new(1.0, 0.0);

assert!((state_fidelity_pure(&sv0, &sv0).unwrap() - 1.0).abs() < 1e-10);
assert!((state_fidelity_pure(&sv0, &sv1).unwrap() - 0.0).abs() < 1e-10);

// 纯态 |0> 与完全混合态的保真度为 0.5
let dm_mixed = DensityMatrix::maximally_mixed(1);
assert!((state_fidelity_pure_mixed(&sv0, &dm_mixed).unwrap() - 0.5).abs() < 1e-10);
```

---

## Trace distance

### `trace_distance_pure(sv1: &Statevector, sv2: &Statevector) -> Result<f64, QisError>`

The trace distance between two pure states, $D(\psi, \phi) = \sqrt{1 - |\langle \psi | \phi \rangle|^2}$, ranging from `0.0` (the same state) to `1.0` (orthogonal). The fidelity is computed internally first, so an inconsistent qubit count likewise returns `QisError::QubitMismatch`.

### `trace_distance_mixed(dm1: &DensityMatrix, dm2: &DensityMatrix) -> Result<f64, QisError>`

The trace distance between two mixed states, $D(\rho, \sigma) = \frac{1}{2}\text{Tr}|\rho - \sigma|$. When both traces are 1, the value equals the sum of the positive eigenvalues of $\rho - \sigma$. When the qubit counts are inconsistent, `QisError::QubitMismatch` is returned.

Example:

```rust
use cqlib_core::qis::state::{DensityMatrix, Statevector};
use cqlib_core::qis::metrics::{trace_distance_mixed, trace_distance_pure};

let sv0 = Statevector::new(1);

let mut sv1 = Statevector::new(1);
sv1.data_mut()[0] = num_complex::Complex64::new(0.0, 0.0);
sv1.data_mut()[1] = num_complex::Complex64::new(1.0, 0.0);

assert!((trace_distance_pure(&sv0, &sv0).unwrap() - 0.0).abs() < 1e-10);
assert!((trace_distance_pure(&sv0, &sv1).unwrap() - 1.0).abs() < 1e-10);

// |0><0| 与 |1><1| 的迹距离为 1
let dm0 = DensityMatrix::new(1);
let dm1 = DensityMatrix::from_density_matrix_state(
    1,
    vec![
        num_complex::Complex64::new(0.0, 0.0),
        num_complex::Complex64::new(0.0, 0.0),
        num_complex::Complex64::new(0.0, 0.0),
        num_complex::Complex64::new(1.0, 0.0),
    ],
)
.unwrap();

assert!((trace_distance_mixed(&dm0, &dm1).unwrap() - 1.0).abs() < 1e-10);
```

---

## Entropy

`entropy` belongs to `metrics`, while `linear_entropy` and `renyi_entropy` belong to `entropy`.

### `entropy(dm: &DensityMatrix) -> Result<f64, QisError>`

The von Neumann entropy $S(\rho) = -\text{Tr}(\rho \log_2 \rho)$, in units of bits. Eigenvalues whose absolute value is smaller than $10^{-12}$ are ignored during the computation, in order to filter out numerical noise. When the eigendecomposition fails, `QisError::UnsupportedOperation` is returned.

### `linear_entropy(dm: &DensityMatrix) -> Result<f64, QisError>`

The linear entropy $S_L(\rho) = 1 - \text{Tr}(\rho^2)$. It is `0.0` for a pure state and $1 - 1/2^N$ for an $N$-qubit maximally mixed state.

### `renyi_entropy(dm: &DensityMatrix, alpha: f64) -> Result<f64, QisError>`

The Rényi entropy $S_\alpha(\rho) = \frac{1}{1-\alpha}\log_2\left(\sum_i \lambda_i^\alpha\right)$, where $\lambda_i$ are the eigenvalues of the density matrix. When $\alpha$ is not greater than 0, `QisError::InvalidParameterValue` is returned; when the difference between $\alpha$ and 1 is smaller than `f64::EPSILON`, the von Neumann entropy is used instead, avoiding division by zero. The result is truncated to a non-negative value.

Example:

```rust
use cqlib_core::qis::state::DensityMatrix;
use cqlib_core::qis::metrics::entropy;
use cqlib_core::qis::entropy::{linear_entropy, renyi_entropy};

// 纯态熵为 0
let pure = DensityMatrix::new(1);
assert!(entropy(&pure).unwrap().abs() < 1e-10);
assert!(linear_entropy(&pure).unwrap().abs() < 1e-10);

// 单比特完全混合态的冯·诺依曼熵为 1 比特，线性熵为 0.5
let mixed = DensityMatrix::maximally_mixed(1);
assert!((entropy(&mixed).unwrap() - 1.0).abs() < 1e-10);
assert!((linear_entropy(&mixed).unwrap() - 0.5).abs() < 1e-10);

// Rényi 熵在 α 趋向 1 时退化为冯·诺依曼熵
assert!((renyi_entropy(&mixed, 1.0).unwrap() - 1.0).abs() < 1e-10);
assert!((renyi_entropy(&mixed, 2.0).unwrap() - 1.0).abs() < 1e-10);
assert!(renyi_entropy(&mixed, -1.0).is_err());
```

---

## Entanglement measures

`entanglement_entropy_pure`, `negativity`, `concurrence` and `entanglement_of_formation` belong to `entropy`, while `partial_transpose` and `logarithmic_negativity` belong to `metrics`.

### `entanglement_entropy_pure(sv: &Statevector, subsys_a: &[usize]) -> Result<f64, QisError>`

The entanglement entropy of a bipartite pure state, that is the von Neumann entropy of the reduced density matrix of subsystem A, $E(|\psi\rangle) = S(\rho_A)$. `subsys_a` gives the qubit indices of subsystem A, and the remaining qubits form subsystem B.

`QisError::InvalidSubsystem` is returned in the following cases: `subsys_a` is empty, contains all qubits, or has duplicate indices. When an index is out of range, `QisError::IndexOutOfBounds` is returned.

### `partial_transpose(dm: &DensityMatrix, target_qubits: &[usize]) -> Result<DensityMatrix, QisError>`

Perform a partial transpose on the given subsystem, transposing only the indices corresponding to `target_qubits` and leaving the remaining qubits unchanged. The matrix is flattened in row-major order, with the index convention `idx = (ket << n) | bra`. When a target index is out of range, `QisError::IndexOutOfBounds` is returned.

### `negativity(dm: &DensityMatrix, subsys_a: &[usize]) -> Result<f64, QisError>`

The negativity, taken as the sum of the absolute values of the negative eigenvalues after the partial transpose, $N(\rho) = \frac{1}{2}\left(\|\rho^{T_A}\|_1 - 1\right)$. On $2 \otimes 2$ and $2 \otimes 3$ systems, a value of 0 is a necessary and sufficient condition for separability. Unlike the logarithmic negativity, this quantity is not additive under the tensor product.

### `logarithmic_negativity(dm: &DensityMatrix, sys_a: &[usize]) -> Result<f64, QisError>`

The logarithmic negativity $E_N(\rho) = \log_2 \|\rho^{T_A}\|_1$, an entanglement measure that is 0 for separable states and greater than 0 for entangled states, and also an upper bound on the distillable entanglement. The implementation performs the partial transpose first, then sums the absolute values of the eigenvalues and takes the base-2 logarithm, truncating the result below at 0.

### `concurrence(dm: &DensityMatrix) -> Result<f64, QisError>`

The concurrence of a two-qubit system, $C(\rho) = \max(0, \lambda_1 - \lambda_2 - \lambda_3 - \lambda_4)$, where $\lambda_i$ are the eigenvalues of $\sqrt{\sqrt{\rho}\,\tilde{\rho}\sqrt{\rho}}$ in descending order and $\tilde{\rho}$ is the spin-flipped density matrix. The value ranges from `0.0` (separable) to `1.0` (maximally entangled). When `dm` is not two-qubit, `QisError::UnsupportedDimension` is returned.

### `entanglement_of_formation(dm: &DensityMatrix) -> Result<f64, QisError>`

The entanglement of formation, the binary entropy derived from the concurrence. It is `0.0` for separable states and `1.0` for maximally entangled states. A non-two-qubit input likewise returns `QisError::UnsupportedDimension`.

Example:

```rust
use cqlib_core::qis::state::{DensityMatrix, Statevector};
use cqlib_core::qis::entropy::{
    concurrence, entanglement_entropy_pure, entanglement_of_formation, negativity,
};
use cqlib_core::qis::metrics::logarithmic_negativity;

// Bell 态 |Φ+⟩ 的纠缠熵为 1 比特
let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();
assert!((entanglement_entropy_pure(&sv, &[0]).unwrap() - 1.0).abs() < 1e-10);

// 同一状态的多种纠缠度量
let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

assert!((negativity(&dm, &[0]).unwrap() - 0.5).abs() < 1e-10);
assert!((logarithmic_negativity(&dm, &[0]).unwrap() - 1.0).abs() < 1e-10);
assert!((concurrence(&dm).unwrap() - 1.0).abs() < 1e-10);
assert!((entanglement_of_formation(&dm).unwrap() - 1.0).abs() < 1e-10);
```

All measures vanish on a separable state:

```rust
use cqlib_core::qis::state::{DensityMatrix, Statevector};
use cqlib_core::qis::entropy::{concurrence, entanglement_entropy_pure, negativity};
use cqlib_core::qis::metrics::logarithmic_negativity;

// |+0> = |+> ⊗ |0>，两个比特之间没有纠缠
let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
assert!(entanglement_entropy_pure(&sv, &[0]).unwrap().abs() < 1e-10);

let dm = DensityMatrix::new(2);
assert!(negativity(&dm, &[0]).unwrap().abs() < 1e-10);
assert!(logarithmic_negativity(&dm, &[0]).unwrap().abs() < 1e-10);
assert!(concurrence(&dm).unwrap().abs() < 1e-10);
```

Partial transpose:

```rust
use cqlib_core::qis::state::DensityMatrix;
use cqlib_core::qis::metrics::partial_transpose;
use num_complex::Complex64;

// Bell 态密度矩阵：|00><00| + |00><11| + |11><00| + |11><11|，各占 0.5
let mut data = vec![Complex64::new(0.0, 0.0); 16];
data[0] = Complex64::new(0.5, 0.0);
data[3] = Complex64::new(0.5, 0.0);
data[12] = Complex64::new(0.5, 0.0);
data[15] = Complex64::new(0.5, 0.0);
let dm = DensityMatrix::from_density_matrix_state(2, data).unwrap();

// 偏转置后 |00><11| 被换到 |01><10| 的位置
let pt = partial_transpose(&dm, &[0]).unwrap();
assert!((pt.data()[0] - Complex64::new(0.5, 0.0)).norm() < 1e-10);
assert!((pt.data()[15] - Complex64::new(0.5, 0.0)).norm() < 1e-10);

// 下标越界
assert!(partial_transpose(&dm, &[2]).is_err());
```

---

## Helper conversion

### `density_matrix_to_faer(dm: &DensityMatrix) -> Mat<c64>`

Convert a density matrix into the matrix type of the eigendecomposition library, for scenarios that need to call the eigendecomposition directly. The input is flattened in row-major order and the dimension is $2^N$. This function does not return a `Result` and is the low-level step shared by the other functions that rely on eigendecomposition.

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::QubitMismatch` | The two states involved in the operation have inconsistent qubit counts. |
| `QisError::InvalidParameterValue` | `alpha` of `renyi_entropy()` is not greater than 0. |
| `QisError::InvalidSubsystem` | The subsystem of `entanglement_entropy_pure()` is empty, contains all qubits, or has duplicate indices. |
| `QisError::IndexOutOfBounds` | A target index of `partial_transpose()` is out of range, or a subsystem index of `entanglement_entropy_pure()` is out of range. |
| `QisError::UnsupportedDimension` | The input of `concurrence()` and `entanglement_of_formation()` is not two-qubit. |
| `QisError::UnsupportedOperation` | The eigendecomposition fails. |

`trace_distance_pure()` returns `QisError::QubitMismatch` indirectly through `state_fidelity_pure()`; `negativity()` and `logarithmic_negativity()` return index-related errors indirectly through `partial_transpose()`.

---

## Related pages

- [QIS Overview](0_overview.md): module overview and terminology.
- [Hamiltonian and Observable](7_hamiltonian.md): compute observable expectation values from a state, usable together with the measures on this page.
