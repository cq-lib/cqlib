# 量子信息

`cqlib.qis`

`cqlib.qis` 是 Cqlib 的量子信息模块。它提供量子态的多种表示与模拟器、Pauli 算子代数、以 Hamiltonian 为核心的观测量体系，以及态保真度、迹距离、熵与纠缠度量等分析工具。这些设施既可以独立用于量子信息层面的计算，也可以直接消费 `cqlib.circuit` 中的线路来获得演化结果。

## Overview

量子信息模块解决两类问题：一是**表示与演化**，即把量子态存下来并让门作用上去；二是**度量**，即从态或观测量中提取可比较的数值。二者共享同一套底层算子代数。

### 量子态的四种表示

四种模拟器的存储代价与适用范围互不相同，选择哪一种取决于线路的门集与需要观察的量：

- 态矢量用 2^N 个复振幅表示纯态，是最紧凑的表示，但不能描述经典混合与噪声。
- 密度矩阵用 2^N × 2^N 的矩阵表示混合态，代价高一个量级，但支持量子信道、部分迹与物理性校验。
- 含噪密度矩阵在密度矩阵之上绑定噪声模型，把 Kraus 噪声按门自动施加，并提供读出噪声下的概率分布。
- 稳定子表用 N 个对易 Pauli 生成元表示 Clifford 态，存储代价与比特数成线性关系，但只接受 Clifford 门。

前三种共享同一套门作用入口（`apply_circuit`、`apply_standard_gate`、单比特与双比特门方法），彼此之间的差异集中在状态存储与噪声处理上。

### 比特下标约定

四种表示使用同一套比特编号约定：比特 i 对应基态下标的第 i 位，比特 0 为最低位；在位串表示中，编号大的比特位于左侧。这一约定在全部页面中一致，不再重复说明。

### 从线路到状态

四种模拟器都提供 `from_circuit()` 静态方法，从 |0…0⟩ 出发模拟线路得到最终状态，输入线路保持不变。需要同时拿到最终状态与运行期经典数据时，使用 `StabilizerState.run_circuit()`。

### 观测量与时间演化

`PauliString` 与 `Hamiltonian` 描述可观测量，两者都可以给出期望值；`Hamiltonian` 还可以通过 Trotter 分解转成演化线路，回到 `cqlib.circuit` 的线路空间中执行。

---

## 常用入口

