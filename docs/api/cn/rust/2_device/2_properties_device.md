# Device / Properties

本页覆盖 `cqlib_core::device` 中用于设备属性建模的类型：

- `InstructionProp`
- `QubitProp`
- `EdgeProp`
- `Device`

## 导入

```rust
use std::collections::HashSet;

use cqlib_core::circuit::{Instruction, StandardGate};
use cqlib_core::device::{
    Device, DeviceError, EdgeProp, InstructionProp, PhysicalQubit, QubitProp, Topology,
};
use time::OffsetDateTime;
```

## InstructionProp

### 构造与方法

- `InstructionProp::new(instruction: Instruction, error_rate: f64) -> InstructionProp`
- `with_length(self, length: f64) -> InstructionProp`
- `set_length(&mut self, length: f64)`
- `with_instruction(self, instruction: Instruction) -> InstructionProp`
- `set_instruction(&mut self, instruction: Instruction)`
- `with_error_rate(self, error_rate: f64) -> InstructionProp`
- `set_error_rate(&mut self, error_rate: f64)`

### 只读接口

- `instruction(&self) -> &Instruction`
- `error_rate(&self) -> f64`
- `length(&self) -> Option<f64>`

## QubitProp

### 构造与 Builder

- `QubitProp::new(readout_error: f64) -> QubitProp`
- `with_prob_meas0_prep1(self, prob: f64) -> QubitProp`
- `set_prob_meas0_prep1(&mut self, prob: f64)`
- `with_prob_meas1_prep0(self, prob: f64) -> QubitProp`
- `set_prob_meas1_prep0(&mut self, prob: f64)`
- `with_t1(self, t1: f64) -> QubitProp`
- `set_t1(&mut self, t1: f64)`
- `with_t2(self, t2: f64) -> QubitProp`
- `set_t2(&mut self, t2: f64)`
- `with_frequency(self, frequency: f64) -> QubitProp`
- `set_frequency(&mut self, frequency: f64)`
- `with_native_instruction(self, prop: InstructionProp) -> Result<QubitProp, DeviceError>`
- `set_native_instruction(&mut self, prop: InstructionProp) -> Result<(), DeviceError>`

### 只读接口

- `readout_error(&self) -> f64`
- `prob_meas0_prep1(&self) -> Option<f64>`
- `prob_meas1_prep0(&self) -> Option<f64>`
- `t1(&self) -> Option<f64>`
- `t2(&self) -> Option<f64>`
- `frequency(&self) -> Option<f64>`
- `native_instructions(&self) -> &[InstructionProp]`

说明：

- 比特属性的原生指令只接受单比特标准门，非单比特指令返回 `DeviceError::InvalidNativeInstructionArity`。

## EdgeProp

### 构造与方法

- `EdgeProp::new() -> EdgeProp`
- `with_native_instruction(self, prop: InstructionProp) -> Result<EdgeProp, DeviceError>`
- `set_native_instruction(&mut self, prop: InstructionProp) -> Result<(), DeviceError>`
- `native_instructions(&self) -> &[InstructionProp]`

说明：

- 边属性的原生指令只接受双比特标准门。

## Device

### 构造

- `Device::new(name: impl Into<String>, qubits: HashSet<PhysicalQubit>, topology: Topology) -> Result<Device, DeviceError>`

说明：

- 拓扑中的每个比特都必须登记在 `qubits` 中，否则返回 `DeviceError::InvalidOnlineQubit`。

### 静态构造方法

- `line(name: impl Into<String>, num_qubits: u32) -> Result<Device, DeviceError>`
- `line_from_qubits(name: impl Into<String>, physical_qubits: Vec<PhysicalQubit>) -> Result<Device, DeviceError>`
- `bidirectional_line(name: impl Into<String>, num_qubits: u32) -> Result<Device, DeviceError>`
- `ring(name: impl Into<String>, num_qubits: u32) -> Result<Device, DeviceError>`
- `star(name: impl Into<String>, num_qubits: u32, center: u32) -> Result<Device, DeviceError>`
- `grid(name: impl Into<String>, rows: u32, cols: u32) -> Result<Device, DeviceError>`
- `from_edges(name: impl Into<String>, num_qubits: u32, edges: &[(u32, u32)]) -> Result<Device, DeviceError>`

说明：

- 静态构造方法生成的比特全部在线；直线类形态的耦合名为空串。

### 配置接口（builder）

- `with_invalid_qubits(self, invalid_qubits: HashSet<PhysicalQubit>) -> Result<Device, DeviceError>`
- `set_invalid_qubits(&mut self, invalid_qubits: HashSet<PhysicalQubit>) -> Result<(), DeviceError>`
- `with_native_gates(self, gates: Vec<Instruction>) -> Result<Device, DeviceError>`
- `set_native_gates(&mut self, gates: Vec<Instruction>) -> Result<(), DeviceError>`
- `with_calibration_time(self, time: OffsetDateTime) -> Device`
- `set_calibration_time(&mut self, time: OffsetDateTime)`
- `with_default_t1(self, t1: f64) -> Device`
- `set_default_t1(&mut self, t1: f64)`
- `with_default_t2(self, t2: f64) -> Device`
- `set_default_t2(&mut self, t2: f64)`
- `with_default_readout_error(self, error: f64) -> Device`
- `set_default_readout_error(&mut self, error: f64)`
- `with_default_single_qubit_error(self, error: f64) -> Device`
- `set_default_single_qubit_error(&mut self, error: f64)`
- `with_default_two_qubit_error(self, error: f64) -> Device`
- `set_default_two_qubit_error(&mut self, error: f64)`

