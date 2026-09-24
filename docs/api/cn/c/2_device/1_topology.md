# Topology（C）

`CTopology` 是硬件连接图的不透明句柄：物理比特与它们之间的有向耦合。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；本页所有接口均在 `cqlib_c.h` 中声明。

`CTopology` 句柄有两个来源：`topology_line` 从比特 ID 数组按直线形态构造；`device_topology` 返回设备拓扑的克隆副本（见 [Device / Properties](2_properties_device.md)）。耦合是有向的：`(control, target)` 上有耦合不代表反方向也有。

---

## 构造与释放

### topology_line(qubits, num_qubits)

按给定顺序构造一条有向直线 `qubits[0] -> qubits[1] -> ...`，耦合名为空串。

参数：

- `qubits` (`const uint32_t*`)：物理比特 ID 数组。
- `num_qubits` (`uintptr_t`)：数组长度。

返回：成功返回新建的 `CTopology*`；指针为 NULL 或构造失败时返回 NULL。

### topology_free(ptr)

释放 `CTopology`，允许传 NULL。

参数：

- `ptr` (`struct CTopology*`)：待释放的句柄。

### topology_num_qubits(ptr)

返回拓扑中的比特数；`ptr` 为 NULL 时返回 0。

参数：

- `ptr` (`const struct CTopology*`)：拓扑句柄。

### topology_num_couplings(ptr)

返回拓扑中的耦合边数；`ptr` 为 NULL 时返回 0。

参数：

- `ptr` (`const struct CTopology*`)：拓扑句柄。

---

## 比特与耦合编辑

### topology_add_qubits(ptr, qubits, len)

向拓扑添加一组比特。

参数：

- `ptr` (`struct CTopology*`)：拓扑句柄。
- `qubits` (`const uint32_t*`)：比特 ID 数组。
- `len` (`uintptr_t`)：数组长度。

返回：0 成功；-1 空指针；-8 比特已存在于拓扑中或数组内重复。

### topology_remove_qubits(ptr, qubits, len)

移除一组比特及所有触及它们的耦合。

参数：

- `ptr` (`struct CTopology*`)：拓扑句柄。
- `qubits` (`const uint32_t*`)：比特 ID 数组。
- `len` (`uintptr_t`)：数组长度。

返回：0 成功；-1 空指针；-2 比特不在拓扑中；-8 数组内重复。

### topology_add_couplings(ptr, edges, num_edges, names)

添加一组有向耦合边。`edges` 指向 `2 * num_edges` 个 u32，按 `(control, target)` 对连续排列；`names` 可选地指向 `num_edges` 个 C 字符串（NULL 条目或 NULL 数组表示该耦合不带名字）。

参数：

- `ptr` (`struct CTopology*`)：拓扑句柄。
- `edges` (`const uint32_t*`)：耦合边数组。
- `num_edges` (`uintptr_t`)：边的数量。
- `names` (`const char *const*`)：耦合名数组，可为 NULL。

返回：0 成功；-1 空指针；-4 名字不是合法 UTF-8；-2 端点比特不在拓扑中；-8 重复或自耦合。

### topology_remove_couplings(ptr, edges, num_edges)

移除一组有向耦合边。`edges` 布局与 `topology_add_couplings` 相同。

参数：

- `ptr` (`struct CTopology*`)：拓扑句柄。
- `edges` (`const uint32_t*`)：耦合边数组。
- `num_edges` (`uintptr_t`)：边的数量。

返回：0 成功；-1 空指针；-2 比特或耦合不在拓扑中；-8 耦合在数组内重复。

---

## 查询与遍历

### topology_contains_qubit(ptr, qubit)

判断比特是否属于拓扑。

参数：

- `ptr` (`const struct CTopology*`)：拓扑句柄。
- `qubit` (`uint32_t`)：比特 ID。

返回：1 属于；0 不属于；-1 空指针。

### topology_get_coupling_name(ptr, control, target)

返回有向耦合 `control -> target` 的名字；耦合存在但未命名时返回空字符串。

参数：

- `ptr` (`const struct CTopology*`)：拓扑句柄。
- `control` (`uint32_t`)：控制比特 ID。
- `target` (`uint32_t`)：目标比特 ID。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；输入为 NULL 或耦合不存在时返回 NULL。

