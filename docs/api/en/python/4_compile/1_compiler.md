# Compiler

The end-to-end compilation entry points of `cqlib.compile`, including the `compile()` function, the compilation configuration object, the compilation result object and the reusable workflow object.

## Import

```python
from cqlib.compile import (
    CompileMode,
    CompileTarget,
    DeviceCompileTarget,
    CompileConfig,
    CompileResult,
    WorkflowStepReport,
    DeviceCompilationMetadata,
    CompilerWorkflow,
    compile,
)
```

---

## Functions

### compile(circuit, *, mode=None, target=None, target_basis=None, device=None, initial_layout=None, resource_policy=None, seed=None)

Run the configured compilation workflow and return the compilation result. The input circuit is not modified.

The top-level package re-exports the same function under the name `compile_circuit`, to avoid clashing with the built-in name and the submodule name; `from cqlib import compile_circuit` and `from cqlib.compile import compile` yield the same function, and the two behave identically.

Parameters:

- `circuit` (`Circuit`): the circuit to compile.
- `mode` (`CompileMode | str | None`): the optimization strength. Accepts `CompileMode.normal()` / `CompileMode.enhanced()`, or the strings `"normal"` / `"enhanced"` (case-insensitive). Defaults to `CompileMode.normal()`.
- `target` (`CompileTarget | None`): an explicit target constraint. Mutually exclusive with `target_basis` and `device`; providing both raises `CompilerConfigError`.
- `target_basis` (`list[str | Instruction] | None`): a convenience form of the target standard gate basis, equivalent to `CompileTarget.basis(...)`. Entries are case-insensitive standard gate name strings or `Instruction` objects; multi-controlled gates have no string form and must be passed as an `Instruction`.
- `device` (`Device | None`): a convenience form of the target device. Provided alone it is equivalent to `CompileTarget.device(device, ...)`; provided together with `target_basis` it is equivalent to `CompileTarget.topology_basis(device, target_basis, ...)`.
- `initial_layout` (`Layout | None`): the logical-to-physical initial layout; takes effect only when `device` is provided.
- `resource_policy` (`ResourcePolicy | None`): the ancilla policy of the pre-layout decomposition stage, see [Resource](8_resource.md).
- `seed` (`int | None`): the deterministic random seed for device layout and routing; takes effect only when `device` is provided.

Returns:

- `CompileResult`

Raises:

- `CompilerConfigError`: the configuration is invalid (mutually exclusive `target` and convenience parameters, an unknown mode name, an empty target gate set, and so on).
- `CompilerTransformError`: a transform fails to execute.
- `CompilerInternalError`: an internal consistency check fails.

Example:

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
        print(step.stage, step.name, step.reason)
```

---

## CompileMode

Optimization strength enumeration.

### Static methods

- `CompileMode.normal()`: conservative logical optimization using production default pass parameters.
- `CompileMode.enhanced()`: a stronger staged workflow using a larger pass budget, performing target-aware cleanup when a target constraint is present.

### Other behavior

- `str(CompileMode.normal()) == "normal"`, `str(CompileMode.enhanced()) == "enhanced"`.
- Supports `==`, `hash`, `copy` and `deepcopy`.

---

## CompileConfig

Compilation configuration snapshot, immutable.

### CompileConfig(*, mode=None, target=None, resource_policy=None)

Parameters:

- `mode` (`CompileMode | str | None`): defaults to `CompileMode.normal()`.
- `target` (`CompileTarget | None`): defaults to `CompileTarget.logical()`.
- `resource_policy` (`ResourcePolicy | None`): defaults to `ResourcePolicy()`.

### Attributes

- `mode -> CompileMode`
- `target -> CompileTarget`
- `resource_policy -> ResourcePolicy`

Example:

```python
from cqlib.compile import CompileConfig, CompileMode, CompileTarget

