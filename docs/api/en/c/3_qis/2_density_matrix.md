# DensityMatrix

The `CDensityMatrix*` handle is the mixed-state simulator: the quantum state is represented as a `2^N × 2^N` density matrix (`4^N` row-major matrix elements) and can describe pure states, mixed states, and the results of quantum channels. This page covers construction and circuit ingestion, matrix data and physicality checks, gate operations, quantum channels and the partial trace, probabilities and measurement, and expectation values. Error codes and memory conventions are described in [Overview](../0_overview.md); for the module summary see [QIS Overview](0_overview.md).

---

## Construction and Release

### density_matrix_new(num_qubits)

```c
struct CDensityMatrix *density_matrix_new(uintptr_t num_qubits);
```

Creates a mixed-state simulator initialized to the pure state `|0...0><0...0|`.

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns: a newly allocated `CDensityMatrix*`; free it with `density_matrix_free`.

### density_matrix_maximally_mixed(num_qubits)

```c
struct CDensityMatrix *density_matrix_maximally_mixed(uintptr_t num_qubits);
```

Creates the maximally mixed state `I / 2^N`: every computational basis state is equally probable with no coherences.

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns: a newly allocated `CDensityMatrix*`; free it with `density_matrix_free`.

### density_matrix_zeros(num_qubits)

```c
struct CDensityMatrix *density_matrix_zeros(uintptr_t num_qubits);
```

Creates a density matrix filled entirely with zeros. This is not a valid physical state (trace = 0); it is useful as an accumulator during operations like Kraus channel application.

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns: a newly allocated `CDensityMatrix*`; free it with `density_matrix_free`; `NULL` when the matrix size overflows the addressable range.

### density_matrix_from_circuit(circuit)

```c
struct CDensityMatrix *density_matrix_from_circuit(const struct CCircuit *circuit);
```

Executes the circuit and returns the evolved density matrix; the input circuit is not modified. The circuit is decomposed into basic gates and executed instruction by instruction; measurement declarations do not take part in the state evolution.

- `circuit` (`const CCircuit*`): circuit handle, see [Circuit](../0_circuit/1_circuit.md).

Returns: a newly allocated `CDensityMatrix*`; `NULL` when `circuit` is NULL or execution fails.

### density_matrix_from_state(num_qubits, initial_state, len)

```c
struct CDensityMatrix *density_matrix_from_state(uintptr_t num_qubits,
                                                 const Complex64 *initial_state,
                                                 uintptr_t len);
```

Constructs the density matrix as the outer product `ρ = |ψ><ψ|` of a normalized pure-state amplitude array; `len` must equal `2^num_qubits`.

- `num_qubits` (`uintptr_t`): the number of qubits.
- `initial_state` (`const Complex64*`): normalized amplitude array of length `2^num_qubits`.
- `len` (`uintptr_t`): array length.

Returns: a newly allocated `CDensityMatrix*`; `NULL` on NULL input, wrong dimension, or a non-normalized state.

### density_matrix_free(ptr)

```c
void density_matrix_free(struct CDensityMatrix *ptr);
```

Frees the handle. Passing NULL is allowed.

- `ptr` (`CDensityMatrix*`): density-matrix handle.

### density_matrix_num_qubits(ptr)

```c
uintptr_t density_matrix_num_qubits(const struct CDensityMatrix *ptr);
```

Returns the number of qubits.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.

Returns: the qubit count; 0 for NULL.

### density_matrix_apply_circuit(ptr, circuit)

```c
int32_t density_matrix_apply_circuit(struct CDensityMatrix *ptr, const struct CCircuit *circuit);
```

Applies the circuit in place to the current density matrix; the circuit's qubit count must match the state.

- `ptr` (`CDensityMatrix*`): density-matrix handle.
- `circuit` (`const CCircuit*`): circuit handle.

Returns: `0` on success; `-1` NULL pointer; `-3` circuit error; `-7` the circuit contains an unsupported operation; `-8` the circuit's qubit count does not match the state.

