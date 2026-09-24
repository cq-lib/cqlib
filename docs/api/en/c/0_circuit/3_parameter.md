# Parameter

The `CParameter*` handle is an opaque handle to a symbolic parameter expression: either a free symbol (e.g. `theta`) or a numeric constant, or an arithmetic combination such as `2 * theta + pi / 2`. This page covers expression parsing, construction, arithmetic and math functions, symbol and state queries, simplification/substitution/derivation, plus the circuit-side parameter registry, the storage form (`CCircuitParam`), resolved values (`CParameterValue`), and symbol-table queries; `circuit_assign_params`, which produces a new circuit from bindings, is covered in [Circuit](1_circuit.md). Error codes and memory conventions are described in [Overview](../0_overview.md).

---

## Expression Parsing and Evaluation

### param_parse(expr)

Parses a symbolic parameter expression and returns a newly allocated handle. Expressions support free symbol names (including Unicode symbols such as `θ`), numeric constants, and arithmetic operations (`+`, `-`, `*`, `/`, unary minus); the constants `pi`/`π` and `e` are available automatically during evaluation, and explicit bindings can override them.

- `expr` (`const char*`): NUL-terminated expression string.

Returns: a newly allocated `CParameter*`, freed with `param_free`; `NULL` when `expr` is NULL, not valid UTF-8, or cannot be parsed as a valid expression.

### param_free(ptr)

Frees a parameter handle. Passing NULL is allowed.

- `ptr` (`CParameter*`): parameter handle.

### param_evaluate(ptr, bindings)

Evaluates the expression to a number under the given bindings.

- `ptr` (`const CParameter*`): parameter handle.
- `bindings` (`const char*`): binding string of the form `"name:value,name2:value2"`; NULL or a malformed string (a value that is not a finite floating-point number) is treated as no bindings.

Returns: the evaluated result; `0.0` when `ptr` is NULL, the expression still contains unbound symbols, or evaluation fails (including a NaN result).

```c
CParameter *expr = param_parse("2 * theta + pi / 2");
double v = param_evaluate(expr, "theta:0.5");   /* v == 1.0 + pi/2 */
param_free(expr);
```

---

## Expression Construction and Arithmetic

### param_symbol(name)

Constructs a free-symbol parameter from a symbol name.

- `name` (`const char*`): NUL-terminated symbol name, including Unicode symbols such as `θ`.

Returns: a newly allocated `CParameter*`, freed with `param_free`; `NULL` when `name` is NULL, not valid UTF-8, or is an invalid symbol name.

### param_pi() / param_e()

Constructs the built-in constants π and e. Both are constant expressions (`param_is_constant` returns 1) and evaluate without bindings.

Returns: a newly allocated `CParameter*`.

### param_from_double(value)

Constructs a constant parameter from a number.

- `value` (`double`): the constant value.

Returns: a newly allocated `CParameter*`; `NULL` when `value` is not finite.

### param_add(a, b) / param_sub(a, b) / param_mul(a, b) / param_div(a, b)

Adds, subtracts, multiplies, or divides two expressions and returns the combined expression. Division does not check for a zero divisor at construction time; evaluation may produce a non-finite value.

- `a`, `b` (`const CParameter*`): operand handles.

Returns: a newly allocated `CParameter*`; `NULL` when either input is NULL.

### param_neg(a)

Unary negation: returns `-a`.

- `a` (`const CParameter*`): operand handle.

Returns: a newly allocated `CParameter*`; `NULL` when `a` is NULL.

### param_pow(a, exp)

Exponentiation: returns `a^exp`.

- `a`, `exp` (`const CParameter*`): base and exponent handles.

Returns: a newly allocated `CParameter*`; `NULL` when either input is NULL.

```c
CParameter *theta = param_symbol("theta");
CParameter *amp = param_mul(param_from_double(0.5), param_sin(theta));
CParameter *shifted = param_add(amp, param_pi());   /* 0.5*sin(theta) + pi */
param_free(shifted);
param_free(amp);
param_free(theta);
```

---

## Math Functions

