# 零噪声外推

`cqlib.error_mitigation.zne`

`cqlib.error_mitigation.zne` 提供零噪声外推的低层接口：按折叠等级构造缓解线路族，用回调逐条估计期望值，再把结果外推到零噪声点。本页覆盖 `ZNEMitigation`、`ZneConfig`、`ExtrapolateMethod` 以及回调类型别名 `Estimator`。

## 导入

```python
from cqlib.error_mitigation.zne import (
    Estimator,
    ExtrapolateMethod,
    ZneConfig,
    ZNEMitigation,
)
```

上述符号同时由 `cqlib.error_mitigation` 重导出。

---

## Estimator

回调类型别名，约定缓解方法如何调用调用方提供的估计函数。

### `Estimator = Callable[[Circuit, Hamiltonian | None, int | None], tuple[float, float]]`

调用约定：以位置参数传入三个实参。

| 参数 | 类型 | 含义 |
| --- | --- | --- |
| `circuit` | `Circuit` | 本次待执行的缓解线路。零噪声外推传入的是按某个折叠等级展开后的线路。 |
| `hamiltonian` | `Hamiltonian \| None` | 待估计的观测量。零噪声外推始终传入配置的 `Hamiltonian`。 |
| `shots` | `int \| None` | 本次执行的采样次数。 |

返回约定：必须返回一个二元组 `(期望值, 方差)`，两个元素都会按浮点数取用。

| 返回位置 | 含义 | 零噪声外推是否使用 |
| --- | --- | --- |
| 第 1 个元素 | 该线路在观测量上的期望值。 | 使用，构成外推的输入序列。 |
| 第 2 个元素 | 该期望值的方差。 | 不使用，仅在虚拟蒸馏中参与后处理。 |

`shots` 的取值取决于调用路径：

- `run_em_sequence()` 始终传入 `None`。
- `run_em_sequence_with_shots()` 传入给定的采样次数。
- 统一流水线的 `RunArgs.zne(shots=...)` 传入该值；未提供时为 `None`。

当 `shots` 为 `None` 时，表示由回调自行决定采样方式。回调必须可调用，否则在流程开始前抛出 `TypeError`。回调内部抛出的异常会在流程返回后原样重新抛出，不会被替换为其他异常类型。

同一别名在 `cqlib.error_mitigation.virtual_distillation` 与 `cqlib.error_mitigation.unified` 中同样定义，三者含义一致；[虚拟蒸馏](2_virtual_distillation.md) 与 [统一流水线](3_unified.md) 中回调形式相同，区别仅在于传入的线路与观测量。

---

## ZNEMitigation

零噪声外推的低层辅助类，持有基础线路的副本，负责折叠、执行与外推。

### ZNEMitigation(circuit, fold_levels)

参数：

- `circuit` (`Circuit`)：基础线路。构造时复制保存，折叠不会修改传入的线路。
- `fold_levels` (`Sequence[int]`)：折叠等级序列，每个等级对应噪声因子 `2 * level + 1`。构造时不做校验，负值在折叠时才会失败。

### 属性

- `circuit -> Circuit`：基础线路的副本。
- `fold_levels -> list[int]`：配置的折叠等级。
- `noise_factors -> list[int]`：由折叠等级导出的噪声因子，满足 `noise_factor = 2 * fold_level + 1`。

### 方法

- `fold_circuits(gate_set=None) -> list[Circuit]`：按每个折叠等级各返回一条缓解线路，返回顺序与 `fold_levels` 一致，长度与 `fold_levels` 相同。
- `run_em_sequence(gate_set, hamiltonian, estimator) -> list[float]`：逐条执行折叠线路，返回期望值列表。等价于以 `shots=None` 调用下一个方法。
- `run_em_sequence_with_shots(gate_set, hamiltonian, shots, estimator) -> list[float]`：同上，并把 `shots` 传给回调。
- `extrapolate(noisy_results, method, degree) -> float`：按 `method` 选择拟合方式，返回零噪声点的外推值。
- `poly_extrapolate(noisy_results, degree) -> float`：用多项式拟合外推。
- `exp_extrapolate(noisy_results) -> float`：用指数衰减模型拟合外推。

### 其他行为

- 支持 `copy`、`deepcopy`，副本与原对象各自独立。
- 不提供值相等比较；两个 `ZNEMitigation` 实例仅在为同一对象时相等。
- `repr()` 形如 `ZNEMitigation(fold_levels=[0, 1, 2], noise_factors=[1, 3, 5])`。

---

## 折叠行为

`fold_circuits()` 的 `gate_set` 决定折叠范围：

- `gate_set` 为 `None`（默认）：全局折叠，把线路整体改写为 `U -> U (U† U)^level`。展开后的操作数随等级线性增长。
- `gate_set` 为指令序列：选择性折叠，只对指令名匹配其中任一指令的操作做同样的展开，其余操作原样保留。

指令名取自传入的 `Instruction`，因此可先按标准门名构造：

