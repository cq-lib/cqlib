# DensityMatrix

`cqlib_core::qis::state`

本页覆盖混合态模拟器 `DensityMatrix`：构造方式、矩阵数据访问、物理性校验、与线路对接的入口、各类门操作、量子信道与偏迹、测量与采样接口，以及期望值计算。

## 导入

```rust
use cqlib_core::qis::DensityMatrix;
```

---

## DensityMatrix

混合态模拟器，把量子态表示为 `2^N × 2^N` 的密度矩阵。相对只用一组复振幅描述的纯态，密度矩阵可以表示纯态、混合态以及量子信道作用后的结果。

```rust
pub struct DensityMatrix {
    pub num_qubits: usize,
}
```

字段：

- `num_qubits` (`usize`)：量子比特数。

矩阵元素按行主序存放于私有缓冲区，长度为 `4^N`，通过 `data()` 访问。实现 `Debug`、`Clone`，以及 `AddAssign`（`+=`）。

方法：

### 构造与数据访问

- `fn new(num_qubits: usize) -> Self`：构造纯态 `|0...0><0...0|`。
- `fn maximally_mixed(num_qubits: usize) -> Self`：构造最大混合态 `I / 2^N`，各计算基态等概率且无相干项。
- `fn zeros(num_qubits: usize) -> Self`：构造全零矩阵；它不是合法物理态（迹为 0），用于作为量子信道等操作的累加起点。
- `fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError>`：由纯态振幅做外积 `ρ = |ψ><ψ|` 构造。
- `fn from_density_matrix_state(num_qubits: usize, dm_state: Vec<Complex64>) -> Result<Self, QisError>`：直接由展平的 `2^N × 2^N` 矩阵构造，并校验 Hermitian 性、半正定性与单位迹。
- `fn data(&self) -> &[Complex64]`：按共享切片读取展平的矩阵元素。
- `fn probabilities(&self) -> Vec<f64>`：返回密度矩阵对角元，即各计算基态上的测量概率。

### 物理性校验

- `fn trace(&self) -> Complex64`：返回迹；合法物理态的迹等于 `1.0`。
- `fn is_hermitian(&self, tol: f64) -> bool`：判断是否满足 `ρ = ρ†`。
- `fn is_positive_semidefinite_approx(&self, tol: f64) -> bool`：判断所有本征值是否不小于 `-tol`；含 NaN 或 Inf 时返回 `false` 而不恐慌。
- `fn validate_physical(&self, tol: f64) -> Result<(), QisError>`：依次校验 Hermitian 性、半正定性与单位迹。

### 与线路对接

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`：执行线路并返回演化后的密度矩阵。
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`：把线路原地作用到当前密度矩阵上，线路的比特数必须与态一致。

两条入口支持的指令与 `Statevector` 相同：标准门、受控门、多控制门、带矩阵表示的酉门与 `Reset`，屏障和延迟被忽略，`Circuit::measure*` 产生的测量声明不参与状态演化。

### 通用门入口

- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`：按标准门枚举分派；`qubits` 与 `params` 的个数必须分别等于 `gate` 要求的值。
- `fn apply_single_qubit_gate(&mut self, qubit: usize, matrix: [[Complex64; 2]; 2]) -> Result<(), QisError>`：作用任意 2×2 酉矩阵。
- `fn apply_two_qubit_gate(&mut self, q0: usize, q1: usize, matrix: [[Complex64; 4]; 4]) -> Result<(), QisError>`：作用任意 4×4 酉矩阵。
- `fn apply_unitary_gate(&mut self, qubits: &[usize], matrix: &ndarray::Array2<Complex64>) -> Result<(), QisError>`：按 `ρ → U ρ U†` 作用任意 `2^n × 2^n` 酉矩阵。

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
- `fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`：`XY(θ)`。
- `fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError>`：绕 XY 平面内指定轴旋转。
- `fn apply_gphase(&mut self, phi: f64)`：全局相位对密度矩阵没有可观测影响，该方法不改变状态，也不返回结果。

### 双比特门与三比特门

- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：交换两个比特。
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 X 门。
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`：受控 Y 门。
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`：受控 Z 门。
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rx(θ)`。
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Ry(θ)`。
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`：受控 `Rz(θ)`。
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·X⊗X)`。
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Y⊗Y)`。
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗Z)`。
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`：`exp(-i·θ/2·Z⊗X)`。
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`：费米子模拟门。
- `fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError>`：Toffoli 门。

### 量子信道与约化

- `fn apply_kraus(&mut self, ops: &[Vec<Complex64>], qs: &[usize]) -> Result<(), QisError>`：作用由 Kraus 算符给出的量子信道，演化为 `ρ → Σ_k K_k ρ K_k†`；每个算符按展平向量给出，比特数由 `qs.len()` 决定。
- `fn partial_trace(&self, keep: &[usize]) -> Result<Self, QisError>`：对 `keep` 之外的比特求偏迹，返回比特数为 `keep.len()` 的新密度矩阵。

### 测量、重置与采样

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`：在 Z 基下测量并使密度矩阵坍缩，返回 `true` 表示结果 `|1>`、`false` 表示结果 `|0>`；坍缩后按下式重新归一化：`ρ' = Π_b ρ Π_b / Tr(Π_b ρ)`。
- `fn measure_all(&mut self) -> Outcome`：按 `0..num_qubits` 的顺序逐个测量，返回位打包的 `Outcome`。
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`：把指定比特重置为 `|0>`。
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`：并行采样若干次独立测量结果，不修改自身。
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`：按线路 `Measurement` 指定的输出约定采样。
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`：返回 `Measurement` 选定的边缘概率分布。

