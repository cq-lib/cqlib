# Unitary Gates

This page covers the C API interfaces for appending matrix-defined custom unitary gates to a circuit; for global conventions on error codes and memory management, see [Overview](../0_overview.md).

---

## circuit_unitary(ptr, label, num_qubits, matrix, qubits, qubits_len)

Appends a custom unitary gate defined by a numeric matrix to the circuit.

Parameters:

- `ptr` (`struct CCircuit *`): target circuit handle.
- `label` (`const char *`): readable gate name, used for display, visualization and IR output.
- `num_qubits` (`uintptr_t`): number of qubits n the gate acts on, at most 20.
- `matrix` (`const Complex64 *`): row-major flattened 2^n × 2^n complex matrix with 2^n·2^n `Complex64` elements; each element is a `{double re; double im}` struct with interleaved real/imaginary parts, the same layout as the `circuit_to_matrix` output.
- `qubits` (`const uint32_t *`): array of qubit indices the gate acts on.
- `qubits_len` (`uintptr_t`): length of `qubits`; must equal `num_qubits`.

Returns: `int32_t`, `0` on success; `-1` on NULL input (`ptr`, `label` or `matrix` is NULL); `-2` on out-of-bounds qubits; `-3` when the matrix is not unitary, the shape mismatches or application fails; `-4` on invalid UTF-8 in `label`; `-8` when `num_qubits` exceeds 20.

The matrix is checked for unitarity when appended: U†U must be the identity matrix within tolerance, otherwise `-3` is returned. A gate defined by a numeric matrix takes no application parameters, and `qubits` maps onto the gate's qubits in array order.

```c
#include <math.h>
#include "cqlib_c.h"

/* Custom 2x2 unitary gate: sqrt(X) */
CCircuit *c = circuit_new(1);

double s = 1.0 / sqrt(2.0);
Complex64 sqrt_x[4] = {
    {s,  0.0}, {0.0, -s},
    {0.0, -s}, {s,  0.0},
};

uint32_t qubits[1] = {0};
if (circuit_unitary(c, "sqrt_x", 1, sqrt_x, qubits, 1) != 0) {
    /* handle error */
}

circuit_free(c);
```

---

## circuit_unitary_with_params(ptr, label, num_qubits, re_exprs, im_exprs, param_names, param_names_len, qubits, qubits_len, params, params_len)

Appends a parameterized custom unitary gate defined by a symbolic matrix to the circuit.

Parameters:

- `ptr` (`struct CCircuit *`): target circuit handle.
- `label` (`const char *`): readable gate name.
- `num_qubits` (`uintptr_t`): number of qubits n the gate acts on, at most 20.
- `re_exprs` (`const char *const *`): row-major flattened array of 2^n·2^n real-part expression strings, e.g. `"cos(theta)"`, `"1"`.
- `im_exprs` (`const char *const *`): array of imaginary-part expression strings with the same layout; may be NULL, in which case all imaginary parts are zero.
- `param_names` (`const char *const *`): list of positional parameter names declaring the order in which application arguments bind to the symbols in the matrix.
- `param_names_len` (`uintptr_t`): length of `param_names`.
- `qubits` (`const uint32_t *`): array of qubit indices the gate acts on.
- `qubits_len` (`uintptr_t`): length of `qubits`; must equal `num_qubits`.
- `params` (`const struct CParameter *const *`): array of parameter handles for the application; the i-th argument binds to `param_names[i]`.
- `params_len` (`uintptr_t`): length of `params`; must equal `param_names_len`.

Returns: `int32_t`, `0` on success; `-1` on NULL input; `-2` on out-of-bounds qubits; `-3` on shape, symbol-declaration or application failure; `-4` on invalid UTF-8 or expression syntax errors; `-8` on invalid sizes (`num_qubits` exceeds 20 or `param_names_len` exceeds 65535).

Expressions use the parameter-expression syntax (the same as `param_parse`); every free symbol appearing in the matrix must be declared in `param_names`. Numeric angles are written as expressions, for example `"pi/2"`.

```c
#include "cqlib_c.h"

/* Parameterized diagonal phase gate diag(1, e^(i·theta)), applied with theta = pi/2 */
CCircuit *c = circuit_new(1);

const char *re[4] = {"1", "0", "0", "cos(theta)"};
const char *im[4] = {"0", "0", "0", "sin(theta)"};
const char *names[1] = {"theta"};

CParameter *angle = param_parse("pi/2");
const struct CParameter *params[1] = {angle};

uint32_t qubits[1] = {0};
if (circuit_unitary_with_params(c, "phase_e", 1,
                                re, im, names, 1,
                                qubits, 1, params, 1) != 0) {
    /* handle error */
}

param_free(angle);
circuit_free(c);
```

