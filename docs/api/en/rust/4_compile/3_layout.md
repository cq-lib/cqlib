# Layout

`cqlib_core::compile::transform::layout` provides initial layout algorithms and layout analysis tools. A layout algorithm maps logical qubits to physical qubits and provides the starting point for the subsequent routing stage.

## Import

```rust
use cqlib_core::compile::transform::layout::{
    CircuitLayoutAnalysis, Interaction, InteractionGraph, LayoutDiagnostics, LayoutObjective,
    LayoutResult, LayoutScore, PhysicalLayoutGraph, Vf2EdgeRequirement, Vf2LayoutConfig,
    analyze_circuit_for_layout, greedy_layout, prepare_sabre_circuit, prepare_sabre_device_target,
    sabre_layout, trivial_layout, vf2_perfect_layout,
};
```

---

## Layout functions

- `trivial_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`: trivial layout, mapping in index order.
- `greedy_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`: greedy layout, placing in order of interaction strength.
- `vf2_perfect_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &Vf2LayoutConfig) -> Result<LayoutResult, CompilerError>`: VF2 exact layout, which produces a perfect layout when the circuit interaction graph can be embedded exactly into the device topology.
- `sabre_layout(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &SabreConfig) -> Result<LayoutResult, CompilerError>`: SABRE layout, refining candidate layouts with the SABRE heuristic.

Precomputed entry points:

- `analyze_circuit_for_layout(circuit: &Circuit) -> CircuitLayoutAnalysis`
- `prepare_sabre_circuit(circuit: &Circuit) -> PreparedSabreCircuit`
- `prepare_sabre_device_target(device: &Device) -> Result<PreparedSabreTarget, CompilerError>`
- `trivial_layout_prepared(analysis: &CircuitLayoutAnalysis, physical: &PhysicalLayoutGraph, objective: &LayoutObjective) -> Result<LayoutResult, CompilerError>`
- `greedy_layout_prepared(...)`: the same as above.
- `vf2_perfect_layout_prepared(analysis, physical, objective, config) -> Result<LayoutResult, CompilerError>`
- `sabre_layout_prepared(prepared, prepared_target, objective, config) -> Result<LayoutResult, CompilerError>`

---

## LayoutObjective

Layout objective weights, used to score candidate layouts.

```rust
pub struct LayoutObjective {
    pub distance_weight: f64,
    pub direction_weight: f64,
    pub two_qubit_error_weight: f64,
    pub readout_error_weight: f64,
}
```

- `LayoutObjective::topology_only() -> Self`: a purely topological objective.
- `LayoutObjective::fidelity_aware() -> Self`: the default fidelity-aware objective.
- `LayoutObjective::auto_from_physical(physical: &PhysicalLayoutGraph) -> Self`: select the fidelity-aware objective when calibration data is available, otherwise fall back to the topological objective.
- `LayoutObjective::fidelity_required(physical: &PhysicalLayoutGraph) -> Result<Self, CompilerError>`: the fidelity-aware objective; raises an error when no calibration data is available.
- `uses_fidelity(&self) -> bool`
- `score_layout(&self, analysis: &CircuitLayoutAnalysis, physical: &PhysicalLayoutGraph, layout: &Layout) -> Result<LayoutScore, CompilerError>`

---

## LayoutScore

The layout scoring result.

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

Diagnostic information of the layout process.

```rust
pub struct LayoutDiagnostics {
    pub is_perfect: bool,
    pub candidates_evaluated: usize,
    pub used_fidelity: bool,
    pub notes: Vec<String>,
}
```

- `LayoutDiagnostics::new() -> Self`
- `with_note(self, note: impl Into<String>) -> Self`: append a diagnostic note.

---

## LayoutResult

The object returned by the layout functions.

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

- `Vf2EdgeRequirement::positive_interactions()`: require only positive interaction edges to match.
- `Vf2EdgeRequirement::all_interactions()`: require all interaction edges to match.

```rust
pub struct Vf2LayoutConfig {
    pub candidate_limit: usize,
    pub call_limit: Option<usize>,
    pub edge_requirement: Vf2EdgeRequirement,
}
```

Implements `Default`.

---

## Layout analysis objects

### Interaction

A single logical qubit pair interaction.

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

The circuit interaction graph:

- `interactions(&self) -> impl Iterator<Item = &Interaction>`
- `is_empty(&self) -> bool`
- `logical_activity(&self) -> impl Iterator<Item = (LogicalQubit, f64)>`

### CircuitLayoutAnalysis

The circuit layout analysis result.

```rust
pub struct CircuitLayoutAnalysis {
    pub logical_qubits: Vec<LogicalQubit>,
    pub interactions: InteractionGraph,
}
```

### DistanceTable

The shortest distance table of the physical graph:

- `qubits(&self) -> &[PhysicalQubit]`
- `distance(&self, a: PhysicalQubit, b: PhysicalQubit) -> Option<u32>`

### PhysicalLayoutGraph

The device physical layout graph:

- `PhysicalLayoutGraph::from_device(device: &Device) -> Result<Self, CompilerError>`
- `physical_qubits(&self) -> &[PhysicalQubit]`
- `distances(&self) -> &DistanceTable`
- `distance(&self, a, b) -> Option<u32>`
- `is_adjacent_undirected(&self, a, b) -> bool`
- `readout_error(&self, qubit) -> Option<f64>`
- `supports_two_qubit_gate_directed(&self, a, b) -> bool`
- `two_qubit_gate_error_directed(&self, a, b) -> Option<f64>`
- `supports_directed_coupling(&self, a, b) -> bool`
- `has_fidelity_data(&self) -> bool`, `has_readout_error_data(&self) -> bool`, `has_two_qubit_error_data(&self) -> bool`

### PreparedSabreCircuit / PreparedSabreTarget

- `PreparedSabreCircuit`: returned by `prepare_sabre_circuit()`, carrying the circuit analysis (`analysis()`, `logical_qubits()`).
- `PreparedSabreTarget`: returned by `prepare_sabre_device_target()`, carrying the physical graph through `physical()`.

---

## Example

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
