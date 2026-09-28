# DensityMatrixNoise

`cqlib.qis.state`

The noisy mixed-state simulator in `cqlib.qis.state`, covering the `DensityMatrixNoise` class: noise model binding, gate application with noise, and probabilities and sampling under readout noise.

## Import

```python
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise
```

---

## DensityMatrixNoise

A simulator that applies Kraus noise automatically according to a noise model on top of the density matrix kernel. After each gate application, the noise corresponding to that gate in the model takes effect on the state immediately; readout noise takes effect at the probability computation and sampling stages.

Data is stored contiguously in row-major order. Qubit i corresponds to bit i of the basis state index, with qubit 0 as the least significant bit; in bitstring representations, the higher-numbered qubit is on the left. After construction the state is |0…0⟩⟨0…0|.

### DensityMatrixNoise(num_qubits, noise_model=None)

Create a simulator. Omitting the noise model is equivalent to an ideal simulation.

Parameters:

- `num_qubits` (`int`): the number of qubits.
- `noise_model` (`NoiseModel | None`): the definition of gate noise and readout noise, defaulting to `None`.

Example:

```python
import math

from cqlib.device import NoiseModel
from cqlib.qis.state import DensityMatrixNoise

sim = DensityMatrixNoise(2)
assert sim.num_qubits == 2
assert sim.state.shape == (4, 4)

probs = sim.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)

with_model = DensityMatrixNoise(2, NoiseModel())
assert with_model.num_qubits == 2
```

---

### Static methods

- `DensityMatrixNoise.from_circuit(circuit, noise_model=None) -> DensityMatrixNoise`: simulate a circuit and return the simulator after evolution. The circuit is decomposed to basis gates before execution, and noise is applied after each gate according to the model. The input circuit is not modified.
  - `circuit` (`Circuit`): the circuit to simulate.
  - `noise_model` (`NoiseModel | None`): the noise model, defaulting to `None`.
  - A `ValueError` is raised when the circuit contains unsupported operations.

Example:

```python
import math

from cqlib import Circuit
from cqlib.device import NoiseModel
from cqlib.qis.state import DensityMatrixNoise

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

ideal = DensityMatrixNoise.from_circuit(circuit)
assert math.isclose(ideal.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(ideal.probabilities()[3], 0.5, abs_tol=1e-10)

noisy = DensityMatrixNoise.from_circuit(circuit, NoiseModel())
assert noisy.num_qubits == 2
```

---

### Attributes

- `num_qubits -> int`: the number of qubits.
- `state -> numpy.ndarray`: a two-dimensional complex matrix of shape (2^num_qubits, 2^num_qubits) representing the current state. Each read returns a copy; modifying the returned value does not affect the simulator.

Example:

```python
import math

from cqlib.qis.state import DensityMatrixNoise

sim = DensityMatrixNoise(2)
sim.apply_h(0)
sim.apply_cx(0, 1)

rho = sim.state
assert rho.shape == (4, 4)
assert math.isclose(abs(rho[0, 3]), 0.5, abs_tol=1e-10)

rho[0, 0] = 999.0
assert sim.state[0, 0] != 999.0
```

---

### Methods

All gate application and state rewriting methods modify the current state in place and return `None`. After application, each gate method applies the noise corresponding to that gate according to the noise model.

**Circuit application**

- `apply_circuit(circuit) -> None`: apply the circuit to the current state; the circuit is decomposed to basis gates before execution. A `ValueError` is raised when the number of circuit qubits does not match the state.

**Standard gate objects**

- `apply_standard_gate_noise(gate, qubits, params=None) -> None`: apply a gate as described by `StandardGate` and apply the corresponding noise.
  - `gate` (`StandardGate`): the standard gate.
  - `qubits` (`list[int]`): the list of target qubit indices.
  - `params` (`list[float] | None`): the parameters of a parameterized gate, defaulting to an empty list.

**Single-qubit gates (no parameters)**

