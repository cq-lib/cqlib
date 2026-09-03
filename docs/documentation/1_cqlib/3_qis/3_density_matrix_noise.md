# DensityMatrixNoise 含噪模拟

`DensityMatrixNoise` 是带噪声模型的密度矩阵模拟器。它在执行量子门时根据 `NoiseModel` 自动注入单比特门噪声、双比特门噪声和读出误差，适合对比理想线路与含噪线路的概率、期望值和采样结果。

底层仍然是密度矩阵，因此规模随 qubit 数增长较快。建议先用小线路验证噪声配置，再扩大到算法实验。

---

## 任务：给 Bell 态加入门噪声

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(
    StandardGate.H,
    0,
    SingleQubitNoise.depolarizing(0.001),
)
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.01),
)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

simulator = DensityMatrixNoise.from_circuit(circuit, noise_model)
print(simulator.probabilities())
```

理想 Bell 态只在 `00` 和 `11` 上有主峰。加入噪声后，`01` 和 `10` 可能出现非零概率。读图或读数时，应先确认噪声模型配置在哪些门和哪些 qubit 上。

---

## 配置常见噪声类型

`NoiseModel` 可以分别添加单比特门噪声、双比特门噪声和读出误差。

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise, TwoQubitNoise

noise_model = NoiseModel()

noise_model.add_single_qubit_error(
    StandardGate.RY,
    0,
    SingleQubitNoise.bit_flip(0.002),
)
noise_model.add_single_qubit_error(
    StandardGate.H,
    1,
    SingleQubitNoise.phase_flip(0.001),
)
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.01),
)
noise_model.add_readout_error(
    0,
    ReadoutError(p_0_given_1=0.02, p_1_given_0=0.01),
)
```

单比特噪声和双比特噪声会影响量子态演化；读出误差影响观测概率或采样结果，不等价于门后的量子态退相干。

---

## 区分态概率和读出概率

`probabilities()` 返回不含读出误差的量子态概率；`probabilities_with_readout(qubits)` 会在指定测量 qubit 上叠加读出误差。

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_readout_error(0, ReadoutError(0.02, 0.03))

simulator = DensityMatrixNoise(1, noise_model)
simulator.apply_x(0)

print("state probabilities:", simulator.probabilities())
print("readout probabilities:", simulator.probabilities_with_readout([0]))
```

如果只想分析量子门噪声后的真实量子态，应查看 `probabilities()` 或底层密度矩阵；如果想模拟实验读数，应查看带 readout 的概率或采样接口。

---

## 逐门执行含噪模拟

除了 `from_circuit()`，也可以像状态模拟器一样逐门调用 `apply_*` 方法。每个门执行后都会根据当前 `NoiseModel` 查找对应噪声。

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(
    StandardGate.X,
    0,
    SingleQubitNoise.bit_flip(0.01),
)

simulator = DensityMatrixNoise(1, noise_model)
simulator.apply_x(0)

print(simulator.probabilities())
```

逐门执行适合定位噪声来源。复杂线路中，如果结果异常，可以把线路拆成若干段，每段后检查概率或期望值。

---

## 期望值和采样

`DensityMatrixNoise` 支持与 `Statevector`、`DensityMatrix` 类似的期望值和采样接口。

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrixNoise, Hamiltonian, PauliString

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

simulator = DensityMatrixNoise.from_circuit(circuit)

observable = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
])

print(simulator.expectation(observable))
print(simulator.sample_shots(16))
```

如果 `NoiseModel` 里配置了读出误差，普通期望值通常表示噪声演化后的量子态期望值；带读出误差的观测结果应通过 readout 概率或相应采样接口单独处理。


---

## 下一步

·[DensityMatrix 混态模拟](2_density_matrix.md):回到密度矩阵本身，理解 Kraus 通道、部分迹和物理性检查。  
·[可视化执行结果](../5_visualization/6_result_visualization.md):把含噪采样结果画成 histogram 或概率分布，检查主峰和长尾。  
·[设备噪声](../2_device/4_noise.md):了解 `NoiseModel`、门噪声和读出误差在设备模块中的建模方式。
