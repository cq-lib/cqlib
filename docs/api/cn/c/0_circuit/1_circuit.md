# Circuit（线路）

`CCircuit*` 是量子线路的主容器句柄，保存逻辑量子比特集合、操作序列、参数表、经典变量与经典值、控制流作用域以及全局相位。本页覆盖线路的构造释放、属性查询、结构变换、检查点事务、全局相位与参数绑定；门函数见 [标准门](5_gate_standard.md)，符号参数细节见 [符号参数](3_parameter.md)。错误码与内存约定见 [Overview](../0_overview.md)。

---

## 构造与释放

### circuit_new(num_qubits)

创建一条包含连续逻辑量子比特的空线路，比特 id 从 `0` 到 `num_qubits - 1`。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回：新分配的 `CCircuit*`，用 `circuit_free` 释放。

### circuit_from_qubits(qubits, len)

用显式给出的比特 id 列表创建线路，允许稀疏编号（例如 `{2, 5}`）。`len` 为 0 时创建空线路，此时 `qubits` 可以为 NULL。

- `qubits` (`const uint32_t*`)：比特 id 数组。
- `len` (`uintptr_t`)：数组长度。

返回：新分配的 `CCircuit*`；`len > 0` 时 `qubits` 为 NULL 或 id 重复返回 `NULL`。

### circuit_from_operations(qubits, qubits_len, operations, operations_len)

用一组比特加上一批已解析的值级操作（由 `circuit_index` 产生，见 [操作与指令](4_operation_instruction.md)）创建线路。操作按顺序经值级降序逐个追加，因此会校验比特归属，符号参数也会驻留进新线路的参数表。

- `qubits` (`const uint32_t*`)：比特 id 数组。
- `qubits_len` (`uintptr_t`)：数组长度；为 0 时创建空比特集合，`qubits` 可为 NULL。
- `operations` (`const CValueOperation* const*`)：已解析操作快照数组。
- `operations_len` (`uintptr_t`)：操作数量。

返回：新分配的 `CCircuit*`，用 `circuit_free` 释放；输入为 NULL（含 `operations` 中出现 NULL 元素）、比特重复或任一操作追加失败时返回 `NULL`。

### circuit_append_value_operation(ptr, op)

向线路追加一条已解析的值级操作（由 `circuit_index` 产生）。

- `ptr` (`CCircuit*`)：线路句柄。
- `op` (`const CValueOperation*`)：已解析操作快照。

返回：`0` 成功；`-1` NULL；`-3` 应用失败（例如操作引用了线路之外的比特）。

### circuit_free(ptr)

释放线路句柄。允许传 `NULL`。

- `ptr` (`CCircuit*`)：线路句柄。

```c
/* 稀疏编号线路：比特 id 为 2 和 5 */
uint32_t ids[2] = {2, 5};
CCircuit *qc = circuit_from_qubits(ids, 2);
if (qc != NULL) {
    /* circuit_width(qc) == 2 */
    circuit_free(qc);
}
```

---

## 属性查询

### circuit_num_qubits(ptr)

返回量子比特数量。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：比特数量；`NULL` 时返回 0。

### circuit_width(ptr)

返回线路宽度，即量子比特数量，语义与 `circuit_num_qubits` 一致。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：比特数量；`NULL` 时返回 0。

### circuit_num_operations(ptr)

返回顶层操作数量。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：操作数量；`NULL` 时返回 0。

### circuit_num_parameters(ptr)

返回线路参数表中驻留的符号参数数量，与 `circuit_parameters_len` 相同。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：参数数量；`NULL` 时返回 0。

### circuit_qubits_len(ptr)

返回比特数量，用于为 `circuit_qubits` 分配缓冲。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：比特数量；`NULL` 时返回 0。

### circuit_qubits(ptr, out, len)

按插入顺序把比特 id 拷贝进 `out`。`out` 为 NULL 或 `len` 不足时只返回数量、不拷贝。

- `ptr` (`const CCircuit*`)：线路句柄。
- `out` (`uint32_t*`)：输出缓冲，可为 NULL（仅查询数量）。
- `len` (`uintptr_t`)：缓冲长度。

返回：比特总数。

### circuit_depth(ptr, recurse)

计算线路深度。`recurse = true` 时递归展开控制流体中的操作。

- `ptr` (`const CCircuit*`)：线路句柄。
- `recurse` (`bool`)：是否递归统计控制流内部操作。

返回：线路深度；`-1` 表示 NULL 句柄，`-3` 表示失败（例如线路含控制流而未递归）。

### circuit_validate(ptr)

验证线路一致性：经典句柄归属、控制流作用域、数据依赖与结构不变量。外部导入或自动生成的线路建议先验证再进入后续流程。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：`0` 成功；`-1` NULL；`-3` 验证失败。

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

if (circuit_validate(qc) == 0) {
    /* num_qubits=2, num_operations=2, depth=2 */
    uintptr_t n = circuit_qubits_len(qc);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    circuit_qubits(qc, ids, n);      /* ids == {0, 1} */
    free(ids);
}
circuit_free(qc);
```

### circuit_contains_qubit(ptr, qubit)

判断 `qubit` 是否属于该线路。

- `ptr` (`const CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：比特 id。

