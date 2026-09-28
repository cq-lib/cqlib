# Virtual Distillation (C)

Virtual distillation estimates the ratio `Tr(O ρ^M) / Tr(ρ^M)` using multiple copies of the density matrix and a copy-swap circuit, where `M` is the copy count and `O` is the observable described by a Hamiltonian. This page covers creating and configuring the `CVirtualDistillation` handle, building the copy-swap circuit, running the numerator/denominator circuits separately, and the full protocol. For the callback contract see the [module overview](0_overview.md); for error codes and handle ownership conventions see the [Overview](../0_overview.md).

---

## Handle and Configuration

### virtual_distillation_new(circuit, copies)

Creates a virtual distillation handle from a base circuit and a copy count. All derived circuits are built from the base circuit.

- `circuit` (`const struct CCircuit *`): the base circuit.
- `copies` (`uintptr_t`): the copy count, at least 2.

Returns `struct CVirtualDistillation *`: the new handle, freed with `virtual_distillation_free`; NULL when `circuit` is NULL or `copies` is less than 2.

### virtual_distillation_free(ptr)

Frees a virtual distillation handle.

- `ptr` (`struct CVirtualDistillation *`): the handle to free; NULL is allowed.

### virtual_distillation_copies(ptr)

Returns the configured copy count.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.

Returns `uintptr_t`: the copy count; 0 when `ptr` is NULL.

### virtual_distillation_set_copies(ptr, copies)

Updates the copy count. On failure the original value is kept.

- `ptr` (`struct CVirtualDistillation *`): the virtual distillation handle.
- `copies` (`uintptr_t`): the new copy count, at least 2.

Returns `int32_t`: 0 on success; -1 (`ptr` NULL); -8 (`copies` less than 2).

---

## Copy-Swap Circuit

### virtual_distillation_build_circuit(ptr)

Builds the copy-swap circuit used by the virtual distillation protocol: the base circuit is first decomposed into gates, then prepared `copies` times side by side — copy `i` is shifted onto the qubit range starting at `i × base width`, with no overlap between copies — and finally a `SWAP` is inserted bitwise between the first copy and each of the other copies.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.

The circuit width is `copies × base width` (the base width being the width of the decomposed circuit); the operation count is `copies` copies of the base operations plus `(copies - 1) × base width` `SWAP` gates.

Returns `struct CCircuit *`: an owned handle of the copy-swap circuit, freed with `circuit_free`; NULL when `ptr` is NULL or construction fails.

---

## Numerator and Denominator

The estimator callback distinguishes the two circuits through the `hamiltonian` parameter: the numerator circuit carries the observable, the denominator circuit has none.

### virtual_distillation_run_numerator_circuit(ptr, hamiltonian, shots, estimator, out_mean, out_variance)

Runs the numerator circuit through the estimator callback to estimate `Tr(O ρ^M)`. The estimator receives the copy-swap circuit, a Hamiltonian expanded to the full copy-swap width, and `shots`. Expansion rule: the original Pauli terms keep their qubit indices, phases and coefficients; the new higher-index qubits are padded with `Z`.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.
- `hamiltonian` (`const struct CHamiltonian *`): the observable; its qubit count must match the base circuit width, see [Hamiltonian](../3_qis/7_hamiltonian.md) for construction.
- `shots` (`uintptr_t`): the shot count forwarded to the estimator.
- `estimator` (`CEstimatorFn`): the estimator callback; see the [module overview](0_overview.md) for the contract.
- `out_mean` (`double *`): receives the estimated mean.
- `out_variance` (`double *`): receives the estimated variance.

Returns `int32_t`: 0 on success; -1 (`ptr`, `hamiltonian`, `out_mean` or `out_variance` NULL); -3 (Hamiltonian qubit count not matching the base circuit width, or copy-swap circuit construction failure); -7 (other execution errors).

### virtual_distillation_run_denominator_circuit(ptr, shots, estimator, out_mean, out_variance)