```python
import math

from cqlib import Circuit
from cqlib.qis import PauliString
from cqlib.qis.state import Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

sv = Statevector.from_circuit(circuit)
assert math.isclose(sv.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(sv.probabilities()[3], 0.5, abs_tol=1e-10)
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **态矢量** | 用 2^N 个复振幅描述的 N 比特纯态，基态 \|i⟩ 的振幅位于下标 `i`。 |
| **密度矩阵** | 用 2^N × 2^N 的厄米半正定矩阵描述的 N 比特量子态，可表示纯态的经典混合。 |
| **含噪模拟** | 在密度矩阵之上按噪声模型施加 Kraus 噪声的模拟方式，门噪声在每次门作用后立即生效。 |
| **稳定子表** | 用 N 个对易 Pauli 生成元描述 Clifford 态的紧凑表示，存储代价随比特数线性增长。 |
| **Clifford 门** | 把 Pauli 群映射到自身的门，例如 H、S、X、Y、Z 与受控 Pauli 门。非 Clifford 门无法用稳定子表模拟。 |
| **反稳定子** | 与稳定子生成元配对的一组 Pauli 算子，用于确定测量结果并维持稳定子表的规范形式。 |
| **辛表示** | Pauli 字符串的紧凑存储形式，用 X、Z 两组比特掩码与一个相位位共同描述算子。 |
| **可观测量** | 可以计算期望值与方差的对象，例如 `PauliString` 与 `Hamiltonian`。 |
| **期望值** | 观测量在给定状态下的平均值，纯态为 ⟨ψ\|O\|ψ⟩，混合态为 Tr(ρ·O)。 |
| **部分迹** | 对部分比特求迹、保留其余比特的操作，用于得到子系统上的约化密度矩阵。 |
| **Trotter 分解** | 把时间演化算子 e^{-iHt} 近似为一串 Pauli 旋转的分解方式。 |
| **读出噪声** | 测量阶段引入的经典翻转，使读出结果与真实测量结果不一致。 |
| **运行时经典数据** | 线路执行过程中产生、并在执行结束后按句柄查询的经典值。 |

---

## `cqlib.qis` API 概览

### 模块入口

| 名字 | 简介 |
| --- | --- |
| [`cqlib.qis`](0_overview.md) | 模块概览，即本页。 |

### 状态模拟

| 名字 | 简介 |
| --- | --- |
| [`Statevector`](1_statevector.md) | 纯态模拟器，用 2^N 个复振幅表示 N 比特量子态。 |
| [`DensityMatrix`](2_density_matrix.md) | 混合态模拟器，支持量子信道、部分迹与物理性校验。 |
| [`DensityMatrixNoise`](3_density_matrix_noise.md) | 含噪混合态模拟器，按噪声模型施加门噪声与读出噪声。 |
| [`StabilizerState`](4_stabilizer.md) | Clifford 态模拟器，用稳定子表表示状态并支持生成元查询。 |
| [`StabilizerCircuitResult`](4_stabilizer.md) | `StabilizerState.run_circuit()` 的结果，携带最终状态与运行时经典数据。 |

### 运行时经典数据

| 名字 | 简介 |
| --- | --- |
| [`RuntimeValue`](5_classical_state.md) | 单个经典值在一次执行中的带类型取值。 |
| [`ClassicalState`](5_classical_state.md) | 一次执行结束后全部运行时经典数据的快照，按句柄查询取值。 |

### Pauli 代数与观测量

| 名字 | 简介 |
| --- | --- |
| [`Phase`](6_pauli.md) | Pauli 群元素相乘时产生的相位因子。 |
| [`Pauli`](6_pauli.md) | 单比特 Pauli 算子。 |
| [`PauliString`](6_pauli.md) | 多比特 Pauli 算子的张量积，采用辛表示存储。 |
| [`Observable`](7_hamiltonian.md) | 可观测量的协议，规定期望值与方差的计算入口。 |
| [`Hamiltonian`](7_hamiltonian.md) | 用 Pauli 字符串线性组合表示的观测量，可计算期望值并生成演化线路。 |

### 时间演化

| 名字 | 简介 |
| --- | --- |
| [`TrotterMode`](8_evolution.md) | 时间演化算子的 Trotter 分解方式，包括一阶、二阶与随机化。 |
| [`Hamiltonian.to_trotter_circuit`](8_evolution.md) | 把 `Hamiltonian` 转成 Trotter 近似演化线路。 |
| [`Hamiltonian.to_evolution_circuit`](8_evolution.md) | 把 `Hamiltonian` 转成时间演化线路。 |

### 度量与熵

| 名字 | 简介 |
| --- | --- |
| [`purity_pure`](9_metrics_entropy.md) / [`purity_mixed`](9_metrics_entropy.md) | 纯态与混合态的纯度。 |
| [`state_fidelity_pure`](9_metrics_entropy.md) / [`state_fidelity_pure_mixed`](9_metrics_entropy.md) / [`state_fidelity_mixed`](9_metrics_entropy.md) | 纯态之间、纯态与混合态之间、混合态之间的状态保真度。 |
| [`trace_distance_pure`](9_metrics_entropy.md) / [`trace_distance_mixed`](9_metrics_entropy.md) | 纯态与混合态的迹距离。 |
| [`entropy`](9_metrics_entropy.md) | 密度矩阵的冯诺依曼熵。 |
| [`partial_transpose`](9_metrics_entropy.md) / [`logarithmic_negativity`](9_metrics_entropy.md) | 部分转置与对数负度。 |
| [`linear_entropy`](9_metrics_entropy.md) / [`renyi_entropy`](9_metrics_entropy.md) | 线性熵与 Rényi 熵。 |
| [`entanglement_entropy_pure`](9_metrics_entropy.md) | 纯态在给定子系统上的纠缠熵。 |
| [`negativity`](9_metrics_entropy.md) / [`concurrence`](9_metrics_entropy.md) / [`entanglement_of_formation`](9_metrics_entropy.md) | 负度、共生与形成纠缠。 |

---

## 快速示例

### 1. 同一线路的纯态与混合态模拟

```python
import math

from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString
from cqlib.qis.state import DensityMatrix, Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

# 纯态：用态矢量计算期望值
sv = Statevector.from_circuit(circuit)
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)

# 混合态：用密度矩阵计算期望值，并取子系统的约化态
dm = DensityMatrix.from_circuit(circuit)
hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 0.5)
hamiltonian.add_term(PauliString.from_str("XX"), 0.5)
hamiltonian.simplify()
assert math.isclose(dm.expectation(hamiltonian), 1.0, abs_tol=1e-10)

reduced = dm.partial_trace([0])
assert math.isclose(reduced.probabilities()[0], 0.5, abs_tol=1e-10)
```

### 2. 带噪声的模拟

```python
import math

from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.1))

sim = DensityMatrixNoise(1, noise_model)
sim.apply_x(0)

probs = sim.probabilities()
assert math.isclose(probs[1], 0.9, abs_tol=0.01)
assert math.isclose(probs[0], 0.1, abs_tol=0.01)
```

### 3. Clifford 线路与稳定子表

```python
from cqlib import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = StabilizerState.from_circuit(circuit)
assert state.probabilities()[0] == 0.5
assert state.probabilities()[3] == 0.5

# 稳定子表的逐行文本表示
assert state.to_stim_format().count("\n") == 2
assert len(state.get_stabilizers()) == 2
```

---

## 校验与错误处理

模拟器在构造、门作用与状态读写各阶段都会做校验。各页的异常表列出全部触发场景，共同的分类如下：

| 异常 | 触发场景 |
| --- | --- |
| `IndexError` | 比特下标越界。 |
| `ValueError` | 维度或长度与比特数不符、振幅未归一化、密度矩阵违反物理约束、Kraus 算符非法、双比特门的目标比特重复、线路含非 Clifford 操作或控制流、可观测量类型或比特数不匹配。 |
| `TypeError` | 输入类型不受支持，例如构造时收到既非数组也非列表的初态，或校验方法的容差参数不是浮点数。 |
| `OverflowError` | `num_qubits` 为负数。 |

各页的异常情况小节给出该页 API 的完整触发场景。
