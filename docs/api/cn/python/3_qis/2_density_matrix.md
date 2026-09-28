# DensityMatrix

`cqlib.qis.state`

`cqlib.qis.state` 中的混合态模拟器，覆盖 `DensityMatrix` 类：状态构造、门作用与量子信道、部分迹、物理性校验、测量与采样、观测值计算。

## 导入

```python
from cqlib.qis.state import DensityMatrix
```

---

## DensityMatrix

用 2^N × 2^N 的密度矩阵 ρ 描述 N 比特量子态的模拟器。与只能表示纯态的态矢量不同，密度矩阵可以表示纯态的经典混合。

数据按行优先顺序连续存放。比特 i 对应基态下标的第 i 位，比特 0 为最低位；在位串表示中，编号大的比特位于左侧。构造后状态为 |0…0⟩⟨0…0|。

### DensityMatrix(num_qubits)

创建一个处于 |0…0⟩⟨0…0| 的纯态。

参数：

- `num_qubits` (`int`)：量子比特数。

示例：

```python
import math

from cqlib.qis.state import DensityMatrix

dm = DensityMatrix(2)
assert dm.num_qubits == 2

probs = dm.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)
assert math.isclose(probs[1], 0.0)
```

---

### 静态方法

- `DensityMatrix.from_state(num_qubits, initial_state) -> DensityMatrix`：由纯态振幅构造，内部取外积 ρ = |ψ⟩⟨ψ|。`initial_state` 为长度 2^num_qubits 的 NumPy 复数组或复数列表。
  - 长度不符或振幅未归一化时抛出 `ValueError`。
- `DensityMatrix.from_density_matrix(num_qubits, dm_state) -> DensityMatrix`：由展平的密度矩阵数据直接构造。
  - `dm_state` (`numpy.ndarray | list`)：长度 4^num_qubits 的复数序列。
  - 长度不符、迹不为 1、非厄米或不满足半正定时抛出 `ValueError`。
- `DensityMatrix.from_circuit(circuit) -> DensityMatrix`：模拟线路从 |0…0⟩⟨0…0| 出发的演化，返回结果状态。输入线路不会被修改。
  - 线路含不受支持的操作时抛出 `ValueError`。
- `DensityMatrix.maximally_mixed(num_qubits) -> DensityMatrix`：构造最大混合态 I / 2^N，即对角元均为 1/2^N 的对角矩阵。`num_qubits` 为 0 时得到 1×1 的 `[1]`。

示例：

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import DensityMatrix

amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
dm = DensityMatrix.from_state(1, amps)
assert math.isclose(dm.probabilities()[0], 0.5)
assert math.isclose(dm.probabilities()[1], 0.5)

# |1><1|
data = np.array([0, 0, 0, 1], dtype=complex)
dm = DensityMatrix.from_density_matrix(1, data)
assert math.isclose(dm.probabilities()[1], 1.0)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = DensityMatrix.from_circuit(circuit)
assert math.isclose(bell.probabilities()[0], 0.5)
assert math.isclose(bell.probabilities()[3], 0.5)

mixed = DensityMatrix.maximally_mixed(2)
assert np.allclose(mixed.data, np.eye(4, dtype=complex) / 4, atol=1e-12)
assert math.isclose(mixed.trace(), 1.0)
```

---

### 属性

- `num_qubits -> int`：量子比特数。
- `data -> numpy.ndarray`：形状为 (2^num_qubits, 2^num_qubits) 的二维复矩阵。每次读取返回副本，修改返回值不影响状态。

示例：

```python
import numpy as np

from cqlib.qis.state import DensityMatrix

dm = DensityMatrix(2)
data = dm.data
assert data.shape == (4, 4)
assert data[0, 0] == 1.0
assert np.allclose(data[0, 1:], 0)

