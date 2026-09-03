# Ansatz

`cqlib.circuit.ansatz`  

```python
from cqlib.circuit.ansatz import (
    EntanglementTopology,
    TwoLocal,
    BasicEntanglerLayers,
    StronglyEntanglingLayers,
    AngleEncoding,
    BasisEncoding,
    ZFeatureMap,
    IQPFeatureMap,
    ZZFeatureMap,
    PauliFeatureMap,
    QAOAAnsatz,
    EvolutionStrategy,
    PauliEvolutionAnsatz,
)
```

`cqlib.circuit.ansatz` 提供了一组可复用的参数化线路模板，主要用于变分量子算法、量子机器学习、特征映射、QAOA 以及 Hamiltonian 演化等场景。用户可以通过这些模板快速生成结构化量子线路，而不必手动逐层添加旋转门和纠缠门。

---

## 通用模板接口

ansatz 模板主要提供以下接口：

| 方法 | 说明 |
| --- | --- |
| `validate()` | 检查模板配置是否合法，例如量子比特数量、门类型、拓扑和参数设置是否匹配。 |
| `build_circuit(prefix)` | 根据当前配置构建 `Circuit`，并使用指定前缀生成参数名。 |
| `num_parameters()` | 返回模板需要的参数数量。 |
| `num_qubits()` | 返回模板作用的量子比特数量。 |

```python
from cqlib.circuit.ansatz import TwoLocal

ansatz = TwoLocal(3)
ansatz.validate()

circuit = ansatz.build_circuit("theta")

assert len(circuit.symbols) == ansatz.num_parameters()
assert circuit.num_qubits == ansatz.num_qubits()
```

---

## 不可变 builder 模式

ansatz 模块中的多数配置接口采用链式调用形式。例如：

```python
from cqlib.circuit.ansatz import TwoLocal, EntanglementTopology
from cqlib.circuit.gates import StandardGate

ansatz = (
    TwoLocal(4)
    .reps(2)
    .rotation_gates([StandardGate.RY, StandardGate.RZ])
    .entanglement_gate(StandardGate.CX)
    .entanglement(EntanglementTopology.linear())
)
```

需要注意的是，这类配置方法通常不会修改原对象，而是返回一个带有新配置的对象。因此，可以安全地从同一个基础模板派生多个不同配置：

```python
base = TwoLocal(4)

linear = base.entanglement(EntanglementTopology.linear())
full = base.entanglement(EntanglementTopology.full())
```

上述代码中，`linear` 和 `full` 是两个不同配置的模板对象，`base` 本身保持不变。

---

## `EntanglementTopology`

`EntanglementTopology` 用于描述线路中二量子比特门或多量子比特项的作用拓扑。它决定了模板在构造纠缠层、feature map 多体项或 Pauli 演化项时，应在哪些量子比特之间建立连接。

```python
EntanglementTopology.linear()
EntanglementTopology.circular()
EntanglementTopology.full()
EntanglementTopology.custom(pairs)
```

| 拓扑 | 说明 |
| --- | --- |
| `linear()` | 最近邻链式连接，例如 `(0, 1), (1, 2), ...`。 |
| `circular()` | 在 `linear` 的基础上增加首尾连接，例如 `(n-1, 0)`。 |
| `full()` | 全连接拓扑，任意两个量子比特之间都可以产生连接。 |
| `custom(pairs)` | 用户自定义连接对，适合硬件拓扑或问题图已知的场景。 |

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `generate_pairs(num_qubits)` | 根据拓扑生成二量子比特连接对。 |
| `generate_k_tuples(k, num_qubits)` | 根据拓扑生成 `k`-local 作用量子比特组。 |

```python
from cqlib.circuit.ansatz import EntanglementTopology

topology = EntanglementTopology.linear()
pairs = topology.generate_pairs(4)

assert pairs == [(0, 1), (1, 2), (2, 3)]
```

---

## `TwoLocal`

`TwoLocal` 是一种常见的硬件友好 ansatz 模板，由交替出现的单量子比特旋转层和二量子比特纠缠层组成。它适合用于 VQE、分类模型、回归模型和通用变分线路实验。

```python
TwoLocal(num_qubits: int)
```

常用配置方法如下：

| 方法 | 说明 |
| --- | --- |
| `reps(n)` | 设置重复层数。 |
| `rotation_gates(gates)` | 设置每层使用的单量子比特参数门。 |
| `entanglement_gate(gate)` | 设置纠缠层使用的二量子比特门。 |
| `entanglement(topology)` | 设置纠缠拓扑。 |
| `skip_final_rotation_layer(skip)` | 设置是否跳过最后一层旋转层。 |

```python
from cqlib.circuit.ansatz import TwoLocal, EntanglementTopology
from cqlib.circuit.gates import StandardGate

ansatz = (
    TwoLocal(3)
    .reps(2)
    .rotation_gates([StandardGate.RY, StandardGate.RZ])
    .entanglement_gate(StandardGate.CX)
    .entanglement(EntanglementTopology.linear())
)

circuit = ansatz.build_circuit("theta")
```

