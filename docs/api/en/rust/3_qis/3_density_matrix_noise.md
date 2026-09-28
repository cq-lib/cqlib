# DensityMatrixNoise

`cqlib_core::qis::state`

This page covers the density matrix simulator with a noise model, `DensityMatrixNoise`: how the noise model is attached, the entry points that connect it to circuits, gate operations, the readout noise interface, and measurement, sampling and expectation value computation.

## Import

```rust
use cqlib_core::qis::DensityMatrixNoise;
use cqlib_core::device::{NoiseModel, ReadoutError, SingleQubitNoise, TwoQubitNoise};
```

---

## DensityMatrixNoise

A simulator that layers a noise model on top of the density matrix simulator. After every gate it queries the noise model for noise entries on the same gate and the same qubits and applies the corresponding channel; when no noise model is configured or no entry matches, only the ideal gate is applied.

```rust
pub struct DensityMatrixNoise {
    pub state: DensityMatrix,
    pub noise_model: Option<NoiseModel>,
}
```

Fields:

- `state` (`DensityMatrix`): the underlying density matrix state.
- `noise_model` (`Option<NoiseModel>`): the noise model applied to gate operations and readout; `None` means ideal simulation.

Implements `Debug` and `Clone`.

Methods:

### Construction and circuit integration

- `fn new(num_qubits: usize, noise_model: Option<NoiseModel>) -> Self`: construct from a qubit count and a noise model; the initial state is `|0...0><0...0|`.
- `fn from_circuit(circuit: &Circuit, noise_model: Option<NoiseModel>) -> Result<Self, QisError>`: execute a circuit and return the simulator; the circuit is first decomposed into basis gates, and noise is applied after every gate.
- `fn apply_circuit(&mut self, circuit: &Circuit) -> Result<(), QisError>`: apply a circuit to the current simulator in place; noise is applied according to the configured model.

### Gate operations

