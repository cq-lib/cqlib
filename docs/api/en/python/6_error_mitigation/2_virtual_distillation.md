# Virtual Distillation

`cqlib.error_mitigation.virtual_distillation`

`cqlib.error_mitigation.virtual_distillation` provides the low-level interface of virtual distillation: copy the base circuit several times and insert SWAPs to construct a copy-swap circuit, then estimate the numerator and the denominator separately, and finally compute the ratio to obtain the mitigated expectation value. This page covers `VirtualDistillation` and `VirtualDistillationConfig`.

## Import

```python
from cqlib.error_mitigation.virtual_distillation import (
    Estimator,
    VirtualDistillationConfig,
    VirtualDistillation,
)
```

`VirtualDistillation` and `VirtualDistillationConfig` are also re-exported by `cqlib.error_mitigation`; for the callback convention see `Estimator` in [Zero-noise extrapolation](1_zne.md).

The ratio estimated by virtual distillation is `Tr(O ρ^M) / Tr(ρ^M)`, where `M` is the number of copies and `O` is the observable represented by the `Hamiltonian`.

---

## VirtualDistillationConfig

The configuration object of virtual distillation; it can be constructed independently of the low-level class and passed to the unified pipeline.

### VirtualDistillationConfig(copies)

Parameters:

- `copies` (`int`): the number of copies of the density matrix. No validation is performed on construction; a value less than 2 fails when the configuration is actually used.

### Attributes

- `copies -> int`: the configured number of copies.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`.
- `repr()` takes a form such as `VirtualDistillationConfig(copies=2)`.

---

## VirtualDistillation

Low-level helper class of virtual distillation; it holds a copy of the base circuit and is responsible for constructing the copy-swap circuit, executing the numerator and the denominator, and computing the ratio.

### VirtualDistillation(circuit, copies)

Parameters:

- `circuit` (`Circuit`): the base circuit. It is copied on construction and the circuit passed in is not modified.
- `copies` (`int`): the number of copies, which must not be less than 2, otherwise `ErrorMitigationError` is raised. The number of copies determines the width of the mitigation circuit: the width of the copy-swap circuit is `copies` times the width of the base circuit.

### Attributes

- `copies -> int`: the currently configured number of copies.

### Methods

- `set_copies(copies)`: update the number of copies. Validation is completed before assignment, so the number of copies of the object stays unchanged when validation fails.
- `build_copy_swap_circuit() -> Circuit`: construct the copy-swap circuit. The base circuit is first decomposed, then copied one by one into their respective qubit ranges, and finally SWAPs are inserted qubit-wise.
- `run_denominator_circuit(shots, estimator) -> tuple[float, float]`: execute the denominator circuit and return the pair given by the callback.
- `run_numerator_circuit(hamiltonian, shots, estimator) -> tuple[float, float]`: execute the numerator circuit and return the pair given by the callback.
- `run_vd(hamiltonian, shots_numerator, shots_denominator, estimator) -> tuple[float, float]`: execute the numerator and the denominator in turn and compute the ratio, returning `(mitigated expectation value, variance)`.

### Other behavior

- Supports `copy` and `deepcopy`; a copy and the original object are independent of each other.
- Does not provide value equality comparison; two `VirtualDistillation` instances are equal only when they are the same object.
- `repr()` takes a form such as `VirtualDistillation(copies=2)`.

---

## copy-swap circuit

The output of `build_copy_swap_circuit()` consists of three parts:

1. The base circuit is first decomposed, ensuring that a group of already expanded operations is copied.
2. The base circuit is copied `copies` times, with the `i`-th copy landing on the qubit range offset by `i` times the base width.
3. For the 1st copy and each of the other copies, SWAPs are inserted qubit-wise. SWAPs are inserted only between the 1st copy and each of the other copies; the other copies are not directly connected to one another.

Therefore, when the number of copies is 2, each operation of the base circuit appears 2 times, plus the qubit-wise SWAPs; as the number of copies increases, the number of SWAPs grows linearly with the number of copies rather than as pairwise combinations of copies.

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.virtual_distillation import VirtualDistillation

circuit = Circuit(1)
circuit.x(0)

vd = VirtualDistillation(circuit, 2)
copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 2
```

---

## Execution conventions

Virtual distillation has only one mitigation circuit; both the numerator and the denominator are executed on that circuit, differing in the observable and the number of shots passed to the callback:

| Method | Circuit received by the callback | Observable received by the callback | Shots received by the callback |
| --- | --- | --- | --- |
| `run_denominator_circuit(shots, estimator)` | copy-swap circuit | `None` | `shots` |
| `run_numerator_circuit(hamiltonian, shots, estimator)` | copy-swap circuit | The `Hamiltonian` expanded by the number of copies | `shots` |
| `run_vd(...)` | copy-swap circuit | Numerator first, then denominator, as above | `shots_numerator` and `shots_denominator` respectively |

The observable used by the numerator is obtained by expanding the `Hamiltonian` passed in: the original Pauli terms are kept on their respective qubits and the higher-order qubits are padded with `Z`, so that the observable qubit count matches the width of the copy-swap circuit. The expansion is performed inside the class, and the caller only needs to pass the observable acting on the base circuit.

`run_vd()` completes two steps in turn: first execute the numerator with `shots_numerator`, then execute the denominator with `shots_denominator`, and then combine the results as follows:

```text
期望值 = 分子期望值 / 分母期望值
方差   = 分子方差 / 分母期望值^2
       + 分子期望值^2 * 分母方差 / 分母期望值^4
```

When the denominator expectation value is `0` the ratio cannot be computed and `ErrorMitigationError` is raised. `run_vd()` requires the observable qubit count to match the base circuit width, and raises `ErrorMitigationError` before calling the callback when they do not match.

`run_denominator_circuit()` and `run_numerator_circuit()` do not combine the results and return the `(expectation value, variance)` pair given by the callback directly, suiting scenarios that need to inspect the numerator and denominator results separately.

Example:

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.virtual_distillation import VirtualDistillation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = VirtualDistillation(circuit, 2)


def estimator(run_circuit, observable, shots):
    if observable is None:
        return (2.0, 1.0)   # 分母：Tr(ρ^M)
    return (1.5, 0.25)      # 分子：Tr(O ρ^M)


expectation, variance = vd.run_vd(
    hamiltonian,
    shots_numerator=3,
    shots_denominator=2,
    estimator=estimator,
)
print(expectation)  # 0.75
print(variance)     # 0.203125
```

In the example above the numerator and the denominator are fixed values, so `0.75 = 1.5 / 2.0` and `0.203125 = 0.25 / 4 + 2.25 / 16`.

---

## Raises

| Exception | When it occurs |
| --- | --- |
| `ErrorMitigationError` | The number of copies is less than 2 (at construction time or in `set_copies()`); the observable qubit count of `run_vd()` does not match the base circuit width; the denominator expectation value is `0`. |
| `CircuitError` | Copy-swap circuit construction fails, or the qubit count after observable expansion does not match the mitigation circuit width. |
| `TypeError` | The `estimator` passed in is not callable. |
