# Classical Data / Control Flow

- `cqlib_core::circuit::ClassicalType`
- `cqlib_core::circuit::ClassicalVar`
- `cqlib_core::circuit::ClassicalValue`
- `cqlib_core::circuit::Measurement`
- `cqlib_core::circuit::ClassicalExpr`
- `cqlib_core::circuit::ClassicalControlOp`
- `cqlib_core::circuit::{IfOp, WhileOp, ForOp, SwitchOp, ControlBody}`

```rust
use cqlib_core::circuit::{ClassicalType, ClassicalVar, ClassicalValue, Measurement, ClassicalExpr, ClassicalControlOp};
use cqlib_core::circuit::{IfOp, WhileOp, ForOp, SwitchOp, ControlBody};
```

This page covers the API in the Rust core related to classical data and structured control flow. These types are used to represent measurement results, classical variables and classical expressions in a circuit, as well as control flow structures such as `if`, `while`, `for` and `switch` based on classical conditions.

---

## Core concepts

| Term | Description |
| --- | --- |
| `ClassicalType` | The static type of classical data, including `Bit`, `Bool`, `UInt(width)` and `BitVec(width)`. |
| `ClassicalVar` | A mutable classical variable handle, created by `Circuit::var()` and updatable through `store()`. |
| `ClassicalValue` | An immutable classical value handle, usually produced by measurement, with SSA-style semantics. |
| `Measurement` | A measurement receipt, recording the measurement result value and the order of the measured qubits. |
| `ClassicalExpr` | A typed classical expression AST, used for variable reads, value reads, logical operations, comparisons, type conversions and bit operations. |
| `ClassicalControlOp` | The storage-layer structured control flow IR, including `if`, `while`, `for`, `switch`, `break` and `continue`. |
| `ValueClassicalControlOp` | The construction-layer control flow IR, suitable for serialization, importers and `Circuit::from_operations()`. |
| `ControlBody` / `ValueControlBody` | Control flow bodies, holding the storage-layer or the construction-layer operation sequence respectively. |

---

## `ClassicalType`

```rust
pub enum ClassicalType {
    Bit,
    Bool,
    UInt(u32),
    BitVec(u32),
}
```

`ClassicalType` describes the static type of classical data. Through explicit types, Cqlib can check at construction time whether the types of expressions, variables, control flow conditions and measurement results match.

| Constructor | Description |
| --- | --- |
| `ClassicalType::Bit` | A single bit, usually representing a single-qubit measurement result, with the value 0 or 1. |
| `ClassicalType::Bool` | A logical Boolean value, used for conditions such as `if` and `while`. |
| `ClassicalType::uint(width)` | Create an unsigned integer type with the given bit width; return `None` when the width is invalid. |
| `ClassicalType::bit_vec(width)` | Create a bit vector type with the given bit width. |

The common methods are as follows:

| Method | Description |
| --- | --- |
| `width()` | Return the bit width of the type. |
| `zero_literal()` | Return the typed 0 literal expression of this type. |
| `one_literal()` | Return the typed 1 literal expression of this type. |
| `measurement_width()` | Return the corresponding measurement width when this type can serve as a measurement result type. |

```rust
use cqlib_core::circuit::ClassicalType;

let bit_ty = ClassicalType::Bit;
let bool_ty = ClassicalType::Bool;
let u8_ty = ClassicalType::uint(8).expect("valid uint width");
let bv4_ty = ClassicalType::bit_vec(4).expect("valid bit vector width");

assert_eq!(bit_ty.width(), 1);
assert_eq!(bool_ty.width(), 1);
assert_eq!(u8_ty.width(), 8);
assert_eq!(bv4_ty.width(), 4);
```

In control flow, `Bit` and `Bool` should be clearly distinguished: `Bit` is closer to a low-level bit value, while `Bool` is closer to a logical decision. To use a single-bit measurement result as a condition, it usually needs to be converted to a `Bool` expression first.

---

## `ClassicalVar` and `ClassicalValue`

`ClassicalVar` and `ClassicalValue` are both classical data handles, but their semantics differ.

| Type | Creation | Description |
| --- | --- | --- |
| `ClassicalVar` | `Circuit::var(ty)` | A mutable classical storage handle, updatable through `Circuit::store()`. |
| `ClassicalValue` | `Circuit::measure()` / `Circuit::measure_bits()` | An immutable classical value handle, usually produced by measurement, with SSA style. |

