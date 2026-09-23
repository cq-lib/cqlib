# ErrorMitigation

A unified error mitigation pipeline that reduces the effect of noise on expectation value estimates through post-processing. It provides two paths, zero-noise extrapolation (ZNE) and virtual distillation.

The usage pattern is fixed to three stages: create with `error_mitigation_new` → sample with `error_mitigation_run` (through the estimator callback provided by the caller) → take the final estimate with `error_mitigation_get_mitigated`.

---

## Methods and constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `MITIGATION_ZNE` | 0 | Zero-noise extrapolation (requires `fold_levels`) |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | Virtual distillation (requires `copies`) |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE polynomial extrapolation post-processing |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE exponential extrapolation post-processing |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | Virtual distillation post-processing |

---

## Functions

### error_mitigation_new(circuit, method, fold_levels, num_levels, copies)

Create a mitigation pipeline.

Parameters:

- `circuit` (`const CCircuit *`): the target circuit, which is not modified.
- `method` (`uint8_t`): `MITIGATION_ZNE` or `MITIGATION_VIRTUAL_DISTILLATION`.
- `fold_levels` (`const int32_t *`): the array of ZNE fold levels (such as `{0, 1, 2}`), read only when `method = MITIGATION_ZNE`.
- `num_levels` (`uintptr_t`): the number of fold levels.
- `copies` (`uintptr_t`): the number of virtual distillation copies, read only when `method = MITIGATION_VIRTUAL_DISTILLATION`.

Returns:

- `CErrorMitigation *`: a heap-allocated pipeline that must be released with `error_mitigation_free`; NULL is returned on failure or when the parameters do not match the method (for example when ZNE is not given `fold_levels`).

### error_mitigation_run(ptr, method, hamiltonian, shots_a, shots_b, estimator)

Run the sampling stage, calling `estimator` circuit by circuit to collect the expectation estimates at each noise factor.

Parameters:

- `method` (`uint8_t`): must agree with the configuration of `error_mitigation_new`.
- `hamiltonian` (`const CHamiltonian *`): the observable to estimate.
- `shots_a` (`uintptr_t`): the number of shots for ZNE; the number of shots of the numerator circuit for virtual distillation.
- `shots_b` (`uintptr_t`): the number of shots of the denominator circuit for virtual distillation (ignored by ZNE).
- `estimator` (`CEstimatorFn`): the estimator callback, which must not be NULL.

**Shots semantics**:

- ZNE: only `shots_a` is used, and `0` means "no shots specified" (left to the callback); `shots_b` is ignored.
- VirtualDistillation: `shots_a` and `shots_b` are the numbers of shots of the numerator and denominator circuits respectively, and **both must be > 0**, otherwise it fails.

Returns:

- `int32_t`; `0` on success, a negative status code on failure.

### error_mitigation_get_mitigated(ptr, process_method, degree, out_expectation, out_variance)

Produce the final mitigated estimate after `run`.

Parameters:

- `process_method` (`uint8_t`): a `PROCESS_*` constant, which must match the pipeline method (a ZNE pipeline uses `PROCESS_ZNE_POLYNOMIAL`/`PROCESS_ZNE_EXPONENTIAL`, a virtual distillation pipeline uses `PROCESS_VIRTUAL_DISTILLATION`).
- `degree` (`uintptr_t`): the polynomial extrapolation degree (only for `PROCESS_ZNE_POLYNOMIAL`); `0` means automatic selection.
- `out_expectation` (`double *`): the mitigated expectation value.
- `out_variance` (`double *`): the estimate variance, which is NaN when unavailable.

Returns:

- `int32_t`; `0` on success; `-7` when `run` has not been called first.

### error_mitigation_free(ptr)

Release the object. Passing NULL is allowed.

---

## Estimator callback CEstimatorFn

```c
typedef void (*CEstimatorFn)(const struct CCircuit *circuit,
                             const struct CHamiltonian *hamiltonian,
                             uintptr_t shots,
                             double *expectation, double *variance);
```

Callback contract:

- It is called **circuit by circuit** during `run` (ZNE calls it once for each noise factor; virtual distillation calls it on the numerator and denominator circuits separately);
- The callback performs the actual sampling inside it (Statevector/density matrix simulation or a device task are both acceptable), and writes the estimated expectation value into `*expectation`;
- The variance is written into `*variance`; NaN may be written when it is unknown;
- A `shots` value of `0` means the caller did not specify the number of shots, and it may be chosen freely;
- **All pointers are valid only during the callback**: `circuit`/`hamiltonian` must not be saved for use after the callback returns, and **`hamiltonian` must not be released inside the callback**;
- The callback should not block for a long time, because it is called repeatedly during the sampling stage.

---

## Complete example: ZNE

```c
/* 估计器：真实场景替换为带噪声执行 */
static void counting_estimator(const CCircuit *qc,
                               const CHamiltonian *obs,
                               uintptr_t shots,
                               double *expectation, double *variance) {
    CStatevector *sv = statevector_from_circuit(qc);
    double e = 0.0;
    statevector_expectation(sv, obs, &e);   // 无噪声参考实现
    statevector_free(sv);
    *expectation = e;
    *variance = 0.0;
    (void)shots;
}

int32_t levels[3] = {0, 1, 2};
CErrorMitigation *em =
    error_mitigation_new(qc, MITIGATION_ZNE, levels, 3, 0);
if (!em) { return 1; }

if (error_mitigation_run(em, MITIGATION_ZNE, obs, 4000, 0,
                         counting_estimator) != 0) {
    error_mitigation_free(em);
    return 1;
}

double mitigated, variance;
error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 0,
                               &mitigated, &variance);
printf("mitigated = %.6f (var %.6f)\n", mitigated, variance);

error_mitigation_free(em);
```

### Virtual distillation path

```c
CErrorMitigation *em =
    error_mitigation_new(qc, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, 2);

/* 分子 4000 shots，分母 4000 shots（都必须 > 0） */
error_mitigation_run(em, MITIGATION_VIRTUAL_DISTILLATION, obs,
                     4000, 4000, counting_estimator);

double mitigated, variance;
error_mitigation_get_mitigated(em, PROCESS_VIRTUAL_DISTILLATION, 0,
                               &mitigated, &variance);
error_mitigation_free(em);
```

When the folded circuits need to be inspected directly see [ZNEMitigation](2_zne_mitigation.md); for the details of the virtual distillation protocol see [VirtualDistillation](3_virtual_distillation.md).
