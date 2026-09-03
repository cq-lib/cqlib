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

### `InstructionProp(instruction, error_rate)`

参数：

- `instruction` (`Instruction`)
- `error_rate` (`float`)

### 方法

- `with_length(length) -> InstructionProp`

### 属性

- `instruction -> Instruction`
- `error_rate -> float`
- `length -> float | None`

## QubitProp

### `QubitProp(readout_error)`

参数：

- `readout_error` (`float`)

### Builder 方法

- `with_prob_meas0_prep1(prob) -> QubitProp`
- `with_prob_meas1_prep0(prob) -> QubitProp`
- `with_t1(t1) -> QubitProp`
- `with_t2(t2) -> QubitProp`
- `with_frequency(frequency) -> QubitProp`
- `with_native_instruction(prop) -> QubitProp`

### 属性

- `readout_error -> float`
- `t1 -> float | None`
- `t2 -> float | None`
- `frequency -> float | None`
- `native_instructions -> list[InstructionProp]`

## EdgeProp

### `EdgeProp()`

### 方法

- `with_native_instruction(prop) -> EdgeProp`

### 属性

- `native_instructions -> list[InstructionProp]`

## Device

### `Device(name, topology)`

参数：

- `name` (`str`)
- `topology` (`Topology`)

### 配置方法（返回新对象）

- `with_native_gates(gates) -> Device`
- `with_default_t1(t1) -> Device`
- `with_default_t2(t2) -> Device`
- `with_default_readout_error(error) -> Device`
- `with_default_single_qubit_error(error) -> Device`
- `with_default_two_qubit_error(error) -> Device`

### 写入方法（原位修改）

- `add_qubit_properties(qubit, props) -> None`
- `add_edge_properties(control, target, props) -> None`

异常情况：

- `ValueError`：比特或耦合边不在拓扑中。

### 查询属性

- `name -> str`
- `qubits -> list[int]`
- `invalid_qubits -> list[int]`
- `topology -> Topology`
- `native_gates -> list[Instruction]`
- `default_single_qubit_error -> float | None`
- `default_two_qubit_error -> float | None`

### 查询方法

- `qubit_properties(qubit) -> QubitProp | None`
- `edge_properties(control, target) -> EdgeProp | None`
- `get_t1(qubit) -> float | None`
- `get_t2(qubit) -> float | None`
- `get_readout_error(qubit) -> float | None`

说明：

- `get_t1/get_t2/get_readout_error` 会优先返回局部配置，缺失时回退到默认值。

## 示例

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CX")])

device = (
    Device("mock_backend", topo)
    .with_default_t1(50.0)
    .with_default_t2(35.0)
    .with_default_readout_error(0.05)
    .with_native_gates(
        [
            Instruction.from_standard_gate(StandardGate.X),
            Instruction.from_standard_gate(StandardGate.CX),
        ]
    )
)

device.add_qubit_properties(0, QubitProp(0.02).with_t1(80.0).with_t2(70.0))
device.add_edge_properties(
    0,
    1,
    EdgeProp().with_native_instruction(
        InstructionProp(
            Instruction.from_standard_gate(StandardGate.CX),
            0.01,
        ).with_length(220.0)
    ),
)

print(device.get_t1(0))  # 80.0
print(device.get_t1(2))  # 50.0（默认值）
```

