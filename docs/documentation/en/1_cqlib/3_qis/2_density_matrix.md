# DensityMatrix Mixed-State Simulation

`DensityMatrix` represents quantum states with a density matrix. It can express pure states as well as mixed states and states after noise, suitable for Kraus channels, partial trace, physicality checks, mixed-state expectation values and entropy metric analysis.

The memory scale of a density matrix is `2^n × 2^n`, usually heavier than `Statevector`. It should be preferred only when mixed-state semantics, noise channels or subsystem analysis are needed.

---

## Task: prepare a `|+>` state with a density matrix

```python
from cqlib.qis import DensityMatrix

density = DensityMatrix(1)
density.apply_h(0)

print(density.data)
print(density.trace())
print(density.probabilities())
```

`data` returns the two-dimensional density matrix, `trace()` should be close to `1.0`, and `probabilities()` returns the diagonal probabilities in the computational basis. For the `|+>` state the probabilities are `[0.5, 0.5]`, but the density matrix also contains off-diagonal coherence terms.

---

## Creating from pure-state amplitudes or a circuit

`DensityMatrix.from_state()` takes a qubit count and pure-state amplitudes; `DensityMatrix.from_circuit()` applies the circuit to the initial `|0...0>` and returns the resulting density matrix.

```python
import numpy as np
from cqlib import Circuit
from cqlib.qis import DensityMatrix

plus = DensityMatrix.from_state(
    1,
    np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex),
)
print(plus.data)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell_density = DensityMatrix.from_circuit(circuit)
print(bell_density.probabilities())
```

If a complete density matrix is already available, `from_density_matrix()` can be used. The input matrix must satisfy the dimension, trace and physicality requirements.

```python
import numpy as np
from cqlib.qis import DensityMatrix

mixed = DensityMatrix.from_density_matrix(
    1,
    np.array(
        [
            0.7 + 0.0j, 0.0 + 0.0j,
            0.0 + 0.0j, 0.3 + 0.0j,
        ],
        dtype=complex,
    ),
)

print(mixed.probabilities())
```

---

## Applying Kraus noise

Kraus operators can be applied directly to a density matrix. The following example applies amplitude damping to the `|1>` state.

```python
import numpy as np
from cqlib.qis import DensityMatrix

gamma = 0.05
k0 = np.array([[1, 0], [0, np.sqrt(1 - gamma)]], dtype=complex)
k1 = np.array([[0, np.sqrt(gamma)], [0, 0]], dtype=complex)

density = DensityMatrix(1)
density.apply_x(0)

density.apply_kraus([0], [k0.flatten(), k1.flatten()])

print(density.probabilities())
```

Kraus channels are suitable for manually verifying the mathematical form of noise. If a device-level `NoiseModel` is already available, [DensityMatrixNoise noisy simulation](3_density_matrix_noise.md) can be used to add noise automatically after gates.

---

## Performing a partial trace on a subsystem

`partial_trace(keep=[...])` keeps the specified qubits and traces out the rest. It is suitable for analyzing a local subsystem within an entangled state.

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrix

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell_density = DensityMatrix.from_circuit(circuit)
subsystem = bell_density.partial_trace(keep=[0])

print(subsystem.data)
print(subsystem.probabilities())
```

Tracing out any single qubit of a Bell state yields a reduced density matrix close to the maximally mixed state. This result shows that a single qubit has no definite Bloch direction by itself, while the global two-qubit state still carries entanglement correlations.

---

## Physicality checks

When a density matrix comes from external data, numerical reconstruction or a custom noise channel, it should be checked whether it is still a valid quantum state.

```python
from cqlib.qis import DensityMatrix

density = DensityMatrix(1)
density.apply_h(0)

tol = 1e-10
print(density.is_hermitian(tol))
print(density.is_positive_semidefinite(tol))
density.validate_physical(tol)
```

A physical density matrix should at least be Hermitian, positive semidefinite and have a trace of `1`. If `validate_physical()` raises an exception, it usually indicates a problem in the input matrix, the noise channel or the numerical post-processing.

---

## Expectation values, measurement and sampling

`DensityMatrix.expectation()` likewise accepts a `PauliString` or a `Hamiltonian`. In addition, the density matrix provides state simulation interfaces such as `measure()`, `measure_all()`, `sample_shots()`, `sample()` and `probs()`.

```python
from cqlib.qis import DensityMatrix, Hamiltonian, PauliString

density = DensityMatrix(2)
density.apply_h(0)
density.apply_cx(0, 1)

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])

print(density.expectation(hamiltonian))
print(density.sample_shots(8))
```

Measurement methods change the current state; sampling methods are suitable for generating finite-shot results. When performing noise simulation or state metric analysis, it is recommended to save a `copy()` first, so that measurement collapse does not affect subsequent computation.

---

## Next steps

- [DensityMatrixNoise noisy simulation](3_density_matrix_noise.md): extend manual Kraus noise to automatic noisy circuit simulation based on `NoiseModel`.
- [Quantum state metrics and entropy](6_metrics_entropy.md): interpret density matrix results with purity, entropy, fidelity and entanglement metrics.
- [Visualizing quantum states](../5_visualization/7_state_visualization.md): inspect the coherence terms and Pauli expansion of a density matrix with state city and Pauli vector diagrams.
