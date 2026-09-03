# StandardGate

`cqlib_core::circuit::StandardGate`

```rust
use cqlib_core::circuit::StandardGate;
```

`StandardGate` 是 Rust core 中用于表示 Cqlib 原生标准量子门的枚举类型。它覆盖常用单量子比特门、参数化旋转门、多量子比特门、受控门、二体 Pauli 旋转门、fSim 门以及全局相位标记等基础门集合。

---

## 枚举变体

```rust
pub enum StandardGate {
    I,
    H,
    RX,
    RXX,
    RXY,
    RY,
    RYY,
    RZ,
    RZX,
    RZZ,
    S,
    SDG,
    SWAP,
    T,
    TDG,
    U,
    X,
    XY,
    X2P,
    X2M,
    XY2P,
    XY2M,
    Y,
    Y2P,
    Y2M,
    Z,
    Phase,
    GPhase,
    CX,
    CCX,
    CY,
    CZ,
    CRX,
    CRY,
    CRZ,
    FSIM,
}
```

---

## 门元数据

`StandardGate` 提供一组元数据方法，用于查询门的作用量子比特数量、内置控制位数量、参数数量和对角性等信息。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `all()` | `&'static [StandardGate]` | 返回所有标准门枚举值，常用于生成门表、测试或目标 basis 检查。 |
| `num_qubits()` | `usize` | 返回该门作用的总量子比特数量。 |
| `num_ctrl_qubits()` | `usize` | 返回门内置控制位数量。 |
| `num_params()` | `usize` | 返回该门需要的参数数量。 |
| `is_diagonal()` | `bool` | 判断该门在计算基下是否为对角门。 |

```rust
use cqlib_core::circuit::StandardGate;

assert_eq!(StandardGate::H.num_qubits(), 1);
assert_eq!(StandardGate::CX.num_ctrl_qubits(), 1);
assert_eq!(StandardGate::RZ.num_params(), 1);
assert!(StandardGate::RZ.is_diagonal());
```

---

## 门分类

### 1. 单量子比特固定门

单量子比特固定门不需要额外参数，常用于状态制备、基变换、相位修正和 Clifford 相关优化。

| 门 | 说明 |
| --- | --- |
| `I` | 恒等门，不改变量子态。 |
| `H` | Hadamard 门，用于在计算基和叠加基之间转换。 |
| `X` / `Y` / `Z` | Pauli 门，分别对应比特翻转、带相位翻转和相位翻转。 |
| `S` / `SDG` | `S` 门及其逆门，属于 Clifford 相位门。 |
| `T` / `TDG` | `T` 门及其逆门，常用于非 Clifford 门集扩展。 |
| `X2P` / `X2M` | X 方向正/负半角旋转门。 |
| `Y2P` / `Y2M` | Y 方向正/负半角旋转门。 |

### 2. 参数化单量子比特门

参数化单量子比特门通过角度参数控制旋转或相位变化，常用于变分线路、参数扫描和硬件校准。

| 门 | 参数数 | 说明 |
| --- | --- | --- |
| `RX` | 1 | X 轴旋转，通常采用 `exp(-i θ X / 2)` 约定。 |
| `RY` | 1 | Y 轴旋转，通常采用 `exp(-i θ Y / 2)` 约定。 |
| `RZ` | 1 | Z 轴旋转，通常采用 `exp(-i θ Z / 2)` 约定。 |
| `RXY` | 2 | XY 平面任意轴旋转。 |
| `U` | 3 | 通用单量子比特门。 |
| `Phase` | 1 | 相位门，对 `|1>` 分量施加相位。 |
| `XY` | 1 | XY 交互族单量子比特参数门。 |
| `XY2P` / `XY2M` | 1 | 正/负半角 XY 门。 |

### 3. 多量子比特门

多量子比特门用于建立量子比特之间的关联，是纠缠态制备、量子算法和编译映射中的核心操作。

| 门 | 参数数 | 说明 |
| --- | --- | --- |
| `CX` | 0 | controlled-X，也称 CNOT。 |
| `CY` | 0 | controlled-Y。 |
| `CZ` | 0 | controlled-Z。 |
| `CCX` | 0 | Toffoli 门，两个控制位和一个目标位。 |
| `SWAP` | 0 | 交换两个量子比特状态。 |
| `RXX` / `RYY` / `RZZ` / `RZX` | 1 | 二量子比特 Pauli 旋转门。 |
| `CRX` / `CRY` / `CRZ` | 1 | 单控制参数化旋转门。 |
| `FSIM` | 2 | fSim 门，常用于描述特定硬件中的双量子比特相互作用。 |

