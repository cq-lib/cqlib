# 量子态指标与熵

`cqlib.qis.metrics` 和 `cqlib.qis.entropy` 提供量子态相似度、混合度、熵和纠缠指标。它们常用于比较理想态与含噪态、判断密度矩阵是否退相干、分析子系统纠缠，以及为算法实验生成辅助评价指标。

使用前应先确认输入对象是 `Statevector` 还是 `DensityMatrix`。纯态指标和混态指标的函数名不同，不要混用。

---

## 常用函数一览

| 模块 | 函数 | 输入 | 含义 |
|---|---|---|---|
| `metrics` | `purity_pure` | `Statevector` | 纯态纯度 |
| `metrics` | `purity_mixed` | `DensityMatrix` | 混态纯度 `Tr(ρ²)` |
| `metrics` | `state_fidelity_pure` | 两个 `Statevector` | 纯态保真度 |
| `metrics` | `state_fidelity_pure_mixed` | `Statevector`, `DensityMatrix` | 纯态与混态保真度 |
| `metrics` | `state_fidelity_mixed` | 两个 `DensityMatrix` | 混态保真度 |
| `metrics` | `trace_distance_pure` | 两个 `Statevector` | 纯态迹距离 |
| `metrics` | `trace_distance_mixed` | 两个 `DensityMatrix` | 混态迹距离 |
| `metrics` | `entropy` | `DensityMatrix` | Von Neumann 熵 |
| `metrics` | `partial_transpose` | `DensityMatrix`, qubit 列表 | 部分转置 |
| `metrics` | `logarithmic_negativity` | `DensityMatrix`, 子系统 | 对数负性 |
| `entropy` | `linear_entropy` | `DensityMatrix` | 线性熵 |
| `entropy` | `renyi_entropy` | `DensityMatrix`, `alpha` | Rényi 熵 |
| `entropy` | `entanglement_entropy_pure` | `Statevector`, 子系统 | 纯态纠缠熵 |
| `entropy` | `negativity` | `DensityMatrix`, 子系统 | 负性 |
| `entropy` | `concurrence` | 2-qubit `DensityMatrix` | 两比特 concurrence |
| `entropy` | `entanglement_of_formation` | 2-qubit `DensityMatrix` | 纠缠形成 |

---

## 任务：比较两个纯态

```python
from cqlib.qis import Statevector, metrics

plus = Statevector(1)
plus.apply_h(0)

minus = Statevector(1)
minus.apply_h(0)
minus.apply_z(0)

print(metrics.purity_pure(plus))
print(metrics.state_fidelity_pure(plus, minus))
print(metrics.trace_distance_pure(plus, minus))
```

保真度越接近 `1`，两个态越相似；迹距离越接近 `0`，两个态越相似。对于正交纯态，保真度应接近 `0`，迹距离应接近 `1`。

---

## 分析混态纯度和熵

```python
import numpy as np
from cqlib.qis import DensityMatrix, entropy, metrics

mixed = DensityMatrix.from_density_matrix(
    1,
    np.array(
        [
            0.7 + 0.0j, 0.0 + 0.0j,
            0.0 + 0.0j, 0.3 + 0.0j,
        ],
        dtype=complex,
    ),
)

print(metrics.purity_mixed(mixed))
print(metrics.entropy(mixed))
print(entropy.linear_entropy(mixed))
print(entropy.renyi_entropy(mixed, alpha=2.0))
```

纯度越接近 `1`，状态越接近纯态；熵和线性熵越大，状态越混合。解释含噪结果时，建议同时查看概率分布和纯度/熵指标，因为概率相似不一定代表相干结构相同。

---

## 计算 Bell 态的纠缠熵

对纯态，可以通过对子系统做约化得到纠缠熵。

```python
from cqlib.qis import Statevector, entropy

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(entropy.entanglement_entropy_pure(state, [0]))
```

Bell 态任意一个单比特子系统都接近最大混合态，因此纠缠熵接近 `1` bit。对于乘积态，纠缠熵应接近 `0`。

---

## 计算负性和对数负性

混态纠缠分析常用负性或对数负性。下面先用 Bell 态线路构造密度矩阵，再计算子系统 `[0]` 的纠缠指标。

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrix, entropy, metrics

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

density = DensityMatrix.from_circuit(circuit)

print(entropy.negativity(density, [0]))
print(metrics.logarithmic_negativity(density, sys_a=[0]))
```

负性为 `0` 通常表示在对应划分下没有通过 PPT 标准检测到纠缠；负性越大，说明部分转置后出现的负特征值越明显。

---

## 两比特 concurrence 和纠缠形成

`concurrence()` 和 `entanglement_of_formation()` 针对两比特密度矩阵。

```python
from cqlib.qis import DensityMatrix, entropy

state = DensityMatrix(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(entropy.concurrence(state))
print(entropy.entanglement_of_formation(state))
```

这两个指标适合两比特纠缠分析。对于更多 qubit 的系统，应改用纠缠熵、负性、对数负性或按子系统划分的其他指标。

---

## 对比理想态和含噪态

下面把理想 Bell 态和一个含噪密度矩阵放在一起比较。

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, TwoQubitNoise
from cqlib.qis import DensityMatrix, DensityMatrixNoise, Statevector, metrics

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

ideal_state = Statevector.from_circuit(circuit)
ideal_density = DensityMatrix.from_circuit(circuit)

noise_model = NoiseModel()
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.02),
)
noisy = DensityMatrixNoise.from_circuit(circuit, noise_model)
noisy_density = DensityMatrix.from_density_matrix(2, noisy.state.reshape(-1))

print(metrics.state_fidelity_pure_mixed(ideal_state, noisy_density))
print(metrics.trace_distance_mixed(ideal_density, noisy_density))
print(metrics.purity_mixed(noisy_density))
```

这类指标适合回答“含噪态离理想态有多远”。如果只看最终 counts，可能会忽略相干项、相位和混合度的变化。

---

## 下一步

·[DensityMatrix 混态模拟](2_density_matrix.md):回到密度矩阵构造、部分迹和物理性验证，理解指标输入从哪里来。  
·[DensityMatrixNoise 含噪模拟](3_density_matrix_noise.md):用含噪模拟生成待比较的混态结果。  
·[可视化量子态](../5_visualization/7_state_visualization.md):把数值指标和 Bloch、state city、Pauli vector 图结合起来解释。
