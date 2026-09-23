# Zero-Noise Extrapolation

`cqlib.error_mitigation.zne`

`cqlib.error_mitigation.zne` provides the low-level interface of zero-noise extrapolation: construct a mitigation circuit family according to the fold levels, estimate the expectation value of each circuit with a callback, and then extrapolate the results to the zero-noise point. This page covers `ZNEMitigation`, `ZneConfig`, `ExtrapolateMethod` and the callback type alias `Estimator`.

## Import

```python
from cqlib.error_mitigation.zne import (
    Estimator,
    ExtrapolateMethod,
    ZneConfig,
    ZNEMitigation,
)
```

The symbols above are also re-exported by `cqlib.error_mitigation`.

---

## Estimator

Callback type alias, specifying how the mitigation methods call the estimation function provided by the caller.

### `Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]`

Calling convention: pass the three arguments as positional arguments.

| Parameter | Type | Meaning |
| --- | --- | --- |
| `circuit` | `Circuit` | The mitigation circuit to execute this time. Zero-noise extrapolation passes the circuit expanded at some fold level. |
| `hamiltonian` | `Hamiltonian \| None` | The observable to estimate. Zero-noise extrapolation always passes the configured `Hamiltonian`. |
| `shots` | `int \| None` | The number of shots for this execution. |

Return convention: an `(expectation value, variance)` pair must be returned, and both elements are used as floating-point numbers.

| Return position | Meaning | Used by zero-noise extrapolation |
| --- | --- | --- |
| The 1st element | The expectation value of that circuit on the observable. | Used; it forms the input sequence of the extrapolation. |
| The 2nd element | The variance of that expectation value. | Not used; it takes part in post-processing only in virtual distillation. |

The value of `shots` depends on the calling path:

- `run_em_sequence()` always passes `None`.
- `run_em_sequence_with_shots()` passes the given number of shots.
- `RunArgs.zne(shots=...)` of the unified pipeline passes that value; it is `None` when not provided.

When `shots` is `None`, it means the callback decides the sampling method itself. The callback must be callable, otherwise a `TypeError` is raised before the flow starts. Exceptions raised inside the callback are re-raised as they are after the flow returns, and are not replaced by another exception type.

The same alias is also defined in `cqlib.error_mitigation.virtual_distillation` and `cqlib.error_mitigation.unified`, with the same meaning in all three; the callback form is the same in [Virtual distillation](2_virtual_distillation.md) and the [Unified pipeline](3_unified.md), differing only in the circuit and the observable passed in.

---

## ZNEMitigation

Low-level helper class of zero-noise extrapolation; it holds a copy of the base circuit and is responsible for folding, execution and extrapolation.

### ZNEMitigation(circuit, fold_levels)

Parameters:

- `circuit` (`Circuit`): the base circuit. It is copied on construction, and folding does not modify the circuit passed in.
- `fold_levels` (`Sequence[int]`): the fold level sequence, where each level corresponds to the noise factor `2 * level + 1`. No validation is performed on construction; a negative value fails only at folding time.

### Attributes

- `circuit -> Circuit`: a copy of the base circuit.
- `fold_levels -> list[int]`: the configured fold levels.
- `noise_factors -> list[int]`: the noise factors derived from the fold levels, satisfying `noise_factor = 2 * fold_level + 1`.

### Methods

- `fold_circuits(gate_set=None) -> list[Circuit]`: return one mitigation circuit per fold level, in the same order as `fold_levels` and with the same length as `fold_levels`.
- `run_em_sequence(gate_set, hamiltonian, estimator) -> list[float]`: execute the folded circuits one by one and return the list of expectation values. Equivalent to calling the next method with `shots=None`.
- `run_em_sequence_with_shots(gate_set, hamiltonian, shots, estimator) -> list[float]`: as above, and passes `shots` to the callback.
- `extrapolate(noisy_results, method, degree) -> float`: select the fitting method according to `method` and return the extrapolated value at the zero-noise point.
- `poly_extrapolate(noisy_results, degree) -> float`: extrapolate with a polynomial fit.
- `exp_extrapolate(noisy_results) -> float`: extrapolate with an exponential decay model fit.

### Other behavior

- Supports `copy` and `deepcopy`; a copy and the original object are independent of each other.
- Does not provide value equality comparison; two `ZNEMitigation` instances are equal only when they are the same object.
- `repr()` takes a form such as `ZNEMitigation(fold_levels=[0, 1, 2], noise_factors=[1, 3, 5])`.

---

## Folding behavior

The `gate_set` of `fold_circuits()` determines the folding scope:

- `gate_set` is `None` (the default): global folding, rewriting the circuit as a whole to `U -> U (U† U)^level`. The number of expanded operations grows linearly with the level.
- `gate_set` is an instruction sequence: selective folding, performing the same expansion only on operations whose instruction name matches any one of those instructions, and leaving the other operations as they are.

