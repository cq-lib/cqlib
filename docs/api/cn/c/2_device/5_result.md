# ExecutionResult（C）

`CExecutionResult` 是任务执行结果的不透明句柄，记录一次任务的生命周期、测量计数与概率分布；`CCountsList` 是「比特串 → 权重」的键值列表。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

`qubits` 数组的第 `i` 项对应结果键的第 `i` 位；比特串按最高位在左书写，与 `qubits` 顺序相反。

---

## 状态常量

`execution_result_status` 写出的标签：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `EXECUTION_STATUS_QUEUED` | 0 | 任务已提交并排队。 |
| `EXECUTION_STATUS_RUNNING` | 1 | 任务正在运行。 |
| `EXECUTION_STATUS_COMPLETED` | 2 | 任务成功完成。 |
| `EXECUTION_STATUS_FAILED` | 3 | 任务失败（细节查询错误码与错误消息）。 |
| `EXECUTION_STATUS_CANCELLED` | 4 | 任务被用户取消。 |

---

## 构造与释放

### execution_result_new(task_id, qubits, num_qubits, shots, backend)

创建一个处于 Queued 状态的执行结果；创建时间戳取当前时间。

参数：

- `task_id` (`const char*`)：任务 ID。
- `qubits` (`const uint32_t*`)：被测比特 ID 数组，第 `i` 项对应结果键的第 `i` 位。
- `num_qubits` (`uintptr_t`)：被测比特数。
- `shots` (`uintptr_t`)：shot 数。
- `backend` (`const char*`)：后端名，可为 NULL。

返回：成功返回新建的 `CExecutionResult*`；失败返回 NULL。

### execution_result_from_counts(task_id, num_qubits, shots, bitstrings, counts, num_entries)

由测量计数直接构造一个已完成（Completed）的结果，被测比特为 `0..num_qubits-1`。

参数：

- `task_id` (`const char*`)：任务 ID。
- `num_qubits` (`uintptr_t`)：被测比特数，每个比特串必须恰好有 `num_qubits` 个字符。
- `shots` (`uintptr_t`)：shot 数。
- `bitstrings` (`const char *const*`)：比特串数组，与 `counts` 平行。
- `counts` (`const uint64_t*`)：计数数组，与 `bitstrings` 平行。
- `num_entries` (`uintptr_t`)：条目数。

返回：成功返回新建的 `CExecutionResult*`；失败返回 NULL。

### execution_result_free(ptr)

释放 `CExecutionResult`，允许传 NULL。

---

## 生命周期

### execution_result_start(ptr)

把任务标记为运行中，并把当前时间记录为开始时间戳。

返回：0 成功；-1 空指针。

### execution_result_finish(ptr, bitstrings, counts, num_entries)

把任务标记为已完成并替换测量计数；`bitstrings` 与 `counts` 为平行数组，重复的比特串会累加；完成时间戳取当前时间。

参数：

- `bitstrings` (`const char *const*`)：比特串数组。
- `counts` (`const uint64_t*`)：计数数组。
- `num_entries` (`uintptr_t`)：条目数。

返回：0 成功；-1 空指针；-4 比特串非法（长度或字符）。

### execution_result_fail(ptr, msg, code)

把任务标记为失败，并记录错误消息与错误码。

参数：

- `msg` (`const char*`)：错误消息。
- `code` (`int32_t`)：错误码。

返回：0 成功；-1 空指针；-4 消息不是合法 UTF-8。

### execution_result_cancel(ptr)

把任务标记为已取消。

返回：0 成功；-1 空指针。

---

## 状态与时间戳

### execution_result_status(ptr, out_tag, out_error_code)

写出执行状态标签（`EXECUTION_STATUS_*` 之一）。状态为失败时，`*out_error_code` 收到保存的错误码（否则为 0）；不需要错误码时可传 NULL。

参数：

- `out_tag` (`uint8_t*`)：写出状态标签。
- `out_error_code` (`int32_t*`)：写出错误码，可为 NULL。

返回：0 成功；-1 空指针。

### execution_result_is_success(ptr)

返回执行是否成功完成。

返回：1 成功；0 否；-1 空指针。

### execution_result_is_terminal(ptr)

返回执行是否已到达终态（已完成、失败或已取消）；Queued 与 Running 不是终态。

返回：1 已到终态；0 否；-1 空指针。

### execution_result_error_message(ptr)

返回失败状态保存的错误消息。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；输入为 NULL 或状态不是失败时返回 NULL。

### execution_result_task_id(ptr)

返回任务 ID。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；失败返回 NULL。

### execution_result_backend(ptr)

返回后端名。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；失败或未设置后端时返回 NULL。

### execution_result_created_at(ptr, out_ms)

写出任务创建时间戳（Unix 纪元起的毫秒数）。

返回：0 并写入 `*out_ms`；-1 空指针。

### execution_result_started_at(ptr, out_ms)

写出执行开始时间戳（Unix 纪元起的毫秒数）。

返回：0 并写入 `*out_ms`；-8 任务尚未开始。

### execution_result_finished_at(ptr, out_ms)

写出执行完成时间戳（Unix 纪元起的毫秒数）。

