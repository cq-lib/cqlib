# Error Mitigation

`cqlib.error_mitigation`

`cqlib.error_mitigation` provides error mitigation capabilities at the level of observable expectation values, covering the two methods of zero-noise extrapolation and virtual distillation. The module provides both low-level single-method helper classes and a unified pipeline: the former expose the steps of folding, execution and extrapolation one by one, and the latter converge the two methods into the same sequential flow.

## Overview

Error mitigation deals with the systematic bias of expectation values: the circuit is still executed according to its original semantics, and the mitigation approach extrapolates away or cancels the impact of noise on the expectation value through additionally constructed circuits and post-processing. It has a different focus from quantum error correction — it does not introduce encoding and syndrome measurement, and its cost is the additional number of circuit executions.

All methods in the module rely on the same convention: the caller provides a callable object that estimates the result of a circuit on an observable, and the module generates the circuit family for that execution, collects the return values and performs post-processing.

### Roles of the three entry points

| Entry point | Coverage | Use case |
| --- | --- | --- |
| `ZNEMitigation` | Zero-noise extrapolation | Need separate control over fold levels, selective folding or the extrapolation method. |
| `VirtualDistillation` | Virtual distillation | Need separate control over the number of copies, or to execute the numerator and denominator circuits in separate steps. |
| `ErrorMitigation` | Both | Complete one full mitigation through a unified flow. |

The low-level classes and the unified pipeline share the same set of configuration objects and callback conventions: `ZneConfig` and `VirtualDistillationConfig` can both be used to construct a `MitigationMethod`, and the meaning of their fields is consistent with that in the low-level classes; `Estimator` is called in the same form on all three entry points. The two do not share state, and the same set of configurations takes effect independently in the low-level classes and in the unified pipeline.

The unified pipeline selects the execution path by method:

- `MitigationMethod.zne(config)` first folds the circuit according to `ZneConfig.fold_levels`, calls the callback on each circuit to collect expectation values, and then uses `get_mitigated()` to perform extrapolation.
- `MitigationMethod.virtual_distillation(config)` first constructs a copy-swap circuit according to `VirtualDistillationConfig.copies`, calls the callback on the same circuit with the numerator and the denominator observables respectively, and then uses `get_mitigated()` to compute the ratio.

### Mitigation circuit family

Neither method executes the input circuit directly; both first construct a family of mitigation circuits:

- Zero-noise extrapolation expands the circuit according to the fold level. Global folding rewrites the circuit as `U -> U (U† U)^level`, so that the circuit depth grows with the level; selective folding performs the same expansion only on operations whose instruction name matches the given gate set, leaving the other operations as they are. When the level is `0` the mitigation circuit is a copy of the input circuit.
- Virtual distillation copies the circuit `copies` times and inserts SWAPs qubit-wise, yielding a copy-swap circuit of `copies` times the width. The numerator and the denominator are executed on the same circuit, differing only in the observable passed to the callback: the denominator is `None` and the numerator is the `Hamiltonian` expanded by the number of copies.

### Data flow

```text
输入线路 + 观测量
  ↓
构造缓解线路族（折叠线路 / copy-swap 线路）
  ↓
回调逐条估计，返回 (期望值, 方差)
  ↓
后处理：外推到零噪声点 / 分子分母求比值
  ↓
MitigatedResult(expectation, variance)
```

The two post-processing procedures handle `variance` differently: zero-noise extrapolation uses only the expectation value sequence and the `variance` of the result is `None`; virtual distillation uses both the expectation values and the variances, and the `variance` of the result has a value.

---

## Common entry points

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import (
    ErrorMitigation,
    ExtrapolateMethod,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
    ZneConfig,
)
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    # 替换为后端或模拟器的期望值估计
    return (len(run_circuit.operations) + 0.5, 0.0)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.zne(ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, RunArgs.zne(shots=256), estimator)
result = mitigation.get_mitigated(
    ProcessArgs.zne(ExtrapolateMethod.polynomial(), degree=1)
)

