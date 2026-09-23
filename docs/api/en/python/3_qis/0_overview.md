# Quantum Information

`cqlib.qis`

`cqlib.qis` is the quantum information module of Cqlib. It provides several representations and simulators of quantum states, the Pauli operator algebra, an observable framework centered on Hamiltonian, and analysis tools such as state fidelity, trace distance, entropy and entanglement measures. These facilities can be used independently for computations at the quantum information level, and can also consume circuits from `cqlib.circuit` directly to obtain evolution results.

## Overview

The quantum information module addresses two kinds of problems: **representation and evolution**, that is, storing a quantum state and letting gates act on it; and **measures**, that is, extracting comparable numerical values from states or observables. The two share the same underlying operator algebra.

### Four representations of quantum states

The four simulators differ in storage cost and applicable scope, and which one to choose depends on the gate set of the circuit and the quantities to be observed:

- The statevector represents a pure state with 2^N complex amplitudes; it is the most compact representation, but cannot describe classical mixtures and noise.
- The density matrix represents a mixed state with a 2^N × 2^N matrix; the cost is an order of magnitude higher, but it supports quantum channels, partial traces and physicality validation.
- The noisy density matrix binds a noise model on top of the density matrix, applies Kraus noise automatically according to gates, and provides probability distributions under readout noise.
- The stabilizer table represents a Clifford state with N commuting Pauli generators; the storage cost is linear in the number of qubits, but only Clifford gates are accepted.

The first three share the same set of gate application entry points (`apply_circuit`, `apply_standard_gate`, single-qubit and two-qubit gate methods), and the differences between them are concentrated in state storage and noise handling.

### Qubit index convention

All four representations use the same qubit numbering convention: qubit i corresponds to bit i of the basis state index, with qubit 0 as the least significant bit; in bitstring representations, the higher-numbered qubit is on the left. This convention is consistent across all pages and is not repeated.

### From circuit to state

All four simulators provide a `from_circuit()` static method, which simulates a circuit starting from |0…0⟩ to obtain the final state; the input circuit is left unchanged. When both the final state and the runtime classical data are needed, use `StabilizerState.run_circuit()`.

### Observables and time evolution

`PauliString` and `Hamiltonian` describe observables, and both can give expectation values; `Hamiltonian` can also be converted into an evolution circuit through Trotter decomposition, returning to the circuit space of `cqlib.circuit` for execution.

---

## Common entry points

