# DensityMatrix

Density matrix simulator: represents a quantum state with a `2^N × 2^N` density matrix and supports mixed states, noise channels and subsystem operations.

---

## Construction and release

### density_matrix_new(num_qubits)

Creates a density matrix initialized to `\|0...0⟩` (a pure state).

Returns:

- `CDensityMatrix *`: a heap-allocated object, released with `density_matrix_free`; returns NULL on failure.

### density_matrix_from_circuit(circuit)

Simulates `circuit` and returns the final state density matrix.

Returns:

- `CDensityMatrix *`; returns NULL on failure.

### density_matrix_free(ptr)

Releases the object. Accepts NULL.

### density_matrix_num_qubits(ptr)

Returns the number of qubits; a NULL handle returns `0`.

---

## Gate operations

All gate functions use the prefix `density_matrix_apply_` and return an `int32_t` status code: `0` success, `-2` qubit out of range, `-7` simulation failure, `-1` NULL handle. The gate set corresponds one-to-one with [Statevector](1_statevector.md#gate-operations); only the prefix is replaced with `density_matrix_apply_`:

| Group | Function |
| --- | --- |
| Single-qubit without parameters | `apply_i/h/x/y/z/s/sdg/t/tdg/x2p/x2m/y2p/y2m(dm, qubit)` |
| Parameterized single-qubit | `apply_rx/ry/rz/phase/xy/xy2p/xy2m(dm, qubit, theta)`, `apply_u(dm, qubit, theta, phi, lambda)`, `apply_rxy(dm, qubit, theta, phi)`, `apply_gphase(dm, phi)` |
| Two-qubit | `apply_cx/cy/cz/swap(dm, q0, q1)`, `apply_rxx/ryy/rzz/rzx(dm, q0, q1, theta)`, `apply_crx/cry/crz(dm, control, target, theta)`, `apply_fsim(dm, q0, q1, theta, phi)` |
| Three-qubit | `apply_ccx(dm, c0, c1, target)` |
| Whole circuit | `apply_circuit(dm, circuit)` (executes the circuit in place) |

---

## Probabilities and measurement

### density_matrix_probabilities_len(ptr) / density_matrix_probabilities(ptr, buffer, len)

Reads the `2^N` basis state probability distribution in two steps, with the same semantics as the corresponding interface of [Statevector](1_statevector.md#probabilities-and-measurement); `density_matrix_probabilities` returns `0` on success and a negative status code on failure.

### density_matrix_expectation(ptr, observable, out)

Computes `⟨H⟩` and writes it to `out`; `0` on success, a negative status code on failure. For a mixed state it is computed as `Tr(Hρ)`.

### density_matrix_measure(ptr, qubit)

Measures `qubit` in the Z basis and collapses. Unlike Statevector, the density matrix version does **not return** the measurement result (it returns an `int32_t` status code, `0` on success).

### density_matrix_measure_all(ptr)

Measures all qubits and collapses; returns the heap-allocated big-endian bit string, released with `cqlib_string_free`; returns NULL on failure.

### density_matrix_sample_shots(ptr, shots)

Independently samples `shots` times and returns `COutcomeList *`; for usage see [Overview](0_overview.md#sampling-results-outcomelist).

### density_matrix_reset(ptr, qubit)

Resets `qubit` to `\|0⟩`. `0` on success, a negative status code on failure.

---

## Kraus channels and partial trace

### density_matrix_apply_kraus(ptr, ops, num_ops, op_dim, qubits, num_target_qubits)

Applies the Kraus noise channel `ρ -> Σ_i K_i ρ K_i†` to the target qubits.

Parameters:

- `ops` (`const Complex64 *`): a flat array of `num_ops × op_dim × op_dim` entries in **row-major order with the real and imaginary parts interleaved**, arranged in the order `K0, K1, ...`.
- `num_ops` (`uintptr_t`): the number of Kraus operators.
- `op_dim` (`uintptr_t`): the dimension of a single operator; must equal `2^num_target_qubits`.
- `qubits` (`const uint32_t *`): the array of target qubits.
- `num_target_qubits` (`uintptr_t`): the number of target qubits.

Returns:

- `int32_t`; `0` on success, `-3` for an invalid structure, `-8` for a parameter mismatch (`op_dim` does not match the number of qubits, and so on).

### density_matrix_partial_trace(ptr, keep_qubits, num_keep)

Takes the partial trace over the qubits outside `keep_qubits` and returns the reduced density matrix that keeps only the specified qubits.

Parameters:

- `keep_qubits` (`const uint32_t *`): the array of qubits to keep.
- `num_keep` (`uintptr_t`): the number of qubits to keep.

Returns:

- `CDensityMatrix *`: a new **independently owned** object, released with `density_matrix_free`; returns NULL on failure.

---

## Example

### Amplitude damping channel

Applies amplitude damping to 1 target qubit (`op_dim = 2`):

```c
double gamma = 0.1;
Complex64 k0[2] = {{1.0, 0.0}, {0.0, 0.0},
                   {0.0, 0.0}, {sqrt(1.0 - gamma), 0.0}};   // diag(1, √(1-γ))
Complex64 k1[2] = {{0.0, 0.0}, {0.0, 0.0},
                   {sqrt(gamma), 0.0}, {0.0, 0.0}};         // √γ |0><1|

Complex64 ops[4];
memcpy(ops, k0, sizeof(k0));
memcpy(ops + 2, k1, sizeof(k1));

uint32_t target[1] = {0};
density_matrix_apply_kraus(dm, ops, 2, 2, target, 1);
```

### Partial trace and expectation

```c
CDensityMatrix *dm = density_matrix_from_circuit(qc);

/* 纠缠后保留 q0，得到 q0 的约化密度矩阵 */
uint32_t keep[1] = {0};
CDensityMatrix *reduced = density_matrix_partial_trace(dm, keep, 1);

double probs[2];
density_matrix_probabilities(reduced, probs, 2);   // 各 0.5（最大混合）
density_matrix_free(reduced);

density_matrix_free(dm);
```
