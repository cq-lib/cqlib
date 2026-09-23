# DensityMatrix

`cqlib.qis.state`

The mixed-state simulator in `cqlib.qis.state`, covering the `DensityMatrix` class: state construction, gate application and quantum channels, partial traces, physicality validation, measurement and sampling, and expectation value computation.

## Import

```python
from cqlib.qis.state import DensityMatrix
```

---

## DensityMatrix

A simulator that describes an N-qubit quantum state with a 2^N × 2^N density matrix ρ. Unlike the statevector, which can only represent pure states, the density matrix can represent classical mixtures of pure states.

Data is stored contiguously in row-major order. Qubit i corresponds to bit i of the basis state index, with qubit 0 as the least significant bit; in bitstring representations, the higher-numbered qubit is on the left. After construction the state is |0…0⟩⟨0…0|.

### DensityMatrix(num_qubits)

Create a pure state in |0…0⟩⟨0…0|.

Parameters:

- `num_qubits` (`int`): the number of qubits.

Example:

```python
import math

from cqlib.qis.state import DensityMatrix

dm = DensityMatrix(2)
assert dm.num_qubits == 2

probs = dm.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)
assert math.isclose(probs[1], 0.0)
```

---

### Static methods

- `DensityMatrix.from_state(num_qubits, initial_state) -> DensityMatrix`: construct from pure-state amplitudes, taking the outer product ρ = |ψ⟩⟨ψ| internally. `initial_state` is a NumPy complex array or a list of complex numbers of length 2^num_qubits.
  - A `ValueError` is raised when the length does not match or the amplitudes are not normalized.
- `DensityMatrix.from_density_matrix(num_qubits, dm_state) -> DensityMatrix`: construct directly from flattened density matrix data.
  - `dm_state` (`numpy.ndarray | list`): a complex sequence of length 4^num_qubits.
  - A `ValueError` is raised when the length does not match, the trace is not 1, or it is not Hermitian or not positive semidefinite.
- `DensityMatrix.from_circuit(circuit) -> DensityMatrix`: simulate the evolution of the circuit starting from |0…0⟩⟨0…0| and return the resulting state. The input circuit is not modified.
  - A `ValueError` is raised when the circuit contains unsupported operations.
- `DensityMatrix.maximally_mixed(num_qubits) -> DensityMatrix`: construct the maximally mixed state I / 2^N, that is, a diagonal matrix whose diagonal entries are all 1/2^N. A `num_qubits` of 0 gives the 1×1 matrix `[1]`.

Example:

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import DensityMatrix

amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
dm = DensityMatrix.from_state(1, amps)
assert math.isclose(dm.probabilities()[0], 0.5)
assert math.isclose(dm.probabilities()[1], 0.5)

# |1><1|
data = np.array([0, 0, 0, 1], dtype=complex)
dm = DensityMatrix.from_density_matrix(1, data)
assert math.isclose(dm.probabilities()[1], 1.0)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = DensityMatrix.from_circuit(circuit)
assert math.isclose(bell.probabilities()[0], 0.5)
assert math.isclose(bell.probabilities()[3], 0.5)

mixed = DensityMatrix.maximally_mixed(2)
assert np.allclose(mixed.data, np.eye(4, dtype=complex) / 4, atol=1e-12)
assert math.isclose(mixed.trace(), 1.0)
```

---

### Attributes

- `num_qubits -> int`: the number of qubits.
- `data -> numpy.ndarray`: a two-dimensional complex matrix of shape (2^num_qubits, 2^num_qubits). Each read returns a copy; modifying the returned value does not affect the state.

Example:

```python
import numpy as np

from cqlib.qis.state import DensityMatrix

dm = DensityMatrix(2)
data = dm.data
assert data.shape == (4, 4)
assert data[0, 0] == 1.0
assert np.allclose(data[0, 1:], 0)

