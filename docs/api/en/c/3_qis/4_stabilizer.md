# Stabilizer (C)

`CStabilizerState` is an opaque handle to the stabilizer-state simulator: it describes a quantum state with a set of mutually commuting Pauli operators (stabilizer generators), can only represent stabilizer states reachable by Clifford circuits, and scales to far more qubits than dense representations; arbitrary-angle rotations, `T` gates, and other non-Clifford operations return errors. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Construction and Release

### stabilizer_new(num_qubits)

Constructs the `|0...0>` stabilizer state with per-qubit destabilizer `X_i` and stabilizer `Z_i` generators, all with phase `+1`.

Parameters:

- `num_qubits` (`uintptr_t`): number of qubits.

Returns: a new `CStabilizerState*` on success; NULL when `num_qubits` is 0.

### stabilizer_free(ptr)

Frees a stabilizer state handle; NULL is allowed.

### stabilizer_from_circuit(circuit)

Executes a Clifford circuit and returns the final stabilizer state. Terminal measurement declarations are treated as output declarations and ignored: the state does not collapse; barriers and delays have no side effect.

Parameters:

- `circuit` (`const CCircuit*`): Clifford circuit, may contain `I`, `H`, `X`, `Y`, `Z`, `S`, `SDG`, `X2P`, `X2M`, `Y2P`, `Y2M`, `CX`, `CY`, `CZ`, `SWAP`, and `Reset`.

Returns: a new `CStabilizerState*` on success; NULL on NULL input, a non-Clifford gate, or execution failure.

### stabilizer_run_circuit(circuit)

Executes a Clifford circuit from `|0...0>` and returns the final state. Unlike `stabilizer_from_circuit`, this entry point executes `MeasureBit` / `MeasureBits` operations, so the returned state is already collapsed.

Parameters:

- `circuit` (`const CCircuit*`): Clifford circuit.

Returns: a collapsed `CStabilizerState*` on success; NULL on NULL input or execution failure.

---

## Applying Circuits

### stabilizer_apply_circuit(ptr, circuit)

Applies a Clifford circuit to the state in place.

Parameters:

- `ptr` (`CStabilizerState*`): stabilizer state handle.
- `circuit` (`const CCircuit*`): Clifford circuit whose qubit count must match the state.

Returns: 0 on success; -1 for NULL pointers; -3 when the circuit contains gates that cannot execute or unresolved symbolic parameters; -7 for unsupported operations such as control flow; -8 when the circuit width differs from the state.

---

## Single-Qubit Clifford Gates

The following entry points apply the corresponding gate to `qubit` in place; the gate-name column gives the semantics of each one:

| Gate | Function | Description |
| --- | --- | --- |
| `H` | `stabilizer_apply_h(ptr, qubit)` | Hadamard gate. |
| `X` | `stabilizer_apply_x(ptr, qubit)` | Pauli-X. |
| `Y` | `stabilizer_apply_y(ptr, qubit)` | Pauli-Y. |
| `Z` | `stabilizer_apply_z(ptr, qubit)` | Pauli-Z. |
| `S` | `stabilizer_apply_s(ptr, qubit)` | S gate. |
| `SDG` | `stabilizer_apply_sdg(ptr, qubit)` | S† gate. |
| `X2P` | `stabilizer_apply_x2p(ptr, qubit)` | √X, i.e. Rx(π/2). |
| `X2M` | `stabilizer_apply_x2m(ptr, qubit)` | √X†, i.e. Rx(-π/2). |
| `Y2P` | `stabilizer_apply_y2p(ptr, qubit)` | √Y, i.e. Ry(π/2). |
| `Y2M` | `stabilizer_apply_y2m(ptr, qubit)` | √Y†, i.e. Ry(-π/2). |

Parameters:

- `ptr` (`CStabilizerState*`): stabilizer state handle.
- `qubit` (`uint32_t`): target qubit index.

Returns: 0 on success; -1 for NULL pointers; -2 for an out-of-bounds qubit; -3 when the gate cannot be executed.

---

## Two-Qubit Clifford Gates

| Gate | Function | Description |
| --- | --- | --- |
| `CX` | `stabilizer_apply_cx(ptr, control, target)` | Controlled-X. |
| `CY` | `stabilizer_apply_cy(ptr, control, target)` | Controlled-Y. |
| `CZ` | `stabilizer_apply_cz(ptr, q0, q1)` | Controlled-Z. |
| `SWAP` | `stabilizer_apply_swap(ptr, q0, q1)` | Swaps two qubits. |

Parameters:

- `ptr` (`CStabilizerState*`): stabilizer state handle.
- `control` / `target` / `q0` / `q1` (`uint32_t`): target qubit indices.

Returns: 0 on success; -1 for NULL pointers; -2 for an out-of-bounds qubit; -8 when both qubits are identical.

### stabilizer_apply_standard_gate(ptr, gate_name, qubits, num_target_qubits, params, num_params)

Applies a Clifford standard gate identified by name (e.g. `"H"`, `"CX"`, `"SWAP"`).

Parameters:

- `ptr` (`CStabilizerState*`): stabilizer state handle.
- `gate_name` (`const char*`): standard gate name.
- `qubits` (`const uint32_t*`): array of `num_target_qubits` target indices.
- `num_target_qubits` (`uintptr_t`): number of target qubits.
- `params` (`const double*`): array of `num_params` gate parameters; NULL is allowed when `num_params` is 0.
- `num_params` (`uintptr_t`): number of gate parameters.

Returns: 0 on success; -1 for NULL pointers; -2 for an out-of-bounds qubit; -8 for an unknown gate name or invalid argument counts; -7 for a non-Clifford gate.

