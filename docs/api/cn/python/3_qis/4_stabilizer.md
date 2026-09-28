# Stabilizer

`cqlib.qis.state`

`cqlib.qis.state` 中的稳定子模拟器，覆盖 `StabilizerState` 与 `StabilizerCircuitResult` 两个类：Clifford 线路的模拟、稳定子生成元查询、测量采样，以及执行后运行期经典数据的读取。

## 导入

```python
from cqlib.qis.state import StabilizerCircuitResult, StabilizerState
```

---

## StabilizerState

用稳定子表描述 N 比特 Clifford 态的模拟器。Clifford 门把 Pauli 群映射到自身，因此状态可以用 N 个对易 Pauli 生成元紧凑表示，而无需存储全部振幅。

该类只接受 Clifford 门。含 T 门、一般角度旋转等非 Clifford 操作的线路无法模拟，此时抛出 `ValueError`。

比特 i 对应基态下标的第 i 位，比特 0 为最低位；在位串表示中，编号大的比特位于左侧。构造后状态为 |0…0⟩。

### StabilizerState(num_qubits)

创建一个处于 |0…0⟩ 的状态。

参数：

- `num_qubits` (`int`)：量子比特数。

示例：

```python
from cqlib.qis.state import StabilizerState

state = StabilizerState(3)
assert state.num_qubits == 3
assert state.probabilities() == [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
assert "StabilizerState" in repr(state)
```

---

### 静态方法

- `StabilizerState.from_circuit(circuit) -> StabilizerState`：模拟 Clifford 线路的演化，返回最终状态。输入线路不会被修改。线路含非 Clifford 操作时抛出 `ValueError`。
- `StabilizerState.run_circuit(circuit) -> StabilizerCircuitResult`：执行 Clifford 线路，返回同时包含最终状态与运行期经典数据的结果对象。输入线路不会被修改。
  - 线路含控制流、或含非 Clifford 操作时抛出 `ValueError`。

示例：

```python
from cqlib import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = StabilizerState.from_circuit(circuit)
assert bell.num_qubits == 2
assert bell.probabilities()[0] == 0.5
assert bell.probabilities()[3] == 0.5
```

---

### 属性

- `num_qubits -> int`：量子比特数。

---

### 方法

所有门作用与状态改写方法均就地修改当前状态，返回 `None`。

**电路作用**

- `apply_circuit(circuit) -> None`：把 Clifford 线路作用到当前状态上。线路含非 Clifford 操作或比特数不匹配时抛出 `ValueError`。

**单比特 Clifford 门**

`apply_h`、`apply_s`、`apply_sdg`、`apply_x`、`apply_y`、`apply_z`、`apply_x2p`、`apply_x2m`、`apply_y2p`、`apply_y2m`，签名均为 `(qubit) -> None`。

**双比特 Clifford 门**

- `apply_cx(control, target)`：受控 X 门。
- `apply_cy(control, target)`：受控 Y 门。
- `apply_cz(q0, q1)`：受控 Z 门。
- `apply_swap(q0, q1)`：SWAP 门。

两个比特下标相同时抛出 `ValueError`。

**稳定子表查询**

- `get_stabilizers() -> list[PauliString]`：返回 N 个稳定子生成元。
- `get_destabilizers() -> list[PauliString]`：返回 N 个反稳定子生成元。
- `to_stim_format() -> str`：返回稳定子生成元的逐行文本表示，每行一个生成元并带符号前缀，例如 `+XX`。

**观测值**

- `pauli_expectation(pauli) -> int`：返回 Pauli 字符串的期望值，取值 `-1`、`0` 或 `1`。比特数不匹配时抛出 `ValueError`。

**概率**

- `probabilities() -> list[float]`：返回各基态的概率分布，长度为 2^num_qubits。不修改状态。
- `probability_of(bits) -> float`：返回给定计算基位串的概率。
  - `bits` (`list[bool]`)：按比特编号给出各比特取值。长度与 `num_qubits` 不符时抛出 `ValueError`。

**测量与采样**

- `reset(qubit) -> None`：测量指定比特，若结果为 1 则翻转为 |0⟩。
- `measure(qubit) -> bool`：测量单个比特并坍缩状态，返回该比特的结果。
- `measure_all() -> Outcome`：测量全部比特并坍缩状态，返回 `Outcome`。位串按比特编号从高到低排列。
- `sample_shots(shots) -> list[Outcome]`：按当前概率分布采样 `shots` 次，不修改状态。
- `sample(measurement, shots) -> ExecutionResult`：按测量回执采样，不修改状态。
  - `measurement` (`Measurement`)：由线路的测量操作得到的测量回执。
  - `shots` (`int`)：采样次数。
- `probs(measurement) -> dict[Outcome, float]`：按测量回执计算边缘概率，不修改状态。

**复制**

