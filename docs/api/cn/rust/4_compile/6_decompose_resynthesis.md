# Decompose / Resynthesis

`cqlib_core::compile::transform::decompose`

门分解、双比特块重综合、目标门集转换与设备降级，位于 `cqlib_core::compile::transform` 的 `decompose`、`resynthesis`、`target_basis` 与 `device_lowering` 子模块。

门分解按操作可用的表示分层：定义展开处理自带实现线路的门，数值合成处理带固定矩阵的酉门，多控制门分解处理受控操作。定义展开先于数值合成执行，使带实现线路的酉门在进入矩阵合成之前已经展开。

## 导入

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

## 定义展开

- `expand_definitions(circuit: &Circuit) -> Result<Circuit, CompilerError>`：展开电路门定义并返回新线路。
- `DecomposeDefinitions`：`Transformer` 适配器，实现 `decompose_definitions` 步骤。

---

## 酉门合成

线路级入口位于 `cqlib_core::compile::transform::decompose`，对应的数值合成原语位于 `cqlib_core::compile::transform::decompose::unitary`。

- `decompose_unitaries(circuit: &Circuit, config: UnitaryDecomposeConfig) -> Result<Circuit, CompilerError>`
- `decompose_unitaries_with_rule_stats(circuit: &Circuit, config: UnitaryDecomposeConfig) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError>`
- `DecomposeUnitaries`：`Transformer` 适配器，配置在构造时绑定，`DecomposeUnitaries::new(config)` 构造，实现 `Default`。

### UnitaryDecomposeConfig

```rust
pub struct UnitaryDecomposeConfig {
    pub two_qubit_target: TwoQubitSynthesisTarget,
    pub recurse_control_flow: bool,
}
```

字段：

- `two_qubit_target`：精确双比特数值合成使用的目标能力，由 `TwoQubitSynthesisTarget::from_instructions(target_basis: Option<&[Instruction]>)` 构造；`None` 表示无目标约束，启用中立的 Pauli 旋转回退。
- `recurse_control_flow`：是否递归进入控制流块。

实现 `Default`，等价于无目标约束且递归进入控制流块。

### DecompositionRuleStats

```rust
pub struct DecompositionRuleStats {
    pub hits: usize,
    pub misses: usize,
    pub inserts: usize,
}
```

pass 内规则缓存统计：命中、未命中与写入次数。

---

## 数值酉门合成

`cqlib_core::compile::transform::decompose::unitary` 提供矩阵到门序列的直接合成，不经过线路遍历。

- `synthesize_numeric_1q_unitary(matrix: &Array2<Complex64>) -> Result<OneQubitUnitaryDecomposition, CompilerError>`
- `synthesize_numeric_2q_unitary(matrix: &Array2<Complex64>, qubits: [Qubit; 2], basis: TwoQubitUnitaryDecomposeBasis) -> Result<TwoQubitUnitarySynthesisResult, CompilerError>`
- `kak_decompose(matrix: &Array2<Complex64>) -> Result<KakDecomposition, CompilerError>`

`Array2` 与 `Complex64` 分别来自 `ndarray` 与 `num_complex`。

针对目标门集的合成规划与代价评估由该子模块内部完成，不作为常规接口使用。

### OneQubitUnitaryDecomposition

```rust
pub struct OneQubitUnitaryDecomposition {
    pub theta: f64,
    pub phi: f64,
    pub lambda: f64,
    pub global_phase: f64,
}
```

所表示的矩阵为 `exp(i * global_phase) * U(theta, phi, lambda)`。

### TwoQubitUnitarySynthesisResult

```rust
pub struct TwoQubitUnitarySynthesisResult {
    pub operations: Vec<ValueOperation>,
    pub global_phase: f64,
}
```

- `operations`：实现该酉门的标准门操作序列，精确到 `global_phase`。
- `global_phase`：乘在操作序列上的标量相位。

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

字段：

- `global_phase`：乘在完整分解上的标量相位。
- `k1l`、`k1r`：作用在 Cartan 相互作用之后的左、右局部因子。
- `k2l`、`k2r`：作用在 Cartan 相互作用之前的左、右局部因子。
- `a`、`b`、`c`：规范 Pauli-XX、Pauli-YY、Pauli-ZZ 相互作用坐标。

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

变体：

- `PauliRotations`：局部 `U` 门加 `RXX`、`RYY`、`RZZ`。
- `Cx`、`Cy`、`Cz`：局部 `U` 门加对应的受控门模板。
- `Rzz`：局部 `U` 门加 `RZZ`。

### TwoQubitSynthesisTarget

目标门集能力描述，供数值酉门合成与双比特块重综合指明后端支持的原生门集。

```rust
pub struct TwoQubitSynthesisTarget {
    // 字段私有
}
```

方法：

- `from_instructions(target_basis: Option<&[Instruction]>) -> Result<Self, CompilerError>`：从目标门集构造能力描述；`None` 表示无目标约束。
- `from_standard_gates(native_1q: Vec<StandardGate>, native_2q: Vec<StandardGate>, fallback_pauli: bool) -> Result<Self, CompilerError>`：直接给定原生单比特门与双比特门。
- `unconstrained() -> Self`：无目标约束，启用中立的 Pauli 旋转回退。
- `native_1q(&self) -> &[StandardGate]` / `native_2q(&self) -> &[StandardGate]`：读取已登记的原生门。
- `fallback_pauli(&self) -> bool`：是否启用 Pauli 旋转回退。

实现 `Default`，等价于 `unconstrained()`。

---

## 多控制门分解

