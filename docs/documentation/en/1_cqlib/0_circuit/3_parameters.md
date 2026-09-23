# Parameter system

`Parameter` is the core type in Cqlib for representing numeric parameters, symbolic variables and parameter expressions, and is commonly used to describe quantum gate angles, circuit global phases, custom symbolic gates and tunable elements in symbolic matrices. With `Parameter`, a circuit template with symbolic parameters can be constructed first, and then bound, evaluated and validated with different parameter values in a later algorithm flow.

`Parameter` can represent a definite constant, a symbolic variable that has not yet been assigned, or a more complex expression composed through arithmetic operations and mathematical functions. This allows Cqlib to perform multiple parameter bindings, parameter sweeps, optimization iterations or symbolic analyses on the same parameterized circuit while the circuit structure stays unchanged.

---

## Creating a `Parameter`

`Parameter` can be created from a symbol name, a numeric value, an expression string or a built-in constant constructor. Different ways of creating it suit different scenarios:
- A symbol name is suitable for defining parameters to be optimized;
- A numeric value is suitable for representing a determined angle;
- An expression string is suitable for parsing a parameter expression from a configuration file or external input;
- A constant constructor is used to represent mathematical constants such as `pi` and `e` uniformly.

### 1. Creating from a symbol name

The most common way is to create a symbolic parameter using a string. A symbolic parameter represents a variable that has not yet been assigned, and is usually used for parameterized quantum gates, the global phase, symbolic matrices or parameters to be optimized in variational algorithms.

```python
from cqlib import Parameter

theta = Parameter("theta")
phi = Parameter("phi")

print(theta.symbols)  # ['theta']
print(theta.as_symbol())  # theta
```

Note that the passed string is parsed as a parameter expression. Therefore, if the string does not conform to the parameter expression syntax, Cqlib raises `ParameterError` to prompt the user to check the variable name or the expression format.

### 2. Creating from a numeric value

If the parameter value is already determined, an integer or floating-point number can be used directly to create a numeric parameter. Numeric parameters are usually used for rotation gates with fixed angles, a fixed global phase, or for constructing definite parameter objects in tests and examples.

```python
from cqlib import Parameter

a = Parameter(1)
b = Parameter(0.25)

print(a.is_constant())     # True
print(b.evaluate({}))      # 0.25
```

A `Parameter` created from a numeric value is a constant expression and does not depend on any symbol binding, so it can be evaluated directly.

### 3. Creating from an expression string

When a parameter expression comes from a configuration file, user input, serialized data or an external tool, `Parameter.from_expression()` can be used to parse the expression from a string.

```python
from cqlib import Parameter

expr = Parameter.from_expression("2*x + sin(y) + pi/2")

print(expr.symbols)     # ['x', 'y']
print(expr.evaluate({"x": 0.1, "y": 0.2}))
```

An expression string can contain variables, constants, arithmetic operations and common mathematical functions. After parsing, the result is still a `Parameter` object, which can continue to be used for quantum gate parameters, the global phase, symbolic matrices or other parameter expression compositions.

The currently supported expression elements include:

- Numbers;
- The constants `pi` and `e`;
- Variable names;
- The operators `+`, `-`, `*`, `/`, `**`;
- Parentheses;
- Functions such as `sin`, `cos`, `tan`, `exp`, `sqrt`, `ln` and `log`.

### 4. Constant constructors

For common mathematical constants, Cqlib provides dedicated constant constructors, such as `Parameter.pi()` and `Parameter.e()`. Using these constructors avoids typing approximate floating-point numbers by hand, thus improving the readability and consistency of expressions.

```python
from cqlib import Parameter

pi = Parameter.pi()
e = Parameter.e()

print(pi.evaluate({}))
print(e.evaluate({}))
```

---

## Arithmetic expressions

`Parameter` supports common Python arithmetic operations, so symbolic parameters, constants and parameter expressions can be combined like ordinary numeric values. These operations make it convenient to construct gate angles, the global phase, symbolic matrix elements or parameter relations that need to be reused in an algorithm.

Note that a `Parameter` expression is an immutable object. Every arithmetic operation returns a new `Parameter` and does not modify the original parameter object. Therefore, the same symbolic parameter can be reused safely, and multiple different expressions can be constructed based on it.

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

`Parameter` also supports mixed operations between numeric values and symbolic parameters. A number can appear on the left or the right side of an expression, and Cqlib converts it automatically into the corresponding constant parameter expression.


---

## Mathematical functions

Besides basic arithmetic operations, `Parameter` also provides a set of common mathematical functions for constructing more complex parameter expressions. These functions can act on a single symbolic parameter or on a composite expression formed by several parameters, and are suitable for describing parameterized gate angles, symbolic matrix elements, parameter transformation relations in an algorithm, and expressions that need differentiation or simplification.

The commonly used mathematical functions are as follows:

