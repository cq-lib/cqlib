# Parameter

`cqlib_core::circuit::Parameter`

```rust
use cqlib_core::circuit::Parameter;
```

`Parameter` 是 Rust core 中用于表示线路参数表达式的核心类型。它既可以表示一个确定的数值常量，也可以表示符号变量，还可以表示由算术运算和数学函数组合而成的表达式。

---

## 创建参数

`Parameter` 支持从符号名、数值常量、表达式字符串和内置数学常量创建。

| 接口 | 说明 |
| --- | --- |
| `Parameter::symbol(name)` | 创建符号变量。 |
| `Parameter::pi()` | 创建数学常量 `π`。 |
| `Parameter::e()` | 创建自然常数 `e`。 |
| `Parameter::new(expr)` | 从底层表达式对象创建参数，通常用于内部或高级场景。 |
| `str::parse::<Parameter>()` | 从字符串解析参数表达式。 |
| `Parameter::from(f64)` | 从浮点数创建数值常量。 |

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let pi = Parameter::pi();
let numeric = Parameter::from(0.25_f64);

let expr: Parameter = "2 * theta + pi / 2".parse()?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

在上述示例中，`theta` 表示一个自由符号，`pi` 和 `numeric` 表示常量参数，`expr` 则是由字符串解析得到的复合表达式。表达式字符串会被解析为数学表达式；若语法无效或包含不支持的结构，会返回 `ParameterError`，而不会被静默当作普通符号处理。

---

## 算术运算

`Parameter` 支持常见算术运算：

- 加法：`+`
- 减法：`-`
- 乘法：`*`
- 除法：`/`
- 一元取负：`-expr`

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let phi = Parameter::symbol("phi");

let expr = theta.clone() * 2.0 + phi.clone() / 2.0 - Parameter::pi();
let neg = -phi;
```

每次运算都会生成新的参数表达式，不会修改原有 `Parameter`。因此，同一个符号参数可以安全地复用于多个表达式中。

---

## 数学函数

除基础算术运算外，`Parameter` 还提供了一组常用数学函数，用于构造更复杂的参数表达式。

| 方法 | 说明 |
| --- | --- |
| `abs()` / `sqrt()` | 绝对值和平方根。 |
| `exp()` / `ln()` / `log(base)` | 指数函数、自然对数和指定底数对数。 |
| `sin()` / `cos()` / `tan()` | 三角函数。 |
| `asin()` / `acos()` / `atan()` | 反三角函数。 |
| `sinh()` / `cosh()` / `tanh()` | 双曲函数。 |
| `floor()` / `ceil()` / `round()` | 取整相关函数。 |
| `pow(exp)` | 幂运算。 |

---

## 求值

```rust
pub fn evaluate(
    &self,
    bindings: &Option<HashMap<&str, f64>>,
) -> Result<f64, ParameterError>
```

`evaluate()` 用于将参数表达式计算为具体的浮点数结果。对于包含自由符号的表达式，需要通过 `bindings` 提供每个符号对应的数值；只有当表达式中的所有自由符号都完成绑定后，才能成功求值。

---

## 符号信息与状态检查

`Parameter` 提供了一组接口，用于检查表达式中的自由符号、常量属性和特殊数值状态。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `get_symbols()` | `HashSet<String>` | 返回表达式中的自由符号集合。 |
| `as_symbol()` | `Option<String>` | 当表达式正好是单个符号时，返回该符号名。 |
| `is_constant()` | `bool` | 判断表达式是否不含自由符号。 |
| `is_exact_zero()` | `Result<bool, ParameterError>` | 判断表达式是否精确表示为 0。 |
| `is_zero()` | `bool` | 在可求值情况下判断是否等于 0。 |
| `is_one()` | `bool` | 在可求值情况下判断是否等于 1。 |

---

## 化简与规范化

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `simplify()` | `Result<Parameter, ParameterError>` | 对表达式进行代数化简。 |
| `canonicalized()` | `Result<Parameter, ParameterError>` | 返回用于参数驻留、比较和去重的规范形式。 |

```rust
use cqlib_core::circuit::Parameter;

let x = Parameter::symbol("x");
let expr = x.clone() + 0.0;

let simplified = expr.simplify()?;
let canonical = simplified.canonicalized()?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## 替换与代入

`Parameter` 支持将表达式中的符号替换为另一个参数表达式。

| 方法 | 说明 |
| --- | --- |
| `replace(symbol, param)` | 替换单个符号。 |
| `substitute_many(bindings)` | 同时替换多个符号。 |

```rust
use cqlib_core::circuit::Parameter;

let x = Parameter::symbol("x");
let y = Parameter::symbol("y");

let expr = x + 2.0;
let replaced = expr.replace("x", y.clone() * 3.0);
```

---

## 求导

```rust
pub fn derivative(&self, var: &str) -> Result<Self, ParameterError>
```

`derivative(var)` 用于对指定符号变量进行符号求导，并返回新的 `Parameter` 表达式。

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let expr = theta.clone().sin() + theta.clone() * theta;

let dtheta = expr.derivative("theta")?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## 等价判断

| 方法 | 说明 |
| --- | --- |
| `provably_equal(other, tolerance)` | 保守判断两个表达式是否相等。 |
| `provably_equal_modulo(other, modulus, tolerance)` | 在给定周期模意义下进行保守相等判断。 |

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let expr1 = theta.clone() + Parameter::pi();
let expr2 = theta - Parameter::pi();

assert!(expr1.provably_equal_modulo(
    &expr2,
    &(2.0 * Parameter::pi()),
    1e-12,
));

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```
