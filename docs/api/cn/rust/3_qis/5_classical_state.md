# 经典状态

`cqlib_core::qis::state`

本页介绍 `cqlib_core::qis` 中的运行期经典数据：`RuntimeValue` 是带类型的运行时值，`ClassicalState` 按线路句柄保存并索引这些值。

线路 IR 保存的是经典句柄而不是具体数值。执行一条包含测量或经典存储的线路时，运行期经典数据把这些句柄填充为实际值：不可变经典值由测量产生，可变经典变量由存储语句写入。句柄的身份由线路 `CircuitId` 与表内索引共同确定，因此不同线路的句柄互不通用。

## 导入

```rust
use cqlib_core::qis::{ClassicalState, RuntimeValue};
```

两个类型也位于 `cqlib_core::qis::state`，两种路径等价。

---

## RuntimeValue

```rust
pub enum RuntimeValue {
    Bit(bool),
    Bool(bool),
    UInt { width: u32, value: u128 },
    BitVec { width: u32, bits: Outcome },
}
```

变体：

| 变体 | 载荷 | 说明 |
| --- | --- | --- |
| `RuntimeValue::Bit(bool)` | `bool` | 单个比特，通常表示单比特测量结果。 |
| `RuntimeValue::Bool(bool)` | `bool` | 逻辑布尔值，用于控制流条件。 |
| `RuntimeValue::UInt { width, value }` | `width: u32`、`value: u128` | 指定位宽的无符号整数，值保存在 `u128` 中。 |
| `RuntimeValue::BitVec { width, bits }` | `width: u32`、`bits: Outcome` | 指定位宽的比特向量，比特承载在设备结果类型 `Outcome` 中。 |

`BitVec` 的位序与设备结果层一致：位索引 `0` 是最低有效位，格式化为字符串时最高位显示在最左。

### 方法

- `fn ty(&self) -> ClassicalType`：返回该运行时值对应的静态经典类型。
- `fn to_bitstring(&self) -> Option<String>`：按最高位在左的顺序返回比特串；`Bit` 与 `BitVec` 返回 `Some`，`Bool` 与 `UInt` 返回 `None`。

### 其他行为

- 支持 `Debug`、`Clone`。
- 按值比较与哈希：类型、位宽与载荷都相同才相等，因此 `RuntimeValue::Bit(true)` 与 `RuntimeValue::Bool(true)` 不相等。

示例：

```rust
use cqlib_core::device::Outcome;
use cqlib_core::qis::RuntimeValue;

let bit = RuntimeValue::Bit(true);
assert_eq!(bit.to_bitstring().as_deref(), Some("1"));

let uint = RuntimeValue::UInt { width: 8, value: 42 };
assert_eq!(uint, RuntimeValue::UInt { width: 8, value: 42 });
assert_eq!(uint.to_bitstring(), None);

// 类型不同则值不相等，即使载荷相同
assert_ne!(RuntimeValue::Bit(true), RuntimeValue::Bool(true));

let bit_vec = RuntimeValue::BitVec {
    width: 3,
    bits: Outcome::from_bitstring("101").unwrap(),
};
assert_eq!(bit_vec.to_bitstring().as_deref(), Some("101"));
```

---

## ClassicalState

```rust
pub struct ClassicalState {
    // 字段不公开
}
```

保存一条线路的运行期经典数据：不可变经典值表、可变经典变量表，以及各自的静态类型表。两者的下标与线路内的句柄索引一一对应。

### 方法

- `fn value(&self, value: ClassicalValue) -> Option<&RuntimeValue>`：返回某个不可变经典值的运行期结果。
- `fn var(&self, var: ClassicalVar) -> Option<&RuntimeValue>`：返回某个可变经典变量的当前运行期值。

两个方法都返回 `None`，当句柄属于另一条线路、类型不匹配、下标超出对应的表，或者对应的值尚未产生（未测量）或尚未存储时。

### 获取方式

`ClassicalState` 不作为普通构造对象使用，而是由稳定子模拟器的线路执行入口返回：

```rust
pub fn run_circuit(circuit: &Circuit) -> Result<CircuitExecutionResult, QisError>
```

`CircuitExecutionResult` 包含最终量子态与本次执行的经典数据：

```rust
pub struct CircuitExecutionResult {
    pub state: StabilizerState,
    pub classical: ClassicalState,
}
```

### 其他行为

- 支持 `Debug`、`Clone`。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let left = c.measure(Qubit::new(0)).unwrap();
let right = c.measure(Qubit::new(1)).unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
let Some(RuntimeValue::Bit(left_bit)) = result.classical.value(left.value()) else {
    panic!("expected first Bell measurement to produce a bit");
};
let Some(RuntimeValue::Bit(right_bit)) = result.classical.value(right.value()) else {
    panic!("expected second Bell measurement to produce a bit");
};

// Bell 态上的两次测量结果必须保持关联
assert_eq!(left_bit, right_bit);
```

多比特测量的结果按 `Circuit::measure_bits()` 的小端输入顺序打包为 `BitVec`：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(3);
c.x(Qubit::new(0)).unwrap();
c.x(Qubit::new(2)).unwrap();
let measured = c
    .measure_bits([Qubit::new(2), Qubit::new(1), Qubit::new(0)])
    .unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
let Some(RuntimeValue::BitVec { width, bits }) = result.classical.value(measured.value()) else {
    panic!("expected BitVec measurement result");
};
assert_eq!(*width, 3);
assert_eq!(bits.to_bitstring(*width as usize), "101");
```

---

## 校验与错误处理

`value()` 与 `var()` 以 `Option` 表达查不到的情况，本身不返回错误。写入侧的校验发生在执行线路时，类型不匹配或句柄不属于当前线路都会作为 `QisError` 返回：

| 错误 | 触发场景 |
| --- | --- |
| `QisError::CircuitError` | 执行线路时使用了不属于该线路的经典句柄，对应 `CircuitError::ForeignClassicalHandle`。 |
| `QisError::UnsupportedOperation` | 经典值或经典变量的类型与线路 IR 中登记的类型不一致，或运行时值的类型无法参与该表达式运算。 |

由于查表使用句柄中携带的 `CircuitId` 与索引，跨线路读取只会得到 `None`，不会读到另一条线路的数据。

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与术语表。
- [Pauli 算子](6_pauli.md)：从测量概率计算 Pauli 串期望值时使用的比特序约定。
