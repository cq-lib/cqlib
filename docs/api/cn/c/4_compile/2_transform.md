# 线路变换（C）

本页覆盖编译 transform 阶段的 C ABI：结构分析快照 `CCircuitAnalysis`、句柄式配置 `CTransformConfig` 与 `CCanonicalizeConfig`、规范化 `canonicalize_circuit` / `canonicalize_circuit_with_config`、知识规则重写 `rewrite_circuit`，以及五个独立 pass 封装 `transform_*`。全部接口不修改输入线路：分析快照写入调用方提供的结构体，变换结果以新句柄或新线路返回。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量

重写模式标签（`CRewriteConfig.mode`）：

| 常量 | 值 | 模式 |
| --- | --- | --- |
| `REWRITE_MODE_OPTIMIZE` | 0 | 保守优化（生产默认）：接受的替换必须严格改善局部代价。 |
| `REWRITE_MODE_LOWERING` | 1 | 显式降级：分解与硬件原生规则可以局部扩张。 |

重写规则类别标签（`transform_config_with_enabled_kinds` 与 `transform_config_enabled_kinds` 使用）：

| 常量 | 值 | 类别 |
| --- | --- | --- |
| `REWRITE_KIND_SIMPLIFY` | 0 | 代数化简。 |
| `REWRITE_KIND_CANCEL` | 1 | 互逆 / 重复抵消。 |
| `REWRITE_KIND_MERGE` | 2 | 相邻门合并。 |
| `REWRITE_KIND_COMMUTE` | 3 | 显式对易。 |
| `REWRITE_KIND_DECOMPOSE` | 4 | 分解 / 降级。 |
| `REWRITE_KIND_CANONICALIZE` | 5 | 规范表示。 |
| `REWRITE_KIND_HARDWARE_NATIVE` | 6 | 硬件原生改写。 |
| `REWRITE_KIND_OTHER` | 7 | 其他。 |

---

## 重写配置

`CRewriteConfig` 是知识重写的输入配置，按值传递：

```c
typedef struct CRewriteConfig {
  uint8_t mode;                          /* One of REWRITE_MODE_*. */
  uint8_t max_rounds;                    /* Fixpoint round limit. */
  uint8_t recurse_control_flow;          /* 1 recurses into control-flow bodies. */
  uint8_t skip_labeled_ops;             /* 1 skips labeled operations. */
  uint8_t preserve_two_qubit_connectivity; /* 1 preserves undirected 2q connectivity. */
  uint8_t _pad[3];                       /* Reserved padding. */
  uintptr_t max_window_ops;              /* Matching-window operation limit. */
  uintptr_t max_pattern_len;             /* Maximum matched pattern length. */
  const char *const *target_gate_names;  /* Borrowed target-basis gate names (NULL = none). */
  uintptr_t target_gate_names_len;       /* Number of entries in target_gate_names. */
} CRewriteConfig;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `mode` | `uint8_t` | 规则集基线，取 `REWRITE_MODE_*` 之一：`OPTIMIZE` 使用生产默认规则，`LOWERING` 追加分解与硬件原生规则。 |
| `max_rounds` | `uint8_t` | 不动点迭代轮数上限。 |
| `recurse_control_flow` | `uint8_t` | 1 时递归进入控制流块内部，0 时不递归。 |
| `skip_labeled_ops` | `uint8_t` | 1 时跳过带标签的操作，0 时不跳过。 |
| `preserve_two_qubit_connectivity` | `uint8_t` | 1 时保持无向二比特连接性（重写不得拆散相邻二比特对），0 时不要求。 |
| `max_window_ops` | `uintptr_t` | 规则匹配窗口内的操作数上限。 |
| `max_pattern_len` | `uintptr_t` | 单条规则可匹配的最大模式长度。 |
| `target_gate_names` | `const char *const *` | 借用的目标基标准门名数组（每个元素为 NUL 结尾字符串）；未配置目标基时为 NULL。通过 `rewrite_config_with_target_instructions` 设置。 |
| `target_gate_names_len` | `uintptr_t` | `target_gate_names` 的条目数。 |
| `_pad[3]` | — | 填充字段，保持结构体布局稳定，勿写入非零值。 |

除 `mode` 外的字段在基线值之上覆盖：以 `rewrite_config_default()` / `rewrite_config_lowering()` 为起点，再修改需要的字段。`target_gate_names` 数组及其字符串是借用而非拷贝：必须保持有效，直到消费该配置的 `rewrite_circuit` / `transform_knowledge_rewrite` 调用结束。

### rewrite_config_default()

```c
struct CRewriteConfig rewrite_config_default(void);
```

按值返回生产优化配置（`REWRITE_MODE_OPTIMIZE`）：`max_rounds = 8`、`recurse_control_flow = 1`、`skip_labeled_ops = 1`、`preserve_two_qubit_connectivity = 0`、`max_window_ops = 16`、`max_pattern_len = 8`。

### rewrite_config_lowering()

```c
struct CRewriteConfig rewrite_config_lowering(void);
```

按值返回显式降级配置：与 `rewrite_config_default()` 相同的字段值，但 `mode = REWRITE_MODE_LOWERING`，规则集追加分解与硬件原生规则。

### rewrite_config_with_target_instructions(config, gate_names, len)

```c
int32_t rewrite_config_with_target_instructions(struct CRewriteConfig *config,
                                                const char *const *gate_names,
                                                uintptr_t len);
