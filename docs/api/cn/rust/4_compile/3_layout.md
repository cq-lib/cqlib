# Layout

`cqlib_core::compile::transform::layout` 提供初始布局算法与布局分析工具。布局算法把逻辑比特映射到物理比特，为后续路由阶段提供起点。

## 导入

```rust
use cqlib_core::compile::transform::layout::{
    CircuitLayoutAnalysis, Interaction, InteractionGraph, LayoutDiagnostics, LayoutObjective,
    LayoutResult, LayoutScore, PhysicalLayoutGraph, Vf2EdgeRequirement, Vf2LayoutConfig,
    analyze_circuit_for_layout, greedy_layout, prepare_sabre_circuit, prepare_sabre_device_target,
    sabre_layout, trivial_layout, vf2_perfect_layout,
};
```

---

## 布局函数

- `trivial_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`：平凡布局，按编号顺序映射。
- `greedy_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`：贪心布局，按交互强度顺序放置。
- `vf2_perfect_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &Vf2LayoutConfig) -> Result<LayoutResult, CompilerError>`：VF2 精确布局，在线路交互图能精确嵌入设备拓扑时给出完美布局。
- `sabre_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &SabreConfig) -> Result<LayoutResult, CompilerError>`：SABRE 布局，使用 SABRE 启发式细化候选布局。

预计算入口：

- `analyze_circuit_for_layout(circuit: &Circuit) -> CircuitLayoutAnalysis`
- `prepare_sabre_circuit(circuit: &Circuit) -> PreparedSabreCircuit`
- `prepare_sabre_device_target(device: &Device) -> Result<PreparedSabreTarget, CompilerError>`
- `trivial_layout_prepared(analysis: &CircuitLayoutAnalysis, physical: &PhysicalLayoutGraph, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`
- `greedy_layout_prepared(...)`：同上。
- `vf2_perfect_layout_prepared(analysis, physical, objective, config) -> Result<LayoutResult, CompilerError>`
- `sabre_layout_prepared(prepared, prepared_target, objective, config) -> Result<LayoutResult, CompilerError>`

---

## LayoutObjective

布局目标权重，用于给候选布局打分。

```rust
pub struct LayoutObjective {
    pub distance_weight: f64,
    pub direction_weight: f64,
    pub two_qubit_error_weight: f64,
    pub readout_error_weight: f64,
}
```

- `LayoutObjective::topology_only() -> Self`：纯拓扑目标。
- `LayoutObjective::fidelity_aware() -> Self`：默认保真度感知目标。
- `LayoutObjective::auto_from_physical(physical: &PhysicalLayoutGraph) -> Self`：有校准数据时选择保真度感知目标，否则退回拓扑目标。
- `LayoutObjective::fidelity_required(physical: &PhysicalLayoutGraph) -> Result<Self, CompilerError>`：保真度感知目标，无校准数据时报错。
- `uses_fidelity(&self) -> bool`
- `score_layout(&self, analysis: &CircuitLayoutAnalysis, physical: &PhysicalLayoutGraph, layout: &Layout) -> Result<LayoutScore, CompilerError>`

---

## LayoutScore

布局打分结果。

```rust
pub struct LayoutScore {
    pub total: f64,
    pub distance: f64,
    pub direction: f64,
    pub two_qubit_error: f64,
    pub readout_error: f64,
    pub used_fidelity: bool,
}
```

---

## LayoutDiagnostics

布局过程诊断信息。

```rust
pub struct LayoutDiagnostics {
    pub is_perfect: bool,
    pub candidates_evaluated: usize,
    pub used_fidelity: bool,
    pub notes: Vec<String>,
}
```

- `LayoutDiagnostics::new() -> Self`
- `with_note(self, note: impl Into<String>) -> Self`：追加诊断说明。

---

## LayoutResult

布局函数返回对象。

```rust
pub struct LayoutResult {
    pub layout: Layout,
    pub score: Option<LayoutScore>,
    pub diagnostics: LayoutDiagnostics,
}
```

---

## Vf2EdgeRequirement / Vf2LayoutConfig

```rust
pub enum Vf2EdgeRequirement { ... }
```

- `Vf2EdgeRequirement::positive_interactions()`：只要求正交互边匹配。
- `Vf2EdgeRequirement::all_interactions()`：要求所有交互边匹配。

```rust
pub struct Vf2LayoutConfig {
    pub candidate_limit: usize,
    pub call_limit: Option<usize>,
    pub edge_requirement: Vf2EdgeRequirement,
}
```

实现 `Default`。

---

## 布局分析对象

### Interaction

一条逻辑比特对交互。

```rust
pub struct Interaction {
    pub left: LogicalQubit,
    pub right: LogicalQubit,
    pub weight: f64,
    pub directed_weight_left_to_right: f64,
    pub directed_weight_right_to_left: f64,
    pub first_seen_order: usize,
}
```

### InteractionGraph

线路交互图：

- `interactions(&self) -> impl Iterator<Item = &Interaction>`
- `is_empty(&self) -> bool`
- `logical_activity(&self) -> impl Iterator<Item = (LogicalQubit, f64)>`

### CircuitLayoutAnalysis

线路布局分析结果。

```rust
pub struct CircuitLayoutAnalysis {
    pub logical_qubits: Vec<LogicalQubit>,
    pub interactions: InteractionGraph,
}
```

### DistanceTable

物理图最短距离表：

- `qubits(&self) -> &[PhysicalQubit]`
- `distance(&self, a: PhysicalQubit, b: PhysicalQubit) -> Option<u32>`

### PhysicalLayoutGraph

设备物理布局图：

- `PhysicalLayoutGraph::from_device(device: &Device) -> Result<Self, CompilerError>`
- `physical_qubits(&self) -> &[PhysicalQubit]`
- `distances(&self) -> &DistanceTable`
- `distance(&self, a, b) -> Option<u32>`
- `is_adjacent_undirected(&self, a, b) -> bool`
- `readout_error(&self, qubit) -> Option<f64>`
- `supports_two_qubit_gate_directed(&self, a, b) -> bool`
- `two_qubit_gate_error_directed(&self, a, b) -> Option<f64>`
- `supports_directed_coupling(&self, a, b) -> bool`
- `has_fidelity_data(&self) -> bool`、`has_readout_error_data(&self) -> bool`、`has_two_qubit_error_data(&self) -> bool`

### PreparedSabreCircuit / PreparedSabreTarget

- `PreparedSabreCircuit`：由 `prepare_sabre_circuit()` 返回，携带线路分析（`analysis()`、`logical_qubits()`）。
- `PreparedSabreTarget`：由 `prepare_sabre_device_target()` 返回，携带 `physical()` 物理图。

---

## 示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::sabre::SabreConfig;
use cqlib_core::compile::transform::layout::{LayoutObjective, sabre_layout};
use cqlib_core::device::Device;

let mut circuit = Circuit::new(3);
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

let device = Device::line("line-3", 3).unwrap();
let result = sabre_layout(
    &circuit,
    &device,
    &LayoutObjective::topology_only(),
    &SabreConfig::default(),
)
.unwrap();

println!("layout: {:?}", result.layout);
```
