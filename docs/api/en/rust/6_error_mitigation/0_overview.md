# Error Mitigation

`cqlib_core::error_mitigation`

`cqlib_core::error_mitigation` reduces the systematic bias caused by noise at the expectation value level, providing two implementation paths, zero-noise extrapolation and virtual distillation, and a unified pipeline that converges both onto the same execution order. Mitigation only corrects the estimated result of an observable expectation value and is not quantum error correction; the cost is an increased number of circuit executions, and the execution itself is provided by the caller through an estimator.

## Overview

Error mitigation acts on observable expectation values: the same base circuit is repeatedly constructed and executed at different noise strengths, and the results are then corrected back to the ideal limit by fitting or by a ratio. The constructed circuits (folded circuits, copy-swap circuits) are ultimately executed and evaluated by an estimator provided by the caller; this module only performs construction, orchestration and post-processing.

### Three kinds of entry point

- **Low-level `ZNEMitigation`** (`cqlib_core::error_mitigation::zne_mitigation`): splits zero-noise extrapolation into independently callable steps — `fold_circuits` only constructs folded circuits, `run_em_sequence` and `run_em_sequence_with_shots` execute them one by one and collect expectation values, and the three methods such as `extrapolate` perform extrapolation on their own.
- **Low-level `VirtualDistillation`** (`cqlib_core::error_mitigation::virtual_distillation`): `build_copy_swap_circuit` constructs the copy-swap circuit, `run_denominator_circuit` and `run_numerator_circuit` take the mean and variance of the denominator and the numerator respectively, and `run_vd` gives the ratio result in one step.
- **Unified pipeline `ErrorMitigation`** (`cqlib_core::error_mitigation::unified`): a state machine advanced in the order `new` → `run` → `get_mitigated`. The mitigation method is selected by `MitigationMethod`, and the inputs of `run` and `get_mitigated` are given by `RunArgs` and `ProcessArgs` respectively, both of which must agree with the selected method. The unified pipeline reuses the same folding, copy-swap and extrapolation implementations as the low-level entry points internally, so the two paths produce identical results.

Use the low-level entry points when the intermediate artifacts (folded circuits, numerator and denominator statistics) need to be inspected step by step; use the unified pipeline when only the final expectation value is of interest.

### Estimator

This module does not include a simulation or execution backend; all mitigation methods obtain values through the same callback:

```rust
pub type Estimator<'a> = dyn Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) + 'a;
```

The parameters are, in order, the circuit to execute, the observable to estimate and the number of shots, and the return value is `(expectation value, variance)`. A denominator circuit carries no observable, so the second parameter is `None`; `ZNEMitigation::run_em_sequence` does not specify a number of shots, so the third parameter is `None`. Public methods uniformly accept it as `&Estimator<'_>`, that is, the trait object reference `&dyn Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64)`, and both closures and function items can be passed by reference directly. Only virtual distillation uses the variance in the return value; zero-noise extrapolation takes only the expectation value.

### Mitigation pipeline state

Each instance of `ErrorMitigation` follows a fixed lifecycle: create → `run()` → `get_mitigated()`. `run` and `get_mitigated` can each succeed only once; calling them out of order or repeatedly returns an error. When a call fails, the instance state stays unchanged and the call can be retried after correcting the parameters. Create a new instance when mitigation is needed more than once.

---

## Common entry points

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{
    ErrorMitigation, ExtrapolateMethod, MitigationMethod, ProcessArgs, RunArgs, ZneConfig,
};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let mut mitigation = ErrorMitigation::new(
    circuit,
    MitigationMethod::Zne(ZneConfig {
        fold_levels: vec![0, 1, 2],
    }),
)
.unwrap();

// 对接模拟器或真机后端：返回 (期望值, 方差)
let estimator = |circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, shots: Option<usize>| {
    assert!(hamiltonian.is_some());
    assert_eq!(shots, Some(256));
    (circuit.operations().len() as f64 + 0.5, 0.0)
};

mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &estimator,
    )
    .unwrap();

let mitigated = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: None,
    })
    .unwrap();