---

## Evaluating a Unitary Matrix at Parameter Values

The C ABI has no standalone unitary-gate handle, so evaluating a gate definition outside a circuit reuses the `circuit_unitary_with_params` inputs: a symbolic matrix plus the ordered formal parameter names.

### unitary_gate_matrix_for_params_len(matrix)

```c
uintptr_t unitary_gate_matrix_for_params_len(const struct CSymbolicMatrix *matrix);
```

Returns the number of `Complex64` elements (`rows * cols`) that `unitary_gate_matrix_for_params` writes for `matrix`; 0 for NULL or a non-square symbolic matrix.

### unitary_gate_matrix_for_params(matrix, param_names, param_names_len, params, params_len, buffer, buffer_len)

```c
int32_t unitary_gate_matrix_for_params(const struct CSymbolicMatrix *matrix,
                                       const char *const *param_names,
                                       uintptr_t param_names_len,
                                       const double *params,
                                       uintptr_t params_len,
                                       Complex64 *buffer,
                                       uintptr_t buffer_len);
```

Evaluates a custom unitary gate's symbolic matrix at positional numeric parameter values and writes the resulting numeric matrix to `buffer` in row-major order. The gate is built and checked exactly as in `circuit_unitary_with_params` (shape, parameter count, finite values, unitarity).

Parameters:

- `matrix` (`const struct CSymbolicMatrix *`): symbolic `2^n × 2^n` matrix (construction in [Symbolic Matrix](10_symbolic_matrix.md)).
- `param_names` (`const char *const *`): ordered formal parameter names binding the application arguments to the matrix symbols; may be NULL when `param_names_len` is 0.
- `param_names_len` (`uintptr_t`): length of `param_names`, at most 65535.
- `params` (`const double *`): numeric parameter values; may be NULL when `params_len` is 0.
- `params_len` (`uintptr_t`): length of `params`.
- `buffer` (`Complex64 *`): output buffer receiving the evaluated row-major matrix.
- `buffer_len` (`uintptr_t`): buffer length; must be at least `unitary_gate_matrix_for_params_len(matrix)`.

Returns: `int32_t`, `0` on success; `-1` on NULL input; `-3` on shape, parameter-count, non-finite-value or unitarity failure; `-4` on invalid UTF-8 parameter names; `-8` on invalid sizes (empty or non-power-of-two matrix, more than 20 qubits, or a too-small buffer).

```c
#include <math.h>
#include "cqlib_c.h"

/* Evaluate U(theta) = cos(theta)*I + i*sin(theta)*X at theta = pi/2 (gives iX) */
struct CParameter *zero = param_parse("0");
struct CParameter *cos_t = param_parse("cos(theta)");
struct CParameter *sin_t = param_parse("sin(theta)");
struct CSymbolicComplex *e00 = symbolic_complex_new(cos_t, zero);
struct CSymbolicComplex *e01 = symbolic_complex_new(zero, sin_t);
struct CSymbolicComplex *e10 = symbolic_complex_new(zero, sin_t);
struct CSymbolicComplex *e11 = symbolic_complex_new(cos_t, zero);
const struct CSymbolicComplex *elements[4] = {e00, e01, e10, e11};
struct CSymbolicMatrix *matrix = symbolic_matrix_new(2, 2, elements, 4);

const char *names[1] = {"theta"};
double params[1] = {3.141592653589793 / 2.0};

uintptr_t len = unitary_gate_matrix_for_params_len(matrix);   /* 4 */
Complex64 out[4];
if (unitary_gate_matrix_for_params(matrix, names, 1, params, 1, out, len) == 0) {
    /* out = {{0,0},{0,1},{0,1},{0,0}} -> the matrix iX */
}

symbolic_matrix_free(matrix);
symbolic_complex_free(e00);
symbolic_complex_free(e01);
symbolic_complex_free(e10);
symbolic_complex_free(e11);
param_free(zero);
param_free(cos_t);
param_free(sin_t);
```

---

## Comparison of the two definition styles

| Function | Matrix definition | Application parameters | Typical use |
| --- | --- | --- | --- |
| `circuit_unitary` | Numeric `Complex64` array | None | Fixed black-box matrices, calibration gates, test oracles. |
| `circuit_unitary_with_params` | Real/imaginary expression strings | `CParameter` array bound positionally via `param_names` | Parameterized custom matrix gates. |

---

## See also

- [Standard Gates](5_gate_standard.md): the built-in standard gate set.
- [Parameter](3_parameter.md): creating `CParameter` and the expression syntax.
- [Circuit Gates](8_gate_circuit_gate.md): composite gates defined by sub-circuits.
- [Circuit to Matrix](13_circuit_to_matrix.md): matrix-layout output matching the `circuit_unitary` input.
