# Statevector

The `CStatevector*` handle is the pure-state simulator: the quantum state is represented as `2^N` complex amplitudes, and every operation goes through the `statevector_*` free functions. This page covers construction and circuit ingestion, amplitude access, gate operations, probabilities and measurement, and expectation values. Error codes and memory conventions are described in [Overview](../0_overview.md); for the module summary see [QIS Overview](0_overview.md).

---

## Construction and Release

### statevector_new(num_qubits)

```c
struct CStatevector *statevector_new(uintptr_t num_qubits);
```

Creates a pure-state simulator initialized to `|0...0>`: the first amplitude is 1, all others are 0.

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns: a newly allocated `CStatevector*`; free it with `statevector_free`.

### statevector_from_circuit(circuit)

```c
struct CStatevector *statevector_from_circuit(const struct CCircuit *circuit);
```

Executes the circuit and returns the evolved state; the input circuit is not modified. The circuit is decomposed into basic gates and executed instruction by instruction; measurement declarations do not take part in the state evolution.

- `circuit` (`const CCircuit*`): circuit handle, see [Circuit](../0_circuit/1_circuit.md).

Returns: a newly allocated `CStatevector*`; `NULL` when `circuit` is NULL or execution fails.

### statevector_from_state(num_qubits, initial_state, len)

```c
struct CStatevector *statevector_from_state(uintptr_t num_qubits,
                                            const Complex64 *initial_state,
                                            uintptr_t len);
```

Creates a state from the given amplitude array: `len` must equal `2^num_qubits` and the array must be normalized. Entry `i` is the coefficient of the computational basis state `|i>`, with qubit 0 as the least significant bit.

- `num_qubits` (`uintptr_t`): the number of qubits.
- `initial_state` (`const Complex64*`): amplitude array of length `2^num_qubits`.
- `len` (`uintptr_t`): array length.

Returns: a newly allocated `CStatevector*`; `NULL` on NULL input, wrong dimension, or a non-normalized state.

### statevector_free(ptr)

```c
void statevector_free(struct CStatevector *ptr);
```

Frees the handle. Passing NULL is allowed.

- `ptr` (`CStatevector*`): statevector handle.

### statevector_num_qubits(ptr)

```c
uintptr_t statevector_num_qubits(const struct CStatevector *ptr);
```

Returns the number of qubits.

- `ptr` (`const CStatevector*`): statevector handle.

Returns: the qubit count; 0 for NULL.

### statevector_apply_circuit(ptr, circuit)

```c
int32_t statevector_apply_circuit(struct CStatevector *ptr, const struct CCircuit *circuit);
```

Applies the circuit in place to the current state; the circuit's qubit count must match the state.

- `ptr` (`CStatevector*`): statevector handle.
- `circuit` (`const CCircuit*`): circuit handle.

Returns: `0` on success; `-1` NULL pointer; `-3` circuit error (a gate without a matrix representation, unresolved symbolic parameters, etc.); `-7` the circuit contains an unsupported operation; `-8` the circuit's qubit count does not match the state.

---

## Amplitude Access

### statevector_data_len(ptr)

```c
uintptr_t statevector_data_len(const struct CStatevector *ptr);
```

Returns the number of complex amplitudes (`2^N`), for sizing the buffer passed to `statevector_data`.

- `ptr` (`const CStatevector*`): statevector handle.

Returns: the amplitude count; 0 for NULL.

### statevector_data(ptr, buffer, len)

```c
int32_t statevector_data(const struct CStatevector *ptr, Complex64 *buffer, uintptr_t len);
```

Copies all amplitudes row-major into `buffer` as `2^N` `Complex64` values.