- `DecomposeMcGates`：`Transformer` 适配器，配置在构造时绑定，用 `DecomposeMcGates::new(config)` 构造，实现 `Default`。
- `decompose_mc_gates(circuit: &Circuit, config: McGateDecomposeConfig) -> Result<TransformOutcome, CompilerError>`：重写多控制门。
- `decompose_mc_gates_with_rule_stats(circuit: &Circuit, config: McGateDecomposeConfig) -> Result<(TransformOutcome, DecompositionRuleStats), CompilerError>`：诊断形式，额外返回本次运行的分解规则统计。
- `decompose_mc_gates_for_device(circuit: &Circuit, device: &Device, resource_policy: ResourcePolicy) -> Result<TransformOutcome, CompilerError>`：在设备可用比特数上限内重写多控制门；这是布局前的逻辑变换，不检查耦合拓扑。

### McGateDecomposeConfig

```rust
pub struct McGateDecomposeConfig {
    pub resource_policy: ResourcePolicy,
    pub resource_limits: ResourceLimits,
}
```

- `resource_policy`：本分解 pass 的辅助比特资源权限。
- `resource_limits`：硬逻辑比特上限。

---

## 多控制门原语

`cqlib_core::compile::transform::decompose::mc_gate` 提供按门族划分的精确合成原语，直接给出操作序列，不做线路遍历、不选择算法、不申请辅助比特。**正常编译流程应使用 `DecomposeMcGates` 走转换器接口**；只有在需要自行控制辅助比特分配或实现自定义降级策略时，才直接调用这些原语。

原语按门族分组，每组通常提供「无辅助比特」「若干干净辅助比特」「若干脏辅助比特」以及针对单辅助比特的固定深度变体：

| 门族 | 覆盖范围 |
| --- | --- |
| MCX | 多控制 X 门。 |
| 多控制 SU(2) | 多控制特殊酉旋转，旋转轴由 `Su2RotationAxis` 给出。 |
| Pauli | 多控制 Pauli 门族。 |
| RZZ | 多控制 RZZ。 |
| Pauli 旋转 | 多控制 Pauli 旋转。 |
| 旋转 | 多控制 `RX` / `RY` / `RZ` 及其固有受控形态。 |
| 相位 | 多控制 `S` / `SDG` / `T` / `TDG` / `Phase`。 |
| QCIS | 面向 QCIS 门集的合成原语。 |
| Hadamard | 多控制 Hadamard。 |
| SWAP | 多控制 SWAP。 |
| FSIM | 多控制 FSIM。 |
| 酉门 | 多控制通用酉门。 |

共同参数：

- `controls` (`&[Qubit]`)：控制比特，须为展开后的全部控制，含门本身固有的控制。
- `target`，或双比特原语的 `first`、`second`：目标比特。
- `clean_ancillas` / `clean_ancilla`：干净辅助比特，须以 `|0>` 进入并恢复为 `|0>`。
- `dirty_ancillas` / `dirty_ancilla`：脏辅助比特，可以任意未知态进入，但须精确恢复。

原语返回操作序列，可组装为线路。超出消耗前缀的额外辅助比特被忽略；辅助比特必须与所有控制比特和目标比特互不相同。完整清单见 `mc_gate` 模块的公开面。

---

## 双比特块重综合

- `resynthesize_two_qubit_blocks(circuit: &Circuit, config: TwoQubitBlockResynthesisConfig) -> Result<TransformOutcome, CompilerError>`
- `ResynthesizeTwoQubitBlocks`：`Transformer` 适配器，配置在构造时绑定，`ResynthesizeTwoQubitBlocks::new(config)` 构造。

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

字段：

- `two_qubit_target`：精确双比特数值合成使用的目标能力。
- `max_block_ops`：单个合成块允许的最大源操作数。
- `max_crossed_ops`：收集块时允许跨越的最大非块操作数。被跨越操作保持在原位置，且必须与合成替换对易。
- `max_scan_span`：双比特锚点单侧的收集预算。
- `skip_labeled_ops`：把带标签操作视为硬边界。
- `recurse_control_flow`：是否递归进入结构化经典控制体。
- `commutation`：本地收集器使用的语义对易引擎配置。

方法：

- `TwoQubitBlockResynthesisConfig::normal(two_qubit_target: TwoQubitSynthesisTarget) -> Self`：默认预算配置。
- `TwoQubitBlockResynthesisConfig::enhanced(two_qubit_target: TwoQubitSynthesisTarget) -> Self`：放宽预算的变体，以编译时间换更好的路由后清理。

---

## 目标门集转换

### TargetBasisLowerer

- `TargetBasisLowerer::new(target_basis: Vec<Instruction>) -> Result<Self, CompilerError>`
- `target_basis(&self) -> &[Instruction]`
- `requires_lowering(&self, circuit: &Circuit) -> bool`
- 实现 `Transformer`。

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

## 设备降级

### DeviceLowerer

```rust
pub struct DeviceLowerer<'a> {
    device: &'a Device,
}
```

- `DeviceLowerer::new(device: &'a Device) -> Self`
- `device(&self) -> &'a Device`
- 实现 `Transformer`，把线路降级到设备原生指令集。

---

## 示例

### 1. 定义展开与酉门合成

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

### 2. 数值酉门合成

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

### 3. 多控制门分解

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

### 4. 双比特块重综合

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

## 错误

- `CompilerError`：分解或合成失败，例如矩阵不是酉矩阵、目标门集无法覆盖输入操作、资源不足。
- `CompilerError::InvalidInput`：输入矩阵比特数不符、含非有限元素或非酉。
- `CompilerError::TransformFailed`：比特重复、辅助比特数量不足、门族不受支持或底层合成失败。
