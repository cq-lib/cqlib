# Symbolic Matrix

This page covers the C API functions for symbolic matrices: symbolic complex values, matrix construction, shape and element queries, simplification, symbol substitution, evaluation under parameter bindings, in-place gate application (the `apply_*` family), and global-phase equivalence checks at both the matrix and the circuit level. See [Overview](../0_overview.md) for the error-code and memory-management conventions.

---

## Core concepts

A symbolic matrix (opaque `CSymbolicMatrix*` handle) has one symbolic complex number per element: its real and imaginary parts are parameter expressions that may contain unbound symbolic parameters. Unlike numeric matrix conversion, symbolic conversion preserves the parameter expressions of the circuit; concrete values can later be substituted through a bindings string.

Numeric evaluation results are written as `Complex64` structs:

```c
typedef struct Complex64 {
  double re;  /* real part */
  double im;  /* imaginary part */
} Complex64;
```

Every function returning a `CSymbolicMatrix*` allocates a new handle; free it with `symbolic_matrix_free`. Functions returning a `CSymbolicComplex*` allocate a new symbolic complex handle; free it with `symbolic_complex_free`.

---

## Symbolic complex values

Each matrix element is a symbolic complex value (opaque `CSymbolicComplex*` handle): its real and imaginary parts are parameter expressions. The functions in this section are the element-level API; they also supply elements to `symbolic_matrix_new` and the `apply_*` permutation/diagonal gates.

### symbolic_complex_new(re, im)

Builds a symbolic complex value from real and imaginary parameter expressions.

- `re` (`const CParameter*`): real-part expression handle.
- `im` (`const CParameter*`): imaginary-part expression handle.

Returns a newly allocated `CSymbolicComplex*`; NULL when either input is NULL.

### symbolic_complex_zero() / symbolic_complex_one() / symbolic_complex_i()

Return the additive identity `0 + 0i`, the multiplicative identity `1 + 0i`, and the imaginary unit `0 + 1i`.

No parameters; each returns a newly allocated `CSymbolicComplex*`.

### symbolic_complex_from_real(re)

Builds a symbolic complex value from a real expression with zero imaginary part.

- `re` (`const CParameter*`): real-part expression handle.

Returns a newly allocated `CSymbolicComplex*`; NULL on NULL input.

### symbolic_complex_from_complex(re, im)

Builds a symbolic complex constant from a pair of floating-point values.

- `re` (`double`): real part.
- `im` (`double`): imaginary part.

Returns a newly allocated `CSymbolicComplex*`; NULL when either component is not finite (NaN/Inf).

### symbolic_complex_exp_i(theta)

Builds `exp(i*theta)`, i.e. `cos(theta) + i*sin(theta)`.

- `theta` (`const CParameter*`): phase expression handle.

Returns a newly allocated `CSymbolicComplex*`; NULL on NULL input.

### symbolic_complex_re(ptr) / symbolic_complex_im(ptr)

Return a clone of the real/imaginary part expression (free with `param_free`).

- `ptr` (`const CSymbolicComplex*`): symbolic complex handle.

Returns a newly allocated `CParameter*`; NULL on NULL input.

### symbolic_complex_evaluate(ptr, bindings, out)

Evaluates the symbolic complex value under parameter bindings.

- `ptr` (`const CSymbolicComplex*`): symbolic complex handle.
- `bindings` (`const char*`): bindings string formatted as `"name:value,..."` (NULL means no bindings).
- `out` (`Complex64*`): receives the result.

Returns 0 on success, -1 on NULL input, -3 on unevaluable symbols, -4 on a malformed bindings string.

### symbolic_complex_simplify(ptr)

Simplifies the real and imaginary parts and returns the simplified result.

- `ptr` (`const CSymbolicComplex*`): source handle.

Returns a newly allocated `CSymbolicComplex*`; NULL on NULL input or simplification failure.

### symbolic_complex_replace(ptr, symbol, replacement)

Replaces every occurrence of `symbol` in both parts with `replacement` and returns the substituted result.

- `ptr` (`const CSymbolicComplex*`): source handle.
- `symbol` (`const char*`): symbol name.
- `replacement` (`const CParameter*`): replacement expression handle.

Returns a newly allocated `CSymbolicComplex*`; NULL on NULL input or a non-UTF-8 symbol name.

### symbolic_complex_is_zero_exact(ptr) / symbolic_complex_is_one_exact(ptr)

