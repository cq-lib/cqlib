# Decompose / Resynthesis

`cqlib_core::compile::transform::decompose`

Gate decomposition, two-qubit block resynthesis, target gate set translation and device lowering, located in the `decompose`, `resynthesis`, `target_basis` and `device_lowering` submodules of `cqlib_core::compile::transform`.

Gate decomposition is layered by the representation available for an operation: definition expansion handles gates that carry their own implementation circuit, numeric synthesis handles unitary gates with a fixed matrix, and multi-controlled gate decomposition handles controlled operations. Definition expansion runs before numeric synthesis, so that unitary gates carrying an implementation circuit are already expanded before entering matrix synthesis.

## Import

```rust
use cqlib_core::compile::transform::decompose::{
    DecompositionRuleStats, DecomposeDefinitions, DecomposeMcGates, DecomposeUnitaries,
    McGateDecomposeConfig, UnitaryDecomposeConfig, decompose_mc_gates,
    decompose_mc_gates_with_rule_stats, decompose_unitaries,
    decompose_unitaries_with_rule_stats, expand_definitions, mc_gate, unitary,
};
use cqlib_core::compile::transform::decompose::unitary::{
    KakDecomposition, OneQubitUnitaryDecomposition, TwoQubitSynthesisTarget,
    TwoQubitUnitaryDecomposeBasis, TwoQubitUnitarySynthesisResult, kak_decompose,
    synthesize_numeric_1q_unitary, synthesize_numeric_2q_unitary,
};
use cqlib_core::compile::transform::resynthesis::{
    ResynthesizeTwoQubitBlocks, TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
};
use cqlib_core::compile::transform::target_basis::{
    TargetBasisCost, TargetBasisCostModel, TargetBasisLowerer, TargetBasisSignature,
};
use cqlib_core::compile::transform::DeviceLowerer;
```

---

## Definition expansion

- `expand_definitions(circuit: &Circuit) -> Result<Circuit, CompilerError>`: expand circuit gate definitions and return a new circuit.
- `DecomposeDefinitions`: a `Transformer` adapter implementing the `decompose_definitions` step.

---

## Unitary gate synthesis

The circuit-level entry points are in `cqlib_core::compile::transform::decompose`, and the corresponding numeric synthesis primitives are in `cqlib_core::compile::transform::decompose::unitary`.

- `decompose_unitaries(circuit: &Circuit, config: UnitaryDecomposeConfig) -> Result<Circuit, CompilerError>`
- `decompose_unitaries_with_rule_stats(circuit: &Circuit, config: UnitaryDecomposeConfig) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError>`
- `DecomposeUnitaries`: a `Transformer` adapter with the configuration bound at construction time, constructed with `DecomposeUnitaries::new(config)`; implements `Default`.

### UnitaryDecomposeConfig

```rust
pub struct UnitaryDecomposeConfig {
    pub two_qubit_target: TwoQubitSynthesisTarget,
    pub recurse_control_flow: bool,
}
```

Fields:

- `two_qubit_target`: the target capabilities used by exact two-qubit numeric synthesis, constructed with `TwoQubitSynthesisTarget::from_instructions(target_basis: Option<&[Instruction]>)`; `None` means no target constraint, enabling the neutral Pauli rotation fallback.
- `recurse_control_flow`: whether to recurse into control flow blocks.

Implements `Default`, equivalent to no target constraint with recursion into control flow blocks.

### DecompositionRuleStats

```rust
pub struct DecompositionRuleStats {
    pub hits: usize,
    pub misses: usize,
    pub inserts: usize,
}
```

In-pass rule cache statistics: hits, misses and insert counts.

---

## Numeric unitary gate synthesis

`cqlib_core::compile::transform::decompose::unitary` provides direct synthesis from a matrix to a gate sequence, without circuit traversal.

- `synthesize_numeric_1q_unitary(matrix: &Array2<Complex64>) -> Result<OneQubitUnitaryDecomposition, CompilerError>`
- `synthesize_numeric_2q_unitary(matrix: &Array2<Complex64>, qubits: [Qubit; 2], basis: TwoQubitUnitaryDecomposeBasis) -> Result<TwoQubitUnitarySynthesisResult, CompilerError>`
- `kak_decompose(matrix: &Array2<Complex64>) -> Result<KakDecomposition, CompilerError>`

`Array2` and `Complex64` come from `ndarray` and `num_complex` respectively.

Synthesis planning and cost evaluation for a target gate set are handled internally by this submodule and are not used as regular interfaces.

### OneQubitUnitaryDecomposition

