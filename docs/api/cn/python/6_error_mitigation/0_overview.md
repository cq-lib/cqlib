# 错误缓解

`cqlib.error_mitigation`

`cqlib.error_mitigation` 提供观测量期望值层面的错误缓解能力，覆盖零噪声外推与虚拟蒸馏两种方法。模块同时给出低层的单方法辅助类与统一流水线：前者把折叠、执行、外推等步骤逐项暴露，后者把两种方法收敛到同一套顺序式流程。

## Overview

错误缓解处理的是期望值的系统性偏差：线路仍然按原语义执行，缓解手段通过额外构造的线路与后处理把噪声对期望值的影响外推或抵消掉。它与量子纠错的关注点不同——不引入编码与综合征测量，代价是额外的线路执行次数。

模块内的全部方法都依赖同一约定：调用方提供一个可调用对象估计某条线路在某个观测量上的结果，模块负责生成该执行的线路族、收集返回值并做后处理。

### 三种入口的分工

| 入口 | 覆盖范围 | 适用场景 |
| --- | --- | --- |
| `ZNEMitigation` | 零噪声外推 | 需要单独控制折叠等级、选择性折叠或外推方法。 |
| `VirtualDistillation` | 虚拟蒸馏 | 需要单独控制拷贝数，或分步执行分子与分母线路。 |
| `ErrorMitigation` | 两者 | 以统一流程完成一次完整的缓解。 |

低层类与统一流水线共享同一套配置对象与回调约定：`ZneConfig`、`VirtualDistillationConfig` 既可用于构造 `MitigationMethod`，其字段含义也与低层类保持一致；`Estimator` 在三条入口上的调用形态相同。两者不共享状态，同一组配置在低层类与统一流水线中各自独立生效。

统一流水线按方法选择执行路径：

- `MitigationMethod.zne(config)` 先按 `ZneConfig.fold_levels` 折叠线路，逐条调用回调收集期望值，再由 `get_mitigated()` 做外推。
- `MitigationMethod.virtual_distillation(config)` 先按 `VirtualDistillationConfig.copies` 构造 copy-swap 线路，在同一条线路上分别以分子与分母两种观测量调用回调，再由 `get_mitigated()` 求比值。

### 缓解线路族

两种方法都不直接执行输入线路，而是先构造一族缓解线路：

- 零噪声外推按折叠等级展开线路。全局折叠把线路改写为 `U -> U (U† U)^level`，使线路深度随等级增长；选择性折叠只对指令名匹配给定门集的操作执行同样的展开，其余操作原样保留。等级为 `0` 时缓解线路即输入线路的副本。
- 虚拟蒸馏把线路复制 `copies` 份并按位插入 SWAP，得到一条宽度为 `copies` 倍的 copy-swap 线路。分子与分母在同一线路上执行，区别只在传给回调的观测量：分母为 `None`，分子为按拷贝数扩展后的 `Hamiltonian`。

### 数据流

```text
输入线路 + 观测量
  ↓
构造缓解线路族（折叠线路 / copy-swap 线路）
  ↓
回调逐条估计，返回 (期望值, 方差)
  ↓
后处理：外推到零噪声点 / 分子分母求比值
  ↓
MitigatedResult(expectation, variance)
```

两种后处理对 `variance` 的处理不同：零噪声外推只使用期望值序列，结果的 `variance` 为 `None`；虚拟蒸馏同时使用期望值与方差，结果的 `variance` 有值。

---

## 常用入口

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import (
    ErrorMitigation,
    ExtrapolateMethod,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
    ZneConfig,
)
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    # 替换为后端或模拟器的期望值估计
    return (len(run_circuit.operations) + 0.5, 0.0)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.zne(ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, RunArgs.zne(shots=256), estimator)
result = mitigation.get_mitigated(
    ProcessArgs.zne(ExtrapolateMethod.polynomial(), degree=1)
)

