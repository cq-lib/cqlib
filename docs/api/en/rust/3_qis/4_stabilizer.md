# StabilizerState

`cqlib_core::qis::state`

This page covers the stabilizer state simulator `StabilizerState` and the circuit execution result `CircuitExecutionResult`: construction, Clifford gate operations, generator access, Pauli expectation values, and the measurement and sampling interfaces.

## Import

```rust
use cqlib_core::qis::StabilizerState;
use cqlib_core::qis::state::stabilizer::CircuitExecutionResult;
```

---

## StabilizerState

A stabilizer state simulator that describes a quantum state with a set of mutually commuting Pauli operators (the stabilizer generators). It can represent only stabilizer states, and therefore accepts only Clifford gates, which map the Pauli group onto itself; it returns an error for non-Clifford operations such as arbitrary-angle rotations, the `T` gate or `fSim`. The qubit scale it can describe is far larger than a dense representation.

```rust
pub struct StabilizerState {
    pub num_qubits: usize,
}
```

Fields:

- `num_qubits` (`usize`): the number of qubits.

The generator table is kept in a private field. Implements `Debug` and `Clone`; `PartialEq` is not implemented, so to compare two states, compare the values returned by `get_stabilizers()`.

Methods:

### Construction

- `fn new(n: usize) -> Self`: construct the `|0...0>` state; the initial generators are the destabilizer `X_i` in row `i` and the stabilizer `Z_i` in row `n+i`, both with phase `+1`.
- `fn num_qubits(&self) -> usize`: return the number of qubits, consistent with the value of the field of the same name.

### Circuit integration

- `fn from_circuit(circuit: &Circuit) -> Result<Self, QisError>`: execute a Clifford circuit and return the final stabilizer state.
- `fn apply_circuit(&mut self, input_circuit: &Circuit) -> Result<(), QisError>`: apply a Clifford circuit to the current state in place; the qubit count of the circuit must match that of the state.
- `fn run_circuit(circuit: &Circuit) -> Result<CircuitExecutionResult, QisError>`: execute a Clifford circuit and return both the final state and the runtime classical data.

`from_circuit` and `apply_circuit` are state-level entry points: they treat the measurement and store operations produced by `Circuit::measure*` as output declarations and ignore them, neither collapsing the state nor writing classical data; when execution semantics are required (measurement collapses immediately and writes classical data), use `run_circuit`.

The supported instructions are the Clifford gates `I`, `H`, `X`, `Y`, `Z`, `S`, `SDG`, `X2P`, `X2M`, `Y2P`, `Y2M`, `CX`, `CY`, `CZ` and `SWAP`, plus the `Reset` instruction; barriers and delays have no effect. `run_circuit` additionally executes `MeasureBit`, `MeasureBits` and `Store`, but does not support control flow gates.

### Clifford gates

