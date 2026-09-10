# QIS 量子信息

`cqlib.qis` 提供量子信息科学相关的基础对象和本地模拟能力，用于检查量子态、计算可观测量、构造 Pauli 哈密顿量、生成 Trotter 演化线路，并分析保真度、熵和纠缠指标。

本章示例都在本地 Python 环境中运行，不连接云平台，也不提交真实量子硬件任务。开始前需要完成 [安装与环境配置](../../0_get_started/1_installation.md)，并确认当前导入的是要测试的 Cqlib 版本。

```python
import cqlib

print(cqlib.__file__)
```

---

## 常用入口

QIS 的 Python 入口集中在 `cqlib.qis`，状态模拟器也可以从 `cqlib.qis.state` 子包导入。

```python
from cqlib.qis import (
    sample,
    Statevector,
    DensityMatrix,
    DensityMatrixNoise,
    StabilizerState,
    StabilizerCircuitResult,
    RuntimeValue,
    ClassicalState,
    Phase,
    Pauli,
    PauliString,
    Hamiltonian,
    TrotterMode,
    metrics,
    entropy,
)
```

| 对象 | 主要用途 | 常见场景 |
|---|---|---|
| `sample` | 直接采样线路并返回 `ExecutionResult` | 获取测量计数、选择输出比特、固定随机种子 |
| `Statevector` | 纯态模拟，保存 `2^n` 个复振幅 | 理想线路验证、VQE/QAOA 小规模能量计算、状态采样 |
| `DensityMatrix` | 密度矩阵模拟，保存 `2^n × 2^n` 复矩阵 | 混态、Kraus 噪声、部分迹、物理性检查 |
| `DensityMatrixNoise` | 带 `NoiseModel` 的密度矩阵模拟器 | 门噪声、读出误差、含噪线路对比 |
| `StabilizerState` | Clifford 稳定子模拟 | 大一些的 Clifford 线路、稳定子生成元、快速采样 |
| `Pauli` / `PauliString` | 单比特和多比特 Pauli 算符 | 可观测量、对易性判断、测量期望值 |
| `Hamiltonian` | Pauli 项稀疏和式 | VQE、QAOA、Ising 模型、时间演化 |
| `TrotterMode` | Trotter-Suzuki 分解模式 | `e^{-iHt}` 演化线路构造 |
| `metrics` / `entropy` | 量子态距离、纯度、熵和纠缠指标 | 态相似度、混合度、纠缠分析 |

这些对象通常不是替代 `Circuit`，而是用来执行和解释 `Circuit`。线路负责表达程序结构，QIS 负责把线路变成可分析的状态、可观测量和数值指标。

---

## 直接采样线路

只需要运行线路、获取计数时，使用 `cqlib.sample`，也可以从 `cqlib.qis` 导入同一个函数：

```python
from cqlib import Circuit, sample

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

result = sample(circuit, shots=1000, seed=42)
print(result.counts)
print(result.probabilities)
```

调用形式为 `sample(circuit, *, shots=1000, seed=None, simulator="statevector", qubits=None)`，返回已完成的 `ExecutionResult`。`probabilities` 是 counts 归一化后的采样频率；精确理论概率仍使用状态对象的 `probabilities()` 或 `probs()`。

| 参数 | 规则 |
|---|---|
| `circuit` | 从全零态开始模拟，不修改输入线路 |
| `shots` | 正整数，默认 1000 |
| `seed` | 可选的 64 位无符号整数；相同种子、线路和选项在同一版本及模拟器下产生相同 counts |
| `simulator` | 默认 `statevector`，可选 `density_matrix` 或 `stabilizer`，均为理想模拟 |
| `qubits` | 可选的非空比特 ID 列表或 `Qubit` 列表，不能重复，必须属于线路；列表顺序决定输出位序 |

未指定 `qubits` 时，默认规则如下：

- 没有测量声明：按 `circuit.qubits` 顺序测量所有比特，包括空闲比特。
- 有末端测量声明：按声明顺序和声明内的比特顺序合并，重复比特只保留首次出现的位置。
- `result.qubits[i]` 对应结果的第 `i` 位，字符串将第 0 位显示在最右侧。例如，输出比特为 `[5, 9]` 时，`"01"` 表示比特 5 为 1、比特 9 为 0。

显式 `qubits` 选择最终 Z 基测量的输出，不会跳过对线路中其他操作的检查。返回值描述量子比特的测量结果，不包含 `Store` 计算后的经典变量值。

线路参数必须已经绑定。支持末端测量：一个比特测量后不能再参与量子门或重置，其他独立比特仍可继续演化。中途测量和经典控制流会明确报错。Barrier、Delay 作为无操作处理。`statevector` 和 `density_matrix` 支持现有模拟器能处理的数值量子门；`stabilizer` 仅支持现有稳定子实现的 Clifford 门。

含 `reset` 的线路须显式选择 `simulator="density_matrix"`，以正确保留重置后的混态统计。需要噪声模型或直接操作状态时，继续使用下面的状态对象 API。

