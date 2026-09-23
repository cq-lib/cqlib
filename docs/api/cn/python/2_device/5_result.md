# Outcome / Status / ExecutionResult

本页覆盖量子任务结果相关 API：

- `Outcome`
- `Status`
- `ExecutionResult`

## 导入

```python
from cqlib.device import ExecutionResult, Outcome, Status
```

---

## Outcome

### `Outcome(bitstring)`

从二进制字符串构造测量结果对象。

异常情况：

- `ValueError`：字符串含非法字符（非 `0/1`）。

### 静态方法

- `Outcome.from_bitstring(bitstring) -> Outcome`：从二进制字符串构造。
- `Outcome.from_indices(width, indices) -> Outcome`：构造一个宽度为 `width`、仅在 `indices` 列出的位置上取值为 `1` 的结果。

### 方法与属性

- `is_one(index) -> bool`：指定位置是否为 `1`。
- `to_bitstring(num_qubits) -> str`：按给定宽度输出比特串。
- `chunks -> list[int]`：内部存储的分块。

说明：

- 比特串一律按标准二进制形式书写（最高位在左），而内部按小端分块存储。
- 位串宽度不由 `Outcome` 保存：解析 `"001"` 与解析 `"1"` 得到同一个 `Outcome`。因此输出宽度必须由调用方通过 `to_bitstring` 指定。
- `Outcome` 支持 `==` 与 `hash(...)`。

### 其他行为

- 支持 `==`、`hash(...)`、`copy`、`deepcopy`。
- `repr(Outcome("101")) == "Outcome(chunks=[5])"`。

## Status

### 静态构造方法

- `Status.queued()`
- `Status.running()`
- `Status.completed()`
- `Status.failed(error_msg, error_code)`
- `Status.cancelled()`

### 属性与方法

- `kind -> str`（`queued/running/completed/failed/cancelled`）
- `error_msg -> str | None`
- `error_code -> int | None`
- `is_terminal() -> bool`
- `is_success() -> bool`

### 其他行为

- 支持 `==` 与 `hash(...)`：状态相同且错误信息、错误码相同时才相等；与非 `Status` 对象比较返回 `False`。
- `repr(Status.completed()) == "Status(completed)"`，`str(Status.completed()) == "Completed"`。

## ExecutionResult

### `ExecutionResult(task_id, qubits, shots, num_qubits, backend=None)`

参数：

- `task_id` (`str`)
- `qubits` (`list[int | Qubit]`)：被测比特列表。
- `shots` (`int`)
- `num_qubits` (`int`)
- `backend` (`str | None`)

构造出的结果处于 `queued` 状态。

### `ExecutionResult.from_counts(task_id, qubits, shots, num_qubits, counts, backend=None)`

静态方法，直接由测量计数构造已完成的结果，等价于依次执行构造方法、`start()`、`finish(counts)` 与 `calc_probabilities()`。

`counts` 为 `dict[str, int]`，key 必须是长度等于 `num_qubits` 的合法二进制比特串。

异常情况：

- `ValueError`：`counts` 的比特串非法，或长度不等于 `num_qubits`。

### 生命周期方法

- `start() -> None`
- `finish(counts) -> None`
- `fail(msg, code) -> None`
- `cancel() -> None`
- `calc_probabilities() -> None`

`finish(counts)` 中 `counts` 类型为 `dict[str, int]`，key 必须是长度等于 `num_qubits` 的合法二进制比特串；写入计数会清除已计算的概率，需要重新调用 `calc_probabilities()`。`calc_probabilities()` 在计数总数为 0 时不产生概率。

异常情况：

- `ValueError`：`counts` 的比特串非法，或长度不等于 `num_qubits`。

### 属性

- `task_id -> str`
- `shots -> int`
- `num_qubits -> int`
- `qubits -> list[Qubit]`：被测比特列表。
- `status -> Status`
- `created_at -> str`：创建时间，ISO 8601 字符串。
- `started_at -> str | None`：开始时间，ISO 8601 字符串。
- `finished_at -> str | None`：完成时间，ISO 8601 字符串。
- `backend -> str | None`
- `counts -> dict[str, int]`
- `probabilities -> dict[str, float] | None`：调用 `calc_probabilities()` 后才可用。

说明：

- `qubits[i]` 对应比特串第 `i` 位（权重 `2**i`）；比特串按最高位在左书写，字符串顺序与 `qubits` 顺序相反。

### 其他行为

- 支持 `copy`、`deepcopy`。
- `repr(result) == "ExecutionResult(task_id='task-1', status='queued', shots=100, num_qubits=2)"`。

## 示例

```python
import pytest
from cqlib import Qubit
from cqlib.device import ExecutionResult, Outcome, Status

outcome = Outcome("101")
assert outcome.is_one(0) is True
assert outcome.is_one(1) is False
assert outcome.to_bitstring(3) == "101"
assert Outcome.from_bitstring("101") == outcome
assert Outcome.from_indices(3, [0, 2]) == outcome

failed = Status.failed("backend down", 42)
assert failed.kind == "failed"
assert failed.error_code == 42
assert failed.is_terminal() is True

result = ExecutionResult("task-1", [Qubit(0), Qubit(1)], 100, 2, "sim")
print(result.status.kind)  # queued

result.start()
result.finish({"00": 60, "11": 40})
result.calc_probabilities()

print(result.status.kind)   # completed
print(result.counts)        # {'00': 60, '11': 40}
print(result.probabilities) # {'00': 0.6, '11': 0.4}

done = ExecutionResult.from_counts("task-2", [Qubit(0), Qubit(1)], 100, 2, {"01": 60, "10": 40})
print(done.status.kind)  # completed
assert done.counts == {"01": 60, "10": 40}

with pytest.raises(ValueError):
    done.finish({"100": 1})
```

