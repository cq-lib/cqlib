# Control flow

Control flow is used to describe conditional branches, loop structures and multi-branch selection logic in a quantum circuit. Unlike an ordinary circuit that only applies quantum gates in a fixed order, control flow can keep structured semantics in a unified circuit representation, such as "execute a section of the circuit when a certain classical condition is met", "repeat a section of the circuit according to a counter variable" or "select a different branch according to an integer value".

This page covers how to construct classical expressions, how to organize circuit structures with interfaces such as `if_`, `if_else`, `while_`, `for_uint` and `switch`, and how to perform basic validation and structural checking after construction.

---

## Classical types

`ClassicalType` is used to describe the type information of classical values in control flow. Through explicit type definitions, whether the types of conditional expressions, loop variables, branch targets and ordinary classical expressions match can be checked during circuit construction.

The commonly used classical types include the following categories:

| Type constructor | Description | Typical use |
|---|---|---|
| `ClassicalType.bit()` | A single bit, with the value `0` or `1` | Single-qubit classical result, bit expression |
| `ClassicalType.bool()` | A logical boolean value | `if` and `while` conditions |
| `ClassicalType.uint(width)` | An unsigned integer of the specified width | Counter, loop variable, `switch` target |
| `ClassicalType.bit_vec(width)` | A bit vector of the specified width | Bit vector expression, bit packing, bit concatenation result |

```python
from cqlib.circuit import ClassicalType

bit_ty = ClassicalType.bit()
bool_ty = ClassicalType.bool()
u3_ty = ClassicalType.uint(3)
bits_ty = ClassicalType.bit_vec(4)

print(bit_ty.width)    # 1
print(u3_ty.width)     # 3
```

Type objects can also be used to create commonly used literal expressions. The following example creates the `0` and `1` literals of a 3-bit unsigned integer type:

```python
zero = ClassicalType.uint(3).zero_literal()
one = ClassicalType.uint(3).one_literal()
```

---

## Classical variables

`Circuit.var(type)` is used to allocate a mutable classical variable in a circuit. Every classical variable has explicit type information, such as `Bool`, `UInt` or `BitVec`, and is bound to the `Circuit` object that created it.

Classical variables are usually used to store and reference classical state in control flow, for example recording a branch flag, maintaining a loop counter, or serving as the target value of `switch` branch determination.

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

`store(target, value)` is used to write a classical expression to a classical variable. When writing, the type of `value` must be compatible with `target.ty`; otherwise the classical data semantics in the circuit are broken.

---

## Classical expressions

`ClassicalExpr` is used to describe the classical computation logic in control flow. It can represent expression structures such as literals, variable reads, logical operations, comparison operations, conditional selection, bit extraction and bit concatenation.

`ClassicalExpr` can be understood as a kind of "classical expression syntax tree". It does not perform the computation immediately itself, but records the classical computation relations and takes part in later validation, transformation and compilation processing as part of the circuit structure.

### 1. Literals

Literals are used to construct fixed classical values directly, such as a boolean value, a single bit, an unsigned integer or a bit vector.

```python
from cqlib.circuit import ClassicalExpr

truth = ClassicalExpr.bool_literal(True)
bit_one = ClassicalExpr.bit_literal(True)
u3_five = ClassicalExpr.uint_literal(3, 5)
bits = ClassicalExpr.bit_vec_literal(4, 0b1010)
```

Here, `uint_literal(width, value)` and `bit_vec_literal(width, value)` need the bit width to be specified explicitly. The bit width is part of the classical type and affects the legality of later comparison, loop and concatenation operations.

### 2. Reading from a variable

A classical variable is itself a handle that can be written to and referenced. If the current value of a variable needs to be read in an expression, `ClassicalExpr.var(var)` can be used to convert the variable into an expression, or the `expr()` method provided by the variable handle can be called directly.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)

flag = c.var(ClassicalType.bool())
flag_expr = ClassicalExpr.var(flag)

# equivalent shorthand
same_flag_expr = flag.expr()
```

### 3. Logical operations

`ClassicalExpr` supports common logical operations. The operators `~`, `&`, `|` and `^` correspond to logical NOT, AND, OR and XOR respectively.

```python
from cqlib.circuit import ClassicalExpr

a = ClassicalExpr.bool_literal(True)
b = ClassicalExpr.bool_literal(False)

expr = a | b
simplified = expr.simplified() 
print(simplified.is_bool_true()) # True
```

### 4. Comparison

To construct a comparison condition, the static methods provided by `ClassicalExpr` should be used.

```python
from cqlib.circuit import ClassicalExpr

x = ClassicalExpr.uint_literal(3, 2)
y = ClassicalExpr.uint_literal(3, 5)

