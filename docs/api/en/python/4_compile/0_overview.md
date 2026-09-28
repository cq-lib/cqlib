# Compilation

`cqlib.compile`

`cqlib.compile` is the compilation pipeline module of Cqlib. Through a single end-to-end `compile()` entry point it covers the complete flow from a logical circuit to a circuit that satisfies the target constraints (a physical circuit under a device target), and it also splits each compilation stage into independently callable transform submodules for callers that need fine-grained control over the compilation process.

## Overview

Compilation is the process of rewriting a given input circuit to match a specific quantum device topology and to optimize it for the execution goal. Most circuits must go through a series of transforms before they are compatible with the target device and before the impact of noise on the results is reduced. The recommended entry point of Cqlib is the `compile()` function: a single call performs canonicalization, knowledge rule optimization, gate decomposition, optional layout and routing, target gate set translation and final validation.

### Compilation pipeline

`compile()` orchestrates the following stages in a fixed order (the stage names correspond to the `stage` field of each record in `CompileResult.steps`):

1. **Target resolution and resource validation** (`pre_init`) — resolve the target constraint (`resolve.target`) and validate the ancilla resource policy (`validate.resources`).
2. **Input canonicalization** (`init`) — perform production-grade canonicalization on the input circuit (`canonicalize.input`).
3. **Definition expansion and pre-decomposition optimization** (`optimization`) — first expand user-defined gate definitions so that knowledge rules can see the operations inside composite gates; then perform pre-decomposition knowledge rule rewriting (`optimize.pre_decomposition`).
4. **Gate decomposition** (`optimization`) — matrix unitary gate synthesis, multi-controlled gate decomposition (requesting ancillas according to the resource policy), post-decomposition canonicalization, commutative cancellation (`optimize.commutative_cancellation`), two-qubit block resynthesis and single-qubit optimization.
5. **Post-decomposition optimization** (`optimization`) — knowledge rule rewriting iterated to a fixed point (`optimize.post_decomposition`).
6. **Routing gate set lowering** (`translation`) — lower gates unsupported by the SABRE gate-arity constraint (such as `CCX`) to operations acting on at most two qubits.
7. **Layout and routing** (`routing`) — under a device target, select the initial layout, insert SWAPs to satisfy the topology adjacency constraint (`route.sabre`), and perform cleanup and resynthesis after routing.
8. **Target gate set translation** (`translation`) — lower the circuit to the target gate set; a device target further lowers it to the native instruction set.
9. **Native optimization and final validation** (`optimization` / `validation`) — run fixed-point optimization on native instructions (`optimize.native_fixed_point`); a device target finally performs device validation (`validate.device`).

Without a device target, the layout, routing and device validation steps are skipped, and the reason for the skip is recorded in `steps`. Every stage can be called separately, skipping `compile()`; see [Transform](2_transform.md) and the following pages.

### Choosing the optimization strength

`CompileMode.normal()` is conservative logical optimization using production default pass parameters; `CompileMode.enhanced()` uses a stronger staged workflow, a larger pass budget, and performs target-aware cleanup when a target constraint is present. For most use cases `normal` is sufficient; use `enhanced` when the compilation time budget is generous and a better post-routing cleanup result is desired.

### Choosing a target constraint

- `CompileTarget.logical()`: logical optimization only; the output is not bound to any hardware. Suitable for algorithm-level circuit simplification.
- `CompileTarget.basis([...])`: lower to an explicit standard gate basis; device topology is not involved.
- `CompileTarget.device(device)`: routing and lowering for a concrete device; the output must match the device's native capabilities, and final device validation is performed.
- `CompileTarget.topology_basis(device, [...])`: route on the device topology while lowering to an explicit gate basis; the device only provides capacity, layout and routing constraints and does not require the gate basis to match the device's native capabilities.

### Reproducibility

Layout and routing in compilation involve randomized heuristics. Passing the `seed` parameter (supported by `compile()`, `CompileTarget.device()` and `CompileTarget.topology_basis()`) yields deterministic results: the same circuit, the same configuration and the same seed produce the same cqlib result. Outputs are not guaranteed to be consistent across different cqlib versions; the seed only guarantees reproducibility within the same version.

---

## Common entry points

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, compile
from cqlib.device import Device

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 2)

device = Device.line("line-3", 3)

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    target_basis=["H", "CX", "RZ"],
    seed=42,
)

