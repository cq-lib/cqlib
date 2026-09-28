# Classical Variables and Control Flow

This page covers the C API functions for classical data and structured control flow: allocating and inspecting classical variables, building and introspecting classical expressions, measurement declarations, classical storage, and callback-driven `if` / `if-else` / `while` / `for` / `switch` control flow. See [Overview](../0_overview.md) for the error-code and memory-management conventions.

---

## Type tags and snapshot structs

The static type of classical data is described by a (tag, width) pair carried in a `CClassicalType` snapshot:

```c
typedef struct CClassicalType {
  uint32_t tag;    /* one of CQLIB_CLASSICAL_TYPE_* */
  uint32_t width;  /* width in bits, always 1 for Bit/Bool */
} CClassicalType;
```

| Type tag | Value | Type | Description |
| --- | --- | --- | --- |
| `CQLIB_CLASSICAL_TYPE_BIT` | 0 | `Bit` | A single bit with value 0 or 1, typically a single-qubit measurement result. |
| `CQLIB_CLASSICAL_TYPE_BOOL` | 1 | `Bool` | A logical Boolean used for `if` / `while` conditions. |
| `CQLIB_CLASSICAL_TYPE_UINT` | 2 | `UInt(width)` | An unsigned integer of the given width. |
| `CQLIB_CLASSICAL_TYPE_BIT_VEC` | 3 | `BitVec(width)` | A bit vector of the given width. |

Immutable classical values read by an expression are reported as `CClassicalValueInfo` snapshots:

```c
typedef struct CClassicalValueInfo {
  uint32_t index;  /* circuit-local value table index */
  uint32_t tag;    /* one of CQLIB_CLASSICAL_TYPE_* */
  uint32_t width;  /* width in bits, always 1 for Bit/Bool */
} CClassicalValueInfo;
```

The node kind of an expression is described by the tag written by `classical_expr_kind`:

| Node tag | Value | Node |
| --- | --- | --- |
| `CQLIB_CLASSICAL_EXPR_VAR` | 0 | Variable read |
| `CQLIB_CLASSICAL_EXPR_VALUE` | 1 | Immutable value read |
| `CQLIB_CLASSICAL_EXPR_BOOL_LITERAL` | 2 | `Bool` literal |
| `CQLIB_CLASSICAL_EXPR_BIT_LITERAL` | 3 | `Bit` literal |
| `CQLIB_CLASSICAL_EXPR_UINT_LITERAL` | 4 | `UInt` literal |
| `CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL` | 5 | `BitVec` literal |
| `CQLIB_CLASSICAL_EXPR_UNARY` | 6 | Unary (`not`) |
| `CQLIB_CLASSICAL_EXPR_BINARY` | 7 | Binary (`and` / `or` / `xor`) |
| `CQLIB_CLASSICAL_EXPR_COMPARE` | 8 | Compare (`eq` / `ne` / `lt` / `le` / `gt` / `ge`) |
| `CQLIB_CLASSICAL_EXPR_CAST` | 9 | Cast (`bit_to_bool` / `bit_vec_to_uint`) |
| `CQLIB_CLASSICAL_EXPR_SELECT` | 10 | Ternary select |
| `CQLIB_CLASSICAL_EXPR_EXTRACT_BIT` | 11 | Single-bit extraction |
| `CQLIB_CLASSICAL_EXPR_EXTRACT_BITS` | 12 | Contiguous bit-range extraction |
| `CQLIB_CLASSICAL_EXPR_CONCAT` | 13 | `BitVec` concatenation |
| `CQLIB_CLASSICAL_EXPR_PACK_BITS` | 14 | `Bit` packing |

---

## Classical variables

### circuit_var(ptr, ty_tag, width)

Allocates a mutable classical variable owned by the circuit; the handle is owned by the caller.

- `ptr` (`CCircuit*`): target circuit handle.
- `ty_tag` (`uint32_t`): type tag, one of `CQLIB_CLASSICAL_TYPE_*`.
- `width` (`uint32_t`): bit width, used only for `UInt` / `BitVec` (zero is rejected); ignored for `Bit` / `Bool`.