The following unary math functions share one signature: `param_xxx(const CParameter* a)` returns a newly allocated `CParameter*` (`NULL` when `a` is NULL) representing the math function applied to the expression. No domain checking happens at construction time; evaluation occurs during `param_evaluate` or simplification.

### param_abs / param_sqrt / param_exp / param_ln

Absolute value, square root, natural exponential, natural logarithm.

### param_sin / param_cos / param_tan / param_asin / param_acos / param_atan

Trigonometric and inverse trigonometric functions.

### param_sinh / param_cosh / param_tanh

Hyperbolic functions.

### param_floor / param_ceil / param_round

Floor, ceiling, and rounding.

### param_log(a, base)

Logarithm to an explicit base.

- `a` (`const CParameter*`): the antilogarithm handle.
- `base` (`const CParameter*`): the base handle.

Returns: a newly allocated `CParameter*`; `NULL` when either input is NULL.

---

## Symbol and State Queries

### param_symbols_len(a)

Returns the number of free symbols in the expression (built-in constants such as π and e are excluded).

- `a` (`const CParameter*`): parameter handle.

Returns: the number of free symbols; `0` when `a` is NULL.

### param_symbols(a, out, len)

Two-step read of free symbol names: call `param_symbols_len` first to size the buffer, then fill `out` with the names sorted by name.

- `a` (`const CParameter*`): parameter handle.
- `out` (`char**`): output buffer, may be NULL (count only); each written element is a newly allocated C string freed with `cqlib_string_free`.
- `len` (`uintptr_t`): buffer length.

Returns: the total number of free symbols; `-1` on NULL; `-8` when the buffer is too small (nothing is written).

### param_as_symbol(a)

Returns the symbol name when the expression is exactly a single free symbol.

Returns: a newly allocated C string freed with `cqlib_string_free`; `NULL` when the expression is not a plain symbol or the input is NULL.

### param_is_constant(a)

Checks whether the expression contains no free symbols (built-in constants such as π and e count as known quantities).

Returns: `1` constant; `0` free symbols present; `-1` on NULL.

### param_is_exact_zero(a) / param_is_zero(a) / param_is_one(a)

- `param_is_exact_zero`: returns 1 only when the expression contains no free symbols and evaluates to exactly 0; returns 0 when free symbols are present.
- `param_is_zero`/`param_is_one`: evaluate directly without bindings and compare against 0 / 1 with tolerance `f64::EPSILON`; return 0 when the expression cannot be evaluated (free symbols present).

Returns: `1`/`0`; `-1` on NULL.

---

## Simplification, Substitution, and Derivatives

### param_simplify(a)

Applies domain-safe symbolic simplification to the expression.

Returns: a newly allocated `CParameter*`; `NULL` on NULL input or when simplification fails.

### param_canonicalized(a)

Simplifies the expression; when the result contains no free symbols it is further folded into a numeric node (fails when the constant evaluates to a non-finite value).

Returns: a newly allocated `CParameter*`; `NULL` on NULL input or when simplification fails.

### param_replace(a, symbol, replacement)

Replaces every occurrence of `symbol` with `replacement` and returns the new expression.

- `a` (`const CParameter*`): parameter handle.
- `symbol` (`const char*`): the symbol name to replace.
- `replacement` (`const CParameter*`): the replacement parameter handle.

Returns: a newly allocated `CParameter*`; `NULL` on NULL input or when `symbol` is not valid UTF-8.

### param_substitute_many(a, names, values, len)

Substitutes several symbols in one call and simplifies the result. `names` and `values` are parallel arrays of length `len`.

- `a` (`const CParameter*`): parameter handle.
- `names` (`const char* const*`): symbol-name array; a NULL element is treated as an error.
- `values` (`const CParameter* const*`): parallel array of replacement handles.
- `len` (`uintptr_t`): array length; when 0, a clone of `a` is returned.

Returns: a newly allocated `CParameter*`; `NULL` on NULL input, NULL array elements, or names that are not valid UTF-8.

### param_derivative(a, var)

Returns the symbolic partial derivative with respect to `var`.

