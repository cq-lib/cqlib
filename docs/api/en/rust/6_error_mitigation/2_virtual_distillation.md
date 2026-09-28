# Virtual Distillation

`cqlib_core::error_mitigation::virtual_distillation`

Virtual distillation estimates the ratio `Tr(O ρ^M) / Tr(ρ^M)` through multiple copies of the density matrix and a copy-swap circuit, where `M` is the number of copies and `O` is the observable represented by the Hamiltonian. This page covers the public API of copy-swap circuit construction and numerator and denominator statistics.

## Import

```rust
use cqlib_core::error_mitigation::VirtualDistillation;
```

---

## VirtualDistillation

The low-level entry point of virtual distillation. It holds the base circuit and the number of copies, and can construct the copy-swap circuit on its own, run the denominator or the numerator circuit on its own, or obtain the ratio and variance in one step.

```rust
pub struct VirtualDistillation {
    circuit: Circuit,
    copies: usize,
}
```

Fields are private, and are read through methods:

- `fn copies(&self) -> usize`: the currently configured number of copies.

Other methods:

- `fn new(circuit: Circuit, copies: usize) -> Result<Self, ErrorMitigationError>`: the creation entry point; it takes ownership of the circuit. The template circuit can only be passed through `new`, and there is no corresponding read interface.
- `fn set_copies(&mut self, copies: usize) -> Result<(), ErrorMitigationError>`: updates the number of copies. On failure the original value stays unchanged.
- `fn build_copy_swap_circuit(&self) -> Result<Circuit, CircuitError>`: constructs the copy-swap circuit from the base circuit.
- `fn run_denominator_circuit(&self, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), CircuitError>`: runs the denominator circuit and returns `(mean, variance)`.
- `fn run_numerator_circuit(&self, hamiltonian: &Hamiltonian, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`: runs the numerator circuit and returns `(mean, variance)`.
- `fn run_vd(&self, hamiltonian: &Hamiltonian, shots_numerator: usize, shots_denominator: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`: runs the complete flow and returns the mitigated `(expectation value, variance)`.

`VirtualDistillation` has no builder methods and no `Default` implementation: an instance can be created only by `new`, and the number of copies is modified through `set_copies`. It implements `Debug` and `Clone`.

### `new(circuit: Circuit, copies: usize) -> Result<Self, ErrorMitigationError>`

Parameters:

- `circuit`: the base circuit; all derived circuits are constructed from it.
- `copies`: the number of copies; must not be less than 2.

Returns:

- `VirtualDistillation`: usable for constructing circuits and taking values.

Raises:

- `ErrorMitigationError::InvalidCopies`: the number of copies is less than 2; the error carries the number of copies passed in.

### `build_copy_swap_circuit(&self) -> Result<Circuit, CircuitError>`

Constructs the copy-swap circuit, which consists of three parts: first the base circuit is decomposed into gates, then `copies` copies of the base circuit are prepared side by side — the `i`-th copy is shifted as a whole onto the qubit range starting at `i × base width`, and the copies do not overlap — and finally a `SWAP` is inserted bit by bit between the first copy and each of the others.

- The circuit width is `copies × base width`, where the base width is the width of the decomposed circuit.
- The number of operations is `copies` sets of base operations plus `(copies - 1) × base width` `SWAP`s.

Returns:

- `Circuit`: the copy-swap circuit.

Raises:

- `CircuitError`: decomposition of the base circuit fails, or appending an operation to the new circuit fails.

### Numerator and denominator

