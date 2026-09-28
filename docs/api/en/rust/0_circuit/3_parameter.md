# Parameter

`cqlib_core::circuit::Parameter`

```rust
use cqlib_core::circuit::Parameter;
```

`Parameter` is the core type representing circuit parameter expressions in the Rust core. It can represent a definite numeric constant, a symbolic variable, or an expression composed of arithmetic operations and mathematical functions.

---

## Creating parameters

`Parameter` supports creation from symbol names, numeric constants, expression strings and built-in mathematical constants.

| Interface | Description |
| --- | --- |
| `Parameter::symbol(name)` | Create a symbolic variable. |
| `Parameter::pi()` | Create the mathematical constant `π`. |
| `Parameter::e()` | Create the natural constant `e`. |
| `Parameter::new(expr)` | Create a parameter from an underlying expression object, usually for internal or advanced scenarios. |
| `as_expr()` | Read a reference to the underlying expression object. |
| `into_expr()` | Consume the parameter and take out the underlying expression object. |
| `str::parse::<Parameter>()` | Parse a parameter expression from a string. |
| `Parameter::from(f64)` | Create a numeric constant from a floating-point number. |
| `Parameter::default()` | Create the default parameter representing the integer `0`. |

Numeric constants also support conversion from `f32`, `u32` and `i32`. `NaN`, positive infinity and negative infinity of `f64` and `f32` are not finite values, and construction fails directly when they are passed in, so filter them out before the call.

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let pi = Parameter::pi();
let numeric = Parameter::from(0.25_f64);

let expr: Parameter = "2 * theta + pi / 2".parse()?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

In the example above, `theta` is a free symbol, `pi` and `numeric` are constant parameters, and `expr` is a composite expression obtained by parsing a string. An expression string is parsed as a mathematical expression; if the syntax is invalid or it contains unsupported structures, `ParameterError` is returned instead of silently treating it as an ordinary symbol.

---

## Arithmetic operations

`Parameter` supports common arithmetic operations:

- Addition: `+`
- Subtraction: `-`
- Multiplication: `*`
- Division: `/`
- Unary negation: `-expr`

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let phi = Parameter::symbol("phi");

let expr = theta.clone() * 2.0 + phi.clone() / 2.0 - Parameter::pi();
let neg = -phi;
```

Every operation produces a new parameter expression and does not modify the original `Parameter`. Therefore the same symbolic parameter can safely be reused in multiple expressions.

The operations above support four ownership combinations: `Parameter` with `Parameter`, `&Parameter` with `&Parameter`, `Parameter` with `&Parameter`, and `&Parameter` with `Parameter`. Mixed use of `Parameter` with `f64`, `f32`, `i32` and `u32` is also supported in both directions. Unary negation is available for both `Parameter` and `&Parameter`.

---

## Mathematical functions

Besides the basic arithmetic operations, `Parameter` also provides a set of common mathematical functions for constructing more complex parameter expressions.

| Method | Description |
| --- | --- |
| `abs()` / `sqrt()` | Absolute value and square root. |
| `exp()` / `ln()` / `log(base)` | Exponential function, natural logarithm and logarithm with a given base. |
| `sin()` / `cos()` / `tan()` | Trigonometric functions. |
| `asin()` / `acos()` / `atan()` | Inverse trigonometric functions. |
| `sinh()` / `cosh()` / `tanh()` | Hyperbolic functions. |
| `floor()` / `ceil()` / `round()` | Rounding-related functions. |
| `pow(exp)` | Exponentiation. |

---

## Evaluation

```rust
pub fn evaluate(
    &self,
    bindings: &Option<HashMap<&str, f64>>,
) -> Result<f64, ParameterError>
```

`evaluate()` computes a parameter expression into a concrete floating-point result. For an expression containing free symbols, supply the value of each symbol through `bindings`; evaluation succeeds only after all free symbols in the expression are bound.

---

## Symbol information and state checks

`Parameter` provides a set of interfaces for inspecting the free symbols, constant properties and special numeric states of an expression.

| Method | Returns | Description |
| --- | --- | --- |
| `get_symbols()` | `HashSet<String>` | Return the set of free symbols in the expression. |
| `as_symbol()` | `Option<String>` | When the expression is exactly a single symbol, return that symbol name. |
| `is_constant()` | `bool` | Determine whether the expression contains no free symbols. |
| `is_exact_zero()` | `Result<bool, ParameterError>` | Determine whether the expression is exactly 0. |
| `is_zero()` | `bool` | Determine whether it equals 0 when it can be evaluated. |
| `is_one()` | `bool` | Determine whether it equals 1 when it can be evaluated. |

---

## Simplification and canonicalization

| Method | Returns | Description |
| --- | --- | --- |
| `simplify()` | `Result<Parameter, ParameterError>` | Perform algebraic simplification on the expression. |
| `canonicalized()` | `Result<Parameter, ParameterError>` | Return the canonical form used for parameter residency, comparison and deduplication. |

```rust
use cqlib_core::circuit::Parameter;

let x = Parameter::symbol("x");
let expr: Parameter = x.clone() + 0.0;

let simplified = expr.simplify()?;
let canonical = simplified.canonicalized()?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## Replacement and substitution

`Parameter` supports replacing a symbol in an expression with another parameter expression.

| Method | Description |
| --- | --- |
| `replace(symbol, param)` | Replace a single symbol. |
| `substitute_many(bindings)` | Replace multiple symbols at once. |

```rust
use cqlib_core::circuit::Parameter;

let x = Parameter::symbol("x");
let y = Parameter::symbol("y");

let expr: Parameter = x + 2.0;
let replaced = expr.replace("x", y.clone() * 3.0);
```

---

## Differentiation

```rust
pub fn derivative(&self, var: &str) -> Result<Self, ParameterError>
```

`derivative(var)` performs symbolic differentiation with respect to the given symbol variable and returns a new `Parameter` expression.

```rust
use cqlib_core::circuit::Parameter;

let theta = Parameter::symbol("theta");
let expr = theta.clone().sin() + theta.clone() * theta;

let dtheta = expr.derivative("theta")?;

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## Equivalence checking

`Parameter` implements `PartialEq`, `Eq` and `Hash`, all three based on the structure of the underlying expression AST: `x + y` and `y + x` differ in structure and are therefore not equal. To compare by mathematical semantics, use the conservative equivalence interfaces below.

| Method | Description |
| --- | --- |
| `provably_equal(other, tolerance)` | Conservatively determine whether two expressions are equal. |
| `provably_equal_modulo(other, modulus, tolerance)` | Perform a conservative equality check modulo a given period. |

`provably_equal_modulo()` is a conservative decision: it returns `true` only when the difference of the two expressions can be evaluated to a concrete value and that difference is an integer multiple of the modulus (within tolerance). If the difference still contains unbound symbols, it returns `false` even when the two expressions mathematically differ by an integer multiple of the period. For example, `theta + pi` and `theta - pi` differ by `2 * pi`, but the difference contains the free symbol `theta`, so the interface cannot prove the two equal.

```rust
use cqlib_core::circuit::Parameter;

let angle = Parameter::from(0.5);
let equivalent = Parameter::from(0.5 + 2.0 * std::f64::consts::PI);

assert!(angle.provably_equal_modulo(
    &equivalent,
    &Parameter::from(2.0 * std::f64::consts::PI),
    1e-12,
));

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```
