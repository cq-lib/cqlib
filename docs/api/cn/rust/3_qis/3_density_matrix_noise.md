# DensityMatrixNoise

`cqlib_core::qis::state`

本页覆盖带噪声模型的密度矩阵模拟器 `DensityMatrixNoise`：噪声模型的接入方式、与线路对接的入口、门操作、读出噪声接口，以及测量、采样与期望值计算。

## 导入

```rust
use cqlib_core::qis::DensityMatrixNoise;
use cqlib_core::device::{NoiseModel, ReadoutError, SingleQubitNoise, TwoQubitNoise};
```

---

## DensityMatrixNoise

在密度矩阵模拟器之上叠加噪声模型的模拟器。它在每个门之后按噪声模型查询同一门、同一比特上的噪声条目并施加相应信道；没有配置噪声模型或没有匹配条目时，只施加理想门。

```rust
pub struct DensityMatrixNoise {
    pub state: DensityMatrix,
    pub noise_model: Option<NoiseModel>,
}
```

字段：

- `state` (`DensityMatrix`)：底层的密度矩阵状态。
- `noise_model` (`Option<NoiseModel>`)：作用于门操作与读出的噪声模型；`None` 表示理想仿真。

实现 `Debug`、`Clone`。

方法：

### 构造与线路对接

