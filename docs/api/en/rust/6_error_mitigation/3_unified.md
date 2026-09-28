# Unified Pipeline

`cqlib_core::error_mitigation::unified`

The unified pipeline converges zero-noise extrapolation and virtual distillation onto the same sequential flow: create an instance, execute and take values, and produce the result by post-processing. This page covers the unified pipeline itself, the three argument enums, the result object and the unified error type of the module.

## Import

```rust
use cqlib_core::error_mitigation::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigatedResult, MitigationMethod,
    ProcessArgs, RunArgs, VirtualDistillationConfig, ZneConfig,
};
```

---

## ErrorMitigation

The recommended default entry point. It stores the base circuit, the selected mitigation method and the pipeline state, and splits "construct the mitigation circuits and take values" and "compute the mitigated result from the raw data" into the two steps `run` and `get_mitigated`.

```rust
pub struct ErrorMitigation {
    // 字段私有：基底线路、缓解方法与流水线状态
}
```

Methods:

- `fn new(circuit: Circuit, method: MitigationMethod) -> Result<Self, ErrorMitigationError>`: creates the pipeline and validates the method configuration.
- `fn run(&mut self, hamiltonian: &Hamiltonian, run_args: RunArgs, estimator: &Estimator<'_>) -> Result<(), ErrorMitigationError>`: constructs and executes the circuits required by the method, and keeps the raw data; it does not return the mitigated value directly.
- `fn get_mitigated(&mut self, process_args: ProcessArgs) -> Result<MitigatedResult, ErrorMitigationError>`: computes the final result from the kept raw data.

The unified pipeline provides no interface for reading the raw data and no interface for modifying the selected method; both are kept only internally. It implements `Debug` and `Clone`.

### State machine

The lifecycle of each instance is fixed as create → `run()` → `get_mitigated()`, and each of the two steps can succeed only once:

| Current state | Call | Result |
| --- | --- | --- |
| Created | `run` | Executes and moves to running. |
| Created | `get_mitigated` | `ErrorMitigationError::RunRequiredBeforeMitigation`. |
| Running | `run` | `ErrorMitigationError::AlreadyRun`. |
| Running | `get_mitigated` with arguments matching the method | Returns `MitigatedResult` and moves to completed. |
| Running | `get_mitigated` with arguments not matching the method | `ErrorMitigationError::ProcessArgsMethodMismatch`. |
| Completed | `run` or `get_mitigated` | `ErrorMitigationError::AlreadyMitigated`. |

When a call fails the state keeps its original value (except the completed state, which accepts no further calls), so after an argument mismatch or a failed input validation the arguments can be corrected and the call retried. Create a new instance to run a second mitigation flow.

### `new(circuit: Circuit, method: MitigationMethod) -> Result<Self, ErrorMitigationError>`

Parameters:

- `circuit`: the base circuit; all derived circuits in the `run` stage are constructed from it.
- `method`: the mitigation method and its configuration.

Returns:

- `ErrorMitigation`: with the state created.

Raises:

- `ErrorMitigationError::InvalidFoldLevel`: a fold level of `MitigationMethod::Zne` is negative.
- `ErrorMitigationError::InvalidCopies`: the number of copies of `MitigationMethod::VirtualDistillation` is less than 2.

### `run(&mut self, hamiltonian: &Hamiltonian, run_args: RunArgs, estimator: &Estimator<'_>) -> Result<(), ErrorMitigationError>`

Constructs the mitigation circuits according to the selected method and takes values one by one; the results stay inside the instance.

Parameters:

- `hamiltonian`: the observable to estimate; its number of qubits must equal the base circuit width.
- `run_args`: the execution arguments; the variant must match `MitigationMethod`.
- `estimator`: the estimator callback; for the form see [Overview](0_overview.md).

Execution details differ by method:

- Zero-noise extrapolation: constructs the folded circuits according to the configured fold levels, calls the estimator one by one with the observable parameter `Some(hamiltonian)` and the number of shots `RunArgs::Zne::shots`, and takes only the expectation value from the return value. The folded circuits, the expectation values, the noise factors and the execution arguments are kept together.
- Virtual distillation: constructs the copy-swap circuit and expands the observable to the width of that circuit; the estimator of the numerator circuit receives `Some(expanded observable)` and `Some(shots_numerator)`, and the denominator circuit receives `None` and `Some(shots_denominator)`. The means and variances of the numerator and the denominator are kept together.

Returns:

- `Result<(), ErrorMitigationError>`: success only means the raw data is in place; the mitigated value is given by `get_mitigated`.

Raises:

