# 编译优化

`cqlib.compile`

`cqlib.compile` 是 Cqlib 的编译管线模块。它以一条端到端的 `compile()` 入口覆盖从逻辑线路到满足目标约束的线路（设备目标下即物理线路）的完整流程，同时把各个编译阶段拆成可独立调用的 transform 子模块，供需要精细控制编译过程的调用方使用。

## Overview

编译是把给定输入线路改写为匹配特定量子设备拓扑、并针对执行目标优化的过程。多数线路都必须经过一系列变换才能兼容目标设备，并降低噪声对结果的影响。Cqlib 的推荐入口是 `compile()` 函数：一次调用完成规范化、知识规则优化、门分解、可选布局与路由、目标门集翻译和最终校验。

### 编译管线

`compile()` 按固定顺序编排以下阶段（阶段名对应 `CompileResult.steps` 中每条记录的 `stage` 字段）：

1. **目标解析与资源校验**（`pre_init`）— 解析目标约束（`resolve.target`），校验辅助比特资源策略（`validate.resources`）。
2. **输入规范化**（`init`）— 对输入线路做生产级规范化（`canonicalize.input`）。
3. **定义展开与预分解优化**（`optimization`）— 先展开用户自定义门定义，让知识规则看到复合门内部的操作；随后执行预分解知识规则重写（`optimize.pre_decomposition`）。
4. **门分解**（`optimization`）— 矩阵酉门合成、多控制门分解（按资源策略申请 ancilla）、分解后规范化、基于对易性的自反门抵消（`optimize.commutative_cancellation`）、双比特块重综合与单比特优化。
5. **分解后优化**（`optimization`）— 知识规则重写迭代至不动点（`optimize.post_decomposition`）。
6. **路由门集降级**（`translation`）— 把超出 SABRE 门元数约束的门（如 `CCX`）降级为至多两比特的操作。
7. **布局与路由**（`routing`）— 在设备目标下选择初始布局，插入 SWAP 满足拓扑邻接约束（`route.sabre`），路由后执行清理与重综合。
8. **目标门集转换**（`translation`）— 把线路降级到目标门集，设备目标进一步降级到原生指令集。
9. **原生优化与最终校验**（`optimization` / `validation`）— 对原生指令做不动点优化（`optimize.native_fixed_point`），设备目标最后执行设备校验（`validate.device`）。

没有设备目标时，布局、路由与设备校验步骤会被跳过，并在 `steps` 中记录跳过原因。每个阶段都可以跳过 `compile()` 单独调用，见 [Transform](2_transform.md) 及后续页面。

### 选择优化力度

`CompileMode.normal()` 是保守的逻辑优化，使用生产默认 pass 参数；`CompileMode.enhanced()` 使用更强的分阶段工作流、更大的 pass 预算，并在有目标约束时执行目标感知清理。对大多数用例，`normal` 已足够；当编译时间预算充裕、希望换取更好的路由后清理效果时使用 `enhanced`。

### 选择目标约束

- `CompileTarget.logical()`：只做逻辑优化，产物不绑定任何硬件。适合算法层面的线路化简。
- `CompileTarget.basis([...])`：降级到显式标准门集，不涉及设备拓扑。
- `CompileTarget.device(device)`：针对具体设备的路由与降级，产物必须匹配设备原生能力，并执行最终设备校验。
- `CompileTarget.topology_basis(device, [...])`：在设备拓扑上路由，同时降级到显式门集；设备只提供容量、布局与路由约束，不强制门集匹配设备原生能力。

### 可复现性

编译中的布局与路由包含随机启发式。传入 `seed` 参数（`compile()`、`CompileTarget.device()`、`CompileTarget.topology_basis()` 均支持）可获得确定性结果：相同线路、相同配置与相同 seed 产生相同 cqlib 结果。不同 cqlib 版本之间的产物不保证一致；seed 只保证同版本内的可重复性。

---

## 常用入口

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, compile
from cqlib.device import Device

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 2)

device = Device.line("line-3", 3)

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    target_basis=["H", "CX", "RZ"],
    seed=42,
)

