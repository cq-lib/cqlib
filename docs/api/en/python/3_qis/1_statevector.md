# Statevector

`cqlib.qis.state`

The pure-state simulator in `cqlib.qis.state`, covering the `Statevector` class: state construction, gate application, measurement and sampling, and expectation value computation.

## Import

```python
from cqlib.qis.state import Statevector
```

---

## Statevector

A simulator that describes an N-qubit pure quantum state |ψ⟩ = Σᵢ αᵢ|i⟩ with 2^N complex amplitudes.

Amplitudes are stored contiguously by index in memory, and the amplitude of the basis state |i⟩ is at index `i`. Qubit i corresponds to bit i of the index, with qubit 0 as the least significant bit; in bitstring representations, the higher-numbered qubit is on the left. After construction all amplitudes are 0 except |0…0⟩, which is 1.

### Statevector(num_qubits)

Create a state in |0…0⟩.

Parameters:

- `num_qubits` (`int`): the number of qubits.

Example:

```python
import math

from cqlib.qis.state import Statevector

sv = Statevector(2)
assert sv.num_qubits == 2

probs = sv.probabilities()
assert len(probs) == 4
assert math.isclose(probs[0], 1.0)
assert math.isclose(probs[1], 0.0)
```

---

### Static methods

- `Statevector.from_state(num_qubits, initial_state) -> Statevector`: construct from the given amplitudes. `initial_state` is a NumPy complex array of length 2^num_qubits, or a list of complex numbers and floats.
  - A `ValueError` is raised when the length does not match 2^num_qubits or the amplitudes are not normalized.
- `Statevector.from_circuit(circuit) -> Statevector`: simulate the evolution of the circuit starting from |0…0⟩ and return the resulting state. The input circuit is not modified.
  - A `ValueError` is raised when the circuit contains non-unitary operations or the number of circuit qubits does not match the state.

Example:

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import Statevector

amps = np.array([1 / np.sqrt(2), 1 / np.sqrt(2)], dtype=complex)
sv = Statevector.from_state(1, amps)
assert math.isclose(sv.probabilities()[0], 0.5)
assert math.isclose(sv.probabilities()[1], 0.5)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = Statevector.from_circuit(circuit)
assert math.isclose(bell.probabilities()[0], 0.5, abs_tol=1e-10)
assert math.isclose(bell.probabilities()[3], 0.5, abs_tol=1e-10)
```

---

### Attributes

- `num_qubits -> int`: the number of qubits.
- `data -> numpy.ndarray`: a one-dimensional complex amplitude array of length 2^num_qubits. Each read returns a copy; modifying the returned value does not affect the state.

Example:

```python
import numpy as np

from cqlib.qis.state import Statevector

sv = Statevector(2)
data = sv.data
assert len(data) == 4
assert data[0] == 1.0
assert np.allclose(data[1:], 0)

