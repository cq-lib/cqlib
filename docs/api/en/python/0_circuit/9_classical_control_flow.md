# Classical Data / Control Flow

- `cqlib.circuit.CircuitId`
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
    CircuitId,
    ClassicalType,
    ClassicalExpr,
    ClassicalControlOp,
    ValueControlBody,
    ValueSwitchCase,
)
```

The classical data and control flow API describes the classical-side information and structured control logic in a quantum circuit. Typical objects include classical types, classical variables, measurement results, classical expressions, and control flow structures such as `if`, `while`, `for` and `switch`.

---

## Core objects

Classical data and control flow objects fall into the following categories:

| Type | Role |
|---|---|
| `ClassicalType` | Describes the static type of a classical value, such as `Bit`, `Bool`, `UInt` and `BitVec`. |
| `ClassicalVar` | A mutable classical variable handle, created by `Circuit.var()`. |
| `ClassicalValue` | An immutable classical value handle, usually produced by measurement. |
| `Measurement` | A measurement receipt, recording the measurement result handle and the order of the measured qubits. |
| `ClassicalExpr` | A side-effect-free classical expression, which can represent literals, variable reads, comparisons and logical composition. |
| `ClassicalControlOp` | A low-level control flow object, used to represent `if`, `while`, `for`, `switch`, `break` and `continue`. |
| `ValueControlBody` | The operation sequence in a control flow branch body or loop body. |
| `ValueSwitchCase` | A single integer match branch in a `switch`. |

---

## `ClassicalType`

`ClassicalType` describes the static type of a classical value. Control flow conditions, loop variables, measurement results and classical expressions all rely on type information for legality checks.

```python
ClassicalType.bit() -> ClassicalType
ClassicalType.bool() -> ClassicalType
ClassicalType.uint(width: int) -> ClassicalType
ClassicalType.bit_vec(width: int) -> ClassicalType
```

Common classical types are as follows:

| Type constructor | Description | Typical use |
|---|---|---|
| `ClassicalType.bit()` | A single bit, whose value is `0` or `1`. | Single-bit measurement results, bit expressions. |
| `ClassicalType.bool()` | A logical Boolean value, representing true or false. | `if` and `while` conditions. |
| `ClassicalType.uint(width)` | An unsigned integer with the given bit width. | Counters, loop variables, `switch` targets. |
| `ClassicalType.bit_vec(width)` | A bit vector with the given bit width. | Multi-bit measurement results, bit concatenation, bit packing results. |

The bit width of `uint(width)` and `bit_vec(width)` must be greater than zero, otherwise `CircuitError` is raised.

```python
from cqlib.circuit import ClassicalType

bit_ty = ClassicalType.bit()
flag_ty = ClassicalType.bool()
u8_ty = ClassicalType.uint(8)
bv4_ty = ClassicalType.bit_vec(4)

print(bit_ty.width)   # 1
print(u8_ty.width)    # 8
```

`ClassicalType` can also create common literal expressions of that type:

```python
u4 = ClassicalType.uint(4)

zero = u4.zero_literal()
one = u4.one_literal()
```

---

## `CircuitId`

`CircuitId` is a circuit-local identity, used to distinguish the classical variables and classical values created by different `Circuit`s.

```python
CircuitId() -> CircuitId
Circuit.id -> CircuitId
```

`Circuit.id` gives the identity of the owning circuit. When low-level IR construction needs to create handles directly without a circuit, `CircuitId()` can be used to construct an identity explicitly; the same group of related handles must use the same `CircuitId`.

```python
from cqlib import Circuit
from cqlib.circuit import CircuitId

circuit = Circuit(1)
print(circuit.id)

explicit = CircuitId()
print(explicit)
```

---

## `ClassicalVar`

`ClassicalVar` represents a mutable classical variable handle, created by `Circuit.var(type)`.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalType

circuit = Circuit(1)

flag = circuit.var(ClassicalType.bool())
counter = circuit.var(ClassicalType.uint(4))
```

When low-level IR construction needs to create variable handles directly without a circuit, use:

