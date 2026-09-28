# Transform

`cqlib_core::compile::transform` provides reusable compilation passes. All passes implement the `Transformer` trait and return a `TransformOutcome`, leaving the input circuit unmodified. The end-to-end `compile()` orchestrates these passes automatically; they can be invoked directly when a single stage needs to be observed or debugged.

## Import

```rust
use cqlib_core::compile::transform::{
    CircuitAnalysis, CanonicalizeConfig, CanonicalizeResult, Canonicalizer,
    canonicalize_circuit, CommutativeCancellation, KnowledgeRewriteResult, KnowledgeRewriteStats,
    KnowledgeRewriter, LowerToRoutingBasis, OptimizeOneQubitRuns, RewriteConfig, RewriteMode,
    TransformOutcome, Transformer, rewrite_circuit,
};
```

---

## Transformer / TransformOutcome

The unified transform trait and its result object.

```rust
pub trait Transformer {
    fn name(&self) -> &'static str;
    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError>;
}
```

The `analysis` parameter of `transform` is an optional precomputed circuit analysis: passing it avoids repeated structural scans across workflow stages, and with `None` the transform derives the facts it needs itself. `TransformOutcome` methods:

- `changed(&self) -> bool`: whether the pass changed the circuit.
- `into_circuit(self, original: &Circuit) -> Circuit`: take out the output circuit; with `Unchanged` it is equivalent to the input (the core workflow should match the enum directly to preserve zero-copy).

---

## CircuitAnalysis

A snapshot of circuit features, used by the workflow to decide which passes to run. The fields are public:

```rust
pub struct CircuitAnalysis {
    pub has_classical_data: bool,
    pub has_classical_control: bool,
    pub has_measurement: bool,
    pub has_classical_values: bool,
    pub has_classical_vars: bool,
    pub has_runtime_classical: bool,
    pub needs_classical_handle_preservation: bool,
    pub has_circuit_gate_definitions: bool,
    pub has_unitary_circuit_definitions: bool,
    pub has_unitary_gates: bool,
    pub has_mc_gates: bool,
}
```

- `CircuitAnalysis::analyze(circuit: &Circuit) -> Self`: analyze the circuit and return the snapshot.

---

## Canonicalizer / canonicalize_circuit

The canonicalization pass, which unifies instruction forms, folds the global phase and removes no-ops.

### CanonicalizeConfig

The fields are private and configured through builder methods:

- `CanonicalizeConfig::new() -> Self`: the production default configuration.
- `CanonicalizeConfig::production() -> Self`: the production default configuration (the same as `new`).
- `with_round_limit(self, round_limit: u8) -> Self`: the maximum number of canonicalization rounds.
- `recurse_control_flow(self, enabled: bool) -> Self`: whether to recurse into control flow blocks.
- The remaining options `fold_gphase`, `canonicalize_instruction_form`, `drop_noops` and `canonicalize_barriers` each have a builder method of the same name.

### Canonicalizer

- `Canonicalizer::new(config: CanonicalizeConfig) -> Self`
- `Canonicalizer::production() -> Self`: the production default configuration.
- `config(&self) -> &CanonicalizeConfig`
- `run(&self, circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError>`
- Implements `Transformer` and `Default`.

### CanonicalizeResult

```rust
pub struct CanonicalizeResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub rounds: u8,
}
```

### canonicalize_circuit(circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError>

The functional entry point, using the production default configuration.

---

## KnowledgeRewriter / rewrite_circuit

The pattern rewriting pass based on knowledge rules, and the core of logical optimization in the compilation pipeline.

### RewriteMode

```rust
pub enum RewriteMode {
    Optimize,
    Lowering,
}
```

- `Optimize`: conservative optimization; an accepted replacement must strictly improve the local cost.
- `Lowering`: explicit lowering; decomposition and hardware native rules may expand locally.

### RewriteConfig

The fields are private and configured through builder methods:

- `RewriteConfig::new() -> Self`: the production default configuration.
- `RewriteConfig::production() -> Self`: the production optimization configuration.
- `RewriteConfig::lowering() -> Self`: the lowering configuration.
- `with_max_rounds(self, max_rounds: u8) -> Self`
- `with_max_window_ops(self, max_window_ops: usize) -> Self`
- `with_max_pattern_len(self, max_pattern_len: usize) -> Self`
- `recurse_control_flow(self, enabled: bool) -> Self`
- `skip_labeled_ops(self, enabled: bool) -> Self`
- `with_enabled_kinds(self, kinds: Vec<RuleKind>) -> Self`
- `with_mode(self, mode: RewriteMode) -> Self`
- `with_target_instructions(self, instructions: Vec<Instruction>) -> Self`
- `try_with_target_instructions(self, instructions: Vec<Instruction>) -> Result<Self, CompilerError>`
- `enabled_kinds(&self) -> &[RuleKind]`
- `target_instruction_basis(&self) -> Option<Vec<Instruction>>`

### KnowledgeRewriter

- `KnowledgeRewriter::new(config: RewriteConfig) -> Self`
- `KnowledgeRewriter::production() -> Self`
- `KnowledgeRewriter::lowering() -> Self`
- `config(&self) -> &RewriteConfig`
- `run(&self, circuit: &Circuit) -> Result<KnowledgeRewriteResult, CompilerError>`
- Implements `Transformer`.

### KnowledgeRewriteResult

```rust
pub struct KnowledgeRewriteResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub stats: KnowledgeRewriteStats,
}
```

### KnowledgeRewriteStats

```rust
pub struct KnowledgeRewriteStats {
    pub rounds_executed: u8,
    pub rules_applied: usize,
    pub changed_sequences: usize,
    pub reached_fixpoint: bool,
}
```

### rewrite_circuit(circuit: &Circuit, config: &RewriteConfig) -> Result<KnowledgeRewriteResult, CompilerError>

The functional entry point.

---

## OptimizeOneQubitRuns

Optimization of runs of consecutive single-qubit gates.

- `OptimizeOneQubitRuns::logical() -> Self`: a target-neutral optimizer using the strict logical cost.
- `OptimizeOneQubitRuns::basis(target_basis: Vec<Instruction>) -> Result<Self, CompilerError>`: optimize by the cost after exact lowering to the target gate set.
- Implements `Transformer`.

---

## CommutativeCancellation

A self-inverse cancellation pass based on commutation.

- `CommutativeCancellation::new() -> Self`
- Implements `Transformer`.

---

## LowerToRoutingBasis

The routing gate set lowering pass, which converts the circuit into a form suitable for the routing stage.

- `LowerToRoutingBasis::new(preferred_basis: Option<Vec<Instruction>>) -> Self`
- Implements `Transformer`.

---

## Example

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::transform::{CanonicalizeConfig, Canonicalizer, Transformer};

let mut circuit = Circuit::new(1);
circuit.i(Qubit::new(0)).unwrap();

let outcome = Canonicalizer::new(CanonicalizeConfig::production())
    .transform(&circuit, None)
    .unwrap();
assert!(outcome.changed());
```
