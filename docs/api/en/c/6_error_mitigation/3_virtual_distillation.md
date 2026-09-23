# VirtualDistillation

The construction entry point of virtual distillation error mitigation. By making multiple copies of a quantum state and performing copy-swap interference, it suppresses the noise contribution to second order in the expectation value estimate. For the end-to-end mitigation flow (including sampling and post-processing) see [ErrorMitigation](1_error_mitigation.md) (`method = MITIGATION_VIRTUAL_DISTILLATION`).

---

## Functions

### virtual_distillation_new(circuit, copies)

Create, for `circuit`, a virtual distillation helper object with `copies` copy registers.

Parameters:

- `circuit` (`const CCircuit *`): the target circuit.
- `copies` (`uintptr_t`): the number of copy registers, which must be > 1.

Returns:

- `CVirtualDistillation *`: a heap-allocated object that must be released with `virtual_distillation_free`; NULL is returned on failure.

### virtual_distillation_build_circuit(ptr)

Build the copy-swap circuit required by the protocol.

Returns:

- `CCircuit *`: an **independently owned** new circuit that must be released with `circuit_free`; NULL is returned on failure.

The constructed circuit has more qubits than the original circuit (it must hold all the copy registers), and can be used to inspect the protocol structure or to be handed to a simulator for verification.

### virtual_distillation_free(ptr)

Release the object. Passing NULL is allowed.

---

## Relationship with ErrorMitigation

The complete "sampling + post-processing" flow is provided by [ErrorMitigation](1_error_mitigation.md):

1. `error_mitigation_new(circuit, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, copies)`;
2. `error_mitigation_run(..., shots_a, shots_b, estimator)` — `shots_a`/`shots_b` are the numbers of shots of the numerator and denominator circuits respectively, and both must be > 0;
3. `error_mitigation_get_mitigated(..., PROCESS_VIRTUAL_DISTILLATION, ...)`.

The interfaces on this page suit scenarios that only need to construct and inspect the protocol circuit.

---

## Example

```c
CVirtualDistillation *vd = virtual_distillation_new(qc, 2);
if (!vd) { return 1; }

CCircuit *protocol = virtual_distillation_build_circuit(vd);
printf("protocol qubits=%zu (原线路 %zu)\n",
       (size_t)circuit_num_qubits(protocol),
       (size_t)circuit_num_qubits(qc));
circuit_free(protocol);

virtual_distillation_free(vd);
```
