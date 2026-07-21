# 编译优化

Cqlib 2.0 Python 绑定以 **`cqlib.compile`** 为推荐入口：一次调用完成规范化、知识规则优化、分解、可选设备布局与 SABRE 路由、目标门集翻译。

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
  → decompose.unitary / mc_gates
  → optimize.post_decomposition
  → route.sabre                    
  → optimize.post_routing        
  → translate.target_basis
  → optimize.target_cleanup
  → canonicalize.output
```

---

## CompileMode

| 模式 | 说明 |
|------|------|
| `CompileMode.normal()` | 生产默认可预测：保守 rewrite 预算与 SABRE 试次 |
| `CompileMode.enhanced()` | 更强 rewrite、更多 SABRE trials、路由后/目标基清理 |

---

## CompileConfig 要点

| 字段 | 作用 |
|------|------|
| `mode` | `normal` / `enhanced` |
| `device` | 可用比特、拓扑、可选 native gates 与标定 |
| `target_basis` | 显式目标门集（优先于 device.native_gates） |
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
