# Routing

`cqlib_core::compile::transform::routing` 提供变换层路由入口。路由在受限拓扑上插入 SWAP，使线路中所有二体交互满足设备邻接约束。

## 导入

```rust
use cqlib_core::compile::transform::routing::{RoutedCircuit, SabreRouteResult, route_sabre, route_with_layout};
use cqlib_core::compile::transform::layout::LayoutObjective;
use cqlib_core::compile::sabre::SabreConfig;
```

---

## 函数

### `route_with_layout(circuit: &Circuit, device: &Device, initial_layout: &Layout, config: &SabreConfig) -> Result<RoutedCircuit, CompilerError>`

已知初始布局条件下的确定性路由。

### `route_sabre(circuit: &Circuit, device: &Device, objective: &LayoutObjective, config: &SabreConfig) -> Result<SabreRouteResult, CompilerError>`

带布局选择的 SABRE 路由：先按 `objective` 选出初始布局，再执行路由。

---

## RoutedCircuit

已知初始布局条件下的路由结果，字段私有：

- `circuit(&self) -> &Circuit`：路由后的线路。
- `into_circuit(self) -> Circuit`：消费并取出线路。
- `initial_layout(&self) -> &Layout`：路由开始前的布局。
- `final_layout(&self) -> &Layout`：路由结束后的布局。
- `swap_count(&self) -> usize`：插入的 SWAP 数量。
- `diagnostics(&self) -> &SabreRoutingDiagnostics`：路由诊断信息，见 [SABRE](5_sabre.md)。

---

## SabreRouteResult

带布局选择的路由结果，字段私有：

- `routed(&self) -> &RoutedCircuit`：路由结果。
- `into_routed(self) -> RoutedCircuit`：消费并取出路由结果。
- `layout_score(&self) -> Option<&LayoutScore>`：选中布局的分数。SABRE 按预测原生路由质量选择胜者，该分数只作诊断，不是路由选择键。
- `layout_diagnostics(&self) -> &LayoutDiagnostics`：布局过程诊断信息。

---

## 示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::sabre::SabreConfig;
use cqlib_core::compile::transform::layout::LayoutObjective;
use cqlib_core::compile::transform::routing::route_sabre;
use cqlib_core::device::Device;

let mut circuit = Circuit::new(3);
circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

let device = Device::line("line-3", 3).unwrap();
let result = route_sabre(
    &circuit,
    &device,
    &LayoutObjective::topology_only(),
    &SabreConfig::default(),
)
.unwrap();

println!("swap_count: {}", result.routed().swap_count());
```

---

## 错误

- `CompilerError::InvalidInput`：SABRE 配置非法，例如 `routing_trials` 为零。
- 其他 `CompilerError` 变体：路由失败，例如初始布局缺失或不可达、设备容量不足。

低层 `sabre_route` 与 `SabreConfig` 的细节见 [SABRE](5_sabre.md)。