ansatz 模块还提供了常见 `TwoLocal` 配置的便捷函数：

```python
real_amplitudes(num_qubits, reps, entanglement) -> TwoLocal
efficient_su2(num_qubits, reps, entanglement) -> TwoLocal
```

| 函数 | 默认结构 | 典型用途 |
| --- | --- | --- |
| `real_amplitudes` | `RY` 旋转 + `CX` 纠缠 | 实数振幅 ansatz、VQE、简单分类任务。 |
| `efficient_su2` | `RY` / `RZ` 旋转 + `CX` 纠缠 | 表达能力更强的通用硬件友好 ansatz。 |

---

## Feature Map

Feature map 用于将经典输入数据编码到量子线路中，是量子机器学习和量子核方法中的重要组成部分。不同 feature map 采用不同的编码方式，例如将输入特征作为旋转角、作为计算基态，或作为 Pauli 演化中的相位参数。

通常情况下，feature map 中的参数代表输入数据特征。实际训练时，用户可以将 feature map 与可训练 ansatz 组合使用。

---

## `AngleEncoding`

```python
AngleEncoding(num_qubits: int, rotation_gate: StandardGate)
```

`AngleEncoding` 将每个输入特征编码为一个单量子比特旋转角。它结构简单、参数含义直观，适合快速构造量子机器学习中的输入编码线路。

```python
from cqlib.circuit.ansatz import AngleEncoding
from cqlib.circuit.gates import StandardGate

encoding = AngleEncoding(4, StandardGate.RX)
circuit = encoding.build_circuit("x")
```

上述示例会为 4 个量子比特分别生成一个以 `x` 为前缀的输入参数，并通过 `RX` 门进行角度编码。

---

## `BasisEncoding`

```python
BasisEncoding(bits: list[bool])
```

`BasisEncoding` 用于按照给定 bitstring 准备计算基态。与角度编码不同，它不引入连续参数，而是根据布尔值决定是否对对应量子比特施加 `X` 门。

```python
from cqlib.circuit.ansatz import BasisEncoding

encoding = BasisEncoding([True, False, True])
circuit = encoding.build_circuit("unused")

assert encoding.num_parameters() == 0
```

---

## `ZFeatureMap`

```python
ZFeatureMap(num_qubits).reps(n)
```

`ZFeatureMap` 是一阶 Pauli-Z 特征映射，通常为每个量子比特引入一个输入参数，并通过 Z 方向相位演化编码特征。其参数数量通常等于量子比特数量。

---

## `IQPFeatureMap`

```python
IQPFeatureMap(num_qubits).reps(n).entanglement(topology)
```

`IQPFeatureMap` 是一种 IQP 风格的对角特征映射。它通常包含单体特征项和多体相互作用项，并通过重复层和纠缠拓扑控制编码结构。默认情况下，该模板通常使用多层结构和较强的纠缠连接。

```python
from cqlib.circuit.ansatz import IQPFeatureMap, EntanglementTopology

fm = (
    IQPFeatureMap(3)
    .reps(2)
    .entanglement(EntanglementTopology.full())
)

circuit = fm.build_circuit("x")
```

---

## `ZZFeatureMap`

```python
ZZFeatureMap(num_qubits).reps(n).entanglement(topology)
```

`ZZFeatureMap` 是二阶 Pauli-Z 特征映射，通常包含单量子比特 Z 项和双量子比特 ZZ 相互作用项。

```python
from cqlib.circuit.ansatz import ZZFeatureMap, EntanglementTopology

fm = ZZFeatureMap(3).reps(2).entanglement(EntanglementTopology.full())
circuit = fm.build_circuit("x")
```

便捷函数：

```python
zz_feature_map(num_qubits, reps, entanglement) -> ZZFeatureMap
```

---

## `PauliFeatureMap`

```python
PauliFeatureMap(num_qubits)
    .reps(n)
    .paulis(paulis)
    .entanglement(topology)
    .parameter_prefix(prefix)
```

`PauliFeatureMap` 支持通过任意 Pauli string 模板构造特征映射。用户可以指定单体项、多体项以及对应的纠缠拓扑，从而灵活表达不同阶数的特征交互。

```python
from cqlib.qis import PauliString
from cqlib.circuit.ansatz import PauliFeatureMap, EntanglementTopology

fm = (
    PauliFeatureMap(3)
    .reps(2)
    .paulis([PauliString.from_str("Z"), PauliString.from_str("ZZ")])
    .entanglement(EntanglementTopology.full())
)

circuit = fm.build_circuit("x")
```

便捷函数：

```python
pauli_feature_map(num_qubits, reps, paulis, entanglement) -> PauliFeatureMap
```

当需要自定义特征交互形式时，`PauliFeatureMap` 比固定结构的 `ZFeatureMap` 或 `ZZFeatureMap` 更灵活。

---

## Layer 模板

Layer 模板用于构造常见的可训练参数化层。与 feature map 不同，这类模板中的参数通常是优化器需要训练的变量。

### 1. `BasicEntanglerLayers`