- `fn apply_h(&mut self, qubit: usize) -> Result<(), QisError>`: Hadamard gate.
- `fn apply_x(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-X.
- `fn apply_y(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-Y.
- `fn apply_z(&mut self, qubit: usize) -> Result<(), QisError>`: Pauli-Z.
- `fn apply_s(&mut self, qubit: usize) -> Result<(), QisError>`: S gate.
- `fn apply_sdg(&mut self, qubit: usize) -> Result<(), QisError>`: S† gate.
- `fn apply_x2p(&mut self, qubit: usize) -> Result<(), QisError>`: `√X`, that is `Rx(π/2)`.
- `fn apply_x2m(&mut self, qubit: usize) -> Result<(), QisError>`: `√X†`, that is `Rx(-π/2)`.
- `fn apply_y2p(&mut self, qubit: usize) -> Result<(), QisError>`: `√Y`, that is `Ry(π/2)`.
- `fn apply_y2m(&mut self, qubit: usize) -> Result<(), QisError>`: `√Y†`, that is `Ry(-π/2)`.
- `fn apply_cx(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled X gate.
- `fn apply_cy(&mut self, control: usize, target: usize) -> Result<(), QisError>`: controlled Y gate.
- `fn apply_cz(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: controlled Z gate.
- `fn apply_swap(&mut self, q0: usize, q1: usize) -> Result<(), QisError>`: swap two qubits.
- `fn apply_standard_gate(&mut self, gate: StandardGate, qubits: &[usize], params: &[f64]) -> Result<(), QisError>`: dispatch by the standard gate enum; an error is raised when `gate` is not in the Clifford set.

### Generators and Pauli expectation values

- `fn get_stabilizers(&self) -> Vec<PauliString>`: return the `n` stabilizer generators.
- `fn get_destabilizers(&self) -> Vec<PauliString>`: return the `n` destabilizer generators.
- `fn pauli_expectation(&self, pauli: &PauliString) -> Result<i32, QisError>`: return `⟨ψ|P|ψ⟩`; a value of `1` means `P` belongs to the stabilizer group, `-1` means it belongs to the negated stabilizer group, and `0` means neither. This test covers products of generators, not just individual generators.
- `fn to_stim_format(&self) -> String`: export the generator table in a compatible format.

### Probabilities, measurement and sampling

- `fn probability_of(&self, bits: &[bool]) -> Result<f64, QisError>`: return the probability of the given computational basis state; a bitstring compatible with the stabilizer state returns `Ok(1.0 / 2.0^k)`, where `k` is the number of qubits whose measurement outcome is random; an incompatible one returns `Ok(0.0)`. The call does not modify `self`.
- `fn probabilities(&self) -> Result<Vec<f64>, QisError>`: return the probability distribution over all computational basis states, of length `2^n`, where index `i` corresponds to the basis state whose binary representation is `i`, with qubit 0 as the least significant bit. This interface is suitable only for small systems and returns an error when `n > 20`.
- `fn measure(&mut self, qubit: usize) -> Result<bool, QisError>`: measure in the Z basis and collapse the state; `false` means the outcome `0` and `true` means the outcome `1`.
- `fn measure_all(&mut self) -> Outcome`: measure all qubits in order and return a bit-packed `Outcome`.
- `fn reset(&mut self, qubit: usize) -> Result<(), QisError>`: reset the given qubit to `|0>`, implemented as a Z-basis measurement followed by an X correction when the outcome is `|1>`.
- `fn sample_shots(&self, shots: usize) -> Vec<Outcome>`: sample a number of independent measurement results in parallel; the same initial state always produces the same set of samples.
- `fn sample(&self, measurement: &Measurement, shots: usize) -> Result<ExecutionResult, QisError>`: sample according to the output convention specified by the circuit `Measurement`, without reading or executing the circuit itself; `measurement.qubits()[i]` corresponds to bit `i` of each `Outcome`.
- `fn probs(&self, measurement: &Measurement) -> Result<HashMap<Outcome, f64>, QisError>`: return the marginal probability distribution selected by `Measurement`; its qubit order is consistent with `sample`. This interface reuses `probabilities` and is therefore likewise subject to the `n <= 20` limit.

---

## CircuitExecutionResult

The object returned by `run_circuit`, containing both the final quantum state and the runtime classical data produced during execution.

```rust
pub struct CircuitExecutionResult {
    pub state: StabilizerState,
    pub classical: ClassicalState,
}
```

Fields:

- `state`: the stabilizer state after all operations have been executed.
- `classical`: measurement results and mutable variables, indexed by the handles in the circuit.

Implements `Debug`. For the concrete form of runtime classical values, see [ClassicalState](5_classical_state.md).

---

## Example

### 1. Stabilizer sampling of a Bell state

```rust
use cqlib_core::qis::StabilizerState;

let mut s = StabilizerState::new(2);
s.apply_h(0).unwrap();
s.apply_cx(0, 1).unwrap();

let shots = s.sample_shots(500);
assert_eq!(shots.len(), 500);
for shot in &shots {
    assert_eq!(shot.is_one(0), shot.is_one(1));
}
```

### 2. Read the stabilizer generators

```rust
use cqlib_core::qis::StabilizerState;

let s = StabilizerState::new(2);
let stabs = s.get_stabilizers();
assert_eq!(stabs.len(), 2);

// 初态 |00⟩ 的稳定子为 +ZI 与 +IZ
let stabs: Vec<String> = stabs.iter().map(|p| p.to_string()).collect();
assert_eq!(stabs[0], "+IZ");
assert_eq!(stabs[1], "+ZI");
```

### 3. Compute the probability of a given basis state

```rust
use cqlib_core::qis::StabilizerState;

let mut s = StabilizerState::new(2);
s.apply_h(0).unwrap();
s.apply_cx(0, 1).unwrap();

// 只有 |00⟩ 与 |11⟩ 可能出现，各占一半
assert!((s.probability_of(&[false, false]).unwrap() - 0.5).abs() < 1e-12);
assert!((s.probability_of(&[true, true]).unwrap() - 0.5).abs() < 1e-12);
assert_eq!(s.probability_of(&[false, true]).unwrap(), 0.0);
assert_eq!(s.probability_of(&[true, false]).unwrap(), 0.0);
```

### 4. Pauli expectation value

```rust
use cqlib_core::qis::{Pauli, PauliString, Phase, StabilizerState};

// |+⟩ 是 +X 的本征态
let mut s = StabilizerState::new(1);
s.apply_h(0).unwrap();

let mut x = PauliString::new(1);
x.set_pauli(0, Pauli::X);
assert_eq!(s.pauli_expectation(&x).unwrap(), 1);

// Bell 态下 ⟨ZI⟩ 为 0
let mut bell = StabilizerState::new(2);
bell.apply_h(0).unwrap();
bell.apply_cx(0, 1).unwrap();

let mut zi = PauliString::new(2);
zi.set_pauli(0, Pauli::Z);
zi.set_pauli(1, Pauli::I);
zi.phase = Phase::Plus;
assert_eq!(bell.pauli_expectation(&zi).unwrap(), 0);
```

### 5. Construct from a circuit and obtain the marginal distribution

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::StabilizerState;

let mut c = Circuit::new(2);
c.h(Qubit::new(0)).unwrap();
c.cx(Qubit::new(0), Qubit::new(1)).unwrap();
let out = c.measure_bits([Qubit::new(1), Qubit::new(0)]).unwrap();

// 该测量声明不使 Bell 态坍缩
let stab = StabilizerState::from_circuit(&c).unwrap();
let probs = stab.probs(&out).unwrap();
assert!((probs.values().sum::<f64>() - 1.0).abs() < 1e-10);
```

### 6. Execute a circuit and read the runtime classical data

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::{RuntimeValue, StabilizerState};

let mut c = Circuit::new(1);
c.x(Qubit::new(0)).unwrap();
let measured = c.measure(Qubit::new(0)).unwrap();

let result = StabilizerState::run_circuit(&c).unwrap();
assert_eq!(
    result.classical.value(measured.value()),
    Some(&RuntimeValue::Bit(true))
);
```

### 7. Non-Clifford gates are rejected

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::qis::StabilizerState;

let mut c = Circuit::new(1);
c.t(Qubit::new(0)).unwrap();

let result = StabilizerState::from_circuit(&c);
assert!(result.is_err());
```

---

## Validation and error handling

| Error | When it occurs |
| --- | --- |
| `QisError::NonCliffordGate` | A non-Clifford gate is applied to a stabilizer state, for example `T`, `T†`, `Rx`, `Ry`, `Rz` or `Phase` at an arbitrary angle, or a general unitary gate. |
| `QisError::QubitMismatch` | The bitstring length of `probability_of` does not match the qubit count; the qubit count of the Pauli string in `pauli_expectation` is inconsistent with the state. |
| `QisError::IndexOutOfBounds` | The qubit index is out of range. |
| `QisError::InvalidParameterValue` | A gate is applied to a zero-qubit system; a two-qubit gate is given the same qubit twice; `probabilities` or `probs` is called when the qubit count exceeds the limit. |
| `QisError::InvalidStateDimension` | The qubit count of the circuit in `apply_circuit` is inconsistent with the state. |
| `QisError::UnsupportedOperation` | The circuit contains an operation not supported by the current entry point, such as control flow. |
| `QisError::CircuitError` | A gate in the circuit cannot be executed or contains an unresolved symbolic parameter. |

Repeated sampling of the same initial state yields the same set of samples, so sampling results can be used for reproducible comparison; when different random results are needed, rebuild the circuit or change the gate sequence.
