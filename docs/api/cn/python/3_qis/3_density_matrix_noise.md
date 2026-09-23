# DensityMatrixNoise

`cqlib.qis.state`

`cqlib.qis.state` 中的含噪混合态模拟器，覆盖 `DensityMatrixNoise` 类：噪声模型绑定、带噪声的门作用、读出噪声下的概率与采样。

## 导入

```python
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise
```

---

## DensityMatrixNoise

在密度矩阵内核之上按噪声模型自动施加 Kraus 噪声的模拟器。每次门作用后，模型中对应该门的噪声立即作用到状态上；读出噪声则在概率计算与采样阶段生效。

数据按行优先顺序连续存放。比特 i 对应基态下标的第 i 位，比特 0 为最低位；在位串表示中，编号大的比特位于左侧。构造后状态为 |0…0⟩⟨0…0|。

### DensityMatrixNoise(num_qubits, noise_model=None)

创建一个模拟器。省略噪声模型时等价于理想模拟。

参数：

- `num_qubits` (`int`)：量子比特数。
- `noise_model` (`NoiseModel | None`)：门噪声与读出噪声的定义，默认 `None`。

示例：

```python
import math

from cqlib.device import NoiseModel
from cqlib.qis.state import DensityMatrixNoise

sim = DensityMatrixNoise(2)
assert sim.num_qubits == 2
assert sim.state.shape == (4, 4)

probs = sim.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)

with_model = DensityMatrixNoise(2, NoiseModel())
assert with_model.num_qubits == 2
```

---

### 静态方法

- `DensityMatrixNoise.from_circuit(circuit, noise_model=None) -> DensityMatrixNoise`：模拟线路，返回演化后的模拟器。线路在执行前会被分解到基础门，噪声按模型在每个门之后施加。输入线路不会被修改。
  - `circuit` (`Circuit`)：待模拟线路。
  - `noise_model` (`NoiseModel | None`)：噪声模型，默认 `None`。
  - 线路含不受支持的操作时抛出 `ValueError`。

示例：

```python
import math

from cqlib import Circuit
from cqlib.device import NoiseModel
from cqlib.qis.state import DensityMatrixNoise

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

ideal = DensityMatrixNoise.from_circuit(circuit)
assert math.isclose(ideal.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(ideal.probabilities()[3], 0.5, abs_tol=1e-10)

noisy = DensityMatrixNoise.from_circuit(circuit, NoiseModel())
assert noisy.num_qubits == 2
```

---

### 属性

- `num_qubits -> int`：量子比特数。
- `state -> numpy.ndarray`：形状为 (2^num_qubits, 2^num_qubits) 的二维复矩阵表示当前状态。每次读取返回副本，修改返回值不影响模拟器。

示例：

```python
import math

from cqlib.qis.state import DensityMatrixNoise

sim = DensityMatrixNoise(2)
sim.apply_h(0)
sim.apply_cx(0, 1)

rho = sim.state
assert rho.shape == (4, 4)
assert math.isclose(abs(rho[0, 3]), 0.5, abs_tol=1e-10)

rho[0, 0] = 999.0
assert sim.state[0, 0] != 999.0
```

---

### 方法

所有门作用与状态改写方法均就地修改当前状态，返回 `None`。每个门方法在作用后会按噪声模型施加该门对应的噪声。

**电路作用**

- `apply_circuit(circuit) -> None`：把线路作用到当前状态上，线路在执行前会被分解到基础门。线路比特数与状态不匹配时抛出 `ValueError`。

**标准门对象**

- `apply_standard_gate_noise(gate, qubits, params=None) -> None`：按 `StandardGate` 描述作用门并施加对应噪声。
  - `gate` (`StandardGate`)：标准门。
  - `qubits` (`list[int]`)：目标比特下标列表。
  - `params` (`list[float] | None`)：参数化门的参数，默认空列表。

**单比特门（无参数）**

`apply_x`、`apply_y`、`apply_z`、`apply_h`、`apply_s`、`apply_sdg`、`apply_t`、`apply_tdg`、`apply_x2p`、`apply_x2m`、`apply_y2p`、`apply_y2m`，签名均为 `(q) -> None`。

**单比特门（带角度）**

- `apply_rx(q, theta)`、`apply_ry(q, theta)`、`apply_rz(q, theta)`：绕 X、Y、Z 轴旋转 `theta` 弧度。
- `apply_phase(q, theta)`：相位门。
- `apply_xy(q, theta)`：XY 门。
- `apply_xy2p(q, theta)`：XY2P 门。
- `apply_xy2m(q, theta)`：XY2M 门。
- `apply_rxy(q, theta, phi)`：布洛赫球上的任意旋转。
- `apply_u(q, theta, phi, lambda_)`：一般单比特酉门，`U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`。

**全局相位**

- `apply_gphase(phi) -> None`：作用全局相位门。

**受控门与双比特门**

- `apply_cx(control, target)`：受控 X（CNOT）门。
- `apply_cy(control, target)`：受控 Y 门。
- `apply_cz(q0, q1)`：受控 Z 门。
- `apply_crx(control, target, theta)`：受控 RX 门。
- `apply_cry(control, target, theta)`：受控 RY 门。
- `apply_crz(control, target, theta)`：受控 RZ 门。
- `apply_swap(q0, q1)`：SWAP 门。
- `apply_rxx(q0, q1, theta)`：RXX（Ising XX）门。
- `apply_ryy(q0, q1, theta)`：RYY（Ising YY）门。
- `apply_rzz(q0, q1, theta)`：RZZ（Ising ZZ）门。
- `apply_rzx(q0, q1, theta)`：RZX 门。
- `apply_fsim(q0, q1, theta, phi)`：费米子模拟门，`theta` 为 iSWAP 角，`phi` 为受控相位角。
- `apply_ccx(c1, c2, t)`：CCX（Toffoli）门，`c1`、`c2` 为控制位，`t` 为目标位。