Both provide the following basic interfaces:

- `index()`
- `circuit_id()`
- `ty()`
- `expr()`

Among them, `ClassicalVar` also provides `id()`, used to represent a stable identity composed of a `CircuitId` and a variable index.

```rust
use cqlib_core::circuit::{Circuit, ClassicalType};

let mut circuit = Circuit::new(1);

let flag = circuit.var(ClassicalType::Bool);
let flag_expr = flag.expr();

assert_eq!(flag.ty(), ClassicalType::Bool);
assert_eq!(flag.circuit_id(), circuit.id());
```

`ClassicalVar` is suitable for representing state that can be updated in control flow, such as loop flags, counters or branch state. `ClassicalValue` usually represents an immutable result produced by one measurement. Because it has SSA-style semantics, it is usually required to be defined before use, and it must not illegally escape the scope in which it is defined.

---

## `Measurement`

`Measurement` is the receipt object returned by a measurement operation, recording the measurement result value and the order of the measured qubits.

| Method | Description |
| --- | --- |
| `value()` | Return the `ClassicalValue` produced by the measurement. |
| `expr()` | Return the `ClassicalExpr` that reads the measurement result. |
| `qubits()` | Return the order of the measured qubits. |
| `width()` | Return the number of measured bits. |
| `ty()` | Return the measurement result type. |
| `check_qubits(num_qubits)` | Check whether the measured qubits are within the given state range. |
| `project(full)` | Project the result corresponding to this measurement out of a complete `Outcome`. |
| `project_basis(basis)` | Project the result corresponding to this measurement from a computational basis index. |

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

let m = circuit.measure(Qubit::new(0))?;
assert_eq!(m.width(), 1);

let expr = m.expr();

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

For a single-qubit measurement, the result is usually of type `Bit`; for a multi-qubit measurement, the result is usually `BitVec(width)`. To use a measurement result in a Boolean condition, call `to_bool()` or the corresponding type conversion interface as needed.

---

## `ClassicalExpr`

`ClassicalExpr` is a typed, side-effect-free classical expression AST. It represents classical-side computation logic, such as reading variables, reading measurement values, constructing literals, performing logical operations, comparisons, type conversions, conditional selection and bit operations.

### Creating expressions

| Method | Description |
| --- | --- |
| `ClassicalExpr::var(var)` | Read a mutable classical variable. |
| `ClassicalExpr::value(value)` | Read an immutable classical value. |
| `bool_literal(value)` | Create a `Bool` literal. |
| `bit_literal(value)` | Create a `Bit` literal. |
| `uint_literal(width, value)` | Create a `UInt` literal with the given bit width. |
| `bit_vec_literal(width, value)` | Create a `BitVec` literal with the given bit width. |

