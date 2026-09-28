# Statevector

`cqlib_core::qis::state`

This page covers the pure state simulator `Statevector`: construction, amplitude access, entry points for interfacing with circuits, the various gate operations, the measurement and sampling interfaces, and expectation value computation.

## Import

```rust
use cqlib_core::qis::Statevector;
```

---

## Statevector

Pure state simulator, representing a quantum state as `2^num_qubits` complex amplitudes.

```rust
pub struct Statevector {
    pub num_qubits: usize,
}
```

Fields:

- `num_qubits` (`usize`): the number of qubits.

The amplitudes are stored in a private aligned buffer of length `2^num_qubits`; component `i` is the coefficient of the computational basis state `|i>`, and qubit 0 corresponds to the least significant bit. The amplitudes are accessed through `data()` and `data_mut()`. Implements `Debug` and `Clone`.

Methods:

### Construction and amplitude access

- `fn new(num_qubits: usize) -> Self`: construct the `|0...0>` state with the first component set to 1 and the rest 0.
- `fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError>`: construct from the given amplitudes; the length must equal `2^num_qubits` and the state must already be normalized.
- `fn data(&self) -> &[Complex64]`: read all amplitudes as a shared slice.
- `fn data_mut(&mut self) -> &mut [Complex64]`: read all amplitudes as a mutable slice; modification may break normalization.
- `fn probabilities(&self) -> Vec<f64>`: return the measurement probability distribution over all computational basis states, that is the squared modulus of each amplitude.

### Interfacing with circuits

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`: execute the circuit and return the evolved state; the input circuit is not modified.
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`: apply the circuit in place to the current state; the number of qubits of the circuit must match that of the state.

Both entry points decompose the circuit first and then execute its instructions one by one. Standard gates, controlled gates, multi-controlled gates, unitary gates with a matrix representation and `Reset` are supported; barrier and delay instructions are ignored, and measurement declarations produced by `Circuit::measure*` do not participate in the state evolution.

### General gate entry points

- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`: dispatch to the corresponding dedicated implementation by the standard gate enum; the number of `qubits` and the number of `params` must each equal the value required by `gate`.
- `fn apply_single_qubit_gate(&mut self, qubit: usize, matrix: [[Complex64; 2]; 2]) -> Result<(), QisError>`: apply an arbitrary 2×2 unitary matrix.
- `fn apply_two_qubit_gate(&mut self, q0: usize, q1: usize, matrix: [[Complex64; 4]; 4]) -> Result<(), QisError>`: apply an arbitrary 4×4 unitary matrix with basis order `|00>`, `|01>`, `|10>`, `|11>`, where `q0` is the high-order qubit.
- `fn apply_unitary_gate(&mut self, qubits: &[usize], matrix: &ndarray::Array2<Complex64>) -> Result<(), QisError>`: apply an arbitrary `2^n × 2^n` unitary matrix, with `qubits` giving the qubits the matrix acts on.
- `fn apply_pauli_rotation(&mut self, pauli: &PauliString, theta: f64) -> Result<(), QisError>`: apply `exp(-i·θ/2·P)` in place, where `P` is a Hermitian Pauli string; this method does not decompose the exponential into a sequence of elementary gates.

### Single-qubit gates

- `fn apply_x(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-X.
- `fn apply_y(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-Y.
- `fn apply_z(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-Z.
- `fn apply_h(&mut self, qubit: usize) -> Result<(), QisError>`: Hadamard gate.
- `fn apply_s(&mut self, qubit: usize) -> Result<(), QisError>`: S gate.
- `fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError>`: S† gate.
- `fn apply_t(&mut self, qubit: usize) -> Result<(), QisError>`: T gate.
- `fn apply_tdg(&mut self, qubit: usize) -> Result<(), QisError>`: T† gate.
- `fn apply_u(&mut self, qubit: usize, theta: f64, phi: f64, lambda: f64) -> Result<(), QisError>`: general single-qubit gate `U(θ, φ, λ)`.
- `fn apply_phase(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: phase gate `P(θ)`.
- `fn apply_rx(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: rotation around the X axis by `θ`.
- `fn apply_ry(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: rotation around the Y axis by `θ`.
- `fn apply_rz(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: rotation around the Z axis by `θ`.
- `fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError>`: `Rx(π/2)`.
- `fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError>`: `Rx(-π/2)`.
- `fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError>`: `Ry(π/2)`.
- `fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError>`: `Ry(-π/2)`.
- `fn apply_xy2p(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: `XY2P(θ)`.
- `fn apply_xy2m(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: `XY2M(θ)`.
- `fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: `XY(θ)`, a different gate from `XY2P` and `XY2M`.
- `fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError>`: rotate by `theta` around the axis in the XY plane at azimuth `phi`.
- `fn apply_gphase(&mut self, phi: f64) -> Result<(), QisError>`: multiply the whole state by the global phase `e^(iφ)`.

### Two-qubit gates

- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: swap two qubits.
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled X gate.
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled Y gate.
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: controlled Z gate; the two parameters are symmetric.
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rx(θ)`.
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Ry(θ)`.
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rz(θ)`.
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·X⊗X)`.
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Y⊗Y)`.
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗Z)`.
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗X)`.
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`: fermionic simulation gate, where `theta` is the iSWAP angle and `phi` is the controlled phase angle.

### Three-qubit gates

- `fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError>`: Toffoli gate; flips the target qubit when both control qubits are `|1>`.

### Measurement, reset and sampling

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`: measure the given qubit in the Z basis and collapse the state; returns `false` for outcome `|0>` and `true` for outcome `|1>`; after measurement the state is renormalized within the corresponding subspace. This operation is destructive.
- `fn measure_with_rng(&mut self, qubit: usize, rng: &mut impl Rng) -> Result<bool, QisError>`: measure using the random number generator provided by the caller, for reproducibility.
- `fn measure_all(&mut self) -> Outcome`: measure one by one in the order `0..num_qubits` and return a bit-packed `Outcome`.
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`: reset the given qubit to `|0>`, implemented as a Z-basis measurement followed by an X correction when the outcome is `|1>`.
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`: sample several independent measurement outcomes in parallel without modifying `self`.
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`: sample according to the output convention specified by the circuit `Measurement`; `measurement.qubits()[i]` corresponds to bit `i` of each `Outcome`.
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`: return the marginal probability distribution selected by `Measurement`; this method does not execute a circuit and uses `Measurement` only as an output convention.

### Expectation value

- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`: compute `⟨ψ|H|ψ⟩`.

---

## Example

### 1. Prepare a Bell state

```rust
use cqlib_core::qis::Statevector;
use num_complex::Complex64;
use std::f64::consts::FRAC_1_SQRT_2;

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

// (|00⟩ + |11⟩)/√2
assert_eq!(sv.num_qubits, 2);
assert!((sv.data()[0] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
assert!((sv.data()[3] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);

let probs = sv.probabilities();
assert!((probs[0] - 0.5).abs() < 1e-10);
assert!((probs[3] - 0.5).abs() < 1e-10);
```

### 2. Construct from a circuit and apply a circuit in place

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::Statevector;

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let sv = Statevector::from_circuit(&circuit).unwrap();
assert_eq!(sv.num_qubits, 2);

let mut sv2 = Statevector::new(2);
sv2.apply_circuit(&circuit).unwrap();
assert!(sv2.data().iter().zip(sv.data()).all(|(a, b)| (a - b).norm() < 1e-10));
```

### 3. Apply a custom unitary gate

```rust
use cqlib_core::qis::Statevector;
use num_complex::Complex64;
use std::f64::consts::FRAC_1_SQRT_2;

// H = [[1/√2, 1/√2], [1/√2, -1/√2]]
let h_matrix = [
    [Complex64::new(FRAC_1_SQRT_2, 0.0), Complex64::new(FRAC_1_SQRT_2, 0.0)],
    [Complex64::new(FRAC_1_SQRT_2, 0.0), Complex64::new(-FRAC_1_SQRT_2, 0.0)],
];

let mut sv = Statevector::new(1);
sv.apply_single_qubit_gate(0, h_matrix).unwrap();

assert!((sv.data()[0] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
assert!((sv.data()[1] - Complex64::new(FRAC_1_SQRT_2, 0.0)).norm() < 1e-10);
```

### 4. Apply a multi-qubit unitary gate

```rust
use cqlib_core::qis::Statevector;
use ndarray::Array2;

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
let before: Vec<_> = sv.data().to_vec();

// 4×4 单位矩阵作用在比特 [0, 1] 上
let identity = Array2::eye(4);
sv.apply_unitary_gate(&[0, 1], &identity).unwrap();

assert!(sv.data().iter().zip(before.iter()).all(|(a, b)| (a - b).norm() < 1e-10));
```

### 5. Pauli string rotation

```rust
use cqlib_core::qis::{Pauli, PauliString, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();

let mut pauli = PauliString::new(2);
pauli.set_pauli(0, Pauli::X);
pauli.set_pauli(1, Pauli::Z);

sv.apply_pauli_rotation(&pauli, 0.413).unwrap();

let norm: f64 = sv.data().iter().map(|amp| amp.norm_sqr()).sum();
assert!((norm - 1.0).abs() < 1e-10);
```

### 6. Measurement and sampling

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::Statevector;

let mut sv = Statevector::new(4);
sv.apply_x(0).unwrap();
sv.apply_x(2).unwrap();

let outcome = sv.measure_all();
assert!(outcome.is_one(0));
assert!(!outcome.is_one(1));
assert!(outcome.is_one(2));
assert!(!outcome.is_one(3));

// 按 Measurement 指定的比特顺序采样
let mut circuit = Circuit::new(2);
circuit.x(Qubit::new(0)).unwrap();
let out = circuit.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

let sv = Statevector::from_circuit(&circuit).unwrap();
let result = sv.sample(&out, 16).unwrap();
assert_eq!(result.shots(), 16);
```

### 7. Compute an expectation value

```rust
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString, Statevector};

let sv = Statevector::new(1);

let mut ps = PauliString::new(1);
ps.set_pauli(0, Pauli::Z);
let h = Hamiltonian::from_pauli(ps);

let exp = sv.expectation(&h).unwrap();
assert!((exp - 1.0).abs() < 1e-10);
```

---

## Validation and error handling

Failures in construction, gate application, measurement and expectation value computation are all returned through `QisError`, and no panics are used.

| Error | When it occurs |
| --- | --- |
| `QisError::InvalidStateDimension` | The number of amplitudes of `from_state` does not equal `2^num_qubits`; the number of circuit qubits of `apply_circuit` does not match that of the state. |
| `QisError::NotNormalized` | The amplitudes passed to `from_state` are not normalized. |
| `QisError::IndexOutOfBounds` | The qubit index is out of range. |
| `QisError::InvalidParameterValue` | A gate is applied to a zero-qubit system; a two-qubit gate is given the same qubit twice; the qubit list of `apply_unitary_gate` contains duplicates; the number of qubits or parameters of `apply_standard_gate` does not match the gate. |
| `QisError::UnsupportedOperation` | The circuit contains an operation the current entry point does not support, for example classical control flow. |
| `QisError::CircuitError` | A gate in the circuit has no matrix representation, contains an unresolved symbolic parameter, or references a qubit that does not exist. |
| `QisError::QubitMismatch` | The number of qubits the observable of `expectation` acts on does not match that of the state. |

Unnormalized amplitudes, a gate operation on a zero-qubit system, and a two-qubit gate formed from the same qubit are all input errors on the caller's side and can usually be avoided before running.
