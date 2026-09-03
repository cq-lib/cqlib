# Classical Data / Control Flow

- `cqlib_core::circuit::ClassicalType`
- `cqlib_core::circuit::ClassicalVar`
- `cqlib_core::circuit::ClassicalValue`
- `cqlib_core::circuit::Measurement`
- `cqlib_core::circuit::ClassicalExpr`
- `cqlib_core::circuit::ClassicalControlOp`
- `cqlib_core::circuit::{IfOp, WhileOp, ForOp, SwitchOp, ControlBody}`

```rust
use cqlib_core::circuit::{ClassicalType, ClassicalVar, ClassicalValue, Measurement, ClassicalExpr, ClassicalControlOp};
use cqlib_core::circuit::{IfOp, WhileOp, ForOp, SwitchOp, ControlBody};
```

本页介绍 Rust core 中与经典数据和结构化控制流相关的 API。它们用于在线路中表示测量结果、经典变量、经典表达式，以及基于经典条件的 `if`、`while`、`for`、`switch` 等控制流结构。

---

## 核心概念

| 概念 | 说明 |
| --- | --- |
| `ClassicalType` | 经典数据的静态类型，包括 `Bit`、`Bool`、`UInt(width)` 和 `BitVec(width)`。 |
| `ClassicalVar` | 可变经典变量句柄，由 `Circuit::var()` 创建，可通过 `store()` 更新。 |
| `ClassicalValue` | 不可变经典值句柄，通常由测量产生，具有 SSA 风格语义。 |
| `Measurement` | 测量回执，记录测量结果值和被测量量子比特顺序。 |
| `ClassicalExpr` | 类型化经典表达式 AST，用于变量读取、值读取、逻辑运算、比较、类型转换和位操作。 |
| `ClassicalControlOp` | 存储层结构化控制流 IR，包括 `if`、`while`、`for`、`switch`、`break`、`continue`。 |
| `ValueClassicalControlOp` | 构造层控制流 IR，适合序列化、导入器和 `Circuit::from_operations()`。 |
| `ControlBody` / `ValueControlBody` | 控制流体，分别保存存储层或构造层操作序列。 |

---

## `ClassicalType`

```rust
pub enum ClassicalType {
    Bit,
    Bool,
    UInt(u32),
    BitVec(u32),
}
```

`ClassicalType` 用于描述经典数据的静态类型。通过显式类型，Cqlib 可以在构造阶段检查表达式、变量、控制流条件和测量结果之间的类型是否匹配。

| 构造 | 说明 |
| --- | --- |
| `ClassicalType::Bit` | 单个 bit，通常表示单量子比特测量结果，取值为 0 或 1。 |
| `ClassicalType::Bool` | 逻辑布尔值，用于 `if`、`while` 等条件判断。 |
| `ClassicalType::uint(width)` | 创建指定位宽的无符号整数类型，宽度非法时返回 `None`。 |
| `ClassicalType::bit_vec(width)` | 创建指定位宽的 bit 向量类型。 |

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `width()` | 返回类型位宽。 |
| `zero_literal()` | 返回该类型对应的类型化 0 字面量表达式。 |
| `one_literal()` | 返回该类型对应的类型化 1 字面量表达式。 |
| `measurement_width()` | 若该类型可作为测量结果类型，则返回对应测量宽度。 |

```rust
use cqlib_core::circuit::ClassicalType;

let bit_ty = ClassicalType::Bit;
let bool_ty = ClassicalType::Bool;
let u8_ty = ClassicalType::uint(8).expect("valid uint width");
let bv4_ty = ClassicalType::bit_vec(4).expect("valid bit vector width");

assert_eq!(bit_ty.width(), 1);
assert_eq!(bool_ty.width(), 1);
assert_eq!(u8_ty.width(), 8);
assert_eq!(bv4_ty.width(), 4);
```

在控制流中，应明确区分 `Bit` 和 `Bool`：`Bit` 更偏向底层位值，`Bool` 更偏向逻辑判断。若要将单 bit 测量结果作为条件使用，通常需要先转换为 `Bool` 表达式。

---

## `ClassicalVar` 与 `ClassicalValue`

`ClassicalVar` 和 `ClassicalValue` 都是经典数据句柄，但二者语义不同。

| 类型 | 创建方式 | 说明 |
| --- | --- | --- |
| `ClassicalVar` | `Circuit::var(ty)` | 可变经典存储句柄，可通过 `Circuit::store()` 更新。 |
| `ClassicalValue` | `Circuit::measure()` / `Circuit::measure_bits()` | 不可变经典值句柄，通常由测量产生，具有 SSA 风格。 |

二者都提供以下基本接口：

- `index()`
- `circuit_id()`
- `ty()`
- `expr()`

其中，`ClassicalVar` 还提供 `id()`，用于表示由 `CircuitId` 和变量索引组成的稳定身份。

