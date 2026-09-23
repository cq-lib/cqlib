# Stabilizer

`cqlib.qis.state`

The stabilizer simulator in `cqlib.qis.state`, covering the two classes `StabilizerState` and `StabilizerCircuitResult`: simulation of Clifford circuits, stabilizer generator queries, measurement and sampling, and reading of runtime classical data after execution.

## Import

```python
from cqlib.qis.state import StabilizerCircuitResult, StabilizerState
```

---

## StabilizerState

A simulator that describes an N-qubit Clifford state with a stabilizer table. Clifford gates map the Pauli group onto itself, so the state can be represented compactly by N commuting Pauli generators instead of storing all amplitudes.

This class accepts only Clifford gates. A circuit containing non-Clifford operations such as T gates or rotations by general angles cannot be simulated, and a `ValueError` is raised in that case.

Qubit i corresponds to bit i of the basis state index, with qubit 0 as the least significant bit; in bitstring representations, the higher-numbered qubit is on the left. After construction the state is |0…0⟩.

### StabilizerState(num_qubits)

Create a state in |0…0⟩.

Parameters:

- `num_qubits` (`int`): the number of qubits.

Example:

```python
from cqlib.qis.state import StabilizerState

state = StabilizerState(3)
assert state.num_qubits == 3
assert state.probabilities() == [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
assert "StabilizerState" in repr(state)
```

---

### Static methods

- `StabilizerState.from_circuit(circuit) -> StabilizerState`: simulate the evolution of a Clifford circuit and return the final state. The input circuit is not modified. A `ValueError` is raised when the circuit contains non-Clifford operations.
- `StabilizerState.run_circuit(circuit) -> StabilizerCircuitResult`: execute a Clifford circuit and return a result object carrying both the final state and the runtime classical data. The input circuit is not modified.
  - A `ValueError` is raised when the circuit contains control flow or non-Clifford operations.

Example:

```python
from cqlib import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

bell = StabilizerState.from_circuit(circuit)
assert bell.num_qubits == 2
assert bell.probabilities()[0] == 0.5
assert bell.probabilities()[3] == 0.5
```

---

### Attributes

- `num_qubits -> int`: the number of qubits.

---

### Methods

All gate application and state rewriting methods modify the current state in place and return `None`.

**Circuit application**

- `apply_circuit(circuit) -> None`: apply a Clifford circuit to the current state. A `ValueError` is raised when the circuit contains non-Clifford operations or the number of qubits does not match.

**Single-qubit Clifford gates**

`apply_h`, `apply_s`, `apply_sdg`, `apply_x`, `apply_y`, `apply_z`, `apply_x2p`, `apply_x2m`, `apply_y2p`, `apply_y2m`, all with the signature `(qubit) -> None`.

**Two-qubit Clifford gates**

- `apply_cx(control, target)`: controlled X gate.
- `apply_cy(control, target)`: controlled Y gate.
- `apply_cz(q0, q1)`: controlled Z gate.
- `apply_swap(q0, q1)`: SWAP gate.

A `ValueError` is raised when the two qubit indices are the same.

**Stabilizer table queries**

- `get_stabilizers() -> list[PauliString]`: return the N stabilizer generators.
- `get_destabilizers() -> list[PauliString]`: return the N destabilizer generators.
- `to_stim_format() -> str`: return a line-by-line text representation of the stabilizer generators, one generator per line with a sign prefix, for example `+XX`.

**Expectation values**

- `pauli_expectation(pauli) -> int`: return the expectation value of a Pauli string, which is `-1`, `0` or `1`. A `ValueError` is raised when the qubit count does not match.

**Probabilities**

- `probabilities() -> list[float]`: return the probability distribution over basis states, with length 2^num_qubits. Does not modify the state.
- `probability_of(bits) -> float`: return the probability of the given computational basis bitstring.
  - `bits` (`list[bool]`): the value of each qubit given by qubit number. A `ValueError` is raised when the length does not match `num_qubits`.

**Measurement and sampling**

- `reset(qubit) -> None`: measure the given qubit and flip it to |0⟩ if the result is 1.
- `measure(qubit) -> bool`: measure a single qubit, collapse the state, and return the result of that qubit.
- `measure_all() -> Outcome`: measure all qubits, collapse the state, and return an `Outcome`. The bitstring is ordered from the highest qubit number to the lowest.
- `sample_shots(shots) -> list[Outcome]`: sample `shots` times according to the current probability distribution, without modifying the state.
- `sample(measurement, shots) -> ExecutionResult`: sample according to a measurement receipt, without modifying the state.
  - `measurement` (`Measurement`): the measurement receipt obtained from a measurement operation of a circuit.
  - `shots` (`int`): the number of shots.
- `probs(measurement) -> dict[Outcome, float]`: compute marginal probabilities according to a measurement receipt, without modifying the state.

**Copying**

- `copy() -> StabilizerState`: return an independent copy; modifying the original state does not affect the copy.

