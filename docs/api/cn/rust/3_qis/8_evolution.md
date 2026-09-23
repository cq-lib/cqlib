# Pauli 演化与 Trotter 分解

`cqlib_core::qis::evolution`

本页介绍把 Pauli 串与 Hamilton 量转换为演化线路的接口：`PauliEvolution` trait 提供单条 Pauli 串的旋转门，`Hamiltonian` 上的两个入口把它扩展成整段时间演化线路，`TrotterMode` 决定多项非对易情形的近似方式。

## 导入

```rust
use cqlib_core::qis::evolution::{PauliEvolution, TrotterMode};
```

`PauliEvolution` 与 `TrotterMode` 也在 `cqlib_core::qis` 顶层重导出。

---

## TrotterMode

Hamilton 量时间演化 $U(t) = e^{-iHt}$ 的 Trotter-Suzuki 分解模式。

```rust
pub enum TrotterMode {
    FirstOrder,
    SecondOrder,
    Randomized(u64),
}
```

变体：

| 变体 | 载荷 | 近似形式 | 误差量级 |
| --- | --- | --- | --- |
| `TrotterMode::FirstOrder` | 无 | $U(t) \approx \left[\prod_k e^{-i c_k t/n \cdot P_k}\right]^n$ | $O(t^2/n)$ |
| `TrotterMode::SecondOrder` | 无 | 每步先按正序、再按逆序各作用半角演化 | $O(t^3/n^2)$ |
| `TrotterMode::Randomized(u64)` | 随机种子 | 与一阶相同，但每步内各项的顺序随机打乱 | 与一阶同阶 |

`Randomized` 携带的种子决定每一项在每一步中的排列顺序，同一个种子给出同一条线路，便于重复实验。

### 其他行为

- 支持 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`。

示例：

```rust
use cqlib_core::qis::evolution::TrotterMode;

// 同一种子的模式相等，可重复使用
assert_eq!(TrotterMode::Randomized(42), TrotterMode::Randomized(42));
assert_ne!(TrotterMode::Randomized(42), TrotterMode::Randomized(43));
assert_ne!(TrotterMode::FirstOrder, TrotterMode::SecondOrder);
```

---

## PauliEvolution

向线路追加 Pauli 旋转门的扩展 trait。

```rust
pub trait PauliEvolution {
    fn pauli_evolution(
        &mut self,
        pauli: &PauliString,
        angle: impl Into<ParameterValue>,
        qubits: &[Qubit],
    ) -> Result<(), CircuitError>;
}
```

### 方法

- `fn pauli_evolution(&mut self, pauli: &PauliString, angle: impl Into<ParameterValue>, qubits: &[Qubit]) -> Result<(), CircuitError>`：追加演化算符 $e^{-i\theta/2 \cdot P}$。

参数：

- `pauli` (`&PauliString`)：要指数化的 Pauli 串 $P$，相位必须为 $\pm 1$（Hermitian）。
- `angle` (`impl Into<ParameterValue>`)：旋转角 $\theta$，可以是固定浮点数，也可以是符号参数。
- `qubits` (`&[Qubit]`)：作用位置，长度必须与 `pauli.num_qubits` 一致。

注意这里的角度约定：$\theta$ 是旋转角的一半因子所对应的角，$e^{-i\theta/2 \cdot P}$ 中的 $\theta$ 取 $2ct$ 才等价于 $e^{-ictP}$。把 Hamilton 量交给演化入口时不需要手动换算，入口内部已按此约定处理。

### 实现者

| 类型 | 说明 |
| --- | --- |
| `Circuit` | 把旋转门直接追加到目标线路的末尾。 |

### 实现步骤

一、相位校验。Pauli 串的内部相位必须是 $\pm 1$，能为酉演化生成元的算符只有 Hermitian 算符，相位为 $\pm i$ 时直接报错。

二、相位吸收。把 $\pm 1$ 吸收进旋转角，得到有效角 $\theta_{\text{eff}} = \theta \cdot \text{phase}$。

三、基变换。对每个非恒等比特施加把分量转到 Z 基的门：`X` 用 `H`，`Y` 先用 $S^\dagger$ 再用 `H`，`Z` 不需要变换。

四、CNOT 链。按非恒等比特下标递增的顺序施加 `CNOT`，逐级累积奇偶性。

五、核心旋转。在最后一个非恒等比特上施加 `RZ(θ_eff)`。

六、逆序 CNOT 链与逆基变换，把比特恢复到原来的基。

当 Pauli 串全为恒等算子时，演化退化为全局相位 $e^{-i\theta/2}$，不产生任何门，相位记在线路的全局相位上。

### 错误

- `CircuitError::QubitCountMismatch`：`qubits` 的长度与 `pauli.num_qubits` 不一致。
- `CircuitError::InvalidOperation`：Pauli 串的相位为 $\pm i$，不能作为酉演化的生成元。

### 示例

单比特旋转：

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(1);
let qubits = circuit.qubits();

// Z 分量不需要基变换，只产生一个 RZ 门
let pauli: PauliString = "Z".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
    .unwrap();

assert_eq!(circuit.operations().len(), 1);
```

