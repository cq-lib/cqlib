# Pauli, PauliString and Hamiltonian

Pauli operators and Pauli Hamiltonians are the foundation of VQE, QAOA, quantum simulation, stabilizer analysis and measurement post-processing. `cqlib.qis` provides the single-qubit `Pauli`, the multi-qubit `PauliString`, the global phase `Phase` and the Pauli term sum `Hamiltonian`.

The focus of this section is not circuit construction, but how to describe observables, determine commutation, compute expectation values and convert a Hamiltonian into an evolution circuit.

---

## Task: understand single-qubit Pauli operators and phases

```python
from cqlib.qis import Pauli

x = Pauli.x()
y = Pauli.y()
z = Pauli.z()

result, phase = x.mul_with_phase(z)

print(result)
print(phase)
print(phase.to_complex())
print(y.to_symplectic())
print(z.to_matrix())
```

Pauli multiplication produces a global phase. Use `mul_with_phase()` when the phase needs to be preserved; ordinary multiplication can be used when only the Pauli type matters, but it should not be used to interpret the complete phase relation.

---

## Constructing a PauliString

`PauliString` represents a multi-qubit Pauli operator. It can be created from a string, or set qubit by qubit.

```python
from cqlib.qis import Pauli, PauliString

string = PauliString.from_str("XZI")
print(string.num_qubits)
print(string.x_bits)
print(string.z_bits)
print(string.x_mask)
print(string.z_mask)

manual = PauliString(3)
manual.set_pauli(0, Pauli.x())
manual.set_pauli(1, Pauli.z())
manual.set_pauli(2, Pauli.i())

print(manual)
```

When strings and bitstring probabilities are used together for post-processing, the qubit ordering convention must be unified. In particular, when computing expectation values from a probability dictionary, confirm which qubit each position of the result string corresponds to.

---

## Determining commutation and multiplication

```python
from cqlib.qis import PauliString

xx = PauliString.from_str("XX")
zz = PauliString.from_str("ZZ")
zi = PauliString.from_str("ZI")
iz = PauliString.from_str("IZ")

print(xx.commutes_with(zz))
print(zi.commutes_with(iz))
print(xx * zz)
```

Commutation determines whether Pauli terms can be combined in the same measurement basis, and also affects the evolution decomposition of a Hamiltonian. When building a VQE or QAOA measurement flow, the commutation structure of the Pauli terms should be checked as early as possible.

---

## Computing Pauli expectation values from a state or probabilities

`PauliString` can compute expectation values directly against a `Statevector` or a `DensityMatrix`, and can also estimate expectation values from a measurement probability dictionary.

```python
from cqlib.qis import PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

zz = PauliString.from_str("ZZ")
print(zz.expectation_statevector(state))

probs = {
    "00": 0.5,
    "11": 0.5,
}
print(zz.expectation(probs))
```

Computing an expectation value from a probability dictionary is only suitable for probabilities already obtained in the corresponding Pauli measurement basis. For a PauliString containing `X` or `Y`, computational-basis sampling probabilities cannot be treated directly as the corresponding measurement results, unless the correct basis-change measurement has already been performed in the circuit.

---

## Constructing a Hamiltonian

`Hamiltonian` is a sparse sum of PauliStrings with coefficients, of the form `H = Σ c_k P_k`.

```python
from cqlib.qis import Hamiltonian, PauliString

hamiltonian = Hamiltonian(2)
hamiltonian.add_term(PauliString.from_str("ZZ"), 1.0)
hamiltonian.add_term(PauliString.from_str("XI"), 0.5)
hamiltonian.add_term(PauliString.from_str("ZZ"), -0.2)

print(hamiltonian.num_terms)
hamiltonian.simplify()
print(hamiltonian.terms)
```

It can also be created from a list in one call:

```python
hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZI"), -1.0),
    (PauliString.from_str("IZ"), -1.0),
    (PauliString.from_str("ZZ"), 0.5),
])
```

`simplify()` merges duplicate PauliStrings and absorbs the internal phase of each PauliString into its coefficient. After constructing a large Hamiltonian, it is recommended to call `simplify()` first before moving on to simulation, measurement grouping or evolution decomposition.

---

## Computing Hamiltonian expectation values and variance

```python
from cqlib.qis import Hamiltonian, PauliString, Statevector

state = Statevector(2)
state.apply_h(0)
state.apply_cx(0, 1)

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 1.0),
    (PauliString.from_str("XX"), 0.5),
])

print(hamiltonian.expectation_statevector(state))
print(hamiltonian.variance_statevector(state))
```

If the input is a density matrix, `expectation_density_matrix()` can be used. If the input comes from real sampling results, `expectation_probs()` can be used to aggregate the probabilities in each Pauli measurement basis.

---

## Constructing a Trotter evolution circuit

`Hamiltonian` can be converted into a quantum circuit that approximately implements `e^{-iHt}`. `TrotterMode` provides first-order, second-order and randomized modes.

```python
from cqlib.qis import Hamiltonian, PauliString, TrotterMode

hamiltonian = Hamiltonian.from_list([
    (PauliString.from_str("ZZ"), 0.5),
    (PauliString.from_str("XI"), 0.3),
])

circuit = hamiltonian.to_trotter_circuit(
    time=1.0,
    steps=4,
    mode=TrotterMode.first_order(),
)

print(circuit)
```

For commuting terms, `to_evolution_circuit()` can use a more direct evolution decomposition; for non-commuting terms, it falls back to the specified Trotter mode.

```python
circuit = hamiltonian.to_evolution_circuit(
    time=1.0,
    steps=4,
    mode=TrotterMode.second_order(),
)
```

The accuracy of a time evolution circuit depends on whether the Hamiltonian terms commute, the total evolution time, the number of Trotter steps and the decomposition order. Before a formal experiment, an error comparison should first be performed on a small-scale system.

---

## Next steps

- [Statevector pure-state simulation](1_statevector.md): apply PauliString and Hamiltonian to an ideal pure state, and verify expectation values and energy functions.
- [DensityMatrix mixed-state simulation](2_density_matrix.md): apply the same set of observables to mixed-state and noisy-state analysis.
- [Quantum state metrics and entropy](6_metrics_entropy.md): beyond energy, continue comparing fidelity, trace distance, purity and entanglement metrics.