---

### Other behavior

- `repr(state)` returns a string of the form `StabilizerState(num_qubits=N)`.
- `__eq__` is not implemented: `==` is the default object identity comparison and does not compare stabilizer table contents.
- `__copy__` and `__deepcopy__` are not implemented: the standard library `copy.copy()` and `copy.deepcopy()` raise `TypeError`. Use the `copy()` method when a copy is needed.

---

### Raises

- `IndexError`: qubit index out of range.
- `ValueError`: non-Clifford operations, circuit containing control flow, duplicate target qubits of a two-qubit gate, bitstring length of `probability_of` inconsistent with `num_qubits`, qubit count mismatch of `pauli_expectation`, or circuit qubit count inconsistent with the state.
- `OverflowError`: `num_qubits` is negative.

---

Example:

```python
import math

from cqlib.qis import PauliString
from cqlib.qis.state import StabilizerState

state = StabilizerState(2)
state.apply_h(0)
state.apply_cx(0, 1)

# 概率与单位置概率
probs = state.probabilities()
assert probs[0] == 0.5
assert probs[3] == 0.5
assert math.isclose(state.probability_of([False, False]), 0.5, abs_tol=1e-10)
assert math.isclose(state.probability_of([True, False]), 0.0, abs_tol=1e-10)

# 稳定子表
stabilizers = state.get_stabilizers()
destabilizers = state.get_destabilizers()
assert len(stabilizers) == 2
assert len(destabilizers) == 2
assert all(isinstance(pauli, PauliString) for pauli in stabilizers)
assert state.to_stim_format().count("\n") == 2

# 观测值
assert state.pauli_expectation(PauliString.from_str("ZZ")) == 1
assert state.pauli_expectation(PauliString.from_str("ZI")) == 0

# 采样不改变状态，结果只落在关联基态上
shots = state.sample_shots(64)
assert len(shots) == 64
assert {outcome.to_bitstring(2) for outcome in shots} <= {"00", "11"}
assert state.probabilities()[0] == 0.5

# 副本独立：快照保留副本创建时的分布
snapshot = state.copy()
state.apply_x(0)
assert snapshot.probabilities()[3] == 0.5
assert state.probabilities()[1] == 0.5
assert state.probabilities()[2] == 0.5

# 全量测量后状态坍缩到单个基态
outcome = state.measure_all()
assert outcome.to_bitstring(2) in {"01", "10"}

# 测量与复位：在确定性基态上结果可预期
basis = StabilizerState(1)
basis.apply_x(0)
assert basis.measure(0) is True
assert basis.probabilities()[1] == 1.0

basis.reset(0)
assert basis.probabilities()[0] == 1.0
```

---

## StabilizerCircuitResult

The object returned by `StabilizerState.run_circuit()`, carrying both the final stabilizer state and the runtime classical data produced during execution.

This object has no public constructor and can only be returned by `StabilizerState.run_circuit()`.

### Attributes

- `state -> StabilizerState`: the final stabilizer state after circuit execution ends.
- `classical -> ClassicalState`: the set of runtime classical values produced during execution, see [Classical State](5_classical_state.md).

### Other behavior

- `repr(result)` returns a string of the form `StabilizerCircuitResult(num_qubits=N)`.
- `__eq__` is not implemented: `==` is the default object identity comparison.
- `__copy__` and `__deepcopy__` are not implemented: the standard library `copy.copy()` and `copy.deepcopy()` raise `TypeError`. When the result needs to be preserved, call `StabilizerState.run_circuit()` again.

---

Example:

```python
from cqlib import Circuit
from cqlib.qis.state import (
    ClassicalState,
    RuntimeValue,
    StabilizerCircuitResult,
    StabilizerState,
)

circuit = Circuit(2)
circuit.x(0)
measurement = circuit.measure(0)
circuit.reset(0)
circuit.h(1)

result = StabilizerState.run_circuit(circuit)
assert isinstance(result, StabilizerCircuitResult)
assert "StabilizerCircuitResult" in repr(result)

# 最终状态
assert result.state.num_qubits == 2
assert result.state.probabilities()[0] == 0.5
assert result.state.probabilities()[2] == 0.5

# 运行期经典数据
assert isinstance(result.classical, ClassicalState)
measured = result.classical.value(measurement.value)
assert isinstance(measured, RuntimeValue)
assert measured.kind == "bit"
assert measured.as_bit() is True
```

---

## Exceptions

The validation failures of the two classes fall into three categories by source:

| Exception | When it occurs |
| --- | --- |
| `IndexError` | Qubit index out of range. |
| `ValueError` | Circuit containing non-Clifford operations or control flow, duplicate target qubits of a two-qubit gate, bitstring length mismatch of `probability_of`, qubit count mismatch of `pauli_expectation`, or circuit qubit count inconsistent with the state. |
| `OverflowError` | `num_qubits` is negative. |
