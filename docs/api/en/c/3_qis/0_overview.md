# Quantum information

The QIS module provides three quantum state simulators and observable tools.

---

## Simulator comparison

| Simulator | Page | State representation | Gate set | Use case |
| --- | --- | --- | --- | --- |
| Statevector | [Statevector](1_statevector.md) | `2^N` complex amplitudes | All standard gates | Exact simulation of N ≤ 30 qubits |
| DensityMatrix | [DensityMatrix](2_density_matrix.md) | `2^N × 2^N` density matrix | All standard gates + Kraus channels | Mixed states, noise, subsystems |
| StabilizerState | [StabilizerState](3_stabilizer_state.md) | Stabilizer table | Clifford gate set only | Large-scale (thousands of qubits) Clifford circuits |

The three simulators share an isomorphic interface family:

- **Construction**: `<sim>_new(n)` creates the `\|0…0⟩` initial state; `<sim>_from_circuit(circuit)` simulates the circuit directly to obtain the final state.
- **Gate operations**: the prefix is `apply_`, and the groups correspond one-to-one with [Circuit gate operations](../0_circuit/1_circuit.md#gate-operations); an unsupported gate returns `-7`.
- **Measurement and sampling**: `measure(qubit)` single-qubit collapse measurement, `measure_all()` full measurement (returns a big-endian bit string), `sample_shots(shots)` independent sampling, `reset(qubit)` reset, and `probabilities_len/probabilities` two-step probability reading.
- **Observables**: `expectation(hamiltonian, out)` computes `⟨H⟩`; StabilizerState uses `pauli_expectation` (the result is always `-1/0/+1`).

Observable tools: [PauliString](4_pauli_string.md) (Pauli strings) and [Hamiltonian](5_hamiltonian.md) (Hermitian operators).

---

## Sampling results OutcomeList

The return type shared by `statevector_sample_shots`, `density_matrix_sample_shots` and `stabilizer_sample_shots`:

```c
void outcome_list_free(struct COutcomeList *ptr);
uintptr_t outcome_list_len(const struct COutcomeList *ptr);
char *outcome_list_get(const struct COutcomeList *ptr, uintptr_t index);
```

| Function | Description |
| --- | --- |
| `outcome_list_len` | Number of sampling results |
| `outcome_list_get(index)` | Returns a big-endian binary bit string (the MSB is the highest qubit), heap-allocated and released with `cqlib_string_free`; out of range returns NULL |
| `outcome_list_free` | Releases the list; accepts NULL |

```c
COutcomeList *samples = statevector_sample_shots(sv, 1000);
for (uintptr_t i = 0; i < outcome_list_len(samples); i++) {
    char *s = outcome_list_get(samples, i);        // "00" / "11" / ...
    printf("%s\n", s);
    cqlib_string_free(s);
}
outcome_list_free(samples);
```
