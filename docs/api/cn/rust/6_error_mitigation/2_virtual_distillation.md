# 虚拟蒸馏

`cqlib_core::error_mitigation::virtual_distillation`

虚拟蒸馏通过多份密度矩阵拷贝与 copy-swap 线路，估计比值 `Tr(O ρ^M) / Tr(ρ^M)`，其中 `M` 为副本数，`O` 为由哈密顿量表示的可观测量。本页覆盖 copy-swap 线路构造与分子、分母统计的公开 API。

## 导入

```rust
use cqlib_core::error_mitigation::VirtualDistillation;
```

---

## VirtualDistillation

虚拟蒸馏的低层入口。它持有基底线路与副本数，可以单独构造 copy-swap 线路、单独跑分母或分子线路，也可以一步得到比值与方差。

```rust
pub struct VirtualDistillation {
    circuit: Circuit,
    copies: usize,
}
```

字段私有，通过方法读取：

- `fn copies(&self) -> usize`：当前配置的副本数。

其余方法：

- `fn new(circuit: Circuit, copies: usize) -> Result<Self, ErrorMitigationError>`：创建入口，取得线路所有权。模板线路只能通过 `new` 传入，没有对应的读接口。
- `fn set_copies(&mut self, copies: usize) -> Result<(), ErrorMitigationError>`：更新副本数。更新失败时保持原值不变。
- `fn build_copy_swap_circuit(&self) -> Result<Circuit, CircuitError>`：由基底线路构造 copy-swap 线路。
- `fn run_denominator_circuit(&self, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), CircuitError>`：跑分母线路，返回 `(均值, 方差)`。
- `fn run_numerator_circuit(&self, hamiltonian: &Hamiltonian, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`：跑分子线路，返回 `(均值, 方差)`。
- `fn run_vd(&self, hamiltonian: &Hamiltonian, shots_numerator: usize, shots_denominator: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`：跑完整流程，返回缓解后的 `(期望值, 方差)`。

`VirtualDistillation` 没有 builder 方法，也没有 `Default` 实现：实例只能由 `new` 创建，副本数通过 `set_copies` 修改。它实现 `Debug` 与 `Clone`。

### `new(circuit: Circuit, copies: usize) -> Result<Self, ErrorMitigationError>`

参数：

- `circuit`：基底线路，后续所有派生线路都由它构造。
- `copies`：副本数，必须不小于 2。

返回：

- `VirtualDistillation`：可用于构造线路与取值。

异常情况：

- `ErrorMitigationError::InvalidCopies`：副本数小于 2，错误中携带传入的副本数。

### `build_copy_swap_circuit(&self) -> Result<Circuit, CircuitError>`

构造 copy-swap 线路，包含三个部分：先对基底线路做门分解，再把基底线路并排准备 `copies` 份——第 `i` 份整体平移到从 `i × 基底宽度` 开始的比特区间，各份之间互不重叠——最后在首份与其他每一份之间逐比特插入 `SWAP`。

- 线路宽度为 `copies × 基底宽度`，其中基底宽度取分解后线路的宽度。
- 操作数为 `copies` 份基底操作加上 `(copies - 1) × 基底宽度` 个 `SWAP`。

返回：

- `Circuit`：copy-swap 线路。

异常情况：

- `CircuitError`：基底线路分解失败，或向新线路追加操作失败。

### 分子与分母

- `fn run_denominator_circuit(&self, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), CircuitError>`：在 copy-swap 线路上估计分母 `Tr(ρ^M)`。估计器收到的可观测量参数为 `None`，shot 数为 `Some(shots)`；回调形态见 [概览](0_overview.md)。
- `fn run_numerator_circuit(&self, hamiltonian: &Hamiltonian, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`：在 copy-swap 线路上估计分子 `Tr(O ρ^M)`。可观测量会在模块内部先扩展到 copy-swap 线路的宽度：原有 Pauli 项保持原比特索引与相位、系数不变，新增的高索引比特上补 `Z`；扩展结果经估计器的可观测量参数传入，shot 数为 `Some(shots)`。
- `fn run_vd(&self, hamiltonian: &Hamiltonian, shots_numerator: usize, shots_denominator: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`：先校验哈密顿量的比特数等于基底线路宽度，再依次执行分子线路与分母线路，最后组合成缓解结果。分子与分母使用各自的 shot 数。

