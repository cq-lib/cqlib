# Device / Properties

本页覆盖 `cqlib.device` 中用于设备标定建模的核心类型：

- `InstructionProp`
- `QubitProp`
- `EdgeProp`
- `Device`

## 导入

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology
```

---

## InstructionProp

指令属性：一条指令、它的误差率与可选时长。

### `InstructionProp(instruction, error_rate)`

参数：

- `instruction` (`Instruction`)：指令对象。
- `error_rate` (`float`)：误差率，取值需在 `[0, 1]` 内。

异常情况：

- `ValueError`：`error_rate` 超出 `[0, 1]` 或不是有限值。

### 属性

- `instruction -> Instruction`：指令对象，可读写。
- `error_rate -> float`：误差率，可读写，写入时按 `[0, 1]` 校验。
- `length -> float | None`：指令时长，可读写，写入时要求非负有限值。

### 其他行为

- 支持 `copy`、`deepcopy`。
- `repr(prop) == "InstructionProp(instruction=X, error_rate=0.01)"`，设置 `length` 后追加 `length=...`。

## QubitProp

比特属性：读出误差、读出混淆概率、`t1` / `t2`、频率与原生指令属性。

### `QubitProp(readout_error)`

参数：

- `readout_error` (`float`)：读出误差，取值需在 `[0, 1]` 内。

异常情况：

- `ValueError`：`readout_error` 超出 `[0, 1]` 或不是有限值。

### 属性

- `readout_error -> float`：只读。
- `prob_meas0_prep1 -> float | None`：可读写，取值需在 `[0, 1]` 内。
- `prob_meas1_prep0 -> float | None`：可读写，取值需在 `[0, 1]` 内。
- `t1 -> float | None`：可读写，要求有限正数。
- `t2 -> float | None`：可读写，要求有限正数。
- `frequency -> float | None`：可读写，要求有限正数。
- `native_instructions -> list[InstructionProp]`：只读，本比特的原生指令属性。

### 方法

- `add_native_instruction(prop) -> None`：追加一条原生指令属性。

异常情况：

- `ValueError`：概率类参数非法、`t1` / `t2` / `frequency` 不是有限正数，或追加的指令不是单比特标准门。
- 写入失败时原有值保持不变。

## EdgeProp

边属性：一条有向边上的原生指令属性。

### `EdgeProp()`

无参构造。

### 方法

- `add_native_instruction(prop) -> None`：追加一条原生指令属性。

异常情况：

- `ValueError`：追加的指令不是双比特标准门，或其误差率、时长非法。

### 属性

- `native_instructions -> list[InstructionProp]`：只读。

## Device

设备对象：拓扑、已登记比特集合、原生门集、校准时间与设备级默认值。

### `Device(name, qubits, topology)`

参数：

- `name` (`str`)：设备名。
- `qubits` (`list[int | Qubit | PhysicalQubit]`)：已登记的物理比特。
- `topology` (`Topology`)：设备拓扑。

异常情况：

- `ValueError`：拓扑中存在未登记在 `qubits` 中的比特。

### 静态构造方法

- `Device.line(name, num_qubits) -> Device`：比特 `0..num_qubits` 组成的有向直线。
- `Device.line_from_qubits(name, physical_qubits) -> Device`：按给定物理比特顺序连成有向直线。
- `Device.bidirectional_line(name, num_qubits) -> Device`：双向直线。
- `Device.ring(name, num_qubits) -> Device`：双向环。
- `Device.star(name, num_qubits, center) -> Device`：以 `center` 为中心的双向星形。
- `Device.grid(name, rows, cols) -> Device`：`rows × cols` 双向网格，比特编号按行优先排列。
- `Device.from_edges(name, num_qubits, edges) -> Device`：比特 `0..num_qubits` 加显式有向边 `list[tuple[int, int]]`。

### 属性

- `name -> str`：只读。
- `qubits -> list[PhysicalQubit]`：只读，已登记的物理比特。
- `invalid_qubits -> list[PhysicalQubit]`：可读写；写入的值必须已登记在设备上，否则拒绝并保留原值。
- `topology -> Topology`：只读。
- `native_gates -> list[Instruction]`：可读写；写入时只接受至多两比特的标准门。
- `usable_qubits -> list[PhysicalQubit]`：只读，已登记且未标记为不可用的比特。
- `num_usable_qubits -> int`：只读。
- `default_t1 -> float | None`：可读写，设备级默认 `t1`。
- `default_t2 -> float | None`：可读写，设备级默认 `t2`。
- `default_readout_error -> float | None`：可读写，设备级默认读出误差。
- `default_single_qubit_error -> float | None`：可读写，设备级默认单比特误差率。
- `default_two_qubit_error -> float | None`：可读写，设备级默认双比特误差率。
- `calibration_time -> datetime | None`：只读，校准时间戳，纳秒精度。

### 写入方法

- `set_calibration_time(datetime) -> None`：写入校准时间戳。
- `add_qubit_properties(qubit, props) -> None`：写入某个比特的属性。
- `add_edge_properties(control, target, props) -> None`：写入某条有向边的属性。

异常情况：

- `ValueError`：比特不在设备中或不在拓扑中、有向边不在拓扑中、默认值非法，或时间超出可表示范围。
- 三类写入失败时设备保留原有值。

### 查询方法

- `qubit_properties(qubit) -> QubitProp | None`
- `edge_properties(control, target) -> EdgeProp | None`
- `get_t1(qubit) -> float | None`
- `get_t2(qubit) -> float | None`
- `get_readout_error(qubit) -> float | None`
- `single_qubit_error(qubit, instruction) -> float | None`
- `two_qubit_error(control, target, instruction) -> float | None`
- `edge_error(control, target) -> float | None`
- `is_usable_qubit(qubit) -> bool`
- `supports_native_instruction(instruction, qargs) -> bool`
- `validate_operation(operation) -> None`
- `validate_circuit(circuit) -> None`

说明：

- `get_t1` / `get_t2` / `get_readout_error` 优先返回局部属性，缺失时回退设备级默认值。
- `single_qubit_error` / `two_qubit_error` 优先取局部原生指令属性，缺失时回退默认的单比特 / 双比特误差率；比特不可用或该方向不存在时返回 `None`。
- `edge_error` 只读标定数据，取该有向边上原生指令的最小误差率。
- `validate_operation` / `validate_circuit` 校验失败时抛 `ValueError`。

### 其他行为

- 支持 `copy`、`deepcopy`。
- `repr(device) == "Device(name='mock_backend')"`。

## 示例

```python
from cqlib import Qubit
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])
device = Device("mock_backend", [Qubit(0), Qubit(1), Qubit(2)], topo)

device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.native_gates = [
    Instruction.from_standard_gate(StandardGate.X),
    Instruction.from_standard_gate(StandardGate.CX),
]

x_inst = Instruction.from_standard_gate(StandardGate.X)
cx_inst = Instruction.from_standard_gate(StandardGate.CX)

qp0 = QubitProp(0.02)
qp0.t1 = 80.0
qp0.t2 = 70.0
qp0.add_native_instruction(InstructionProp(x_inst, 0.001))
device.add_qubit_properties(0, qp0)

ep01 = EdgeProp()
ep01.add_native_instruction(InstructionProp(cx_inst, 0.02))
device.add_edge_properties(0, 1, ep01)

assert device.get_t1(0) == 80.0
assert device.get_t1(2) == 50.0  # 回退到设备级默认值
assert device.single_qubit_error(0, x_inst) == 0.001
assert device.two_qubit_error(0, 1, cx_inst) == 0.02
assert device.edge_error(0, 1) == 0.02
assert device.num_usable_qubits == 3

device.invalid_qubits = [2]
assert device.is_usable_qubit(2) is False
assert device.num_usable_qubits == 2
```