| Method | Description |
|---|---|
| `sin()` / `cos()` / `tan()` | Trigonometric functions |
| `asin()` / `acos()` / `atan()` | Inverse trigonometric functions |
| `sinh()` / `cosh()` / `tanh()` | Hyperbolic functions |
| `exp()` | Exponential function |
| `ln()` | Natural logarithm |
| `log(base=None)` | Logarithm; when `base=None`, it is treated as the natural logarithm |
| `sqrt()` | Square root |
| `abs()` | Absolute value |
| `floor()` / `ceil()` / `round()` | Rounding-related functions |

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = (2 * x + y).sin().exp()
grad = expr.derivative("x")

print(expr)
print(grad)
```

The mathematical functions above support chained calls and can also be combined with arithmetic operations such as addition, subtraction, multiplication, division and exponentiation. Because every function call returns a new `Parameter` object and the original parameter is not modified, an existing symbolic parameter can be reused safely to construct many different expressions.

---

## Evaluation

`evaluate(bindings)` is used to compute a `Parameter` expression into a concrete floating-point result. For an expression that contains free symbols, the numeric value corresponding to each symbol needs to be provided through the `bindings` parameter; Cqlib can perform numeric evaluation only after all free symbols in the expression have been bound.

```python
from cqlib import Parameter

theta = Parameter("theta")
expr = (2 * theta + 1).sin()

value = expr.evaluate({"theta": 0.5})
print(value)
```

If the expression itself contains no free symbols, for example when it is formed by constants or mathematical constants, an empty dictionary can be passed, or the `bindings` parameter can be omitted.

```python
from cqlib import Parameter

expr = Parameter.pi() / 2

print(expr.evaluate({}))
print(expr.evaluate())
```

Note that `evaluate()` requires the expression to produce a valid result in the real domain. Cqlib usually raises `ParameterError` in the following cases:
- The expression contains unbound free symbols;
- The bindings dictionary is missing a necessary parameter name;
- A binding value contains an illegal numeric value such as `NaN` or infinity;
- The expression cannot be evaluated in the real domain.

Therefore, before a parameter expression is used for circuit matrix computation, parameter binding or validation of optimizer results, confirm first that all free symbols are bound correctly and that the binding values are within the legal numeric range.

---

## Simplification

`simplify()` is used to perform algebraic simplification on a `Parameter` expression and returns a new parameter expression. This interface is commonly used to reduce redundant expressions, canonicalize parameter forms, and tidy up expression structures before parameter binding, symbolic matrix construction or circuit analysis.

Note that `simplify()` does not modify the original expression, but generates a new `Parameter` object. This ensures that the original parameter expression is reused safely in multiple circuit templates or algorithm flows.

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

## Differentiation

`derivative(var)` is used to perform symbolic differentiation of a `Parameter` expression with respect to a specified symbolic variable, and returns a new `Parameter` expression. This interface deals with differentiation relations at the level of classical parameter expressions, and is suitable for analyzing the dependence of gate angles, the global phase, symbolic matrix elements or other parameter combination relations on a certain variable.

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

Note that `Parameter.derivative()` only deals with the classical symbolic expression itself and does not directly compute the gradient of a quantum circuit with respect to some objective function. The overall circuit gradient usually also depends on the quantum state evolution, the measured observables, the simulator or backend execution results, as well as parameter-shift rules, the adjoint method or other quantum gradient computation methods.

---

## Replacement and substitution

In parameterized circuits and symbolic expression handling, a symbol in an expression sometimes needs to be replaced by another parameter expression. Here, Cqlib provides the `replace()` and `substitute()` interfaces for symbol-level replacement in `Parameter` expressions.

### 1. `replace()`

`replace()` is used to replace a single symbol in an expression. The name of the symbol to be replaced needs to be specified, together with the `Parameter` expression that replaces it.

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")

expr = x + 2
new_expr = expr.replace("x", 3 * y)

print(new_expr)
```

### 2. `substitute()`

`substitute()` is used to replace multiple symbols at the same time, and is suitable for completing batch symbol substitution in one operation. A dictionary can be used to specify the correspondence between multiple symbols and replacement expressions.

```python
from cqlib import Parameter

x = Parameter("x")
y = Parameter("y")
z = Parameter("z")

expr = x * y + 1
new_expr = expr.substitute({"x": z + 1, "y": Parameter(2)})

print(new_expr.simplify())
```

Note in particular that `replace()` and `substitute()` perform symbolic expression replacement; the replacement value should be a `Parameter` object or an expression convertible to `Parameter`, rather than a `float` result used directly for numeric evaluation.

---

## Equivalence checking

In parameterized circuits, symbolic matrices and compilation and optimization, it is sometimes necessary to determine whether two `Parameter` expressions represent the same mathematical meaning. Here, Cqlib provides a fairly conservative equivalence checking interface. "Conservative" means that when the interface returns `True`, the two expressions can be regarded as provably equivalent under the current rules and tolerance; when the interface returns `False`, it does not necessarily mean that the two expressions are definitely not equivalent — it may also mean only that the current symbolic rules cannot prove them equal.