- `ptr` (`const CStatevector*`): statevector handle.
- `buffer` (`Complex64*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `statevector_data_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

```c
uintptr_t n = statevector_data_len(sv);
Complex64 *amps = malloc(n * sizeof(Complex64));
if (statevector_data(sv, amps, n) == 0) {
    /* amps[i] is the amplitude of |i> */
}
free(amps);
```

### statevector_data_mut(ptr, buffer, len)

```c
int32_t statevector_data_mut(struct CStatevector *ptr, const Complex64 *buffer, uintptr_t len);
```

Overwrites the amplitudes by copying `len` `Complex64` values from `buffer` into the statevector; the writable counterpart of `statevector_data`.

- `ptr` (`CStatevector*`): statevector handle.
- `buffer` (`const Complex64*`): input array of `2^N` amplitudes.
- `len` (`uintptr_t`): array length; must equal `statevector_data_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

Overwriting the amplitudes can break normalization; the caller is responsible for providing a valid statevector — the simulator keeps operating on the written data as-is.

---

## Gate Operations

All gate functions return `int32_t`: `0` on success; `-1` when the handle is NULL; `-2` when a qubit index is out of bounds; parameterized gates return `-8` when an angle is NaN or Inf. Single-qubit gates share the signature:

```c
int32_t statevector_apply_h(struct CStatevector *ptr, uint32_t qubit);
```

### Non-parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_h(ptr, qubit)` | H | Hadamard gate. |
| `statevector_apply_x(ptr, qubit)` | X | Pauli-X, bit flip. |
| `statevector_apply_y(ptr, qubit)` | Y | Pauli-Y. |
| `statevector_apply_z(ptr, qubit)` | Z | Pauli-Z, phase flip. |
| `statevector_apply_s(ptr, qubit)` | S | S gate (√Z). |
| `statevector_apply_sdg(ptr, qubit)` | S† | Inverse of S. |
| `statevector_apply_t(ptr, qubit)` | T | T gate (√S). |
| `statevector_apply_tdg(ptr, qubit)` | T† | Inverse of T. |
| `statevector_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`. |
| `statevector_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`. |
| `statevector_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`. |
| `statevector_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`. |

### Parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_rx(ptr, qubit, theta)` | Rx(θ) | Rotation by `θ` about the X axis. |
| `statevector_apply_ry(ptr, qubit, theta)` | Ry(θ) | Rotation by `θ` about the Y axis. |
| `statevector_apply_rz(ptr, qubit, theta)` | Rz(θ) | Rotation by `θ` about the Z axis. |
| `statevector_apply_phase(ptr, qubit, theta)` | P(θ) | Phase gate, multiplies the `\|1>` component by `e^(iθ)`. |
| `statevector_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | Generic single-qubit gate. |
| `statevector_apply_xy(ptr, qubit, theta)` | XY(θ) | XY gate. |
| `statevector_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P gate. |
| `statevector_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M gate. |
| `statevector_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | Rotation by `θ` about the axis at azimuth `φ` in the XY plane. |
| `statevector_apply_gphase(ptr, phi)` | GPhase(φ) | Multiplies the whole state by the global phase `e^(iφ)`; the signature has no qubit parameter. |

GPhase signature:

```c
int32_t statevector_apply_gphase(struct CStatevector *ptr, double phi);
```

### Two-qubit gates

Shared signature `int32_t statevector_apply_<gate>(struct CStatevector *ptr, uint32_t q0, uint32_t q1, ...)`; the two qubit parameters of controlled gates are named `control` / `target`.

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_cx(ptr, control, target)` | CX | Controlled X. |
| `statevector_apply_cy(ptr, control, target)` | CY | Controlled Y. |
| `statevector_apply_cz(ptr, q0, q1)` | CZ | Controlled Z; the two parameters are symmetric. |
| `statevector_apply_swap(ptr, q0, q1)` | SWAP | Swaps two qubits. |
| `statevector_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`. |
| `statevector_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`. |
| `statevector_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`. |
| `statevector_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`. |
| `statevector_apply_crx(ptr, control, target, theta)` | CrX(θ) | Controlled `Rx(θ)`. |
| `statevector_apply_cry(ptr, control, target, theta)` | CrY(θ) | Controlled `Ry(θ)`. |
| `statevector_apply_crz(ptr, control, target, theta)` | CrZ(θ) | Controlled `Rz(θ)`. |
| `statevector_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | Fermionic simulation gate; `theta` is the iSWAP angle and `phi` the controlled-phase angle. |

### Three-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli gate; flips the target when both controls are `\|1>`. |

### Generic gate entry points

#### statevector_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

```c
int32_t statevector_apply_standard_gate(struct CStatevector *ptr,
                                        const char *gate_name,
                                        const uint32_t *qubits,
                                        uintptr_t num_target_qubits,
                                        const double *params,
                                        uintptr_t num_params);
```

Applies a standard gate by name (e.g. `"H"`, `"CX"`, `"RZZ"`, `"FSIM"`), dispatching to the dedicated implementation.

- `ptr` (`CStatevector*`): statevector handle.
- `gate_name` (`const char*`): NUL-terminated standard-gate name.
- `qubits` (`const uint32_t*`): array of target qubits with length `num_target_qubits`.
- `num_target_qubits` (`uintptr_t`): number of target qubits; must match the gate's arity.
- `params` (`const double*`): gate-parameter array; may be NULL when `num_params` is 0.
- `num_params` (`uintptr_t`): number of parameters; must match the gate's arity.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` unknown gate name, gate name that is not valid UTF-8, qubit or parameter count that does not match the gate, or a non-finite parameter value.

#### statevector_apply_unitary_gate(ptr, qubits, num_target_qubits, matrix, dim)

```c
int32_t statevector_apply_unitary_gate(struct CStatevector *ptr,
                                       const uint32_t *qubits,
                                       uintptr_t num_target_qubits,
                                       const Complex64 *matrix,
                                       uintptr_t dim);
```

Applies an arbitrary unitary matrix to the given qubits.

- `ptr` (`CStatevector*`): statevector handle.
- `qubits` (`const uint32_t*`): array of target qubits; indices must not repeat.
- `num_target_qubits` (`uintptr_t`): number of target qubits.
- `matrix` (`const Complex64*`): row-major `dim × dim` matrix.
- `dim` (`uintptr_t`): matrix dimension; must equal `2^num_target_qubits`.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` when `num_target_qubits` or `dim` is 0, the dimension does not match, or a qubit repeats.

#### statevector_apply_pauli_rotation(ptr, pauli, theta)

```c
int32_t statevector_apply_pauli_rotation(struct CStatevector *ptr,
                                         const struct CPauliString *pauli,
                                         double theta);
```

Applies the Pauli-string rotation `exp(-i·θ/2·P)` in place without gate decomposition. The Pauli string must span the full register and be Hermitian; see [Pauli](6_pauli.md) for construction.

- `ptr` (`CStatevector*`): statevector handle.
- `pauli` (`const CPauliString*`): Pauli-string handle.
- `theta` (`double`): rotation angle.

Returns: `0` on success; `-1` NULL pointers; `-8` when `theta` is not finite or the Pauli string's qubit count does not match the state.

---

## Probabilities and Measurement

### statevector_probabilities_len(ptr)

```c
uintptr_t statevector_probabilities_len(const struct CStatevector *ptr);
```

Returns the length of the computational-basis probability distribution (`2^N`).

- `ptr` (`const CStatevector*`): statevector handle.

Returns: the probability count; 0 for NULL.

### statevector_probabilities(ptr, buffer, len)

```c
int32_t statevector_probabilities(const struct CStatevector *ptr, double *buffer, uintptr_t len);
```

Copies the measurement probabilities over all computational basis states (the squared modulus of each amplitude) into `buffer`.

- `ptr` (`const CStatevector*`): statevector handle.
- `buffer` (`double*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `statevector_probabilities_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

### statevector_measure(ptr, qubit)

```c
int32_t statevector_measure(struct CStatevector *ptr, uint32_t qubit);
```

Measures the qubit in the Z basis and collapses the state; after the measurement the state is renormalized within the corresponding subspace. The operation is destructive.

- `ptr` (`CStatevector*`): statevector handle.
- `qubit` (`uint32_t`): the measured qubit.

Returns: `0` for outcome `|0>`, `1` for outcome `|1>`; `-1` NULL; `-2` out-of-bounds qubit.

### statevector_measure_all(ptr)

```c
char *statevector_measure_all(struct CStatevector *ptr);
```

Measures qubits in order `0..num_qubits`, collapsing the state, and returns the outcome as a big-endian bitstring (MSB = qubit `N-1`, LSB = qubit `0`).

- `ptr` (`CStatevector*`): statevector handle.

Returns: a heap-allocated C string, freed with `cqlib_string_free`; `NULL` on error.

### statevector_sample_shots(ptr, shots)

```c
struct COutcomeList *statevector_sample_shots(const struct CStatevector *ptr, uintptr_t shots);
```

Samples `shots` independent measurement outcomes in parallel from the current distribution without modifying the state. Use `outcome_list_len` for the count, `outcome_list_get` to fetch each big-endian bitstring (freed with `cqlib_string_free`), and `outcome_list_free` to free the list — see [ClassicalState](5_classical_state.md).

- `ptr` (`const CStatevector*`): statevector handle.
- `shots` (`uintptr_t`): number of shots.

Returns: a newly allocated `COutcomeList*`; `NULL` on error.

### statevector_reset(ptr, qubit)

```c
int32_t statevector_reset(struct CStatevector *ptr, uint32_t qubit);
```

Resets the qubit to `|0>`: performs a Z-basis measurement and applies an X correction when the outcome is `|1>`.

- `ptr` (`CStatevector*`): statevector handle.
- `qubit` (`uint32_t`): target qubit.

Returns: `0` on success; `-1` NULL; `-2` out-of-bounds qubit.

---

## Expectation Values

### statevector_expectation(ptr, observable, out)

```c
int32_t statevector_expectation(const struct CStatevector *ptr,
                                const struct CHamiltonian *observable,
                                double *out);
```

Computes `⟨ψ|H|ψ⟩` and writes it to `out`.

- `ptr` (`const CStatevector*`): statevector handle.
- `observable` (`const CHamiltonian*`): observable handle, see [Hamiltonian](7_hamiltonian.md).
- `out` (`double*`): output pointer.

Returns: `0` on success; `-1` NULL pointers; `-8` when the observable's qubit count does not match the state.

```c
/* <ZZ> of the Bell state equals 1 */
struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
statevector_expectation(sv, h, &exp);   /* exp ≈ 1.0 */
hamiltonian_free(h);
```

---

## Examples

A complete workflow: prepare a Bell state, read probabilities, sample, compute an expectation value, and measure.

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* Bell state (|00> + |11>)/sqrt(2) */
    struct CStatevector *sv = statevector_new(2);
    if (sv == NULL) return 1;
    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);

    /* Probabilities: P(00) = P(11) = 0.5 */
    uintptr_t n = statevector_probabilities_len(sv);   /* 4 */
    double *probs = malloc(n * sizeof(double));
    statevector_probabilities(sv, probs, n);
    printf("P(00)=%.3f P(11)=%.3f\n", probs[0], probs[3]);
    free(probs);

    /* Sample 100 shots: only "00" and "11" appear */
    struct COutcomeList *shots = statevector_sample_shots(sv, 100);
    for (uintptr_t i = 0; i < outcome_list_len(shots); i++) {
        char *bits = outcome_list_get(shots, i);
        printf("%s ", bits);
        cqlib_string_free(bits);
    }
    printf("\n");
    outcome_list_free(shots);

    /* Expectation <ZZ> = 1 */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *h = hamiltonian_from_pauli(zz);
    double exp = 0.0;
    statevector_expectation(sv, h, &exp);
    printf("<ZZ>=%.3f\n", exp);
    hamiltonian_free(h);

    /* Destructive full measurement collapses the state */
    char *bits = statevector_measure_all(sv);
    printf("measured: %s\n", bits);   /* "00" or "11" */
    cqlib_string_free(bits);

    statevector_free(sv);
    return 0;
}
```