- `copy() -> StabilizerState`：返回独立副本，修改原状态不影响副本。

---

### 其他行为

- `repr(state)` 返回形如 `StabilizerState(num_qubits=N)` 的字符串。
- 未实现 `__eq__`：`==` 为默认的对象同一性比较，不比较稳定子表内容。
- 未实现 `__copy__`、`__deepcopy__`：标准库 `copy.copy()` 与 `copy.deepcopy()` 会抛出 `TypeError`。需要副本时使用 `copy()` 方法。

---

### 异常情况

- `IndexError`：比特下标越界。
- `ValueError`：非 Clifford 操作、线路含控制流、双比特门的目标比特重复、`probability_of` 的位串长度与 `num_qubits` 不符、`pauli_expectation` 的比特数不匹配、线路比特数与状态不匹配。
- `OverflowError`：`num_qubits` 为负数。

---

示例：

```python
import math

from cqlib.qis import PauliString
from cqlib.qis.state import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

# 概率与单位置概率
probs = state.probabilities()
assert probs[0] == 0.5
assert probs[3] == 0.5
assert math.isclose(state.probability_of([False, False]), 0.5, abs_tol=1e-10)
assert math.isclose(state.probability_of([True, False]), 0.0, abs_tol=1e-10)

# 稳定子表
stabilizers = state.get_stabilizers()
destabilizers = state.get_destabilizers()
assert len(stabilizers) == 2
assert len(destabilizers) == 2
assert all(isinstance(pauli, PauliString) for pauli in stabilizers)
assert state.to_stim_format().count("\n") == 2

# 观测值
assert state.pauli_expectation(PauliString.from_str("ZZ")) == 1
assert state.pauli_expectation(PauliString.from_str("ZI")) == 0

# 采样不改变状态，结果只落在关联基态上
shots = state.sample_shots(64)
assert len(shots) == 64
assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
assert state.probabilities()[0] == 0.5

# 副本独立：快照保留副本创建时的分布
snapshot = state.copy()
state.apply_x(0)
assert snapshot.probabilities()[3] == 0.5
assert state.probabilities()[1] == 0.5
assert state.probabilities()[2] == 0.5

# 全量测量后状态坍缩到单个基态
outcome = state.measure_all()
assert outcome.to_bitstring(2) in {"01", "10"}

# 测量与复位：在确定性基态上结果可预期
basis = StabilizerState(1)
basis.apply_x(0)
assert basis.measure(0) is True
assert basis.probabilities()[1] == 1.0

basis.reset(0)
assert basis.probabilities()[0] == 1.0
```

---

## StabilizerCircuitResult

`StabilizerState.run_circuit()` 的返回对象，同时携带最终稳定子状态与执行过程中产生的运行期经典数据。

该对象没有公开构造函数，只能由 `StabilizerState.run_circuit()` 返回。

### 属性

- `state -> StabilizerState`：线路执行结束后的最终稳定子状态。
- `classical -> ClassicalState`：执行过程中产生的运行期经典值集合，见 [Classical State](5_classical_state.md)。

### 其他行为

- `repr(result)` 返回形如 `StabilizerCircuitResult(num_qubits=N)` 的字符串。
- 未实现 `__eq__`：`==` 为默认的对象同一性比较。
- 未实现 `__copy__`、`__deepcopy__`：标准库 `copy.copy()` 与 `copy.deepcopy()` 会抛出 `TypeError`。需要保留结果时重新调用 `StabilizerState.run_circuit()`。

---

示例：

```python
from cqlib import Circuit
from cqlib.qis.state import (
    ClassicalState,
    RuntimeValue,
    StabilizerCircuitResult,
    StabilizerState,
)

circuit = Circuit(2)
circuit.x(0)
measurement = circuit.measure(0)
circuit.reset(0)
circuit.h(1)

result = StabilizerState.run_circuit(circuit)
assert isinstance(result, StabilizerCircuitResult)
assert "StabilizerCircuitResult" in repr(result)

# 最终状态
assert result.state.num_qubits == 2
assert result.state.probabilities()[0] == 0.5
assert result.state.probabilities()[2] == 0.5

# 运行期经典数据
assert isinstance(result.classical, ClassicalState)
measured = result.classical.value(measurement.value)
assert isinstance(measured, RuntimeValue)
assert measured.kind == "bit"
assert measured.as_bit() is True
```

---

## 异常

两个类的校验失败按来源分为三类：

| 异常 | 触发场景 |
| --- | --- |
| `IndexError` | 比特下标越界。 |
| `ValueError` | 线路含非 Clifford 操作或控制流、双比特门的目标比特重复、`probability_of` 的位串长度不符、`pauli_expectation` 的比特数不匹配、线路比特数与状态不匹配。 |
| `OverflowError` | `num_qubits` 为负数。 |