```

把按值重写配置限制到显式的标准门目标指令基，指令基由 `len` 个门名解析而来（如 `"H"`、`"CX"`）。门名数组是借用的（见上方字段表）并在设置时立即校验；空指令基被拒绝，配置保持不变。目标基只在 `config` 使用 `REWRITE_MODE_LOWERING` 时生效。

- `config` (`CRewriteConfig*`)：重写配置，就地修改。
- `gate_names` (`const char *const *`)：`len` 个 NUL 结尾标准门名组成的数组。
- `len` (`uintptr_t`)：`gate_names` 的条目数。

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `config` 为 NULL，或 `len` 大于 0 但 `gate_names` 为 NULL 或含 NULL 条目。 |
| `-4` | ParseError | 某个条目不是合法 UTF-8 或门名未知。 |
| `-6` | CompilerError | 核心拒绝该指令基。 |
| `-8` | InvalidParam | 指令基为空（`len` 为 0）；配置保持不变。 |

---

## 变换配置句柄

`CTransformConfig` 是知识重写 / 变换配置核心对象的透明句柄，其按值 C 形式为 `CRewriteConfig`。与按值结构体不同，它分配在堆上并就地修改：每个 `transform_config_with_*` 设置器直接改写句柄内的配置并返回 `int32_t` 状态码——设置器不可链式调用。规则类别字段使用[常量](#常量)一节的 `REWRITE_KIND_*` 标签。

### transform_config_default()

```c
struct CTransformConfig *transform_config_default(void);
```

创建生产（保守优化）默认的变换配置：`mode = REWRITE_MODE_OPTIMIZE`、`max_rounds = 8`、`max_window_ops = 16`、`max_pattern_len = 8`、启用控制流递归与带标签操作跳过、启用的规则类别为 `SIMPLIFY`、`CANCEL`、`MERGE`、`CANONICALIZE`，未设置目标指令基组。

返回：成功返回新建的 `CTransformConfig*`（用 `transform_config_free` 释放）；分配失败返回 NULL。

### transform_config_lowering()

```c
struct CTransformConfig *transform_config_lowering(void);
```

创建显式知识降级默认的变换配置：与 `transform_config_default()` 相同，但 `mode = REWRITE_MODE_LOWERING`，启用的规则集放宽为六个类别（追加 `DECOMPOSE` 与 `HARDWARE_NATIVE`）。

返回：成功返回新建的 `CTransformConfig*`（用 `transform_config_free` 释放）；分配失败返回 NULL。

### transform_config_free(ptr)

```c
void transform_config_free(struct CTransformConfig *ptr);
```

释放变换配置句柄，允许传 NULL。

### transform_config_with_mode(ptr, mode)

```c
int32_t transform_config_with_mode(struct CTransformConfig *ptr, uint8_t mode);
```

就地设置重写模式；`mode` 必须是 `REWRITE_MODE_*` 之一。

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL。 |
| `-8` | InvalidParam | `mode` 不是合法的 `REWRITE_MODE_*` 标签。 |

### transform_config_with_max_rounds(ptr, max_rounds)

```c
int32_t transform_config_with_max_rounds(struct CTransformConfig *ptr, uint8_t max_rounds);
```

就地设置不动点迭代轮数上限。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### transform_config_with_max_window_ops(ptr, max_window_ops)

```c
int32_t transform_config_with_max_window_ops(struct CTransformConfig *ptr,
                                             uintptr_t max_window_ops);
