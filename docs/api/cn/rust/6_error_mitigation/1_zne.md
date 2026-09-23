# 零噪声外推

`cqlib_core::error_mitigation::zne_mitigation`

零噪声外推通过门折叠把基底线路改写成一组噪声强度递增的线路，在每条折叠线路上取出期望值，再用拟合外推到零噪声极限。本页覆盖折叠、批量取值与外推的公开 API。

## 导入

```rust
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation, ZneConfig};
```

---

## ZNEMitigation

零噪声外推的低层入口。它持有基底线路与折叠等级，把折叠、取值与外推分成三个可独立调用的阶段。

```rust
pub struct ZNEMitigation {
    circuit: Circuit,
    fold_levels: Vec<i32>,
    noise_factors: Vec<i32>,
}
```

字段私有，通过方法读取：

- `fn circuit(&self) -> &Circuit`：原始（未折叠）的基底线路。
- `fn fold_levels(&self) -> &[i32]`：配置的折叠等级，顺序与构造时一致。
- `fn noise_factors(&self) -> &[i32]`：与折叠等级一一对应的噪声因子，恒为 `2 * level + 1`。

其余方法：

- `fn new(circuit: Circuit, fold_levels: Vec<i32>) -> Self`：创建入口，取得线路所有权并据等级换算出噪声因子。构造阶段不校验等级是否非负。
- `fn fold_circuits(&self, gate_set: Option<&[Instruction]>) -> Result<Vec<Circuit>, CircuitError>`：按每个折叠等级各构造一条折叠线路。
- `fn run_em_sequence(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`：逐等级折叠并调用估计器取值，估计器收到的 shot 数为 `None`。
- `fn run_em_sequence_with_shots(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, shots: Option<usize>, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`：同上，并把 shot 数透传给估计器。
- `fn extrapolate(&self, noisy_results: &[f64], method: ExtrapolateMethod, degree: usize) -> Result<f64, ErrorMitigationError>`：按方法分派到下面两个外推方法。
- `fn poly_extrapolate(&self, noisy_results: &[f64], degree: usize) -> Result<f64, ErrorMitigationError>`：多项式外推。
- `fn exp_extrapolate(&self, noisy_results: &[f64]) -> Result<f64, ErrorMitigationError>`：指数外推。

`ZNEMitigation` 没有 builder 方法，也没有 `Default` 实现：实例只能由 `new` 创建，折叠等级在读接口中不可修改，改变等级需要重新创建实例。它实现 `Debug` 与 `Clone`。

### `fold_circuits(&self, gate_set: Option<&[Instruction]>) -> Result<Vec<Circuit>, CircuitError>`

按等级构造折叠线路，返回的向量长度等于折叠等级数，顺序与 `fold_levels` 一致。

参数：

- `gate_set`：可选的门集合，决定折叠范围。传 `None` 为全局折叠，把整条线路改写为 `U -> U (U† U)^level`，其中逆线路由基底线路整体取逆得到；传 `Some` 为选择性折叠，逐条操作检查名称，命中集合中任一名称的操作才被放大，其余操作原样保留。

返回：

- `Vec<Circuit>`：每个折叠等级对应的折叠线路。

异常情况：

- `CircuitError::InvalidControlOperation`：某一折叠等级为负。
- `CircuitError`：基底线路取逆失败等线路层面的错误。

等级为 0 时直接返回基底线路的副本，不做任何追加。选择性折叠按操作名称匹配，名称由 `Instruction::name()` 给出，与操作携带的参数或自身的线路定义无关。

### `run_em_sequence_with_shots(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, shots: Option<usize>, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`

逐等级折叠并调用估计器，返回每条折叠线路的期望值。`run_em_sequence` 是它的无 shot 版本，固定把 `None` 传给估计器。

参数：

- `gate_set`：同 `fold_circuits`，控制折叠范围。
- `hamiltonian`：待估计的可观测量，其比特数必须等于基底线路的宽度。
- `shots`：每次执行的 shot 数，原样透传给估计器。
- `estimator`：估计器回调，每条折叠线路调用一次，第二个参数恒为 `Some(hamiltonian)`；回调形态见 [概览](0_overview.md)。

返回：

- `Vec<f64>`：各折叠线路的期望值，顺序与 `fold_levels` 一致。估计器返回的方差在此阶段被丢弃。

异常情况：

- `ErrorMitigationError::HamiltonianQubitCountMismatch`：哈密顿量的比特数与基底线路宽度不一致，错误中同时给出期望值与实际值。
- `ErrorMitigationError::Circuit`：折叠失败时透传的线路错误。

### 外推

三个外推方法都以噪声因子为自变量、以 `noisy_results` 为因变量，返回零噪声（自变量为 0）处的估计值，并且都要求 `noisy_results` 非空且长度等于噪声因子个数。

- `fn poly_extrapolate(&self, noisy_results: &[f64], degree: usize) -> Result<f64, ErrorMitigationError>`：最小二乘多项式拟合，返回 `x = 0` 处的截距。`degree` 为多项式次数，必须小于数据点数。
- `fn exp_extrapolate(&self, noisy_results: &[f64]) -> Result<f64, ErrorMitigationError>`：按 `y(x) = A * exp(-x / tau)` 建模，在 log 空间对 `ln(y) = ln(A) + m * x` 做线性回归，返回 `A`。
- `fn extrapolate(&self, noisy_results: &[f64], method: ExtrapolateMethod, degree: usize) -> Result<f64, ErrorMitigationError>`：`Polynomial` 使用 `degree` 转发到 `poly_extrapolate`；`Exponential` 忽略 `degree` 转发到 `exp_extrapolate`。

