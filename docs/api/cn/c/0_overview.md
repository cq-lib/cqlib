# C API 概览

## 欢迎查阅 Cqlib C API 参考手册！

Cqlib C 绑定（`binding-c`）以纯 C ABI 提供量子线路构造、符号参数、IR 转换、设备建模、量子信息、编译优化、可视化与错误缓解等能力。全部接口遵循同一风格：自由函数 + 不透明句柄（opaque handle）+ `int32_t` 错误码，头文件为自动生成的 [crates/binding-c/include/cqlib_c.h](../../../../crates/binding-c/include/cqlib_c.h)。

---

## 文档导航

### 量子电路（circuit）

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [量子比特](0_circuit/2_qubit.md)
- [符号参数](0_circuit/3_parameter.md)
- [操作与指令](0_circuit/4_operation_instruction.md)
- [标准门](0_circuit/5_gate_standard.md)
- [自定义酉门](0_circuit/6_gate_unitary.md)
- [多控制门](0_circuit/7_gate_mc_gate.md)
- [复合门](0_circuit/8_gate_circuit_gate.md)
- [经典数据与控制流](0_circuit/9_classical_control_flow.md)
- [符号矩阵](0_circuit/10_symbolic_matrix.md)
- [Ansatz](0_circuit/11_ansatz.md)
- [控制流图](0_circuit/12_cfg.md)
- [线路转矩阵](0_circuit/13_circuit_to_matrix.md)
- [线路 DAG](0_circuit/14_circuit_dag.md)

### 中间表达（ir）

- [Overview](1_ir/0_overview.md)
- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### 设备模块（device）

- [Overview](2_device/0_overview.md)
- [拓扑](2_device/1_topology.md)
- [设备与噪声属性](2_device/2_properties_device.md)
- [布局](2_device/3_layout.md)
- [噪声](2_device/4_noise.md)
- [执行结果](2_device/5_result.md)
- [量子比特标识](2_device/6_qubits.md)

### 量子信息（qis）

- [Overview](3_qis/0_overview.md)
- [态矢量](3_qis/1_statevector.md)
- [密度矩阵](3_qis/2_density_matrix.md)
- [噪声密度矩阵](3_qis/3_density_matrix_noise.md)
- [稳定器](3_qis/4_stabilizer.md)
- [经典态](3_qis/5_classical_state.md)
- [Pauli](3_qis/6_pauli.md)
- [哈密顿量](3_qis/7_hamiltonian.md)
- [Pauli 演化](3_qis/8_evolution.md)
- [度量与熵](3_qis/9_metrics_entropy.md)

### 编译优化（compile）

- [Overview](4_compile/0_overview.md)
- [编译器](4_compile/1_compiler.md)
- [线路变换](4_compile/2_transform.md)
- [布局](4_compile/3_layout.md)
- [路由](4_compile/4_routing.md)
- [SABRE](4_compile/5_sabre.md)
- [分解与重综合](4_compile/6_decompose_resynthesis.md)
- [知识规则](4_compile/7_knowledge.md)
- [资源管理](4_compile/8_resource.md)
- [对易检查](4_compile/9_commutation.md)

### 可视化（visualization）

- [Overview](5_visualization/0_overview.md)
- [文本线路图](5_visualization/1_draw_text.md)
- [SVG 线路图](5_visualization/2_draw_figure.md)
- [状态图](5_visualization/3_state_plots.md)
- [结果图](5_visualization/4_result_plots.md)
- [落盘与输出](5_visualization/5_render_to_file.md)
- [Visual IR](5_visualization/6_visual_ir.md)

### 错误缓解（error_mitigation）

- [Overview](6_error_mitigation/0_overview.md)
- [零噪声外推](6_error_mitigation/1_zne.md)
- [虚拟蒸馏](6_error_mitigation/2_virtual_distillation.md)
- [统一流水线](6_error_mitigation/3_unified.md)

---

## 快速上手：Bell 态

下面的完整示例构造两比特 Bell 线路，精确模拟后用两步式接口读取概率分布，并释放全部资源：

