# ExecutionResult

`CExecutionResult` 记录任务采样数据。当前 C API 提供从测量计数构造的入口（设备端任务执行入口后续版本提供）。

比特串为大端格式：MSB 在左、qubit `N-1` 最先、qubit `0` 最后，与 [measure_all](../3_qis/1_statevector.md#概率与测量) 的输出方向一致。

---

## 函数

### execution_result_from_counts(task_id, num_qubits, shots, bitstrings, counts, num_entries)

由测量计数构造执行结果。

参数：

- `task_id` (`const char *`)：任务标识字符串。
- `num_qubits` (`uintptr_t`)：比特数。
- `shots` (`uintptr_t`)：采样总次数。
- `bitstrings` (`const char *const *`)：比特串数组。
- `counts` (`const uint64_t *`)：与 `bitstrings` 平行的计数数组。
- `num_entries` (`uintptr_t`)：条目数。

约束：

- 每个比特串长度必须等于 `num_qubits`，否则返回 NULL。

返回：

- `CExecutionResult *`：堆分配结果，需 `execution_result_free` 释放；失败返回 NULL。

### execution_result_shots(result)

返回采样总次数。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### execution_result_num_qubits(result)

返回比特数。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### execution_result_counts(result)

返回测量计数列表。

返回：

- `CCountsList *`：**独立拥有**的新对象，需 `counts_list_free` 释放；列表值为计数（以 `f64` 承载的整数权重）；失败返回 NULL。

### execution_result_probabilities(result)

计算（需要时从计数推导）并返回概率分布列表。

返回：

- `CCountsList *`：**独立拥有**的新对象，需 `counts_list_free` 释放；列表值为概率（`0..1` 的 `f64`）；失败返回 NULL。

### execution_result_free(ptr)

释放执行结果。允许传 NULL。

---

## CountsList 遍历

`execution_result_counts` 与 `execution_result_probabilities` 共用 `CCountsList`：

```c
uintptr_t counts_list_len(const struct CCountsList *ptr);
char *counts_list_get_key(const struct CCountsList *ptr, uintptr_t index);
double counts_list_get_value(const struct CCountsList *ptr, uintptr_t index);
void counts_list_free(struct CCountsList *ptr);
```

| 函数 | 返回 | 说明 |
| --- | --- | --- |
| `counts_list_len` | 条目数 | NULL 句柄返回 `0` |
| `counts_list_get_key(index)` | `char *` | 堆分配比特串，需 `cqlib_string_free`；越界返回 NULL |
| `counts_list_get_value(index)` | `double` | 计数或概率；错误返回 NaN |
| `counts_list_free` | `void` | 释放列表，允许传 NULL |

---

## 示例

### 构造与遍历

```c
const char *bits[2] = {"00", "11"};
const uint64_t cnt[2] = {480, 520};

CExecutionResult *res = execution_result_from_counts("task-1", 2, 1000, bits, cnt, 2);
if (!res) { return 1; }

printf("shots=%zu qubits=%zu\n",
       (size_t)execution_result_shots(res),
       (size_t)execution_result_num_qubits(res));

CCountsList *counts = execution_result_counts(res);
for (uintptr_t i = 0; i < counts_list_len(counts); i++) {
    char *k = counts_list_get_key(counts, i);
    printf("count(%s) = %.0f\n", k, counts_list_get_value(counts, i));
    cqlib_string_free(k);
}
counts_list_free(counts);

CCountsList *probs = execution_result_probabilities(res);
for (uintptr_t i = 0; i < counts_list_len(probs); i++) {
    char *k = counts_list_get_key(probs, i);
    printf("P(%s) = %.3f\n", k, counts_list_get_value(probs, i));
    cqlib_string_free(k);
}
counts_list_free(probs);

execution_result_free(res);
```
