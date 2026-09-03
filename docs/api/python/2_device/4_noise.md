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

- `bit_flip(p) -> SingleQubitNoise`
- `phase_flip(p) -> SingleQubitNoise`
- `pauli(px, py, pz) -> SingleQubitNoise`
- `depolarizing(p) -> SingleQubitNoise`
- `amplitude_damping(gamma) -> SingleQubitNoise`
- `phase_damping(lambda_) -> SingleQubitNoise`

### 方法与属性

- `is_valid() -> bool`
- `to_kraus() -> list[numpy.ndarray]`
- `kind -> str`

## TwoQubitNoise

### 静态构造方法

- `depolarizing(p) -> TwoQubitNoise`
- `independent(q0_noise, q1_noise) -> TwoQubitNoise`
- `correlated_pauli(op_q0, op_q1, p) -> TwoQubitNoise`

`correlated_pauli` 的 `op_q0/op_q1` 仅接受 `I/X/Y/Z`。

异常情况：

- `ValueError`：泡利字符非法。

### 方法与属性

- `is_valid() -> bool`
- `to_kraus() -> list[numpy.ndarray]`
- `kind -> str`

## ReadoutError

### `ReadoutError(p_0_given_1, p_1_given_0)`

属性：

- `p_0_given_1 -> float`
- `p_1_given_0 -> float`

方法：

- `is_valid() -> bool`

## OperationKey

### 静态构造方法

- `new_single(gate, q0) -> OperationKey`
- `new_double(gate, q0, q1) -> OperationKey`
- `new_triple(gate, q0, q1, q2) -> OperationKey`

异常情况：

- `ValueError`：多比特门的比特有重复（qubit collision）或参数非法。

### 属性

- `gate -> StandardGate`
- `qubits -> list[int]`

说明：

- 实现了 `__eq__` 与 `__hash__`，可作为字典键使用。

## NoiseModel

### `NoiseModel()`

### 写入方法

- `add_readout_error(qubit, error) -> None`
- `add_single_qubit_error(gate, qubit, noise) -> None`
- `add_two_qubit_error(gate, q0, q1, noise) -> None`

异常情况：

- `ValueError`：噪声参数非法、比特冲突等。

### 查询方法

- `get_readout_error(qubit) -> ReadoutError | None`
- `get_single_qubit_errors(key) -> list[SingleQubitNoise] | None`
- `get_two_qubit_errors(key) -> list[TwoQubitNoise] | None`

## 示例

```python
from cqlib.circuit import StandardGate
from cqlib.device import (
    NoiseModel,
    OperationKey,
    ReadoutError,
    SingleQubitNoise,
    TwoQubitNoise,
)

nm = NoiseModel()
nm.add_readout_error(0, ReadoutError(0.1, 0.2))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.01))
nm.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))

skey = OperationKey.new_single(StandardGate.X, 0)
tkey = OperationKey.new_double(StandardGate.CX, 0, 1)

print(nm.get_readout_error(0))
print(nm.get_single_qubit_errors(skey))
print(nm.get_two_qubit_errors(tkey))
```

