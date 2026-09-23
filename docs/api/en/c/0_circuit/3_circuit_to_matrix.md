# Circuit To Matrix

Export a small purely quantum gate circuit as a dense unitary matrix. The API uses a uniform two-step output: first call `circuit_to_matrix_len` to obtain the dimension, then call `circuit_to_matrix` to fill the buffer.

Matrix elements are written out as `double` values in **row-major order with the real and imaginary parts interleaved**, and the layout is `[re₀, im₀, re₁, im₁, ...]`; the complex element `(r, c)` is located at `out[2 * (r * d + c)]` (real part) and `out[2 * (r * d + c) + 1]` (imaginary part), where `d = 2^N` is the side length of the matrix.

---

## Functions

### circuit_to_matrix_len(circuit, qubits_order, order_len)

Return the total number of complex elements of the matrix, `d * d` (`d = 2^N`).

Parameters:

- `circuit` (`const CCircuit *`): a purely quantum gate circuit.
- `qubits_order` (`const uintptr_t *`): the qubit order array; NULL uses the default order.
- `order_len` (`uintptr_t`): the length of the order array; pass 0 when `qubits_order` is NULL.

Returns:

- `uintptr_t`: the total number of complex elements of the matrix; returns `0` on error (a NULL handle, a circuit containing non-unitary operations and so on).

### circuit_to_matrix(circuit, qubits_order, order_len, out, buffer_len)

Write the matrix into `out`.

Parameters:

- `circuit`, `qubits_order`, `order_len`: the same as above.
- `out` (`double *`): the output buffer, which must hold at least `2 * circuit_to_matrix_len(...)` `double` values.
- `buffer_len` (`uintptr_t`): the capacity of `out` (counted in `double`).

Returns:

- `uintptr_t`: the number of complex elements actually written; returns `0` on error.

Error scenarios:

- The circuit is NULL or contains non-unitary operations such as `measure`/`reset` (the circuit does not correspond to a unitary matrix);
- The buffer capacity is insufficient;
- `qubits_order` is invalid (its length does not match the number of qubits in the circuit, or it references the same qubit more than once).

---

## qubits_order semantics

`qubits_order` controls the arrangement order of the tensor factors of the matrix:

- Default (NULL): qubit `0` is the least significant tensor factor, and the matrix is expanded as `q_{N-1} ⊗ ... ⊗ q_1 ⊗ q_0`;
- Passing a permutation (such as `[1, 0]`): swap the order of the high and low bits, giving the matrix after qubit reordering.

The effect of the diagonal order is shown only in the arrangement of matrix rows and columns; unitarity and the spectrum are unchanged.

---

## Example

### Unitary matrix of a Bell circuit

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

uintptr_t d = circuit_to_matrix_len(qc, NULL, 0);        // d * d = 16，d = 4
double *mat = malloc(sizeof(double) * 2 * d);
if (circuit_to_matrix(qc, NULL, 0, mat, 2 * d) == 0) {
    /* (0,0) 元素 = 1/√2 + 0i */
    printf("m[0][0] = %f%+fi\n", mat[0], mat[1]);
}

free(mat);
circuit_free(qc);
```

### Qubit reordering

```c
uintptr_t order[2] = {1, 0};
uintptr_t d = circuit_to_matrix_len(qc, order, 2);
double *mat = malloc(sizeof(double) * 2 * d);
circuit_to_matrix(qc, order, 2, mat, 2 * d);
/* mat 现在是比特交换顺序下的酉矩阵 */

free(mat);
```

The matrix size of a larger circuit grows exponentially with the number of qubits (N qubits need `2 * 4^N` `double` values), so this interface suits exact verification scenarios with a small N.