返回：`1` 属于；`0` 不属于；`-1` NULL。

### circuit_has_same_qubits(a, b)

判断两条线路是否拥有相同的有序比特域。

- `a` (`const CCircuit*`)：第一条线路句柄。
- `b` (`const CCircuit*`)：第二条线路句柄。

返回：`1` 相同；`0` 不同；`-1` NULL。

### circuit_operations_structurally_equal(a, b)

判断两条线路是否结构相等：有序比特、参数表与操作序列全部一致（参数下标跨两张参数表重映射后比较，线路内的经典句柄按结构比较）。

- `a` (`const CCircuit*`)：第一条线路句柄。
- `b` (`const CCircuit*`)：第二条线路句柄。

返回：`1` 相等；`0` 不相等；`-1` NULL。

---

## 量子比特管理

### circuit_add_qubits(ptr, qubits, count)

向线路追加新的逻辑量子比特。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubits` (`const uint32_t*`)：新比特 id 数组。
- `count` (`uintptr_t`)：追加数量。

返回：`0` 成功；`-1` NULL；`-3` 失败（例如与已有比特重复）。

---

## 结构变换

### circuit_compose(ptr, other, qubits_map, map_len)

把 `other` 追加到当前线路末尾。`qubits_map` 按 `other` 的比特顺序将其比特映射到当前线路的比特；传 NULL 映射（`map_len` 为 0）时按 id 恒等合并，`other` 中不存在于当前线路的比特会被并入。

- `ptr` (`CCircuit*`)：目标线路（被修改）。
- `other` (`const CCircuit*`)：被组合线路（不修改）。
- `qubits_map` (`const uint32_t*`)：比特映射数组，可为 NULL。
- `map_len` (`uintptr_t`)：映射长度。

返回：`0` 成功；`-1` NULL；`-3` 失败（映射不合法或结构校验失败）。

```c
CCircuit *lhs = circuit_new(3);
CCircuit *rhs = circuit_new(2);
circuit_cx(rhs, 0, 1);

/* rhs 的比特 0、1 依次映射到 lhs 的比特 1、2 */
uint32_t map[2] = {1, 2};
int32_t rc = circuit_compose(lhs, rhs, map, 2);   /* rc == 0 */

circuit_free(rhs);
circuit_free(lhs);
```

### circuit_inverse(ptr)

返回当前线路的反线路：操作逆序并逐个取逆，全局相位取反。原线路不变。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：新分配的 `CCircuit*`，用 `circuit_free` 释放；包含不可逆操作（测量、reset 等）或失败时返回 `NULL`。

### circuit_decompose(ptr)

返回展开复合门后的线路副本：由线路定义的门（`circuit_circuit_gate` 追加的复合门）被展开为基础操作。原线路不变。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：新分配的 `CCircuit*`；失败返回 `NULL`。

### circuit_remove_operation(ptr, index)

删除下标为 `index` 的顶层操作。

- `ptr` (`CCircuit*`)：线路句柄。
- `index` (`uintptr_t`)：操作下标。

返回：`0` 成功；`-1` NULL；`-3` 下标越界或删除后结构校验失败。

### circuit_remove_operations(ptr, indices, len)

按下标批量删除顶层操作。`indices` 按删除前的操作列表解释；重复下标被忽略；任一失败时整体回滚（原子操作）。

- `ptr` (`CCircuit*`)：线路句柄。
- `indices` (`const uintptr_t*`)：操作下标数组。
- `len` (`uintptr_t`)：下标数量（可为 0）。

返回：`0` 成功（含空下标列表）；`-1` NULL；`-3` 任一下标越界或有经典值仍被引用。

```c
CCircuit *qc = circuit_new(1);
circuit_rx(qc, 0, 0.7);

CCircuit *inv = circuit_inverse(qc);   /* 反线路：RX(-0.7) */
if (inv != NULL && circuit_validate(inv) == 0) {
    /* 使用 inv ... */
}
circuit_free(inv);
circuit_free(qc);
```

---

## 检查点事务

检查点函数族通过不透明的 `CCheckpoint*` 令牌捕获与恢复线路状态（操作列表、参数与符号表、经典表、控制流作用域）。令牌由 `circuit_checkpoint` / `circuit_begin` 产生，且只能被 `circuit_commit`、`circuit_rollback_to` 或 `circuit_rollback_control_body_transaction` 消费一次；未消费的令牌用 `checkpoint_free` 释放。

```c
CCircuit *qc = circuit_new(1);
CCheckpoint *cp = circuit_checkpoint(qc);
circuit_h(qc, 0);                        /* 试验性操作 */

if (circuit_validate(qc) != 0) {
    circuit_rollback_to(qc, cp);         /* 撤销检查点之后的全部变更 */
} else {
    circuit_commit(qc, cp);              /* 保留这些操作 */
}