```rust
use cqlib_core::circuit::{Circuit, ClassicalType};

let mut circuit = Circuit::new(1);

let flag = circuit.var(ClassicalType::Bool);
let flag_expr = flag.expr();

assert_eq!(flag.ty(), ClassicalType::Bool);
assert_eq!(flag.circuit_id(), circuit.id());
```

`ClassicalVar` 适合表示在控制流中可更新的状态，例如循环标志、计数器或分支状态。`ClassicalValue` 则通常表示一次测量产生的不可变结果。由于它具有 SSA 风格语义，通常要求先定义后使用，并且不能越过定义它的作用域非法逃逸。

---

## `Measurement`

`Measurement` 是测量操作返回的回执对象，用于记录测量结果值以及被测量的量子比特顺序。

| 方法 | 说明 |
| --- | --- |
| `value()` | 返回测量产生的 `ClassicalValue`。 |
| `expr()` | 返回读取该测量结果的 `ClassicalExpr`。 |
| `qubits()` | 返回被测量量子比特顺序。 |
| `width()` | 返回测量 bit 数。 |
| `ty()` | 返回测量结果类型。 |
| `check_qubits(num_qubits)` | 检查测量量子比特是否在给定状态范围内。 |
| `project(full)` | 从完整 `Outcome` 中投影出该测量对应的结果。 |
| `project_basis(basis)` | 从计算基下标中投影该测量对应的结果。 |

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

let m = circuit.measure(Qubit::new(0))?;
assert_eq!(m.width(), 1);

let expr = m.expr();

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

对于单量子比特测量，结果通常为 `Bit` 类型；对于多量子比特测量，结果通常为 `BitVec(width)`。如果希望将测量结果用于布尔条件，可根据需要调用 `to_bool()` 或相应类型转换接口。

---

## `ClassicalExpr`

`ClassicalExpr` 是类型化、无副作用的经典表达式 AST。它用于表示经典侧计算逻辑，例如读取变量、读取测量值、构造字面量、进行逻辑运算、比较、类型转换、条件选择和位操作。

### 创建表达式

| 方法 | 说明 |
| --- | --- |
| `ClassicalExpr::var(var)` | 读取可变经典变量。 |
| `ClassicalExpr::value(value)` | 读取不可变经典值。 |
| `bool_literal(value)` | 创建 `Bool` 字面量。 |
| `bit_literal(value)` | 创建 `Bit` 字面量。 |
| `uint_literal(width, value)` | 创建指定位宽 `UInt` 字面量。 |
| `bit_vec_literal(width, value)` | 创建指定位宽 `BitVec` 字面量。 |