```python
import math

from cqlib import Circuit
from cqlib.qis import PauliString
from cqlib.qis.state import Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

sv = Statevector.from_circuit(circuit)
assert math.isclose(sv.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(sv.probabilities()[3], 0.5, abs_tol=1e-10)
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Statevector** | An N-qubit pure state described by 2^N complex amplitudes, where the amplitude of the basis state \|i⟩ is at index `i`. |
| **Density matrix** | An N-qubit quantum state described by a 2^N × 2^N Hermitian positive semidefinite matrix, which can represent classical mixtures of pure states. |
| **Noisy simulation** | A simulation approach that applies Kraus noise according to a noise model on top of the density matrix, where gate noise takes effect immediately after each gate application. |
| **Stabilizer table** | A compact representation of a Clifford state by N commuting Pauli generators, whose storage cost grows linearly with the number of qubits. |
| **Clifford gate** | A gate that maps the Pauli group onto itself, such as H, S, X, Y, Z and controlled Pauli gates. Non-Clifford gates cannot be simulated with a stabilizer table. |
| **Destabilizer** | A set of Pauli operators paired with the stabilizer generators, used to determine measurement outcomes and maintain the canonical form of the stabilizer table. |
| **Symplectic representation** | A compact storage form of a Pauli string, describing an operator by two bit masks for X and Z together with a phase bit. |
| **Observable** | An object for which an expectation value and a variance can be computed, such as `PauliString` and `Hamiltonian`. |
| **Expectation value** | The average of an observable in a given state: ⟨ψ\|O\|ψ⟩ for a pure state and Tr(ρ·O) for a mixed state. |
| **Partial trace** | The operation of tracing out part of the qubits and keeping the rest, used to obtain the reduced density matrix of a subsystem. |
| **Trotter decomposition** | A decomposition that approximates the time evolution operator e^{-iHt} by a sequence of Pauli rotations. |
| **Readout noise** | Classical flips introduced at the measurement stage, which make readout results inconsistent with the true measurement results. |
| **Runtime classical data** | Classical values produced during circuit execution and queried by handle after execution ends. |

---

## `cqlib.qis` API overview

### Module entry

| Name | Description |
| --- | --- |
| [`cqlib.qis`](0_overview.md) | Module overview, that is, this page. |

### State simulation

| Name | Description |
| --- | --- |
| [`Statevector`](1_statevector.md) | Pure-state simulator, representing an N-qubit quantum state with 2^N complex amplitudes. |
| [`DensityMatrix`](2_density_matrix.md) | Mixed-state simulator, supporting quantum channels, partial traces and physicality validation. |
| [`DensityMatrixNoise`](3_density_matrix_noise.md) | Noisy mixed-state simulator, applying gate noise and readout noise according to a noise model. |
| [`StabilizerState`](4_stabilizer.md) | Clifford state simulator, representing states with a stabilizer table and supporting generator queries. |
| [`StabilizerCircuitResult`](4_stabilizer.md) | The result of `StabilizerState.run_circuit()`, carrying the final state and the runtime classical data. |

### Runtime classical data

| Name | Description |
| --- | --- |
| [`RuntimeValue`](5_classical_state.md) | The typed value of a single classical value in one execution. |
| [`ClassicalState`](5_classical_state.md) | A snapshot of all runtime classical data after one execution ends, with values queried by handle. |

### Pauli algebra and observables

| Name | Description |
| --- | --- |
| [`Phase`](6_pauli.md) | The phase factor produced when elements of the Pauli group are multiplied. |
| [`Pauli`](6_pauli.md) | A single-qubit Pauli operator. |
| [`PauliString`](6_pauli.md) | A tensor product of multi-qubit Pauli operators, stored in the symplectic representation. |
| [`Observable`](7_hamiltonian.md) | The protocol for observables, defining the entry points for computing expectation values and variances. |
| [`Hamiltonian`](7_hamiltonian.md) | An observable represented as a linear combination of Pauli strings, whose expectation value can be computed and from which evolution circuits can be generated. |

### Time evolution

| Name | Description |
| --- | --- |
| [`TrotterMode`](8_evolution.md) | The Trotter decomposition mode of the time evolution operator, including first order, second order and randomized. |
| [`Hamiltonian.to_trotter_circuit`](8_evolution.md) | Convert `Hamiltonian` into a Trotter-approximated evolution circuit. |
| [`Hamiltonian.to_evolution_circuit`](8_evolution.md) | Convert `Hamiltonian` into a time evolution circuit. |

### Metrics and entropy

| Name | Description |
| --- | --- |
| [`purity_pure`](9_metrics_entropy.md) / [`purity_mixed`](9_metrics_entropy.md) | Purity of pure and mixed states. |
| [`state_fidelity_pure`](9_metrics_entropy.md) / [`state_fidelity_pure_mixed`](9_metrics_entropy.md) / [`state_fidelity_mixed`](9_metrics_entropy.md) | State fidelity between pure states, between a pure state and a mixed state, and between mixed states. |
| [`trace_distance_pure`](9_metrics_entropy.md) / [`trace_distance_mixed`](9_metrics_entropy.md) | Trace distance of pure and mixed states. |
| [`entropy`](9_metrics_entropy.md) | Von Neumann entropy of a density matrix. |
| [`partial_transpose`](9_metrics_entropy.md) / [`logarithmic_negativity`](9_metrics_entropy.md) | Partial transpose and logarithmic negativity. |
| [`linear_entropy`](9_metrics_entropy.md) / [`renyi_entropy`](9_metrics_entropy.md) | Linear entropy and Rényi entropy. |
| [`entanglement_entropy_pure`](9_metrics_entropy.md) | Entanglement entropy of a pure state on a given subsystem. |
| [`negativity`](9_metrics_entropy.md) / [`concurrence`](9_metrics_entropy.md) / [`entanglement_of_formation`](9_metrics_entropy.md) | Negativity, concurrence and entanglement of formation. |

---

## Quick examples

### 1. Pure-state and mixed-state simulation of the same circuit

```python
import math

from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString
from cqlib.qis.state import DensityMatrix, Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

# 纯态：用态矢量计算期望值
sv = Statevector.from_circuit(circuit)
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)

# 混合态：用密度矩阵计算期望值，并取子系统的约化态
dm = DensityMatrix.from_circuit(circuit)
hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 0.5)
hamiltonian.add_term(PauliString.from_str("XX"), 0.5)
hamiltonian.simplify()
assert math.isclose(dm.expectation(hamiltonian), 1.0, abs_tol=1e-10)

reduced = dm.partial_trace([0])
assert math.isclose(reduced.probabilities()[0], 0.5, abs_tol=1e-10)
```

### 2. Simulation with noise

```python
import math

from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.1))

sim = DensityMatrixNoise(1, noise_model)
sim.apply_x(0)

probs = sim.probabilities()
assert math.isclose(probs[1], 0.9, abs_tol=0.01)
assert math.isclose(probs[0], 0.1, abs_tol=0.01)
```

### 3. Clifford circuits and the stabilizer table

```python
from cqlib import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = StabilizerState.from_circuit(circuit)
assert state.probabilities()[0] == 0.5
assert state.probabilities()[3] == 0.5

# 稳定子表的逐行文本表示
assert state.to_stim_format().count("\n") == 2
assert len(state.get_stabilizers()) == 2
```

---

## Validation and error handling

The simulators validate at the stages of construction, gate application and state read/write. The exception tables on each page list all trigger conditions, and the shared categories are as follows:

| Exception | When it occurs |
| --- | --- |
| `IndexError` | Qubit index out of range. |
| `ValueError` | Dimension or length inconsistent with the number of qubits, amplitudes not normalized, density matrix violating physical constraints, invalid Kraus operator, duplicate target qubits of a two-qubit gate, circuit containing non-Clifford operations or control flow, or observable type or qubit count mismatch. |
| `TypeError` | Unsupported input type, for example an initial state that is neither an array nor a list received by the constructor, or a tolerance parameter of a validation method that is not a floating-point number. |
| `OverflowError` | `num_qubits` is negative. |

The Raises section of each page gives the complete trigger conditions of that page's API.
