# 统一流水线与 Estimator

`ErrorMitigation` 是推荐的顺序式入口：先 `run()` 收集各缓解线路的 estimator 输出，再 `get_mitigated()` 做方法相关的后处理。

---

## Python 入口

```python
import cqlib.error_mitigation as em
from cqlib.circuit import Circuit
from cqlib.qis import Hamiltonian, PauliString
```

---

## 状态机

每个 `ErrorMitigation` 实例遵循固定生命周期：

```text
创建 → run() → get_mitigated() → 结束
```

| 规则 | 说明 |
|------|------|
| `run()` 只能调用一次 | 重复调用抛出 `ErrorMitigationError` |
| 必须先 `run()` 再 `get_mitigated()` | 否则抛出 `ErrorMitigationError` |
| `get_mitigated()` 只能调用一次 | 重复调用抛出 `ErrorMitigationError` |

---

## ZNE 流水线示例

```python
circuit = Circuit(1)
circuit.x(0)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.zne(em.ZneConfig([0, 1, 2])),
)

def estimator(run_circuit, observable, shots):
    assert observable is not None
    assert shots == 128
    return (0.5 * len(run_circuit.operations), 0.0)

mitigation.run(hamiltonian, em.RunArgs.zne(shots=128), estimator)
result = mitigation.get_mitigated(
    em.ProcessArgs.zne(em.ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)
print(result.variance)  # ZNE: None
```

---

## Virtual Distillation 流水线示例

```python
circuit = Circuit(1)
hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])

mitigation = em.ErrorMitigation(
    circuit,
    em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2)),
)

def estimator(run_circuit, observable, shots):
    assert run_circuit.width == 2
    if observable is None:
        return (2.0, 1.0)
    return (1.5, 0.25)

mitigation.run(hamiltonian, em.RunArgs.virtual_distillation(3, 2), estimator)
result = mitigation.get_mitigated(em.ProcessArgs.virtual_distillation())

print(result.expectation)
print(result.variance)
```

---

## 配置对象

### MitigationMethod

```python
em.MitigationMethod.zne(em.ZneConfig([0, 1, 2]))
em.MitigationMethod.virtual_distillation(em.VirtualDistillationConfig(2))
```

### RunArgs

| 构造方式 | 参数 |
|----------|------|
| `RunArgs.zne(gate_set=None, shots=None)` | 可选选择性折叠门集与 shot 数 |
| `RunArgs.virtual_distillation(shots_numerator, shots_denominator)` | 分子/分母 shot 预算 |

### ProcessArgs

| 构造方式 | 参数 |
|----------|------|
| `ProcessArgs.zne(method, degree=None)` | 外推方法与多项式阶数 |
| `ProcessArgs.virtual_distillation()` | 无额外参数 |

### MitigatedResult

| 字段 | 说明 |
|------|------|
| `expectation` | 缓解后的期望值 |
| `variance` | 缓解后方差；ZNE 为 `None`，VD 通常有值 |

---

## Estimator 实现要点

```python
def estimator(run_circuit, observable, shots):
    # run_circuit: 可能是折叠线路或 copy-swap 线路
    # observable: Hamiltonian 或 None（VD 分母）
    # shots: int 或 None
    return (expectation, variance)
```

实现时请确认：

- 返回值必须是 `(float, float)`；
- ZNE 的 `run_em_sequence_with_shots` 会把 `shots` 传给 estimator；
- VD 分母路径上 `observable is None`；
- estimator 内部异常会原样向上抛出。

---

## 常见错误

| 场景 | 异常 |
|------|------|
| `fold_levels` 含负数 | `ErrorMitigationError` |
| `copies < 2` | `ErrorMitigationError` |
| 未 `run()` 就 `get_mitigated()` | `ErrorMitigationError` |
| `estimator` 不可调用 | `TypeError` |
| estimator 返回值格式错误 | `TypeError` / `ValueError` |

---

## 低层 API 与模块导出

除 `ErrorMitigation` 外，也可直接使用：

- `ZNEMitigation` — 见 [零噪声外推（ZNE）](1_zne.md)；
- `VirtualDistillation` — 见 [虚拟蒸馏](2_virtual_distillation.md)。

`cqlib.error_mitigation.__all__` 导出：`Estimator`、`ErrorMitigationError`、`ExtrapolateMethod`、`ZneConfig`、`VirtualDistillationConfig`、`MitigationMethod`、`RunArgs`、`ProcessArgs`、`MitigatedResult`、`ZNEMitigation`、`VirtualDistillation`、`ErrorMitigation`。

---

## 说明

- 统一流水线与低层 API 共享同一套 `Estimator` 类型别名；
- 读出误差矫正属于 Tianyan 模块，不在 `cqlib.error_mitigation` 内；
- 对接真实硬件时，请将 estimator 封装为后端 shot 采集与期望值汇总逻辑。
