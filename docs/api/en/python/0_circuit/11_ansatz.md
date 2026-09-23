# Ansatz

`cqlib.circuit.ansatz`  

```python
from cqlib.circuit.ansatz import (
    EntanglementTopology,
    TwoLocal,
    BasicEntanglerLayers,
    StronglyEntanglingLayers,
    AngleEncoding,
    BasisEncoding,
    ZFeatureMap,
    IQPFeatureMap,
    ZZFeatureMap,
    PauliFeatureMap,
    QAOAAnsatz,
    EvolutionStrategy,
    EvolutionInfo,
    PauliEvolutionAnsatz,
    real_amplitudes,
    efficient_su2,
    zz_feature_map,
    pauli_feature_map,
)
```

All names exported by the module are listed above, including the four convenience functions `real_amplitudes`, `efficient_su2`, `zz_feature_map` and `pauli_feature_map`, and `EvolutionInfo`.

`cqlib.circuit.ansatz` provides a set of reusable parameterized circuit templates, mainly for scenarios such as variational quantum algorithms, quantum machine learning, feature maps, QAOA and Hamiltonian evolution. These templates allow structured quantum circuits to be generated quickly, without adding rotation gates and entanglement gates layer by layer by hand.

---

## General template interfaces

Ansatz templates mainly provide the following interfaces:

| Method | Description |
| --- | --- |
| `validate()` | Check whether the template configuration is legal, for example whether the qubit count, gate types, topology and parameter settings match. |
| `build_circuit(prefix)` | Build a `Circuit` from the current configuration and generate parameter names with the given prefix. |
| `num_parameters()` | Return the number of parameters the template requires. |
| `num_qubits()` | Return the number of qubits the template acts on. |

```python
from cqlib.circuit.ansatz import TwoLocal

ansatz = TwoLocal(3)
ansatz.validate()

circuit = ansatz.build_circuit("theta")

assert len(circuit.symbols) == ansatz.num_parameters()
assert circuit.num_qubits == ansatz.num_qubits()
```

`validate()` raises `ValueError` when the configuration is illegal, for example when the qubit count, gate types, topology or parameter settings do not match each other; `build_circuit()` likewise raises `ValueError` in the same situations. Every template object supports `copy` and `deepcopy`, and carries a `repr` and a `str` convenient for debugging.

---

## Immutable builder pattern

Most configuration interfaces in the ansatz module use chained calls. For example:

```python
from cqlib.circuit.ansatz import TwoLocal, EntanglementTopology
from cqlib.circuit.gates import StandardGate

ansatz = (
    TwoLocal(4)
    .reps(2)
    .rotation_gates([StandardGate.RY, StandardGate.RZ])
    .entanglement_gate(StandardGate.CX)
    .entanglement(EntanglementTopology.linear())
)
```

Note that such configuration methods usually do not modify the original object but return an object with the new configuration. Therefore, several different configurations can safely be derived from the same base template:

```python
base = TwoLocal(4)

linear = base.entanglement(EntanglementTopology.linear())
full = base.entanglement(EntanglementTopology.full())
```

In the code above, `linear` and `full` are two template objects with different configurations, and `base` itself stays unchanged.

---

## `EntanglementTopology`

`EntanglementTopology` describes the topology on which two-qubit gates or multi-qubit terms act in a circuit. It determines between which qubits the template should establish connections when constructing an entanglement layer, feature map multi-body terms or Pauli evolution terms.

```python
EntanglementTopology.linear()
EntanglementTopology.circular()
EntanglementTopology.full()
EntanglementTopology.custom(pairs)
```

| Topology | Description |
| --- | --- |
| `linear()` | Nearest-neighbor chain connection, for example `(0, 1), (1, 2), ...`. |
| `circular()` | Adds a connection between the ends on top of `linear`, for example `(n-1, 0)`. |
| `full()` | Fully connected topology, in which any two qubits can be connected. |
| `custom(pairs)` | User-defined connection pairs, suitable for scenarios where the hardware topology or the problem graph is known. |

