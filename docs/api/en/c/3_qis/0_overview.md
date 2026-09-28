# QIS (C)

The C binding's quantum information interface provides local quantum simulation through opaque handles (`CStatevector`, `CDensityMatrix`, `CDensityMatrixNoise`, `CStabilizerState`, and others) plus free functions: state construction and evolution, gate operations, probabilities and measurement, expectation values, and noise simulation. Error codes and the conventions for freeing handles and strings follow the [Overview](../0_overview.md).

---

## The three dense simulators

| Simulator | State representation | Data size | Noise support | Readout noise | Typical use |
| --- | --- | --- | --- | --- | --- |
| [Statevector](1_statevector.md) | Pure state | `2^N` complex amplitudes | None | None | Ideal universal circuits, minimal memory |
| [DensityMatrix](2_density_matrix.md) | Mixed state | `4^N` matrix elements | Kraus-operator channels | None | Mixed states, quantum channels, partial trace |
| [DensityMatrixNoise](3_density_matrix_noise.md) | Mixed state | `4^N` matrix elements | Full noise model (gate noise applied automatically) | Yes | Device-level noisy simulation |

The three simulators share the same set of gate functions, named `<simulator>_apply_<gate>`:

```c
statevector_apply_h(ptr, qubit);
density_matrix_apply_h(ptr, qubit);
density_matrix_noise_apply_h(ptr, qubit);
```

The gate set covers: non-parameterized single-qubit gates H, X, Y, Z, S, S†, T, T†, X2P, X2M, Y2P, Y2M; parameterized single-qubit gates Rx, Ry, Rz, P, U, XY, XY2P, XY2M, RXY, GPhase; two-qubit gates CX, CY, CZ, SWAP, RXX, RYY, RZZ, RZX, CrX, CrY, CrZ, FSIM; and the three-qubit gate CCX. Each simulator also provides a name-dispatched `apply_standard_gate` family and the whole-circuit entry point `apply_circuit`; Statevector and DensityMatrixNoise additionally provide the arbitrary-unitary entry point `apply_unitary_gate`.

All three simulators interface with circuit handles: `*_from_circuit` executes a circuit and returns the evolved state; `*_apply_circuit` applies a circuit in place to an existing state, and the circuit's qubit count must match the state. See [Circuit](../0_circuit/1_circuit.md) for the circuit handle.

---

## Shared conventions

- **Handle lifetime**: constructors such as `*_new`, `*_from_circuit`, `*_from_state`, and `*_maximally_mixed` return heap-allocated handles released with the matching `*_free`; every `*_free` accepts NULL; constructors return `NULL` on failure.
- **Error codes**: functions returning `int32_t` use `0` for success and negative values for errors; the measurement entry points `*_measure` use `0`/`1` for the outcomes `|0>`/`|1>`, with negative values still indicating errors. Typical QIS error codes: `-1` (NULL pointer), `-2` (qubit index out of bounds), `-3` (circuit error), `-7` (unsupported operation), `-8` (invalid parameter: non-finite angle, buffer-length mismatch, non-normalized input state, qubit-count mismatch, unknown gate name).
- **Two-step array output**: for probabilities and matrix data, first call the `*_len` function for the element count, allocate a buffer, then call the fill function (e.g. `statevector_probabilities_len` + `statevector_probabilities`, `density_matrix_data_len` + `density_matrix_data`); when `len` does not match the actual count the fill function returns `-8`.
- **Strings**: `*_measure_all` returns a big-endian bitstring (MSB = qubit `N-1`, LSB = qubit `0`); release it with `cqlib_string_free`.
- **Complex64**: `{ double re; double im; }`; amplitudes and matrix elements are stored row-major with real and imaginary parts interleaved.
- **Sampling results**: `*_sample_shots` returns a `COutcomeList*` whose entries are big-endian bitstring `char*` values; use `outcome_list_len` for the count, `outcome_list_get` to fetch an entry (released with `cqlib_string_free`), and `outcome_list_free` to free the list — see [ClassicalState](5_classical_state.md).
- **Observables**: the expectation-value functions accept a `CHamiltonian*` handle; see [Hamiltonian](7_hamiltonian.md) for construction and [Pauli](6_pauli.md) for the Pauli-string components.

---

## Pages

| Page | Contents |
| --- | --- |
| [Statevector](1_statevector.md) | Pure-state simulator: amplitude access, gates, probabilities, measurement, sampling, and expectation values. |
| [DensityMatrix](2_density_matrix.md) | Mixed-state simulator: matrix data, physicality checks, Kraus channels, partial trace, and expectation values. |
| [DensityMatrixNoise](3_density_matrix_noise.md) | Noisy simulator: automatic gate noise and readout-error probability distributions. |
| [StabilizerState](4_stabilizer.md) | Stabilizer-state simulator for large Clifford circuits. |
| [ClassicalState](5_classical_state.md) | Runtime classical data and `COutcomeList`. |
| [Pauli](6_pauli.md) | Pauli operators, Pauli strings, and their matrix representation. |
| [Hamiltonian](7_hamiltonian.md) | Hamiltonians and observables. |
| [Evolution](8_evolution.md) | The Pauli-string evolution gate `circuit_pauli_evolution`. |
| [Metrics / Entropy](9_metrics_entropy.md) | Fidelity, purity, trace distance, entropy, and entanglement measures. |

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Run the same Bell circuit on both simulators */
    struct CStatevector *sv = statevector_new(2);
    struct CDensityMatrix *dm = density_matrix_new(2);

    statevector_apply_h(sv, 0);
    statevector_apply_cx(sv, 0, 1);
    density_matrix_apply_h(dm, 0);
    density_matrix_apply_cx(dm, 0, 1);

    /* Pure and mixed representations agree: P(00) = P(11) = 0.5 */
    double p_sv[4], p_dm[4];
    statevector_probabilities(sv, p_sv, 4);
    density_matrix_probabilities(dm, p_dm, 4);
    printf("P(00): sv=%.3f dm=%.3f\n", p_sv[0], p_dm[0]);
    printf("P(11): sv=%.3f dm=%.3f\n", p_sv[3], p_dm[3]);

    statevector_free(sv);
    density_matrix_free(dm);
    return 0;
}
```

For constructing and populating the `CNoiseModel` handle, see [NoiseModel](../2_device/4_noise.md).
