# Hamiltonian 与 Observable

`cqlib_core::qis::hamiltonian`
`cqlib_core::qis::observable`

本页介绍由 Pauli 串与复系数构成的算符 `Hamiltonian`，以及计算实数期望值的统一接口 `Observable` trait。

`Hamiltonian` 是 $2^N \times 2^N$ 矩阵的稀疏表示，形式为 $H = \sum_k c_k P_k$，其中 $c_k$ 是复系数，$P_k$ 是 $N$ 比特 Pauli 串。该结构允许非厄米算符；实数期望值和酉演化入口会额外检查厄米性。

## 导入

```rust
use cqlib_core::qis::hamiltonian::Hamiltonian;
use cqlib_core::qis::observable::Observable;
```

两个类型也在 `cqlib_core::qis` 顶层重导出。

---

## Hamiltonian

```rust
pub struct Hamiltonian {
    pub num_qubits: usize,
    pub terms: Vec<(PauliString, Complex64)>,
}
```

字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `num_qubits` | `usize` | 观测量作用的比特数。 |
| `terms` | `Vec<(PauliString, Complex64)>` | 项列表，每项由 Pauli 串与复系数组成。 |

### 构造

- `fn new(num_qubits: usize) -> Self`：创建指定比特数下的零算子，项列表为空。
- `fn from_pauli(pauli: PauliString) -> Self`：用单个 Pauli 串构造，系数取 `1.0`。等价于 `PauliString` 的 `Into` 转换。
- `fn from_list(ops: Vec<(PauliString, Complex64)>) -> Result<Self, QisError>`：从项列表构造。空列表返回 `num_qubits` 为 0 的零算子；各项比特数不一致时返回 `QisError::QubitMismatch`。等价于 `TryFrom<Vec<(PauliString, Complex64)>>`。
- `fn add_term(&mut self, op: PauliString, coeff: Complex64) -> Result<(), QisError>`：追加一项。`op` 的比特数与 `num_qubits` 不一致时返回 `QisError::QubitMismatch`。

### 化简与缩放

- `fn simplify(&mut self)`：化简项列表。该操作分两步：把每个 `PauliString` 的内部相位吸收进系数，使所有 Pauli 串归一到 `Phase::Plus`；再按 X、Z 分量向量分组，合并相同算子的系数。绝对值小于 $10^{-10}$ 的系数会被丢弃。
- `fn scale(&mut self, factor: Complex64)`：把所有项的系数乘以同一个复因子。

### 安全 Pauli 和式代数

以下接口返回 `Result`，验证比特数、X/Z 位向量长度、有限系数及系数乘法/累加结果。返回的算符已吸收相位、按 Z 再 X 向量确定性排序并合并重复项，且只删除精确零；不采用 `simplify()` 的默认截断。

| 接口 | 语义 |
| --- | --- |
| `identity(num_qubits) -> Self` | 指定寄存器上的单位算符；零比特时为标量 1。 |
| `from_terms(num_qubits, terms) -> Result<Self, QisError>` | 从 `(PauliString, Complex64)` 的 `IntoIterator` 安全构造；空输入保留声明宽度。 |
| `try_add(&self, other: &Self)` / `try_sub(&self, other: &Self)` | 同宽度加减，返回 `Result<Self, QisError>`，不修改输入。 |
| `try_mul(&self, other: &Self) -> Result<Self, QisError>` | 矩阵乘积 $AB$，右操作数先作用；允许非厄米结果。 |
| `scaled(&self, factor: Complex64) -> Result<Self, QisError>` | 返回缩放结果；零算符也验证因子。 |
| `adjoint(&self) -> Result<Self, QisError>` | 相位归一化后共轭系数，得到 $A^\dagger$。 |
| `normalized(&self) -> Result<Self, QisError>` | 验证、吸收相位、合并同类项，仅删除精确零。 |
| `chop(&mut self, atol: f64) -> Result<(), QisError>` | 先归并，再删除复数模长 `≤ atol` 的项；失败时原值不变。 |
| `equiv(&self, other: &Self, atol: f64) -> Result<bool, QisError>` | 判断归一化后 $A-B$ 的每个系数模长是否 `≤ atol`。 |
| `is_hermitian(&self, atol: f64) -> Result<bool, QisError>` | 等价于与伴随的 `equiv`，不投影到实数。 |
| `tensor(&self, other: &Self) -> Result<Self, QisError>` | $A\otimes B$，B 占低比特，A 占高比特。 |
| `remap_qubits(&self, mapping: &[usize], target_width: usize) -> Result<Self, QisError>` | `mapping[source] = destination`；支持置换和嵌入，未使用位置补 I。 |

