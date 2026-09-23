# DensityMatrix

`cqlib_core::qis::state`

This page covers the mixed state simulator `DensityMatrix`: construction, matrix data access, physicality validation, entry points for interfacing with circuits, the various gate operations, quantum channels and reduction, the measurement and sampling interfaces, and expectation value computation.

## Import

```rust
use cqlib_core::qis::DensityMatrix;
```

---

## DensityMatrix

Mixed state simulator, representing a quantum state as a `2^N × 2^N` density matrix. Compared with a pure state described by a single set of complex amplitudes, a density matrix can represent pure states, mixed states and the result of applying a quantum channel.

```rust
pub struct DensityMatrix {
    pub num_qubits: usize,
}
```

Fields:

- `num_qubits` (`usize`): the number of qubits.

The matrix elements are stored in row-major order in a private buffer of length `4^N`, accessed through `data()`. Implements `Debug`, `Clone` and `AddAssign` (`+=`).

Methods:

### Construction and data access

- `fn new(num_qubits: usize) -> Self`: construct the pure state `|0...0><0...0|`.
- `fn maximally_mixed(num_qubits: usize) -> Self`: construct the maximally mixed state `I / 2^N`, where every computational basis state is equally probable and there are no coherence terms.
- `fn zeros(num_qubits: usize) -> Self`: construct an all-zero matrix; it is not a valid physical state (trace 0) and is used as the accumulation starting point for operations such as quantum channels.
- `fn from_state(num_qubits: usize, initial_state: Vec<Complex64>) -> Result<Self, QisError>`: construct by the outer product of pure state amplitudes `ρ = |ψ><ψ|`.
- `fn from_density_matrix_state(num_qubits: usize, dm_state: Vec<Complex64>) -> Result<Self, QisError>`: construct directly from a flattened `2^N × 2^N` matrix, validating Hermiticity, positive semidefiniteness and unit trace.
- `fn data(&self) -> &[Complex64]`: read the flattened matrix elements as a shared slice.
- `fn probabilities(&self) -> Vec<f64>`: return the diagonal entries of the density matrix, that is the measurement probabilities on each computational basis state.

### Physicality validation

- `fn trace(&self) -> Complex64`: return the trace; the trace of a valid physical state equals `1.0`.
- `fn is_hermitian(&self, tol: f64) -> bool`: determine whether `ρ = ρ†` holds.
- `fn is_positive_semidefinite_approx(&self, tol: f64) -> bool`: determine whether all eigenvalues are not less than `-tol`; returns `false` without panicking when NaN or Inf is present.
- `fn validate_physical(&self, tol: f64) -> Result<(), QisError>`: validate Hermiticity, positive semidefiniteness and unit trace in order.

### Interfacing with circuits

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`: execute the circuit and return the evolved density matrix.
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`: apply the circuit in place to the current density matrix; the number of qubits of the circuit must match that of the state.

The two entry points support the same instructions as `Statevector`: standard gates, controlled gates, multi-controlled gates, unitary gates with a matrix representation and `Reset`; barriers and delays are ignored, and measurement declarations produced by `Circuit::measure*` do not participate in the state evolution.

### General gate entry points

- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`: dispatch by the standard gate enum; the number of `qubits` and `params` must each equal the value required by `gate`.
- `fn apply_single_qubit_gate(&mut self, qubit: usize, matrix: [[Complex64; 2]; 2]) -> Result<(), QisError>`: apply an arbitrary 2×2 unitary matrix.
- `fn apply_two_qubit_gate(&mut self, q0: usize, q1: usize, matrix: [[Complex64; 4]; 4]) -> Result<(), QisError>`: apply an arbitrary 4×4 unitary matrix.
- `fn apply_unitary_gate(&mut self, qubits: &[usize], matrix: &ndarray::Array2<Complex64>) -> Result<(), QisError>`: apply an arbitrary `2^n × 2^n` unitary matrix as `ρ → U ρ U†`.

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
- `fn apply_xy(&mut self, qubit: usize, theta: f64) -> Result<(), QisError>`: `XY(θ)`.
- `fn apply_rxy(&mut self, qubit: usize, theta: f64, phi: f64) -> Result<(), QisError>`: rotate around the specified axis in the XY plane.
- `fn apply_gphase(&mut self, phi: f64)`: global phase has no observable effect on a density matrix; this method does not change the state and returns no result.

### Two-qubit and three-qubit gates

- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: swap two qubits.
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled X gate.
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled Y gate.
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: controlled Z gate.
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rx(θ)`.
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Ry(θ)`.
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rz(θ)`.
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·X⊗X)`.
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Y⊗Y)`.
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗Z)`.
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗X)`.
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`: fermionic simulation gate.
- `fn apply_ccx(&mut self, c0: usize, c1: usize, target: usize) -> Result<(), QisError>`: Toffoli gate.

### Quantum channels and reduction

- `fn apply_kraus(&mut self, ops: &[Vec<Complex64>], qs: &[usize]) -> Result<(), QisError>`: apply the quantum channel given by Kraus operators, evolving as `ρ → Σ_k K_k ρ K_k†`; each operator is given as a flattened vector, and the number of qubits is determined by `qs.len()`.
- `fn partial_trace(&self, keep: &[usize]) -> Result<Self, QisError>`: compute the partial trace over the qubits outside `keep`, returning a new density matrix with `keep.len()` qubits.

### Measurement, reset and sampling

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`: measure in the Z basis and collapse the density matrix; returns `true` for outcome `|1>` and `false` for outcome `|0>`; after collapse it is renormalized as `ρ' = Π_b ρ Π_b / Tr(Π_b ρ)`.
- `fn measure_all(&mut self) -> Outcome`: measure one by one in the order `0..num_qubits` and return a bit-packed `Outcome`.
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`: reset the given qubit to `|0>`.
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`: sample several independent measurement outcomes in parallel without modifying `self`.
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`: sample according to the output convention specified by the circuit `Measurement`.
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`: return the marginal probability distribution selected by `Measurement`.

