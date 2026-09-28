# 线路控制流图（CCircuitCFG）

`CCircuitCFG*` 是量子线路的控制流图（Control Flow Graph, CFG）分析视图：基本块保存一段顺序执行的操作，块尾终结符描述控制转移，控制流区域记录结构化 `if` / `while` / `for` / `switch` 的边界与入口。块以稳定节点索引（`uint32_t`）寻址，`UINT32_MAX` 为"缺失块 / 未设置入口"哨兵值；本页接口除通用错误码外，另以 `-2` 表示块不存在或索引为哨兵值。典型工作流：`circuit_cfg_from_circuit` → 检查块 / 终结符 / 边 / 区域 → `circuit_cfg_validate` → `circuit_cfg_to_circuit`。错误码、句柄释放与两步式数组输出等通用约定见 [Overview](../0_overview.md)。

---

## 构造与转换

### circuit_cfg_new(num_qubits)

创建一个空 CFG，包含密集编号的量子比特 `0..num_qubits`。新图没有任何块，也未设置入口块，编辑之后才能通过 `circuit_cfg_validate`。

- `num_qubits` (`uintptr_t`)：逻辑量子比特数量。

返回：新分配的 `CCircuitCFG*`。

### circuit_cfg_from_qubits(qubits, qubits_len)

按给定逻辑量子比特编号集合创建空 CFG：编号按插入顺序保留，重复编号会被去除，适合稀疏逻辑编号。

- `qubits` (`const uint32_t*`)：逻辑量子比特编号数组。
- `qubits_len` (`uintptr_t`)：数组长度。

返回：新分配的 `CCircuitCFG*`（用 `circuit_cfg_free` 释放）；`qubits_len > 0` 而 `qubits` 为 NULL 时返回 NULL。

### circuit_cfg_from_circuit(ptr)

把结构化线路展开为通过校验的 CFG。每个条件、循环与 `switch` 头块带有一个控制流区域，记录体入口、汇合块与外层操作元数据；线性线路展开为单个入口块，标签为 `entry`，终结符为 `CFG_TERMINATOR_RETURN`。

- `ptr` (`const struct CCircuit*`)：源线路句柄。

返回：新分配的 `CCircuitCFG*`（用 `circuit_cfg_free` 释放）；`ptr` 为 NULL 或展开失败（存在无法结构化的控制流）时返回 NULL。

```c
struct CCircuit* circuit = circuit_new(2);
circuit_h(circuit, 0);
circuit_cx(circuit, 0, 1);

struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
/* circuit_cfg_num_blocks(cfg) == 1：线性线路只有一个入口块 */
```

### circuit_cfg_free(ptr)

释放 CFG 句柄。

- `ptr` (`struct CCircuitCFG*`)：待释放句柄，允许传 NULL。

### circuit_cfg_to_circuit(ptr)

把 CFG 还原为结构化线路。符号表、参数表与全局相位随还原一并写回；内部先执行一次与 `circuit_cfg_validate` 相同的结构检查。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：新分配的 `CCircuit*`（用 `circuit_free` 释放）；`ptr` 为 NULL 或还原失败（图结构被破坏或控制流区域无法映射回结构化形式）时返回 NULL。

### circuit_cfg_validate(ptr)

