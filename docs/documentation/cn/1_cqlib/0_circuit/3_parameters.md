# 参数系统

`Parameter` 是 Cqlib 中用于表示数值参数、符号变量和参数表达式的核心类型，常用于描述量子门角度、线路全局相位、自定义符号门以及符号矩阵中的可调元素。通过 `Parameter`，您可以先构造具有符号参数的线路模板，再从后续算法流程中根据不同参数取值进行绑定、求值和验证。

`Parameter` 不仅可以表示一个确定的常数，也可以表示一个尚未赋值的符号变量，还可以通过算术运算和数学函数组合成更复杂的表达式。这使得 Cqlib 能够在保持线路结构不变的前提下，对同一条参数化线路执行多次参数绑定、参数扫描、优化迭代或符号分析。

---

## 创建 `Parameter`

`Parameter` 可以通过符号名、数值、表达式字符串或内置常量构造器创建。不同创建方式适用于不同场景：
- 符号名适合定义待优化参数；
- 数值适合表示确定角度；
- 表达式字符串适合从配置文件或外部输入中解析参数表达式；
- 常量构造器则用于统一表示 `pi` 和 `e` 等数学常量。

### 1. 从符号名创建

最常见的方式是使用字符串创建符号参数。符号参数表示一个尚未赋值的变量，通常用于参数化量子门、全局相位、符号矩阵或变分算法中的待优化参数。

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

print(theta.symbols)  # ['theta']
print(theta.as_symbol())  # theta
```

需要注意的是，传入的字符串会按照参数表达式进行解析。因此，如果字符串不符合参数表达式语法，Cqlib 会抛出 `ParameterError`，以提示用户检查变量名或表达式格式。

### 2. 从数值创建

如果参数值已经确定，可以直接使用整数或浮点数创建数值参数。数值参数通常用于固定角度的旋转门、固定全局相位，或在测试与示例中构造确定的参数对象。

```python
from cqlib import Parameter

a = Parameter(1)
b = Parameter(0.25)

print(a.is_constant())     # True
print(b.evaluate({}))      # 0.25
```

由数值创建的 `Parameter` 是常量表达式，不依赖任何符号绑定，因此可以直接求值。

### 3. 从表达式字符串创建

当参数表达式来自配置文件、用户输入、序列化数据或外部工具时，可以使用 `Parameter.from_expression()` 从字符串解析表达式。

```python
from cqlib import Parameter

expr = Parameter.from_expression("2*x + sin(y) + pi/2")

print(expr.symbols)     # ['x', 'y']
print(expr.evaluate({"x": 0.1, "y": 0.2}))
```

表达式字符串可以包含变量、常量、算术运算和常用数学函数。解析完成后，得到的仍然是一个 `Parameter` 对象，可以继续用于量子门参数、全局相位、符号矩阵或其他参数表达式组合中。

当前支持的表达式元素包括：

- 数字；
- 常量 `pi`、`e`；
- 变量名；
- 运算符 `+`、`-`、`*`、`/`、`**`；
- 括号；
- 函数，如 `sin`、`cos`、`tan`、`exp`、`sqrt`、`ln`、`log` 等。

### 4. 常量构造器

对于常用数学常量，Cqlib 提供了专门的常量构造器，例如 `Parameter.pi()` 和 `Parameter.e()`。使用这些构造器可以避免手动输入近似浮点数，从而提高表达式的可读性和一致性。

```python
from cqlib import Parameter

pi = Parameter.pi()
e = Parameter.e()

print(pi.evaluate({}))
print(e.evaluate({}))
```

---

## 算术表达式

`Parameter` 支持常见的 Python 算术运算，您可以像组合普通数值一样组合符号参数、常量和参数表达式。通过这些运算，可以方便地构造门角度、全局相位、符号矩阵元素或算法中需要复用的参数关系。

需要注意的是，`Parameter` 表达式是不可变对象。每次算术运算都会返回一个新的 `Parameter`，不会修改原有参数对象。因此，您可以安全地复用同一个符号参数，并基于它构造多个不同的表达式。

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

expr1 = theta + phi
expr2 = 2 * theta - phi / 3
expr3 = -(theta + 1)
expr4 = theta ** 2
expr5 = theta.pow(Parameter(0.5))

print(expr1)
print(expr2)
print(expr3)
print(expr4)
print(expr5)
```