### 期望值

- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`：计算 `Tr(ρ·H)`。

---

## 示例

### 1. 制备并读取概率

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(1);
dm.apply_h(0).unwrap();

let probs = dm.probabilities();
assert_eq!(probs.len(), 2);
assert!((probs[0] - 0.5).abs() < 1e-10);
assert!((probs[1] - 0.5).abs() < 1e-10);
```

### 2. 最大混合态与迹

```rust
use cqlib_core::qis::DensityMatrix;

let dm = DensityMatrix::maximally_mixed(2);
assert_eq!(dm.num_qubits, 2);
assert_eq!(dm.data().len(), 16);
assert!((dm.trace().re - 1.0).abs() < 1e-10);
```

### 3. 由纯态与由矩阵构造

```rust
use cqlib_core::qis::DensityMatrix;
use num_complex::Complex64;

// 由纯态振幅构造
let state = vec![Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)];
let dm = DensityMatrix::from_state(1, state).unwrap();
assert!((dm.data()[0].re - 0.0).abs() < 1e-10);
assert!((dm.data()[3].re - 1.0).abs() < 1e-10);

// 由展平的密度矩阵构造
let mut flat = vec![Complex64::new(0.0, 0.0); 4];
flat[0] = Complex64::new(0.5, 0.0);
flat[3] = Complex64::new(0.5, 0.0);
let mixed = DensityMatrix::from_density_matrix_state(1, flat).unwrap();
assert!((mixed.probabilities()[0] - 0.5).abs() < 1e-10);
assert!((mixed.probabilities()[1] - 0.5).abs() < 1e-10);
```

### 4. 校验物理性

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(1);
dm.apply_h(0).unwrap();

assert!(dm.is_hermitian(1e-10));
assert!(dm.is_positive_semidefinite_approx(1e-10));
assert!(dm.validate_physical(1e-10).is_ok());
```

### 5. 作用量子信道

```rust
use cqlib_core::qis::DensityMatrix;
use num_complex::Complex64;

let p: f64 = 0.3;
let k0 = vec![
    Complex64::new((1.0 - p).sqrt(), 0.0),
    Complex64::new(0.0, 0.0),
    Complex64::new(0.0, 0.0),
    Complex64::new((1.0 - p).sqrt(), 0.0),
];
let k1 = vec![
    Complex64::new(0.0, 0.0),
    Complex64::new(p.sqrt(), 0.0),
    Complex64::new(p.sqrt(), 0.0),
    Complex64::new(0.0, 0.0),
];

let mut dm = DensityMatrix::new(1);
dm.apply_kraus(&[k0, k1], &[0]).unwrap();

assert!((dm.trace().re - 1.0).abs() < 1e-10);
```

### 6. 求偏迹

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

// 追踪掉比特 1，比特 0 变成最大混合态
let reduced = dm.partial_trace(&[0]).unwrap();
assert_eq!(reduced.num_qubits, 1);
assert!((reduced.probabilities()[0] - 0.5).abs() < 1e-10);
assert!((reduced.probabilities()[1] - 0.5).abs() < 1e-10);
```

### 7. 累加密度矩阵

```rust
use cqlib_core::qis::DensityMatrix;

// 以全零矩阵作为累加起点
let mut acc = DensityMatrix::zeros(1);
acc += DensityMatrix::new(1);

assert!((acc.trace().re - 1.0).abs() < 1e-10);
assert!((acc.probabilities()[0] - 1.0).abs() < 1e-10);
```

### 8. 期望值

```rust
use cqlib_core::qis::{DensityMatrix, Hamiltonian, Pauli, PauliString};

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

let mut ps = PauliString::new(2);
ps.set_pauli(0, Pauli::Z);
ps.set_pauli(1, Pauli::Z);
let h = Hamiltonian::from_pauli(ps);

let exp = dm.expectation(&h).unwrap();
assert!((exp - 1.0).abs() < 1e-10);
```

---

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::InvalidStateDimension` | `from_state` 的振幅数量不等于 `2^num_qubits`；`from_density_matrix_state` 的矩阵长度不等于 `4^num_qubits`；`apply_circuit` 的线路比特数与态不一致。 |
| `QisError::NotNormalized` | `from_state`、`from_density_matrix_state` 或 `validate_physical` 检出的迹不为 1。 |
| `QisError::NotHermitian` | `from_density_matrix_state` 或 `validate_physical` 检出的非自伴矩阵。 |
| `QisError::NotPositiveSemidefinite` | `from_density_matrix_state` 或 `validate_physical` 检出的负本征值。 |
| `QisError::IndexOutOfBounds` | 比特下标超出范围。 |
| `QisError::InvalidParameterValue` | 零比特系统上的门操作；双比特门给出相同比特；`apply_kraus` 的算符列表为空、比特列表为空或算符尺寸与比特数不符；`partial_trace` 的保留比特列表含重复项；`apply_standard_gate` 的比特数或参数个数与门不符。 |
| `QisError::UnsupportedOperation` | 线路包含当前入口不支持的操作。 |
| `QisError::CircuitError` | 线路中的门没有矩阵表示、含未解析的符号参数，或引用了不存在的比特；`expectation` 的观测量比特数不匹配。 |

`+=` 要求两个密度矩阵的比特数相同，比特数不一致时直接恐慌，调用前应自行确认。
