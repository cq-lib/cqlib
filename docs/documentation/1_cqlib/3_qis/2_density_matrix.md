# DensityMatrix 混态模拟

`DensityMatrix` 用密度矩阵表示量子态。它既可以表达纯态，也可以表达混态和噪声后的状态，适合进行 Kraus 通道、部分迹、物理性检查、混态期望值和熵指标分析。

密度矩阵的内存规模是 `2^n × 2^n`，通常比 `Statevector` 更重。只有在需要混态语义、噪声通道或子系统分析时，才应优先选择它。

---

## 任务：用密度矩阵制备 `|+>` 态

```python
from cqlib.qis import DensityMatrix

density = DensityMatrix(1)
density.apply_h(0)

print(density.data)
print(density.trace())
print(density.probabilities())
```

`data` 返回二维密度矩阵，`trace()` 应接近 `1.0`，`probabilities()` 返回计算基上的对角概率。对于 `|+>` 态，概率是 `[0.5, 0.5]`，但密度矩阵还包含非对角相干项。

---

## 从纯态振幅或线路创建

`DensityMatrix.from_state()` 接收 qubit 数和纯态振幅；`DensityMatrix.from_circuit()` 会把线路作用到初始 `|0...0>` 后得到密度矩阵。

```python
import numpy as np
from cqlib import Circuit
from cqlib.qis import DensityMatrix

plus = DensityMatrix.from_state(
    1,
    np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex),
)
print(plus.data)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell_density = DensityMatrix.from_circuit(circuit)
print(bell_density.probabilities())
```

如果已经有完整密度矩阵，可以用 `from_density_matrix()`。输入矩阵需要满足维度、迹和物理性要求。

```python
import numpy as np
from cqlib.qis import DensityMatrix

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

print(mixed.probabilities())
```

---

## 施加 Kraus 噪声

密度矩阵可以直接施加 Kraus 算符。下面示例对 `|1>` 态施加振幅阻尼。

```python
import numpy as np
from cqlib.qis import DensityMatrix

gamma = 0.05
k0 = np.array([[1, 0], [0, np.sqrt(1 - gamma)]], dtype=complex)
k1 = np.array([[0, np.sqrt(gamma)], [0, 0]], dtype=complex)

density = DensityMatrix(1)
density.apply_x(0)

density.apply_kraus([0], [k0.flatten(), k1.flatten()])

print(density.probabilities())
```

Kraus 通道适合手动验证噪声数学形式。如果已经有设备级 `NoiseModel`，可以使用 [DensityMatrixNoise 含噪模拟](3_density_matrix_noise.md) 自动在门后加入噪声。

---

## 对子系统做部分迹

`partial_trace(keep=[...])` 会保留指定 qubit，迹掉其余 qubit。它适合分析纠缠态中的局部子系统。

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrix

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell_density = DensityMatrix.from_circuit(circuit)
subsystem = bell_density.partial_trace(keep=[0])

print(subsystem.data)
print(subsystem.probabilities())
```

对 Bell 态任意单比特做部分迹，会得到接近最大混合态的局部密度矩阵。这个结果说明单个 qubit 本身没有确定 Bloch 方向，但全局二比特状态仍然具有纠缠相关性。

---

## 物理性检查

当密度矩阵来自外部数据、数值重构或自定义噪声通道时，应检查它是否仍然是合法量子态。

```python
from cqlib.qis import DensityMatrix

density = DensityMatrix(1)
density.apply_h(0)

tol = 1e-10
print(density.is_hermitian(tol))
print(density.is_positive_semidefinite(tol))
density.validate_physical(tol)
```

物理密度矩阵至少应满足 Hermitian、半正定和迹为 `1`。如果 `validate_physical()` 抛出异常，通常说明输入矩阵、噪声通道或数值后处理存在问题。

---

## 期望值、测量和采样

`DensityMatrix.expectation()` 同样可以接收 `PauliString` 或 `Hamiltonian`。此外，密度矩阵也提供 `measure()`、`measure_all()`、`sample_shots()`、`sample()` 和 `probs()` 等状态模拟接口。

```python
from cqlib.qis import DensityMatrix, Hamiltonian, PauliString

density = DensityMatrix(2)
density.apply_h(0)
density.apply_cx(0, 1)

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])

print(density.expectation(hamiltonian))
print(density.sample_shots(8))
```

测量方法会改变当前状态；采样方法适合生成有限 shot 结果。做噪声模拟或态指标分析时，建议先保存一份 `copy()`，避免测量坍缩影响后续计算。

---

## 下一步

·[DensityMatrixNoise 含噪模拟](3_density_matrix_noise.md):把手动 Kraus 噪声扩展为基于 `NoiseModel` 的自动含噪线路模拟。  
·[量子态指标与熵](6_metrics_entropy.md):用纯度、熵、保真度和纠缠指标解释密度矩阵结果。  
·[可视化量子态](../5_visualization/7_state_visualization.md):用 state city 和 Pauli vector 检查密度矩阵的相干项和 Pauli 展开。
