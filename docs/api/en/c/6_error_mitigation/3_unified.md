# Unified Pipeline (C)

The unified pipeline `CErrorMitigation` wraps zero-noise extrapolation and virtual distillation into a "sampling → post-processing" two-stage flow: `error_mitigation_run` folds/derives the circuits and samples through the estimator callback, and `error_mitigation_get_mitigated` produces the mitigated result once sampling completes. For the callback contract and constants see the [module overview](0_overview.md); for error codes and handle ownership conventions see the [Overview](../0_overview.md).

Values of the `method` parameter:

| Constant | Value | Method | Extra parameter |
| --- | --- | --- | --- |
| `MITIGATION_ZNE` | 0 | Zero-noise extrapolation. | `fold_levels` fold-level array. |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | Virtual distillation. | `copies` copy count. |

Values of the `process_method` parameter:

| Constant | Value | Flow |
| --- | --- | --- |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE + polynomial extrapolation. |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE + exponential extrapolation. |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | Virtual distillation. |

---

## error_mitigation_new(circuit, method, fold_levels, num_levels, copies)

Creates a unified pipeline handle from a base circuit and a mitigation method.

- `circuit` (`const struct CCircuit *`): the base circuit.
- `method` (`uint8_t`): the mitigation method tag, see the table above.
- `fold_levels` (`const int32_t *`): the fold-level array, used when `method == MITIGATION_ZNE` (e.g. `{0, 1, 2}`).
- `num_levels` (`uintptr_t`): the number of elements in `fold_levels`; must be greater than 0 when `method == MITIGATION_ZNE`.
- `copies` (`uintptr_t`): the copy count, used when `method == MITIGATION_VIRTUAL_DISTILLATION`, at least 2.

Returns `struct CErrorMitigation *`: the new handle, freed with `error_mitigation_free`; NULL when `circuit` is NULL, `method` is not a valid tag, or the parameters do not match the method (ZNE without fold levels, VD with a copy count less than 2).

---

## error_mitigation_free(ptr)

Frees a unified pipeline handle.

- `ptr` (`struct CErrorMitigation *`): the handle to free; NULL is allowed.

---

## error_mitigation_run(ptr, method, hamiltonian, shots_a, shots_b, estimator)

Executes the sampling stage: folds/derives the circuits according to the configured method and invokes the estimator callback once per circuit execution. `method` must match the method configured in `error_mitigation_new`.

- `ptr` (`struct CErrorMitigation *`): the unified pipeline handle.
- `method` (`uint8_t`): the mitigation method tag; must match the one used at creation.
- `hamiltonian` (`const struct CHamiltonian *`): the observable to estimate, see [Hamiltonian](../3_qis/7_hamiltonian.md) for construction.
- `shots_a` (`uintptr_t`): the shot count for the ZNE method (0 = unspecified); the numerator-circuit shot count for the virtual distillation method.
- `shots_b` (`uintptr_t`): ignored by the ZNE method; the denominator-circuit shot count for the virtual distillation method.
- `estimator` (`CEstimatorFn`): the estimator callback; see the [module overview](0_overview.md) for the contract.

Returns `int32_t`: 0 on success; -1 (`ptr` or `hamiltonian` NULL); -8 (`method` is not a valid tag); -7 (`method` not matching the configured method, or other execution errors).

---

## error_mitigation_get_mitigated(ptr, process_method, degree, out_expectation, out_variance)

Executes the post-processing stage: produces the mitigated result after `error_mitigation_run`.

- `ptr` (`struct CErrorMitigation *`): the unified pipeline handle.
- `process_method` (`uint8_t`): the post-processing method tag, see the table above.
- `degree` (`uintptr_t`): the polynomial degree for ZNE extrapolation; 0 selects it automatically.
- `out_expectation` (`double *`): receives the mitigated expectation value.
- `out_variance` (`double *`): receives the mitigated variance; NaN when unavailable.

Returns `int32_t`: 0 on success; -1 (`ptr`, `out_expectation` or `out_variance` NULL); -8 (`process_method` is not a valid tag); -7 (`process_method` not matching the configured method, or other post-processing errors).

---

## Example

A complete flow with the virtual distillation method (the callback branches on the numerator circuit carrying the observable vs. the denominator circuit having none, see the [module overview](0_overview.md)):

```c
#include <stdio.h>
#include "cqlib_c.h"

/* Estimator callback: the numerator circuit carries the observable
   (exact simulation); the denominator circuit has none */
static void estimate(const struct CCircuit *circuit,
                     const struct CHamiltonian *hamiltonian,
                     uintptr_t shots,
                     double *expectation,
                     double *variance) {
    if (hamiltonian != NULL) {
        struct CStatevector *sv = statevector_from_circuit(circuit);
        double value = 0.0;
        statevector_expectation(sv, hamiltonian, &value);
        statevector_free(sv);
        *expectation = value;
        *variance = 0.0;
    } else {
        /* Denominator Tr(rho^M): estimated from measurement outcomes by a
           sampling estimator; a sample value is returned here */
        *expectation = 1.0;
        *variance = 0.0;
    }
    (void)shots;
}

int main(void) {
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);

    /* Unified pipeline: virtual distillation with 3 copies */
    struct CErrorMitigation *em =
        error_mitigation_new(qc, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, 3);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);  /* takes ownership of z */

    /* Sampling stage: numerator shots=512, denominator shots=256 */
    error_mitigation_run(em, MITIGATION_VIRTUAL_DISTILLATION, obs, 512, 256, estimate);

    /* Post-processing stage: the mitigated expectation and variance directly */
    double value = 0.0, variance = 0.0;
    error_mitigation_get_mitigated(em, PROCESS_VIRTUAL_DISTILLATION, 0,
                                   &value, &variance);

    error_mitigation_free(em);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

For the `new → run → get_mitigated` flow with the ZNE method, see the [module overview](0_overview.md); for fine-grained folding and extrapolation control see [Zero-Noise Extrapolation](1_zne.md), and for separate numerator/denominator execution see [Virtual Distillation](2_virtual_distillation.md).
