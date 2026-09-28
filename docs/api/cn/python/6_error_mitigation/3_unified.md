# 统一流水线

`cqlib.error_mitigation.unified`

`cqlib.error_mitigation.unified` 提供顺序式的统一流水线：先按选定方法执行缓解线路并收集回调节果，再做方法相关的后处理。本页覆盖 `ErrorMitigation` 及其配套的 `MitigationMethod`、`RunArgs`、`ProcessArgs`、`MitigatedResult` 与 `ErrorMitigationError`。

## 导入

```python
from cqlib.error_mitigation.unified import (
    Estimator,
    ErrorMitigationError,
    MitigationMethod,
    RunArgs,
    ProcessArgs,
    MitigatedResult,
    ErrorMitigation,
)
```

除 `Estimator` 外，上述符号同时由 `cqlib.error_mitigation` 重导出。回调约定见 [零噪声外推](1_zne.md) 中的 `Estimator`。

---

## MitigationMethod

缓解方法选择，通过静态方法构造，在构造 `ErrorMitigation` 时确定本实例使用哪种方法。

### 静态方法

- `MitigationMethod.zne(config)`：配置为零噪声外推。参数 `config`（`ZneConfig`）给出折叠等级。
- `MitigationMethod.virtual_distillation(config)`：配置为虚拟蒸馏。参数 `config`（`VirtualDistillationConfig`）给出拷贝数。

### 属性

- `method_type -> str`：方法标识，取值为 `"zne"` 或 `"virtual_distillation"`。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。
- `repr()` 形如 `MitigationMethod.zne(ZneConfig(fold_levels=[0, 1, 2]))`。

---

## RunArgs

一次执行所需的运行参数，与所选方法一一对应。方法选 `zne` 时只能用 `RunArgs.zne(...)`，否则 `run()` 抛出 `ErrorMitigationError`。

### 静态方法

- `RunArgs.zne(gate_set=None, shots=None)`：
  - `gate_set` (`Sequence[Instruction] | None`)：选择性折叠的门集。默认 `None`，表示全局折叠。
  - `shots` (`int | None`)：传给回调的采样次数。默认 `None`。
- `RunArgs.virtual_distillation(shots_numerator, shots_denominator)`：
  - `shots_numerator` (`int`)：分子的采样次数。
  - `shots_denominator` (`int`)：分母的采样次数。

### 属性

- `method_type -> str`：方法标识。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。比较 `zne` 参数时按指令名比对 `gate_set` 中的元素，而非按对象同一性。
- `repr()` 形如 `RunArgs.zne(gate_set=None, shots=Some(256))`。

示例：

```python
from cqlib.circuit import Instruction, StandardGate
from cqlib.error_mitigation.unified import RunArgs

global_folding = RunArgs.zne(shots=256)
selective_folding = RunArgs.zne(
    gate_set=[Instruction.from_standard_gate(StandardGate.X)],
    shots=256,
)

vd_args = RunArgs.virtual_distillation(shots_numerator=3, shots_denominator=2)
```

---

## ProcessArgs

后处理参数，与所选方法一一对应，在 `get_mitigated()` 时传入。

### 静态方法

- `ProcessArgs.zne(method, degree=None)`：
  - `method` (`ExtrapolateMethod`)：外推方法。
  - `degree` (`int | None`)：多项式阶数。默认 `None`，此时取 `min(数据点个数 - 1, 1)`。指数拟合时该参数不参与计算。
- `ProcessArgs.virtual_distillation()`：虚拟蒸馏无额外后处理参数。

### 属性

- `method_type -> str`：方法标识。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。
- `repr()` 形如 `ProcessArgs.zne(method=polynomial, degree=Some(1))`。

---

## MitigatedResult

缓解结果，由 `get_mitigated()` 返回，不可直接构造。属性为只读。

### 属性

- `expectation -> float`：缓解后的期望值。
- `variance -> float | None`：缓解后的方差。零噪声外推的结果为 `None`；虚拟蒸馏的结果有值。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。
- `repr()` 形如 `MitigatedResult(expectation=0.5, variance=None)`。

---

## ErrorMitigation

统一流水线，按 `run()` → `get_mitigated()` 的顺序完成一次缓解。

### ErrorMitigation(circuit, method)

参数：

- `circuit` (`Circuit`)：基础线路。构造时复制保存。
- `method` (`MitigationMethod`)：缓解方法。

构造时校验方法配置：零噪声外推的折叠等级含负值、或虚拟蒸馏的拷贝数小于 2 时抛出 `ErrorMitigationError`。

### 方法

- `run(hamiltonian, run_args, estimator) -> None`：按 `run_args` 执行方法对应的缓解线路，并把回调节果记录下来。
- `get_mitigated(process_args) -> MitigatedResult`：对已记录的结果做后处理，返回缓解结果。