`apply_x`, `apply_y`, `apply_z`, `apply_h`, `apply_s`, `apply_sdg`, `apply_t`, `apply_tdg`, `apply_x2p`, `apply_x2m`, `apply_y2p`, `apply_y2m`, all with the signature `(q) -> None`.

**Single-qubit gates (with angle)**

- `apply_rx(q, theta)`, `apply_ry(q, theta)`, `apply_rz(q, theta)`: rotate by `theta` radians around the X, Y and Z axes.
- `apply_phase(q, theta)`: phase gate.
- `apply_xy(q, theta)`: XY gate.
- `apply_xy2p(q, theta)`: XY2P gate.
- `apply_xy2m(q, theta)`: XY2M gate.
- `apply_rxy(q, theta, phi)`: arbitrary rotation on the Bloch sphere.
- `apply_u(q, theta, phi, lambda_)`: general single-qubit unitary gate, `U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`.

**Global phase**

- `apply_gphase(phi) -> None`: apply a global phase gate.

**Controlled gates and two-qubit gates**

- `apply_cx(control, target)`: controlled X (CNOT) gate.
- `apply_cy(control, target)`: controlled Y gate.
- `apply_cz(q0, q1)`: controlled Z gate.
- `apply_crx(control, target, theta)`: controlled RX gate.
- `apply_cry(control, target, theta)`: controlled RY gate.
- `apply_crz(control, target, theta)`: controlled RZ gate.
- `apply_swap(q0, q1)`: SWAP gate.
- `apply_rxx(q0, q1, theta)`: RXX (Ising XX) gate.
- `apply_ryy(q0, q1, theta)`: RYY (Ising YY) gate.
- `apply_rzz(q0, q1, theta)`: RZZ (Ising ZZ) gate.
- `apply_rzx(q0, q1, theta)`: RZX gate.
- `apply_fsim(q0, q1, theta, phi)`: fermionic simulation gate, where `theta` is the iSWAP angle and `phi` is the controlled phase angle.
- `apply_ccx(c1, c2, t)`: CCX (Toffoli) gate, where `c1` and `c2` are the control qubits and `t` is the target qubit.

**Custom matrix gates**

- `apply_unitary_gate(qubits, matrix) -> None`: apply a unitary gate on any number of qubits, acting on `qubits`. This entry point does not apply noise.
  - `qubits` (`list[int]`): the list of qubit indices the gate acts on.
  - `matrix` (`numpy.ndarray`): a 2^n × 2^n complex matrix, where n is the length of `qubits`; a `ValueError` is raised when the shape does not match the number of qubits.

**Expectation values**

- `expectation(observable) -> float`: compute the expectation value on the noisy state.
  - `observable` (`Hamiltonian | PauliString`): the observable. A `ValueError` is raised when the qubit count does not match or the type is not supported.

**Probabilities**

- `probabilities() -> list[float]`: return the probability of each basis state without readout noise, with length 2^num_qubits. Does not modify the state.
- `probabilities_with_readout(qubits) -> list[float]`: return the probability distribution after readout noise is applied on the given qubits, with length 2^n, where n is the length of `qubits`.
  - `qubits` (`list[int]`): the list of qubit indices taking part in readout noise modeling. An `IndexError` is raised when an index is out of range.

**Measurement and sampling**

- `measure(qubit) -> bool`: measure a single qubit, collapse the state, and return the result of that qubit.
- `measure_all() -> Outcome`: measure all qubits, collapse the state, and return an `Outcome`. The bitstring is ordered from the highest qubit number to the lowest.
- `sample_shots(shots) -> list[Outcome]`: sample `shots` times according to the distribution including readout noise, without modifying the state.
- `sample(measurement, shots) -> ExecutionResult`: sample according to a measurement receipt, without modifying the state.
  - `measurement` (`Measurement`): the measurement receipt obtained from a measurement operation of a circuit.
  - `shots` (`int`): the number of shots.