Returns a newly allocated `CClassicalVar*`; returns NULL for a NULL circuit or invalid type parameters.

### classical_var_free(ptr)

Frees a classical variable handle.

- `ptr` (`CClassicalVar*`): handle to free; NULL is allowed.

No return value.

### classical_var_id(ptr)

Returns the circuit-local stable identity id of the variable.

- `ptr` (`const CClassicalVar*`): variable handle.

Returns the `uint32_t` id, or `UINT32_MAX` for NULL.

### classical_var_index(ptr)

Returns the variable's index in the circuit's variable table.

- `ptr` (`const CClassicalVar*`): variable handle.

Returns the `uint32_t` table index, or `UINT32_MAX` for NULL.

### classical_var_ty(ptr, tag, width)

Writes the variable's type (tag + width) to the out-parameters.

- `ptr` (`const CClassicalVar*`): variable handle.
- `tag` (`uint32_t*`): out-parameter receiving a `CQLIB_CLASSICAL_TYPE_*` tag.
- `width` (`uint32_t*`): out-parameter receiving the width.

Returns 0 on success, -1 on NULL pointers.

### classical_var_expr(ptr)

Creates a new expression that reads the current runtime value of the variable.

- `ptr` (`const CClassicalVar*`): variable handle.

Returns a newly allocated `CClassicalExpr*` (free with `classical_expr_free`); NULL on NULL input.

### circuit_classical_vars_len(ptr)

Returns the number of classical variables allocated by the circuit.

- `ptr` (`const CCircuit*`): target circuit handle.

Returns the count, or 0 for NULL.

### circuit_classical_vars(ptr, buffer, len)

Two-step copy of (tag, width) snapshots for all of the circuit's classical variables into `buffer` (call `circuit_classical_vars_len` first for the length).

- `ptr` (`const CCircuit*`): target circuit handle.
- `buffer` (`CClassicalType*`): array receiving the snapshots.
- `len` (`uintptr_t`): array length, must equal `circuit_classical_vars_len`.

Returns 0 on success, -1 on NULL pointers, -8 when `len` does not equal the variable count.

### circuit_validate_classical_var(ptr, var)

Checks that a classical variable belongs to this circuit and keeps its recorded type.

- `ptr` (`const CCircuit*`): target circuit handle.
- `var` (`const CClassicalVar*`): variable handle to check.

Returns 0 on success, -1 on NULL pointers, -3 otherwise.

### circuit_validate_classical_expr(ptr, expr)

Checks that every classical variable and value read by `expr` belongs to this circuit.

- `ptr` (`const CCircuit*`): target circuit handle.
- `expr` (`const CClassicalExpr*`): expression handle to check.

Returns 0 on success, -1 on NULL pointers, -3 otherwise.

---

## Classical expression constructors

All constructors return a newly allocated `CClassicalExpr*` (free with `classical_expr_free`) and NULL on failure.

### classical_expr_free(ptr)

Frees a classical expression handle.

- `ptr` (`CClassicalExpr*`): handle to free; NULL is allowed.

No return value.

### classical_expr_bit_literal(value)

Creates a `Bit` literal expression (false = 0, true = 1).

- `value` (`bool`): literal value.

Returns the new expression handle.

### classical_expr_bool_literal(value)

Creates a `Bool` literal expression.

- `value` (`bool`): literal value.

Returns the new expression handle.

### classical_expr_uint_literal(width, lo, hi)

Creates a `UInt(width)` literal. The 128-bit value is passed as two 64-bit halves (`lo` is least-significant).

- `width` (`uint32_t`): bit width.
- `lo` (`uint64_t`): least-significant 64 bits.
- `hi` (`uint64_t`): most-significant 64 bits.

Returns the new expression handle; NULL when the width is zero, exceeds 128, or the value does not fit.

### classical_expr_bit_vec_literal(width, lo, hi)

Creates a `BitVec(width)` literal. The 128-bit packed little-endian value is passed as two 64-bit halves (`lo` is least-significant).

- `width` (`uint32_t`): bit width.
- `lo` (`uint64_t`): least-significant 64 bits of the packed value.
- `hi` (`uint64_t`): most-significant 64 bits of the packed value.

