# DensityMatrixNoise

The `CDensityMatrixNoise*` handle is the noisy mixed-state simulator: it layers a `CNoiseModel` on top of the density-matrix simulator, so that after every gate the model is consulted for a noise entry matching the same gate on the same qubits and the corresponding channel is applied; readout errors only change the reported probability distribution. This page covers construction and circuit ingestion, gate operations, probabilities and readout noise, measurement and sampling, and expectation values. Error codes and memory conventions are described in [Overview](../0_overview.md); for the module summary see [QIS Overview](0_overview.md).

---

## Construction and Release

### density_matrix_noise_new(num_qubits, noise_model)

```c
struct CDensityMatrixNoise *density_matrix_noise_new(uintptr_t num_qubits,
                                                     const struct CNoiseModel *noise_model);
```

Creates the noisy simulator initialized to `|0...0><0...0|`; pass NULL as `noise_model` for ideal (noiseless) simulation.

- `num_qubits` (`uintptr_t`): the number of qubits.
- `noise_model` (`const CNoiseModel*`): noise-model handle, may be NULL; it is cloned at construction, so the model handle can be released independently afterwards.

Returns: a newly allocated `CDensityMatrixNoise*`; free it with `density_matrix_noise_free`.

### density_matrix_noise_from_circuit(circuit, noise_model)

```c
struct CDensityMatrixNoise *density_matrix_noise_from_circuit(const struct CCircuit *circuit,
                                                              const struct CNoiseModel *noise_model);
```

Executes the circuit and returns the noisy simulator: the circuit is first decomposed into basic gates, with noise applied after each gate; pass NULL as `noise_model` for ideal simulation.

- `circuit` (`const CCircuit*`): circuit handle, see [Circuit](../0_circuit/1_circuit.md).
- `noise_model` (`const CNoiseModel*`): noise-model handle, may be NULL.

Returns: a newly allocated `CDensityMatrixNoise*`; `NULL` when `circuit` is NULL or execution fails.

### density_matrix_noise_free(ptr)

```c
void density_matrix_noise_free(struct CDensityMatrixNoise *ptr);
```

Frees the handle. Passing NULL is allowed.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.

### density_matrix_noise_num_qubits(ptr)

```c
uintptr_t density_matrix_noise_num_qubits(const struct CDensityMatrixNoise *ptr);
```

Returns the number of qubits.

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.

Returns: the qubit count; 0 for NULL.

### density_matrix_noise_apply_circuit(ptr, circuit)

```c
int32_t density_matrix_noise_apply_circuit(struct CDensityMatrixNoise *ptr,
                                           const struct CCircuit *circuit);
```

Applies the circuit in place to the current simulator: the circuit is first decomposed, and after each gate the matching gate noise from the noise model is injected.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.
- `circuit` (`const CCircuit*`): circuit handle.

Returns: `0` on success; `-1` NULL pointer; `-3` circuit error; `-7` the circuit contains an unsupported operation; `-8` the circuit's qubit count does not match the simulator.

---

## Gate Operations

All gate functions return `int32_t`: `0` on success; `-1` when the handle is NULL; `-2` when a qubit index is out of bounds; parameterized gates return `-8` when an angle is NaN or Inf. After each gate, matching gate noise from the noise model is injected; when no model is configured or no entry matches, only the ideal gate is applied. Single-qubit gates share the signature:

```c
int32_t density_matrix_noise_apply_h(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

### Non-parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_noise_apply_h(ptr, qubit)` | H | Hadamard gate. |
| `density_matrix_noise_apply_x(ptr, qubit)` | X | Pauli-X, bit flip. |
| `density_matrix_noise_apply_y(ptr, qubit)` | Y | Pauli-Y. |
| `density_matrix_noise_apply_z(ptr, qubit)` | Z | Pauli-Z, phase flip. |
| `density_matrix_noise_apply_s(ptr, qubit)` | S | S gate (√Z). |
| `density_matrix_noise_apply_sdg(ptr, qubit)` | S† | Inverse of S. |
| `density_matrix_noise_apply_t(ptr, qubit)` | T | T gate (√S). |
| `density_matrix_noise_apply_tdg(ptr, qubit)` | T† | Inverse of T. |
| `density_matrix_noise_apply_x2p(ptr, qubit)` | X2P | `Rx(π/2)`. |
| `density_matrix_noise_apply_x2m(ptr, qubit)` | X2M | `Rx(-π/2)`. |
| `density_matrix_noise_apply_y2p(ptr, qubit)` | Y2P | `Ry(π/2)`. |
| `density_matrix_noise_apply_y2m(ptr, qubit)` | Y2M | `Ry(-π/2)`. |

### Parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_noise_apply_rx(ptr, qubit, theta)` | Rx(θ) | Rotation by `θ` about the X axis. |
| `density_matrix_noise_apply_ry(ptr, qubit, theta)` | Ry(θ) | Rotation by `θ` about the Y axis. |
| `density_matrix_noise_apply_rz(ptr, qubit, theta)` | Rz(θ) | Rotation by `θ` about the Z axis. |
| `density_matrix_noise_apply_phase(ptr, qubit, theta)` | P(θ) | Phase gate, multiplies the `\|1>` component by `e^(iθ)`. |
| `density_matrix_noise_apply_u(ptr, qubit, theta, phi, lambda)` | U(θ, φ, λ) | Generic single-qubit gate. |
| `density_matrix_noise_apply_xy(ptr, qubit, theta)` | XY(θ) | XY gate. |
| `density_matrix_noise_apply_xy2p(ptr, qubit, theta)` | XY2P(θ) | XY2P gate. |
| `density_matrix_noise_apply_xy2m(ptr, qubit, theta)` | XY2M(θ) | XY2M gate. |
| `density_matrix_noise_apply_rxy(ptr, qubit, theta, phi)` | RXY(θ, φ) | Rotation by `θ` about the axis at azimuth `φ` in the XY plane. |
| `density_matrix_noise_apply_gphase(ptr, phi)` | GPhase(φ) | Multiplies the whole state by the global phase `e^(iφ)`; the signature has no qubit parameter. |

GPhase signature:

```c
int32_t density_matrix_noise_apply_gphase(struct CDensityMatrixNoise *ptr, double phi);
```

### Two-qubit gates

Shared signature `int32_t density_matrix_noise_apply_<gate>(struct CDensityMatrixNoise *ptr, uint32_t q0, uint32_t q1, ...)`; the two qubit parameters of controlled gates are named `control` / `target`.

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_noise_apply_cx(ptr, control, target)` | CX | Controlled X. |
| `density_matrix_noise_apply_cy(ptr, control, target)` | CY | Controlled Y. |
| `density_matrix_noise_apply_cz(ptr, q0, q1)` | CZ | Controlled Z; the two parameters are symmetric. |
| `density_matrix_noise_apply_swap(ptr, q0, q1)` | SWAP | Swaps two qubits. |
| `density_matrix_noise_apply_rxx(ptr, q0, q1, theta)` | RXX(θ) | `exp(-i·θ/2·X⊗X)`. |
| `density_matrix_noise_apply_ryy(ptr, q0, q1, theta)` | RYY(θ) | `exp(-i·θ/2·Y⊗Y)`. |
| `density_matrix_noise_apply_rzz(ptr, q0, q1, theta)` | RZZ(θ) | `exp(-i·θ/2·Z⊗Z)`. |
| `density_matrix_noise_apply_rzx(ptr, q0, q1, theta)` | RZX(θ) | `exp(-i·θ/2·Z⊗X)`. |
| `density_matrix_noise_apply_crx(ptr, control, target, theta)` | CrX(θ) | Controlled `Rx(θ)`. |
| `density_matrix_noise_apply_cry(ptr, control, target, theta)` | CrY(θ) | Controlled `Ry(θ)`. |
| `density_matrix_noise_apply_crz(ptr, control, target, theta)` | CrZ(θ) | Controlled `Rz(θ)`. |
| `density_matrix_noise_apply_fsim(ptr, q0, q1, theta, phi)` | FSIM | Fermionic simulation gate; `theta` is the iSWAP angle and `phi` the controlled-phase angle. |

### Three-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `density_matrix_noise_apply_ccx(ptr, c0, c1, target)` | CCX | Toffoli gate; flips the target when both controls are `\|1>`. |

### Generic gate entry points

#### density_matrix_noise_apply_standard_gate_noise(ptr, gate_name, qubits, qubit_len, params, param_len)

```c
int32_t density_matrix_noise_apply_standard_gate_noise(struct CDensityMatrixNoise *ptr,
                                                       const char *gate_name,
                                                       const uint32_t *qubits,
                                                       uintptr_t qubit_len,
                                                       const double *params,
                                                       uintptr_t param_len);
