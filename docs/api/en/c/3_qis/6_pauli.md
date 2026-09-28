# Pauli (C)

`CPauliString` is an opaque handle to a Pauli string: each of N qubits carries an `I`/`X`/`Y`/`Z` operator plus an overall phase. Single-qubit operators use the `PAULI_*` constants, and the text format of a string is `[+|-][i|j]<operators>`. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants and Format Conventions

Single-qubit Pauli operator tags:

| Constant | Value | Operator |
| --- | --- | --- |
| `PAULI_I` | 0 | Identity I. |
| `PAULI_X` | 1 | Pauli X. |
| `PAULI_Y` | 2 | Pauli Y. |
| `PAULI_Z` | 3 | Pauli Z. |

Text format and bit order:

- The format is `[+|-][i|j]<operators>` with operators `I`, `X`, `Y`, `Z`; the phase is one of `+`, `-`, `+i` (also written `+j`), `-i` (also written `-j`), defaulting to `+` when omitted.
- The bit order is reversed with respect to reading order: **the first character of the string corresponds to the highest qubit index**. For example, after parsing `"XZI"`, qubit 2 carries `X`, qubit 1 carries `Z`, and qubit 0 carries `I`; `pauli_string_to_string` outputs with the same convention (e.g. `"+XZI"`).

---

## Single-Qubit Operator Multiplication

### pauli_mul_with_phase(left, right, out_pauli, out_phase)

```c
int32_t pauli_mul_with_phase(uint8_t left, uint8_t right, uint8_t *out_pauli, Complex64 *out_phase);
```

Multiplies two single-qubit Pauli operators as `left * right` and writes the resulting operator tag (one of `PAULI_I/X/Y/Z`) to `*out_pauli` and the Pauli-group phase factor (one of `1`, `i`, `-1`, `-i`) to `*out_phase`; for example `X * Z = -iY` and `Z * X = iY`.

Returns: `0` on success; `-1` when an output pointer is NULL; `-8` when an input tag is not one of `PAULI_I/X/Y/Z`.

---

## Construction and Release

### pauli_string_new(num_qubits)

Creates an all-identity Pauli string.

Parameters:

- `num_qubits` (`uintptr_t`): number of qubits.

Returns: a new `CPauliString*` on success; NULL when `num_qubits` is 0.

### pauli_string_free(ptr)

Frees a Pauli string handle; NULL is allowed.

### pauli_string_parse(source)

Parses a C string in the `[+|-][i|j]<operators>` format (e.g. `"XYZ"`, `"+ZZI"`, `"-iZII"`).

Returns: a new `CPauliString*` on success; NULL on NULL input, invalid UTF-8, or parse failure.

---

## Operator Access

### pauli_string_set_pauli(ptr, idx, pauli)

Sets the operator at qubit `idx` to `pauli` (one of `PAULI_*`).

Returns: 0 on success; -1 for NULL pointers; -2 for an out-of-bounds `idx`; -8 for an invalid operator tag.

### pauli_string_get_pauli(ptr, idx)

Reads the operator at qubit `idx`.

Returns: one of `PAULI_I`/`PAULI_X`/`PAULI_Y`/`PAULI_Z`; 255 (`0xFF`) for NULL input or out-of-bounds.

### pauli_string_try_get_pauli(ptr, idx, out)

Non-panicking operator read: writes the operator tag to `*out`.

Returns: 0 on success; -1 for NULL pointers; -2 for out-of-bounds.

### pauli_string_try_set_pauli(ptr, idx, pauli)

Non-panicking operator write: `pauli` must be one of `PAULI_*`.

Returns: 0 on success; -1 for NULL pointers; -2 for out-of-bounds; -8 for an invalid operator tag.

---

## Attribute Queries

### pauli_string_num_qubits(ptr)

Returns the number of qubits the string acts on; 0 for NULL.

### pauli_string_to_string(ptr)

Formats the Pauli string as a `"+XYZ"`-style C string (first character is the highest qubit).

Returns: a heap-allocated string; free with `cqlib_string_free`. NULL input returns NULL.

### pauli_string_x_mask(ptr)

Returns the X-component bitmask: bit `i` is set when qubit `i` carries `X` or `Y`; 0 for NULL.

### pauli_string_z_mask(ptr)

Returns the Z-component bitmask: bit `i` is set when qubit `i` carries `Z` or `Y`; 0 for NULL.

