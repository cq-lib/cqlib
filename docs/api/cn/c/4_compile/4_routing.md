# 路由（C）

本页覆盖变换层路由入口 `route_with_layout` / `route_sabre`，以及路由结果句柄 `CRoutedCircuit` / `CSabreRouteResult` 的全部读取接口：路由后线路、初始与最终布局、SWAP 数量、路由诊断与布局元数据。路由在设备拓扑上插入 SWAP，使线路中所有二体交互落在相邻物理比特上；路由后线路使用物理比特编号。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；`SabreConfigC` 配置结构、`CSabreRoutingDiagnostics` 诊断结构与低层 `sabre_route` 见 [SABRE](5_sabre.md)。

---

## 常量

布局目标标签（`route_sabre` 的 `objective` 参数）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | 仅按拓扑评分：距离与方向分量。 |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | 拓扑分数之上叠加保真度项（双比特门误差权重 10.0、读出误差权重 1.0）。 |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | 按设备数据自动选择：设备携带双比特门/读出误差数据时启用对应保真度项，否则退化为仅拓扑。 |
| `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` | 3 | 强制保真度项：与保真度感知相同，但设备不含可用校准数据时失败。 |

---

## 路由入口

### route_with_layout(circuit, device, initial_layout, config)

在调用方给定的初始布局上做确定性路由：不执行布局选择、细化或评分。适合已持有布局（例如来自布局搜索或上一次 SABRE 运行）的调用方。

```c
struct CRoutedCircuit *route_with_layout(const struct CCircuit *circuit,
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
| `config` | `const SabreConfigC*` | SABRE 配置；NULL 使用默认配置（见 [SABRE](5_sabre.md)）。 |

返回：成功返回新建的 `CRoutedCircuit*`（用 `routed_circuit_free` 释放）；任一句柄为 NULL、配置无法转换（例如 `num_lookahead_weights` 超过 8）或路由失败（配置非法、可用物理比特不足、交互不可达等）时返回 NULL。

### route_sabre(circuit, device, objective, config)

带布局选择的 SABRE 路由：先按 `objective` 选出初始布局，再从该布局路由原始正向线路。选中的布局伴随路由结果一起返回。

```c
struct CSabreRouteResult *route_sabre(const struct CCircuit *circuit,
                                      const struct CDevice *device,
                                      uint8_t objective,
                                      const struct SabreConfigC *config);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `circuit` | `const CCircuit*` | 输入线路。 |
| `device` | `const CDevice*` | 目标设备。 |
| `objective` | `uint8_t` | 布局目标，取 `LAYOUT_OBJECTIVE_*` 之一。 |
| `config` | `const SabreConfigC*` | SABRE 配置；NULL 使用默认配置。相同种子、相同线路与设备产生相同路由结果。 |

返回：成功返回新建的 `CSabreRouteResult*`（用 `sabre_route_result_free` 释放）；任一句柄为 NULL、`objective` 不是合法标签、配置无法转换或布局选择/路由失败时返回 NULL。

---

## RoutedCircuit 读取

`route_with_layout` 的结果句柄。

### routed_circuit_free(ptr)

释放路由结果句柄，允许传 NULL。

```c
void routed_circuit_free(struct CRoutedCircuit *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CRoutedCircuit*` | 待释放句柄，可为 NULL。 |

### routed_circuit_circuit(ptr)

取出路由后的物理线路（深拷贝，线路比特为物理比特编号，所有二体操作在可用物理拓扑上相邻）。