```

就地设置匹配窗口内的操作数上限。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### transform_config_with_max_pattern_len(ptr, max_pattern_len)

```c
int32_t transform_config_with_max_pattern_len(struct CTransformConfig *ptr,
                                              uintptr_t max_pattern_len);
```

就地设置单条匹配模式的最大长度。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### transform_config_with_enabled_kinds(ptr, kinds, len)

```c
int32_t transform_config_with_enabled_kinds(struct CTransformConfig *ptr,
                                            const uint8_t *kinds,
                                            uintptr_t len);
```

就地替换启用的规则类别：`kinds` 指向 `len` 个 `REWRITE_KIND_*` 标签（`len` 为 0 时可为 NULL，表示关闭全部规则类别）。调用被拒绝时配置保持不变。

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL，或 `len` 大于 0 但 `kinds` 为 NULL。 |
| `-8` | InvalidParam | `kinds` 中存在不是合法 `REWRITE_KIND_*` 的标签。 |

### transform_config_with_target_instructions(ptr, gate_names, len)

```c
int32_t transform_config_with_target_instructions(struct CTransformConfig *ptr,
                                                  const char *const *gate_names,
                                                  uintptr_t len);
```

把配置限制到显式的标准门目标指令基组，基组由 `len` 个门名解析（如 `"H"`、`"CX"`）。空基组被拒绝；调用被拒绝时配置保持不变。

返回：`0` 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL，或 `len` 大于 0 但 `gate_names` 为 NULL 或含 NULL 条目。 |
| `-4` | ParseError | 某条目非法 UTF-8 或未知门名。 |
| `-6` | CompilerError | 核心拒绝该基组。 |
| `-8` | InvalidParam | 基组为空（`len` 为 0）。 |

### transform_config_mode(ptr)

```c
int32_t transform_config_mode(const struct CTransformConfig *ptr);
```

返回配置的 `REWRITE_MODE_*` 模式标签。

返回：模式标签；NULL 输入返回 `-1`。

### transform_config_max_rounds(ptr)

```c
uint8_t transform_config_max_rounds(const struct CTransformConfig *ptr);
```

返回配置的不动点迭代轮数上限。

返回：轮数上限；NULL 输入返回 0。

### transform_config_max_window_ops(ptr)

```c
uintptr_t transform_config_max_window_ops(const struct CTransformConfig *ptr);
```

返回配置的匹配窗口操作数上限。

返回：操作数上限；NULL 输入返回 0。

### transform_config_max_pattern_len(ptr)

```c
uintptr_t transform_config_max_pattern_len(const struct CTransformConfig *ptr);
```

返回配置的最大匹配模式长度。

返回：模式长度上限；NULL 输入返回 0。

### transform_config_recurse_control_flow(ptr)

```c
int32_t transform_config_recurse_control_flow(const struct CTransformConfig *ptr);
```

判断配置是否递归进入控制流块内部。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### transform_config_skip_labeled_ops(ptr)

```c
int32_t transform_config_skip_labeled_ops(const struct CTransformConfig *ptr);
```

判断配置是否跳过带标签的操作。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### transform_config_enabled_kinds_len(ptr) / transform_config_enabled_kinds(ptr, out, len)

```c
uintptr_t transform_config_enabled_kinds_len(const struct CTransformConfig *ptr);
uintptr_t transform_config_enabled_kinds(const struct CTransformConfig *ptr,
                                         uint8_t *out,
                                         uintptr_t len);