The instruction name is taken from the `Instruction` passed in, so it can first be constructed by standard gate name:

The `zne` in the examples below is a `ZNEMitigation` instance.

```python
from cqlib.circuit import Instruction, StandardGate

gate_set = [Instruction.from_standard_gate(StandardGate.X)]
folded = zne.fold_circuits(gate_set)
```

When the fold level is `0` no expansion is performed and a copy of the base circuit is returned. When the level is negative, folding fails and raises `CircuitError`.

---

## Execution conventions

Both `run_em_sequence()` and `run_em_sequence_with_shots()` require the observable qubit count to match the width of the base circuit, otherwise they raise `ErrorMitigationError`. The `gate_set` of both must be passed explicitly: pass `None` for global folding and an instruction sequence for selective folding.

Both methods call the callback once per fold level, and the returned list has the same length as `fold_levels`, with each list element being the 1st element of the pair returned by the callback:

```python
noisy = zne.run_em_sequence(None, hamiltonian, estimator)
```

The `shots` passed by `run_em_sequence()` is always `None`; use `run_em_sequence_with_shots()` when the number of shots needs to be handed to the callback.

---

## Extrapolation

All three extrapolation methods take `noise_factors` as the independent variable and `noisy_results` as the observed values, and return the fitted value at an independent variable of `0`.

### `extrapolate(noisy_results, method, degree) -> float`

Select the fitting method according to `method`:

- `ExtrapolateMethod.polynomial()`: uses `degree`, equivalent to `poly_extrapolate(noisy_results, degree)`.
- `ExtrapolateMethod.exponential()`: ignores `degree`, equivalent to `exp_extrapolate(noisy_results)`.

Parameters:

- `noisy_results` (`Sequence[float]`): the expectation values at the various fold levels; the length must be the same as that of `noise_factors`.
- `method` (`ExtrapolateMethod`): the fitting method.
- `degree` (`int`): the polynomial degree. This parameter does not take part in the computation for an exponential fit, but must still be passed.

Returns:

- `float`: the extrapolated value at the zero-noise point.

### `poly_extrapolate(noisy_results, degree) -> float`

Perform a least-squares fit of the given degree on the data points and return the constant term. `degree` must be strictly less than the number of noise factors; `noisy_results` must correspond one-to-one with the noise factors and must not be empty.

### `exp_extrapolate(noisy_results) -> float`

Perform a linear regression in log space according to `y(x) = A * exp(-x / tau)` and return the value `A` at the zero-noise point. Before the call, all `noisy_results` must be positive — log space requires non-negative values, and an entry equal to `0` is likewise rejected.

Example:

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.zne import ZNEMitigation

circuit = Circuit(1)
circuit.x(0)

zne = ZNEMitigation(circuit, [0, 1, 2])
print(zne.noise_factors)  # [1, 3, 5]

noisy_results = [2.75, 6.75, 10.75]
print(zne.poly_extrapolate(noisy_results, 1))  # 0.75
```

The data points in the example above satisfy `y = 0.75 + 2x`, and the constant term of the first-order fit is the value at the zero-noise point.

---

## ExtrapolateMethod

Extrapolation method, constructed through static methods. An instance is an immutable value object and can be passed to `ZNEMitigation.extrapolate()` and to `ProcessArgs.zne()` of the unified pipeline.

### Static methods

- `ExtrapolateMethod.polynomial()`: polynomial fit, requiring a degree.
- `ExtrapolateMethod.exponential()`: exponential decay fit in log space, not using a degree.

### Other behavior

- `str()` returns `"polynomial"` or `"exponential"`; `repr()` returns `ExtrapolateMethod.polynomial()` or `ExtrapolateMethod.exponential()`.
- Supports `==`, `hash`, `copy` and `deepcopy`.

---

## ZneConfig

The configuration object of zero-noise extrapolation; it can be constructed independently of the low-level class and passed to the unified pipeline.

### ZneConfig(fold_levels)

Parameters:

- `fold_levels` (`Sequence[int]`): the fold level sequence; negative values are not validated on construction.

### Attributes

- `fold_levels -> list[int]`: the configured fold levels.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`.
- `repr()` takes a form such as `ZneConfig(fold_levels=[0, 1, 2])`.

Example:

```python
from cqlib.error_mitigation.zne import ZneConfig

config = ZneConfig([0, 1, 2])
print(config.fold_levels)  # [0, 1, 2]
```

---

## Raises

| Exception | When it occurs |
| --- | --- |
| `ErrorMitigationError` | The observable qubit count does not match the base circuit width; the extrapolation input is empty; the extrapolation input length does not match the number of noise factors; the polynomial degree is not less than the number of data points; the exponential extrapolation input contains a non-positive value; the normal equations or the regression system of the fit are singular. |
| `CircuitError` | The fold level is negative and constructing the folded circuit fails. |
| `TypeError` | The `estimator` passed in is not callable. |