`run_vd` 的异常情况：

- `ErrorMitigationError::HamiltonianQubitCountMismatch`：哈密顿量的比特数与基底线路宽度不一致，错误中同时给出期望值与实际值。
- `ErrorMitigationError::ZeroDenominatorMean`：分母线路的均值为零，比值无法计算。
- `ErrorMitigationError::Circuit`：扩展后的可观测量宽度与 copy-swap 线路宽度不一致时透传的线路错误。

### 比值与方差

`run_vd` 的返回值按 `mu_vd = mu_num / mu_den` 组合期望值；方差按一阶泰勒近似、并假设分子与分母相互独立，组合为 `var_num / mu_den^2 + mu_num^2 * var_den / mu_den^4`。这组公式只在模块内部使用，对外只通过 `run_vd` 取得结果，统一流水线在后处理阶段复用同一组公式。

### 示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::VirtualDistillation;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

// 两份单比特副本：X 作用于比特 0、X 作用于比特 1，再加一对逐比特 SWAP
let vd = VirtualDistillation::new(circuit, 2).unwrap();
let copy_swap = vd.build_copy_swap_circuit().unwrap();
assert_eq!(copy_swap.width(), 2);
assert_eq!(copy_swap.operations().len(), 3);
```

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::{ErrorMitigationError, VirtualDistillation};

// 副本数通过 set_copies 调整，非法值被拒绝且不改变原值
let mut vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
assert_eq!(vd.copies(), 2);

vd.set_copies(3).unwrap();
assert_eq!(vd.copies(), 3);

let err = vd.set_copies(1).unwrap_err();
assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
assert_eq!(vd.copies(), 3);
```

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::VirtualDistillation;
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

// 分子线路带可观测量，分母线路不带；两者使用各自的 shot 数
let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
let (mu_vd, var_vd) = vd
    .run_vd(&hamiltonian, 3, 2, &|_circuit, hamiltonian_arg, shots| {
        if hamiltonian_arg.is_some() {
            assert_eq!(shots, Some(3));
            (1.5, 0.25)
        } else {
            assert_eq!(shots, Some(2));
            (2.0, 1.0)
        }
    })
    .unwrap();

assert!((mu_vd - 0.75).abs() < 1e-12);
assert!((var_vd - 0.203125).abs() < 1e-12);
```

---

## VirtualDistillationConfig

虚拟蒸馏的方法配置，由统一流水线所在子模块 `cqlib_core::error_mitigation::unified` 定义并经模块根重导出，供 `MitigationMethod::VirtualDistillation` 携带。

```rust
pub struct VirtualDistillationConfig {
    pub copies: usize,
}
```

字段：

- `copies`：副本数，最少为 2。

没有 `Default` 实现，必须显式给出 `copies`。实现 `Debug`、`Clone`、`PartialEq`、`Eq`。

`ErrorMitigation::new` 在创建阶段检查该字段，小于 2 即返回 `ErrorMitigationError::InvalidCopies`。

---

## 错误

| 错误 | 触发场景 |
| --- | --- |
| `ErrorMitigationError::InvalidCopies` | 副本数小于 2，见 `new`、`set_copies` 与 `ErrorMitigation::new`。 |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | `run_vd` 收到的哈密顿量比特数与基底线路宽度不一致。 |
| `ErrorMitigationError::ZeroDenominatorMean` | 分母线路的均值为零。 |
| `ErrorMitigationError::Circuit` | 携带 `CircuitError::QubitCountMismatch`，表示扩展后的可观测量宽度与 copy-swap 线路宽度不一致。 |
| `CircuitError` | `build_copy_swap_circuit` 与 `run_denominator_circuit` 直接返回的线路错误。 |

错误类型的完整定义见 [统一流水线](3_unified.md)。
