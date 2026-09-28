# Operation / Instruction（操作与指令）

`COperation*` 与 `CValueOperation*` 是读取线路操作的两个只读视图。存储层操作 `COperation` 的参数可能以所属线路参数表下标的形式记录，需结合线路上下文解析；`circuit_index` 返回的 `CValueOperation` 是解析后的自包含快照，参数为 `CParameterValue`（固定数值或完整符号表达式），独立于线路后续变更。延迟指令见本页末尾。门函数与门名清单见 [标准门](5_gate_standard.md)，`CParameterValue` 的内存规则见 [符号参数](3_parameter.md)，错误码与内存约定见 [Overview](../0_overview.md)。

---

## 存储层操作

`COperation*` 表示"指令 + 作用比特 + 参数 + 可选标签"的一次具体应用。`operation_new` 按门名创建标准门操作，参数为固定 double 数值。

### operation_new(gate_name, qubits, qubits_len, params, params_len)

按标准门名（如 `"H"`、`"CX"`、`"RZZ"`）创建操作，`params` 为固定 double 参数值。

- `gate_name` (`const char*`)：标准门名。
- `qubits` (`const uint32_t*`)：作用比特 id 数组。
- `qubits_len` (`uintptr_t`)：比特数量。
- `params` (`const double*`)：固定参数值数组。
- `params_len` (`uintptr_t`)：参数数量。

返回：新分配的 `COperation*`，用 `operation_free` 释放；`gate_name` 为 NULL 或未知门名时返回 `NULL`；`qubits_len > 0` 时 `qubits` 为 NULL、或 `params_len > 0` 时 `params` 为 NULL 同样返回 `NULL`。

### operation_free(ptr)

释放操作句柄。允许传 `NULL`。

- `ptr` (`COperation*`)：操作句柄。

### operation_name(ptr)

返回指令名（如 `"H"`、`"RZZ"`）。

- `ptr` (`const COperation*`)：操作句柄。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；`NULL` 时返回 `NULL`。

### operation_is_standard_gate(ptr)

判断操作是否由标准门指令驱动。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_num_qubits(ptr)

返回操作作用的比特数量。

- `ptr` (`const COperation*`)：操作句柄。

返回：比特数量；`NULL` 时返回 0。

### operation_qubits_len(ptr)

返回操作作用的比特数量，用于为 `operation_qubits` 分配缓冲。

- `ptr` (`const COperation*`)：操作句柄。

返回：比特数量；`NULL` 时返回 0。

### operation_qubits(ptr, buffer, len)

按作用顺序把比特 id 拷贝进 `buffer`。

