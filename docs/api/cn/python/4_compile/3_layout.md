# Layout

`cqlib.compile.transform.layout` 提供初始布局算法与布局分析工具。布局算法把逻辑比特映射到物理比特，为后续路由阶段提供起点。

## 导入

```python
from cqlib.compile.transform.layout import (
    trivial_layout,
    greedy_layout,
    vf2_perfect_layout,
    sabre_layout,
    analyze_circuit_for_layout,
    prepare_sabre_circuit,
    prepare_sabre_device_target,
    trivial_layout_prepared,
    greedy_layout_prepared,
    vf2_perfect_layout_prepared,
    sabre_layout_prepared,
    LayoutObjective,
    LayoutScore,
    LayoutDiagnostics,
    LayoutResult,
    Vf2EdgeRequirement,
    Vf2LayoutConfig,
    Interaction,
    InteractionGraph,
    CircuitLayoutAnalysis,
    DistanceTable,
    PhysicalLayoutGraph,
    PreparedSabreCircuit,
    PreparedSabreTarget,
)
```

---

## 布局函数

### trivial_layout(circuit, device, objective=None)

平凡布局：按编号顺序把逻辑比特映射到物理比特。

### greedy_layout(circuit, device, objective=None)

贪心布局：按交互强度顺序把逻辑比特放到物理比特上。

### vf2_perfect_layout(circuit, device, objective=None, config=None)

VF2 精确布局：在线路交互图能精确嵌入设备拓扑时给出完美布局。

参数：

- `config` (`Vf2LayoutConfig | None`)：VF2 搜索配置。

### sabre_layout(circuit, device, objective=None, config=None)

SABRE 布局：使用 SABRE 启发式细化候选布局。

参数：

- `config` (`SabreConfig | None`)：SABRE 配置，见 [SABRE](5_sabre.md)。

以上函数通用参数：

- `circuit` (`Circuit`)：待布局线路。
- `device` (`Device`)：目标设备。
- `objective` (`LayoutObjective | None`)：布局目标权重，默认 `LayoutObjective.topology_only()`。

返回：

- `LayoutResult`

### 预计算入口

对反复布局的场景，可先做一次线路分析或设备准备，再复用中间结果：

- `analyze_circuit_for_layout(circuit) -> CircuitLayoutAnalysis`：线路交互图分析。
- `prepare_sabre_circuit(circuit) -> PreparedSabreCircuit`：SABRE 线路预处理。
- `prepare_sabre_device_target(device) -> PreparedSabreTarget`：设备物理图预处理。
- `trivial_layout_prepared(analysis, physical, objective=None) -> LayoutResult`
- `greedy_layout_prepared(analysis, physical, objective=None) -> LayoutResult`
- `vf2_perfect_layout_prepared(analysis, physical, objective=None, config=None) -> LayoutResult`
- `sabre_layout_prepared(prepared, prepared_target, objective=None, config=None) -> LayoutResult`

其中 `analysis` 为 `CircuitLayoutAnalysis`，`physical` 为 `PhysicalLayoutGraph`，`prepared` 为 `PreparedSabreCircuit`，`prepared_target` 为 `PreparedSabreTarget`。

示例：

```python
from cqlib import Circuit
from cqlib.compile.transform.layout import sabre_layout
from cqlib.device import Device

circuit = Circuit(3)
circuit.cx(0, 2)

device = Device.line("line-3", 3)
result = sabre_layout(circuit, device)
print(result.layout)
```

---

## LayoutObjective

布局目标权重，用于给候选布局打分。

### LayoutObjective(*, distance_weight=1.0, direction_weight=1.0, two_qubit_error_weight=0.0, readout_error_weight=0.0)

参数：

- `distance_weight` (`float`)：最短距离项权重。
- `direction_weight` (`float`)：方向项权重。
- `two_qubit_error_weight` (`float`)：双比特门错误率项权重。
- `readout_error_weight` (`float`)：读出错误率项权重。

### 静态方法

- `LayoutObjective.topology_only()`：纯拓扑目标。
- `LayoutObjective.fidelity_aware()`：默认保真度感知目标。
- `LayoutObjective.auto_from_device(device)`：设备有可用校准数据时选择保真度感知目标，否则退回拓扑目标。
- `LayoutObjective.auto_from_physical(physical)`：同上，作用于已准备的物理图。
- `LayoutObjective.fidelity_required(device)`：保真度感知目标；设备没有可用校准数据时抛错。
- `LayoutObjective.fidelity_required_from_physical(physical)`：同上，作用于已准备的物理图。