**自定义矩阵门**

- `apply_unitary_gate(qubits, matrix) -> None`：作用作用于 `qubits` 的任意比特数酉门。该入口不施加噪声。
  - `qubits` (`list[int]`)：门作用的比特下标列表。
  - `matrix` (`numpy.ndarray`)：2^n × 2^n 复矩阵，n 为 `qubits` 长度；形状与比特数不符时抛出 `ValueError`。

**观测值**

- `expectation(observable) -> float`：计算带噪声状态下的观测值。
  - `observable` (`Hamiltonian | PauliString`)：可观测量。比特数不匹配或类型不受支持时抛出 `ValueError`。

**概率**

- `probabilities() -> list[float]`：返回不含读出噪声的各基态概率，长度为 2^num_qubits。不修改状态。
- `probabilities_with_readout(qubits) -> list[float]`：返回在指定比特上施加读出噪声后的概率分布，长度为 2^n，n 为 `qubits` 长度。
  - `qubits` (`list[int]`)：参与读出噪声建模的比特下标列表。下标越界时抛出 `IndexError`。

**测量与采样**

- `measure(qubit) -> bool`：测量单个比特并坍缩状态，返回该比特的结果。
- `measure_all() -> Outcome`：测量全部比特并坍缩状态，返回 `Outcome`。位串按比特编号从高到低排列。
- `sample_shots(shots) -> list[Outcome]`：按含读出噪声的分布采样 `shots` 次，不修改状态。
- `sample(measurement, shots) -> ExecutionResult`：按测量回执采样，不修改状态。
  - `measurement` (`Measurement`)：由线路的测量操作得到的测量回执。
  - `shots` (`int`)：采样次数。
- `probs(measurement) -> dict[Outcome, float]`：按测量回执计算不含读出噪声的边缘概率，不修改状态。
- `probs_with_readout(measurement) -> dict[Outcome, float]`：按测量回执计算施加读出噪声后的边缘概率，不修改状态。

**复制**

- `copy() -> DensityMatrixNoise`：返回独立副本，噪声模型一并复制。修改原模拟器不影响副本。

---

### 其他行为

- `repr(sim)` 返回形如 `DensityMatrixNoise(num_qubits=N, state_shape=(2^N, 2^N))` 的字符串。
- 未实现 `__eq__`：`==` 为默认的对象同一性比较，不比较状态内容。
- 未实现 `__copy__`、`__deepcopy__`：标准库 `copy.copy()` 与 `copy.deepcopy()` 会抛出 `TypeError`。需要副本时使用 `copy()` 方法。

---

### 异常情况

- `IndexError`：比特下标越界。
- `ValueError`：其他校验失败，包括自定义门矩阵形状与比特数不符、可观测量类型或比特数不匹配、线路含不受支持的操作、线路比特数与状态不匹配。
- `OverflowError`：`num_qubits` 为负数。

---

示例：

```python
import math

import numpy as np

from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise

# 理想模拟：不带噪声模型时与纯密度矩阵结果一致
ideal = DensityMatrixNoise(2)
ideal.apply_h(0)
ideal.apply_cx(0, 1)
assert math.isclose(ideal.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(ideal.probabilities()[3], 0.5, abs_tol=1e-10)

# 比特翻转噪声：X 门后以 p=0.1 翻转
noise_model = NoiseModel()
noise_model.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.1))

sim = DensityMatrixNoise(1, noise_model)
sim.apply_x(0)
probs = sim.probabilities()
assert math.isclose(probs[1], 0.9, abs_tol=0.01)
assert math.isclose(probs[0], 0.1, abs_tol=0.01)

# 理想概率与读出噪声下的概率相互独立
circuit_readout = NoiseModel()
circuit_readout.add_readout_error(0, ReadoutError(p_0_given_1=0.1, p_1_given_0=0.2))

readout_sim = DensityMatrixNoise(1, circuit_readout)
readout_sim.apply_x(0)
assert readout_sim.probabilities() == [0.0, 1.0]
readout_probs = readout_sim.probabilities_with_readout([0])
assert math.isclose(readout_probs[0], 0.1, abs_tol=1e-10)
assert math.isclose(readout_probs[1], 0.9, abs_tol=1e-10)

# 副本独立，且保留噪声行为
snapshot = sim.copy()
sim.apply_h(0)
assert math.isclose(snapshot.probabilities()[1], 0.9, abs_tol=0.01)

# 自定义酉门入口不施加噪声
matrix = np.array(
    [
        [1 / np.sqrt(2), 0, 1 / np.sqrt(2), 0],
        [0, 1 / np.sqrt(2), 0, 1 / np.sqrt(2)],
        [0, 1 / np.sqrt(2), 0, -1 / np.sqrt(2)],
        [1 / np.sqrt(2), 0, -1 / np.sqrt(2), 0],
    ],
    dtype=complex,
)
plain = DensityMatrixNoise(2)
plain.apply_unitary_gate([0, 1], matrix)
assert math.isclose(plain.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(np.trace(plain.state @ plain.state).real, 1.0, abs_tol=1e-10)
```

---

## 异常

`DensityMatrixNoise` 的校验失败按来源分为三类：

| 异常 | 触发场景 |
| --- | --- |
| `IndexError` | 比特下标越界，包括 `probabilities_with_readout` 的 `qubits` 超出范围。 |
| `ValueError` | 自定义门矩阵形状与比特数不符、可观测量类型或比特数不匹配、线路比特数与状态不匹配、线路含不受支持的操作。 |
| `OverflowError` | `num_qubits` 为负数。 |
