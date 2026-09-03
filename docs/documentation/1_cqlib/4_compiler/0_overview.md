# 编译优化

Cqlib 2.0 Python 绑定以 **`cqlib.compile`** 为推荐入口：一次调用完成规范化、知识规则优化、分解、可选设备布局与 SABRE 路由、目标门集翻译，以及严格设备目标所需的原生指令 lowering、固定点优化和验证。

---

## 常用入口

```python
from cqlib import Circuit
from cqlib.compile import CompileConfig, CompileMode, compile
from cqlib.compile.transform.layout import vf2_perfect_layout, sabre_layout
from cqlib.compile.transform.routing import route_sabre
from cqlib.compile.transform import KnowledgeRewriter, RewriteConfig
```

---

## 推荐编译管线

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
    if step.changed or not step.skipped:
        print(step.stage, step.name, step.reason)

compiled = result.circuit
```

上例同时传入 `device` 和 `target_basis`，因此选择的是 `TopologyBasis` 语义：设备用于容量、布局和路由，显式门集约束输出，但不承诺输出满足设备的原生指令能力。只有配置了原生能力的严格 `Device` 目标，才会在 Enhanced 模式下运行后文所述的 SABRE Pareto beam。

---

## 分步调试管线

需要单独观察布局、路由或规则优化时，可拆开调用：

```python
from cqlib.compile.transform.layout import LayoutObjective, vf2_perfect_layout
from cqlib.compile.transform.routing import route_sabre
from cqlib.compile.sabre import SabreConfig

objective = LayoutObjective.topology_only()
config = SabreConfig.deterministic_seeded(42)

# 1) 仅布局（不插 SWAP）
layout_result = vf2_perfect_layout(circuit, device, objective)

# 2) 布局 + 路由（插 SWAP）
route_result = route_sabre(circuit, device, objective, config)
print("swap_count:", route_result.swap_count)
```

---

## 编译目标

1. 将逻辑线路适配目标设备拓扑（布局 + 路由，必要时插入 SWAP）；
2. 减少双比特门数量、线路深度与冗余单比特门；
3. 将门序列 lowering 为目标原生门集。

---

## 流水线概览

```text
逻辑线路 (Circuit)
  → canonicalize.input
  → decompose.definitions
  → optimize.pre_decomposition
  → decompose.unitary / decompose.mc_gates
  → canonicalize.after_decomposition
  → optimize.post_decomposition
  → decompose.routing_basis                         [物理目标]
  → route.sabre                                     [物理目标]
  → post-routing resynthesis / cleanup              [Enhanced + 物理目标]
  → translate.target_basis / target cleanup         [显式目标门集]
  → canonicalize.output
  → lower.device_instructions                       [严格 Device 目标]
  → native-input canonicalization / fixed point     [严格 Device 目标]
  → validate.device / validate.topology             [按目标类型]
  → select.sabre_pareto_beam                        [Enhanced + 严格 Device 目标]
  → result.circuit
```

`canonicalize.output` 只结束通用输出表示的整理。对于严格 `Device` 目标，后面仍会执行精确原生指令 lowering、`optimize.native_fixed_point` 和 `validate.device`。

### Enhanced 严格设备的 Pareto 选择

Enhanced 严格设备编译会在第一次路由前保存不可变的 pre-routing prefix，并把普通完整编译结果作为 candidate 0：

```text
保存 pre-routing prefix
  → candidate 0：route + 完整后缀 + validate.device
  → bounded SABRE Pareto beam
      → 每个探索候选从同一 prefix 开始
      → route + 同一完整后缀 + validate.device
  → select.sabre_pareto_beam
      ├─ 存在满足契约的改进候选：选择已验证的 winner
      └─ 否则：保留已验证的 candidate 0
  → result.circuit
```

候选必须在每个控制流作用域上满足精确 Pareto 契约，并且至少严格改善原生双比特门数量或深度，才能替换 candidate 0。因此，`selection` 出现在 `validation` 之后不表示输出绕过了验证：selection 不再变换线路，只在已完成相同 validation 后缀的候选之间决策。

成功返回时，`result.steps` 记录公共前缀、最终保留的 baseline 或 winner 路径，以及末尾的 `select.sabre_pareto_beam`；它不是所有探索分支的完整执行日志。候选尝试数、丢弃数、首次可恢复错误，以及 candidate 0/winner 的质量摘要记录在 selection 的 `reason` 中。

---

## CompileMode

| 模式 | 说明 |
|------|------|
| `CompileMode.normal()` | 生产默认可预测：保守 rewrite 预算与 SABRE 试次 |
| `CompileMode.enhanced()` | 更高的 rewrite、重综合、native 固定点优化和 SABRE 搜索预算，并增加路由后/目标门集清理；严格 `Device` 目标还会运行已验证候选间的有界 SABRE Pareto beam |

---

## CompileConfig 要点

| 字段 | 作用 |
|------|------|
| `mode` | `normal` / `enhanced` |
| `device` | 可用比特、拓扑、native gates 与可选标定；单独使用时选择严格 `Device` 目标 |
| `target_basis` | 显式目标门集；与 `device` 同时使用时选择 `TopologyBasis`，不承诺设备原生兼容，也不运行 Pareto beam |
| `initial_layout` | 跳过自动布局，直接用给定映射做 SABRE 路由 |
| `resource_policy` | 分解阶段辅助比特策略 |
| `seed` | 启发式布局/路由随机试次 |

---

## CompileConfig 与 CompilerWorkflow

需要复用同一配置编译多条线路时，使用 `CompilerWorkflow`：

```python
from cqlib import Circuit
from cqlib.compile import CompileConfig, CompileMode, CompilerWorkflow
from cqlib.device import Device

circuits = []
c1 = Circuit(2)
c1.cx(0, 1)
circuits.append(c1)

c2 = Circuit(3)
c2.h(0)
c2.cx(0, 2)
circuits.append(c2)

config = CompileConfig(
    mode=CompileMode.enhanced(),
    device=Device.line("line-8", 8),
    target_basis=["H", "CX", "RZ"],
    seed=42,
)
workflow = CompilerWorkflow(config)

for circuit in circuits:
    result = workflow.run(circuit)
    print(result.changed, len(result.circuit.operations))
```

---

## 分解与资源策略

工作流中的 `decompose.definitions` / `decompose.unitary` / `decompose.mc_gates` 也可单独调用（调试多控门分解时有用）：

```python
from cqlib.compile.resource import ResourcePolicy
from cqlib.compile.transform.decompose import (
    decompose_mc_gates_for_device,
    decompose_unitaries,
)

# 多控门分解（受设备容量约束）
result = decompose_mc_gates_for_device(
    circuit,
    device,
    resource_policy=ResourcePolicy(max_pre_layout_clean_ancillas=2),
)

# 矩阵酉门分解
unitary_result = decompose_unitaries(circuit)
```

`ResourcePolicy` 控制编译器可创建的 **clean ancilla** 数量；设备 **硬容量** 由 `device.num_usable_qubits` 决定，二者独立。

---

## 下一步

- [初始布局（Layout）](1_layout.md)：学习 VF2、greedy、sabre_layout 等初始映射算法。
- [SABRE 路由映射](2_sabre_mapping.md)：了解如何用启发式 SWAP 将线路路由到设备拓扑。
- [模板匹配与知识规则优化](3_template_optimization.md)：掌握 `compile()` 与 `KnowledgeRewriter` 的局部优化能力。
- [对易与 Clifford-RZ 优化](4_commutative_and_clifford.md)：理解对易判定如何支撑旋转合并与规则重排。
