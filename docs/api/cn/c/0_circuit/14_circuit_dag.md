# 线路 DAG

`CCircuitDag*` 是线路在操作依赖层面的视图：节点表示一条操作，边表示两条操作因共享同一份资源而产生的先后顺序。可共享的资源包括量子比特、可变经典变量、不可变经典值，以及承载无具体数据资源之操作的全局序；每条线在图中对应一对输入 / 输出哨兵节点。典型流程：`circuit_dag_from_circuit` → 检查与校验 → `circuit_dag_to_circuit`；DAG 还支持深入查询（节点识别、遍历、分层、线时间线、门连续段、控制流），并在还原回线路之前就地修改（追加、删除、替换）。错误码、句柄释放与两步式数组输出等通用约定见 [Overview](../0_overview.md)。

---

## 构造与释放

### circuit_dag_from_circuit(ptr)

从已有线路构造依赖 DAG，同时复制线路的量子比特、参数表、符号名、经典变量与经典值类型表以及全局相位；构造结束时 DAG 已通过内部校验。

- `ptr` (`const struct CCircuit*`)：源线路句柄。

返回：新分配的 `CCircuitDag*`（用 `circuit_dag_free` 释放）；`ptr` 为 NULL 或构造失败时返回 NULL。

### circuit_dag_from_operations(qubits, qubits_len, ops, ops_len)

从显式给定的量子比特与操作切片构造依赖 DAG，面向局部分析的窄入口：不接收参数表、经典声明与全局相位；`qubits` 的每个元素直接作为量子比特编号，每条操作必须作用于已注册编号，重复编号、越界引用或非自包含操作（含参数表索引、经典变量 / 值引用或控制流体）都会使构造失败。

- `qubits` (`const uint32_t*`)：量子比特编号数组。
- `qubits_len` (`uintptr_t`)：量子比特数量。
- `ops` (`const struct COperation* const*`)：操作句柄数组（用 `operation_new` 创建，见 [操作与指令](4_operation_instruction.md)）。
- `ops_len` (`uintptr_t`)：操作数量。

返回：新分配的 `CCircuitDag*`（用 `circuit_dag_free` 释放）；输入数组为 NULL 而其长度为正（`qubits_len > 0` 时 `qubits` 为 NULL、`ops_len > 0` 时 `ops` 为 NULL、数组含 NULL 元素）或构造失败时返回 NULL。

### circuit_dag_free(ptr)

释放 DAG 句柄。

- `ptr` (`struct CCircuitDag*`)：待释放句柄，允许传 NULL。

---

## 转换与校验

### circuit_dag_to_circuit(ptr)

按源顺序从 DAG 重建线路，并恢复量子比特集合、经典类型表与全局相位；源句柄保持存活且不变。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：新分配的 `CCircuit*`（用 `circuit_free` 释放）；`ptr` 为 NULL 或还原失败（图含环、非法参数索引等）时返回 NULL。

### circuit_dag_validate(ptr)

校验 DAG 的一致性：图是否含环、每条线的输入输出哨兵是否合法、操作引用的量子比特与参数索引是否有效、边承载的资源是否合法等。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-3` 图含环或非法。

---

## 元数据与统计

### circuit_dag_num_qubits(ptr)

返回量子比特数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：量子比特数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_num_ops(ptr)

返回操作节点数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：操作节点数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_is_empty(ptr)

判断 DAG 是否没有操作节点。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：`1` 为空；`0` 非空；`-1` `ptr` 为 NULL。

### circuit_dag_qubits_len(ptr)

返回 DAG 中量子比特编号的数量，用于分配输出缓冲区。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：编号数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_qubits(ptr, buffer, len)

按注册顺序把量子比特编号拷贝进 `buffer`（两步式：先 `circuit_dag_qubits_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或编号非空时 `buffer` 为 NULL；`-8` `len` 不足。

### circuit_dag_parameters_len(ptr)

返回已注册参数的数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：参数数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_parameters(ptr, out, len)

按注册顺序把参数名拷贝进 `out`（两步式：先 `circuit_dag_parameters_len`）。每个写出元素是新分配的 C 字符串，需分别用 `cqlib_string_free` 释放。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `out` (`char**`)：输出数组。
- `len` (`uintptr_t`)：数组容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或参数非空时 `out` 为 NULL；`-8` `len` 不足。

### circuit_dag_add_parameter(ptr, param, out_index, out_inserted)

