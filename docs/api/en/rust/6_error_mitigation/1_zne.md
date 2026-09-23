# Zero-Noise Extrapolation

`cqlib_core::error_mitigation::zne_mitigation`

Zero-noise extrapolation rewrites the base circuit through gate folding into a set of circuits with increasing noise strength, takes an expectation value on each folded circuit, and then extrapolates to the zero-noise limit by fitting. This page covers the public API of folding, batch evaluation and extrapolation.

## Import

```rust
use cqlib_core::circuit::gate::Instruction;
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation, ZneConfig};
```

---

## ZNEMitigation

The low-level entry point of zero-noise extrapolation. It holds the base circuit and the fold levels, and splits folding, evaluation and extrapolation into three independently callable stages.

```rust
pub struct ZNEMitigation {
    circuit: Circuit,
    fold_levels: Vec<i32>,
    noise_factors: Vec<i32>,
}
```

Fields are private, and are read through methods:

- `fn circuit(&self) -> &Circuit`: the original (unfolded) base circuit.
- `fn fold_levels(&self) -> &[i32]`: the configured fold levels, in the same order as at construction.
- `fn noise_factors(&self) -> &[i32]`: the noise factors corresponding one-to-one to the fold levels; always `2 * level + 1`.

Other methods:

- `fn new(circuit: Circuit, fold_levels: Vec<i32>) -> Self`: the creation entry point; it takes ownership of the circuit and converts the levels into noise factors. The construction stage does not validate whether the levels are non-negative.
- `fn fold_circuits(&self, gate_set: Option<&[Instruction]>) -> Result<Vec<Circuit>, CircuitError>`: constructs one folded circuit for each fold level.
- `fn run_em_sequence(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`: folds level by level and calls the estimator for evaluation; the estimator receives `None` as the number of shots.
- `fn run_em_sequence_with_shots(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, shots: Option<usize>, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`: the same as above, and passes the number of shots through to the estimator.
- `fn extrapolate(&self, noisy_results: &[f64], method: ExtrapolateMethod, degree: usize) -> Result<f64, ErrorMitigationError>`: dispatches to the two extrapolation methods below according to the method.
- `fn poly_extrapolate(&self, noisy_results: &[f64], degree: usize) -> Result<f64, ErrorMitigationError>`: polynomial extrapolation.
- `fn exp_extrapolate(&self, noisy_results: &[f64]) -> Result<f64, ErrorMitigationError>`: exponential extrapolation.

`ZNEMitigation` has no builder methods and no `Default` implementation: an instance can be created only by `new`, the fold levels cannot be modified through the read interface, and changing the levels requires creating a new instance. It implements `Debug` and `Clone`.

### `fold_circuits(&self, gate_set: Option<&[Instruction]>) -> Result<Vec<Circuit>, CircuitError>`

Constructs a folded circuit per level; the length of the returned vector equals the number of fold levels, in the same order as `fold_levels`.

Parameters:

- `gate_set`: the optional gate set that determines the folding scope. Passing `None` means global folding, rewriting the whole circuit as `U -> U (U† U)^level`, where the inverse circuit is obtained by inverting the base circuit as a whole; passing `Some` means selective folding, which checks the name of each operation, and only operations matching any name in the set are amplified while the remaining operations are kept as they are.

Returns:

- `Vec<Circuit>`: the folded circuit corresponding to each fold level.

Raises:

- `CircuitError::InvalidControlOperation`: a fold level is negative.
- `CircuitError`: circuit-level errors such as a failure to invert the base circuit.

At level 0 the copy of the base circuit is returned directly, with nothing appended. Selective folding matches by operation name; the name is given by `Instruction::name()` and is independent of the parameters carried by the operation or of its own circuit definition.

### `run_em_sequence_with_shots(&self, gate_set: Option<&[Instruction]>, hamiltonian: &Hamiltonian, shots: Option<usize>, estimator: &Estimator<'_>) -> Result<Vec<f64>, ErrorMitigationError>`

Folds level by level and calls the estimator, returning the expectation value of each folded circuit. `run_em_sequence` is its shot-free version, always passing `None` to the estimator.

Parameters:

- `gate_set`: the same as in `fold_circuits`; controls the folding scope.
- `hamiltonian`: the observable to estimate; its number of qubits must equal the width of the base circuit.
- `shots`: the number of shots of each execution, passed through to the estimator as is.
- `estimator`: the estimator callback, called once per folded circuit, with the second parameter always `Some(hamiltonian)`; for the callback form see [Overview](0_overview.md).

Returns:

- `Vec<f64>`: the expectation value of each folded circuit, in the same order as `fold_levels`. The variance returned by the estimator is discarded at this stage.

Raises:

- `ErrorMitigationError::HamiltonianQubitCountMismatch`: the number of qubits of the Hamiltonian is inconsistent with the width of the base circuit; the error carries both the expected and the actual value.
- `ErrorMitigationError::Circuit`: the circuit error passed through when folding fails.

### Extrapolation

All three extrapolation methods take the noise factor as the independent variable and `noisy_results` as the dependent variable, return the estimate at zero noise (independent variable 0), and require `noisy_results` to be non-empty and of the same length as the number of noise factors.