```rust
use cqlib_core::circuit::ClassicalExpr;

let truth = ClassicalExpr::bool_literal(true);
let bit_one = ClassicalExpr::bit_literal(true);
let u3 = ClassicalExpr::uint_literal(3, 5)?;
let bits = ClassicalExpr::bit_vec_literal(4, 0b1010)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### Logical and bit operations

| Method | Description |
| --- | --- |
| `try_not(expr)` | Construct a NOT expression. |
| `try_and(lhs, rhs)` | Construct an AND expression. |
| `try_or(lhs, rhs)` | Construct an OR expression. |
| `try_xor(lhs, rhs)` | Construct an XOR expression. |

Expressions also implement convenience operators such as `!`, `&`, `|` and `^`. For library code, importers or compiler passes, the `try_*` family of methods is preferable, because they can return type errors explicitly and avoid panics or hard-to-locate problems caused by type mismatches.

### Type conversion

| Method | Description |
| --- | --- |
| `bit_to_bool(expr)` | Convert `Bit` to `Bool`. |
| `bit_vec_to_uint(expr)` | Convert `BitVec` to `UInt`. |
| `to_bool(self)` | Convert `Bit` or `UInt` to `Bool`. |
| `to_uint(self)` | Convert `Bit` or `BitVec` to `UInt`. |

A common use is to convert a measured `Bit` into `Bool` and then use it as an `if` or `while` condition.

### Comparison and selection

| Method | Description |
| --- | --- |
| `eq(lhs, rhs)` | Construct an equality comparison. |
| `ne(lhs, rhs)` | Construct an inequality comparison. |
| `lt(lhs, rhs)` | Construct a less-than comparison. |
| `le(lhs, rhs)` | Construct a less-than-or-equal comparison. |
| `gt(lhs, rhs)` | Construct a greater-than comparison. |
| `ge(lhs, rhs)` | Construct a greater-than-or-equal comparison. |
| `select(condition, then_expr, else_expr)` | Construct a ternary conditional selection expression. |

A comparison result is usually a `Bool` expression and can be used directly as the condition of `if_`, `if_else` or `while_`. When constructing a comparison, the types of the expressions on the two sides should be compatible, for example a magnitude comparison between `UInt` expressions of the same bit width.

### Bit extraction and concatenation

| Method | Description |
| --- | --- |
| `extract_bit(value, index)` | Extract a single bit from a `UInt` or `BitVec`. |
| `extract_bits(value, offset, width)` | Extract a contiguous bit range. |
| `concat(parts)` | Concatenate several `BitVec` expressions. |
| `pack_bits(bits)` | Pack several `Bit` expressions into a `BitVec`. |

### Introspection and simplification

| Method | Description |
| --- | --- |
| `ty()` | Return the static type of the expression. |
| `kind()` | Return the underlying AST node. |
| `vars()` | Return the set of all mutable classical variables read by the expression. |
| `values()` | Return the set of all immutable classical values read by the expression. |
| `collect_vars(out)` | Write the classical variables read into the given set. |
| `collect_values(out)` | Write the classical values read into the given set. |
| `remap_classical_ids(var_map, value_map)` | Remap the classical handles referenced by the expression according to the mapping tables. |
| `simplified()` | Return the expression after structural simplification. |
| `is_bool_true()` / `is_bool_false()` | Determine whether it is a `Bool` literal. |
| `is_bit_true()` / `is_bit_false()` | Determine whether it is a `Bit` literal. |

---

## Expression AST types

The underlying nodes of `ClassicalExpr` are described by the following types, all of which are visible at the root of the `cqlib_core::circuit` module.

```rust
pub enum ClassicalUnaryOp { Not }
pub enum ClassicalBinaryOp { And, Or, Xor }
pub enum ClassicalCompareOp { Eq, Ne, Lt, Le, Gt, Ge }
pub enum ClassicalCast { BitToBool, BitVecToUInt }
```

```rust
pub enum ClassicalExprKind {
    Var(ClassicalVar),
    Value(ClassicalValue),
    BoolLiteral(bool),
    BitLiteral(bool),
    UIntLiteral { width: NonZeroU32, value: u128 },
    BitVecLiteral { width: NonZeroU32, value: u128 },
    Unary { op: ClassicalUnaryOp, expr: ClassicalExpr },
    Binary { op: ClassicalBinaryOp, lhs: ClassicalExpr, rhs: ClassicalExpr },
    Compare { op: ClassicalCompareOp, lhs: ClassicalExpr, rhs: ClassicalExpr },
    Cast { cast: ClassicalCast, expr: ClassicalExpr },
    Select { condition: ClassicalExpr, then_expr: ClassicalExpr, else_expr: ClassicalExpr },
    ExtractBit { value: ClassicalExpr, index: u32 },
    ExtractBits { value: ClassicalExpr, offset: u32, width: NonZeroU32 },
    Concat { parts: Box<[ClassicalExpr]> },
    PackBits { bits: Box<[ClassicalExpr]> },
}
```

| Type | Description |
| --- | --- |
| `ClassicalExprKind` | The expression AST node enum, covering variable reads, literals, operations, comparisons, type conversions and bit operations. |
| `ClassicalExprNode` | An AST node, carrying the static type of the expression together with the node content. |
| `ClassicalUnaryOp` / `ClassicalBinaryOp` | Unary and binary logical operation operators. |
| `ClassicalCompareOp` | Comparison operators. |
| `ClassicalCast` | Explicit type conversion operators. |

---

## `Circuit` high-level control flow API

`Circuit` provides closure-style control flow construction interfaces, used to construct structured control flow in a fairly natural Rust code style.

```rust
circuit.if_(condition, body)
circuit.if_else(condition, then_body, else_body)
circuit.while_(condition, body)
circuit.for_uint(var, start, stop, step, body)
circuit.switch(target, build)
circuit.break_loop()
circuit.continue_loop()
```

### 1. `if_`

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);
let m = circuit.measure(Qubit::new(0))?;

circuit.if_(m.expr().to_bool()?, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`if_` constructs a conditional structure without an `else` branch. The condition must be a `Bool` expression, and the body callback constructs the circuit fragment executed when the condition holds.

### 2. `if_else`

```rust
circuit.if_else(
    m.expr().to_bool()?,
    |then_body| {
        then_body.x(Qubit::new(1))?;
        Ok(())
    },
    |else_body| {
        else_body.z(Qubit::new(1))?;
        Ok(())
    },
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`if_else` constructs a two-way conditional branch. Both branches are committed as part of the same control flow structure; if construction of either branch fails, the whole control flow construction is rolled back.

### 3. `while_` / `for_uint` / `switch`

| Method | Description |
| --- | --- |
| `while_(condition, body)` | Construct a runtime `while` loop based on a `Bool` condition. |
| `for_uint(var, start, stop, step, body)` | Construct a half-open interval loop based on a `UInt` variable. |
| `switch(target, build)` | Construct a multi-way selection based on exact `UInt` matching. |
| `break_loop()` | Exit the nearest loop or the control flow body that permits the jump. |
| `continue_loop()` | Jump to the next iteration of the nearest loop. |

`for_uint` requires the loop variable, the start, the stop and the step to be `UInt` expressions of the same bit width; the target of `switch` must be a `UInt`, and case values must be representable in that bit width.

### 4. `switch` and `SwitchBuilder`

The callback parameter of `switch(target, build)` is a `SwitchBuilder`, used to register each case and the optional default branch in order.

| Method | Description |
| --- | --- |
| `value(value, body)` | Register one exact-match case branch. |
| `default(body)` | Register the default branch; registering it repeatedly returns an error. |

```rust
use cqlib_core::circuit::{Circuit, ClassicalType, Qubit};

let mut circuit = Circuit::new(2);
let state = circuit.var(ClassicalType::uint(2).expect("valid uint width"));

circuit.switch(state.expr(), |case| {
    case.value(0, |body| {
        body.x(Qubit::new(0))?;
        Ok(())
    })?;
    case.value(1, |body| {
        body.h(Qubit::new(1))?;
        Ok(())
    })?;
    case.default(|body| {
        body.z(Qubit::new(0))?;
        Ok(())
    })?;
    Ok(())
})?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Low-level control flow IR

Besides the high-level closure-style API, the Rust core also provides a low-level control flow IR, suitable for importers, deserializers, compiler passes and low-level tests.

| Type | Description |
| --- | --- |
| `ControlBody` | The storage-layer control flow body, containing `Vec<Operation>`. |
| `ValueControlBody` | The construction-layer control flow body, containing `Vec<ValueOperation>`. |
| `IfOp` | The conditional branch structure. |
| `WhileOp` | The `while` loop structure. |
| `ForOp` | The `UInt` half-open interval loop structure. |
| `SwitchOp` / `SwitchCase` | The `switch` multi-way branch structure. |
| `ClassicalControlOp` | The storage-layer control flow enum, containing the control flow operations above. |
| `ValueClassicalControlOp` | The construction-layer control flow enum, suitable for self-contained IR. |

`Circuit::append_control(op)` can append a storage-layer `ClassicalControlOp`. For self-contained import scenarios, `ValueClassicalControlOp` and `ValueOperation` can also be combined and handed to `Circuit::from_operations()`.

---

## Validation rules

`Circuit::validate()` and the related append procedures check the consistency of classical data and control flow structures. Typical checks include:

- `ClassicalVar` and `ClassicalValue` must belong to the current `CircuitId`;
- an immutable `ClassicalValue` must be defined before it is read;
- a value defined inside a control flow scope must not escape to an outer scope;
- `break` and `continue` may appear only inside a valid control flow body;
- after a control transfer, ordinary operations of the same body cannot continue to be appended;
- the conditions of `if` and `while` must be `Bool` expressions;
- the variable, the start, the stop and the step of `for_uint` must be `UInt` of the same bit width;
- the `switch` target must be a `UInt`, and case values must be representable in that bit width;
- the variables and values involved in a classical expression must satisfy type compatibility requirements.

For circuits that are externally imported, generated automatically by a program or constructed by hand at the low-level IR layer, an explicit call is recommended before entering later compiler stages or backend execution:

```rust
circuit.validate()?;
```

This detects handle ownership, type mismatches, scope escapes and invalid control flow jump positions as early as possible.