把 `param` 的拷贝驻留到 DAG 的参数表中：参数不存在时插入，并注册其全部符号名。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `param` (`const struct CParameter*`)：参数句柄，用 `param_parse` 创建（见 [符号参数](3_parameter.md)）。
- `out_index` (`uintptr_t*`)：参数在表中的索引的可选输出；允许传 NULL。
- `out_inserted` (`int32_t*`)：可选输出标志；`1` 表示本次新插入，`0` 表示已存在；允许传 NULL。

返回：`0` 成功；`-1` `ptr` 或 `param` 为 NULL。

### circuit_dag_symbols_len(ptr)

返回 DAG 引用的自由符号名数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：符号数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_symbols(ptr, out, len)

把符号名拷贝进 `out`（两步式：先 `circuit_dag_symbols_len`）。每个写出元素是新分配的 C 字符串，需分别用 `cqlib_string_free` 释放。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `out` (`char**`)：输出数组。
- `len` (`uintptr_t`)：数组容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或符号非空时 `out` 为 NULL；`-8` `len` 不足。

---

## 线

边承载的资源称为线（wire），同时也是线的身份。量子比特线在构造时全部物化；经典变量线与经典值线只在被操作引用时物化；全局序线始终存在。线由 `(tag, id)` 标识：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `DAG_WIRE_QUBIT` | 0 | 量子比特线，`id` 为量子比特编号 |
| `DAG_WIRE_CLASSICAL_VAR` | 1 | 可变经典变量线，`id` 为变量编号 |
| `DAG_WIRE_CLASSICAL_VALUE` | 2 | 不可变经典值线，`id` 为值索引 |
| `DAG_WIRE_GLOBAL_ORDER` | 3 | 全局序线，无载荷（`id` 为 0） |

### circuit_dag_wires_len(ptr)

返回 DAG 中已物化线的数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：线数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_wires(ptr, tags, ids, len)

把已物化线的标签与载荷编号拷贝进 `tags` 与 `ids` 两个并行数组（两步式：先 `circuit_dag_wires_len`）。全局序线在前，其后依次为各条量子比特线；`DAG_WIRE_GLOBAL_ORDER` 线的 `id` 为 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tags` (`uint32_t*`)：`DAG_WIRE_*` 标签数组。
- `ids` (`uint32_t*`)：载荷编号数组。
- `len` (`uintptr_t`)：数组容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或线非空时任一数组为 NULL；`-8` `len` 不足。

### circuit_dag_has_wire(ptr, tag, id)

判断 DAG 是否已物化给定线。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT`（`id` 为量子比特编号）或 `DAG_WIRE_GLOBAL_ORDER`（`id` 被忽略）。
- `id` (`uint32_t`)：线的载荷编号。

返回：`1` 已物化；`0` 未物化；`-1` `ptr` 为 NULL；`-8` 其他 `tag` 取值。

---

## 节点识别

图中每个节点——操作节点或线哨兵——都由一个 `uint32_t` 节点编号标识。以下接口用于区分节点种类并取回存储内容。

### circuit_dag_is_operation(ptr, node)

判断 `node` 是否为操作节点。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：`1` 为操作节点；`0` 为线哨兵；`-1` `ptr` 为 NULL；`-2` 未知节点编号。

### circuit_dag_node_kind(ptr, node)

以新分配的 C 字符串返回 `node` 的种类：`"wire_in"`、`"wire_out"` 或 `"operation"`。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：种类字符串（用 `cqlib_string_free` 释放）；`ptr` 为 NULL 或节点编号未知时返回 NULL。

### circuit_dag_operation(ptr, node)

返回操作节点所存操作的一份拷贝。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：新分配的 `COperation*`（用 `operation_free` 释放，见 [操作与指令](4_operation_instruction.md)）；`ptr` 为 NULL 或该节点不是操作节点时返回 NULL。

---

## 图遍历

所有遍历接口都采用两步式数组输出：先用 `_len` 变体查询数量，再调用拷贝变体。各查询列举的内容有所不同：

- `circuit_dag_op_nodes` / `circuit_dag_topological_op_nodes` 分别按源顺序和确定性拓扑顺序列举操作节点（并列时按字典序打破，因此顺序在多次运行之间稳定）。
- `circuit_dag_predecessors` / `circuit_dag_successors` 返回节点的原始图邻居：包含线哨兵，每条连接边计一次（两个节点共享多份资源时，每条线各连一条边——同一条线上绝不产生平行边），且顺序不指定。用 `_len` 变体确定缓冲区大小总是足够的。
- 按线过滤的查询（`*_on_wire`、`quantum_*`、`classical_*`）只返回操作邻居，已去重并按确定性拓扑键排序。

