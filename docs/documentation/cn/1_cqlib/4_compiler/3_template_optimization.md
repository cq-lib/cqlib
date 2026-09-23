# 模板匹配与知识规则优化

模板优化用于在线路中寻找一段局部门序列，并把它替换为语义等价、成本更低或更符合目标门集的序列。日常使用时，您通常不需要手动维护模板列表，直接调用 `compile()` 即可使用内置规则完成常见优化。

如果需要观察或调试局部优化过程，可以使用 `KnowledgeRewriter`。它和 `compile()` 使用同一套内置知识规则。规则通常由以下几部分组成：

- `match`：要识别的源操作序列；
- `require`：可选的参数约束，例如角度相等或模 `2π`、`4π` 相等；
- `rewrite`：替换后的目标操作序列。

典型规则包括相邻逆门抵消、旋转合并、零角度归一化、门分解、目标门集改写以及显式对易规则。

`compile()` 和 `KnowledgeRewriter` 都不会修改输入线路，而是返回包含新线路的结果对象。优化前后的线路应保持量子语义等价；如果规则涉及全局相位，Cqlib 会显式保留或折叠相应的 `GPhase` 信息。

---

## 在编译管线中使用

推荐从统一的 `compile()` 入口使用模板优化能力。编译工作流会自动组合规范化、定义展开、知识规则重写、门分解、可选路由和目标门集转换。

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

对于多数使用场景，不需要直接调用规则重写器。`compile()` 会选择生产配置，并在输出前再次规范化线路表示。

---

## 直接运行知识规则重写

如果需要在测试、诊断或自定义编译流程中单独运行局部规则优化，可以直接使用 `KnowledgeRewriter`。

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

`KnowledgeRewriter.production()` 使用保守优化规则，默认启用简化、抵消、合并和规范化相关规则。重写器只会接受能带来局部收益的改写，避免在线路的等价写法之间来回切换。

也可以使用函数式入口：

```python
from cqlib.compile.transform import rewrite_circuit

result = rewrite_circuit(circuit)
optimized = result.circuit
```

---

## 规则示例

内建规则使用轻量 DSL 描述。例如 H-H 抵消可以理解为：

```text
rule cancel_h {
    match { H 0, H 0 }
    rewrite {}
}
```

旋转合并规则可以理解为：

```text
rule merge_rz {
    match { RZ(a) 0, RZ(b) 0 }
    rewrite { RZ(a + b) 0 }
}
```

带条件的规则会先检查参数关系。例如两个互逆的 `RZ` 只有在角度和满足对应模关系时才会被删去；如果相差一个全局相位，规则会显式生成 `GPhase`，再交给规范化器折叠到线路全局相位中。

---

## 优化配置

`RewriteConfig` 用于控制规则搜索范围和应用方式。Python 侧使用构造函数传入配置项。

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

几个常用参数的含义如下：

- `max_rounds`：最多执行多少轮重写；如果提前到达不动点，会提前停止；
- `max_window_ops`：一次局部搜索允许查看的最大操作窗口；
- `max_pattern_len`：可匹配规则的最大长度；
- `recurse_control_flow`：是否递归优化控制流 body；
- `skip_labeled_ops`：是否跳过带 label 的操作，避免破坏调试标记或外部约定。

如果只是想使用默认生产配置，可以直接写：

```python
config = RewriteConfig.production()
result = KnowledgeRewriter(config).run(circuit)
```

---

## 目标门集改写

模板规则不仅用于“减少门数”，也用于把线路改写到指定目标门集。推荐通过 `compile()` 的 `target_basis` 参数指定目标门集：

```python
from cqlib.circuit import Circuit
from cqlib.compile import compile

circuit = Circuit(2)
circuit.cx(0, 1)

result = compile(circuit, target_basis=["H", "CZ"])
optimized = result.circuit

print([op.instruction for op in optimized.operations])
```

如果是在自定义编译流程中直接调用知识规则重写，可以使用 lowering 配置并显式传入目标指令：

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

在完整编译工作流中，目标门集转换发生在物理路由之后，因为路由可能插入新的 `SWAP` 或暴露新的局部清理机会。

---

## 与传统模板匹配的关系

传统模板优化通常从“给定模板线路，在线路中找等价子序列”出发。Cqlib 当前实现更接近“知识规则重写”：

- 模板被结构化为经过验证的规则；
- 参数关系由 `require` 条件表达；
- 规则按类别启用，便于区分优化、分解和硬件原生门转换；
- 重写器使用局部成本模型控制是否应用规则；
- 工作流会在多个阶段重复调用，直到达到不动点或预算上限。

因此，用户可以把模板优化理解为“基于内置知识库的局部门序列改写”。当前文档中的推荐入口是 `compile()`；需要单独观察局部规则效果时，再使用 `KnowledgeRewriter`。

---

## 使用建议

- 普通编译优先使用 `compile()`，不要手动拼接多个优化步骤；
- 自定义流程中优先使用 `KnowledgeRewriter.production()`，只有做目标门集转换时才使用 lowering 模式；
- 添加新规则时必须同时考虑结构合法性、参数约束和语义等价性；
- 优化前后应比较门数、双比特门数、深度和关键线路的矩阵或状态等价性；
- 如果规则可能改变全局相位，应显式保留 `GPhase` 或确认全局相位对任务无影响。

---

## 下一步

- [对易与 Clifford-RZ 优化](4_commutative_and_clifford.md)：了解对易判定如何帮助规则重排和旋转合并。