```python
BasicEntanglerLayers(num_qubits)
    .reps(n)
    .rotation_gate(gate)
    .entanglement_gate(gate)
```

`BasicEntanglerLayers` 表示基础的“单量子比特旋转 + 环形纠缠”层结构。每一层通常先在各量子比特上施加参数化旋转门，再按固定环形结构施加纠缠门。

---

### 2. `StronglyEntanglingLayers`

```python
StronglyEntanglingLayers(num_qubits)
    .reps(n)
    .entanglement_gate(gate)
    .ranges(ranges)
```

`StronglyEntanglingLayers` 表示表达能力更强的纠缠层模板。它通常使用通用单量子比特旋转门和范围化环形纠缠结构。`ranges` 用于指定每一层的连接跨度，使不同层可以采用不同的纠缠范围。

```python
from cqlib.circuit.ansatz import StronglyEntanglingLayers

ansatz = StronglyEntanglingLayers(4).reps(3).ranges([1, 2])
circuit = ansatz.build_circuit("w")
```

---

## `QAOAAnsatz`

```python
QAOAAnsatz(cost_operator: Hamiltonian)
```

`QAOAAnsatz` 用于构造 QAOA线路。QAOA 在线路结构上交替应用 cost Hamiltonian 和 mixer Hamiltonian 的时间演化，一般用于组合优化问题。

其基本形式可以写为：

```text
U(beta, gamma) = product_l exp(-i beta_l H_M) exp(-i gamma_l H_C)
```

其中，`H_C` 表示问题 Hamiltonian，`H_M` 表示 mixer Hamiltonian，`gamma_l` 和 `beta_l` 是第 `l` 层中的变分参数。

常用配置方法如下：

| 方法 | 说明 |
| --- | --- |
| `reps(n)` | 设置 QAOA 层数 `p`，总参数数通常为 `2 * p`。 |
| `mixer(mixer_operator)` | 设置自定义 mixer Hamiltonian。 |
| `initial_state(circuit)` | 设置初态制备线路。 |
| `evolution_strategy(strategy)` | 设置 Hamiltonian 演化策略。 |

```python
from cqlib.qis import Hamiltonian, PauliString
from cqlib.circuit.ansatz import QAOAAnsatz

h_c = Hamiltonian(2)
h_c.add_term(PauliString.from_str("ZZ"), 0.5)

ansatz = QAOAAnsatz(h_c).reps(3)
circuit = ansatz.build_circuit("qaoa")

assert ansatz.num_parameters() == 6
```

---

## Hamiltonian Evolution

Hamiltonian 演化模板用于构造形如 `exp(-i H t)` 的量子线路。对于由 Pauli 项组成的 Hamiltonian，可以根据各项是否对易选择精确演化或 Trotter 分解策略。

### 1. `EvolutionStrategy`

`EvolutionStrategy` 用于指定 Hamiltonian 演化的线路分解方式。

| 静态方法 | 说明 |
| --- | --- |
| `exact()` | 对两两对易的 Hamiltonian 使用精确 Pauli rotation 乘积；若非对易，构建电路时会报错。 |
| `auto(steps=1)` | 自动选择精确演化或一阶 Trotter 分解。 |
| `trotter(mode, steps)` | 显式指定 Trotter-Suzuki 分解方式和步数。 |

```python
from cqlib.circuit.ansatz import EvolutionStrategy
from cqlib.qis import TrotterMode

exact = EvolutionStrategy.exact()
auto = EvolutionStrategy.auto(steps=4)
second = EvolutionStrategy.trotter(TrotterMode.second_order(), steps=8)
```

---

### 2. `EvolutionInfo`

`EvolutionInfo` 由 `PauliEvolutionAnsatz.evolution_info()` 返回，用于描述当前 Hamiltonian 演化策略和实际分解信息。

| 属性 | 说明 |
| --- | --- |
| `is_exact` | 当前分解是否为数学精确演化。 |
| `steps` | 实际使用的分解步数。 |
| `trotter_mode` | 当前 Trotter 模式；当使用 exact 策略时为 `None`。 |
| `all_terms_commute` | Hamiltonian 中的项是否两两对易。 |
| `num_terms` | 化简后的 Pauli 项数量。 |

---

### 3. `PauliEvolutionAnsatz`

```python
PauliEvolutionAnsatz(hamiltonian)
    .with_strategy(strategy)
    .with_time_param_name(name)
```

`PauliEvolutionAnsatz` 用于构造 `exp(-i H t)` 的参数化演化线路。生成的线路通常只包含一个时间参数，默认参数名为 `{prefix}_t`，也可以通过 `with_time_param_name()` 自定义。

```python
from cqlib.qis import Hamiltonian, PauliString
from cqlib.circuit.ansatz import PauliEvolutionAnsatz, EvolutionStrategy

h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("ZI"), 0.3)

ansatz = PauliEvolutionAnsatz(h).with_strategy(EvolutionStrategy.auto())
info = ansatz.evolution_info()
circuit = ansatz.build_circuit("evo")
```

