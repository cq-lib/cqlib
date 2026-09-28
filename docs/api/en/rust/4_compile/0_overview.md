# Compilation

`cqlib_core::compile`

`cqlib_core::compile` is the compilation pipeline module of Cqlib. Through the end-to-end `compile()` function it covers the complete flow from a logical circuit to a circuit that satisfies the target constraint (a physical circuit under a device target), and it also splits each compilation stage into independently callable transforms for callers that need fine-grained control.

## Overview

Compilation is the process of rewriting a given input circuit so that it matches a specific quantum device topology and is optimized for the execution target. Most circuits must go through a series of transforms before they are compatible with the target device and before the impact of noise on the results is reduced. The recommended entry point of Cqlib is the `compile()` function: a single call performs canonicalization, knowledge rule optimization, gate decomposition, optional layout and routing, target gate set translation and final validation.

### Compilation pipeline

Every step of `compile()` leaves a record in `CompileResult.steps`, and the `stage` field of that record is a **coarse stage classification** with exactly seven possible values:

| `stage` | Classification |
| --- | --- |
| `pre_init` | Target constraint resolution and resource preflight, completed before the circuit is changed. |
| `init` | Establish a stable high-level IR: input canonicalization and expansion of custom gate definitions. |
| `optimization` | Simplification: knowledge rule rewriting, commutative cancellation, resynthesis and single-qubit optimization. |
| `translation` | Reduction of operation kinds: unitary synthesis, multi-controlled gate decomposition, routing gate set lowering, target gate set translation and device native lowering. |
| `routing` | Initial layout selection and SWAP insertion. |
| `output` | Output canonicalization of the artifact. |
| `validation` | Device validation. |

The `stage` field and the step name are two different things: the step name (such as `optimize.pre_decomposition`) describes what the step does, while `stage` only states which class the step belongs to. The narrative division of the compilation flow below can therefore **span several `stage` values**; the two are not in one-to-one correspondence.

`compile()` orchestrates the following steps in a fixed order:

1. **Target resolution and resource validation** (`pre_init`) — resolve the target constraint (`resolve.target`) and validate the ancilla resource policy (`validate.resources`).
2. **Input canonicalization** (`init`) — perform production-grade canonicalization on the input circuit (`canonicalize.input`).
3. **Definition expansion and pre-decomposition optimization** (`init` → `optimization`) — first expand user custom gate definitions (`decompose.definitions`) so that knowledge rules can see the operations inside composite gates; then perform pre-decomposition knowledge rule rewriting (`optimize.pre_decomposition`).
4. **Gate decomposition** (`translation` → `optimization`) — matrix unitary gate synthesis (`decompose.unitary`), multi-controlled gate decomposition (`decompose.mc_gates`, requesting ancillas according to the resource policy), followed by post-decomposition canonicalization (`canonicalize.after_decomposition`), commutative cancellation (`optimize.commutative_cancellation`), two-qubit block resynthesis (`resynthesize.two_qubit_blocks`) and single-qubit optimization (`optimize.one_qubit.post_decomposition`).
5. **Post-decomposition optimization** (`optimization`) — knowledge rule rewriting (`optimize.post_decomposition`), then iterate single-qubit optimization and two-qubit block resynthesis to a fixed point (`optimize.one_qubit_fixed_point`).
6. **Routing gate set lowering** (`translation`) — lower gates that the SABRE gate-arity constraint does not support (such as `CCX`) into operations on at most two qubits (`decompose.routing_basis`).
7. **Layout and routing** (`routing` → `optimization`) — under a device target, select the initial layout and insert SWAPs to satisfy the topology adjacency constraint (`route.sabre`); in enhanced mode, perform cleanup and resynthesis after routing (`optimize.post_routing`).
8. **Target gate set translation** (`translation`) — lower the circuit to the target gate set (`translate.target_basis`); a device target lowers it further to the native instruction set (`lower.device_instructions`).
9. **Native optimization and final validation** (`optimization` / `validation`) — optimize native instructions to a fixed point (`optimize.native_fixed_point`); a device target finally performs device validation (`validate.device`).

Without a device target, the layout, routing and device validation steps are skipped, and the skip reason is recorded in `steps`. Each stage can also be invoked individually, bypassing `compile()`; see [Transform](2_transform.md) and the following pages.

### Choosing the optimization strength

`CompileMode::Normal` is conservative logical optimization and uses the production default pass parameters; `CompileMode::Enhanced` uses a stronger staged workflow and a larger pass budget, and performs target-aware cleanup when a target constraint is present. For most use cases `Normal` is sufficient; use `Enhanced` when the compilation time budget is ample and better post-routing cleanup is preferred.

