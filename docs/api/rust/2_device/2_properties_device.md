# Device / Properties

本页覆盖 `cqlib_core::device` 中用于设备属性建模的类型：

- `InstructionProp`
- `QubitProp`
- `EdgeProp`
- `Device`

## 导入

```rust
use cqlib_core::circuit::{Instruction, Qubit, StandardGate};
use cqlib_core::device::{Device, DeviceError, EdgeProp, InstructionProp, QubitProp, Topology};
use time::OffsetDateTime;
```

## InstructionProp

### 构造与方法

- `InstructionProp::new(instruction: Instruction, error_rate: f64) -> InstructionProp`
- `with_length(self, length: f64) -> InstructionProp`

### 只读接口

- `instruction(&self) -> &Instruction`
- `error_rate(&self) -> f64`
- `length(&self) -> Option<f64>`

## QubitProp

### 构造与 Builder

- `QubitProp::new(readout_error: f64) -> QubitProp`
- `with_prob_meas0_prep1(self, prob: f64) -> QubitProp`
- `with_prob_meas1_prep0(self, prob: f64) -> QubitProp`
- `with_t1(self, t1: f64) -> QubitProp`
- `with_t2(self, t2: f64) -> QubitProp`
- `with_frequency(self, frequency: f64) -> QubitProp`
- `with_native_instruction(self, prop: InstructionProp) -> QubitProp`

### 只读接口

- `readout_error(&self) -> f64`
- `t1(&self) -> Option<f64>`
- `t2(&self) -> Option<f64>`
- `frequency(&self) -> Option<f64>`
- `native_instructions(&self) -> &[InstructionProp]`

## EdgeProp

### 构造与方法

- `EdgeProp::new() -> EdgeProp`
- `with_native_instruction(self, prop: InstructionProp) -> EdgeProp`
- `native_instructions(&self) -> &[InstructionProp]`

## Device

### 构造

- `Device::new(name: String, topology: Topology) -> Device`

### 配置接口（builder）

- `with_native_gates(self, gates: Vec<Instruction>) -> Device`
- `with_calibration_time(self, time: OffsetDateTime) -> Device`
- `with_default_t1(self, t1: f64) -> Device`
- `with_default_t2(self, t2: f64) -> Device`
- `with_default_readout_error(self, error: f64) -> Device`
- `with_default_single_qubit_error(self, error: f64) -> Device`
- `with_default_two_qubit_error(self, error: f64) -> Device`

### 写入接口

- `add_qubit_properties(&mut self, qubit: Qubit, props: QubitProp) -> Result<(), DeviceError>`
- `add_edge_properties(&mut self, control: Qubit, target: Qubit, props: EdgeProp) -> Result<(), DeviceError>`

常见错误：

- `DeviceError::QubitNotInTopology`
- `DeviceError::EdgeNotInTopology`

### 查询接口

- `name(&self) -> &str`
- `qubits(&self) -> impl Iterator<Item = Qubit>`
- `invalid_qubits(&self) -> impl Iterator<Item = Qubit>`
- `topology(&self) -> &Topology`
- `native_gates(&self) -> &[Instruction]`
- `qubit_properties(&self, qubit: Qubit) -> Option<&QubitProp>`
- `edge_properties(&self, control: Qubit, target: Qubit) -> Option<&EdgeProp>`
- `get_t1(&self, qubit: Qubit) -> Option<f64>`
- `get_t2(&self, qubit: Qubit) -> Option<f64>`
- `get_readout_error(&self, qubit: Qubit) -> Option<f64>`
- `default_single_qubit_error(&self) -> Option<f64>`
- `default_two_qubit_error(&self) -> Option<f64>`

说明：

- `get_t1/get_t2/get_readout_error` 会优先读取局部属性，缺失时回退默认值。

## 示例

```rust
use cqlib_core::circuit::{Instruction, Qubit, StandardGate};
use cqlib_core::device::{Device, EdgeProp, InstructionProp, QubitProp, Topology};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let q2 = Qubit::new(2);

let topo = Topology::new(
    vec![q0, q1, q2],
    vec![(q0, q1, "CX".to_string()), (q1, q2, "CX".to_string())],
);

let mut device = Device::new("mock_backend".to_string(), topo)
    .with_default_t1(50.0)
    .with_default_t2(35.0)
    .with_default_readout_error(0.05)
    .with_native_gates(vec![
        Instruction::from(StandardGate::X),
        Instruction::from(StandardGate::CX),
    ]);

device
    .add_qubit_properties(q0, QubitProp::new(0.02).with_t1(80.0).with_t2(70.0))
    .unwrap();

device
    .add_edge_properties(
        q0,
        q1,
        EdgeProp::new().with_native_instruction(
            InstructionProp::new(Instruction::from(StandardGate::CX), 0.01).with_length(220.0),
        ),
    )
    .unwrap();

assert_eq!(device.get_t1(q0), Some(80.0));
assert_eq!(device.get_t1(q2), Some(50.0)); // 回退默认值
```