```rust
use cqlib_core::circuit::ClassicalExpr;

let truth = ClassicalExpr::bool_literal(true);
let bit_one = ClassicalExpr::bit_literal(true);
let u3 = ClassicalExpr::uint_literal(3, 5)?;
let bits = ClassicalExpr::bit_vec_literal(4, 0b1010)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 逻辑与位操作

| 方法 | 说明 |
| --- | --- |
| `try_not(expr)` | 构造 NOT 表达式。 |
| `try_and(lhs, rhs)` | 构造 AND 表达式。 |
| `try_or(lhs, rhs)` | 构造 OR 表达式。 |
| `try_xor(lhs, rhs)` | 构造 XOR 表达式。 |

表达式还实现了 `!`、`&`、`|`、`^` 等便捷操作符。对于库代码、导入器或编译器 pass，更推荐使用 `try_*` 系列方法，因为它们可以显式返回类型错误，避免由于类型不匹配引发 panic 或不易定位的问题。

### 类型转换

| 方法 | 说明 |
| --- | --- |
| `bit_to_bool(expr)` | 将 `Bit` 转换为 `Bool`。 |
| `bit_vec_to_uint(expr)` | 将 `BitVec` 转换为 `UInt`。 |
| `to_bool(self)` | 将 `Bit` 或 `UInt` 转换为 `Bool`。 |
| `to_uint(self)` | 将 `Bit` 或 `BitVec` 转换为 `UInt`。 |

常见用法是将测量得到的 `Bit` 转换为 `Bool`，再作为 `if` 或 `while` 条件。

### 比较与选择

| 方法 | 说明 |
| --- | --- |
| `eq(lhs, rhs)` | 构造相等比较。 |
| `ne(lhs, rhs)` | 构造不等比较。 |
| `lt(lhs, rhs)` | 构造小于比较。 |
| `le(lhs, rhs)` | 构造小于等于比较。 |
| `gt(lhs, rhs)` | 构造大于比较。 |
| `ge(lhs, rhs)` | 构造大于等于比较。 |
| `select(condition, then_expr, else_expr)` | 构造三元条件选择表达式。 |

比较结果通常为 `Bool` 表达式，可直接用于 `if_`、`if_else` 或 `while_` 的条件。构造比较时，左右两侧表达式类型应兼容，例如相同位宽的 `UInt` 表达式之间进行大小比较。

### Bit 提取和拼接

| 方法 | 说明 |
| --- | --- |
| `extract_bit(value, index)` | 从 `UInt` 或 `BitVec` 中提取单个 bit。 |
| `extract_bits(value, offset, width)` | 提取连续 bit 区间。 |
| `concat(parts)` | 拼接多个 `BitVec` 表达式。 |
| `pack_bits(bits)` | 将多个 `Bit` 表达式打包为 `BitVec`。 |
| `simplified()` | 返回化简后的表达式。 |

---

## `Circuit` 高层控制流 API

`Circuit` 提供闭包式控制流构造接口，用于以较自然的 Rust 代码风格构造结构化控制流。

```rust
circuit.if_(condition, body)
circuit.if_else(condition, then_body, else_body)
circuit.while_(condition, body)
circuit.for_uint(var, start, stop, step, body)
circuit.switch(target, build)
circuit.break_loop()
circuit.continue_loop()
```

### 1. `if_`

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);
let m = circuit.measure(Qubit::new(0))?;

circuit.if_(m.expr().to_bool()?, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`if_` 用于构造无 `else` 分支的条件结构。条件须是 `Bool` 表达式，body 回调用于构造条件成立时执行的线路片段。

### 2. `if_else`

```rust
circuit.if_else(
    m.expr().to_bool()?,
    |then_body| {
        then_body.x(Qubit::new(1))?;
        Ok(())
    },
    |else_body| {
        else_body.z(Qubit::new(1))?;
        Ok(())
    },
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`if_else` 用于构造二选一条件分支。两个分支都会作为同一个控制流结构的一部分提交；如果任一分支构造失败，整个控制流构造会回滚。

### 3. `while_` / `for_uint` / `switch`

| 方法 | 说明 |
| --- | --- |
| `while_(condition, body)` | 构造基于 `Bool` 条件的运行时 `while` 循环。 |
| `for_uint(var, start, stop, step, body)` | 构造基于 `UInt` 变量的半开区间循环。 |
| `switch(target, build)` | 构造基于 `UInt` 精确匹配的多分支选择。 |
| `break_loop()` | 退出最近的循环或允许跳转的控制流体。 |
| `continue_loop()` | 跳转到最近循环的下一轮。 |

`for_uint` 要求循环变量、起点、终点和步长都是相同位宽的 `UInt` 表达式；`switch` 的 target 必须为 `UInt`，case 值必须能由该位宽表示。

---

## 低层控制流 IR

除高层闭包式 API 外，Rust core 也提供了低层控制流 IR，适合导入器、反序列化器、编译器 pass 和底层测试使用。

| 类型 | 说明 |
| --- | --- |
| `ControlBody` | 存储层控制流体，包含 `Vec<Operation>`。 |
| `ValueControlBody` | 构造层控制流体，包含 `Vec<ValueOperation>`。 |
| `IfOp` | 条件分支结构。 |
| `WhileOp` | `while` 循环结构。 |
| `ForOp` | `UInt` 半开区间循环结构。 |
| `SwitchOp` / `SwitchCase` | `switch` 多分支结构。 |
| `ClassicalControlOp` | 存储层控制流枚举，包含上述控制流操作。 |
| `ValueClassicalControlOp` | 构造层控制流枚举，适合自包含 IR。 |

`Circuit::append_control(op)` 可以追加存储层 `ClassicalControlOp`。对于自包含导入场景，也可以通过 `ValueClassicalControlOp` 和 `ValueOperation` 组合后交给 `Circuit::from_operations()` 处理。

---

## 校验规则

`Circuit::validate()` 和相关 append 过程会检查经典数据与控制流结构的一致性。典型校验包括：

- `ClassicalVar` 和 `ClassicalValue` 必须属于当前 `CircuitId`；
- 不可变 `ClassicalValue` 必须先定义后读取；
- 控制流作用域内定义的值不能逃逸到外层作用域；
- `break` 和 `continue` 只能出现在合法的控制流体内；
- 控制转移之后不能继续追加同一 body 的普通操作；
- `if` 和 `while` 条件必须是 `Bool` 表达式；
- `for_uint` 的变量、起点、终点和步长必须是相同位宽的 `UInt`；
- `switch` target 必须是 `UInt`，case 值必须能被该位宽表示；
- 经典表达式中涉及的变量和值必须满足类型兼容要求。

对于外部导入、程序自动生成或底层 IR 手动构造的线路，建议在进入编译器后续阶段或后端执行前显式调用：

```rust
circuit.validate()?;
```

这样可以尽早发现句柄归属、类型不匹配、作用域逃逸和控制流跳转位置错误。