- `ErrorMitigationError::RunArgsMethodMismatch`: the variant of `run_args` is inconsistent with the configured method.
- `ErrorMitigationError::HamiltonianQubitCountMismatch`: the number of qubits of the Hamiltonian is inconsistent with the base circuit width.
- `ErrorMitigationError::AlreadyRun` / `ErrorMitigationError::AlreadyMitigated`: the current state does not allow another execution.
- `ErrorMitigationError::Circuit`: the circuit error passed through when folding, decomposition or circuit construction fails.

### `get_mitigated(&mut self, process_args: ProcessArgs) -> Result<MitigatedResult, ErrorMitigationError>`

Performs the method-specific post-processing on the kept raw data.

Parameters:

- `process_args`: the post-processing arguments; the variant must match `MitigationMethod`.

Post-processing details differ by method:

- Zero-noise extrapolation: the extrapolation mode is selected by `ProcessArgs::Zne::method`. In the `Polynomial` branch, when `degree` is `None` the value `min(number of data points - 1, 1)` is taken, where the number of data points equals the number of fold levels, that is, a first-degree polynomial extrapolation by default; the `Exponential` branch does not use `degree`. The `variance` in the result is `None`.
- Virtual distillation: combines the ratio and the variance from the means and variances of the numerator and the denominator; the `variance` in the result is `Some`.

Returns:

- `MitigatedResult`: the mitigated expectation value and an optional variance.

Raises:

- `ErrorMitigationError::RunRequiredBeforeMitigation`: `run` has not been called.
- `ErrorMitigationError::ProcessArgsMethodMismatch`: the variant of `process_args` is inconsistent with the configured method.
- `ErrorMitigationError::AlreadyMitigated`: a mitigated result has already been taken once.
- `ErrorMitigationError::ZeroDenominatorMean`, `EmptyNoisyResults`, `NoisyResultsLengthMismatch`, `InvalidPolynomialDegree`, `NonPositiveNoisyResults`, `SingularExponentialFit`, `SingularPolynomialFit`: the extrapolation or ratio errors passed through in the post-processing stage; see [Zero-noise extrapolation](1_zne.md) and [Virtual distillation](2_virtual_distillation.md).

### Example

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigatedResult, MitigationMethod,
    ProcessArgs, RunArgs, ZneConfig,
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

mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &|circuit, hamiltonian_arg, shots| {
            assert!(hamiltonian_arg.is_some());
            assert_eq!(shots, Some(256));
            (circuit.operations().len() as f64 + 0.5, 0.0)
        },
    )
    .unwrap();

// run 只能成功调用一次
let rerun_err = mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &|_circuit, _hamiltonian, _shots| (0.0, 0.0),
    )
    .unwrap_err();
assert!(matches!(rerun_err, ErrorMitigationError::AlreadyRun));

// degree 为 None 时按 min(数据点数 - 1, 1) 取值，此处数据点数为 3
let mitigated = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: None,
    })
    .unwrap();

assert_eq!(
    mitigated,
    MitigatedResult {
        expectation: 0.5,
        variance: None,
    }
);

// get_mitigated 也只能成功调用一次
let second_err = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: Some(1),
    })
    .unwrap_err();
assert!(matches!(second_err, ErrorMitigationError::AlreadyMitigated));
```

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

---

## MitigationMethod

The mitigation method enum, carrying the configuration of the corresponding method.

```rust
pub enum MitigationMethod {
    Zne(ZneConfig),
    VirtualDistillation(VirtualDistillationConfig),
}
```

- `Zne(ZneConfig)`: selects zero-noise extrapolation and configures the fold levels.
- `VirtualDistillation(VirtualDistillationConfig)`: selects virtual distillation and configures the number of copies.

Implements `Debug`, `Clone`, `PartialEq`, `Eq`. The method and the two kinds of argument must match:

| Method | Execution arguments | Post-processing arguments |
| --- | --- | --- |
| `MitigationMethod::Zne` | `RunArgs::Zne` | `ProcessArgs::Zne` |
| `MitigationMethod::VirtualDistillation` | `RunArgs::VirtualDistillation` | `ProcessArgs::VirtualDistillation` |

For the configuration structs see [Zero-noise extrapolation](1_zne.md) and [Virtual distillation](2_virtual_distillation.md).

---

## RunArgs

The argument enum of the execution stage.

```rust
pub enum RunArgs {
    Zne {
        gate_set: Option<Vec<Instruction>>,
        shots: Option<usize>,
    },
    VirtualDistillation {
        shots_numerator: usize,
        shots_denominator: usize,
    },
}
```

- `Zne::gate_set`: the gate set of selective folding; `None` means global folding, and matching is performed by operation name.
- `Zne::shots`: the number of shots passed through to the estimator; `None` means it is not specified.
- `VirtualDistillation::shots_numerator`: the number of shots of the numerator circuit.
- `VirtualDistillation::shots_denominator`: the number of shots of the denominator circuit.

Implements `Debug`, `Clone`, `PartialEq`, `Eq`, where `PartialEq` is a custom implementation: the `Zne` branch compares `shots` and the name of each operation in `gate_set`, requiring the same number of elements and the same name one by one, and does not compare the parameters carried by the operations or their circuit definitions, so different operation instances with the same name are judged equal; the `VirtualDistillation` branch compares the two shot numbers. Variants that differ are judged unequal.

---

## ProcessArgs

The argument enum of the post-processing stage.

```rust
pub enum ProcessArgs {
    Zne {
        method: ExtrapolateMethod,
        degree: Option<usize>,
    },
    VirtualDistillation,
}
```

- `Zne::method`: the extrapolation method.
- `Zne::degree`: the polynomial degree. In the `ExtrapolateMethod::Polynomial` branch, `None` takes `min(number of data points - 1, 1)`, where the number of data points equals the number of fold levels; the `ExtrapolateMethod::Exponential` branch ignores this field.
- `VirtualDistillation`: no additional fields.

Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`.

