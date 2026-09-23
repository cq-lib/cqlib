# Error Mitigation

The Cqlib Python bindings provide error mitigation capabilities at the expectation-value level through **`cqlib.error_mitigation`**, currently supporting:

- **Zero-noise extrapolation (ZNE)**: constructs a circuit family at different noise strengths through gate folding, then extrapolates to the zero-noise limit;
- **Virtual distillation (VD)**: estimates `Tr(O ρ^M) / Tr(ρ^M)` through copy-swap circuits;
- **Unified pipeline (`ErrorMitigation`)**: wraps the above methods in the order `run()` → `get_mitigated()`.

Error mitigation reduces the systematic bias of **observable expectation values**; it is not quantum error correction. It usually increases the number of circuit executions and requires an **estimator** (a backend or simulator callback) to be supplied externally.

---

## Common entry points

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString

from cqlib.error_mitigation import (
    ErrorMitigation,
    ExtrapolateMethod,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
    VirtualDistillation,
    ZNEMitigation,
    ZneConfig,
)
```

> `cqlib.__init__` does not re-export the symbols above; import `cqlib.error_mitigation` explicitly.

---

## Recommended workflow

Most scenarios use the unified entry point `ErrorMitigation`:

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

def estimator(run_circuit, observable, shots):
    # replace with an expectation-value estimate from a simulator or a real backend
    return (0.5 * len(run_circuit.operations), 0.0)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print("expectation:", result.expectation)
print("variance:", result.variance)
```

When folding, copy-swap or extrapolation steps need to be debugged separately, the lower-level APIs `ZNEMitigation` or `VirtualDistillation` can be used instead.

---

## Estimator contract

All mitigation methods rely on the same **Estimator** signature:

```python
from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

Estimator = Callable[
    [Circuit, Hamiltonian | None, int | None],
    tuple[float, float],
]
```

| Parameter | Meaning |
|------|------|
| `run_circuit` | The circuit to be executed (possibly a folded circuit or a copy-swap circuit) |
| `observable` | The `Hamiltonian` to be estimated; `None` for the denominator circuit |
| `shots` | The shot count of this execution; may be `None` in some APIs |
| Return value | `(expectation, variance)` |

Cqlib does **not** provide a built-in simulator or backend estimator; connect `estimator` to QIS simulation, a density matrix noise model or the Tianyan backend.

---

## Data flow overview

```text
Original Circuit + Hamiltonian
  ↓
Construct the mitigation circuit family (ZNE folding / VD copy-swap)
  ↓
estimator runs each circuit and returns (expectation, variance)
  ↓
ZNE: extrapolate to noise_factor = 0
VD: compute the numerator / denominator ratio
  ↓
MitigatedResult(expectation, variance?)
```

---

## Module breakdown

| Module | Description |
|------|------|
| `cqlib.error_mitigation.zne` | `ZNEMitigation`, `ExtrapolateMethod`, `ZneConfig` |
| `cqlib.error_mitigation.virtual_distillation` | `VirtualDistillation`, `VirtualDistillationConfig` |
| `cqlib.error_mitigation.unified` | `ErrorMitigation`, `MitigationMethod`, `RunArgs`, `ProcessArgs` |

---

## Capability boundaries

- This module focuses on **ZNE** and **Virtual Distillation**;
- **Readout error correction** is not part of this module; see the Tianyan chapter [`5_readout_mitigation.md`](../7_tianyan/5_readout_mitigation.md);
- Each `ErrorMitigation` instance can call `run()` only once and `get_mitigated()` only once.

---

## Next steps

- [Zero-Noise Extrapolation (ZNE)](1_zne.md): learn gate folding, `noise_factors` and extrapolation methods.
- [Virtual Distillation (VD)](2_virtual_distillation.md): understand copy-swap circuits and ratio estimation.
- [Unified pipeline and Estimator](3_unified_api.md): master the `ErrorMitigation` state machine and error handling.