检查图、操作、参数、边、终结符与区域元数据的不变量：入口块是否有效、每个块是否恰有一个终结符且出边数量与类型一致、`Return` 是否无出边、全部块是否从入口可达、区域元数据是否与图结构一致、`Branch` 条件是否为 `Bool` 表达式、`Switch` 目标是否为 `UInt` 表达式且出边数等于 case 数加一等。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-3` 校验失败。

---

## 全局信息

### circuit_cfg_num_qubits(ptr)

返回逻辑量子比特数量。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：量子比特数量；`ptr` 为 NULL 时返回 0。

### circuit_cfg_num_blocks(ptr)

返回图中块的数量。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：块数量；`ptr` 为 NULL 时返回 0。

### circuit_cfg_qubits(ptr, buffer, len)

按插入顺序把逻辑量子比特编号拷贝进 `buffer`（两步式：先 `circuit_cfg_num_qubits` 取长度）。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或编号集合非空时 `buffer` 为 NULL；`-8` `len` 小于量子比特数量。

### circuit_cfg_classical_vars_len(ptr)

返回可变经典变量静态类型表的长度。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：变量数量；`ptr` 为 NULL 时返回 0。

### circuit_cfg_classical_vars(ptr, buffer, len)

把可变经典变量类型按 `(tag, width)` 快照拷贝进 `buffer`（两步式：先 `circuit_cfg_classical_vars_len`）。`tag` 取 `CQLIB_CLASSICAL_TYPE_BIT` / `CQLIB_CLASSICAL_TYPE_BOOL` / `CQLIB_CLASSICAL_TYPE_UINT` / `CQLIB_CLASSICAL_TYPE_BIT_VEC` 之一；手工创建的 CFG 该表为空，`circuit_cfg_from_circuit` 创建的 CFG 保存源线路的静态快照。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `buffer` (`struct CClassicalType*`)：输出数组，元素为 `{ uint32_t tag; uint32_t width; }`。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或表非空时 `buffer` 为 NULL；`-8` `len` 不足。

### circuit_cfg_classical_values_len(ptr)

返回不可变经典值静态类型表的长度。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：经典值数量；`ptr` 为 NULL 时返回 0。

### circuit_cfg_classical_values(ptr, buffer, len)

把不可变经典值类型（如测量产生的 `BitVec`）按 `(tag, width)` 快照拷贝进 `buffer`，约定与 `circuit_cfg_classical_vars` 相同。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `buffer` (`struct CClassicalType*`)：输出数组。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或表非空时 `buffer` 为 NULL；`-8` `len` 不足。

### circuit_cfg_entry_block(ptr)

返回当前指定的入口块索引。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。

返回：入口块索引；尚未设置入口块或 `ptr` 为 NULL 时返回 `UINT32_MAX`。

---

## 块与操作

### circuit_cfg_add_block(ptr)

向图末尾添加一个空块。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。

返回：新块的稳定节点索引；`ptr` 为 NULL 时返回 `UINT32_MAX`。

### circuit_cfg_push_operation(ptr, node, op)

向块末尾追加一条普通（非控制流）操作；操作被克隆，源句柄仍归调用者所有。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `op` (`const struct COperation*`)：操作句柄（用 `operation_new` 创建，见 [操作与指令](4_operation_instruction.md)）。

返回：`0` 成功；`-1` `ptr` 或 `op` 为 NULL；`-2` 块不存在。

### circuit_cfg_extend_operations(ptr, node, ops, len)

向块 `node` 末尾批量追加克隆的操作。`ops` 中的每个句柄都会在修改前先完成校验与克隆，因此只要任一元素为 NULL，块就保持原样。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `ops` (`const struct COperation* const*`)：操作句柄数组。
- `len` (`uintptr_t`)：操作数量（可为 0）。

返回：`0` 成功；`-1` NULL 输入；`-2` `node` 不指向任何块。

### circuit_cfg_blocks(ptr, buffer, len)

按图顺序把全部块的稳定节点索引拷贝进 `buffer`（两步式：先 `circuit_cfg_num_blocks`）。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `buffer` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或图非空时 `buffer` 为 NULL；`-8` `len` 小于块数量。

### circuit_cfg_block_label(ptr, node)

读取块的诊断标签。`circuit_cfg_from_circuit` 展开的块带有结构化标签（如 `entry`、`while_body_…`）。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：新分配的 C 字符串（用 `cqlib_string_free` 释放）；块无标签、不存在或 `ptr` 为 NULL 时返回 NULL。

### circuit_cfg_with_label(ptr, node, label)

设置或替换块 `node` 的标签。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `label` (`const char*`)：新标签字符串。

返回：`0` 成功；`-1` NULL 输入；`-2` `node` 不指向任何块；`-4` `label` 不是合法 UTF-8。

### circuit_cfg_block_is_empty(ptr, node)

判断块是否既无操作也无终结符。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`1` 为空；`0` 非空；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_block_operations_len(ptr, node)

返回块内普通操作数量（终结符不计入）。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：操作数量；`ptr` 为 NULL 或块不存在时返回 0。

### circuit_cfg_block_operations(ptr, node, out, len)

把块内普通操作克隆进 `out`（两步式：先 `circuit_cfg_block_operations_len`）。每个写出元素是新分配的 `COperation*`，需分别用 `operation_free` 释放。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `out` (`struct COperation**`)：输出数组。
- `len` (`uintptr_t`)：数组容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或操作非空时 `out` 为 NULL；`-2` 块不存在；`-8` `len` 不足。

---

## 终结符

合法 CFG 中，每个基本块必须恰有一个终结符，且其出边数量与类型和终结符一致；`Return` 不允许出边。`Branch` / `ForLoop` / `Switch` 终结符由 `circuit_cfg_from_circuit` 展开 `circuit_if_else` / `circuit_for_uint` / `circuit_switch` 等结构化线路生成，见 [经典数据与控制流](9_classical_control_flow.md)；`Jump` / `Return` / `Break` / `Continue` 可用下列四个设置接口替换。

### circuit_cfg_block_has_terminator(ptr, node)

判断块是否已设置终结符。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`1` 已设置；`0` 未设置；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_block_terminator_tag(ptr, node)

返回块终结符标签。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`CFG_TERMINATOR_*` 常量之一；未设置终结符时返回 `0`；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_terminator_target(ptr, node)

返回 `Jump` / `Break` / `Continue` 终结符的跳转目标块。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：目标块索引；其他终结符、未设置终结符、块不存在或 `ptr` 为 NULL 时返回 `UINT32_MAX`。

### circuit_cfg_terminator_condition(ptr, node)

克隆 `Branch` / `Switch` 终结符的条件表达式。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：新分配的 `CClassicalExpr*`（用 `classical_expr_free` 释放）；`ptr` 为 NULL、块不存在、未设置终结符或终结符不带条件时返回 NULL。

### circuit_cfg_terminator_for_info(ptr, node, var, start, stop, step)

写出 `ForLoop` 终结符的循环变量与 start / stop / step 三个表达式（半开区间 `[start, stop)`）。四个句柄均为克隆：`var` 用 `classical_var_free` 释放，三个表达式用 `classical_expr_free` 释放。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `var` (`struct CClassicalVar**`)：循环变量输出。
- `start` (`struct CClassicalExpr**`)：起始表达式输出。
- `stop` (`struct CClassicalExpr**`)：结束表达式输出。
- `step` (`struct CClassicalExpr**`)：步长表达式输出。

返回：`0` 成功；`-1` `ptr` 为 NULL 或任一输出指针为 NULL；`-2` 块不存在；`-8` 终结符不是 `ForLoop`。

### circuit_cfg_set_terminator_jump(ptr, node, target)

把块终结符替换为指向 `target` 的无条件跳转。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `target` (`uint32_t`)：目标块索引。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_set_terminator_return(ptr, node)

把块终结符替换为 `Return`（执行结束）。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_set_terminator_break(ptr, node, target)

把块终结符替换为结构化 `Break`，指向所跳出循环或 `switch` 结构的头块。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `target` (`uint32_t`)：头块索引。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_set_terminator_continue(ptr, node, target)

把块终结符替换为结构化 `Continue`，指向循环头块。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `target` (`uint32_t`)：循环头块索引。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 块不存在。

---

## 边与遍历

### circuit_cfg_add_edge(ptr, source, target, flow, case_lo, case_hi)

在两个已存在的块之间添加一条有向语义边。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `source` (`uint32_t`)：起点块索引。
- `target` (`uint32_t`)：终点块索引。
- `flow` (`uint32_t`)：`CFG_FLOW_*` 标签之一。
- `case_lo` (`uint64_t`)：`CFG_FLOW_CASE` 的 128 位 case 值低 64 位（最低有效），其余标签传 0。
- `case_hi` (`uint64_t`)：case 值高 64 位。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 任一端点不存在（含哨兵值）；`-8` 未知 `flow` 标签。

### circuit_cfg_set_entry_block(ptr, node)

设置图入口块；索引是否指向真实块由 `circuit_cfg_validate` 检查。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：入口块索引。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` `node` 为哨兵值 `UINT32_MAX`。

