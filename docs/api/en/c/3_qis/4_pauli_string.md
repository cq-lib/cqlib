# PauliString

`CPauliString` represents a Pauli operator string (a sequence of I/X/Y/Z). It can be used as the observable in stabilizer expectation value computation, and can also be combined with weights through [Hamiltonian](5_hamiltonian.md) into a general observable.

---

## Functions

### pauli_string_new(num_qubits)

Creates an all-`I` string on n qubits.

Parameters:

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns:

- `CPauliString *`: a heap-allocated object, released with `pauli_string_free`; returns NULL when `num_qubits = 0` or on failure.

### pauli_string_parse(source)

Parses a Pauli string (such as `"XYZ"`, `"ZZI"`).

Parameters:

- `source` (`const char *`): a string containing only `I/X/Y/Z`.

Returns:

- `CPauliString *`; returns NULL when parsing fails.

**Direction convention**: the characters from left to right correspond to the qubits from the **most significant to the least significant** (the first character is the highest qubit), the same direction as the big-endian bit string output by `measure_all`. For example, after `parse("XYZ")`, `get_pauli(0)` is `PAULI_Z`.

### pauli_string_set_pauli(ptr, idx, pauli)

Sets the operator at index `idx` to `pauli`.

Parameters:

- `idx` (`uintptr_t`): the qubit index (`0` is the least significant).
- `pauli` (`uint8_t`): the constant `PAULI_I` / `PAULI_X` / `PAULI_Y` / `PAULI_Z` (0–3).

Returns:

- `int32_t`; `0` on success, a negative status code when the index is out of range or the constant is invalid.

### pauli_string_get_pauli(ptr, idx)

Reads the operator at index `idx`.

Returns:

- `uint8_t`: one of `PAULI_I/X/Y/Z`; returns `0xFF` on error.

### pauli_string_num_qubits(ptr)

Returns the number of qubits; a NULL handle returns `0`.

### pauli_string_to_string(ptr)

Formats the string with a sign bit (such as `"+XYZ"`).

Returns:

- `char *`: a heap-allocated string, released with `cqlib_string_free`; returns NULL on failure.

### pauli_string_matrix_len(ptr) / pauli_string_matrix(ptr, buffer, len)

Exports the `2^N × 2^N` dense matrix in two steps:

- `pauli_string_matrix_len` returns the matrix **side length** `2^N`; the buffer needs `2^N × 2^N` `Complex64` entries;
- `pauli_string_matrix` fills it in row-major order as `Complex64`; `0` on success, a negative status code when the buffer is insufficient.

### pauli_string_free(ptr)

Releases the object. Accepts NULL.

---

## Example

### Construction and read/write

```c
/* parse 与逐位 set 两种构造方式等价 */
CPauliString *p = pauli_string_parse("XYZ");
printf("%u\n", pauli_string_get_pauli(p, 0));   // PAULI_Z（首字符对应最高位）

pauli_string_set_pauli(p, 2, PAULI_I);          // 修改最高位为 I
char *text = pauli_string_to_string(p);         // "+IZ"
printf("%s\n", text);
cqlib_string_free(text);

pauli_string_free(p);
```

### Dense matrix export

```c
CPauliString *x = pauli_string_parse("X");

uintptr_t side = pauli_string_matrix_len(x);        // 2
Complex64 *mat = malloc(sizeof(Complex64) * side * side);
pauli_string_matrix(x, mat, side * side);

/* mat 行主序 = [[0, 1], [1, 0]] */
printf("%f%+fi %f%+fi\n", mat[0].re, mat[0].im, mat[1].re, mat[1].im);

free(mat);
pauli_string_free(x);
```

For use with stabilizer expectation values see [StabilizerState § Stabilizer access](3_stabilizer_state.md#stabilizer-access).
