# SABRE（C）

`CSabreRoutingResult` 是底层 SABRE 路由 `sabre_route` 的结果句柄。本页覆盖 SABRE 配置结构 `SabreConfigC` 的构造与校验、已知初始布局下的路由入口、路由结果读取，以及布局规范化 `normalize_initial_layout` 与可达性验证 `validate_reachable_interactions`。固定随机种子可复现多轮试探的平局裁决与选择。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；带布局选择的路由入口 `route_sabre` / `route_with_layout` 见 [路由](4_routing.md)。

---

## 常量

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `SABRE_MAX_LOOKAHEAD_WEIGHTS` | 8 | `SabreHeuristicConfigC.lookahead_weights` 数组长度；`num_lookahead_weights` 超过该值时配置无法转换（-8）。 |

---

## 配置构造与校验

### sabre_config_default()

返回核心默认 SABRE 配置（按值）：`layout_trials=10`、`layout_assignment_budget=1000000`、启用 VF2 预检查（`candidate_limit=10`、`call_limit=1000000`）、`refinement_iterations=1`、`routing_trials=1`、无种子；启发式为 `basic_weight=1.0`、`lookahead_weights=[0.5, 0.25, 0.125, 0.0625, 0.03125]`、启用拥塞控制（`decay_increment=0.002`、`decay_reset=10`）、`attempt_limit=1000`、`best_epsilon=1e-10`。

```c
struct SabreConfigC sabre_config_default(void);
```

返回：默认配置结构体（按值，无需释放）。

### sabre_config_deterministic_seeded(seed)

返回适合可复现运行的紧凑确定性配置：压缩试探规模（`layout_trials=2`、`layout_assignment_budget=100000`、VF2 预检查 `candidate_limit=10`、`call_limit=100000`、`refinement_iterations=1`、`routing_trials=1`），固定随机种子为 `seed`（`has_seed=1`），并把 `attempt_limit` 收紧到 20；其余字段与默认配置一致。