### circuit_cfg_outgoing_edges_len(ptr, node)

返回块的出边数量。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：出边数量；`ptr` 为 NULL 或块不存在时返回 0。

### circuit_cfg_outgoing_edges(ptr, node, out_targets, flow_tags, case_lo, case_hi, len)

把块出边拷贝进四个并行数组（两步式：先 `circuit_cfg_outgoing_edges_len`）。边按插入顺序写出，条件头块的真分支在前。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。
- `out_targets` (`uint32_t*`)：目标块索引数组。
- `flow_tags` (`uint32_t*`)：`CFG_FLOW_*` 标签数组。
- `case_lo` (`uint64_t*`)：`CFG_FLOW_CASE` 边的 128 位 case 值低 64 位，其余边写 0。
- `case_hi` (`uint64_t*`)：case 值高 64 位数组。
- `len` (`uintptr_t`)：数组容量。

返回：`0` 成功；`-1` `ptr` 为 NULL，或出边非空时任一数组为 NULL；`-2` 块不存在；`-8` `len` 不足。

---

## 区域与循环

控制流区域记录结构化控制流的边界与语义：仅靠图边无法完整恢复 `if` / `while` / `for` / `switch` 的结构化语义，区域元数据描述每个结构的分支入口、退出块与汇合块。修改图结构时应同步维护区域元数据——保持区域与图边一致是 `circuit_cfg_to_circuit` 成功还原结构化线路的关键。

