# Hamiltonian (C)

`CHamiltonian` is an opaque handle to an observable (Hamiltonian): a sum of Pauli-string terms with complex coefficients, supporting simplification, scaling, dense matrix output, and time-evolution circuit construction. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

---

## Constants

Trotter-Suzuki decomposition mode tags (the `mode` parameter of `hamiltonian_to_trotter_circuit` and `hamiltonian_to_evolution_circuit`):

| Constant | Value | Mode |
| --- | --- | --- |
| `TROTTER_MODE_FIRST_ORDER` | 0 | First-order Lie-Trotter. |
| `TROTTER_MODE_SECOND_ORDER` | 1 | Second-order Strang splitting. |

---

## Construction and Release

### hamiltonian_new(num_qubits)

Creates an empty Hamiltonian (no Pauli terms) acting on `num_qubits` qubits.

Parameters:

- `num_qubits` (`uintptr_t`): number of qubits.

Returns: a new `CHamiltonian*`.

### hamiltonian_free(ptr)

Frees a Hamiltonian handle; NULL is allowed.

### hamiltonian_from_pauli(pauli)

Constructs a Hamiltonian from a single Pauli string (coefficient 1.0).

**Ownership**: this function takes ownership of the input `CPauliString*` and frees it; after the call, do not use or free the original handle again.

Parameters:

- `pauli` (`CPauliString*`): Pauli string handle whose ownership is taken.

Returns: a new `CHamiltonian*` on success; NULL on NULL input.

### hamiltonian_add_term(ptr, pauli, coeff_re, coeff_im)

Adds a Pauli term with a complex coefficient.

**Ownership**: the Pauli string is cloned; the input handle is not consumed and remains owned by the caller.

Parameters:

- `ptr` (`CHamiltonian*`): Hamiltonian handle.
- `pauli` (`const CPauliString*`): Pauli string handle (cloned).
- `coeff_re` (`double`): real part of the coefficient.
- `coeff_im` (`double`): imaginary part of the coefficient.

Returns: 0 on success; -1 for NULL pointers; -8 when the Pauli string width differs from the Hamiltonian.

### hamiltonian_simplify(ptr)

Simplifies in place: combines terms with the same Pauli string and removes near-zero coefficients.

Returns: 0 on success; -1 for NULL pointers.

### hamiltonian_scale(ptr, factor_re, factor_im)

Scales all coefficients by a complex factor in place.

Parameters:

- `factor_re` (`double`): real part of the factor.
- `factor_im` (`double`): imaginary part of the factor.

Returns: 0 on success; -1 for NULL pointers; -8 for a non-finite factor (NaN or infinity).

---

## Queries

### hamiltonian_num_qubits(ptr)

Returns the number of qubits; 0 for NULL.

### hamiltonian_num_terms(ptr)

Returns the number of Pauli terms; 0 for NULL.

### hamiltonian_all_terms_commute(ptr)

Tests whether all Pauli terms mutually commute. When they do, the time evolution decomposes exactly into per-term Pauli rotations.

Returns: 1 when all terms commute; 0 otherwise; -1 for NULL.

---

## Dense Matrix Output

### hamiltonian_to_matrix_len(ptr) / hamiltonian_to_matrix(ptr, buffer, len)

Two-step read of the dense matrix: `*_len` returns the side length `2^N` (0 for NULL), and the fill function copies the `2^N × 2^N` matrix into `buffer` in row-major order (`Complex64` values). `len` must equal the square of the side length.

Returns (fill function): 0 on success; -1 for NULL pointers; -8 when `len` does not match the matrix element count.

---

## Expectation Values and Variance

### CProbMeasurement

Descriptor of one measured basis and its observed outcome distribution, used by `hamiltonian_expectation_probs`:

```c
typedef struct CProbMeasurement {
  const struct CPauliString *basis;  /* borrowed, not consumed */
  const char *const *states;         /* len computational-basis bitstrings */
  const double *probs;               /* len probabilities, paired with states */
  uintptr_t len;                     /* number of entries in states and probs */
} CProbMeasurement;
```

- `basis` (`const CPauliString*`): measurement basis as a Pauli string; borrowed, not consumed, and freed by the caller.
- `states` (`const char* const*`): array of `len` computational-basis bitstrings, one `'0'`/`'1'` character per qubit, big-endian (qubit 0 is the last character).
- `probs` (`const double*`): array of `len` observed probabilities, paired with `states` by index.
- `len` (`uintptr_t`): number of entries in `states` and `probs`.

### hamiltonian_expectation_probs(ptr, measurements, measurements_len, out)

Computes the expectation value of the Hamiltonian from measurement outcome probabilities. `measurements` lists one entry per measured basis; each term of the Hamiltonian is evaluated in the first compatible measured basis — a basis is compatible with a term when it agrees with the term on every non-identity factor. The result is written to `out`. The distributions only need to cover the observed outcomes; unlisted outcomes contribute zero.

```c
int32_t hamiltonian_expectation_probs(const struct CHamiltonian *ptr,
                                      const struct CProbMeasurement *measurements,
                                      uintptr_t measurements_len,
                                      double *out);
```

Parameters:

- `ptr` (`const CHamiltonian*`): Hamiltonian handle.
- `measurements` (`const CProbMeasurement*`): array of `measurements_len` measurement descriptors.
- `measurements_len` (`uintptr_t`): number of entries in `measurements`; `0` supplies no measured basis.
- `out` (`double*`): receives the expectation value.

Error codes:

| Value | Scenario |
| --- | --- |
| `0` | Success. |
| `-1` | `ptr` or `out` is NULL; `measurements` is NULL with a non-zero count; or an entry has a NULL `basis`, or NULL `states`/`probs`/bitstring with a non-zero `len`. |
| `-4` | A bitstring in `states` is not valid UTF-8. |
| `-7` | A term has no compatible measurement basis, a bitstring is malformed or its length differs from the Hamiltonian's qubit count, or another core failure. |

### hamiltonian_variance_statevector(ptr, sv, out)

Computes the variance of the Hamiltonian for a statevector, `Var(H) = <H^2> - <H>^2`, and writes it to `out`.

```c
int32_t hamiltonian_variance_statevector(const struct CHamiltonian *ptr,
                                         const struct CStatevector *sv,
                                         double *out);
```

Parameters:

- `ptr` (`const CHamiltonian*`): Hamiltonian handle.
- `sv` (`const CStatevector*`): input statevector; its qubit count must match the Hamiltonian's.
- `out` (`double*`): receives the variance.

Error codes:

| Value | Scenario |
| --- | --- |
| `0` | Success. |
| `-1` | `ptr`, `sv`, or `out` is NULL. |
| `-7` | The Hamiltonian has non-Hermitian terms, or another simulation error. |
| `-8` | The statevector's qubit count does not match the Hamiltonian, or another invalid parameter. |

---

## Time-Evolution Circuits

### hamiltonian_to_trotter_circuit(ptr, time, steps, mode)

Converts the Hamiltonian to a Trotterized time-evolution circuit approximating `U(t) = exp(-i H t)`.

Parameters:

- `ptr` (`const CHamiltonian*`): Hamiltonian handle.
- `time` (`double`): evolution time; must be finite.
- `steps` (`uintptr_t`): number of Trotter steps.
- `mode` (`uint32_t`): `TROTTER_MODE_FIRST_ORDER` or `TROTTER_MODE_SECOND_ORDER`.

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input, a non-finite time, an invalid mode, or construction failure.

### hamiltonian_to_evolution_circuit(ptr, time, steps, mode)

Converts the Hamiltonian to a time-evolution circuit: uses the exact single-pass decomposition when all terms commute, and falls back to the Trotter approximation (`mode`, `steps`) otherwise.

Parameters:

- `ptr` (`const CHamiltonian*`): Hamiltonian handle.
- `time` (`double`): evolution time; must be finite.
- `steps` (`uintptr_t`): number of Trotter steps.
- `mode` (`uint32_t`): `TROTTER_MODE_FIRST_ORDER` or `TROTTER_MODE_SECOND_ORDER`.

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input, a non-finite time, an invalid mode, or construction failure.

---

## Example

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build H = 0.5 * ZI + 0.5 * IZ ("ZI": qubit1=Z, qubit0=I) */
    struct CHamiltonian *ham = hamiltonian_new(2);
    struct CPauliString *zi = pauli_string_parse("ZI");
    struct CPauliString *iz = pauli_string_parse("IZ");
    hamiltonian_add_term(ham, zi, 0.5, 0.0);  /* cloned; handle stays valid */
    hamiltonian_add_term(ham, iz, 0.5, 0.0);
    pauli_string_free(zi);
    pauli_string_free(iz);

    /* 2. Queries and simplification */
    uintptr_t nq = hamiltonian_num_qubits(ham);       /* 2 */
    uintptr_t nt = hamiltonian_num_terms(ham);        /* 2 */
    int32_t commute = hamiltonian_all_terms_commute(ham);  /* 1 (single-qubit terms commute) */
    hamiltonian_simplify(ham);

    /* 3. Two-step read of the 4x4 dense matrix */
    uintptr_t dim = hamiltonian_to_matrix_len(ham);   /* 4 */
    Complex64 *matrix = malloc(dim * dim * sizeof(Complex64));
    hamiltonian_to_matrix(ham, matrix, dim * dim);
    free(matrix);

    /* 4. Time-evolution circuit (exact when terms commute) */
    struct CCircuit *evo =
        hamiltonian_to_evolution_circuit(ham, 0.1, 1, TROTTER_MODE_FIRST_ORDER);
    if (evo != NULL) {
        circuit_free(evo);
    }

    /* 5. Construct from a single Pauli string (ownership taken) */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *from_pauli = hamiltonian_from_pauli(zz);  /* zz is consumed */

    /* 6. Expectation value from measured outcome probabilities, and the
       statevector variance (H = Z⊗Z on the Bell state, P(00) = P(11) = 0.5) */
    struct CStatevector *bell = statevector_new(2);
    statevector_apply_h(bell, 0);
    statevector_apply_cx(bell, 0, 1);

    struct CPauliString *basis = pauli_string_parse("ZZ");
    const char *states[4] = {"00", "01", "10", "11"};
    double probs[4] = {0.5, 0.0, 0.0, 0.5};
    struct CProbMeasurement measurement;
    measurement.basis = basis;      /* borrowed, not consumed */
    measurement.states = states;
    measurement.probs = probs;
    measurement.len = 4;

    double expectation = 0.0;
    hamiltonian_expectation_probs(from_pauli, &measurement, 1, &expectation);  /* 1.0 */

    double variance = 0.0;
    hamiltonian_variance_statevector(from_pauli, bell, &variance);  /* 0.0 (eigenstate of Z⊗Z) */

    pauli_string_free(basis);
    statevector_free(bell);

    hamiltonian_free(from_pauli);
    hamiltonian_free(ham);
    return 0;
}
```

Hamiltonians serve as observables in the error-mitigation workflows (see [Zero-Noise Extrapolation](../6_error_mitigation/1_zne.md) and [Virtual Distillation](../6_error_mitigation/2_virtual_distillation.md)); their expectation values can be computed directly with the statevector simulator (see [Statevector](1_statevector.md)).