```

Applies a standard gate by name (e.g. `"H"`, `"CX"`, `"RZZ"`, `"GPhase"`, `"FSIM"`): the ideal gate is applied first, then the gate noise matching the noise model.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.
- `gate_name` (`const char*`): NUL-terminated standard-gate name.
- `qubits` (`const uint32_t*`): array of target qubits with length `qubit_len`; may be NULL when `qubit_len` is 0.
- `qubit_len` (`uintptr_t`): number of target qubits; must match the gate's arity.
- `params` (`const double*`): gate-parameter array; may be NULL when `param_len` is 0.
- `param_len` (`uintptr_t`): number of parameters; must match the gate's arity.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-4` the gate name is not valid UTF-8; `-8` unknown gate name, qubit or parameter count that does not match the gate, or a non-finite parameter value.

#### density_matrix_noise_apply_unitary_gate(ptr, qubits, qubit_len, matrix, matrix_dim)

```c
int32_t density_matrix_noise_apply_unitary_gate(struct CDensityMatrixNoise *ptr,
                                                const uint32_t *qubits,
                                                uintptr_t qubit_len,
                                                const Complex64 *matrix,
                                                uintptr_t matrix_dim);
```

Applies an arbitrary unitary as `ρ → U ρ U†`. A generic unitary has no corresponding standard-gate type, so no gate noise is injected; use the dedicated gate functions when noise modeling is required.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.
- `qubits` (`const uint32_t*`): array of target qubits; indices must not repeat.
- `qubit_len` (`uintptr_t`): number of target qubits.
- `matrix` (`const Complex64*`): row-major `matrix_dim × matrix_dim` matrix.
- `matrix_dim` (`uintptr_t`): matrix dimension; must equal `2^qubit_len`.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` when `qubit_len` or `matrix_dim` is 0, the dimension does not match, or a qubit repeats.

---

## Probabilities and Readout Noise

The simulator separates noise of the quantum state from noise of the readout: gate noise acts on the density matrix itself, while readout errors only change the reported probability distribution.

### density_matrix_noise_probabilities_len(ptr)

```c
uintptr_t density_matrix_noise_probabilities_len(const struct CDensityMatrixNoise *ptr);
```

Returns the length of the computational-basis probability distribution (`2^N`).

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.

Returns: the probability count; 0 for NULL.

### density_matrix_noise_probabilities(ptr, buffer, len)

```c
int32_t density_matrix_noise_probabilities(const struct CDensityMatrixNoise *ptr,
                                           double *buffer,
                                           uintptr_t len);
```

Copies the ideal probability distribution (the density-matrix diagonal, including applied gate noise but excluding readout errors) into `buffer`.

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.
- `buffer` (`double*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `density_matrix_noise_probabilities_len`.

Returns: `0` on success; `-1` NULL pointers; `-8` when `len` does not match the actual count.

### density_matrix_noise_probabilities_with_readout_len(ptr)

```c
uintptr_t density_matrix_noise_probabilities_with_readout_len(const struct CDensityMatrixNoise *ptr);
```

Returns the buffer length of the readout-error probability distribution (`2^N`); the buffer size is independent of the qubit selection passed to `density_matrix_noise_probabilities_with_readout`.

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.

Returns: the probability count; 0 for NULL.

### density_matrix_noise_probabilities_with_readout(ptr, buffer, len, qubits, qubit_len)

```c
int32_t density_matrix_noise_probabilities_with_readout(const struct CDensityMatrixNoise *ptr,
                                                        double *buffer,
                                                        uintptr_t len,
                                                        const uint32_t *qubits,
                                                        uintptr_t qubit_len);
```

Takes the ideal probability distribution and applies the readout errors registered in the noise model for the qubits listed in `qubits`, copying the result into `buffer`.

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.
- `buffer` (`double*`): output buffer.
- `len` (`uintptr_t`): buffer length; must equal `density_matrix_noise_probabilities_with_readout_len`.
- `qubits` (`const uint32_t*`): array of qubits to apply readout errors to; may be NULL when `qubit_len` is 0, in which case the result matches `density_matrix_noise_probabilities`.
- `qubit_len` (`uintptr_t`): number of qubits.

Returns: `0` on success; `-1` NULL pointers; `-2` out-of-bounds qubit; `-8` when `len` does not match the actual count.

---

## Measurement, Reset, and Sampling

### density_matrix_noise_measure(ptr, qubit)

