# Layout（C）

`CLayout` 是逻辑比特到物理比特映射的不透明句柄，维护路由过程中的逻辑↔物理双向映射；未承载逻辑比特的物理比特是空闲物理比特。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 构造与释放

### layout_new(logical, num_logical, physical, num_physical)

创建一个布局：`logical` 数组中的逻辑比特按顺序映射到 `physical` 数组中的物理比特。

参数：

- `logical` (`const uint32_t*`)：逻辑比特 ID 数组。
- `num_logical` (`uintptr_t`)：逻辑比特数，必须非零。
- `physical` (`const uint32_t*`)：物理比特 ID 数组。
- `num_physical` (`uintptr_t`)：物理比特数，必须与 `num_logical` 相同。

返回：成功返回新建的 `CLayout*`；失败返回 NULL。

### layout_from_pairs(pairs, num_pairs, physical_count)

由 `(logical, physical)` 对创建布局。`pairs` 指向 `2 * num_pairs` 个 u32，按 `(逻辑, 物理)` 对连续排列；`physical_count` 定义物理比特总数（`0..physical_count`），未被引用的物理比特保持空闲。

参数：

- `pairs` (`const uint32_t*`)：比特对数组。
- `num_pairs` (`uintptr_t`)：对数，必须非零。
- `physical_count` (`uint32_t`)：物理比特总数。

返回：成功返回新建的 `CLayout*`；失败返回 NULL。

### layout_free(ptr)

释放 `CLayout`，允许传 NULL。

---

## 映射查询

### layout_get(ptr, logical)

返回逻辑比特映射到的物理比特。

返回：已映射时返回物理比特 ID；未映射或出错时返回 `UINT32_MAX`。

### layout_get_logical(ptr, physical)

返回物理比特承载的逻辑比特（`layout_get` 的反向查询）。

返回：已占用时返回逻辑比特 ID；物理比特空闲或未知时返回 `UINT32_MAX`，调用方需将其视为哨兵值。

### layout_is_physical_vacant(ptr, physical)

判断物理比特是否属于布局且空闲。

返回：1 空闲；0 已占用或未知；-1 空指针。

### layout_num_logical(ptr)

返回已映射的逻辑比特数；`ptr` 为 NULL 时返回 0。

### layout_num_physical(ptr)

返回物理比特总数；`ptr` 为 NULL 时返回 0。

### layout_num_vacant_physical(ptr)

返回空闲物理比特数；`ptr` 为 NULL 时返回 0。

---

## 绑定与交换

### layout_bind(ptr, logical, physical)

把一个未映射的逻辑比特绑定到空闲物理比特上。

返回：0 成功；-1 空指针；-2 物理比特不属于该布局；-8 任一侧已参与映射。

### layout_unbind(ptr, logical, out_physical)

解除逻辑比特的映射，并把释放的物理比特 ID 写到 `*out_physical`。

返回：0 成功；-1 空指针；-8 该逻辑比特未绑定。

### layout_swap_physical(ptr, phys_a, phys_b)

交换两个物理比特上承载的逻辑比特；任一侧可为空闲比特（与空闲比特交换即把逻辑比特搬到空闲位置），这是路由过程中移动逻辑比特的基本操作。

返回：0 成功；-1 空指针；-2 任一物理比特不属于该布局。

---

## 整体读取

以下接口均为两步式：先取长度，再填充缓冲区，填充函数返回总数。

### layout_l2p_map_len(ptr) / layout_l2p_map(ptr, out, len)

逻辑→物理映射按 `(logical, physical)` 对输出，按逻辑 ID 升序排列；`out` 需容纳 `2 * len` 个 u32。

### layout_p2l_map_len(ptr) / layout_p2l_map(ptr, out, len)

物理→逻辑映射按 `(physical, logical)` 对输出，按物理 ID 升序排列；`out` 需容纳 `2 * len` 个 u32。

### layout_logical_qubits_len(ptr) / layout_logical_qubits(ptr, out, len)

已映射的逻辑比特 ID，升序输出。

### layout_physical_qubits_len(ptr) / layout_physical_qubits(ptr, out, len)

布局可用的全部物理比特 ID（含空闲比特），升序输出。

### layout_vacant_physical_qubits_len(ptr) / layout_vacant_physical_qubits(ptr, out, len)

空闲物理比特 ID，升序输出。

---

## 布局结果辅助

以下接口读取 SABRE 布局产物：物理布局图的全对最短距离表，以及线路布局分析中每个逻辑比特的活动度权重。句柄来自编译侧布局 API（[初始布局](../4_compile/3_layout.md)）：`CPhysicalLayoutGraph` 来自 `physical_layout_graph_from_device`，`CCircuitLayoutAnalysis` 来自 `analyze_circuit_for_layout`（或 `layout_result_analysis`），`CPreparedSabreCircuit` 来自 `prepare_sabre_circuit`。

### layout_result_distances(ptr, out, len)