```

两步式读取启用的规则类别（第一步：`*_len` 返回总数；第二步：填充接口把 `REWRITE_KIND_*` 标签拷入 `out`，实际拷贝 `min(总数, len)` 个）。`out` 为 NULL 时填充接口只返回总数、不拷贝。

返回（两者一致）：启用的类别总数；NULL 输入返回 0。

### transform_config_target_instruction_basis_len(ptr) / transform_config_target_instruction_basis(ptr, out, len)

```c
uintptr_t transform_config_target_instruction_basis_len(const struct CTransformConfig *ptr);
uintptr_t transform_config_target_instruction_basis(const struct CTransformConfig *ptr,
                                                    char **out,
                                                    uintptr_t len);
```

两步式读取配置的目标基组指令名（未设置目标基组时为 0）。填充接口把门名拷入 `out`，实际拷贝 `min(总数, len)` 个；每个写入条目都是新建的 C 字符串，用 `cqlib_string_free` 释放。`out` 为 NULL 时只返回总数、不拷贝。

返回（两者一致）：基组指令总数；NULL 输入或未设置目标基组返回 0。

---

## 结构分析

`CCircuitAnalysis` 是线路特征快照，全部标志位取 0 或 1：

```c
typedef struct CCircuitAnalysis {
  uint8_t has_classical_data;
  uint8_t has_classical_control;
  uint8_t has_measurement;
  uint8_t has_classical_values;
  uint8_t has_classical_vars;
  uint8_t has_runtime_classical;
  uint8_t needs_classical_handle_preservation;
  uint8_t has_circuit_gate_definitions;
  uint8_t has_unitary_circuit_definitions;
  uint8_t has_unitary_gates;
  uint8_t has_mc_gates;
} CCircuitAnalysis;
```

| 字段 | 含义 |
| --- | --- |
| `has_classical_data` | 线路含经典数据操作。 |
| `has_classical_control` | 线路含经典控制流。 |
| `has_measurement` | 线路含测量操作。 |
| `has_classical_values` | 线路注册经典值。 |
| `has_classical_vars` | 线路注册经典变量。 |
| `has_runtime_classical` | 线路使用任意运行时经典构造。 |
| `needs_classical_handle_preservation` | 重写必须为该线路保持经典句柄。 |
| `has_circuit_gate_definitions` | 线路含未展开的线路型自定义门定义。 |
| `has_unitary_circuit_definitions` | 线路含带线路体的酉定义。 |
| `has_unitary_gates` | 线路含矩阵型酉门。 |
| `has_mc_gates` | 线路含多控门。 |

### circuit_analyze(circuit, out)

```c
int32_t circuit_analyze(const struct CCircuit *circuit, struct CCircuitAnalysis *out);
```

分析线路并把结构特征快照写入 `*out`。

- `circuit` (`const CCircuit*`)：输入线路。
- `out` (`CCircuitAnalysis*`)：快照输出。

返回：`0` 成功；`-1` 任一参数为 NULL。

---

## 规范化

规范化 pass 统一指令形式、折叠全局相位并消除空操作，结果由 `CCanonicalizeResult` 句柄承载。`canonicalize_circuit` 以生产默认配置执行；`canonicalize_circuit_with_config` 使用调用方构造的 `CCanonicalizeConfig` 句柄。

### canonicalize_circuit(circuit)

```c
struct CCanonicalizeResult *canonicalize_circuit(const struct CCircuit *circuit);
```

以生产默认配置规范化线路，输入线路不被修改。

- `circuit` (`const CCircuit*`)：输入线路。

返回：成功返回新建的 `CCanonicalizeResult*`（用 `canonicalize_result_free` 释放）；NULL 输入或执行失败返回 NULL。

### canonicalize_circuit_with_config(circuit, config)

```c
struct CCanonicalizeResult *canonicalize_circuit_with_config(const struct CCircuit *circuit,
                                                             const struct CCanonicalizeConfig *config);