circuit_free(qc);
```

### circuit_checkpoint(ptr)

捕获线路状态检查点。返回的令牌只能被 `circuit_commit`、`circuit_rollback_to` 或 `circuit_rollback_control_body_transaction` 消费一次。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：新分配的 `CCheckpoint*`；NULL 输入时返回 `NULL`。

### circuit_begin(ptr)

开启一个外部驱动的构造事务；等价于 `circuit_checkpoint`。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：新分配的 `CCheckpoint*`；NULL 输入时返回 `NULL`。

### circuit_commit(ptr, checkpoint)

提交自令牌捕获以来新增的状态，并消费该令牌。

- `ptr` (`CCircuit*`)：线路句柄。
- `checkpoint` (`CCheckpoint*`)：`circuit_checkpoint` / `circuit_begin` 产生的令牌。

返回：`0` 成功；`-1` NULL。

### circuit_rollback_to(ptr, checkpoint)

回滚自令牌捕获以来新增的全部状态，并消费该令牌。

- `ptr` (`CCircuit*`)：线路句柄。
- `checkpoint` (`CCheckpoint*`)：`circuit_checkpoint` / `circuit_begin` 产生的令牌。

返回：`0` 成功；`-1` NULL。

### circuit_rollback_control_body_transaction(ptr, checkpoint)

回滚自令牌捕获以来新增的全部状态，并消费该令牌；行为与 `circuit_rollback_to` 相同，按核心事务名保留。

- `ptr` (`CCircuit*`)：线路句柄。
- `checkpoint` (`CCheckpoint*`)：`circuit_checkpoint` / `circuit_begin` 产生的令牌。

返回：`0` 成功；`-1` NULL。

### checkpoint_free(ptr)

释放未消费的检查点令牌。允许传 `NULL`。

- `ptr` (`CCheckpoint*`)：待释放令牌。

---

## 全局相位

### circuit_global_phase(ptr, out)

读取线路全局相位。结果写入 `CParameterValue`：固定数值，或符号情形下新分配的 `CParameter*`（用 `param_free` 释放，见 [符号参数](3_parameter.md)）。

- `ptr` (`const CCircuit*`)：线路句柄。
- `out` (`CParameterValue*`)：写出结构。

返回：`0` 成功；`-1` NULL。

### circuit_set_global_phase(ptr, phase)

把全局相位设置为数值。

- `ptr` (`CCircuit*`)：线路句柄。
- `phase` (`double`)：全局相位（弧度）。

返回：`0` 成功；`-1` NULL；`-3` 相位为非有限值。

### circuit_set_global_phase_param(ptr, param)

把全局相位设置为符号参数（参数被克隆）。

- `ptr` (`CCircuit*`)：线路句柄。
- `param` (`const CParameter*`)：符号参数句柄。

返回：`0` 成功；`-1` NULL。

---

## 参数绑定

### circuit_assign_params(circuit, bindings)

返回一条新线路：能由绑定串求值的驻留参数替换为固定数值，其余保持符号形式；原线路不变，可作为模板复用。

- `circuit` (`const CCircuit*`)：线路句柄。
- `bindings` (`const char*`)：绑定串，格式为 `"name:value,name2:value2"`（与 `param_evaluate` 相同）；传 NULL 或格式非法时按不提供绑定处理。

返回：新分配的 `CCircuit*`，用 `circuit_free` 释放；线路含经典控制流或替换求值失败时返回 `NULL`。

```c
CCircuit *tpl = circuit_new(1);
CParameter *theta = param_parse("theta");
circuit_rx_param(tpl, 0, theta);
param_free(theta);

CCircuit *bound = circuit_assign_params(tpl, "theta:0.5");
/* bound 中不再引用符号 theta */

circuit_free(bound);
circuit_free(tpl);
```

---

## 非酉指令与恒等门

### circuit_measure(ptr, qubit)

在 `qubit` 上追加计算基测量指令，产生线路所有的不可变经典 `Bit` 值。读取测量结果、写入已有经典变量等变体见 [经典数据与控制流](9_classical_control_flow.md)。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：被测比特 id。

返回：`0` 成功；`-1` NULL；`-2` 比特越界；`-3` 失败。

### circuit_reset(ptr, qubit)

在 `qubit` 上追加复位指令，将比特复位到 `|0>`。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：比特 id。

返回：`0` 成功；`-1` NULL；`-2` 比特越界；`-3` 失败。

### circuit_barrier(ptr, qubits, count)

在给定比特上插入 barrier，阻止相关操作跨越该边界重排。`qubits` 传 NULL 或 `count` 为 0 时对所有线路比特插入全局 barrier。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubits` (`const uint32_t*`)：比特 id 数组，可为 NULL。
- `count` (`uintptr_t`)：比特数量。

返回：`0` 成功；`-1` NULL；`-3` 比特不存在或失败。

### circuit_id(ptr, qubit)

在 `qubit` 上追加恒等门，是 `circuit_i` 的别名，为前端命名兼容保留。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：比特 id。

返回：`0` 成功；`-1` NULL；`-2` 比特越界；`-3` 失败。
