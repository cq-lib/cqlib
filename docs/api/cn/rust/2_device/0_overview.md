# 设备

`cqlib_core::device`

`cqlib_core::device` 描述量子设备的静态信息与任务执行状态：比特与耦合构成的拓扑、比特与边的校准属性、逻辑比特到物理比特的布局、噪声模型，以及执行结果与状态。

## Overview

设备模块把硬件相关的数据拆成几组互相独立的对象：拓扑只描述连接关系，属性只描述标定数据，布局只描述映射，噪声模型只描述信道。编译与执行各自按需取用，互不耦合。

### 拓扑

`Topology` 由物理比特列表与耦合列表构成，内部维护一张有向图，`graph()` 可以取出底层图引用。耦合是有向的：`(control, target)` 上有耦合，不代表反方向也有，因此连接性查询分成 `successors` / `predecessors`（有向）与 `neighbors_undirected`（无向）两类，度数查询也分成 `out_degree` / `in_degree`。耦合可以带一个名字标签，仅作标识之用。

### 设备与属性

`Device` 在拓扑之上叠加设备级默认值与局部覆盖，并记录设备名、已注册比特、不可用比特、原生门集与校准时间。设备级配置项大多有两套写法：`with_*` 消费并返回自身，用于链式构造；`set_*` 就地修改。比特级属性由 `QubitProp` 描述（`t1`、`t2`、`frequency`、读出误差与读出混淆概率），边级属性由 `EdgeProp` 描述，两者都持有一组 `InstructionProp`（指令、误差率与可选时长）。

查询某个比特的属性时，局部值优先，未设置则回退到设备级默认值：`get_t1` 先看该比特的 `t1`，取不到再返回 `default_t1`。设备还提供 `line`、`line_from_qubits`、`bidirectional_line`、`ring`、`star`、`grid`、`from_edges` 等工厂方法，按常见形态直接生成拓扑与比特集合。

### 原生指令能力

设备级 `native_gates` 是全局默认能力；某个比特或某条有向边若通过 `with_native_instruction` / `set_native_instruction` 配置了自己的指令列表，该列表就是此处能力的完整覆盖，不会回写默认值。校准数据只让受支持的指令带上误差率与时长，不会让不受支持的指令变为受支持。写入原生能力前会先完成校验，被拒绝的更新不会改变原有能力。`validate_operation`、`validate_value_operation` 与 `validate_circuit` 用同一套规则检查线路是否能在该设备上执行。

### 布局

`Layout` 维护逻辑比特到物理比特的映射，未承载逻辑比特的物理比特是空闲物理比特。映射可以按 `l2p_map` / `p2l_map` 整体读取，也可以按单个比特查询；`swap_physical` 交换两个物理位置上承载的逻辑比特，这是路由过程中移动逻辑比特的基本操作。

### 噪声模型

`NoiseModel` 按比特与操作登记噪声信道：读出误差用 `ReadoutError` 表示，单比特信道由 `SingleQubitNoise` 的变体给出（比特翻转、相位翻转、Pauli、去极化、振幅阻尼、相位阻尼），双比特信道由 `TwoQubitNoise` 给出（去极化、独立信道组合、关联 Pauli）。信道可以用 `to_kraus` 转成 Kraus 算符，供密度矩阵模拟使用。查询时用 `OperationKey` 指定门与比特组合。

### 执行结果

`ExecutionResult` 记录一次任务的生命周期：`new` 构造后处于 `Status::Queued`，`start` 后进入运行态，`finish` / `fail` / `cancel` 结束任务，`from_counts` 可以直接由测量计数构造出已完成的结果。测量结果以 `Outcome` 表示，它把比特串压缩成 64 位分块，同时支持按位置取位与还原比特串；`counts` 与 `probabilities` 分别是各结果的测量次数与归一化概率。

---

## 常用入口