### circuit_cfg_region_tag(ptr, node)

返回块拥有的控制流区域标签。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`CFG_REGION_*` 常量之一；块无区域时返回 `0`；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_is_loop_header(ptr, node)

判断块是否为循环头：拥有 `While` 或 `For` 区域返回 `1`；`Switch` 区域入口不算循环头。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：块索引。

返回：`1` 是；`0` 否；`-1` `ptr` 为 NULL；`-2` 块不存在。

### circuit_cfg_region_if(ptr, node, then_entry, else_entry, merge_block, has_else)

写出 `If` 区域布局。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `then_entry` (`uint32_t*`)：then 分支入口块。
- `else_entry` (`uint32_t*`)：else 分支入口块；无 else 体时为汇合块。
- `merge_block` (`uint32_t*`)：汇合块。
- `has_else` (`int32_t*`)：源线路是否存在 else 分支体（1/0）。

返回：`0` 成功；`-1` `ptr` 为 NULL 或任一输出指针为 NULL；`-2` 块不存在；`-8` 块没有 `If` 区域。

### circuit_cfg_region_loop(ptr, node, body_entry, exit_block)

写出 `While` / `For` 区域布局。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `body_entry` (`uint32_t*`)：循环体入口块。
- `exit_block` (`uint32_t*`)：退出块。

返回：`0` 成功；`-1` `ptr` 为 NULL 或输出指针为 NULL；`-2` 块不存在；`-8` 块没有循环区域。

### circuit_cfg_region_switch_cases_len(ptr, node)

返回 `Switch` 区域显式 case 的数量。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。

返回：case 数量；`ptr` 为 NULL、块不存在或区域不是 `Switch` 时返回 0。

### circuit_cfg_region_switch(ptr, node, case_lo, case_hi, case_entries, len, default_entry, merge_block, has_default)

写出 `Switch` 区域布局（两步式：先 `circuit_cfg_region_switch_cases_len`）。

