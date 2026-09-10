# 快速开始

本指南将带您使用 Cqlib 构建一个最小可用的量子线路，并完成一次端到端的“工程闭环”流程。

在开始之前，请确保您的环境已准备就绪：

- Python 环境：支持 Python 3.10 – 3.14
- 完成 Cqlib 安装与环境配置（安装方式见 “[Cqlib 安装与环境配置](../0_get_started/1_installation.md)”）

---

## 第一个量子线路：Bell 态

本节通过一个最小但完整的 Bell 态示例，介绍 Cqlib 构建量子线路的基本流程。通过该示例，您将了解如何创建线路、添加量子门、查看线路结构，并进一步进行状态模拟、测量采样、线路可视化和 IR 导出等操作。

Bell 态是量子计算中最常见的双比特纠缠态之一，通常用于演示叠加与纠缠的基本概念。构造 Bell 态需要以下两个步骤：

- 对第 `0` 个量子比特施加 `H` 门，使其进入叠加态；
- 以第 `0` 个量子比特为控制比特、第 `1` 个量子比特为目标比特施加 CX 门，从而在两个量子比特之间建立纠缠关系。

理想情况下，最终得到的量子态为：

```text
(|00> + |11>) / sqrt(2)
```

## 1. 创建线路

首先创建一条两比特量子线路，并向其中依次添加 `H` 门和 `CX` 门：

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(circuit.num_qubits)   # 2
print(len(circuit))         # 2
print(circuit.operations)   # 查看底层 Operation 列表
```

## 2. 运行线路并获取计数

使用 `sample()` 直接运行线路 1000 次，获取测量计数：

```python
from cqlib import sample

result = sample(circuit, shots=1000, seed=42)
print(result.counts)
```

输出结果类似：

```text
{'00': 506, '11': 494}
```

默认使用本地理想状态向量模拟器，从全零态开始，在 Z 基测量全部线路比特。若线路已经声明末端测量，则默认只返回这些比特的结果。`result` 是 `ExecutionResult`，可以直接读取 `counts`、`probabilities`、`shots` 和 `qubits`。

`seed=42` 使同一版本、相同线路和模拟器的采样计数可复现；省略 `seed` 时随机采样。Bell 态只会得到 `00` 和 `11`，计数通常大致相近。

也可以显式选择模拟器和输出比特顺序：

```python
result = sample(circuit, shots=1000, seed=42, simulator="density_matrix", qubits=[1, 0])
print(result.counts)
```

`qubits` 中第一个比特对应结果字符串最右侧。支持的模拟器包括 `statevector`、`density_matrix` 和 `stabilizer`；稳定子模拟器只支持其实现的 Clifford 门。线路参数须已绑定，中途测量和经典控制流会报错；含 `reset` 的线路请选择 `density_matrix`。完整规则见 [直接采样线路](../1_cqlib/3_qis/0_overview.md#直接采样线路)。

以下步骤用于进一步查看线路、分析状态和导出程序。

## 3. 使用文本图查看线路

将量子线路渲染为文本图以查看线路结构：

```python
from cqlib.visualization import draw_text

print(draw_text(circuit))
```

## 4. 转为矩阵验证

对于小规模纯量子门线路，可以将整条线路转换为完整酉矩阵，用于验证线路的数学行为：

```python
matrix = circuit.to_matrix()
print(matrix)
```

## 5. 导出 OpenQASM 2.0 / 3.0

此外，Cqlib 还提供了 OpenQASM 2.0 和 OpenQASM 3.0 的导出接口：

```python
from cqlib.ir import qasm2, qasm3

print(qasm2.dumps(circuit))
print(qasm3.dumps(circuit))
```

## 6. 状态向量模拟

使用状态向量模拟来查看线路作用后的量子态分布：

```python
from cqlib.qis import Statevector

sv = Statevector.from_circuit(circuit)
print(sv.data)
print(sv.probabilities())
```

对于 Bell 态线路，理想情况下，`probabilities()` 的结果应接近：

```text
[0.5, 0.0, 0.0, 0.5]
```

这表示测量时只会得到 `00` 和 `11` 两种结果，并且二者概率相同；而 `01` 和 `10` 的概率接近 `0`，这体现了 Bell 态中两个量子比特之间的纠缠关联。

---

## 下一步

- [量子线路](../1_cqlib/0_circuit/0_overview.md)：了解 Cqlib 中描述量子程序的基础模块。
- [量子门与指令](../1_cqlib/0_circuit/1_gates.md)：了解内置门、自定义门、复合门和非酉指令。
- [线路结构与构造](../1_cqlib/0_circuit/2_structures.md)：掌握 `Circuit` 的生命周期、索引、组合和操作表示。
- [天衍量子云平台客户端](../1_cqlib/7_tianyan/0_overview.md)：如果需要把线路提交到云端后端执行，可继续学习 QCIS 导出、后端选择、任务提交和结果获取流程。