# data 是副本，就地修改不会写回状态
data[0, 0] = 999.0
assert dm.data[0, 0] == 1.0
```

---

### Methods

All gate application and state rewriting methods modify the current state in place and return `None`.

**Circuit application**

- `apply_circuit(circuit) -> None`: apply the circuit to the current state. A `ValueError` is raised when the number of circuit qubits does not match the state.

**Standard gate objects**

- `apply_standard_gate(gate, qubits, params=None) -> None`: apply a gate as described by `StandardGate`.
  - `gate` (`StandardGate`): the standard gate.
  - `qubits` (`list[int]`): the list of target qubit indices.
  - `params` (`list[float] | None`): the parameters of a parameterized gate, defaulting to an empty list.

**Single-qubit gates (no parameters)**

`apply_x`, `apply_y`, `apply_z`, `apply_h`, `apply_s`, `apply_sdg`, `apply_t`, `apply_tdg`, `apply_x2p`, `apply_x2m`, `apply_y2p`, `apply_y2m`, all with the signature `(qubit) -> None`.

**Single-qubit gates (with angle)**

- `apply_rx(qubit, theta)`, `apply_ry(qubit, theta)`, `apply_rz(qubit, theta)`: rotate by `theta` radians around the X, Y and Z axes.
- `apply_phase(qubit, theta)`: phase gate.
- `apply_xy(qubit, theta)`: XY gate.
- `apply_xy2p(qubit, theta)`: XY2P gate.
- `apply_xy2m(qubit, theta)`: XY2M gate.
- `apply_rxy(qubit, theta, phi)`: RXY gate.
- `apply_u(qubit, theta, phi, lambda_)`: general single-qubit unitary gate, `U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`.

**Global phase**

- `apply_gphase(phi) -> None`: has no observable effect on the density matrix; the state is unchanged after the call.

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
- `apply_ccx(c0, c1, target)`: CCX (Toffoli) gate.

**Custom matrix gates**

- `apply_single_qubit_gate(qubit, matrix) -> None`: apply the single-qubit gate given by a 2×2 complex matrix; a `ValueError` is raised when the shape does not match.
- `apply_double_qubits_gate(q0, q1, matrix) -> None`: apply the two-qubit gate given by a 4×4 complex matrix; a `ValueError` is raised when the shape does not match.
- `apply_unitary_gate(qubits, matrix) -> None`: apply a unitary gate on any number of qubits, acting on `qubits`, with the evolution ρ → U ρ U†. The matrix shape must be 2^n × 2^n, where n is the length of `qubits`.
  - `qubits` (`list[int]`): the list of qubit indices the gate acts on.
  - `matrix` (`numpy.ndarray`): the unitary matrix; a `ValueError` is raised when the shape does not match the number of qubits.

**Quantum channels**

- `apply_kraus(qubits, ops) -> None`: apply the quantum channel given by Kraus operators, with the evolution ρ → Σ_k K_k ρ K_k†.
  - `qubits` (`list[int]`): the list of qubit indices the channel acts on.
  - `ops` (`list[numpy.ndarray]`): the list of Kraus operators, each a flattened complex array. A `ValueError` is raised when a Kraus operator is invalid.

**Reduction and validation**

- `partial_trace(keep) -> DensityMatrix`: take the partial trace over the remaining qubits and return the reduced density matrix containing only the qubits given by `keep`.
  - `keep` (`list[int]`): the list of qubit indices to keep after reduction. An `IndexError` is raised when an index is out of range.
- `trace() -> float`: return the trace (the sum of the diagonal entries). The trace of a physical state is always 1. The real part of the result is taken.
- `is_hermitian(tol=1e-10) -> bool`: determine whether ρ = ρ† holds within the tolerance.
- `is_positive_semidefinite(tol=1e-10) -> bool`: determine whether all eigenvalues satisfy λᵢ ≥ -tol. For a Hermitian matrix this test is deterministic; for a non-Hermitian matrix or a matrix containing NaN or Inf it returns `False`.
- `validate_physical(tol=1e-10) -> None`: validate Hermiticity, positive semidefiniteness and unit trace at once. A `ValueError` is raised when any constraint is not satisfied.

**Expectation values**

- `expectation(observable) -> float`: compute Tr(ρ · O).
  - `observable` (`Hamiltonian | PauliString`): the observable. A `ValueError` is raised when the qubit count does not match or the type is not supported.

**Measurement and sampling**

- `probabilities() -> list[float]`: return the probability of each basis state, that is, the diagonal entries ρ_ii of the density matrix, with length 2^num_qubits. Does not modify the state.
- `reset(qubit) -> None`: measure the given qubit and flip it to |0⟩ if the result is 1.
- `measure(qubit) -> bool`: measure a single qubit, collapse the state, and return the result of that qubit.
- `measure_all() -> Outcome`: measure all qubits, collapse the state, and return an `Outcome`. The bitstring is ordered from the highest qubit number to the lowest.
- `sample_shots(shots) -> list[Outcome]`: sample `shots` times according to the current probability distribution, without modifying the state.
- `sample(measurement, shots) -> ExecutionResult`: sample according to a measurement receipt, without modifying the state.
  - `measurement` (`Measurement`): the measurement receipt obtained from a measurement operation of a circuit.
  - `shots` (`int`): the number of shots.
- `probs(measurement) -> dict[Outcome, float]`: compute marginal probabilities according to a measurement receipt, without modifying the state.

**Copying**

- `copy() -> DensityMatrix`: return an independent copy; modifying the original state does not affect the copy.

---

### Other behavior

- `repr(dm)` returns a string of the form `DensityMatrix(num_qubits=N, shape=(2^N, 2^N))`.
- `__eq__` is not implemented: `==` is the default object identity comparison and does not compare matrix contents.
- `__copy__` and `__deepcopy__` are not implemented: the standard library `copy.copy()` and `copy.deepcopy()` raise `TypeError`. Use the `copy()` method when a copy is needed.

---

### Raises

- `IndexError`: qubit index of `partial_trace` out of range.
- `ValueError`: other validation failures, including amplitude length mismatch or amplitudes not normalized, density matrix data length mismatch or violation of physical constraints, duplicate target qubits of a two-qubit gate, custom gate matrix shape mismatch, invalid Kraus operator, observable type or qubit count mismatch, and failure of `validate_physical`.
- `TypeError`: `from_state` and `from_density_matrix` received an input that is neither an array nor a list; the `tol` of `is_hermitian`, `is_positive_semidefinite` and `validate_physical` is not a floating-point number.

---

Example:

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis import Hamiltonian, PauliString
from cqlib.qis.state import DensityMatrix

# 用线路构造 Bell 态
circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

dm = DensityMatrix.from_circuit(circuit)
assert dm.is_hermitian() is True
assert dm.is_positive_semidefinite() is True
assert dm.validate_physical() is None
assert math.isclose(dm.trace(), 1.0, abs_tol=1e-10)

# 观测量
assert math.isclose(dm.expectation(PauliString.from_str("ZI")), 0.0, abs_tol=1e-10)

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 0.5)
hamiltonian.add_term(PauliString.from_str("XX"), 0.5)
hamiltonian.simplify()
assert math.isclose(dm.expectation(hamiltonian), 1.0, abs_tol=1e-10)

# 部分迹：Bell 态的单比特约化是最大混合态
reduced = dm.partial_trace([0])
assert reduced.num_qubits == 1
assert math.isclose(reduced.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(reduced.probabilities()[1], 0.5, abs_tol=1e-10)

# 量子信道：退极化信道保持迹为 1
noisy = DensityMatrix(1)
p = 0.3
k0 = np.sqrt(1 - p) * np.eye(2, dtype=complex)
k1 = np.sqrt(p / 3) * np.array([[0, 1], [1, 0]], dtype=complex)
k2 = np.sqrt(p / 3) * np.array([[0, -1j], [1j, 0]], dtype=complex)
k3 = np.sqrt(p / 3) * np.array([[1, 0], [0, -1]], dtype=complex)
noisy.apply_kraus([0], [k0.flatten(), k1.flatten(), k2.flatten(), k3.flatten()])
assert np.isclose(noisy.trace(), 1.0, atol=1e-10)

# 副本独立
snapshot = dm.copy()
dm.apply_h(0)
assert math.isclose(snapshot.probabilities()[0], 0.5, abs_tol=1e-10)
```

---

## Exceptions

The validation failures of `DensityMatrix` fall into three categories by source:

| Exception | When it occurs |
| --- | --- |
| `IndexError` | Qubit index of `partial_trace` out of range. |
| `ValueError` | Amplitude length inconsistent with 2^num_qubits, amplitudes not normalized, density matrix data length mismatch or violation of the trace, Hermiticity or positive semidefinite constraints, duplicate target qubits of a two-qubit gate, custom gate matrix shape mismatch, invalid Kraus operator, observable type or qubit count mismatch, or failure of `validate_physical`. |
| `TypeError` | `from_state` or `from_density_matrix` received an unsupported type; wrong type of the `tol` parameter. |