Returns the new expression handle; NULL when the width is zero, exceeds 128, or the value does not fit.

### classical_type_zero_literal(ty_tag, width)

Creates the zero literal of a classical type.

- `ty_tag` (`uint32_t`): type tag, one of `CQLIB_CLASSICAL_TYPE_*`.
- `width` (`uint32_t`): bit width, used for `UInt` / `BitVec`.

Returns the new expression handle; NULL on invalid type parameters or a width exceeding the 128-bit literal representation.

### classical_type_one_literal(ty_tag, width)

Creates the one literal of a classical type.

- `ty_tag` (`uint32_t`): type tag, one of `CQLIB_CLASSICAL_TYPE_*`.
- `width` (`uint32_t`): bit width, used for `UInt` / `BitVec`.

Returns the new expression handle; NULL on invalid type parameters or a width exceeding the 128-bit literal representation.

### classical_expr_var(var)

Creates an expression that reads the current runtime value of `var`.

- `var` (`const CClassicalVar*`): variable handle.

Returns the new expression handle; NULL on NULL input.

### classical_expr_not(expr)

Creates a `not` expression over a `Bool` or `Bit` operand.

- `expr` (`const CClassicalExpr*`): operand.

Returns the new expression handle; NULL on NULL input or an invalid operand type.

### classical_expr_and(lhs, rhs)

Creates an `and` expression over matching `Bool` or matching `Bit` operands.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or a type mismatch.

### classical_expr_or(lhs, rhs)

Creates an `or` expression over matching `Bool` or matching `Bit` operands.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or a type mismatch.

### classical_expr_xor(lhs, rhs)

Creates an `xor` expression over matching `Bool` or matching `Bit` operands.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or a type mismatch.

### classical_expr_eq(lhs, rhs)

Creates an equality comparison over same-typed operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or a type mismatch.

### classical_expr_ne(lhs, rhs)

Creates an inequality comparison over same-typed operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or a type mismatch.

### classical_expr_lt(lhs, rhs)

Creates an unsigned less-than comparison over matching `UInt` operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or invalid operand types.

### classical_expr_le(lhs, rhs)

Creates an unsigned less-than-or-equal comparison over matching `UInt` operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or invalid types.

### classical_expr_gt(lhs, rhs)

Creates an unsigned greater-than comparison over matching `UInt` operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or invalid operand types.

### classical_expr_ge(lhs, rhs)

Creates an unsigned greater-than-or-equal comparison over matching `UInt` operands, producing `Bool`.

- `lhs` (`const CClassicalExpr*`): left operand.
- `rhs` (`const CClassicalExpr*`): right operand.

Returns the new expression handle; NULL on NULL input or invalid types.

### classical_expr_select(condition, then_expr, else_expr)

Creates a ternary select `condition ? then_expr : else_expr`. The condition must be `Bool` and both branches must have the same type.

- `condition` (`const CClassicalExpr*`): condition expression.
- `then_expr` (`const CClassicalExpr*`): expression taken when the condition holds.
- `else_expr` (`const CClassicalExpr*`): expression taken otherwise.

Returns the new expression handle; NULL on NULL input or invalid operand types.

### classical_expr_extract_bit(value, index)

Extracts one bit (index 0 = least-significant) from a `UInt` or `BitVec` expression, producing `Bit`.

- `value` (`const CClassicalExpr*`): expression to extract from.
- `index` (`uint32_t`): bit index.

Returns the new expression handle; NULL on NULL input, a non-integer operand, or an out-of-bounds index.

### classical_expr_extract_bits(value, offset, width)

Extracts the contiguous bit range `[offset, offset + width)` from a `UInt` or `BitVec` expression, producing `BitVec(width)`. Offset 0 starts at the least-significant bit.

- `value` (`const CClassicalExpr*`): expression to extract from.
- `offset` (`uint32_t`): starting offset.
- `width` (`uint32_t`): range width.

Returns the new expression handle; NULL on NULL input or an invalid range.

### classical_expr_pack_bits(bits, len)

