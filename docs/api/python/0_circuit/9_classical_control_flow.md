# Classical Data / Control Flow

- `cqlib.circuit.ClassicalType`
- `cqlib.circuit.ClassicalVar`
- `cqlib.circuit.ClassicalValue`
- `cqlib.circuit.Measurement`
- `cqlib.circuit.ClassicalExpr`
- `cqlib.circuit.ClassicalControlOp`
- `cqlib.circuit.ValueControlBody`
- `cqlib.circuit.ValueSwitchCase`

```python
from cqlib.circuit import (
    ClassicalType,
    ClassicalExpr,
    ClassicalControlOp,
    ValueControlBody,
    ValueSwitchCase,
)
```

经典数据与控制流 API 用于描述量子线路中的经典侧信息和结构化控制逻辑。典型对象包括经典类型、经典变量、测量结果、经典表达式，以及 `if`、`while`、`for`、`switch` 等控制流结构。

---

## 核心对象

经典数据与控制流对象分为以下几类：

| 类型 | 作用 |
|---|---|
| `ClassicalType` | 描述经典值的静态类型，如 `Bit`、`Bool`、`UInt`、`BitVec`。 |
| `ClassicalVar` | 可变经典变量句柄，由 `Circuit.var()` 创建。 |
| `ClassicalValue` | 不可变经典值句柄，通常由测量产生。 |
| `Measurement` | 测量回执，记录测量结果句柄和被测量的量子比特顺序。 |
| `ClassicalExpr` | 无副作用的经典表达式，可表示字面量、变量读取、比较和逻辑组合。 |
| `ClassicalControlOp` | 低层控制流对象，用于表示 `if`、`while`、`for`、`switch`、`break` 和 `continue`。 |
| `ValueControlBody` | 控制流分支体或循环体中的操作序列。 |
| `ValueSwitchCase` | `switch` 中的单个整数匹配分支。 |

---

## `ClassicalType`

`ClassicalType` 用于描述经典值的静态类型。控制流条件、循环变量、测量结果和经典表达式都依赖类型信息进行合法性检查。

```python
ClassicalType.bit() -> ClassicalType
ClassicalType.bool() -> ClassicalType
ClassicalType.uint(width: int) -> ClassicalType
ClassicalType.bit_vec(width: int) -> ClassicalType
```

常用经典类型如下：

| 类型构造 | 说明 | 典型用途 |
|---|---|---|
| `ClassicalType.bit()` | 单个 bit，取值为 `0` 或 `1`。 | 单比特测量结果、位表达式。 |
| `ClassicalType.bool()` | 逻辑布尔值，表示真或假。 | `if`、`while` 条件。 |
| `ClassicalType.uint(width)` | 指定位宽的无符号整数。 | 计数器、循环变量、`switch` 目标。 |
| `ClassicalType.bit_vec(width)` | 指定位宽的 bit 向量。 | 多比特测量结果、位拼接、位打包结果。 |

```python
from cqlib.circuit import ClassicalType

bit_ty = ClassicalType.bit()
flag_ty = ClassicalType.bool()
u8_ty = ClassicalType.uint(8)
bv4_ty = ClassicalType.bit_vec(4)

print(bit_ty.width)   # 1
print(u8_ty.width)    # 8
```

`ClassicalType` 还可以创建该类型下的常用字面量表达式：

```python
u4 = ClassicalType.uint(4)

zero = u4.zero_literal()
one = u4.one_literal()
```

---

## `CircuitId`

`CircuitId` 是线路本地身份标识，用于区分不同 `Circuit` 创建的经典变量和经典值。

```python
from cqlib import Circuit

circuit = Circuit(1)
print(circuit.id)
```

---

## `ClassicalVar`

`ClassicalVar` 表示可变经典变量句柄，由 `Circuit.var(type)` 创建。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType

circuit = Circuit(1)

