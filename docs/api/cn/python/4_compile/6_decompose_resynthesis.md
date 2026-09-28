# Decompose / Resynthesis

`cqlib.compile.transform.decompose`

门分解、双比特块重综合、目标门集转换与设备降级，位于 `cqlib.compile.transform` 的 `decompose`、`resynthesis`、`target_basis` 与 `device_lowering` 子模块。

门分解按操作可用的表示分层：定义展开处理自带实现线路的门，数值合成处理带固定矩阵的酉门，多控制门分解处理受控操作。定义展开先于数值合成执行，使带实现线路的酉门在进入矩阵合成之前已经展开。

## 导入

```python
from cqlib.compile.transform.decompose import (
    DecompositionRuleStats,
    McGateDecomposeConfig,
    TwoQubitUnitaryDecomposeBasis,
    UnitaryDecomposeConfig,
    decompose_mc_gates,
    decompose_mc_gates_for_device,
    decompose_mc_gates_with_rule_stats,
    decompose_unitaries,
    decompose_unitaries_with_rule_stats,
    expand_definitions,
    mc_gate,
    unitary,
)
from cqlib.compile.transform.resynthesis import (
    ResynthesizeTwoQubitBlocks,
    TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
)
from cqlib.compile.transform.target_basis import (
    TargetBasisCost,
    TargetBasisCostModel,
    TargetBasisLowerer,
    TargetBasisSignature,
)
from cqlib.compile.transform import DeviceLowerer
```

---

## 定义展开

### expand_definitions(circuit)

展开电路门定义，不修改输入线路。

参数：

- `circuit` (`Circuit`)：待展开线路。

返回：

- `TransformResult`；`circuit` 字段为展开后的线路，`changed` 表示是否发生改写。

---

## 酉门合成

线路级入口位于 `cqlib.compile.transform.decompose`，对应的数值合成原语位于 `cqlib.compile.transform.decompose.unitary`。

### decompose_unitaries(circuit, config=None)

用矩阵合成单比特与双比特酉门。

参数：

- `circuit` (`Circuit`)：待合成线路。
- `config` (`UnitaryDecomposeConfig | None`)：分解配置，默认 `UnitaryDecomposeConfig()`。

返回：

- `TransformResult`

异常情况：

- `CompilerTransformError`：`UnitaryGate` 没有矩阵表示，或矩阵的比特数与门的比特数不符。

### decompose_unitaries_with_rule_stats(circuit, config=None)

同上，并返回 pass 内规则缓存统计。

返回：

- `(TransformResult, DecompositionRuleStats)`

### UnitaryDecomposeConfig(*, two_qubit_basis=None, target_basis=None, recurse_control_flow=True)

参数：

- `two_qubit_basis` (`TwoQubitUnitaryDecomposeBasis | None`)：双比特合成门集。
- `target_basis` (`list[str | Instruction] | None`)：目标门集约束，条目形式同 `compile()` 的 `target_basis`。
- `recurse_control_flow` (`bool`)：是否递归进入控制流块。

`two_qubit_basis` 与 `target_basis` 互斥，同时给出会被拒绝。

属性：

- `two_qubit_basis -> TwoQubitUnitaryDecomposeBasis | None`
- `target_basis -> list[Instruction] | None`
- `recurse_control_flow -> bool`

异常情况：

- `CompilerConfigError`：`two_qubit_basis` 与 `target_basis` 同时给出，或目标门集为空。

### TwoQubitUnitaryDecomposeBasis

双比特合成门集，取值为以下静态方法之一：

- `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`：局部 `U` 门加 `RXX`、`RYY`、`RZZ`。
- `TwoQubitUnitaryDecomposeBasis.cx()`：局部 `U` 门加 `CX`。
- `TwoQubitUnitaryDecomposeBasis.cy()`：局部 `U` 门加 `CY`。
- `TwoQubitUnitaryDecomposeBasis.cz()`：局部 `U` 门加 `CZ`。
- `TwoQubitUnitaryDecomposeBasis.rzz()`：局部 `U`、`H`、`RX` 加 `RZZ`。

