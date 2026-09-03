# 噪声模型

cqlib.device 提供了从单比特噪声、双比特噪声到整体噪声容器的完整建模接口，用于量子噪声仿真中的信道建模。

---

## 单比特噪声

SingleQubitNoise 提供多种静态工厂方法创建不同类型的噪声信道：

| 工厂方法 | 信道 | 含义 |
|---|---|---|
| it_flip(p) | 比特翻转 | 以概率 p 执行 X 门 |
| phase_flip(p) | 相位翻转 | 以概率 p 执行 Z 门 |
| pauli(px, py, pz) | 一般泡利 | 独立指定 X/Y/Z 错误概率 |
| depolarizing(p) | 去极化 | 以概率 p 随机施加 X/Y/Z 错误 |
| mplitude_damping(gamma) | 振幅阻尼 | T1 能量弛豫模型 |
| phase_damping(lambda_) | 相位阻尼 | T2 纯退相干模型 |

```python
from cqlib.device import SingleQubitNoise

sq = SingleQubitNoise.depolarizing(0.01)
pauli = SingleQubitNoise.pauli(px=0.001, py=0.0005, pz=0.002)
bitflip = SingleQubitNoise.bit_flip(0.01)
amp = SingleQubitNoise.amplitude_damping(gamma=0.05)
phase = SingleQubitNoise.phase_damping(0.1)

# Kraus 算符导出
kraus = sq.to_kraus()
print("Kraus 算符数:", len(kraus))          # 4
print("Kraus 矩阵形状:", kraus[0].shape)    # (2, 2)

# 有效性校验
print("去极化噪声有效:", sq.is_valid())       # True
print("振幅阻尼有效:", amp.is_valid())        # True
```

**注意**：
- SingleQubitNoise 没有 .kind 属性。要区分噪声类型，需自行通过构造方法跟踪
- pauli(px, py, pz) 要求 px + py + pz <= 1，否则 is_valid() 返回 False

---

## 双比特噪声

```python
from cqlib.device import SingleQubitNoise, TwoQubitNoise
from cqlib.qis import Pauli

# 去极化噪声（15 个非单位 Pauli 算符等概率）
tq = TwoQubitNoise.depolarizing(0.01)
print("去极化 Kraus 形状:", tq.to_kraus()[0].shape)  # (4, 4)

# 独立噪声（每个比特各自独立施加单比特噪声）
ind = TwoQubitNoise.independent(
    SingleQubitNoise.phase_flip(0.02),
    SingleQubitNoise.bit_flip(0.03),
)
print("独立噪声 Kraus 形状:", ind.to_kraus()[0].shape)

# 关联 Pauli 噪声
corr = TwoQubitNoise.correlated_pauli(Pauli.x(), Pauli.x(), p=0.01)
print("关联 Pauli Kraus 形状:", corr.to_kraus()[0].shape)
```

**Pauli 构造函数说明**（均为静态方法，返回 Pauli 对象）：

| 正确用法 | 错误用法 |
|---|---|
| Pauli.x() | Pauli.X |
| Pauli.y() | Pauli.Y |
| Pauli.z() | Pauli.Z |
| Pauli.i() | Pauli.I |

---

## 读出误差

ReadoutError 描述测量过程中的判别错误：

```python
from cqlib.device import ReadoutError

ro = ReadoutError(p_0_given_1=0.02, p_1_given_0=0.01)
print("P(测到 0 | 制备为 1):", ro.p_0_given_1)  # 0.02
print("P(测到 1 | 制备为 0):", ro.p_1_given_0)  # 0.01
print("读出误差是否有效:", ro.is_valid())          # True
```

---

## NoiseModel：噪声容器

NoiseModel 聚合所有噪声源，支持按比特/门类型进行增删查：

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, OperationKey, ReadoutError, SingleQubitNoise, TwoQubitNoise

nm = NoiseModel()

# 添加噪声（所有 add_* 方法返回 None，参数校验不通过时抛出 ValueError）
nm.add_readout_error(0, ReadoutError(0.02, 0.01))
nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(0.005))
nm.add_two_qubit_error(StandardGate.CX, 0, 1, TwoQubitNoise.depolarizing(0.02))

# 读出误差查询
ro = nm.get_readout_error(0)
print("读出误差:", ro)

# 单比特噪声查询（通过 OperationKey）
skey = OperationKey.new_single(StandardGate.X, 0)
errs = nm.get_single_qubit_errors(skey)
print("X 门噪声通道数:", len(errs))

# 双比特噪声查询
tkey = OperationKey.new_double(StandardGate.CX, 0, 1)
errs2 = nm.get_two_qubit_errors(tkey)
print("CX 门噪声通道数:", len(errs2))
```

**说明**：
- dd_single_qubit_error(gate, qubit, noise) 的 gate 参数传入 StandardGate 枚举值（如 StandardGate.X），而非 Instruction 对象
- dd_two_qubit_error(gate, q0, q1, noise) 同样使用 StandardGate 枚举值
- OperationKey.new_single(gate, q0) 和 
ew_double(gate, q0, q1) 使用 StandardGate
- get_single_qubit_errors(key) 返回 list[SingleQubitNoise] | None，无匹配时返回 None

---

## 参数校验

NoiseModel.add_*() 和噪声构造器会在参数不合法时抛出 ValueError：

```python
from cqlib.circuit import StandardGate
from cqlib.device import NoiseModel, SingleQubitNoise, TwoQubitNoise

nm = NoiseModel()

try:
    # 概率超出 [0, 1] 范围
    nm.add_single_qubit_error(StandardGate.X, 0, SingleQubitNoise.bit_flip(1.5))
except ValueError as e:
    print("无效概率被拦截:", e)

try:
    # 双比特门作用在同一比特上
    nm.add_two_qubit_error(StandardGate.CX, 0, 0, TwoQubitNoise.depolarizing(0.01))
except ValueError as e:
    print("无效配置被拦截:", e)
```

---

## 噪声与设备结合

在实际使用中，Device 提供门误差率查询接口，而 NoiseModel 提供详细的信道模型。两者可以配合使用：

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.device import Device, NoiseModel, OperationKey, SingleQubitNoise, Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])
device = Device("noisy", [0, 1, 2], topo)
device.default_single_qubit_error = 0.001

noise = NoiseModel()
noise.add_single_qubit_error(StandardGate.H, 0, SingleQubitNoise.depolarizing(0.002))

# Device 查询（注意需要 Instruction 对象）
h_inst = Instruction.from_standard_gate(StandardGate.H)
print("设备报告的 H 门误差:", device.single_qubit_error(0, h_inst))

# NoiseModel 查询噪声通道
skey = OperationKey.new_single(StandardGate.H, 0)
channels = noise.get_single_qubit_errors(skey)
print("噪声通道数:", len(channels))
```

---

## 下一步

- [执行结果与状态](5_result.md)：熟悉 Outcome、Status、ExecutionResult 的完整生命周期及错误处理
- [量子信息](../3_qis/0_overview.md)：掌握 Statevector、DensityMatrix、Pauli 等基础
- [编译优化](../4_compiler/0_overview.md)：了解编译管线的布局、路由与优化
