# 初始布局（Layout）

初始布局只负责选择逻辑比特到物理比特的初始映射，不插入 SWAP。

---

## Python 入口

```python
from cqlib import Circuit
from cqlib.compile.transform.layout import (
    LayoutObjective,
    Vf2LayoutConfig,
    vf2_perfect_layout,
    greedy_layout,
    sabre_layout,
    trivial_layout,
)
from cqlib.device import Device
```

---

## 快速示例：VF2 完美嵌入

```python
circuit = Circuit(3)
circuit.cx(0, 1)
circuit.cx(1, 2)

device = Device.line("line-5", 5)
objective = LayoutObjective.topology_only()

result = vf2_perfect_layout(circuit, device, objective)
print(result.layout)                    # 逻辑 → 物理映射
print(result.diagnostics.is_perfect)    # 是否所有交互都邻接
```

若无完美嵌入，`vf2_perfect_layout` 会抛出 `ValueError`；此时改用 `greedy_layout` 或 `sabre_layout`，再交给 `route_sabre` 或 `route_with_layout`。

---

## Trivial 布局（恒等映射）

逻辑比特 `i → 物理 i`，用于基线对比或已对齐拓扑：

```python
result = trivial_layout(circuit, device, objective)
print(result.layout.l2p_map)
```

---

## Greedy 布局

```python
objective = LayoutObjective.fidelity_aware()
result = greedy_layout(circuit, device, objective)
print("is_perfect:", result.diagnostics.is_perfect)
print("score:", result.score.total if result.score else None)
```

特点：确定性、速度快，适合作为 SABRE 布局种子；长链/大扇出上可能 `is_perfect=False`。

---

## SABRE 初始布局

```python
from cqlib.compile.sabre import SabreConfig

config = SabreConfig.deterministic_seeded(42)
result = sabre_layout(circuit, device, objective, config)
```

`sabre_layout` 生成多组候选并经前向/后向试跑精修，仍不对外插入 SWAP。

---

## LayoutObjective（布局评分）

| 构造方式 | 行为 |
|----------|------|
| `LayoutObjective.topology_only()` | 仅拓扑距离与方向不匹配 |
| `LayoutObjective.fidelity_aware()` | 默认保真度权重 |
| `LayoutObjective.fidelity_required(device)` | 要求设备有可用标定 |
| `LayoutObjective.auto_from_device(device)` | 有标定则 fidelity，否则 topology |

Enhanced 模式 `compile(..., device=...)` 在设备有标定时使用 `fidelity_required` 逻辑。

---

## VF2 配置

```python
from cqlib.compile.transform.layout import Vf2EdgeRequirement

config = Vf2LayoutConfig(
    candidate_limit=10,
    edge_requirement=Vf2EdgeRequirement.positive_interactions(),
)
result = vf2_perfect_layout(circuit, device, objective, config)
```

---

## 布局结果交给路由

```python
from cqlib.compile.transform.routing import route_with_layout
from cqlib.compile.sabre import SabreConfig

layout_result = vf2_perfect_layout(circuit, device, objective)
routed = route_with_layout(
    circuit,
    device,
    layout_result.layout,
    SabreConfig.deterministic_seeded(42),
)
print("swaps:", routed.swap_count)
```

---

## 说明

- 输入为 **逻辑** `Circuit` + `Device`；输出 `LayoutResult.layout` 为 `cqlib.device.Layout`。
- 布局阶段 **不改变门语义**，**不插入 SWAP**。
- 设备标定通过 `Device` 上的 readout / two-qubit 误差字段参与 fidelity 评分。

---

## 下一步

- [SABRE 路由映射](2_sabre_mapping.md)：将布局结果交给 `route_sabre` 或 `route_with_layout` 完成物理路由。
- [编译优化](0_overview.md)：回顾 `compile()` 工作流与各编译阶段的整体关系。
