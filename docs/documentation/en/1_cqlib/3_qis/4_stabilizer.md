# StabilizerState Stabilizer Simulation

`StabilizerState` represents Clifford circuits with a stabilizer tableau. It is suitable for simulating circuits that contain only Clifford gates, and can directly read stabilizer generators, destabilizers, Pauli expectation values and measurement results.

When a circuit contains arbitrary-angle rotations, `T` gates, `fSim` or other non-Clifford operations, `Statevector` or `DensityMatrix` should be used instead.

---

## Task: simulate a Bell state with stabilizers

```python
from cqlib.qis import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(state.probabilities())
print(state.get_stabilizers())
```

The stabilizer generators of a Bell state can be used to check the entanglement structure. Compared with looking only at probabilities, stabilizer generators express more directly which Pauli constraints the state satisfies.

---

## Constructing a stabilizer state from a Circuit

If a Clifford circuit is already available, a stabilizer state can be constructed directly from the circuit.

```python
from cqlib import Circuit
from cqlib.qis import StabilizerState

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 1)
circuit.cx(1, 2)

state = StabilizerState.from_circuit(circuit)
print(state.probabilities())
```

`apply_circuit()` can also be used to apply a circuit in place to an existing stabilizer state.

```python
prefix = Circuit(3)
prefix.h(0)

suffix = Circuit(3)
suffix.cx(0, 1)
suffix.cx(1, 2)

state = StabilizerState(3)
state.apply_circuit(prefix)
state.apply_circuit(suffix)

print(state.get_stabilizers())
```

This segmented approach is suitable for checking the intermediate stabilizer structure of Clifford encoding circuits, error correction circuits or large-scale GHZ circuits.

---

## Executing a stabilizer circuit with classical state

`run_circuit()` returns a `StabilizerCircuitResult`, which contains the final stabilizer state and the classical state.

```python
from cqlib import Circuit
from cqlib.qis import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)

result = StabilizerState.run_circuit(circuit)

print(result.state)
print(result.classical)
```

When a circuit contains measurement or classical values in a dynamic circuit, `run_circuit()` is more suitable than reading only the final quantum state for preserving the classical execution results.

---

## Measurement, reset and sampling

Stabilizer states support single-qubit measurement, full measurement, reset and finite-shot sampling.

```python
from cqlib.qis import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

shots = state.sample_shots(32)
counts = {}
for outcome in shots:
    bitstring = outcome.to_bitstring(state.num_qubits)
    counts[bitstring] = counts.get(bitstring, 0) + 1

print(counts)

measured = state.measure_all()
print(measured.to_bitstring(2))
```

`measure()` and `measure_all()` collapse the current stabilizer state; `sample_shots()` is suitable for generating sampling results without handling the collapse process manually.

---

## Inspecting stabilizers, destabilizers and Pauli expectation values

```python
from cqlib.qis import PauliString, StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

print("stabilizers:", state.get_stabilizers())
print("destabilizers:", state.get_destabilizers())
print("ZZ:", state.pauli_expectation(PauliString.from_str("ZZ")))
print("XI:", state.pauli_expectation(PauliString.from_str("XI")))
print(state.to_stim_format())
```

`pauli_expectation()` returns `-1`, `0` or `1`. This is suitable for quickly checking whether a given Pauli observable is determined by the current stabilizer state.

---

## Supported scope

`StabilizerState` is suitable for the following operations:

- single-qubit Clifford gates: `H`, `S`, `Sdg`, `X`, `Y`, `Z`, `X2p`, `X2m`, `Y2p`, `Y2m`;
- two-qubit Clifford gates: `CX`, `CY`, `CZ`, `SWAP`;
- measurement, reset, sampling and probability reading under stabilizer semantics.

If a circuit contains non-Clifford operations, the stabilizer simulator cannot express the complete state exactly. In that case, return to `Statevector` or `DensityMatrix`, or first isolate the non-Clifford fragments for separate analysis.


---

## Next steps

- [Statevector pure-state simulation](1_statevector.md): when a circuit contains non-Clifford gates, continue verification with the pure-state simulator.
- [Pauli, PauliString and Hamiltonian](5_pauli_and_hamiltonian.md): understand the PauliString representation and the Pauli expectation values behind stabilizer generators.
- [Debugging circuits with text diagrams](../5_visualization/1_draw_text.md): when a stabilizer circuit is long, first check the gate order and control qubits with a text diagram.