### 属性

- `distance_weight`、`direction_weight`、`two_qubit_error_weight`、`readout_error_weight`（`float`），与构造参数一一对应。
- `uses_fidelity -> bool`：是否使用保真度项。

### 方法

- `score_layout(analysis, physical, layout) -> LayoutScore`：用线路分析、物理图和一个完整布局计算分数。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`。

---

## LayoutScore

布局打分结果。

### 属性

- `total -> float`：总分。
- `distance -> float`：距离项。
- `direction -> float`：方向项。
- `two_qubit_error -> float`：双比特错误项。
- `readout_error -> float`：读出错误项。
- `used_fidelity -> bool`：是否使用保真度项。

---

## LayoutDiagnostics

布局过程诊断信息。

### 属性

- `is_perfect -> bool`：是否找到完美布局。
- `candidates_evaluated -> int`：评估的候选数。
- `used_fidelity -> bool`：是否使用保真度项。
- `notes -> list[str]`：诊断说明。

---

## LayoutResult

布局函数返回对象。

### 属性

- `layout -> Layout`：选出的逻辑到物理布局。
- `score -> LayoutScore | None`：打分结果，无打分时为 `None`。
- `diagnostics -> LayoutDiagnostics`：诊断信息。

---

## Vf2EdgeRequirement

VF2 搜索的边要求。

### 静态方法

- `Vf2EdgeRequirement.positive_interactions()`：只要求正交互边匹配。
- `Vf2EdgeRequirement.all_interactions()`：要求所有交互边匹配。

---

## Vf2LayoutConfig

VF2 搜索配置。

### Vf2LayoutConfig(*, candidate_limit=10, call_limit=None, edge_requirement=None)

参数：

- `candidate_limit` (`int`)：候选布局数量上限。
- `call_limit` (`int | None`)：搜索调用次数上限。
- `edge_requirement` (`Vf2EdgeRequirement | None`)：边匹配要求。

### 属性

`candidate_limit`、`call_limit`、`edge_requirement`，与构造参数一一对应。

---

## 布局分析对象

### Interaction

一条逻辑比特对交互。

属性：

- `left -> Qubit`、`right -> Qubit`：交互两端逻辑比特。
- `weight -> float`：交互总权重。
- `directed_weight_left_to_right -> float`：从左到右的有向权重。
- `directed_weight_right_to_left -> float`：从右到左的有向权重。
- `first_seen_order -> int`：首次出现顺序。

### InteractionGraph

线路交互图。

属性与方法：

- `interactions -> list[Interaction]`：交互列表。
- `is_empty() -> bool`：是否无交互。
- `logical_activity() -> list[tuple[Qubit, float]]`：每个逻辑比特的总交互权重。

### CircuitLayoutAnalysis

线路布局分析结果，由 `analyze_circuit_for_layout()` 返回。

属性：

- `logical_qubits -> list[Qubit]`：逻辑比特列表。
- `interactions -> InteractionGraph`：交互图。

### DistanceTable

物理图最短距离表。

属性与方法：

- `qubits -> list[Qubit]`：物理比特列表。
- `distance(a, b) -> int | None`：两物理比特间最短距离，不可达时为 `None`。

### PhysicalLayoutGraph

设备物理布局图，由 `PhysicalLayoutGraph.from_device(device)` 构造。

属性与方法：

- `physical_qubits -> list[Qubit]`
- `distances -> DistanceTable`
- `has_fidelity_data -> bool`：是否有保真度数据。
- `has_readout_error_data -> bool`：是否有读出错误率数据。
- `has_two_qubit_error_data -> bool`：是否有双比特门错误率数据。
- `distance(a, b) -> int | None`
- `is_adjacent_undirected(a, b) -> bool`
- `readout_error(qubit) -> float | None`
- `supports_two_qubit_gate_directed(a, b) -> bool`
- `two_qubit_gate_error_directed(a, b) -> float | None`
- `supports_directed_coupling(a, b) -> bool`

### PreparedSabreCircuit

SABRE 线路预处理结果，由 `prepare_sabre_circuit()` 返回。

属性：

- `analysis -> CircuitLayoutAnalysis`
- `logical_qubits -> list[Qubit]`

### PreparedSabreTarget

设备物理图预处理结果，由 `prepare_sabre_device_target()` 返回。

属性：

- `physical -> PhysicalLayoutGraph`

---

## 异常情况

- `CompilerConfigError`：配置非法，例如保真度目标在无校准数据设备上使用。
- `CompilerTransformError`：布局无法完成，例如交互无法在拓扑上满足。