### circuit_dag_op_nodes_len(ptr)

返回按源顺序计的操作节点数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：操作节点数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_op_nodes(ptr, buffer, len)

按源顺序把操作节点编号拷贝进 `buffer`（两步式：先 `circuit_dag_op_nodes_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或节点非空时 `buffer` 为 NULL；`-8` `len` 不足。

### circuit_dag_topological_op_nodes_len(ptr)

返回按确定性拓扑顺序计的操作节点数量。图含环时同样返回 0，因此在图健康状况未知时应与 `circuit_dag_validate` 配合使用。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：操作节点数量；`ptr` 为 NULL 或图含环时返回 0。

### circuit_dag_topological_op_nodes(ptr, buffer, len)

按确定性拓扑顺序把操作节点编号拷贝进 `buffer`（两步式：先 `circuit_dag_topological_op_nodes_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或节点非空时 `buffer` 为 NULL；`-3` 图含环；`-8` `len` 不足。

### circuit_dag_predecessors_len(ptr, node)

返回 `node` 的原始直接前驱条目数——包含哨兵，每条连接边计一条（见本节开头的说明）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：条目数量；`ptr` 为 NULL 或 `node` 未知时返回 0。

### circuit_dag_predecessors(ptr, node, buffer, len)

把 `node` 的原始直接前驱拷贝进 `buffer`（两步式：先 `circuit_dag_predecessors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或条目非空时 `buffer` 为 NULL；`-2` 未知节点编号；`-8` `len` 不足。

### circuit_dag_successors_len(ptr, node)

返回 `node` 的原始直接后继条目数——包含哨兵，每条连接边计一条。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：条目数量；`ptr` 为 NULL 或 `node` 未知时返回 0。

### circuit_dag_successors(ptr, node, buffer, len)

把 `node` 的原始直接后继拷贝进 `buffer`（两步式：先 `circuit_dag_successors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或条目非空时 `buffer` 为 NULL；`-2` 未知节点编号；`-8` `len` 不足。

### circuit_dag_predecessors_on_wire_len(ptr, node, tag, id)

返回 `node` 通过线 `(tag, id)` 相连的操作前驱数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT`（`id` 为量子比特编号）或 `DAG_WIRE_GLOBAL_ORDER`（`id` 被忽略）；经典标签携带线路作用域的句柄，C 侧无法重建，将以 `-8` 拒绝。下文所有 `(tag, id)` 线输入同样如此。
- `id` (`uint32_t`)：线的载荷编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知、标签未知或线 / 节点组合非法时返回 0。

### circuit_dag_predecessors_on_wire(ptr, node, tag, id, buffer, len)

把 `node` 通过线 `(tag, id)` 相连的操作前驱拷贝进 `buffer`（两步式：先 `circuit_dag_predecessors_on_wire_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或邻居非空时 `buffer` 为 NULL；`-2` 未知节点编号；`-3` 线 / 节点组合非法（例如该线不属于本 DAG）；`-8` 未知线标签或缓冲区不足。

### circuit_dag_successors_on_wire_len(ptr, node, tag, id)

返回 `node` 通过线 `(tag, id)` 相连的操作后继数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知、标签未知或线 / 节点组合非法时返回 0。

### circuit_dag_successors_on_wire(ptr, node, tag, id, buffer, len)

把 `node` 通过线 `(tag, id)` 相连的操作后继拷贝进 `buffer`（两步式：先 `circuit_dag_successors_on_wire_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或邻居非空时 `buffer` 为 NULL；`-2` 未知节点编号；`-3` 线 / 节点组合非法；`-8` 未知线标签或缓冲区不足。

### circuit_dag_quantum_predecessors_len(ptr, node)

返回 `node` 通过任意量子比特线相连的操作前驱数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知或查询失败时返回 0。

### circuit_dag_quantum_predecessors(ptr, node, buffer, len)

把 `node` 通过量子比特线相连的操作前驱拷贝进 `buffer`（两步式：先 `circuit_dag_quantum_predecessors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 未知节点编号；`-3` 查询失败；`-8` `len` 不足。

### circuit_dag_quantum_successors_len(ptr, node)

返回 `node` 通过任意量子比特线相连的操作后继数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知或查询失败时返回 0。

### circuit_dag_quantum_successors(ptr, node, buffer, len)

