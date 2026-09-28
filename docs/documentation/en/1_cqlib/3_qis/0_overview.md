# QIS Quantum Information

`cqlib.qis` provides fundamental objects and local simulation capabilities related to quantum information science, for inspecting quantum states, computing observables, constructing Pauli Hamiltonians, generating Trotter evolution circuits, and analyzing fidelity, entropy and entanglement metrics.

The examples in this chapter all run in a local Python environment, without connecting to a cloud platform or submitting tasks to real quantum hardware. Before starting, complete [Cqlib installation and environment setup](../../0_get_started/1_installation.md) and confirm that the imported Cqlib version is the one to be tested.

```python
import cqlib

print(cqlib.__file__)
```

---

## Common entry points

The Python entry points of QIS are concentrated in `cqlib.qis`, and state simulators can also be imported from the `cqlib.qis.state` subpackage.

```python
from cqlib.qis import (
    Statevector,
    DensityMatrix,
    DensityMatrixNoise,
    StabilizerState,
    StabilizerCircuitResult,
    RuntimeValue,
    ClassicalState,
    Phase,
    Pauli,
    PauliString,
    Hamiltonian,
    TrotterMode,
    metrics,
    entropy,
)
```

| Object | Main use | Common scenarios |
|---|---|---|
| `Statevector` | Pure-state simulation, storing `2^n` complex amplitudes | Ideal circuit verification, small-scale VQE/QAOA energy computation, state sampling |
| `DensityMatrix` | Density matrix simulation, storing a `2^n × 2^n` complex matrix | Mixed states, Kraus noise, partial trace, physicality checks |
| `DensityMatrixNoise` | Density matrix simulator with a `NoiseModel` | Gate noise, readout error, comparison of noisy circuits |
| `StabilizerState` | Clifford stabilizer simulation | Larger Clifford circuits, stabilizer generators, fast sampling |
| `Pauli` / `PauliString` | Single-qubit and multi-qubit Pauli operators | Observables, commutation checks, measurement expectation values |
| `Hamiltonian` | Sparse sum of Pauli terms | VQE, QAOA, Ising model, time evolution |
| `TrotterMode` | Trotter-Suzuki decomposition mode | Construction of `e^{-iHt}` evolution circuits |
| `metrics` / `entropy` | Quantum state distance, purity, entropy and entanglement metrics | State similarity, mixedness, entanglement analysis |

These objects usually do not replace `Circuit`; they are used to execute and interpret `Circuit`. The circuit expresses program structure, and QIS turns the circuit into analyzable states, observables and numerical metrics.

---

## Starting from the Bell state

The Bell state is the most suitable example for getting started with QIS: it is short, but it simultaneously contains superposition, entanglement, probability distributions and observable expectation values.

```python
from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString, Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = Statevector.from_circuit(circuit)

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 1.0)

print(state.probabilities())
print(hamiltonian.expectation_statevector(state))
```

The computational-basis probabilities of an ideal Bell state should be concentrated on `|00>` and `|11>`, so the expectation value of `ZZ` is close to `1.0`. Such checks are often used to confirm that the circuit structure, the qubit indices and the observable definitions are consistent.

---

## How to choose a state representation

Different state representations differ in memory and semantics. When writing an algorithm prototype, it is recommended to first clarify which kind of state the problem requires.

| Task | Recommended object | Reason |
|---|---|---|
| Only ideal pure states are needed | `Statevector` | Lower memory overhead than a density matrix, suitable for quick verification |
| Mixed states or Kraus channels are needed | `DensityMatrix` | Can express coherence terms, decoherence and non-pure states |
| A device noise model needs to be applied automatically | `DensityMatrixNoise` | Combined with `NoiseModel`, noise is injected automatically after gates |
| The circuit contains only Clifford gates | `StabilizerState` | More efficient for Clifford structures, suitable for stabilizer analysis |
| Expectation values need to be estimated only from sampling probabilities | `PauliString` / `Hamiltonian` | Expectation values can be computed from Pauli measurement results |

