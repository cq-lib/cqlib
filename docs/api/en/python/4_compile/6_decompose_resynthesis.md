# Decompose / Resynthesis

`cqlib.compile.transform.decompose`

Gate decomposition, two-qubit block resynthesis, target gate set translation and device lowering, located in the `decompose`, `resynthesis`, `target_basis` and `device_lowering` submodules of `cqlib.compile.transform`.

Gate decomposition is layered according to the representations available for an operation: definition expansion handles gates that carry their own implementation circuit, numeric synthesis handles unitary gates with a fixed matrix, and multi-controlled gate decomposition handles controlled operations. Definition expansion runs before numeric synthesis, so that a unitary gate with an implementation circuit is already expanded before it enters matrix synthesis.

## Import

```python
from cqlib.compile.transform.decompose import (
    DecompositionRuleStats,
    McGateDecomposeConfig,
    TwoQubitUnitaryDecomposeBasis,
    UnitaryDecomposeConfig,
    decompose_mc_gates,
    decompose_mc_gates_for_device,
    decompose_mc_gates_with_rule_stats,
    decompose_unitaries,
    decompose_unitaries_with_rule_stats,
    expand_definitions,
    mc_gate,
    unitary,
)
from cqlib.compile.transform.resynthesis import (
    ResynthesizeTwoQubitBlocks,
    TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
)
from cqlib.compile.transform.target_basis import (
    TargetBasisCost,
    TargetBasisCostModel,
    TargetBasisLowerer,
    TargetBasisSignature,
)
from cqlib.compile.transform import DeviceLowerer
```

---

## Definition expansion

### expand_definitions(circuit)

Expand circuit gate definitions without modifying the input circuit.

Parameters:

- `circuit` (`Circuit`): the circuit to expand.

Returns:

- `TransformResult`; the `circuit` field is the expanded circuit, and `changed` indicates whether any rewriting occurred.

---

## Unitary synthesis

The circuit-level entry point is located in `cqlib.compile.transform.decompose`, and the corresponding numeric synthesis primitives are located in `cqlib.compile.transform.decompose.unitary`.

### decompose_unitaries(circuit, config=None)

Synthesize one-qubit and two-qubit unitary gates from matrices.

Parameters:

- `circuit` (`Circuit`): the circuit to synthesize.
- `config` (`UnitaryDecomposeConfig | None`): the decomposition configuration; defaults to `UnitaryDecomposeConfig()`.

Returns:

- `TransformResult`

Raises:

- `CompilerTransformError`: the `UnitaryGate` has no matrix representation, or the qubit count of the matrix does not match the qubit count of the gate.

### decompose_unitaries_with_rule_stats(circuit, config=None)

As above, and additionally returns the rule cache statistics within the pass.

Returns:

- `(TransformResult, DecompositionRuleStats)`

### UnitaryDecomposeConfig(*, two_qubit_basis=None, target_basis=None, recurse_control_flow=True)

Parameters:

- `two_qubit_basis` (`TwoQubitUnitaryDecomposeBasis | None`): the two-qubit synthesis gate basis.
- `target_basis` (`list[str | Instruction] | None`): the target gate set constraint; entries take the same form as `target_basis` in `compile()`.
- `recurse_control_flow` (`bool`): whether to recurse into control flow blocks.

`two_qubit_basis` and `target_basis` are mutually exclusive; providing both is rejected.

Attributes:

- `two_qubit_basis -> TwoQubitUnitaryDecomposeBasis | None`
- `target_basis -> list[Instruction] | None`
- `recurse_control_flow -> bool`

Raises:

- `CompilerConfigError`: `two_qubit_basis` and `target_basis` are both provided, or the target gate set is empty.

### TwoQubitUnitaryDecomposeBasis

Two-qubit synthesis gate basis, taking the value of one of the following static methods:

- `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`: local `U` gates plus `RXX`, `RYY`, `RZZ`.
- `TwoQubitUnitaryDecomposeBasis.cx()`: local `U` gates plus `CX`.
- `TwoQubitUnitaryDecomposeBasis.cy()`: local `U` gates plus `CY`.
- `TwoQubitUnitaryDecomposeBasis.cz()`: local `U` gates plus `CZ`.
- `TwoQubitUnitaryDecomposeBasis.rzz()`: local `U`, `H`, `RX` plus `RZZ`.

