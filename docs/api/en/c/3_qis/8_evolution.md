# Pauli Evolution (C)

This page covers exponentiating a single Pauli string `P` into the evolution operator `e^(-i·θ/2·P)` and appending it to a circuit. Time-evolution circuits for a full Hamiltonian (Trotter decomposition, and the automatic choice between the exact decomposition and the Trotter approximation) are provided by `hamiltonian_to_trotter_circuit` and `hamiltonian_to_evolution_circuit` on the [Hamiltonian](7_hamiltonian.md) page. Error codes, handle and string ownership conventions follow the [Overview](../0_overview.md).

---

## circuit_pauli_evolution(circuit, pauli, angle, qubits, qubits_len)

Appends the evolution operator `e^(-i·θ/2·P)` to the end of the circuit, where `θ` is `angle`.

```c
int32_t circuit_pauli_evolution(struct CCircuit *circuit,
                                const struct CPauliString *pauli,
                                double angle,
                                const uint32_t *qubits,
                                uintptr_t qubits_len);
```

Generated gate sequence:

1. **Phase check**: the Pauli string phase must be ±1 (Hermitian); a phase of ±i cannot generate a unitary evolution and returns `-3`.
2. **Phase absorption**: the ±1 phase is absorbed into the rotation angle, giving the effective angle `θ_eff = θ · phase`.
3. **Basis change**: each non-identity qubit receives gates that rotate its component into the Z basis — `H` for `X`, `S†` then `H` for `Y`; `Z` needs no change.
4. **CNOT ladder**: `CX` gates are applied in increasing order of the non-identity qubit indices, accumulating parity level by level.
5. **Core rotation**: `RZ(θ_eff)` is applied on the last non-identity qubit.
6. **Reverse CNOT ladder and inverse basis changes** restore the qubits to their original basis.

When the Pauli string consists entirely of identity operators, the evolution reduces to the global phase `e^(-i·θ/2)` and produces no gates; the phase is recorded on the circuit's global phase.

Parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| `circuit` | `CCircuit*` | Target circuit handle; the evolution gates are appended to its end. |
| `pauli` | `const CPauliString*` | Pauli string `P` to exponentiate; construction is covered in [Pauli](6_pauli.md). |
| `angle` | `double` | Rotation angle `θ` (fixed floating-point value). |
| `qubits` | `const uint32_t*` | Array of circuit positions, ordered like the Pauli string positions. |
| `qubits_len` | `uintptr_t` | Number of positions; must equal `pauli_string_num_qubits(pauli)`. |

Return value (error codes):

| Value | Scenario |
| --- | --- |
| `0` | Success. |
| `-1` | `circuit` or `pauli` is NULL; or `qubits_len > 0` while `qubits` is NULL. |
| `-2` | A qubit index in `qubits` is out of bounds. |
| `-3` | The position count does not match the Pauli string qubit count, or the Pauli string phase is ±i. |
| `-8` | `angle` is not finite (NaN or infinity). |

---

## Example

```c
#include <assert.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Append e^(-i * pi/4 * XZ): basis changes, a CNOT ladder,
       one RZ, and the inverse ladder. */
    CCircuit* circuit = circuit_new(2);
    CPauliString* pauli = pauli_string_parse("XZ");
    assert(pauli != NULL);

    const uint32_t qubits[2] = {0, 1};
    assert(circuit_pauli_evolution(circuit, pauli, 1.5707963267948966, qubits, 2) == 0);
    assert(circuit_num_operations(circuit) > 0);

    /* NULL arguments are rejected. */
    assert(circuit_pauli_evolution(NULL, pauli, 1.0, qubits, 2) == -1);

    /* Out-of-bounds qubit index. */
    const uint32_t bad[2] = {0, 5};
    assert(circuit_pauli_evolution(circuit, pauli, 1.0, bad, 2) == -2);

    /* Position-count mismatch: a 3-qubit Pauli string on 2 positions. */
    CPauliString* big = pauli_string_parse("XXX");
    assert(circuit_pauli_evolution(circuit, big, 1.0, qubits, 2) == -3);

    pauli_string_free(big);
    pauli_string_free(pauli);
    circuit_free(circuit);
    return 0;
}
```

---

## Related pages

- [QIS Overview](0_overview.md): module overview and shared conventions.
- [Pauli](6_pauli.md): Pauli string parsing from text and phase values.
- [Hamiltonian](7_hamiltonian.md): Trotter and exact evolution entries for a full Hamiltonian.