flag = circuit.var(ClassicalType.bool())
counter = circuit.var(ClassicalType.uint(4))
```

常用属性和方法如下：

| 接口 | 说明 |
|---|---|
| `id` | 由 `circuit_id` 和 `index` 组成的稳定标识。 |
| `index` | 变量在所属线路中的位置。 |
| `circuit_id` | 创建该变量的线路身份。 |
| `ty` | 变量的静态经典类型。 |
| `expr()` | 构造读取该变量当前值的 `ClassicalExpr`。 |

```python
from cqlib.circuit import ClassicalExpr

circuit.store(flag, ClassicalExpr.bool_literal(True))
circuit.store(counter, ClassicalExpr.uint_literal(4, 3))

print(flag.index)
print(flag.ty)
print(flag.circuit_id == circuit.id)
```

`store(target, value)` 用于向经典变量写入表达式。写入时，`value` 的类型必须与 `target.ty` 兼容。若类型不匹配，后续校验或转换通常会失败。

需要注意的是，`ClassicalVar` 具有线路归属关系，不应在不同 `Circuit` 之间直接混用。如果需要在另一条线路中表达相同逻辑，应在该线路中重新创建对应变量。

---

## `ClassicalValue`

`ClassicalValue` 表示不可变经典值句柄，通常由测量操作产生。

```python
from cqlib import Circuit

circuit = Circuit(1)

measurement = circuit.measure(0)
value = measurement.value
```

常用属性和方法如下：

| 接口 | 说明 |
|---|---|
| `index` | 经典值在所属线路中的位置。 |
| `circuit_id` | 创建该经典值的线路身份。 |
| `ty` | 经典值的静态类型。 |
| `expr()` | 构造读取该经典值的表达式。 |

```python
expr = value.expr()
```

---

## `Measurement`

`Measurement` 是测量操作返回的回执对象，用于记录测量结果句柄和被测量的量子比特顺序。

常用属性和方法如下：

| 属性/方法 | 说明 |
|---|---|
| `value` | 测量产生的 `ClassicalValue` 结果句柄。 |
| `qubits` | 被测量的量子比特顺序。 |
| `ty` | 测量结果类型；单量子比特为 `Bit`，多量子比特为 `BitVec`。 |
| `width` | 测量 bit 数。 |
| `expr()` | 构造读取测量结果的表达式。 |
| `check_qubits(num_qubits)` | 检查被测量量子比特是否在给定量子比特范围内。 |
| `project(full)` | 从完整 `Outcome` 中投影出该测量对应结果。 |
| `project_basis(basis)` | 从计算基下标中投影该测量对应结果。 |

```python
from cqlib import Circuit

circuit = Circuit(3)

m = circuit.measure_bits([0, 2])

print(m.width)       # 2
print(m.ty)
print(m.qubits)

expr = m.expr()
```

如果需要将测量结果作为条件使用，可以将 `Bit` 或 `BitVec` 转换为适合控制流条件的表达式。例如，单比特测量结果可以转换为 `Bool`：

```python
single = circuit.measure(1)
condition = single.expr().to_bool()
```

---

## `ClassicalExpr`

`ClassicalExpr` 它用于描述经典计算关系，如读取变量、读取测量值、构造字面量、执行逻辑运算、比较表达式、条件选择、位抽取和位拼接。

### 1. 创建表达式

| 静态方法 | 说明 |
|---|---|
| `ClassicalExpr.var(var)` | 读取可变经典变量。 |
| `ClassicalExpr.value(value)` | 读取不可变经典值。 |
| `ClassicalExpr.bool_literal(value)` | 构造 `Bool` 字面量。 |
| `ClassicalExpr.bit_literal(value)` | 构造 `Bit` 字面量。 |
| `ClassicalExpr.uint_literal(width, value)` | 构造指定位宽 `UInt` 字面量。 |
| `ClassicalExpr.bit_vec_literal(width, value)` | 构造指定位宽 `BitVec` 字面量。 |

```python
from cqlib.circuit import ClassicalExpr