print("expectation:", result.expectation)
print("variance:", result.variance)
```

调用顺序由对象自身约束：每个 `ErrorMitigation` 实例只能 `run()` 一次、`get_mitigated()` 一次，且必须先 `run()` 再 `get_mitigated()`。

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **错误缓解** | 在期望值层面降低观测量系统性偏差的处理，不改变线路的酉语义，也不等同于量子纠错。 |
| **折叠等级** | 零噪声外推中控制线路展开次数的非负整数，由 `fold_levels` 给出。 |
| **噪声因子** | 由折叠等级导出的噪声放大倍数，满足 `noise_factor = 2 * fold_level + 1`。 |
| **全局折叠** | `gate_set` 为 `None` 时对整条线路做 `U -> U (U† U)^level` 的展开。 |
| **选择性折叠** | 只对指令名匹配 `gate_set` 的操作做折叠，其余操作原样保留。 |
| **外推方法** | 把不同噪声因子下的期望值外推到零噪声点的拟合模型，包括多项式与指数两种。 |
| **拷贝数** | 虚拟蒸馏中密度矩阵的拷贝份数，由 `copies` 给出，最小为 2。 |
| **copy-swap 线路** | 由输入线路复制 `copies` 份并按位插入 SWAP 得到的缓解线路。 |
| **分子线路** | 用于估计 `Tr(O ρ^M)` 的 copy-swap 线路执行，观测量为按拷贝数扩展后的 `Hamiltonian`。 |
| **分母线路** | 用于估计 `Tr(ρ^M)` 的 copy-swap 线路执行，观测量为 `None`。 |
| **estimator** | 调用方提供的回调，执行一条缓解线路并返回 `(期望值, 方差)` 二元组。 |
| **统一流水线** | 由 `ErrorMitigation` 承载的顺序式流程：`run()` 收集估计值，`get_mitigated()` 做后处理。 |

---

## `cqlib.error_mitigation` API 概览

### 零噪声外推

| 名字 | 简介 |
| --- | --- |
| [`ZNEMitigation`](1_zne.md) | 低层零噪声外推辅助类，负责折叠、执行与外推。 |
| [`ZneConfig`](1_zne.md) | 零噪声外推配置，包含折叠等级。 |
| [`ExtrapolateMethod`](1_zne.md) | 外推方法，包括 `polynomial` 与 `exponential`。 |
| [`Estimator`](1_zne.md) | 回调类型别名，约定三参数与二元组返回。 |

### 虚拟蒸馏

| 名字 | 简介 |
| --- | --- |
| [`VirtualDistillation`](2_virtual_distillation.md) | 低层虚拟蒸馏辅助类，负责构造 copy-swap 线路与求比值。 |
| [`VirtualDistillationConfig`](2_virtual_distillation.md) | 虚拟蒸馏配置，包含拷贝数。 |

### 统一流水线

| 名字 | 简介 |
| --- | --- |
| [`ErrorMitigation`](3_unified.md) | 统一流水线，按 `run()` → `get_mitigated()` 顺序完成一次缓解。 |
| [`MitigationMethod`](3_unified.md) | 缓解方法选择，包括 `zne` 与 `virtual_distillation`。 |
| [`RunArgs`](3_unified.md) | 执行参数，与所选方法一一对应。 |
| [`ProcessArgs`](3_unified.md) | 后处理参数，与所选方法一一对应。 |
| [`MitigatedResult`](3_unified.md) | 缓解结果，包含期望值与可选的方差。 |
| [`ErrorMitigationError`](3_unified.md) | 错误缓解异常的基类。 |

---

## 快速示例

### 1. 低层零噪声外推

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import ExtrapolateMethod, ZNEMitigation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

zne = ZNEMitigation(circuit, [0, 1, 2])
print(zne.fold_levels)    # [0, 1, 2]
print(zne.noise_factors)  # [1, 3, 5]

folded = zne.fold_circuits()
print([len(item.operations) for item in folded])  # [1, 3, 5]


def estimator(run_circuit, observable, shots):
    return (0.5 * len(run_circuit.operations), 0.0)


noisy = zne.run_em_sequence_with_shots(None, hamiltonian, 256, estimator)
print(noisy)  # [0.5, 1.5, 2.5]

mitigated = zne.extrapolate(noisy, ExtrapolateMethod.polynomial(), 1)
print(mitigated)  # 0.0
```

### 2. 低层虚拟蒸馏

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import VirtualDistillation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = VirtualDistillation(circuit, 2)
copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 2


def estimator(run_circuit, observable, shots):
    if observable is None:
        return (2.0, 1.0)   # 分母：Tr(ρ^M)
    return (1.5, 0.25)      # 分子：Tr(O ρ^M)


expectation, variance = vd.run_vd(
    hamiltonian,
    shots_numerator=3,
    shots_denominator=2,
    estimator=estimator,
)
print(expectation)  # 0.75
print(variance)     # 0.203125
```

### 3. 统一流水线

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation import (
    ErrorMitigation,
    MitigationMethod,
    VirtualDistillationConfig,
    ProcessArgs,
    RunArgs,
)
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    if observable is None:
        return (2.0, 1.0)
    return (1.5, 0.25)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.virtual_distillation(VirtualDistillationConfig(2)),
)

mitigation.run(
    hamiltonian,
    RunArgs.virtual_distillation(shots_numerator=3, shots_denominator=2),
    estimator,
)
result = mitigation.get_mitigated(ProcessArgs.virtual_distillation())

print(result.expectation)  # 0.75
print(result.variance)     # 0.203125
```

---

## 校验与错误处理

模块中的校验分两类：构造期的配置校验（折叠等级、拷贝数）与执行期的一致性校验（观测量比特数、流水线调用顺序、参数与方法是否匹配）。

| 异常 | 触发场景 |
| --- | --- |
| `ErrorMitigationError` | 配置或调用顺序非法，例如折叠等级为负、拷贝数小于 2、观测量比特数与线路不符、未 `run()` 就调用 `get_mitigated()`、重复 `run()` 或 `get_mitigated()`、`RunArgs` / `ProcessArgs` 与实际方法不匹配，以及外推输入非法（见 [零噪声外推](1_zne.md)）。 |
| `CircuitError` | 缓解线路构造失败，例如在 `ZNEMitigation` 上以负折叠等级调用 `fold_circuits()`。 |
| `ValueError` | 观测量相关的运算失败，例如按拷贝数扩展 `Hamiltonian` 时出错。 |
| `TypeError` | 传入的 `estimator` 不可调用。 |

`ErrorMitigationError` 与 `CircuitError` 同为 `cqlib.circuit.CqlibError` 的子类。回调内部抛出的异常不会被替换，会在缓解流程返回后原样重新抛出。
