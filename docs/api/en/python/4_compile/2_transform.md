# Transform

`cqlib.compile.transform` provides reusable compilation passes. Each pass returns a result object and does not modify the input circuit. The end-to-end `compile()` orchestrates these passes automatically; they can also be called directly when a stage needs to be observed or debugged on its own.

## Import

```python
from cqlib.compile.transform import (
    TransformResult,
    CircuitAnalysis,
    CanonicalizeConfig,
    Canonicalizer,
    CanonicalizeResult,
    canonicalize_circuit,
    RewriteMode,
    RewriteConfig,
    KnowledgeRewriter,
    KnowledgeRewriteResult,
    KnowledgeRewriteStats,
    rewrite_circuit,
    OptimizeOneQubitRuns,
    CommutativeCancellation,
    LowerToRoutingBasis,
    lower_to_routing_basis,
)
```

---

## TransformResult

The generic transform result object.

### Attributes

- `circuit -> Circuit`: the output circuit.
- `changed -> bool`: whether the pass changed the circuit.

### Other behavior

- Supports `==`, `copy` and `deepcopy`.

---

## CircuitAnalysis

Circuit feature snapshot, used by the workflow to decide which passes to execute. The full module path of this class is `cqlib.compile.transform.analysis`.

### CircuitAnalysis.analyze(circuit)

Static method that analyzes a circuit and returns the snapshot.

### Attributes

- `has_classical_data -> bool`: whether classical data is present.
- `has_classical_control -> bool`: whether classical control flow is present.
- `has_measurement -> bool`: whether measurements are present.
- `has_classical_values -> bool`: whether classical values (such as measurement results) are present.
- `has_classical_vars -> bool`: whether classical variables are present.
- `has_runtime_classical -> bool`: whether runtime classical semantics are present (a measurement value used by a later condition).
- `needs_classical_handle_preservation -> bool`: whether the transform must preserve classical handles.
- `has_circuit_gate_definitions -> bool`: whether circuit gate definitions are present.
- `has_unitary_circuit_definitions -> bool`: whether unitary circuit definitions are present.
- `has_unitary_gates -> bool`: whether custom unitary gates are present.
- `has_mc_gates -> bool`: whether multi-controlled gates are present.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform import CircuitAnalysis

circuit = Circuit(2)
circuit.h(0)
m = circuit.measure(0)

analysis = CircuitAnalysis.analyze(circuit)
assert analysis.has_measurement
```

---

## Canonicalizer / canonicalize_circuit

Canonicalization pass that unifies instruction forms, folds the global phase and removes no-ops.

### CanonicalizeConfig(*, round_limit=8, recurse_control_flow=true, fold_gphase=true, canonicalize_instruction_form=true, drop_noops=true, canonicalize_barriers=true)

Parameters:

- `round_limit` (`int`): the maximum number of canonicalization rounds.
- `recurse_control_flow` (`bool`): whether to recurse into control flow blocks.
- `fold_gphase` (`bool`): whether to fold the global phase.
- `canonicalize_instruction_form` (`bool`): whether to unify instruction forms.
- `drop_noops` (`bool`): whether to drop no-ops.
- `canonicalize_barriers` (`bool`): whether to canonicalize barriers.

### Static methods

- `CanonicalizeConfig.production()`: the production default configuration.

### Attributes

`round_limit`, `recurse_control_flow`, `fold_gphase`, `canonicalize_instruction_form`, `drop_noops`, `canonicalize_barriers`, corresponding one-to-one with the constructor parameters.

### Canonicalizer(config=None)

Parameters:

- `config` (`CanonicalizeConfig | None`): defaults to `CanonicalizeConfig.production()`.

### Static methods

- `Canonicalizer.production()`: construct with the production default configuration.

### Attributes and methods

- `config -> CanonicalizeConfig`
- `run(circuit) -> CanonicalizeResult`

### CanonicalizeResult

Attributes:

- `circuit -> Circuit`
- `changed -> bool`
- `rounds -> int`: the number of canonicalization rounds actually executed.

### canonicalize_circuit(circuit, config=None)

Functional entry point, equivalent to `Canonicalizer(config).run(circuit)`.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform import canonicalize_circuit

circuit = Circuit(1)
circuit.i(0)

result = canonicalize_circuit(circuit)
assert result.circuit is not circuit
assert len(circuit.operations) == 1
assert len(result.circuit.operations) == 0
assert result.changed
```

---

## KnowledgeRewriter / rewrite_circuit

Pattern rewriting pass based on knowledge rules; it is the core of logical optimization in the compilation pipeline.

### RewriteMode

Rewrite mode enumeration:

- `RewriteMode.optimize()`: conservative optimization; an accepted replacement must strictly improve the local cost.
- `RewriteMode.lowering()`: explicit lowering; decomposition and hardware native rules may expand locally.
- `RewriteMode.optimize().name -> "optimize"`, `RewriteMode.lowering().name -> "lowering"`.

