# Compiler

The end-to-end compilation entry points of `cqlib_core::compile`, including the `compile()` function, the compilation configuration object, the compilation result object and the reusable workflow object.

## Import

```rust
use cqlib_core::compile::{
    CompileConfig, CompileMode, CompileResult, CompileTarget, CompilerWorkflow,
    DeviceCompilationMetadata, DeviceCompileTarget, WorkflowStepReport, compile,
};
```

---

## Functions

### `compile(circuit: &Circuit, config: CompileConfig) -> Result<CompileResult, CompilerError>`

Run the configured compilation workflow and return the compilation result. The input circuit is not modified. The returned result records the optimized circuit and the step reports in execution order; a device target additionally returns the initial and final layouts. An error is raised when the configured target, native implementation or final device validation cannot be satisfied.

Example:

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

assert_eq!(result.mode, CompileMode::Normal);
assert!(!result.steps.is_empty());
assert_eq!(result.circuit.qubits().len(), 2);
```

---

## CompileMode

Optimization strength enum.

```rust
pub enum CompileMode {
    Normal,
    Enhanced,
}
```

- `CompileMode::Normal`: conservative logical optimization, using the production default pass parameters (the default value).
- `CompileMode::Enhanced`: a stronger staged workflow with a larger pass budget, performing target-aware cleanup when a target constraint is present.

Implements `Default` (as `Normal`), `Copy`, `Eq`, `Hash`.

---

## CompileConfig

Compilation configuration, describing the logical optimization strength, the optional target constraint and the ancilla resource permissions available before layout.

```rust
pub struct CompileConfig {
    pub mode: CompileMode,
    pub target: CompileTarget,
    pub resource_policy: ResourcePolicy,
}
```

Fields:

- `mode`: the optimization workflow mode.
- `target`: a mutually exclusive logical, basis or physical device target.
- `resource_policy`: the ancilla resource permissions of the pre-layout decomposition passes. Controls whether allocating logical clean ancillas or borrowing dirty input qubits is allowed. A device target derives hard capacity from the available physical qubits and is not subject to this policy.

---

## CompileTarget

Target constraint enum.

```rust
pub enum CompileTarget {
    Logical,
    Basis(Vec<Instruction>),
    Device(DeviceCompileTarget),
    TopologyBasis {
        device_target: DeviceCompileTarget,
        basis: Vec<Instruction>,
    },
}
```

- `Logical`: compiles in the logical qubit space, with no target-related lowering.
- `Basis(Vec<Instruction>)`: lowers to an explicit standard basis.
- `Device(DeviceCompileTarget)`: routing and lowering for a concrete device.
- `TopologyBasis { device_target, basis }`: routes on the device topology while lowering to an explicit basis. The device is used only for capacity, layout and routing, and the output basis is not required to match the native capabilities of the device, so no exact device native lowering or final device validation is performed.

---

## DeviceCompileTarget

Device compilation input.

```rust
pub struct DeviceCompileTarget {
    pub device: Device,
    pub initial_layout: Option<Layout>,
    pub seed: Option<u32>,
}
```

Fields:

- `device`: an ordered native-capability device that constrains the output.
- `initial_layout`: the logical-to-physical initial layout provided by the caller.
- `seed`: the deterministic random seed of the device layout and routing heuristics.

---

## CompileResult

The object returned by `compile()`.

```rust
pub struct CompileResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub mode: CompileMode,
    pub steps: Vec<WorkflowStepReport>,
    pub device_metadata: Option<DeviceCompilationMetadata>,
}
```

Methods:

- `step(&self, name: &str) -> Option<&WorkflowStepReport>`: return the first step report whose name matches.
- `step_changed(&self, name: &str) -> bool`: whether the report with that name contains a record that was not skipped and did change the circuit.

---

## WorkflowStepReport

The execution record of a single workflow step.

```rust
pub struct WorkflowStepReport {
    pub stage: &'static str,
    pub name: &'static str,
    pub changed: bool,
    pub skipped: bool,
    pub reason: Option<String>,
}
```

Fields:

- `stage`: the coarse-grained stage name, for example `"pre_init"`, `"init"`, `"optimization"`, `"translation"`, `"routing"`, `"output"`, `"validation"`.
- `name`: the step name within the workflow.
- `changed`: whether the step changed the circuit representation.
- `skipped`: whether the step was intentionally skipped.
- `reason`: the skip or configuration description.

---

## DeviceCompilationMetadata

The physical layout information produced by device compilation.

```rust
pub struct DeviceCompilationMetadata {
    pub initial_layout: Layout,
    pub final_layout: Layout,
}
```

- `initial_layout`: the logical-to-physical layout before routing starts.
- `final_layout`: the logical-to-physical layout after all routing SWAPs.

---

## CompilerWorkflow

A reusable compilation workflow object.

- `CompilerWorkflow::new(config: CompileConfig) -> Self`
- `config(&self) -> &CompileConfig`
- `run(&self, circuit: &Circuit) -> Result<CompileResult, CompilerError>`

Example:

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::{CompileConfig, CompileMode, CompileTarget, CompilerWorkflow};
use cqlib_core::compile::resource::ResourcePolicy;

let workflow = CompilerWorkflow::new(CompileConfig {
    mode: CompileMode::Enhanced,
    target: CompileTarget::Logical,
    resource_policy: ResourcePolicy::default(),
});

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

let result = workflow.run(&circuit).unwrap();
```

---

## Errors

`CompilerError` is the unified error type of the compilation module; invalid configuration (such as `routing_trials` being zero or an empty target gate set), transform execution failures and final device validation failures are all returned through it.
