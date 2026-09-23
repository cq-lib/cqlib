# Statevector

Statevector simulator: represents a pure state exactly with `2^N` complex amplitudes, supports all standard gates, and suits exact simulation of N ≤ 30 qubits.

---

## Construction and release

### statevector_new(num_qubits)

Creates a statevector initialized to `\|0...0⟩`.

Parameters:

- `num_qubits` (`uintptr_t`): the number of qubits.

Returns:

- `CStatevector *`: a heap-allocated object, released with `statevector_free`; returns NULL on failure.

### statevector_from_circuit(circuit)

Simulates `circuit` and returns the final state.

Parameters:

- `circuit` (`const CCircuit *`): the input circuit.

Returns:

- `CStatevector *`; returns NULL on failure (NULL, simulation failure, and so on).

### statevector_free(ptr)

Releases the statevector. Accepts NULL.

### statevector_num_qubits(ptr)

Returns the number of qubits; a NULL handle returns `0`.

---

## Gate operations

All gate functions use the prefix `statevector_apply_` and return an `int32_t` status code: `0` success, `-2` qubit out of range, `-7` simulation failure, `-1` NULL handle. The groups correspond one-to-one with [Circuit gate operations](../0_circuit/1_circuit.md#gate-operations).

### 1. Single-qubit gates without parameters

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_i(sv, qubit)` | `I` | Identity gate |
| `statevector_apply_h(sv, qubit)` | `H` | Hadamard gate |
| `statevector_apply_x(sv, qubit)` | `X` | Pauli-X gate |
| `statevector_apply_y(sv, qubit)` | `Y` | Pauli-Y gate |
| `statevector_apply_z(sv, qubit)` | `Z` | Pauli-Z gate |
| `statevector_apply_s(sv, qubit)` | `S` | S gate |
| `statevector_apply_sdg(sv, qubit)` | `S†` | S dagger gate |
| `statevector_apply_t(sv, qubit)` | `T` | T gate |
| `statevector_apply_tdg(sv, qubit)` | `T†` | T dagger gate |
| `statevector_apply_x2p(sv, qubit)` | `X2P` | Rotation by +π/2 about the X axis |
| `statevector_apply_x2m(sv, qubit)` | `X2M` | Rotation by −π/2 about the X axis |
| `statevector_apply_y2p(sv, qubit)` | `Y2P` | Rotation by +π/2 about the Y axis |
| `statevector_apply_y2m(sv, qubit)` | `Y2M` | Rotation by −π/2 about the Y axis |

### 2. Parameterized single-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_rx(sv, qubit, theta)` | `RX` | Rotation about the X axis |
| `statevector_apply_ry(sv, qubit, theta)` | `RY` | Rotation about the Y axis |
| `statevector_apply_rz(sv, qubit, theta)` | `RZ` | Rotation about the Z axis |
| `statevector_apply_phase(sv, qubit, theta)` | `P` | Phase gate |
| `statevector_apply_u(sv, qubit, theta, phi, lambda)` | `U` | General single-qubit gate |
| `statevector_apply_xy(sv, qubit, theta)` | `XY` | XY interaction gate |
| `statevector_apply_xy2p(sv, qubit, theta)` | `XY2P` | Positive half-angle XY gate |
| `statevector_apply_xy2m(sv, qubit, theta)` | `XY2M` | Negative half-angle XY gate |
| `statevector_apply_rxy(sv, qubit, theta, phi)` | `RXY` | Rotation about an arbitrary axis in the XY plane |
| `statevector_apply_gphase(sv, phi)` | `gphase` | Global phase |

### 3. Two-qubit gates

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_cx(sv, control, target)` | `CX` | CNOT gate |
| `statevector_apply_cy(sv, control, target)` | `CY` | controlled-Y gate |
| `statevector_apply_cz(sv, q0, q1)` | `CZ` | controlled-Z gate |
| `statevector_apply_swap(sv, q0, q1)` | `SWAP` | Swaps the states of the two qubits |
| `statevector_apply_rxx(sv, q0, q1, theta)` | `RXX` | `exp(-i θ XX / 2)` |
| `statevector_apply_ryy(sv, q0, q1, theta)` | `RYY` | `exp(-i θ YY / 2)` |
| `statevector_apply_rzz(sv, q0, q1, theta)` | `RZZ` | `exp(-i θ ZZ / 2)` |
| `statevector_apply_rzx(sv, q0, q1, theta)` | `RZX` | `exp(-i θ ZX / 2)` |
| `statevector_apply_crx(sv, control, target, theta)` | `CRX` | controlled-RX gate |
| `statevector_apply_cry(sv, control, target, theta)` | `CRY` | controlled-RY gate |
| `statevector_apply_crz(sv, control, target, theta)` | `CRZ` | controlled-RZ gate |
| `statevector_apply_fsim(sv, q0, q1, theta, phi)` | `fSim` | fSim gate |

### 4. Three-qubit gates and whole circuits

| Function | Gate | Description |
| --- | --- | --- |
| `statevector_apply_ccx(sv, c0, c1, target)` | `CCX` | Toffoli gate |
| `statevector_apply_circuit(sv, circuit)` | — | Executes the whole circuit **in place** (measurement/reset in the circuit take effect as well) |

---

## Probabilities and measurement

### statevector_probabilities_len(ptr)

Returns the number of basis state probabilities, `2^N`; a NULL handle returns `0`. Used to allocate the `buffer` of `statevector_probabilities`.

### statevector_probabilities(ptr, buffer, len)

Copies the `2^N` basis state probabilities (computational basis `|q_{N-1}...q_0⟩`, with indices in big-endian binary order) into `buffer`.

Returns:

- `int32_t`; `0` on success, a negative status code on an error such as an insufficient buffer.

### statevector_expectation(ptr, observable, out)

Computes the observable expectation value `⟨H⟩` and writes it to `out`.

Parameters:

- `observable` (`const CHamiltonian *`): see [Hamiltonian](5_hamiltonian.md).

Returns:

- `int32_t`; `0` on success, a negative status code on failure.

### statevector_measure(ptr, qubit)

Measures `qubit` in the Z basis and **collapses** the statevector.

Returns:

- `int32_t`: the measurement result `0` (`\|0⟩`) or `1` (`\|1⟩`); a negative status code on failure.

### statevector_measure_all(ptr)

Measures all qubits and collapses.

Returns:

- `char *`: the heap-allocated big-endian bit string (the MSB is qubit `N-1`, the LSB is qubit `0`), released with `cqlib_string_free`; returns NULL on failure.

### statevector_sample_shots(ptr, shots)

Independently samples `shots` measurement results (the original state is not collapsed).

Returns:

- `COutcomeList *`: a heap-allocated sampling list, released with `outcome_list_free`; for usage see [Overview](0_overview.md#sampling-results-outcomelist); returns NULL on failure.

### statevector_reset(ptr, qubit)

Resets `qubit` to `\|0⟩` (a conditional flip after measurement).

Returns:

- `int32_t`; `0` on success, a negative status code on failure.

---

## Complete example

```c
/* 构造 Bell 线路并模拟 */
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

CStatevector *sv = statevector_from_circuit(qc);

/* 概率分布 */
double probs[4];
statevector_probabilities(sv, probs, 4);
/* probs = [0.5, 0, 0, 0.5] */

/* 采样 */
COutcomeList *samples = statevector_sample_shots(sv, 1000);
for (uintptr_t i = 0; i < outcome_list_len(samples); i++) {
    char *s = outcome_list_get(samples, i);
    printf("%s\n", s);
    cqlib_string_free(s);
}
outcome_list_free(samples);

/* 坍缩测量 */
int32_t bit = statevector_measure(sv, 0);      // 0 或 1
char *all = statevector_measure_all(sv);       // 如 "11"
cqlib_string_free(all);

statevector_free(sv);
circuit_free(qc);
```

### Gate-by-gate construction and Hamiltonian expectation

```c
CStatevector *sv = statevector_new(1);
statevector_apply_h(sv, 0);
statevector_apply_rz(sv, 0, 0.5);

/* ⟨Z⟩ */
CPauliString *z = pauli_string_parse("Z");
CHamiltonian *h = hamiltonian_from_pauli(z);   // 接管 z 所有权

double ez;
statevector_expectation(sv, h, &ez);
printf("<Z> = %f\n", ez);

hamiltonian_free(h);
statevector_free(sv);
```
