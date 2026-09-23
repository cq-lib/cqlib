# Parameter

`cqlib.circuit.Parameter`  

```python
from cqlib import Parameter
```

`Parameter` 是 Cqlib 中用于表示电路参数和数学表达式的核心类型。它既可以表示一个确定的数值常量，也可以表示尚未赋值的符号变量，还可以通过算术运算和数学函数组合成更复杂的参数表达式。

在量子线路中，`Parameter` 常用于描述旋转角、延迟时间、全局相位、自定义符号门矩阵元素，以及变分算法中的待优化参数。通过参数化表达式，用户可以先构造一条具有符号参数的线路模板，再在后续流程中根据不同取值进行参数绑定、矩阵计算、模拟执行或编译处理。

---

## 创建参数

`Parameter` 支持从数值、符号名和表达式字符串创建参数对象，也提供了常用数学常量的构造方法。

```python
Parameter(value: int | float | str)
Parameter.from_expression(expr: str) -> Parameter
Parameter.pi() -> Parameter
Parameter.e() -> Parameter
```

| 输入 | 说明 |
| --- | --- |
| `Parameter(3.14)` | 创建数值常量参数。 |
| `Parameter("theta")` | 创建名为 `theta` 的符号参数。 |
| `Parameter("2 * theta + pi / 2")` | 从表达式字符串解析参数表达式。 |
| `Parameter.pi()` | 创建数学常量 `pi`。 |
| `Parameter.e()` | 创建数学常量 `e`。 |

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = 2 * theta + Parameter.pi() / 2
```

在上述示例中，`theta` 是一个自由符号，`expr` 是由符号参数、数值常量和数学常量组合得到的表达式。

---

## 支持的表达式能力

`Parameter` 支持的表达式元素包括：

- 数字常量和符号变量；
- 数学常量：`pi`、`e`；
- 算术运算符：`+`、`-`、`*`、`/`、`**`；
- 括号，用于控制表达式优先级；
- 常见函数：`sin`、`cos`、`tan`、`asin`、`acos`、`atan`、`exp`、`ln`、`log`、`sqrt`、`abs`、`sinh`、`cosh`、`tanh`、`floor`、`ceil`、`round`。

除表达式字符串外，`Parameter` 也提供了方法形式的数学函数，以便于直接构造表达式。

```python
from cqlib import Parameter

theta = Parameter("theta")

expr = theta.sin() + theta.cos()
root = (theta * theta + 1).sqrt()
```

---

## 算术运算符

`Parameter` 支持常见算术运算：

| 运算 | Python 写法 |
| --- | --- |
| 加法 | `a + b`、`1.0 + a` |
| 减法 | `a - b`、`1.0 - a` |
| 乘法 | `a * b`、`2.0 * a` |
| 除法 | `a / b`、`1.0 / a` |
| 幂运算 | `a ** b`、`a.pow(b)` |
| 取负 | `-a` |

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

expr = 2 * theta - phi / 3 + Parameter.pi()
```

每次算术运算都会返回一个新的 `Parameter` 对象，不会修改原有参数。因此，同一个符号参数可以安全地复用于多个表达式。

---

## 数学函数方法

`Parameter` 支持常用数学函数方法：

| 方法 | 说明 |
| --- | --- |
| `sin()` / `cos()` / `tan()` | 三角函数。 |
| `asin()` / `acos()` / `atan()` | 反三角函数。 |
| `exp()` / `ln()` / `log(base=None)` | 指数函数和对数函数。 |
| `sqrt()` / `abs()` | 平方根和绝对值。 |
| `sinh()` / `cosh()` / `tanh()` | 双曲函数。 |
| `floor()` / `ceil()` / `round()` | 取整相关函数。 |

示例：

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = (2 * x + y).sin().exp()
```

---

## 求值

`evaluate(bindings)` 用于将参数表达式计算为具体浮点数。

```python
evaluate(bindings: dict[str, float] | None = None) -> float
```

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = 2 * theta + 1

assert expr.evaluate({"theta": 0.5}) == 2.0
```

对于不含自由符号的常量表达式，可以直接求值：

```python
from cqlib import Parameter

value = (Parameter.pi() / 2).evaluate()
print(value)
```

---

## 符号信息与状态检查

`Parameter` 提供了一组查询接口，用于检查表达式中包含哪些自由符号，以及表达式是否为常量或特定数值。

| 接口 | 返回 | 说明 |
| --- | --- | --- |
| `symbols` | `list[str]` | 表达式中包含的所有唯一自由符号。 |
| `as_symbol()` | `str / None` | 当表达式正好是单个符号时，返回符号名；否则返回 `None`。 |
| `is_constant()` | `bool` | 当表达式不包含自由符号时返回 `True`。 |
| `is_exact_zero()` | `bool` | 当表达式在语法上就是精确 `0` 时返回 `True`。 |
| `is_zero()` | `bool` | 当表达式可求值且结果等于 `0` 时返回 `True`。 |
| `is_one()` | `bool` | 当表达式可求值且结果等于 `1` 时返回 `True`。 |

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")
expr = theta + phi

assert set(expr.symbols) == {"theta", "phi"}
assert theta.as_symbol() == "theta"
assert Parameter(1).is_one()
```

---

## 化简与规范化

`Parameter` 支持表达式化简和规范化。

```python
simplify() -> Parameter
canonicalized() -> Parameter
```

- `simplify()` 会返回代数化简后的新表达式。

```python
from cqlib import Parameter

x = Parameter("x")
expr = (x + 0).simplify()
```

- `canonicalized()` 返回 Cqlib 内部用于参数驻留、比较、去重或编译器处理的规范存储形式。

---

## 替换与代入

`replace()` 和 `substitute()` 用于对表达式中的符号进行替换。

```python
replace(symbol: str, param: Parameter) -> Parameter
substitute(bindings: dict[str, Parameter]) -> Parameter
```

示例：

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = x + 2
replaced = expr.replace("x", 3 * y)
substituted = replaced.substitute({"y": Parameter(0.5)})
```

`replace(symbol, param)` 用于替换单个符号；`substitute(bindings)` 用于一次性替换多个符号。二者都会返回新的 `Parameter` 表达式，不会修改原始对象。

---

## 求导

`derivative(var)` 用于对指定符号变量求导，并返回新的 `Parameter` 表达式。

```python
derivative(var: str) -> Parameter
```

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = theta.sin() + theta * theta

dtheta = expr.derivative("theta")
```

该接口处理的是经典参数表达式层面的符号求导。例如，它可以计算门角度表达式、全局相位表达式或符号矩阵元素对某个变量的导数。

---

## 等价判断

`Parameter` 提供等价判断接口，用于判断两个表达式是否可以被证明为相等。

```python
provably_equal(other: Parameter, tolerance: float = 1e-12) -> bool
provably_equal_modulo(
    other: Parameter,
    modulus: Parameter,
    tolerance: float = 1e-12,
) -> bool
```

示例：

```python
from cqlib import Parameter

theta = Parameter("theta")

expr1 = theta + 0
expr2 = theta

assert expr1.provably_equal(expr2)
```