config = CompileConfig(mode=CompileMode.enhanced(), target=CompileTarget.logical())
assert config.mode == CompileMode.enhanced()
assert config.target.kind == "logical"
```

---

## CompileTarget

Target constraint enumeration. Instances are constructed through static methods.

### Static methods

- `CompileTarget.logical()`: compile in the logical qubit space without target-related lowering.
- `CompileTarget.basis(instructions)`: lower to an explicit standard gate basis.
  - `instructions` (`list[str | Instruction]`): a non-empty list of gate basis entries; the entry requirements are the same as those of `target_basis` in `compile()`.
- `CompileTarget.device(device, *, initial_layout=None, seed=None)`: routing and lowering for a concrete device; the output is required to match the device's native capabilities and final device validation is performed. `device` must already be configured with a native gate set (`device.native_gates` non-empty), otherwise construction fails; when only the topology and layout matter and the output is not required to land on the device native gate basis, use `topology_basis` instead.
- `CompileTarget.topology_basis(device, instructions, *, initial_layout=None, seed=None)`: route on the device topology while lowering to an explicit gate basis; the device is used only for capacity, layout and routing, the gate basis is not required to match the device's native capabilities, and therefore the device is not required to have a native gate set configured.

### Attributes

- `kind -> str`: the target category, taking the values `"logical"`, `"basis"`, `"device"`, `"topology_basis"`.

Raises:

- `CompilerConfigError`: the gate basis is empty, a gate basis entry is not a known standard gate, or the device used by a `device` target has no native gate set.

Example:

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.compile import CompileTarget
from cqlib.device import Device

logical = CompileTarget.logical()
basis = CompileTarget.basis(["H", "CX", "RZ"])

device = Device.line("line-3", 3)
device.native_gates = [
    Instruction.from_standard_gate(StandardGate.H),
    Instruction.from_standard_gate(StandardGate.CX),
]
device_target = CompileTarget.device(device, seed=42)

topo_basis = CompileTarget.topology_basis(Device.line("line-3", 3), ["H", "CX", "RZ"])
```

---

## DeviceCompileTarget

Device compilation input snapshot.

### DeviceCompileTarget(device, *, initial_layout=None, seed=None)

Parameters:

- `device` (`Device`): the device with ordered native capabilities that constrains the output.
- `initial_layout` (`Layout | None`): the logical-to-physical initial layout provided by the caller.
- `seed` (`int | None`): the deterministic random seed for device layout and routing.

### Attributes

- `device -> Device`
- `initial_layout -> Layout | None`
- `seed -> int | None`

---

## CompileResult

The object returned by `compile()` and `CompilerWorkflow.run()`.

### Attributes

- `circuit -> Circuit`: the optimized circuit.
- `changed -> bool`: whether any step changed the input representation.
- `mode -> CompileMode`: the mode used by this run.
- `steps -> list[WorkflowStepReport]`: the step-by-step reports in execution order.
- `device_metadata -> DeviceCompilationMetadata | None`: the physical layout data when routing on a device topology; `None` for a non-device target.

### Methods

- `step(name) -> WorkflowStepReport | None`: return the first step report whose name matches.
- `step_changed(name) -> bool`: whether the reports under that name contain a record that was not skipped and did change the circuit.

### Other behavior

- Supports `==`, `copy` and `deepcopy`.

Example:

```python
from cqlib import Circuit
from cqlib.compile import compile

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

result = compile(circuit)
assert result.circuit is not circuit
assert result.step_changed("canonicalize.input") or not result.changed
```

---

## WorkflowStepReport

The execution record of a single workflow step.

### Attributes

- `stage -> str`: the coarse-grained stage name, for example `"pre_init"`, `"init"`, `"optimization"`, `"translation"`, `"routing"`, `"output"`, `"validation"`.
- `name -> str`: the step name within the workflow.
- `changed -> bool`: whether the step changed the circuit representation.
- `skipped -> bool`: whether the step was deliberately skipped.
- `reason -> str | None`: the skip or configuration explanation.

### Other behavior

- Supports `copy` and `deepcopy`.

---

## DeviceCompilationMetadata

The physical layout information produced by device compilation.

### Attributes

- `initial_layout -> Layout`: the logical-to-physical layout before routing starts.
- `final_layout -> Layout`: the logical-to-physical layout after all routing SWAPs.

### Other behavior

- Supports `==`, `copy` and `deepcopy`.

---

## CompilerWorkflow

Reusable compilation workflow object.

### CompilerWorkflow(config=None)

Parameters:

- `config` (`CompileConfig | None`): the configuration snapshot; defaults to `CompileConfig()`.

### Attributes

- `config -> CompileConfig`

### Methods

- `run(circuit) -> CompileResult`: run the workflow according to the configuration without modifying the input circuit.

Example:

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, CompilerWorkflow, CompileConfig

workflow = CompilerWorkflow(CompileConfig(mode=CompileMode.enhanced()))

for _ in range(3):
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    result = workflow.run(circuit)
```

---

## Exceptions

- `CompilerError`: the compilation error base class.
- `CompilerConfigError`: configuration error.
- `CompilerTransformError`: transform execution error.
- `CompilerInternalError`: internal error.

All of the exceptions above are exported from `cqlib.compile`.