### DecompositionRuleStats

Rule cache statistics within the pass:

- `hits -> int`: the number of cache hits.
- `misses -> int`: the number of cache misses.
- `inserts -> int`: the number of cache writes.

---

## Numeric unitary synthesis

`cqlib.compile.transform.decompose.unitary` provides direct synthesis from a matrix to a gate sequence, without circuit traversal.

### synthesize_numeric_1q_unitary(matrix)

Synthesize a numeric one-qubit unitary matrix into `U` gate angles and a global phase.

Parameters:

- `matrix` (`list[list[complex]]`): a 2×2 unitary matrix, convertible to a two-dimensional `complex128` array.

Returns:

- `OneQubitUnitaryDecomposition`

Raises:

- `CompilerConfigError`: the matrix is not 2×2.
- `TypeError`: the input cannot be converted to a two-dimensional array.

### synthesize_numeric_2q_unitary(matrix, first, second, basis=None)

Synthesize a numeric two-qubit unitary matrix into a gate sequence.

Parameters:

- `matrix` (`list[list[complex]]`): a 4×4 unitary matrix, convertible to a two-dimensional `complex128` array.
- `first`, `second` (`int | Qubit`): the acted-on qubits, which must be distinct.
- `basis` (`TwoQubitUnitaryDecomposeBasis | None`): the synthesis gate basis; defaults to `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`.

Returns:

- `TwoQubitUnitarySynthesisResult`

Raises:

- `CompilerConfigError`: the matrix is not 4×4, or `first` and `second` point to the same qubit.

### kak_decompose(matrix)

Compute the canonical KAK decomposition of a numeric 4×4 unitary matrix.

Parameters:

- `matrix` (`list[list[complex]]`): a 4×4 unitary matrix, convertible to a two-dimensional `complex128` array.

Returns:

- `KakDecomposition`

Raises:

- `TypeError`: the input cannot be converted to a two-dimensional array.

### OneQubitUnitaryDecomposition

The `U` gate decomposition result of a one-qubit unitary matrix.

Attributes:

- `theta -> float`: the polar angle of the synthesized `U` gate.
- `phi -> float`: the first azimuthal angle of the synthesized `U` gate.
- `lambda_ -> float`: the second azimuthal angle of the synthesized `U` gate.
- `global_phase -> float`: the scalar phase multiplied onto the synthesized `U` gate.

Other behavior:

- Supports `copy` and `deepcopy`, and compares by value.

### TwoQubitUnitarySynthesisResult

The synthesis result of a two-qubit unitary matrix.

Attributes:

- `operations -> list[ValueOperation]`: the standard gate operation sequence implementing the unitary, exact up to `global_phase`.
- `global_phase -> float`: the scalar phase multiplied onto the operation sequence.

Other behavior:

- Supports `copy` and `deepcopy`.

### KakDecomposition

The canonical KAK decomposition of a two-qubit unitary matrix.

Attributes:

- `global_phase -> float`: the scalar phase multiplied onto the complete decomposition.
- `k1l -> numpy.ndarray`: the left local factor acting after the Cartan interaction.
- `k1r -> numpy.ndarray`: the right local factor acting after the Cartan interaction.
- `k2l -> numpy.ndarray`: the left local factor acting before the Cartan interaction.
- `k2r -> numpy.ndarray`: the right local factor acting before the Cartan interaction.
- `a -> float`: the canonical Pauli-XX interaction coordinate.
- `b -> float`: the canonical Pauli-YY interaction coordinate.
- `c -> float`: the canonical Pauli-ZZ interaction coordinate.

Other behavior:

- Each `k` factor is an independent copy; modifying the returned value does not affect the decomposition object.
- Supports `copy` and `deepcopy`.

---

## Multi-controlled gate decomposition

### decompose_mc_gates(circuit, config=None)

Decompose multi-controlled gates using the configured ancilla resources. This entry point has no target device and therefore does not check device capacity; use `decompose_mc_gates_for_device` when capacity checking is needed.