返回：0 并写入 `*out_ms`；-8 任务尚未结束。

---

## 测量结果

### execution_result_shots(ptr)

返回 shot 数；`ptr` 为 NULL 时返回 0。

### execution_result_num_qubits(ptr)

返回被测比特数；`ptr` 为 NULL 时返回 0。

### execution_result_qubits_len(ptr) / execution_result_qubits(ptr, out, len)

两步式输出被测比特 ID，第 `i` 项对应结果键的第 `i` 位；填充函数返回总数。

### execution_result_to_bitstring(ptr, outcome)

把原始结果掩码格式化为按被测比特宽度书写的大端比特串：`outcome` 的第 `i` 位对应被测比特 `i`，超出宽度的高位被截断。

参数：

- `outcome` (`uint64_t`)：原始结果掩码。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；输入为 NULL 时返回 NULL。

### execution_result_counts(ptr)

返回测量计数。

返回：成功返回新建的 `CCountsList*`（比特串为键、整数计数以 f64 为权重），用 `counts_list_free` 释放；失败返回 NULL。

### execution_result_probabilities(ptr)

返回概率分布（需要时先按计数归一化计算）。

返回：成功返回新建的 `CCountsList*`（比特串为键、f64 概率为权重），用 `counts_list_free` 释放；失败返回 NULL。

### execution_result_calc_probabilities_len(ptr) / execution_result_calc_probabilities(ptr, out, len)

两步式概率分布：`calc_probabilities_len` 计算分布（结果缓存在对象上）并返回条目数；`calc_probabilities` 重新计算并把概率复制到 `out`。条目按结果值升序排列，结果值的第 `i` 位对应被测比特 `i`（与 `execution_result_to_bitstring` 一致）。两者都接收可变句柄；`len` 较小时只复制前 `len` 项。填充函数返回总条目数；`ptr` 为 NULL 时返回 0。

---

## CCountsList

`execution_result_counts` 与 `execution_result_probabilities` 的返回值：键为测量结果比特串，前者权重为计数，后者为概率。

### counts_list_len(ptr)

返回列表条目数；`ptr` 为 NULL 时返回 0。

### counts_list_get_key(ptr, index)

返回下标为 `index` 的比特串键。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；越界返回 NULL。

### counts_list_get_value(ptr, index)

返回下标为 `index` 的权重（计数或概率）；出错返回 NaN。

### counts_list_free(ptr)

释放 `CCountsList`，允许传 NULL。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 路径一：由计数直接构造已完成的结果 */
    const char *bits[2] = {"00", "11"};
    uint64_t cnts[2] = {750, 250};

    struct CExecutionResult *r = execution_result_from_counts(
        "task-1", 2, 1000, bits, cnts, 2);
    if (r == NULL) {
        return 1;
    }

    uint8_t status = 255;
    int32_t err_code = 0;
    execution_result_status(r, &status, &err_code);  /* EXECUTION_STATUS_COMPLETED */

    struct CCountsList *counts = execution_result_counts(r);
    if (counts != NULL) {
        for (uintptr_t i = 0; i < counts_list_len(counts); i++) {
            char *key = counts_list_get_key(counts, i);
            double w = counts_list_get_value(counts, i);
            printf("%s: %f\n", key, w);  /* "00": 750, "11": 250 */
            cqlib_string_free(key);
        }
        counts_list_free(counts);
    }

    struct CCountsList *probs = execution_result_probabilities(r);
    /* 同样遍历：权重为 0.75 / 0.25 */
    counts_list_free(probs);

    /* 状态标记、比特串与两步式概率缓冲区 */
    int32_t ok = execution_result_is_success(r);     /* 1 */
    int32_t done = execution_result_is_terminal(r);  /* 1 */

    char *bs = execution_result_to_bitstring(r, 0b01);     /* "01" */
    char *bs2 = execution_result_to_bitstring(r, 0b101);   /* "11"：第 2 位超出宽度，被截断 */
    cqlib_string_free(bs);
    cqlib_string_free(bs2);

    uintptr_t n_probs = execution_result_calc_probabilities_len(r);  /* 2 */
    double pvals[2];
    execution_result_calc_probabilities(r, pvals, n_probs);
    /* 结果值升序：pvals[0] == 0.75（"00"），pvals[1] == 0.25（"11"） */

    execution_result_free(r);

    /* 路径二：完整生命周期 */
    uint32_t qubits[2] = {0, 1};
    struct CExecutionResult *r2 = execution_result_new(
        "task-2", qubits, 2, 1000, "sim");
    if (r2 == NULL) {
        return 1;
    }

    execution_result_start(r2);              /* Queued -> Running */
    execution_result_finish(r2, bits, cnts, 2);  /* Running -> Completed */

    char *task = execution_result_task_id(r2);      /* "task-2" */
    char *backend = execution_result_backend(r2);   /* "sim" */
    cqlib_string_free(task);
    cqlib_string_free(backend);

    int64_t created = 0, started = 0, finished = 0;
    execution_result_created_at(r2, &created);
    execution_result_started_at(r2, &started);
    execution_result_finished_at(r2, &finished);

    execution_result_free(r2);
    return 0;
}
```