### Choosing the target constraint

- `CompileTarget::Logical`: performs only logical optimization, and the output is not bound to any hardware. Suitable for circuit simplification at the algorithmic level.
- `CompileTarget::Basis(vec![...])`: lowers to an explicit standard basis, without involving device topology.
- `CompileTarget::Device(DeviceCompileTarget { .. })`: routing and lowering for a concrete device; the output must match the native capabilities of the device, and final device validation is performed.
- `CompileTarget::TopologyBasis { .. }`: routes on the device topology while lowering to an explicit basis; the device only provides capacity, layout and routing constraints and does not require the output basis to match the native capabilities of the device.

### Reproducibility

Layout and routing during compilation involve randomized heuristics. Passing `seed` in `DeviceCompileTarget` yields deterministic results: the same circuit, the same configuration and the same seed produce the same cqlib result. Outputs are not guaranteed to be identical across cqlib versions; the seed only guarantees reproducibility within the same version.

---

## Common entry points

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
use cqlib_core::compile::resource::ResourcePolicy;

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let result = compile(
    &circuit,
    CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Logical,
        resource_policy: ResourcePolicy::default(),
    },
)
.unwrap();

for step in &result.steps {
    if step.changed && !step.skipped {
        println!("{} {}", step.stage, step.name);
    }
}
```

---

## Core concepts and terms

| Term | Description |
| --- | --- |
| **Compilation pipeline** | The flow that progressively converts an abstract circuit into a circuit satisfying the target constraint, including stages such as logical optimization, decomposition, layout, routing and target gate set translation. |
| **Pass** | An independent transform step in the compilation pipeline. Most passes can either be orchestrated automatically by `compile()` or invoked individually. |
| **Transformer** | The unified trait of a transform. Implements `transform(&self, circuit, analysis) -> Result<TransformOutcome, CompilerError>`; the input circuit is left unchanged. |
| **Target constraint** | The conditions the compilation output must satisfy: logical space (`CompileTarget::Logical`), standard basis (`Basis`), physical device (`Device`) and device topology plus basis (`TopologyBasis`). |
| **Initial layout** | The mapping from logical qubits to physical qubits before compilation starts. It can be provided by the caller or selected automatically by a layout algorithm. |
| **Routing** | The process of inserting movement operations such as SWAPs on a constrained topology so that all two-body interactions satisfy the adjacency constraint. |
| **Gate decomposition** | Expanding a composite gate, custom unitary gate or multi-controlled gate into a sequence of more basic gates. |
| **Knowledge rule** | A rule describing adjacent operation patterns and their replacement form, used for pattern simplification, cancellation and native gate matching. |
| **Ancilla** | An auxiliary qubit requested on demand during decomposition. `AncillaRequirement::CleanZero` requires it to be in `\|0>` on entry to and exit from the transform, while `Dirty` requires its original state to be fully restored. |

---

## `cqlib_core::compile` API overview

### End-to-end compilation entry points

| Name | Description |
| --- | --- |
| [`compile`](1_compiler.md) | The recommended entry point; a single call performs configuration, execution and result reporting. |
| [`CompileConfig`](1_compiler.md) | Compilation configuration, containing the mode, target constraint and resource policy. |
| [`CompileMode`](1_compiler.md) | Optimization strength enum, including `Normal` and `Enhanced`. |
| [`CompileTarget`](1_compiler.md) | Target constraint enum, including logical, basis, device and topology plus basis. |
| [`DeviceCompileTarget`](1_compiler.md) | Device target input, containing the device, an optional initial layout and a random seed. |
| [`CompileResult`](1_compiler.md) | Compilation result, containing the optimized circuit, the step reports and the device layout metadata. |
| [`WorkflowStepReport`](1_compiler.md) | The execution record of a single workflow step. |
| [`DeviceCompilationMetadata`](1_compiler.md) | The initial and final layouts produced by device compilation. |
| [`CompilerWorkflow`](1_compiler.md) | A reusable workflow object. |

### Base transform passes

| Name | Description |
| --- | --- |
| [`Transformer`](2_transform.md) / [`TransformOutcome`](2_transform.md) | The unified transform trait and its result object. |
| [`CircuitAnalysis`](2_transform.md) | A snapshot of circuit features. |
| [`Canonicalizer`](2_transform.md) / [`canonicalize_circuit`](2_transform.md) | The canonicalization pass. |
| [`KnowledgeRewriter`](2_transform.md) / [`rewrite_circuit`](2_transform.md) | A pattern rewriting pass based on knowledge rules. |
| [`OptimizeOneQubitRuns`](2_transform.md) | Optimization of runs of consecutive single-qubit gates. |
| [`CommutativeCancellation`](2_transform.md) | Self-inverse cancellation based on commutation. |
| [`LowerToRoutingBasis`](2_transform.md) | Routing gate set lowering. |

### Layout and routing

| Name | Description |
| --- | --- |
| [`trivial_layout`](3_layout.md) / [`greedy_layout`](3_layout.md) / [`vf2_perfect_layout`](3_layout.md) / [`sabre_layout`](3_layout.md) | Four initial layout algorithms. |
| [`LayoutObjective`](3_layout.md) / [`LayoutScore`](3_layout.md) | Layout objective weights and scoring results. |
| [`CircuitLayoutAnalysis`](3_layout.md) / [`InteractionGraph`](3_layout.md) | Circuit interaction graph analysis. |
| [`PhysicalLayoutGraph`](3_layout.md) / [`DistanceTable`](3_layout.md) | Device physical graph and shortest distance table. |
| [`route_with_layout`](4_routing.md) / [`route_sabre`](4_routing.md) | Transform-level routing entry points. |
| [`RoutedCircuit`](4_routing.md) / [`SabreRouteResult`](4_routing.md) | Routing result objects. |
| [`SabreConfig`](5_sabre.md) / [`SabreHeuristicConfig`](5_sabre.md) / [`SabreVf2PrepassConfig`](5_sabre.md) | SABRE algorithm configuration. |
| [`sabre_route`](5_sabre.md) | SABRE routing given a known initial layout. |

### Decomposition, resynthesis and target gate set

| Name | Description |
| --- | --- |
| [`decompose` submodule](6_decompose_resynthesis.md) | Definition expansion, unitary gate synthesis and multi-controlled gate decomposition. |
| [`ResynthesizeTwoQubitBlocks`](6_decompose_resynthesis.md) / [`resynthesize_two_qubit_blocks`](6_decompose_resynthesis.md) | Two-qubit block resynthesis. |
| [`TargetBasisLowerer`](6_decompose_resynthesis.md) / [`TargetBasisCostModel`](6_decompose_resynthesis.md) | Target gate set translation and cost evaluation. |
| [`DeviceLowerer`](6_decompose_resynthesis.md) | Device native instruction lowering. |

### Knowledge rules and resource management

| Name | Description |
| --- | --- |
| [`Rule`](7_knowledge.md) / [`RuleItem`](7_knowledge.md) / [`Condition`](7_knowledge.md) | Knowledge rule construction. |
| [`RuleLibrary`](7_knowledge.md) / [`RuleMetadata`](7_knowledge.md) / [`RuleKind`](7_knowledge.md) | Rule library validation, classification and queries. |
| [`MatchBindings`](7_knowledge.md) / [`rule_matches_operations`](7_knowledge.md) | Rule structural matching. |
| [`ResourceManager`](8_resource.md) / [`ResourcePolicy`](8_resource.md) / [`ResourceLimits`](8_resource.md) | Ancilla resource planning and leasing. |
| [`Commutation`](9_commutation.md) / [`CommutationChecker`](9_commutation.md) / [`check_commutation`](9_commutation.md) | Conservative proof of commutation. |

---

## Quick examples

### 1. End-to-end compilation

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, compile};
use cqlib_core::compile::resource::ResourcePolicy;

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let result = compile(
    &circuit,
    CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Logical,
        resource_policy: ResourcePolicy::default(),
    },
)
.unwrap();

assert!(!result.steps.is_empty());
```

### 2. Running a single transform

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::transform::{Canonicalizer, CanonicalizeConfig, Transformer};

let mut circuit = Circuit::new(1);
circuit.i(Qubit::new(0)).unwrap();

let outcome = Canonicalizer::new(CanonicalizeConfig::production())
    .transform(&circuit, None)
    .unwrap();
assert!(outcome.changed());
```

---

## Validation and error handling

The compilation process can fail during configuration resolution, transform execution and device validation, and failures are uniformly returned through `CompilerError`. Common cases:

| Error | When it occurs |
| --- | --- |
| `CompilerError::InvalidInput` | Invalid configuration, for example `routing_trials` is zero or the target gate set is empty. |
| Other `CompilerError` variants | Transform execution failure, unreachable routing, insufficient device capacity, and so on. |

For device-related errors, refer to the device module documentation; for knowledge rule parsing errors, refer to [Knowledge](7_knowledge.md); for insufficient resource errors, refer to [Resource](8_resource.md).