```rust
pub struct OneQubitUnitaryDecomposition {
    pub theta: f64,
    pub phi: f64,
    pub lambda: f64,
    pub global_phase: f64,
}
```

The matrix represented is `exp(i * global_phase) * U(theta, phi, lambda)`.

### TwoQubitUnitarySynthesisResult

```rust
pub struct TwoQubitUnitarySynthesisResult {
    pub operations: Vec<ValueOperation>,
    pub global_phase: f64,
}
```

- `operations`: the sequence of standard gate operations implementing the unitary, exact up to `global_phase`.
- `global_phase`: the scalar phase multiplied onto the operation sequence.

### KakDecomposition

```rust
pub struct KakDecomposition {
    pub global_phase: f64,
    pub k1l: Array2<Complex64>,
    pub k1r: Array2<Complex64>,
    pub k2l: Array2<Complex64>,
    pub k2r: Array2<Complex64>,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}
```

Fields:

- `global_phase`: the scalar phase multiplied onto the complete decomposition.
- `k1l`, `k1r`: the left and right local factors applied after the Cartan interaction.
- `k2l`, `k2r`: the left and right local factors applied before the Cartan interaction.
- `a`, `b`, `c`: the canonical Pauli-XX, Pauli-YY and Pauli-ZZ interaction coordinates.

### TwoQubitUnitaryDecomposeBasis

```rust
pub enum TwoQubitUnitaryDecomposeBasis {
    PauliRotations,
    Cx,
    Cy,
    Cz,
    Rzz,
}
```

Variants:

- `PauliRotations`: local `U` gates plus `RXX`, `RYY` and `RZZ`.
- `Cx`, `Cy`, `Cz`: local `U` gates plus the corresponding controlled gate template.
- `Rzz`: local `U` gates plus `RZZ`.

### TwoQubitSynthesisTarget

A target gate set capability description that tells numeric unitary gate synthesis and two-qubit block resynthesis which native gate set the backend supports.

```rust
pub struct TwoQubitSynthesisTarget {
    // 字段私有
}
```

Methods:

- `from_instructions(target_basis: Option<&[Instruction]>) -> Result<Self, CompilerError>`: construct a capability description from a target gate set; `None` means no target constraint.
- `from_standard_gates(native_1q: Vec<StandardGate>, native_2q: Vec<StandardGate>, fallback_pauli: bool) -> Result<Self, CompilerError>`: give the native single-qubit and two-qubit gates directly.
- `unconstrained() -> Self`: no target constraint, enabling the neutral Pauli rotation fallback.
- `native_1q(&self) -> &[StandardGate]` / `native_2q(&self) -> &[StandardGate]`: read the registered native gates.
- `fallback_pauli(&self) -> bool`: whether the Pauli rotation fallback is enabled.

Implements `Default`, equivalent to `unconstrained()`.

---

## Multi-controlled gate decomposition

- `DecomposeMcGates`: a `Transformer` adapter with the configuration bound at construction time, constructed with `DecomposeMcGates::new(config)`; implements `Default`.
- `decompose_mc_gates(circuit: &Circuit, config: McGateDecomposeConfig) -> Result<TransformOutcome, CompilerError>`: rewrite multi-controlled gates.
- `decompose_mc_gates_with_rule_stats(circuit: &Circuit, config: McGateDecomposeConfig) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError>`: the diagnostic form, additionally returning the decomposition rule statistics of this run.
- `decompose_mc_gates_for_device(circuit: &Circuit, device: &Device, resource_policy: ResourcePolicy) -> Result<TransformOutcome, CompilerError>`: rewrite multi-controlled gates within the limit of the qubits available on the device; this is a pre-layout logical transform and does not check the coupling topology.

### McGateDecomposeConfig

```rust
pub struct McGateDecomposeConfig {
    pub resource_policy: ResourcePolicy,
    pub resource_limits: ResourceLimits,
}
```

- `resource_policy`: the ancilla resource permissions of this decomposition pass.
- `resource_limits`: the hard logical qubit limit.

---

## Multi-controlled gate primitives

`cqlib_core::compile::transform::decompose::mc_gate` provides exact synthesis primitives grouped by gate family; they return the operation sequence directly and do not traverse circuits, select algorithms or request ancillas. **The normal compilation flow should use `DecomposeMcGates` through the transformer interface**; call these primitives directly only when ancilla allocation must be controlled manually or a custom lowering strategy is being implemented.

The primitives are grouped by gate family, and each group usually provides a no-ancilla form, a several-clean-ancillas form, a several-dirty-ancillas form and a fixed-depth variant for a single ancilla:

| Gate family | Coverage |
| --- | --- |
| MCX | Multi-controlled X gates. |
| Multi-controlled SU(2) | Multi-controlled special unitary rotations, with the rotation axis given by `Su2RotationAxis`. |
| Pauli | The multi-controlled Pauli gate family. |
| RZZ | Multi-controlled RZZ. |
| Pauli rotations | Multi-controlled Pauli rotations. |
| Rotations | Multi-controlled `RX` / `RY` / `RZ` and their intrinsic controlled forms. |
| Phase | Multi-controlled `S` / `SDG` / `T` / `TDG` / `Phase`. |
| QCIS | Synthesis primitives for the QCIS gate set. |
| Hadamard | Multi-controlled Hadamard. |
| SWAP | Multi-controlled SWAP. |
| FSIM | Multi-controlled FSIM. |
| Unitary gates | Multi-controlled general unitary gates. |

Common parameters:

- `controls` (`&[Qubit]`): the control qubits, which must be the full set of controls after expansion, including the intrinsic controls of the gate itself.
- `target`, or `first` and `second` for two-qubit primitives: the target qubits.
- `clean_ancillas` / `clean_ancilla`: clean ancillas, which must enter as `|0>` and be restored to `|0>`.
- `dirty_ancillas` / `dirty_ancilla`: dirty ancillas, which may enter in any unknown state but must be restored exactly.

The primitives return an operation sequence that can be assembled into a circuit. Extra ancillas beyond the consumed prefix are ignored; ancillas must be distinct from all control qubits and target qubits. The complete list is in the public surface of the `mc_gate` module.

---

## Two-qubit block resynthesis

- `resynthesize_two_qubit_blocks(circuit: &Circuit, config: TwoQubitBlockResynthesisConfig) -> Result<TransformOutcome, CompilerError>`
- `ResynthesizeTwoQubitBlocks`: a `Transformer` adapter with the configuration bound at construction time, constructed with `ResynthesizeTwoQubitBlocks::new(config)`.

### TwoQubitBlockResynthesisConfig

```rust
pub struct TwoQubitBlockResynthesisConfig {
    pub two_qubit_target: TwoQubitSynthesisTarget,
    pub max_block_ops: usize,
    pub max_crossed_ops: usize,
    pub max_scan_span: usize,
    pub skip_labeled_ops: bool,
    pub recurse_control_flow: bool,
    pub commutation: CommutationConfig,
}
```

Fields:

- `two_qubit_target`: the target capabilities used by exact two-qubit numeric synthesis.
- `max_block_ops`: the maximum number of source operations allowed in a single synthesis block.
- `max_crossed_ops`: the maximum number of non-block operations that may be crossed when collecting a block. Crossed operations stay in place and must commute with the synthesis replacement.
- `max_scan_span`: the collection budget on each side of a two-qubit anchor.
- `skip_labeled_ops`: treat labeled operations as hard boundaries.
- `recurse_control_flow`: whether to recurse into structured classical control bodies.
- `commutation`: the semantic commutation engine configuration used by the local collector.

Methods:

- `TwoQubitBlockResynthesisConfig::normal(two_qubit_target: TwoQubitSynthesisTarget) -> Self`: the default budget configuration.
- `TwoQubitBlockResynthesisConfig::enhanced(two_qubit_target: TwoQubitSynthesisTarget) -> Self`: a relaxed-budget variant that trades compilation time for better post-routing cleanup.

---

## Target gate set translation

### TargetBasisLowerer

- `TargetBasisLowerer::new(target_basis: Vec<Instruction>) -> Result<Self, CompilerError>`
- `target_basis(&self) -> &[Instruction]`
- `requires_lowering(&self, circuit: &Circuit) -> bool`
- Implements `Transformer`.

### TargetBasisSignature

- `TargetBasisSignature::from_standard_gates(gates: &[StandardGate]) -> Self`

### TargetBasisCost

```rust
pub struct TargetBasisCost {
    pub two_qubit_ops: usize,
    pub depth: usize,
    pub total_ops: usize,
    pub parameterized_ops: usize,
}
```

### TargetBasisCostModel

- `TargetBasisCostModel::new(target_basis: Vec<Instruction>) -> Result<Self, CompilerError>`
- `TargetBasisCostModel::from_lowerer(lowerer: Arc<TargetBasisLowerer>) -> Result<Self, CompilerError>`
- `target_basis(&self) -> &[Instruction]`
- `cost_of_fixed_operations(&self, qubits: Vec<Qubit>, operations: Vec<ValueOperation>) -> Result<TargetBasisCost, CompilerError>`

---

## Device lowering

### DeviceLowerer

```rust
pub struct DeviceLowerer<'a> {
    device: &'a Device,
}
```

