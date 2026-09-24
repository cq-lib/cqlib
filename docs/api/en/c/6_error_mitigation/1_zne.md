# Zero-Noise Extrapolation (C)

Zero-noise extrapolation (ZNE) folds the circuit at several noise factors (`2 * fold level + 1`), estimates the expectation value for each folded circuit, and extrapolates back to the zero-noise limit. This page covers creating the `CZneMitigation` handle, generating folded circuits, reading the configuration, extrapolation and the mitigation sequence, plus traversal of the folded-circuit list `CCircuitList`. For the callback contract and module constants see the [module overview](0_overview.md); for error codes and handle ownership conventions see the [Overview](../0_overview.md).

---

## Handle Lifecycle

### zne_mitigation_new(circuit, fold_levels, num_levels)

Creates a ZNE mitigation handle from a base circuit and a fold-level array. Noise factors are derived from the fold levels as `2 * level + 1`.

- `circuit` (`const struct CCircuit *`): the base circuit; all folding operates on copies of it.
- `fold_levels` (`const int32_t *`): the fold-level array, e.g. `{0, 1, 2}`.
- `num_levels` (`uintptr_t`): the number of elements in `fold_levels`.

Returns `struct CZneMitigation *`: the new handle, freed with `zne_mitigation_free`; NULL when `circuit` is NULL, `num_levels` is 0, or `fold_levels` is NULL.

### zne_mitigation_free(ptr)

Frees a ZNE mitigation handle.

- `ptr` (`struct CZneMitigation *`): the handle to free; NULL is allowed.

---

## Folding and the Circuit List

### zne_mitigation_fold_circuit(ptr, gate_names)

Folds the circuit at every configured level and returns the folded-circuit list (one circuit per fold level).

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `gate_names` (`const char *`): optional comma-separated gate-name list (e.g. `"H,CX"`) that folds only the named gates; NULL folds the whole circuit globally.

Returns `struct CCircuitList *`: the folded-circuit list, freed with `circuit_list_free`; NULL when `ptr` is NULL, `gate_names` contains an unknown gate name or invalid UTF-8, or folding fails.

### circuit_list_len(ptr)

Returns the number of circuits in the list.

- `ptr` (`const struct CCircuitList *`): the folded-circuit list.

Returns `uintptr_t`: the circuit count; 0 when `ptr` is NULL.

### circuit_list_get(ptr, index)

Returns the circuit at `index` as an independent clone.

- `ptr` (`const struct CCircuitList *`): the folded-circuit list.
- `index` (`uintptr_t`): the circuit index.

Returns `struct CCircuit *`: an owned clone of the circuit, freed with `circuit_free`; NULL when `ptr` is NULL or `index` is out of bounds.

### circuit_list_free(ptr)

Frees a folded-circuit list. Circuit clones obtained via `circuit_list_get` are not freed with the list — release each one with `circuit_free`.

- `ptr` (`struct CCircuitList *`): the list to free; NULL is allowed.

---

## Configuration Access

### zne_circuit(ptr)

Returns the original (unfolded) circuit as an independent clone.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.

Returns `struct CCircuit *`: the circuit clone, freed with `circuit_free`; NULL when `ptr` is NULL.

### zne_mitigation_noise_factor(ptr, index)

Returns the noise factor for fold level `index` (`2 * level + 1`).

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `index` (`uintptr_t`): the fold-level index.

Returns `int32_t`: the noise factor; 0 when `ptr` is NULL or `index` is out of bounds.

### zne_fold_levels_len(ptr) / zne_fold_levels(ptr, buffer, len)

Two-step read of the configured fold-level array: `zne_fold_levels_len` returns the element count, `zne_fold_levels` copies the array into the caller-provided buffer.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `buffer` (`int32_t *`): buffer receiving the fold levels, sized at least `zne_fold_levels_len(ptr)`.
- `len` (`uintptr_t`): the buffer length; must equal `zne_fold_levels_len(ptr)`.

`zne_fold_levels_len` returns `uintptr_t`: the element count, 0 when `ptr` is NULL. `zne_fold_levels` returns `int32_t`: 0 on success; -1 (`ptr` or `buffer` NULL); -8 (`len` does not match the element count).

### zne_noise_factors_len(ptr) / zne_noise_factors(ptr, buffer, len)

Two-step read of the noise-factor array (`2 * level + 1` per fold level): `zne_noise_factors_len` returns the element count, `zne_noise_factors` copies the array into the caller-provided buffer.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `buffer` (`int32_t *`): buffer receiving the noise factors, sized at least `zne_noise_factors_len(ptr)`.
- `len` (`uintptr_t`): the buffer length; must equal `zne_noise_factors_len(ptr)`.