```rust
use std::collections::HashSet;

use cqlib_core::device::{Device, PhysicalQubit, QubitProp, Topology};

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "CX".to_string())]).unwrap();

let mut device = Device::new("demo".to_string(), HashSet::from_iter([q0, q1]), topology)
    .unwrap()
    .with_default_t1(40.0);

device.add_qubit_properties(q0, QubitProp::new(0.02).with_t1(60.0)).unwrap();

assert_eq!(device.get_t1(q0), Some(60.0)); // 局部属性
assert_eq!(device.get_t1(q1), Some(40.0)); // 回退到设备级默认值
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **物理比特** | 设备上的一个物理位置，用 `PhysicalQubit` 表示。 |
| **逻辑比特** | 线路中的一根线，用 `LogicalQubit` 表示；即使编号相同，也与物理比特互相区分。 |
| **拓扑** | 物理比特与它们之间耦合构成的有向图，由 `Topology` 表示。 |
| **耦合** | 两个物理比特之间可以执行双比特门的连接关系。 |
| **有向耦合** | 带方向的耦合：`(control, target)` 上存在耦合不代表反方向也存在。 |
| **原生指令** | 设备上可以直接执行的指令；设备级集合是默认能力，比特与边可以各自覆盖。 |
| **局部覆盖** | 比特或边的原生指令列表，一旦非空就完整取代设备级默认能力，不回写默认值。 |
| **默认值回退** | 查询比特属性时，该比特没有局部值就返回设备级默认值。 |
| **初始布局** | 逻辑比特到物理比特的初始映射，由 `Layout` 表示。 |
| **空闲物理比特** | 当前没有承载逻辑比特的物理比特。 |
| **噪声信道** | 描述一种噪声作用的算符集合，可用 `to_kraus` 转为 Kraus 算符。 |
| **读出误差** | 测量时把 `\|0>` 读成 1 或把 `\|1>` 读成 0 的概率，由 `ReadoutError` 的 `p_0_given_1` 与 `p_1_given_0` 两个字段给出。 |
| **测量结果** | 一次采样得到的比特串，用 `Outcome` 表示。 |

---

## `cqlib_core::device` API 概览

### 拓扑

| 名字 | 简介 |
| --- | --- |
| [`Topology`](1_topology.md) | 物理比特与耦合列表构成的有向图，含增删与连接性查询。 |
| [`Topology::line`](1_topology.md) | 按直线形态构造拓扑的构造函数。 |

### 设备与属性

| 名字 | 简介 |
| --- | --- |
| [`Device`](2_properties_device.md) | 设备对象：拓扑、比特集合、原生门集、校准时间与默认属性。 |
| [`Device::line`](2_properties_device.md) / [`Device::ring`](2_properties_device.md) / [`Device::star`](2_properties_device.md) / [`Device::grid`](2_properties_device.md) | 按常见形态构建设备的工厂方法。 |
| [`QubitProp`](2_properties_device.md) | 比特属性：读取误差、混淆概率、`t1` / `t2`、频率与原生指令属性。 |
| [`EdgeProp`](2_properties_device.md) | 边属性：该有向边上的原生指令属性。 |
| [`InstructionProp`](2_properties_device.md) | 指令属性：指令对象、误差率与可选时长。 |
| [`Device::validate_circuit`](2_properties_device.md) / [`Device::validate_operation`](2_properties_device.md) | 按原生能力与拓扑校验线路或单个操作。 |

### 布局

| 名字 | 简介 |
| --- | --- |
| [`Layout`](3_layout.md) | 逻辑比特到物理比特的映射，含绑定、解绑与物理比特交换。 |
| [`Layout::from_pairs`](3_layout.md) | 由 `(逻辑编号, 物理编号)` 对与物理比特总数构造布局。 |
| [`LogicalQubit`](3_layout.md) / [`PhysicalQubit`](3_layout.md) | 逻辑比特与物理比特的类型，`Display` 形式分别为 `L0` 与 `P11`。 |

### 噪声模型

| 名字 | 简介 |
| --- | --- |
| [`NoiseModel`](4_noise.md) | 按比特与操作登记噪声信道的容器。 |
| [`SingleQubitNoise`](4_noise.md) | 单比特噪声信道枚举，含比特翻转、相位翻转、Pauli、去极化与两类阻尼。 |
| [`TwoQubitNoise`](4_noise.md) | 双比特噪声信道枚举，含去极化、独立组合与关联 Pauli。 |
| [`ReadoutError`](4_noise.md) | 读出误差，分别记录 `P(0\|1)` 与 `P(1\|0)`。 |
| [`OperationKey`](4_noise.md) | 噪声查询的键，由门与比特下标组合而成。 |

### 执行结果

| 名字 | 简介 |
| --- | --- |
| [`ExecutionResult`](5_result.md) | 任务执行结果，含计数、概率与时间戳。 |
| [`Status`](5_result.md) | 任务状态：`Queued`、`Running`、`Completed`、`Failed` 与 `Cancelled`。 |
| [`Outcome`](5_result.md) | 紧凑的测量结果比特串，支持按位访问与还原。 |

---

## 快速示例

### 1. 构建设备并配置属性

```rust
use std::collections::HashSet;

use cqlib_core::device::{Device, PhysicalQubit, QubitProp, Topology};

let device = Device::line("line", 3).unwrap();
assert_eq!(device.name(), "line");
assert_eq!(device.qubits().count(), 3);
assert_eq!(device.num_usable_qubits(), 3);
assert!(device
    .topology()
    .supports_directed_coupling(PhysicalQubit::new(0), PhysicalQubit::new(1)));
