# NoiseModel / Noise Channels

本页覆盖 `cqlib.device` 中噪声建模相关 API：

- `SingleQubitNoise`
- `TwoQubitNoise`
- `ReadoutError`
- `OperationKey`
- `NoiseModel`

## 导入

```python
from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)
```

---

## SingleQubitNoise

### 静态构造方法

- `bit_flip(p) -> SingleQubitNoise`：比特翻转信道，以概率 `p` 作用 X。
- `phase_flip(p) -> SingleQubitNoise`：相位翻转信道，以概率 `p` 作用 Z。
- `pauli(px, py, pz) -> SingleQubitNoise`：一般 Pauli 信道，三个概率需满足 `px + py + pz <= 1`。
- `depolarizing(p) -> SingleQubitNoise`：去极化信道，以概率 `p` 作用随机 Pauli 错误，三种 Pauli 各占 `p/3`。
- `amplitude_damping(gamma) -> SingleQubitNoise`：振幅阻尼信道。
- `phase_damping(lambda_) -> SingleQubitNoise`：相位阻尼信道。

异常情况：

- `ValueError`：概率不在 `[0, 1]` 内或不是有限值；`pauli` 的三个概率之和超过 1。

### 方法与属性

- `is_valid() -> bool`：噪声参数是否物理有效。
- `to_kraus() -> list[numpy.ndarray]`：Kraus 算符列表，每个为 2×2 复数矩阵。

说明：

- 信道种类由静态构造方法决定，没有 `kind` 属性。

### 其他行为

- 支持 `==`：构造方法与参数相同的两个信道相等。
- `repr(SingleQubitNoise.bit_flip(0.01)) == "SingleQubitNoise.bit_flip(0.01)"`。

## TwoQubitNoise

### 静态构造方法

- `depolarizing(p) -> TwoQubitNoise`：双比特去极化信道。
- `independent(q0_noise, q1_noise) -> TwoQubitNoise`：两个单比特信道的独立组合。
- `correlated_pauli(op_q0, op_q1, p) -> TwoQubitNoise`：关联 Pauli 信道，`op_q0` / `op_q1` 为 `Pauli` 对象，例如 `Pauli.x()`、`Pauli.z()`。

异常情况：

- `ValueError`：`p` 不在 `[0, 1]` 内或不是有限值。
- `TypeError`：`op_q0` / `op_q1` 不是 `Pauli` 对象。

### 方法与属性

- `is_valid() -> bool`
- `to_kraus() -> list[numpy.ndarray]`：Kraus 算符列表，每个为 4×4 复数矩阵。
- `kind -> str`：`depolarizing` / `independent` / `correlated_pauli`。

### 其他行为

- 支持 `==`。

## ReadoutError

### `ReadoutError(p_0_given_1, p_1_given_0)`

参数：

- `p_0_given_1` (`float`)：真实态为 1 却读成 0 的概率。
- `p_1_given_0` (`float`)：真实态为 0 却读成 1 的概率。

异常情况：

- `ValueError`：概率不在 `[0, 1]` 内或不是有限值。

### 属性

- `p_0_given_1 -> float`
- `p_1_given_0 -> float`

### 方法

- `is_valid() -> bool`

### 其他行为

- 支持 `==`。
- `repr(ReadoutError(0.1, 0.2)) == "ReadoutError(p_0_given_1=0.1, p_1_given_0=0.2)"`。

## OperationKey

### 静态构造方法

- `new_single(gate, q0) -> OperationKey`
- `new_double(gate, q0, q1) -> OperationKey`
- `new_triple(gate, q0, q1, q2) -> OperationKey`

异常情况：

- `ValueError`：多比特门的比特有重复。

### 属性

- `gate -> StandardGate`：门类型。键只记录门类型，参数化门返回的对象参数一律为 0。
- `qubits -> list[int]`：参与操作的比特下标。

说明：

- 实现了 `__eq__` 与 `__hash__`，可作为字典键使用；门或比特组合不同则键不相等。
- `repr(OperationKey.new_single(StandardGate.X, 0)) == "OperationKey(gate=X, qubits=[0])"`。

## NoiseModel

### `NoiseModel()`

### 写入方法

- `add_readout_error(qubit, error) -> None`：为某个比特登记读出误差。
- `add_single_qubit_error(gate, qubit, noise) -> None`：为某个单比特门在指定比特上登记噪声信道。
- `add_two_qubit_error(gate, q0, q1, noise) -> None`：为某个双比特门在指定比特对上登记噪声信道。

异常情况：

- `ValueError`：噪声参数非法，或双比特门的两个比特相同。

### 查询方法

- `get_readout_error(qubit) -> ReadoutError | None`
- `get_single_qubit_errors(key) -> list[SingleQubitNoise] | None`
- `get_two_qubit_errors(key) -> list[TwoQubitNoise] | None`

说明：

- 同一个门与比特组合可以登记多条信道，查询返回列表；未登记的组合返回 `None`。

## 示例

```python
import pytest
from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)
from cqlib.qis import Pauli

nm = NoiseModel()
nm.add_readout_error(0, ReadoutError(0.1, 0.2))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.01))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.phase_flip(0.02))
nm.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))
nm.add_two_qubit_error(
    StandardGate.CX, 0, 1, TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 0.05)
)

skey = OperationKey.new_single(StandardGate.X, 0)
tkey = OperationKey.new_double(StandardGate.CX, 0, 1)

assert len(nm.get_single_qubit_errors(skey)) == 2
assert len(nm.get_two_qubit_errors(tkey)) == 2
assert nm.get_readout_error(0) == ReadoutError(0.1, 0.2)
assert nm.get_readout_error(9) is None

assert TwoQubitNoise.depolarizing(0.02).kind == "depolarizing"
assert TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 0.05).kind == "correlated_pauli"
assert SingleQubitNoise.depolarizing(0.1).is_valid() is True

with pytest.raises(ValueError):
    SingleQubitNoise.pauli(0.5, 0.5, 0.5)

with pytest.raises(ValueError):
    TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.z(), 1.5)

with pytest.raises(TypeError):
    TwoQubitNoise.correlated_pauli("X", Pauli.z(), 0.05)
```
