# 度量与熵

`cqlib_core::qis::metrics`  
`cqlib_core::qis::entropy`

本页介绍量子态之间的度量与信息量函数。`metrics` 集合纯度、保真度、迹距离、冯·诺依曼熵与偏转置相关的量；`entropy` 集合线性熵、Rényi 熵，以及针对纠缠的度量。两模块的函数都以态矢量或密度矩阵为输入，返回 `Result<f64, QisError>`（`density_matrix_to_faer` 除外）。

## 导入

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

两个模块都在 `cqlib_core::qis` 下公开，`metrics` 与 `entropy` 本身不在顶层重导出。

---

## 纯度

### `purity_pure(sv: &Statevector) -> Result<f64, QisError>`

返回态矢量的模方 $\langle \psi | \psi \rangle$。归一化的态矢量取值为 `1.0`，因此该函数也可以作为归一化程度的数值校验。

### `purity_mixed(dm: &DensityMatrix) -> Result<f64, QisError>`

返回密度矩阵的纯度 $\text{Tr}(\rho^2)$。$\rho$ 为 Hermitian 矩阵，实现上按全部元素模方求和。取值落在 $1/2^N$（完全混合态）与 `1.0`（纯态）之间。

示例：

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

## 保真度

### `state_fidelity_pure(sv1: &Statevector, sv2: &Statevector) -> Result<f64, QisError>`

两个纯态之间的保真度 $F(\psi, \phi) = |\langle \psi | \phi \rangle|^2$，取值 `0.0`（正交）到 `1.0`（同一态）。比特数不一致时返回 `QisError::QubitMismatch`。

### `state_fidelity_pure_mixed(sv: &Statevector, dm: &DensityMatrix) -> Result<f64, QisError>`

纯态与混合态之间的保真度 $F(\psi, \rho) = \langle \psi | \rho | \psi \rangle$。比特数不一致时返回 `QisError::QubitMismatch`。

### `state_fidelity_mixed(dm1: &DensityMatrix, dm2: &DensityMatrix) -> Result<f64, QisError>`

两个混合态之间的保真度 $F(\rho, \sigma) = \left(\text{Tr}\sqrt{\sqrt{\rho}\,\sigma\sqrt{\rho}}\right)^2$。比特数不一致时返回 `QisError::QubitMismatch`，特征分解失败时返回 `QisError::UnsupportedOperation`。

示例：

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

## 迹距离

### `trace_distance_pure(sv1: &Statevector, sv2: &Statevector) -> Result<f64, QisError>`

两个纯态之间的迹距离 $D(\psi, \phi) = \sqrt{1 - |\langle \psi | \phi \rangle|^2}$，取值 `0.0`（同一态）到 `1.0`（正交）。内部先求保真度，因此比特数不一致时同样返回 `QisError::QubitMismatch`。

### `trace_distance_mixed(dm1: &DensityMatrix, dm2: &DensityMatrix) -> Result<f64, QisError>`

两个混合态之间的迹距离 $D(\rho, \sigma) = \frac{1}{2}\text{Tr}|\rho - \sigma|$。两者迹都为 1 时，该值等于 $\rho - \sigma$ 的正特征值之和。比特数不一致时返回 `QisError::QubitMismatch`。

示例：

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

## 熵

`entropy` 属于 `metrics`，`linear_entropy` 与 `renyi_entropy` 属于 `entropy`。

### `entropy(dm: &DensityMatrix) -> Result<f64, QisError>`

冯·诺依曼熵 $S(\rho) = -\text{Tr}(\rho \log_2 \rho)$，以比特为单位。计算时忽略绝对值小于 $10^{-12}$ 的特征值，以滤除数值噪声。特征分解失败时返回 `QisError::UnsupportedOperation`。

### `linear_entropy(dm: &DensityMatrix) -> Result<f64, QisError>`

线性熵 $S_L(\rho) = 1 - \text{Tr}(\rho^2)$。纯态为 `0.0`，$N$ 比特完全混合态为 $1 - 1/2^N$。

### `renyi_entropy(dm: &DensityMatrix, alpha: f64) -> Result<f64, QisError>`

Rényi 熵 $S_\alpha(\rho) = \frac{1}{1-\alpha}\log_2\left(\sum_i \lambda_i^\alpha\right)$，$\lambda_i$ 为密度矩阵的特征值。$\alpha$ 不大于 0 时返回 `QisError::InvalidParameterValue`；$\alpha$ 与 1 的差小于 `f64::EPSILON` 时改用冯·诺依曼熵，避免除零。结果截断为非负值。

