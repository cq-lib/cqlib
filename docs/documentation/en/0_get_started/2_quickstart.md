# Quickstart

This guide builds a minimal working quantum circuit with Cqlib and completes an end-to-end "engineering loop" workflow.

Before starting, ensure that the environment is ready:

- Python environment: Python 3.10 – 3.14 is supported
- Cqlib is installed and the environment is configured (for installation, see "[Cqlib Installation and Environment Setup](../0_get_started/1_installation.md)")

---

## The first quantum circuit: Bell state

This section uses a minimal but complete Bell state example to introduce the basic workflow of building a quantum circuit with Cqlib. The example covers how to create a circuit, add quantum gates and view the circuit structure, and further operations such as state simulation, measurement sampling, circuit visualization and IR export.

The Bell state is one of the most common two-qubit entangled states in quantum computing, usually used to demonstrate the basic concepts of superposition and entanglement. Constructing a Bell state takes the following two steps:

- Apply an `H` gate to qubit `0` to put it into a superposition state;
- Apply a CX gate with qubit `0` as the control and qubit `1` as the target, thereby establishing entanglement between the two qubits.

Ideally, the resulting quantum state is:

```text
(|00> + |11>) / sqrt(2)
```

## 1. Create a circuit

First create a two-qubit quantum circuit and append an `H` gate and a `CX` gate to it in order:

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

print(circuit.num_qubits)   # 2
print(len(circuit))         # 2
print(circuit.operations)   # view the underlying Operation list
```

## 2. View the circuit as a text diagram

Render the quantum circuit as a text diagram to view the circuit structure:

```python
from cqlib.visualization import draw_text

print(draw_text(circuit))
```

## 3. Convert to a matrix for verification

For small circuits made up purely of quantum gates, the whole circuit can be converted into a complete unitary matrix to verify its mathematical behavior:

```python
matrix = circuit.to_matrix()
print(matrix)
```

## 4. Export OpenQASM 2.0 / 3.0

Cqlib also provides export interfaces for OpenQASM 2.0 and OpenQASM 3.0:

```python
from cqlib.ir import qasm2, qasm3

print(qasm2.dumps(circuit))
print(qasm3.dumps(circuit))
```

## 5. Statevector simulation

Use statevector simulation to view the quantum state distribution after the circuit is applied:

```python
from cqlib.qis import Statevector

sv = Statevector.from_circuit(circuit)
print(sv.data)
print(sv.probabilities())
```

For a Bell state circuit, the result of `probabilities()` should ideally be close to:

```text
[0.5, 0.0, 0.0, 0.5]
```

This means that measurement yields only the two outcomes `00` and `11`, with equal probability, while the probabilities of `01` and `10` are close to `0`, reflecting the entanglement correlation between the two qubits in the Bell state.

## 6. Sampling measurement

Once the statevector is obtained, repeated sampling can be performed to simulate the statistical results of an actual measurement process:

```python
shots = sv.sample_shots(1000)
counts = {}
for outcome in shots:
    bitstring = outcome.to_bitstring(2)
    counts[bitstring] = counts.get(bitstring, 0) + 1

print(counts)
```

The output is similar to:

```text
{'00': 506, '11': 494}
```

Because the sampling process is random, the counts obtained from each run are not exactly the same. For an ideal Bell state, however, after a large number of samples the occurrence counts of `00` and `11` should be roughly equal, while `01` and `10` usually do not appear or have a probability close to zero.

---

## Next steps

- [Quantum Circuit](../1_cqlib/0_circuit/0_overview.md): learn about the basic module for describing quantum programs in Cqlib.
- [Quantum gates and instructions](../1_cqlib/0_circuit/1_gates.md): learn about built-in gates, custom gates, composite gates and non-unitary instructions.
- [Circuit structure and construction](../1_cqlib/0_circuit/2_structures.md): understand the lifecycle, indexing, composition and operation representation of `Circuit`.
- [Tianyan quantum cloud platform client](../1_cqlib/7_tianyan/0_overview.md): to submit circuits to a cloud backend for execution, continue with QCIS export, backend selection, task submission and result retrieval.