print("changed:", result.changed)
for step in result.steps:
    if step.changed and not step.skipped:
        print(step.stage, step.name)
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **编译管线** | 把抽象线路逐步转换为满足目标约束的线路的流程，包括逻辑优化、分解、布局、路由和目标门集翻译等阶段。 |
| **Pass** | 编译管线中的一个独立变换步骤，例如规范化、知识规则重写或路由。多数 pass 既可以由 `compile()` 自动编排，也可以单独调用。 |
| **Transform** | 一个可复用的变换对象，例如 `Canonicalizer`、`KnowledgeRewriter`。调用 `run()` 得到结果对象，输入线路保持不变。 |
| **目标约束** | 编译产物需要满足的条件，包括逻辑空间（无约束）、标准门集（`CompileTarget.basis`）、物理设备（`CompileTarget.device`）以及设备拓扑加门集（`CompileTarget.topology_basis`）。 |
| **初始布局** | 编译开始前逻辑比特到物理比特的映射。可由调用方提供，也可由布局算法自动选择。 |
| **路由** | 在受限拓扑上通过插入 SWAP 等移动操作使所有二体交互满足邻接约束的过程。 |
| **门分解** | 把复合门、自定义酉门或多控制门展开为更基础的门序列。 |
| **知识规则** | 描述相邻操作模式及其替换形式的规则，用于模式化简、抵消和硬件原生门匹配。 |
| **Ancilla** | 分解过程中按需申请的辅助量子比特。干净 ancilla 要求进出变换时处于 `\|0>`，脏 ancilla 要求完整还原其原状态。 |

---

## `cqlib.compile` API 概览

### 端到端编译入口

| 名字 | 简介 |
| --- | --- |
| [`compile`](1_compiler.md) | 推荐入口，一次调用完成配置、执行和结果报告。 |
| [`CompileConfig`](1_compiler.md) | 编译配置快照，包含模式、目标约束和资源策略。 |
| [`CompileMode`](1_compiler.md) | 优化力度枚举，包括 `normal` 与 `enhanced`。 |
| [`CompileTarget`](1_compiler.md) | 目标约束枚举，包括逻辑、门集、设备与拓扑加门集。 |
| [`DeviceCompileTarget`](1_compiler.md) | 设备目标输入，包含设备、可选初始布局与随机种子。 |
| [`CompileResult`](1_compiler.md) | 编译结果，包含优化后线路、分步报告和设备布局元数据。 |
| [`WorkflowStepReport`](1_compiler.md) | 单个工作流步骤的执行记录。 |
| [`DeviceCompilationMetadata`](1_compiler.md) | 设备编译产生的初始与最终布局。 |
| [`CompilerWorkflow`](1_compiler.md) | 可复用的工作流对象，按配置反复执行编译。 |

### 基础 transform pass

| 名字 | 简介 |
| --- | --- |
| [`TransformResult`](2_transform.md) | transform 通用结果，包含输出线路与是否变更标志。 |
| [`CircuitAnalysis`](2_transform.md) | 线路特征快照，例如是否含测量、控制流、复合门等。 |
| [`Canonicalizer`](2_transform.md) / [`canonicalize_circuit`](2_transform.md) | 规范化 pass，统一指令形式、消除空操作、折叠全局相位。 |
| [`KnowledgeRewriter`](2_transform.md) / [`rewrite_circuit`](2_transform.md) | 基于知识规则的模式重写 pass。 |
| [`OptimizeOneQubitRuns`](2_transform.md) | 单比特连续门串优化。 |
| [`CommutativeCancellation`](2_transform.md) | 基于对易性的自反门抵消。 |
| [`LowerToRoutingBasis`](2_transform.md) / [`lower_to_routing_basis`](2_transform.md) | 路由门集降级，保证路由阶段只面对可移动的门。 |

### 布局与路由

| 名字 | 简介 |
| --- | --- |
| [`trivial_layout`](3_layout.md) / [`greedy_layout`](3_layout.md) / [`vf2_perfect_layout`](3_layout.md) / [`sabre_layout`](3_layout.md) | 四种初始布局算法。 |
| [`LayoutObjective`](3_layout.md) / [`LayoutScore`](3_layout.md) | 布局目标权重与打分结果。 |
| [`CircuitLayoutAnalysis`](3_layout.md) / [`InteractionGraph`](3_layout.md) | 线路交互图分析，布局算法的输入。 |
| [`PhysicalLayoutGraph`](3_layout.md) / [`DistanceTable`](3_layout.md) | 设备物理图与最短距离表。 |
| [`route_with_layout`](4_routing.md) / [`route_sabre`](4_routing.md) | 变换层路由入口。 |
| [`RoutedCircuit`](4_routing.md) / [`SabreRouteResult`](4_routing.md) | 路由结果对象。 |
| [`SabreConfig`](5_sabre.md) / [`SabreHeuristicConfig`](5_sabre.md) / [`SabreVf2PrepassConfig`](5_sabre.md) | SABRE 算法配置。 |
| [`sabre_route`](5_sabre.md) | 已知初始布局条件下的 SABRE 路由。 |

