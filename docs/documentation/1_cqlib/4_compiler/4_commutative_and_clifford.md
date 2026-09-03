# 对易与 Clifford-RZ 优化

对易分析用于判断两个具体操作是否可以交换顺序。它本身不一定直接减少门数，但可以为后续规则匹配、旋转合并、取消冗余门和目标门集整理创造条件。Clifford-RZ 优化常见于包含 `H`、`S`、`X`、`Z`、`CX`、`CZ` 与 `RZ`、`Phase`、`T` 等相位门的线路片段。

日常使用时，推荐直接调用 `compile()`。编译器会在合适的阶段自动使用对易判断、规范化和知识规则重写。只有在编写自定义分析或调试优化规则时，才需要直接使用 `CommutationChecker`。

`compile()` 和对易检查接口都不会修改输入线路；它们会返回新的结果对象或证明对象。对易检查只负责回答“是否能安全交换这两个操作”，并不是一个直接改写整条线路的优化步骤。

---

## 对易检查入口

在 Python 中，对易检查以 `ValueOperation` 为输入。每个 `ValueOperation` 都是一条完整的操作应用，包含门、作用比特和参数。

```python
from cqlib.circuit import Parameter, Qubit, StandardGate, ValueOperation
from cqlib.compile.commutation import CommutationChecker

lhs = ValueOperation.from_standard_gate(
    StandardGate.CX,
    [Qubit(0), Qubit(1)],
)
rhs = ValueOperation.from_standard_gate(
    StandardGate.RZ(Parameter("theta")),
    [Qubit(0)],
)

checker = CommutationChecker.builtin()
proof = checker.check(lhs, rhs)

if proof is not None:
    print("exact:", proof.is_exact())
    print("phase:", proof.phase)
```

也可以使用共享的函数式入口：

```python
from cqlib.compile.commutation import check_commutation

proof = check_commutation(lhs, rhs)
```

返回值可能是：

- 精确对易证明：表示可以精确交换；
- 带全局相位的对易证明：表示交换会引入一个全局相位；
- `None`：表示当前检查器无法证明可交换，不等价于已经证明不可交换。

这种保守语义很重要。编译器只在有证明时使用对易结论，避免因为证明覆盖不足而破坏线路语义。

---

## 检查顺序

内建检查器按固定顺序尝试证明：

1. 快速局部事实：恒等门、全局相位、作用在不同比特集合上的门、完全相同的操作；
2. 代数证明：Pauli 轴、Pauli 旋转、对角门、受控单轴门和部分对称双比特门；
3. 规则库证明：从内建知识规则中提取 `A; B -> B; A` 形式的显式交换规则；
4. 小规模矩阵检查：在比特数限制内构造局部矩阵，验证 `AB` 与 `BA` 是否相等或只差全局相位。

默认配置会启用规则库证明和矩阵回退检查，并将矩阵检查的联合支撑比特数限制为 4。

```python
from cqlib.compile.commutation import CommutationChecker, CommutationConfig

config = CommutationConfig(
    enable_rule_oracle=True,
    enable_matrix_fallback=True,
    max_matrix_qubits=4,
)

checker = CommutationChecker.with_config(config)
proof = checker.check(lhs, rhs)
```

如果在大规模静态分析中更关注速度，可以关闭矩阵回退检查；如果希望完全依赖结构规则，也可以同时关闭规则库证明和矩阵检查，只保留基础结构事实和代数证明。

---

## 常见对易关系

编译器可以识别多类常见关系：

- 作用在不相交比特集合上的两个门总是精确对易；
- 对角门之间对易，例如 `RZ`、`Phase`、`S`、`T`、`CZ`、`RZZ`；
- 同轴旋转对易，例如连续的 `RZ(a)` 与 `RZ(b)`；
- Pauli 字符串旋转在反对易位置为偶数时对易；
- `CX` 与控制位上的 Z 轴操作对易；
- `CX` 与目标位上的 X 轴操作对易；
- `CZ` 与任一端点上的 Z 轴操作对易；
- `SWAP`、`FSIM`、对称 Pauli 旋转等部分双比特门族在相同无序比特对上有额外的结构化对易事实。