说明：

- `with_native_gates` 与 `set_native_gates` 只接受至多两比特的标准门，非法项使整次写入被拒绝，原有门集保持不变。

### 写入接口

- `add_qubit_properties(&mut self, qubit: PhysicalQubit, props: QubitProp) -> Result<(), DeviceError>`
- `add_edge_properties(&mut self, control: PhysicalQubit, target: PhysicalQubit, props: EdgeProp) -> Result<(), DeviceError>`

常见错误：

- `DeviceError::InvalidOnlineQubit`
- `DeviceError::QubitNotInDevice`
- `DeviceError::QubitNotInTopology`
- `DeviceError::EdgeNotInTopology`
- `DeviceError::NonStandardNativeInstruction`
- `DeviceError::InvalidNativeInstructionArity`
- `DeviceError::InvalidNativeInstructionErrorRate`
- `DeviceError::InvalidNativeInstructionDuration`

### 查询接口

- `name(&self) -> &str`
- `qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `invalid_qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `is_usable_qubit(&self, qubit: PhysicalQubit) -> bool`
- `usable_qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `num_usable_qubits(&self) -> usize`
- `topology(&self) -> &Topology`
- `native_gates(&self) -> &[Instruction]`
- `qubit_properties(&self, qubit: PhysicalQubit) -> Option<&QubitProp>`
- `edge_properties(&self, control: PhysicalQubit, target: PhysicalQubit) -> Option<&EdgeProp>`
- `supports_native_instruction(&self, instruction: &Instruction, qargs: &[PhysicalQubit]) -> bool`
- `validate_operation(&self, operation: &Operation) -> Result<(), DeviceValidationError>`
- `validate_value_operation(&self, operation: &ValueOperation) -> Result<(), DeviceValidationError>`
- `validate_circuit(&self, circuit: &Circuit) -> Result<(), DeviceValidationError>`
- `single_qubit_error(&self, qubit: PhysicalQubit, instruction: &Instruction) -> Option<f64>`
- `two_qubit_error(&self, control: PhysicalQubit, target: PhysicalQubit, instruction: &Instruction) -> Option<f64>`
- `edge_error(&self, control: PhysicalQubit, target: PhysicalQubit) -> Option<f64>`
- `get_t1(&self, qubit: PhysicalQubit) -> Option<f64>`
- `get_t2(&self, qubit: PhysicalQubit) -> Option<f64>`
- `get_readout_error(&self, qubit: PhysicalQubit) -> Option<f64>`
- `default_single_qubit_error(&self) -> Option<f64>`
- `default_two_qubit_error(&self) -> Option<f64>`
- `default_t1(&self) -> Option<f64>`
- `default_t2(&self) -> Option<f64>`
- `default_readout_error(&self) -> Option<f64>`
- `calibration_time(&self) -> Option<OffsetDateTime>`

说明：

- `get_t1` / `get_t2` / `get_readout_error` 优先读取局部属性，缺失时回退默认值。
- `single_qubit_error` / `two_qubit_error` 优先取局部原生指令属性，缺失时回退默认的单比特 / 双比特误差率；比特不可用或该方向不存在时返回 `None`。
- `edge_error` 只读标定数据，取该有向边上原生指令的最小误差率。

## 示例

```rust
use std::collections::HashSet;

use cqlib_core::circuit::{Instruction, StandardGate};
use cqlib_core::device::{Device, EdgeProp, InstructionProp, PhysicalQubit, QubitProp, Topology};

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let q2 = PhysicalQubit::new(2);

let topology = Topology::new(
    vec![q0, q1, q2],
    vec![(q0, q1, "CX".to_string()), (q1, q2, "CX".to_string())],
)
.unwrap();

let mut device = Device::new("mock_backend", HashSet::from([q0, q1, q2]), topology)
    .unwrap()
    .with_default_t1(50.0)
    .with_default_t2(35.0)
    .with_default_readout_error(0.05)
    .with_native_gates(vec![
        Instruction::Standard(StandardGate::H),
        Instruction::Standard(StandardGate::CX),
    ])
    .unwrap();

let qp0 = QubitProp::new(0.02)
    .with_t1(80.0)
    .with_t2(70.0)
    .with_native_instruction(InstructionProp::new(
        Instruction::Standard(StandardGate::H),
        0.001,
    ))
    .unwrap();
device.add_qubit_properties(q0, qp0).unwrap();

let ep01 = EdgeProp::new()
    .with_native_instruction(InstructionProp::new(
        Instruction::Standard(StandardGate::CX),
        0.02,
    ))
    .unwrap();
device.add_edge_properties(q0, q1, ep01).unwrap();

assert_eq!(device.get_t1(q0), Some(80.0));
assert_eq!(device.get_t1(q2), Some(50.0)); // 回退默认值
assert_eq!(
    device.single_qubit_error(q0, &Instruction::Standard(StandardGate::H)),
    Some(0.001)
);
assert_eq!(
    device.two_qubit_error(q0, q1, &Instruction::Standard(StandardGate::CX)),
    Some(0.02)
);
assert_eq!(device.edge_error(q0, q1), Some(0.02));
assert_eq!(device.num_usable_qubits(), 3);

device.set_invalid_qubits(HashSet::from([q2])).unwrap();
assert!(!device.is_usable_qubit(q2));
assert_eq!(device.num_usable_qubits(), 2);
```
