# Zero-Noise Extrapolation (ZNE)

Zero-noise extrapolation (ZNE) actively amplifies circuit noise through unitary folding to obtain several noisy expectations, then extrapolates to the zero-noise point.

---

## Python entry points

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## Quick example: folding, execution and extrapolation

```python
circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

zne = em.ZNEMitigation(circuit, [0, 1, 2])

print(zne.fold_levels)      # [0, 1, 2]
print(zne.noise_factors)    # [1, 3, 5]

folded = zne.fold_circuits()
print([len(c.operations) for c in folded])  # [1, 3, 5]

def estimator(folded_circuit, observable, shots):
    return (0.5 * len(folded_circuit.operations), 0.0)

noisy = zne.run_em_sequence_with_shots(None, hamiltonian, 256, estimator)
print("noisy:", noisy)  # [0.5, 1.5, 2.5]

mitigated = zne.extrapolate(
    noisy,
    em.ExtrapolateMethod.polynomial(),
    degree=1,
)
print("mitigated:", mitigated)
```

`ZNEMitigation` holds a copy of the original circuit; `fold_circuits()` does not modify the input `circuit`.

---

## fold_levels and noise_factors

Each `fold_level` corresponds to a noise factor:

```text
noise_factor = 2 * fold_level + 1
```

| `fold_level` | `noise_factor` | Meaning |
|--------------|----------------|------|
| `0` | `1` | No folding |
| `1` | `3` | One `U(U†U)` fold |
| `2` | `5` | Two folds |

`fold_levels` must contain non-negative integers; at least two points are needed for extrapolation. `ZneConfig` and `ZNEMitigation` do not validate negative numbers at construction; a negative value raises `ErrorMitigationError` when `ErrorMitigation` is constructed.

---

## Selective gate folding

With the default `gate_set=None`, folding is global. To fold only specific gates, pass a list of target instructions:

```python
from cqlib.circuit import Instruction, StandardGate

gate_set = [Instruction.from_standard_gate(StandardGate.X)]
folded = zne.fold_circuits(gate_set)
```

Only operations whose names match an instruction in `gate_set` are folded.

---

## Running the folded sequence

| API | Description |
|-----|------|
| `run_em_sequence(gate_set, hamiltonian, estimator)` | Executes the folded circuits one by one and returns a list of expectation values |
| `run_em_sequence_with_shots(gate_set, hamiltonian, shots, estimator)` | Same as above, and passes `shots` to the estimator |

`estimator` must be callable and must return a `(float, float)` tuple. The type signature is as follows:

```python
from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

Estimator = Callable[
    [Circuit, Hamiltonian | None, int | None],
    tuple[float, float],
]
```

---

## Extrapolation methods

| API | Description |
|-----|------|
| `extrapolate(noisy_results, method, degree)` | General entry point |
| `poly_extrapolate(noisy_results, degree)` | Polynomial fit |
| `exp_extrapolate(noisy_results)` | Exponential decay fit (log space) |

`ExtrapolateMethod` construction:

```python
em.ExtrapolateMethod.polynomial()
em.ExtrapolateMethod.exponential()
```

For polynomial extrapolation, `degree` should be less than `len(noisy_results)`; first-degree polynomial extrapolation is the most common default.

---

## Integration with the unified pipeline

```python
def estimator(folded_circuit, observable, shots):
    return (0.5 * len(folded_circuit.operations), 0.0)

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)
print(result.variance)  # None for ZNE
```

---

## Notes

- ZNE mitigates expectation-value bias and does not guarantee recovery of the fully noiseless state;
- Folding increases circuit depth and execution cost; `fold_levels` and `shots` should be recorded for formal experiments;
- Extrapolation results are sensitive to estimator quality, so control verification is recommended for key circuits.

---

## Next steps

- [Virtual Distillation (VD)](2_virtual_distillation.md): understand multi-copy copy-swap and ratio estimation.
- [Unified pipeline and Estimator](3_unified_api.md): wrap the complete ZNE workflow with `ErrorMitigation`.