多比特旋转：

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

// ZZ 链：CNOT(0->1) - RZ(1) - CNOT(0->1)
let pauli: PauliString = "ZZ".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI / 2.0, &qubits)
    .unwrap();

let ops = circuit.operations();
assert_eq!(ops.len(), 3);
assert_eq!(ops[0].qubits[0], qubits[0]);
assert_eq!(ops[0].qubits[1], qubits[1]);
assert_eq!(ops[1].qubits[0], qubits[1]);
```

全恒等串只贡献全局相位：

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

let pauli: PauliString = "II".parse().unwrap();
circuit
    .pauli_evolution(&pauli, std::f64::consts::PI, &qubits)
    .unwrap();

assert_eq!(circuit.operations().len(), 0);
```

比特数不一致：

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::qis::evolution::PauliEvolution;
use cqlib_core::qis::pauli::PauliString;

let mut circuit = Circuit::new(2);
let qubits = circuit.qubits();

let pauli: PauliString = "XXX".parse().unwrap();
assert!(circuit.pauli_evolution(&pauli, 1.0, &qubits).is_err());
```

---

## Hamiltonian 上的演化入口

两个入口都接受相同的参数：

- `time` (`f64`)：总演化时间 $t$。
- `steps` (`usize`)：Trotter 步数 $n$，必须大于 0。
- `mode` (`TrotterMode`)：分解模式。

两者都先复制一份 Hamilton 量并化简，再检查系数的虚部：虚部绝对值超过 $10^{-10}$ 时返回 `QisError::NotHermitian`，因为只有 Hermitian 算符才能生成酉演化。

### `fn to_trotter_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`

按指定模式生成近似演化线路，输出线路的比特数与 `num_qubits` 一致。

对于系数为 $c_k$ 的项 $P_k$，时间片 $\Delta t = t/n$，项内旋转角取 $2 c_k \Delta t$。一阶模式逐片按正序作用所有项；二阶模式每片先按正序作用半角 $\Delta t/2$ 的演化，再按逆序作用一遍；随机模式在一阶的基础上对每片内各项的顺序做随机排列，排列由 `TrotterMode::Randomized` 携带的种子决定。

示例：

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// H = 0.5 * ZZ
let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

// 2 步，每步 3 个门
let circuit = h.to_trotter_circuit(1.0, 2, TrotterMode::FirstOrder).unwrap();
assert_eq!(circuit.operations().len(), 6);

// 二阶模式每片再重复一遍半角演化，1 步共 6 个门
let circuit = h.to_trotter_circuit(1.0, 1, TrotterMode::SecondOrder).unwrap();
assert_eq!(circuit.operations().len(), 6);
```

### `fn to_evolution_circuit(&self, time: f64, steps: usize, mode: TrotterMode) -> Result<Circuit, QisError>`

自动在精确分解与 Trotter 近似之间选择。先判断所有项是否两两对易：