Parameters:

- `circuit` (`Circuit`): the circuit to decompose.
- `config` (`McGateDecomposeConfig | None`): the decomposition configuration; defaults to `McGateDecomposeConfig()`.

Returns:

- `TransformResult`

### decompose_mc_gates_with_rule_stats(circuit, config=None)

As above, and additionally returns the rule cache statistics.

Returns:

- `(TransformResult, DecompositionRuleStats)`

### decompose_mc_gates_for_device(circuit, device, resource_policy=None)

Decompose multi-controlled gates under the device capacity constraint. This entry point constrains the logical qubit count of the circuit not to exceed the number of qubits available on the device, and does not check the coupling topology.

Parameters:

- `circuit` (`Circuit`): the circuit to decompose.
- `device` (`Device`): provides the available physical qubit capacity constraint.
- `resource_policy` (`ResourcePolicy | None`): the ancilla policy.

Returns:

- `TransformResult`

Raises:

- `CompilerConfigError`: the circuit width exceeds the number of qubits available on the device.

### McGateDecomposeConfig(*, resource_policy=None, resource_limits=None)

Parameters:

- `resource_policy` (`ResourcePolicy | None`): the ancilla policy.
- `resource_limits` (`ResourceLimits | None`): the resource limits.

Attributes:

- `resource_policy -> ResourcePolicy`
- `resource_limits -> ResourceLimits`

---

## Multi-controlled gate primitives

`cqlib.compile.transform.decompose.mc_gate` provides exact synthesis primitives divided by gate family; they give the operation sequence directly, without circuit traversal, without selecting an algorithm and without requesting ancillas. **The normal compilation flow should use `decompose_mc_gates` or `decompose_mc_gates_for_device`**; call these primitives directly only when ancilla allocation must be controlled manually or a custom lowering strategy is being implemented.

The primitives are grouped by gate family, and each group usually provides a "no ancilla", a "one or more clean ancillas", a "one or more dirty ancillas" and a fixed-depth variant for a single ancilla:

| Gate family | Coverage |
| --- | --- |
| MCX | Multi-controlled X gates. |
| Multi-controlled SU(2) | Multi-controlled special unitary rotations, with the rotation axis given by `Su2RotationAxis`. |
| Pauli | The multi-controlled Pauli gate family. |
| RZZ | Multi-controlled RZZ. |
| Pauli rotation | Multi-controlled Pauli rotations. |
| Rotation | Multi-controlled `RX` / `RY` / `RZ` and their intrinsic controlled forms. |
| Phase | Multi-controlled `S` / `SDG` / `T` / `TDG` / `Phase`. |
| QCIS | Synthesis primitives aimed at the QCIS gate set. |
| Hadamard | Multi-controlled Hadamard. |
| SWAP | Multi-controlled SWAP. |
| FSIM | Multi-controlled FSIM. |
| Unitary | Multi-controlled general unitary gates. |

Common parameters:

- `controls` (`list[int | Qubit]`): the control qubits, which must be the complete set of expanded controls, including the controls intrinsic to the gate itself.
- `target`, or `first` and `second` for two-qubit primitives: the target qubits.
- `clean_ancillas` / `clean_ancilla`: clean ancillas, which must enter as `|0>` and be restored to `|0>`.
- `dirty_ancillas` / `dirty_ancilla`: dirty ancillas, which may enter in an arbitrary unknown state but must be restored exactly.

All primitives return `list[ValueOperation]`, which can be assembled into a circuit with `Circuit.from_operations`. Extra ancillas beyond the consumed prefix are ignored; ancillas must be distinct from all control qubits and target qubits. For the complete list see the public surface of `cqlib.compile.transform.decompose.mc_gate`.

---

## Two-qubit block resynthesis

### resynthesize_two_qubit_blocks(circuit, config=None)

Resynthesize two-qubit gate blocks; the functional entry point.

Parameters:

- `circuit` (`Circuit`): the circuit to resynthesize.
- `config` (`TwoQubitBlockResynthesisConfig | None`): the resynthesis configuration.

Returns:

- `TransformResult`