# data 是副本，就地修改不会写回状态
data[0, 0] = 999.0
assert dm.data[0, 0] == 1.0
```

---

### 方法

所有门作用与状态改写方法均就地修改当前状态，返回 `None`。

**电路作用**

- `apply_circuit(circuit) -> None`：把线路作用到当前状态上。线路比特数与状态不匹配时抛出 `ValueError`。

**标准门对象**

- `apply_standard_gate(gate, qubits, params=None) -> None`：按 `StandardGate` 描述作用门。
  - `gate` (`StandardGate`)：标准门。
  - `qubits` (`list[int]`)：目标比特下标列表。
  - `params` (`list[float] | None`)：参数化门的参数，默认空列表。

**单比特门（无参数）**

`apply_x`、`apply_y`、`apply_z`、`apply_h`、`apply_s`、`apply_sdg`、`apply_t`、`apply_tdg`、`apply_x2p`、`apply_x2m`、`apply_y2p`、`apply_y2m`，签名均为 `(qubit) -> None`。

**单比特门（带角度）**

- `apply_rx(qubit, theta)`、`apply_ry(qubit, theta)`、`apply_rz(qubit, theta)`：绕 X、Y、Z 轴旋转 `theta` 弧度。
- `apply_phase(qubit, theta)`：相位门。
- `apply_xy(qubit, theta)`：XY 门。
- `apply_xy2p(qubit, theta)`：XY2P 门。
- `apply_xy2m(qubit, theta)`：XY2M 门。
- `apply_rxy(qubit, theta, phi)`：RXY 门。
- `apply_u(qubit, theta, phi, lambda_)`：一般单比特酉门，`U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`。

**全局相位**

- `apply_gphase(phi) -> None`：对密度矩阵没有可观测影响，调用后状态不变。

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

**自定义矩阵门**

- `apply_single_qubit_gate(qubit, matrix) -> None`：作用 2×2 复矩阵给出的单比特门；形状不符时抛出 `ValueError`。
- `apply_double_qubits_gate(q0, q1, matrix) -> None`：作用 4×4 复矩阵给出的双比特门；形状不符时抛出 `ValueError`。
- `apply_unitary_gate(qubits, matrix) -> None`：作用作用于 `qubits` 的任意比特数酉门，演化形式为 ρ → U ρ U†。矩阵形状需为 2^n × 2^n，n 为 `qubits` 长度。
  - `qubits` (`list[int]`)：门作用的比特下标列表。
  - `matrix` (`numpy.ndarray`)：酉矩阵；形状与比特数不符时抛出 `ValueError`。

**量子信道**

- `apply_kraus(qubits, ops) -> None`：作用 Kraus 算符给出的量子信道，演化形式为 ρ → Σ_k K_k ρ K_k†。
  - `qubits` (`list[int]`)：信道作用的比特下标列表。
  - `ops` (`list[numpy.ndarray]`)：Kraus 算符列表，每个算符是展平的复数数组。Kraus 算符非法时抛出 `ValueError`。

**约化与校验**

- `partial_trace(keep) -> DensityMatrix`：对其余比特求部分迹，返回仅含 `keep` 给出比特的约化密度矩阵。
  - `keep` (`list[int]`)：约化后保留的比特下标列表。下标越界时抛出 `IndexError`。
- `trace() -> float`：返回迹（对角元之和）。物理态的迹恒为 1。取结果的实部。
- `is_hermitian(tol=1e-10) -> bool`：判断是否在容差内满足 ρ = ρ†。
- `is_positive_semidefinite(tol=1e-10) -> bool`：判断全部本征值是否满足 λᵢ ≥ -tol。对厄米矩阵该判定是确定的；非厄米矩阵或含 NaN、Inf 的矩阵返回 `False`。
- `validate_physical(tol=1e-10) -> None`：一次性校验厄米性、半正定性与单位迹。任一约束不满足时抛出 `ValueError`。

**观测值**

- `expectation(observable) -> float`：计算 Tr(ρ · O)。
  - `observable` (`Hamiltonian | PauliString`)：可观测量。比特数不匹配或类型不受支持时抛出 `ValueError`。

**测量与采样**

- `probabilities() -> list[float]`：返回各基态的概率，即密度矩阵的对角元 ρ_ii，长度为 2^num_qubits。不修改状态。
- `reset(qubit) -> None`：测量指定比特，若结果为 1 则翻转为 |0⟩。
- `measure(qubit) -> bool`：测量单个比特并坍缩状态，返回该比特的结果。
- `measure_all() -> Outcome`：测量全部比特并坍缩状态，返回 `Outcome`。位串按比特编号从高到低排列。
- `sample_shots(shots) -> list[Outcome]`：按当前概率分布采样 `shots` 次，不修改状态。
- `sample(measurement, shots) -> ExecutionResult`：按测量回执采样，不修改状态。
  - `measurement` (`Measurement`)：由线路的测量操作得到的测量回执。
  - `shots` (`int`)：采样次数。
- `probs(measurement) -> dict[Outcome, float]`：按测量回执计算边缘概率，不修改状态。

**复制**

- `copy() -> DensityMatrix`：返回独立副本，修改原状态不影响副本。

---

### 其他行为

- `repr(dm)` 返回形如 `DensityMatrix(num_qubits=N, shape=(2^N, 2^N))` 的字符串。
- 未实现 `__eq__`：`==` 为默认的对象同一性比较，不比较矩阵内容。
- 未实现 `__copy__`、`__deepcopy__`：标准库 `copy.copy()` 与 `copy.deepcopy()` 会抛出 `TypeError`。需要副本时使用 `copy()` 方法。

---

### 异常情况

- `IndexError`：`partial_trace` 的比特下标越界。
- `ValueError`：其他校验失败，包括振幅长度不符或未归一化、密度矩阵数据长度不符或违反物理约束、双比特门的目标比特重复、自定义门矩阵形状不符、Kraus 算符非法、可观测量类型或比特数不匹配、`validate_physical` 未通过。
- `TypeError`：`from_state` 与 `from_density_matrix` 收到既非数组也非列表的输入；`is_hermitian`、`is_positive_semidefinite`、`validate_physical` 的 `tol` 不是浮点数。

---

示例：

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString
from cqlib.qis.state import DensityMatrix

# 用线路构造 Bell 态
circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

dm = DensityMatrix.from_circuit(circuit)
assert dm.is_hermitian() is True
assert dm.is_positive_semidefinite() is True
assert dm.validate_physical() is None
assert math.isclose(dm.trace(), 1.0, abs_tol=1e-10)

# 观测量
assert math.isclose(dm.expectation(PauliString.from_str("ZI")), 0.0, abs_tol=1e-10)

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 0.5)
hamiltonian.add_term(PauliString.from_str("XX"), 0.5)
hamiltonian.simplify()
assert math.isclose(dm.expectation(hamiltonian), 1.0, abs_tol=1e-10)

# 部分迹：Bell 态的单比特约化是最大混合态
reduced = dm.partial_trace([0])
assert reduced.num_qubits == 1
assert math.isclose(reduced.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(reduced.probabilities()[1], 0.5, abs_tol=1e-10)

# 量子信道：退极化信道保持迹为 1
noisy = DensityMatrix(1)
p = 0.3
k0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
k1 = np.sqrt(p / 3) * np.array([[0, 1], [1, 0]], dtype=complex)
k2 = np.sqrt(p / 3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
k3 = np.sqrt(p / 3) * np.array([[1, 0], [0, -1]], dtype=complex)
noisy.apply_kraus([0], [k0.flatten(), k1.flatten(), k2.flatten(), k3.flatten()])
assert np.isclose(noisy.trace(), 1.0, atol=1e-10)

# 副本独立
snapshot = dm.copy()
dm.apply_h(0)
assert math.isclose(snapshot.probabilities()[0], 0.5, abs_tol=1e-10)
```

---

## 异常

`DensityMatrix` 的校验失败按来源分为三类：

| 异常 | 触发场景 |
| --- | --- |
| `IndexError` | `partial_trace` 的比特下标越界。 |
| `ValueError` | 振幅长度与 2^num_qubits 不符、振幅未归一化、密度矩阵数据长度不符或违反迹、厄米性、半正定约束、双比特门的目标比特重复、自定义门矩阵形状不符、Kraus 算符非法、可观测量类型或比特数不匹配、`validate_physical` 未通过。 |
| `TypeError` | `from_state`、`from_density_matrix` 收到不支持的类型；`tol` 参数类型错误。 |