所有容差必须有限且非负。`equiv` 使用逐项系数容差，不是矩阵算符范数界；宽度不同返回 `QubitMismatch`，不会隐式扩展寄存器。`is_hermitian` 的容差衡量 $A-A^\dagger$，归一化后等于虚部幅值的两倍，与 `FermionicOp` 一致。已有 `normalized_hermitian()` 使用独立消费者契约：允许虚部幅值不超过 `1e-10`，将其投影到实数，但保留任何非零实系数。

`remap_qubits` 要求映射长度等于源宽度，目标位置不重复且小于 `target_width`，包括零/单位算符。它不表示费米子模式置换或部分迹。零算符保留寄存器宽度，稀疏运算支持超过 64 个量子比特；乘法可展开成两侧项数的乘积，仍可能产生大量项。

若张量或嵌入结果需要构造 Pauli 项，宽度超过 BitVec 的可表示长度时返回 `UnsupportedOperation`，不会触发长度断言。零算符不需要位向量，可以保留更大的声明宽度。

```rust
use cqlib_core::qis::{Hamiltonian, QisError};
use num_complex::Complex64;

let x = Hamiltonian::from_pauli("X".parse()?);
let z = Hamiltonian::from_pauli("Z".parse()?);
let sum = x.try_add(&z)?;
assert!(sum.try_mul(&sum)?.equiv(&Hamiltonian::identity(1).scaled(2.0.into())?, 0.0)?);

let xz = x.try_mul(&z)?; // XZ = -iY
let minus_i_y = Hamiltonian::from_terms(1, [("Y".parse()?, Complex64::new(0.0, -1.0))])?;
assert!(xz.equiv(&minus_i_y, 0.0)?);
assert!(!xz.is_hermitian(0.0)?);

// [X,Z]，以及 X ⊗ Z：最右字符对应比特 0。
let commutator = xz.try_sub(&z.try_mul(&x)?)?;
assert!(commutator.equiv(&minus_i_y.scaled(2.0.into())?, 0.0)?);
assert!(x.tensor(&z)?.equiv(&Hamiltonian::from_pauli("XZ".parse()?), 0.0)?);
assert!(x.remap_qubits(&[2], 3)?.equiv(&Hamiltonian::from_pauli("XII".parse()?), 0.0)?);
# Ok::<(), QisError>(())
```

旧 `Add`、`scale`、`simplify` 和构造器保留原契约；新调用优先使用安全接口。`PartialEq` 比较存储结构，项顺序、重复项或相位分布不同的等价算符可能不相等，应使用 `equiv`。

### 查询与转换

- `fn all_terms_commute(&self) -> bool`：判断所有 Pauli 项是否两两对易。为真时 $e^{-iHt}$ 可以按项分解为精确演化。
- `fn to_matrix(&self) -> Result<Array2<Complex64>, QisError>`：验证并返回 $2^N \times 2^N$ 稠密矩阵。空算符返回对应维度的零矩阵，零比特输入返回 1×1 矩阵。分配前检查维度、元素数和字节数；大小溢出或内存预留失败返回 `UnsupportedOperation`。非法结构或非有限系数（包括矩阵元素累加溢出）返回相应错误。需要 $O(4^N)$ 内存，仅适用于小系统。
- `fn to_trotter_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`：见 [Trotter 演化](8_evolution.md)。
- `fn to_evolution_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`：见 [Trotter 演化](8_evolution.md)。

### 其他行为

- `Display`：逐项输出 `(系数) * Pauli串`，系数用复数的显示形式，项之间以 ` + ` 连接。项列表为空时输出空串。
- `Add`：把两条 Hamilton 量的项列表直接拼接，不做合并；两边比特数不同时 panic。需要合并相同项时，在相加后调用 `simplify()`。
- `From<PauliString>`、`TryFrom<Vec<(PauliString, Complex64)>>`：分别对应 `from_pauli()` 与 `from_list()`。
- 实现 `Observable`，见下文。
- 支持 `Debug`、`Clone`、`PartialEq`。