### ResynthesizeTwoQubitBlocks(config=None)

Reusable pass object.

Parameters:

- `config` (`TwoQubitBlockResynthesisConfig | None`): the resynthesis configuration.

Attributes and methods:

- `config -> TwoQubitBlockResynthesisConfig`
- `run(circuit) -> TransformResult`

### TwoQubitBlockResynthesisConfig(*, two_qubit_basis=None, target_basis=None, enhanced=False, max_block_ops=None, max_crossed_ops=None, max_scan_span=None, skip_labeled_ops=True, recurse_control_flow=True, commutation=None)

Parameters:

- `two_qubit_basis` (`TwoQubitUnitaryDecomposeBasis | None`): the two-qubit synthesis gate basis.
- `target_basis` (`list[str | Instruction] | None`): the target gate set constraint.
- `enhanced` (`bool`): whether to use the enhanced budget, trading compilation time for better post-routing cleanup.
- `max_block_ops` (`int | None`): the maximum number of operations in a block.
- `max_crossed_ops` (`int | None`): the maximum number of crossed operations in a block; a crossed operation stays in place and must commute with the synthesized replacement.
- `max_scan_span` (`int | None`): the collection budget on one side of a two-qubit anchor.
- `skip_labeled_ops` (`bool`): whether to treat labeled operations as hard boundaries.
- `recurse_control_flow` (`bool`): whether to recurse into control flow blocks.
- `commutation` (`CommutationConfig | None`): the commutation check configuration.

---

## Target gate set translation

### TargetBasisLowerer(target_basis)

Translate a circuit to a target gate set.

Parameters:

- `target_basis` (`list[str | Instruction]`): a non-empty target gate set; entries take the same form as `target_basis` in `compile()`.

Attributes and methods:

- `target_basis -> list[Instruction]`
- `run(circuit) -> TransformResult`

### TargetBasisSignature

Target gate set signature:

- `TargetBasisSignature.from_standard_gates(gates)`: construct a signature from a list of standard gates.

### TargetBasisCost

Target gate set cost statistics:

- `two_qubit_ops -> int`: the number of two-qubit gates.
- `depth -> int`: the depth.
- `total_ops -> int`: the total number of operations.
- `parameterized_ops -> int`: the number of parameterized operations.

### TargetBasisCostModel

Target gate set cost model:

- `signature -> TargetBasisSignature`
- `target_basis -> list[Instruction]`
- `cost_of_fixed_operations(qubits, operations) -> TargetBasisCost`: compute the cost of a fixed operation set. `qubits` is a list of qubit indices or `Qubit` objects, and `operations` is a list of `ValueOperation`.

---

## Device lowering

### DeviceLowerer(device)

Lower a circuit according to the device native instruction set.

Parameters:

- `device` (`Device`): the target device.

Attributes and methods:

- `device -> Device`
- `run(circuit) -> TransformResult`

---

## Examples

### 1. Definition expansion and unitary synthesis

```python
import numpy as np

from cqlib.circuit import Circuit, UnitaryGate
from cqlib.compile.transform.decompose import (
    decompose_unitaries_with_rule_stats,
    expand_definitions,
)

definition = Circuit(1)
definition.h(0)
circuit = Circuit(1)
circuit.append_circuit_gate(definition.to_gate("custom_h"), [0])

expanded = expand_definitions(circuit)
assert expanded.changed is True
assert [op.instruction.instruction.name for op in circuit.operations] == ["custom_h"]
assert [op.instruction.instruction.name for op in expanded.circuit.operations] == ["H"]

matrix = np.array([[0, 1], [1, 0]], dtype=np.complex128)
gate = UnitaryGate("x_matrix", 1).with_matrix(matrix)
circuit = Circuit(2)
circuit.append_unitary_gate(gate, [0])
circuit.append_unitary_gate(gate, [1])

result, stats = decompose_unitaries_with_rule_stats(circuit)
assert result.changed is True
assert all(
    operation.instruction.instruction.name == "U"
    for operation in result.circuit.operations
)
assert (stats.hits, stats.misses, stats.inserts) == (1, 1, 1)
```

### 2. Numeric unitary synthesis