```

按给定配置规范化线路；输入线路不被修改，配置也不被消耗——配置仍归调用方所有，可重复使用。

- `circuit` (`const CCircuit*`)：输入线路。
- `config` (`const CCanonicalizeConfig*`)：规范化配置句柄（见下文）。

返回：成功返回新建的 `CCanonicalizeResult*`（用 `canonicalize_result_free` 释放）；NULL 输入或执行失败返回 NULL。

### canonicalize_result_free(ptr)

```c
void canonicalize_result_free(struct CCanonicalizeResult *ptr);
```

释放规范化结果句柄。允许传 NULL。

### canonicalize_result_circuit(ptr)

```c
struct CCircuit *canonicalize_result_circuit(const struct CCanonicalizeResult *ptr);
```

取出规范化后的线路（深拷贝，与 `ptr` 独立）。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；NULL 输入或失败返回 NULL。

### canonicalize_result_changed(ptr)

```c
int32_t canonicalize_result_changed(const struct CCanonicalizeResult *ptr);
```

判断规范化是否改变了线路表示。

返回：`1` 有改动；`0` 无改动；`-1` 空指针。

### canonicalize_result_rounds(ptr)

```c
uint8_t canonicalize_result_rounds(const struct CCanonicalizeResult *ptr);
```

返回实际执行的规范化轮数。

返回：轮数；NULL 输入返回 0。

### canonicalize_config_default()

```c
struct CCanonicalizeConfig *canonicalize_config_default(void);
```

创建生产默认的规范化配置：轮数上限 8，且全部可选行为启用（控制流递归、`GPhase` 折叠、指令形式规范化、严格空操作消除、barrier 规范化）。

返回：成功返回新建的 `CCanonicalizeConfig*`（用 `canonicalize_config_free` 释放）；分配失败返回 NULL。

### canonicalize_config_free(ptr)

```c
void canonicalize_config_free(struct CCanonicalizeConfig *ptr);
```

释放规范化配置句柄，允许传 NULL。

### canonicalize_config_with_round_limit(ptr, round_limit)

```c
int32_t canonicalize_config_with_round_limit(struct CCanonicalizeConfig *ptr, uint8_t round_limit);
```

就地设置规范化轮数上限。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_with_recurse_control_flow(ptr, enabled)

```c
int32_t canonicalize_config_with_recurse_control_flow(struct CCanonicalizeConfig *ptr,
                                                      int32_t enabled);
```

就地设置是否递归规范化控制流块内部（`enabled` 非 0）或不递归（`0`）。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_with_fold_gphase(ptr, enabled)

```c
int32_t canonicalize_config_with_fold_gphase(struct CCanonicalizeConfig *ptr, int32_t enabled);
```

就地设置是否把 `GPhase` 操作折叠进作用域局部相位。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_with_canonicalize_instruction_form(ptr, enabled)

```c
int32_t canonicalize_config_with_canonicalize_instruction_form(struct CCanonicalizeConfig *ptr,
                                                               int32_t enabled);
```

就地设置是否把 `McGate` 形式折叠为已有的标准门。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_with_drop_noops(ptr, enabled)

```c
int32_t canonicalize_config_with_drop_noops(struct CCanonicalizeConfig *ptr, int32_t enabled);
```

就地设置是否执行严格空操作消除。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_with_canonicalize_barriers(ptr, enabled)

```c
int32_t canonicalize_config_with_canonicalize_barriers(struct CCanonicalizeConfig *ptr,
                                                       int32_t enabled);