示例：

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
h.add_term("XX".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();

assert_eq!(h.terms.len(), 2);
// ZZ 与 XX 在每个量子比特上都同时反交换，整体相乘后相互抵消，因此两者对易
assert!(h.all_terms_commute());
```

换成在各量子比特上对易性不同的项，判定结果为假：

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

稠密矩阵：

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

计算可观测量期望值的统一接口，用于在不同的量子态表示上求同一个观测量。

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

### 必需方法

- `fn expectation_statevector(&self, sv: &Statevector) -> Result<f64, QisError>`：计算纯态期望值 $\langle \psi | O | \psi \rangle$。态的比特数与 `num_qubits()` 不一致时返回 `QisError::QubitMismatch`。
- `fn expectation_density_matrix(&self, dm: &DensityMatrix) -> Result<f64, QisError>`：计算混态期望值 $\text{Tr}(\rho O)$。比特数不一致时返回 `QisError::QubitMismatch`。
- `fn expectation_probs(&self, measurements: &[(PauliString, HashMap<String, f64>)]) -> Result<f64, QisError>`：从测量概率计算期望值。每个元素是一个测量基及其状态串到概率的映射。状态串按小端书写，最右字符对应比特 0。
- `fn num_qubits(&self) -> usize`：返回观测量作用的比特数，用于求期望值前的维度校验。

### 默认方法

- `fn variance_statevector(&self, sv: &Statevector) -> Result<f64, QisError>`：计算纯态方差 $\text{Var}(O) = \langle O^2 \rangle - \langle O \rangle^2$。trait 中的默认实现返回 `QisError::UnsupportedOperation`，支持方差计算的观测量需要覆写该方法。

实数期望值和方差接口在吸收相位、合并重复项后检查厄米性，不删除非零小系数。原始或合并后系数非有限时返回 `QisError::InvalidParameterValue`，非厄米算符返回 `QisError::NotHermitian`。

### 实现者

| 类型 | 说明 |
| --- | --- |
| `Hamiltonian` | 逐项计算后按系数累加，用于多项哈密顿量。 |
| `PauliString` | 单项 Pauli 串，用于单个可观测量的直接测量。 |

### `expectation_probs` 的测量基匹配

对每一个非恒等项，接口在给定的测量列表中查找相容的测量基：若测量基在项的非恒等位置上的 X、Z 分量与该项一致，即视为相容。恒等项直接贡献系数与全局相位，不需要测量基。

找不到相容测量基时返回 `QisError::UnsupportedOperation`。`Hamiltonian` 与 `PauliString` 的实现都按此规则匹配，状态串相关的错误分支略有差异：

| 情况 | `Hamiltonian` | `PauliString` |
| --- | --- | --- |
| 状态串长度与比特数不一致 | `QisError::DimensionMismatch` | `QisError::QubitMismatch` |
| 状态串出现非 `0`、`1` 字符 | `QisError::PauliStringParseError` | `QisError::UnsupportedOperation` |
| 找不到相容测量基 | `QisError::UnsupportedOperation` | `QisError::UnsupportedOperation` |

`Hamiltonian` 的实现忽略项中 Y 分量引入的相位，只计入全局相位与系数。

### 示例

Bell 态上的纯态期望值：

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

从测量概率求期望值。三条 Z 项都对同一个 ZZ 测量基相容：

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

方差：

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

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::QubitMismatch` | 构造或追加项时 Pauli 串的比特数与 `num_qubits` 不一致；求期望值或方差时态与观测量的比特数不一致。 |
| `QisError::NotHermitian` | 求方差时观测量不是 Hermitian，例如 `PauliString` 的相位为 `±i`，或化简后仍残留虚部超过容差的系数。 |
| `QisError::UnsupportedOperation` | 未覆写 `variance_statevector()` 的观测量调用该方法；`expectation_probs()` 找不到相容的测量基。 |
| `QisError::DimensionMismatch` | `Hamiltonian::expectation_probs()` 中状态串长度与比特数不一致。 |
| `QisError::PauliStringParseError` | `Hamiltonian::expectation_probs()` 中状态串出现非 `0`、`1` 字符。 |

`Add` 在两侧比特数不一致时 panic，而不是返回错误。

方差计算对数值噪声有一定容忍：结果落在零附近的小幅负值会被截断为 0，超出容差的负值改为返回 `QisError::InvalidParameterValue`。

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与术语表。
- [Pauli 算子与 Pauli 串](6_pauli.md)：项的构造、对易性与辛表示。
- [Trotter 演化](8_evolution.md)：把 Hamilton 量转换为演化线路。
