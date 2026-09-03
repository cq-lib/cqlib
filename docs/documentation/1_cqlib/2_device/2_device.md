# 设备属性建模

Device 模块负责汇总后端的全量硬件特征。它将抽象的物理拓扑（Topology）与具体的标定参数相结合，为噪声感知编译和高保真度仿真提供数据支撑。

Cqlib 采用**"全局默认 + 局部覆盖"**的标定策略：查询参数时优先返回局部值，未配置则回退至全局默认。

---

## 核心对象

| 对象 | 用途 |
|---|---|
| InstructionProp | 特定门指令的物理表现（误差率、执行时长），构造时需传入 Instruction 对象（通过 Instruction.from_standard_gate() 创建）|
| QubitProp | 单比特特性（T1/T2、读出误差、频率、本原门列表）|
| EdgeProp | 耦合边特性（原生双比特指令集）|
| Device | 顶层实体，整合拓扑与属性，提供全局默认值和查询接口 |

---

## 构建设备基准

```python
from cqlib.circuit import StandardGate
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("demo_backend", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_t2 = 35.0
device.default_readout_error = 0.05
device.default_single_qubit_error = 0.001
device.default_two_qubit_error = 0.01

print("设备名:", device.name)
print("寄存器比特数:", len(device.qubits))
```

**说明**：全局默认值仅用于后续回退查询。如果某个比特没有设置局部值，调用 get_t1(q) 时会返回 default_t1。

### 工厂方法

Device 提供了快速构造常见拓扑结构的工厂方法，省去手动创建 Topology 的步骤：

```python
from cqlib.device import Device

d1 = Device.line("line_dev", num_qubits=5)                    # 单向线型
d2 = Device.bidirectional_line("bi_line", num_qubits=5)      # 双向线型
d3 = Device.ring("ring_dev", num_qubits=4)                    # 双向环形
d4 = Device.star("star_dev", num_qubits=5, center=0)         # 双向星形
d5 = Device.grid("grid_dev", rows=3, cols=4)                  # 双向网格（行主序）
d6 = Device.from_edges("custom", num_qubits=4, edges=[(0, 1), (1, 2)])  # 自定义有向边

print("线型:", d1.num_usable_qubits)
print("双向线型:", d2.num_usable_qubits)
print("环形:", d3.num_usable_qubits)
print("星形:", d4.num_usable_qubits)
print("网格:", d5.num_usable_qubits)
print("自定义:", d6.num_usable_qubits)
```

---

## 注入局部标定数据

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, EdgeProp, InstructionProp, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

# ---- 单比特标定 ----
q0 = QubitProp(readout_error=0.02)
q0.t1 = 80.0              # T1 弛豫时间（微秒）
q0.t2 = 70.0              # T2 退相干时间（微秒）
q0.frequency = 5.1        # 频率（GHz）

# 设置测量判别误差
q0.prob_meas0_prep1 = 0.02  # P(测到 0 | 制备为 1)
q0.prob_meas1_prep0 = 0.01  # P(测到 1 | 制备为 0)

# 添加本原单比特门（注意：native_instructions 是列表属性，通过 append 添加）
x_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.X),
    error_rate=0.001,
)
x_prop.length = 20.0  # 门时长（纳秒）
q0.native_instructions.append(x_prop)

device.add_qubit_properties(0, q0)

# ---- 耦合边标定 ----
cx_prop = InstructionProp(
    Instruction.from_standard_gate(StandardGate.CX),
    error_rate=0.015,
)
cx_prop.length = 200.0

edge = EdgeProp()
edge.add_native_instruction(cx_prop)
device.add_edge_properties(0, 1, edge)

print("比特 0 局部属性已注入")
```

**注意**：
- InstructionProp 构造器的第一个参数必须是 Instruction 对象，使用 Instruction.from_standard_gate(StandardGate.X) 创建。不允许直接传入 StandardGate.X
- QubitProp.native_instructions 是只读列表属性，不能直接赋值（q0.native_instructions = [...] 会抛出 AttributeError），应通过 .append() 添加元素
- EdgeProp.native_instructions 也是只读属性，需通过 dd_native_instruction() 方法添加

---

## 参数查询与回退机制

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)
device.default_t1 = 50.0
device.default_single_qubit_error = 0.001

# 未设置局部值时回退至默认
print("比特 1 的 T1（回退默认）:", device.get_t1(1))

# 为比特 0 设置局部标定后，局部值优先
q0_prop = QubitProp(readout_error=0.02)
q0_prop.t1 = 80.0
device.add_qubit_properties(0, q0_prop)
print("比特 0 的 T1（局部值）:", device.get_t1(0))

# 查询单比特门误差率（注意：第三个参数需传入 Instruction 对象）
x_inst = Instruction.from_standard_gate(StandardGate.X)
print("比特 0 的 X 门误差（回退默认）:", device.single_qubit_error(0, x_inst))

# 查询读出误差（全局默认）
device.default_readout_error = 0.05
print("比特 0 的读出误差（回退默认）:", device.get_readout_error(0))
```

**回退链说明**：
- get_t1(q)：优先返回 QubitProp(q).t1，未设置则返回 device.default_t1
- get_readout_error(q)：优先返回 QubitProp(q).readout_error，未设置则返回 device.default_readout_error
- single_qubit_error(q, inst)：依次查找 → ① 本原门误差 ② 单比特默认误差 ③ 设备全局默认值
- 如果比特不可用（未注册或标记为无效），以上查询均返回 None

---

## 无效比特管理

```python
from cqlib.device import Device, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

# 标记比特 2 为无效（离线/故障）
device.invalid_qubits = [2]   # 注意：使用列表，不是集合

print("可用比特数:", device.num_usable_qubits)  # 2
print("可用比特列表:", device.usable_qubits)     # [Qubit(0), Qubit(1)]
print("比特 2 是否可用:", device.is_usable_qubit(2))  # False
```

---

## 健壮性校验

```python
from cqlib.device import Device, QubitProp, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("dev", [0, 1, 2], topo)

try:
    device.add_qubit_properties(99, QubitProp(0.01))
except ValueError as e:
    print("添加不存在的比特属性:", e)

try:
    device.add_edge_properties(0, 9, EdgeProp())
except ValueError as e:
    print("添加不存在的边属性:", e)
```

---

## 下一步

- [布局映射](3_layout.md)：学习 Layout 的逻辑-物理比特双向映射与 SWAP 路由操作
- [噪声模型](4_noise.md)：了解 NoiseModel、SingleQubitNoise、TwoQubitNoise、ReadoutError 等噪声信道的用法
- [执行结果与状态](5_result.md)：熟悉 Outcome、Status、ExecutionResult 的完整生命周期及错误处理
