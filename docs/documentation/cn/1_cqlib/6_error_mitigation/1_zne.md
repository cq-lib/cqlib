# 零噪声外推（ZNE）

零噪声外推（Zero-Noise Extrapolation, ZNE）通过unitary folding主动放大线路噪声，得到多个 noisy expectation，再外推到零噪声点。

---

## Python 入口

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## 快速示例：折叠、执行与外推

```python
circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

zne = em.ZNEMitigation(circuit, [0, 1, 2])

print(zne.fold_levels)      # [0, 1, 2]
print(zne.noise_factors)    # [1, 3, 5]

folded = zne.fold_circuits()
print([len(c.operations) for c in folded])  # [1, 3, 5]

def estimator(folded_circuit, observable, shots):
    return (0.5 * len(folded_circuit.operations), 0.0)

noisy = zne.run_em_sequence_with_shots(None, hamiltonian, 256, estimator)
print("noisy:", noisy)  # [0.5, 1.5, 2.5]

mitigated = zne.extrapolate(
    noisy,
    em.ExtrapolateMethod.polynomial(),
    degree=1,
)
print("mitigated:", mitigated)
```

`ZNEMitigation` 持有原始线路的副本；`fold_circuits()` 不会修改输入 `circuit`。

---

## fold_levels 与 noise_factors

每个 `fold_level` 对应噪声因子：

```text
noise_factor = 2 * fold_level + 1
```

| `fold_level` | `noise_factor` | 含义 |
|--------------|----------------|------|
| `0` | `1` | 不折叠 |
| `1` | `3` | 一次 `U(U†U)` 折叠 |
| `2` | `5` | 两次折叠 |

`fold_levels` 必须为非负整数；至少两个点才能做外推。`ZneConfig` 与 `ZNEMitigation` 构造时不会校验负数；负值会在构造 `ErrorMitigation` 时抛出 `ErrorMitigationError`。

---

## 选择性门折叠

默认 `gate_set=None` 时做全局折叠。若只想折叠特定门，可传入目标指令列表：

```python
from cqlib.circuit import Instruction, StandardGate

gate_set = [Instruction.from_standard_gate(StandardGate.X)]
folded = zne.fold_circuits(gate_set)
```

仅名称匹配 `gate_set` 中指令的操作会被折叠。

---

## 执行折叠序列

| API | 说明 |
|-----|------|
| `run_em_sequence(gate_set, hamiltonian, estimator)` | 逐条执行折叠线路，返回期望值列表 |
| `run_em_sequence_with_shots(gate_set, hamiltonian, shots, estimator)` | 同上，并将 `shots` 传给 estimator |

`estimator` 必须可调用，且返回 `(float, float)` 二元组。类型签名如下：

```python
from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

Estimator = Callable[
    [Circuit, Hamiltonian | None, int | None],
    tuple[float, float],
]
```

---

## 外推方法

| API | 说明 |
|-----|------|
| `extrapolate(noisy_results, method, degree)` | 通用入口 |
| `poly_extrapolate(noisy_results, degree)` | 多项式拟合 |
| `exp_extrapolate(noisy_results)` | 指数衰减拟合（对数空间） |

`ExtrapolateMethod` 构造：

```python
em.ExtrapolateMethod.polynomial()
em.ExtrapolateMethod.exponential()
```

多项式外推时，`degree` 应小于 `len(noisy_results)`；一阶多项式外推是最常用默认。

---

## 与统一流水线集成

```python
def estimator(folded_circuit, observable, shots):
    return (0.5 * len(folded_circuit.operations), 0.0)

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)
print(result.variance)  # ZNE 为 None
```

---

## 说明

- ZNE 缓解的是期望值偏差，不保证恢复完整无噪声态；
- 折叠会增加线路深度与执行成本，正式实验应记录 `fold_levels` 与 `shots`；
- 外推结果对 estimator 质量敏感，建议对关键线路做对照验证。

---

## 下一步

- [虚拟蒸馏（Virtual Distillation）](2_virtual_distillation.md)：了解多拷贝 copy-swap 与比值估计。
- [统一流水线与 Estimator](3_unified_api.md)：用 `ErrorMitigation` 封装完整 ZNE 流程。
