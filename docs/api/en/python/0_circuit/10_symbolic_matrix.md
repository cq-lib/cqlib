# SymbolicMatrix

- `cqlib.circuit.SymbolicComplex`
- `cqlib.circuit.SymbolicMatrix`

```python
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
```

`SymbolicMatrix` is the basic type in Cqlib for representing symbolic matrices. It is composed of several `SymbolicComplex` elements, and the real and imaginary parts of each `SymbolicComplex` can hold `Parameter` expressions. Through this design, Cqlib can keep symbolic parameters in matrix elements instead of evaluating them immediately into ordinary complex numbers when the matrix is constructed.

## Core concepts

In Cqlib, a symbolic matrix consists of two levels of structure:

| Type | Role |
| --- | --- |
| `Parameter` | Represents a number, a symbolic variable or a parameter expression. |
| `SymbolicComplex` | Represents one complex element, whose real and imaginary parts are both `Parameter`. |
| `SymbolicMatrix` | Represents a dense matrix composed of `SymbolicComplex` elements. |

Elements in an ordinary numeric matrix are usually `complex`, whereas elements in a `SymbolicMatrix` can contain symbolic parameters. For example, a matrix element can be `exp(i * theta)`, `cos(theta)` or an expression composed of several parameters. This allows the same matrix structure to be evaluated repeatedly under different parameter values, and also makes it easy to preserve parameter relations at compilation and validation stages.

## `SymbolicComplex`

`SymbolicComplex` represents a symbolic complex number. Its real and imaginary parts are both `Parameter`, so it can represent either an ordinary complex number or a complex expression containing symbolic parameters.

```python
SymbolicComplex(real: Parameter, imag: Parameter)
```

For example, a symbolic complex number can be written as:

```text
real + i * imag
```

Here both `real` and `imag` can be constants, a single symbol or a more complex parameter expression.

### 1. Construction methods

`SymbolicComplex` provides a set of common static methods for quickly creating symbolic complex numbers such as zero, the imaginary unit, pure real numbers and phase factors.

| Static method | Description |
| --- | --- |
| `zero()` | Create `0 + 0i`. |
| `one()` | Create `1 + 0i`. |
| `i()` | Create `0 + 1i`. |
| `from_real(value)` | Create a pure real complex number from a real number or a parameter expression. |
| `exp_i(theta)` | Create `exp(i * theta)`, that is, `cos(theta) + i sin(theta)`. |

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex

theta = Parameter("theta")

zero = SymbolicComplex.zero()
one = SymbolicComplex.one()
imag = SymbolicComplex.i()
phase = SymbolicComplex.exp_i(theta)
```

### 2. Attributes and methods

| Interface | Returns | Description |
| --- | --- | --- |
| `real` | `Parameter` | The real part of the symbolic complex number. |
| `imag` | `Parameter` | The imaginary part of the symbolic complex number. |
| `symbols` | `list[str]` | The free symbol names contained in the real and imaginary parts. |
| `evaluate(bindings=None)` | `complex` | Evaluate the symbolic complex number into a `complex` according to parameter bindings. |
| `simplify()` | `SymbolicComplex` | Simplify the real and imaginary part expressions. |
| `replace(symbol, value)` | `SymbolicComplex` | Replace the given symbol with a new parameter expression. |
| `is_zero_exact()` | `bool` | Determine whether it is syntactically exactly equal to `0`. |
| `is_one_exact()` | `bool` | Determine whether it is syntactically exactly equal to `1`. |
| `simplifies_to_zero()` | `bool` | Determine whether it equals `0` after simplification. |

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex

theta = Parameter("theta")
value = SymbolicComplex.exp_i(theta)

numeric = value.evaluate({"theta": 0.0})
print(numeric)  # (1+0j)
```

## `SymbolicMatrix`

`SymbolicMatrix` represents a dense matrix composed of `SymbolicComplex` elements. The matrix is passed in by rows, and every row should have the same length.

```python
SymbolicMatrix(rows: list[list[SymbolicComplex]])
```

The following example constructs a simple `2 × 2` symbolic matrix:

```python
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

i = SymbolicComplex.i()
zero = SymbolicComplex.zero()

matrix = SymbolicMatrix([
    [zero, i],
    [i, zero],
])
```

### 1. Attributes and methods

| Interface | Returns | Description |
| --- | --- | --- |
| `shape` | `tuple[int, int]` | The matrix shape, in the form `(rows, cols)`. |
| `symbols` | `list[str]` | The free symbol names contained in all matrix elements. |
| `evaluate(bindings=None)` | `numpy.ndarray` | Evaluate the symbolic matrix into a numeric complex matrix. |
| `simplify()` | `SymbolicMatrix` | Simplify all elements in the matrix. |
| `substitute(replacements)` | `SymbolicMatrix` | Replace several symbols in the matrix at the same time. |
| `rows()` | `list[list[SymbolicComplex]]` | Return the nested row representation of the matrix. |
| `__getitem__((row, col))` | `SymbolicComplex` | Read the element at the given position; negative indices are supported. |
| `__len__()` | `int` | Return the number of matrix rows. |

```python
import numpy as np
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

theta = Parameter("theta")

m = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

numeric = m.evaluate({"theta": np.pi})
print(numeric.shape)  # (2, 2)
```

### 2. Simplification, substitution and evaluation

`SymbolicMatrix` supports simplification, symbolic substitution and numeric evaluation of the parameter expressions in matrix elements. These operations do not modify the original matrix object; they return a new symbolic matrix or a numeric matrix result.

```python
from cqlib import Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix

theta = Parameter("theta")
phi = Parameter("phi")

m = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

renamed = m.substitute({"theta": phi})
simplified = renamed.simplify()
numeric = simplified.evaluate({"phi": 0.5})
```

### 3. Creating parameterized custom gates

A typical use of `SymbolicMatrix` is defining parameterized custom unitary gates. Through `UnitaryGate.with_symbolic_matrix()`, a symbolic matrix can be registered as the matrix definition of a custom gate, and the parameter order for applying the gate can be specified.

```python
from cqlib import Circuit, Parameter
from cqlib.circuit import SymbolicComplex, SymbolicMatrix
from cqlib.circuit.gates import UnitaryGate

theta = Parameter("theta")

symbolic = SymbolicMatrix([
    [SymbolicComplex.one(), SymbolicComplex.zero()],
    [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
])

gate = UnitaryGate("DiagPhase", 1, num_params=1).with_symbolic_matrix(
    symbolic,
    ["theta"],
)

circuit = Circuit(1)
circuit.append_unitary_gate(gate, [0], params=[Parameter("phi")])
```

In the example above, `with_symbolic_matrix(symbolic, ["theta"])` means the custom gate has one positional parameter, corresponding to `theta` in the symbolic matrix. When the gate is appended to a circuit through `append_unitary_gate()`, the `params=[Parameter("phi")]` passed in replaces `theta` in the gate definition with the external parameter `phi` according to the parameter order.

### 4. Obtaining a symbolic matrix from a circuit

Besides constructing a symbolic matrix by hand, a symbolic matrix can also be generated from a parameterized circuit. `Circuit.to_symbolic_matrix(qubits_order=None)` converts a circuit into a matrix representation that preserves parameter expressions.

```python
Circuit.to_symbolic_matrix(qubits_order=None) -> SymbolicMatrix
```

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

matrix = circuit.to_symbolic_matrix()
print(matrix.symbols)
```

When a circuit contains symbolic parameters, `to_symbolic_matrix()` can preserve those parameter structures, which suits small-scale parameterized circuit validation, gate definition checks and compiler rule testing.

### 5. Complexity and applicable boundary

For a circuit with `n` qubits, the full matrix size is `2^n × 2^n` and the number of matrix elements is `4^n`. When matrix elements also contain symbolic expressions, the expression complexity can also grow with the number of gates and parameters.

---

## Other behavior

- Both `SymbolicComplex` and `SymbolicMatrix` support `copy` and `deepcopy`.
- `SymbolicComplex` supports `==` comparison; `SymbolicMatrix` does not provide equality comparison, so to compare two matrices, the results of `rows()` can be compared.
- Both carry a `repr` convenient for debugging.