把 `node` 通过量子比特线相连的操作后继拷贝进 `buffer`（两步式：先 `circuit_dag_quantum_successors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 未知节点编号；`-3` 查询失败；`-8` `len` 不足。

### circuit_dag_classical_predecessors_len(ptr, node)

返回 `node` 通过任意经典线（经典变量线与经典值线）相连的操作前驱数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知或查询失败时返回 0。

### circuit_dag_classical_predecessors(ptr, node, buffer, len)

把 `node` 通过经典线相连的操作前驱拷贝进 `buffer`（两步式：先 `circuit_dag_classical_predecessors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 未知节点编号；`-3` 查询失败；`-8` `len` 不足。

### circuit_dag_classical_successors_len(ptr, node)

返回 `node` 通过任意经典线相连的操作后继数量；结果已去重并按确定性拓扑键排序。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。

返回：邻居数量；`ptr` 为 NULL、`node` 未知或查询失败时返回 0。

### circuit_dag_classical_successors(ptr, node, buffer, len)

把 `node` 通过经典线相连的操作后继拷贝进 `buffer`（两步式：先 `circuit_dag_classical_successors_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 未知节点编号；`-3` 查询失败；`-8` `len` 不足。

---

## 分层与深度

分层遵循 ASAP（as soon as possible，尽早调度）语义：每条操作落在其操作前驱所在最晚层的下一层，因此第 0 层即前层（front layer），且每条操作恰好出现在一个层中。层内节点保持确定性拓扑顺序。

`circuit_dag_layers` 与下文的连续段收集接口采用 CSR 风格的分组输出：`nodes` 接收所有分组的节点编号首尾相接的平铺结果，`offsets` 接收 `groups_len + 1` 个边界值且 `offsets[0] == 0`；第 `k` 组占据 `nodes[offsets[k]..offsets[k+1]]`。`nodes_cap` 应足以容纳所有节点，`offsets_cap` 应足以容纳分组数加一。

### circuit_dag_front_layer_len(ptr)

返回没有操作前驱的操作节点数量——即立即可执行的集合，按确定性拓扑顺序排列。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：前层大小；`ptr` 为 NULL 或图含环时返回 0。

### circuit_dag_front_layer(ptr, buffer, len)

把前层操作节点编号拷贝进 `buffer`（两步式：先 `circuit_dag_front_layer_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或节点非空时 `buffer` 为 NULL；`-3` 图含环；`-8` `len` 不足。

### circuit_dag_layers_len(ptr)

返回 ASAP 层数，即依赖深度。空 DAG 返回 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：层数量；`ptr` 为 NULL 或图含环时返回 0。

### circuit_dag_layers(ptr, nodes, nodes_cap, offsets, offsets_cap)

以 CSR 风格拷贝 ASAP 分层：各层节点按层序平铺进 `nodes`，边界写入 `offsets`（见本节开头的说明）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `nodes` (`uint32_t*`)：平铺的层节点；每层内按确定性拓扑顺序排列。
- `nodes_cap` (`uintptr_t`)：`nodes` 容量；必须足以容纳所有操作节点。
- `offsets` (`uintptr_t*`)：层边界，`offsets[0] == 0`。
- `offsets_cap` (`uintptr_t`)：`offsets` 容量；必须足以容纳层数加一。

返回：`0` 成功；`-1` `ptr` 为 NULL，或数据非空时任一缓冲区为 NULL；`-3` 图含环；`-8` 任一容量不足。

### circuit_dag_node_layers_len(ptr)

返回按节点计的层条目数量，即操作节点数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：条目数量；`ptr` 为 NULL 或图含环时返回 0。

### circuit_dag_node_layers(ptr, nodes, layers, len)

