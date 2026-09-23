# SABRE

`cqlib_core::compile::sabre` 提供 SWAP 路由的底层入口与配置。当初始布局已知、线路必须满足设备拓扑约束时使用 `sabre_route`。固定随机种子可复现多轮试探的平局裁决与选择。

## 导入

```rust
use cqlib_core::compile::sabre::{
    SabreConfig, SabreHeuristicConfig, SabreRoutingDiagnostics, SabreRoutingResult,
    SabreVf2PrepassConfig, normalize_initial_layout, sabre_route, validate_reachable_interactions,
};
```

---

## 函数

### `sabre_route(circuit: &Circuit, device: &Device, initial_layout: &Layout, config: &SabreConfig) -> Result<SabreRoutingResult, CompilerError>`

已知初始布局条件下的 SABRE 路由。

### `normalize_initial_layout(logical_qubits: &[LogicalQubit], device: &Device, initial_layout: &Layout) -> Result<Layout, CompilerError>`

把完整逻辑到物理布局对设备可用比特做规范化。

### `validate_reachable_interactions(circuit: &Circuit, device: &Device, initial_layout: &Layout) -> Result<(), CompilerError>`

在不执行路由的前提下验证原生移动可达性。

---

## SabreConfig

SABRE 布局细化与路由共用的配置。

```rust
pub struct SabreConfig {
    pub layout_trials: usize,
    pub layout_assignment_budget: usize,
    pub vf2_prepass: Option<SabreVf2PrepassConfig>,
    pub refinement_iterations: usize,
    pub routing_trials: usize,
    pub seed: Option<u64>,
    pub heuristic: SabreHeuristicConfig,
}
```

字段：

- `layout_trials`：在确定性与交互感知候选之外增加的随机化初始布局数量。
- `layout_assignment_budget`：布局组件分配状态的最大探索量；耗尽会报告预算耗尽，区别于已证明不可行。
- `vf2_prepass`：有界 VF2 预检查，用于添加拓扑完美候选；`None` 关闭预检查。
- `refinement_iterations`：每个布局试探的正向/反向细化轮数。
- `routing_trials`：每个完整细化布局执行的完整路由试探数。搜索直接返回最佳路由，不为中间细化状态执行路由。
- `seed`：确定性随机种子。相同种子产生相同 cqlib 结果。
- `heuristic`：交换选择启发式配置。

方法：

- `SabreConfig::deterministic_seeded(seed: u64) -> Self`：紧凑的确定性配置，适合测试与示例。
- `validate(&self) -> Result<(), CompilerError>`：校验路由字段（有意忽略布局细化字段，路由从具体初始布局开始，不依赖布局专用旋钮）。

实现 `Default`：`layout_trials=10`、`layout_assignment_budget=1_000_000`、`vf2_prepass=Some(SabreVf2PrepassConfig { candidate_limit: 10, call_limit: 1_000_000 })`、`refinement_iterations=1`、`routing_trials=1`、`seed=None`、`heuristic=SabreHeuristicConfig::default()`。

---

## SabreHeuristicConfig

SABRE 交换选择启发式配置。主分数组合当前前沿层和、按活跃层缩放的前瞻与乘法拥塞；有原生方案时在窄结构窗口内用精确原生双比特代价裁决候选。

```rust
pub struct SabreHeuristicConfig {
    pub basic_weight: f64,
    pub lookahead_weights: Vec<f64>,
    pub decay_increment: Option<f64>,
    pub decay_reset: usize,
    pub attempt_limit: usize,
    pub best_epsilon: f64,
}
```

字段：

- `basic_weight`：当前前沿层总距离的权重。
- `lookahead_weights`：每个活跃层缩放前瞻层总和的权重；默认 `[0.5, 0.25, 0.125, 0.0625, 0.03125]`。
- `decay_increment`：启发式 SWAP 使用某个物理比特后其拥塞乘数的增量；`None` 关闭拥塞控制。
- `decay_reset`：衰减值重置前的启发式 SWAP 尝试次数。
- `attempt_limit`：未路由前沿节点时允许的连续启发式 SWAP 次数，超过后回退到最短路径逃逸。
- `best_epsilon`：候选 SWAP 分数视为平局的浮点容差。

实现 `Default`：`basic_weight=1.0`、`lookahead_weights=[0.5, 0.25, 0.125, 0.0625, 0.03125]`、`decay_increment=Some(0.002)`、`decay_reset=10`、`attempt_limit=1000`、`best_epsilon=1e-10`。

---

## SabreVf2PrepassConfig

用于给 SABRE 布局候选播种的有界 VF2 预检查。

```rust
pub struct SabreVf2PrepassConfig {
    pub candidate_limit: usize,
    pub call_limit: usize,
}
```

- `candidate_limit`：VF2 打分的完整完美映射数量上限。
- `call_limit`：VF2 尝试的部分映射扩展数量上限。

---

## SabreRoutingResult

`sabre_route` 的返回对象。

```rust
pub struct SabreRoutingResult {
    pub circuit: Circuit,
    pub initial_layout: Layout,
    pub final_layout: Layout,
    pub swap_count: usize,
    pub diagnostics: SabreRoutingDiagnostics,
}
```

- `circuit`：插入 SWAP 后的物理线路。
- `initial_layout`：选中试探使用的初始逻辑到物理布局。
- `final_layout`：所有路由操作之后的最终布局。
- `swap_count`：插入的 SWAP 数量（含控制流尾声）。
- `diagnostics`：路由搜索行为诊断。

---

## SabreRoutingDiagnostics

路由过程诊断信息，字段公开：

```rust
pub struct SabreRoutingDiagnostics {
    pub trials_evaluated: usize,
    pub selected_trial_index: usize,
    pub fallback_count: usize,
    pub control_flow_blocks_routed: usize,
    pub two_qubit_depth: usize,
    pub operation_count: usize,
    pub native_two_qubit_count: usize,
    pub native_two_qubit_depth: usize,
    pub native_total_depth: usize,
    pub native_operation_count: usize,
    pub unknown_loop_count: usize,
    pub requirement_signature_count: usize,
    pub eager_pair_state_count: usize,
    pub lazy_pair_l1_lookup_count: usize,
    pub lazy_pair_l1_hit_count: usize,
    pub lazy_pair_l1_cached_count: usize,
}
```

实现 `Default` 与 `PartialEq`。
