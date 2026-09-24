# Error Mitigation (C)

The error mitigation module suppresses the influence of noise on expectation-value estimates through three complementary pipelines: zero-noise extrapolation (ZNE) folds the circuit at several noise factors and extrapolates back to the zero-noise limit, virtual distillation (VD) uses multiple circuit copies and a copy-swap circuit to estimate the ratio `Tr(O ρ^M) / Tr(ρ^M)`, and the unified pipeline `CErrorMitigation` wraps both into a "sampling → post-processing" two-stage flow. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md).

The three submodules share a single estimator callback type, `CEstimatorFn`: each mitigation flow hands the circuit to be estimated to the callback, which reports the expectation value and variance.

---

## Page Navigation

| Page | Content |
| --- | --- |
| [Zero-Noise Extrapolation](1_zne.md) | `CZneMitigation`: circuit folding, noise factors, extrapolation and the mitigation sequence; `CCircuitList` traversal. |
| [Virtual Distillation](2_virtual_distillation.md) | `CVirtualDistillation`: the copy-swap circuit, numerator/denominator circuits and the full protocol. |
| [Unified Pipeline](3_unified.md) | `CErrorMitigation`: the `new → run → get_mitigated` two-stage flow. |

---

## Constants

Mitigation method tags (the `method` parameter of `error_mitigation_new`):

| Constant | Value | Method | Extra parameter |
| --- | --- | --- | --- |
| `MITIGATION_ZNE` | 0 | Zero-noise extrapolation. | `fold_levels` fold-level array. |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | Virtual distillation. | `copies` copy count. |

Post-processing tags (the `process_method` parameter of `error_mitigation_get_mitigated`):

| Constant | Value | Flow |
| --- | --- | --- |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE + polynomial extrapolation. |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE + exponential extrapolation. |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | Virtual distillation. |

ZNE extrapolation method tags (the `method` parameter of `zne_extrapolate`):

| Constant | Value | Method |
| --- | --- | --- |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | Polynomial fit. |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | Exponential-decay fit. |

---

## CEstimatorFn Callback Contract

```c
typedef void (*CEstimatorFn)(const struct CCircuit *circuit,
                             const struct CHamiltonian *hamiltonian,
                             uintptr_t shots,
                             double *expectation,
                             double *variance);
```

Mitigation flows obtain expectation estimates through this callback. The contract:

- **Pointer validity**: `circuit`, `hamiltonian` and all out pointers are valid only for the duration of the callback; do not keep using them or store them beyond the callback.
- **hamiltonian ownership**: the `hamiltonian` handle is owned by the library and must not be freed by the callback. The Hamiltonian the callback receives may already have been expanded to the width of the folded/copy circuit as required by the flow.
- **Out writes**: the callback must write the expectation value through `expectation`; write the variance through `variance` when known, NaN otherwise. In the virtual distillation denominator estimate, `hamiltonian` is NULL — treat it as "no observable".
- **shots semantics**: `shots == 0` means "not specified" and the callback decides the sampling scale; a non-zero value is the shot count suggested by the caller.

Inside the callback you can freely call simulator interfaces (e.g. `statevector_from_circuit` + `statevector_expectation` from the [Statevector](../3_qis/1_statevector.md) page) to compute the estimate.

---

## Example

```c
#include <stdio.h>
#include "cqlib_c.h"

/* Estimator callback: exactly simulates the folded circuit and computes <H> */
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
    *variance = 0.0;  /* unknown variance: NaN is also valid */
    (void)shots;
}

int main(void) {
    struct CCircuit *qc = circuit_new(1);
    circuit_x(qc, 0);

    /* Unified pipeline: ZNE with fold levels {0, 1, 2} */
    int32_t levels[3] = {0, 1, 2};
    struct CErrorMitigation *em =
        error_mitigation_new(qc, MITIGATION_ZNE, levels, 3, 0);

    struct CPauliString *z = pauli_string_parse("Z");
    struct CHamiltonian *obs = hamiltonian_from_pauli(z);

    /* Sampling stage → post-processing stage */
    error_mitigation_run(em, MITIGATION_ZNE, obs, 0, 0, estimate);
    double value = 0.0, variance = 0.0;
    error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 0,
                                   &value, &variance);

    error_mitigation_free(em);
    hamiltonian_free(obs);
    circuit_free(qc);
    return 0;
}
```

For the low-level entry points (fine-grained ZNE folding/extrapolation control, separate virtual distillation numerator/denominator execution), see [Zero-Noise Extrapolation](1_zne.md) and [Virtual Distillation](2_virtual_distillation.md).