把每条操作节点的 ASAP 层号以 `(node, layer)` 对的形式拷贝进并行数组 `nodes` 与 `layers`，按确定性拓扑顺序排列（两步式：先 `circuit_dag_node_layers_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `nodes` (`uint32_t*`)：节点编号。
- `layers` (`uintptr_t*`)：同一位置节点的层号。
- `len` (`uintptr_t`)：两个数组的容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或条目非空时任一数组为 NULL；`-3` 图含环；`-8` `len` 不足。

### circuit_dag_depth(ptr)

返回 ASAP 依赖深度，即层数。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：以有符号值表示的深度（空 DAG 为 `0`）；`-1` `ptr` 为 NULL；`-3` 图含环。

---

## 线时间线

每条线都是一条时间线：`circuit_dag_wire_in` / `circuit_dag_wire_out` 报告其端点，`circuit_dag_nodes_on_wire` 按线穿引操作的顺序遍历。本节所有 `(tag, id)` 输入均接受 `DAG_WIRE_QUBIT`（`id` 为量子比特编号）与 `DAG_WIRE_GLOBAL_ORDER`（`id` 被忽略）；经典标签将以 `-8` 拒绝，因为经典线携带线路作用域的句柄，C 侧无法重建——需要基于节点的经典连通性时，请使用 [图遍历](#图遍历)中的 `circuit_dag_classical_predecessors` / `circuit_dag_classical_successors`。

### circuit_dag_is_wire_idle(ptr, tag, id)

判断该线上是否没有操作节点。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。

返回：`1` 空闲；`0` 在用；`-1` `ptr` 为 NULL；`-3` 该线不属于本 DAG；`-8` 其他 `tag` 取值。

### circuit_dag_nodes_on_wire_len(ptr, tag, id)

按线上顺序返回线上操作节点的数量；`ptr` 为 NULL、标签未知或该线不属于本 DAG 时返回 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。

返回：节点数量；上述非法状态返回 0。

### circuit_dag_nodes_on_wire(ptr, tag, id, buffer, len)

按线上顺序把线上的操作节点拷贝进 `buffer`（两步式：先 `circuit_dag_nodes_on_wire_len`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或节点非空时 `buffer` 为 NULL；`-3` 该线不属于本 DAG；`-8` 未知线标签或缓冲区不足。

### circuit_dag_wire_in(ptr, tag, id)

返回该线的输入哨兵节点编号。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。

返回：哨兵节点编号；`-1` `ptr` 为 NULL；`-3` 该线不属于本 DAG；`-8` 其他 `tag` 取值。

### circuit_dag_wire_out(ptr, tag, id)

返回该线的输出哨兵节点编号。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。

返回：哨兵节点编号；`-1` `ptr` 为 NULL；`-3` 该线不属于本 DAG；`-8` 其他 `tag` 取值。

---

## 控制流与测量标志

### circuit_dag_has_control_flow(ptr)

判断顶层操作中是否存在结构化控制流（`if` / `while` / `for` / `switch` / `break` / `continue`）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：`1` 为真；`0` 否；`-1` `ptr` 为 NULL。

### circuit_dag_has_nested_control_flow(ptr)

判断是否有控制流体内部还包含结构化控制流。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：`1` 为真；`0` 否；`-1` `ptr` 为 NULL。

### circuit_dag_has_measurement(ptr)

判断是否有顶层操作测量量子比特，包括直接测量和控制流体内的递归测量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：`1` 为真；`0` 否；`-1` `ptr` 为 NULL。

---

## 操作计数

### circuit_dag_operation_count_by_name_len(ptr)

返回顶层操作中不同指令名的数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：不同名称数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_operation_count_by_name(ptr, names, counts, len)

把顶层操作按名称的计数拷贝进并行数组 `names` 与 `counts`，按首次出现（插入）顺序排列（两步式：先 `circuit_dag_operation_count_by_name_len`）。每个写出的名称是新分配的 C 字符串，需分别用 `cqlib_string_free` 释放。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `names` (`char**`)：指令名输出数组。
- `counts` (`uintptr_t*`)：各名称计数的输出数组。
- `len` (`uintptr_t`)：两个数组的容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或条目非空时任一输出数组为 NULL；`-8` `len` 不足。

### circuit_dag_operation_count_by_name_recursive_len(ptr)

返回顶层操作以及（递归地）控制流体内部操作中不同指令名的数量。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：不同名称数量；`ptr` 为 NULL 时返回 0。

### circuit_dag_operation_count_by_name_recursive(ptr, names, counts, len)

把顶层与嵌套操作按名称的计数拷贝进并行数组 `names` 与 `counts`，按首次出现（插入）顺序排列（两步式：先 `circuit_dag_operation_count_by_name_recursive_len`）。每个写出的名称是新分配的 C 字符串，需分别用 `cqlib_string_free` 释放。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `names` (`char**`)：指令名输出数组。
- `counts` (`uintptr_t*`)：各名称计数的输出数组。
- `len` (`uintptr_t`)：两个数组的容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或条目非空时任一输出数组为 NULL；`-8` `len` 不足。

---

## 连续段收集

一个连续段（run）是沿一条线的时间线上连续的操作节点极大区段，其中每个节点都满足判据；连续段沿该线按时间线顺序报告。判据如下：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `DAG_RUN_CRITERION_1Q_GATE` | 0 | 单量子比特门：操作恰好作用于 1 个量子比特，且其指令为 `Standard`、`McGate`、`UnitaryGate` 或 `CircuitGate` |
| `DAG_RUN_CRITERION_2Q_GATE` | 1 | 双量子比特门：在相同指令限制下恰好作用于 2 个量子比特 |

拷贝变体采用 [分层与深度](#分层与深度)所述的 CSR 风格分组输出：连续段节点首尾相接平铺进 `nodes`，边界写入 `offsets` 且 `offsets[0] == 0`。

### circuit_dag_collect_1q_runs_len(ptr)

返回所有量子比特线上连续单量子比特门段的数量；`ptr` 为 NULL 或收集失败时返回 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：连续段数量；上述非法状态返回 0。

### circuit_dag_collect_1q_runs(ptr, nodes, nodes_cap, offsets, offsets_cap)

以 CSR 风格拷贝单量子比特门连续段（见本节开头的说明）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `nodes` (`uint32_t*`)：平铺的连续段节点。
- `nodes_cap` (`uintptr_t`)：`nodes` 容量；必须足以容纳所有连续段节点。
- `offsets` (`uintptr_t*`)：连续段边界，`offsets[0] == 0`。
- `offsets_cap` (`uintptr_t`)：`offsets` 容量；必须足以容纳连续段数量加一。

返回：`0` 成功；`-1` `ptr` 为 NULL，或数据非空时任一缓冲区为 NULL；`-3` 时间线断裂；`-8` 任一容量不足。

### circuit_dag_collect_2q_runs_len(ptr)

返回所有量子比特线上连续双量子比特门段的数量；`ptr` 为 NULL 或收集失败时返回 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。

返回：连续段数量；上述非法状态返回 0。

### circuit_dag_collect_2q_runs(ptr, nodes, nodes_cap, offsets, offsets_cap)

以 CSR 风格拷贝双量子比特门连续段（见本节开头的说明）。双量子比特门触及两条量子比特线，因此同一节点可能同时出现在其两条线各自的连续段中。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `nodes` (`uint32_t*`)：平铺的连续段节点。
- `nodes_cap` (`uintptr_t`)：`nodes` 容量；必须足以容纳所有连续段节点。
- `offsets` (`uintptr_t*`)：连续段边界，`offsets[0] == 0`。
- `offsets_cap` (`uintptr_t`)：`offsets` 容量；必须足以容纳连续段数量加一。

返回：`0` 成功；`-1` `ptr` 为 NULL，或数据非空时任一缓冲区为 NULL；`-3` 时间线断裂；`-8` 任一容量不足。

### circuit_dag_collect_runs_on_wire_len(ptr, tag, id, criterion)

返回一条线上匹配 `criterion` 的连续门段数量；`ptr` 为 NULL、标签或判据未知或收集失败时返回 0。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。
- `criterion` (`uint32_t`)：`DAG_RUN_CRITERION_1Q_GATE` 或 `DAG_RUN_CRITERION_2Q_GATE`。

返回：连续段数量；上述非法状态返回 0。

### circuit_dag_collect_runs_on_wire(ptr, tag, id, criterion, nodes, nodes_cap, offsets, offsets_cap)

以 CSR 风格拷贝一条线上匹配 `criterion` 的连续门段（见本节开头的说明）。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `tag` (`uint32_t`)：`DAG_WIRE_QUBIT` 或 `DAG_WIRE_GLOBAL_ORDER`。
- `id` (`uint32_t`)：线的载荷编号。
- `criterion` (`uint32_t`)：`DAG_RUN_CRITERION_1Q_GATE` 或 `DAG_RUN_CRITERION_2Q_GATE`。
- `nodes` (`uint32_t*`)：平铺的连续段节点。
- `nodes_cap` (`uintptr_t`)：`nodes` 容量；必须足以容纳所有连续段节点。
- `offsets` (`uintptr_t*`)：连续段边界，`offsets[0] == 0`。
- `offsets_cap` (`uintptr_t`)：`offsets` 容量；必须足以容纳连续段数量加一。

返回：`0` 成功；`-1` `ptr` 为 NULL，或数据非空时任一缓冲区为 NULL；`-3` 线非法或时间线断裂；`-8` 未知线标签或判据，或容量不足。

---

## 修改 DAG

以下函数就地编辑 DAG。每次变更都会重建底层图，因此变更前捕获的节点编号可能不再指向原来的节点——每次成功变更后应重新查询（例如用 `circuit_dag_op_nodes`）。传入的操作会被拷贝（构造层操作先降级），因此调用者保留其句柄的所有权。

### circuit_dag_apply_operation_back(ptr, operation, out_node)

把 `operation` 的拷贝追加到 DAG 末端。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `operation` (`const struct COperation*`)：待追加的操作。
- `out_node` (`uint32_t*`)：新节点编号的可选输出；允许传 NULL。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-3` 失败（例如操作引用了 DAG 不认识的资源）。