---

## Measurement, Reset, and Sampling

### stabilizer_measure(ptr, qubit)

Measures `qubit` in the Z basis, collapsing the state.

Returns: the measurement outcome `0` or `1`; -1 for NULL pointers; -2 for an out-of-bounds qubit.

### stabilizer_measure_all(ptr)

Measures all qubits in order, collapsing the state.

Returns: a big-endian bitstring (leftmost character is qubit N-1, rightmost is qubit 0); free with `cqlib_string_free`. NULL input returns NULL.

### stabilizer_reset(ptr, qubit)

Resets `qubit` to `|0>` (one Z-basis measurement plus a conditional X correction).

Returns: 0 on success; -1 for NULL pointers; -2 for an out-of-bounds qubit.

### stabilizer_sample_shots(ptr, shots)

Samples `shots` independent measurement outcomes in parallel; the same initial state always produces the same sample set.

Parameters:

- `ptr` (`const CStabilizerState*`): stabilizer state handle.
- `shots` (`uintptr_t`): number of samples.

Returns: a `COutcomeList*` (free with `outcome_list_free`); NULL input returns NULL. See [Classical State](5_classical_state.md) for traversing the bitstrings.

---

## Probability Distribution

### stabilizer_probabilities_len(ptr) / stabilizer_probabilities(ptr, buffer, len)

Two-step read of the probability distribution over all computational basis states: `*_len` returns the length `2^N` (0 for NULL), and the fill function copies the distribution into `buffer`, where index `i` corresponds to the basis state whose binary representation is `i` (qubit 0 is the least-significant bit). Only suitable for small systems.

Returns (fill function): 0 on success; -1 for NULL pointers; -8 when `len` differs from `2^N` or the system exceeds the size limit.

### stabilizer_probability_of(ptr, bits, len, out)

Returns the probability of measuring a specific computational-basis outcome: a bitstring compatible with the stabilizer state returns `1/2^k`, where `k` is the number of qubits with random outcomes; an incompatible one returns 0. The call does not modify the state.

Parameters:

- `ptr` (`const CStabilizerState*`): stabilizer state handle.
- `bits` (`const bool*`): `len` booleans where `bits[0]` is qubit 0.
- `len` (`uintptr_t`): must equal the number of qubits.
- `out` (`double*`): probability output.

Returns: 0 on success; -1 for NULL pointers; -8 on a length mismatch.

---

## Generators and Pauli Expectations

### stabilizer_num_qubits(ptr)

Returns the number of qubits; 0 for NULL.

### stabilizer_get_stabilizers_len(ptr) / stabilizer_get_stabilizer(ptr, index)

Reads the stabilizer generators: `*_len` returns the generator count (always N), and `get_stabilizer` returns the `index`-th generator as a `"+XYZ"`-style Pauli label (bit-order convention in [Pauli](6_pauli.md)); free with `cqlib_string_free`. Out-of-bounds indices return NULL.

### stabilizer_get_destabilizers_len(ptr) / stabilizer_get_destabilizer(ptr, index)

Reads the destabilizer generators: `*_len` returns the generator count (always N), and `get_destabilizer` returns the `index`-th generator as a Pauli label in the same `"+XYZ"` format as `stabilizer_get_stabilizer`; free with `cqlib_string_free`. Out-of-bounds indices return NULL.

### stabilizer_pauli_expectation(ptr, pauli, out)

Computes the Pauli expectation value ⟨P⟩: `1` when `P` belongs to the stabilizer group, `-1` when it belongs to the negated stabilizer group, and `0` otherwise. The test covers products of generators, not just individual generators.

Parameters:

- `ptr` (`const CStabilizerState*`): stabilizer state handle.
- `pauli` (`const CPauliString*`): Pauli string whose qubit count must match the state.
- `out` (`int32_t*`): expectation output (-1, 0, or +1).

Returns: 0 on success; -1 for NULL pointers; -8 when the Pauli string width differs from the state.

### stabilizer_to_stim_format(ptr)

Exports the generator table in Stim-compatible text format (one `"+XYZ"` Pauli label per line).

Returns: a C string; free with `cqlib_string_free`. NULL input returns NULL.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Bell state: H(0) + CX(0, 1) */
    struct CStabilizerState *s = stabilizer_new(2);
    if (s == NULL) {
        return 1;
    }
    stabilizer_apply_h(s, 0);
    stabilizer_apply_cx(s, 0, 1);

    /* 2. Read a generator label ("+XY" style) and a Pauli expectation */
    char *stab = stabilizer_get_stabilizer(s, 0);
    struct CPauliString *zz = pauli_string_parse("ZZ");
    int32_t e_zz = 0;
    stabilizer_pauli_expectation(s, zz, &e_zz);  /* 1: ZZ stabilizes the Bell state */
    cqlib_string_free(stab);
    pauli_string_free(zz);

    /* 3. Probability of a specific basis state: |00> and |11> split evenly */
    bool bits[2] = {false, false};
    double p00 = 0.0;
    stabilizer_probability_of(s, bits, 2, &p00);  /* 0.5 */

    /* 4. Sample 500 shots and traverse the OutcomeList */
    struct COutcomeList *shots = stabilizer_sample_shots(s, 500);
    uintptr_t n = outcome_list_len(shots);  /* 500 */
    for (uintptr_t i = 0; i < n; i++) {
        char *bs = outcome_list_get(shots, i);  /* "00" or "11" */
        cqlib_string_free(bs);
    }
    outcome_list_free(shots);

    stabilizer_free(s);
    return 0;
}
```
