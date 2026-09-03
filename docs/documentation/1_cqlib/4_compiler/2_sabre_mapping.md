# SABRE 路由映射

SABRE 通过启发式 SWAP 将逻辑两比特门路由到设备允许的物理耦合上。

---

## 两个入口

| API | 作用 |
|-----|------|
| `route_sabre(circuit, device, objective, config)` | 自动 `sabre_layout` + 路由 |
| `route_with_layout(circuit, device, initial_layout, config)` | 仅路由，跳过布局搜索 |

```python
from cqlib import Circuit
from cqlib.compile.transform.layout import LayoutObjective
from cqlib.compile.transform.routing import route_sabre, route_with_layout
from cqlib.compile.sabre import SabreConfig
from cqlib.device import Device, Layout

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
objective = LayoutObjective.topology_only()
config = SabreConfig.deterministic_seeded(42)

result = route_sabre(circuit, device, objective, config)
print("swap_count:", result.swap_count)
print("ops:", len(result.circuit.operations))
```

---

## 与 compile 工作流集成

下面的 `device` 必须已经配置可执行的原生指令能力；`device` 单独作为目标时采用严格设备编译语义。

```python
from cqlib.compile import CompileMode, compile

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    seed=42,
)

for step in result.steps:
    if step.name in {
        "route.sabre",
        "validate.device",
        "select.sabre_pareto_beam",
    }:
        print(step.stage, step.name, step.changed, step.reason)
```

`initial_layout` 已提供时，工作流跳过自动布局，仍用相同 SABRE 路由器。

### Enhanced 严格设备的 Pareto beam

对于 `CompileMode.enhanced()` 和严格 `Device` 目标，第一次 `route.sabre` 的完整编译结果是 candidate 0。candidate 0 完成路由后清理、原生指令 lowering、native 固定点优化和 `validate.device` 后，编译器才启动有界 Pareto beam。每个探索候选从同一个 pre-routing prefix 出发，并重新执行相同后缀直到设备验证。

`select.sabre_pareto_beam` 只接受在每个控制流作用域上都不劣化精确 native 质量、且严格减少原生双比特门数量或深度的已验证候选；没有这样的候选时保留 candidate 0。该 selection 是最后的编排与诊断步骤，不会在验证后继续变换线路。

`result.steps` 记录最终保留路径，而不是所有被尝试的路线。候选数量、丢弃情况、是否接受 winner，以及 candidate 0/winner 的质量摘要可从 `select.sabre_pareto_beam.reason` 查看。`TopologyBasis` 目标即使执行 SABRE 路由也不会运行这个 beam；例如同时向 `compile()` 传入 `device` 和 `target_basis` 时采用的就是 `TopologyBasis` 语义。

---

## 示例：仅路由（跳过布局搜索）

```python
from cqlib.device import Layout

initial = Layout.from_pairs([(0, 0), (1, 2), (2, 1)], physical_count=3)
routed = route_with_layout(circuit, device, initial, config)
print("swaps:", routed.swap_count)
```

---

## 示例：compile 传入 initial_layout

```python
from cqlib.compile import compile

result = compile(
    circuit,
    device=device,
    initial_layout=initial,
    seed=42,
)
```

---

## SabreConfig

| 字段 | 含义 |
|------|------|
| `layout_trials` | 随机初始布局数量；interaction-aware、greedy、VF2 候选额外加入 |
| `layout_assignment_budget` | 断连设备上的 movement component 分配搜索上限 |
| `vf2_prepass` | 有希望精确嵌入时使用的有界 VF2 预检查 |
| `refinement_iterations` | 每个候选的前向+后向精修轮数 |
| `routing_trials` | 每个初始候选跨轻量 refinement checkpoints 分配的完整 route 总数 |
| `seed` | 确定性种子 |
| `heuristic` | `SabreHeuristicConfig`：默认单层、按设备宽度缩放且对高度重复线路提前停止的 lookahead；使用乘法 congestion 抑制物理位的连续复用；精确同分候选由 trial seed 选择 |

```python
from cqlib.compile.sabre import SabreConfig

config = SabreConfig(
    seed=42,
    layout_trials=24,
    refinement_iterations=2,
    routing_trials=2,
)
```

快捷构造：`SabreConfig.deterministic_seeded(42)`。

---

## 示例：固定 seed 的可复现记录

```python
compile_record = {
    "seed": 123,
    "layout_trials": 24,
    "refinement_iterations": 2,
    "routing_trials": 2,
}
```

路由含随机性，正式实验必须固定并记录 `seed`。

自动 layout + routing 使用融合搜索：每个候选保存初始及前向/后向精修产生的
轻量 checkpoint，并把 `routing_trials` 分配给成本较低且映射不同的 checkpoint；
完整 route 直接参与全局流式归约，不再为 layout 评分后重复路由。最终固定按预测
native 2Q 数量、native 2Q 深度、native 总深度和稳定候选索引排序。

这里描述的是单次 `route_sabre()` 及工作流 candidate 0 的路由内选择。Enhanced 严格设备编译随后运行的 Pareto beam 会把候选继续 lowering、优化并验证，再依据最终精确 native 质量决定是否替换 candidate 0。

---

## 输出语义

- `result.circuit`：物理比特编号上的线路，含插入的 `SWAP`；
- `swap_count`：应与线路中 SWAP 门数量一致；
- `layout_score`：获胜 layout 在请求 objective 下的诊断分数，不是 SABRE 最终选择依据；
- `diagnostics.native_two_qubit_count`、`native_two_qubit_depth`、`native_total_depth`：获胜 route 的结构性 native 质量估计；
- 路由保证无向物理邻接。

---

## 下一步

- [模板匹配与知识规则优化](3_template_optimization.md)：了解路由后知识规则如何进一步清理与改写线路。
- [初始布局（Layout）](1_layout.md)：复习初始布局算法与 `LayoutObjective` 的评分方式。