- `fn poly_extrapolate(&self, noisy_results: &[f64], degree: usize) -> Result<f64, ErrorMitigationError>`: least-squares polynomial fitting; returns the intercept at `x = 0`. `degree` is the polynomial degree and must be less than the number of data points.
- `fn exp_extrapolate(&self, noisy_results: &[f64]) -> Result<f64, ErrorMitigationError>`: models as `y(x) = A * exp(-x / tau)`, performs a linear regression on `ln(y) = ln(A) + m * x` in log space, and returns `A`.
- `fn extrapolate(&self, noisy_results: &[f64], method: ExtrapolateMethod, degree: usize) -> Result<f64, ErrorMitigationError>`: `Polynomial` forwards to `poly_extrapolate` using `degree`; `Exponential` ignores `degree` and forwards to `exp_extrapolate`.

### Example

```rust
use cqlib_core::circuit::gate::{Instruction, StandardGate};
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::ZNEMitigation;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.h(q0).unwrap();
circuit.s(q0).unwrap();

// 折叠等级 [0, 1, 2] 对应噪声因子 [1, 3, 5]
let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);
assert_eq!(zne.fold_levels(), &[0, 1, 2]);
assert_eq!(zne.noise_factors(), &[1, 3, 5]);

// 全局折叠：等级 0 原样返回，等级 1 把整条线路折叠为 U -> U U† U
let global = zne.fold_circuits(None).unwrap();
assert_eq!(global[0].operations().len(), 2);
assert_eq!(global[1].operations().len(), 6);

// 选择性折叠：只放大 S 门，H 门保持原样
let gate_set = vec![Instruction::Standard(StandardGate::S)];
let selective = zne.fold_circuits(Some(&gate_set)).unwrap();
assert_eq!(selective[1].operations().len(), 4);
```

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

// 估计器返回 (期望值, 方差)，零噪声外推只使用期望值
let noisy = zne
    .run_em_sequence_with_shots(
        None,
        &hamiltonian,
        Some(256),
        &|circuit, hamiltonian_arg, shots| {
            assert!(hamiltonian_arg.is_some());
            assert_eq!(shots, Some(256));
            (circuit.operations().len() as f64 + 0.5, 0.0)
        },
    )
    .unwrap();

// 噪声因子为 [1, 3, 5]，实测值满足 y = 0.5 + x
assert_eq!(noisy, vec![1.5, 3.5, 5.5]);

// 多项式外推取 x = 0 处的截距
let mitigated = zne
    .extrapolate(&noisy, ExtrapolateMethod::Polynomial, 1)
    .unwrap();
assert!((mitigated - 0.5).abs() < 1e-10);

// 指数外推：模型为 y = A * exp(-x / tau)，返回 A，同时忽略 degree
let a = 2.5_f64;
let tau = 4.0_f64;
let noisy: Vec<f64> = zne
    .noise_factors()
    .iter()
    .map(|&x| a * (-(x as f64) / tau).exp())
    .collect();
let mitigated = zne
    .extrapolate(&noisy, ExtrapolateMethod::Exponential, 99)
    .unwrap();
assert!((mitigated - a).abs() < 1e-10);
```

---

## ExtrapolateMethod

The extrapolation method enum; determines which fitting path `extrapolate` takes.

```rust
pub enum ExtrapolateMethod {
    Polynomial,
    Exponential,
}
```

- `Polynomial`: polynomial extrapolation. The `degree` parameter of `extrapolate` takes effect in this branch.
- `Exponential`: exponential extrapolation. The `degree` parameter of `extrapolate` is ignored in this branch.

Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.

---

## ZneConfig

The method configuration of zero-noise extrapolation; defined in the submodule `cqlib_core::error_mitigation::unified` where the unified pipeline lives, re-exported from the module root, and carried by `MitigationMethod::Zne`.

```rust
pub struct ZneConfig {
    pub fold_levels: Vec<i32>,
}
```

Fields:

- `fold_levels`: the list of fold levels, which determines the number of folded circuits and the noise factors; level `l` corresponds to the noise factor `2 * l + 1`.

There is no `Default` implementation, so `fold_levels` must be given explicitly. Implements `Debug`, `Clone`, `PartialEq`, `Eq`.

`ErrorMitigation::new` checks the fold levels at the creation stage and returns `ErrorMitigationError::InvalidFoldLevel` if any level is negative; an empty vector of levels passes the creation-stage check, but later extrapolation raises an error because there are no data points.

---

## Errors

Folding, evaluation and extrapolation fall into two kinds of error: folding returns circuit errors, and evaluation and extrapolation return the unified error of the module.

| Error | When it occurs |
| --- | --- |
| `CircuitError::InvalidControlOperation` | `fold_circuits` encounters a negative fold level. |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | The number of qubits of the Hamiltonian received by `run_em_sequence` or `run_em_sequence_with_shots` is inconsistent with the width of the base circuit. |
| `ErrorMitigationError::EmptyNoisyResults` | The result sequence received by extrapolation is empty, for example when the fold levels are an empty vector. |
| `ErrorMitigationError::NoisyResultsLengthMismatch` | The length of the extrapolation result sequence does not equal the number of noise factors. |
| `ErrorMitigationError::InvalidPolynomialDegree` | The polynomial degree is not less than the number of data points. |
| `ErrorMitigationError::NonPositiveNoisyResults` | The results received by exponential extrapolation contain a non-positive value. |
| `ErrorMitigationError::SingularExponentialFit` | The linear regression of exponential extrapolation degenerates. |
| `ErrorMitigationError::SingularPolynomialFit` | The normal equation matrix of polynomial extrapolation is singular. |
| `ErrorMitigationError::Circuit` / `Qis` | Errors passed through from folding or circuit operations. |

For the complete definition of the error type, see [Unified pipeline](3_unified.md).