```python
ClassicalVar(circuit_id: CircuitId, index: int, ty: ClassicalType) -> ClassicalVar
```

Common attributes and methods are as follows:

| Interface | Description |
|---|---|
| `id` | The local identity of the variable in its owning circuit; the value is the same as `index`. |
| `index` | The position of the variable in its owning circuit. |
| `circuit_id` | The circuit identity that created the variable. |
| `ty` | The static classical type of the variable. |
| `expr()` | Construct a `ClassicalExpr` that reads the current value of the variable. |

```python
from cqlib.circuit import ClassicalExpr

circuit.store(flag, ClassicalExpr.bool_literal(True))
circuit.store(counter, ClassicalExpr.uint_literal(4, 3))

print(flag.index)
print(flag.ty)
print(flag.circuit_id == circuit.id)
```

`store(target, value)` writes an expression into a classical variable. When writing, `value.ty` must be exactly the same as `target.ty`, otherwise `store()` raises `CircuitError` directly; if `target` belongs to another circuit, `CircuitError` is likewise raised.

Note that `ClassicalVar` has a circuit ownership relation and should not be mixed directly between different `Circuit`s. If the same logic needs to be expressed in another circuit, the corresponding variable should be created again in that circuit.

---

## `ClassicalValue`

`ClassicalValue` represents an immutable classical value handle, usually produced by a measurement operation.

```python
from cqlib import Circuit

circuit = Circuit(1)

measurement = circuit.measure(0)
value = measurement.value
```

When low-level IR construction needs to create classical value handles directly without a circuit, use:

```python
ClassicalValue(circuit_id: CircuitId, index: int, ty: ClassicalType) -> ClassicalValue
```

Common attributes and methods are as follows:

| Interface | Description |
|---|---|
| `index` | The position of the classical value in its owning circuit. |
| `circuit_id` | The circuit identity that created the classical value. |
| `ty` | The static type of the classical value. |
| `expr()` | Construct an expression that reads the classical value. |

```python
expr = value.expr()
```

---

## `Measurement`

`Measurement` is the receipt object returned by a measurement operation, recording the measurement result handle and the order of the measured qubits.

```python
Measurement(value: ClassicalValue, qubits: list[Qubit]) -> Measurement
```

`Measurement` is usually returned by `Circuit.measure()` or `Circuit.measure_bits()`; low-level IR construction can also create it directly with the form above.

Common attributes and methods are as follows:

| Attribute/method | Description |
|---|---|
| `value` | The `ClassicalValue` result handle produced by the measurement. |
| `qubits` | The order of the measured qubits. |
| `ty` | The measurement result type; `Bit` for a single qubit and `BitVec` for several qubits. |
| `width` | The number of measured bits. |
| `expr()` | Construct an expression that reads the measurement result. |
| `check_qubits(num_qubits)` | Check whether the measured qubits are within the given qubit range. |
| `project(full)` | Project the result corresponding to this measurement out of a complete `Outcome`. |
| `project_basis(basis)` | Project the result corresponding to this measurement from a computational basis index. |

```python
from cqlib import Circuit

circuit = Circuit(3)

m = circuit.measure_bits([0, 2])

print(m.width)       # 2
print(m.ty)
print(m.qubits)

expr = m.expr()
```

If a measurement result is needed as a condition, a single-bit measurement result can be converted to `Bool` directly:

```python
single = circuit.measure(1)
condition = single.expr().to_bool()
```

---

## `ClassicalExpr`

`ClassicalExpr` describes classical computation relations, such as reading variables, reading measurement values, constructing literals, performing logical operations, comparing expressions, conditional selection, bit extraction and bit concatenation.

### 1. Creating expressions

| Static method | Description |
|---|---|
| `ClassicalExpr.var(var)` | Read a mutable classical variable. |
| `ClassicalExpr.value(value)` | Read an immutable classical value. |
| `ClassicalExpr.bool_literal(value)` | Construct a `Bool` literal. |
| `ClassicalExpr.bit_literal(value)` | Construct a `Bit` literal. |
| `ClassicalExpr.uint_literal(width, value)` | Construct a `UInt` literal with the given bit width. |
| `ClassicalExpr.bit_vec_literal(width, value)` | Construct a `BitVec` literal with the given bit width. |