```c
#include <cqlib_c.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* 1. 构造 Bell 线路：H(0) + CX(0, 1) */
    CCircuit *qc = circuit_new(2);
    if (qc == NULL) {
        return 1;
    }
    if (circuit_h(qc, 0) != 0 || circuit_cx(qc, 0, 1) != 0) {
        circuit_free(qc);
        return 1;
    }

    /* 2. 精确模拟 */
    CStatevector *sv = statevector_from_circuit(qc);
    if (sv == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* 3. 两步式输出：先查长度，再填充缓冲 */
    uintptr_t len = statevector_probabilities_len(sv);
    double *probs = malloc(len * sizeof(double));
    if (probs == NULL || statevector_probabilities(sv, probs, len) != 0) {
        free(probs);
        statevector_free(sv);
        circuit_free(qc);
        return 1;
    }
    /* probs = {0.5, 0.0, 0.0, 0.5} */
    printf("P(00) = %f, P(11) = %f\n", probs[0], probs[3]);

    /* 4. 释放（顺序与获取顺序相反） */
    free(probs);
    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

### 编译与链接

Windows（MinGW gcc，gnu 目标）：

```bash
cargo build --release -p binding-c --target x86_64-pc-windows-gnu

gcc -Icrates/binding-c/include main.c -Ltarget/x86_64-pc-windows-gnu/release -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 -o bell
```

运行时需要能定位 `binding_c.dll`：将 `target/x86_64-pc-windows-gnu/release` 的绝对路径加入 `PATH`，或把 DLL 复制到可执行文件旁。macOS 与 Linux 的链接方式、测试工程与更多示例见 [crates/binding-c/README.md](../../../../crates/binding-c/README.md)。

---

## 错误码

返回 `int32_t` 的接口用 `0` 表示成功，负值表示错误：

| 值 | 名称 | 典型场景 |
| --- | --- | --- |
| `0` | Ok | 成功。 |
| `-1` | NullPtr | 传入 NULL 句柄、NULL 数组或非法 C 字符串。 |
| `-2` | QubitOutOfBounds | 量子比特编号超出线路或设备范围。 |
| `-3` | CircuitError | 线路结构错误：验证失败、门作用比特数量或参数数量不匹配、同一操作重复使用比特、不可逆、作用比特重复等。 |
| `-4` | ParseError | 解析错误：未知门名、非法表达式语法、非法 UTF-8、非法 IR 文本。 |
| `-5` | IoError | 文件读写失败。 |
| `-6` | CompilerError | 编译流程错误。 |
| `-7` | SimulationError | 模拟错误：非 Clifford 门进入稳定器模拟、尚未运行等。 |
| `-8` | InvalidParam | 非法参数：概率越界、缓冲区过小、非法标签值等。 |

返回指针的接口用 `NULL` 表示失败；`param_evaluate` 求值失败（含符号未绑定）返回 `0.0`；`circuit_depth` 返回负值表示错误。布尔类查询返回 `1`/`0`，`-1` 通常表示 NULL 输入。

---

## 内存所有权与释放

1. **构造与变换返回堆句柄**：`circuit_new`、`circuit_from_qubits`、`circuit_inverse`、`circuit_decompose`、`circuit_assign_params`、`param_parse`、`circuit_to_gate` 等返回新分配的不透明句柄，调用方用对应 `*_free` 释放（`circuit_free`、`param_free`、`circuit_gate_free` 等）。所有 `*_free` 都允许传 `NULL`。
2. **字符串用 `cqlib_string_free` 释放**：凡是返回 `char*` 的接口（`operation_name`、`value_operation_name`、`circuit_gate_name`、`circuit_symbols` 写出的每个元素等），都用 `cqlib_string_free` 释放。
3. **数组采用两步式输出**：先调用 `*_len` 接口查询数量并分配缓冲，再调用填充接口。缓冲不足时的行为分两类：多数填充接口返回负错误码（通常 `-8`）；部分接口（如 `circuit_qubits`）只返回总数而不拷贝。
4. **嵌套列表配套 free**：`circuit_parameters`（每项是新分配的 `CParameter*`，用 `param_free` 逐个释放）、`circuit_cfg_block_operations`（每项用 `operation_free` 释放）等写出的每个元素都是独立句柄，必须逐个释放。
5. **显式接管所有权的接口**：`circuit_to_gate` 克隆输入线路生成复合门句柄；`circuit_index` 返回的 `CValueOperation*` 是独立于后续线路变更的只读快照（用 `value_operation_free` 释放）；`CParameterValue.tag == PARAMETER_VALUE_TAG_PARAM` 时其中的 `param` 字段是新分配的 `CParameter*`（用 `param_free` 释放）。这些对象的所有权完全归调用方。

---

## 常量标签

头文件通过 `#define` 导出以下标签常量，均为稳定 ABI 的一部分。

