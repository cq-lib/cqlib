# 分解与重综合（C）

本页覆盖线路级分解 pass（电路门定义展开 `decompose_expand_definitions`、矩阵酉门合成 `decompose_unitaries`、多控制门改写 `decompose_mc_gates`）、双比特块重综合 `resynthesize_two_qubit_blocks`、数值酉矩阵合成原语（单比特 / 双比特）、酉合成目标 `decompose_unitary_target_*`、KAK（Weyl）分解、目标门集降级 `target_basis_lowerer_*` 与设备降级 `decompose_lower_to_device`。分解按操作可用的表示分层：定义展开先于数值合成执行，使自带实现线路的酉门在进入矩阵合成之前已经展开。矩阵输入使用 `Complex64`（`{double re; double im}` 交错对，行主序），布局说明见 [酉门](../0_circuit/6_gate_unitary.md)。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量

双比特合成基组标签（`synthesize_numeric_2q_unitary` 的 `basis` 参数）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `TWO_QUBIT_BASIS_PAULI_ROTATIONS` | 0 | 局域 `U` + `RXX`/`RYY`/`RZZ` 泡利旋转。 |
| `TWO_QUBIT_BASIS_CX` | 1 | 局域 `U` + `CX` 模板。 |
| `TWO_QUBIT_BASIS_CY` | 2 | 局域 `U` + `CY` 模板。 |
| `TWO_QUBIT_BASIS_CZ` | 3 | 局域 `U` + `CZ` 模板。 |
| `TWO_QUBIT_BASIS_RZZ` | 4 | 局域 `U` + `RZZ`。 |

KAK 局域因子标签（`kak_decomposition_local_factor` 的 `factor` 参数）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `KAK_FACTOR_K1L` | 0 | `k1l`（交互之后作用的左侧局域因子）。 |
| `KAK_FACTOR_K1R` | 1 | `k1r`（交互之后作用的右侧局域因子）。 |
| `KAK_FACTOR_K2L` | 2 | `k2l`（交互之前作用的左侧局域因子）。 |
| `KAK_FACTOR_K2R` | 3 | `k2r`（交互之前作用的右侧局域因子）。 |

---

## 配置构造

### decompose_unitary_config_default()

按值返回默认酉门分解配置：`recurse_control_flow=1`（递归进入控制流体）。

```c
struct CUnitaryDecomposeConfig decompose_unitary_config_default(void);
```

返回：默认配置结构体（按值，无需释放）。

### mc_gate_config_default()

按值返回默认多控制门分解配置：不预布局干净 ancilla（`max_pre_layout_clean_ancillas=0`）、不允许脏借用（`allow_dirty_borrowing=0`）、不设置总比特数硬限制（`has_max_total_qubits=0`）。

```c
struct CMcGateDecomposeConfig mc_gate_config_default(void);
```

返回：默认配置结构体（按值，无需释放）。

### resynthesis_config_normal()

按值返回常规预算的双比特块重综合配置：`max_block_ops=16`、`max_crossed_ops=4`、`max_scan_span=32`、`skip_labeled_ops=1`、`recurse_control_flow=1`、启用知识规则对易 oracle（`enable_rule_oracle=1`）、关闭矩阵回退（`enable_matrix_fallback=0`）、`max_matrix_qubits=4`。

```c
struct CResynthesisConfig resynthesis_config_normal(void);
```

返回：常规预算配置结构体（按值，无需释放）。

### resynthesis_config_enhanced()

按值返回增强预算的双比特块重综合配置：在常规预算基础上放宽块收集规模（`max_block_ops=32`、`max_crossed_ops=8`、`max_scan_span=64`），其余字段与常规预算一致。

```c
struct CResynthesisConfig resynthesis_config_enhanced(void);
```

返回：增强预算配置结构体（按值，无需释放）。

---

## 线路级分解 pass

### decompose_expand_definitions(circuit)

展开线路中电路门（自带实现线路的门）的定义，把门调用替换回其实现线路中的普通操作，返回重建线路。

