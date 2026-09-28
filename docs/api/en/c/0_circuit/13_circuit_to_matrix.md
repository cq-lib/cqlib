# Circuit to Matrix

`circuit_to_matrix_len` and `circuit_to_matrix` convert a numeric circuit into its unitary matrix: `n` qubits yield a `2^n × 2^n` matrix, written row-major as interleaved `(re, im)` doubles and including the circuit's global phase. The matrix size grows as `4^n` with the qubit count, so these entry points suit small circuits. For conversions that keep symbolic expressions see [Symbolic Matrix](10_symbolic_matrix.md); for the common conventions (error codes, handle release, two-step array output) see [Overview](../0_overview.md).

---

## Two-step output

### circuit_to_matrix_len(ptr, qubits_order, order_len)

Returns the number of complex elements in the unitary matrix (rows × cols), used to allocate the output buffer. Pass `qubits_order` = NULL to use the default ordering.

- `ptr` (`const struct CCircuit*`): circuit handle.
- `qubits_order` (`const uintptr_t*`): qubit order array; may be NULL.
- `order_len` (`uintptr_t`): length of the order array.

Returns: the complex element count, or 0 when `ptr` is NULL or the conversion fails (including unresolved symbolic parameters, non-unitary operations, or an invalid qubit order).

### circuit_to_matrix(ptr, qubits_order, order_len, out, buffer_len)

Copies the unitary matrix into `out` as interleaved `(real, imag)` doubles and returns the number of complex elements written.

- `ptr` (`const struct CCircuit*`): circuit handle.
- `qubits_order` (`const uintptr_t*`): qubit order array; may be NULL.
- `order_len` (`uintptr_t`): length of the order array.
- `out` (`double*`): output buffer; it must hold at least `2 × circuit_to_matrix_len(...)` doubles.
- `buffer_len` (`uintptr_t`): buffer capacity (in doubles).

Returns: the number of complex elements written, or 0 when `ptr` or `out` is NULL, `buffer_len` is too small (rejected before writing), or the conversion fails.

Complex element `i` is `{ out[2*i], out[2*i+1] }`, located at matrix row `i / 2^n`, column `i % 2^n` (row-major).

---

## Qubit order

`qubits_order` selects the qubit arrangement used for the basis-state indices of the matrix. It must be an exact permutation of the circuit's qubit set — no missing, duplicated, or unknown qubits — otherwise the conversion fails (both entry points return 0).

- `NULL` / `order_len == 0`: qubits are sorted by numeric id;
- the first qubit in the order corresponds to the least-significant basis-state bit (little-endian); under the default ordering the lowest-numbered qubit (for example qubit 0) is the least-significant bit.

```c
/* Explicit order: qubit 1 is the least-significant bit */
uintptr_t order[2] = {1, 0};
uintptr_t len = circuit_to_matrix_len(circuit, order, 2);
```

---

## Parameterized circuits

The numeric matrix requires every symbolic parameter in the circuit to resolve to a concrete value. While unresolved symbolic parameters remain, both entry points return 0; bind the parameters first with `circuit_assign_params` (see [Circuit](1_circuit.md):

```c
struct CParameter* theta = param_parse("theta");

struct CCircuit* circuit = circuit_new(1);
circuit_rx_param(circuit, 0, theta);

/* theta is unbound: returns 0 */
uintptr_t unbound = circuit_to_matrix_len(circuit, NULL, 0);

/* Bind theta = 0.5, then convert: 1 qubit → 2 × 2 = 4 */
struct CCircuit* bound = circuit_assign_params(circuit, "theta:0.5");
uintptr_t len = circuit_to_matrix_len(bound, NULL, 0);
```

---

## Global phase

The matrix includes the circuit's global phase `theta`: if `U` is the circuit matrix without the global phase, the conversion returns `exp(i * theta) * U`.

---

## Example

Two-step retrieval of the 4 × 4 unitary of a 2-qubit circuit (H + CX):

```c
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* Step 1: query the complex element count (2^2 × 2^2 = 16) */
    uintptr_t len = circuit_to_matrix_len(circuit, NULL, 0);
    if (len == 0) {
        circuit_free(circuit);
        return 1;
    }

    /* Step 2: allocate at least 2 * len doubles and copy */
    double* buffer = (double*)malloc(len * 2 * sizeof(double));
    uintptr_t written = circuit_to_matrix(circuit, NULL, 0, buffer, len * 2);
    if (written != len) {
        free(buffer);
        circuit_free(circuit);
        return 1;
    }

    /* For example, element [0][3] lives at buffer[6] (real) and buffer[7] (imag) */
    printf("U[0][3] = %g + %gi\n", buffer[6], buffer[7]);

    free(buffer);
    circuit_free(circuit);
    return 0;
}
```

For the circuit-building and parameter-binding entry points (`circuit_new`, `circuit_assign_params`, and friends) see [Circuit](1_circuit.md).
