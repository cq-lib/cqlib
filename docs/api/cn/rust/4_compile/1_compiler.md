# Compiler

`cqlib_core::compile` 的端到端编译入口，包括 `compile()` 函数、编译配置对象、编译结果对象与可复用工作流对象。

## 导入

```rust
use cqlib_core::compile::{
    CompileConfig, CompileMode, CompileResult, CompileTarget, CompilerWorkflow,
    DeviceCompilationMetadata, DeviceCompileTarget, WorkflowStepReport, compile,
};
```

---

## 函数

### `compile(circuit: &Circuit, config: CompileConfig) -> Result<CompileResult, CompilerError>`

运行配置好的编译工作流，返回编译结果。输入线路不会被修改。返回结果记录优化后的线路与按执行顺序排列的分步报告；设备目标额外返回初始与最终布局。配置的目标、原生实现或最终设备校验无法满足时报错。

示例：

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

优化力度枚举。

```rust
pub enum CompileMode {
    Normal,
    Enhanced,
}
```

- `CompileMode::Normal`：保守的逻辑优化，使用生产默认 pass 参数（默认值）。
- `CompileMode::Enhanced`：更强的分阶段工作流，使用更大的 pass 预算，并在有目标约束时执行目标感知清理。

实现 `Default`（为 `Normal`）、`Copy`、`Eq`、`Hash`。

---

## CompileConfig

编译配置，描述逻辑优化力度、可选目标约束与布局前可用的辅助比特资源权限。

```rust
pub struct CompileConfig {
    pub mode: CompileMode,
    pub target: CompileTarget,
    pub resource_policy: ResourcePolicy,
}
```

字段：

- `mode`：优化工作流模式。
- `target`：互斥的逻辑、门集或物理设备目标。
- `resource_policy`：预布局分解 pass 的辅助比特资源权限。控制是否允许分配逻辑干净 ancilla 或借用脏输入比特。设备目标会从可用物理比特推导硬容量，不受此策略限制。

---

## CompileTarget

目标约束枚举。

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

- `Logical`：在逻辑比特空间编译，不做目标相关降级。
- `Basis(Vec<Instruction>)`：降级到显式标准门集。
- `Device(DeviceCompileTarget)`：针对具体设备的路由与降级。
- `TopologyBasis { device_target, basis }`：在设备拓扑上路由，同时降级到显式门集。设备只用于容量、布局与路由，不要求产物门集匹配设备原生能力，因此不执行精确设备原生降级或最终设备校验。

---

## DeviceCompileTarget

设备编译输入。

```rust
pub struct DeviceCompileTarget {
    pub device: Device,
    pub initial_layout: Option<Layout>,
    pub seed: Option<u32>,
}
```

字段：

- `device`：约束产物的有序原生能力设备。
- `initial_layout`：调用方提供的逻辑到物理初始布局。
- `seed`：设备布局与路由启发式的确定性随机种子。

---

## CompileResult

`compile()` 的返回对象。

```rust
pub struct CompileResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub mode: CompileMode,
    pub steps: Vec<WorkflowStepReport>,
    pub device_metadata: Option<DeviceCompilationMetadata>,
}
```

方法：

- `step(&self, name: &str) -> Option<&WorkflowStepReport>`：返回第一个名字匹配的分步报告。
- `step_changed(&self, name: &str) -> bool`：该名字的报告中是否有未被跳过且确实变更了线路的记录。

---

## WorkflowStepReport

单个工作流步骤的执行记录。

```rust
pub struct WorkflowStepReport {
    pub stage: &'static str,
    pub name: &'static str,
    pub changed: bool,
    pub skipped: bool,
    pub reason: Option<String>,
}
```

字段：

- `stage`：粗粒度阶段名，例如 `"pre_init"`、`"init"`、`"optimization"`、`"translation"`、`"routing"`、`"output"`、`"validation"`。
- `name`：工作流内步骤名。
- `changed`：该步骤是否改变了线路表示。
- `skipped`：该步骤是否被有意跳过。
- `reason`：跳过或配置说明。

---

## DeviceCompilationMetadata

设备编译产生的物理布局信息。

```rust
pub struct DeviceCompilationMetadata {
    pub initial_layout: Layout,
    pub final_layout: Layout,
}
```

- `initial_layout`：路由开始前的逻辑到物理布局。
- `final_layout`：所有路由 SWAP 之后的逻辑到物理布局。

---

## CompilerWorkflow

可复用的编译工作流对象。

- `CompilerWorkflow::new(config: CompileConfig) -> Self`
- `config(&self) -> &CompileConfig`
- `run(&self, circuit: &Circuit) -> Result<CompileResult, CompilerError>`

示例：

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

## 错误

`CompilerError` 是编译模块的统一错误类型，配置非法（如 `routing_trials` 为零、空目标门集）、变换执行失败与最终设备校验失败均通过它返回。