### 4. 全局相位门

`GPhase` 是零量子比特全局相位标记，参数数量为 1。它用于表示全局相位因子，通常不作用于任何具体量子比特。

---

## 矩阵接口

```rust
pub fn matrix(
    &self,
    params: &[f64],
) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError>
```

`matrix()` 返回该标准门的局部数值矩阵，调用时传入的 `params` 数量须等于 `self.num_params()`。

```rust
use cqlib_core::circuit::StandardGate;

let h = StandardGate::H.matrix(&[])?;
let rx = StandardGate::RX.matrix(&[std::f64::consts::PI / 2.0])?;

assert_eq!(h.shape(), &[2, 2]);
assert_eq!(rx.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 数学约定

Cqlib 中的旋转门采用量子计算中常见的半角指数定义。例如：

```text
RX(θ) = exp(-i θ X / 2)
RY(θ) = exp(-i θ Y / 2)
RZ(θ) = exp(-i θ Z / 2)
```

二体 Pauli 旋转门也使用类似约定：

```text
RXX(θ) = exp(-i θ X⊗X / 2)
RYY(θ) = exp(-i θ Y⊗Y / 2)
RZZ(θ) = exp(-i θ Z⊗Z / 2)
RZX(θ) = exp(-i θ Z⊗X / 2)
```

`Phase(λ)` 表示对 `|1>` 分量施加相位；`GPhase(λ)` 表示全局相位因子。

---

## 反门接口

```rust
pub fn inverse(
    &self,
    params: &[Parameter],
) -> Option<(StandardGate, SmallVec<[Parameter; 3]>)>
```

`inverse()` 返回当前标准门的反门枚举及变换后的参数列表。如果当前门不可逆，或传入参数数量与 `num_params()` 不匹配，则返回 `None`。

常见反门关系如下：

| 门 | 逆 |
| --- | --- |
| `H` | `H` |
| `X` / `Y` / `Z` | 自身 |
| `S` | `SDG` |
| `SDG` | `S` |
| `T` | `TDG` |
| `TDG` | `T` |
| `RX(θ)` | `RX(-θ)` |
| `RY(θ)` | `RY(-θ)` |
| `RZ(θ)` | `RZ(-θ)` |
| `CX` / `CY` / `CZ` | 自身 |
| `SWAP` | 自身 |
| `CCX` | 自身 |

```rust
use cqlib_core::circuit::{Parameter, StandardGate};

let theta = Parameter::symbol("theta");
let inverse = StandardGate::RX.inverse(&[theta.clone()]).unwrap();

assert_eq!(inverse.0, StandardGate::RX);

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## `StandardGate` 与 `Circuit`

Rust 侧普通手写线路通常优先使用 `Circuit` 提供的便捷方法，例如 `h()`、`cx()`、`rz()` 等。这些方法会自动构造对应的标准门指令，并检查量子比特数量、参数数量和量子比特归属关系。

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut c = Circuit::new(1);
c.rx(Qubit::new(0), "theta")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在编写导入器、编译器 pass 或底层测试时，也可以显式将 `StandardGate` 转换为 `Instruction`，再通过 `Circuit::append()` 追加到线路中。

```rust
use cqlib_core::circuit::{Circuit, ParameterValue, Qubit, StandardGate};

let mut c = Circuit::new(1);

c.append(
    StandardGate::RX.into(),
    [Qubit::new(0)],
    [ParameterValue::from("theta")],
    None,
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `StandardGate` 与 `MCGate`

`CX`、`CY`、`CZ`、`CCX`、`CRX`、`CRY`、`CRZ` 等标准门可以看作常见受控门的内置枚举形式。对于这些门，编译器和矩阵实现可以直接使用标准门的优化路径。

当需要更多控制位，或希望对任意基础标准门添加控制位时，应使用 `MCGate`。例如，多控制 X 门、多控制相位门和受控自定义标准门通常更适合通过 `MCGate` 表示。

| 标准门 | 可理解为 |
| --- | --- |
| `CX` | 单控制 `X` |
| `CY` | 单控制 `Y` |
| `CZ` | 单控制 `Z` |
| `CCX` | 双控制 `X` |
| `CRX(θ)` | 单控制 `RX(θ)` |
| `CRY(θ)` | 单控制 `RY(θ)` |
| `CRZ(θ)` | 单控制 `RZ(θ)` |