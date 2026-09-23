# StabilizerState

`cqlib_core::qis::state`

本页覆盖稳定子态模拟器 `StabilizerState` 与线路执行结果 `CircuitExecutionResult`：构造方式、Clifford 门操作、生成元读取、Pauli 期望值、测量与采样接口。

## 导入

```rust
use cqlib_core::qis::StabilizerState;
use cqlib_core::qis::state::stabilizer::CircuitExecutionResult;
```

---

## StabilizerState

稳定子态模拟器，用一组相互对易的 Pauli 算子（稳定子生成元）描述量子态。它只能表示稳定子态，因而只接受把 Pauli 群映射到自身的 Clifford 门；遇到任意角旋转、`T` 门或 `fSim` 等非 Clifford 操作时返回错误。可以描述的比特规模远大于稠密表示。

```rust
pub struct StabilizerState {
    pub num_qubits: usize,
}
```

字段：

- `num_qubits` (`usize`)：量子比特数。

生成元表存放在私有字段中。实现 `Debug` 与 `Clone`；没有实现 `PartialEq`，需要比较两个态时比较 `get_stabilizers()` 的返回值。

方法：

### 构造

- `fn new(n: usize) -> Self`：构造 `|0...0>` 态；初始生成元为第 `i` 行的 destabilizer `X_i` 与第 `n+i` 行的稳定子 `Z_i`，相位均为 `+1`。
- `fn num_qubits(&self) -> usize`：返回量子比特数，与同名字段取值一致。