示例：

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

## 纠缠度量

`entanglement_entropy_pure`、`negativity`、`concurrence` 与 `entanglement_of_formation` 属于 `entropy`，`partial_transpose` 与 `logarithmic_negativity` 属于 `metrics`。

### `entanglement_entropy_pure(sv: &Statevector, subsys_a: &[usize]) -> Result<f64, QisError>`

二分纯态的纠缠熵，即子系统 A 的约化密度矩阵的冯·诺依曼熵 $E(|\psi\rangle) = S(\rho_A)$。`subsys_a` 给出子系统 A 的比特下标，其余比特组成子系统 B。

以下情况返回 `QisError::InvalidSubsystem`：`subsys_a` 为空、包含全部比特，或存在重复下标。下标越界时返回 `QisError::IndexOutOfBounds`。

### `partial_transpose(dm: &DensityMatrix, target_qubits: &[usize]) -> Result<DensityMatrix, QisError>`

对指定子系统做偏转置，只转置 `target_qubits` 对应的下标，其余比特保持不变。矩阵按行优先展平，下标约定为 `idx = (ket << n) | bra`。目标下标越界时返回 `QisError::IndexOutOfBounds`。

### `negativity(dm: &DensityMatrix, subsys_a: &[usize]) -> Result<f64, QisError>`

负度，取偏转置后负特征值绝对值之和 $N(\rho) = \frac{1}{2}\left(\|\rho^{T_A}\|_1 - 1\right)$。在 $2 \otimes 2$ 与 $2 \otimes 3$ 系统上，取值为 0 是可分性的充要条件。与对数负度不同，该量在张量积下不满足可加性。

### `logarithmic_negativity(dm: &DensityMatrix, sys_a: &[usize]) -> Result<f64, QisError>`

对数负度 $E_N(\rho) = \log_2 \|\rho^{T_A}\|_1$，即可分态下为 0、纠缠态下大于 0 的纠缠度量，也是对可提纯纠缠量的上界。实现上先做偏转置，再把特征值绝对值求和后取以 2 为底的对数，结果下限截断为 0。

### `concurrence(dm: &DensityMatrix) -> Result<f64, QisError>`

双比特系统的 concurrence $C(\rho) = \max(0, \lambda_1 - \lambda_2 - \lambda_3 - \lambda_4)$，$\lambda_i$ 为 $\sqrt{\sqrt{\rho}\,\tilde{\rho}\sqrt{\rho}}$ 按降序排列的特征值，$\tilde{\rho}$ 为自旋翻转后的密度矩阵。取值 `0.0`（可分）到 `1.0`（最大纠缠）。`dm` 不是双比特时返回 `QisError::UnsupportedDimension`。

### `entanglement_of_formation(dm: &DensityMatrix) -> Result<f64, QisError>`

纠缠生成度，由 concurrence 导出的二元熵。可分态为 `0.0`，最大纠缠态为 `1.0`。非双比特输入同样返回 `QisError::UnsupportedDimension`。

示例：

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

可分态上各度量都归零：

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

偏转置：

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

## 辅助转换

### `density_matrix_to_faer(dm: &DensityMatrix) -> Mat<c64>`

把密度矩阵转换为特征分解库的矩阵类型，供需要直接调用特征分解的场景使用。输入按行优先展平，维度取 $2^N$。该函数不返回 `Result`，是其余依赖特征分解的函数所共用的底层步骤。

---

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::QubitMismatch` | 参与运算的两个态比特数不一致。 |
| `QisError::InvalidParameterValue` | `renyi_entropy()` 的 `alpha` 不大于 0。 |
| `QisError::InvalidSubsystem` | `entanglement_entropy_pure()` 的子系统为空、包含全部比特或存在重复下标。 |
| `QisError::IndexOutOfBounds` | `partial_transpose()` 的目标下标越界，或 `entanglement_entropy_pure()` 的子系统下标越界。 |
| `QisError::UnsupportedDimension` | `concurrence()` 与 `entanglement_of_formation()` 的输入不是双比特。 |
| `QisError::UnsupportedOperation` | 特征分解失败。 |

`trace_distance_pure()` 通过 `state_fidelity_pure()` 间接返回 `QisError::QubitMismatch`；`negativity()` 与 `logarithmic_negativity()` 通过 `partial_transpose()` 间接返回下标相关错误。

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与术语表。
- [Hamiltonian 与 Observable](7_hamiltonian.md)：从态求可观测量期望值，可与本页的度量配合使用。
