# 误差缓解

Cqlib Python 绑定以 **`cqlib.error_mitigation`** 提供期望值层面的误差缓解能力，当前支持：

- **零噪声外推（ZNE）**：通过门折叠构造不同噪声强度下的线路族，再外推到零噪声极限；
- **虚拟蒸馏（Virtual Distillation）**：通过 copy-swap 线路估计 `Tr(O ρ^M) / Tr(ρ^M)`；
- **统一流水线（`ErrorMitigation`）**：按 `run()` → `get_mitigated()` 顺序封装上述方法。

误差缓解用于降低**观测量期望值**的系统性偏差，不是量子纠错；通常会增加线路执行次数，且需要用户自行提供 **estimator**（后端或模拟器回调）。

---

## 常用入口

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString

from cqlib.error_mitigation import (
    ErrorMitigation,
    ExtrapolateMethod,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
    VirtualDistillation,
    ZNEMitigation,
    ZneConfig,
)
```

> `cqlib.__init__` 未 re-export 上述符号，请显式 `import cqlib.error_mitigation`。

---

## 推荐工作流

大多数场景使用统一入口 `ErrorMitigation`：

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

def estimator(run_circuit, observable, shots):
    # 替换为模拟器或真实后端的期望值估计
    return (0.5 * len(run_circuit.operations), 0.0)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print("expectation:", result.expectation)
print("variance:", result.variance)
```

需要单独调试折叠、copy-swap 或外推步骤时，可改用低层 API `ZNEMitigation` 或 `VirtualDistillation`。

---

## Estimator 约定

所有缓解方法都依赖同一个 **Estimator** 签名：

```python
from collections.abc import Callable

from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian

Estimator = Callable[
    [Circuit, Hamiltonian | None, int | None],
    tuple[float, float],
]
```

| 参数 | 含义 |
|------|------|
| `run_circuit` | 待执行的线路（可能是折叠线路或 copy-swap 线路） |
| `observable` | 待估计的 `Hamiltonian`；分母线路时为 `None` |
| `shots` | 本次执行的 shot 数；部分 API 可为 `None` |
| 返回值 | `(expectation, variance)` |

Cqlib **不提供**内置 simulator/backend estimator；请将 `estimator` 对接 QIS 模拟、密度矩阵噪声模型或 Tianyan 后端。

---

## 数据流概览

```text
原始 Circuit + Hamiltonian
  ↓
构造缓解线路族（ZNE 折叠 / VD copy-swap）
  ↓
estimator 逐条执行并返回 (expectation, variance)
  ↓
ZNE：外推到 noise_factor = 0
VD：计算 numerator / denominator 比值
  ↓
MitigatedResult(expectation, variance?)
```

---

## 模块划分

| 模块 | 说明 |
|------|------|
| `cqlib.error_mitigation.zne` | `ZNEMitigation`、`ExtrapolateMethod`、`ZneConfig` |
| `cqlib.error_mitigation.virtual_distillation` | `VirtualDistillation`、`VirtualDistillationConfig` |
| `cqlib.error_mitigation.unified` | `ErrorMitigation`、`MitigationMethod`、`RunArgs`、`ProcessArgs` |

---

## 能力边界

- 本模块聚焦 **ZNE** 与 **Virtual Distillation**；
- **读出误差矫正** 不在此模块，见 Tianyan 章节 [`5_readout_mitigation.md`](../7_tianyan/5_readout_mitigation.md)；
- 每个 `ErrorMitigation` 实例只能 `run()` 一次、`get_mitigated()` 一次。

---

## 下一步

- [零噪声外推（ZNE）](1_zne.md)：学习门折叠、`noise_factors` 与外推方法。
- [虚拟蒸馏（Virtual Distillation）](2_virtual_distillation.md)：了解 copy-swap 线路与比值估计。
- [统一流水线与 Estimator](3_unified_api.md)：掌握 `ErrorMitigation` 状态机与错误处理。