Return whether both parts are **exactly** zero/one (no simplification; constant structure only).

- `ptr` (`const CSymbolicComplex*`): symbolic complex handle.

Returns 1/0, or -1 on NULL input.

### symbolic_complex_simplifies_to_zero(ptr)

Returns whether the value simplifies to exactly zero.

- `ptr` (`const CSymbolicComplex*`): symbolic complex handle.

Returns 1 when it simplifies to zero, 0 when not, -1 on NULL input, -3 on simplification failure.

### symbolic_complex_free(ptr)

Frees a symbolic complex handle.

- `ptr` (`CSymbolicComplex*`): handle to free; NULL is allowed.

No return value.

---

## Matrix construction and release

### symbolic_eye(dim)

Creates a `dim x dim` symbolic identity matrix.

- `dim` (`uintptr_t`): matrix dimension.

Returns a newly allocated `CSymbolicMatrix*`; NULL only on allocation failure.

### symbolic_matrix_new(rows, cols, elements, len)

Builds a `rows x cols` symbolic matrix from an array of element handles in row-major order.

- `rows` (`uintptr_t`): row count, must be greater than 0.
- `cols` (`uintptr_t`): column count, must be greater than 0.
- `elements` (`const CSymbolicComplex* const*`): `rows * cols` symbolic complex handles.
- `len` (`uintptr_t`): array length, must equal `rows * cols`.

Returns a newly allocated `CSymbolicMatrix*`; NULL when `elements` is NULL, the length does not match, a dimension is zero, or any element handle is NULL.

### standard_gate_symbolic_matrix(gate_name, params, len)

Builds the symbolic unitary matrix of a standard gate selected by name (for example `"RX"`, `"RZZ"`; same name set as `circuit_multi_control`).

- `gate_name` (`const char*`): standard gate name.
- `params` (`const CParameter* const*`): array of gate parameter handles.
- `len` (`uintptr_t`): number of parameters.

Returns a newly allocated `CSymbolicMatrix*`; NULL on NULL input, invalid UTF-8, an unknown gate name, or a parameter-count mismatch.

### circuit_to_symbolic_matrix(ptr, order, order_len)

Computes the symbolic unitary matrix of a circuit.

- `ptr` (`const CCircuit*`): target circuit handle.
- `order` (`const uint32_t*`): array of qubit ids ordered from least-significant to most-significant bit; NULL sorts by qubit index.
- `order_len` (`uintptr_t`): array length; 0 selects the default ordering.

Returns a newly allocated `CSymbolicMatrix*`; NULL on NULL input, an order/qubit-set mismatch, or non-unitary operations.

### symbolic_matrix_controlled(base, num_ctrls)

Embeds `base` into the bottom-right block of a larger identity matrix, adding `num_ctrls` control qubits.

- `base` (`const CSymbolicMatrix*`): base matrix.
- `num_ctrls` (`uintptr_t`): number of control qubits.

Returns a newly allocated `CSymbolicMatrix*`; NULL on NULL input.

### symbolic_matrix_free(ptr)

Frees a symbolic matrix handle.

- `ptr` (`CSymbolicMatrix*`): handle to free; NULL is allowed.

No return value.

---

## Shape and element queries

### symbolic_matrix_rows(ptr)

Returns the row count.

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.

Returns the row count, or 0 for NULL.

### symbolic_matrix_cols(ptr)

Returns the column count.

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.

Returns the column count, or 0 for NULL.

### symbolic_matrix_element_str(ptr, row, col)

Formats one matrix element as a human-readable parameter expression (for example `"cos(theta*0.5)"`).

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.
- `row` (`uintptr_t`): row index.
- `col` (`uintptr_t`): column index.

Returns a newly allocated C string (free with `cqlib_string_free`); NULL on NULL input or out-of-bounds indices.

### symbolic_matrix_element(ptr, row, col)

Returns a clone of the `[row, col]` element (free with `symbolic_complex_free`) for element-wise access.

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.
- `row` (`uintptr_t`): row index.
- `col` (`uintptr_t`): column index.

Returns a newly allocated `CSymbolicComplex*`; NULL on NULL input or out-of-bounds indices.

### symbolic_matrix_evaluate_len(ptr)

Returns the flattened element count (`rows * cols`), i.e. the buffer length required for numeric evaluation.

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.

Returns the element count, or 0 for NULL.

---

## Simplification and substitution