### DecompositionRuleStats

pass 内规则缓存统计：

- `hits -> int`：缓存命中次数。
- `misses -> int`：缓存未命中次数。
- `inserts -> int`：缓存写入次数。

---

## 数值酉门合成

`cqlib.compile.transform.decompose.unitary` 提供矩阵到门序列的直接合成，不经过线路遍历。

### synthesize_numeric_1q_unitary(matrix)

把数值单比特酉矩阵合成 `U` 门角度与全局相位。

参数：

- `matrix` (`list[list[complex]]`)：2×2 酉矩阵，可转换为二维 `complex128` 数组。

返回：

- `OneQubitUnitaryDecomposition`

异常情况：

- `CompilerConfigError`：矩阵不是 2×2。
- `TypeError`：输入无法转换为二维数组。

### synthesize_numeric_2q_unitary(matrix, first, second, basis=None)

把数值双比特酉矩阵合成为门序列。

参数：

- `matrix` (`list[list[complex]]`)：4×4 酉矩阵，可转换为二维 `complex128` 数组。
- `first`、`second` (`int | Qubit`)：作用比特，须互不相同。
- `basis` (`TwoQubitUnitaryDecomposeBasis | None`)：合成门集，默认 `TwoQubitUnitaryDecomposeBasis.pauli_rotations()`。

返回：

- `TwoQubitUnitarySynthesisResult`

异常情况：

- `CompilerConfigError`：矩阵不是 4×4，或 `first` 与 `second` 指向同一比特。

### kak_decompose(matrix)

计算数值 4×4 酉矩阵的规范 KAK 分解。

参数：

- `matrix` (`list[list[complex]]`)：4×4 酉矩阵，可转换为二维 `complex128` 数组。

返回：

- `KakDecomposition`

异常情况：

- `TypeError`：输入无法转换为二维数组。

### OneQubitUnitaryDecomposition

单比特酉矩阵的 `U` 门分解结果。

属性：

- `theta -> float`：合成的 `U` 门的极角。
- `phi -> float`：合成的 `U` 门的第一个方位角。
- `lambda_ -> float`：合成的 `U` 门的第二个方位角。
- `global_phase -> float`：乘在合成的 `U` 门上的标量相位。

其他行为：

- 支持 `copy`、`deepcopy`，可按值比较。

### TwoQubitUnitarySynthesisResult

双比特酉矩阵的合成结果。

属性：

- `operations -> list[ValueOperation]`：实现该酉门的标准门操作序列，精确到 `global_phase`。
- `global_phase -> float`：乘在操作序列上的标量相位。

其他行为：

- 支持 `copy`、`deepcopy`。

### KakDecomposition

双比特酉矩阵的规范 KAK 分解。

属性：

- `global_phase -> float`：乘在完整分解上的标量相位。
- `k1l -> numpy.ndarray`：作用在 Cartan 相互作用之后的左局部因子。
- `k1r -> numpy.ndarray`：作用在 Cartan 相互作用之后的右局部因子。
- `k2l -> numpy.ndarray`：作用在 Cartan 相互作用之前的左局部因子。
- `k2r -> numpy.ndarray`：作用在 Cartan 相互作用之前的右局部因子。
- `a -> float`：规范 Pauli-XX 相互作用坐标。
- `b -> float`：规范 Pauli-YY 相互作用坐标。
- `c -> float`：规范 Pauli-ZZ 相互作用坐标。

其他行为：

- 各 `k` 因子为独立副本，修改返回值不影响分解对象。
- 支持 `copy`、`deepcopy`。

---

## 多控制门分解

### decompose_mc_gates(circuit, config=None)

用配置的辅助比特资源分解多控制门。此入口没有目标设备，因此不检查设备容量；需要容量检查时用 `decompose_mc_gates_for_device`。

参数：

- `circuit` (`Circuit`)：待分解线路。
- `config` (`McGateDecomposeConfig | None`)：分解配置，默认 `McGateDecomposeConfig()`。