```

就地设置是否规范化 barrier 作用域并合并相邻 barrier。

返回：`0` 成功；`-1`（`NullPtr`）`ptr` 为 NULL。

### canonicalize_config_round_limit(ptr)

```c
uint8_t canonicalize_config_round_limit(const struct CCanonicalizeConfig *ptr);
```

返回配置的轮数上限。

返回：轮数上限；NULL 输入返回 0。

### canonicalize_config_recurse_control_flow(ptr)

```c
int32_t canonicalize_config_recurse_control_flow(const struct CCanonicalizeConfig *ptr);
```

判断是否启用控制流递归。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### canonicalize_config_fold_gphase(ptr)

```c
int32_t canonicalize_config_fold_gphase(const struct CCanonicalizeConfig *ptr);
```

判断是否启用 `GPhase` 折叠。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### canonicalize_config_canonicalize_instruction_form(ptr)

```c
int32_t canonicalize_config_canonicalize_instruction_form(const struct CCanonicalizeConfig *ptr);
```

判断是否启用指令形式规范化。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### canonicalize_config_drop_noops(ptr)

```c
int32_t canonicalize_config_drop_noops(const struct CCanonicalizeConfig *ptr);
```

判断是否启用严格空操作消除。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

### canonicalize_config_canonicalize_barriers(ptr)

```c
int32_t canonicalize_config_canonicalize_barriers(const struct CCanonicalizeConfig *ptr);
```

判断是否启用 barrier 规范化。

返回：`1` 启用；`0` 未启用；`-1` NULL 输入。

---

## 知识重写

知识重写 pass 基于知识规则做模式化简与抵消，是编译管线逻辑优化的核心。结果由 `CKnowledgeRewriteResult` 句柄承载。

### rewrite_circuit(circuit, config)

```c
struct CKnowledgeRewriteResult *rewrite_circuit(const struct CCircuit *circuit,
                                                struct CRewriteConfig config);
```

按给定配置对线路执行知识重写，输入线路不被修改。

- `circuit` (`const CCircuit*`)：输入线路。
- `config` (`struct CRewriteConfig`)：重写配置，按值传递。

返回：成功返回新建的 `CKnowledgeRewriteResult*`（用 `knowledge_rewrite_result_free` 释放）；NULL 输入、`config.mode` 或目标指令基非法、执行失败返回 NULL。

### knowledge_rewrite_result_free(ptr)

```c
void knowledge_rewrite_result_free(struct CKnowledgeRewriteResult *ptr);
```

释放知识重写结果句柄。允许传 NULL。

### knowledge_rewrite_result_circuit(ptr)

```c
struct CCircuit *knowledge_rewrite_result_circuit(const struct CKnowledgeRewriteResult *ptr);
```

取出重写后的线路（深拷贝，与 `ptr` 独立）。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；NULL 输入或失败返回 NULL。

### knowledge_rewrite_result_changed(ptr)

```c
int32_t knowledge_rewrite_result_changed(const struct CKnowledgeRewriteResult *ptr);
```

判断重写是否改变了线路表示。

返回：`1` 有改动；`0` 无改动；`-1` 空指针。

### knowledge_rewrite_result_stats(ptr, out)

```c
int32_t knowledge_rewrite_result_stats(const struct CKnowledgeRewriteResult *ptr,
                                       struct CKnowledgeRewriteStats *out);
```

把重写运行统计快照写入 `*out`。`CKnowledgeRewriteStats` 布局如下：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `rounds_executed` | `uint8_t` | 实际执行的不动点轮数。 |
| `reached_fixpoint` | `uint8_t` | 在 `max_rounds` 内观测到稳定轮时为 1。 |
| `rules_applied` | `uintptr_t` | 发射到重建序列中的规则补丁数。 |
| `changed_sequences` | `uintptr_t` | 补丁集非空的操作序列数。 |

- `ptr` (`const CKnowledgeRewriteResult*`)：重写结果句柄。
- `out` (`CKnowledgeRewriteStats*`)：统计快照输出。

返回：`0` 成功；`-1` 任一参数为 NULL。

### knowledge_rewrite_result_diagnostics(ptr, out)

```c
int32_t knowledge_rewrite_result_diagnostics(const struct CKnowledgeRewriteResult *ptr,
                                             struct CKnowledgeRewriteDiagnostics *out);
