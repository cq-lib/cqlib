# Routing

`cqlib.compile.transform.routing` 提供变换层路由入口。路由在受限拓扑上插入 SWAP，使线路中所有二体交互满足设备邻接约束。

## 导入

```python
from cqlib.compile.transform.routing import (
    route_with_layout,
    route_sabre,
    RoutedCircuit,
    SabreRouteResult,
)
```

---

## 函数

### route_with_layout(circuit, device, initial_layout, config=None)

已知初始布局条件下的确定性路由。

参数：

- `circuit` (`Circuit`)：待路由线路。
- `device` (`Device`)：目标设备。
- `initial_layout` (`Layout`)：逻辑到物理初始布局。
- `config` (`SabreConfig | None`)：SABRE 配置，默认 `SabreConfig()`。

返回：

- `RoutedCircuit`

### route_sabre(circuit, device, objective=None, config=None)

带布局选择的 SABRE 路由：先按 `objective` 选出初始布局，再执行路由。

参数：

- `circuit` (`Circuit`)：待路由线路。
- `device` (`Device`)：目标设备。
- `objective` (`LayoutObjective | None`)：布局目标，默认 `LayoutObjective.topology_only()`。
- `config` (`SabreConfig | None`)：SABRE 配置，默认 `SabreConfig()`。

返回：

- `SabreRouteResult`

示例：

```python
from cqlib import Circuit
from cqlib.compile.transform.routing import route_sabre
from cqlib.device import Device

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
result = route_sabre(circuit, device)
print("swap_count:", result.swap_count)
```

---

## RoutedCircuit

已知初始布局条件下的路由结果。

### 属性

- `circuit -> Circuit`：路由后的线路。
- `initial_layout -> Layout`：路由开始前的布局。
- `final_layout -> Layout`：路由结束后的布局。
- `swap_count -> int`：插入的 SWAP 数量。
- `diagnostics -> SabreRoutingDiagnostics`：路由诊断信息，见 [SABRE](5_sabre.md)。
- `changed -> bool`：路由是否改变了线路。

---

## SabreRouteResult

带布局选择的路由结果，在 `RoutedCircuit` 的基础上附带布局打分。

### 属性

- `routed -> RoutedCircuit`：路由结果。
- `layout_score -> LayoutScore | None`：选中布局的分数。
- `layout_diagnostics -> LayoutDiagnostics`：布局过程诊断信息。
- `circuit`、`initial_layout`、`final_layout`、`swap_count`、`diagnostics`、`changed`：同 `RoutedCircuit`。

示例：

```python
from cqlib import Circuit
from cqlib.compile.transform.routing import route_sabre
from cqlib.device import Device

circuit = Circuit(2)
circuit.cx(0, 1)

device = Device.line("line-3", 3)
result = route_sabre(circuit, device)

print("selected layout:", result.routed.initial_layout)
print("score:", result.layout_score.total if result.layout_score else None)
```

---

## 异常情况

- `CompilerConfigError`：SABRE 配置非法，例如 `routing_trials` 为零。
- `CompilerTransformError`：路由失败，例如初始布局缺失或不可达、设备容量不足。

低层 `sabre_route` 与 `SabreConfig` 的细节见 [SABRE](5_sabre.md)。