### circuit_dag_apply_value_operation_back(ptr, operation, out_node)

把自包含的构造层操作 `operation` 降级（见 [操作与指令](4_operation_instruction.md)）并追加到 DAG 末端。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `operation` (`const struct CValueOperation*`)：待追加的构造层操作。
- `out_node` (`uint32_t*`)：新节点编号的可选输出；允许传 NULL。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-3` 降级失败或 DAG 失败。

### circuit_dag_apply_operation_front(ptr, operation, out_node)

把 `operation` 的拷贝前置到 DAG 首端。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `operation` (`const struct COperation*`)：待前置的操作。
- `out_node` (`uint32_t*`)：新节点编号的可选输出；允许传 NULL。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-3` 失败。

### circuit_dag_apply_value_operation_front(ptr, operation, out_node)

把自包含的构造层操作 `operation` 降级并前置到 DAG 首端。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `operation` (`const struct CValueOperation*`)：待前置的构造层操作。
- `out_node` (`uint32_t*`)：新节点编号的可选输出；允许传 NULL。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-3` 降级失败或 DAG 失败。

### circuit_dag_remove_op_node(ptr, node)

删除操作节点 `node`；各线的时间线将重新穿引剩余操作。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：待删除的节点编号。

返回：被删除的操作，为新分配的 `COperation*`（用 `operation_free` 释放）；`ptr` 为 NULL、节点不存在或不是操作节点时返回 NULL。

### circuit_dag_substitute_node(ptr, node, operation)

用 `operation` 的拷贝替换 `node` 处存储的操作。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：待替换的节点编号。
- `operation` (`const struct COperation*`)：替换用操作。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-2` `node` 不是操作节点；`-3` 新操作不适配本 DAG（未知量子比特、未注册参数、非法经典引用等）。