---

## MitigatedResult

The final mitigation result.

```rust
pub struct MitigatedResult {
    pub expectation: f64,
    pub variance: Option<f64>,
}
```

Fields:

- `expectation`: the mitigated expectation value.
- `variance`: the variance of the mitigation result, given only by virtual distillation; `None` for zero-noise extrapolation.

Implements `Debug`, `Clone`, `PartialEq`.

---

## ErrorMitigationError

The unified error type of the module; configuration validation, input validation, pipeline state and extrapolation fitting errors are all returned through it.

```rust
pub enum ErrorMitigationError {
    Circuit(CircuitError),
    Qis(QisError),
    InvalidCopies(usize),
    HamiltonianQubitCountMismatch { expected: usize, actual: usize },
    ZeroDenominatorMean,
    InvalidFoldLevel(i32),
    RunRequiredBeforeMitigation,
    AlreadyRun,
    AlreadyMitigated,
    RunArgsMethodMismatch,
    ProcessArgsMethodMismatch,
    EmptyNoisyResults,
    NoisyResultsLengthMismatch { expected: usize, actual: usize },
    InvalidPolynomialDegree { degree: usize, num_points: usize },
    NonPositiveNoisyResults,
    SingularExponentialFit,
    SingularPolynomialFit,
}
```

The first two variants pass through errors from the circuit module and the quantum information module, and the remaining variants give concrete values. Implements `Debug` and `Display`.

| Error | When it occurs |
| --- | --- |
| `Circuit(CircuitError)` | Folding, circuit decomposition or circuit construction fails. |
| `Qis(QisError)` | A computation at the quantum information level fails. |
| `InvalidCopies(usize)` | The number of copies of virtual distillation is less than 2: `VirtualDistillation::new`, `VirtualDistillation::set_copies` and `ErrorMitigation::new` all check this. |
| `HamiltonianQubitCountMismatch { expected, actual }` | The number of qubits of the Hamiltonian is inconsistent with the base circuit width; `expected` is the circuit width and `actual` is the number of qubits of the Hamiltonian. |
| `ZeroDenominatorMean` | The mean of the denominator circuit of virtual distillation is zero. |
| `InvalidFoldLevel(i32)` | `ErrorMitigation::new` receives a negative fold level. |
| `RunRequiredBeforeMitigation` | `get_mitigated` is called without `run` having been called. |
| `AlreadyRun` | `run` is called a second time on the same instance. |
| `AlreadyMitigated` | An instance that has completed mitigation calls `run` or `get_mitigated` again. |
| `RunArgsMethodMismatch` | The argument variant of `run` is inconsistent with the configured method. |
| `ProcessArgsMethodMismatch` | The argument variant of `get_mitigated` is inconsistent with the configured method. |
| `EmptyNoisyResults` | The result sequence received by extrapolation is empty, for example when the fold levels are an empty vector. |
| `NoisyResultsLengthMismatch { expected, actual }` | The length of the extrapolation result sequence does not equal the number of noise factors. |
| `InvalidPolynomialDegree { degree, num_points }` | The polynomial degree is not less than the number of data points. |
| `NonPositiveNoisyResults` | Exponential extrapolation receives a non-positive result value. |
| `SingularExponentialFit` | The linear regression of exponential extrapolation degenerates. |
| `SingularPolynomialFit` | The normal equation matrix of polynomial extrapolation is singular. |