`zne_noise_factors_len` returns `uintptr_t`: the element count, 0 when `ptr` is NULL. `zne_noise_factors` returns `int32_t`: 0 on success; -1 (`ptr` or `buffer` NULL); -8 (`len` does not match the element count).

---

## Two-Step Folded-Circuit Output

### zne_fold_circuits_len(ptr, gate_names)

Returns the number of circuits produced by the two-step folded-circuit output (one per fold level).

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `gate_names` (`const char *`): optional comma-separated gate-name list, with the same semantics as in `zne_mitigation_fold_circuit`.

Returns `uintptr_t`: the circuit count; 0 when `ptr` is NULL or `gate_names` contains an unknown gate name / invalid UTF-8.

### zne_fold_circuits(ptr, gate_names, buffer, len)

Folds the circuit at every configured level and writes each folded circuit as an owned handle into the caller-provided buffer.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `gate_names` (`const char *`): optional comma-separated gate-name list; NULL folds globally.
- `buffer` (`struct CCircuit **`): buffer receiving the circuit handles, sized `zne_fold_circuits_len(ptr, gate_names)`; every element is an owned `CCircuit *` that must be freed with `circuit_free`.
- `len` (`uintptr_t`): the buffer length; must equal `zne_fold_circuits_len(ptr, gate_names)`.

Returns `int32_t`: 0 on success; -1 (`ptr` or `buffer` NULL); -4 (`gate_names` contains an unknown gate name or invalid UTF-8); -8 (`len` does not match the fold-level count); -3 (folding failure).

---

## Extrapolation

The `method` parameter of `zne_extrapolate` takes the following tags:

| Constant | Value | Method |
| --- | --- | --- |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | Polynomial fit. |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | Exponential-decay fit. |

### zne_extrapolate(ptr, noisy_results, len, method, degree, out_value)

Extrapolates the zero-noise expectation value from the noisy expectation values measured at each noise factor.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `noisy_results` (`const double *`): one expectation value per noise factor, in fold-level order.
- `len` (`uintptr_t`): the number of expectation values.
- `method` (`uint8_t`): the extrapolation method tag, see the table above. The polynomial method uses `degree`; the exponential method requires all values to be positive.
- `degree` (`uintptr_t`): the polynomial degree; must be smaller than the number of data points.
- `out_value` (`double *`): receives the extrapolated value.

Returns `int32_t`: 0 on success; -1 (`ptr`, `noisy_results` or `out_value` NULL); -8 (invalid `method` tag, empty results, `len` not matching the noise-factor count, invalid polynomial degree, or non-positive values in an exponential fit); -7 (other fitting errors).

### zne_poly_extrapolate(ptr, noisy_results, len, degree, out_value)

Extrapolates the zero-noise expectation value with a polynomial fit of the given degree; equivalent to the polynomial branch of `zne_extrapolate`.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `noisy_results` (`const double *`): one expectation value per noise factor, in fold-level order.
- `len` (`uintptr_t`): the number of expectation values.
- `degree` (`uintptr_t`): the polynomial degree; must be smaller than the noise-factor count.
- `out_value` (`double *`): receives the extrapolated value.

Returns `int32_t`: 0 on success; -1 (`ptr`, `noisy_results` or `out_value` NULL); -8 (empty results, `len` not matching the noise-factor count, or invalid degree); -7 (other fitting errors).

### zne_exp_extrapolate(ptr, noisy_results, len, out_value)

Extrapolates the zero-noise expectation value with an exponential-decay model `y(x) = A * exp(-x / τ)`, writing the fitted value `A` at `x = 0`. All input values must be positive.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `noisy_results` (`const double *`): one expectation value per noise factor, in fold-level order.
- `len` (`uintptr_t`): the number of expectation values.
- `out_value` (`double *`): receives the fitted value `A`.

Returns `int32_t`: 0 on success; -1 (`ptr`, `noisy_results` or `out_value` NULL); -8 (empty results, `len` not matching the noise-factor count, or any non-positive value); -7 (other fitting errors).

---

## Mitigation Sequence

### zne_run_em_sequence_len(ptr)

Returns the number of expectation values produced by the mitigation sequence (one per fold level).

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.

Returns `uintptr_t`: the expectation-value count; 0 when `ptr` is NULL.

### zne_run_em_sequence_with_shots_len(ptr)

Returns the number of expectation values produced by the mitigation sequence with explicit shots (one per fold level); the same as `zne_run_em_sequence_len`.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.

Returns `uintptr_t`: the expectation-value count; 0 when `ptr` is NULL.

### zne_run_em_sequence(ptr, gate_names, hamiltonian, estimator, buffer, len)

