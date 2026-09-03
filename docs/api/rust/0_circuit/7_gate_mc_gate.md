# MCGate

`cqlib_core::circuit::MCGate`

```rust
use cqlib_core::circuit::{MCGate, StandardGate};
```

`MCGate` 用于表示“在一个标准门前增加若干控制位”得到的多控制门。它以 `StandardGate` 作为基础门，并在该基础门原有作用量子比特之前添加新的控制量子比特。只有当所有控制位都满足控制条件时，基础门才会作用于目标量子比特。

---

## 构造函数

```rust
pub fn new(num_controls: u8, gate: StandardGate) -> Self
```

`MCGate::new(num_controls, gate)` 用于在基础标准门 `gate` 前新增 `num_controls` 个控制位。

| 参数 | 说明 |
| --- | --- |
| `num_controls` | 新增控制位数量。 |
| `gate` | 被控制的基础标准门。 |

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let ccx = MCGate::new(2, StandardGate::X);
let mch = MCGate::new(3, StandardGate::H);
```

在上述示例中，`MCGate::new(2, StandardGate::X)` 表示一个双控制 `X` 门，其语义等价于常见的 Toffoli 门；`MCGate::new(3, StandardGate::H)` 表示一个三控制 `H` 门。

---

## 属性方法

`MCGate` 提供了一组元数据方法，用于查询控制位数量、总作用量子比特数量、参数数量和基础门类型。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `num_ctrl_qubits()` | `usize` | 返回总控制位数量，包括新增控制位以及基础门自带的控制位。 |
| `num_qubits()` | `usize` | 返回该多控制门作用的总量子比特数量。 |
| `num_params()` | `usize` | 返回基础门所需的参数数量。 |
| `base_gate()` | `&StandardGate` | 返回被控制的基础标准门。 |

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::CX);

assert_eq!(gate.num_ctrl_qubits(), 2); // 新增 1 个控制位 + CX 自带 1 个控制位
assert_eq!(gate.num_qubits(), 3);
assert_eq!(gate.num_params(), 0);
assert_eq!(*gate.base_gate(), StandardGate::CX);
```

---

## 控制位与目标位顺序

应用 `MCGate` 时，量子比特顺序具有明确语义：

```text
[new_control_0, new_control_1, ..., base_gate_qubit_0, base_gate_qubit_1, ...]
```

也就是说，新增控制位始终排在最前面，随后才是基础门本身所需的量子比特。如果基础门本身已经带有控制位，则基础门内部的控制位仍按该标准门原本的顺序保留。

例如，`MCGate::new(2, StandardGate::X)` 作用于三个量子比特：

```text
[control_0, control_1, target]
```

其中前两个是新增控制位，最后一个是 `X` 门的目标位。

而 `MCGate::new(1, StandardGate::CX)` 作用于三个量子比特：

```text
[new_control, cx_control, cx_target]
```

其中第一个是新增控制位，第二个和第三个分别是原 `CX` 门的控制位和目标位。

---

## 矩阵

```rust
pub fn matrix(
    &self,
    params: &[f64],
) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError>
```

`matrix(params)` 用于返回多控制门的局部数值矩阵。参数列表 `params` 对应基础门的参数，数量必须与 `self.num_params()` 一致。

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::RZ);

let matrix = gate.matrix(&[std::f64::consts::PI / 2.0])?;
assert_eq!(matrix.shape(), &[4, 4]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，基础门是单量子比特参数门 `RZ`，新增一个控制位后得到一个两量子比特受控 `RZ` 门，因此矩阵形状为 `(4, 4)`。

一般来说，如果 `MCGate` 作用于 `n` 个量子比特，其矩阵形状为：

```text
2^n × 2^n
```

---

## 反门

```rust
pub fn inverse(&self, params: &[Parameter]) -> Option<(MCGate, SmallVec<[Parameter; 3]>)>
```

`inverse(params)` 用于返回当前多控制门的反门及变换后的参数列表。多控制门的反门等价于“对基础门取逆后再添加相同控制位”。

```rust
use cqlib_core::circuit::{MCGate, StandardGate};

let gate = MCGate::new(1, StandardGate::S);

let (inverse, params) = gate.inverse(&[]).unwrap();

assert_eq!(*inverse.base_gate(), StandardGate::SDG);
assert!(params.is_empty());
```

---

## `MCGate` 与 `StandardGate`

部分常见受控门已经作为 `StandardGate` 的标准枚举存在。它们可以看作 `MCGate` 的特例。

| 标准门 | 等价形式 |
| --- | --- |
| `StandardGate::CX` | `MCGate::new(1, StandardGate::X)` |
| `StandardGate::CY` | `MCGate::new(1, StandardGate::Y)` |
| `StandardGate::CZ` | `MCGate::new(1, StandardGate::Z)` |
| `StandardGate::CCX` | `MCGate::new(2, StandardGate::X)` |
| `StandardGate::CRX` | `MCGate::new(1, StandardGate::RX)` |
| `StandardGate::CRY` | `MCGate::new(1, StandardGate::RY)` |
| `StandardGate::CRZ` | `MCGate::new(1, StandardGate::RZ)` |