- `a` (`const CParameter*`): parameter handle.
- `var` (`const char*`): the differentiation variable name.

Returns: a newly allocated `CParameter*`; `NULL` on NULL input, non-UTF-8 `var`, or when differentiation fails.

```c
CParameter *expr = param_parse("2 * theta");
CParameter *deriv = param_derivative(expr, "theta");   /* deriv == 2 */
double v = param_evaluate(deriv, NULL);                /* v == 2.0 */
param_free(deriv);
param_free(expr);
```

---

## Equivalence Checks

### param_provably_equal(a, b, tolerance)

Conservatively checks whether two expressions are equal within `tolerance` (directly equal, or both evaluate without bindings and differ by at most the tolerance).

- `a`, `b` (`const CParameter*`): handles to compare.
- `tolerance` (`double`): numeric tolerance.

Returns: `1` equal; `0` not equal or unprovable; `-1` on NULL.

### param_provably_equal_modulo(a, b, modulus, tolerance)

Conservatively checks whether `a - b` is an integer multiple of `modulus` (direct difference test or modulo reduction, within `tolerance`).

- `a`, `b` (`const CParameter*`): handles to compare.
- `modulus` (`const CParameter*`): modulus handle.
- `tolerance` (`double`): numeric tolerance.

Returns: `1` equivalent; `0` not equivalent or unprovable; `-1` on NULL.

```c
CParameter *p = param_pi();
CParameter *three_pi = param_mul(param_from_double(3.0), param_pi());
CParameter *mod = param_mul(param_from_double(2.0), param_pi());
/* p and three_pi differ by an integer multiple of 2*pi */
int32_t eq = param_provably_equal_modulo(p, three_pi, mod, 1e-9);   /* eq == 1 */
param_free(mod);
param_free(three_pi);
param_free(p);
```

---

## Circuit Parameter Registry

The circuit maintains a parameter table: after canonicalization and interning, symbolic parameters appear in operations and the global phase in one of two storage forms — a fixed value or a table index. Interning registers the expression itself and does not mark the parameter as referenced.

### circuit_add_parameter(ptr, param)

Clones a symbolic parameter and interns it into the circuit's parameter table. Interning only registers the expression and its symbols, without marking it as used.

- `ptr` (`CCircuit*`): circuit handle.
- `param` (`const CParameter*`): parameter handle.

Returns: `0` on success; `-1` for NULL.

### circuit_parameters_len(ptr)

Returns the number of parameters interned in the circuit's parameter table.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the interned parameter count; 0 for NULL.

### circuit_parameters(ptr, out, len)

Two-step read of the parameter table: call `circuit_parameters_len` first to size the buffer, then this copies cloned parameter handles into `out`.

- `ptr` (`const CCircuit*`): circuit handle.
- `out` (`CParameter**`): output buffer, may be NULL (count query only); each written element is a newly allocated `CParameter*` that the caller frees individually with `param_free`.
- `len` (`uintptr_t`): buffer length.

Returns: the total number of interned parameters.

---

## Storage Form and Resolved Values

Parameters inside operations and the global phase use the storage form `CCircuitParam`; when reading them back you can either restore the full symbolic expression or resolve the value into a fixed number or a symbolic parameter.

### CCircuitParam

| Field | Type | Description |
| --- | --- | --- |
| `tag` | `uint8_t` | `CIRCUIT_PARAM_TAG_FIXED` (0, fixed value) or `CIRCUIT_PARAM_TAG_INDEX` (1, parameter-table index). |
| `index` | `uint32_t` | Parameter-table index, valid when `tag` is `INDEX`. |
| `value` | `double` | Fixed value, valid when `tag` is `FIXED`. |

### CParameterValue

| Field | Type | Description |
| --- | --- | --- |
| `tag` | `uint8_t` | `PARAMETER_VALUE_TAG_FIXED` (0, fixed value) or `PARAMETER_VALUE_TAG_PARAM` (1, symbolic parameter). |
| `value` | `double` | Fixed value, valid when `tag` is `FIXED`. |
| `param` | `CParameter*` | Newly allocated symbolic parameter handle valid when `tag` is `PARAM`; free it with `param_free`; NULL in the fixed case. |