### 参数存储标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `CIRCUIT_PARAM_TAG_FIXED` | 0 | `CCircuitParam.tag`：固定数值，存放在 `value` 字段。 |
| `CIRCUIT_PARAM_TAG_INDEX` | 1 | `CCircuitParam.tag`：线路参数表驻留表达式，`index` 字段存放表下标。 |
| `PARAMETER_VALUE_TAG_FIXED` | 0 | `CParameterValue.tag`：固定数值，存放在 `value` 字段，`param` 为 NULL。 |
| `PARAMETER_VALUE_TAG_PARAM` | 1 | `CParameterValue.tag`：符号参数，`param` 指向新分配的 `CParameter*`。 |
| `OPERATION_PARAM_FIXED` | 0 | `operation_params` 写出的参数标签：固定数值。 |
| `OPERATION_PARAM_INDEX` | 1 | `operation_params` 写出的参数标签：所属线路参数表下标，需结合线路解析。 |

### Ansatz 拓扑与演化策略

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `ENTANGLEMENT_LINEAR` | 0 | TwoLocal 纠缠层拓扑：线性最近邻链。 |
| `ENTANGLEMENT_CIRCULAR` | 1 | 线性链加首尾环绕边。 |
| `ENTANGLEMENT_FULL` | 2 | 全连接比特对。 |
| `ENTANGLEMENT_CUSTOM` | 3 | 自定义显式控制/目标 id 对数组。 |
| `EVOLUTION_STRATEGY_EXACT` | 0 | QAOA 演化策略：逐项精确演化。 |
| `EVOLUTION_STRATEGY_AUTO` | 1 | 自动在精确与 Trotter 之间选择。 |
| `EVOLUTION_STRATEGY_TROTTER` | 2 | 显式 Trotter 乘积公式。 |
| `TROTTER_FIRST_ORDER` | 0 | 一阶乘积公式。 |
| `TROTTER_SECOND_ORDER` | 1 | 二阶乘积公式。 |
| `TROTTER_RANDOMIZED` | 2 | 随机化一阶公式。 |
| `TROTTER_MODE_FIRST_ORDER` | 0 | Trotter-Suzuki 分解模式：一阶 Lie-Trotter。 |
| `TROTTER_MODE_SECOND_ORDER` | 1 | Trotter-Suzuki 分解模式：二阶 Strang 分裂。 |

### 控制流图标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `CFG_FLOW_TRUE_BRANCH` | 1 | `if`/`while`/`for` 头的真出口边。 |
| `CFG_FLOW_FALSE_BRANCH` | 2 | 假出口边。 |
| `CFG_FLOW_UNCONDITIONAL` | 3 | 无条件跳转 / 结构化汇合边。 |
| `CFG_FLOW_CASE` | 4 | switch 精确匹配 case 边（128 位值写在 case 两半）。 |
| `CFG_FLOW_DEFAULT_CASE` | 5 | switch default 边。 |
| `CFG_FLOW_BREAK` | 6 | 结构化 `break` 边。 |
| `CFG_FLOW_CONTINUE` | 7 | 结构化 `continue` 边。 |
| `CFG_TERMINATOR_BRANCH` | 1 | 布尔分支头终结器。 |
| `CFG_TERMINATOR_FOR_LOOP` | 2 | 无符号区间循环头终结器。 |
| `CFG_TERMINATOR_SWITCH` | 3 | 精确匹配多路分支头终结器。 |
| `CFG_TERMINATOR_JUMP` | 4 | 无条件跳转终结器。 |
| `CFG_TERMINATOR_BREAK` | 5 | 结构化 `break` 终结器。 |
| `CFG_TERMINATOR_CONTINUE` | 6 | 结构化 `continue` 终结器。 |
| `CFG_TERMINATOR_RETURN` | 7 | 执行结束终结器。 |
| `CFG_REGION_IF` | 1 | 结构化条件区域。 |
| `CFG_REGION_WHILE` | 2 | 结构化 while 循环区域。 |
| `CFG_REGION_FOR` | 3 | 结构化区间循环区域。 |
| `CFG_REGION_SWITCH` | 4 | 结构化精确匹配 switch 区域。 |

### 经典类型与表达式节点

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `CQLIB_CLASSICAL_TYPE_BIT` | 0 | 经典类型 `Bit`。 |
| `CQLIB_CLASSICAL_TYPE_BOOL` | 1 | 经典类型 `Bool`。 |
| `CQLIB_CLASSICAL_TYPE_UINT` | 2 | 经典类型 `UInt`。 |
| `CQLIB_CLASSICAL_TYPE_BIT_VEC` | 3 | 经典类型 `BitVec`。 |

