# 控制流

控制流用于描述量子线路中的条件分支、循环结构和多分支选择逻辑。与只按固定顺序执行量子门的普通线路不同，控制流可以在统一的线路表示中保留“在满足某个经典条件时执行某段线路”“按照计数变量重复执行某段线路”或“根据整数值选择不同分支”等结构化语义。

通过本节内容，您可以了解如何构造经典表达式，如何使用 `if_`、`if_else`、`while_`、`for_uint` 和 `switch` 等接口组织线路结构，以及如何在构造完成后进行基本校验和结构检查。

---

## 经典类型

`ClassicalType` 用于描述控制流中经典值的类型信息。通过显式的类型定义，您可以在构造线路时检查条件表达式、循环变量、分支目标和普通经典表达式之间的类型是否匹配。

常用的经典类型包括以下几类：

| 类型构造 | 说明 | 典型用途 |
|---|---|---|
| `ClassicalType.bit()` | 单个 bit，取值为 `0` 或 `1` | 单比特经典结果、位表达式 |
| `ClassicalType.bool()` | 逻辑布尔值 | `if`、`while` 条件 |
| `ClassicalType.uint(width)` | 指定位宽无符号整数 | 计数器、循环变量、`switch` 目标 |
| `ClassicalType.bit_vec(width)` | 指定位宽 bit 向量 | 位向量表达式、位打包、位拼接结果 |

```python
from cqlib.circuit import ClassicalType

bit_ty = ClassicalType.bit()
bool_ty = ClassicalType.bool()
u3_ty = ClassicalType.uint(3)
bits_ty = ClassicalType.bit_vec(4)

print(bit_ty.width)    # 1
print(u3_ty.width)     # 3
```

类型对象还可以用于创建常用字面量表达式。下列示例创建了 3 位无符号整数类型下的 `0` 和 `1` 字面量：

```python
zero = ClassicalType.uint(3).zero_literal()
one = ClassicalType.uint(3).one_literal()
```

---

## 经典变量

`Circuit.var(type)` 用于在线路中分配一个可变经典变量。每个经典变量都具有明确的类型信息，如 `Bool`、`UInt` 或 `BitVec`，并且会绑定到创建它的 `Circuit` 对象。

经典变量通常用于在控制流中保存和引用经典状态，例如记录分支标志、维护循环计数器，或作为 `switch` 分支判断的目标值。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)

flag = c.var(ClassicalType.bool())
counter = c.var(ClassicalType.uint(4))

c.store(flag, ClassicalExpr.bool_literal(True))
c.store(counter, ClassicalExpr.uint_literal(4, 3))

print(flag.index)
print(flag.ty)
print(flag.circuit_id == c.id)
```

`store(target, value)` 用于向经典变量写入一个经典表达式。写入时，`value` 的类型必须与 `target.ty` 兼容，否则会破坏线路中的经典数据语义。

---

## 经典表达式

`ClassicalExpr` 用于描述控制流中的经典计算逻辑。它可以表示字面量、变量读取、逻辑运算、比较运算、条件选择、位抽取和位拼接等表达式结构。

您可以将 `ClassicalExpr` 理解为一类“经典表达式语法树”。它本身不立即执行计算，而是记录经典计算关系，并作为线路结构的一部分参与后续校验、转换和编译处理。

### 1. 字面量

字面量用于直接构造固定的经典值，例如布尔值、单个 bit、无符号整数或 bit 向量。

```python
from cqlib.circuit import ClassicalExpr

truth = ClassicalExpr.bool_literal(True)
bit_one = ClassicalExpr.bit_literal(True)
u3_five = ClassicalExpr.uint_literal(3, 5)
bits = ClassicalExpr.bit_vec_literal(4, 0b1010)
```

其中，`uint_literal(width, value)` 和 `bit_vec_literal(width, value)` 需要显式指定位宽。位宽是经典类型的一部分，会影响后续比较、循环和拼接操作的合法性。

### 2. 从变量读取

经典变量本身是一个可写入、可引用的句柄。如果需要在表达式中读取变量的当前值，可以使用 `ClassicalExpr.var(var)` 将变量转换为表达式，也可以直接调用变量句柄提供的 `expr()` 方法。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)

flag = c.var(ClassicalType.bool())
flag_expr = ClassicalExpr.var(flag)

# 等价的便捷写法
same_flag_expr = flag.expr()
```

