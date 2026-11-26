# Virtual Distillation

`cqlib.error_mitigation.virtual_distillation`

`cqlib.error_mitigation.virtual_distillation` provides the low-level interface of virtual distillation: copy the base circuit several times, add one ancillary qubit prepared by a Hadamard gate and apply ancilla-controlled SWAPs between adjacent copies (a Hadamard-test circuit), then estimate the numerator and the denominator separately, and finally compute the ratio to obtain the mitigated expectation value. This page covers `VirtualDistillation` and `VirtualDistillationConfig`.

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
- `copies` (`int`): the number of copies, which must not be less than 2, otherwise `ErrorMitigationError` is raised. The number of copies determines the width of the mitigation circuit: the width of the copy-swap circuit is `copies` times the width of the base circuit plus 1 (the extra qubit is the ancillary qubit of the Hadamard test).

### Attributes

- `copies -> int`: the currently configured number of copies.

### Methods

- `set_copies(copies)`: update the number of copies. Validation is completed before assignment, so the number of copies of the object stays unchanged when validation fails.
- `build_copy_swap_circuit() -> Circuit`: construct the copy-swap circuit. The base circuit is first decomposed, then copied one by one into their respective qubit ranges, and finally a Hadamard gate is applied to the trailing ancillary qubit, followed by SWAPs between adjacent copies controlled by the ancillary qubit.
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
3. One ancillary qubit is appended as the last qubit, prepared in `|+⟩` by an `H` gate; then, between each pair of adjacent copies, SWAPs are inserted qubit-wise, all controlled by the ancillary qubit (CSWAP). This ladder of controlled SWAPs implements the derangement of the copies required by the Hadamard test.

Therefore, when the number of copies is 2, each operation of the base circuit appears 2 times, plus one `H` on the ancillary qubit and the qubit-wise controlled SWAPs; as the number of copies increases, the number of controlled SWAPs grows linearly with the number of copies rather than as pairwise combinations of copies.

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.virtual_distillation import VirtualDistillation

circuit = Circuit(1)
circuit.x(0)

vd = VirtualDistillation(circuit, 2)
copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 3 = copies * base_width + 1
```

---

## Execution conventions

Virtual distillation has only one mitigation circuit; both the numerator and the denominator are executed on that circuit, differing in the observable and the number of shots passed to the callback:

| Method | Circuit received by the callback | Observable received by the callback | Shots received by the callback |
| --- | --- | --- | --- |
| `run_denominator_circuit(shots, estimator)` | copy-swap circuit | `Hamiltonian` of `X` on the ancillary qubit (identities elsewhere) | `shots` |
| `run_numerator_circuit(hamiltonian, shots, estimator)` | copy-swap circuit | The `Hamiltonian` expanded to the full width (`O` on the first copy ⊗ `X` on the ancillary qubit) | `shots` |
| `run_vd(...)` | copy-swap circuit | Numerator first, then denominator, as above | `shots_numerator` and `shots_denominator` respectively |

Both the numerator and the denominator observables are concrete `Hamiltonian` objects; they are told apart by content — the numerator observable has non-identity Paulis on the first copy's qubits, while the denominator observable is non-identity only on the ancillary (last) qubit.

The observable used by the numerator is obtained by expanding the `Hamiltonian` passed in: the original Pauli terms are kept on their respective qubits (the first copy), the remaining copies are padded with `I`, and an `X` is appended on the ancillary qubit, so that the observable qubit count matches the width of the copy-swap circuit. The expansion is performed inside the class, and the caller only needs to pass the observable acting on the base circuit.

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
    # 分子与分母都携带观测量：分子在第 0 份副本的比特上有非恒等 Pauli，
    # 分母只有辅助比特（最后一个比特）上的 X
    is_numerator = any(
        qubit < observable.num_qubits - 1
        for term, _ in observable.terms
        for qubit in term.support()
    )
    if is_numerator:
        return (1.5, 0.25)      # 分子：Tr(O ρ^M)
    return (2.0, 1.0)           # 分母：Tr(ρ^M)


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