Packs single-bit expressions into a `BitVec`; the first bit becomes output bit index 0.

- `bits` (`const CClassicalExpr* const*`): array of expression handles.
- `len` (`uintptr_t`): array length.

Returns the new expression handle; NULL on NULL entries, empty input, or non-`Bit` operands.

### classical_expr_concat(parts, len)

Concatenates `Bit` / `BitVec` expressions into a larger `BitVec`; the first part occupies the least-significant output bits.

- `parts` (`const CClassicalExpr* const*`): array of expression handles.
- `len` (`uintptr_t`): array length.

Returns the new expression handle; NULL on NULL entries, empty input, or invalid part types.

### classical_expr_bit_to_bool(expr)

Explicitly casts a `Bit` expression to `Bool`.

- `expr` (`const CClassicalExpr*`): expression to cast.

Returns the new expression handle; NULL on NULL input or a non-`Bit` operand.

### classical_expr_bit_vec_to_uint(expr)

Explicitly casts a `BitVec` expression to a little-endian `UInt` of the same width.

- `expr` (`const CClassicalExpr*`): expression to cast.

Returns the new expression handle; NULL on NULL input or a non-`BitVec` operand.

### classical_expr_to_bool(expr)

Converts a `Bit` expression to `Bool` (alias of `classical_expr_bit_to_bool`).

- `expr` (`const CClassicalExpr*`): expression to convert.

Returns the new expression handle; NULL on NULL input or a non-`Bit` operand.

### classical_expr_to_uint(expr)

Converts a `BitVec` expression to a little-endian `UInt` (alias of `classical_expr_bit_vec_to_uint`).

- `expr` (`const CClassicalExpr*`): expression to convert.

Returns the new expression handle; NULL on NULL input or a non-`BitVec` operand.

### classical_expr_simplified(ptr)

Returns a structurally simplified copy of the expression. Simplification eliminates literal-driven redundancies without evaluating runtime reads.

- `ptr` (`const CClassicalExpr*`): source expression handle.

Returns a newly allocated simplified copy; NULL on NULL input.

---

## Expression queries

### classical_expr_kind(ptr, kind)

Writes the expression node kind tag (one of `CQLIB_CLASSICAL_EXPR_*`) to `kind`.

- `ptr` (`const CClassicalExpr*`): expression handle.
- `kind` (`uint32_t*`): out-parameter receiving the tag.

Returns 0 on success, -1 on NULL pointers.

### classical_expr_ty(ptr, tag, width)

Writes the expression's static type (tag + width) to the out-parameters.

- `ptr` (`const CClassicalExpr*`): expression handle.
- `tag` (`uint32_t*`): out-parameter receiving a `CQLIB_CLASSICAL_TYPE_*` tag.
- `width` (`uint32_t*`): out-parameter receiving the width.

Returns 0 on success, -1 on NULL pointers.

### classical_expr_is_bit_true(ptr)

Returns whether the expression is the `Bit` literal `true` (1).

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns `1` when yes, `0` otherwise, or `-1` for NULL.

### classical_expr_is_bit_false(ptr)

Returns whether the expression is the `Bit` literal `false` (0).

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns `1` when yes, `0` otherwise, or `-1` for NULL.

### classical_expr_is_bool_true(ptr)

Returns whether the expression is the `Bool` literal `true`.

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns `1` when yes, `0` otherwise, or `-1` for NULL.

### classical_expr_is_bool_false(ptr)

Returns whether the expression is the `Bool` literal `false`.

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns `1` when yes, `0` otherwise, or `-1` for NULL.

### classical_expr_values_len(ptr)

Returns the number of distinct immutable classical values read by the expression.

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns the count, or 0 for NULL.

### classical_expr_values(ptr, buffer, len)

Two-step copy of (index, tag, width) snapshots for every immutable classical value read by the expression into `buffer` (call `classical_expr_values_len` first for the length).

- `ptr` (`const CClassicalExpr*`): expression handle.
- `buffer` (`CClassicalValueInfo*`): array receiving the snapshots.
- `len` (`uintptr_t`): array length, must equal `classical_expr_values_len`.