### circuit_dag_substitute_value_node(ptr, node, operation)

把自包含的构造层操作 `operation` 降级，并用它替换 `node` 处存储的操作。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：待替换的节点编号。
- `operation` (`const struct CValueOperation*`)：替换用构造层操作。

返回：`0` 成功；`-1` `ptr` 或 `operation` 为 NULL；`-2` `node` 不是操作节点；`-3` 降级失败或 DAG 失败。

### circuit_dag_substitute_node_with_dag(ptr, node, replacement)

用 `replacement` DAG 的拷贝替换操作节点 `node`。以下两条规则保证调度合法，违反时以 `-3` 失败：

- 资源足迹：`replacement` 中的每条操作只能读取被替换节点读取或写入的线，且只能写入被替换节点写入的线；全局序资源不受此限。
- 控制流形态：控制流操作只能被恰好包含一条同类型控制流操作的替换 DAG 替换；普通操作可在资源足迹规则内被含多条操作的 DAG 替换。

替换 DAG 自身也必须通过 DAG 校验（含环或非法的替换以 `-3` 失败）。

- `ptr` (`struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：待替换的节点编号。
- `replacement` (`const struct CCircuitDag*`)：替换用 DAG。

返回：`0` 成功；`-1` `ptr` 或 `replacement` 为 NULL；`-2` `node` 不是操作节点；`-3` 校验失败。

---

## 控制流检查

`CDagControlFlow*` 是操作节点所附（递归）控制流结构的只读快照，由 `circuit_dag_control_flow` 产生，用 `dag_control_flow_free` 释放。结构的种类：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `DAG_CONTROL_FLOW_IF` | 0 | `if` / `else`：体依次为 `then` 体、（存在时的）`else` 体 |
| `DAG_CONTROL_FLOW_WHILE` | 1 | `while`：单个体 |
| `DAG_CONTROL_FLOW_FOR` | 2 | `for`：单个体 |
| `DAG_CONTROL_FLOW_SWITCH` | 3 | `switch`：每个 case 一个体，按 case 顺序；可选的 default 体单独报告 |
| `DAG_CONTROL_FLOW_BREAK` | 4 | `break`：无体 |
| `DAG_CONTROL_FLOW_CONTINUE` | 5 | `continue`：无体 |

此处返回的体 DAG 是独立的深拷贝（`CCircuitDag*`，用 `circuit_dag_free` 释放）；对拷贝的修改不会传播回父 DAG。

### circuit_dag_control_flow(ptr, node, out)

把 `node` 所附的控制流结构快照到 `out`。

- `ptr` (`const struct CCircuitDag*`)：DAG 句柄。
- `node` (`uint32_t`)：节点编号。
- `out` (`struct CDagControlFlow**`)：快照句柄的输出。

返回：`1` 成功（`*out` 持有新分配的 `CDagControlFlow*`）；`0` 该节点是普通操作（`*out` 被置为 NULL）；`-1` `ptr` 或 `out` 为 NULL；`-2` 未知节点编号或非操作节点。

### dag_control_flow_kind(ptr)

返回快照的种类标签。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。

返回：`DAG_CONTROL_FLOW_*` 标签；`-1` `ptr` 为 NULL。

### dag_control_flow_len(ptr)

返回体 DAG 的数量：`if` 携带 `then` 体加可选的 `else` 体，`while` 与 `for` 各一个体，`switch` 每个 case 一个体，`break` 与 `continue` 无体。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。

返回：体数量；`ptr` 为 NULL 时返回 0。

### dag_control_flow_body(ptr, index)

按上表所列顺序返回第 `index` 个体的一份拷贝。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。
- `index` (`uintptr_t`)：体索引。

返回：新分配的 `CCircuitDag*`（用 `circuit_dag_free` 释放）；`ptr` 为 NULL 或 `index` 越界时返回 NULL。

### dag_control_flow_has_default(ptr)

判断 `switch` 快照是否携带 default 体。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。

返回：`1` 存在；`0` 不存在或结构不是 `switch`；`-1` `ptr` 为 NULL。

### dag_control_flow_default_body(ptr)

返回 `switch` default 体的一份拷贝。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。

返回：新分配的 `CCircuitDag*`（用 `circuit_dag_free` 释放）；`ptr` 为 NULL、结构不是 `switch` 或没有 default 体时返回 NULL。

### dag_control_flow_body_value(ptr, index, lo, hi)

把附着在 `switch` 第 `index` 个体上的 128 位 case 值写入 `lo` 与 `hi`。

- `ptr` (`const struct CDagControlFlow*`)：快照句柄。
- `index` (`uintptr_t`)：case 索引，与 `dag_control_flow_body` 对应。
- `lo` (`uint64_t*`)：case 值低 64 位的输出。
- `hi` (`uint64_t*`)：case 值高 64 位的输出。

返回：`0` 成功；`-1` `ptr`、`lo` 或 `hi` 为 NULL；`-8` 结构不是 `switch` 或 `index` 越界。

### dag_control_flow_free(ptr)

释放控制流快照句柄。

- `ptr` (`struct CDagControlFlow*`)：快照句柄，允许传 NULL。

---

## 示例：线路 → DAG → 统计 → 回线路

```c
#include <stdint.h>
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    circuit_h(circuit, 0);
    circuit_cx(circuit, 0, 1);

    /* 线路 → DAG */
    struct CCircuitDag* dag = circuit_dag_from_circuit(circuit);

    /* 校验与统计 */
    int32_t ok = circuit_dag_validate(dag);                  /* 0 */
    uintptr_t num_ops = circuit_dag_num_ops(dag);            /* 2 */
    int32_t empty = circuit_dag_is_empty(dag);               /* 0 */

    uint32_t ids[2] = {0, 0};
    circuit_dag_qubits(dag, ids, 2);                         /* ids = {0, 1} */

    /* 线清单：全局序线在前，随后两条量子比特线 */
    uintptr_t wires = circuit_dag_wires_len(dag);            /* 3 */
    uint32_t tags[3] = {0, 0, 0};
    uint32_t wire_ids[3] = {0, 0, 0};
    circuit_dag_wires(dag, tags, wire_ids, 3);
    /* tags = {DAG_WIRE_GLOBAL_ORDER, DAG_WIRE_QUBIT, DAG_WIRE_QUBIT}
       wire_ids = {0, 0, 1} */

    int32_t has_q0 = circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 0);   /* 1 */
    int32_t has_q5 = circuit_dag_has_wire(dag, DAG_WIRE_QUBIT, 5);   /* 0 */

    /* DAG → 线路：重建结果与源线路形状一致
     * （2 量子比特、2 条操作） */
    struct CCircuit* lowered = circuit_dag_to_circuit(dag);

    printf("validate=%d ops=%llu empty=%d wires=%llu lowered_ops=%llu\n",
           (int)ok, (unsigned long long)num_ops, (int)empty,
           (unsigned long long)wires,
           (unsigned long long)circuit_num_operations(lowered));

    circuit_free(lowered);
    circuit_dag_free(dag);
    circuit_free(circuit);
    return 0;
}
```

线路构造接口见 [线路](1_circuit.md)，操作句柄构造见 [操作与指令](4_operation_instruction.md)。