把物理布局图的全对无向最短路径距离表复制到 `out`（两步式）。表按图的物理比特 ID 升序行优先排列（`n` 个比特共 `n * n` 项；轴顺序可用 `physical_layout_graph_physical_qubits` 查询）。不可达的比特对写入 `UINT32_MAX`。

参数：

- `ptr` (`const struct CPhysicalLayoutGraph*`)：物理布局图。
- `out` (`uint32_t*`)：接收距离表；传 NULL 仅查询长度。
- `len` (`uintptr_t`)：缓冲区容量（项数）；值较小时只复制前 `len` 项。

返回：总项数（`n * n`）；`ptr` 为 NULL 时返回 0。

### layout_result_logical_activity(ptr, out_qubits, out_activity, len)

把每个逻辑比特的活动度权重（该比特所有关联交互权重之和）复制到平行数组，按逻辑 ID 升序（两步式）：`out_qubits[i]` 接收逻辑比特 ID，`out_activity[i]` 接收对应活动度。任一数组为 NULL 仅当两者同时为 NULL（长度查询）时允许。

参数：

- `ptr` (`const struct CCircuitLayoutAnalysis*`)：线路布局分析。
- `out_qubits` (`uint32_t*`)：接收逻辑比特 ID。
- `out_activity` (`double*`)：接收活动度权重。
- `len` (`uintptr_t`)：缓冲区容量（项数）；值较小时只复制前 `len` 项。

返回：总项数；`ptr` 为 NULL 时返回 0。

### layout_result_analysis(ptr)

把 SABRE 预备线路中捕获的线路布局分析提取为独立的 `CCircuitLayoutAnalysis*`（与 `analyze_circuit_for_layout` 返回相同的句柄类型），用 `circuit_layout_analysis_free` 释放。

参数：

- `ptr` (`const struct CPreparedSabreCircuit*`)：SABRE 预备线路。

返回：成功返回新建的 `CCircuitLayoutAnalysis*`；输入为 NULL 时返回 NULL。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 逻辑 {0, 1} 按序映射到物理 {10, 11}，物理总数 3（12 空闲） */
    uint32_t logical[2] = {0, 1};
    uint32_t physical[3] = {10, 11, 12};
    struct CLayout *layout = layout_new(logical, 2, physical, 3);
    if (layout == NULL) {
        return 1;
    }

    uintptr_t n_logical = layout_num_logical(layout);          /* 2 */
    uintptr_t n_physical = layout_num_physical(layout);        /* 3 */
    uintptr_t n_vacant = layout_num_vacant_physical(layout);  /* 1 */

    uint32_t p = layout_get(layout, 0);                 /* 10 */
    uint32_t l = layout_get_logical(layout, 10);        /* 0 */
    uint32_t vacant_l = layout_get_logical(layout, 12); /* UINT32_MAX（空闲） */
    int32_t is_vacant = layout_is_physical_vacant(layout, 12);  /* 1 */

    /* 整体读取 */
    uintptr_t pairs_len = layout_l2p_map_len(layout);  /* 2 */
    uint32_t pairs[4];
    layout_l2p_map(layout, pairs, pairs_len);         /* {0,10, 1,11} */

    /* 交换物理 11 与 12：逻辑 1 移到 12 */
    layout_swap_physical(layout, 11, 12);

    /* 解绑逻辑 0：释放物理 10 */
    uint32_t freed = 0;
    layout_unbind(layout, 0, &freed);  /* 0，freed == 10 */

    /* 重新绑定到空闲比特 11 */
    int32_t rc = layout_bind(layout, 0, 11);  /* 0 */

    /* 布局结果辅助 */
    struct CDevice *dev = device_line("mock_backend", 3);
    struct CPhysicalLayoutGraph *graph = physical_layout_graph_from_device(dev);
    uintptr_t n_dist = layout_result_distances(graph, NULL, 0);  /* 9（3x3） */
    uint32_t dist[9];
    layout_result_distances(graph, dist, n_dist);
    /* dist == {0,1,2, 1,0,1, 2,1,0}，行优先 */

    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 1);
    circuit_cx(circuit, 0, 1);
    circuit_cx(circuit, 1, 2);

    struct CCircuitLayoutAnalysis *analysis = analyze_circuit_for_layout(circuit);
    uintptr_t n_act = layout_result_logical_activity(analysis, NULL, NULL, 0);  /* 3 */
    uint32_t lq[3];
    double act[3];
    layout_result_logical_activity(analysis, lq, act, n_act);
    /* lq == {0,1,2}，act == {2.0, 3.0, 1.0} */
    circuit_layout_analysis_free(analysis);

    /* 复用 SABRE 预备线路中捕获的分析 */
    struct CPreparedSabreCircuit *prepared = prepare_sabre_circuit(circuit);
    struct CCircuitLayoutAnalysis *reused = layout_result_analysis(prepared);
    circuit_layout_analysis_free(reused);
    prepared_sabre_circuit_free(prepared);

    circuit_free(circuit);
    physical_layout_graph_free(graph);
    device_free(dev);

    layout_free(layout);
    return 0;
}
```