Common methods are as follows:

| Method | Description |
| --- | --- |
| `generate_pairs(num_qubits)` | Generate two-qubit connection pairs according to the topology. |
| `generate_k_tuples(k, num_qubits)` | Generate groups of `k`-local qubits according to the topology. |

```python
from cqlib.circuit.ansatz import EntanglementTopology

topology = EntanglementTopology.linear()
pairs = topology.generate_pairs(4)

assert pairs == [(0, 1), (1, 2), (2, 3)]
```

---

## `TwoLocal`

`TwoLocal` is a common hardware-friendly ansatz template, composed of alternating single-qubit rotation layers and two-qubit entanglement layers. It suits VQE, classification models, regression models and general variational circuit experiments.

```python
TwoLocal(num_qubits: int)
```

Common configuration methods are as follows:

| Method | Description |
| --- | --- |
| `reps(n)` | Set the number of repeated layers. |
| `rotation_gates(gates)` | Set the single-qubit parameterized gates used in each layer. |
| `entanglement_gate(gate)` | Set the two-qubit gate used in the entanglement layer. |
| `entanglement(topology)` | Set the entanglement topology. |
| `skip_final_rotation_layer(skip)` | Set whether to skip the final rotation layer. |

```python
from cqlib.circuit.ansatz import TwoLocal, EntanglementTopology
from cqlib.circuit.gates import StandardGate

ansatz = (
    TwoLocal(3)
    .reps(2)
    .rotation_gates([StandardGate.RY, StandardGate.RZ])
    .entanglement_gate(StandardGate.CX)
    .entanglement(EntanglementTopology.linear())
)

circuit = ansatz.build_circuit("theta")
```

The ansatz module also provides convenience functions for common `TwoLocal` configurations:

```python
real_amplitudes(num_qubits, reps, entanglement) -> TwoLocal
efficient_su2(num_qubits, reps, entanglement) -> TwoLocal
```

| Function | Default structure | Typical use |
| --- | --- | --- |
| `real_amplitudes` | `RY` rotations + `CX` entanglement | Real-amplitude ansatz, VQE, simple classification tasks. |
| `efficient_su2` | `RY` / `RZ` rotations + `CX` entanglement | A more expressive general hardware-friendly ansatz. |

---

## Feature Map

A feature map encodes classical input data into a quantum circuit and is an important part of quantum machine learning and quantum kernel methods. Different feature maps use different encoding approaches, for example treating input features as rotation angles, as computational basis states, or as phase parameters in a Pauli evolution.

Usually the parameters in a feature map represent input data features. In actual training, a feature map can be used in combination with a trainable ansatz.

---

## `AngleEncoding`

```python
AngleEncoding(num_qubits: int, rotation_gate: StandardGate)
```

`AngleEncoding` encodes each input feature as a single-qubit rotation angle. Its structure is simple and its parameters are intuitive, so it suits quickly constructing input encoding circuits in quantum machine learning.

```python
from cqlib.circuit.ansatz import AngleEncoding
from cqlib.circuit.gates import StandardGate

encoding = AngleEncoding(4, StandardGate.RX)
circuit = encoding.build_circuit("x")
```

The example above generates one input parameter prefixed with `x` for each of the 4 qubits and performs angle encoding through the `RX` gate.

---

## `BasisEncoding`

```python
BasisEncoding(bits: list[bool])
```

`BasisEncoding` prepares a computational basis state from a given bitstring. Unlike angle encoding, it introduces no continuous parameters and instead decides from the Boolean values whether to apply an `X` gate to the corresponding qubit.

```python
from cqlib.circuit.ansatz import BasisEncoding

encoding = BasisEncoding([True, False, True])
circuit = encoding.build_circuit("unused")

assert encoding.num_parameters() == 0
```

---

## `ZFeatureMap`

```python
ZFeatureMap(num_qubits).reps(n)
```