### 与线路对接

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`：执行 Clifford 线路并返回最终的稳定子态。
- `fn apply_circuit(&mut self, input_circuit: &Circuit) -> Result<(), QisError>`：把 Clifford 线路原地作用到当前态上，线路的比特数必须与态一致。
- `fn run_circuit(circuit: &Circuit) -> Result<CircuitExecutionResult, QisError>`：执行 Clifford 线路，同时返回最终态与运行期经典数据。

`from_circuit` 与 `apply_circuit` 属于态层面的入口，它们把 `Circuit::measure*` 产生的测量与存储操作当作输出声明忽略，不使态坍缩，也不写入经典数据；需要执行语义（测量立即坍缩并写入经典数据）时应使用 `run_circuit`。

支持的指令为 Clifford 门 `I`、`H`、`X`、`Y`、`Z`、`S`、`SDG`、`X2P`、`X2M`、`Y2P`、`Y2M`、`CX`、`CY`、`CZ`、`SWAP`，以及 `Reset` 指令；屏障与延迟无副作用。`run_circuit` 额外执行 `MeasureBit`、`MeasureBits` 与 `Store`，但不支持控制流门。

### Clifford 门

- `fn apply_h(&mut self, qubit: usize) -> Result<(), QisError>`：Hadamard 门。
- `fn apply_x(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-X。
- `fn apply_y(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-Y。
- `fn apply_z(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-Z。
- `fn apply_s(&mut self, qubit: usize) -> Result<(), QisError>`：S 门。
- `fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError>`：S† 门。
- `fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError>`：`√X`，即 `Rx(π/2)`。
- `fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError>`：`√X†`，即 `Rx(-π/2)`。
- `fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError>`：`√Y`，即 `Ry(π/2)`。
- `fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError>`：`√Y†`，即 `Ry(-π/2)`。
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 X 门。
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 Y 门。
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：受控 Z 门。
- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：交换两个比特。
- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`：按标准门枚举分派；`gate` 不属于 Clifford 集合时报错。

### 生成元与 Pauli 期望值

- `fn get_stabilizers(&self) -> Vec<PauliString>`：返回 `n` 个稳定子生成元。
- `fn get_destabilizers(&self) -> Vec<PauliString>`：返回 `n` 个 destabilizer 生成元。
- `fn pauli_expectation(&self, pauli: &PauliString) -> Result<i32, QisError>`：返回 `⟨ψ|P|ψ⟩`，取值 `1` 表示 `P` 属于稳定子群、`-1` 表示属于取负的稳定子群、`0` 表示两者皆非。该判定覆盖生成元之积，不只针对单个生成元。
- `fn to_stim_format(&self) -> String`：以兼容格式导出生成元表。

### 概率、测量与采样

- `fn probability_of(&self, bits: &[bool]) -> Result<f64, QisError>`：返回指定计算基态的概率；与稳定子态相容的位串返回 `Ok(1.0 / 2.0^k)`，其中 `k` 是测量结果随机的比特数；不相容时返回 `Ok(0.0)`。该调用不修改自身。
- `fn probabilities(&self) -> Result<Vec<f64>, QisError>`：返回全部计算基态上的概率分布，长度为 `2^n`，下标 `i` 对应二进制表示为 `i` 的基态，比特 0 为最低位。该接口只适用于小规模系统，`n > 20` 时返回错误。
- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`：在 Z 基下测量并使态坍缩，返回 `false` 表示结果 `0`、`true` 表示结果 `1`。
- `fn measure_all(&mut self) -> Outcome`：按顺序测量所有比特，返回位打包的 `Outcome`。
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`：把指定比特重置为 `|0>`，实现为一次 Z 基测量并在结果为 `|1>` 时补一个 X 修正。
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`：并行采样若干次独立测量结果；同一初始态总是产生相同的样本集合。
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`：按线路 `Measurement` 指定的输出约定采样，不读取也不执行线路本身；`measurement.qubits()[i]` 对应每个 `Outcome` 的第 `i` 位。
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`：返回 `Measurement` 选定的边缘概率分布；其比特顺序与 `sample` 一致。该接口复用 `probabilities`，因此同样受 `n <= 20` 限制。

---

## CircuitExecutionResult

`run_circuit` 的返回对象，同时包含最终量子态与执行期间产生的运行期经典数据。

```rust
pub struct CircuitExecutionResult {
    pub state: StabilizerState,
    pub classical: ClassicalState,
}
```

字段：

- `state`：全部操作执行完毕后的稳定子态。
- `classical`：测量结果与可变变量，按线路内的句柄索引。

实现 `Debug`。运行期经典值的具体形态见 [ClassicalState](5_classical_state.md)。

---

## 示例

### 1. Bell 态的稳定子采样

```rust
use cqlib_core::qis::StabilizerState;

let mut s = StabilizerState::new(2);
s.apply_h(0).unwrap();
s.apply_cx(0, 1).unwrap();

let shots = s.sample_shots(500);
assert_eq!(shots.len(), 500);
for shot in &shots {
    assert_eq!(shot.is_one(0), shot.is_one(1));
}
```

### 2. 读取稳定子生成元

```rust
use cqlib_core::qis::StabilizerState;

let s = StabilizerState::new(2);
let stabs = s.get_stabilizers();
assert_eq!(stabs.len(), 2);

// 初态 |00⟩ 的稳定子为 +ZI 与 +IZ
let stabs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
assert_eq!(stabs[0], "+IZ");
assert_eq!(stabs[1], "+ZI");
```

### 3. 计算指定基态的概率

```rust
use cqlib_core::qis::StabilizerState;

let mut s = StabilizerState::new(2);
s.apply_h(0).unwrap();
s.apply_cx(0, 1).unwrap();

// 只有 |00⟩ 与 |11⟩ 可能出现，各占一半
assert!((s.probability_of(&[false, false]).unwrap() - 0.5).abs() < 1e-12);
assert!((s.probability_of(&[true, true]).unwrap() - 0.5).abs() < 1e-12);
assert_eq!(s.probability_of(&[false, true]).unwrap(), 0.0);
assert_eq!(s.probability_of(&[true, false]).unwrap(), 0.0);
```

### 4. Pauli 期望值

```rust
use cqlib_core::qis::{Pauli, PauliString, Phase, StabilizerState};

// |+⟩ 是 +X 的本征态
let mut s = StabilizerState::new(1);
s.apply_h(0).unwrap();

let mut x = PauliString::new(1);
x.set_pauli(0, Pauli::X);
assert_eq!(s.pauli_expectation(&x).unwrap(), 1);

// Bell 态下 ⟨ZI⟩ 为 0
let mut bell = StabilizerState::new(2);
bell.apply_h(0).unwrap();
bell.apply_cx(0, 1).unwrap();

let mut zi = PauliString::new(2);
zi.set_pauli(0, Pauli::Z);
zi.set_pauli(1, Pauli::I);
zi.phase = Phase::Plus;
assert_eq!(bell.pauli_expectation(&zi).unwrap(), 0);
```

### 5. 由线路构造并获取边缘分布

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::StabilizerState;

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let out = c.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

// 该测量声明不使 Bell 态坍缩
let stab = StabilizerState::from_circuit(&c).unwrap();
let probs = stab.probs(&out).unwrap();
assert!((probs.values().sum::<f64>() - 1.0).abs() < 1e-10);
```

### 6. 执行线路并读取运行期经典数据

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(1);
c.x(Qubit::new(0)).unwrap();
let measured = c.measure(Qubit::new(0)).unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
assert_eq!(
    result.classical.value(measured.value()),
    Some(&RuntimeValue::Bit(true))
);
```

### 7. 非 Clifford 门被拒绝

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::StabilizerState;

let mut c = Circuit::new(1);
c.t(Qubit::new(0)).unwrap();

let result = StabilizerState::from_circuit(&c);
assert!(result.is_err());
```

---

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::NonCliffordGate` | 向稳定子态作用非 Clifford 门，例如 `T`、`T†`、任意角度的 `Rx`、`Ry`、`Rz`、`Phase`、通用酉门。 |
| `QisError::QubitMismatch` | `probability_of` 的位串长度与比特数不符；`pauli_expectation` 的 Pauli 串比特数与态不一致。 |
| `QisError::IndexOutOfBounds` | 比特下标超出范围。 |
| `QisError::InvalidParameterValue` | 在零比特系统上作用门；双比特门给出了相同比特；`probabilities` 或 `probs` 在比特数超过上限时被调用。 |
| `QisError::InvalidStateDimension` | `apply_circuit` 的线路比特数与态不一致。 |
| `QisError::UnsupportedOperation` | 线路包含控制流等当前入口不支持的操作。 |
| `QisError::CircuitError` | 线路中的门无法执行或含未解析的符号参数。 |

对同一初始态重复采样会得到相同的样本集合，因此采样结果可用于复现对比；需要不同的随机结果时应重新构造线路或改变门序列。