print("changed:", result.changed)
for step in result.steps:
    if step.changed and not step.skipped:
        print(step.stage, step.name)
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Compilation pipeline** | The flow that progressively converts an abstract circuit into a circuit satisfying the target constraints, including stages such as logical optimization, decomposition, layout, routing and target gate set translation. |
| **Pass** | An independent transform step in the compilation pipeline, for example canonicalization, knowledge rule rewriting or routing. Most passes can either be orchestrated automatically by `compile()` or be called separately. |
| **Transform** | A reusable transform object, for example `Canonicalizer` or `KnowledgeRewriter`. Calling `run()` yields a result object; the input circuit stays unchanged. |
| **Target constraint** | The conditions the compilation output must satisfy, including logical space (no constraint), a standard gate basis (`CompileTarget.basis`), a physical device (`CompileTarget.device`), and device topology plus gate basis (`CompileTarget.topology_basis`). |
| **Initial layout** | The mapping from logical qubits to physical qubits before compilation starts. It can be provided by the caller or selected automatically by a layout algorithm. |
| **Routing** | The process of making all two-body interactions satisfy the adjacency constraint on a restricted topology by inserting movement operations such as SWAPs. |
| **Gate decomposition** | Expanding composite gates, custom unitary gates or multi-controlled gates into more elementary gate sequences. |
| **Knowledge rule** | A rule describing an adjacent operation pattern and its replacement form, used for pattern simplification, cancellation and hardware native gate matching. |
| **Ancilla** | An auxiliary qubit requested on demand during decomposition. A clean ancilla is required to be in `\|0>` when entering and leaving the transform, and a dirty ancilla is required to have its original state fully restored. |

---

## `cqlib.compile` API Overview

### End-to-end compilation entry point

| Name | Description |
| --- | --- |
| [`compile`](1_compiler.md) | The recommended entry point; a single call performs configuration, execution and result reporting. |
| [`CompileConfig`](1_compiler.md) | Compilation configuration snapshot, containing the mode, the target constraint and the resource policy. |
| [`CompileMode`](1_compiler.md) | Optimization strength enumeration, including `normal` and `enhanced`. |
| [`CompileTarget`](1_compiler.md) | Target constraint enumeration, including logical, basis, device, and topology plus basis. |
| [`DeviceCompileTarget`](1_compiler.md) | Device target input, containing the device, an optional initial layout and a random seed. |
| [`CompileResult`](1_compiler.md) | Compilation result, containing the optimized circuit, the step-by-step report and the device layout metadata. |
| [`WorkflowStepReport`](1_compiler.md) | The execution record of a single workflow step. |
| [`DeviceCompilationMetadata`](1_compiler.md) | The initial and final layouts produced by device compilation. |
| [`CompilerWorkflow`](1_compiler.md) | Reusable workflow object that repeatedly runs compilation according to the configuration. |

### Basic transform passes

| Name | Description |
| --- | --- |
| [`TransformResult`](2_transform.md) | The generic transform result, containing the output circuit and the changed flag. |
| [`CircuitAnalysis`](2_transform.md) | Circuit feature snapshot, for example whether it contains measurements, control flow or composite gates. |
| [`Canonicalizer`](2_transform.md) / [`canonicalize_circuit`](2_transform.md) | Canonicalization pass that unifies instruction forms, removes no-ops and folds the global phase. |
| [`KnowledgeRewriter`](2_transform.md) / [`rewrite_circuit`](2_transform.md) | Pattern rewriting pass based on knowledge rules. |
| [`OptimizeOneQubitRuns`](2_transform.md) | Optimization of consecutive single-qubit gate runs. |
| [`CommutativeCancellation`](2_transform.md) | Self-inverse cancellation based on commutation. |
| [`LowerToRoutingBasis`](2_transform.md) / [`lower_to_routing_basis`](2_transform.md) | Routing gate set lowering, ensuring the routing stage only faces movable gates. |

### Layout and routing

