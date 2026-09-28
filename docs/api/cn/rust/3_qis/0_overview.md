# QIS 量子信息

`cqlib_core::qis`

`cqlib_core::qis` 提供量子信息科学相关的基础对象与本地模拟能力：量子态的多种表示、Pauli 算子与 Hamilton 量、Pauli 演化线路的构造，以及保真度、熵和纠缠度量。本页是该模块的概览与导航。

## Overview

量子线路的仿真结果落在一个量子态上。不同的任务需要不同的态表示：只包含标准门与旋转门的线路可以用纯态表示描述；包含噪声或需要描述量子信道的场景必须使用混合态表示；只包含 Clifford 门的线路则可以使用稳定子表示，用多项式规模的数据描述量子态。

`cqlib_core::qis` 把「态的表示」与「作用于态的操作」组织在一起：每个模拟器都提供一组作用于态的常用操作，并通过与线路 IR 对接的入口接收整条线路。可观测量、演化与度量则建立在态表示之上。

### 四种态表示

| 表示 | 适用场景 | 噪声支持 | 可描述的规模 |
| --- | --- | --- | --- |
| `Statevector` | 理想通用线路，纯态 | 无 | 小规模系统 |
| `DensityMatrix` | 混合态与量子信道 | Kraus 算符 | 小规模系统 |
| `DensityMatrixNoise` | 带噪声的设备级仿真 | 完整噪声模型（门噪声与读出噪声） | 小规模系统 |
| `StabilizerState` | 仅含 Clifford 门的线路 | 无 | 大规模系统 |

四种表示共享一致的接口命名：门操作按名字对应（`apply_h`、`apply_cx` 等），测量与采样接口一致（`measure`、`measure_all`、`sample_shots`、`sample`、`probs`）。

### 与线路的对接

三种稠密表示都提供两个与线路对接的入口：

- `from_circuit`：执行一条线路并返回演化后的态，不修改输入线路。
- `apply_circuit`：把线路原地作用到已有的态上，要求线路的比特数与态一致。

`StabilizerState` 额外提供 `run_circuit`，它在执行 Clifford 线路的同时返回运行期经典数据，返回类型为 `CircuitExecutionResult`。

### 算子、演化与度量

Pauli 算子与 Pauli 串是描述可观测量与稳定子生成元的基本对象。`Hamiltonian` 由带系数的 Pauli 串构成，实现 `Observable` trait 后可以作为观测量交给态的 `expectation` 方法求期望值。`PauliEvolution` 把 Pauli 串转换为可执行的演化线路。`metrics` 与 `entropy` 两个模块提供纯度、保真度、迹距离、熵与纠缠度量。

---

## 常用入口

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};

// 纯态模拟
let mut sv = Statevector::new(3);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();
let probs_sv = sv.probabilities();

// 混合态模拟
let mut dm = DensityMatrix::new(3);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();
let probs_dm = dm.probabilities();

// 纯态下两种表示给出相同的结果
assert!((probs_sv[0] - probs_dm[0]).abs() < 1e-10);
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **态矢量** | 纯态在计算基下的复振幅向量，第 `i` 个分量对应基态 `\|i>`。 |
| **混合态** | 不能用单一态矢量描述的量子态，由密度矩阵表示。 |
| **密度矩阵** | 描述量子态的矩阵，对角元给出计算基下的测量概率。 |
| **量子信道** | 态的保迹映射，用一组 Kraus 算符 `K_k` 表示。 |
| **稳定子态** | 由一组相互对易的 Pauli 算子（稳定子生成元）共同确定的本征态。 |
| **Clifford 门** | 把 Pauli 群映射到自身的门；稳定子表示只接受这类门。 |
| **读出噪声** | 测量环节引入的经典误差，作用于概率分布而非量子态本身。 |
| **坍缩** | 测量导致态投影到与测量结果一致的子空间。 |
| **采样** | 按态的概率分布生成多次独立测量结果。 |
| **可观测量** | 可获得期望值的物理量；由 `Observable` trait 描述。 |

---

## `cqlib_core::qis` API 概览

### 模块与态表示