### topology_qubits_len(ptr) / topology_qubits(ptr, out, len)

两步式输出拓扑中的全部比特 ID。

- `topology_qubits_len(ptr)` 返回比特总数，`ptr` 为 NULL 时返回 0。
- `topology_qubits(ptr, out, len)` 把比特 ID 拷贝到 `out`，返回总数。

### topology_neighbors_undirected_len(ptr, qubit) / topology_neighbors_undirected(ptr, qubit, out, len)

两步式输出比特的无向邻居（后继与前驱去重，升序排列）。比特不在拓扑中或 `ptr` 为 NULL 时长度为 0。

参数：

- `qubit` (`uint32_t`)：比特 ID。

### topology_predecessors_len(ptr, qubit) / topology_predecessors(ptr, qubit, out, len)

两步式输出比特的前驱（有耦合指向它的比特，升序排列）。

### topology_successors_len(ptr, qubit) / topology_successors(ptr, qubit, out, len)

两步式输出比特的后继（经出边可达的比特，升序排列）。

### topology_undirected_edges_len(ptr) / topology_undirected_edges(ptr, out, len)

两步式输出去重后的无向耦合对：双向耦合折叠为一对，按 `(low, high)` 升序排列。`out` 需容纳 `2 * len` 个 u32，返回对的总数。

---

## 度与支持性

### topology_in_degree(ptr, qubit)

返回比特的入度（指向它的耦合数）；`ptr` 为 NULL 或比特不在拓扑中时返回 0。

### topology_out_degree(ptr, qubit)

返回比特的出度（从它出发的耦合数）；`ptr` 为 NULL 或比特不在拓扑中时返回 0。

### topology_supports_directed_coupling(ptr, control, target)

判断有向耦合 `control -> target` 是否存在。

返回：1 存在；0 不存在；-1 空指针。

### topology_supports_coupling_either_direction(ptr, a, b)

判断两个比特之间是否存在任一方向的耦合。

返回：1 存在；0 不存在；-1 空指针。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t ids[3] = {0, 1, 2};
    struct CTopology *topo = topology_line(ids, 3);
    if (topo == NULL) {
        return 1;
    }

    /* 3 个比特、2 条有向耦合：0->1、1->2 */
    printf("qubits=%llu couplings=%llu\n",
           (unsigned long long)topology_num_qubits(topo),
           (unsigned long long)topology_num_couplings(topo));

    /* 连接性 */
    int32_t fwd = topology_supports_directed_coupling(topo, 1, 2);       /* 1 */
    int32_t bwd = topology_supports_directed_coupling(topo, 2, 1);       /* 0 */
    int32_t either = topology_supports_coupling_either_direction(topo, 2, 1);  /* 1 */

    /* 遍历比特 1 的邻居与度 */
    uintptr_t succ_len = topology_successors_len(topo, 1);  /* 1 */
    uint32_t succ[1];
    topology_successors(topo, 1, succ, succ_len);          /* {2} */

    uintptr_t nb_len = topology_neighbors_undirected_len(topo, 1);  /* 2 */
    uint32_t nb[2];
    topology_neighbors_undirected(topo, 1, nb, nb_len);             /* {0, 2} */

    uintptr_t in_deg = topology_in_degree(topo, 1);   /* 1 */
    uintptr_t out_deg = topology_out_degree(topo, 1);  /* 1 */

    /* 添加并移除比特与耦合 */
    uint32_t extra[1] = {3};
    if (topology_add_qubits(topo, extra, 1) == 0) {
        uint32_t edge[2] = {2, 3};
        const char *names[1] = {"CX"};
        topology_add_couplings(topo, edge, 1, names);
        topology_remove_couplings(topo, edge, 1);
        topology_remove_qubits(topo, extra, 1);
    }

    /* 无向边 */
    uintptr_t pairs = topology_undirected_edges_len(topo);  /* 2 */
    uint32_t buf[4];
    topology_undirected_edges(topo, buf, pairs);  /* {0,1, 1,2} */

    topology_free(topo);
    return 0;
}
```