`ZFeatureMap` is a first-order Pauli-Z feature map; it usually introduces one input parameter per qubit and encodes features through phase evolution in the Z direction. Its number of parameters usually equals the number of qubits.

---

## `IQPFeatureMap`

```python
IQPFeatureMap(num_qubits).reps(n).entanglement(topology)
```

`IQPFeatureMap` is an IQP-style diagonal feature map. It usually contains single-body feature terms and multi-body interaction terms, and controls the encoding structure through repeated layers and entanglement topology. By default, this template usually uses a multi-layer structure and fairly strong entanglement connections.

```python
from cqlib.circuit.ansatz import IQPFeatureMap, EntanglementTopology

fm = (
    IQPFeatureMap(3)
    .reps(2)
    .entanglement(EntanglementTopology.full())
)

circuit = fm.build_circuit("x")
```

---

## `ZZFeatureMap`

```python
ZZFeatureMap(num_qubits).reps(n).entanglement(topology)
```

`ZZFeatureMap` is a second-order Pauli-Z feature map, usually containing single-qubit Z terms and two-qubit ZZ interaction terms.

```python
from cqlib.circuit.ansatz import ZZFeatureMap, EntanglementTopology

fm = ZZFeatureMap(3).reps(2).entanglement(EntanglementTopology.full())
circuit = fm.build_circuit("x")
```

Convenience function:

```python
zz_feature_map(num_qubits, reps, entanglement) -> ZZFeatureMap
```

---

## `PauliFeatureMap`

```python
ansatz = (
    PauliFeatureMap(num_qubits)
    .reps(n)
    .paulis(paulis)
    .entanglement(topology)
    .parameter_prefix(prefix)
)
```

`PauliFeatureMap` supports constructing a feature map from arbitrary Pauli string templates. Single-body terms, multi-body terms and the corresponding entanglement topology can be specified, so feature interactions of different orders can be expressed flexibly.

```python
from cqlib.qis import PauliString
from cqlib.circuit.ansatz import PauliFeatureMap, EntanglementTopology

fm = (
    PauliFeatureMap(3)
    .reps(2)
    .paulis([PauliString.from_str("Z"), PauliString.from_str("ZZ")])
    .entanglement(EntanglementTopology.full())
)

circuit = fm.build_circuit("x")
```

Convenience function:

```python
pauli_feature_map(num_qubits, reps, paulis, entanglement) -> PauliFeatureMap
```

When custom feature interaction forms are needed, `PauliFeatureMap` is more flexible than the fixed-structure `ZFeatureMap` or `ZZFeatureMap`.

---

## Layer templates

Layer templates construct common trainable parameterized layers. Unlike feature maps, the parameters in these templates are usually variables that an optimizer needs to train.

### 1. `BasicEntanglerLayers`

```python
ansatz = (
    BasicEntanglerLayers(num_qubits)
    .reps(n)
    .rotation_gate(gate)
    .entanglement_gate(gate)
)
```

`BasicEntanglerLayers` represents the basic "single-qubit rotation + ring entanglement" layer structure. Each layer usually applies parameterized rotation gates on the qubits first and then applies entanglement gates in a fixed ring structure.

---

### 2. `StronglyEntanglingLayers`

```python
ansatz = (
    StronglyEntanglingLayers(num_qubits)
    .reps(n)
    .entanglement_gate(gate)
    .ranges(ranges)
)
```

`StronglyEntanglingLayers` represents a more expressive entanglement layer template. It usually uses general single-qubit rotation gates and a ranged ring entanglement structure. `ranges` specifies the connection span of each layer, so that different layers can use different entanglement ranges.

```python
from cqlib.circuit.ansatz import StronglyEntanglingLayers

ansatz = StronglyEntanglingLayers(4).reps(3).ranges([1, 2])
circuit = ansatz.build_circuit("w")
```

---

## `QAOAAnsatz`

```python
QAOAAnsatz(cost_operator: Hamiltonian)
```