`Parameter` 也支持数值与符号参数混合运算。数字可以出现在表达式的左侧或右侧，Cqlib 会自动将其转换为对应的常量参数表达式。


---

## 数学函数

除基础算术运算外，`Parameter` 还提供了一组常用数学函数，用于构造更复杂的参数表达式。这些函数可以作用于单个符号参数，也可以作用于由多个参数组合而成的复合表达式，适合描述参数化门角度、符号矩阵元素、算法中的参数变换关系以及需要求导或化简的表达式。

常用数学函数如下：

| 方法 | 说明 |
|---|---|
| `sin()` / `cos()` / `tan()` | 三角函数 |
| `asin()` / `acos()` / `atan()` | 反三角函数 |
| `sinh()` / `cosh()` / `tanh()` | 双曲函数 |
| `exp()` | 指数函数 |
| `ln()` | 自然对数 |
| `log(base=None)` | 对数；当 `base=None` 时，按自然对数处理 |
| `sqrt()` | 平方根 |
| `abs()` | 绝对值 |
| `floor()` / `ceil()` / `round()` | 取整相关函数 |

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = (2 * x + y).sin().exp()
grad = expr.derivative("x")

print(expr)
print(grad)
```

上述数学函数支持链式调用，也可以与加减乘除、幂运算等算术操作组合使用。由于每次函数调用都会返回新的 `Parameter` 对象，原始参数不会被修改，因此您可以安全地复用已有符号参数构造多种不同表达式。

---

## 求值

`evaluate(bindings)` 用于将 `Parameter` 表达式计算为具体的浮点数结果。对于包含自由符号的表达式，您需要通过 `bindings` 参数提供每个符号对应的数值；只有当表达式中的所有自由符号都完成绑定后，Cqlib 才能进行数值求值。

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = (2 * theta + 1).sin()

value = expr.evaluate({"theta": 0.5})
print(value)
```

如果表达式本身不包含自由符号，例如由常量或数学常量构成，则可以传入空字典，也可以省略 `bindings` 参数。

```python
from cqlib import Parameter

expr = Parameter.pi() / 2

print(expr.evaluate({}))
print(expr.evaluate())
```

需要注意的是，`evaluate()` 要求表达式能够在实数域内得到有效结果。如果存在以下情况，Cqlib 通常会抛出 `ParameterError`：
- 表达式中存在未绑定的自由符号；
- 绑定字典缺少必要的参数名；
- 绑定值包含 `NaN` 或无穷大等非法数值；
- 表达式在实数域内无法求值。

因此，在将参数表达式用于线路矩阵计算、参数绑定或优化器结果验证前，建议先确认所有自由符号均已正确绑定，并确保绑定值位于合法的数值范围内。

---

## 化简

`simplify()` 用于对 `Parameter` 表达式进行代数化简，并返回一个新的参数表达式。该接口常用于减少冗余表达式、规范化参数形式，以及在参数绑定、符号矩阵构造或线路分析前整理表达式结构。

需要注意的是，`simplify()` 不会修改原始表达式，而是生成新的 `Parameter` 对象。这样可以保证原始参数表达式在多个线路模板或算法流程中被安全复用。

```python
from cqlib import Parameter

theta = Parameter("theta")

examples = [
    theta + 0,
    theta * 1,
    theta - theta,
    (Parameter(0)).sin(),
    (Parameter(1)).ln(),
]

for expr in examples:
    print(expr, "=>", expr.simplify())
```

---

## 求导

`derivative(var)` 用于对 `Parameter` 表达式中的指定符号变量进行符号求导，并返回一个新的 `Parameter` 表达式。该接口处理的是经典参数表达式层面的求导关系，适合用于分析门角度、全局相位、符号矩阵元素或其他参数组合关系对某个变量的依赖。

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

expr = (2 * theta + phi).sin()

d_theta = expr.derivative("theta").simplify()
d_phi = expr.derivative("phi").simplify()