Returns 0 on success, -1 on NULL pointers, -8 when `len` does not equal the count.

### classical_expr_vars_len(ptr)

Returns the number of distinct classical variables read by the expression.

- `ptr` (`const CClassicalExpr*`): expression handle.

Returns the count, or 0 for NULL.

### classical_expr_vars(ptr, buffer, len)

Two-step fill of `buffer` with owned handles for every distinct classical variable read by the expression, in ascending id order (call `classical_expr_vars_len` first for the length). Each returned handle must be released with `classical_var_free`.

- `ptr` (`const CClassicalExpr*`): expression handle.
- `buffer` (`CClassicalVar**`): array receiving owned handles.
- `len` (`uintptr_t`): array length, must equal `classical_expr_vars_len`.

Returns 0 on success, -1 on NULL pointers, -8 when `len` does not equal the count.

### classical_expr_remap_classical_ids(ptr, old_var_ids, new_var_ids, var_len, old_value_indices, new_value_indices, value_len)

Returns a copy of the expression with circuit-local classical ids remapped. `old_var_ids[i]` maps the variable with that id to id `new_var_ids[i]`, and `old_value_indices[i]` maps the immutable value with that index to index `new_value_indices[i]`. Entries whose old id does not occur in the expression are ignored, but every variable and value actually read by the expression must be covered, otherwise the remap fails.

- `ptr` (`const CClassicalExpr*`): source expression handle.
- `old_var_ids` (`const uint32_t*`): array of old variable ids.
- `new_var_ids` (`const uint32_t*`): array of new variable ids, paired by index.
- `var_len` (`uintptr_t`): length of the variable mapping arrays.
- `old_value_indices` (`const uint32_t*`): array of old immutable value indices.
- `new_value_indices` (`const uint32_t*`): array of new value indices, paired by index.
- `value_len` (`uintptr_t`): length of the value mapping arrays.

Returns the new expression handle (free with `classical_expr_free`); NULL on NULL inputs (including NULL arrays for non-zero lengths) or a missing mapping.

---

## Measurement declarations

### circuit_measure_bits(ptr, qubits, len)

Measures several qubits and returns an expression reading the immutable `BitVec` result. The first qubit maps to bit index 0 (least-significant).

- `ptr` (`CCircuit*`): target circuit handle.
- `qubits` (`const uint32_t*`): array of qubit ids.
- `len` (`uintptr_t`): number of qubits.

Returns a newly allocated `CClassicalExpr*`; NULL on error (NULL pointers, empty list, qubit out of bounds).

### circuit_measure_into(ptr, qubit, target)

Measures one qubit and stores the result into `target` (a `Bit` variable); returns an expression reading the immutable measurement value.

- `ptr` (`CCircuit*`): target circuit handle.
- `qubit` (`uint32_t`): qubit id to measure.
- `target` (`const CClassicalVar*`): `Bit` variable handle receiving the result.

Returns a newly allocated `CClassicalExpr*`; NULL on error (NULL pointers, qubit out of bounds, wrong target type).

### circuit_measure_bits_into(ptr, qubits, len, target)

Measures several qubits and stores the `BitVec` result into `target`. The target width must equal the number of qubits.

- `ptr` (`CCircuit*`): target circuit handle.
- `qubits` (`const uint32_t*`): array of qubit ids.
- `len` (`uintptr_t`): number of qubits.
- `target` (`const CClassicalVar*`): `BitVec` variable handle receiving the result.

Returns a `CClassicalExpr*` reading the immutable measurement value; NULL on error.

---

## Classical storage

### circuit_store(ptr, target, value)

Stores the runtime value of `value` into the mutable variable `target`. The expression type must match the target type.

- `ptr` (`CCircuit*`): target circuit handle.
- `target` (`const CClassicalVar*`): variable to write.
- `value` (`const CClassicalExpr*`): expression providing the value.

Returns 0 on success, -1 on NULL pointers, -3 on validation failure.

---

## Control flow

Control-flow functions build branch and loop bodies through a C callback:

```c
typedef int32_t (*CqlibBodyFn)(struct CCircuit *circuit, void *user_data);
```

