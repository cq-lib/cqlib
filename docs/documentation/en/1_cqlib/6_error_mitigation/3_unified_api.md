# Unified pipeline and Estimator

`ErrorMitigation` is the recommended sequential entry point: `run()` first collects the estimator output of each mitigation circuit, then `get_mitigated()` performs method-specific post-processing.

---

## Python entry points

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## State machine

Each `ErrorMitigation` instance follows a fixed lifecycle:

```text
create -> run() -> get_mitigated() -> end
```

| Rule | Description |
|------|------|
| `run()` can be called only once | A repeated call raises `ErrorMitigationError` |
| `run()` must be called before `get_mitigated()` | Otherwise `ErrorMitigationError` is raised |
| `get_mitigated()` can be called only once | A repeated call raises `ErrorMitigationError` |

---

## ZNE pipeline example

```python
circuit = Circuit(1)
circuit.x(0)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

def estimator(run_circuit, observable, shots):
    assert observable is not None
    assert shots == 128
    return (0.5 * len(run_circuit.operations), 0.0)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)
print(result.variance)  # ZNE: None
```

---

## Virtual Distillation pipeline example

```python
circuit = Circuit(1)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2)),
)

def estimator(run_circuit, observable, shots):
    assert run_circuit.width == 2
    if observable is None:
        return (2.0, 1.0)
    return (1.5, 0.25)

mitigation.run(hamiltonian, em.RunArgs.virtual_distillation(3, 2), estimator)
result = mitigation.get_mitigated(em.ProcessArgs.virtual_distillation())

print(result.expectation)
print(result.variance)
```

---

## Configuration objects

### MitigationMethod

```python
em.MitigationMethod.zne(em.ZneConfig([0, 1, 2]))
em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2))
```

### RunArgs

| Constructor | Parameters |
|----------|------|
| `RunArgs.zne(gate_set=None, shots=None)` | Optional selective fold gate set and shot count |
| `RunArgs.virtual_distillation(shots_numerator, shots_denominator)` | Numerator/denominator shot budget |

### ProcessArgs

| Constructor | Parameters |
|----------|------|
| `ProcessArgs.zne(method, degree=None)` | Extrapolation method and polynomial degree |
| `ProcessArgs.virtual_distillation()` | No additional parameters |

### MitigatedResult

| Field | Description |
|------|------|
| `expectation` | Expectation value after mitigation |
| `variance` | Variance after mitigation; `None` for ZNE and usually a value for VD |

---

## Key points for implementing an Estimator

```python
def estimator(run_circuit, observable, shots):
    # run_circuit: may be a folded circuit or a copy-swap circuit
    # observable: a Hamiltonian or None (VD denominator)
    # shots: an int or None
    return (expectation, variance)
```

Confirm the following when implementing:

- The return value must be `(float, float)`;
- The `run_em_sequence_with_shots` of ZNE passes `shots` to the estimator;
- On the VD denominator path, `observable is None`;
- Exceptions inside the estimator are propagated upwards as-is.

---

## Common errors

| Scenario | Exception |
|------|------|
| `fold_levels` contains a negative number | `ErrorMitigationError` is raised when `ErrorMitigation` is constructed (`ZneConfig([-1])` itself does not raise) |
| `copies < 2` | Constructing `VirtualDistillation` or `ErrorMitigation` raises `ErrorMitigationError` |
| `get_mitigated()` without `run()` | `ErrorMitigationError` |
| `estimator` is not callable | `TypeError` |
| estimator return value has the wrong format | `TypeError` / `ValueError` |

---

## Lower-level APIs and module exports

Besides `ErrorMitigation`, the following can also be used directly:

- `ZNEMitigation` — see [Zero-Noise Extrapolation (ZNE)](1_zne.md);
- `VirtualDistillation` — see [Virtual Distillation](2_virtual_distillation.md).

`cqlib.error_mitigation.__all__` exports: `Estimator`, `ErrorMitigationError`, `ExtrapolateMethod`, `ZneConfig`, `VirtualDistillationConfig`, `MitigationMethod`, `RunArgs`, `ProcessArgs`, `MitigatedResult`, `ZNEMitigation`, `VirtualDistillation`, `ErrorMitigation`.

---

## Notes

- The unified pipeline and the lower-level APIs share the same `Estimator` type alias;
- Readout error correction belongs to the Tianyan module and is not part of `cqlib.error_mitigation`;
- When integrating with real hardware, wrap the estimator as backend shot acquisition and expectation-value aggregation logic.