```python
from cqlib.circuit import ClassicalExpr

true_expr = ClassicalExpr.bool_literal(True)
bit_one = ClassicalExpr.bit_literal(True)
u4_three = ClassicalExpr.uint_literal(4, 3)
bits = ClassicalExpr.bit_vec_literal(4, 0b1010)

print(true_expr.ty)   # ClassicalType.bool()
print(bits.ty)        # ClassicalType.bit_vec(4)
```

An expression object carries a `ty` attribute that returns its static classical type, which can be used to check the types of conditions and operands before control flow is constructed.

### 2. Logical and bit operations

`ClassicalExpr` supports common logical operations. The operators `~`, `&`, `|` and `^` correspond to negation, AND, OR and XOR respectively.

| Method / operator | Description |
|---|---|
| `not_()` / `~expr` | Logical or bit negation. |
| `and_(rhs)` / `expr & rhs` | AND operation. |
| `or_(rhs)` / `expr \| rhs` | OR operation. |
| `xor(rhs)` / `expr ^ rhs` | XOR operation. |

```python
from cqlib.circuit import ClassicalExpr

a = ClassicalExpr.bool_literal(True)
b = ClassicalExpr.bool_literal(False)

expr = (a & ~b) | b
print(expr.simplified())
```

### 3. Type conversion

| Method | Description |
|---|---|
| `bit_to_bool()` | Convert `Bit` to `Bool`. |
| `to_bool()` | Convert `Bit` to `Bool`, equivalent to `bit_to_bool()`. |
| `bit_vec_to_uint()` | Convert `BitVec` to `UInt`. |
| `to_uint()` | Convert `BitVec` to `UInt`, equivalent to `bit_vec_to_uint()`. |

```python
from cqlib.circuit import ClassicalExpr

bit_as_bool = ClassicalExpr.bit_literal(True).bit_to_bool()
bits_as_uint = ClassicalExpr.bit_vec_literal(4, 0b1010).to_uint()
```

When an expression is to be used as an `if_()` or `while_()` condition, its type should be `Bool`. If the original expression is a `Bit`, it can be converted explicitly through `to_bool()`; a `UInt` or `BitVec` expression needs to extract a single `Bit` with `extract_bit()` first and then convert it to `Bool`.

### 4. Comparison and selection

Runtime comparisons can use the static methods provided by `ClassicalExpr`.

| Static method | Description |
|---|---|
| `equal(lhs, rhs)` | Equality comparison. |
| `not_equal(lhs, rhs)` | Inequality comparison. |
| `lt(lhs, rhs)` | Less-than comparison. |
| `le(lhs, rhs)` | Less-than-or-equal comparison. |
| `gt(lhs, rhs)` | Greater-than comparison. |
| `ge(lhs, rhs)` | Greater-than-or-equal comparison. |
| `select(condition, then_expr, else_expr)` | Select one of two expressions according to a `Bool` condition. |

```python
from cqlib.circuit import ClassicalExpr

x = ClassicalExpr.uint_literal(3, 2)
y = ClassicalExpr.uint_literal(3, 5)

cond1 = ClassicalExpr.lt(x, y)
cond2 = ClassicalExpr.equal(x, y)
selected = ClassicalExpr.select(cond1, x, y)
```

A comparison result is usually a `Bool` expression and can be used directly as the condition of `if_()`, `if_else()` or `while_()`. The two branch expressions of `select()` should have compatible types.

### 5. Bit extraction and concatenation

| Method | Description |
|---|---|
| `extract_bit(index)` | Extract one `Bit` from a `UInt` or `BitVec`. |
| `extract_bits(offset, width)` | Extract a contiguous bit range from a `UInt` or `BitVec`; the result is a `BitVec` of the given bit width. |
| `concat(parts)` | Concatenate several `Bit` or `BitVec` expressions; the result is a `BitVec`. |
| `pack_bits(bits)` | Pack several `Bit` expressions into a `BitVec`. |

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