Callback contract:

- `circuit` is the same `CCircuit` pointer that was passed to the control-flow function and is valid only for the duration of the callback; body operations must be appended through it with the regular circuit functions (`circuit_h`, `circuit_x`, `circuit_measure`, nested control-flow builders, ...).
- `user_data` is passed through untouched, is never dereferenced by the bindings, and may be NULL.
- Return 0 on success. Any non-zero return aborts the whole control operation: everything the callback appended is rolled back and the value is propagated to the caller of the control-flow function. Returning the error code of a failed gate call (for example `return circuit_x(circuit, 0);`) is the idiomatic pattern.

### circuit_if(ptr, condition, body_fn, user_data)

Appends a structured `if` operation controlled by a boolean classical expression. `body_fn` is invoked once; every operation it appends becomes the `then` body.

- `ptr` (`CCircuit*`): target circuit handle.
- `condition` (`const CClassicalExpr*`): condition expression.
- `body_fn` (`CqlibBodyFn`): callback building the branch body.
- `user_data` (`void*`): user pointer passed to the callback; may be NULL.

Returns 0 on success, -1 for NULL arguments, -3 when the condition fails validation (for example a non-`Bool` type or handles from another circuit), or the non-zero status returned by `body_fn` after rollback.

### circuit_if_else(ptr, condition, then_fn, else_fn, user_data)

Appends a structured `if` / `else` operation controlled by a boolean classical expression. `then_fn` runs first, then `else_fn`; each callback's appended operations become the corresponding branch body.

- `ptr` (`CCircuit*`): target circuit handle.
- `condition` (`const CClassicalExpr*`): condition expression.
- `then_fn` (`CqlibBodyFn`): callback building the then body.
- `else_fn` (`CqlibBodyFn`): callback building the else body.
- `user_data` (`void*`): user pointer passed to both callbacks; may be NULL.

Returns 0 on success, -1 for NULL arguments, -3 on validation failure, or the non-zero status returned by either callback after rollback.

### circuit_while(ptr, condition, body_fn, user_data)

Appends a structured `while` loop controlled by a boolean classical expression. The condition is re-evaluated at runtime. `circuit_break_loop` / `circuit_continue_loop` may be called from the callback, but only as the final operation of the body (terminal control transfer).

- `ptr` (`CCircuit*`): target circuit handle.
- `condition` (`const CClassicalExpr*`): loop condition expression.
- `body_fn` (`CqlibBodyFn`): callback building the loop body.
- `user_data` (`void*`): user pointer passed to the callback; may be NULL.

Returns 0 on success, -1 for NULL arguments, -3 on validation failure, or the non-zero status returned by `body_fn` after rollback.

### circuit_for_uint(ptr, var, start, stop, step, body_fn, user_data)

Appends an unsigned runtime range loop with half-open `[start, stop)` semantics. `var` must be a `UInt` variable of this circuit; `start`, `stop`, and `step` must be `UInt` expressions of the same width. When the body needs to read the loop variable, rebuild its expression from `var` with `classical_expr_var`.

- `ptr` (`CCircuit*`): target circuit handle.
- `var` (`const CClassicalVar*`): loop variable.
- `start` (`const CClassicalExpr*`): start expression.
- `stop` (`const CClassicalExpr*`): stop expression (exclusive).
- `step` (`const CClassicalExpr*`): step expression.
- `body_fn` (`CqlibBodyFn`): callback building the loop body.
- `user_data` (`void*`): user pointer passed to the callback; may be NULL.

Returns 0 on success, -1 for NULL arguments, -3 on validation failure (type mismatch or foreign handles), or the non-zero status returned by `body_fn` after rollback.

### circuit_switch(ptr, target, case_values_lo, case_values_hi, case_bodies, case_count, default_body, user_data)

Appends a structured exact-value `switch` operation over a `UInt` expression. Case values are 128-bit quantities split into two 64-bit halves: `value = lo | (hi << 64)`. Every value must fit the target width and appear at most once; `default_body` may be NULL, in which case the switch has no default branch. All callbacks receive the same `user_data`.