### 示例

```rust
use cqlib_core::circuit::gate::{Instruction, StandardGate};
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::ZNEMitigation;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.h(q0).unwrap();
circuit.s(q0).unwrap();

// 折叠等级 [0, 1, 2] 对应噪声因子 [1, 3, 5]
let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);
assert_eq!(zne.fold_levels(), &[0, 1, 2]);
assert_eq!(zne.noise_factors(), &[1, 3, 5]);

// 全局折叠：等级 0 原样返回，等级 1 把整条线路折叠为 U -> U U† U
let global = zne.fold_circuits(None).unwrap();
assert_eq!(global[0].operations().len(), 2);
assert_eq!(global[1].operations().len(), 6);

// 选择性折叠：只放大 S 门，H 门保持原样
let gate_set = vec![Instruction::Standard(StandardGate::S)];
let selective = zne.fold_circuits(Some(&gate_set)).unwrap();
assert_eq!(selective[1].operations().len(), 4);
```

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

// 估计器返回 (期望值, 方差)，零噪声外推只使用期望值
let noisy = zne
    .run_em_sequence_with_shots(
        None,
        &hamiltonian,
        Some(256),
        &|circuit, hamiltonian_arg, shots| {
            assert!(hamiltonian_arg.is_some());
            assert_eq!(shots, Some(256));
            (circuit.operations().len() as f64 + 0.5, 0.0)
        },
    )
    .unwrap();

// 噪声因子为 [1, 3, 5]，实测值满足 y = 0.5 + x
assert_eq!(noisy, vec![1.5, 3.5, 5.5]);

// 多项式外推取 x = 0 处的截距
let mitigated = zne
    .extrapolate(&noisy, ExtrapolateMethod::Polynomial, 1)
    .unwrap();
assert!((mitigated - 0.5).abs() < 1e-10);

// 指数外推：模型为 y = A * exp(-x / tau)，返回 A，同时忽略 degree
let a = 2.5_f64;
let tau = 4.0_f64;
let noisy: Vec<f64> = zne
    .noise_factors()
    .iter()
    .map(|&x| a * (-(x as f64) / tau).exp())
    .collect();
let mitigated = zne
    .extrapolate(&noisy, ExtrapolateMethod::Exponential, 99)
    .unwrap();
assert!((mitigated - a).abs() < 1e-10);
```

---

## ExtrapolateMethod

外推方法枚举，决定 `extrapolate` 走哪条拟合路径。

```rust
pub enum ExtrapolateMethod {
    Polynomial,
    Exponential,
}
```

- `Polynomial`：多项式外推。`extrapolate` 的 `degree` 参数在该分支生效。
- `Exponential`：指数外推。`extrapolate` 的 `degree` 参数在该分支被忽略。

实现 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`。

---

## ZneConfig

零噪声外推的方法配置，由统一流水线所在子模块 `cqlib_core::error_mitigation::unified` 定义并经模块根重导出，供 `MitigationMethod::Zne` 携带。

```rust
pub struct ZneConfig {
    pub fold_levels: Vec<i32>,
}
```

字段：

- `fold_levels`：折叠等级列表，决定折叠线路条数与噪声因子；等级 `l` 对应噪声因子 `2 * l + 1`。

没有 `Default` 实现，必须显式给出 `fold_levels`。实现 `Debug`、`Clone`、`PartialEq`、`Eq`。

`ErrorMitigation::new` 在创建阶段检查其中的折叠等级，任一等极为负即返回 `ErrorMitigationError::InvalidFoldLevel`；等级为空向量可以通过创建阶段的检查，但后续外推会因没有数据点而报错。

---

## 错误

折叠、取值与外推分属两类错误：折叠返回线路错误，取值与外推返回模块统一错误。

| 错误 | 触发场景 |
| --- | --- |
| `CircuitError::InvalidControlOperation` | `fold_circuits` 遇到负的折叠等级。 |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | `run_em_sequence` 或 `run_em_sequence_with_shots` 收到的哈密顿量比特数与基底线路宽度不一致。 |
| `ErrorMitigationError::EmptyNoisyResults` | 外推收到的结果序列为空，例如折叠等级为空向量。 |
| `ErrorMitigationError::NoisyResultsLengthMismatch` | 外推结果序列的长度不等于噪声因子个数。 |
| `ErrorMitigationError::InvalidPolynomialDegree` | 多项式次数不小于数据点数。 |
| `ErrorMitigationError::NonPositiveNoisyResults` | 指数外推收到的结果中存在非正值。 |
| `ErrorMitigationError::SingularExponentialFit` | 指数外推的线性回归退化。 |
| `ErrorMitigationError::SingularPolynomialFit` | 多项式外推的正规方程矩阵奇异。 |
| `ErrorMitigationError::Circuit` / `Qis` | 由折叠或线路操作透传的错误。 |

错误类型的完整定义见 [统一流水线](3_unified.md)。