`QAOAAnsatz` constructs a QAOA circuit. In terms of circuit structure, QAOA alternately applies the time evolution of the cost Hamiltonian and of the mixer Hamiltonian, and is generally used for combinatorial optimization problems.

Its basic form can be written as:

```text
U(beta, gamma) = product_l exp(-i beta_l H_M) exp(-i gamma_l H_C)
```

Here `H_C` is the problem Hamiltonian, `H_M` is the mixer Hamiltonian, and `gamma_l` and `beta_l` are the variational parameters in the `l`-th layer.

Common configuration methods are as follows:

| Method | Description |
| --- | --- |
| `reps(n)` | Set the number of QAOA layers `p`; the total number of parameters is usually `2 * p`. |
| `mixer(mixer_operator)` | Set a custom mixer Hamiltonian. |
| `initial_state(circuit)` | Set the initial state preparation circuit. |
| `evolution_strategy(strategy)` | Set the Hamiltonian evolution strategy. |

```python
from cqlib.qis import Hamiltonian, PauliString
from cqlib.circuit.ansatz import QAOAAnsatz

h_c = Hamiltonian(2)
h_c.add_term(PauliString.from_str("ZZ"), 0.5)

ansatz = QAOAAnsatz(h_c).reps(3)
circuit = ansatz.build_circuit("qaoa")

assert ansatz.num_parameters() == 6
```

---

## Hamiltonian Evolution

Hamiltonian evolution templates construct quantum circuits of the form `exp(-i H t)`. For a Hamiltonian composed of Pauli terms, an exact evolution or a Trotter decomposition strategy can be chosen according to whether the terms commute.

### 1. `EvolutionStrategy`

`EvolutionStrategy` specifies how the circuit for a Hamiltonian evolution is decomposed.

| Static method | Description |
| --- | --- |
| `exact()` | Use an exact product of Pauli rotations for a Hamiltonian whose terms commute pairwise; if they do not commute, an error is reported when the circuit is built. |
| `auto(steps=1)` | Automatically choose exact evolution or a first-order Trotter decomposition. |
| `trotter(mode, steps)` | Explicitly specify the Trotter-Suzuki decomposition mode and the number of steps. |

```python
from cqlib.circuit.ansatz import EvolutionStrategy
from cqlib.qis import TrotterMode

exact = EvolutionStrategy.exact()
auto = EvolutionStrategy.auto(steps=4)
second = EvolutionStrategy.trotter(TrotterMode.second_order(), steps=8)
```

---

### 2. `EvolutionInfo`

`EvolutionInfo` is returned by `PauliEvolutionAnsatz.evolution_info()` and describes the current Hamiltonian evolution strategy and the actual decomposition information.

| Attribute | Description |
| --- | --- |
| `is_exact` | Whether the current decomposition is a mathematically exact evolution. |
| `steps` | The number of decomposition steps actually used. |
| `trotter_mode` | The current Trotter mode; `None` when the exact strategy is used. |
| `all_terms_commute` | Whether the terms in the Hamiltonian commute pairwise. |
| `num_terms` | The number of Pauli terms after simplification. |

---

### 3. `PauliEvolutionAnsatz`

```python
ansatz = (
    PauliEvolutionAnsatz(hamiltonian)
    .with_strategy(strategy)
    .with_time_param_name(name)
)
```

`PauliEvolutionAnsatz` constructs a parameterized evolution circuit for `exp(-i H t)`. The generated circuit usually contains a single time parameter whose default name is `{prefix}_t`, and which can also be customized through `with_time_param_name()`.

```python
from cqlib.qis import Hamiltonian, PauliString
from cqlib.circuit.ansatz import PauliEvolutionAnsatz, EvolutionStrategy

h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("ZI"), 0.3)

ansatz = PauliEvolutionAnsatz(h).with_strategy(EvolutionStrategy.auto())
info = ansatz.evolution_info()
circuit = ansatz.build_circuit("evo")
```