`classical_expr_kind` 写出的节点类型标签：`CQLIB_CLASSICAL_EXPR_VAR`（0）、`_VALUE`（1）、`_BOOL_LITERAL`（2）、`_BIT_LITERAL`（3）、`_UINT_LITERAL`（4）、`_BIT_VEC_LITERAL`（5）、`_UNARY`（6）、`_BINARY`（7）、`_COMPARE`（8）、`_CAST`（9）、`_SELECT`（10）、`_EXTRACT_BIT`（11）、`_EXTRACT_BITS`（12）、`_CONCAT`（13）、`_PACK_BITS`（14）。

### DAG 线标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `DAG_WIRE_QUBIT` | 0 | 量子比特时间线，负载为比特 id。 |
| `DAG_WIRE_CLASSICAL_VAR` | 1 | 可变经典存储线，负载为变量 id。 |
| `DAG_WIRE_CLASSICAL_VALUE` | 2 | 不可变经典值线，负载为值 id。 |
| `DAG_WIRE_GLOBAL_ORDER` | 3 | 全局顺序资源线，无负载。 |

### 编译配置标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `COMPILE_MODE_NORMAL` | 0 | 普通编译模式。 |
| `COMPILE_MODE_ENHANCED` | 1 | 增强编译模式。 |
| `COMPILE_TARGET_LOGICAL` | 0 | 逻辑比特目标。 |
| `COMPILE_TARGET_BASIS` | 1 | 基门集目标。 |
| `COMPILE_TARGET_DEVICE` | 2 | 设备目标。 |
| `COMPILE_TARGET_TOPOLOGY_BASIS` | 3 | 拓扑基门集目标。 |

### 编译 pass 标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `REWRITE_MODE_OPTIMIZE` | 0 | 知识重写模式：优化。 |
| `REWRITE_MODE_LOWERING` | 1 | 知识重写模式：向目标门集降低。 |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | 布局目标：仅拓扑（距离 + 方向失配）。 |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | 布局目标：保真度感知（叠加错误率项）。 |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | 布局目标：由设备校准数据自动选择。 |
| `VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS` | 0 | VF2 边要求：仅正权重交互。 |
| `VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS` | 1 | VF2 边要求：全部交互。 |
| `SABRE_MAX_LOOKAHEAD_WEIGHTS` | 8 | `CSabreConfigC.lookahead_weights` 数组的最大长度。 |
| `TWO_QUBIT_BASIS_PAULI_ROTATIONS` | 0 | 双比特综合目标基：Pauli 旋转。 |
| `TWO_QUBIT_BASIS_CX` | 1 | 双比特综合目标基：CX。 |
| `TWO_QUBIT_BASIS_CY` | 2 | 双比特综合目标基：CY。 |
| `TWO_QUBIT_BASIS_CZ` | 3 | 双比特综合目标基：CZ。 |
| `TWO_QUBIT_BASIS_RZZ` | 4 | 双比特综合目标基：RZZ。 |
| `KAK_FACTOR_K1L` | 0 | KAK 分解局域因子：第一个局域门（左）。 |
| `KAK_FACTOR_K1R` | 1 | KAK 分解局域因子：第一个局域门（右）。 |
| `KAK_FACTOR_K2L` | 2 | KAK 分解局域因子：第二个局域门（左）。 |
| `KAK_FACTOR_K2R` | 3 | KAK 分解局域因子：第二个局域门（右）。 |

### 知识规则与资源标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `RULE_KIND_SIMPLIFY` | 0 | 知识规则类别：化简。 |
| `RULE_KIND_CANCEL` | 1 | 知识规则类别：消相。 |
| `RULE_KIND_MERGE` | 2 | 知识规则类别：合并。 |
| `RULE_KIND_COMMUTE` | 3 | 知识规则类别：对易换位。 |
| `RULE_KIND_DECOMPOSE` | 4 | 知识规则类别：分解。 |
| `RULE_KIND_CANONICALIZE` | 5 | 知识规则类别：规范化。 |
| `RULE_KIND_HARDWARE_NATIVE` | 6 | 知识规则类别：硬件原生。 |
| `RULE_KIND_OTHER` | 7 | 知识规则类别：其他。 |
| `RESOURCE_REQUIREMENT_CLEAN_ZERO` | 0 | 资源需求：\|0⟩ 辅助比特（干净比特）。 |
| `RESOURCE_REQUIREMENT_DIRTY` | 1 | 资源需求：脏比特（任意态辅助比特）。 |