### symbolic_matrix_simplify(ptr)

Simplifies every element of the matrix and returns the simplified result as a new matrix.

- `ptr` (`const CSymbolicMatrix*`): source matrix handle.

Returns a newly allocated `CSymbolicMatrix*`; NULL on NULL input or simplification failure.

### symbolic_matrix_substitute(ptr, names, params, len)

Simultaneously substitutes symbols in the matrix: `names[i]` is a symbol name and `params[i]` the replacement expression.

- `ptr` (`const CSymbolicMatrix*`): source matrix handle.
- `names` (`const char* const*`): array of symbol names.
- `params` (`const CParameter* const*`): array of replacement expression handles.
- `len` (`uintptr_t`): number of substitutions.

Returns a newly allocated `CSymbolicMatrix*`; NULL on NULL input, invalid UTF-8, or a substitution error (for example a reserved prefix).

---

## Numeric evaluation

### symbolic_matrix_evaluate(ptr, bindings, buffer, len)

Evaluates every element under parameter bindings into `buffer` in row-major order.

- `ptr` (`const CSymbolicMatrix*`): symbolic matrix handle.
- `bindings` (`const char*`): bindings string formatted as `"name:value,name:value"` (NULL means no bindings).
- `buffer` (`Complex64*`): array receiving the results.
- `len` (`uintptr_t`): array length, must be at least `symbolic_matrix_evaluate_len`.

Returns 0 on success, -1 on NULL input, -3 on unevaluable symbols, -4 on a malformed bindings string, -8 when `len` is too small.

---

## In-place gate application (apply_* family)

The `apply_*` functions apply a quantum gate to a symbolic matrix **in place**: the effect is a left multiplication by the gate's operator restricted to the subspace spanned by the target bits, and the shape is preserved. Target-bit constraints shared by all functions:

- the matrix row count must be a power of two and every bit in `bits` must be smaller than `log2(rows)`;
- the `bits` array must be non-empty and free of duplicates; the gate matrix dimension of a multi-bit gate must equal `1 << bits_len`.

### Target-bit ordering

The `bits` parameter of the `apply_*` functions uses the system's **Little-Endian (least-significant bit first)** convention: `bits[k]` corresponds to bit `k` of the gate's local row index. Contrast this with the circuit API, where controlled gates receive their qubits as `[control, target]` (most-significant first); `circuit_to_symbolic_matrix` reverses that sequence internally before applying. When reproducing a circuit's effect with `apply_standard_gate` you must therefore pass the **reversed order yourself**: `circuit_cx(c, 0, 1)` equals applying `"CX"` with `bits = [1, 0]`.

### symbolic_matrix_apply_standard_gate(ptr, gate_name, bits, bits_len, params, params_len)

Applies a standard gate by name (same name set as `circuit_multi_control`, e.g. `"H"`, `"CX"`, `"RX"`) with symbolic gate parameters.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate_name` (`const char*`): standard gate name.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.
- `params` (`const CParameter* const*`): gate parameter handle array; NULL is allowed for parameterless gates.
- `params_len` (`uintptr_t`): number of gate parameters.

Returns 0 on success, -1 when the matrix, gate name, or a parameter handle is NULL, -3 on a gate-bit/parameter-count mismatch with the matrix, -4 on a non-UTF-8 gate name, -8 on an unknown gate name or invalid bits.

### symbolic_matrix_apply_gate(ptr, gate, bits, bits_len)

Applies a symbolic gate matrix (dimension `1 << bits_len`) to the target bits.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const CSymbolicMatrix*`): gate matrix handle.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 on NULL input, -3 when the gate dimension does not match the target bits, -8 on invalid bits.

### symbolic_matrix_apply_gate_num(ptr, gate, gate_dim, bits, bits_len)

Numeric variant of `symbolic_matrix_apply_gate`: `gate` points at `gate_dim * gate_dim` row-major `Complex64` values and `gate_dim` must equal `1 << bits_len`.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const Complex64*`): gate matrix values.
- `gate_dim` (`uintptr_t`): gate matrix dimension.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix or `gate` is NULL, -3 when the gate does not match the target bits, -8 when `gate_dim` is 0, differs from `1 << bits_len`, or the bits are invalid.

### symbolic_matrix_apply_single_qubit_gate(ptr, gate, bit)

Applies a `2x2` symbolic gate to a single target bit.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const CSymbolicMatrix*`): `2x2` gate matrix handle.
- `bit` (`uintptr_t`): target bit.

