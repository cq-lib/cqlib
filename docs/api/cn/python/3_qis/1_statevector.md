# Statevector

`cqlib.qis.state`

`cqlib.qis.state` 中的纯态模拟器，覆盖 `Statevector` 类：状态构造、门作用、测量与采样、观测值计算。

## 导入

```python
from cqlib.qis.state import Statevector
```

---

## Statevector

用 2^N 个复振幅描述 N 比特纯量子态 |ψ⟩ = Σᵢ αᵢ|i⟩ 的模拟器。

振幅在内存中按下标连续存放，基态 |i⟩ 的振幅位于下标 `i`。比特 i 对应该下标的第 i 位，比特 0 为最低位；在位串表示中，编号大的比特位于左侧。构造后所有振幅为 0，仅 |0…0⟩ 为 1。

### Statevector(num_qubits)

创建一个处于 |0…0⟩ 的状态。

参数：

- `num_qubits` (`int`)：量子比特数。

示例：

```python
import math

from cqlib.qis.state import Statevector

sv = Statevector(2)
assert sv.num_qubits == 2

probs = sv.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)
assert math.isclose(probs[1], 0.0)
```

---

### 静态方法

- `Statevector.from_state(num_qubits, initial_state) -> Statevector`：用给定振幅构造。`initial_state` 为长度 2^num_qubits 的 NumPy 复数组，或由复数、浮点数组成的列表。
  - 长度与 2^num_qubits 不符、或振幅未归一化时抛出 `ValueError`。
- `Statevector.from_circuit(circuit) -> Statevector`：模拟线路从 |0…0⟩ 出发的演化，返回结果状态。输入线路不会被修改。
  - 线路含非酉操作、或线路与状态比特数不符时抛出 `ValueError`。

示例：

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import Statevector

amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
sv = Statevector.from_state(1, amps)
assert math.isclose(sv.probabilities()[0], 0.5)
assert math.isclose(sv.probabilities()[1], 0.5)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = Statevector.from_circuit(circuit)
assert math.isclose(bell.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(bell.probabilities()[3], 0.5, abs_tol=1e-10)
```

---

### 属性

- `num_qubits -> int`：量子比特数。
- `data -> numpy.ndarray`：长度 2^num_qubits 的一维复振幅数组。每次读取返回副本，修改返回值不影响状态。

示例：

```python
import numpy as np

from cqlib.qis.state import Statevector

sv = Statevector(2)
data = sv.data
assert len(data) == 4
assert data[0] == 1.0
assert np.allclose(data[1:], 0)

# data 是副本，就地修改不会写回状态
data[0] = 999.0
assert sv.data[0] == 1.0
```

---

### 方法

所有门作用与状态改写方法均就地修改当前状态，返回 `None`。

**电路作用**

- `apply_circuit(circuit) -> None`：把线路作用到当前状态上。线路比特数不匹配时抛出 `ValueError`。

**标准门对象**

- `apply_standard_gate(gate, qubits, params=None) -> None`：按 `StandardGate` 描述作用门。
  - `gate` (`StandardGate`)：标准门。
  - `qubits` (`list[int]`)：目标比特下标列表。
  - `params` (`list[float] | None`)：参数化门的参数，默认空列表。

**单比特门（无参数）**

`apply_x`、`apply_y`、`apply_z`、`apply_h`、`apply_s`、`apply_sdg`、`apply_t`、`apply_tdg`、`apply_x2p`、`apply_x2m`、`apply_y2p`、`apply_y2m`，签名均为 `(qubit) -> None`。

**单比特门（带角度）**

- `apply_rx(qubit, theta)`、`apply_ry(qubit, theta)`、`apply_rz(qubit, theta)`：绕 X、Y、Z 轴旋转 `theta` 弧度。
- `apply_phase(qubit, theta)`：相位门，`theta` 为相位角。
- `apply_xy(qubit, theta)`：XY 门。
- `apply_xy2p(qubit, theta)`：XY2P 门，等价于 `Rz(θ - π/2) Ry(π/2) Rz(π/2 - θ)`。
- `apply_xy2m(qubit, theta)`：XY2M 门，等价于 `Rz(-θ + π/2) Ry(-π/2) Rz(-π/2 + θ)`。
- `apply_rxy(qubit, theta, phi)`：RXY 门。
- `apply_u(qubit, theta, phi, lambda_)`：一般单比特酉门，`U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`。

**全局相位**

- `apply_gphase(phi)`：给整个状态乘上全局相位 `e^{iφ}`。不改变 `probabilities()`。

**Pauli 字符串旋转**

- `apply_pauli_rotation(pauli, theta)`：作用 `exp(-i θ/2 · P)`，要求 `P` 为厄米 Pauli 字符串且覆盖状态的全部比特。

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
- `apply_ccx(c0, c1, target)`：CCX（Toffoli）门。

双比特门的两个比特下标相同时抛出 `ValueError`。

**自定义矩阵门**

- `apply_single_qubit_gate(qubit, matrix) -> None`：作用 `matrix` 给出的单比特门。
  - `matrix` (`numpy.ndarray`)：2×2 复矩阵；形状不符时抛出 `ValueError`。
- `apply_double_qubits_gate(q0, q1, matrix) -> None`：作用 `matrix` 给出的双比特门。
  - `matrix` (`numpy.ndarray`)：4×4 复矩阵；形状不符时抛出 `ValueError`。
- `apply_unitary_gate(qubits, matrix) -> None`：作用作用于 `qubits` 的任意比特数酉门。
  - `qubits` (`list[int]`)：门作用的比特下标列表。
  - `matrix` (`numpy.ndarray`)：2^n × 2^n 复矩阵，n 为 `qubits` 长度；形状不符时抛出 `ValueError`。

**观测值**

- `expectation(observable) -> float`：计算 ⟨ψ|O|ψ⟩。
  - `observable` (`Hamiltonian | PauliString`)：可观测量。比特数不匹配或类型不受支持时抛出 `ValueError`。

**测量与状态改写**

- `probabilities() -> list[float]`：返回各基态的概率 p(i) = |αᵢ|²，长度为 2^num_qubits。不修改状态。
- `reset(qubit) -> None`：测量指定比特，若结果为 1 则翻转为 |0⟩。
- `measure(qubit) -> bool`：测量单个比特并坍缩状态，返回该比特的结果。
- `measure_all() -> Outcome`：测量全部比特并坍缩状态，返回 `Outcome`。位串按比特编号从高到低排列。
- `sample_shots(shots) -> list[Outcome]`：按当前概率分布采样 `shots` 次，不修改状态。
- `sample(measurement, shots) -> ExecutionResult`：按测量回执采样，不修改状态。
  - `measurement` (`Measurement`)：由线路的测量操作得到的测量回执。
  - `shots` (`int`)：采样次数。
- `probs(measurement) -> dict[Outcome, float]`：按测量回执计算边缘概率，不修改状态。

**复制**

- `copy() -> Statevector`：返回独立副本，修改原状态不影响副本。

---

### 其他行为

- `repr(sv)` 返回形如 `Statevector(num_qubits=N, amplitudes=2^N)` 的字符串。
- 未实现 `__eq__`：`==` 为默认的对象同一性比较，不比较状态内容。
- 未实现 `__copy__`、`__deepcopy__`：标准库 `copy.copy()` 与 `copy.deepcopy()` 会抛出 `TypeError`。需要副本时使用 `copy()` 方法。

---

### 异常情况

- `IndexError`：比特下标越界。
- `ValueError`：其他校验失败，包括振幅长度不符或未归一化、双比特门的目标比特重复、自定义门矩阵形状不符、可观测量类型或比特数不匹配、`apply_circuit` 的线路比特数不匹配。

---

示例：

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis import PauliString
from cqlib.qis.state import Statevector

# 用线路构造 Bell 态
circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

sv = Statevector.from_circuit(circuit)
probs = sv.probabilities()
assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
assert math.isclose(probs[3], 0.5, abs_tol=1e-10)

# 期望值
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)

# 采样不改变状态，结果只落在关联基态上
shots = sv.sample_shots(64)
assert len(shots) == 64
assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
assert math.isclose(sv.probabilities()[0], 0.5, abs_tol=1e-10)

# 测量并坍缩：先在基态上验证确定性结果
basis = Statevector(1)
basis.apply_x(0)
assert basis.measure(0) is True
assert math.isclose(basis.probabilities()[1], 1.0, abs_tol=1e-10)

# 全量测量后概率集中在单个基态上
sv.measure_all()
assert math.isclose(sum(sv.probabilities()), 1.0, abs_tol=1e-10)

# 副本独立：全局相位不改变概率分布
clone = sv.copy()
clone.apply_gphase(np.pi / 2)
assert np.allclose(sv.probabilities(), clone.probabilities(), atol=1e-10)
```

---

## 异常

`Statevector` 的校验失败按来源分为两类：

| 异常 | 触发场景 |
| --- | --- |
| `IndexError` | 比特下标越界。 |
| `ValueError` | 振幅长度与 2^num_qubits 不符、振幅未归一化、双比特门的目标比特重复、自定义门矩阵形状不符、可观测量类型或比特数不匹配、线路比特数不匹配、线路含非酉操作。 |
