# Transform

`cqlib.compile.transform` 提供可复用的编译 pass。每个 pass 返回结果对象，不修改输入线路。端到端 `compile()` 自动编排这些 pass；需要单独观察或调试某个阶段时可以直接调用。

## 导入

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

transform 通用结果对象。

### 属性

- `circuit -> Circuit`：输出线路。
- `changed -> bool`：pass 是否改变了线路。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`。

---

## CircuitAnalysis

线路特征快照，供工作流决定执行哪些 pass。该类的完整模块路径为 `cqlib.compile.transform.analysis`。

### CircuitAnalysis.analyze(circuit)

静态方法，分析线路并返回快照。

### 属性

- `has_classical_data -> bool`：是否包含经典数据。
- `has_classical_control -> bool`：是否包含经典控制流。
- `has_measurement -> bool`：是否包含测量。
- `has_classical_values -> bool`：是否包含经典值（如测量结果）。
- `has_classical_vars -> bool`：是否包含经典变量。
- `has_runtime_classical -> bool`：是否存在运行时经典语义（测量值被后续条件使用）。
- `needs_classical_handle_preservation -> bool`：变换是否必须保持经典句柄。
- `has_circuit_gate_definitions -> bool`：是否包含电路门定义。
- `has_unitary_circuit_definitions -> bool`：是否包含酉电路定义。
- `has_unitary_gates -> bool`：是否包含自定义酉门。
- `has_mc_gates -> bool`：是否包含多控制门。

示例：

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

规范化 pass，统一指令形式、折叠全局相位、消除空操作。

### CanonicalizeConfig(*, round_limit=8, recurse_control_flow=true, fold_gphase=true, canonicalize_instruction_form=true, drop_noops=true, canonicalize_barriers=true)

参数：

- `round_limit` (`int`)：规范化最大轮数。
- `recurse_control_flow` (`bool`)：是否递归进入控制流块。
- `fold_gphase` (`bool`)：是否折叠全局相位。
- `canonicalize_instruction_form` (`bool`)：是否统一指令形式。
- `drop_noops` (`bool`)：是否删除空操作。
- `canonicalize_barriers` (`bool`)：是否规范化 barrier。

### 静态方法

- `CanonicalizeConfig.production()`：生产默认配置。

### 属性

`round_limit`、`recurse_control_flow`、`fold_gphase`、`canonicalize_instruction_form`、`drop_noops`、`canonicalize_barriers`，与构造参数一一对应。

### Canonicalizer(config=None)

参数：

- `config` (`CanonicalizeConfig | None`)：默认 `CanonicalizeConfig.production()`。

### 静态方法

- `Canonicalizer.production()`：使用生产默认配置构造。

### 属性与方法

- `config -> CanonicalizeConfig`
- `run(circuit) -> CanonicalizeResult`

### CanonicalizeResult

属性：

- `circuit -> Circuit`
- `changed -> bool`
- `rounds -> int`：实际执行的规范化轮数。

### canonicalize_circuit(circuit, config=None)

函数式入口，等价于 `Canonicalizer(config).run(circuit)`。

示例：

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

基于知识规则的模式重写 pass，是编译管线逻辑优化的核心。

### RewriteMode

重写模式枚举：

- `RewriteMode.optimize()`：保守优化，接受的替换必须严格改善局部代价。
- `RewriteMode.lowering()`：显式降级，分解与硬件原生规则可以局部扩张。
- `RewriteMode.optimize().name -> "optimize"`，`RewriteMode.lowering().name -> "lowering"`。

### RewriteConfig(*, max_rounds=8, max_window_ops=16, max_pattern_len=8, recurse_control_flow=true, skip_labeled_ops=true, enabled_kinds=None, mode=None, target_instructions=None)

参数：

- `max_rounds` (`int`)：最大重写轮数。
- `max_window_ops` (`int`)：滑动窗口最大操作数。
- `max_pattern_len` (`int`)：模式最大长度。
- `recurse_control_flow` (`bool`)：是否递归进入控制流块。
- `skip_labeled_ops` (`bool`)：是否跳过带标签的操作。
- `enabled_kinds` (`list[RuleKind] | None`)：启用的规则类别，`None` 表示全部。
- `mode` (`RewriteMode | None`)：默认 `RewriteMode.optimize()`。
- `target_instructions` (`list[str | Instruction] | None`)：可选目标指令基，用于目标感知代价评估。

### 静态方法

- `RewriteConfig.production()`：生产优化配置。
- `RewriteConfig.lowering()`：降级配置。

### 属性

`max_rounds`、`max_window_ops`、`max_pattern_len`、`recurse_control_flow`、`skip_labeled_ops`、`enabled_kinds`、`mode`、`target_instructions`，与构造参数一一对应。

### KnowledgeRewriter(config=None)

参数：

- `config` (`RewriteConfig | None`)：默认 `RewriteConfig.production()`。

### 静态方法

- `KnowledgeRewriter.production()`、`KnowledgeRewriter.lowering()`：使用对应预置配置构造。

### 属性与方法

- `config -> RewriteConfig`
- `run(circuit) -> KnowledgeRewriteResult`

### KnowledgeRewriteResult

属性：

- `circuit -> Circuit`
- `changed -> bool`
- `stats -> KnowledgeRewriteStats`

### KnowledgeRewriteStats

属性：

- `rounds_executed -> int`：实际执行轮数。
- `rules_applied -> int`：应用规则次数。
- `changed_sequences -> int`：发生变化的操作序列数。
- `reached_fixpoint -> bool`：是否达到不动点。

### rewrite_circuit(circuit, config=None)

函数式入口，等价于 `KnowledgeRewriter(config).run(circuit)`。

示例：

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

单比特连续门串优化。该类的完整模块路径为 `cqlib.compile.transform.one_qubit_optimization`。

### 静态方法

- `OptimizeOneQubitRuns.logical()`：目标中立的优化器，使用严格逻辑代价。
- `OptimizeOneQubitRuns.basis(target_basis)`：按精确降级到 `target_basis` 后的代价优化。`target_basis` 为大小写不敏感的标准门名字符串或 `Instruction` 对象列表（多控制门必须用 `Instruction`）。

### 属性与方法

- `policy -> str`：当前接受策略，取值 `"logical"` 或 `"basis"`。
- `target_basis -> list[Instruction] | None`：显式目标门集；逻辑优化时为 `None`。
- `run(circuit) -> TransformResult`

示例：

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

基于对易性的自反门抵消 pass。该类的完整模块路径为 `cqlib.compile.transform.commutative_cancellation`。

### CommutativeCancellation()

无参构造。

### 方法

- `run(circuit) -> TransformResult`

示例：

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

路由门集降级 pass，把线路转换为适合路由阶段处理的形式。

### LowerToRoutingBasis(preferred_basis=None)

参数：

- `preferred_basis` (`list[Instruction] | None`)：偏好门集；`None` 使用默认路由门集。

### 方法

- `run(circuit) -> TransformResult`

### lower_to_routing_basis(circuit, preferred_basis=None)

函数式入口。

示例：

```python
from cqlib import Circuit
from cqlib.compile.transform import lower_to_routing_basis

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

result = lower_to_routing_basis(circuit)
```