true_expr = ClassicalExpr.bool_literal(True)
bit_one = ClassicalExpr.bit_literal(True)
u4_three = ClassicalExpr.uint_literal(4, 3)
bits = ClassicalExpr.bit_vec_literal(4, 0b1010)
```

### 2. 逻辑与位操作

`ClassicalExpr` 支持常见逻辑运算。运算符 `~`、`&`、`|`、`^` 分别对应非、与、或和异或。

| 方法 / 运算符 | 说明 |
|---|---|
| `not_()` / `~expr` | 逻辑或 bit 取反。 |
| `and_(rhs)` / `expr & rhs` | 与运算。 |
| `or_(rhs)` / `expr \| rhs` | 或运算。 |
| `xor(rhs)` / `expr ^ rhs` | 异或运算。 |

```python
from cqlib.circuit import ClassicalExpr

a = ClassicalExpr.bool_literal(True)
b = ClassicalExpr.bool_literal(False)

expr = (a & ~b) | b
print(expr.simplified())
```

### 3. 类型转换

| 方法 | 说明 |
|---|---|
| `bit_to_bool()` | 将 `Bit` 转换为 `Bool`。 |
| `to_bool()` | 将 `Bit` 或 `UInt` 转换为 `Bool`。 |
| `bit_vec_to_uint()` | 将 `BitVec` 转换为 `UInt`。 |
| `to_uint()` | 将 `Bit` 或 `BitVec` 转换为 `UInt`。 |

```python
from cqlib.circuit import ClassicalExpr

bit_as_bool = ClassicalExpr.bit_literal(True).bit_to_bool()
bits_as_uint = ClassicalExpr.bit_vec_literal(4, 0b1010).to_uint()
```

当表达式需要作为 `if_()` 或 `while_()` 条件时，应确保其类型为 `Bool`。如果原始表达式是 `Bit` 或 `UInt`，可以通过 `to_bool()` 进行显式转换。

### 4. 比较与选择

运行时比较可以使用 `ClassicalExpr` 提供的静态方法。

| 静态方法 | 说明 |
|---|---|
| `equal(lhs, rhs)` | 相等比较。 |
| `not_equal(lhs, rhs)` | 不等比较。 |
| `lt(lhs, rhs)` | 小于比较。 |
| `le(lhs, rhs)` | 小于等于比较。 |
| `gt(lhs, rhs)` | 大于比较。 |
| `ge(lhs, rhs)` | 大于等于比较。 |
| `select(condition, then_expr, else_expr)` | 根据 `Bool` 条件选择两个表达式之一。 |

```python
from cqlib.circuit import ClassicalExpr

x = ClassicalExpr.uint_literal(3, 2)
y = ClassicalExpr.uint_literal(3, 5)

cond1 = ClassicalExpr.lt(x, y)
cond2 = ClassicalExpr.equal(x, y)
selected = ClassicalExpr.select(cond1, x, y)
```

比较结果通常是 `Bool` 表达式，可以直接作为 `if_()`、`if_else()` 或 `while_()` 的条件。`select()` 的两个分支表达式应具有兼容类型。

### 5. Bit 提取和拼接

| 方法 | 说明 |
|---|---|
| `extract_bit(index)` | 从 `UInt` 或 `BitVec` 中提取一个 bit。 |
| `extract_bits(offset, width)` | 提取连续 `Bit` 区间。 |
| `concat(parts)` | 拼接多个 `BitVec` 表达式。 |
| `pack_bits(bits)` | 将多个 `Bit` 表达式打包为 `BitVec`。 |

```python
from cqlib.circuit import ClassicalExpr

bits = ClassicalExpr.bit_vec_literal(4, 0b1010)

low_bit = bits.extract_bit(0)
middle = bits.extract_bits(offset=1, width=2)

