# NoiseModel / Noise Channels

This page covers the noise modeling API in `cqlib_core::device`:

- `SingleQubitNoise`
- `TwoQubitNoise`
- `ReadoutError`
- `OperationKey`
- `NoiseModel`
- `NoiseError`

## Import

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::noise::NoiseError;
use cqlib_core::device::{
    NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise,
};
use cqlib_core::qis::Pauli;
```

## NoiseError

Defined in `cqlib_core::device::noise`; not re-exported from the `cqlib_core::device` module root.

Common error variants:

- `InvalidProbability { value, context }`
- `QubitCollision { qubits }`
- `InconsistentArity { expected, actual }`
- `Internal(String)`

## SingleQubitNoise

Enum variants:

- `BitFlip(f64)`
- `PhaseFlip(f64)`
- `Pauli { px, py, pz }`
- `Depolarizing(f64)`
- `AmplitudeDamping(f64)`
- `PhaseDamping(f64)`

Methods:

- `is_valid(&self) -> bool`
- `to_kraus(&self) -> Vec<Array2<Complex64>>`

## TwoQubitNoise

Enum variants:

- `Depolarizing(f64)`
- `Independent { q0_noise: SingleQubitNoise, q1_noise: SingleQubitNoise }`
- `CorrelatedPauli { op_q0: Pauli, op_q1: Pauli, p: f64 }`

Methods:

- `is_valid(&self) -> bool`
- `to_kraus(&self) -> Vec<Array2<Complex64>>`

## ReadoutError

Struct fields:

- `p_0_given_1: f64`
- `p_1_given_0: f64`

Methods:

- `is_valid(&self) -> bool`

## OperationKey

Construction:

- `OperationKey::new_single(gate: StandardGate, q0: Qubit) -> OperationKey`
- `OperationKey::new_double(gate: StandardGate, q0: Qubit, q1: Qubit) -> Result<OperationKey, NoiseError>`
- `OperationKey::new_triple(gate: StandardGate, q0: Qubit, q1: Qubit, q2: Qubit) -> Result<OperationKey, NoiseError>`

Queries:

- `qubits(&self) -> &[usize]`
- `gate(&self) -> &StandardGate`

Notes:

- `OperationKey` implements `Hash`/`Eq` and can be used as a `HashMap` key.

## NoiseModel

Construction:

- `NoiseModel::new() -> NoiseModel`

Writes:

- `add_readout_error(&mut self, qubit: Qubit, error: ReadoutError) -> Result<(), NoiseError>`
- `add_single_qubit_error(&mut self, gate: StandardGate, qubit: Qubit, noise: SingleQubitNoise) -> Result<(), NoiseError>`
- `add_two_qubit_error(&mut self, gate: StandardGate, q0: Qubit, q1: Qubit, noise: TwoQubitNoise) -> Result<(), NoiseError>`

Queries:

- `get_readout_error(&self, key: &Qubit) -> Option<&ReadoutError>`
- `get_single_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<SingleQubitNoise>>`
- `get_two_qubit_errors(&self, key: &OperationKey) -> Option<&Vec<TwoQubitNoise>>`

Notes:

- Multiple channels can be registered for the same gate and qubit combination, and a query returns the list; an unregistered combination returns `None`.
- Two identical qubits for a two-qubit gate make `add_two_qubit_error` return `NoiseError::QubitCollision`.

## Example

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise};
use cqlib_core::qis::Pauli;

let mut nm = NoiseModel::new();

nm.add_readout_error(
    Qubit::new(0),
    ReadoutError {
        p_0_given_1: 0.02,
        p_1_given_0: 0.01,
    },
)
.unwrap();

nm.add_single_qubit_error(
    StandardGate::X,
    Qubit::new(0),
    SingleQubitNoise::BitFlip(0.005),
)
.unwrap();
nm.add_single_qubit_error(
    StandardGate::X,
    Qubit::new(0),
    SingleQubitNoise::PhaseFlip(0.004),
)
.unwrap();

nm.add_two_qubit_error(
    StandardGate::CX,
    Qubit::new(0),
    Qubit::new(1),
    TwoQubitNoise::Depolarizing(0.02),
)
.unwrap();
nm.add_two_qubit_error(
    StandardGate::CX,
    Qubit::new(0),
    Qubit::new(1),
    TwoQubitNoise::CorrelatedPauli {
        op_q0: Pauli::X,
        op_q1: Pauli::Z,
        p: 0.05,
    },
)
.unwrap();

let skey = OperationKey::new_single(StandardGate::X, Qubit::new(0));
let tkey = OperationKey::new_double(StandardGate::CX, Qubit::new(0), Qubit::new(1)).unwrap();

assert_eq!(nm.get_single_qubit_errors(&skey).unwrap().len(), 2);
assert_eq!(nm.get_two_qubit_errors(&tkey).unwrap().len(), 2);
assert_eq!(nm.get_readout_error(&Qubit::new(0)).unwrap().p_0_given_1, 0.02);
assert!(nm.get_readout_error(&Qubit::new(9)).is_none());
```