返回：

- `TransformResult`

### decompose_mc_gates_with_rule_stats(circuit, config=None)

同上，并返回规则缓存统计。

返回：

- `(TransformResult, DecompositionRuleStats)`

### decompose_mc_gates_for_device(circuit, device, resource_policy=None)

在设备容量约束下分解多控制门。此入口限制线路的逻辑比特数不超过设备可用比特数，不检查耦合拓扑。

参数：

- `circuit` (`Circuit`)：待分解线路。
- `device` (`Device`)：提供可用物理比特容量约束。
- `resource_policy` (`ResourcePolicy | None`)：辅助比特策略。

返回：

- `TransformResult`

异常情况：

- `CompilerConfigError`：线路宽度超出设备可用比特数。

### McGateDecomposeConfig(*, resource_policy=None, resource_limits=None)

参数：

- `resource_policy` (`ResourcePolicy | None`)：辅助比特策略。
- `resource_limits` (`ResourceLimits | None`)：资源上限。

属性：

- `resource_policy -> ResourcePolicy`
- `resource_limits -> ResourceLimits`

---

## 多控制门原语

`cqlib.compile.transform.decompose.mc_gate` 提供按门族划分的精确合成原语，直接给出操作序列，不做线路遍历、不选择算法、不申请辅助比特。**正常编译流程应使用 `decompose_mc_gates` 或 `decompose_mc_gates_for_device`**；只有在需要自行控制辅助比特分配或实现自定义降级策略时，才直接调用这些原语。

原语按门族分组，每组通常提供「无辅助比特」「若干干净辅助比特」「若干脏辅助比特」以及针对单辅助比特的固定深度变体：

| 门族 | 覆盖范围 |
| --- | --- |
| MCX | 多控制 X 门。 |
| 多控制 SU(2) | 多控制特殊酉旋转，旋转轴由 `Su2RotationAxis` 给出。 |
| Pauli | 多控制 Pauli 门族。 |
| RZZ | 多控制 RZZ。 |
| Pauli 旋转 | 多控制 Pauli 旋转。 |
| 旋转 | 多控制 `RX` / `RY` / `RZ` 及其固有受控形态。 |
| 相位 | 多控制 `S` / `SDG` / `T` / `TDG` / `Phase`。 |
| QCIS | 面向 QCIS 门集的合成原语。 |
| Hadamard | 多控制 Hadamard。 |
| SWAP | 多控制 SWAP。 |
| FSIM | 多控制 FSIM。 |
| 酉门 | 多控制通用酉门。 |

共同参数：

- `controls` (`list[int | Qubit]`)：控制比特，须为展开后的全部控制，含门本身固有的控制。
- `target`，或双比特原语的 `first`、`second`：目标比特。
- `clean_ancillas` / `clean_ancilla`：干净辅助比特，须以 `|0>` 进入并恢复为 `|0>`。
- `dirty_ancillas` / `dirty_ancilla`：脏辅助比特，可以任意未知态进入，但须精确恢复。

所有原语返回 `list[ValueOperation]`，可用 `Circuit.from_operations` 组装成线路。超出消耗前缀的额外辅助比特被忽略；辅助比特必须与所有控制比特和目标比特互不相同。完整清单见 `cqlib.compile.transform.decompose.mc_gate` 的公开面。

---

## 双比特块重综合

### resynthesize_two_qubit_blocks(circuit, config=None)

重综合双比特门块，函数式入口。

参数：

- `circuit` (`Circuit`)：待重综合线路。
- `config` (`TwoQubitBlockResynthesisConfig | None`)：重综合配置。

返回：

- `TransformResult`

### ResynthesizeTwoQubitBlocks(config=None)

可复用 pass 对象。

参数：

- `config` (`TwoQubitBlockResynthesisConfig | None`)：重综合配置。

属性与方法：

- `config -> TwoQubitBlockResynthesisConfig`
- `run(circuit) -> TransformResult`

