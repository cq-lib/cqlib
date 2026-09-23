# 虚拟蒸馏

`cqlib.error_mitigation.virtual_distillation`

`cqlib.error_mitigation.virtual_distillation` 提供虚拟蒸馏的低层接口：把基础线路复制若干份并插入 SWAP 构造 copy-swap 线路，再分别估计分子与分母，最后求比值得到缓解后的期望值。本页覆盖 `VirtualDistillation` 与 `VirtualDistillationConfig`。

## 导入

```python
from cqlib.error_mitigation.virtual_distillation import (
    Estimator,
    VirtualDistillationConfig,
    VirtualDistillation,
)
```

`VirtualDistillation` 与 `VirtualDistillationConfig` 同时由 `cqlib.error_mitigation` 重导出；回调约定见 [零噪声外推](1_zne.md) 中的 `Estimator`。

虚拟蒸馏估计的比值是 `Tr(O ρ^M) / Tr(ρ^M)`，其中 `M` 为拷贝数，`O` 为 `Hamiltonian` 表示的观测量。

---

## VirtualDistillationConfig

虚拟蒸馏的配置对象，可在低层类之外独立构造，并传给统一流水线。

### VirtualDistillationConfig(copies)

参数：

- `copies` (`int`)：密度矩阵的拷贝份数。构造时不做校验，小于 2 时会在真正使用该配置时失败。

### 属性

- `copies -> int`：配置的拷贝数。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。
- `repr()` 形如 `VirtualDistillationConfig(copies=2)`。

---

## VirtualDistillation

虚拟蒸馏的低层辅助类，持有基础线路的副本，负责构造 copy-swap 线路、执行分子与分母、以及求比值。

### VirtualDistillation(circuit, copies)

参数：

- `circuit` (`Circuit`)：基础线路。构造时复制保存，不会修改传入的线路。
- `copies` (`int`)：拷贝份数，必须不小于 2，否则抛出 `ErrorMitigationError`。拷贝数决定缓解线路的宽度：copy-swap 线路的宽度为基础线路宽度的 `copies` 倍。

### 属性

- `copies -> int`：当前配置的拷贝数。

### 方法

- `set_copies(copies)`：更新拷贝数。校验在赋值之前完成，校验失败时对象的拷贝数保持不变。
- `build_copy_swap_circuit() -> Circuit`：构造 copy-swap 线路。基础线路先做分解，再逐份复制到各自比特区间，最后按位插入 SWAP。
- `run_denominator_circuit(shots, estimator) -> tuple[float, float]`：执行分母线路，返回回调给出的二元组。
- `run_numerator_circuit(hamiltonian, shots, estimator) -> tuple[float, float]`：执行分子线路，返回回调给出的二元组。
- `run_vd(hamiltonian, shots_numerator, shots_denominator, estimator) -> tuple[float, float]`：依次执行分子与分母并求比值，返回 `(缓解后的期望值, 方差)`。

### 其他行为

- 支持 `copy`、`deepcopy`，副本与原对象各自独立。
- 不提供值相等比较；两个 `VirtualDistillation` 实例仅在为同一对象时相等。
- `repr()` 形如 `VirtualDistillation(copies=2)`。

---

## copy-swap 线路

`build_copy_swap_circuit()` 的输出由三部分构成：

1. 基础线路先做分解，保证复制的是一组已展开的操作。
2. 基础线路按 `copies` 份复制，第 `i` 份落在偏移 `i` 倍基础宽度的比特区间上。
3. 对第 1 份与其余每一份，逐位插入 SWAP。SWAP 只在第 1 份与其他各份之间插入，其余份之间不直接相连。

因此拷贝数为 2 时，基础线路每条操作出现 2 次，另加按位 SWAP；拷贝数增加时 SWAP 数量按份数线性增长，而不随拷贝数两两配对增长。

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.virtual_distillation import VirtualDistillation

circuit = Circuit(1)
circuit.x(0)

vd = VirtualDistillation(circuit, 2)
copy_swap = vd.build_copy_swap_circuit()
print(copy_swap.width)  # 2
```

---

## 执行约定

虚拟蒸馏的缓解线路只有一条，分子与分母都在该线路上执行，区别在于传给回调的观测量与采样次数：

| 方法 | 回调收到的线路 | 回调收到的观测量 | 回调收到的采样次数 |
| --- | --- | --- | --- |
| `run_denominator_circuit(shots, estimator)` | copy-swap 线路 | `None` | `shots` |
| `run_numerator_circuit(hamiltonian, shots, estimator)` | copy-swap 线路 | 按拷贝数扩展后的 `Hamiltonian` | `shots` |
| `run_vd(...)` | copy-swap 线路 | 先分子后分母，同上 | 分别为 `shots_numerator` 与 `shots_denominator` |

分子所用的观测量由传入的 `Hamiltonian` 扩展得到：原有 Pauli 项保留在各自比特上，更高位的比特补 `Z`，使观测量比特数与 copy-swap 线路宽度一致。扩展由类内部完成，调用方只需传入作用在基础线路上的观测量。

`run_vd()` 依次完成两步：先按 `shots_numerator` 执行分子，再按 `shots_denominator` 执行分母，然后按下式合成结果：

```text
期望值 = 分子期望值 / 分母期望值
方差   = 分子方差 / 分母期望值^2
       + 分子期望值^2 * 分母方差 / 分母期望值^4
```

分母期望值为 `0` 时无法求比值，抛出 `ErrorMitigationError`。`run_vd()` 要求观测量比特数与基础线路宽度一致，不符时在调用回调之前就抛出 `ErrorMitigationError`。

`run_denominator_circuit()` 与 `run_numerator_circuit()` 不做合成，直接返回回调给出的 `(期望值, 方差)` 二元组，适合需要分别查看分子与分母结果的场景。

示例：

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.virtual_distillation import VirtualDistillation
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

vd = VirtualDistillation(circuit, 2)


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

上例中的分子与分母为定值，故 `0.75 = 1.5 / 2.0`，`0.203125 = 0.25 / 4 + 2.25 / 16`。

---

## 异常情况

| 异常 | 触发场景 |
| --- | --- |
| `ErrorMitigationError` | 拷贝数小于 2（构造时或 `set_copies()` 时）；`run_vd()` 的观测量比特数与基础线路宽度不符；分母期望值为 `0`。 |
| `CircuitError` | copy-swap 线路构造失败，或观测量扩展后的比特数与缓解线路宽度不符。 |
| `TypeError` | 传入的 `estimator` 不可调用。 |