assert_eq!(mitigated.expectation, 0.5);
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Error mitigation** | A collection of methods that correct the systematic bias caused by noise at the expectation value level; it is not quantum error correction. |
| **Estimator** | A callback provided by the caller, in the form of `Estimator`; it receives the circuit to execute, an optional observable and an optional number of shots, and returns `(expectation value, variance)`. |
| **Zero-noise extrapolation (ZNE)** | A method that amplifies circuit noise through gate folding to obtain a set of expectation values at different noise strengths, and then extrapolates to the zero-noise limit. |
| **Fold level** | The noise amplification level of zero-noise extrapolation. Level 0 corresponds to the original circuit, and level `l` corresponds to the noise factor `2 * l + 1`. |
| **Noise factor** | The factor by which the noise of a folded circuit is amplified relative to the original circuit; it is always equal to `2 * fold level + 1`. |
| **Gate folding** | A construction that rewrites a circuit as `U -> U (U† U)^level`; folding only the specified gates is called selective folding. |
| **Virtual distillation** | A method that estimates `Tr(O ρ^M) / Tr(ρ^M)` through copies and a copy-swap circuit, where `M` is the number of copies. |
| **Copy** | The number of copies of the base circuit prepared in parallel in virtual distillation; the minimum is 2. |
| **copy-swap circuit** | A circuit that places multiple copies of the base circuit side by side on non-overlapping qubit ranges and inserts a bitwise SWAP between the first copy and each of the others; its width equals the number of copies multiplied by the width of the base circuit. |
| **Numerator circuit / denominator circuit** | The two statistics circuits of virtual distillation. The numerator circuit carries the expanded observable; the denominator circuit carries no observable, and the estimator receives `None`. |
| **Mitigation pipeline** | The sequential flow represented by `ErrorMitigation`: `new` creates it, `run` collects the raw data, and `get_mitigated` performs the method-specific post-processing. |

---

## `cqlib_core::error_mitigation` API overview

### Low-level entry points

| Name | Description |
| --- | --- |
| [`ZNEMitigation`](1_zne.md) | The low-level entry point of zero-noise extrapolation: gate folding, batch evaluation and extrapolation. |
| [`ExtrapolateMethod`](1_zne.md) | The extrapolation method enum, including polynomial and exponential. |
| [`ZneConfig`](1_zne.md) | The fold level configuration of zero-noise extrapolation, used by the unified pipeline and the method enum. |
| [`VirtualDistillation`](2_virtual_distillation.md) | The low-level entry point of virtual distillation: copy-swap circuit construction and numerator and denominator statistics. |
| [`VirtualDistillationConfig`](2_virtual_distillation.md) | The copy count configuration of virtual distillation, used by the unified pipeline and the method enum. |

### Unified pipeline

| Name | Description |
| --- | --- |
| [`ErrorMitigation`](3_unified.md) | The sequential unified entry point, orchestrating construction, execution and post-processing. |
| [`MitigationMethod`](3_unified.md) | The mitigation method enum, carrying the configuration of the corresponding method. |
| [`RunArgs`](3_unified.md) | The argument enum of the execution stage. |
| [`ProcessArgs`](3_unified.md) | The argument enum of the post-processing stage. |
| [`MitigatedResult`](3_unified.md) | The final mitigation result, containing the expectation value and an optional variance. |

### Callbacks and errors

| Name | Description |
| --- | --- |
| [`Estimator`](0_overview.md) | The estimator callback type alias, shared by all mitigation methods. |
| [`ErrorMitigationError`](3_unified.md) | The unified error type of the module. |

---

## Quick examples

### 1. Running virtual distillation through the unified pipeline

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::{
    ErrorMitigation, MitigationMethod, ProcessArgs, RunArgs, VirtualDistillationConfig,
};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let mut mitigation = ErrorMitigation::new(
    Circuit::new(1),
    MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
)
.unwrap();

// 分子线路带可观测量，分母线路不带
mitigation
    .run(
        &hamiltonian,
        RunArgs::VirtualDistillation {
            shots_numerator: 3,
            shots_denominator: 2,
        },
        &|_circuit, hamiltonian_arg, shots| {
            if hamiltonian_arg.is_some() {
                assert_eq!(shots, Some(3));
                (1.5, 0.25)
            } else {
                assert_eq!(shots, Some(2));
                (2.0, 1.0)
            }
        },
    )
    .unwrap();