### circuit_map_param(ptr, param, out)

Canonicalizes a symbolic parameter and interns it into the circuit's parameter table, writing out its storage form (fixed value or table index).

- `ptr` (`CCircuit*`): circuit handle.
- `param` (`const CParameter*`): parameter handle.
- `out` (`CCircuitParam*`): storage form written out.

Returns: `0` on success; `-1` for NULL; `-3` on evaluation failure.

### circuit_resolve_parameter(ptr, param)

Restores a storage-form parameter back into the full symbolic expression.

- `ptr` (`const CCircuit*`): circuit handle.
- `param` (`const CCircuitParam*`): storage form.

Returns: a newly allocated `CParameter*`, freed with `param_free`; `NULL` on NULL input, invalid tag, or resolution failure.

### circuit_parameter_value(ptr, param, out)

Resolves the current value of a storage-form parameter without interning anything new.

- `ptr` (`const CCircuit*`): circuit handle.
- `param` (`const CCircuitParam*`): storage form.
- `out` (`CParameterValue*`): resolved value written out; in the symbolic case, free the `param` field with `param_free`.

Returns: `0` on success; `-1` for NULL; `-3` for an invalid table index; `-8` for an invalid tag.

```c
CCircuit *qc = circuit_new(1);
CParameter *theta = param_parse("theta");
circuit_rx_param(qc, 0, theta);

CCircuitParam storage;
CParameterValue value;
if (circuit_map_param(qc, theta, &storage) == 0 &&
    circuit_parameter_value(qc, &storage, &value) == 0) {
    if (value.tag == PARAMETER_VALUE_TAG_PARAM) {
        param_free(value.param);   /* symbolic case: free the handle */
    }
}
param_free(theta);
circuit_free(qc);
```

---

## Symbol Table Queries

The circuit distinguishes two symbol sets: the registry (which may include symbols interned but no longer referenced) and the symbols actually referenced by the executable IR (the global phase and operation parameters, including nested control-flow bodies).

### circuit_symbols_len(ptr)

Returns the number of symbol names in the registry; the registry may include symbols that are interned but no longer referenced by the circuit's operations.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the symbol count; 0 for NULL.

### circuit_symbols(ptr, out, len)

Two-step read of the registered symbol names: each written element is a freshly allocated C string the caller frees with `cqlib_string_free`.

- `ptr` (`const CCircuit*`): circuit handle.
- `out` (`char**`): output buffer, may be NULL (count query only).
- `len` (`uintptr_t`): buffer length.

Returns: the total number of registered symbols.

### circuit_used_symbols_len(ptr)

Returns the number of symbols referenced by the circuit's executable IR.

- `ptr` (`const CCircuit*`): circuit handle.

Returns: the symbol count; 0 for NULL.

### circuit_used_symbols(ptr, out, len)

Two-step read of the IR-referenced symbol names: each written element is a freshly allocated C string the caller frees with `cqlib_string_free`.

- `ptr` (`const CCircuit*`): circuit handle.
- `out` (`char**`): output buffer, may be NULL (count query only).
- `len` (`uintptr_t`): buffer length.

Returns: the total number of used symbols.

### circuit_uses_symbol(ptr, name)

Checks whether a symbol is referenced by the circuit's executable IR (the global phase and operation parameters, including nested control-flow bodies).

- `ptr` (`const CCircuit*`): circuit handle.
- `name` (`const char*`): symbol name.

Returns: `1` when used; `0` when not; `-1` for NULL; `-4` when the symbol name is not valid UTF-8.

```c
uintptr_t n = circuit_used_symbols_len(qc);
char **names = malloc(n * sizeof(char *));
circuit_used_symbols(qc, names, n);
for (uintptr_t i = 0; i < n; ++i) {
    if (circuit_uses_symbol(qc, names[i]) == 1) {
        /* names[i] is referenced by the IR */
    }
    cqlib_string_free(names[i]);
}
free(names);
```