### 对易检查标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `COMMUTATION_RESULT_UNPROVEN` | 0 | 未能证明对易。 |
| `COMMUTATION_RESULT_EXACT` | 1 | 严格对易。 |
| `COMMUTATION_RESULT_UP_TO_GLOBAL_PHASE` | 2 | 在全局相位意义下对易。 |

### 噪声通道标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `NOISE_BIT_FLIP` | 0 | 单比特翻转通道，参数为翻转概率 p。 |
| `NOISE_PHASE_FLIP` | 1 | 相位翻转通道，参数为翻转概率 p。 |
| `NOISE_DEPOLARIZING` | 2 | 去极化通道，参数为去极化参数 p。 |
| `NOISE_AMPLITUDE_DAMPING` | 3 | 幅度阻尼通道，参数为阻尼系数 gamma。 |
| `NOISE_PHASE_DAMPING` | 4 | 相位阻尼通道，参数为散射概率。 |
| `NOISE_PAULI` | 5 | 一般单比特 Pauli 通道，参数为 px/py/pz（各自非负，和 ≤ 1）。 |
| `NOISE_TWO_DEPOLARIZING` | 0 | 双比特去极化通道。 |
| `NOISE_TWO_INDEPENDENT` | 1 | 双比特独立通道（q0/q1 各带一个单比特通道）。 |
| `NOISE_TWO_CORRELATED_PAULI` | 2 | 关联 Pauli 通道（概率 p 加 `NOISE_PAULI_OP_*` 算符）。 |
| `NOISE_PAULI_OP_I` / `_X` / `_Y` / `_Z` | 0 / 1 / 2 / 3 | 关联 Pauli 通道作用在 q0/q1 上的 Pauli 算符。 |

### 执行状态标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `EXECUTION_STATUS_QUEUED` | 0 | 任务已提交排队。 |
| `EXECUTION_STATUS_RUNNING` | 1 | 任务正在运行。 |
| `EXECUTION_STATUS_COMPLETED` | 2 | 任务成功完成。 |
| `EXECUTION_STATUS_FAILED` | 3 | 任务失败（错误码与消息另行查询）。 |
| `EXECUTION_STATUS_CANCELLED` | 4 | 任务已取消。 |

### 错误缓解标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `MITIGATION_ZNE` | 0 | 缓解方法：零噪声外推（附加参数为折叠层数组）。 |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | 缓解方法：虚拟蒸馏（附加参数为副本数）。 |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | 缓解流程：ZNE + 多项式外推。 |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | 缓解流程：ZNE + 指数外推。 |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | 缓解流程：虚拟蒸馏。 |
| `ZNE_EXTRAPOLATE_POLYNOMIAL` | 0 | `zne_extrapolate` 外推方法：多项式。 |
| `ZNE_EXTRAPOLATE_EXPONENTIAL` | 1 | `zne_extrapolate` 外推方法：指数。 |

### Pauli 算符标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `PAULI_I` | 0 | 单位算符 I。 |
| `PAULI_X` | 1 | Pauli X。 |
| `PAULI_Y` | 2 | Pauli Y。 |
| `PAULI_Z` | 3 | Pauli Z。 |

### 参数显示模式标签

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `PARAM_MODE_NUMERIC` | 0 | 数值显示模式。 |
| `PARAM_MODE_SYMBOLIC` | 1 | 符号显示模式。 |
| `PARAM_MODE_SYMBOLIC_WITH_VALUE` | 2 | 符号加数值显示模式。 |
| `PARAM_MODE_PI_FRACTION_PREFERRED` | 3 | 优先 π 分数显示模式。 |

---

## 线程与 Complex64 布局

### 线程与并发

- 无全局初始化或清理调用：加载动态库（Windows 为 `binding_c.dll`，Linux 为 `libbinding_c.so`，macOS 为 `libbinding_c.dylib`）后直接调用接口。
- 句柄是堆上独立对象，所有权归调用方；多线程共享同一句柄时，访问的串行化由调用方负责。不同句柄之间相互独立，可各自使用。

### Complex64 布局

```c
typedef struct Complex64 {
  double re;
  double im;
} Complex64;
```

复数以 `(re, im)` 交错的连续双精度对表示。矩阵类接口（`circuit_to_matrix`、`operation_matrix` 等）按行主序输出交错双精度数组，缓冲区需容纳 `2 × 元素个数` 个 `double`。