### 分解、重综合与目标门集

| 名字 | 简介 |
| --- | --- |
| [`expand_definitions`](6_decompose_resynthesis.md) / [`decompose_unitaries`](6_decompose_resynthesis.md) / [`decompose_mc_gates`](6_decompose_resynthesis.md) | 定义展开、酉门合成与多控制门分解。 |
| [`UnitaryDecomposeConfig`](6_decompose_resynthesis.md) / [`McGateDecomposeConfig`](6_decompose_resynthesis.md) | 分解配置。 |
| [`ResynthesizeTwoQubitBlocks`](6_decompose_resynthesis.md) / [`resynthesize_two_qubit_blocks`](6_decompose_resynthesis.md) | 双比特块重综合。 |
| [`TargetBasisLowerer`](6_decompose_resynthesis.md) / [`TargetBasisCostModel`](6_decompose_resynthesis.md) | 目标门集转换与代价评估。 |
| [`DeviceLowerer`](6_decompose_resynthesis.md) | 设备原生指令降级。 |

### 知识规则与资源管理

| 名字 | 简介 |
| --- | --- |
| [`Rule`](7_knowledge.md) / [`RuleItem`](7_knowledge.md) / [`Condition`](7_knowledge.md) | 知识规则构造。 |
| [`RuleLibrary`](7_knowledge.md) / [`RuleMetadata`](7_knowledge.md) / [`RuleKind`](7_knowledge.md) | 规则库验证、分类与查询。 |
| [`loads`](7_knowledge.md) / [`load`](7_knowledge.md) / [`dumps`](7_knowledge.md) / [`dump`](7_knowledge.md) | 规则 DSL 读写。 |
| [`rule_matches_operations`](7_knowledge.md) / [`MatchBindings`](7_knowledge.md) | 规则结构匹配。 |
| [`ResourceManager`](8_resource.md) / [`ResourcePolicy`](8_resource.md) / [`ResourceLimits`](8_resource.md) | Ancilla 资源规划与租借。 |
| [`Commutation`](9_commutation.md) / [`CommutationChecker`](9_commutation.md) / [`check_commutation`](9_commutation.md) | 对易性保守证明。 |

---

## 快速示例

### 1. 端到端编译

```python
from cqlib import Circuit
from cqlib.compile import CompileMode, compile
from cqlib.device import Device

circuit = Circuit(3)
circuit.h(0)
circuit.cx(0, 2)

device = Device.line("line-3", 3)

result = compile(
    circuit,
    mode=CompileMode.enhanced(),
    device=device,
    target_basis=["H", "CX", "RZ"],
    seed=42,
)

print("changed:", result.changed)
for step in result.steps:
    if step.changed and not step.skipped:
        print(step.stage, step.name)
```

### 2. 单独执行一个 transform

```python
from cqlib import Circuit
from cqlib.compile.transform import canonicalize_circuit

circuit = Circuit(1)
circuit.i(0)

result = canonicalize_circuit(circuit)
assert result.circuit is not circuit
assert len(circuit.operations) == 1
assert len(result.circuit.operations) == 0
assert result.changed
```

### 3. SABRE 路由

```python
from cqlib import Circuit
from cqlib.compile.sabre import SabreConfig, sabre_route
from cqlib.device import Device, Layout

circuit = Circuit(2)
circuit.cx(0, 1)

result = sabre_route(
    circuit,
    Device.line("line-3", 3),
    Layout.from_pairs([(0, 0), (1, 2)], physical_count=3),
    SabreConfig(routing_trials=1, seed=7),
)
assert result.swap_count == 1
```

---

## 校验与错误处理

编译过程在配置解析、变换执行和设备校验阶段都可能失败。常见异常包括：

| 异常 | 触发场景 |
| --- | --- |
| `CompilerConfigError` | 编译配置非法，例如 `target` 与 `target_basis` / `device` 同时提供、未知的 mode 名称、空目标门集。 |
| `CompilerTransformError` | 某个变换 pass 执行失败，例如分解或路由无法在给定约束下完成。 |
| `CompilerInternalError` | 编译内部一致性检查失败。 |
| `CompilerError` | 上述异常的公共基类。 |

设备相关错误请参考设备模块文档；知识规则解析错误请参考 [Knowledge](7_knowledge.md)；资源不足错误请参考 [Resource](8_resource.md)。