- `fn apply_standard_gate_noise(&mut self, gate: StandardGate, qs: &[usize], params: &[f64]) -> Result<(), QisError>`: the unified entry point for standard gates; it validates and applies the ideal gate first, then applies matching gate noise according to the noise model.
- `fn apply_x(&mut self, q: usize) -> Result<(), QisError>`: Pauli-X.
- `fn apply_y(&mut self, q: usize) -> Result<(), QisError>`: Pauli-Y.
- `fn apply_z(&mut self, q: usize) -> Result<(), QisError>`: Pauli-Z.
- `fn apply_h(&mut self, q: usize) -> Result<(), QisError>`: Hadamard gate.
- `fn apply_s(&mut self, q: usize) -> Result<(), QisError>`: S gate.
- `fn apply_sdg(&mut self, q: usize) -> Result<(), QisError>`: S† gate.
- `fn apply_t(&mut self, q: usize) -> Result<(), QisError>`: T gate.
- `fn apply_tdg(&mut self, q: usize) -> Result<(), QisError>`: T† gate.
- `fn apply_u(&mut self, q: usize, theta: f64, phi: f64, lam: f64) -> Result<(), QisError>`: general single-qubit gate.
- `fn apply_phase(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: phase gate `P(θ)`.
- `fn apply_rx(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: rotation around the X axis by `θ`.
- `fn apply_ry(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: rotation around the Y axis by `θ`.
- `fn apply_rz(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: rotation around the Z axis by `θ`.
- `fn apply_gphase(&mut self, theta: f64) -> Result<(), QisError>`: global phase.
- `fn apply_x2p(&mut self, q: usize) -> Result<(), QisError>`: `Rx(π/2)`.
- `fn apply_x2m(&mut self, q: usize) -> Result<(), QisError>`: `Rx(-π/2)`.
- `fn apply_y2p(&mut self, q: usize) -> Result<(), QisError>`: `Ry(π/2)`.
- `fn apply_y2m(&mut self, q: usize) -> Result<(), QisError>`: `Ry(-π/2)`.
- `fn apply_rxy(&mut self, q: usize, theta: f64, phi: f64) -> Result<(), QisError>`: rotation around the specified axis in the XY plane.
- `fn apply_xy2p(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: `XY2P(θ)`.
- `fn apply_xy2m(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: `XY2M(θ)`.
- `fn apply_xy(&mut self, q: usize, theta: f64) -> Result<(), QisError>`: `XY(θ)`.
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled X gate.
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled Y gate.
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: controlled Z gate.
- `fn apply_ccx(&mut self, c1: usize, c2: usize, t: usize) -> Result<(), QisError>`: Toffoli gate.
- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: swap two qubits.
- `fn apply_crx(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rx(θ)`.
- `fn apply_cry(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Ry(θ)`.
- `fn apply_crz(&mut self, control: usize, target: usize, theta: f64) -> Result<(), QisError>`: controlled `Rz(θ)`.
- `fn apply_rxx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·X⊗X)`.
- `fn apply_ryy(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Y⊗Y)`.
- `fn apply_rzz(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗Z)`.
- `fn apply_rzx(&mut self, q0: usize, q1: usize, theta: f64) -> Result<(), QisError>`: `exp(-i·θ/2·Z⊗X)`.
- `fn apply_fsim(&mut self, q0: usize, q1: usize, theta: f64, phi: f64) -> Result<(), QisError>`: fermionic simulation gate.
- `fn apply_unitary_gate(&mut self, qs: &[usize], mat: &ndarray::Array2<Complex64>) -> Result<(), QisError>`: apply an arbitrary unitary matrix. This entry point has no corresponding standard gate type, so no gate noise is applied; when noise modeling is required, use the concrete gate methods.

### Probabilities and readout noise

The simulator expresses "noise on the quantum state" and "noise on readout" separately: the former acts on the density matrix itself, while the latter only changes the reported probability distribution.

- `fn probabilities(&self) -> Vec<f64>`: return the diagonal entries of the density matrix, that is the measurement probability of each computational basis state. Includes gate noise already applied and excludes readout noise.
- `fn probabilities_with_readout(&self, qubits: &[usize]) -> Result<Vec<f64>, QisError>`: based on `probabilities`, apply readout noise to the qubits listed in `qubits`.
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`: return the marginal probability distribution selected by `Measurement`, excluding readout noise.
- `fn probs_with_readout(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`: based on `probs`, apply readout noise to `measurement.qubits()`.

### Measurement, sampling and expectation values

- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`: measure in the Z basis and collapse the underlying density matrix; the measurement operation applies no noise.
- `fn measure_all(&mut self) -> Outcome`: measure one by one in the order `0..num_qubits` and return a bit-packed `Outcome`.
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`: reset the given qubit to `|0>`; reset applies no readout noise.
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`: sample a number of independent measurement results in parallel; the quantum state itself is sampled, and no readout noise is applied.
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`: sample according to the output convention specified by the circuit `Measurement`, likewise with no readout noise applied.
- `fn expectation(&self, h: &dyn Observable) -> Result<f64, QisError>`: compute `Tr(ρ·O)`; the result includes gate noise and excludes readout noise.

### Configuring the noise model

The noise model is provided by the device module; the configuration methods include:

- `NoiseModel::new()`: construct an empty model.
- `add_single_qubit_error(StandardGate, Qubit, SingleQubitNoise) -> Result<(), NoiseError>`: configure noise for a single-qubit gate.
- `add_two_qubit_error(StandardGate, Qubit, Qubit, TwoQubitNoise) -> Result<(), NoiseError>`: configure noise for a two-qubit gate.
- `add_readout_error(Qubit, ReadoutError) -> Result<(), NoiseError>`: configure readout noise for the given qubit.

For details of the discrete noise types, see the device module documentation.

---

## Example

### 1. Single-qubit simulation with bit flip noise

```rust
use cqlib_core::circuit::{Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, SingleQubitNoise};
use cqlib_core::qis::DensityMatrixNoise;

let mut noise_model = NoiseModel::new();
noise_model
    .add_single_qubit_error(StandardGate::X, Qubit::new(0), SingleQubitNoise::BitFlip(0.1))
    .unwrap();

let mut sim = DensityMatrixNoise::new(1, Some(noise_model));
sim.apply_x(0).unwrap();

// 10% 比特翻转使 P(|1>) 为 0.9
let probs = sim.probabilities_with_readout(&[0]).unwrap();
assert!((probs[1] - 0.9).abs() < 1e-6);
assert!((probs[0] - 0.1).abs() < 1e-6);
```

### 2. Construct a simulator from a circuit

```rust
use cqlib_core::circuit::{Circuit, Qubit, StandardGate};
use cqlib_core::device::{NoiseModel, SingleQubitNoise};
use cqlib_core::qis::DensityMatrixNoise;

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0)).unwrap();

let mut noise_model = NoiseModel::new();
noise_model
    .add_single_qubit_error(StandardGate::X, Qubit::new(0), SingleQubitNoise::BitFlip(0.1))
    .unwrap();

let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
assert_eq!(sim.state.num_qubits, 1);
assert!((sim.probabilities_with_readout(&[0]).unwrap()[1] - 0.9).abs() < 1e-6);
```

### 3. Readout noise

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{NoiseModel, ReadoutError};
use cqlib_core::qis::DensityMatrixNoise;

let mut noise_model = NoiseModel::new();
noise_model
    .add_readout_error(
        Qubit::new(0),
        ReadoutError {
            p_0_given_1: 0.0,
            p_1_given_0: 0.1,
        },
    )
    .unwrap();

let sim = DensityMatrixNoise::new(1, Some(noise_model));
let probs = sim.probabilities_with_readout(&[0]).unwrap();

// 真实态仍为 |0>，读出误差使 P(|1>) 为 0.1
assert!((probs[1] - 0.1).abs() < 1e-6);
assert!((probs[0] - 0.9).abs() < 1e-6);
```

### 4. Distinguish the ideal distribution from the post-readout distribution

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::device::{NoiseModel, Outcome, ReadoutError};
use cqlib_core::qis::DensityMatrixNoise;

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0)).unwrap();
let out = circuit.measure(Qubit::new(0)).unwrap();

