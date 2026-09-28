# Compiler

`cqlib.compile` 的端到端编译入口，包括 `compile()` 函数、编译配置对象、编译结果对象与可复用工作流对象。

## 导入

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

## 函数

### compile(circuit, *, mode=None, target=None, target_basis=None, device=None, initial_layout=None, resource_policy=None, seed=None)

运行配置好的编译工作流，返回编译结果。输入线路不会被修改。

顶层包以 `compile_circuit` 之名重导出同一函数，避免与内建名及子模块名冲突；`from cqlib import compile_circuit` 与 `from cqlib.compile import compile` 拿到的是同一个函数，两者行为完全一致。

参数：

- `circuit` (`Circuit`)：待编译线路。
- `mode` (`CompileMode | str | None`)：优化力度。接受 `CompileMode.normal()` / `CompileMode.enhanced()`，或字符串 `"normal"` / `"enhanced"`（大小写不敏感）。默认 `CompileMode.normal()`。
- `target` (`CompileTarget | None`)：显式目标约束。与 `target_basis`、`device` 互斥，同时提供会抛出 `CompilerConfigError`。
- `target_basis` (`list[str | Instruction] | None`)：目标标准门集的便捷写法，等价于 `CompileTarget.basis(...)`。条目为大小写不敏感的标准门名字符串或 `Instruction` 对象；多控制门没有字符串形式，必须以 `Instruction` 传入。
- `device` (`Device | None`)：目标设备的便捷写法。单独提供时等价于 `CompileTarget.device(device, ...)`；与 `target_basis` 同时提供时等价于 `CompileTarget.topology_basis(device, target_basis, ...)`。
- `initial_layout` (`Layout | None`)：逻辑到物理的初始布局，仅在提供 `device` 时生效。
- `resource_policy` (`ResourcePolicy | None`)：预布局分解阶段的辅助比特策略，见 [Resource](8_resource.md)。
- `seed` (`int | None`)：设备布局与路由的确定性随机种子，仅在提供 `device` 时生效。

返回：

- `CompileResult`

异常情况：

- `CompilerConfigError`：配置非法（`target` 与便捷参数互斥、未知 mode 名称、空目标门集等）。
- `CompilerTransformError`：变换执行失败。
- `CompilerInternalError`：内部一致性检查失败。

示例：

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

优化力度枚举。

### 静态方法

- `CompileMode.normal()`：保守的逻辑优化，使用生产默认 pass 参数。
- `CompileMode.enhanced()`：更强的分阶段工作流，使用更大的 pass 预算，并在有目标约束时执行目标感知清理。

### 其他行为

- `str(CompileMode.normal()) == "normal"`，`str(CompileMode.enhanced()) == "enhanced"`。
- 支持 `==`、`hash`、`copy`、`deepcopy`。

---

## CompileConfig

编译配置快照，不可变。

### CompileConfig(*, mode=None, target=None, resource_policy=None)

参数：

- `mode` (`CompileMode | str | None`)：默认 `CompileMode.normal()`。
- `target` (`CompileTarget | None`)：默认 `CompileTarget.logical()`。
- `resource_policy` (`ResourcePolicy | None`)：默认 `ResourcePolicy()`。

### 属性

- `mode -> CompileMode`
- `target -> CompileTarget`
- `resource_policy -> ResourcePolicy`

示例：

```python
from cqlib.compile import CompileConfig, CompileMode, CompileTarget

config = CompileConfig(mode=CompileMode.enhanced(), target=CompileTarget.logical())
assert config.mode == CompileMode.enhanced()
assert config.target.kind == "logical"
```

---

## CompileTarget

目标约束枚举。实例通过静态方法构造。

### 静态方法

- `CompileTarget.logical()`：在逻辑比特空间编译，不做目标相关降级。
- `CompileTarget.basis(instructions)`：降级到显式标准门集。
  - `instructions` (`list[str | Instruction]`)：非空门集条目列表，条目要求同 `compile()` 的 `target_basis`。
- `CompileTarget.device(device, *, initial_layout=None, seed=None)`：针对具体设备的路由与降级，要求产物匹配设备原生能力并做最终设备校验。`device` 必须已经配置原生门集（`device.native_gates` 非空），否则构造失败；只关心拓扑与布局、不要求产物落到设备原生门集时，改用 `topology_basis`。
- `CompileTarget.topology_basis(device, instructions, *, initial_layout=None, seed=None)`：在设备拓扑上路由，同时降级到显式门集；设备只用于容量、布局与路由，不要求门集匹配设备原生能力，因此不要求设备配置原生门集。

### 属性

- `kind -> str`：目标类别，取值 `"logical"`、`"basis"`、`"device"`、`"topology_basis"`。

异常情况：

- `CompilerConfigError`：门集为空、门集条目不是已知的标准门，或 `device` 目标所用的设备没有原生门集。

示例：

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

设备编译输入快照。

### DeviceCompileTarget(device, *, initial_layout=None, seed=None)

参数：

- `device` (`Device`)：约束产物的有序原生能力设备。
- `initial_layout` (`Layout | None`)：调用方提供的逻辑到物理初始布局。
- `seed` (`int | None`)：设备布局与路由的确定性随机种子。

### 属性

- `device -> Device`
- `initial_layout -> Layout | None`
- `seed -> int | None`

---

## CompileResult

`compile()` 与 `CompilerWorkflow.run()` 的返回对象。

### 属性

- `circuit -> Circuit`：优化后的线路。
- `changed -> bool`：是否有任何步骤改变了输入表示。
- `mode -> CompileMode`：本次运行使用的模式。
- `steps -> list[WorkflowStepReport]`：按执行顺序排列的分步报告。
- `device_metadata -> DeviceCompilationMetadata | None`：使用设备拓扑路由时的物理布局数据；非设备目标时为 `None`。

### 方法

- `step(name) -> WorkflowStepReport | None`：返回第一个名字匹配的分步报告。
- `step_changed(name) -> bool`：该名字的报告中是否有未被跳过且确实变更了线路的记录。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`。

示例：

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

单个工作流步骤的执行记录。

### 属性

- `stage -> str`：粗粒度阶段名，例如 `"pre_init"`、`"init"`、`"optimization"`、`"translation"`、`"routing"`、`"output"`、`"validation"`。
- `name -> str`：工作流内步骤名。
- `changed -> bool`：该步骤是否改变了线路表示。
- `skipped -> bool`：该步骤是否被有意跳过。
- `reason -> str | None`：跳过或配置说明。

### 其他行为

- 支持 `copy`、`deepcopy`。

---

## DeviceCompilationMetadata

设备编译产生的物理布局信息。

### 属性

- `initial_layout -> Layout`：路由开始前的逻辑到物理布局。
- `final_layout -> Layout`：所有路由 SWAP 之后的逻辑到物理布局。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`。

---

## CompilerWorkflow

可复用的编译工作流对象。

### CompilerWorkflow(config=None)

参数：

- `config` (`CompileConfig | None`)：配置快照，默认使用 `CompileConfig()`。

### 属性

- `config -> CompileConfig`

### 方法

- `run(circuit) -> CompileResult`：按配置执行工作流，不修改输入线路。

示例：

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

## 异常

- `CompilerError`：编译错误基类。
- `CompilerConfigError`：配置错误。
- `CompilerTransformError`：变换执行错误。
- `CompilerInternalError`：内部错误。

以上异常均从 `cqlib.compile` 导出。