```

把重写性能诊断快照写入 `*out`。`CKnowledgeRewriteDiagnostics` 布局如下：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `direct_reuses` | `uintptr_t` | 被可复用会话不动点证明跳过的重写阶段数。 |
| `dirty_anchors` | `uintptr_t` | 密度回退前由已验证脏区间提供的锚点数。 |
| `full_scan_fallbacks` | `uintptr_t` | 被保守提升为全量扫描的增量执行次数。 |
| `condition_cache_hits` | `uintptr_t` | 复用的缓存规则条件结果数。 |
| `condition_cache_misses` | `uintptr_t` | 本地缓存未命中的可缓存规则条件绑定数。 |
| `symbolic_fallbacks` | `uintptr_t` | 由保守符号路径求值的条件数。 |

- `ptr` (`const CKnowledgeRewriteResult*`)：重写结果句柄。
- `out` (`CKnowledgeRewriteDiagnostics*`)：诊断快照输出。

返回：`0` 成功；`-1` 任一参数为 NULL。

---

## 独立 pass 封装

`transform_*` 系列是单个 transform pass 的便捷封装：直接返回变换后的线路，省去中间结果句柄。全部接口输入线路不被修改，输出用 `circuit_free` 释放。

### transform_canonicalize(circuit)

```c
struct CCircuit *transform_canonicalize(const struct CCircuit *circuit);
```

以生产默认配置规范化线路并返回重建后的线路（等价于 `canonicalize_circuit` 后再取出线路）。

返回：成功返回新建的 `CCircuit*`；NULL 输入或执行失败返回 NULL。

### transform_knowledge_rewrite(circuit, config)

```c
struct CCircuit *transform_knowledge_rewrite(const struct CCircuit *circuit,
                                             struct CRewriteConfig config);
```

按给定配置执行知识重写并返回重建后的线路。

- `circuit` (`const CCircuit*`)：输入线路。
- `config` (`struct CRewriteConfig`)：重写配置，按值传递。

返回：成功返回新建的 `CCircuit*`；NULL 输入、`config.mode` 或目标指令基非法、执行失败返回 NULL。

### transform_optimize_one_qubit_runs(circuit)

```c
struct CCircuit *transform_optimize_one_qubit_runs(const struct CCircuit *circuit);
```

通过精确综合融合固定数值的单比特连续门串（目标中立的逻辑代价），产物操作数不会多于输入。

返回：成功返回新建的 `CCircuit*`；NULL 输入或执行失败返回 NULL。

### transform_commutative_cancellation(circuit)

```c
struct CCircuit *transform_commutative_cancellation(const struct CCircuit *circuit);
```

基于全局对易性分析抵消自反门对（如相邻的 `H·H`、`CX·CX`）。

返回：成功返回新建的 `CCircuit*`；NULL 输入或执行失败返回 NULL。

### transform_lower_to_routing_basis(circuit)

```c
struct CCircuit *transform_lower_to_routing_basis(const struct CCircuit *circuit);
```

把线路降级到路由门集：超出 SABRE 门元数约束的门（如 `CCX`）被展开为至多两比特的操作，为路由阶段做准备。

返回：成功返回新建的 `CCircuit*`；NULL 输入或执行失败返回 NULL。

---

## 示例

结构分析、规范化与知识重写：

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Structural analysis of a Bell-pair circuit */
    struct CCircuit *bell = circuit_new(2);
    circuit_h(bell, 0);
    circuit_cx(bell, 0, 1);

    struct CCircuitAnalysis analysis;
    memset(&analysis, 0, sizeof(analysis));
    if (circuit_analyze(bell, &analysis) != 0) {
        circuit_free(bell);
        return 1;
    }
    /* analysis.has_classical_control == 0, analysis.has_unitary_gates == 0 */

    /* 2. Canonicalize and read the result handle */
    struct CCanonicalizeResult *canon = canonicalize_circuit(bell);
    if (canon == NULL) {
        circuit_free(bell);
        return 1;
    }
    uint8_t rounds = canonicalize_result_rounds(canon);   /* >= 1 */
    struct CCircuit *canonical = canonicalize_result_circuit(canon);
    if (canonical != NULL) {
        circuit_free(canonical);  /* deep clone, independent of canon */
    }
    canonicalize_result_free(canon);

    /* 3. Knowledge rewrite with the production configuration */
    struct CRewriteConfig config = rewrite_config_default();
    struct CKnowledgeRewriteResult *rewritten = rewrite_circuit(bell, config);
    if (rewritten != NULL) {
        int32_t changed = knowledge_rewrite_result_changed(rewritten);
        struct CKnowledgeRewriteStats stats;
        struct CKnowledgeRewriteDiagnostics diagnostics;
        memset(&stats, 0, sizeof(stats));
        memset(&diagnostics, 0, sizeof(diagnostics));
        if (knowledge_rewrite_result_stats(rewritten, &stats) == 0) {
            printf("rewrite: rounds=%u rules=%llu\n",
                   stats.rounds_executed, (unsigned long long)stats.rules_applied);
        }
        knowledge_rewrite_result_diagnostics(rewritten, &diagnostics);
        knowledge_rewrite_result_free(rewritten);
    }
    circuit_free(bell);

    /* 4. Standalone pass wrappers return rebuilt circuits directly */
    struct CCircuit *runs = circuit_new(1);
    circuit_h(runs, 0);
    circuit_t(runs, 0);
    circuit_tdg(runs, 0);
    struct CCircuit *fused = transform_optimize_one_qubit_runs(runs);
    if (fused != NULL) {
        circuit_free(fused);
    }

    struct CCircuit *lowered = transform_lower_to_routing_basis(runs);
    if (lowered != NULL) {
        circuit_free(lowered);
    }
    circuit_free(runs);
    return 0;
}
```