### 3. 逻辑运算

`ClassicalExpr` 支持常用逻辑运算。运算符 `~`、`&`、`|`、`^` 分别对应逻辑非、与、或和异或。

```python
from cqlib.circuit import ClassicalExpr

a = ClassicalExpr.bool_literal(True)
b = ClassicalExpr.bool_literal(False)

expr = a | b
simplified = expr.simplified() 
print(simplified.is_bool_true()) # True
```

### 4. 比较

若要构造比较条件，应使用 `ClassicalExpr` 提供的静态方法。

```python
from cqlib.circuit import ClassicalExpr

x = ClassicalExpr.uint_literal(3, 2)
y = ClassicalExpr.uint_literal(3, 5)

cond1 = ClassicalExpr.lt(x, y)
cond2 = ClassicalExpr.equal(x, y)
cond3 = ClassicalExpr.not_equal(x, y)
cond4 = ClassicalExpr.ge(y, x)
```

支持的比较方法包括：

- `equal(lhs, rhs)`
- `not_equal(lhs, rhs)`
- `lt(lhs, rhs)`
- `le(lhs, rhs)`
- `gt(lhs, rhs)`
- `ge(lhs, rhs)`

比较结果通常是 `Bool` 表达式，可以直接作为 `if_`、`if_else` 或 `while_` 的条件。

### 5. 选择、抽取和拼接

除基础逻辑和比较运算外，`ClassicalExpr` 还支持条件选择、位抽取和位拼接等操作，可用于构造更复杂的经典控制逻辑。

```python
from cqlib.circuit import ClassicalExpr

cond = ClassicalExpr.bool_literal(True)
a = ClassicalExpr.uint_literal(3, 1)
b = ClassicalExpr.uint_literal(3, 7)

selected = ClassicalExpr.select(cond, a, b)

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

其中：
- `select(cond, a, b)` 表示根据布尔条件在两个表达式之间进行选择：当 `cond` 为真时选择 `a`，否则选择 `b`；
- `extract_bit()` 用于从 `BitVec` 中抽取单个 bit，`extract_bits()` 用于抽取连续的多位片段；
- `pack_bits(bits)` 用于将多个 Bit 表达式打包为一个 BitVec 表达式。

---

## `if_` 条件分支

`if_(condition, body)` 用于在线路中追加一个不包含 `else` 分支的条件控制结构。该接口表示当给定条件满足时，执行分支体中的线路操作；当条件不满足时，跳过该分支体。

其中：
- `condition` 必须是 `Bool` 类型的 `ClassicalExpr` 表达式，用于描述条件判断逻辑；
- `body` 是一个回调函数，用于构造条件满足时需要执行的线路内容。回调函数会接收一个临时线路构造器，您可以在其中继续追加量子门或其他受支持的操作。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

c.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(0),
)
```

对于只包含一条操作的简单分支，您可以使用 `lambda` 写法，使代码更加简洁。对于包含多条操作的分支体，建议使用普通函数定义，以提高代码可读性和后续维护性。

```python
def then_body(body):
    body.h(0)
    body.x(0)

c.if_(ClassicalExpr.bool_literal(True), then_body)
```

控制流回调采用原子化构造方式。也就是说，只有当回调函数完整执行成功后，分支体才会被提交到线路中；如果回调执行过程中抛出异常，Cqlib 会放弃本次分支体构造，避免在线路中留下不完整或不一致的控制流结构。

```python
def failing_body(body):
    body.x(0)
    raise RuntimeError("construction failed")

try:
    c.if_(ClassicalExpr.bool_literal(True), failing_body)
except RuntimeError:
    pass
```