```python
from cqlib import Parameter

x = Parameter("x")

a = (x + 0).simplify()
b = x

print(a.provably_equal(b))
```

| Method | Description |
|---|---|
| `provably_equal(other, tolerance=1e-12)` | Conservatively determines, within the given tolerance, whether two expressions can be proven equal |
| `provably_equal_modulo(other, modulus, tolerance=1e-12)` | Determines whether two expressions are equivalent under the specified modulus |

Note that equivalence checking usually depends on expression simplification, algebraic rules and numeric tolerance. For expressions with a more complex structure, call `simplify()` first and then perform equivalence checking.

---

## State checking methods

`Parameter` provides a set of state checking methods, used to determine whether an expression contains free symbols, whether it is a constant, whether it is equal to a particular value, or whether it can be treated as a single symbol. These methods are commonly used in scenarios such as parameter validation, compilation and optimization, circuit canonicalization, pre-binding checks and symbolic matrix handling.

Through these interfaces, whether a parameter expression meets the requirements of the current flow can be confirmed in advance before matrix computation, circuit conversion or backend execution. For example, before numeric matrix computation it is usually necessary to confirm that no unbound symbol exists in the circuit; during compilation and optimization it may be necessary to identify whether the global phase is zero, or to determine whether a certain rotation angle can be eliminated.

The commonly used state checking methods are as follows:

| Method | Description |
|---|---|
| `symbols` | Returns the list of free symbols contained in the expression |
| `canonicalized()` | Returns the canonicalized form used for parameter interning and internal comparison |
| `is_exact_zero()` | Determines whether the expression represents the constant `0` exactly |
| `is_constant()` | Determines whether the expression contains no free variables |
| `is_zero()` | Determines whether the expression can be evaluated to `0` under the current conditions |
| `is_one()` | Determines whether the expression can be evaluated to `1` under the current conditions |
| `as_symbol()` | If the expression is exactly a single symbol, returns that symbol name; otherwise returns `None` |

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

## Using parameters in a circuit

In Cqlib, a parameterized quantum gate can accept an ordinary numeric parameter or a `Parameter` expression as input. Numeric parameters are suitable for constructing circuits whose angles are already determined; `Parameter` expressions are suitable for constructing parameterized circuit templates and, in a later algorithm flow, for parameter binding, parameter sweeps or optimization iterations.

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

## Parameter binding

Parameter binding means replacing the symbolic parameters in a circuit with concrete values, producing a circuit that can be used further for matrix computation, simulation execution, IR export or backend running. Cqlib completes parameter binding through `assign_parameters()`. This interface accepts a mapping from parameter names to values, such as `dict[str, float]`, and returns a new bound circuit.

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

Cqlib also supports partial parameter binding. If the bindings dictionary provides values for only some of the symbols, the symbols that are bound are replaced by numeric values, and the unbound symbols continue to be kept in the circuit.

```python
partial = c.assign_parameters({"theta": 0.25})

print(partial[0].params)  # [0.25]
print(partial[1].params)  # [Parameter("0.25 + phi")]
print(partial.symbols)    # ['phi']
```

---

## Relation to matrix conversion

When a parameterized circuit undergoes matrix conversion, there are two cases, the numeric matrix and the symbolic matrix:
- A numeric matrix requires all parameters in the circuit to be bound to concrete values;
- A symbolic matrix can keep parameter expressions, and is used for symbolic analysis and validation of small-scale circuits.

### 1. Numeric matrix

`to_matrix()` is used to convert a quantum circuit into a numeric matrix. Because every element of a numeric matrix must be a definite complex value, all symbolic parameters in the circuit need to be bound to numeric values before `to_matrix()` is called.

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

c = Circuit(1)
c.rx(0, theta)

bound = c.assign_parameters({"theta": 0.3})
matrix = bound.to_matrix()
```

### 2. Symbolic matrix

To keep the parameter structure in the circuit matrix, `to_symbolic_matrix()` can be used. This interface generates a symbolic matrix whose elements may contain `Parameter` expressions, and is suitable for small-scale parameterized circuit validation, symbolic gate definition and compilation rule checking.

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

## Using parameters in custom symbolic gates

Cqlib supports using parameters in custom symbolic gates. Through `SymbolicComplex` and `SymbolicMatrix`, a matrix containing `Parameter` expressions can be defined, and this matrix can be used as the symbolic matrix representation of a `UnitaryGate`.

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

Note that `params` in `with_symbolic_matrix(matrix, params)` is used to define the parameter order of the custom gate. When `append_unitary_gate()` is called in a circuit, the parameters passed in are bound to the parameter names in the symbolic matrix in this order.

---

## Next steps

- [Circuit analysis and transformation](4_circuit_analysis.md): use tools such as inversion, decomposition, matrix conversion and operation inspection.
- [Control flow](5_control_flow.md): use measurement results, classical variables and structured control flow to construct dynamic circuits.
- [Intermediate Representation](../1_ir/0_overview.md): understand the bidirectional conversion flow between Circuit and IR.