print(d_theta)
print(d_phi)
print(d_theta.evaluate({"theta": 0.3, "phi": 0.4}))
```

需要注意的是，`Parameter.derivative()` 只处理经典符号表达式本身，并不直接计算量子线路关于某个目标函数的梯度。线路整体梯度通常还取决于量子态演化、测量可观测量、模拟器或后端执行结果，以及参数位移规则、伴随法或其他量子梯度计算方法。

---

## 替换与代入

在参数化线路和符号表达式处理中，有时需要将表达式中的某个符号替换为另一个参数表达式。这里，Cqlib 提供了 `replace()` 和 `substitute()` 两类接口，用于对 `Parameter` 表达式进行符号级替换。

### 1. `replace()`

`replace()` 用于替换表达式中的单个符号。您需要指定待替换的符号名称，并提供替换后的 `Parameter` 表达式。

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = x + 2
new_expr = expr.replace("x", 3 * y)

print(new_expr)
```

### 2. `substitute()`

`substitute()` 用于同时替换多个符号，适合在一次操作中完成批量符号代入。您可以通过字典指定多个符号与替换表达式之间的对应关系。

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")
z = Parameter("z")

expr = x * y + 1
new_expr = expr.substitute({"x": z + 1, "y": Parameter(2)})

print(new_expr.simplify())
```

需要特别注意的是，`replace()` 和 `substitute()` 执行的是符号表达式替换，替换值应为 `Parameter` 对象或可转换为 `Parameter` 的表达式，而非直接用于数值求值的 `float` 结果。

---

## 等价性判断

在参数化线路、符号矩阵和编译优化过程中，有时需要判断两个 `Parameter` 表达式是否表示相同的数学含义。这里，Cqlib 提供了较为保守的等价判断接口。所谓“保守”，是指当接口返回 `True` 时，可以认为两个表达式在当前规则和容差范围内可证明等价；当接口返回 `False` 时，并不一定表示两个表达式必然不等价，也可能只是当前符号规则无法证明它们相等。

```python
from cqlib import Parameter

x = Parameter("x")

a = (x + 0).simplify()
b = x

print(a.provably_equal(b))
```

| 方法 | 说明 |
|---|---|
| `provably_equal(other, tolerance=1e-12)` | 在给定容差范围内，保守判断两个表达式是否可以证明相等 |
| `provably_equal_modulo(other, modulus, tolerance=1e-12)` | 在指定模数下判断两个表达式是否等价 |

需要注意的是，等价性判断通常依赖表达式化简、代数规则和数值容差。对于结构较复杂的表达式，建议先调用 `simplify()` 进行化简，再执行等价性判断。

---

## 状态检查方法

`Parameter` 提供了一组状态检查方法，用于判断表达式是否包含自由符号、是否为常量、是否等于特定数值，或是否可以作为单个符号处理。这些方法常用于参数校验、编译优化、线路规范化、参数绑定前检查以及符号矩阵处理等场景。

通过这些接口，您可以在进入矩阵计算、线路转换或后端执行之前，提前确认参数表达式是否满足当前流程的要求。例如，在执行数值矩阵计算前，通常需要确认线路中不存在未绑定符号；在编译优化中，可能需要识别全局相位是否为零，或判断某个旋转角度是否可以被消去。

常用状态检查方法如下：

| 方法 | 说明 |
|---|---|
| `symbols` | 返回表达式中包含的自由符号列表 |
| `canonicalized()` | 返回用于参数驻留和内部比较的规范化表达形式 |
| `is_exact_zero()` | 判断表达式是否精确表示常数 `0` |
| `is_constant()` | 判断表达式是否不包含自由变量 |
| `is_zero()` | 判断表达式在当前条件下是否可求值为 `0` |
| `is_one()` | 判断表达式在当前条件下是否可求值为 `1` |
| `as_symbol()` | 如果表达式恰好是单个符号，则返回该符号名；否则返回 `None` |

```python
from cqlib import Parameter

theta = Parameter("theta")
zero = Parameter(0)