- `ptr` (`CCircuit*`): target circuit handle.
- `target` (`const CClassicalExpr*`): `UInt` target expression.
- `case_values_lo` (`const uint64_t*`): least-significant 64 bits of each case value.
- `case_values_hi` (`const uint64_t*`): most-significant 64 bits of each case value.
- `case_bodies` (`const CqlibBodyFn*`): callback array matching the case values.
- `case_count` (`uintptr_t`): number of cases.
- `default_body` (`CqlibBodyFn`): default-branch callback; may be NULL.
- `user_data` (`void*`): user pointer passed to all callbacks.

Returns 0 on success, -1 for NULL arguments (including a NULL case body when `case_count > 0`), -3 on validation failure (non-`UInt` target, duplicate or out-of-range case values), or the first non-zero status returned by a callback after rollback.

### circuit_break_loop(ptr)

Appends a `break` to the nearest enclosing loop or switch body. Legal only inside a `circuit_while`, `circuit_for_uint`, or `circuit_switch` body callback (or a body nested inside one), and only as the final operation of that body.

- `ptr` (`CCircuit*`): target circuit handle.

Returns 0 on success, -1 for a NULL circuit, -3 when no enclosing loop or switch body is active or the transfer is not terminal.

### circuit_continue_loop(ptr)

Appends a `continue` to the nearest enclosing loop body. Legal only inside a `circuit_while` or `circuit_for_uint` body callback (or a body nested inside one), and only as the final operation of that body.

- `ptr` (`CCircuit*`): target circuit handle.

Returns 0 on success, -1 for a NULL circuit, -3 when no enclosing loop body is active or the transfer is not terminal.

### circuit_append_control(ptr, source, index)

Appends a validated copy of the control-flow operation stored at `index` in `source`'s operation list to `ptr`. The same circuit may be passed as both `ptr` and `source`, which duplicates one of its control operations. Cross-circuit copies require the operation to be self-contained (no classical handles from `source`).

- `ptr` (`CCircuit*`): target circuit handle.
- `source` (`const CCircuit*`): source circuit handle.
- `index` (`uintptr_t`): index into the source operation list.

Returns 0 on success, -1 for NULL arguments, -8 when `index` is out of range or the operation at `index` is not a control-flow operation, -3 when the copy fails validation (foreign classical handles, illegal `break` / `continue` scoping, or unknown qubits).

---

## Complete example

Declare a classical variable, build a boolean expression, then add an `if` / `else` branch:

```c
#include <stdio.h>
#include "cqlib_c.h"

static int32_t then_body(struct CCircuit *circuit, void *user_data) {
    (void)user_data;
    return circuit_x(circuit, 1);   /* condition holds: apply X on qubit 1 */
}

static int32_t else_body(struct CCircuit *circuit, void *user_data) {
    (void)user_data;
    return circuit_z(circuit, 1);   /* condition fails: apply Z on qubit 1 */
}

int main(void) {
    struct CCircuit *circuit = circuit_new(2);
    if (circuit == NULL) {
        return 1;
    }

    /* 1. Declare a Bool classical variable */
    struct CClassicalVar *flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);

    /* 2. Build the boolean expression: flag == true */
    struct CClassicalExpr *flag_read = classical_expr_var(flag);
    struct CClassicalExpr *true_lit = classical_expr_bool_literal(true);
    struct CClassicalExpr *cond = classical_expr_eq(flag_read, true_lit);

    /* 3. if / else branches */
    int32_t status = circuit_if_else(circuit, cond, then_body, else_body, NULL);
    if (status != 0) {
        fprintf(stderr, "circuit_if_else failed: %d\n", status);
    }

    /* Free the expression and variable handles */
    classical_expr_free(cond);
    classical_expr_free(true_lit);
    classical_expr_free(flag_read);
    classical_var_free(flag);
    circuit_free(circuit);
    return status;
}
```

See [Circuit](1_circuit.md) for the circuit constructors (`circuit_new` / `circuit_free`) and [Standard Gates](5_gate_standard.md) for the gate functions (`circuit_x` / `circuit_z`).
