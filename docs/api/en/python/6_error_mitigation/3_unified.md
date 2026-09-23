# Unified Pipeline

`cqlib.error_mitigation.unified`

`cqlib.error_mitigation.unified` provides a sequential unified pipeline: first execute the mitigation circuits according to the selected method and collect the callback results, then perform method-related post-processing. This page covers `ErrorMitigation` and its companions `MitigationMethod`, `RunArgs`, `ProcessArgs`, `MitigatedResult` and `ErrorMitigationError`.

## Import

```python
from cqlib.error_mitigation.unified import (
    Estimator,
    ErrorMitigationError,
    MitigationMethod,
    RunArgs,
    ProcessArgs,
    MitigatedResult,
    ErrorMitigation,
)
```

Except for `Estimator`, the symbols above are also re-exported by `cqlib.error_mitigation`. For the callback convention see `Estimator` in [Zero-noise extrapolation](1_zne.md).

---

## MitigationMethod

Mitigation method selection, constructed through static methods; it determines which method this instance uses when the `ErrorMitigation` is constructed.

### Static methods

- `MitigationMethod.zne(config)`: the configuration is zero-noise extrapolation. The parameter `config` (`ZneConfig`) gives the fold levels.
- `MitigationMethod.virtual_distillation(config)`: the configuration is virtual distillation. The parameter `config` (`VirtualDistillationConfig`) gives the number of copies.

### Attributes

- `method_type -> str`: the method identifier, taking the values `"zne"` or `"virtual_distillation"`.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`.
- `repr()` takes a form such as `MitigationMethod.zne(ZneConfig(fold_levels=[0, 1, 2]))`.

---

## RunArgs

The run arguments required for one execution, corresponding one-to-one with the selected method. When the method is `zne`, only `RunArgs.zne(...)` can be used, otherwise `run()` raises `ErrorMitigationError`.

### Static methods

- `RunArgs.zne(gate_set=None, shots=None)`:
  - `gate_set` (`Sequence[Instruction] | None`): the gate set for selective folding. Defaults to `None`, meaning global folding.
  - `shots` (`int | None`): the number of shots passed to the callback. Defaults to `None`.
- `RunArgs.virtual_distillation(shots_numerator, shots_denominator)`:
  - `shots_numerator` (`int`): the number of shots for the numerator.
  - `shots_denominator` (`int`): the number of shots for the denominator.

### Attributes

- `method_type -> str`: the method identifier.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`. When comparing `zne` arguments, the elements of `gate_set` are compared by instruction name rather than by object identity.
- `repr()` takes a form such as `RunArgs.zne(gate_set=None, shots=Some(256))`.

Example:

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.error_mitigation.unified import RunArgs

global_folding = RunArgs.zne(shots=256)
selective_folding = RunArgs.zne(
    gate_set=[Instruction.from_standard_gate(StandardGate.X)],
    shots=256,
)

vd_args = RunArgs.virtual_distillation(shots_numerator=3, shots_denominator=2)
```

---

## ProcessArgs

Post-processing arguments, corresponding one-to-one with the selected method, passed at `get_mitigated()` time.

### Static methods

- `ProcessArgs.zne(method, degree=None)`:
  - `method` (`ExtrapolateMethod`): the extrapolation method.
  - `degree` (`int | None`): the polynomial degree. Defaults to `None`, in which case `min(number of data points - 1, 1)` is used. This parameter does not take part in the computation for an exponential fit.
- `ProcessArgs.virtual_distillation()`: virtual distillation has no additional post-processing arguments.

### Attributes

- `method_type -> str`: the method identifier.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`.
- `repr()` takes a form such as `ProcessArgs.zne(method=polynomial, degree=Some(1))`.

---

## MitigatedResult

Mitigation result, returned by `get_mitigated()`; it cannot be constructed directly. The attributes are read-only.

### Attributes

- `expectation -> float`: the mitigated expectation value.
- `variance -> float | None`: the mitigated variance. The result of zero-noise extrapolation is `None`; the result of virtual distillation has a value.

### Other behavior

- Supports `==`, `copy` and `deepcopy`; does not provide `hash`.
- `repr()` takes a form such as `MitigatedResult(expectation=0.5, variance=None)`.

---

## ErrorMitigation

Unified pipeline, completing one mitigation in the order `run()` → `get_mitigated()`.

### ErrorMitigation(circuit, method)

Parameters:

- `circuit` (`Circuit`): the base circuit. It is copied on construction.
- `method` (`MitigationMethod`): the mitigation method.