```python
import numpy as np

from cqlib.circuit import Circuit, Qubit, UnitaryGate
from cqlib.compile.transform.decompose import TwoQubitUnitaryDecomposeBasis
from cqlib.compile.transform.decompose.unitary import (
    kak_decompose,
    synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary,
)

source = (
    np.exp(0.37j)
    * np.array([[1, 1], [1, -1]], dtype=np.complex128)
    / np.sqrt(2)
)
decomposition = synthesize_numeric_1q_unitary(source)
circuit = Circuit(1)
circuit.u(0, decomposition.theta, decomposition.phi, decomposition.lambda_)
reconstructed = np.exp(1j * decomposition.global_phase) * circuit.to_matrix()
np.testing.assert_allclose(reconstructed, source, atol=1e-10)

swap = np.array(
    [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]],
    dtype=np.complex128,
)
synthesis = synthesize_numeric_2q_unitary(
    swap, 0, Qubit(1), TwoQubitUnitaryDecomposeBasis.cx()
)
expected = Circuit(2)
expected.append_unitary_gate(UnitaryGate("source", 2).with_matrix(swap), [0, 1])
synthesized = Circuit.from_operations([Qubit(0), Qubit(1)], synthesis.operations)
synthesized.set_global_phase(synthesis.global_phase)
np.testing.assert_allclose(
    synthesized.to_matrix(), expected.to_matrix(), atol=1e-8
)

kak = kak_decompose(swap)
assert kak.k1l.shape == (2, 2)
assert isinstance(kak.global_phase, float)
```

### 3. Multi-controlled gate decomposition

```python
from cqlib.circuit import Circuit, MCGate, StandardGate
from cqlib.compile.resource import ResourceLimits, ResourcePolicy
from cqlib.compile.transform.decompose import (
    McGateDecomposeConfig,
    decompose_mc_gates_with_rule_stats,
)

circuit = Circuit(4)
gate = MCGate(3, StandardGate.X)
circuit.append_mc_gate(gate, [0, 1, 2, 3])
circuit.append_mc_gate(gate, [0, 1, 2, 3])

config = McGateDecomposeConfig(
    resource_policy=ResourcePolicy(
        max_pre_layout_clean_ancillas=2,
        allow_dirty_borrowing=True,
    ),
    resource_limits=ResourceLimits(max_total_qubits=8),
)
result, stats = decompose_mc_gates_with_rule_stats(circuit, config)
assert result.changed is True
assert all(
    not operation.instruction.instruction.is_mcgate
    for operation in result.circuit.operations
)
assert stats.inserts >= 1
```

### 4. Two-qubit block resynthesis and target gate set translation

```python
from cqlib.circuit import Circuit
from cqlib.compile.transform.decompose import TwoQubitUnitaryDecomposeBasis
from cqlib.compile.transform.resynthesis import (
    ResynthesizeTwoQubitBlocks,
    TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
)
from cqlib.compile.transform.target_basis import TargetBasisLowerer

circuit = Circuit(2)
circuit.cx(0, 1)
circuit.cx(0, 1)
config = TwoQubitBlockResynthesisConfig(
    two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx()
)

result = resynthesize_two_qubit_blocks(circuit, config)
transformer = ResynthesizeTwoQubitBlocks(config)
assert transformer.config == config
assert result.changed is True
assert len(result.circuit.operations) == 0

lowerer = TargetBasisLowerer(["h", "CZ"])
assert [instruction.name for instruction in lowerer.target_basis] == ["H", "CZ"]
```

---

## Raises

- `CompilerTransformError`: decomposition or synthesis fails, for example the matrix is not unitary, the target gate set cannot cover the input operations, the number of ancillas is insufficient, or qubits are duplicated.
- `CompilerConfigError`: the configuration is invalid, for example an empty target gate set, `two_qubit_basis` and `target_basis` both provided, or the circuit width exceeding the device capacity.
- `ParameterError`: a gate parameter is not a finite value.
- `ValueError`: a primitive requiring exactly two ancillas received a different number.
- `ResourceError`: multi-controlled gate decomposition fails to request ancillas, see [Resource](8_resource.md).