```c
struct SabreConfigC sabre_config_deterministic_seeded(uint64_t seed);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `seed` | `uint64_t` | 确定性随机种子。相同种子、相同线路与设备产生相同路由结果。 |

返回：确定性配置结构体（按值，无需释放）。

### sabre_config_validate(config)

校验配置的路由相关字段：`routing_trials` 必须大于 0；权重（`basic_weight`、各 `lookahead_weights`、`decay_increment`）必须有限且非负；启用拥塞控制时 `decay_reset` 必须大于 0；`best_epsilon` 必须有限且非负。布局细化字段（如 `layout_trials`）不参与校验。

```c
int32_t sabre_config_validate(const struct SabreConfigC *config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `config` | `const SabreConfigC*` | 待校验配置。 |

返回：0 合法。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `config` 为 NULL。 |
| `-6` | CompilerError | 核心校验拒绝：`routing_trials` 为 0、权重非法、启用拥塞控制但 `decay_reset` 为 0、`best_epsilon` 非法。 |
| `-8` | InvalidParam | `num_lookahead_weights` 超过 `SABRE_MAX_LOOKAHEAD_WEIGHTS`，配置无法转换。 |

---

## 路由入口

### sabre_route(circuit, device, initial_layout, config)

已知初始布局条件下的底层 SABRE 路由：插入 SWAP 使全部二体交互在设备可用物理拓扑上相邻，控制流块被递归路由并在离开块前恢复入口布局。返回线路使用物理比特编号。

```c
struct CSabreRoutingResult *sabre_route(const struct CCircuit *circuit,
                                        const struct CDevice *device,
                                        const struct CLayout *initial_layout,
                                        const struct SabreConfigC *config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `device` | `const CDevice*` | 目标设备（构造方式见 [设备与噪声属性](../2_device/2_properties_device.md)）。 |
| `initial_layout` | `const CLayout*` | 初始逻辑→物理映射（构造方式见 [Layout](../2_device/3_layout.md)）。 |
| `config` | `const SabreConfigC*` | SABRE 配置；NULL 使用默认配置。 |

返回：成功返回新建的 `CSabreRoutingResult*`（用 `sabre_routing_result_free` 释放）；任一句柄为 NULL、配置无法转换或校验失败（例如 `routing_trials` 为 0）或路由失败时返回 NULL。

---

## 结果读取

### sabre_routing_result_free(ptr)

释放路由结果句柄，允许传 NULL。

```c
void sabre_routing_result_free(struct CSabreRoutingResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CSabreRoutingResult*` | 待释放句柄，可为 NULL。 |

### sabre_routing_result_circuit(ptr)

取出插入 SWAP 后的物理线路（深拷贝，线路比特为物理比特编号）。

```c
struct CCircuit *sabre_routing_result_circuit(const struct CSabreRoutingResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；NULL 输入返回 NULL。

### sabre_routing_result_initial_layout(ptr)

取出选中试探使用的初始逻辑→物理布局（深拷贝）。

```c
struct CLayout *sabre_routing_result_initial_layout(const struct CSabreRoutingResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### sabre_routing_result_final_layout(ptr)

取出所有路由操作（含控制流尾声）执行完毕后的最终逻辑→物理布局（深拷贝）。

```c
struct CLayout *sabre_routing_result_final_layout(const struct CSabreRoutingResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### sabre_routing_result_swap_count(ptr)

返回插入的 SWAP 操作数量（含控制流尾声）。

```c
uintptr_t sabre_routing_result_swap_count(const struct CSabreRoutingResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | 路由结果句柄。 |

返回：SWAP 数量；NULL 输入返回 0。

### sabre_routing_result_diagnostics(ptr, out)

把路由诊断快照写入 `*out`。

```c
int32_t sabre_routing_result_diagnostics(const struct CSabreRoutingResult *ptr,
                                         struct CSabreRoutingDiagnostics *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRoutingResult*` | 路由结果句柄。 |
| `out` | `struct CSabreRoutingDiagnostics*` | 诊断快照输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |

---

## 布局工具

### normalize_initial_layout(logical_qubits, num_logical, device, initial_layout)

把完整逻辑→物理布局规范化到设备可用物理比特上：`logical_qubits` 中的每个逻辑比特必须已被 `initial_layout` 映射，且映射目标必须是设备的可用物理比特；返回布局的物理比特集合为设备全部可用物理比特。

```c
struct CLayout *normalize_initial_layout(const uint32_t *logical_qubits,
                                         uintptr_t num_logical,
                                         const struct CDevice *device,
                                         const struct CLayout *initial_layout);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `logical_qubits` | `const uint32_t*` | 逻辑比特 ID 数组；`num_logical` 为 0 时可为 NULL。 |
| `num_logical` | `uintptr_t` | 逻辑比特数。 |
| `device` | `const CDevice*` | 目标设备。 |
| `initial_layout` | `const CLayout*` | 待规范化的布局。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；`device` 或 `initial_layout` 为 NULL、`num_logical` 大于 0 但 `logical_qubits` 为 NULL，或规范化失败（逻辑比特未映射、映射到不可用物理比特）时返回 NULL。

### validate_reachable_interactions(circuit, device, initial_layout)

在不执行路由的前提下验证：从 `initial_layout` 出发，线路中的每个交互都可达（交互双方位于设备拓扑的同一连通分量，可通过 SWAP 就近）。验证前会先把布局规范化到设备可用物理比特。

```c
int32_t validate_reachable_interactions(const struct CCircuit *circuit,
                                        const struct CDevice *device,
                                        const struct CLayout *initial_layout);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `device` | `const CDevice*` | 目标设备。 |
| `initial_layout` | `const CLayout*` | 初始逻辑→物理映射。 |

返回：0 全部交互可达。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | 任一句柄为 NULL。 |
| `-6` | CompilerError | 存在不可达交互（例如交互一端落在无耦合的物理比特上）或布局无法规范化。 |

---

## SabreConfigC

SABRE 布局细化与路由共用的配置，按值传递：

```c
typedef struct SabreConfigC {
  uintptr_t layout_trials;             /* Randomized starting layouts. */
  uintptr_t layout_assignment_budget;  /* Max layout assignment states. */
  uint8_t vf2_prepass_enabled;         /* 1 enables the bounded VF2 prepass. */
  struct SabreVf2PrepassConfigC vf2_prepass;
  uintptr_t refinement_iterations;     /* Refinement rounds per layout trial. */
  uintptr_t routing_trials;            /* Complete routing trials per layout. */
  uint64_t seed;                       /* Used when has_seed != 0. */
  uint8_t has_seed;                    /* 1 uses seed. */
  struct SabreHeuristicConfigC heuristic;
  uint64_t _reserved[4];               /* Reserved padding. */
} SabreConfigC;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `layout_trials` | `uintptr_t` | 在确定性候选之外增加的随机化初始布局数量。 |
| `layout_assignment_budget` | `uintptr_t` | 布局组件分配状态的最大探索量；耗尽报告预算耗尽，区别于已证明不可行。 |
| `vf2_prepass_enabled` | `uint8_t` | 1 启用有界 VF2 预检查，0 关闭（此时 `vf2_prepass` 被忽略）。 |
| `vf2_prepass` | `struct SabreVf2PrepassConfigC` | VF2 预检查限制（`vf2_prepass_enabled != 0` 时使用）。 |
| `refinement_iterations` | `uintptr_t` | 每个布局试探的正向/反向细化轮数。 |
| `routing_trials` | `uintptr_t` | 每个完整细化布局执行的完整路由试探数，必须大于 0。 |
| `seed` | `uint64_t` | 确定性种子（`has_seed != 0` 时使用）。 |
| `has_seed` | `uint8_t` | 1 使用 `seed`，0 不设置种子。 |
| `heuristic` | `struct SabreHeuristicConfigC` | 交换选择启发式配置。 |
| `_reserved[4]` | `uint64_t[4]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |

### SabreVf2PrepassConfigC

```c
typedef struct SabreVf2PrepassConfigC {
  uintptr_t candidate_limit;  /* Complete perfect mappings scored. */
  uintptr_t call_limit;       /* Partial mapping extensions attempted. */
  uint64_t _reserved[2];      /* Reserved padding. */
} SabreVf2PrepassConfigC;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `candidate_limit` | `uintptr_t` | VF2 打分的完整完美映射数量上限。 |
| `call_limit` | `uintptr_t` | VF2 尝试的部分映射扩展数量上限。 |
| `_reserved[2]` | `uint64_t[2]` | 填充字段，保持结构体布局稳定。 |

### SabreHeuristicConfigC

SABRE 交换选择启发式配置：主分数组合当前前沿层距离和、按活跃层缩放的前瞻层与与乘法拥塞；有原生方案时在窄结构窗口内用精确原生双比特代价裁决候选。

```c
typedef struct SabreHeuristicConfigC {
  double basic_weight;     /* Front-layer total distance weight. */
  double lookahead_weights[SABRE_MAX_LOOKAHEAD_WEIGHTS];
  uintptr_t num_lookahead_weights;  /* Leading entries in use. */
  double decay_increment;  /* Congestion multiplier increment. */
  uint8_t decay_enabled;   /* 1 enables congestion control. */
  uintptr_t decay_reset;   /* Attempts before decay resets. */
  uintptr_t attempt_limit; /* Heuristic SWAPs before fallback. */
  double best_epsilon;     /* Tie tolerance for candidate scores. */
  uint8_t _reserved[8];    /* Reserved padding. */
} SabreHeuristicConfigC;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `basic_weight` | `double` | 当前前沿层总距离的权重，须有限且非负。 |
| `lookahead_weights` | `double[8]` | 各活跃层缩放前瞻层总和的权重；仅前 `num_lookahead_weights` 个条目生效，其余为填充。 |
| `num_lookahead_weights` | `uintptr_t` | `lookahead_weights` 中前导有效条目数，不得超过 `SABRE_MAX_LOOKAHEAD_WEIGHTS`。 |
| `decay_increment` | `double` | 某物理比特被启发式 SWAP 使用后其拥塞乘数的增量。 |
| `decay_enabled` | `uint8_t` | 1 启用拥塞控制（使用 `decay_increment`），0 关闭。 |
| `decay_reset` | `uintptr_t` | 衰减值重置前的启发式 SWAP 尝试次数；启用拥塞控制时必须大于 0。 |
| `attempt_limit` | `uintptr_t` | 未路由前沿节点时允许的连续启发式 SWAP 次数，超过后回退到最短路径逃逸。 |
| `best_epsilon` | `double` | 候选 SWAP 分数视为平局的浮点容差，须有限且非负。 |
| `_reserved[8]` | `uint8_t[8]` | 填充字段，保持结构体布局稳定。 |

---

## CSabreRoutingDiagnostics

路由过程诊断快照（`sabre_routing_result_diagnostics` 及 [路由](4_routing.md) 各 diagnostics 接口的输出）：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `trials_evaluated` | `uintptr_t` | 评估的路由试探数。 |
| `selected_trial_index` | `uintptr_t` | 选中的路由试探下标（从 0 开始）。 |
| `fallback_count` | `uintptr_t` | 最短路径回退的使用次数。 |
| `control_flow_blocks_routed` | `uintptr_t` | 递归路由的控制流块数。 |
| `two_qubit_depth` | `uintptr_t` | 选中路由操作流的 ASAP 双比特深度。 |
| `operation_count` | `uintptr_t` | 选中路由流中的操作总数。 |
| `native_two_qubit_count` | `uintptr_t` | 设备降级后预测的原生双比特操作数。 |
| `native_two_qubit_depth` | `uintptr_t` | 使用选中精确 qargs 原生方案的 ASAP 双比特深度。 |
| `native_total_depth` | `uintptr_t` | 使用选中精确 qargs 原生方案的 ASAP 总深度。 |
| `native_operation_count` | `uintptr_t` | 设备降级后预测的原生操作总数。 |
| `unknown_loop_count` | `uintptr_t` | 总执行代价未知的动态 `for`/`while` 循环数。 |
| `requirement_signature_count` | `uintptr_t` | 已准备的不同单比特/比特对需求签名数。 |
| `eager_pair_state_count` | `uintptr_t` | 目标急切保留的比特对放置下界状态数。 |
| `lazy_pair_l1_lookup_count` | `uintptr_t` | 选中试探发出的惰性比特对状态下界探测数。 |
| `lazy_pair_l1_hit_count` | `uintptr_t` | 选中试探由其本地 L1 缓存命中的探测数。 |
| `lazy_pair_l1_cached_count` | `uintptr_t` | 选中试探有界 L1 缓存保留的比特对状态条目数。 |

---

## 示例

直线拓扑 0–1–2 上，从恒等布局出发做底层 SABRE 路由，随后规范化布局并验证可达性：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Line topology 0 - 1 - 2 with a non-adjacent cx(0, 2) */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* 2. Deterministic seeded config: same seed -> same routing result */
    struct SabreConfigC config = sabre_config_deterministic_seeded(42);
    if (sabre_config_validate(&config) != 0) {
        circuit_free(circuit);
        device_free(device);
        return 1;
    }

    /* 3. Low-level SABRE route from the identity layout */
    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    struct CLayout *layout = layout_new(logical, 3, physical, 3);

    struct CSabreRoutingResult *result = sabre_route(circuit, device, layout, &config);
    if (result != NULL) {
        /* 4. Routed circuit, swap count, layouts, and diagnostics */
        struct CCircuit *routed = sabre_routing_result_circuit(result);
        uintptr_t swaps = sabre_routing_result_swap_count(result);  /* >= 1 */
        struct CLayout *final_layout = sabre_routing_result_final_layout(result);
        struct CSabreRoutingDiagnostics diagnostics;
        if (sabre_routing_result_diagnostics(result, &diagnostics) == 0) {
            printf("trials=%lu swaps=%lu\n",
                   (unsigned long)diagnostics.trials_evaluated,
                   (unsigned long)swaps);
        }
        circuit_free(routed);
        layout_free(final_layout);
        sabre_routing_result_free(result);
    }

    /* 5. Normalize the layout onto usable qubits */
    struct CLayout *normalized = normalize_initial_layout(logical, 3, device, layout);
    if (normalized != NULL) {
        layout_free(normalized);
    }

    /* 6. Reachability check: qubits 0 and 2 share one connected component */
    int32_t reachable = validate_reachable_interactions(circuit, device, layout);  /* 0 */

    layout_free(layout);
    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

需要自动选择初始布局时使用 `route_sabre`，见 [路由](4_routing.md)。