print(theta.is_constant())       # False
print(theta.as_symbol())         # theta
print(zero.is_exact_zero())      # True
print((theta - theta).simplify().is_exact_zero())
```

---

## 在线路中使用参数

在 Cqlib 中，参数化量子门可以接受普通数值参数或 `Parameter` 表达式作为输入。数值参数适合构造角度已经确定的线路；`Parameter` 表达式则适合构造参数化线路模板，并在后续算法流程中进行参数绑定、参数扫描或优化迭代。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(2)
c.rx(0, theta)
c.ry(1, phi)
c.rzz(0, 1, 2 * theta - phi)

print(c.parameters)
print(c.symbols)
```

---

## 参数绑定

参数绑定是指将线路中的符号参数替换为具体数值，生成可以进一步用于矩阵计算、模拟执行、IR 导出或后端运行的线路。Cqlib 通过 `assign_parameters()` 完成参数绑定。该接口接受一个由参数名到数值的映射，如 `dict[str, float]`，并返回绑定后的新线路。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

c = Circuit(1)
c.rx(0, theta)
c.rz(0, theta + phi)

bound = c.assign_parameters({"theta": 0.25, "phi": 0.5})

print(bound[0].params)
print(bound[1].params)
print(bound.parameters)
```

Cqlib 也支持部分参数绑定。如果绑定字典中只提供了部分符号的取值，则已经绑定的符号会被替换为数值，未绑定的符号会继续保留在线路中。

```python
partial = c.assign_parameters({"theta": 0.25})

print(partial[0].params)  # [0.25]
print(partial[1].params)  # [Parameter("0.25 + phi")]
print(partial.symbols)    # ['phi']
```

---

## 与矩阵转换的关系

参数化线路在进行矩阵转换时，分为数值矩阵和符号矩阵两种情况：
- 数值矩阵要求线路中的所有参数都已经绑定为具体数值；
- 符号矩阵则可以保留参数表达式，用于小规模线路的符号分析和验证。

### 1. 数值矩阵

`to_matrix()` 用于将量子线路转换为数值矩阵。由于数值矩阵中的每个元素都必须是确定的复数值，因此在调用 `to_matrix()` 之前，线路中的所有符号参数都需要完成数值绑定。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

c = Circuit(1)
c.rx(0, theta)

bound = c.assign_parameters({"theta": 0.3})
matrix = bound.to_matrix()
```

### 2. 符号矩阵

如果您希望在线路矩阵中保留参数结构，可以使用 `to_symbolic_matrix()`。该接口会生成符号矩阵，其中矩阵元素可以包含 `Parameter` 表达式，适合用于小规模参数化线路验证、符号门定义和编译规则检查。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

c = Circuit(1)
c.rz(0, theta)

symbolic = c.to_symbolic_matrix()
print(symbolic.shape)
print(symbolic.symbols)

numeric = symbolic.evaluate({"theta": 0.5})
```

---

## 在自定义符号门中使用参数

Cqlib 支持在自定义符号门中使用参数。通过 `SymbolicComplex` 和 `SymbolicMatrix`，您可以定义包含 `Parameter` 表达式的矩阵，并将该矩阵作为 `UnitaryGate` 的符号矩阵表示。

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix, UnitaryGate

theta = Parameter("theta")

matrix = SymbolicMatrix(
    [
        [SymbolicComplex.one(), SymbolicComplex.zero()],
        [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
    ]
)

gate = UnitaryGate("SymbolicPhase", 1, num_params=1).with_symbolic_matrix(
    matrix,
    ["theta"],
)

c = Circuit(1)
c.append_unitary_gate(gate, [0], [0.25])
```

需要注意的是，`with_symbolic_matrix(matrix, params)` 中的 `params` 用于定义该自定义门的参数顺序。在线路中调用 `append_unitary_gate()` 时，传入的参数会按照这一顺序与符号矩阵中的参数名称进行绑定。

---

## 下一步

- [线路分析与转换](4_circuit_analysis.md)：使用反演、分解、矩阵转换和操作检查等工具。
- [控制流](5_control_flow.md)：使用测量结果、经典变量和结构化控制流构造动态线路。
- [中间表示](../1_ir/0_overview.md)：掌握 Circuit 与 IR 之间的双向转换流程。