Returns 0 on success, -1 on NULL input, -8 when the gate is not `2x2`, the row count is not a power of two, or `bit` is out of range.

### symbolic_matrix_apply_single_qubit_gate_num(ptr, gate, bit)

Numeric variant of `symbolic_matrix_apply_single_qubit_gate`: `gate` points at 4 row-major `Complex64` values.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const Complex64*`): the 4 gate matrix elements.
- `bit` (`uintptr_t`): target bit.

Returns 0 on success, -1 on NULL input, -8 when the row count is not a power of two or `bit` is out of range.

### symbolic_matrix_apply_two_qubit_gate(ptr, gate, b0, b1)

Applies a `4x4` symbolic gate to two target bits.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const CSymbolicMatrix*`): `4x4` gate matrix handle.
- `b0`, `b1` (`uintptr_t`): the two target bits.

Returns 0 on success, -1 on NULL input, -8 when the gate is not `4x4`, the row count is not a power of two, or the bits are duplicated/out of range.

### symbolic_matrix_apply_two_qubit_gate_num(ptr, gate, b0, b1)

Numeric variant of `symbolic_matrix_apply_two_qubit_gate`: `gate` points at 16 row-major `Complex64` values. Same error codes as above.

### symbolic_matrix_apply_general_gate(ptr, gate, bits, bits_len)

Applies a symbolic gate of arbitrary arity (a square matrix of dimension `1 << bits_len`) to the target bits.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const CSymbolicMatrix*`): gate matrix handle (square, power-of-two dimension).
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 on NULL input, -8 when the gate shape does not match `1 << bits_len` or the bits are invalid.

### symbolic_matrix_apply_general_gate_num(ptr, gate, bits, bits_len)

Numeric variant of `symbolic_matrix_apply_general_gate`: `gate` points at `(1 << bits_len)` squared row-major `Complex64` values.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `gate` (`const Complex64*`): gate matrix values.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix or `gate` is NULL, -8 when `bits_len` overflows or the bits are invalid.

### symbolic_matrix_apply_permutation_gate(ptr, indices, values, len, bits, bits_len)

Applies a symbolic permutation gate in place: `permutation[i] = (indices[i], values[i])` means output local row `i` equals `values[i]` times input local row `indices[i]`.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `indices` (`const uintptr_t*`): source local row indices.
- `values` (`const CSymbolicComplex* const*`): symbolic factor handles paired with `indices`.
- `len` (`uintptr_t`): permutation length, must equal `1 << bits_len`.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix is NULL, an `indices[i]` is out of range, or a `values[i]` is NULL, -8 when `len` differs from `1 << bits_len`, `indices`/`values` is NULL, or the bits are invalid.

### symbolic_matrix_apply_permutation_gate_num(ptr, indices, values, len, bits, bits_len)

Numeric variant of `symbolic_matrix_apply_permutation_gate`: `values` points at `len` raw `Complex64` factors.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `indices` (`const uintptr_t*`): source local row indices.
- `values` (`const Complex64*`): numeric factor array.
- `len` (`uintptr_t`): permutation length, must equal `1 << bits_len`.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix or `values` is NULL, -8 when an `indices[i]` is out of range, `len` differs from `1 << bits_len`, `indices` is NULL, or the bits are invalid.

### symbolic_matrix_apply_diagonal_gate(ptr, diagonal, len, bits, bits_len)

Applies a symbolic diagonal gate in place: `diagonal[i]` scales local row `i`.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `diagonal` (`const CSymbolicComplex* const*`): diagonal element handles.
- `len` (`uintptr_t`): diagonal length, must equal `1 << bits_len`.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix is NULL or any diagonal handle is NULL, -8 when `len` differs from `1 << bits_len`, `diagonal` is NULL, or the bits are invalid.

### symbolic_matrix_apply_diagonal_gate_num(ptr, diagonal, len, bits, bits_len)

Numeric variant of `symbolic_matrix_apply_diagonal_gate`: `diagonal` points at `len` raw `Complex64` values.

- `ptr` (`CSymbolicMatrix*`): target matrix (mutated in place).
- `diagonal` (`const Complex64*`): diagonal values.
- `len` (`uintptr_t`): diagonal length, must equal `1 << bits_len`.
- `bits` (`const uintptr_t*`): target-bit array.
- `bits_len` (`uintptr_t`): number of target bits.

Returns 0 on success, -1 when the matrix or `diagonal` is NULL, -8 when `len` differs from `1 << bits_len` or the bits are invalid.

### Example: reproducing a circuit matrix in place

```c
/* Reproduce the symbolic matrix of circuit_h(c, 0) + circuit_cx(c, 0, 1).
 * The CX [control, target] pair must be reversed to [target, control]
 * for the apply_* Little-Endian convention. */