The method configuration is validated at construction time: when the fold levels of zero-noise extrapolation contain a negative value, or the number of copies of virtual distillation is less than 2, `ErrorMitigationError` is raised.

### Methods

- `run(hamiltonian, run_args, estimator) -> None`: execute the mitigation circuits corresponding to the method according to `run_args`, and record the callback results.
- `get_mitigated(process_args) -> MitigatedResult`: perform post-processing on the recorded results and return the mitigation result.

### Other behavior

- Supports `copy` and `deepcopy`; a copy carries the current state.
- Does not provide value equality comparison.
- `repr()` is fixed as `ErrorMitigation()`.

### State machine

Each instance completes one mitigation only, and both the call order and the argument matching are constrained by the instance itself:

| Current state | Call | Result |
| --- | --- | --- |
| Constructed | `run()` | Executes and transitions to the run state. |
| Constructed | `get_mitigated()` | Raises `ErrorMitigationError`, requiring `run()` to be completed first. |
| Run | `get_mitigated()` | Returns the result and transitions to the mitigated state. |
| Run | `run()` | Raises `ErrorMitigationError`; repeating `run()` is not allowed. |
| Mitigated | `run()` or `get_mitigated()` | Raises `ErrorMitigationError`; calling again is not allowed. |

When the `RunArgs` passed to `run()` do not match the method given at construction, `ErrorMitigationError` is raised; in that case the callback is not called and the state does not change. When the `ProcessArgs` passed to `get_mitigated()` do not match the method, `ErrorMitigationError` is likewise raised.

### Execution conventions

Zero-noise extrapolation: `run()` constructs the mitigation circuits according to the configured fold levels, calls the callback once per circuit, with the observable being the `Hamiltonian` passed in and the number of shots taken from `RunArgs.zne(shots=...)`. Only the 1st element of the pair returned by the callback is used, and the variance is discarded. `get_mitigated()` fits the extrapolation input, and the `variance` of the result is `None`.

Virtual distillation: `run()` first constructs the copy-swap circuit, then calls the callback twice on the same circuit — the numerator first, then the denominator. The numerator receives the observable expanded by the number of copies and `shots_numerator`, and the denominator receives `None` and `shots_denominator`. `get_mitigated()` computes the ratio according to the combination formula of [Virtual distillation](2_virtual_distillation.md), and the `variance` of the result has a value.

### Example

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.unified import (
    ErrorMitigation,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
)
from cqlib.error_mitigation.zne import ExtrapolateMethod, ZneConfig
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    return (len(run_circuit.operations) + 0.5, 0.0)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.zne(ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, RunArgs.zne(shots=256), estimator)
result = mitigation.get_mitigated(
    ProcessArgs.zne(ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)  # 0.5
print(result.variance)     # None
```

In the example above the operation counts of the folded circuits are `[1, 3, 5]` in order, the callback gives `[1.5, 3.5, 5.5]`, and the zero-noise point value of the first-order fit is `0.5`.

---

## ErrorMitigationError

The base class of error mitigation exceptions, inheriting from `cqlib.circuit.CqlibError`.

| Exception | When it occurs |
| --- | --- |
| `ErrorMitigationError` | See the situations listed in the table below. |
| `CircuitError` | Folded circuit or copy-swap circuit construction fails. |
| `ValueError` | An observable-related operation fails, for example an error while expanding the `Hamiltonian` by the number of copies. |
| `TypeError` | The `estimator` passed in is not callable. |

The common situations covered by `ErrorMitigationError`:

| When it occurs | Description |
| --- | --- |
| The fold levels contain a negative value at construction | Reported at `ErrorMitigation()` construction time; `ZneConfig` itself does not raise. |
| The number of copies is less than 2 at construction | Reported at `ErrorMitigation()` construction time. |
| The observable qubit count does not match the base circuit width | Reported in `run()`; the callback is not called. |
| `RunArgs` do not match the actual method | Reported in `run()`; the callback is not called. |
| `ProcessArgs` do not match the actual method | Reported in `get_mitigated()`. |
| `get_mitigated()` is called without `run()` | `run()` must be completed first. |
| `run()` or `get_mitigated()` is repeated | Each may be called only once per instance. |
| The extrapolation input is invalid or the fit fails | See [Zero-noise extrapolation](1_zne.md). |
| The denominator expectation value of virtual distillation is `0` | The ratio cannot be computed; see [Virtual distillation](2_virtual_distillation.md). |