这些结论会帮助规则重写器把可合并的门移动到一起。例如：

```text
RZ(a) q; S q; RZ(b) q
```

因为 `RZ` 与 `S` 同属 Z 轴相位族，可以整理为：

```text
S q; RZ(a) q; RZ(b) q
```

随后旋转合并规则可以把两个 `RZ` 合并为 `RZ(a + b)`。

---

## Clifford-RZ 片段的优化思路

Clifford-RZ 线路通常由离散 Clifford 门和 Z 轴旋转交替组成。优化目标不是简单地把所有门重新排序，而是在保持语义的前提下做局部整理：

- 抵消相邻自反门，例如 `H H`、`X X`、`CX CX`；
- 抵消逆门对，例如 `S SDG`、`T TDG`；
- 合并同轴旋转，例如 `RZ(a) RZ(b) -> RZ(a + b)`；
- 删除零角度旋转和恒等门；
- 把 `S S`、`T T` 等 Clifford/相位组合改写为更短形式；
- 在目标门集转换前后保留或折叠必要的 `GPhase`。

这些优化分别由规范化器和知识规则重写器负责。用户一般通过 `compile()` 间接使用它们，不需要寻找单独的 `CliffordRzOptimization` 类。

```python
from cqlib.circuit import Circuit
from cqlib.compile import CompileMode, compile

circuit = Circuit(1)
circuit.rz(0, 0.1)
circuit.rz(0, 0.2)
circuit.rz(0, -0.3)

result = compile(circuit, mode=CompileMode.enhanced())
optimized = result.circuit

print("changed:", result.changed)
print("before:", len(circuit.operations))
print("after:", len(optimized.operations))
```

`CompileMode.enhanced()` 会使用更高的规则搜索、重综合、native 固定点优化和 SABRE 搜索预算，并在路由和目标门集转换后增加清理步骤。对于严格 `Device` 目标，它还会让多个路由候选分别完成后续 lowering、优化和验证，再通过有界 Pareto beam 选择最终结果。对于 Clifford-RZ 片段较多、路由后容易暴露新相邻门的线路，增强模式通常更容易找到额外的合并或抵消机会。

---

## 与模板规则的配合

对易分析和模板优化的关系可以理解为：

1. 对易检查证明两个相邻操作可以交换；
2. 交换后，原本被隔开的局部模式变成相邻模式；
3. 知识规则库匹配该模式；
4. 重写器应用取消、合并或归一化规则；
5. 规范化器清理参数、全局相位和表示细节。

例如 Z 轴门族的对易规则可以把 `RZ`、`S`、`T`、`Phase` 排列到更容易合并或抵消的位置。`CX` 和 `CZ` 的对易规则则能在两比特门附近移动单比特相位门，减少后续目标门集转换中的冗余。

---

## 全局相位

某些交换或抵消只在全局相位意义下成立。Cqlib 用 `Commutation` 的 phase 信息和 `GPhase` 显式表示这类情况。顶层 `GPhase` 在规范化阶段会被合并到线路的 `global_phase`；控制流 body 内的相位不能随意提升为全局相位，因此会保留在 body 表示中。

在算法验证、态矢量比较和门矩阵比较中，需要明确是否忽略全局相位。对于采样概率而言，全局相位通常不可观测；对于受控子线路、相位估计和进一步封装为复合门的场景，则应谨慎保留。

---

## 使用建议

- 用户级编译优先使用 `compile()`，让工作流自动安排对易、规则重写和规范化的顺序；
- 自定义分析中使用 `CommutationChecker.builtin()`，并把 `None` 当作“未证明”而不是“不可交换”；
- 大线路批量分析时可以关闭矩阵回退检查，以避免局部矩阵构造带来的开销；
- Clifford-RZ 优化后应记录门数、双比特门数、线路深度和全局相位变化；
- 对关键线路建议使用矩阵等价或状态等价测试确认优化前后语义一致。

---

## 下一步

- [编译优化](0_overview.md)：回顾完整 `compile()` 管线与各阶段配置要点。
- [模板匹配与知识规则优化](3_template_optimization.md)：了解知识规则重写如何与对易分析配合。