### RewriteConfig(*, max_rounds=8, max_window_ops=16, max_pattern_len=8, recurse_control_flow=true, skip_labeled_ops=true, enabled_kinds=None, mode=None, target_instructions=None)

Parameters:

- `max_rounds` (`int`): the maximum number of rewriting rounds.
- `max_window_ops` (`int`): the maximum number of operations in the sliding window.
- `max_pattern_len` (`int`): the maximum pattern length.
- `recurse_control_flow` (`bool`): whether to recurse into control flow blocks.
- `skip_labeled_ops` (`bool`): whether to skip labeled operations.
- `enabled_kinds` (`list[RuleKind] | None`): the enabled rule kinds; `None` means all.
- `mode` (`RewriteMode | None`): defaults to `RewriteMode.optimize()`.
- `target_instructions` (`list[str | Instruction] | None`): an optional target instruction basis, used for target-aware cost evaluation.

### Static methods

- `RewriteConfig.production()`: the production optimization configuration.
- `RewriteConfig.lowering()`: the lowering configuration.

### Attributes

`max_rounds`, `max_window_ops`, `max_pattern_len`, `recurse_control_flow`, `skip_labeled_ops`, `enabled_kinds`, `mode`, `target_instructions`, corresponding one-to-one with the constructor parameters.

### KnowledgeRewriter(config=None)

Parameters:

- `config` (`RewriteConfig | None`): defaults to `RewriteConfig.production()`.

### Static methods

- `KnowledgeRewriter.production()`, `KnowledgeRewriter.lowering()`: construct with the corresponding preset configuration.

### Attributes and methods

- `config -> RewriteConfig`
- `run(circuit) -> KnowledgeRewriteResult`

### KnowledgeRewriteResult

Attributes:

- `circuit -> Circuit`
- `changed -> bool`
- `stats -> KnowledgeRewriteStats`

### KnowledgeRewriteStats

Attributes:

- `rounds_executed -> int`: the number of rounds actually executed.
- `rules_applied -> int`: the number of rule applications.
- `changed_sequences -> int`: the number of operation sequences that changed.
- `reached_fixpoint -> bool`: whether a fixed point was reached.

### rewrite_circuit(circuit, config=None)

Functional entry point, equivalent to `KnowledgeRewriter(config).run(circuit)`.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform import rewrite_circuit

circuit = Circuit(2)
circuit.h(0)
circuit.h(0)
circuit.cx(0, 1)

result = rewrite_circuit(circuit)
print(result.stats.rules_applied)
```

---

## OptimizeOneQubitRuns

Optimization of consecutive single-qubit gate runs. The full module path of this class is `cqlib.compile.transform.one_qubit_optimization`.

### Static methods

- `OptimizeOneQubitRuns.logical()`: a target-neutral optimizer using the strict logical cost.
- `OptimizeOneQubitRuns.basis(target_basis)`: optimize by the exact cost after lowering to `target_basis`. `target_basis` is a list of case-insensitive standard gate name strings or `Instruction` objects (multi-controlled gates must use `Instruction`).

### Attributes and methods

- `policy -> str`: the currently accepted policy, taking the values `"logical"` or `"basis"`.
- `target_basis -> list[Instruction] | None`: the explicit target gate set; `None` for logical optimization.
- `run(circuit) -> TransformResult`

Example:

```python
from cqlib import Circuit, Parameter
from cqlib.compile.transform import OptimizeOneQubitRuns

circuit = Circuit(1)
circuit.rx(0, Parameter("a"))
circuit.rx(0, Parameter("b"))

result = OptimizeOneQubitRuns.logical().run(circuit)
```

---

## CommutativeCancellation

Self-inverse cancellation pass based on commutation. The full module path of this class is `cqlib.compile.transform.commutative_cancellation`.

### CommutativeCancellation()

No-argument constructor.

### Methods

- `run(circuit) -> TransformResult`

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform import CommutativeCancellation

circuit = Circuit(2)
circuit.x(0)
circuit.cx(0, 1)
circuit.x(0)

result = CommutativeCancellation().run(circuit)
```

---

## LowerToRoutingBasis / lower_to_routing_basis

Routing gate set lowering pass, converting the circuit into a form suitable for the routing stage.

### LowerToRoutingBasis(preferred_basis=None)

Parameters:

- `preferred_basis` (`list[Instruction] | None`): the preferred gate basis; `None` uses the default routing gate basis.

### Methods

- `run(circuit) -> TransformResult`

### lower_to_routing_basis(circuit, preferred_basis=None)

Functional entry point.

Example:

```python
from cqlib import Circuit
from cqlib.compile.transform import lower_to_routing_basis

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

result = lower_to_routing_basis(circuit)
```