packed = ClassicalExpr.pack_bits(
    [
        ClassicalExpr.bit_literal(True),
        ClassicalExpr.bit_literal(False),
    ]
)
```

### 6. 化简和字面量检查

| 方法 | 说明 |
|---|---|
| `simplified()` | 返回化简后的表达式。 |
| `is_bool_true()` / `is_bool_false()` | 判断是否为 `Bool` 常量。 |
| `is_bit_true()` / `is_bit_false()` | 判断是否为 `Bit` 常量。 |

---

## 高层控制流 API

`Circuit` 提供一组高层闭包式控制流接口，用于构造结构化条件分支、循环和多分支选择：

```python
circuit.if_(condition, body)
circuit.if_else(condition, then_body, else_body)
circuit.while_(condition, body)
circuit.for_uint(var, start, stop, step, body)
circuit.switch(target, build)
circuit.break_loop()
circuit.continue_loop()
```

这些接口通过回调函数构造分支体或循环体。回调函数接收临时线路构造器，用户可以在其中追加需要放入控制流体内的操作。构造完成后，整个控制流结构会作为一条操作加入线路。

### 1. `if_`

`if_(condition, body)` 用于构造不带 `else` 分支的条件结构。`condition` 须是 `Bool` 表达式。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(2)

circuit.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(1),
)
```

如果条件来自单比特测量结果，通常需要先转换为 `Bool`：

```python
m = circuit.measure(0)
circuit.if_(m.expr().to_bool(), lambda body: body.x(1))
```

### 2. `if_else`

`if_else(condition, then_body, else_body)` 用于构造带有 `else` 分支的条件结构。当条件为真时执行 `then_body`，否则执行 `else_body`。

```python
circuit.if_else(
    m.expr().to_bool(),
    lambda then_body: then_body.x(1),
    lambda else_body: else_body.z(1),
)
```

### 3. `while_`

`while_(condition, body)` 用于构造基于 `Bool` 条件的循环结构。

```python
from cqlib.circuit import ClassicalType, ClassicalExpr

flag = circuit.var(ClassicalType.bool())
circuit.store(flag, ClassicalExpr.bool_literal(True))

circuit.while_(
    flag.expr(),
    lambda body: body.store(flag, ClassicalExpr.bool_literal(False)),
)
```

在循环体中可以使用 `break_loop()` 和 `continue_loop()` 控制循环流程：

```python
circuit.while_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.break_loop(),
)
```

### 4. `for_uint`

`for_uint(var, start, stop, step, body)` 用于构造基于无符号整数变量的半开区间循环。循环变量 `var` 必须是 `UInt` 类型，`start`、`stop` 和 `step` 必须与变量位宽一致。

```python
from cqlib.circuit import ClassicalType, ClassicalExpr

i = circuit.var(ClassicalType.uint(4))
u4 = ClassicalType.uint(4)

circuit.for_uint(
    i,
    u4.zero_literal(),
    ClassicalExpr.uint_literal(4, 3),
    u4.one_literal(),
    lambda body, i_expr: body.x(0),
)
```

循环范围采用 `[start, stop)` 语义。`body` 回调接收两个参数：临时线路构造器和当前循环变量对应的 `UInt` 表达式。

### 5. `switch`

`switch(target, build)` 用于根据 `UInt` 表达式进行精确整数匹配，并选择对应分支执行。

```python
target = ClassicalExpr.uint_literal(2, 1)

def build_cases(case):
    case.value(0, lambda body: body.x(0))
    case.value(1, lambda body: body.z(0))
    case.default(lambda body: body.h(0))

circuit.switch(target, build_cases)
```

### 6. 事务语义

闭包式控制流 API 具有事务语义。也就是说，只有当回调函数成功完成后，对应分支体或控制流结构才会提交到线路中。如果回调过程中抛出异常，或者追加操作时发生校验失败，本次构造会回滚，不会在线路中留下半构造状态。

```python
circuit = Circuit(1)

try:
    circuit.if_(
        ClassicalExpr.bool_literal(True),
        lambda body: (_ for _ in ()).throw(RuntimeError("failed")),
    )
except RuntimeError:
    pass
```

---

## 低层控制流 IR

对于 IR 转换器、反序列化器、编译器测试或底层结构构造，也可以直接使用低层控制流对象。

### `ValueControlBody`

```python
ValueControlBody(operations: list[ValueOperation])
```

