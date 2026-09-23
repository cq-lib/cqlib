# 设备

`cqlib.device`

`cqlib.device` 描述量子设备的静态信息与任务执行状态：比特与耦合构成的拓扑、比特与边的校准属性、逻辑比特到物理比特的布局、噪声模型，以及执行结果与状态。

## Overview

设备模块把硬件相关的数据拆成几组互相独立的对象：拓扑只描述连接关系，属性只描述标定数据，布局只描述映射，噪声模型只描述信道。编译与执行各自按需取用，互不耦合。

### 拓扑

`Topology` 由物理比特集合与耦合列表构成。耦合是有向的：`(control, target)` 上有耦合，不代表反方向也有，因此连接性查询分成 `successors` / `predecessors`（有向）与 `neighbors_undirected`（无向）两类，度数查询也分成 `out_degree` / `in_degree`。耦合可以带一个名字标签，仅作标识之用。

### 设备与属性

`Device` 在拓扑之上叠加设备级默认值与局部覆盖，并记录设备名、已注册比特、不可用比特、原生门集与校准时间。比特级属性由 `QubitProp` 描述（`t1`、`t2`、`frequency`、读出误差与读出混淆概率），边级属性由 `EdgeProp` 描述，两者都持有一组 `InstructionProp`（指令、误差率与可选的时长）。

查询某个比特的属性时，局部值优先，未设置则回退到设备级默认值：`device.get_t1(0)` 取局部值，`device.get_t1(2)` 取 `default_t1`。设备还提供 `line`、`ring`、`star`、`grid` 等静态构造方法，按常见形态直接生成拓扑与比特集合。

### 原生指令能力

设备级 `native_gates` 是全局默认能力；某个比特或某条有向边若通过 `add_native_instruction` 配置了自己的指令列表，该列表就是此处能力的完整覆盖，不会回写默认值。校准数据只让受支持的指令带上误差率与时长，不会让不受支持的指令变为受支持。`validate_operation` / `validate_circuit` 用同一套规则校验线路是否能在该设备上执行。

### 布局

`Layout` 维护逻辑比特到物理比特的映射，未承载逻辑比特的物理比特是空闲物理比特。映射可以按 `l2p_map` / `p2l_map` 整体读取，也可以按单个比特查询；`swap_physical` 交换两个物理位置上承载的逻辑比特，这是路由过程中移动逻辑比特的基本操作。

### 噪声模型

`NoiseModel` 按比特与操作登记噪声信道：读出误差用 `ReadoutError` 表示，单比特信道由 `SingleQubitNoise` 的静态方法生成（比特翻转、相位翻转、Pauli、去极化、振幅阻尼、相位阻尼），双比特信道由 `TwoQubitNoise` 生成（去极化、独立信道组合、关联 Pauli）。信道可以用 `to_kraus` 转成 Kraus 算符，供密度矩阵模拟使用。查询时用 `OperationKey` 指定门与比特组合。

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, ReadoutError, TwoQubitNoise

model = NoiseModel()
model.add_readout_error(0, ReadoutError(0.1, 0.2))
model.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))
```

### 执行结果

`ExecutionResult` 记录一次任务的生命周期：创建后处于 `Status.queued()`，`start` 后进入运行态，`finish` / `fail` / `cancel` 结束任务。测量结果以 `Outcome` 表示，它把比特串压缩成 64 位分块，同时支持按位置取位与还原比特串；`counts` 与 `probabilities` 分别是各结果的测量次数与归一化概率。

---

## 常用入口

```python
from cqlib import Qubit
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])
device = Device("mock_backend", [Qubit(0), Qubit(1), Qubit(2)], topo)
device.default_t1 = 50.0
device.native_gates = [Instruction.from_standard_gate(StandardGate.X)]

qp0 = QubitProp(0.02)
qp0.t1 = 80.0
device.add_qubit_properties(0, qp0)