### TwoQubitBlockResynthesisConfig(*, two_qubit_basis=None, target_basis=None, enhanced=False, max_block_ops=None, max_crossed_ops=None, max_scan_span=None, skip_labeled_ops=True, recurse_control_flow=True, commutation=None)

参数：

- `two_qubit_basis` (`TwoQubitUnitaryDecomposeBasis | None`)：双比特合成门集。
- `target_basis` (`list[str | Instruction] | None`)：目标门集约束。
- `enhanced` (`bool`)：是否使用增强预算，以编译时间换更好的路由后清理。
- `max_block_ops` (`int | None`)：块内最大操作数。
- `max_crossed_ops` (`int | None`)：块内最大交叉操作数；被跨越操作保持原位，且必须与合成替换对易。
- `max_scan_span` (`int | None`)：双比特锚点单侧的收集预算。
- `skip_labeled_ops` (`bool`)：是否把带标签操作视为硬边界。
- `recurse_control_flow` (`bool`)：是否递归进入控制流块。
- `commutation` (`CommutationConfig | None`)：对易检查配置。

---

## 目标门集转换

### TargetBasisLowerer(target_basis)

把线路翻译到目标门集。

参数：

- `target_basis` (`list[str | Instruction]`)：非空目标门集，条目形式同 `compile()` 的 `target_basis`。

属性与方法：

- `target_basis -> list[Instruction]`
- `run(circuit) -> TransformResult`

### TargetBasisSignature

目标门集签名：

- `TargetBasisSignature.from_standard_gates(gates)`：从标准门列表构造签名。

### TargetBasisCost

目标门集代价统计：

- `two_qubit_ops -> int`：双比特门数量。
- `depth -> int`：深度。
- `total_ops -> int`：操作总数。
- `parameterized_ops -> int`：参数化操作数量。

### TargetBasisCostModel

目标门集代价模型：

- `signature -> TargetBasisSignature`
- `target_basis -> list[Instruction]`
- `cost_of_fixed_operations(qubits, operations) -> TargetBasisCost`：计算固定操作集合的代价。`qubits` 为比特编号或 `Qubit` 对象列表，`operations` 为 `ValueOperation` 列表。

---

## 设备降级

### DeviceLowerer(device)

按设备原生指令集降级线路。

参数：

- `device` (`Device`)：目标设备。

属性与方法：

- `device -> Device`
- `run(circuit) -> TransformResult`

---

## 示例

### 1. 定义展开与酉门合成

```python
import numpy as np

from cqlib.circuit import Circuit, UnitaryGate
from cqlib.compile.transform.decompose import (
    decompose_unitaries_with_rule_stats,
    expand_definitions,
)

definition = Circuit(1)
definition.h(0)
circuit = Circuit(1)
circuit.append_circuit_gate(definition.to_gate("custom_h"), [0])

expanded = expand_definitions(circuit)
assert expanded.changed is True
assert [op.instruction.instruction.name for op in circuit.operations] == ["custom_h"]
assert [op.instruction.instruction.name for op in expanded.circuit.operations] == ["H"]

matrix = np.array([[0, 1], [1, 0]], dtype=np.complex128)
gate = UnitaryGate("x_matrix", 1).with_matrix(matrix)
circuit = Circuit(2)
circuit.append_unitary_gate(gate, [0])
circuit.append_unitary_gate(gate, [1])

result, stats = decompose_unitaries_with_rule_stats(circuit)
assert result.changed is True
assert all(
    operation.instruction.instruction.name == "U"
    for operation in result.circuit.operations
)
assert (stats.hits, stats.misses, stats.inserts) == (1, 1, 1)
```

### 2. 数值酉门合成

