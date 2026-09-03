# NoiseModel / Noise Channels

本页覆盖 `cqlib_core::device` 中噪声建模 API：

- `SingleQubitNoise`
- `TwoQubitNoise`
- `ReadoutError`
- `OperationKey`
- `NoiseModel`
- `NoiseError`

## 导入

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::{
    NoiseError, NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise,
};
```

## NoiseError

常见错误变体：

- `InvalidProbability { value, context }`
- `QubitCollision { qubits }`
- `InconsistentArity { expected, actual }`
- `Internal(String)`

## SingleQubitNoise

枚举变体：

- `BitFlip(f64)`
- `PhaseFlip(f64)`
- `Pauli { px, py, pz }`
- `Depolarizing(f64)`
- `AmplitudeDamping(f64)`
- `PhaseDamping(f64)`

方法：

- `is_valid(&self) -> bool`
- `to_kraus(&self) -> Vec<Array2<Complex64>>`

## TwoQubitNoise

枚举变体：

- `Depolarizing(f64)`
- `Independent { q0_noise: SingleQubitNoise, q1_noise: SingleQubitNoise }`
- `CorrelatedPauli { op_q0: Pauli, op_q1: Pauli, p: f64 }`

方法：

- `is_valid(&self) -> bool`
- `to_kraus(&self) -> Vec<Array2<Complex64>>`

## ReadoutError

结构体字段：

- `p_0_given_1: f64`
- `p_1_given_0: f64`

方法：

- `is_valid(&self) -> bool`

## OperationKey

构造：

- `OperationKey::new_single(gate, q0) -> OperationKey`
- `OperationKey::new_double(gate, q0, q1) -> Result<OperationKey, NoiseError>`
- `OperationKey::new_triple(gate, q0, q1, q2) -> Result<OperationKey, NoiseError>`

查询：

- `qubits(&self) -> &[usize]`
- `gate(&self) -> &StandardGate`

说明：

- `OperationKey` 实现了 `Hash`/`Eq`，可用作 `HashMap` 键。

## NoiseModel

构造：

- `NoiseModel::new() -> NoiseModel`

写入：

- `add_readout_error(&mut self, qubit, error) -> Result<(), String>`
- `add_single_qubit_error(&mut self, gate, qubit, noise) -> Result<(), NoiseError>`
- `add_two_qubit_error(&mut self, gate, q0, q1, noise) -> Result<(), NoiseError>`

查询：

- `get_readout_error(&self, key: &Qubit) -> Option<&ReadoutError>`
- `get_single_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<SingleQubitNoise>>`
- `get_two_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<TwoQubitNoise>>`

## 示例

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise};

let mut nm = NoiseModel::new();

nm.add_readout_error(
    Qubit::new(0),
    ReadoutError {
        p_0_given_1: 0.02,
        p_1_given_0: 0.01,
    },
)
.unwrap();

nm.add_single_qubit_error(
    StandardGate::X,
    Qubit::new(0),
    SingleQubitNoise::BitFlip(0.005),
)
.unwrap();

nm.add_two_qubit_error(
    StandardGate::CX,
    Qubit::new(0),
    Qubit::new(1),
    TwoQubitNoise::Depolarizing(0.02),
)
.unwrap();

let skey = OperationKey::new_single(StandardGate::X, Qubit::new(0));
let tkey = OperationKey::new_double(StandardGate::CX, Qubit::new(0), Qubit::new(1)).unwrap();

assert!(nm.get_single_qubit_errors(&skey).is_some());
assert!(nm.get_two_qubit_errors(&tkey).is_some());
```