print("expectation:", result.expectation)
print("variance:", result.variance)
```

The call order is constrained by the object itself: each `ErrorMitigation` instance can be `run()` only once and `get_mitigated()` only once, and `run()` must be called before `get_mitigated()`.

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Error mitigation** | Processing that reduces the systematic bias of an observable at the expectation value level; it does not change the unitary semantics of the circuit and is not equivalent to quantum error correction. |
| **Fold level** | The non-negative integer controlling the number of circuit expansions in zero-noise extrapolation, given by `fold_levels`. |
| **Noise factor** | The noise amplification factor derived from the fold level, satisfying `noise_factor = 2 * fold_level + 1`. |
| **Global folding** | The expansion `U -> U (U† U)^level` applied to the whole circuit when `gate_set` is `None`. |
| **Selective folding** | Folding applied only to operations whose instruction name matches `gate_set`, leaving the other operations as they are. |
| **Extrapolation method** | The fitting model that extrapolates expectation values at different noise factors to the zero-noise point, including the polynomial and exponential kinds. |
| **Copies** | The number of copies of the density matrix in virtual distillation, given by `copies`, with a minimum of 2. |
| **copy-swap circuit** | The mitigation circuit obtained by copying the input circuit `copies` times and inserting SWAPs qubit-wise. |
| **Numerator circuit** | The copy-swap circuit execution used to estimate `Tr(O ρ^M)`, with the observable being the `Hamiltonian` expanded by the number of copies. |
| **Denominator circuit** | The copy-swap circuit execution used to estimate `Tr(ρ^M)`, with the observable being `None`. |
| **estimator** | The callback provided by the caller, which executes one mitigation circuit and returns an `(expectation value, variance)` pair. |
| **Unified pipeline** | The sequential flow carried by `ErrorMitigation`: `run()` collects the estimates and `get_mitigated()` performs post-processing. |

---

## `cqlib.error_mitigation` API Overview

### Zero-noise extrapolation

| Name | Description |
| --- | --- |
| [`ZNEMitigation`](1_zne.md) | Low-level zero-noise extrapolation helper class, responsible for folding, execution and extrapolation. |
| [`ZneConfig`](1_zne.md) | Zero-noise extrapolation configuration, containing the fold levels. |
| [`ExtrapolateMethod`](1_zne.md) | Extrapolation method, including `polynomial` and `exponential`. |
| [`Estimator`](1_zne.md) | Callback type alias, specifying three parameters and a two-element return. |

### Virtual distillation

| Name | Description |
| --- | --- |
| [`VirtualDistillation`](2_virtual_distillation.md) | Low-level virtual distillation helper class, responsible for constructing the copy-swap circuit and computing the ratio. |
| [`VirtualDistillationConfig`](2_virtual_distillation.md) | Virtual distillation configuration, containing the number of copies. |

### Unified pipeline

| Name | Description |
| --- | --- |
| [`ErrorMitigation`](3_unified.md) | Unified pipeline, completing one mitigation in the order `run()` → `get_mitigated()`. |
| [`MitigationMethod`](3_unified.md) | Mitigation method selection, including `zne` and `virtual_distillation`. |
| [`RunArgs`](3_unified.md) | Execution arguments, corresponding one-to-one with the selected method. |
| [`ProcessArgs`](3_unified.md) | Post-processing arguments, corresponding one-to-one with the selected method. |
| [`MitigatedResult`](3_unified.md) | Mitigation result, containing the expectation value and an optional variance. |
| [`ErrorMitigationError`](3_unified.md) | The base class of error mitigation exceptions. |

---

## Quick examples

### 1. Low-level zero-noise extrapolation

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import ExtrapolateMethod, ZNEMitigation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

zne = ZNEMitigation(circuit, [0, 1, 2])
print(zne.fold_levels)    # [0, 1, 2]
print(zne.noise_factors)  # [1, 3, 5]

folded = zne.fold_circuits()
print([len(item.operations) for item in folded])  # [1, 3, 5]


def estimator(run_circuit, observable, shots):
    return (0.5 * len(run_circuit.operations), 0.0)


noisy = zne.run_em_sequence_with_shots(None, hamiltonian, 256, estimator)
print(noisy)  # [0.5, 1.5, 2.5]

mitigated = zne.extrapolate(noisy, ExtrapolateMethod.polynomial(), 1)
print(mitigated)  # 0.0
```

### 2. Low-level virtual distillation

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import VirtualDistillation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = VirtualDistillation(circuit, 2)
copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 2


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

### 3. Unified pipeline

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import (
    ErrorMitigation,
    MitigationMethod,
    VirtualDistillationConfig,
    ProcessArgs,
    RunArgs,
)
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    if observable is None:
        return (2.0, 1.0)
    return (1.5, 0.25)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.virtual_distillation(VirtualDistillationConfig(2)),
)

mitigation.run(
    hamiltonian,
    RunArgs.virtual_distillation(shots_numerator=3, shots_denominator=2),
    estimator,
)
result = mitigation.get_mitigated(ProcessArgs.virtual_distillation())

print(result.expectation)  # 0.75
print(result.variance)     # 0.203125
```

---

## Validation and error handling

Validation in the module falls into two kinds: configuration validation at construction time (fold levels, copies) and consistency validation at execution time (observable qubit count, pipeline call order, and whether parameters match the method).

| Exception | When it occurs |
| --- | --- |
| `ErrorMitigationError` | The configuration or the call order is invalid, for example a negative fold level, fewer than 2 copies, an observable qubit count that does not match the circuit, calling `get_mitigated()` without `run()`, repeating `run()` or `get_mitigated()`, a `RunArgs` / `ProcessArgs` that does not match the actual method, or invalid extrapolation input (see [Zero-noise extrapolation](1_zne.md)). |
| `CircuitError` | Mitigation circuit construction fails, for example calling `fold_circuits()` on a `ZNEMitigation` with a negative fold level. |
| `ValueError` | An observable-related operation fails, for example an error while expanding the `Hamiltonian` by the number of copies. |
| `TypeError` | The `estimator` passed in is not callable. |

`ErrorMitigationError` and `CircuitError` are both subclasses of `cqlib.circuit.CqlibError`. Exceptions raised inside the callback are not replaced; they are re-raised as they are after the mitigation flow returns.