`ValueControlBody` 表示控制流体中的操作序列。

| 接口 | 说明 |
|---|---|
| `operations` | 控制流体内的操作序列。 |
| `__len__()` | 返回操作数量。 |
| `has_measurement()` | 判断该控制流体是否直接或递归包含测量。 |
| `reads_value(value)` | 判断该控制流体是否读取指定 `ClassicalValue`。 |

### `ValueSwitchCase`

```python
ValueSwitchCase(value: int, body: ValueControlBody)
```

`ValueSwitchCase` 表示 `switch` 中的一个精确整数匹配分支。`value` 是匹配值，`body` 是命中该分支时执行的控制流体。

### `ClassicalControlOp`

`ClassicalControlOp` 是低层控制流对象，可直接表示 `if`、`while`、`for`、`switch` 和跳转指令。

| 静态方法 | 说明 |
|---|---|
| `ClassicalControlOp.if_(condition, then_body, else_body=None)` | 构造 `if` 或 `if-else` 控制流。 |
| `ClassicalControlOp.while_(condition, body)` | 构造 `while` 循环。 |
| `ClassicalControlOp.for_uint(var, start, stop, step, body)` | 构造基于 `UInt` 变量的 `for` 循环。 |
| `ClassicalControlOp.switch(target, cases, default=None)` | 构造 `switch` 多分支选择。 |
| `ClassicalControlOp.break_()` | 构造 `break` 跳转。 |
| `ClassicalControlOp.continue_()` | 构造 `continue` 跳转。 |

常用属性如下：

| 属性 | 说明 |
|---|---|
| `kind` | 控制流类型，如 `"if"`、`"while"`、`"for"`、`"switch"`、`"break"`、`"continue"`。 |
| `condition` | `if` 或 `while` 条件。 |
| `then_body` / `else_body` | `if` 控制流的分支体。 |
| `body` | `while` 或 `for` 控制流主体。 |
| `var` / `start` / `stop` / `step` | `for` 循环相关组件。 |
| `target` / `cases` / `default` | `switch` 多分支相关组件。 |
| `has_measurement()` | 判断控制流内部是否包含测量。 |
| `reads_value(value)` | 判断控制流内部是否读取指定经典值。 |

---

## 验证规则

`Circuit.validate()` 会检查经典数据和控制流结构是否满足线路内部一致性要求。

校验通常覆盖以下规则：

- 经典变量和经典值必须属于当前线路；
- 测量值必须先定义后使用；
- 控制流作用域内定义的值不能逃逸到作用域外；
- `if` 和 `while` 的条件必须是 `Bool` 表达式；
- `for_uint` 的循环变量必须是 `UInt` 类型；
- `for_uint` 的 `start`、`stop` 和 `step` 必须与循环变量位宽一致；
- `switch` 的目标必须是 `UInt` 表达式；
- `break` 和 `continue` 必须位于合法作用域内；
- 手动构造的低层控制流对象必须与当前线路兼容。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(1)
circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

circuit.validate()
```

---

## 与矩阵转换的关系

包含测量、经典数据写入或控制流的线路通常不能直接表示为单个固定的幺正矩阵。原因在于，这类线路描述的是带有运行时经典信息、条件分支或循环结构的程序，而不是固定顺序的纯量子门序列。

```python
from cqlib import Circuit
from cqlib.circuit import CircuitError, ClassicalExpr

circuit = Circuit(1)
circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

try:
    circuit.to_matrix()
except CircuitError:
    print("control-flow circuits do not have one fixed unitary matrix")
```

如果只需要验证某个分支中的纯量子操作，可以将该分支体单独构造为静态 `Circuit`，先对分支体进行矩阵验证，再将其组合到控制流中。

```python
branch = Circuit(1)
branch.x(0)
branch_matrix = branch.to_matrix()

circuit = Circuit(1)
circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.compose(branch))
```

这种方式可以将“量子门级验证”和“控制流结构构造”分开处理，使测试和调试过程更加清晰。