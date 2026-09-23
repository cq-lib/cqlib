# Parameter

`cqlib.circuit.Parameter`  

```python
from cqlib import Parameter
```

`Parameter` is the core type in Cqlib for representing circuit parameters and mathematical expressions. It can represent either a definite numeric constant or a symbolic variable that has not been assigned a value yet, and can also be composed into more complex parameter expressions through arithmetic operations and mathematical functions.

In quantum circuits, `Parameter` is commonly used to describe rotation angles, delay durations, global phase, elements of custom symbolic gate matrices, and the parameters to be optimized in variational algorithms. Through parameterized expressions, a circuit template with symbolic parameters can first be constructed, and parameter binding, matrix computation, simulation execution or compilation can then be performed under different values in a later flow.

---

## Creating parameters

`Parameter` supports creating parameter objects from numbers, symbol names and expression strings, and also provides constructors for common mathematical constants.

```python
Parameter(value: int | float | str)
Parameter.from_expression(expr: str) -> Parameter
Parameter.pi() -> Parameter
Parameter.e() -> Parameter
```

| Input | Description |
| --- | --- |
| `Parameter(3.14)` | Create a numeric constant parameter. |
| `Parameter("theta")` | Create a symbolic parameter named `theta`. |
| `Parameter("2 * theta + pi / 2")` | Parse a parameter expression from an expression string. |
| `Parameter.pi()` | Create the mathematical constant `pi`. |
| `Parameter.e()` | Create the mathematical constant `e`. |

Example:

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = 2 * theta + Parameter.pi() / 2
```

In the example above, `theta` is a free symbol and `expr` is an expression composed of a symbolic parameter, a numeric constant and a mathematical constant.

---

## Supported expression capabilities

The expression elements supported by `Parameter` include:

- numeric constants and symbolic variables;
- mathematical constants: `pi`, `e`;
- arithmetic operators: `+`, `-`, `*`, `/`, `**`;
- parentheses, used to control expression precedence;
- common functions: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `exp`, `ln`, `log`, `sqrt`, `abs`, `sinh`, `cosh`, `tanh`, `floor`, `ceil`, `round`.

Besides expression strings, `Parameter` also provides mathematical functions in method form, so that expressions can be constructed directly.

```python
from cqlib import Parameter

theta = Parameter("theta")

expr = theta.sin() + theta.cos()
root = (theta * theta + 1).sqrt()
```

---

## Arithmetic operators

`Parameter` supports common arithmetic operations:

| Operation | Python form |
| --- | --- |
| Addition | `a + b`, `1.0 + a` |
| Subtraction | `a - b`, `1.0 - a` |
| Multiplication | `a * b`, `2.0 * a` |
| Division | `a / b`, `1.0 / a` |
| Exponentiation | `a ** b`, `a.pow(b)` |
| Negation | `-a` |

Example:

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

expr = 2 * theta - phi / 3 + Parameter.pi()
```

Every arithmetic operation returns a new `Parameter` object and does not modify the original parameter. Therefore, the same symbolic parameter can be safely reused in several expressions.

---

## Mathematical function methods

`Parameter` supports common mathematical function methods:

| Method | Description |
| --- | --- |
| `sin()` / `cos()` / `tan()` | Trigonometric functions. |
| `asin()` / `acos()` / `atan()` | Inverse trigonometric functions. |
| `exp()` / `ln()` / `log(base=None)` | Exponential and logarithmic functions. |
| `sqrt()` / `abs()` | Square root and absolute value. |
| `sinh()` / `cosh()` / `tanh()` | Hyperbolic functions. |
| `floor()` / `ceil()` / `round()` | Rounding-related functions. |

Example:

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = (2 * x + y).sin().exp()
```

---

## Evaluation

`evaluate(bindings)` computes a parameter expression into a concrete floating-point number.

```python
evaluate(bindings: dict[str, float] | None = None) -> float
```

Example:

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = 2 * theta + 1

assert expr.evaluate({"theta": 0.5}) == 2.0
```

A constant expression without free symbols can be evaluated directly:

```python
from cqlib import Parameter

value = (Parameter.pi() / 2).evaluate()
print(value)
```

---

## Symbol information and state checks

`Parameter` provides a set of query interfaces for checking which free symbols an expression contains, and whether the expression is a constant or a specific value.

| Interface | Returns | Description |
| --- | --- | --- |
| `symbols` | `list[str]` | All unique free symbols contained in the expression. |
| `as_symbol()` | `str / None` | Return the symbol name when the expression is exactly a single symbol; otherwise return `None`. |
| `is_constant()` | `bool` | Return `True` when the expression contains no free symbols. |
| `is_exact_zero()` | `bool` | Return `True` when the expression is syntactically exactly `0`. |
| `is_zero()` | `bool` | Return `True` when the expression can be evaluated and the result equals `0`. |
| `is_one()` | `bool` | Return `True` when the expression can be evaluated and the result equals `1`. |

Example:

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

## Simplification and canonicalization

`Parameter` supports expression simplification and canonicalization.

```python
simplify() -> Parameter
canonicalized() -> Parameter
```

- `simplify()` returns a new algebraically simplified expression.

```python
from cqlib import Parameter

x = Parameter("x")
expr = (x + 0).simplify()
```

- `canonicalized()` returns the canonical storage form used internally by Cqlib for parameter interning, comparison, deduplication or compiler processing.

---

## Replacement and substitution

`replace()` and `substitute()` are used to replace symbols in an expression.

```python
replace(symbol: str, param: Parameter) -> Parameter
substitute(bindings: dict[str, Parameter]) -> Parameter
```

Example:

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = x + 2
replaced = expr.replace("x", 3 * y)
substituted = replaced.substitute({"y": Parameter(0.5)})
```

`replace(symbol, param)` replaces a single symbol; `substitute(bindings)` replaces several symbols at once. Both return a new `Parameter` expression and do not modify the original object.

---

## Differentiation

`derivative(var)` differentiates with respect to the given symbolic variable and returns a new `Parameter` expression.

```python
derivative(var: str) -> Parameter
```

Example:

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = theta.sin() + theta * theta

dtheta = expr.derivative("theta")
```

This interface performs symbolic differentiation at the level of classical parameter expressions. For example, it can compute the derivative of a gate angle expression, a global phase expression or a symbolic matrix element with respect to a variable.

---

## Equivalence checks

`Parameter` provides equivalence check interfaces, used to determine whether two expressions can be proven equal.

```python
provably_equal(other: Parameter, tolerance: float = 1e-12) -> bool
provably_equal_modulo(
    other: Parameter,
    modulus: Parameter,
    tolerance: float = 1e-12,
) -> bool
```

Example:

```python
from cqlib import Parameter

theta = Parameter("theta")

expr1 = theta + 0
expr2 = theta

assert expr1.provably_equal(expr2)
```