let mitigated = mitigation
    .get_mitigated(ProcessArgs::VirtualDistillation)
    .unwrap();

assert!((mitigated.expectation - 0.75).abs() < 1e-12);
assert!((mitigated.variance.unwrap() - 0.203125).abs() < 1e-12);
```

### 2. Global folding and selective folding

```rust
use cqlib_core::circuit::gate::{Instruction, StandardGate};
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation};

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.h(q0).unwrap();
circuit.s(q0).unwrap();

let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);
assert_eq!(zne.noise_factors(), &[1, 3, 5]);

// 全局折叠：三个等级各得到一条折叠线路
let global = zne.fold_circuits(None).unwrap();
assert_eq!(global.len(), 3);

// 选择性折叠：只放大 S 门，H 门保持原样
let gate_set = vec![Instruction::Standard(StandardGate::S)];
let selective = zne.fold_circuits(Some(&gate_set)).unwrap();
assert_eq!(selective[1].operations().len(), 4);

// 以噪声因子为自变量外推到零噪声
let extrapolated = zne
    .extrapolate(&[1.5, 3.5, 5.5], ExtrapolateMethod::Polynomial, 1)
    .unwrap();
assert!((extrapolated - 0.5).abs() < 1e-10);
```

### 3. Numerator and denominator of low-level virtual distillation

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::VirtualDistillation;
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();

// 两份基底线路加一对逐比特 SWAP
let copy_swap = vd.build_copy_swap_circuit().unwrap();
assert_eq!(copy_swap.width(), 2);

let (mu_vd, var_vd) = vd
    .run_vd(&hamiltonian, 3, 2, &|_circuit, hamiltonian_arg, shots| {
        if hamiltonian_arg.is_some() {
            assert_eq!(shots, Some(3));
            (1.5, 0.25)
        } else {
            assert_eq!(shots, Some(2));
            (2.0, 1.0)
        }
    })
    .unwrap();

assert!((mu_vd - 0.75).abs() < 1e-12);
assert!((var_vd - 0.203125).abs() < 1e-12);
```

---

## Validation and error handling

Validation during mitigation happens in two places: configuration-level validation occurs at the creation stage, and input-level validation occurs at the folding, execution and extrapolation stages, and both are returned through `ErrorMitigationError`. When a fold level is negative, the low-level folding entry point returns the circuit error `CircuitError::InvalidControlOperation`, whereas the creation stage of the unified pipeline rejects that configuration outright.

| Error | When it occurs |
| --- | --- |
| `ErrorMitigationError::InvalidCopies` | The number of copies of virtual distillation is less than 2. |
| `ErrorMitigationError::InvalidFoldLevel` | A fold level of zero-noise extrapolation is negative; rejected in `ErrorMitigation::new`. |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | The number of qubits of the Hamiltonian is inconsistent with the width of the base circuit; see [ZNE](1_zne.md) and [Virtual distillation](2_virtual_distillation.md). |
| `ErrorMitigationError::ZeroDenominatorMean` | The mean of the denominator circuit of virtual distillation is zero, so the ratio cannot be computed. |
| `ErrorMitigationError::RunRequiredBeforeMitigation` / `AlreadyRun` / `AlreadyMitigated` | The call order of the mitigation pipeline is wrong, or a call was repeated. |
| `ErrorMitigationError::RunArgsMethodMismatch` / `ProcessArgsMethodMismatch` | The execution arguments or post-processing arguments are inconsistent with `MitigationMethod`. |
| `ErrorMitigationError::EmptyNoisyResults` / `NoisyResultsLengthMismatch` / `InvalidPolynomialDegree` / `NonPositiveNoisyResults` / `SingularExponentialFit` / `SingularPolynomialFit` | Invalid extrapolation input or a failed fit. |
| `ErrorMitigationError::Circuit` / `Qis` | Errors passed through from circuit operations or quantum information computation. |

For the complete definition of all variants, see [Unified pipeline](3_unified.md).

---

## Next steps

- [Zero-noise extrapolation](1_zne.md): folding semantics, noise factors and extrapolation methods.
- [Virtual distillation](2_virtual_distillation.md): copy-swap circuits, numerator and denominator statistics.
- [Unified pipeline](3_unified.md): the `ErrorMitigation` state machine, argument enums and error types.