---

## 从 Bell 态开始

Bell 态是 QIS 入门最合适的例子：它很短，但同时包含叠加、纠缠、概率分布和可观测量期望值。

```python
from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString, Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = Statevector.from_circuit(circuit)

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 1.0)

print(state.probabilities())
print(hamiltonian.expectation_statevector(state))
```

理想 Bell 态的计算基概率应集中在 `|00>` 和 `|11>`，因此 `ZZ` 的期望值接近 `1.0`。这类检查常用于确认线路结构、比特索引和可观测量定义是否一致。

---

## 如何选择状态表示

不同状态表示的内存和语义不同。写算法原型时，建议先明确问题需要哪一种状态。

| 任务 | 推荐对象 | 原因 |
|---|---|---|
| 只需要理想纯态 | `Statevector` | 内存开销低于密度矩阵，适合快速验证 |
| 需要混态或 Kraus 通道 | `DensityMatrix` | 可以表达相干项、退相干和非纯态 |
| 需要自动套用设备噪声模型 | `DensityMatrixNoise` | 与 `NoiseModel` 结合，门后自动注入噪声 |
| 线路只含 Clifford 门 | `StabilizerState` | 对 Clifford 结构更高效，适合稳定子分析 |
| 只需要从采样概率估计期望值 | `PauliString` / `Hamiltonian` | 可以从 Pauli 测量结果计算期望值 |

如果线路包含测量、重置或动态控制流，通常不能把整条线路看成单一酉矩阵。此时应根据具体目标选择：调试结构用可视化，分析理想酉片段用 `Statevector`，分析噪声和混态用 `DensityMatrix` 或 `DensityMatrixNoise`。

---

## QIS 的典型工作流

一个常见闭环如下：先构造线路，再选择状态模拟器，接着定义可观测量，最后计算概率、期望值或指标。

```python
from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString, Statevector, metrics

ansatz = Circuit(2)
ansatz.ry(0, 0.3)
ansatz.cx(0, 1)
ansatz.ry(1, -0.2)

state = Statevector.from_circuit(ansatz)

observable = Hamiltonian.from_list([
    (PauliString.from_str("ZI"), -1.0),
    (PauliString.from_str("IZ"), -1.0),
    (PauliString.from_str("ZZ"), 0.5),
])

print("probabilities:", state.probabilities())
print("energy:", observable.expectation_statevector(state))
print("purity:", metrics.purity_pure(state))
```

这个流程对应很多算法的内层计算：VQE 关心 `energy`，QAOA 关心成本哈密顿量期望值，误差诊断关心保真度、迹距离、纯度和熵。

---

## 本章学习路线

阅读顺序如下：

1. [Statevector 纯态模拟](1_statevector.md)：学习纯态构造、门作用、线路执行、采样和期望值。
2. [DensityMatrix 混态模拟](2_density_matrix.md)：学习密度矩阵、Kraus 噪声、部分迹和物理性检查。
3. [DensityMatrixNoise 含噪模拟](3_density_matrix_noise.md)：学习如何把 `NoiseModel` 用于门噪声和读出误差模拟。
4. [StabilizerState 稳定子模拟](4_stabilizer.md)：学习 Clifford 线路的稳定子表示、测量和生成元分析。
5. [Pauli、PauliString 与 Hamiltonian](5_pauli_and_hamiltonian.md)：学习 Pauli 群、对易性、哈密顿量、期望值和 Trotter 演化。
6. [量子态指标与熵](6_metrics_entropy.md)：学习纯度、保真度、迹距离、Von Neumann 熵、Rényi 熵和纠缠指标。

建议先阅读前两节，建立纯态和混态的基本语义，再阅读 Pauli 与 Hamiltonian。含噪模拟、稳定子模拟和指标计算可以根据算法需求穿插阅读。

---

## 什么时候使用 QIS 模块

在量子程序开发中，QIS 模块通常用于以下位置：

- 写完一段小线路后，验证最终概率分布是否符合预期；
- 把 ansatz 绑定到一组参数后，计算可观测量期望值；
- 对比理想态、含噪态和目标态之间的保真度或迹距离；
- 对混态做部分迹，查看子系统状态；
- 用 PauliString 检查对易性、测量基和哈密顿量项；
- 把 Pauli 哈密顿量转换为 Trotter 演化线路；
- 用稳定子模拟器快速检查 Clifford 线路。

QIS 数值结果不能替代线路可视化和单元测试。推荐在关键算法模块中同时保留线路图、概率检查、期望值检查和必要的物理性检查。

---

## 下一步

·[Statevector 纯态模拟](1_statevector.md):先用理想纯态检查门序列、概率分布、采样和可观测量期望值。
·[DensityMatrix 混态模拟](2_density_matrix.md):学习密度矩阵、Kraus 噪声、部分迹和物理性验证。
·[Pauli、PauliString 与 Hamiltonian](5_pauli_and_hamiltonian.md):掌握 Pauli 表示、哈密顿量建模、期望值计算和 Trotter 演化。
