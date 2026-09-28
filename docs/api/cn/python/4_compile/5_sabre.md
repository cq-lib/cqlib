# SABRE

`cqlib.compile.sabre` 提供 SWAP 路由的底层入口与配置。当初始布局已知、线路必须满足设备拓扑约束时使用 `sabre_route`。固定随机种子可复现多轮试探的平局裁决与选择。

## 导入

```python
from cqlib.compile.sabre import (
    SabreConfig,
    SabreHeuristicConfig,
    SabreVf2PrepassConfig,
    SabreRoutingResult,
    SabreRoutingDiagnostics,
    sabre_route,
    normalize_initial_layout,
    validate_reachable_interactions,
)
```

---

## sabre_route(circuit, device, initial_layout, config=None)

已知初始布局条件下的 SABRE 路由。

参数：

- `circuit` (`Circuit`)：待路由线路。
- `device` (`Device`)：目标设备。
- `initial_layout` (`Layout`)：逻辑到物理初始布局。
- `config` (`SabreConfig | None`)：默认 `SabreConfig()`。

返回：

- `SabreRoutingResult`

示例：

```python
from cqlib import Circuit
from cqlib.compile.sabre import SabreConfig, sabre_route
from cqlib.device import Device, Layout

circuit = Circuit(2)
circuit.cx(0, 1)

result = sabre_route(
    circuit,
    Device.line("line-3", 3),
    Layout.from_pairs([(0, 0), (1, 2)], physical_count=3),
    SabreConfig(routing_trials=1, seed=7),
)
assert result.swap_count == 1
```

---

## 辅助函数

### normalize_initial_layout(logical_qubits, device, initial_layout)

把完整逻辑到物理布局对设备可用比特做规范化。

参数：

- `logical_qubits` (`list[Qubit]`)：线路逻辑比特列表。
- `device` (`Device`)：目标设备。
- `initial_layout` (`Layout`)：待规范化的初始布局。

返回：

- `Layout`

### validate_reachable_interactions(circuit, device, initial_layout)

在不执行路由的前提下验证原生移动可达性。

参数：

- `circuit` (`Circuit`)：待检查线路。
- `device` (`Device`)：目标设备。
- `initial_layout` (`Layout`)：初始布局。

返回：

- `None`；不可达时抛出 `CompilerError`。

---

## SabreConfig

SABRE 布局细化与路由共用的配置。

### SabreConfig(*, layout_trials=10, layout_assignment_budget=1_000_000, vf2_prepass=SabreVf2PrepassConfig(candidate_limit=10, call_limit=1_000_000), refinement_iterations=1, routing_trials=1, seed=None, heuristic=None)

参数：

- `layout_trials` (`int`)：在确定性与交互感知候选之外增加的随机化初始布局数量。
- `layout_assignment_budget` (`int`)：布局组件分配状态的最大探索量；耗尽会报告预算耗尽，区别于已证明不可行。
- `vf2_prepass` (`SabreVf2PrepassConfig | None`)：有界 VF2 预检查，用于添加拓扑完美候选；`None` 关闭预检查。
- `refinement_iterations` (`int`)：每个布局试探的正向/反向细化轮数。
- `routing_trials` (`int`)：每个完整细化布局执行的完整路由试探数。搜索直接返回最佳路由，不为中间细化状态执行路由。
- `seed` (`int | None`)：确定性随机种子。相同种子产生相同 cqlib 结果。
- `heuristic` (`SabreHeuristicConfig | None`)：交换选择启发式配置，默认 `SabreHeuristicConfig()`。

### 静态方法

- `SabreConfig.deterministic_seeded(seed)`：紧凑的确定性配置，适合测试与示例。所有试探数较小、随机种子固定、交换尝试有界。

### 属性

`layout_trials`、`layout_assignment_budget`、`vf2_prepass`、`refinement_iterations`、`routing_trials`、`seed`、`heuristic`，与构造参数一一对应。

### 异常情况

- `CompilerConfigError`：`routing_trials` 为零等路由字段非法（布局细化字段不影响路由校验）。

---

## SabreHeuristicConfig

SABRE 交换选择启发式配置。主分数组合当前前沿层和、按活跃层缩放的前瞻与乘法拥塞；有原生方案时在窄结构窗口内用精确原生双比特代价裁决候选。

### SabreHeuristicConfig(*, basic_weight=1.0, lookahead_weights=None, decay_increment=0.002, decay_reset=10, attempt_limit=1000, best_epsilon=1e-10)

参数：

- `basic_weight` (`float`)：当前前沿层总距离的权重。
- `lookahead_weights` (`list[float] | None`)：每个活跃层缩放前瞻层总和的权重；默认 `[0.5, 0.25, 0.125, 0.0625, 0.03125]`。
- `decay_increment` (`float | None`)：启发式 SWAP 使用某个物理比特后，其拥塞乘数的增量；`None` 关闭拥塞控制。
- `decay_reset` (`int`)：衰减值重置前的启发式 SWAP 尝试次数。
- `attempt_limit` (`int`)：未路由前沿节点时允许的连续启发式 SWAP 次数，超过后回退到最短路径逃逸。
- `best_epsilon` (`float`)：候选 SWAP 分数视为平局的浮点容差。

### 属性

`basic_weight`、`lookahead_weights`、`decay_increment`、`decay_reset`、`attempt_limit`、`best_epsilon`，与构造参数一一对应。

---

## SabreVf2PrepassConfig

用于给 SABRE 布局候选播种的有界 VF2 预检查。

### SabreVf2PrepassConfig(*, candidate_limit=10, call_limit=1_000_000)

参数：

- `candidate_limit` (`int`)：VF2 打分的完整完美映射数量上限。
- `call_limit` (`int`)：VF2 尝试的部分映射扩展数量上限。

### 属性

`candidate_limit`、`call_limit`，与构造参数一一对应。

---

## SabreRoutingResult

`sabre_route` 的返回对象。

### 属性

- `circuit -> Circuit`：路由后的线路。
- `initial_layout -> Layout`：路由开始前的布局。
- `final_layout -> Layout`：路由结束后的布局。
- `swap_count -> int`：插入的 SWAP 数量。
- `diagnostics -> SabreRoutingDiagnostics`：路由诊断信息。

---

## SabreRoutingDiagnostics

路由过程诊断信息。

### 属性

- `trials_evaluated -> int`：评估的路由试探数。
- `selected_trial_index -> int`：选中试探的索引。
- `fallback_count -> int`：回退次数。
- `control_flow_blocks_routed -> int`：已路由的控制流块数量。
- `two_qubit_depth -> int`：路由后线路的双比特深度。
- `operation_count -> int`：操作总数。
- `native_two_qubit_count -> int`：原生双比特门数量。
- `native_two_qubit_depth -> int`：原生双比特深度。
- `native_total_depth -> int`：原生总深度。
- `native_operation_count -> int`：原生操作总数。
- `unknown_loop_count -> int`：未知循环数量。
- `requirement_signature_count -> int`：需求签名数量。
- `eager_pair_state_count -> int`：急切对状态数量。
- `lazy_pair_l1_lookup_count -> int`：惰性对一级查找次数。
- `lazy_pair_l1_hit_count -> int`：惰性对一级命中次数。
- `lazy_pair_l1_cached_count -> int`：惰性对一级缓存数量。