struct CSymbolicMatrix *matrix = symbolic_eye(4);
const uintptr_t h_bits[1] = {0};
const uintptr_t cx_bits[2] = {1, 0};
symbolic_matrix_apply_standard_gate(matrix, "H", h_bits, 1, NULL, 0);
symbolic_matrix_apply_standard_gate(matrix, "CX", cx_bits, 2, NULL, 0);
/* matrix is now equivalent to circuit_to_symbolic_matrix(c, NULL, 0) */
```

---

## Equivalence checks

### symbolic_matrices_equivalent(lhs, rhs)

Returns whether two symbolic matrices can be proven equivalent up to a global phase.

- `lhs` (`const CSymbolicMatrix*`): left-hand matrix.
- `rhs` (`const CSymbolicMatrix*`): right-hand matrix.

Returns 1 when equivalent, 0 when not, -1 on NULL input, -3 on simplification failure.

### circuits_equivalent(lhs, rhs, order, order_len)

Compares two circuits up to a global phase in the requested qubit order.

- `lhs` (`const CCircuit*`): left-hand circuit handle.
- `rhs` (`const CCircuit*`): right-hand circuit handle.
- `order` (`const uint32_t*`): array of qubit ids ordered from least-significant to most-significant bit; NULL (with `order_len` 0) sorts by qubit index.
- `order_len` (`uintptr_t`): array length.

Returns 1 when equivalent, 0 when not, -1 on NULL input, -3 on matrix construction failure (including an order/qubit-set mismatch), -8 when `order` is NULL while `order_len` is greater than 0.

---

## Qubit order convention

The `order` parameter of `circuit_to_symbolic_matrix` and `circuits_equivalent` fixes the correspondence between matrix basis indices and circuit qubits:

- `order` NULL (`order_len` 0): default ordering by qubit index;
- explicit `order`: the qubit at array position `i` corresponds to bit `i` of the basis index (least-significant first);
- an explicit order must match the circuit's qubit set exactly, with no missing, duplicated, or unknown qubits.

For example, when the circuit holds qubits 0 and 2, both `[0, 2]` and `[2, 0]` are valid orders, but they yield matrix representations under different axis orders.

---

## Example: parameterized circuit → symbolic matrix → bound evaluation

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a parameterized circuit: RX(theta) on qubit 0 */
    struct CCircuit *circuit = circuit_new(1);
    struct CParameter *theta = param_parse("theta");
    if (circuit_rx_param(circuit, 0, theta) != 0) {
        return 1;
    }

    /* 2. Convert to a symbolic matrix (default order by qubit index) */
    struct CSymbolicMatrix *matrix = circuit_to_symbolic_matrix(circuit, NULL, 0);
    if (matrix == NULL) {
        return 1;
    }

    /* Inspect an element's symbolic form, e.g. "cos(theta*0.5)" */
    char *element = symbolic_matrix_element_str(matrix, 0, 0);
    if (element != NULL) {
        printf("matrix[0][0] = %s\n", element);
        cqlib_string_free(element);
    }

    /* 3. Bind theta = pi/2 and evaluate into a numeric matrix */
    uintptr_t len = symbolic_matrix_evaluate_len(matrix);
    Complex64 *values = malloc(len * sizeof(Complex64));
    int32_t status = symbolic_matrix_evaluate(matrix, "theta:1.5707963267948966",
                                              values, len);
    if (status == 0) {
        /* values[0] is cos(pi/4); values is row-major */
        printf("matrix[0][0] = %f + %fi\n", values[0].re, values[0].im);
    }

    free(values);
    symbolic_matrix_free(matrix);
    param_free(theta);
    circuit_free(circuit);
    return status;
}
```

See [Parameter](3_parameter.md) for parameter expression parsing (`param_parse` / `param_free`), [Standard Gates](5_gate_standard.md) for parameterized rotation gates (`circuit_rx_param`), and [Circuit](1_circuit.md) for circuit construction.