### 其他行为

- 支持 `copy`、`deepcopy`，副本携带当前状态。
- 不提供值相等比较。
- `repr()` 固定为 `ErrorMitigation()`。

### 状态机

每个实例只完成一次缓解，调用顺序与参数匹配都由实例自身约束：

| 当前状态 | 调用 | 结果 |
| --- | --- | --- |
| 已构造 | `run()` | 执行并转入已运行状态。 |
| 已构造 | `get_mitigated()` | 抛出 `ErrorMitigationError`，要求先完成 `run()`。 |
| 已运行 | `get_mitigated()` | 返回结果并转入已缓解状态。 |
| 已运行 | `run()` | 抛出 `ErrorMitigationError`，不允许重复 `run()`。 |
| 已缓解 | `run()` 或 `get_mitigated()` | 抛出 `ErrorMitigationError`，不允许再次调用。 |

若 `run()` 传入的 `RunArgs` 与构造时的方法不匹配，抛出 `ErrorMitigationError`，此时回调不会被调用，状态也不改变。`get_mitigated()` 传入的 `ProcessArgs` 与方法不匹配时同样抛出 `ErrorMitigationError`。

### 执行约定

零噪声外推：`run()` 按配置的折叠等级构造缓解线路，对每条线路调用一次回调，观测量为传入的 `Hamiltonian`，采样次数取自 `RunArgs.zne(shots=...)`。回调返回的二元组只取第 1 个元素，方差被丢弃。`get_mitigated()` 对外推输入做拟合，结果的 `variance` 为 `None`。

虚拟蒸馏：`run()` 先构造 copy-swap 线路，再在同一条线路上调用两次回调——先分子，后分母。分子传入按拷贝数扩展后的观测量与 `shots_numerator`，分母传入 `None` 与 `shots_denominator`。`get_mitigated()` 按 [虚拟蒸馏](2_virtual_distillation.md) 的合成公式求比值，结果的 `variance` 有值。

### 示例

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.unified import (
    ErrorMitigation,
    MitigationMethod,
    ProcessArgs,
    RunArgs,
)
from cqlib.error_mitigation.zne import ExtrapolateMethod, ZneConfig
from cqlib.qis import Hamiltonian, PauliString

circuit = Circuit(1)
circuit.x(0)

hamiltonian = Hamiltonian.from_list([(PauliString.from_str("Z"), 1.0)])


def estimator(run_circuit, observable, shots):
    return (len(run_circuit.operations) + 0.5, 0.0)


mitigation = ErrorMitigation(
    circuit,
    MitigationMethod.zne(ZneConfig([0, 1, 2])),
)

mitigation.run(hamiltonian, RunArgs.zne(shots=256), estimator)
result = mitigation.get_mitigated(
    ProcessArgs.zne(ExtrapolateMethod.polynomial(), degree=1)
)

print(result.expectation)  # 0.5
print(result.variance)     # None
```

上例中折叠线路的操作数依次为 `[1, 3, 5]`，回调给出 `[1.5, 3.5, 5.5]`，一阶拟合的零噪声点取值为 `0.5`。

---

## ErrorMitigationError

错误缓解异常的基类，继承自 `cqlib.circuit.CqlibError`。

| 异常 | 触发场景 |
| --- | --- |
| `ErrorMitigationError` | 见下表所列情形。 |
| `CircuitError` | 折叠线路或 copy-swap 线路构造失败。 |
| `ValueError` | 观测量相关的运算失败，例如按拷贝数扩展 `Hamiltonian` 时出错。 |
| `TypeError` | 传入的 `estimator` 不可调用。 |

`ErrorMitigationError` 覆盖的常见情形：

| 触发场景 | 说明 |
| --- | --- |
| 构造时折叠等级含负值 | 在 `ErrorMitigation()` 构造时报告，`ZneConfig` 本身不报错。 |
| 构造时拷贝数小于 2 | 在 `ErrorMitigation()` 构造时报告。 |
| 观测量比特数与基础线路宽度不符 | 在 `run()` 中报告，回调不会被调用。 |
| `RunArgs` 与实际方法不匹配 | 在 `run()` 中报告，回调不会被调用。 |
| `ProcessArgs` 与实际方法不匹配 | 在 `get_mitigated()` 中报告。 |
| 未 `run()` 就调用 `get_mitigated()` | 要求先完成 `run()`。 |
| 重复 `run()` 或重复 `get_mitigated()` | 每个实例只允许各调用一次。 |
| 外推输入非法或拟合失败 | 见 [零噪声外推](1_zne.md)。 |
| 虚拟蒸馏的分母期望值为 `0` | 无法求比值，见 [虚拟蒸馏](2_virtual_distillation.md)。 |