- `ptr` (`const struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `case_lo` (`uint64_t*`)：case `i` 的 128 位匹配值低 64 位。
- `case_hi` (`uint64_t*`)：case `i` 的 128 位匹配值高 64 位。
- `case_entries` (`uint32_t*`)：case `i` 的分支入口块。
- `len` (`uintptr_t`)：case 数组容量。
- `default_entry` (`uint32_t*`)：默认分支入口块；无默认体时为汇合块。
- `merge_block` (`uint32_t*`)：汇合块。
- `has_default` (`int32_t*`)：源线路是否存在默认分支体（1/0）。

返回：`0` 成功；`-1` `ptr` 为 NULL，或必填输出指针为 NULL；`-2` 块不存在；`-8` 块没有 `Switch` 区域，或 `len` 小于 case 数量。

### circuit_cfg_set_if_region(ptr, node, then_entry, else_entry, merge_block, has_else, outer_op)

向头块挂接 `If` 区域。入口块索引是否指向真实块由 `circuit_cfg_validate` 检查。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `then_entry` (`uint32_t`)：then 分支入口块。
- `else_entry` (`uint32_t`)：else 分支入口块。
- `merge_block` (`uint32_t`)：汇合块。
- `has_else` (`int32_t`)：是否存在 else 分支体（1/0）。
- `outer_op` (`const struct COperation*`)：外层操作元数据（作用的量子比特、参数、标签），传 NULL 表示空元数据。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 头块不存在。

### circuit_cfg_set_while_region(ptr, node, body_entry, exit_block, outer_op)

向头块挂接 `While` 区域。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `body_entry` (`uint32_t`)：循环体入口块。
- `exit_block` (`uint32_t`)：退出块。
- `outer_op` (`const struct COperation*`)：外层操作元数据，可为 NULL。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 头块不存在。

### circuit_cfg_set_for_region(ptr, node, body_entry, exit_block, outer_op)

向头块挂接 `For` 区域，参数含义与 `circuit_cfg_set_while_region` 相同。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `body_entry` (`uint32_t`)：循环体入口块。
- `exit_block` (`uint32_t`)：退出块。
- `outer_op` (`const struct COperation*`)：外层操作元数据，可为 NULL。

返回：`0` 成功；`-1` `ptr` 为 NULL；`-2` 头块不存在。

### circuit_cfg_set_switch_region(ptr, node, case_lo, case_hi, case_entries, len, default_entry, merge_block, has_default, outer_op)

向头块挂接 `Switch` 区域：case `i` 的 128 位匹配值由 `case_lo[i]`（低 64 位）与 `case_hi[i]`（高 64 位）组成，`case_entries[i]` 为其分支入口块。

- `ptr` (`struct CCircuitCFG*`)：CFG 句柄。
- `node` (`uint32_t`)：头块索引。
- `case_lo` (`const uint64_t*`)：各 case 匹配值低 64 位。
- `case_hi` (`const uint64_t*`)：各 case 匹配值高 64 位。
- `case_entries` (`const uint32_t*`)：各 case 分支入口块。
- `len` (`uintptr_t`)：case 数量。
- `default_entry` (`uint32_t`)：默认分支入口块。
- `merge_block` (`uint32_t`)：汇合块。
- `has_default` (`int32_t`)：是否存在默认分支体（1/0）。
- `outer_op` (`const struct COperation*`)：外层操作元数据，可为 NULL。

返回：`0` 成功；`-1` `ptr` 为 NULL，或 `len > 0` 时任一 case 数组为 NULL；`-2` 头块不存在。

---

## 标签常量

出边标签（`circuit_cfg_add_edge` 的 `flow`、`circuit_cfg_outgoing_edges` 的 `flow_tags`）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `CFG_FLOW_TRUE_BRANCH` | 1 | `if` / `while` / `for` 头块的真分支出边 |
| `CFG_FLOW_FALSE_BRANCH` | 2 | 假分支出边 |
| `CFG_FLOW_UNCONDITIONAL` | 3 | 顺序跳转 / 结构化汇合 |
| `CFG_FLOW_CASE` | 4 | `switch` 精确值匹配边（128 位值在 case 半字中） |
| `CFG_FLOW_DEFAULT_CASE` | 5 | `switch` 默认分支边 |
| `CFG_FLOW_BREAK` | 6 | 结构化 `break` 边 |
| `CFG_FLOW_CONTINUE` | 7 | 结构化 `continue` 边 |

终结符标签（`circuit_cfg_block_terminator_tag`）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `CFG_TERMINATOR_BRANCH` | 1 | 条件分支头（`Branch`） |
| `CFG_TERMINATOR_FOR_LOOP` | 2 | 无符号区间循环头（`ForLoop`） |
| `CFG_TERMINATOR_SWITCH` | 3 | 精确值多路分支头（`Switch`） |
| `CFG_TERMINATOR_JUMP` | 4 | 无条件跳转 |
| `CFG_TERMINATOR_BREAK` | 5 | 结构化 `break` |
| `CFG_TERMINATOR_CONTINUE` | 6 | 结构化 `continue` |
| `CFG_TERMINATOR_RETURN` | 7 | 执行结束（`Return`） |

区域标签（`circuit_cfg_region_tag`）：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `CFG_REGION_IF` | 1 | 结构化条件区域 |
| `CFG_REGION_WHILE` | 2 | 结构化 `while` 循环区域 |
| `CFG_REGION_FOR` | 3 | 结构化区间循环区域 |
| `CFG_REGION_SWITCH` | 4 | 结构化精确值 `switch` 区域 |

---

## 示例：构建 if-else CFG 并转回线路

先在线路层构造 `if (flag) { x(1) } else { z(1) }`，展开为 CFG 后读取 `If` 区域布局与头块出边，最后校验并还原为结构化线路：

```c
#include <stdint.h>
#include <stdio.h>
#include "cqlib_c.h"

/* then 分支体：X(1) */
static int32_t append_x(struct CCircuit* circuit, void* user_data) {
    (void)user_data;
    return circuit_x(circuit, 1);
}

/* else 分支体：Z(1) */
static int32_t append_z(struct CCircuit* circuit, void* user_data) {
    (void)user_data;
    return circuit_z(circuit, 1);
}

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    struct CClassicalVar* flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    struct CClassicalExpr* condition = classical_expr_var(flag);
    if (circuit_if_else(circuit, condition, append_x, append_z, NULL) != 0) {
        return 1;
    }

    /* 1. 展开为 CFG */
    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    if (cfg == NULL) {
        return 1;
    }

    /* 2. 定位 if 头块，读取区域布局与出边 */
    uint32_t then_entry = UINT32_MAX;
    uint32_t else_entry = UINT32_MAX;
    uint32_t merge_block = UINT32_MAX;
    int32_t has_else = 0;
    uint32_t block_count = (uint32_t)circuit_cfg_num_blocks(cfg);

    for (uint32_t node = 0; node < block_count; node++) {
        if (circuit_cfg_region_tag(cfg, node) == CFG_REGION_IF) {
            circuit_cfg_region_if(cfg, node, &then_entry, &else_entry,
                                  &merge_block, &has_else);

            /* 头块终结符为 Branch，两条出边按插入顺序：真分支在前 */
            uint32_t targets[2] = {0, 0};
            uint32_t tags[2] = {0, 0};
            uint64_t lo[2] = {0, 0};
            uint64_t hi[2] = {0, 0};
            circuit_cfg_outgoing_edges(cfg, node, targets, tags, lo, hi, 2);
            /* targets[0] == then_entry 且 tags[0] == CFG_FLOW_TRUE_BRANCH，
               targets[1] == else_entry 且 tags[1] == CFG_FLOW_FALSE_BRANCH */
            break;
        }
    }

    /* 3. 校验并转回结构化线路 */
    struct CCircuit* lowered = NULL;
    if (circuit_cfg_validate(cfg) == 0) {
        lowered = circuit_cfg_to_circuit(cfg);
    }

    circuit_free(lowered);
    circuit_cfg_free(cfg);
    classical_expr_free(condition);
    classical_var_free(flag);
    circuit_free(circuit);
    return 0;
}
```

线路构造接口（`circuit_new`、`circuit_if_else` 等）见 [线路](1_circuit.md)，经典表达式构造（`classical_expr_var` 等）见 [经典数据与控制流](9_classical_control_flow.md)。