```c
struct CCircuit *routed_circuit_circuit(const struct CRoutedCircuit *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；NULL 输入返回 NULL。

### routed_circuit_initial_layout(ptr)

取出路由开始前的初始逻辑→物理布局（深拷贝）。

```c
struct CLayout *routed_circuit_initial_layout(const struct CRoutedCircuit *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### routed_circuit_final_layout(ptr)

取出所有路由操作执行完毕后的最终逻辑→物理布局（深拷贝）。

```c
struct CLayout *routed_circuit_final_layout(const struct CRoutedCircuit *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### routed_circuit_swap_count(ptr)

返回插入的 SWAP 操作数量。

```c
uintptr_t routed_circuit_swap_count(const struct CRoutedCircuit *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |

返回：SWAP 数量；NULL 输入返回 0。

### routed_circuit_diagnostics(ptr, out)

把路由诊断快照写入 `*out`（字段含义见 [SABRE](5_sabre.md) 的 `CSabreRoutingDiagnostics`）。

```c
int32_t routed_circuit_diagnostics(const struct CRoutedCircuit *ptr,
                                   struct CSabreRoutingDiagnostics *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |
| `out` | `struct CSabreRoutingDiagnostics*` | 诊断快照输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |

### routed_circuit_changed(ptr, original)

判断路由是否可观察地改动了 `original`：插入了 SWAP、物理比特集合与输入不同、全局相位改变、初始布局非恒等映射，或操作流出现结构差异，任一成立即视为改动。

```c
int32_t routed_circuit_changed(const struct CRoutedCircuit *ptr, const struct CCircuit *original);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CRoutedCircuit*` | 路由结果句柄。 |
| `original` | `const CCircuit*` | 路由前的原始线路。 |

返回：1 有改动；0 无改动。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `original` 为 NULL。 |

---

## SabreRouteResult 读取

`route_sabre` 的结果句柄，除路由数据外还携带选中布局的元数据。

### sabre_route_result_free(ptr)

释放路由结果句柄，允许传 NULL。

```c
void sabre_route_result_free(struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `struct CSabreRouteResult*` | 待释放句柄，可为 NULL。 |

### sabre_route_result_routed(ptr)

取出内嵌的路由结果与路由元数据（深拷贝）。

```c
struct CRoutedCircuit *sabre_route_result_routed(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CRoutedCircuit*`（用 `routed_circuit_free` 释放）；NULL 输入返回 NULL。

### sabre_route_result_circuit(ptr)

取出路由后的物理线路（深拷贝），等价于先取 `routed` 再读其线路。

```c
struct CCircuit *sabre_route_result_circuit(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；NULL 输入返回 NULL。

### sabre_route_result_initial_layout(ptr)

取出路由使用的初始逻辑→物理布局（深拷贝）。

```c
struct CLayout *sabre_route_result_initial_layout(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### sabre_route_result_final_layout(ptr)

取出路由完成后的最终逻辑→物理布局（深拷贝）。

```c
struct CLayout *sabre_route_result_final_layout(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### sabre_route_result_swap_count(ptr)

返回插入的 SWAP 操作数量。

```c
uintptr_t sabre_route_result_swap_count(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：SWAP 数量；NULL 输入返回 0。

### sabre_route_result_diagnostics(ptr, out)

把路由诊断快照写入 `*out`（字段含义见 [SABRE](5_sabre.md) 的 `CSabreRoutingDiagnostics`）。

```c
int32_t sabre_route_result_diagnostics(const struct CSabreRouteResult *ptr,
                                       struct CSabreRoutingDiagnostics *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |
| `out` | `struct CSabreRoutingDiagnostics*` | 诊断快照输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |

### sabre_route_result_changed(ptr, original)

判断路由是否可观察地改动了 `original`，判定规则与 `routed_circuit_changed` 一致。

```c
int32_t sabre_route_result_changed(const struct CSabreRouteResult *ptr,
                                   const struct CCircuit *original);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |
| `original` | `const CCircuit*` | 路由前的原始线路。 |

返回：1 有改动；0 无改动。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `original` 为 NULL。 |

---

## 布局元数据

`route_sabre` 额外记录的选中布局信息。

### sabre_route_result_layout_score(ptr, out)

把选中初始布局的分数快照写入 `*out`。该分数只作诊断用途，SABRE 按预测原生路由质量选择胜者。

```c
int32_t sabre_route_result_layout_score(const struct CSabreRouteResult *ptr,
                                        struct CLayoutScore *out);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |
| `out` | `struct CLayoutScore*` | 布局分数快照输出。 |

返回：0 成功。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 或 `out` 为 NULL。 |
| `-8` | InvalidParam | 结果未携带布局分数。 |

`CLayoutScore` 字段：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `total` | `double` | 按布局目标加权后的总分。 |
| `distance` | `double` | 加权距离分量的原始值。 |
| `direction` | `double` | 方向失配分量的原始值。 |
| `two_qubit_error` | `double` | 未加权的有效双比特放置代价。 |
| `readout_error` | `double` | 读出误差分量的原始值。 |
| `used_fidelity` | `uint8_t` | 1 表示布局目标使用了保真度项。 |
| `_reserved[7]` | `uint8_t[7]` | 填充字段，保持结构体布局稳定。 |

### sabre_route_result_layout_is_perfect(ptr)

判断选中布局是否把全部正权重交互直接落在相邻硬件边上（无需任何 SWAP）。

```c
int32_t sabre_route_result_layout_is_perfect(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：1 完美；0 非完美。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL。 |

### sabre_route_result_layout_candidates_evaluated(ptr)

返回选择初始布局时评估过的候选布局数量。

```c
uintptr_t sabre_route_result_layout_candidates_evaluated(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：候选数量；NULL 输入返回 0。

### sabre_route_result_layout_used_fidelity(ptr)

判断保真度数据是否参与了选中布局的评分。

```c
int32_t sabre_route_result_layout_used_fidelity(const struct CSabreRouteResult *ptr);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |

返回：1 参与了评分；0 未参与。

| 错误码 | 名称 | 场景 |
| --- | --- | --- |
| `-1` | NullPtr | `ptr` 为 NULL。 |

### sabre_route_result_layout_notes_len(ptr) / sabre_route_result_layout_note(ptr, index)

两步式读取布局诊断备注：`*_len` 返回备注条数（NULL 返回 0），`layout_note` 返回第 `index` 条备注。

```c
uintptr_t sabre_route_result_layout_notes_len(const struct CSabreRouteResult *ptr);
```

```c
char *sabre_route_result_layout_note(const struct CSabreRouteResult *ptr, uintptr_t index);
```

参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `ptr` | `const CSabreRouteResult*` | 路由结果句柄。 |
| `index` | `uintptr_t` | 备注下标（`layout_note`）。 |

返回：`layout_note` 返回堆上 C 字符串，用 `cqlib_string_free` 释放；NULL 输入或下标越界返回 NULL。

---

## 示例

直线拓扑 0–1–2 上路由 `cx(0, 2)`，并读取两类结果的全部元数据：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Line topology 0 - 1 - 2 and a non-adjacent cx(0, 2) */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* 2. Route from an explicit identity layout (logical i -> physical i) */
    uint32_t logical[3] = {0, 1, 2};
    uint32_t physical[3] = {0, 1, 2};
    struct CLayout *layout = layout_new(logical, 3, physical, 3);

    struct CRoutedCircuit *routed = route_with_layout(circuit, device, layout, NULL);
    if (routed != NULL) {
        /* 3. Read the routed circuit, layouts, and diagnostics */
        uintptr_t swaps = routed_circuit_swap_count(routed);      /* >= 1 */
        int32_t changed = routed_circuit_changed(routed, circuit); /* 1 */
        struct CCircuit *physical_circuit = routed_circuit_circuit(routed);
        struct CLayout *initial = routed_circuit_initial_layout(routed);
        struct CLayout *final_layout = routed_circuit_final_layout(routed);
        struct CSabreRoutingDiagnostics diagnostics;
        if (routed_circuit_diagnostics(routed, &diagnostics) == 0) {
            printf("trials=%lu swaps=%lu changed=%d\n",
                   (unsigned long)diagnostics.trials_evaluated,
                   (unsigned long)swaps, changed);
        }
        circuit_free(physical_circuit);
        layout_free(initial);
        layout_free(final_layout);
        routed_circuit_free(routed);
    }
    layout_free(layout);

    /* 4. route_sabre: select a layout and route in one call */
    struct CSabreRouteResult *result =
        route_sabre(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    if (result != NULL) {
        struct CLayoutScore score;
        if (sabre_route_result_layout_score(result, &score) == 0) {
            printf("layout score total=%f perfect=%d\n",
                   score.total, sabre_route_result_layout_is_perfect(result));
        }
        printf("candidates=%lu\n",
               (unsigned long)sabre_route_result_layout_candidates_evaluated(result));

        /* 5. Walk the layout diagnostic notes (two-step pattern) */
        uintptr_t notes = sabre_route_result_layout_notes_len(result);
        for (uintptr_t i = 0; i < notes; i++) {
            char *note = sabre_route_result_layout_note(result, i);
            if (note != NULL) {
                cqlib_string_free(note);
            }
        }

        struct CRoutedCircuit *inner = sabre_route_result_routed(result);
        if (inner != NULL) {
            routed_circuit_free(inner);
        }
        sabre_route_result_free(result);
    }

    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

把路由结果降级到设备原生门集见 [分解与重综合](6_decompose_resynthesis.md) 的 `decompose_lower_to_device`；完整的编译工作流（含路由与降级）见 [编译器](1_compiler.md)。