If a circuit contains measurement, reset or dynamic control flow, the whole circuit usually cannot be regarded as a single unitary matrix. In that case the choice depends on the specific goal: visualization for debugging structure, `Statevector` for analyzing ideal unitary fragments, and `DensityMatrix` or `DensityMatrixNoise` for analyzing noise and mixed states.

---

## A typical QIS workflow

A common closed loop is as follows: first construct the circuit, then choose a state simulator, then define observables, and finally compute probabilities, expectation values or metrics.

```python
from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString, Statevector, metrics

ansatz = Circuit(2)
ansatz.ry(0, 0.3)
ansatz.cx(0, 1)
ansatz.ry(1, -0.2)

state = Statevector.from_circuit(ansatz)

observable = Hamiltonian.from_list([
    (PauliString.from_str("ZI"), -1.0),
    (PauliString.from_str("IZ"), -1.0),
    (PauliString.from_str("ZZ"), 0.5),
])

print("probabilities:", state.probabilities())
print("energy:", observable.expectation_statevector(state))
print("purity:", metrics.purity_pure(state))
```

This flow corresponds to the inner computation of many algorithms: VQE cares about `energy`, QAOA cares about the expectation value of the cost Hamiltonian, and error diagnosis cares about fidelity, trace distance, purity and entropy.

---

## Learning path for this chapter

The reading order is as follows:

1. [Statevector pure-state simulation](1_statevector.md): learn pure-state construction, gate application, circuit execution, sampling and expectation values.
2. [DensityMatrix mixed-state simulation](2_density_matrix.md): learn density matrices, Kraus noise, partial trace and physicality checks.
3. [DensityMatrixNoise noisy simulation](3_density_matrix_noise.md): learn how to apply `NoiseModel` to gate noise and readout error simulation.
4. [StabilizerState stabilizer simulation](4_stabilizer.md): learn the stabilizer representation, measurement and generator analysis of Clifford circuits.
5. [Pauli, PauliString and Hamiltonian](5_pauli_and_hamiltonian.md): learn the Pauli group, commutation, Hamiltonians, expectation values and Trotter evolution.
6. [Quantum state metrics and entropy](6_metrics_entropy.md): learn purity, fidelity, trace distance, Von Neumann entropy, Rényi entropy and entanglement metrics.

It is recommended to read the first two sections first, in order to establish the basic semantics of pure and mixed states, and then read the Pauli and Hamiltonian sections. Noisy simulation, stabilizer simulation and metric computation can be read in an interleaved way according to algorithm requirements.

---

## When to use the QIS module

In quantum program development, the QIS module is usually used in the following places:

- after writing a small circuit, verifying whether the final probability distribution matches expectations;
- after binding an ansatz to a set of parameters, computing observable expectation values;
- comparing fidelity or trace distance among an ideal state, a noisy state and a target state;
- performing a partial trace on a mixed state to inspect the state of a subsystem;
- using PauliString to check commutation, measurement bases and Hamiltonian terms;
- converting a Pauli Hamiltonian into a Trotter evolution circuit;
- using the stabilizer simulator to quickly check a Clifford circuit.

QIS numerical results cannot replace circuit visualization and unit tests. In critical algorithm modules, it is recommended to keep circuit diagrams, probability checks, expectation value checks and the necessary physicality checks at the same time.

---

## Next steps

- [Statevector pure-state simulation](1_statevector.md): first use an ideal pure state to check gate sequences, probability distributions, sampling and observable expectation values.
- [DensityMatrix mixed-state simulation](2_density_matrix.md): learn density matrices, Kraus noise, partial trace and physicality verification.
- [Pauli, PauliString and Hamiltonian](5_pauli_and_hamiltonian.md): master the Pauli representation, Hamiltonian modeling, expectation value computation and Trotter evolution.