---

## Matrix Data and Physicality

### density_matrix_data_len(ptr)

```c
uintptr_t density_matrix_data_len(const struct CDensityMatrix *ptr);
```

Returns the number of matrix elements (`4^N`), for sizing the buffer passed to `density_matrix_data`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.

Returns: the element count; 0 for NULL.

### density_matrix_data(ptr, buffer, len)

```c
int32_t density_matrix_data(const struct CDensityMatrix *ptr, Complex64 *buffer, uintptr_t len);
```

Copies the flattened row-major matrix elements into `buffer` as `4^N` `Complex64` values; element `r·2^N + c` is row `r`, column `c` of the matrix.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `buffer` (`Complex64*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `density_matrix_data_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

### density_matrix_trace(ptr, out_re, out_im)

```c
int32_t density_matrix_trace(const struct CDensityMatrix *ptr, double *out_re, double *out_im);
```

Computes the trace and writes its real and imaginary parts to `out_re` / `out_im`; the trace of a valid physical state equals `1.0`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `out_re` (`double*`): output for the real part.
- `out_im` (`double*`): output for the imaginary part.

Returns: `0` on success; `-1` when any pointer is NULL.

### density_matrix_is_hermitian(ptr, tol, out)

```c
int32_t density_matrix_is_hermitian(const struct CDensityMatrix *ptr, double tol, bool *out);
```

Checks whether the matrix satisfies `ρ = ρ†` within tolerance `tol` and writes the result to `out`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `tol` (`double`): tolerance; must be finite.
- `out` (`bool*`): result output.

Returns: `0` on success; `-1` NULL pointers; `-8` when `tol` is not finite.

### density_matrix_is_positive_semidefinite(ptr, tol, out)

```c
int32_t density_matrix_is_positive_semidefinite(const struct CDensityMatrix *ptr, double tol, bool *out);
```

Checks whether the matrix is positive semidefinite within tolerance `tol` (all eigenvalues satisfy `λ >= -tol`) and writes the result to `out`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `tol` (`double`): tolerance; must be finite.
- `out` (`bool*`): result output.

Returns: `0` on success; `-1` NULL pointers; `-8` when `tol` is not finite.

### density_matrix_validate_physical(ptr, tol)

```c
int32_t density_matrix_validate_physical(const struct CDensityMatrix *ptr, double tol);
```

Validates the physicality constraints of the density matrix within tolerance `tol`, in order: Hermiticity (`ρ = ρ†`), positive semidefiniteness, and unit trace. The matrix is only read.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `tol` (`double`): tolerance; must be finite.

Returns: `0` when the state is physical; `-1` NULL handle; `-7` not Hermitian or not positive semidefinite; `-8` non-finite `tol` or trace not equal to `1`.

---

## Gate Operations

All gate functions return `int32_t`: `0` on success; `-1` when the handle is NULL; `-2` when a qubit index is out of bounds; parameterized gates return `-8` when an angle is NaN or Inf. Single-qubit gates share the signature:

```c
int32_t density_matrix_apply_h(struct CDensityMatrix *ptr, uint32_t qubit);
```

### Non-parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_apply_h(ptr, qubit)` | H | Hadamard gate. |
| `density_matrix_apply_x(ptr, qubit)` | X | Pauli-X, bit flip. |
| `density_matrix_apply_y(ptr, qubit)` | Y | Pauli-Y. |
| `density_matrix_apply_z(ptr, qubit)` | Z | Pauli-Z, phase flip. |
| `density_matrix_apply_s(ptr, qubit)` | S | S gate (√Z). |
| `density_matrix_apply_sdg(ptr, qubit)` | S† | Inverse of S. |
| `density_matrix_apply_t(ptr, qubit)` | T | T gate (√S). |
| `density_matrix_apply_tdg(ptr, qubit)` | T† | Inverse of T. |
| `density_matrix_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`. |
| `density_matrix_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`. |
| `density_matrix_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`. |
| `density_matrix_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`. |

### Parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_apply_rx(ptr, qubit, theta)` | Rx(θ) | Rotation by `θ` about the X axis. |
| `density_matrix_apply_ry(ptr, qubit, theta)` | Ry(θ) | Rotation by `θ` about the Y axis. |
| `density_matrix_apply_rz(ptr, qubit, theta)` | Rz(θ) | Rotation by `θ` about the Z axis. |
| `density_matrix_apply_phase(ptr, qubit, theta)` | P(θ) | Phase gate, multiplies the `\|1>` component by `e^(iθ)`. |
| `density_matrix_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | Generic single-qubit gate. |
| `density_matrix_apply_xy(ptr, qubit, theta)` | XY(θ) | XY gate. |
| `density_matrix_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P gate. |
| `density_matrix_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M gate. |
| `density_matrix_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | Rotation by `θ` about the axis at azimuth `φ` in the XY plane. |
| `density_matrix_apply_gphase(ptr, phi)` | GPhase(φ) | Global phase. A density matrix is invariant under a global phase, so the call always returns `0` without changing the state; the signature has no qubit parameter. |

GPhase signature:

```c
int32_t density_matrix_apply_gphase(struct CDensityMatrix *ptr, double phi);
```

### Two-qubit gates

Shared signature `int32_t density_matrix_apply_<gate>(struct CDensityMatrix *ptr, uint32_t q0, uint32_t q1, ...)`; the two qubit parameters of controlled gates are named `control` / `target`.

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_apply_cx(ptr, control, target)` | CX | Controlled X. |
| `density_matrix_apply_cy(ptr, control, target)` | CY | Controlled Y. |
| `density_matrix_apply_cz(ptr, q0, q1)` | CZ | Controlled Z; the two parameters are symmetric. |
| `density_matrix_apply_swap(ptr, q0, q1)` | SWAP | Swaps two qubits. |
| `density_matrix_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`. |
| `density_matrix_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`. |
| `density_matrix_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`. |
| `density_matrix_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`. |
| `density_matrix_apply_crx(ptr, control, target, theta)` | CrX(θ) | Controlled `Rx(θ)`. |
| `density_matrix_apply_cry(ptr, control, target, theta)` | CrY(θ) | Controlled `Ry(θ)`. |
| `density_matrix_apply_crz(ptr, control, target, theta)` | CrZ(θ) | Controlled `Rz(θ)`. |
| `density_matrix_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | Fermionic simulation gate; `theta` is the iSWAP angle and `phi` the controlled-phase angle. |

### Three-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli gate; flips the target when both controls are `\|1>`. |

### Generic gate entry points

#### density_matrix_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

```c
int32_t density_matrix_apply_standard_gate(struct CDensityMatrix *ptr,
                                           const char *gate_name,
                                           const uint32_t *qubits,
                                           uintptr_t num_target_qubits,
                                           const double *params,
                                           uintptr_t num_params);
```

Applies a standard gate by name (e.g. `"H"`, `"CX"`, `"RZZ"`, `"FSIM"`), dispatching to the dedicated implementation.

- `ptr` (`CDensityMatrix*`): density-matrix handle.
- `gate_name` (`const char*`): NUL-terminated standard-gate name.
- `qubits` (`const uint32_t*`): array of target qubits with length `num_target_qubits`.
- `num_target_qubits` (`uintptr_t`): number of target qubits; must match the gate's arity.
- `params` (`const double*`): gate-parameter array; may be NULL when `num_params` is 0.
- `num_params` (`uintptr_t`): number of parameters; must match the gate's arity.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` unknown gate name, gate name that is not valid UTF-8, qubit or parameter count that does not match the gate, or a non-finite parameter value.

---

## Quantum Channels and Partial Trace

### density_matrix_apply_kraus(ptr, ops, num_ops, op_dim, qubits, num_target_qubits)

```c
int32_t density_matrix_apply_kraus(struct CDensityMatrix *ptr,
                                   const Complex64 *ops,
                                   uintptr_t num_ops,
                                   uintptr_t op_dim,
                                   const uint32_t *qubits,
                                   uintptr_t num_target_qubits);
```

Applies the quantum channel given by Kraus operators, evolving `ρ → Σ_k K_k ρ K_k†`.

- `ptr` (`CDensityMatrix*`): density-matrix handle.
- `ops` (`const Complex64*`): flattened operator array holding `num_ops × op_dim × op_dim` row-major `Complex64` values, one operator after another.
- `num_ops` (`uintptr_t`): number of operators.
- `op_dim` (`uintptr_t`): dimension of a single operator, equal to `2^num_target_qubits`.
- `qubits` (`const uint32_t*`): array of target qubits.
- `num_target_qubits` (`uintptr_t`): number of target qubits.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` when `num_ops`, `op_dim`, or `num_target_qubits` is 0, or an operator's size does not match the qubit count.

```c
/* Bit-flip channel with p = 0.3 on qubit 0 */
double p = 0.3, s0 = sqrt(1.0 - p), s1 = sqrt(p);
Complex64 ops[8] = {
    {s0, 0}, {0, 0}, {0, 0}, {s0, 0},   /* K0 = sqrt(1-p) * I */
    {0, 0}, {s1, 0}, {s1, 0}, {0, 0},   /* K1 = sqrt(p) * X   */
};
uint32_t q = 0;
density_matrix_apply_kraus(dm, ops, 2, 2, &q, 1);
```

### density_matrix_partial_trace(ptr, keep_qubits, num_keep)

```c
struct CDensityMatrix *density_matrix_partial_trace(const struct CDensityMatrix *ptr,
                                                    const uint32_t *keep_qubits,
                                                    uintptr_t num_keep);
```

Traces out every qubit not listed in `keep_qubits` and returns a new density matrix with `num_keep` qubits.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `keep_qubits` (`const uint32_t*`): array of qubits to keep; may be NULL when `num_keep` is 0.
- `num_keep` (`uintptr_t`): number of qubits to keep.

Returns: a newly allocated `CDensityMatrix*`, freed with `density_matrix_free`; `NULL` on error.

---

## Probabilities and Measurement

### density_matrix_probabilities_len(ptr)

```c
uintptr_t density_matrix_probabilities_len(const struct CDensityMatrix *ptr);
```

Returns the length of the computational-basis probability distribution (`2^N`).

- `ptr` (`const CDensityMatrix*`): density-matrix handle.

Returns: the probability count; 0 for NULL.

### density_matrix_probabilities(ptr, buffer, len)

```c
int32_t density_matrix_probabilities(const struct CDensityMatrix *ptr,
                                     double *buffer,
                                     uintptr_t len);
```

Copies the diagonal of the density matrix (the measurement probability of each computational basis state) into `buffer`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `buffer` (`double*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `density_matrix_probabilities_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

### density_matrix_measure(ptr, qubit)

```c
int32_t density_matrix_measure(struct CDensityMatrix *ptr, uint32_t qubit);
```

Measures the qubit in the Z basis and collapses the density matrix: `ρ' = Π_b ρ Π_b / Tr(Π_b ρ)`.

- `ptr` (`CDensityMatrix*`): density-matrix handle.
- `qubit` (`uint32_t`): the measured qubit.

Returns: `0` for outcome `|0>`, `1` for outcome `|1>`; `-1` NULL; `-2` out-of-bounds qubit.

### density_matrix_measure_all(ptr)

```c
char *density_matrix_measure_all(struct CDensityMatrix *ptr);
```

Measures qubits in order `0..num_qubits`, collapsing the state, and returns the outcome as a big-endian bitstring (MSB = qubit `N-1`, LSB = qubit `0`).

- `ptr` (`CDensityMatrix*`): density-matrix handle.

Returns: a heap-allocated C string, freed with `cqlib_string_free`; `NULL` on error.

### density_matrix_sample_shots(ptr, shots)

```c
struct COutcomeList *density_matrix_sample_shots(const struct CDensityMatrix *ptr, uintptr_t shots);
```

Samples `shots` independent measurement outcomes in parallel from the current distribution without modifying the state. Use `outcome_list_len` for the count, `outcome_list_get` to fetch each big-endian bitstring (freed with `cqlib_string_free`), and `outcome_list_free` to free the list — see [ClassicalState](5_classical_state.md).

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `shots` (`uintptr_t`): number of shots.

Returns: a newly allocated `COutcomeList*`; `NULL` on error.

### density_matrix_reset(ptr, qubit)

```c
int32_t density_matrix_reset(struct CDensityMatrix *ptr, uint32_t qubit);
```

Resets the qubit to `|0>`.

- `ptr` (`CDensityMatrix*`): density-matrix handle.
- `qubit` (`uint32_t`): target qubit.

Returns: `0` on success; `-1` NULL; `-2` out-of-bounds qubit.

---

## Expectation Values

### density_matrix_expectation(ptr, observable, out)

```c
int32_t density_matrix_expectation(const struct CDensityMatrix *ptr,
                                   const struct CHamiltonian *observable,
                                   double *out);
```

Computes `Tr(ρ·H)` and writes it to `out`.

- `ptr` (`const CDensityMatrix*`): density-matrix handle.
- `observable` (`const CHamiltonian*`): observable handle, see [Hamiltonian](7_hamiltonian.md).
- `out` (`double*`): output pointer.

Returns: `0` on success; `-1` NULL pointers; `-8` when the observable's qubit count does not match the state.

---

## Examples

### 1. Bell state, matrix data, and probabilities

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    struct CDensityMatrix *dm = density_matrix_new(2);
    density_matrix_apply_h(dm, 0);
    density_matrix_apply_cx(dm, 0, 1);

    /* Probabilities: P(00) = P(11) = 0.5 */
    double probs[4];
    density_matrix_probabilities(dm, probs, 4);
    printf("P(00)=%.3f P(11)=%.3f\n", probs[0], probs[3]);

    /* Row-major matrix data: 16 Complex64 elements */
    uintptr_t n = density_matrix_data_len(dm);   /* 16 */
    Complex64 *data = malloc(n * sizeof(Complex64));
    density_matrix_data(dm, data, n);
    /* data[0] is rho[0][0], data[3] is rho[0][3], data[15] is rho[3][3] */
    printf("rho[3][3].re=%.3f\n", data[15].re);
    free(data);

    density_matrix_free(dm);
    return 0;
}
```

### 2. Maximally mixed state, trace, and Hermiticity check

```c
struct CDensityMatrix *dm = density_matrix_maximally_mixed(2);
/* 4^2 = 16 matrix elements, trace = 1, Hermitian */
double tr_re, tr_im;
density_matrix_trace(dm, &tr_re, &tr_im);        /* tr_re = 1.0, tr_im = 0.0 */
bool herm = false;
density_matrix_is_hermitian(dm, 1e-10, &herm);   /* herm = true */
density_matrix_free(dm);
```

### 3. Partial trace

```c
struct CDensityMatrix *dm = density_matrix_new(2);
density_matrix_apply_h(dm, 0);
density_matrix_apply_cx(dm, 0, 1);

/* Trace out qubit 1: qubit 0 becomes maximally mixed */
uint32_t keep[1] = {0};
struct CDensityMatrix *reduced = density_matrix_partial_trace(dm, keep, 1);
/* density_matrix_num_qubits(reduced) == 1, P(0) = P(1) = 0.5 */
density_matrix_free(reduced);
density_matrix_free(dm);
```

### 4. Expectation value

```c
struct CDensityMatrix *dm = density_matrix_new(2);
density_matrix_apply_h(dm, 0);
density_matrix_apply_cx(dm, 0, 1);

struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
density_matrix_expectation(dm, h, &exp);   /* exp ≈ 1.0 */
hamiltonian_free(h);
density_matrix_free(dm);
```