带配置的规范化与变换配置句柄：

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a canonicalize config and tighten the round limit */
    struct CCanonicalizeConfig *config = canonicalize_config_default();
    if (config == NULL) {
        return 1;
    }
    /* canonicalize_config_round_limit(config) == 8, all optional behaviors on */
    canonicalize_config_with_round_limit(config, 2);

    struct CCircuit *circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);
    circuit_t(circuit, 1);

    /* 2. Canonicalize under the config; the config stays owned by the caller */
    struct CCanonicalizeResult *result = canonicalize_circuit_with_config(circuit, config);
    if (result != NULL) {
        struct CCircuit *canonical = canonicalize_result_circuit(result);
        /* canonicalize_result_rounds(result) <= 2 */
        circuit_free(canonical);
        canonicalize_result_free(result);
    }

    /* 3. Reuse the same config with a tighter limit */
    canonicalize_config_with_round_limit(config, 1);
    struct CCanonicalizeResult *limited = canonicalize_circuit_with_config(circuit, config);
    /* canonicalize_result_rounds(limited) <= 1 */
    canonicalize_result_free(limited);
    canonicalize_config_free(config);
    circuit_free(circuit);

    /* 4. Transform config: inspect defaults, then restrict in place */
    struct CTransformConfig *tconfig = transform_config_default();
    if (tconfig == NULL) {
        return 1;
    }
    /* transform_config_mode(tconfig) == REWRITE_MODE_OPTIMIZE,
       transform_config_enabled_kinds_len(tconfig) == 4 */
    const uint8_t kinds[2] = {REWRITE_KIND_CANCEL, REWRITE_KIND_DECOMPOSE};
    transform_config_with_enabled_kinds(tconfig, kinds, 2);

    const char *const gates[2] = {"H", "CX"};
    if (transform_config_with_target_instructions(tconfig, gates, 2) == 0) {
        char *basis[2] = {NULL, NULL};
        uintptr_t total = transform_config_target_instruction_basis(tconfig, basis, 2);
        for (uintptr_t i = 0; i < total && i < 2; i++) {
            cqlib_string_free(basis[i]);  /* each entry: newly allocated */
        }
    }
    transform_config_with_mode(tconfig, 99);  /* -8, config unchanged */
    transform_config_free(tconfig);
    return 0;
}
```

单比特门串经 `transform_optimize_one_qubit_runs` 融合后操作数不增；端到端编排这些 pass 的编译入口见 [编译器](1_compiler.md)，布局与路由见 [初始布局](3_layout.md)、[路由](4_routing.md)。