| 名字 | 简介 |
| --- | --- |
| [`cqlib_core::qis`](0_overview.md) | 模块概览、术语与错误处理。 |
| [`Statevector`](1_statevector.md) | 纯态模拟器，保存 `2^n` 个复振幅。 |
| [`DensityMatrix`](2_density_matrix.md) | 混合态模拟器，支持 Kraus 算符与偏迹。 |
| [`DensityMatrixNoise`](3_density_matrix_noise.md) | 带噪声模型的密度矩阵模拟器。 |
| [`StabilizerState`](4_stabilizer.md) | 稳定子态模拟器，支持大规模 Clifford 线路。 |

### 经典数据

| 名字 | 简介 |
| --- | --- |
| [`ClassicalState`](5_classical_state.md) / [`RuntimeValue`](5_classical_state.md) | Clifford 线路运行期产生的经典值与变量。 |

### 算子与可观测量

| 名字 | 简介 |
| --- | --- |
| [`Pauli`](6_pauli.md) / [`Phase`](6_pauli.md) | 单比特 Pauli 算子与相位。 |
| [`PauliString`](6_pauli.md) / [`PauliIter`](6_pauli.md) | Pauli 串及其按比特迭代器。 |
| [`Hamiltonian`](7_hamiltonian.md) / [`Observable`](7_hamiltonian.md) | 带系数的 Pauli 串之和，以及可观测量的统一接口。 |

### 演化与度量

| 名字 | 简介 |
| --- | --- |
| [`PauliEvolution`](8_evolution.md) / [`TrotterMode`](8_evolution.md) | Pauli 串演化线路的构造与 Trotter 分解模式。 |
| [`metrics`](9_metrics_entropy.md) / [`entropy`](9_metrics_entropy.md) | 保真度、纯度、迹距离与熵、纠缠度量。 |

---

## 快速示例

### 1. 纯态与混合态给出相同概率分布

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

let probs_sv = sv.probabilities();
let probs_dm = dm.probabilities();
for (a, b) in probs_sv.iter().zip(probs_dm.iter()) {
    assert!((a - b).abs() < 1e-10);
}
```

### 2. 稳定子态采样

```rust
use cqlib_core::qis::StabilizerState;
use std::collections::HashMap;

let mut state = StabilizerState::new(2);
state.apply_h(0).unwrap();
state.apply_cx(0, 1).unwrap();

let mut counts = HashMap::new();
for outcome in state.sample_shots(1000) {
    *counts.entry(outcome.to_bitstring(2)).or_insert(0usize) += 1;
}

// Bell 态的采样结果只会出现关联结果
assert!(counts.keys().all(|bits| bits == "00" || bits == "11"));
```

### 3. 求可观测量的期望值

```rust
use cqlib_core::qis::{Hamiltonian, PauliString, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

let exp = sv.expectation(&h).unwrap();
assert!(exp.is_finite());
```

---

## 校验与错误处理

`QisError` 是 `cqlib_core::qis` 的统一错误类型，态构造、门作用、测量与期望值计算失败都通过它返回。

| 错误 | 触发场景 |
| --- | --- |
| `QisError::QubitMismatch` | 算子或线路的比特数与态不一致。 |
| `QisError::DimensionMismatch` | 矩阵或向量的维度不匹配。 |
| `QisError::InvalidStateDimension` | 态矢量的长度不是 2 的幂，或与声明的比特数不符。 |
| `QisError::InvalidProbability` | 概率取值超出 `[0, 1]`。 |
| `QisError::IndexOutOfBounds` | 访问的比特或振幅下标越界。 |
| `QisError::UnsupportedOperation` | 当前模拟器不支持该操作。 |
| `QisError::PauliStringParseError` | Pauli 串解析失败。 |
| `QisError::NotNormalized` | 显式提供的态未归一化。 |
| `QisError::NotHermitian` | 要求为 Hermitian 的算子不满足自伴性。 |
| `QisError::NotPositiveSemidefinite` | 密度矩阵不是半正定。 |
| `QisError::InvalidParameterValue` | 参数取值非法。 |
| `QisError::InvalidSubsystem` | 纠缠度量中的子系统描述非法。 |
| `QisError::UnsupportedDimension` | 操作要求的维度不满足。 |
| `QisError::NonCliffordGate` | 向稳定子态作用了非 Clifford 门。 |
| `QisError::CircuitError` | 线路执行失败，例如门没有矩阵表示或存在未解析的符号参数。 |

各页的具体触发条件见 [Statevector](1_statevector.md)、[DensityMatrix](2_density_matrix.md)、[DensityMatrixNoise](3_density_matrix_noise.md) 与 [StabilizerState](4_stabilizer.md)。