assert device.get_t1(0) == 80.0  # 局部属性
assert device.get_t1(2) == 50.0  # 回退到设备级默认值
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **物理比特** | 设备上的一个物理位置，用 `PhysicalQubit` 表示。 |
| **逻辑比特** | 线路中的一根线，用 `LogicalQubit` 表示；即使编号相同，也与物理比特互相区分。 |
| **拓扑** | 物理比特与它们之间耦合构成的图，由 `Topology` 表示。 |
| **耦合** | 两个物理比特之间可以执行双比特门的连接关系。 |
| **有向耦合** | 带方向的耦合：`(control, target)` 上存在耦合不代表反方向也存在。 |
| **原生指令** | 设备上可以直接执行的指令；设备级集合是默认能力，比特与边可以各自覆盖。 |
| **局部覆盖** | 比特或边的原生指令列表，一旦非空就完整取代设备级默认能力，不回写默认值。 |
| **默认值回退** | 查询比特属性时，该比特没有局部值就返回设备级默认值。 |
| **初始布局** | 逻辑比特到物理比特的初始映射，由 `Layout` 表示。 |
| **空闲物理比特** | 当前没有承载逻辑比特的物理比特。 |
| **噪声信道** | 描述一种噪声作用的算符集合，可用 `to_kraus` 转为 Kraus 算符。 |
| **读出误差** | 测量时把 `\|0>` 读成 1 或把 `\|1>` 读成 0 的概率。 |
| **测量结果** | 一次采样得到的比特串，用 `Outcome` 表示。 |

---

## `cqlib.device` API 概览

### 拓扑

| 名字 | 简介 |
| --- | --- |
| [`Topology`](1_topology.md) | 物理比特与耦合列表构成的有向图，含增删与连接性查询。 |
| [`Topology.line`](1_topology.md) | 按直线形态构造拓扑的静态方法。 |

### 设备与属性

| 名字 | 简介 |
| --- | --- |
| [`Device`](2_properties_device.md) | 设备对象：拓扑、比特集合、原生门集、校准时间与默认属性。 |
| [`Device.line`](2_properties_device.md) / [`Device.ring`](2_properties_device.md) / [`Device.star`](2_properties_device.md) / [`Device.grid`](2_properties_device.md) | 按常见形态构建设备的静态方法。 |
| [`QubitProp`](2_properties_device.md) | 比特属性：读取误差、混淆概率、`t1` / `t2`、频率与原生指令属性。 |
| [`EdgeProp`](2_properties_device.md) | 边属性：该有向边上的原生指令属性。 |
| [`InstructionProp`](2_properties_device.md) | 指令属性：指令对象、误差率与可选时长。 |
| [`Device.validate_circuit`](2_properties_device.md) / [`Device.validate_operation`](2_properties_device.md) | 按原生能力与拓扑校验线路或单个操作。 |

### 布局

| 名字 | 简介 |
| --- | --- |
| [`Layout`](3_layout.md) | 逻辑比特到物理比特的映射，含绑定、解绑与物理比特交换。 |
| [`Layout.from_pairs`](3_layout.md) | 由 `(逻辑编号, 物理编号)` 对与物理比特总数构造布局。 |
| [`LogicalQubit`](3_layout.md) / [`PhysicalQubit`](3_layout.md) | 逻辑比特与物理比特的类型，字符串形式分别为 `L0` 与 `P11`。 |

### 噪声模型

| 名字 | 简介 |
| --- | --- |
| [`NoiseModel`](4_noise.md) | 按比特与操作登记噪声信道的容器。 |
| [`SingleQubitNoise`](4_noise.md) | 单比特噪声信道，含比特翻转、相位翻转、Pauli、去极化与两类阻尼。 |
| [`TwoQubitNoise`](4_noise.md) | 双比特噪声信道，含去极化、独立组合与关联 Pauli。 |
| [`ReadoutError`](4_noise.md) | 读出误差，分别记录 `P(0\|1)` 与 `P(1\|0)`。 |
| [`OperationKey`](4_noise.md) | 噪声查询的键，由门与比特下标组合而成。 |

### 执行结果