- `ptr` (`const COperation*`)：操作句柄。
- `buffer` (`uint32_t*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲长度。

返回：`0` 成功；`-1` NULL；`-8` `len` 小于比特数量。

### operation_params_len(ptr)

返回操作携带的参数数量。

- `ptr` (`const COperation*`)：操作句柄。

返回：参数数量；`NULL` 时返回 0。

### operation_params(ptr, tags, values, len)

把参数拷贝进 `tags` 与 `values` 两个平行数组。`tags[i]` 为 `OPERATION_PARAM_*` 常量；`values[i]` 为固定数值，或 `OPERATION_PARAM_INDEX` 标记下的参数表下标（以 double 表示）。

- `ptr` (`const COperation*`)：操作句柄。
- `tags` (`uint32_t*`)：输出参数标签数组。
- `values` (`double*`)：输出参数值数组。
- `len` (`uintptr_t`)：缓冲长度。

返回：`0` 成功；`-1` NULL；`-8` `len` 小于参数数量。

参数标签：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `OPERATION_PARAM_FIXED` | 0 | 固定数值，`values[i]` 为数值本身。 |
| `OPERATION_PARAM_INDEX` | 1 | 所属线路参数表下标，`values[i]` 为下标值，需结合线路参数表解析。 |

### operation_label(ptr)

返回操作的可选元数据标签。

- `ptr` (`const COperation*`)：操作句柄。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；操作无标签或句柄为 NULL 时返回 `NULL`。

```c
uint32_t qubits[2] = {0, 1};
double params[1] = {0.7};
COperation *op = operation_new("RZZ", qubits, 2, params, 1);
if (op != NULL) {
    char *name = operation_name(op);        /* "RZZ" */
    cqlib_string_free(name);

    uintptr_t n = operation_qubits_len(op);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    operation_qubits(op, ids, n);            /* ids == {0, 1} */
    free(ids);

    operation_free(op);
}
```

---

## 指令种类判别

每个判别函数检查操作携带的指令种类，返回值统一为：`1` 是、`0` 否、`-1` 句柄为 NULL。

### operation_is_unitary(ptr)

判断操作是否使用用户自定义幺正指令。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_mcgate(ptr)

判断操作是否使用多受控门指令。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_circuit_gate(ptr)

判断操作是否使用线路定义的门指令。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_directive(ptr)

判断操作是否使用非酉 directive 指令（barrier、measure、reset）。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_delay(ptr)

判断操作是否使用延迟指令。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_classical_control(ptr)

判断操作是否使用结构化经典控制流指令（`if` / `while` / `for` / `switch` / `break` / `continue`）。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_classical_data(ptr)

判断操作是否使用经典数据指令（store、测量写入值）。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_quantum_gate(ptr)

判断操作是否为酉量子门（标准门、多受控门、用户自定义幺正门或线路定义的门）。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 否；`-1` NULL。

### operation_is_instruction(ptr)

判断操作携带的是普通指令而非结构化经典控制流。

- `ptr` (`const COperation*`)：操作句柄。

返回：`1` 是；`0` 为经典控制流操作；`-1` NULL。

---

## 指令内省

### operation_gate_arity(ptr, qubits, params)

写出指令固有的 `(qubit_count, parameter_count)`。

- `ptr` (`const COperation*`)：操作句柄。
- `qubits` (`uintptr_t*`)：写出的量子比特数量。
- `params` (`uintptr_t*`)：写出的参数数量。

返回：`0` 成功；`-1` NULL；`-3` 数量为可变（barrier、多比特测量、经典控制流）。

### operation_as_instruction(ptr)

当操作携带普通（非控制流）指令时，返回其拥有所有权的克隆。

- `ptr` (`const COperation*`)：操作句柄。

返回：新分配的 `COperation*`，用 `operation_free` 释放；输入为 NULL 或操作是经典控制流指令时返回 `NULL`。

### operation_directive(ptr)

以 `OPERATION_DIRECTIVE_*` 标签形式返回操作携带的 directive。

- `ptr` (`const COperation*`)：操作句柄。

返回：directive 标签；操作不是 directive 时返回 `OPERATION_DIRECTIVE_NONE`（0）；`-1` NULL。

directive 标签：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `OPERATION_DIRECTIVE_NONE` | 0 | 操作不携带 directive 指令。 |
| `OPERATION_DIRECTIVE_BARRIER` | 1 | Barrier directive。 |
| `OPERATION_DIRECTIVE_MEASURE` | 2 | Measure directive。 |
| `OPERATION_DIRECTIVE_RESET` | 3 | Reset directive。 |

### operation_result(ptr, out)

把测量操作产生的不可变经典值快照写入 `out`：线路内值表下标加上 (tag, width) 类型分量（`CClassicalValueInfo` 布局见 [经典数据与控制流](9_classical_control_flow.md)）。

- `ptr` (`const COperation*`)：操作句柄。
- `out` (`CClassicalValueInfo*`)：输出结构。

返回：`0` 成功；`-1` NULL；`-3` 操作不是测量（无经典结果）。

### operation_reads_value(ptr, expr)

判断操作是否直接或递归读取 `expr` 引用的不可变经典值。

- `ptr` (`const COperation*`)：操作句柄。
- `expr` (`const CClassicalExpr*`)：引用经典值的表达式句柄。

返回：`1` 是；`0` 否；`-1` NULL；`-8` `expr` 不是普通的值读取表达式。

---

## 操作矩阵

三个接口配合使用：先查长度，再取维度与数据。非酉指令或含未解析符号参数的操作没有数值矩阵。

### operation_matrix_len(ptr)

返回幺正矩阵的复数元素总数（行数 × 列数）。

- `ptr` (`const COperation*`)：操作句柄。

返回：复数元素数量；`NULL` 输入或操作无数值矩阵（非酉指令或未解析的符号参数）时返回 0。

### operation_matrix_dims(ptr, rows, cols)

把矩阵形状写入 `rows` 与 `cols`。

- `ptr` (`const COperation*`)：操作句柄。
- `rows` (`uintptr_t*`)：写出行数。
- `cols` (`uintptr_t*`)：写出列数。

返回：`0` 成功；`-1` NULL；`-3` 操作无数值矩阵。

### operation_matrix(ptr, out, buffer_len)

把幺正矩阵按行主序、交错 `(re, im)` double 拷贝进 `out`，缓冲至少容纳 `2 × operation_matrix_len` 个 double。

- `ptr` (`const COperation*`)：操作句柄。
- `out` (`double*`)：输出缓冲。
- `buffer_len` (`uintptr_t`)：缓冲长度（double 数量）。

返回：写出的复数元素数量；失败时返回 0。

```c
uint32_t q0 = 0;
COperation *h = operation_new("H", &q0, 1, NULL, 0);
if (h != NULL) {
    uintptr_t n = operation_matrix_len(h);              /* 4（2x2 复数元素） */
    double *buf = malloc(2 * n * sizeof(double));
    uintptr_t written = operation_matrix(h, buf, 2 * n); /* written == 4 */
    free(buf);
    operation_free(h);
}
```

---

## 解析快照

`circuit_index` 返回顶层操作的解析快照 `CValueOperation*`：只读、自包含，参数为 `CParameterValue`（固定数值或完整符号表达式，符号情形的 `param` 字段用 `param_free` 释放，见 [符号参数](3_parameter.md)），不随线路后续变更而改变。

### circuit_index(ptr, index)

返回下标 `index` 处顶层操作的解析快照。

- `ptr` (`const CCircuit*`)：线路句柄。
- `index` (`uintptr_t`)：操作下标。

返回：新分配的 `CValueOperation*`，用 `value_operation_free` 释放；NULL 输入或下标越界时返回 `NULL`。

### value_operation_free(ptr)

释放快照句柄。允许传 `NULL`。

- `ptr` (`CValueOperation*`)：快照句柄。

### value_operation_name(ptr)

返回操作的指令名（如 `"H"`、`"delay"`）。

- `ptr` (`const CValueOperation*`)：快照句柄。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；`NULL` 时返回 `NULL`。

### value_operation_instruction_type(ptr)

返回操作的指令类别（如 `"standard"`、`"delay"`）。

- `ptr` (`const CValueOperation*`)：快照句柄。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；`NULL` 时返回 `NULL`。

### value_operation_label(ptr)

返回操作的可选元数据标签。

- `ptr` (`const CValueOperation*`)：快照句柄。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；操作无标签或句柄为 NULL 时返回 `NULL`。

### value_operation_num_qubits(ptr)

返回操作作用的比特数量，用于为 `value_operation_qubits` 分配缓冲。

- `ptr` (`const CValueOperation*`)：快照句柄。

返回：比特数量；`NULL` 时返回 0。

### value_operation_qubits(ptr, out, len)

两步式读取作用比特 id：先调用 `value_operation_num_qubits` 分配缓冲，再拷贝进 `out`。

- `ptr` (`const CValueOperation*`)：快照句柄。
- `out` (`uint32_t*`)：输出缓冲，可为 NULL（仅查询数量）。
- `len` (`uintptr_t`)：缓冲长度。

返回：比特总数。

### value_operation_num_params(ptr)

返回操作携带的参数数量。

- `ptr` (`const CValueOperation*`)：快照句柄。

返回：参数数量；`NULL` 时返回 0。

### value_operation_param(ptr, index, out)

读取下标 `index` 处的参数，写出 `CParameterValue`。

- `ptr` (`const CValueOperation*`)：快照句柄。
- `index` (`uintptr_t`)：参数下标。
- `out` (`CParameterValue*`)：写出的解析值；符号情形的 `param` 字段用 `param_free` 释放。

返回：`0` 成功；`-1` NULL；`-3` 下标越界。

```c
CCircuit *qc = circuit_new(1);
circuit_rx(qc, 0, 0.7);

CValueOperation *op = circuit_index(qc, 0);
if (op != NULL) {
    char *type = value_operation_instruction_type(op);   /* "standard" */
    cqlib_string_free(type);

    CParameterValue v;
    if (value_operation_param(op, 0, &v) == 0) {
        /* v.tag == PARAMETER_VALUE_TAG_FIXED, v.value == 0.7 */
    }
    value_operation_free(op);
}
circuit_free(qc);
```

---

## 延迟指令

### circuit_delay(ptr, qubit, duration)

在 `qubit` 上追加数值时长的延迟（空闲时段）指令。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：比特 id。
- `duration` (`double`)：时长。

返回：`0` 成功；`-1` NULL；`-2` 比特越界；`-3` 时长非有限值。

### circuit_delay_param(ptr, qubit, duration)

在 `qubit` 上追加符号时长的延迟指令（时长参数被克隆）。

- `ptr` (`CCircuit*`)：线路句柄。
- `qubit` (`uint32_t`)：比特 id。
- `duration` (`const CParameter*`)：符号时长句柄。

返回：`0` 成功；`-1` NULL；`-2` 比特越界；`-3` 失败。

```c
CCircuit *qc = circuit_new(2);
circuit_delay(qc, 0, 100.0);          /* 数值时长 */

CParameter *tau = param_parse("tau");
circuit_delay_param(qc, 1, tau);      /* 符号时长 */
param_free(tau);

circuit_free(qc);
```