# data 是副本，就地修改不会写回状态
data[0] = 999.0
assert sv.data[0] == 1.0
```

---

### Methods

All gate application and state rewriting methods modify the current state in place and return `None`.

**Circuit application**

- `apply_circuit(circuit) -> None`: apply the circuit to the current state. A `ValueError` is raised when the number of circuit qubits does not match.

**Standard gate objects**

- `apply_standard_gate(gate, qubits, params=None) -> None`: apply a gate as described by `StandardGate`.
  - `gate` (`StandardGate`): the standard gate.
  - `qubits` (`list[int]`): the list of target qubit indices.
  - `params` (`list[float] | None`): the parameters of a parameterized gate, defaulting to an empty list.

**Single-qubit gates (no parameters)**

`apply_x`, `apply_y`, `apply_z`, `apply_h`, `apply_s`, `apply_sdg`, `apply_t`, `apply_tdg`, `apply_x2p`, `apply_x2m`, `apply_y2p`, `apply_y2m`, all with the signature `(qubit) -> None`.

**Single-qubit gates (with angle)**

- `apply_rx(qubit, theta)`, `apply_ry(qubit, theta)`, `apply_rz(qubit, theta)`: rotate by `theta` radians around the X, Y and Z axes.
- `apply_phase(qubit, theta)`: phase gate, where `theta` is the phase angle.
- `apply_xy(qubit, theta)`: XY gate.
- `apply_xy2p(qubit, theta)`: XY2P gate, equivalent to `Rz(θ - π/2) Ry(π/2) Rz(π/2 - θ)`.
- `apply_xy2m(qubit, theta)`: XY2M gate, equivalent to `Rz(-θ + π/2) Ry(-π/2) Rz(-π/2 + θ)`.
- `apply_rxy(qubit, theta, phi)`: RXY gate.
- `apply_u(qubit, theta, phi, lambda_)`: general single-qubit unitary gate, `U(θ, φ, λ) = Rz(φ) Ry(θ) Rz(λ)`.

**Global phase**

- `apply_gphase(phi)`: multiply the whole state by the global phase `e^{iφ}`. Does not change `probabilities()`.

**Pauli string rotations**

- `apply_pauli_rotation(pauli, theta)`: apply `exp(-i θ/2 · P)`, where `P` must be a Hermitian Pauli string covering all qubits of the state.

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

A `ValueError` is raised when the two qubit indices of a two-qubit gate are the same.

**Custom matrix gates**

- `apply_single_qubit_gate(qubit, matrix) -> None`: apply the single-qubit gate given by `matrix`.
  - `matrix` (`numpy.ndarray`): a 2×2 complex matrix; a `ValueError` is raised when the shape does not match.
- `apply_double_qubits_gate(q0, q1, matrix) -> None`: apply the two-qubit gate given by `matrix`.
  - `matrix` (`numpy.ndarray`): a 4×4 complex matrix; a `ValueError` is raised when the shape does not match.
- `apply_unitary_gate(qubits, matrix) -> None`: apply a unitary gate on any number of qubits, acting on `qubits`.
  - `qubits` (`list[int]`): the list of qubit indices the gate acts on.
  - `matrix` (`numpy.ndarray`): a 2^n × 2^n complex matrix, where n is the length of `qubits`; a `ValueError` is raised when the shape does not match.

**Expectation values**

- `expectation(observable) -> float`: compute ⟨ψ|O|ψ⟩.
  - `observable` (`Hamiltonian | PauliString`): the observable. A `ValueError` is raised when the qubit count does not match or the type is not supported.

**Measurement and state rewriting**

- `probabilities() -> list[float]`: return the probability p(i) = |αᵢ|² of each basis state, with length 2^num_qubits. Does not modify the state.
- `reset(qubit) -> None`: measure the given qubit and flip it to |0⟩ if the result is 1.
- `measure(qubit) -> bool`: measure a single qubit, collapse the state, and return the result of that qubit.
- `measure_all() -> Outcome`: measure all qubits, collapse the state, and return an `Outcome`. The bitstring is ordered from the highest qubit number to the lowest.
- `sample_shots(shots) -> list[Outcome]`: sample `shots` times according to the current probability distribution, without modifying the state.
- `sample(measurement, shots) -> ExecutionResult`: sample according to a measurement receipt, without modifying the state.
  - `measurement` (`Measurement`): the measurement receipt obtained from a measurement operation of a circuit.
  - `shots` (`int`): the number of shots.
- `probs(measurement) -> dict[Outcome, float]`: compute marginal probabilities according to a measurement receipt, without modifying the state.

**Copying**

- `copy() -> Statevector`: return an independent copy; modifying the original state does not affect the copy.

---

### Other behavior

- `repr(sv)` returns a string of the form `Statevector(num_qubits=N, amplitudes=2^N)`.
- `__eq__` is not implemented: `==` is the default object identity comparison and does not compare state contents.
- `__copy__` and `__deepcopy__` are not implemented: the standard library `copy.copy()` and `copy.deepcopy()` raise `TypeError`. Use the `copy()` method when a copy is needed.

---

### Raises

- `IndexError`: qubit index out of range.
- `ValueError`: other validation failures, including amplitude length mismatch or amplitudes not normalized, duplicate target qubits of a two-qubit gate, custom gate matrix shape mismatch, observable type or qubit count mismatch, and circuit qubit count mismatch in `apply_circuit`.

---

Example:

```python
import math

import numpy as np

from cqlib import Circuit
from cqlib.qis import PauliString
from cqlib.qis.state import Statevector

# 用线路构造 Bell 态
circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

sv = Statevector.from_circuit(circuit)
probs = sv.probabilities()
assert math.isclose(probs[0], 0.5, abs_tol=1e-10)
assert math.isclose(probs[3], 0.5, abs_tol=1e-10)

# 期望值
assert math.isclose(sv.expectation(PauliString.from_str("ZZ")), 1.0, abs_tol=1e-10)

# 采样不改变状态，结果只落在关联基态上
shots = sv.sample_shots(64)
assert len(shots) == 64
assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
assert math.isclose(sv.probabilities()[0], 0.5, abs_tol=1e-10)

# 测量并坍缩：先在基态上验证确定性结果
basis = Statevector(1)
basis.apply_x(0)
assert basis.measure(0) is True
assert math.isclose(basis.probabilities()[1], 1.0, abs_tol=1e-10)

# 全量测量后概率集中在单个基态上
sv.measure_all()
assert math.isclose(sum(sv.probabilities()), 1.0, abs_tol=1e-10)

# 副本独立：全局相位不改变概率分布
clone = sv.copy()
clone.apply_gphase(np.pi / 2)
assert np.allclose(sv.probabilities(), clone.probabilities(), atol=1e-10)
```

---

## Exceptions

The validation failures of `Statevector` fall into two categories by source:

| Exception | When it occurs |
| --- | --- |
| `IndexError` | Qubit index out of range. |
| `ValueError` | Amplitude length inconsistent with 2^num_qubits, amplitudes not normalized, duplicate target qubits of a two-qubit gate, custom gate matrix shape mismatch, observable type or qubit count mismatch, circuit qubit count mismatch, or circuit containing non-unitary operations. |