- `fn run_denominator_circuit(&self, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), CircuitError>`: estimates the denominator `Tr(ρ^M)` on the copy-swap circuit. The observable parameter the estimator receives is `None` and the number of shots is `Some(shots)`; for the callback form see [Overview](0_overview.md).
- `fn run_numerator_circuit(&self, hamiltonian: &Hamiltonian, shots: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`: estimates the numerator `Tr(O ρ^M)` on the copy-swap circuit. The observable is first expanded inside the module to the width of the copy-swap circuit: the original Pauli terms keep their original qubit indices, phase and coefficients unchanged, and `Z` is filled in on the newly added higher-index qubits; the expanded result is passed in through the observable parameter of the estimator, and the number of shots is `Some(shots)`.
- `fn run_vd(&self, hamiltonian: &Hamiltonian, shots_numerator: usize, shots_denominator: usize, estimator: &Estimator<'_>) -> Result<(f64, f64), ErrorMitigationError>`: first validates that the number of qubits of the Hamiltonian equals the width of the base circuit, then executes the numerator circuit and the denominator circuit in turn, and finally combines them into the mitigated result. The numerator and the denominator use their own number of shots.

Raises for `run_vd`:

- `ErrorMitigationError::HamiltonianQubitCountMismatch`: the number of qubits of the Hamiltonian is inconsistent with the width of the base circuit; the error carries both the expected and the actual value.
- `ErrorMitigationError::ZeroDenominatorMean`: the mean of the denominator circuit is zero, so the ratio cannot be computed.
- `ErrorMitigationError::Circuit`: the circuit error passed through when the width of the expanded observable is inconsistent with the width of the copy-swap circuit.

### Ratio and variance

The return value of `run_vd` combines the expectation value as `mu_vd = mu_num / mu_den`; the variance is combined, under a first-order Taylor approximation and the assumption that the numerator and the denominator are independent, as `var_num / mu_den^2 + mu_num^2 * var_den / mu_den^4`. These formulas are used only inside the module; externally the result is obtained only through `run_vd`, and the unified pipeline reuses the same formulas in the post-processing stage.

### Example

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::VirtualDistillation;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

// 两份单比特副本：X 作用于比特 0、X 作用于比特 1，再加一对逐比特 SWAP
let vd = VirtualDistillation::new(circuit, 2).unwrap();
let copy_swap = vd.build_copy_swap_circuit().unwrap();
assert_eq!(copy_swap.width(), 2);
assert_eq!(copy_swap.operations().len(), 3);
```

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::{ErrorMitigationError, VirtualDistillation};

// 副本数通过 set_copies 调整，非法值被拒绝且不改变原值
let mut vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
assert_eq!(vd.copies(), 2);

vd.set_copies(3).unwrap();
assert_eq!(vd.copies(), 3);

let err = vd.set_copies(1).unwrap_err();
assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
assert_eq!(vd.copies(), 3);
```

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::VirtualDistillation;
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

// 分子线路带可观测量，分母线路不带；两者使用各自的 shot 数
let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
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

## VirtualDistillationConfig

The method configuration of virtual distillation; defined in the submodule `cqlib_core::error_mitigation::unified` where the unified pipeline lives, re-exported from the module root, and carried by `MitigationMethod::VirtualDistillation`.

```rust
pub struct VirtualDistillationConfig {
    pub copies: usize,
}
```

Fields:

- `copies`: the number of copies; the minimum is 2.

There is no `Default` implementation, so `copies` must be given explicitly. Implements `Debug`, `Clone`, `PartialEq`, `Eq`.

`ErrorMitigation::new` checks this field at the creation stage and returns `ErrorMitigationError::InvalidCopies` when it is less than 2.

---

## Errors

| Error | When it occurs |
| --- | --- |
| `ErrorMitigationError::InvalidCopies` | The number of copies is less than 2; see `new`, `set_copies` and `ErrorMitigation::new`. |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | The number of qubits of the Hamiltonian received by `run_vd` is inconsistent with the width of the base circuit. |
| `ErrorMitigationError::ZeroDenominatorMean` | The mean of the denominator circuit is zero. |
| `ErrorMitigationError::Circuit` | Carries `CircuitError::QubitCountMismatch`, indicating that the width of the expanded observable is inconsistent with the width of the copy-swap circuit. |
| `CircuitError` | The circuit error returned directly by `build_copy_swap_circuit` and `run_denominator_circuit`. |

For the complete definition of the error type, see [Unified pipeline](3_unified.md).