- `fn new(num_qubits: usize, noise_model: Option<NoiseModel>) -> Self`：按比特数与噪声模型构造，初始态为 `|0...0><0...0|`。
- `fn from_circuit(circuit: &Circuit, noise_model: Option<NoiseModel>) -> Result<Self, QisError>`：执行线路并返回模拟器；线路会先被分解为基础门，噪声在每个门之后施加。
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`：把线路原地作用到当前模拟器上，噪声按已配置的模型施加。

### 门操作

- `fn apply_standard_gate_noise(&mut self, gate: StandardGate, qs: &[usize], params: &[f64]) -> Result<(), QisError>`：标准门的统一入口，先校验并施加理想门，再按噪声模型施加匹配的门噪声。
- `fn apply_x(&mut self, q: usize) -> Result<(), QisError>`：Pauli-X。
- `fn apply_y(&mut self, q: usize) -> Result<(), QisError>`：Pauli-Y。
- `fn apply_z(&mut self, q: usize) -> Result<(), QisError>`：Pauli-Z。
- `fn apply_h(&mut self, q: usize) -> Result<(), QisError>`：Hadamard 门。
- `fn apply_s(&mut self, q: usize) -> Result<(), QisError>`：S 门。
- `fn apply_sdg(&mut self, q: usize) -> Result<(), QisError>`：S† 门。
- `fn apply_t(&mut self, q: usize) -> Result<(), QisError>`：T 门。
- `fn apply_tdg(&mut self, q: usize) -> Result<(), QisError>`：T† 门。
- `fn apply_u(&mut self, q: usize, theta: f64, phi: f64, lam: f64) -> Result<(), QisError>`：通用单比特门。
- `fn apply_phase(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：相位门 `P(θ)`。
- `fn apply_rx(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：绕 X 轴旋转 `θ`。
- `fn apply_ry(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：绕 Y 轴旋转 `θ`。
- `fn apply_rz(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：绕 Z 轴旋转 `θ`。
- `fn apply_gphase(&mut self, theta: f64) -> Result<(), QisError>`：全局相位。
- `fn apply_x2p(&mut self, q: usize) -> Result<(), QisError>`：`Rx(π/2)`。
- `fn apply_x2m(&mut self, q: usize) -> Result<(), QisError>`：`Rx(-π/2)`。
- `fn apply_y2p(&mut self, q: usize) -> Result<(), QisError>`：`Ry(π/2)`。
- `fn apply_y2m(&mut self, q: usize) -> Result<(), QisError>`：`Ry(-π/2)`。
- `fn apply_rxy(&mut self, q: usize, theta: f64, phi: f64) -> Result<(), QisError>`：绕 XY 平面内指定轴旋转。
- `fn apply_xy2p(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：`XY2P(θ)`。
- `fn apply_xy2m(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：`XY2M(θ)`。
- `fn apply_xy(&mut self, q: usize, theta: f64) -> Result<(), QisError>`：`XY(θ)`。
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 X 门。
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 Y 门。
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：受控 Z 门。
- `fn apply_ccx(&mut self, c1: usize, c2: usize, t: usize) -> Result<(), QisError>`：Toffoli 门。
- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：交换两个比特。
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rx(θ)`。
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Ry(θ)`。
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rz(θ)`。
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·X⊗X)`。
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Y⊗Y)`。
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗Z)`。
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗X)`。
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`：费米子模拟门。
- `fn apply_unitary_gate(&mut self, qs: &[usize], mat: &ndarray::Array2<Complex64>) -> Result<(), QisError>`：作用任意酉矩阵。该入口没有对应的标准门类型，因此不施加门噪声；需要噪声建模时应使用具体的门方法。

### 概率与读出噪声

模拟器把「量子态的噪声」与「读出的噪声」分开表达：前者作用于密度矩阵本身，后者只改变报告出来的概率分布。

- `fn probabilities(&self) -> Vec<f64>`：返回密度矩阵对角元，即各计算基态的测量概率。包含已经施加的门噪声，不含读出噪声。
- `fn probabilities_with_readout(&self, qubits: &[usize]) -> Result<Vec<f64>, QisError>`：在 `probabilities` 的基础上，对 `qubits` 中列出的比特施加读出噪声。
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`：返回 `Measurement` 选定的边缘概率分布，不含读出噪声。
- `fn probs_with_readout(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`：在 `probs` 的基础上，对 `measurement.qubits()` 施加读出噪声。

### 测量、采样与期望值

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`：在 Z 基下测量并使底层密度矩阵坍缩；测量操作不施加噪声。
- `fn measure_all(&mut self) -> Outcome`：按 `0..num_qubits` 的顺序逐个测量，返回位打包的 `Outcome`。
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`：把指定比特重置为 `|0>`；重置不施加读出噪声。
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`：并行采样若干次独立测量结果；采样的是量子态本身，不施加读出噪声。
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`：按线路 `Measurement` 指定的输出约定采样，同样不施加读出噪声。
- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`：计算 `Tr(ρ·O)`；结果包含门噪声，不含读出噪声。

### 噪声模型的配置

噪声模型由设备模块提供，配置方法包括：

- `NoiseModel::new()`：构造空模型。
- `add_single_qubit_error(StandardGate, Qubit, SingleQubitNoise) -> Result<(), NoiseError>`：为单比特门配置噪声。
- `add_two_qubit_error(StandardGate, Qubit, Qubit, TwoQubitNoise) -> Result<(), NoiseError>`：为双比特门配置噪声。
- `add_readout_error(Qubit, ReadoutError) -> Result<(), NoiseError>`：为指定比特配置读出噪声。

噪声的离散类型细节见设备模块文档。

---

## 示例

### 1. 带比特翻转噪声的单比特仿真

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, SingleQubitNoise};
use cqlib_core::qis::DensityMatrixNoise;

let mut noise_model = NoiseModel::new();
noise_model
    .add_single_qubit_error(StandardGate::X, Qubit::new(0), SingleQubitNoise::BitFlip(0.1))
    .unwrap();

let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
sim.apply_x(0).unwrap();

// 10% 比特翻转使 P(|1>) 为 0.9
let probs = sim.probabilities_with_readout(&[0]).unwrap();
assert!((probs[1] - 0.9).abs() < 1e-6);
assert!((probs[0] - 0.1).abs() < 1e-6);
```

### 2. 由线路构造模拟器

```rust
use cqlib_core::circuit::{Circuit, Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, SingleQubitNoise};
use cqlib_core::qis::DensityMatrixNoise;

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0)).unwrap();

let mut noise_model = NoiseModel::new();
noise_model
    .add_single_qubit_error(StandardGate::X, Qubit::new(0), SingleQubitNoise::BitFlip(0.1))
    .unwrap();

let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
assert_eq!(sim.state.num_qubits, 1);
assert!((sim.probabilities_with_readout(&[0]).unwrap()[1] - 0.9).abs() < 1e-6);
```

### 3. 读出噪声

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{NoiseModel, ReadoutError};
use cqlib_core::qis::DensityMatrixNoise;

let mut noise_model = NoiseModel::new();
noise_model
    .add_readout_error(
        Qubit::new(0),
        ReadoutError {
            p_0_given_1: 0.0,
            p_1_given_0: 0.1,
        },
    )
    .unwrap();

let sim = DensityMatrixNoise::new(1, Some(noise_model));
let probs = sim.probabilities_with_readout(&[0]).unwrap();

// 真实态仍为 |0>，读出误差使 P(|1>) 为 0.1
assert!((probs[1] - 0.1).abs() < 1e-6);
assert!((probs[0] - 0.9).abs() < 1e-6);
```

### 4. 区分理想分布与读出后的分布

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::device::{NoiseModel, Outcome, ReadoutError};
use cqlib_core::qis::DensityMatrixNoise;

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0)).unwrap();
let out = circuit.measure(Qubit::new(0)).unwrap();

let mut noise_model = NoiseModel::new();
noise_model
    .add_readout_error(
        Qubit::new(0),
        ReadoutError {
            p_0_given_1: 1.0,
            p_1_given_0: 0.0,
        },
    )
    .unwrap();

let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
let ideal = sim.probs(&out).unwrap();
let with_readout = sim.probs_with_readout(&out).unwrap();

assert!((ideal[&Outcome::from_bitstring("1").unwrap()] - 1.0).abs() < 1e-10);
assert!((with_readout[&Outcome::from_bitstring("0").unwrap()] - 1.0).abs() < 1e-10);
```

### 5. 采样不施加读出噪声

```rust
use cqlib_core::qis::DensityMatrixNoise;

let mut sim = DensityMatrixNoise::new(2, None);
sim.apply_h(0).unwrap();
sim.apply_cx(0, 1).unwrap();

let shots = sim.sample_shots(200);
assert_eq!(shots.len(), 200);
for outcome in &shots {
    assert_eq!(outcome.is_one(0), outcome.is_one(1));
}
```

---

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::InvalidParameterValue` | `apply_standard_gate_noise` 的比特数或参数个数与门不符；噪声操作本身失败。 |
| `QisError::InvalidStateDimension` | `apply_circuit` 的线路比特数与模拟器不一致。 |
| `QisError::UnsupportedOperation` | 线路包含当前入口不支持的操作，例如经典控制流。 |
| `QisError::CircuitError` | 线路中的门没有矩阵表示、含未解析的符号参数，或引用了不存在的比特。 |
| `QisError::IndexOutOfBounds` | 比特下标超出范围。 |
| `QisError::QubitMismatch` | `expectation` 的观测量比特数不匹配。 |
| `cqlib_core::device::NoiseError` | 噪声模型的配置方法失败，例如概率取值非法或门与比特的元数不匹配。 |

读出噪声只在 `probabilities_with_readout` 与 `probs_with_readout` 中生效；`probabilities`、`probs`、`sample_shots` 与 `sample` 报告的都是施加读出噪声之前的分布。
