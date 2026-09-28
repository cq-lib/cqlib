# Quantum State Metrics and Entropy

`cqlib.qis.metrics` and `cqlib.qis.entropy` provide quantum state similarity, mixedness, entropy and entanglement metrics. They are commonly used to compare ideal and noisy states, to judge whether a density matrix has decohered, to analyze subsystem entanglement, and to generate auxiliary evaluation metrics for algorithm experiments.

Before use, first confirm whether the input object is a `Statevector` or a `DensityMatrix`. The function names for pure-state metrics and mixed-state metrics differ, and the two should not be mixed up.

---

## Overview of common functions

| Module | Function | Input | Meaning |
|---|---|---|---|
| `metrics` | `purity_pure` | `Statevector` | Pure-state purity |
| `metrics` | `purity_mixed` | `DensityMatrix` | Mixed-state purity `Tr(ρ²)` |
| `metrics` | `state_fidelity_pure` | Two `Statevector` | Pure-state fidelity |
| `metrics` | `state_fidelity_pure_mixed` | `Statevector`, `DensityMatrix` | Pure and mixed state fidelity |
| `metrics` | `state_fidelity_mixed` | Two `DensityMatrix` | Mixed-state fidelity |
| `metrics` | `trace_distance_pure` | Two `Statevector` | Pure-state trace distance |
| `metrics` | `trace_distance_mixed` | Two `DensityMatrix` | Mixed-state trace distance |
| `metrics` | `entropy` | `DensityMatrix` | Von Neumann entropy |
| `metrics` | `partial_transpose` | `DensityMatrix`, qubit list | Partial transpose |
| `metrics` | `logarithmic_negativity` | `DensityMatrix`, subsystem | Logarithmic negativity |
| `entropy` | `linear_entropy` | `DensityMatrix` | Linear entropy |
| `entropy` | `renyi_entropy` | `DensityMatrix`, `alpha` | Rényi entropy |
| `entropy` | `entanglement_entropy_pure` | `Statevector`, subsystem | Pure-state entanglement entropy |
| `entropy` | `negativity` | `DensityMatrix`, subsystem | Negativity |
| `entropy` | `concurrence` | two-qubit `DensityMatrix` | Two-qubit concurrence |
| `entropy` | `entanglement_of_formation` | two-qubit `DensityMatrix` | Entanglement of formation |

---

## Task: compare two pure states

```python
from cqlib.qis import Statevector, metrics

plus = Statevector(1)
plus.apply_h(0)

minus = Statevector(1)
minus.apply_h(0)
minus.apply_z(0)

print(metrics.purity_pure(plus))
print(metrics.state_fidelity_pure(plus, minus))
print(metrics.trace_distance_pure(plus, minus))
```

The closer the fidelity is to `1`, the more similar the two states are; the closer the trace distance is to `0`, the more similar the two states are. For orthogonal pure states, the fidelity should be close to `0` and the trace distance should be close to `1`.

---

## Analyzing mixed-state purity and entropy

```python
import numpy as np
from cqlib.qis import DensityMatrix, entropy, metrics

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

print(metrics.purity_mixed(mixed))
print(metrics.entropy(mixed))
print(entropy.linear_entropy(mixed))
print(entropy.renyi_entropy(mixed, alpha=2.0))
```

The closer the purity is to `1`, the closer the state is to a pure state; the larger the entropy and the linear entropy, the more mixed the state. When interpreting noisy results, it is recommended to inspect both the probability distribution and the purity/entropy metrics, because similar probabilities do not necessarily mean the same coherence structure.

---

## Computing the entanglement entropy of a Bell state

For pure states, the entanglement entropy can be obtained by reducing over a subsystem.

```python
from cqlib.qis import Statevector, entropy

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(entropy.entanglement_entropy_pure(state, [0]))
```

The reduced density matrix of any single-qubit subsystem of a Bell state is exactly the maximally mixed state I/2, so the entanglement entropy is exactly `1` bit. For a product state, the entanglement entropy is exactly `0`.

---

## Computing negativity and logarithmic negativity

Negativity or logarithmic negativity is commonly used for mixed-state entanglement analysis. The following example first constructs a density matrix from a Bell state circuit, and then computes the entanglement metric of subsystem `[0]`.

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrix, entropy, metrics

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

density = DensityMatrix.from_circuit(circuit)

print(entropy.negativity(density, [0]))
print(metrics.logarithmic_negativity(density, sys_a=[0]))
```

A negativity of `0` usually means that no entanglement is detected by the PPT criterion under the corresponding partition; a larger negativity means that the negative eigenvalues appearing after the partial transpose are more pronounced.

---

## Two-qubit concurrence and entanglement of formation

`concurrence()` and `entanglement_of_formation()` target two-qubit density matrices.

```python
from cqlib.qis import DensityMatrix, entropy

state = DensityMatrix(2)
state.apply_h(0)
state.apply_cx(0, 1)

print(entropy.concurrence(state))
print(entropy.entanglement_of_formation(state))
```

These two metrics are suitable for two-qubit entanglement analysis. For systems with more qubits, use entanglement entropy, negativity, logarithmic negativity or other metrics defined by subsystem partition instead.

---

## Comparing ideal and noisy states

The following example compares an ideal Bell state with a noisy density matrix.

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, TwoQubitNoise
from cqlib.qis import DensityMatrix, DensityMatrixNoise, Statevector, metrics

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

ideal_state = Statevector.from_circuit(circuit)
ideal_density = DensityMatrix.from_circuit(circuit)

noise_model = NoiseModel()
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.02),
)
noisy = DensityMatrixNoise.from_circuit(circuit, noise_model)
noisy_density = DensityMatrix.from_density_matrix(2, noisy.state.reshape(-1))

print(metrics.state_fidelity_pure_mixed(ideal_state, noisy_density))
print(metrics.trace_distance_mixed(ideal_density, noisy_density))
print(metrics.purity_mixed(noisy_density))
```

Such metrics are suitable for answering "how far the noisy state is from the ideal state". Looking only at the final counts may ignore changes in coherence terms, phase and mixedness.

---

## Next steps

- [DensityMatrix mixed-state simulation](2_density_matrix.md): return to density matrix construction, partial trace and physicality verification, to understand where the metric inputs come from.
- [DensityMatrixNoise noisy simulation](3_density_matrix_noise.md): use noisy simulation to generate the mixed-state results to be compared.
- [Visualizing quantum states](../5_visualization/7_state_visualization.md): interpret numerical metrics together with Bloch, state city and Pauli vector diagrams.
