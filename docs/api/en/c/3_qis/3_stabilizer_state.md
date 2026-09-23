# StabilizerState

Stabilizer simulator: represents a quantum state with a stabilizer table, supports only the Clifford gate set, and can simulate large-scale (thousands of qubits) circuits.

---

## Construction and release

### stabilizer_new(num_qubits)

Creates a stabilizer state with the `\|0...0⟩` initial state.

Returns:

- `CStabilizerState *`: a heap-allocated object, released with `stabilizer_free`; returns NULL on failure.

### stabilizer_from_circuit(circuit)

Simulates a Clifford circuit and returns the final state.

Returns:

- `CStabilizerState *`; when the circuit contains a non-Clifford gate the simulation fails and returns NULL (applying gates one by one returns `-7`).

### stabilizer_free(ptr)

Releases the object. Accepts NULL.

### stabilizer_num_qubits(ptr)

Returns the number of qubits; a NULL handle returns `0`.

---

## Gate operations

The prefix is `stabilizer_apply_`; returns an `int32_t` status code: `0` success, `-2` qubit out of range, `-7` non-Clifford gate, `-1` NULL handle.

| Group | Function | Gate |
| --- | --- | --- |
| Single-qubit without parameters | `apply_h/x/y/z/s/sdg/x2p/x2m/y2p/y2m(st, qubit)` | Within the Clifford set |
| Two-qubit | `apply_cx/cy/cz/swap(st, q0, q1)` | Within the Clifford set |
| Whole circuit | `apply_circuit(st, circuit)` | The circuit must consist entirely of Clifford gates |

**Only the Clifford gate set is supported**: `H`, `S`, `S†`, `X`, `Y`, `Z`, `X2P/X2M/Y2P/Y2M` (±π/2 rotations) and `CX`, `CY`, `CZ`, `SWAP`. Non-Clifford gates (such as `T`, `T†`, `RX`, rotations by an arbitrary angle) are not supported; applying one returns `-7`.

---

## Probabilities and measurement

### stabilizer_probabilities_len(ptr) / stabilizer_probabilities(ptr, buffer, len)

Reads the `2^N` basis state probability distribution in two steps, with the same semantics as [Statevector](1_statevector.md#probabilities-and-measurement).

### stabilizer_measure(ptr, qubit)

Measures `qubit` in the Z basis and updates the stabilizer table.

Returns:

- `int32_t`: the measurement result `0`/`\|0⟩` or `1`/`\|1⟩`; a negative status code on failure.

### stabilizer_measure_all(ptr)

Measures all qubits and collapses; returns the heap-allocated big-endian bit string, released with `cqlib_string_free`; returns NULL on failure.

### stabilizer_sample_shots(ptr, shots)

Independently samples `shots` times and returns `COutcomeList *`; for usage see [Overview](0_overview.md#sampling-results-outcomelist).

### stabilizer_reset(ptr, qubit)

Resets `qubit` to `\|0⟩`. `0` on success, a negative status code on failure.

---

## Stabilizer access

### stabilizer_pauli_expectation(ptr, pauli, out)

Computes the Pauli string expectation value `⟨P⟩` and writes it to `out`.

Parameters:

- `pauli` (`const CPauliString *`): see [PauliString](4_pauli_string.md).

Returns:

- `int32_t`; `0` on success, a negative status code on failure.

The value is always `-1`, `0` or `+1` (the eigenvalue of a Pauli operator on a stabilizer state can only be ±1; it is `0` when the Pauli string does not commute with the stabilizer group).

### stabilizer_get_stabilizers_len(ptr)

Returns the number of stabilizer generators (always equal to the number of qubits `N`); a NULL handle returns `0`.

### stabilizer_get_stabilizer(ptr, index)

Returns the string form of the `index`-th generator (such as `"+XX"`: a sign bit followed by the Pauli letter of each qubit).

Returns:

- `char *`: a heap-allocated string, released with `cqlib_string_free`; out of range returns NULL.

---

## Example

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);        // Bell 态制备（全 Clifford）

CStabilizerState *st = stabilizer_from_circuit(qc);

/* Pauli 期望：Bell 态 ⟨ZZ⟩ = +1，⟨XX⟩ = +1 */
CPauliString *zz = pauli_string_parse("ZZ");
CPauliString *xx = pauli_string_parse("XX");

int32_t vzz, vxx;
stabilizer_pauli_expectation(st, zz, &vzz);   // +1
stabilizer_pauli_expectation(st, xx, &vxx);   // +1
printf("<ZZ>=%d <XX>=%d\n", vzz, vxx);

pauli_string_free(zz);
pauli_string_free(xx);

/* 生成元列表 */
uintptr_t n = stabilizer_get_stabilizers_len(st);   // 2
for (uintptr_t i = 0; i < n; i++) {
    char *gen = stabilizer_get_stabilizer(st, i);
    printf("g%zu = %s\n", (size_t)i, gen);          // 如 "+XX"、"+ZZ"
    cqlib_string_free(gen);
}

stabilizer_free(st);
circuit_free(qc);
```