```python
import numpy as np

from cqlib.circuit import Circuit, Qubit, UnitaryGate
from cqlib.compile.transform.decompose import TwoQubitUnitaryDecomposeBasis
from cqlib.compile.transform.decompose.unitary import (
    kak_decompose,
    synthesize_numeric_1q_unitary,
    synthesize_numeric_2q_unitary,
)

source = (
    np.exp(0.37j)
    * np.array([[1, 1], [1, -1]], dtype=np.complex128)
    / np.sqrt(2)
)
decomposition = synthesize_numeric_1q_unitary(source)
circuit = Circuit(1)
circuit.u(0, decomposition.theta, decomposition.phi, decomposition.lambda_)
reconstructed = np.exp(1j * decomposition.global_phase) * circuit.to_matrix()
np.testing.assert_allclose(reconstructed, source, atol=1e-10)

swap = np.array(
    [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]],
    dtype=np.complex128,
)
synthesis = synthesize_numeric_2q_unitary(
    swap, 0, Qubit(1), TwoQubitUnitaryDecomposeBasis.cx()
)
expected = Circuit(2)
expected.append_unitary_gate(UnitaryGate("source", 2).with_matrix(swap), [0, 1])
synthesized = Circuit.from_operations([Qubit(0), Qubit(1)], synthesis.operations)
synthesized.set_global_phase(synthesis.global_phase)
np.testing.assert_allclose(
    synthesized.to_matrix(), expected.to_matrix(), atol=1e-8
)

kak = kak_decompose(swap)
assert kak.k1l.shape == (2, 2)
assert isinstance(kak.global_phase, float)
```

### 3. 多控制门分解

```python
from cqlib.circuit import Circuit, MCGate, StandardGate
from cqlib.compile.resource import ResourceLimits, ResourcePolicy
from cqlib.compile.transform.decompose import (
    McGateDecomposeConfig,
    decompose_mc_gates_with_rule_stats,
)

circuit = Circuit(4)
gate = MCGate(3, StandardGate.X)
circuit.append_mc_gate(gate, [0, 1, 2, 3])
circuit.append_mc_gate(gate, [0, 1, 2, 3])

config = McGateDecomposeConfig(
    resource_policy=ResourcePolicy(
        max_pre_layout_clean_ancillas=2,
        allow_dirty_borrowing=True,
    ),
    resource_limits=ResourceLimits(max_total_qubits=8),
)
result, stats = decompose_mc_gates_with_rule_stats(circuit, config)
assert result.changed is True
assert all(
    not operation.instruction.instruction.is_mcgate
    for operation in result.circuit.operations
)
assert stats.inserts >= 1
```

### 4. 双比特块重综合与目标门集转换

```python
from cqlib.circuit import Circuit
from cqlib.compile.transform.decompose import TwoQubitUnitaryDecomposeBasis
from cqlib.compile.transform.resynthesis import (
    ResynthesizeTwoQubitBlocks,
    TwoQubitBlockResynthesisConfig,
    resynthesize_two_qubit_blocks,
)
from cqlib.compile.transform.target_basis import TargetBasisLowerer

circuit = Circuit(2)
circuit.cx(0, 1)
circuit.cx(0, 1)
config = TwoQubitBlockResynthesisConfig(
    two_qubit_basis=TwoQubitUnitaryDecomposeBasis.cx()
)

result = resynthesize_two_qubit_blocks(circuit, config)
transformer = ResynthesizeTwoQubitBlocks(config)
assert transformer.config == config
assert result.changed is True
assert len(result.circuit.operations) == 0

lowerer = TargetBasisLowerer(["h", "CZ"])
assert [instruction.name for instruction in lowerer.target_basis] == ["H", "CZ"]
```

---

## 异常情况

- `CompilerTransformError`：分解或合成失败，例如矩阵不是酉矩阵、目标门集无法覆盖输入操作、辅助比特数量不足或比特重复。
- `CompilerConfigError`：配置非法，例如空目标门集、`two_qubit_basis` 与 `target_basis` 同时给出、线路宽度超出设备容量。
- `ParameterError`：门参数非有限值。
- `ValueError`：要求恰好两个辅助比特的原语收到其他数量。
- `ResourceError`：多控制门分解申请辅助比特失败，见 [Resource](8_resource.md)。