- 对易时走精确路径，按 $U(t) = \prod_k e^{-i c_k t P_k}$ 逐项各作用一次，此时 `steps` 与 `mode` 不起作用。
- 不对易时退化为 `to_trotter_circuit` 的同一套逻辑，`steps` 与 `mode` 生效。

示例：

```rust
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// ZZ 与 ZI 对易，直接走精确路径
let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();
h.add_term("ZI".parse::<PauliString>().unwrap(), 0.3.into()).unwrap();

let circuit = h.to_evolution_circuit(1.0, 1, TrotterMode::FirstOrder).unwrap();
assert_eq!(circuit.num_qubits(), 2);
```

对易情形下精确路径与一阶 Trotter 给出相同的线路矩阵：

```rust
use cqlib_core::circuit::circuit_to_matrix;
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

let mut h = Hamiltonian::new(1);
h.add_term("Z".parse::<PauliString>().unwrap(), 0.7.into()).unwrap();

let time = 0.4_f64;
let exact = h.to_evolution_circuit(time, 1, TrotterMode::FirstOrder).unwrap();
let trotter = h.to_trotter_circuit(time, 1, TrotterMode::FirstOrder).unwrap();

let m_exact = circuit_to_matrix(&exact, None).unwrap();
let m_trotter = circuit_to_matrix(&trotter, None).unwrap();
for (a, b) in m_exact.iter().zip(m_trotter.iter()) {
    assert!((a - b).norm() < 1e-12);
}
```

精确路径的矩阵形式：

```rust
use cqlib_core::circuit::circuit_to_matrix;
use cqlib_core::qis::{Hamiltonian, PauliString};
use cqlib_core::qis::evolution::TrotterMode;

// H = 0.5*Z，t = π，U = e^{-i*0.5*π*Z} = diag(-i, i)
let mut h = Hamiltonian::new(1);
h.add_term("Z".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

let circuit = h
    .to_evolution_circuit(std::f64::consts::PI, 1, TrotterMode::FirstOrder)
    .unwrap();
let matrix = circuit_to_matrix(&circuit, None).unwrap();

assert!((matrix[[0, 0]] - num_complex::Complex64::new(0.0, -1.0)).norm() < 1e-12);
assert!((matrix[[1, 1]] - num_complex::Complex64::new(0.0, 1.0)).norm() < 1e-12);
```

---

## 校验与错误处理

| 错误 | 触发场景 |
| --- | --- |
| `QisError::InvalidParameterValue` | `steps` 为 0，或 Hamilton 量的项列表为空。 |
| `QisError::NotHermitian` | 化简后仍存在虚部绝对值超过容差的系数。 |
| `QisError::UnsupportedOperation` | 线路构造过程中出错，例如项中存在相位为 $\pm i$ 的 Pauli 串。 |
| `CircuitError::QubitCountMismatch` | `pauli_evolution()` 的 `qubits` 长度与 Pauli 串比特数不一致。 |
| `CircuitError::InvalidOperation` | `pauli_evolution()` 的 Pauli 串相位为 $\pm i$。 |

`steps` 与空 Hamilton 量的检查发生在最前面，因此 `to_evolution_circuit()` 即使会走精确路径，也仍然要求 `steps` 大于 0。

示例：

```rust
use cqlib_core::qis::Hamiltonian;
use cqlib_core::qis::evolution::TrotterMode;

let h = Hamiltonian::new(2);
assert!(h.to_trotter_circuit(1.0, 1, TrotterMode::FirstOrder).is_err());

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse().unwrap(), 0.5.into()).unwrap();
assert!(h.to_trotter_circuit(1.0, 0, TrotterMode::FirstOrder).is_err());
assert!(h.to_evolution_circuit(1.0, 0, TrotterMode::FirstOrder).is_err());
```

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与术语表。
- [Hamiltonian 与 Observable](7_hamiltonian.md)：项的构造、化简、对易性判断与期望值。
- [Pauli 算子与 Pauli 串](6_pauli.md)：相位取值与字符串解析约定。
