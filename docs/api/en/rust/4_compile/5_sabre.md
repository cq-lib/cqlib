# SABRE

`cqlib_core::compile::sabre` provides the low-level entry points and configuration for SWAP routing. Use `sabre_route` when the initial layout is known and the circuit must satisfy the device topology constraint. Fixing the random seed makes the tie-breaking and selection across multiple trials reproducible.

## Import

```rust
use cqlib_core::compile::sabre::{
    SabreConfig, SabreHeuristicConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    SabreVf2PrepassConfig, normalize_initial_layout, sabre_route, validate_reachable_interactions,
};
```

---

## Functions

### `sabre_route(circuit: &Circuit, device: &Device, initial_layout: &Layout, config: &SabreConfig) -> Result<SabreRoutingResult, CompilerError>`

SABRE routing given a known initial layout.

### `normalize_initial_layout(logical_qubits: &[LogicalQubit], device: &Device, initial_layout: &Layout) -> Result<Layout, CompilerError>`

Normalize a complete logical-to-physical layout against the qubits available on the device.

### `validate_reachable_interactions(circuit: &Circuit, device: &Device, initial_layout: &Layout) -> Result<(), CompilerError>`

Validate native movement reachability without performing routing.

---

## SabreConfig

Configuration shared by SABRE layout refinement and routing.

```rust
pub struct SabreConfig {
    pub layout_trials: usize,
    pub layout_assignment_budget: usize,
    pub vf2_prepass: Option<SabreVf2PrepassConfig>,
    pub refinement_iterations: usize,
    pub routing_trials: usize,
    pub seed: Option<u64>,
    pub heuristic: SabreHeuristicConfig,
}
```

Fields:

- `layout_trials`: the number of randomized initial layouts added on top of the deterministic and interaction-aware candidates.
- `layout_assignment_budget`: the maximum amount of exploration of layout component assignment states; exhaustion reports budget exhaustion, which is distinguished from proven infeasibility.
- `vf2_prepass`: the bounded VF2 prepass, used to add topologically perfect candidates; `None` disables the prepass.
- `refinement_iterations`: the number of forward/reverse refinement rounds per layout trial.
- `routing_trials`: the number of complete routing trials performed for each fully refined layout. The search returns the best route directly and does not route intermediate refinement states.
- `seed`: the deterministic random seed. The same seed produces the same cqlib result.
- `heuristic`: the swap selection heuristic configuration.

Methods:

- `SabreConfig::deterministic_seeded(seed: u64) -> Self`: a compact deterministic configuration, suitable for tests and examples.
- `validate(&self) -> Result<(), CompilerError>`: validate the routing fields (layout refinement fields are intentionally ignored, since routing starts from a concrete initial layout and does not depend on layout-specific knobs).

Implements `Default`: `layout_trials=10`, `layout_assignment_budget=1_000_000`, `vf2_prepass=Some(SabreVf2PrepassConfig { candidate_limit: 10, call_limit: 1_000_000 })`, `refinement_iterations=1`, `routing_trials=1`, `seed=None`, `heuristic=SabreHeuristicConfig::default()`.

---

## SabreHeuristicConfig

SABRE swap selection heuristic configuration. The main score combines the current frontier layer sum, the lookahead scaled by active layers and multiplicative congestion; when a native plan is available, candidates are decided within a narrow structural window using the exact native two-qubit cost.

```rust
pub struct SabreHeuristicConfig {
    pub basic_weight: f64,
    pub lookahead_weights: Vec<f64>,
    pub decay_increment: Option<f64>,
    pub decay_reset: usize,
    pub attempt_limit: usize,
    pub best_epsilon: f64,
}
```

Fields:

- `basic_weight`: the weight of the total distance of the current frontier layer.
- `lookahead_weights`: the weight scaling the lookahead layer sum per active layer; the default is `[0.5, 0.25, 0.125, 0.0625, 0.03125]`.
- `decay_increment`: the increment of the congestion multiplier of a physical qubit after a heuristic SWAP uses it; `None` disables congestion control.
- `decay_reset`: the number of heuristic SWAP attempts before the decay value is reset.
- `attempt_limit`: the number of consecutive heuristic SWAPs allowed while no frontier node has been routed; beyond it the search falls back to shortest-path escape.
- `best_epsilon`: the floating-point tolerance at which candidate SWAP scores are treated as tied.

Implements `Default`: `basic_weight=1.0`, `lookahead_weights=[0.5, 0.25, 0.125, 0.0625, 0.03125]`, `decay_increment=Some(0.002)`, `decay_reset=10`, `attempt_limit=1000`, `best_epsilon=1e-10`.

---

## SabreVf2PrepassConfig

The bounded VF2 prepass used to seed SABRE layout candidates.

```rust
pub struct SabreVf2PrepassConfig {
    pub candidate_limit: usize,
    pub call_limit: usize,
}
```

- `candidate_limit`: the upper limit on the number of complete perfect mappings scored by VF2.
- `call_limit`: the upper limit on the number of partial mapping extensions attempted by VF2.

---

## SabreRoutingResult

The object returned by `sabre_route`.

```rust
pub struct SabreRoutingResult {
    pub circuit: Circuit,
    pub initial_layout: Layout,
    pub final_layout: Layout,
    pub swap_count: usize,
    pub diagnostics: SabreRoutingDiagnostics,
}
```

- `circuit`: the physical circuit after SWAP insertion.
- `initial_layout`: the initial logical-to-physical layout used by the selected trial.
- `final_layout`: the final layout after all routing operations.
- `swap_count`: the number of inserted SWAPs (including the control flow epilogue).
- `diagnostics`: diagnostics of the routing search behavior.

---

## SabreRoutingDiagnostics

Diagnostic information of the routing process; the fields are public:

```rust
pub struct SabreRoutingDiagnostics {
    pub trials_evaluated: usize,
    pub selected_trial_index: usize,
    pub fallback_count: usize,
    pub control_flow_blocks_routed: usize,
    pub two_qubit_depth: usize,
    pub operation_count: usize,
    pub native_two_qubit_count: usize,
    pub native_two_qubit_depth: usize,
    pub native_total_depth: usize,
    pub native_operation_count: usize,
    pub unknown_loop_count: usize,
    pub requirement_signature_count: usize,
    pub eager_pair_state_count: usize,
    pub lazy_pair_l1_lookup_count: usize,
    pub lazy_pair_l1_hit_count: usize,
    pub lazy_pair_l1_cached_count: usize,
}
```

Implements `Default` and `PartialEq`.
