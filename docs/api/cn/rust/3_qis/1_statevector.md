# Statevector

`cqlib_core::qis::state`

本页覆盖纯态模拟器 `Statevector`：构造方式、振幅访问、与线路对接的入口、各类门操作、测量与采样接口，以及期望值计算。

## 导入

```rust
use cqlib_core::qis::Statevector;
```

---

## Statevector

纯态模拟器，把量子态表示为 `2^num_qubits` 个复振幅。

```rust
pub struct Statevector {
    pub num_qubits: usize,
}
```

字段：

- `num_qubits` (`usize`)：量子比特数。

振幅存放在私有的对齐缓冲区中，长度为 `2^num_qubits`；下标 `i` 的分量是计算基态 `|i>` 的系数，比特 0 对应最低位。振幅通过 `data()` 与 `data_mut()` 访问。实现 `Debug`、`Clone`。

方法：

### 构造与振幅访问

- `fn new(num_qubits: usize) -> Self`：构造 `|0...0>` 态，首分量置 1，其余为 0。
- `fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError>`：用给定振幅构造，要求长度等于 `2^num_qubits` 且已经归一化。
- `fn data(&self) -> &[Complex64]`：按共享切片读取全部振幅。
- `fn data_mut(&mut self) -> &mut [Complex64]`：按可变切片读取全部振幅；改写可能破坏归一化。
- `fn probabilities(&self) -> Vec<f64>`：返回全部计算基态上的测量概率分布，即各振幅模的平方。

### 与线路对接

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`：执行线路并返回演化后的态，输入线路不被修改。
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`：把线路原地作用到当前态上，线路的比特数必须与态一致。

两条入口都会先对线路做分解，再逐条执行其中的指令。支持标准门、受控门、多控制门、带矩阵表示的酉门与 `Reset`，屏障和延迟指令被忽略，`Circuit::measure*` 产生的测量声明不参与状态演化。

### 通用门入口

- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`：按标准门枚举分派到对应的专用实现；`qubits` 的个数与 `params` 的个数必须分别等于 `gate` 要求的值。
- `fn apply_single_qubit_gate(&mut self, qubit: usize, matrix: [[Complex64; 2]; 2]) -> Result<(), QisError>`：作用任意 2×2 酉矩阵。
- `fn apply_two_qubit_gate(&mut self, q0: usize, q1: usize, matrix: [[Complex64; 4]; 4]) -> Result<(), QisError>`：作用任意 4×4 酉矩阵，基序为 `|00>`、`|01>`、`|10>`、`|11>`，其中 `q0` 为高位。
- `fn apply_unitary_gate(&mut self, qubits: &[usize], matrix: &ndarray::Array2<Complex64>) -> Result<(), QisError>`：作用任意 `2^n × 2^n` 酉矩阵，`qubits` 给出该矩阵作用的比特。
- `fn apply_pauli_rotation(&mut self, pauli: &PauliString, theta: f64) -> Result<(), QisError>`：原地作用 `exp(-i·θ/2·P)`，其中 `P` 为 Hermitian 的 Pauli 串；该方法不把指数分解为基础门序列。

### 单比特门

- `fn apply_x(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-X。
- `fn apply_y(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-Y。
- `fn apply_z(&mut self, qubit: usize) -> Result<(), QisError>`：Pauli-Z。
- `fn apply_h(&mut self, qubit: usize) -> Result<(), QisError>`：Hadamard 门。
- `fn apply_s(&mut self, qubit: usize) -> Result<(), QisError>`：S 门。
- `fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError>`：S† 门。
- `fn apply_t(&mut self, qubit: usize) -> Result<(), QisError>`：T 门。
- `fn apply_tdg(&mut self, qubit: usize) -> Result<(), QisError>`：T† 门。
- `fn apply_u(&mut self, qubit: usize, theta: f64, phi: f64, lambda: f64) -> Result<(), QisError>`：通用单比特门 `U(θ, φ, λ)`。
- `fn apply_phase(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：相位门 `P(θ)`。
- `fn apply_rx(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：绕 X 轴旋转 `θ`。
- `fn apply_ry(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：绕 Y 轴旋转 `θ`。
- `fn apply_rz(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：绕 Z 轴旋转 `θ`。
- `fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError>`：`Rx(π/2)`。
- `fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError>`：`Rx(-π/2)`。
- `fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError>`：`Ry(π/2)`。
- `fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError>`：`Ry(-π/2)`。
- `fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：`XY2P(θ)`。
- `fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：`XY2M(θ)`。
- `fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：`XY(θ)`，与 `XY2P`、`XY2M` 是不同的门。
- `fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError>`：绕 XY 平面内方位角为 `phi` 的轴旋转 `theta`。
- `fn apply_gphase(&mut self, phi: f64) -> Result<(), QisError>`：给整个态乘上全局相位 `e^(iφ)`。

### 双比特门

- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：交换两个比特。
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 X 门。
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 Y 门。
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：受控 Z 门，两个参数地位对称。
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rx(θ)`。
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Ry(θ)`。
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rz(θ)`。
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·X⊗X)`。
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Y⊗Y)`。
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗Z)`。
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗X)`。
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`：费米子模拟门，`theta` 为 iSWAP 角，`phi` 为受控相位角。

### 三比特门

- `fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError>`：Toffoli 门，两个控制位同时为 `|1>` 时翻转目标位。

### 测量、重置与采样

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`：在 Z 基下测量指定比特并使态坍缩，返回 `false` 表示结果 `|0>`、`true` 表示结果 `|1>`；测量后态在对应子空间内重新归一化。该操作是破坏性的。
- `fn measure_with_rng(&mut self, qubit: usize, rng: &mut impl Rng) -> Result<bool, QisError>`：使用调用方提供的随机数发生器测量，便于复现。
- `fn measure_all(&mut self) -> Outcome`：按 `0..num_qubits` 的顺序逐个测量，返回位打包的 `Outcome`。
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`：把指定比特重置为 `|0>`，实现为一次 Z 基测量并在结果为 `|1>` 时补一个 X 修正。
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`：并行采样若干次独立测量结果，不修改自身。
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`：按线路 `Measurement` 指定的输出约定采样；`measurement.qubits()[i]` 对应每个 `Outcome` 的第 `i` 位。
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`：返回 `Measurement` 选定的边缘概率分布；该方法不执行线路，只把 `Measurement` 当作输出约定使用。

### 期望值

- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`：计算 `⟨ψ|H|ψ⟩`。

---

## 示例

### 1. 制备 Bell 态

```rust
use cqlib_core::qis::Statevector;
use num_complex::Complex64;
use std::f64::consts::FRAC_1_SQRT_2;

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

// (|00⟩ + |11⟩)/√2
assert_eq!(sv.num_qubits, 2);
assert!((sv.data()[0] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
assert!((sv.data()[3] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);

let probs = sv.probabilities();
assert!((probs[0] - 0.5).abs() < 1e-10);
assert!((probs[3] - 0.5).abs() < 1e-10);
```

### 2. 从线路构造与原地作用线路

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::Statevector;

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let sv = Statevector::from_circuit(&circuit).unwrap();
assert_eq!(sv.num_qubits, 2);

let mut sv2 = Statevector::new(2);
sv2.apply_circuit(&circuit).unwrap();
assert!(sv2.data().iter().zip(sv.data()).all(|(a, b)| (a - b).norm() < 1e-10));
```

### 3. 作用自定义酉门

```rust
use cqlib_core::qis::Statevector;
use num_complex::Complex64;
use std::f64::consts::FRAC_1_SQRT_2;

// H = [[1/√2, 1/√2], [1/√2, -1/√2]]
let h_matrix = [
    [Complex64::new(FRAC_1_SQRT_2, 0.0), Complex64::new(FRAC_1_SQRT_2, 0.0)],
    [Complex64::new(FRAC_1_SQRT_2, 0.0), Complex64::new(-FRAC_1_SQRT_2, 0.0)],
];

let mut sv = Statevector::new(1);
sv.apply_single_qubit_gate(0, h_matrix).unwrap();

assert!((sv.data()[0] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
assert!((sv.data()[1] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
```

### 4. 作用多比特酉门

```rust
use cqlib_core::qis::Statevector;
use ndarray::Array2;

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
let before: Vec<_> = sv.data().to_vec();

// 4×4 单位矩阵作用在比特 [0, 1] 上
let identity = Array2::eye(4);
sv.apply_unitary_gate(&[0, 1], &identity).unwrap();

assert!(sv.data().iter().zip(before.iter()).all(|(a, b)| (a - b).norm() < 1e-10));
```

### 5. Pauli 串旋转

```rust
use cqlib_core::qis::{Pauli, PauliString, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();

let mut pauli = PauliString::new(2);
pauli.set_pauli(0, Pauli::X);
pauli.set_pauli(1, Pauli::Z);

sv.apply_pauli_rotation(&pauli, 0.413).unwrap();

let norm: f64 = sv.data().iter().map(|amp| amp.norm_sqr()).sum();
assert!((norm - 1.0).abs() < 1e-10);
```

### 6. 测量与采样

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::Statevector;

let mut sv = Statevector::new(4);
sv.apply_x(0).unwrap();
sv.apply_x(2).unwrap();

let outcome = sv.measure_all();
assert!(outcome.is_one(0));
assert!(!outcome.is_one(1));
assert!(outcome.is_one(2));
assert!(!outcome.is_one(3));

// 按 Measurement 指定的比特顺序采样
let mut circuit = Circuit::new(2);
circuit.x(Qubit::new(0)).unwrap();
let out = circuit.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

let sv = Statevector::from_circuit(&circuit).unwrap();
let result = sv.sample(&out, 16).unwrap();
assert_eq!(result.shots(), 16);
```

### 7. 计算期望值

```rust
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString, Statevector};

let sv = Statevector::new(1);

let mut ps = PauliString::new(1);
ps.set_pauli(0, Pauli::Z);
let h = Hamiltonian::from_pauli(ps);

let exp = sv.expectation(&h).unwrap();
assert!((exp - 1.0).abs() < 1e-10);
```

---

## 校验与错误处理

构造、门作用、测量与期望值计算失败都通过 `QisError` 返回，不使用恐慌。

| 错误 | 触发场景 |
| --- | --- |
| `QisError::InvalidStateDimension` | `from_state` 的振幅数量不等于 `2^num_qubits`；`apply_circuit` 的线路比特数与态不一致。 |
| `QisError::NotNormalized` | `from_state` 传入的振幅未归一化。 |
| `QisError::IndexOutOfBounds` | 比特下标超出范围。 |
| `QisError::InvalidParameterValue` | 在零比特系统上作用门；双比特门给出了相同比特；`apply_unitary_gate` 的比特列表含重复项；`apply_standard_gate` 的比特数或参数个数与门不符。 |
| `QisError::UnsupportedOperation` | 线路包含当前入口不支持的操作，例如经典控制流。 |
| `QisError::CircuitError` | 线路中的门没有矩阵表示、含未解析的符号参数，或引用了不存在的比特。 |
| `QisError::QubitMismatch` | `expectation` 的观测量作用比特数与态不一致。 |

未归一化的振幅、零比特系统上的门操作与相同比特构成的双比特门都属于调用方的输入错误，通常在运行前即可避免。
