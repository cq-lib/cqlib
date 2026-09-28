# QIS Quantum Information

`cqlib_core::qis`

`cqlib_core::qis` provides the foundational objects and local simulation capabilities related to quantum information science: several representations of quantum states, Pauli operators and Hamiltonians, construction of Pauli evolution circuits, and fidelity, entropy and entanglement metrics. This page is the overview and navigation for the module.

## Overview

The simulation result of a quantum circuit lands on a quantum state. Different tasks require different state representations: a circuit containing only standard gates and rotation gates can be described with a pure state representation; a scenario involving noise or requiring the description of a quantum channel must use a mixed state representation; a circuit containing only Clifford gates can use the stabilizer representation, describing the quantum state with data of polynomial size.

`cqlib_core::qis` organizes "the representation of a state" together with "the operations acting on a state": every simulator provides a set of common operations acting on states and accepts a whole circuit through an entry point that interfaces with the circuit IR. Observables, evolution and metrics are built on top of the state representations.

### Four state representations

| Representation | Use case | Noise support | Describable scale |
| --- | --- | --- | --- |
| `Statevector` | Ideal universal circuits, pure states | None | Small-scale systems |
| `DensityMatrix` | Mixed states and quantum channels | Kraus operators | Small-scale systems |
| `DensityMatrixNoise` | Device-level simulation with noise | Full noise model (gate noise and readout noise) | Small-scale systems |
| `StabilizerState` | Circuits containing only Clifford gates | None | Large-scale systems |

The four representations share consistent interface naming: gate operations correspond by name (`apply_h`, `apply_cx`, and so on), and the measurement and sampling interfaces are identical (`measure`, `measure_all`, `sample_shots`, `sample`, `probs`).

### Interfacing with circuits

All three dense representations provide two entry points for interfacing with circuits:

- `from_circuit`: execute a circuit and return the evolved state without modifying the input circuit.
- `apply_circuit`: apply a circuit in place to an existing state; the number of qubits of the circuit must match that of the state.

`StabilizerState` additionally provides `run_circuit`, which returns runtime classical data while executing a Clifford circuit, with the return type `CircuitExecutionResult`.

### Operators, evolution and metrics

Pauli operators and Pauli strings are the basic objects for describing observables and stabilizer generators. `Hamiltonian` is composed of Pauli strings with coefficients; once it implements the `Observable` trait it can be passed as an observable to the `expectation` method of a state to obtain an expectation value. `PauliEvolution` converts Pauli strings into executable evolution circuits. The `metrics` and `entropy` modules provide purity, fidelity, trace distance, entropy and entanglement metrics.

---

## Common entry points

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};

// 纯态模拟
let mut sv = Statevector::new(3);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();
let probs_sv = sv.probabilities();

// 混合态模拟
let mut dm = DensityMatrix::new(3);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();
let probs_dm = dm.probabilities();

// 纯态下两种表示给出相同的结果
assert!((probs_sv[0] - probs_dm[0]).abs() < 1e-10);
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Statevector** | The complex amplitude vector of a pure state in the computational basis; component `i` corresponds to the basis state `\|i>`. |
| **Mixed state** | A quantum state that cannot be described by a single statevector; represented by a density matrix. |
| **Density matrix** | A matrix describing a quantum state; the diagonal entries give the measurement probabilities in the computational basis. |
| **Quantum channel** | A trace-preserving map of states, represented by a set of Kraus operators `K_k`. |
| **Stabilizer state** | The eigenstate jointly determined by a set of mutually commuting Pauli operators (the stabilizer generators). |
| **Clifford gate** | A gate that maps the Pauli group to itself; the stabilizer representation accepts only this kind of gate. |
| **Readout noise** | The classical error introduced by the measurement step; it acts on the probability distribution rather than on the quantum state itself. |
| **Collapse** | Measurement causes the state to be projected onto the subspace consistent with the measurement outcome. |
| **Sampling** | Generating multiple independent measurement outcomes according to the probability distribution of the state. |
| **Observable** | A physical quantity whose expectation value can be obtained; described by the `Observable` trait. |

---

## `cqlib_core::qis` API overview

### Modules and state representations

| Name | Description |
| --- | --- |
| [`cqlib_core::qis`](0_overview.md) | Module overview, terminology and error handling. |
| [`Statevector`](1_statevector.md) | Pure state simulator, storing `2^n` complex amplitudes. |
| [`DensityMatrix`](2_density_matrix.md) | Mixed state simulator, supporting Kraus operators and partial trace. |
| [`DensityMatrixNoise`](3_density_matrix_noise.md) | Density matrix simulator with a noise model. |
| [`StabilizerState`](4_stabilizer.md) | Stabilizer state simulator, supporting large-scale Clifford circuits. |