下文示例中的 `zne` 为 `ZNEMitigation` 实例。

```python
from cqlib.circuit import Instruction, StandardGate

gate_set = [Instruction.from_standard_gate(StandardGate.X)]
folded = zne.fold_circuits(gate_set)
```

折叠等级为 `0` 时不展开，返回基础线路的副本。等级为负时折叠失败并抛出 `CircuitError`。

---

## 执行约定

`run_em_sequence()` 与 `run_em_sequence_with_shots()` 都要求观测量比特数与基础线路宽度一致，否则抛出 `ErrorMitigationError`。两者的 `gate_set` 都需要显式传入：全局折叠时传 `None`，选择性折叠时传指令序列。

两个方法对每个折叠等级各调用一次回调，返回的列表长度与 `fold_levels` 相同，列表元素为回调返回二元组的第 1 个元素：

```python
noisy = zne.run_em_sequence(None, hamiltonian, estimator)
```

`run_em_sequence()` 传入的 `shots` 始终为 `None`；需要把采样次数交给回调时使用 `run_em_sequence_with_shots()`。

---

## 外推

三个外推方法都以 `noise_factors` 作为自变量、以 `noisy_results` 作为观测值，返回自变量为 `0` 处的拟合值。

### `extrapolate(noisy_results, method, degree) -> float`

按 `method` 选择拟合方式：

- `ExtrapolateMethod.polynomial()`：使用 `degree`，等价于 `poly_extrapolate(noisy_results, degree)`。
- `ExtrapolateMethod.exponential()`：忽略 `degree`，等价于 `exp_extrapolate(noisy_results)`。

参数：

- `noisy_results` (`Sequence[float]`)：各个折叠等级下的期望值，长度必须与 `noise_factors` 相同。
- `method` (`ExtrapolateMethod`)：拟合方法。
- `degree` (`int`)：多项式阶数。指数拟合时该参数不参与计算，仍需传入。

返回：

- `float`：零噪声点的外推值。

### `poly_extrapolate(noisy_results, degree) -> float`

对数据点做 `degree` 阶多项式最小二乘拟合，返回常数项。`degree` 必须严格小于噪声因子个数；`noisy_results` 必须与噪声因子一一对应，且不能为空。

### `exp_extrapolate(noisy_results) -> float`

按 `y(x) = A * exp(-x / tau)` 做对数空间线性回归，返回零噪声点的值 `A`。调用前所有 `noisy_results` 必须为正值——对数空间要求非负，而取值为 `0` 的项同样会被拒绝。

示例：

```python
from cqlib.circuit import Circuit
from cqlib.error_mitigation.zne import ZNEMitigation

circuit = Circuit(1)
circuit.x(0)

zne = ZNEMitigation(circuit, [0, 1, 2])
print(zne.noise_factors)  # [1, 3, 5]

noisy_results = [2.75, 6.75, 10.75]
print(zne.poly_extrapolate(noisy_results, 1))  # 0.75
```

上例中的数据点满足 `y = 0.75 + 2x`，一阶拟合的常数项即零噪声点的值。

---

## ExtrapolateMethod

外推方法，通过静态方法构造。实例为不可变值对象，可传给 `ZNEMitigation.extrapolate()` 与统一流水线的 `ProcessArgs.zne()`。

### 静态方法

- `ExtrapolateMethod.polynomial()`：多项式拟合，需要阶数。
- `ExtrapolateMethod.exponential()`：对数空间的指数衰减拟合，不使用阶数。

### 其他行为

- `str()` 返回 `"polynomial"` 或 `"exponential"`；`repr()` 返回 `ExtrapolateMethod.polynomial()` 或 `ExtrapolateMethod.exponential()`。
- 支持 `==`、`hash`、`copy`、`deepcopy`。

---

## ZneConfig

零噪声外推的配置对象，可在低层类之外独立构造，并传给统一流水线。

### ZneConfig(fold_levels)

参数：

- `fold_levels` (`Sequence[int]`)：折叠等级序列，负值不在构造时校验。

### 属性

- `fold_levels -> list[int]`：配置的折叠等级。

### 其他行为

- 支持 `==`、`copy`、`deepcopy`；不提供 `hash`。
- `repr()` 形如 `ZneConfig(fold_levels=[0, 1, 2])`。

示例：

```python
from cqlib.error_mitigation.zne import ZneConfig

config = ZneConfig([0, 1, 2])
print(config.fold_levels)  # [0, 1, 2]
```

---

## 异常情况

| 异常 | 触发场景 |
| --- | --- |
| `ErrorMitigationError` | 观测量比特数与基础线路宽度不符；外推输入为空；外推输入长度与噪声因子个数不符；多项式阶数不小于数据点个数；指数外推的输入存在非正值；拟合的正规方程或回归系统奇异。 |
| `CircuitError` | 折叠等级为负，构造折叠线路失败。 |
| `TypeError` | 传入的 `estimator` 不可调用。 |
