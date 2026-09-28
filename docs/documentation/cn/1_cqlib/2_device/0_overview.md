# 设备模块概述

cqlib.device 是 Cqlib 中用于描述量子后端硬件能力、噪声特性与执行结果的数据模块，位于 cqlib.device 包下。

在量子计算工作流中，Circuit 表达的是"算法怎么做"，而 device 表达的是"硬件允许怎么做"。它为以下关键工程环节提供统一的底层支撑：

- **物理约束感知**：定义硬件拓扑（哪些比特支持双比特门耦合及其方向限制）
- **高保真度建模**：精细化管理比特相干时间（T1/T2）、读出误差及门保真度
- **动态布局追踪**：在编译路由阶段实时维护逻辑比特与物理比特的映射
- **噪声仿真**：构建可用于量子噪声仿真的信道模型，支持 Kraus 算符表示
- **任务全生命周期管理**：追踪任务从提交、入队、运行到结果回传的完整闭环

---

## 核心对象一览

| 对象 | 用途 | 关键能力 |
|---|---|---|
| Topology | 硬件拓扑图 | 有向耦合建模、动态增删节点/边、邻接查询、line 工厂方法 |
| Device | 完整硬件描述 | 全局默认 + 局部覆盖的标定参数、工厂方法快速构造（line/bidirectional_line/ring/star/grid/from_edges）|
| QubitProp / EdgeProp / InstructionProp | 标定属性容器 | 存储比特/边的 T1/T2、读出误差、门保真度与时长 |
| Layout | 逻辑-物理映射 | 双向映射查询、SWAP 路由操作、动态绑定/解绑 |
| NoiseModel / SingleQubitNoise / TwoQubitNoise / ReadoutError | 噪声信道 | 比特翻转、相位翻转、去极化、振幅/相位阻尼等，支持 Kraus 算符导出 |
| Outcome / Status / ExecutionResult | 执行结果 | 测量结果封装、任务状态机（queued/running/completed/failed/cancelled）、概率计算 |

---

## 快速示例：从拓扑到结果

以下示例演示了 device 模块的完整工作流，涵盖拓扑定义、设备创建、标定注入、布局映射、噪声配置和结果管理。

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import (
    Device, EdgeProp, ExecutionResult, InstructionProp, Layout,
    NoiseModel, OperationKey, QubitProp, ReadoutError,
    SingleQubitNoise, Topology, TwoQubitNoise,
)

# 1) 定义硬件拓扑：比特列表 + (控制, 目标, 门名称) 三元组
topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# 2) 创建设备，设置全局默认标定参数
device = Device("demo_backend", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.default_single_qubit_error = 0.001
device.default_two_qubit_error = 0.01

# 3) 注入局部标定参数（会覆盖全局默认值）
q0_prop = QubitProp(readout_error=0.02)
q0_prop.t1 = 80.0
q0_prop.t2 = 70.0
device.add_qubit_properties(0, q0_prop)

cx_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.CX),
    error_rate=0.015,
)
cx_prop.length = 220.0
edge_prop = EdgeProp()
edge_prop.add_native_instruction(cx_prop)
device.add_edge_properties(0, 1, edge_prop)

# 4) 布局映射：逻辑比特 → 物理比特
layout = Layout.from_pairs([(0, 11), (1, 10)], physical_count=13)
layout.swap_physical(11, 12)

# 5) 噪声模型配置
noise = NoiseModel()
noise.add_readout_error(0, ReadoutError(0.02, 0.01))
noise.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.005))
noise.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))

# 6) 任务执行结果
result = ExecutionResult("task-1", [0, 1], 100, 2, "demo_backend")
result.start()
result.finish({"00": 60, "11": 40})
result.calc_probabilities()

# 7) 查询与验证输出
print("设备名:", device.name)
print("比特 0 的 T1（局部值）:", device.get_t1(0))
print("比特 2 的 T1（回退至全局默认）:", device.get_t1(2))
print("可用比特数:", device.num_usable_qubits)
print("逻辑→物理映射:", layout.l2p_map)
print("比特 0 读出误差:", noise.get_readout_error(0))
print("任务状态:", result.status.kind)
print("概率分布:", result.probabilities)

# 通过 OperationKey 查询噪声通道
skey = OperationKey.new_single(StandardGate.X, 0)
qubit_noises = noise.get_single_qubit_errors(skey)
if qubit_noises:
    print("X 门噪声通道数:", len(qubit_noises))
```

**说明**：

- get_t1()、get_t2()、get_readout_error() 遵循**局部值优先，无局部值则回退全局默认**的查询策略
- l2p_map 返回 LogicalQubit → PhysicalQubit 的映射字典
- probabilities 由 calc_probabilities() 将原始计数归一化得到，结果为 dict[str, float]
- NoiseModel.add_*() 方法均返回 None，参数校验不通过时抛出 ValueError
- 涉及指令参数的方法（如 Device.single_qubit_error）需传入 Instruction 对象（通过 Instruction.from_standard_gate() 构造），而非 StandardGate 枚举值
- Layout 的 init_map 参数要求 dict[Qubit, Qubit]，因此直接传入 {0: 11} 会报类型错误。推荐使用 Layout.from_pairs() 作为替代方案

## 下一步

- [拓扑建模](1_topology.md)：理解 Topology 的有向图模型及其实例化、查询与动态修改操作
- [设备属性建模](2_device.md)：掌握 Device 的全局默认 + 局部覆盖标定策略
- [布局映射](3_layout.md)：学习 Layout 的逻辑-物理比特双向映射与 SWAP 路由操作
- [噪声模型](4_noise.md)：了解 NoiseModel、SingleQubitNoise、TwoQubitNoise、ReadoutError 等噪声信道的用法
- [执行结果与状态](5_result.md)：熟悉 Outcome、Status、ExecutionResult 的完整生命周期及错误处理