### 6. Simplification and literal checks

| Method | Description |
|---|---|
| `simplified()` | Return the simplified expression. |
| `is_bool_true()` / `is_bool_false()` | Determine whether it is a `Bool` constant. |
| `is_bit_true()` / `is_bit_false()` | Determine whether it is a `Bit` constant. |

### 7. Object behavior

- Supports `==` and `hash`, and can be used as a dictionary key or a set element.
- `expr1 == expr2` is a structural equality comparison of expressions, determining whether two expression objects represent the same structure; to construct an expression for a runtime comparison, use static methods such as `ClassicalExpr.equal()`.
- Supports `copy` and `deepcopy`.
- `repr` gives the structural representation of the expression, which is convenient for debugging.

---

## High-level control flow API

`Circuit` provides a set of high-level closure-style control flow interfaces, used to construct structured conditional branches, loops and multi-way selection:

```python
circuit.if_(condition, body)
circuit.if_else(condition, then_body, else_body)
circuit.while_(condition, body)
circuit.for_uint(var, start, stop, step, body)
circuit.switch(target, build)
circuit.break_loop()
circuit.continue_loop()
```

These interfaces construct a branch body or loop body through a callback function. The callback receives a temporary circuit builder, in which the operations to be placed inside the control flow body can be appended. After construction is complete, the whole control flow structure is added to the circuit as one operation.

### 1. `if_`

`if_(condition, body)` constructs a conditional structure without an `else` branch. `condition` must be a `Bool` expression.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(2)

circuit.if_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.x(1),
)
```

If the condition comes from a single-bit measurement result, it usually needs to be converted to `Bool` first:

```python
m = circuit.measure(0)
circuit.if_(m.expr().to_bool(), lambda body: body.x(1))
```

### 2. `if_else`

`if_else(condition, then_body, else_body)` constructs a conditional structure with an `else` branch. `then_body` is executed when the condition is true, otherwise `else_body` is executed.

```python
circuit.if_else(
    m.expr().to_bool(),
    lambda then_body: then_body.x(1),
    lambda else_body: else_body.z(1),
)
```

### 3. `while_`

`while_(condition, body)` constructs a loop structure based on a `Bool` condition.

```python
from cqlib.circuit import ClassicalType, ClassicalExpr

flag = circuit.var(ClassicalType.bool())
circuit.store(flag, ClassicalExpr.bool_literal(True))

circuit.while_(
    flag.expr(),
    lambda body: body.store(flag, ClassicalExpr.bool_literal(False)),
)
```

`break_loop()` and `continue_loop()` can be used inside a loop body to control the loop flow:

```python
circuit.while_(
    ClassicalExpr.bool_literal(True),
    lambda body: body.break_loop(),
)
```

### 4. `for_uint`

`for_uint(var, start, stop, step, body)` constructs a half-open interval loop based on an unsigned integer variable. The loop variable `var` must be of type `UInt`, and `start`, `stop` and `step` must agree with the bit width of the variable.

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

The loop range uses `[start, stop)` semantics. The `body` callback receives two arguments: a temporary circuit builder and the `UInt` expression corresponding to the current loop variable.

### 5. `switch`

`switch(target, build)` performs exact integer matching on a `UInt` expression and selects the corresponding branch to execute.

```python
target = ClassicalExpr.uint_literal(2, 1)

def build_cases(case):
    case.value(0, lambda body: body.x(0))
    case.value(1, lambda body: body.z(0))
    case.default(lambda body: body.h(0))