| 名字 | 简介 |
| --- | --- |
| [`ExecutionResult`](5_result.md) | 任务执行结果，含计数、概率与时间戳。 |
| [`Status`](5_result.md) | 任务状态：排队、运行、完成、失败与取消。 |
| [`Outcome`](5_result.md) | 紧凑的测量结果比特串，支持按位访问与还原。 |

---

## 快速示例

### 1. 构建设备并配置属性

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

qp0 = QubitProp(0.02)
qp0.t1 = 80.0
qp0.t2 = 70.0
device.add_qubit_properties(0, qp0)

ep01 = EdgeProp()
cx_inst = Instruction.from_standard_gate(StandardGate.CX)
ep01.add_native_instruction(InstructionProp(cx_inst, 0.06))
device.add_edge_properties(0, 1, ep01)
```

写入不存在的比特或边会被拒绝：

```python
import pytest

with pytest.raises(ValueError):
    device.add_qubit_properties(9, QubitProp(0.01))

with pytest.raises(ValueError):
    device.add_edge_properties(0, 9, EdgeProp())
```

### 2. 建立布局并交换物理比特

```python
from cqlib import Qubit
from cqlib.device import Layout, PhysicalQubit

layout = Layout(
    logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)}
)
assert layout.num_logical == 2
assert layout.num_physical == 3
assert layout.num_vacant_physical == 1
assert layout.get_physical(0) == PhysicalQubit(11)

on_11 = layout.get_logical(11)
on_12 = layout.get_logical(12)

layout.swap_physical(11, 12)
assert layout.get_logical(11) == on_12
assert layout.get_logical(12) == on_11
```

映射表可以整体读取：

```python
from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit

layout = Layout(logical=[0], physical=[10, 11], init_map={Qubit(0): Qubit(11)})
assert layout.l2p_map == {LogicalQubit(0): PhysicalQubit(11)}
assert layout.p2l_map == {PhysicalQubit(11): LogicalQubit(0)}
```

### 3. 记录一次执行结果

```python
from cqlib import Qubit
from cqlib.device import ExecutionResult

result = ExecutionResult(
    task_id="task-1",
    qubits=[Qubit(0), Qubit(1)],
    shots=100,
    num_qubits=2,
    backend="sim",
)
assert result.status.kind == "queued"

result.start()
assert result.status.kind == "running"

result.finish({"00": 60, "11": 40})
assert result.status.kind == "completed"
assert result.counts["00"] == 60

result.calc_probabilities()
assert result.probabilities["00"] == 0.6
```

---

## 校验与错误处理

设备绑定层的失败统一抛 `ValueError`，没有自定义异常类型；异常消息直接来自底层的错误描述。

| 异常 | 触发场景 |
| --- | --- |
| `ValueError` | 比特或边不在设备/拓扑中，例如向不存在的比特添加属性、向不存在的方向添加边属性。 |
| `ValueError` | 拓扑与布局的构造或修改冲突：重复比特、重复耦合、自耦合、逻辑比特重复或已绑定、物理比特已被占用等。 |
| `ValueError` | 概率类参数不合法：误差率、读出误差、噪声信道参数超出 `[0, 1]` 或不是有限值；Pauli 信道的三个概率之和超过 1。 |
| `ValueError` | 时间类参数不合法：`t1` / `t2` / `frequency` 必须为有限正数。 |
| `ValueError` | 原生指令不合法：设备级原生门只接受至多两比特的标准门，比特级与边级分别只接受单比特与双比特标准门；被拒绝时设备保留原有能力。 |
| `ValueError` | `validate_operation` / `validate_circuit` 校验失败：比特不可用、缺少有向耦合、指令不在原生门集内或指令尚未分解。 |
| `ValueError` | 测量数据不合法：`Outcome` 的比特串含非 `0` / `1` 字符；`finish` 或 `from_counts` 的键长度不等于 `num_qubits`，或不同键折叠到同一个结果。 |
| `ValueError` | `set_calibration_time` 传入的时间超出可表示范围。 |
| `TypeError` / `OverflowError` | 参数类型或取值域不符，例如比特编号传入非整数或负数。 |

概率类参数在写入时即校验，因此非法值不会落到设备或噪声模型对象上。