cond1 = ClassicalExpr.lt(x, y)
cond2 = ClassicalExpr.equal(x, y)
cond3 = ClassicalExpr.not_equal(x, y)
cond4 = ClassicalExpr.ge(y, x)
```

The supported comparison methods include:

- `equal(lhs, rhs)`
- `not_equal(lhs, rhs)`
- `lt(lhs, rhs)`
- `le(lhs, rhs)`
- `gt(lhs, rhs)`
- `ge(lhs, rhs)`

The comparison result is usually a `Bool` expression and can be used directly as the condition of `if_`, `if_else` or `while_`.

### 5. Selection, extraction and concatenation

Besides basic logical and comparison operations, `ClassicalExpr` also supports operations such as conditional selection, bit extraction and bit concatenation, which can be used to construct more complex classical control logic.

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

Here:
- `select(cond, a, b)` means selecting between two expressions according to a boolean condition: when `cond` is true, `a` is selected, otherwise `b` is selected;
- `extract_bit()` is used to extract a single bit from a `BitVec`, and `extract_bits()` is used to extract a contiguous multi-bit fragment;
- `pack_bits(bits)` is used to pack multiple Bit expressions into one BitVec expression.

---

## `if_` conditional branch

`if_(condition, body)` is used to append a conditional control structure that does not contain an `else` branch to a circuit. This interface means that when the given condition is met, the circuit operations in the branch body are executed; when the condition is not met, the branch body is skipped.

Here:
- `condition` must be a `ClassicalExpr` expression of type `Bool`, used to describe the condition determination logic;
- `body` is a callback function, used to construct the circuit content to be executed when the condition is met. The callback function receives a temporary circuit builder, in which quantum gates or other supported operations can continue to be appended.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

c.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(0),
)
```

For a simple branch containing only one operation, the `lambda` form can be used to make the code more concise. For a branch body containing multiple operations, an ordinary function definition is recommended, to improve code readability and later maintainability.

```python
def then_body(body):
    body.h(0)
    body.x(0)

c.if_(ClassicalExpr.bool_literal(True), then_body)
```

Control flow callbacks adopt an atomic construction approach. That is, the branch body is committed to the circuit only after the callback function has completed successfully; if an exception is raised during callback execution, Cqlib abandons this branch body construction, avoiding an incomplete or inconsistent control flow structure left in the circuit.

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

## `if_else` conditional branch

`if_else(condition, then_body, else_body)` is used to construct a conditional control structure with an `else` branch in a circuit. This interface means that when the `condition` is true, the `then_body` branch is executed; when the condition is false, the `else_body` branch is executed. Both branches are constructed through callback functions and are stored in the circuit as part of the same conditional control structure.

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

Like `if_`, the two branches of `if_else` also adopt an atomic construction approach. The related control flow structure is committed to the circuit only after both callback functions have completed construction successfully; if either branch raises an exception during construction, this control flow construction is abandoned, avoiding an incomplete or inconsistent branch structure left in the circuit.

---

## `while_` loop

`while_(condition, body)` is used to construct a loop structure based on a boolean condition in a circuit. This interface means that when the `condition` is met, the operations defined in the loop body `body` are executed; before each iteration begins, whether to continue is determined according to the condition expression.

Here:
- `condition` must be a `ClassicalExpr` expression of type `Bool`;
- `body` is a callback function, used to describe the circuit operations in the loop body. The callback function receives a temporary circuit builder, in which quantum gates, control flow jumps or other supported operations can be appended.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)

c.while_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.break_loop(),
)
```

In the loop body, the following jump operations can be used to control the loop execution flow:

- `break_loop()`: exits the innermost loop;
- `continue_loop()`: skips the remaining operations in the current loop body and proceeds to the next determination of the innermost loop.

```python
def loop_body(body):
    body.x(0)
    body.continue_loop()

c = Circuit(1)
c.while_(ClassicalExpr.bool_literal(True), loop_body)
```

---

## `for_uint` loop

`for_uint(var, start, stop, step, body)` is used to construct a loop structure based on an unsigned integer variable in a circuit, with the loop range `[start, stop)`: it starts from `start`, increments by `step` each iteration, and ends the loop when the loop variable reaches or exceeds `stop`.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr, ClassicalType

c = Circuit(1)
loop_var = c.var(ClassicalType.uint(3))

def body(builder, index_expr):
    # index_expr is a UInt expression that reads loop_var
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

## `switch` multi-branch selection

`switch(target, build)` is used to construct a multi-branch control structure based on integer value matching in a circuit. This interface selects one of several registered branches for execution according to the value of the `target` expression. `build` is a callback function, used to register the different integer matching branches and an optional default branch.

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

## Structure validation

Compared with an ordinary linear circuit, a control flow structure has higher requirements on the internal consistency of the circuit. Therefore, after a circuit containing control flow is constructed, calling `validate()` to perform structural validation on the circuit is recommended.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

c = Circuit(1)
c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

c.validate()
```

---

## Next steps

- [Intermediate Representation](../1_ir/0_overview.md): understand the bidirectional conversion flow between Circuit and IR, supporting circuit persistence, cross-toolchain exchange and later compilation.
- [QCIS support](../1_ir/1_qcis.md): export a Cqlib circuit as QCIS instructions or load a circuit from a QCIS file.
- [OpenQASM 2.0 support](../1_ir/2_qasm2.md): import and export between Cqlib circuits and the OpenQASM 2.0 format.