assert!(!device
    .topology()
    .supports_directed_coupling(PhysicalQubit::new(1), PhysicalQubit::new(0)));

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let topology = Topology::new(vec![q0, q1], vec![(q0, q1, "CX".to_string())]).unwrap();
let mut device = Device::new("demo".to_string(), HashSet::from_iter([q0, q1]), topology)
    .unwrap()
    .with_default_t1(40.0)
    .with_default_t2(20.0);

device
    .add_qubit_properties(q0, QubitProp::new(0.02).with_t1(60.0).with_t2(30.0))
    .unwrap();

assert_eq!(device.get_t1(q0), Some(60.0));
assert_eq!(device.get_readout_error(q0), Some(0.02));
```

### 2. 建立布局

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
let physical = vec![
    PhysicalQubit::new(100),
    PhysicalQubit::new(101),
    PhysicalQubit::new(102),
];

let layout = Layout::new(logical, physical, None).unwrap();

assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);
assert_eq!(layout.num_vacant_physical(), 1);
assert_eq!(
    layout.vacant_physical_qubits().collect::<Vec<_>>(),
    vec![PhysicalQubit::new(102)]
);
```

也可以直接由比特对构造：

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

let layout = Layout::from_pairs(&[(2, 3), (0, 1)], 5).unwrap();

assert_eq!(layout.num_vacant_physical(), 3);
assert_eq!(layout.get_physical(LogicalQubit::new(2)), Some(PhysicalQubit::new(3)));
assert_eq!(layout.get_physical(LogicalQubit::new(1)), None);
```

### 3. 由测量计数构造执行结果

```rust
use std::collections::HashMap;

use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, Status};

let zero = Outcome::from_indices(1, []);
let one = Outcome::from_indices(1, [0]);

let mut counts = HashMap::new();
counts.insert(zero.clone(), 3);
counts.insert(one.clone(), 1);

let result = ExecutionResult::from_counts(
    "task".to_string(),
    vec![Qubit::new(0)],
    4,
    1,
    Some("backend".to_string()),
    counts,
);

assert_eq!(result.status(), &Status::Completed);
assert_eq!(result.counts().get(&zero), Some(&3));

let probabilities = result.probabilities().as_ref().unwrap();
assert_eq!(probabilities.get(&zero), Some(&0.75));
```

---

## 校验与错误处理

设备模块的错误按领域分成几个枚举，模块根导出 `DeviceError`、`DeviceValidationError`、`TopologyError` 与 `LayoutError`；`NoiseError` 与 `OutcomeError` 位于各自的子模块中。

| 错误 | 触发场景 |
| --- | --- |
| `TopologyError::{QubitNotFound, CouplingNotFound, QubitAlreadyExists, CouplingAlreadyExists, SelfCoupling, DuplicateQubitRemoval, DuplicateCouplingRemoval}` | 拓扑构造与修改：操作未知或重复的比特与耦合、自耦合、重复删除。 |
| `DeviceError::{InvalidOnlineQubit, QubitNotInDevice, QubitNotInTopology, EdgeNotInTopology, InvalidTopology}` | 设备构造与属性写入时，比特或边不在设备或拓扑中，或底层拓扑本身非法。 |
| `DeviceError::{NonStandardNativeInstruction, InvalidNativeInstructionArity, InvalidNativeInstructionErrorRate, InvalidNativeInstructionDuration}` | 原生指令不合法：非标准门、元数超出允许范围（设备级至多两比特、比特级恰好一比特、边级恰好两比特），或误差率与时长取值非法。 |
| `DeviceValidationError::{UnusablePhysicalQubit, MissingDirectedCoupling, UnsupportedInstruction, UndecomposedInstruction}` | `validate_operation`、`validate_value_operation` 与 `validate_circuit` 校验失败：比特不可用、缺少有向耦合、指令不在原生门集内或指令尚未分解。 |
| `LayoutError::{TooManyLogicalQubits, DuplicateLogicalQubit, DuplicatePhysicalQubit, InvalidLogicalQubit, InvalidPhysicalQubit, LogicalQubitAlreadyBound, PhysicalQubitAlreadyOccupied, LogicalQubitNotBound}` | 布局构造与绑定操作：逻辑比特多于物理比特、重复或非法编号、目标位置已被占用、解绑未绑定的逻辑比特。 |
| `NoiseError::{InvalidProbability, QubitCollision, InconsistentArity, Internal}` | 噪声信道参数超出 `[0, 1]`、同一个门内比特重复、元数与门不符；路径为 `cqlib_core::device::noise::NoiseError`。 |
| `OutcomeError::InvalidCharacter` | `Outcome::from_bitstring` 的比特串中含非 `0` / `1` 字符；路径为 `cqlib_core::device::result::OutcomeError`。 |

设备级与比特/边级的原生能力写入都会先校验再落盘，被拒绝的更新不会改变对象原有状态。