### Expectation value

- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`: compute `Tr(ρ·H)`.

---

## Example

### 1. Prepare a state and read the probabilities

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(1);
dm.apply_h(0).unwrap();

let probs = dm.probabilities();
assert_eq!(probs.len(), 2);
assert!((probs[0] - 0.5).abs() < 1e-10);
assert!((probs[1] - 0.5).abs() < 1e-10);
```

### 2. Maximally mixed state and trace

```rust
use cqlib_core::qis::DensityMatrix;

let dm = DensityMatrix::maximally_mixed(2);
assert_eq!(dm.num_qubits, 2);
assert_eq!(dm.data().len(), 16);
assert!((dm.trace().re - 1.0).abs() < 1e-10);
```

### 3. Construction from a pure state and from a matrix

```rust
use cqlib_core::qis::DensityMatrix;
use num_complex::Complex64;

// 由纯态振幅构造
let state = vec![Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)];
let dm = DensityMatrix::from_state(1, state).unwrap();
assert!((dm.data()[0].re - 0.0).abs() < 1e-10);
assert!((dm.data()[3].re - 1.0).abs() < 1e-10);

// 由展平的密度矩阵构造
let mut flat = vec![Complex64::new(0.0, 0.0); 4];
flat[0] = Complex64::new(0.5, 0.0);
flat[3] = Complex64::new(0.5, 0.0);
let mixed = DensityMatrix::from_density_matrix_state(1, flat).unwrap();
assert!((mixed.probabilities()[0] - 0.5).abs() < 1e-10);
assert!((mixed.probabilities()[1] - 0.5).abs() < 1e-10);
```

### 4. Validate physicality

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(1);
dm.apply_h(0).unwrap();

assert!(dm.is_hermitian(1e-10));
assert!(dm.is_positive_semidefinite_approx(1e-10));
assert!(dm.validate_physical(1e-10).is_ok());
```

### 5. Apply a quantum channel

```rust
use cqlib_core::qis::DensityMatrix;
use num_complex::Complex64;

let p: f64 = 0.3;
let k0 = vec![
    Complex64::new((1.0 - p).sqrt(), 0.0),
    Complex64::new(0.0, 0.0),
    Complex64::new(0.0, 0.0),
    Complex64::new((1.0 - p).sqrt(), 0.0),
];
let k1 = vec![
    Complex64::new(0.0, 0.0),
    Complex64::new(p.sqrt(), 0.0),
    Complex64::new(p.sqrt(), 0.0),
    Complex64::new(0.0, 0.0),
];

let mut dm = DensityMatrix::new(1);
dm.apply_kraus(&[k0, k1], &[0]).unwrap();

assert!((dm.trace().re - 1.0).abs() < 1e-10);
```

### 6. Compute a partial trace

```rust
use cqlib_core::qis::DensityMatrix;

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

// 追踪掉比特 1，比特 0 变成最大混合态
let reduced = dm.partial_trace(&[0]).unwrap();
assert_eq!(reduced.num_qubits, 1);
assert!((reduced.probabilities()[0] - 0.5).abs() < 1e-10);
assert!((reduced.probabilities()[1] - 0.5).abs() < 1e-10);
```

### 7. Accumulate density matrices

```rust
use cqlib_core::qis::DensityMatrix;

// 以全零矩阵作为累加起点
let mut acc = DensityMatrix::zeros(1);
acc += DensityMatrix::new(1);

assert!((acc.trace().re - 1.0).abs() < 1e-10);
assert!((acc.probabilities()[0] - 1.0).abs() < 1e-10);
```

### 8. Expectation value

```rust
use cqlib_core::qis::{DensityMatrix, Hamiltonian, Pauli, PauliString};

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

let mut ps = PauliString::new(2);
ps.set_pauli(0, Pauli::Z);
ps.set_pauli(1, Pauli::Z);
let h = Hamiltonian::from_pauli(ps);

let exp = dm.expectation(&h).unwrap();
assert!((exp - 1.0).abs() < 1e-10);
```

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::InvalidStateDimension` | The number of amplitudes of `from_state` does not equal `2^num_qubits`; the matrix length of `from_density_matrix_state` does not equal `4^num_qubits`; the number of circuit qubits of `apply_circuit` does not match that of the state. |
| `QisError::NotNormalized` | A trace not equal to 1 detected by `from_state`, `from_density_matrix_state` or `validate_physical`. |
| `QisError::NotHermitian` | A non-self-adjoint matrix detected by `from_density_matrix_state` or `validate_physical`. |
| `QisError::NotPositiveSemidefinite` | A negative eigenvalue detected by `from_density_matrix_state` or `validate_physical`. |
| `QisError::IndexOutOfBounds` | The qubit index is out of range. |
| `QisError::InvalidParameterValue` | A gate operation on a zero-qubit system; a two-qubit gate given the same qubit twice; the operator list of `apply_kraus` is empty, the qubit list is empty, or the operator size does not match the number of qubits; the kept-qubit list of `partial_trace` contains duplicates; the number of qubits or parameters of `apply_standard_gate` does not match the gate. |
| `QisError::UnsupportedOperation` | The circuit contains an operation the current entry point does not support. |
| `QisError::CircuitError` | A gate in the circuit has no matrix representation, contains an unresolved symbolic parameter, or references a qubit that does not exist; the number of qubits of the observable of `expectation` does not match. |

`+=` requires both density matrices to have the same number of qubits; a mismatch panics directly, so the caller must confirm the qubit counts before the call.