| Name | Description |
| --- | --- |
| [`trivial_layout`](3_layout.md) / [`greedy_layout`](3_layout.md) / [`vf2_perfect_layout`](3_layout.md) / [`sabre_layout`](3_layout.md) | Four initial layout algorithms. |
| [`LayoutObjective`](3_layout.md) / [`LayoutScore`](3_layout.md) | Layout objective weights and scoring result. |
| [`CircuitLayoutAnalysis`](3_layout.md) / [`InteractionGraph`](3_layout.md) | Circuit interaction graph analysis, the input of the layout algorithms. |
| [`PhysicalLayoutGraph`](3_layout.md) / [`DistanceTable`](3_layout.md) | Device physical graph and shortest distance table. |
| [`route_with_layout`](4_routing.md) / [`route_sabre`](4_routing.md) | Transform-layer routing entry points. |
| [`RoutedCircuit`](4_routing.md) / [`SabreRouteResult`](4_routing.md) | Routing result objects. |
| [`SabreConfig`](5_sabre.md) / [`SabreHeuristicConfig`](5_sabre.md) / [`SabreVf2PrepassConfig`](5_sabre.md) | SABRE algorithm configuration. |
| [`sabre_route`](5_sabre.md) | SABRE routing given a known initial layout. |

### Decomposition, resynthesis and target gate set

| Name | Description |
| --- | --- |
| [`expand_definitions`](6_decompose_resynthesis.md) / [`decompose_unitaries`](6_decompose_resynthesis.md) / [`decompose_mc_gates`](6_decompose_resynthesis.md) | Definition expansion, unitary gate synthesis and multi-controlled gate decomposition. |
| [`UnitaryDecomposeConfig`](6_decompose_resynthesis.md) / [`McGateDecomposeConfig`](6_decompose_resynthesis.md) | Decomposition configurations. |
| [`ResynthesizeTwoQubitBlocks`](6_decompose_resynthesis.md) / [`resynthesize_two_qubit_blocks`](6_decompose_resynthesis.md) | Two-qubit block resynthesis. |
| [`TargetBasisLowerer`](6_decompose_resynthesis.md) / [`TargetBasisCostModel`](6_decompose_resynthesis.md) | Target gate set translation and cost evaluation. |
| [`DeviceLowerer`](6_decompose_resynthesis.md) | Device native instruction lowering. |

### Knowledge rules and resource management

| Name | Description |
| --- | --- |
| [`Rule`](7_knowledge.md) / [`RuleItem`](7_knowledge.md) / [`Condition`](7_knowledge.md) | Knowledge rule construction. |
| [`RuleLibrary`](7_knowledge.md) / [`RuleMetadata`](7_knowledge.md) / [`RuleKind`](7_knowledge.md) | Rule library validation, classification and queries. |
| [`loads`](7_knowledge.md) / [`load`](7_knowledge.md) / [`dumps`](7_knowledge.md) / [`dump`](7_knowledge.md) | Rule DSL reading and writing. |
| [`rule_matches_operations`](7_knowledge.md) / [`MatchBindings`](7_knowledge.md) | Rule structural matching. |
| [`ResourceManager`](8_resource.md) / [`ResourcePolicy`](8_resource.md) / [`ResourceLimits`](8_resource.md) | Ancilla resource planning and borrowing. |
| [`Commutation`](9_commutation.md) / [`CommutationChecker`](9_commutation.md) / [`check_commutation`](9_commutation.md) | Conservative commutation proofs. |

---

## Quick examples

### 1. End-to-end compilation

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, compile
from cqlib.device import Device

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 2)

device = Device.line("line-3", 3)

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    target_basis=["H", "CX", "RZ"],
    seed=42,
)

print("changed:", result.changed)
for step in result.steps:
    if step.changed and not step.skipped:
        print(step.stage, step.name)
```

### 2. Running a single transform

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

### 3. SABRE routing

```python
from cqlib import Circuit
from cqlib.compile.sabre import SabreConfig, sabre_route
from cqlib.device import Device, Layout

circuit = Circuit(2)
circuit.cx(0, 1)

result = sabre_route(
    circuit,
    Device.line("line-3", 3),
    Layout.from_pairs([(0, 0), (1, 2)], physical_count=3),
    SabreConfig(routing_trials=1, seed=7),
)
assert result.swap_count == 1
```

---

## Validation and error handling

The compilation process may fail at the configuration parsing, transform execution and device validation stages. Common exceptions include:

| Exception | When it occurs |
| --- | --- |
| `CompilerConfigError` | The compilation configuration is invalid, for example `target` is provided together with `target_basis` / `device`, the mode name is unknown, or the target gate set is empty. |
| `CompilerTransformError` | A transform pass fails to execute, for example decomposition or routing cannot be completed under the given constraints. |
| `CompilerInternalError` | A compilation internal consistency check fails. |
| `CompilerError` | The common base class of the exceptions above. |

For device-related errors see the device module documentation; for knowledge rule parsing errors see [Knowledge](7_knowledge.md); for insufficient resource errors see [Resource](8_resource.md).
