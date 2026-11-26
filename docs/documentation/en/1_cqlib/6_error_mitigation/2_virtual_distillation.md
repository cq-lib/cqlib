# Virtual Distillation (VD)

Virtual distillation (VD) estimates the following through multiple copies of the density matrix and a **copy-swap** circuit (the Hadamard-test circuit of Huggins et al., PRX 11, 041036):

```text
Tr(O ρ^M) / Tr(ρ^M)
```

Here `M` is the number of copies (`copies`) and `O` is the observable represented by a `Hamiltonian`.

---

## Python entry points

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## Quick example: copy-swap and run_vd

```python
circuit = Circuit(1)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = em.VirtualDistillation(circuit, copies=2)

copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 3 = copies * base_width + 1 (the last qubit is the ancilla)

def estimator(run_circuit, observable, shots):
    # Both the numerator and the denominator carry a Hamiltonian; the
    # numerator also acts on the first copy's qubits, the denominator only
    # carries X on the ancillary (last) qubit.
    is_numerator = any(
        qubit < observable.num_qubits - 1
        for term, _ in observable.terms
        for qubit in term.support()
    )
    if is_numerator:
        return (1.5, 0.25)      # numerator: Tr(O ρ^M)
    return (2.0, 1.0)           # denominator: Tr(ρ^M)

mean, var = vd.run_vd(hamiltonian, shots_numerator=3, shots_denominator=2, estimator=estimator)
print("expectation:", mean)   # 0.75
print("variance:", var)
```

`copies` must be **≥ 2**; otherwise `ErrorMitigationError` is raised.

---

## Constructing the copy-swap circuit

```python
vd = em.VirtualDistillation(circuit, copies=2)
copy_swap = vd.build_copy_swap_circuit()
```

- The output circuit width is `copies` × the base circuit width **+ 1**; the last qubit is an ancillary qubit;
- The ancillary qubit is first prepared in `|+⟩` by an `H` gate, then adjacent copies are coupled by a ladder of `SWAP`s, each **controlled by the ancillary qubit** (CSWAP), which implements the derangement of the copies;
- The original `circuit` is not modified.

The number of copies can be updated through `set_copies(copies)`, and the circuit can then be reconstructed.

---

## Step-by-step execution

| API | Description |
|-----|------|
| `run_denominator_circuit(shots, estimator)` | Estimates `Tr(ρ^M)`; the observable is `X` on the ancillary qubit |
| `run_numerator_circuit(hamiltonian, shots, estimator)` | Estimates `Tr(O ρ^M)`; the observable is `O` on the first copy ⊗ `X` on the ancillary qubit |
| `run_vd(hamiltonian, shots_numerator, shots_denominator, estimator)` | Completes the numerator and denominator in one call and returns their ratio |

`estimator` receives a concrete `Hamiltonian` in both calls: the numerator observable acts non-trivially on the first copy's qubits and carries `X` on the ancillary qubit, while the denominator observable carries only `X` on the ancillary qubit (identities elsewhere).

---

## copies and resource overhead

| `copies` | Circuit width | Typical effect |
|----------|----------|----------|
| `2` | 3 qubits (2 copies + 1 ancilla) | Minimum configuration, lower overhead |
| `3` | 4 qubits (3 copies + 1 ancilla) | Stronger distillation, higher overhead |

The larger `M` is, the more the dominant eigenstate component is emphasized, but the preparation and measurement cost increases significantly.

---

## Integration with the unified pipeline

```python
mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2)),
)

def estimator(run_circuit, observable, shots):
    is_numerator = any(
        qubit < observable.num_qubits - 1
        for term, _ in observable.terms
        for qubit in term.support()
    )
    if is_numerator:
        return (1.5, 0.25)
    return (2.0, 1.0)

mitigation.run(hamiltonian, em.RunArgs.virtual_distillation(3, 2), estimator)
result = mitigation.get_mitigated(em.ProcessArgs.virtual_distillation())

print(result.expectation)
print(result.variance)
```

The `MitigatedResult.variance` of VD usually has a value.

---

## Notes

- VD suits scenarios where the dominant eigenstate is to be amplified and the impact of mixed noise reduced;
- The numerator observable is expanded internally to `O` on the first copy ⊗ identities ⊗ `X` on the ancillary qubit, and the denominator observable is `X` on the ancillary qubit, so Pauli terms do not need to be expanded manually;
- Formal experiments should record the shot budget of the numerator and the denominator separately.

---

## Next steps

- [Unified pipeline and Estimator](3_unified_api.md): learn how `ErrorMitigation` wraps the VD workflow.