Runs the denominator circuit through the estimator callback to estimate `Tr(ρ^M)`. The estimator receives the copy-swap circuit, a NULL Hamiltonian, and `shots`.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.
- `shots` (`uintptr_t`): the shot count forwarded to the estimator.
- `estimator` (`CEstimatorFn`): the estimator callback.
- `out_mean` (`double *`): receives the estimated mean.
- `out_variance` (`double *`): receives the estimated variance.

Returns `int32_t`: 0 on success; -1 (`ptr`, `out_mean` or `out_variance` NULL); -3 (execution failure).

### virtual_distillation_build_copy_swap_circuit(ptr)

Constructs the copy-swap circuit from the base circuit: the base circuit is first decomposed into gates, then prepared `copies` times side by side — copy `i` is shifted onto the qubit range starting at `i × base width`, with no overlap between copies — and finally a `SWAP` is inserted bitwise between the first copy and each of the other copies. The circuit width is `copies × base width` (the base width being the width of the decomposed circuit); the operation count is `copies` copies of the base operations plus `(copies - 1) × base width` `SWAP` gates.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.

Returns `struct CCircuit *`: an owned handle of the copy-swap circuit, freed with `circuit_free`; NULL when `ptr` is NULL or construction fails.

---

## Full Protocol

### virtual_distillation_run_vd(ptr, hamiltonian, shots_numerator, shots_denominator, estimator, out_mean, out_variance)

Runs the full virtual distillation protocol: it first checks that the Hamiltonian qubit count equals the base circuit width, then runs the numerator circuit and the denominator circuit in turn, and finally combines them into the mitigated result. The numerator and denominator use their own shot counts.

- `ptr` (`const struct CVirtualDistillation *`): the virtual distillation handle.
- `hamiltonian` (`const struct CHamiltonian *`): the observable; its qubit count must match the base circuit width.
- `shots_numerator` (`uintptr_t`): the shot count of the numerator circuit.
- `shots_denominator` (`uintptr_t`): the shot count of the denominator circuit.
- `estimator` (`CEstimatorFn`): the estimator callback.
- `out_mean` (`double *`): receives the mitigated expectation value `mu_num / mu_den`.
- `out_variance` (`double *`): receives the mitigated variance.

Returns `int32_t`: 0 on success; -1 (any pointer NULL); -8 (Hamiltonian qubit count not matching the base circuit width); -7 (zero denominator mean or other execution errors).

The variance is combined with a first-order Taylor approximation, assuming the numerator and denominator are independent: `var_num / mu_den^2 + mu_num^2 * var_den / mu_den^4`. Sampling randomness is owned by the estimator (the core API takes no seed).

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

/* The numerator circuit carries the observable, the denominator does not */
static void estimate(const struct CCircuit *circuit,
                     const struct CHamiltonian *hamiltonian,
                     uintptr_t shots,
                     double *expectation,
                     double *variance) {
    if (hamiltonian != NULL) {
        *expectation = 1.5;
        *variance = 0.25;
    } else {
        *expectation = 2.0;
        *variance = 1.0;
    }
    (void)circuit;
    (void)shots;
}

int main(void) {
    /* Two single-qubit copies */
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);
    struct CVirtualDistillation *vd = virtual_distillation_new(qc, 2);

    /* Copy-swap circuit: width = 2 × 1 = 2 */
    struct CCircuit *copy_swap = virtual_distillation_build_circuit(vd);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);  /* takes ownership of z */

    /* mu_vd = 1.5 / 2.0 = 0.75 */
    double mean = 0.0, variance = 0.0;
    virtual_distillation_run_vd(vd, obs, 3, 2, estimate, &mean, &variance);
    printf("VD estimate: %f\n", mean);

    circuit_free(copy_swap);
    virtual_distillation_free(vd);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

The numerator and denominator can also be run separately, each yielding its own mean and variance:

```c
double num_mean = 0.0, num_var = 0.0;
virtual_distillation_run_numerator_circuit(vd, obs, 512, estimate,
                                           &num_mean, &num_var);
double den_mean = 0.0, den_var = 0.0;
virtual_distillation_run_denominator_circuit(vd, 512, estimate,
                                             &den_mean, &den_var);
```