- `DeviceLowerer::new(device: &'a Device) -> Self`
- `device(&self) -> &'a Device`
- Implements `Transformer`; lowers the circuit to the device native instruction set.

---

## Example

### 1. Definition expansion and unitary gate synthesis

```rust
use cqlib_core::circuit::{Circuit, Qubit, UnitaryGate};
use cqlib_core::compile::transform::decompose::{
    UnitaryDecomposeConfig, decompose_unitaries, expand_definitions,
};
use ndarray::array;
use num_complex::Complex64;

// 定义展开：把 CircuitGate 展开为其定义线路
let mut definition = Circuit::new(1);
definition.h(Qubit::new(0)).unwrap();
let gate = definition.to_gate("custom_h").unwrap();

let mut circuit = Circuit::new(1);
circuit.append(gate, [Qubit::new(0)], [], None).unwrap();

let expanded = expand_definitions(&circuit).unwrap();
assert_eq!(expanded.num_qubits(), 1);

// 自定义酉门合成
let matrix = array![
    [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
];
let gate = UnitaryGate::new("x_matrix", 1, 0).with_matrix(matrix).unwrap();
let mut circuit = Circuit::new(1);
circuit.unitary(gate, vec![Qubit::new(0)]).unwrap();

let synthesized =
    decompose_unitaries(&circuit, UnitaryDecomposeConfig::default()).unwrap();
assert_eq!(synthesized.num_qubits(), 1);
```

### 2. Numeric unitary gate synthesis

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::compile::transform::decompose::unitary::{
    TwoQubitUnitaryDecomposeBasis, kak_decompose, synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary,
};
use ndarray::array;
use num_complex::Complex64;

let source = array![
    [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
];
let decomposition = synthesize_numeric_1q_unitary(&source).unwrap();
assert!(decomposition.theta.is_finite());
assert!(decomposition.global_phase.is_finite());

let swap = array![
    [
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0)
    ],
    [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0)
    ],
    [
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0)
    ],
    [
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0)
    ],
];
let synthesis = synthesize_numeric_2q_unitary(
    &swap,
    [Qubit::new(0), Qubit::new(1)],
    TwoQubitUnitaryDecomposeBasis::Cx,
)
.unwrap();
assert!(!synthesis.operations.is_empty());

let kak = kak_decompose(&swap).unwrap();
assert_eq!(kak.k1l.dim(), (2, 2));
assert!(kak.a.is_finite());
```

### 3. Multi-controlled gate decomposition

```rust
use cqlib_core::circuit::{Circuit, Instruction, MCGate, Qubit, StandardGate};
use cqlib_core::compile::resource::ResourcePolicy;
use cqlib_core::compile::transform::TransformOutcome;
use cqlib_core::compile::transform::decompose::{McGateDecomposeConfig, decompose_mc_gates};

let mut circuit = Circuit::new(3);
circuit
    .append(
        Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))),
        [Qubit::new(0), Qubit::new(1), Qubit::new(2)],
        [],
        None,
    )
    .unwrap();

let result = decompose_mc_gates(
    &circuit,
    McGateDecomposeConfig {
        resource_policy: ResourcePolicy::default(),
        ..McGateDecomposeConfig::default()
    },
)
.unwrap();

let TransformOutcome::Changed(decomposed) = result else {
    panic!("the MC gate should be decomposed");
};
assert!(matches!(
    decomposed.operations()[0].instruction,
    Instruction::Standard(StandardGate::CCX),
));
```

### 4. Two-qubit block resynthesis

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::transform::decompose::unitary::TwoQubitSynthesisTarget;
use cqlib_core::compile::transform::resynthesis::{
    TwoQubitBlockResynthesisConfig, resynthesize_two_qubit_blocks,
};

let mut circuit = Circuit::new(2);
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let config =
    TwoQubitBlockResynthesisConfig::normal(TwoQubitSynthesisTarget::unconstrained());
assert_eq!(config.max_block_ops, 16);

let result = resynthesize_two_qubit_blocks(&circuit, config).unwrap();
assert!(matches!(
    result,
    cqlib_core::compile::transform::TransformOutcome::Changed(_)
));
```

---

## Errors

- `CompilerError`: decomposition or synthesis failure, for example the matrix is not unitary, the target gate set cannot cover the input operations, or resources are insufficient.
- `CompilerError::InvalidInput`: the input matrix has the wrong number of qubits, contains non-finite elements, or is not unitary.
- `CompilerError::TransformFailed`: duplicated qubits, insufficient ancillas, an unsupported gate family, or an underlying synthesis failure.