---

## `if_else` 条件分支

`if_else(condition, then_body, else_body)` 用于在线路中构造带有 `else` 分支的条件控制结构。该接口表示当 `condition` 条件为真时，执行 `then_body` 分支；当条件为假时，执行 `else_body` 分支。两个分支均通过回调函数构造，并作为同一个条件控制结构的一部分保存在线路中。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

c.if_else(
    ClassicalExpr.bool_literal(False),
    lambda then_body: then_body.x(0),
    lambda else_body: else_body.z(0),
)

control = c[0].instruction.classical_control
print(control.kind)  # if
```

与 `if_` 类似，`if_else` 的两个分支也采用原子化构造方式。只有当两个回调函数都成功完成构造后，相关控制流结构才会被提交到线路中；如果任一分支在构造过程中抛出异常，本次控制流构造会被放弃，避免在线路中留下不完整或不一致的分支结构。

---

## `while_` 循环

`while_(condition, body)` 用于在线路中构造基于布尔条件的循环结构。该接口表示当 `condition` 条件满足时，执行循环体 `body` 中定义的操作；每轮循环开始前都会根据条件表达式判断是否继续执行。

其中：
- `condition` 必须是 `Bool` 类型的 `ClassicalExpr` 表达式；
- `body` 是一个回调函数，用于描述循环体中的线路操作。回调函数会接收一个临时线路构造器，用户可以在其中追加量子门、控制流跳转或其他受支持的操作。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

c.while_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.break_loop(),
)
```

在循环体中，可以使用以下跳转操作控制循环执行流程：

- `break_loop()`：退出最近一层循环；
- `continue_loop()`：跳过当前循环体中剩余操作，进入最近一层循环的下一轮判断。

```python
def loop_body(body):
    body.x(0)
    body.continue_loop()

c = Circuit(1)
c.while_(ClassicalExpr.bool_literal(True), loop_body)
```

---

## `for_uint` 循环

`for_uint(var, start, stop, step, body)` 用于在线路中构造基于无符号整数变量的循环结构，循环范围为 `[start, stop)`：从 `start` 开始，每轮按照 `step` 递增，当循环变量达到或超过 `stop` 时结束循环。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)
loop_var = c.var(ClassicalType.uint(3))

def body(builder, index_expr):
    # index_expr 是读取 loop_var 的 UInt 表达式
    builder.rx(0, 0.25)

c.for_uint(
    loop_var,
    ClassicalExpr.uint_literal(3, 0),
    ClassicalExpr.uint_literal(3, 3),
    ClassicalExpr.uint_literal(3, 1),
    body,
)
```

---

## `switch` 多分支选择

`switch(target, build)` 用于在线路中构造基于整数值匹配的多分支控制结构。该接口会根据 `target` 表达式的取值，在多个已注册分支中选择一个进行执行。`build` 是一个回调函数，用于注册不同的整数匹配分支以及可选的默认分支。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

def build_switch(builder):
    builder.value(0, lambda body: body.x(0))
    builder.value(1, lambda body: body.z(0))
    builder.default(lambda body: body.h(0))

c.switch(ClassicalExpr.uint_literal(2, 1), build_switch)
```

---

## 校验与作用域

与普通线性线路相比，控制流结构对线路内部一致性的要求更高。因此，在构造包含控制流的线路后，建议调用 `validate()` 对线路进行结构校验。

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)
c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

c.validate()
```

---

## 下一步

- [中间表示](../1_ir/0_overview.md)：掌握 Circuit 与 IR 之间的双向转换流程，支持线路持久化、跨工具链交换和后续编译处理。
- [QCIS 支持](../1_ir/1_qcis.md)：将 Cqlib 线路导出为 QCIS 指令或从 QCIS 文件加载线路。
- [OpenQASM 2.0 支持](../1_ir/2_qasm2.md)：实现 Cqlib 线路与 OpenQASM 2.0 格式之间的导入导出。