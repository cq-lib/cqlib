# Statevector Pure-State Simulation

`Statevector` is used for local ideal pure-state simulation. It stores an `n`-qubit state as a complex amplitude vector of length `2^n`, suitable for verifying small and medium-scale unitary circuits, checking probability distributions, computing Pauli/Hamiltonian expectation values, and simulating finite-shot sampling.

`Statevector` is initialized to `|0...0>` by default. None of the examples connect to hardware or introduce noise.

---

## Task: prepare and read a Bell state

```python
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(state.data)
print(state.probabilities())
```

`data` returns the complex amplitude array, and `probabilities()` returns the computational-basis probability distribution. The ideal probabilities of a Bell state are concentrated on indices `0` and `3`, that is, `|00>` and `|11>`.

When reading `Statevector` results, three things should be confirmed:

- whether the length of the amplitude array is `2 ** num_qubits`;
- whether the sum of probabilities is close to `1.0`;
- whether the main probability peaks fall on the expected bitstrings.

---

## Constructing a state from initial amplitudes

If a set of normalized complex amplitudes is already available, `from_state()` can be used to construct a pure state directly.

```python
import numpy as np
from cqlib.qis import Statevector

plus = Statevector.from_state(
    1,
    np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex),
)

print(plus.probabilities())
```

The length of the passed array must be equal to `2 ** num_qubits`, and the state must be normalized. This entry point is suitable for converting external numerical results into Cqlib state objects, which can then be used to compute expectation values, fidelity or sampling results.

---

## Executing pure-state simulation from a Circuit

A more common usage is to construct a `Circuit` first and then use `from_circuit()` to execute the entire ideal circuit.

```python
from cqlib import Circuit
from cqlib.qis import Statevector

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

state = Statevector.from_circuit(circuit)
print(state.probabilities())
```

`Statevector.from_circuit()` is suitable for executing ideal circuit fragments that contain no non-unitary operations. If the circuit contains measurement, reset or dynamic control flow, first confirm whether these operations belong to the current simulation goal; when necessary, switch to step-by-step simulation, density matrix simulation or a dedicated dynamic execution flow.

---

## Applying a circuit in place to an existing state

When different circuit fragments need to be appended repeatedly starting from the same initial state, the state can be created first and `apply_circuit()` then called.

```python
from cqlib import Circuit
from cqlib.qis import Statevector

prefix = Circuit(2)
prefix.h(0)

suffix = Circuit(2)
suffix.cx(0, 1)

state = Statevector(2)
state.apply_circuit(prefix)
state.apply_circuit(suffix)

print(state.probabilities())
```

This style is suitable for debugging in segments. After each module is executed, probabilities can be printed or expectation values computed, in order to locate which segment of the circuit changed the state structure.

---

## Applying common quantum gates directly

`Statevector` provides `apply_*` methods corresponding to standard gates, including Pauli gates, Clifford gates, rotation gates, controlled gates, two-qubit rotation gates, `fSim`, `CCX` and user-defined unitary matrices.

```python
import numpy as np
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_x(0)
state.apply_rz(0, np.pi / 4)
state.apply_cx(0, 1)
state.apply_rzz(0, 1, 0.2)

print(state.probabilities())
```

For custom matrices, `apply_single_qubit_gate()`, `apply_double_qubits_gate()` or `apply_unitary_gate()` can be used.

```python
import numpy as np
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_x(0)

swap = np.array(
    [
        [1, 0, 0, 0],
        [0, 0, 1, 0],
        [0, 1, 0, 0],
        [0, 0, 0, 1],
    ],
    dtype=complex,
)
state.apply_unitary_gate([0, 1], swap)

print(state.probabilities())
```

When custom matrices are used, the matrix dimension must be ensured to match the number of qubits it acts on, and the unitarity requirement must be satisfied.

---

## Computing observable expectation values

`Statevector.expectation()` accepts a `PauliString` or a `Hamiltonian`.

```python
from cqlib.qis import Hamiltonian, PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

zz = PauliString.from_str("ZZ")
print(state.expectation(zz))

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])
print(state.expectation(hamiltonian))
```

`expectation_statevector(state)` can also be called from the observable side. In VQE or QAOA, it is recommended to settle on one style, so that state objects and observables are not confused in logs and post-processing code.

---

## Measurement, reset and sampling

`measure()` measures the specified qubit and collapses the current state, while `measure_all()` measures all qubits. `sample_shots()` is used to sample according to the current distribution and does not modify the original state, which makes it applicable to debugging scenarios.

```python
from cqlib.qis import Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

shots = state.sample_shots(1000)
counts = {}
for outcome in shots:
    bitstring = outcome.to_bitstring(state.num_qubits)
    counts[bitstring] = counts.get(bitstring, 0) + 1

print(counts)
print(state.probabilities())
```

If exact theoretical probabilities are needed, use `probabilities()`; if the statistical fluctuations of a finite number of shots need to be simulated, use `sample_shots()`.


---

## Next steps

- [DensityMatrix mixed-state simulation](2_density_matrix.md): switch from pure-state simulation to a density matrix when mixed states, Kraus noise and partial trace are needed.
- [Pauli, PauliString and Hamiltonian](5_pauli_and_hamiltonian.md): learn how to connect state results to observables, energy functions and Trotter evolution.
- [Visualizing quantum states](../5_visualization/7_state_visualization.md): interpret pure-state simulation results with Bloch, state city and Pauli vector diagrams.