```c
struct CCircuit *decompose_expand_definitions(const struct CCircuit *circuit);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或展开失败时返回 NULL。

电路门的构造见 [电路门](../0_circuit/8_gate_circuit_gate.md)。

### decompose_unitaries(circuit, config)

合成线路中带固定矩阵的酉门（`circuit_unitary` 添加的门），把矩阵替换为标准门操作序列，返回重建线路。等价于 `stats` 传 NULL 的 `decompose_unitaries_with_rule_stats`。

```c
struct CCircuit *decompose_unitaries(const struct CCircuit *circuit,
                                     struct CUnitaryDecomposeConfig config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `config` | `struct CUnitaryDecomposeConfig` | 分解配置（按值）。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或合成失败时返回 NULL。

矩阵酉门的构造见 [酉门](../0_circuit/6_gate_unitary.md)。

### decompose_unitaries_with_rule_stats(circuit, config, stats)

`decompose_unitaries` 的诊断形式：当 `stats` 非 NULL 时，把本次运行 pass 局部分解规则缓存的统计快照写入 `*stats`。

```c
struct CCircuit *decompose_unitaries_with_rule_stats(const struct CCircuit *circuit,
                                                     struct CUnitaryDecomposeConfig config,
                                                     struct CDecompositionRuleStats *stats);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `config` | `struct CUnitaryDecomposeConfig` | 分解配置（按值）。 |
| `stats` | `struct CDecompositionRuleStats*` | 规则缓存统计输出；NULL 表示不需要统计。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或合成失败时返回 NULL（此时 `*stats` 保持不变）。

### decompose_mc_gates(circuit, config)

改写线路中的多控制门（`circuit_multi_control` 添加的门），将其替换为单控制 / 无控制操作序列，返回重建线路。等价于 `stats` 传 NULL 的 `decompose_mc_gates_with_rule_stats`。

```c
struct CCircuit *decompose_mc_gates(const struct CCircuit *circuit,
                                    struct CMcGateDecomposeConfig config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `config` | `struct CMcGateDecomposeConfig` | 资源与限制配置（按值）。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或改写失败时返回 NULL。

多控制门的构造见 [多控制门](../0_circuit/7_gate_mc_gate.md)。

### decompose_mc_gates_with_rule_stats(circuit, config, stats)

`decompose_mc_gates` 的诊断形式：当 `stats` 非 NULL 时，把本次运行 pass 局部分解规则缓存的统计快照写入 `*stats`。

```c
struct CCircuit *decompose_mc_gates_with_rule_stats(const struct CCircuit *circuit,
                                                    struct CMcGateDecomposeConfig config,
                                                    struct CDecompositionRuleStats *stats);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `config` | `struct CMcGateDecomposeConfig` | 资源与限制配置（按值）。 |
| `stats` | `struct CDecompositionRuleStats*` | 规则缓存统计输出；NULL 表示不需要统计。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或改写失败时返回 NULL（此时 `*stats` 保持不变）。

### decompose_mc_gates_for_device(circuit, device, policy)

在设备可用比特容量内改写线路中的多控制门（预布局逻辑变换）：改写方案受设备的可用物理比特数约束。`policy` 可为 NULL，此时选用默认资源策略。

```c
struct CCircuit *decompose_mc_gates_for_device(const struct CCircuit *circuit,
                                               const struct CDevice *device,
                                               const struct CResourcePolicy *policy);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `device` | `const CDevice*` | 目标设备（构造方式见 [设备与噪声属性](../2_device/2_properties_device.md)）。 |
| `policy` | `const CResourcePolicy*` | 资源策略；NULL 选用默认值（见 [资源策略](8_resource.md)）。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；任一句柄为 NULL 或改写失败时返回 NULL。

---

## 双比特块重综合

### resynthesize_two_qubit_blocks(circuit, config)

重综合线路中的双比特块：围绕双比特锚点在有限预算内收集候选块，利用对易引擎重排并重新合成，返回重建线路。

```c
struct CCircuit *resynthesize_two_qubit_blocks(const struct CCircuit *circuit,
                                               struct CResynthesisConfig config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `config` | `struct CResynthesisConfig` | 重综合预算配置（按值）。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；`circuit` 为 NULL 或重综合失败时返回 NULL。

---

## 数值酉合成原语

### synthesize_numeric_1q_unitary(matrix, out)

把 2×2 酉矩阵合成为 Cqlib 的 `U` 门约定：表示的矩阵为 `exp(i * global_phase) * U(theta, phi, lambda)`。`matrix` 指向 4 个行主序 `Complex64` 元素。

```c
int32_t synthesize_numeric_1q_unitary(const Complex64 *matrix,
                                      struct COneQubitUnitaryDecomposition *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 4 个行主序复数元素（2×2 矩阵）。 |
| `out` | `struct COneQubitUnitaryDecomposition*` | 合成结果输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `matrix` 或 `out` 为 NULL。 |
| `-6` | CompilerError | 矩阵含非有限值、非酉，或被合成器拒绝。 |

### synthesize_numeric_2q_unitary(matrix, first, second, basis)

把 4×4 酉矩阵合成为标准门操作序列，输出到 `CTwoQubitUnitarySynthesis` 句柄。`matrix` 指向 16 个行主序 `Complex64` 元素；`first` 与 `second` 为目标比特；`basis` 为 `TWO_QUBIT_BASIS_*` 之一。

```c
struct CTwoQubitUnitarySynthesis *synthesize_numeric_2q_unitary(const Complex64 *matrix,
                                                                uint32_t first,
                                                                uint32_t second,
                                                                uint8_t basis);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 16 个行主序复数元素（4×4 矩阵）。 |
| `first` | `uint32_t` | 第一个目标比特 ID。 |
| `second` | `uint32_t` | 第二个目标比特 ID。 |
| `basis` | `uint8_t` | 合成基组，`TWO_QUBIT_BASIS_*` 之一。 |

返回：成功返回新建的 `CTwoQubitUnitarySynthesis*`（用 `two_qubit_synthesis_free` 释放）；`matrix` 为 NULL、`basis` 不是有效标签，或合成失败（例如矩阵含非有限值或非酉）时返回 NULL。

---

## 酉合成目标

`CDecomposeUnitaryTarget` 描述酉合成步骤可以发出的原生门集：原生单比特基组、原生双比特基组与 Pauli 旋转回退开关。它还能在原生基组下评估固定操作序列的精确目标基组代价。

### decompose_unitary_target_unconstrained()

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_unconstrained(void);
```

创建无约束合成目标：不带任何原生基组限制，启用中性的精确 Pauli 旋转回退。

返回：新建的 `CDecomposeUnitaryTarget*`（用 `decompose_unitary_target_free` 释放）。

### decompose_unitary_target_from_instructions(gate_names, len)

从工作流风格的目标基组列表（标准门名，如 `"H"`、`"RZ"`、`"CX"`）构造合成目标：单比特门名成为原生单比特基组，双比特门名成为原生双比特基组，精确 Pauli 旋转回退保持启用。

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_from_instructions(
    const char *const *gate_names,
    uintptr_t len);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `gate_names` | `const char* const*` | 标准门名数组；`len` 为 0 时可为 NULL。 |
| `len` | `uintptr_t` | 门名数量。 |

返回：成功返回新建的 `CDecomposeUnitaryTarget*`（用 `decompose_unitary_target_free` 释放）；`len` 大于 0 但 `gate_names` 为 NULL、某条目非法 UTF-8 或为未知门名，或核心拒绝该基组（例如空基组）时返回 NULL。

### decompose_unitary_target_from_standard_gates(native_1q_names, num_1q, native_2q_names, num_2q, fallback_pauli)

从显式的原生单比特与双比特标准门名列表以及 Pauli 旋转回退开关构造合成目标。`native_1q_names` 中的每个名字必须是单比特标准门，`native_2q_names` 中的每个名字必须是双比特标准门。

```c
struct CDecomposeUnitaryTarget *decompose_unitary_target_from_standard_gates(
    const char *const *native_1q_names,
    uintptr_t num_1q,
    const char *const *native_2q_names,
    uintptr_t num_2q,
    uint8_t fallback_pauli);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `native_1q_names` | `const char* const*` | 原生单比特标准门名；`num_1q` 为 0 时可为 NULL。 |
| `num_1q` | `uintptr_t` | 单比特门名数量。 |
| `native_2q_names` | `const char* const*` | 原生双比特标准门名；`num_2q` 为 0 时可为 NULL。 |
| `num_2q` | `uintptr_t` | 双比特门名数量。 |
| `fallback_pauli` | `uint8_t` | 非 0 启用精确 Pauli 旋转回退。 |

返回：成功返回新建的 `CDecomposeUnitaryTarget*`（用 `decompose_unitary_target_free` 释放）；NULL 输入、非法 UTF-8、未知门名、门元数不符或合并基组为空时返回 NULL。

### decompose_unitary_target_free(ptr)

释放酉分解合成目标句柄，允许传 NULL。

```c
void decompose_unitary_target_free(struct CDecomposeUnitaryTarget *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CDecomposeUnitaryTarget*` | 待释放句柄，可为 NULL。 |

### decompose_unitary_target_fallback_pauli(ptr)

判断目标是否允许精确 Pauli 旋转回退。

```c
int32_t decompose_unitary_target_fallback_pauli(const struct CDecomposeUnitaryTarget *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CDecomposeUnitaryTarget*` | 合成目标句柄。 |

返回：`1` 允许回退；`0` 不允许；`-1` NULL 输入。

### decompose_unitary_target_set_fallback_pauli(ptr, value)

设置目标是否允许 Pauli 旋转回退。目标会围绕其原生门列表重建，这要求基组非空；调用被拒绝时目标保持不变。

```c
int32_t decompose_unitary_target_set_fallback_pauli(struct CDecomposeUnitaryTarget *ptr,
                                                    uint8_t value);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CDecomposeUnitaryTarget*` | 合成目标句柄。 |
| `value` | `uint8_t` | 非 0 允许回退。 |

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL。 |
| `-8` | InvalidParam | 目标无约束（无原生门）。 |

### decompose_unitary_target_native_1q_len(ptr) / decompose_unitary_target_native_1q(ptr, out, len)

```c
uintptr_t decompose_unitary_target_native_1q_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_native_1q(const struct CDecomposeUnitaryTarget *ptr,
                                             char **out,
                                             uintptr_t len);
```

两步式读取原生单比特门名（第一步：`*_len` 返回总数；第二步：填充接口把门名拷入 `out`，实际拷贝 `min(总数, len)` 个）。每个写入条目都是新建的 C 字符串，用 `cqlib_string_free` 释放；`out` 为 NULL 时填充接口只返回总数、不拷贝。

返回（两者一致）：原生单比特门总数；NULL 输入返回 0。

### decompose_unitary_target_native_2q_len(ptr) / decompose_unitary_target_native_2q(ptr, out, len)

```c
uintptr_t decompose_unitary_target_native_2q_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_native_2q(const struct CDecomposeUnitaryTarget *ptr,
                                             char **out,
                                             uintptr_t len);
```

两步式读取原生双比特门名（缓冲区拷贝模式与单比特一对相同；每个写入条目用 `cqlib_string_free` 释放）。

返回（两者一致）：原生双比特门总数；NULL 输入返回 0。

### decompose_unitary_target_basis_len(ptr) / decompose_unitary_target_basis(ptr, out, len)

```c
uintptr_t decompose_unitary_target_basis_len(const struct CDecomposeUnitaryTarget *ptr);
uintptr_t decompose_unitary_target_basis(const struct CDecomposeUnitaryTarget *ptr,
                                         char **out,
                                         uintptr_t len);
```

两步式读取合并的原生基组（先单比特门、后双比特门；缓冲区拷贝模式相同；每个写入条目用 `cqlib_string_free` 释放）。

返回（两者一致）：基组门总数；NULL 输入返回 0。

### decompose_unitary_target_cost_of_fixed_operations(ptr, gate_names, qubit_ids, qubit_counts, num_ops, out)

在目标的原生基组下计算固定标准门操作序列的精确目标基组代价。序列由 `num_ops` 个门名给出，各操作作用的比特 ID 按顺序平铺在 `qubit_ids` 中，每个操作的比特数记录在 `qubit_counts` 中。参数按固定 0 值处理。要求目标受约束（无约束目标没有可用于计价的基组）。代价快照写入 `*out`（`CTargetBasisCost` 布局见 [配置与结果结构体](#配置与结果结构体)）。

```c
int32_t decompose_unitary_target_cost_of_fixed_operations(
    const struct CDecomposeUnitaryTarget *ptr,
    const char *const *gate_names,
    const uint32_t *qubit_ids,
    const uint32_t *qubit_counts,
    uintptr_t num_ops,
    struct CTargetBasisCost *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CDecomposeUnitaryTarget*` | 合成目标句柄。 |
| `gate_names` | `const char* const*` | 序列中每个操作的门名。 |
| `qubit_ids` | `const uint32_t*` | 全部操作的比特 ID，按序列顺序平铺。 |
| `qubit_counts` | `const uint32_t*` | 每个操作作用的比特数。 |
| `num_ops` | `uintptr_t` | 序列中的操作数。 |
| `out` | `struct CTargetBasisCost*` | 代价快照输出。 |

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |
| `-4` | ParseError | 某门名非法 UTF-8 或为未知门名。 |
| `-6` | CompilerError | 核心拒绝该序列。 |
| `-8` | InvalidParam | 目标无约束（无原生基组）。 |

---

## CTwoQubitUnitarySynthesis 读取

`synthesize_numeric_2q_unitary` 的结果句柄：发出的操作序列与全局相位。

### two_qubit_synthesis_free(ptr)

释放双比特合成结果句柄，允许传 NULL。

```c
void two_qubit_synthesis_free(struct CTwoQubitUnitarySynthesis *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CTwoQubitUnitarySynthesis*` | 待释放句柄，可为 NULL。 |

### two_qubit_synthesis_num_operations(ptr)

返回发出的操作数量。

```c
uintptr_t two_qubit_synthesis_num_operations(const struct CTwoQubitUnitarySynthesis *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | 合成结果句柄。 |

返回：操作数量；NULL 输入返回 0。

### two_qubit_synthesis_global_phase(ptr)

返回乘在整个操作序列上的全局相位。

```c
double two_qubit_synthesis_global_phase(const struct CTwoQubitUnitarySynthesis *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | 合成结果句柄。 |

返回：全局相位（弧度）；NULL 输入返回 0.0。

### two_qubit_synthesis_operation_name(ptr, index)

返回第 `index` 个操作的指令名。

```c
char *two_qubit_synthesis_operation_name(const struct CTwoQubitUnitarySynthesis *ptr,
                                         uintptr_t index);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | 合成结果句柄。 |
| `index` | `uintptr_t` | 操作下标（从 0 开始）。 |

返回：成功返回新建的 C 字符串（用 `cqlib_string_free` 释放）；NULL 输入或下标越界时返回 NULL。

### two_qubit_synthesis_operation_qubits_len(ptr, index)

返回第 `index` 个操作作用的比特数量（两步式读取的第一步）。

```c
uintptr_t two_qubit_synthesis_operation_qubits_len(const struct CTwoQubitUnitarySynthesis *ptr,
                                                   uintptr_t index);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | 合成结果句柄。 |
| `index` | `uintptr_t` | 操作下标（从 0 开始）。 |

返回：比特数量；NULL 输入或下标越界时返回 0。

### two_qubit_synthesis_operation_qubits(ptr, index, out, len)

把第 `index` 个操作作用的比特 ID 拷贝到 `out`（两步式读取的第二步；先调用 `two_qubit_synthesis_operation_qubits_len` 查询长度）。当 `len` 小于总数时只拷贝前 `len` 个。

```c
uintptr_t two_qubit_synthesis_operation_qubits(const struct CTwoQubitUnitarySynthesis *ptr,
                                               uintptr_t index,
                                               uint32_t *out,
                                               uintptr_t len);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTwoQubitUnitarySynthesis*` | 合成结果句柄。 |
| `index` | `uintptr_t` | 操作下标（从 0 开始）。 |
| `out` | `uint32_t*` | 比特 ID 输出数组。 |
| `len` | `uintptr_t` | `out` 的容量。 |

返回：该操作作用的总比特数；NULL 输入或下标越界时返回 0。

---

## KAK 分解

### kak_decompose(matrix)

把 4×4 酉矩阵分解为 KAK（Weyl）坐标与局域因子。`matrix` 指向 16 个行主序 `Complex64` 元素。

```c
struct CKakDecomposition *kak_decompose(const Complex64 *matrix);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `matrix` | `const Complex64*` | 16 个行主序复数元素（4×4 矩阵）。 |

返回：成功返回新建的 `CKakDecomposition*`（用 `kak_decomposition_free` 释放）；`matrix` 为 NULL 或分解失败（例如矩阵非有限或非酉）时返回 NULL。

### kak_decomposition_free(ptr)

释放 KAK 分解句柄，允许传 NULL。

```c
void kak_decomposition_free(struct CKakDecomposition *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CKakDecomposition*` | 待释放句柄，可为 NULL。 |

### kak_decomposition_global_phase(ptr)

返回乘在整个分解上的标量相位。

```c
double kak_decomposition_global_phase(const struct CKakDecomposition *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK 分解句柄。 |

返回：全局相位（弧度）；NULL 输入返回 0.0。

### kak_decomposition_coordinates(ptr, a, b, c)

把规范 Pauli XX/YY/ZZ 交互坐标 `a`、`b`、`c` 写入给定输出指针。

```c
int32_t kak_decomposition_coordinates(const struct CKakDecomposition *ptr,
                                      double *a,
                                      double *b,
                                      double *c);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK 分解句柄。 |
| `a` | `double*` | XX 交互坐标输出。 |
| `b` | `double*` | YY 交互坐标输出。 |
| `c` | `double*` | ZZ 交互坐标输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | 任一指针为 NULL。 |

### kak_decomposition_local_factor(ptr, factor, out)

按 `KAK_FACTOR_*` 标签选取一个 2×2 局域因子，作为 4 个行主序 `Complex64` 元素拷贝到 `out`。

```c
int32_t kak_decomposition_local_factor(const struct CKakDecomposition *ptr,
                                       uint8_t factor,
                                       Complex64 *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CKakDecomposition*` | KAK 分解句柄。 |
| `factor` | `uint8_t` | 局域因子标签，`KAK_FACTOR_*` 之一。 |
| `out` | `Complex64*` | 4 个行主序复数元素输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |
| `-8` | InvalidParam | `factor` 不是有效的 `KAK_FACTOR_*` 标签。 |

---

## 目标门集降级

### target_basis_lowerer_new(gate_names, len)

从标准门名数组构造目标门集降级器：`gate_names` 的每个条目必须是 C ABI 各处接受的标准门名（例如 `"H"`、`"RZ"`、`"CX"`），这些门构成目标基组。

```c
struct CTargetBasisLowerer *target_basis_lowerer_new(const char *const *gate_names, uintptr_t len);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `gate_names` | `const char* const*` | 标准门名数组；`len` 为 0 时可为 NULL。 |
| `len` | `uintptr_t` | 门名数量。 |

返回：成功返回新建的 `CTargetBasisLowerer*`（用 `target_basis_lowerer_free` 释放）；`len` 大于 0 但 `gate_names` 为 NULL、某条目为 NULL、含无效 UTF-8、含未知门名，或基组为空（`len` 为 0）时返回 NULL。

### target_basis_lowerer_free(ptr)

释放目标门集降级器句柄，允许传 NULL。

```c
void target_basis_lowerer_free(struct CTargetBasisLowerer *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CTargetBasisLowerer*` | 待释放句柄，可为 NULL。 |

### target_basis_lowerer_num_gates(ptr)

返回目标基组中的门数量。

```c
uintptr_t target_basis_lowerer_num_gates(const struct CTargetBasisLowerer *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | 降级器句柄。 |

返回：门数量；NULL 输入返回 0。

### target_basis_lowerer_requires_lowering(ptr, circuit)

判断把线路转换到目标基组是否可能改变或拒绝其中的类门操作。

```c
int32_t target_basis_lowerer_requires_lowering(const struct CTargetBasisLowerer *ptr,
                                               const struct CCircuit *circuit);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | 降级器句柄。 |
| `circuit` | `const CCircuit*` | 输入线路。 |

返回：1 表示需要降级（存在基组外的类门操作）；0 表示不需要。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `circuit` 为 NULL。 |

### target_basis_lowerer_apply(ptr, circuit)

把线路降到目标基组，返回重建线路。

```c
struct CCircuit *target_basis_lowerer_apply(const struct CTargetBasisLowerer *ptr,
                                            const struct CCircuit *circuit);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CTargetBasisLowerer*` | 降级器句柄。 |
| `circuit` | `const CCircuit*` | 输入线路。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；任一句柄为 NULL 或降级失败时返回 NULL。

---

## 设备降级

### decompose_lower_to_device(circuit, device)

把线路降到设备的原生指令集，返回重建线路。

```c
struct CCircuit *decompose_lower_to_device(const struct CCircuit *circuit,
                                           const struct CDevice *device);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `device` | `const CDevice*` | 目标设备；其原生门集（`device_with_native_gates` 设置，见 [设备与噪声属性](../2_device/2_properties_device.md)）决定降级目标。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；任一句柄为 NULL 或降级失败时返回 NULL。

---

## 配置与结果结构体

### CUnitaryDecomposeConfig

```c
typedef struct CUnitaryDecomposeConfig {
  uint8_t recurse_control_flow;
  uint8_t _reserved[7];
} CUnitaryDecomposeConfig;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `recurse_control_flow` | `uint8_t` | 1 递归进入控制流体，0 不递归。 |
| `_reserved[7]` | `uint8_t[7]` | 填充字段，保持结构体布局稳定。 |

### CMcGateDecomposeConfig

```c
typedef struct CMcGateDecomposeConfig {
  uintptr_t max_pre_layout_clean_ancillas;
  uint8_t allow_dirty_borrowing;
  uint8_t has_max_total_qubits;
  uint8_t _pad[6];
  uintptr_t max_total_qubits;
  uint64_t _reserved[2];
} CMcGateDecomposeConfig;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `max_pre_layout_clean_ancillas` | `uintptr_t` | 布局前允许创建的干净逻辑 ancilla 总数上限。 |
| `allow_dirty_borrowing` | `uint8_t` | 1 允许按脏契约借用输入比特。 |
| `has_max_total_qubits` | `uint8_t` | 1 启用 `max_total_qubits` 硬限制，0 不限制。 |
| `_pad[6]` | `uint8_t[6]` | 填充字段，保持结构体布局稳定。 |
| `max_total_qubits` | `uintptr_t` | 总逻辑比特数硬限制（`has_max_total_qubits != 0` 时生效）。 |
| `_reserved[2]` | `uint64_t[2]` | 填充字段，保持结构体布局稳定。 |

### CResynthesisConfig

```c
typedef struct CResynthesisConfig {
  uintptr_t max_block_ops;
  uintptr_t max_crossed_ops;
  uintptr_t max_scan_span;
  uint8_t skip_labeled_ops;
  uint8_t recurse_control_flow;
  uint8_t enable_rule_oracle;
  uint8_t enable_matrix_fallback;
  uint8_t _pad[4];
  uintptr_t max_matrix_qubits;
  uint64_t _reserved[2];
} CResynthesisConfig;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `max_block_ops` | `uintptr_t` | 单个有界候选块内源操作数上限。 |
| `max_crossed_ops` | `uintptr_t` | 收集块期间允许跨越的非块操作数上限。 |
| `max_scan_span` | `uintptr_t` | 双比特锚点每一侧的收集预算。 |
| `skip_labeled_ops` | `uint8_t` | 1 把带标签操作视为硬边界。 |
| `recurse_control_flow` | `uint8_t` | 1 递归进入结构化经典控制体。 |
| `enable_rule_oracle` | `uint8_t` | 1 启用知识规则对易 oracle。 |
| `enable_matrix_fallback` | `uint8_t` | 1 启用对易引擎的局部矩阵回退。 |
| `_pad[4]` | `uint8_t[4]` | 填充字段，保持结构体布局稳定。 |
| `max_matrix_qubits` | `uintptr_t` | 矩阵回退的最大联合支持规模。 |
| `_reserved[2]` | `uint64_t[2]` | 填充字段，保持结构体布局稳定。 |

### CDecompositionRuleStats

分解 pass 局部规则缓存统计快照（`decompose_unitaries_with_rule_stats` / `decompose_mc_gates_with_rule_stats` 的输出）：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `hits` | `uintptr_t` | 本次运行的缓存命中数。 |
| `misses` | `uintptr_t` | 本次运行的缓存未命中数。 |
| `inserts` | `uintptr_t` | 本次运行的缓存写入数。 |

### CTargetBasisCost

目标基组降级代价快照（`decompose_unitary_target_cost_of_fixed_operations` 的输出）：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `two_qubit_ops` | `uintptr_t` | 降级序列中的双比特操作数。 |
| `depth` | `uintptr_t` | 降级序列的关键路径深度。 |
| `total_ops` | `uintptr_t` | 降级序列的操作总数（不含全局相位）。 |
| `parameterized_ops` | `uintptr_t` | 降级序列中带参数的操作数。 |

### COneQubitUnitaryDecomposition

单比特数值合成结果快照（`synthesize_numeric_1q_unitary` 的输出）；表示的矩阵为 `exp(i * global_phase) * U(theta, phi, lambda)`：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `theta` | `double` | 极向旋转角。 |
| `phi` | `double` | 第一方位角。 |
| `lambda` | `double` | 第二方位角。 |
| `global_phase` | `double` | 乘在合成门上的标量相位。 |

---

## 示例

先跑一遍线路级分解 pass（定义展开 → 酉门合成 → 多控制门改写 → 双比特块重综合），调用顺序对照测试套件：

```c
#include <math.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. A frozen circuit-gate is expanded back into plain operations */
    struct CCircuit *inner = circuit_new(1);
    circuit_h(inner, 0);
    struct CCircuitGate *gate = circuit_to_gate(inner, "my_h");
    struct CCircuit *outer = circuit_new(2);
    uint32_t gate_qubits[1] = {0};
    circuit_circuit_gate(outer, gate, gate_qubits, 1, NULL, 0);
    struct CCircuit *expanded = decompose_expand_definitions(outer);  /* 1 op */

    /* 2. A matrix-backed H is synthesized into standard operations */
    struct CCircuit *unitary_circuit = circuit_new(2);
    Complex64 h_matrix[4] = {
        {1.0 / sqrt(2.0), 0.0}, {1.0 / sqrt(2.0), 0.0},
        {1.0 / sqrt(2.0), 0.0}, {-1.0 / sqrt(2.0), 0.0},
    };
    uint32_t unitary_targets[1] = {0};
    circuit_unitary(unitary_circuit, "my_u", 1, h_matrix, unitary_targets, 1);
    struct CDecompositionRuleStats stats;
    struct CCircuit *synthesized = decompose_unitaries_with_rule_stats(
        unitary_circuit, decompose_unitary_config_default(), &stats);
    /* stats.hits / stats.misses / stats.inserts reflect the rule cache */

    /* 3. A doubly-controlled X is rewritten within device capacity */
    struct CCircuit *mc_circuit = circuit_new(3);
    uint32_t controls[2] = {0, 1};
    uint32_t targets[1] = {2};
    circuit_multi_control(mc_circuit, "X", controls, 2, targets, 1, NULL, 0);
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *rewritten = decompose_mc_gates_for_device(mc_circuit, device, NULL);

    /* 4. Two-qubit blocks are resynthesized under the enhanced budget */
    struct CCircuit *block_circuit = circuit_new(2);
    circuit_h(block_circuit, 0);
    circuit_cx(block_circuit, 0, 1);
    struct CCircuit *resynthesized =
        resynthesize_two_qubit_blocks(block_circuit, resynthesis_config_enhanced());

    circuit_free(resynthesized);
    circuit_free(block_circuit);
    circuit_free(rewritten);
    device_free(device);
    circuit_free(mc_circuit);
    circuit_free(synthesized);
    circuit_free(unitary_circuit);
    circuit_free(expanded);
    circuit_gate_free(gate);
    circuit_free(outer);
    circuit_free(inner);
    return 0;
}
```

数值合成、KAK 分解与降级（单位阵的 2q 合成与 KAK 分解的交互坐标均为零）：

```c
#include <math.h>
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 1q synthesis of the identity yields U(0, 0, 0), zero phase */
    Complex64 identity2[4] = {{1.0, 0.0}, {0.0, 0.0}, {0.0, 0.0}, {1.0, 0.0}};
    struct COneQubitUnitaryDecomposition one_q;
    memset(&one_q, 0, sizeof(one_q));
    if (synthesize_numeric_1q_unitary(identity2, &one_q) != 0) {
        return 1;
    }

    /* 2. 2q synthesis of the identity in the CX basis */
    Complex64 identity4[16];
    memset(identity4, 0, sizeof(identity4));
    for (int i = 0; i < 16; i += 5) {
        identity4[i].re = 1.0;
    }
    struct CTwoQubitUnitarySynthesis *synthesis =
        synthesize_numeric_2q_unitary(identity4, 0, 1, TWO_QUBIT_BASIS_CX);
    if (synthesis != NULL) {
        for (uintptr_t i = 0; i < two_qubit_synthesis_num_operations(synthesis); i++) {
            char *name = two_qubit_synthesis_operation_name(synthesis, i);
            uint32_t qubits[8];
            uintptr_t len = two_qubit_synthesis_operation_qubits_len(synthesis, i);
            uintptr_t copied = two_qubit_synthesis_operation_qubits(synthesis, i, qubits, len);
            printf("op=%s qubits=%lu\n", name, (unsigned long)copied);
            cqlib_string_free(name);
        }
        two_qubit_synthesis_free(synthesis);
    }

    /* 3. KAK decomposition of the identity: zero interaction coordinates */
    struct CKakDecomposition *kak = kak_decompose(identity4);
    if (kak != NULL) {
        double a = 0.0, b = 0.0, c = 0.0;
        Complex64 k1l[4];
        if (kak_decomposition_coordinates(kak, &a, &b, &c) == 0) {
            printf("a=%f b=%f c=%f\n", a, b, c);  /* ~0 */
        }
        kak_decomposition_local_factor(kak, KAK_FACTOR_K1L, k1l);  /* 0 */
        kak_decomposition_free(kak);
    }

    /* 4. Target-basis lowering: T is outside {H, RZ, CX} */
    const char *basis_names[3] = {"H", "RZ", "CX"};
    struct CTargetBasisLowerer *lowerer = target_basis_lowerer_new(basis_names, 3);
    if (lowerer != NULL) {
        struct CCircuit *t_circuit = circuit_new(1);
        circuit_t(t_circuit, 0);
        if (target_basis_lowerer_requires_lowering(lowerer, t_circuit) == 1) {
            struct CCircuit *lowered = target_basis_lowerer_apply(lowerer, t_circuit);
            circuit_free(lowered);
        }
        circuit_free(t_circuit);
        target_basis_lowerer_free(lowerer);
    }

    /* 5. Device lowering: a native circuit passes through unchanged */
    uint32_t edges[2] = {0, 1};
    struct CDevice *device = device_from_edges("line-2", 2, edges, 1);
    device_with_native_gates(device, "H,RZ,CX");
    struct CCircuit *native_circuit = circuit_new(2);
    circuit_h(native_circuit, 0);
    circuit_cx(native_circuit, 0, 1);
    struct CCircuit *device_lowered = decompose_lower_to_device(native_circuit, device);  /* 2 ops */

    circuit_free(device_lowered);
    circuit_free(native_circuit);
    device_free(device);
    return 0;
}
```

酉合成目标：原生基组读回与代价评估：

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

static void sort_names(char **names, uintptr_t len) {
    for (uintptr_t i = 0; i + 1 < len; i++) {
        for (uintptr_t j = i + 1; j < len; j++) {
            if (strcmp(names[i], names[j]) > 0) {
                char *tmp = names[i];
                names[i] = names[j];
                names[j] = tmp;
            }
        }
    }
}

int main(void) {
    /* 1. 由工作流风格的基组列表构建目标 */
    const char *const basis_gates[3] = {"H", "RZ", "CX"};
    struct CDecomposeUnitaryTarget *target =
        decompose_unitary_target_from_instructions(basis_gates, 3);
    if (target == NULL) {
        return 1;
    }
    /* decompose_unitary_target_fallback_pauli(target) == 1,
       native_1q_len == 2（H、RZ），native_2q_len == 1（CX），basis_len == 3 */

    /* 2. 两步式读出合并后的基组（单比特门在前） */
    char *basis[3] = {NULL, NULL, NULL};
    uintptr_t total = decompose_unitary_target_basis(target, basis, 3);
    sort_names(basis, total);
    for (uintptr_t i = 0; i < total && i < 3; i++) {
        printf("basis[%llu]=%s\n", (unsigned long long)i, basis[i]);
        cqlib_string_free(basis[i]);
    }

    /* 3. 固定门操作序列在原生基组下的代价 */
    const char *const ops[3] = {"H", "CX", "H"};
    const uint32_t qubit_ids[4] = {0, 0, 1, 1};
    const uint32_t op_counts[3] = {1, 2, 1};
    struct CTargetBasisCost cost;
    memset(&cost, 0, sizeof(cost));
    if (decompose_unitary_target_cost_of_fixed_operations(target, ops, qubit_ids,
                                                          op_counts, 3, &cost) == 0) {
        /* cost.total_ops == 3, cost.two_qubit_ops == 1,
           cost.parameterized_ops == 0, cost.depth == 3 */
        printf("ops=%llu two_qubit=%llu depth=%llu\n",
               (unsigned long long)cost.total_ops,
               (unsigned long long)cost.two_qubit_ops,
               (unsigned long long)cost.depth);
    }

    /* 4. 带参数的门按带参数操作计数 */
    const char *const rz_ops[1] = {"RZ"};
    const uint32_t rz_counts[1] = {1};
    decompose_unitary_target_cost_of_fixed_operations(target, rz_ops, qubit_ids,
                                                      rz_counts, 1, &cost);
    /* cost.total_ops == 1, cost.parameterized_ops == 1 */

    /* 5. 切换 fallback 会围绕原生列表重建目标 */
    decompose_unitary_target_set_fallback_pauli(target, 0);
    /* decompose_unitary_target_fallback_pauli(target) == 0 */
    decompose_unitary_target_set_fallback_pauli(target, 1);

    decompose_unitary_target_free(target);
    return 0;
}
```