let mut noise_model = NoiseModel::new();
noise_model
    .add_readout_error(
        Qubit::new(0),
        ReadoutError {
            p_0_given_1: 1.0,
            p_1_given_0: 0.0,
        },
    )
    .unwrap();

let sim = DensityMatrixNoise::from_circuit(&circuit, Some(noise_model)).unwrap();
let ideal = sim.probs(&out).unwrap();
let with_readout = sim.probs_with_readout(&out).unwrap();

assert!((ideal[&Outcome::from_bitstring("1").unwrap()] - 1.0).abs() < 1e-10);
assert!((with_readout[&Outcome::from_bitstring("0").unwrap()] - 1.0).abs() < 1e-10);
```

### 5. Sampling applies no readout noise

```rust
use cqlib_core::qis::DensityMatrixNoise;

let mut sim = DensityMatrixNoise::new(2, None);
sim.apply_h(0).unwrap();
sim.apply_cx(0, 1).unwrap();

let shots = sim.sample_shots(200);
assert_eq!(shots.len(), 200);
for outcome in &shots {
    assert_eq!(outcome.is_one(0), outcome.is_one(1));
}
```

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::InvalidParameterValue` | The qubit count or parameter count of `apply_standard_gate_noise` does not match the gate; the noise operation itself failed. |
| `QisError::InvalidStateDimension` | The qubit count of the circuit in `apply_circuit` is inconsistent with the simulator. |
| `QisError::UnsupportedOperation` | The circuit contains an operation not supported by the current entry point, for example classical control flow. |
| `QisError::CircuitError` | A gate in the circuit has no matrix representation, contains an unresolved symbolic parameter, or references a qubit that does not exist. |
| `QisError::IndexOutOfBounds` | The qubit index is out of range. |
| `QisError::QubitMismatch` | The qubit count of the observable passed to `expectation` does not match. |
| `cqlib_core::device::NoiseError` | A noise model configuration method failed, for example an invalid probability value or an arity mismatch between the gate and the qubits. |

Readout noise takes effect only in `probabilities_with_readout` and `probs_with_readout`; `probabilities`, `probs`, `sample_shots` and `sample` all report the distribution before readout noise is applied.
