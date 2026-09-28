# DensityMatrixNoise Noisy Simulation

`DensityMatrixNoise` is a density matrix simulator with a noise model. When executing quantum gates it injects single-qubit gate noise, two-qubit gate noise and readout error automatically according to the `NoiseModel`, suitable for comparing the probabilities, expectation values and sampling results of ideal and noisy circuits.

The underlying representation is still a density matrix, so the scale grows quickly with the number of qubits. It is recommended to verify the noise configuration with a small circuit first, and then extend it to algorithm experiments.

---

## Task: add gate noise to a Bell state

```python
from cqlib import Circuit
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(
    StandardGate.H,
    0,
    SingleQubitNoise.depolarizing(0.001),
)
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.01),
)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

simulator = DensityMatrixNoise.from_circuit(circuit, noise_model)
print(simulator.probabilities())
```

An ideal Bell state has main peaks only on `00` and `11`. After noise is added, `01` and `10` may show non-zero probabilities. When reading a diagram or values, first confirm which gates and which qubits the noise model is configured on.

---

## Configuring common noise types

`NoiseModel` can add single-qubit gate noise, two-qubit gate noise and readout error separately.

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise, TwoQubitNoise

noise_model = NoiseModel()

noise_model.add_single_qubit_error(
    StandardGate.RY,
    0,
    SingleQubitNoise.bit_flip(0.002),
)
noise_model.add_single_qubit_error(
    StandardGate.H,
    1,
    SingleQubitNoise.phase_flip(0.001),
)
noise_model.add_two_qubit_error(
    StandardGate.CX,
    0,
    1,
    TwoQubitNoise.depolarizing(0.01),
)
noise_model.add_readout_error(
    0,
    ReadoutError(p_0_given_1=0.02, p_1_given_0=0.01),
)
```

Single-qubit noise and two-qubit noise affect the evolution of the quantum state; readout error affects the observed probabilities or sampling results, and is not equivalent to decoherence of the quantum state after a gate.

---

## Distinguishing state probabilities from readout probabilities

`probabilities()` returns the quantum state probabilities without readout error; `probabilities_with_readout(qubits)` superimposes readout error on the specified measurement qubits.

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_readout_error(0, ReadoutError(0.02, 0.03))

simulator = DensityMatrixNoise(1, noise_model)
simulator.apply_x(0)

print("state probabilities:", simulator.probabilities())
print("readout probabilities:", simulator.probabilities_with_readout([0]))
```

To analyze only the real quantum state after gate noise, inspect `probabilities()` or the underlying density matrix; to simulate experimental readout, inspect the probabilities with readout or the sampling interfaces.

---

## Executing noisy simulation gate by gate

Besides `from_circuit()`, the `apply_*` methods can also be called gate by gate, as with a state simulator. After each gate is executed, the corresponding noise is looked up from the current `NoiseModel`.

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise
from cqlib.qis import DensityMatrixNoise

noise_model = NoiseModel()
noise_model.add_single_qubit_error(
    StandardGate.X,
    0,
    SingleQubitNoise.bit_flip(0.01),
)

simulator = DensityMatrixNoise(1, noise_model)
simulator.apply_x(0)

print(simulator.probabilities())
```

Gate-by-gate execution is suitable for locating the source of noise. In a complex circuit, if the result is abnormal, the circuit can be split into several segments, and the probabilities or expectation values checked after each segment.

---

## Expectation values and sampling

`DensityMatrixNoise` supports expectation value and sampling interfaces similar to those of `Statevector` and `DensityMatrix`.

```python
from cqlib import Circuit
from cqlib.qis import DensityMatrixNoise, Hamiltonian, PauliString

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

simulator = DensityMatrixNoise.from_circuit(circuit)

observable = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
])

print(simulator.expectation(observable))
print(simulator.sample_shots(16))
```

If readout error is configured in the `NoiseModel`, an ordinary expectation value usually denotes the expectation value of the quantum state after noisy evolution; observed results with readout error should be handled separately through the readout probabilities or the corresponding sampling interfaces.


---

## Next steps

- [DensityMatrix mixed-state simulation](2_density_matrix.md): return to the density matrix itself, to understand Kraus channels, partial trace and physicality checks.
- [Visualizing execution results](../5_visualization/6_result_visualization.md): plot noisy sampling results as a histogram or probability distribution, and inspect the main peaks and the long tail.
- [Noise model](../2_device/4_noise.md): learn how `NoiseModel`, gate noise and readout error are modeled in the device module.
