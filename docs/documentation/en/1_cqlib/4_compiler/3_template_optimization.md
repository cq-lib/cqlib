# Template Matching and Knowledge Rule Optimization

Template optimization is used to find a local gate sequence in a circuit and replace it with a sequence that is semantically equivalent, lower in cost or better suited to the target gate set. In everyday use, the template list usually does not need to be maintained manually; calling `compile()` directly applies the built-in rules to complete common optimizations.

To observe or debug the local optimization process, `KnowledgeRewriter` can be used. It and `compile()` use the same set of built-in knowledge rules. A rule usually consists of the following parts:

- `match`: the source operation sequence to be recognized;
- `require`: optional parameter constraints, such as equal angles, or equality modulo `2π` / `4π`;
- `rewrite`: the target operation sequence after replacement.

Typical rules include cancellation of adjacent inverse gates, rotation merging, zero-angle normalization, gate decomposition, target gate set rewriting and explicit commutation rules.

Neither `compile()` nor `KnowledgeRewriter` modifies the input circuit; instead they return a result object containing a new circuit. The circuits before and after optimization should remain quantum-semantically equivalent; if a rule involves a global phase, Cqlib explicitly preserves or folds the corresponding `GPhase` information.

---

## Using it in the compilation pipeline

It is recommended to use template optimization through the unified `compile()` entry point. The compilation workflow automatically combines canonicalization, definition expansion, knowledge rule rewriting, gate decomposition, optional routing and target gate set translation.

```python
from cqlib.circuit import Circuit
from cqlib.compile import compile

circuit = Circuit(1)
circuit.h(0)
circuit.h(0)

result = compile(circuit)
optimized = result.circuit

print("changed:", result.changed)
print("before:", len(circuit.operations))
print("after:", len(optimized.operations))

for step in result.steps:
    if "optimize" in step.name:
        print(step.name, step.changed)
```

For most usage scenarios, the rule rewriter does not need to be called directly. `compile()` chooses the production configuration and canonicalizes the circuit representation once more before output.

---

## Running knowledge rule rewriting directly

To run local rule optimization separately in tests, diagnostics or a custom compilation flow, `KnowledgeRewriter` can be used directly.

```python
from cqlib.circuit import Circuit
from cqlib.compile.transform import KnowledgeRewriter

circuit = Circuit(1)
circuit.x(0)
circuit.x(0)

result = KnowledgeRewriter.production().run(circuit)
optimized = result.circuit

print("changed:", result.changed)
print("rounds:", result.stats.rounds_executed)
print("rules:", result.stats.rules_applied)
print("after:", len(optimized.operations))
```

`KnowledgeRewriter.production()` uses conservative optimization rules, with simplification, cancellation, merging and canonicalization rules enabled by default. The rewriter accepts only rewrites that bring a local benefit, avoiding back-and-forth switching between equivalent forms of a circuit.

A functional entry point is also available:

```python
from cqlib.compile.transform import rewrite_circuit

result = rewrite_circuit(circuit)
optimized = result.circuit
```

---

## Rule examples

Built-in rules are described with a lightweight DSL. For example, H-H cancellation can be understood as:

```text
rule cancel_h {
    match { H 0, H 0 }
    rewrite {}
}
```

The rotation merging rule can be understood as:

```text
rule merge_rz {
    match { RZ(a) 0, RZ(b) 0 }
    rewrite { RZ(a + b) 0 }
}
```

Rules with conditions check the parameter relations first. For example, two mutually inverse `RZ` gates are removed only when the sum of their angles satisfies the corresponding modular relation; if they differ by a global phase, the rule explicitly generates a `GPhase`, which the canonicalizer then folds into the global phase of the circuit.

---

## Optimization configuration

`RewriteConfig` controls the rule search scope and the way rules are applied. On the Python side, configuration items are passed through the constructor.

```python
from cqlib.compile.transform import KnowledgeRewriter, RewriteConfig

config = RewriteConfig(
    max_rounds=8,
    max_window_ops=16,
    max_pattern_len=8,
    recurse_control_flow=True,
    skip_labeled_ops=True,
)

result = KnowledgeRewriter(config).run(circuit)
```

The meanings of several common parameters are as follows:

- `max_rounds`: the maximum number of rewriting rounds; if a fixed point is reached early, execution stops early;
- `max_window_ops`: the maximum operation window that a single local search may inspect;
- `max_pattern_len`: the maximum length of a matchable rule;
- `recurse_control_flow`: whether to recursively optimize control flow bodies;
- `skip_labeled_ops`: whether to skip operations carrying a label, so that debug markers or external conventions are not broken.

To use only the default production configuration, simply write:

```python
config = RewriteConfig.production()
result = KnowledgeRewriter(config).run(circuit)
```

---

## Target gate set rewriting

Template rules are used not only to "reduce the gate count", but also to rewrite a circuit to a specified target gate set. The recommended way to specify the target gate set is through the `target_basis` parameter of `compile()`:

```python
from cqlib.circuit import Circuit
from cqlib.compile import compile

circuit = Circuit(2)
circuit.cx(0, 1)

result = compile(circuit, target_basis=["H", "CZ"])
optimized = result.circuit

print([op.instruction for op in optimized.operations])
```

To call knowledge rule rewriting directly in a custom compilation flow, use the lowering configuration and pass the target instructions explicitly:

```python
from cqlib.circuit import Circuit, Instruction, StandardGate
from cqlib.compile.transform import KnowledgeRewriter, RewriteConfig, RewriteMode

circuit = Circuit(2)
circuit.cx(0, 1)

basis = [
    Instruction.from_standard_gate(StandardGate.H),
    Instruction.from_standard_gate(StandardGate.CZ),
    Instruction.from_standard_gate(StandardGate.RZ),
]

config = RewriteConfig(
    mode=RewriteMode.lowering(),
    target_instructions=basis,
)

result = KnowledgeRewriter(config).run(circuit)
```

In the complete compilation workflow, target gate set translation happens after physical routing, because routing may insert new `SWAP` gates or expose new local cleanup opportunities.

---

## Relation to traditional template matching

Traditional template optimization usually starts from "given a template circuit, find an equivalent subsequence in the circuit". The current Cqlib implementation is closer to "knowledge rule rewriting":

- templates are structured into verified rules;
- parameter relations are expressed by `require` conditions;
- rules are enabled by category, which makes it easy to distinguish optimization, decomposition and hardware-native gate translation;
- the rewriter uses a local cost model to control whether a rule is applied;
- the workflow invokes it repeatedly at multiple stages, until a fixed point or a budget limit is reached.

Template optimization can therefore be understood as "local gate sequence rewriting based on a built-in knowledge base". The recommended entry point in this documentation is `compile()`; use `KnowledgeRewriter` when the effect of individual local rules needs to be observed.

---

## Usage recommendations

- for ordinary compilation, prefer `compile()` rather than manually chaining multiple optimization steps;
- in custom flows, prefer `KnowledgeRewriter.production()`; use the lowering mode only for target gate set translation;
- when adding a new rule, structural legality, parameter constraints and semantic equivalence must all be considered;
- before and after optimization, compare the gate count, the two-qubit gate count, the depth and, for critical circuits, matrix or state equivalence;
- if a rule may change the global phase, explicitly preserve `GPhase` or confirm that the global phase has no effect on the task.

---

## Next steps

- [Commutation and Clifford-RZ optimization](4_commutative_and_clifford.md): learn how commutation checking helps rule reordering and rotation merging.