Runs the mitigation sequence: folds the circuit at every configured level and estimates one expectation value per folded circuit through the `estimator` callback. The callback receives shot count 0 ("not specified").

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `gate_names` (`const char *`): optional comma-separated gate-name list; NULL folds globally.
- `hamiltonian` (`const struct CHamiltonian *`): the observable to estimate, see [Hamiltonian](../3_qis/7_hamiltonian.md) for construction.
- `estimator` (`CEstimatorFn`): the estimator callback; see the [module overview](0_overview.md) for the contract.
- `buffer` (`double *`): buffer receiving the expectation values, sized `zne_run_em_sequence_len(ptr)`.
- `len` (`uintptr_t`): the buffer length; must equal `zne_run_em_sequence_len(ptr)`.

Returns `int32_t`: 0 on success; -1 (`ptr`, `hamiltonian` or `buffer` NULL); -4 (`gate_names` contains an unknown gate name or invalid UTF-8); -8 (`len` does not match the noise-factor count); -3 (folding failure); -7 (other execution errors).

### zne_run_em_sequence_with_shots(ptr, gate_names, hamiltonian, shots, estimator, buffer, len)

Same as `zne_run_em_sequence`, but forwards `shots` to the estimator callback as the suggested shot count.

- `ptr` (`const struct CZneMitigation *`): the ZNE mitigation handle.
- `gate_names` (`const char *`): optional comma-separated gate-name list; NULL folds globally.
- `hamiltonian` (`const struct CHamiltonian *`): the observable to estimate.
- `shots` (`uintptr_t`): the shot count forwarded to the estimator.
- `estimator` (`CEstimatorFn`): the estimator callback.
- `buffer` (`double *`): buffer receiving the expectation values, sized `zne_run_em_sequence_with_shots_len(ptr)`.
- `len` (`uintptr_t`): the buffer length; must equal `zne_run_em_sequence_with_shots_len(ptr)`.

Returns `int32_t`: the same error codes as `zne_run_em_sequence`.

---

## Example

A complete fold → per-circuit estimation → polynomial extrapolation flow (the estimator simulates exactly with the statevector, see [Statevector](../3_qis/1_statevector.md)):

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

/* Estimator callback: exactly simulates the folded circuit and computes <ZZ> */
static void estimate(const struct CCircuit *circuit,
                     const struct CHamiltonian *hamiltonian,
                     uintptr_t shots,
                     double *expectation,
                     double *variance) {
    struct CStatevector *sv = statevector_from_circuit(circuit);
    double value = 0.0;
    statevector_expectation(sv, hamiltonian, &value);
    statevector_free(sv);
    *expectation = value;
    *variance = 0.0;
    (void)shots;
}

int main(void) {
    /* Bell circuit: H(0) + CX(0, 1) */
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* Fold levels {0, 1, 2} → noise factors {1, 3, 5} */
    int32_t levels[3] = {0, 1, 2};
    struct CZneMitigation *zne = zne_mitigation_new(qc, levels, 3);

    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *obs = hamiltonian_from_pauli(zz);  /* takes ownership of zz */

    /* Fold + estimate per circuit → one expectation value per noise factor */
    uintptr_t len = zne_run_em_sequence_len(zne);
    double noisy_results[3];
    zne_run_em_sequence(zne, NULL, obs, estimate, noisy_results, len);

    /* Polynomial extrapolation back to the zero-noise limit */
    double value = 0.0;
    zne_extrapolate(zne, noisy_results, len, ZNE_EXTRAPOLATE_POLYNOMIAL, 2, &value);
    printf("ZNE estimate: %f\n", value);

    zne_mitigation_free(zne);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

Fine-grained path: fold into a `CCircuitList` and traverse it, or fetch the owned circuit array directly with the two-step interface:

```c
/* Fold into a list, then read one by one (each get returns an owned clone) */
struct CCircuitList *folded = zne_mitigation_fold_circuit(zne, "H,CX");
for (uintptr_t i = 0; i < circuit_list_len(folded); i++) {
    struct CCircuit *c = circuit_list_get(folded, i);
    int32_t factor = zne_mitigation_noise_factor(zne, i);  /* noise factor of level i */
    /* estimate the expectation value on circuit c here */
    circuit_free(c);
}
circuit_list_free(folded);

/* Two-step: the buffer receives owned circuit handles, free each one */
uintptr_t m = zne_fold_circuits_len(zne, NULL);
struct CCircuit **buf = malloc(m * sizeof(struct CCircuit *));
zne_fold_circuits(zne, NULL, buf, m);
for (uintptr_t i = 0; i < m; i++) {
    circuit_free(buf[i]);
}
free(buf);
```