circuit.switch(target, build_cases)
```

### 6. Transactional semantics

The closure-style control flow API has transactional semantics. That is, the corresponding branch body or control flow structure is committed to the circuit only after the callback function completes successfully. If an exception is raised during the callback, or validation fails while appending an operation, this construction is rolled back and leaves no half-constructed state in the circuit.

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

## Low-level control flow IR

For IR converters, deserializers, compiler tests or low-level structure construction, the low-level control flow objects can also be used directly.

### `ValueControlBody`

```python
ValueControlBody(operations: list[ValueOperation])
```

`ValueControlBody` represents the operation sequence in a control flow body.

| Interface | Description |
|---|---|
| `operations` | The operation sequence inside the control flow body. |
| `__len__()` | Return the number of operations. |
| `has_measurement()` | Determine whether the control flow body contains measurement directly or recursively. |
| `reads_value(value)` | Determine whether the control flow body reads the given `ClassicalValue`. |

### `ValueSwitchCase`

```python
ValueSwitchCase(value: int, body: ValueControlBody)
```

`ValueSwitchCase` represents an exact integer match branch in a `switch`. `value` is the match value and `body` is the control flow body executed when that branch is taken.

### `ClassicalControlOp`

`ClassicalControlOp` is a low-level control flow object that can directly represent `if`, `while`, `for`, `switch` and jump instructions.

| Static method | Description |
|---|---|
| `ClassicalControlOp.if_(condition, then_body, else_body=None)` | Construct `if` or `if-else` control flow. |
| `ClassicalControlOp.while_(condition, body)` | Construct a `while` loop. |
| `ClassicalControlOp.for_uint(var, start, stop, step, body)` | Construct a `for` loop based on a `UInt` variable. |
| `ClassicalControlOp.switch(target, cases, default=None)` | Construct a `switch` multi-way selection. |
| `ClassicalControlOp.break_()` | Construct a `break` jump. |
| `ClassicalControlOp.continue_()` | Construct a `continue` jump. |

Common attributes are as follows:

| Attribute | Description |
|---|---|
| `kind` | The control flow type, such as `"if"`, `"while"`, `"for"`, `"switch"`, `"break"` and `"continue"`. |
| `condition` | The `if` or `while` condition. |
| `then_body` / `else_body` | The branch bodies of `if` control flow. |
| `body` | The body of `while` or `for` control flow. |
| `var` / `start` / `stop` / `step` | The components related to a `for` loop. |
| `target` / `cases` / `default` | The components related to a `switch` multi-way selection. |
| `has_measurement()` | Determine whether measurement is contained inside the control flow. |
| `reads_value(value)` | Determine whether the given classical value is read inside the control flow. |

`ValueControlBody`, `ValueSwitchCase` and `ClassicalControlOp` all support `copy` and `deepcopy`, and carry a `repr` convenient for debugging.

---

## Validation rules

`Circuit.validate()` checks whether classical data and control flow structures satisfy the internal consistency requirements of a circuit.

Validation usually covers the following rules:

- classical variables and classical values must belong to the current circuit;
- a measurement value must be defined before it is used;
- values defined inside a control flow scope must not escape the scope;
- the conditions of `if` and `while` must be `Bool` expressions;
- the loop variable of `for_uint` must be of type `UInt`;
- `start`, `stop` and `step` of `for_uint` must agree with the bit width of the loop variable;
- the target of `switch` must be a `UInt` expression;
- `break` and `continue` must be located in a valid scope;
- low-level control flow objects constructed by hand must be compatible with the current circuit.

```python
from cqlib import Circuit
from cqlib.circuit import ClassicalExpr

circuit = Circuit(1)
circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

circuit.validate()
```

---

## Relation to matrix conversion

A circuit containing measurement, classical data writes or control flow usually cannot be represented directly as one fixed unitary matrix. The reason is that such a circuit describes a program with runtime classical information, conditional branches or loop structures, rather than a fixed sequence of purely quantum gates.

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

If only the pure quantum operations in one branch need to be validated, that branch body can be constructed separately as a static `Circuit`, matrix-validated first, and then composed into the control flow.

```python
branch = Circuit(1)
branch.x(0)
branch_matrix = branch.to_matrix()

circuit = Circuit(1)
circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.compose(branch))
```

This approach handles "gate-level validation" and "control flow structure construction" separately, making testing and debugging clearer.