- `probs(measurement) -> dict[Outcome, float]`: compute marginal probabilities without readout noise according to a measurement receipt, without modifying the state.
- `probs_with_readout(measurement) -> dict[Outcome, float]`: compute marginal probabilities with readout noise applied according to a measurement receipt, without modifying the state.

**Copying**

- `copy() -> DensityMatrixNoise`: return an independent copy, with the noise model copied as well. Modifying the original simulator does not affect the copy.

---

### Other behavior

- `repr(sim)` returns a string of the form `DensityMatrixNoise(num_qubits=N, state_shape=(2^N, 2^N))`.
- `__eq__` is not implemented: `==` is the default object identity comparison and does not compare state contents.
- `__copy__` and `__deepcopy__` are not implemented: the standard library `copy.copy()` and `copy.deepcopy()` raise `TypeError`. Use the `copy()` method when a copy is needed.

---

### Raises

- `IndexError`: qubit index out of range.
- `ValueError`: other validation failures, including custom gate matrix shape inconsistent with the number of qubits, observable type or qubit count mismatch, circuit containing unsupported operations, and circuit qubit count inconsistent with the state.
- `OverflowError`: `num_qubits` is negative.

---

Example:

```python
import math

import numpy as np

from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError, SingleQubitNoise
from cqlib.qis.state import DensityMatrixNoise

# 理想模拟：不带噪声模型时与纯密度矩阵结果一致
ideal = DensityMatrixNoise(2)
ideal.apply_h(0)
ideal.apply_cx(0, 1)
assert math.isclose(ideal.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(ideal.probabilities()[3], 0.5, abs_tol=1e-10)

# 比特翻转噪声：X 门后以 p=0.1 翻转
noise_model = NoiseModel()
noise_model.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.1))

sim = DensityMatrixNoise(1, noise_model)
sim.apply_x(0)
probs = sim.probabilities()
assert math.isclose(probs[1], 0.9, abs_tol=0.01)
assert math.isclose(probs[0], 0.1, abs_tol=0.01)

# 理想概率与读出噪声下的概率相互独立
circuit_readout = NoiseModel()
circuit_readout.add_readout_error(0, ReadoutError(p_0_given_1=0.1, p_1_given_0=0.2))

readout_sim = DensityMatrixNoise(1, circuit_readout)
readout_sim.apply_x(0)
assert readout_sim.probabilities() == [0.0, 1.0]
readout_probs = readout_sim.probabilities_with_readout([0])
assert math.isclose(readout_probs[0], 0.1, abs_tol=1e-10)
assert math.isclose(readout_probs[1], 0.9, abs_tol=1e-10)

# 副本独立，且保留噪声行为
snapshot = sim.copy()
sim.apply_h(0)
assert math.isclose(snapshot.probabilities()[1], 0.9, abs_tol=0.01)

# 自定义酉门入口不施加噪声
matrix = np.array(
    [
        [1 / np.sqrt(2), 0, 1 / np.sqrt(2), 0],
        [0, 1 / np.sqrt(2), 0, 1 / np.sqrt(2)],
        [0, 1 / np.sqrt(2), 0, -1 / np.sqrt(2)],
        [1 / np.sqrt(2), 0, -1 / np.sqrt(2), 0],
    ],
    dtype=complex,
)
plain = DensityMatrixNoise(2)
plain.apply_unitary_gate([0, 1], matrix)
assert math.isclose(plain.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(np.trace(plain.state @ plain.state).real, 1.0, abs_tol=1e-10)
```

---

## Exceptions

The validation failures of `DensityMatrixNoise` fall into three categories by source:

| Exception | When it occurs |
| --- | --- |
| `IndexError` | Qubit index out of range, including `qubits` of `probabilities_with_readout` out of range. |
| `ValueError` | Custom gate matrix shape inconsistent with the number of qubits, observable type or qubit count mismatch, circuit qubit count inconsistent with the state, or circuit containing unsupported operations. |
| `OverflowError` | `num_qubits` is negative. |