```c
int32_t density_matrix_noise_measure(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

Measures the qubit in the Z basis and collapses the underlying density matrix; the measurement itself applies no noise.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.
- `qubit` (`uint32_t`): the measured qubit.

Returns: `0` for outcome `|0>`, `1` for outcome `|1>`; `-1` NULL; `-2` out-of-bounds qubit.

### density_matrix_noise_measure_all(ptr)

```c
char *density_matrix_noise_measure_all(struct CDensityMatrixNoise *ptr);
```

Measures qubits in order `0..num_qubits`, collapsing the state, and returns the outcome as a big-endian bitstring (MSB = qubit `N-1`, LSB = qubit `0`).

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.

Returns: a heap-allocated C string, freed with `cqlib_string_free`; `NULL` on error.

### density_matrix_noise_reset(ptr, qubit)

```c
int32_t density_matrix_noise_reset(struct CDensityMatrixNoise *ptr, uint32_t qubit);
```

Resets the qubit to `|0>`; the reset applies no readout error.

- `ptr` (`CDensityMatrixNoise*`): noisy-simulator handle.
- `qubit` (`uint32_t`): target qubit.

Returns: `0` on success; `-1` NULL; `-2` out-of-bounds qubit.

### density_matrix_noise_sample_shots(ptr, shots)

```c
struct COutcomeList *density_matrix_noise_sample_shots(const struct CDensityMatrixNoise *ptr,
                                                       uintptr_t shots);
```

Samples `shots` independent measurement outcomes in parallel from the current distribution without modifying the state; the sampling draws from the quantum state's own distribution and excludes readout errors. Use `outcome_list_len` for the count, `outcome_list_get` to fetch each big-endian bitstring (freed with `cqlib_string_free`), and `outcome_list_free` to free the list — see [ClassicalState](5_classical_state.md).

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.
- `shots` (`uintptr_t`): number of shots.

Returns: a newly allocated `COutcomeList*`; `NULL` on error.

---

## Expectation Values

### density_matrix_noise_expectation(ptr, observable, out)

```c
int32_t density_matrix_noise_expectation(const struct CDensityMatrixNoise *ptr,
                                         const struct CHamiltonian *observable,
                                         double *out);
```

Computes `<H> = Tr(H·ρ)` and writes it to `out`; the result includes the applied gate noise but excludes readout errors (readout errors do not affect the quantum state).

- `ptr` (`const CDensityMatrixNoise*`): noisy-simulator handle.
- `observable` (`const CHamiltonian*`): observable handle, see [Hamiltonian](7_hamiltonian.md).
- `out` (`double*`): output pointer.

Returns: `0` on success; `-1` NULL pointers; `-8` when the observable's qubit count does not match the simulator.

---

## Configuring the noise model

The `CNoiseModel` handle comes from the device module; its construction and registration functions include `noise_model_new`, `noise_model_add_single_qubit` (selecting a channel by the `NOISE_BIT_FLIP` style tags), `noise_model_add_single_qubit_pauli`, `noise_model_add_two_qubit` (`NOISE_TWO_DEPOLARIZING`), and `noise_model_add_readout` (asymmetric readout errors `p_0_given_1` / `p_1_given_0`). See [NoiseModel](../2_device/4_noise.md).

---

## Examples

### 1. Bell state with gate noise and readout noise

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 5% depolarizing on CX(0,1), 10% readout error on qubit 0 */
    struct CNoiseModel *model = noise_model_new();
    noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.05);
    noise_model_add_readout(model, 0, 0.0, 0.1);

    struct CDensityMatrixNoise *sim = density_matrix_noise_new(2, model);
    density_matrix_noise_apply_h(sim, 0);
    density_matrix_noise_apply_cx(sim, 0, 1);

    /* Ideal distribution: gate noise only, no readout error */
    double ideal[4];
    density_matrix_noise_probabilities(sim, ideal, 4);
    printf("ideal   P(00)=%.3f P(11)=%.3f\n", ideal[0], ideal[3]);

    /* With readout error applied to qubit 0 */
    double with_ro[4];
    uint32_t measured[1] = {0};
    density_matrix_noise_probabilities_with_readout(sim, with_ro, 4, measured, 1);
    printf("readout P(00)=%.3f P(11)=%.3f\n", with_ro[0], with_ro[3]);

    density_matrix_noise_free(sim);
    noise_model_free(model);
    return 0;
}
```

### 2. Constructing from a circuit and computing an expectation value

```c
struct CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

struct CNoiseModel *model = noise_model_new();
noise_model_add_two_qubit(model, "CX", 0, 1, NOISE_TWO_DEPOLARIZING, 0.05);

struct CDensityMatrixNoise *sim = density_matrix_noise_from_circuit(qc, model);

/* <ZZ> stays close to 1 under weak depolarizing noise */
struct CPauliString *zz = pauli_string_parse("ZZ");
struct CHamiltonian *h = hamiltonian_from_pauli(zz);  /* takes ownership of zz */
double exp = 0.0;
density_matrix_noise_expectation(sim, h, &exp);
printf("<ZZ>=%.3f\n", exp);
hamiltonian_free(h);

density_matrix_noise_free(sim);
noise_model_free(model);
circuit_free(qc);
```