### Classical data

| Name | Description |
| --- | --- |
| [`ClassicalState`](5_classical_state.md) / [`RuntimeValue`](5_classical_state.md) | Classical values and variables produced at runtime by a Clifford circuit. |

### Operators and observables

| Name | Description |
| --- | --- |
| [`Pauli`](6_pauli.md) / [`Phase`](6_pauli.md) | Single-qubit Pauli operators and phases. |
| [`PauliString`](6_pauli.md) / [`PauliIter`](6_pauli.md) | Pauli strings and their per-qubit iterator. |
| [`Hamiltonian`](7_hamiltonian.md) / [`Observable`](7_hamiltonian.md) | A sum of Pauli strings with coefficients, and the unified interface for observables. |

### Evolution and metrics

| Name | Description |
| --- | --- |
| [`PauliEvolution`](8_evolution.md) / [`TrotterMode`](8_evolution.md) | Construction of Pauli string evolution circuits and the Trotter decomposition mode. |
| [`metrics`](9_metrics_entropy.md) / [`entropy`](9_metrics_entropy.md) | Fidelity, purity, trace distance, and entropy and entanglement metrics. |

---

## Quick examples

### 1. Pure and mixed states give the same probability distribution

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

let mut dm = DensityMatrix::new(2);
dm.apply_h(0).unwrap();
dm.apply_cx(0, 1).unwrap();

let probs_sv = sv.probabilities();
let probs_dm = dm.probabilities();
for (a, b) in probs_sv.iter().zip(probs_dm.iter()) {
    assert!((a - b).abs() < 1e-10);
}
```

### 2. Stabilizer state sampling

```rust
use cqlib_core::qis::StabilizerState;
use std::collections::HashMap;

let mut state = StabilizerState::new(2);
state.apply_h(0).unwrap();
state.apply_cx(0, 1).unwrap();

let mut counts = HashMap::new();
for outcome in state.sample_shots(1000) {
    *counts.entry(outcome.to_bitstring(2)).or_insert(0usize) += 1;
}

// Bell 态的采样结果只会出现关联结果
assert!(counts.keys().all(|bits| bits == "00" || bits == "11"));
```

### 3. Compute the expectation value of an observable

```rust
use cqlib_core::qis::{Hamiltonian, PauliString, Statevector};

let mut sv = Statevector::new(2);
sv.apply_h(0).unwrap();
sv.apply_cx(0, 1).unwrap();

let mut h = Hamiltonian::new(2);
h.add_term("ZZ".parse::<PauliString>().unwrap(), 0.5.into()).unwrap();

let exp = sv.expectation(&h).unwrap();
assert!(exp.is_finite());
```

---

## Validation and error handling

`QisError` is the unified error type of `cqlib_core::qis`; failures in state construction, gate application, measurement and expectation value computation are all returned through it.

| Error | When it occurs |
| --- | --- |
| `QisError::QubitMismatch` | The number of qubits of an operator or circuit does not match that of the state. |
| `QisError::DimensionMismatch` | The dimensions of a matrix or vector do not match. |
| `QisError::InvalidStateDimension` | The length of the statevector is not a power of 2, or does not match the declared number of qubits. |
| `QisError::InvalidProbability` | A probability value is outside `[0, 1]`. |
| `QisError::IndexOutOfBounds` | The accessed qubit or amplitude index is out of range. |
| `QisError::UnsupportedOperation` | The current simulator does not support the operation. |
| `QisError::PauliStringParseError` | Pauli string parsing failed. |
| `QisError::NotNormalized` | An explicitly provided state is not normalized. |
| `QisError::NotHermitian` | An operator required to be Hermitian is not self-adjoint. |
| `QisError::NotPositiveSemidefinite` | The density matrix is not positive semidefinite. |
| `QisError::InvalidParameterValue` | A parameter value is invalid. |
| `QisError::InvalidSubsystem` | The subsystem description in an entanglement metric is invalid. |
| `QisError::UnsupportedDimension` | The dimension required by the operation is not satisfied. |
| `QisError::NonCliffordGate` | A non-Clifford gate was applied to a stabilizer state. |
| `QisError::CircuitError` | Circuit execution failed, for example a gate has no matrix representation or an unresolved symbolic parameter is present. |

See [Statevector](1_statevector.md), [DensityMatrix](2_density_matrix.md), [DensityMatrixNoise](3_density_matrix_noise.md) and [StabilizerState](4_stabilizer.md) for the specific trigger conditions on each page.
