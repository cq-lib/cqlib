# Statevector 纯态模拟

`Statevector` 用于本地理想纯态模拟。它把 `n` 比特状态保存为长度为 `2^n` 的复振幅向量，适合验证小中规模酉线路、检查概率分布、计算 Pauli/Hamiltonian 期望值，并模拟有限 shot 采样。

`Statevector` 默认初始化为 `|0...0>`。示例都不连接硬件，也不引入噪声。

---

## 任务：制备并读取 Bell 态

```python
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(state.data)
print(state.probabilities())
```

`data` 返回复振幅数组，`probabilities()` 返回计算基概率分布。Bell 态的理想概率集中在索引 `0` 和 `3`，即 `|00>` 与 `|11>`。

阅读 `Statevector` 结果时，重点确认三件事：

- 振幅数组长度是否为 `2 ** num_qubits`；
- 概率和是否接近 `1.0`；
- 概率主峰是否落在预期 bitstring 上。

---

## 从初始振幅构造状态

如果已经有一组归一化复振幅，可以用 `from_state()` 直接构造纯态。

```python
import numpy as np
from cqlib.qis import Statevector

plus = Statevector.from_state(
    1,
    np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex),
)

print(plus.probabilities())
```

传入数组长度必须等于 `2 ** num_qubits`，并且状态需要归一化。这个入口适合把外部数值结果转换为 Cqlib 状态对象，再继续计算期望值、保真度或采样结果。

---

## 从 Circuit 执行纯态模拟

更常见的用法是先构造 `Circuit`，再用 `from_circuit()` 执行整条理想线路。

```python
from cqlib import Circuit
from cqlib.qis import Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = Statevector.from_circuit(circuit)
print(state.probabilities())
```

`Statevector.from_circuit()` 适合执行不含非酉操作的理想线路片段。如果线路里包含测量、重置或动态控制流，应先确认这些操作是否属于当前模拟目标；必要时改用逐步模拟、密度矩阵模拟或专门的动态执行流程。

---

## 对已有状态原地作用线路

当需要从同一个初态出发反复追加不同线路片段时，可以先创建状态，再调用 `apply_circuit()`。

```python
from cqlib import Circuit
from cqlib.qis import Statevector

prefix = Circuit(2)
prefix.h(0)

suffix = Circuit(2)
suffix.cx(0, 1)

state = Statevector(2)
state.apply_circuit(prefix)
state.apply_circuit(suffix)

print(state.probabilities())
```

这种写法适合分段调试。每执行一个模块后都可以打印概率或计算期望值，定位是哪一段线路改变了状态结构。

---

## 直接施加常用量子门

`Statevector` 提供与标准门相对应的 `apply_*` 方法，包括 Pauli 门、Clifford 门、旋转门、受控门、双比特旋转门、`fSim`、`CCX` 和用户自定义幺正矩阵。

```python
import numpy as np
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_x(0)
state.apply_rz(0, np.pi / 4)
state.apply_cx(0, 1)
state.apply_rzz(0, 1, 0.2)

print(state.probabilities())
```

对于自定义矩阵，可以使用 `apply_single_qubit_gate()`、`apply_double_qubits_gate()` 或 `apply_unitary_gate()`。

```python
import numpy as np
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_x(0)

swap = np.array(
    [
        [1, 0, 0, 0],
        [0, 0, 1, 0],
        [0, 1, 0, 0],
        [0, 0, 0, 1],
    ],
    dtype=complex,
)
state.apply_unitary_gate([0, 1], swap)

print(state.probabilities())
```

使用自定义矩阵时，需要自行保证矩阵维度与作用 qubit 数一致，并满足酉性要求。

---

## 计算可观测量期望值

`Statevector.expectation()` 可以接收 `PauliString` 或 `Hamiltonian`。

```python
from cqlib.qis import Hamiltonian, PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

zz = PauliString.from_str("ZZ")
print(state.expectation(zz))

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])
print(state.expectation(hamiltonian))
```

也可以从可观测量一侧调用 `expectation_statevector(state)`。在 VQE 或 QAOA 中，推荐固定使用一种写法，避免在日志和后处理代码中混淆状态对象与可观测量对象。

---

## 测量、重置和采样

`measure()` 会测量指定 qubit 并坍缩当前状态，`measure_all()` 会测量所有 qubit。`sample_shots()` 用于按当前分布采样，不会用于修改原状态的调试场景。

```python
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

shots = state.sample_shots(1000)
counts = {}
for outcome in shots:
    bitstring = outcome.to_bitstring(state.num_qubits)
    counts[bitstring] = counts.get(bitstring, 0) + 1

print(counts)
print(state.probabilities())
```

如果需要精确理论概率，用 `probabilities()`；如果需要模拟有限 shot 的统计涨落，用 `sample_shots()`。


---

## 下一步

·[DensityMatrix 混态模拟](2_density_matrix.md):在需要混态、Kraus 噪声和部分迹时，从纯态模拟切换到密度矩阵。  
·[Pauli、PauliString 与 Hamiltonian](5_pauli_and_hamiltonian.md):学习如何把状态结果连接到可观测量、能量函数和 Trotter 演化。  
·[可视化量子态](../5_visualization/7_state_visualization.md):用 Bloch、state city 和 Pauli vector 图解释纯态模拟结果。