### pauli_string_y_phase(ptr, out)

Computes the phase factor `i^n` contributed by the n `Y` operators of the string.

Returns: 0 on success (complex value written to `*out`); -1 for NULL pointers.

### pauli_string_support_len(ptr) / pauli_string_support(ptr, buffer, len)

Two-step read of the support: `*_len` returns the number of non-identity qubits (0 for NULL), and the fill function copies the support indices (ascending) into `buffer` as `uint32_t` values.

Returns (fill function): 0 on success; -1 for NULL pointers; -8 when `len` differs from `pauli_string_support_len`.

---

## Commutation and Matrices

### pauli_string_commutes_with(ptr, other)

Tests whether two Pauli strings commute.

Returns: 1 when they commute; 0 otherwise; -1 for NULL handles; -8 when the qubit counts differ.

### pauli_string_matrix_len(ptr)

Returns the side length of the dense matrix (`2^N`); 0 for NULL.

### pauli_string_matrix(ptr, buffer, len)

Copies the `2^N × 2^N` dense matrix into `buffer` in row-major order (`Complex64` values). `len` must equal `pauli_string_matrix_len(ptr) * pauli_string_matrix_len(ptr)`.

Returns: 0 on success; -1 for NULL pointers; -8 when `len` does not match the matrix element count.

---

## Expectation Values

### pauli_string_expectation(ptr, bitstrings, probs, len, out)

Computes the expectation value of the Pauli string over a probability distribution on the computational basis, `⟨P⟩ = Σ_s p(s)·⟨s|P|s⟩`, given as parallel arrays: `bitstrings[i]` pairs with `probs[i]`.

Conventions:

- Each bitstring uses the little-endian convention (the rightmost character is qubit 0) and must be exactly `num_qubits` characters of `'0'`/`'1'`.
- Strings containing `X` or `Y` operators yield `0.0`.
- Strings containing only `I` and `Z` evaluate `phase × Σ_s p(s) × (-1)^(Σ_i z[i]·s[i])`, where `phase` is the string's global phase and `z[i]` is the Z component of qubit `i`.

Parameters:

- `ptr` (`const struct CPauliString*`): Pauli string handle.
- `bitstrings` (`const char* const*`): array of NUL-terminated bitstrings; may be NULL when `len` is 0.
- `probs` (`const double*`): probability array parallel to `bitstrings`; may be NULL when `len` is 0.
- `len` (`uintptr_t`): number of distribution entries.
- `out` (`double*`): output pointer.

Returns: 0 on success; -1 when `ptr` or `out` is NULL, or when either array is NULL while `len > 0`; -4 when a bitstring is not valid UTF-8; -7 when a bitstring has the wrong length or contains characters other than `'0'`/`'1'`; -8 when a bitstring array entry is NULL.

---

## Example

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Parsing: first character is the highest qubit, "XZI" -> qubit2=X, qubit1=Z, qubit0=I */
    struct CPauliString *ps = pauli_string_parse("XZI");
    uintptr_t n = pauli_string_num_qubits(ps);      /* 3 */
    uint8_t p2 = pauli_string_get_pauli(ps, 2);     /* PAULI_X (1) */
    uint8_t p0 = pauli_string_get_pauli(ps, 0);     /* PAULI_I (0) */

    /* 2. Masks and support */
    uintptr_t x_mask = pauli_string_x_mask(ps);     /* bit 2 -> 0b100 */
    uintptr_t z_mask = pauli_string_z_mask(ps);     /* bit 1 -> 0b010 */
    uintptr_t support_len = pauli_string_support_len(ps);  /* 2 */
    uint32_t support[2];
    pauli_string_support(ps, support, support_len); /* {1, 2} */

    /* 3. Text output and commutation */
    char *text = pauli_string_to_string(ps);        /* "+XZI" */
    struct CPauliString *zz = pauli_string_parse("ZZI");
    int32_t commutes = pauli_string_commutes_with(ps, zz);  /* 0 (X and Z anticommute on qubit 2) */

    /* 4. Two-step read of the 8x8 dense matrix */
    uintptr_t dim = pauli_string_matrix_len(ps);    /* 8 */
    Complex64 *matrix = malloc(dim * dim * sizeof(Complex64));
    pauli_string_matrix(ps, matrix, dim * dim);
    free(matrix);

    cqlib_string_free(text);
    pauli_string_free(zz);
    pauli_string_free(ps);
    return 0;
}
```
