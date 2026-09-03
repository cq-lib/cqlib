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

- `Outcome.from_bitstring(bitstring) -> Outcome`

### 方法与属性

- `is_one(index) -> bool`
- `to_bitstring(num_qubits) -> str`
- `chunks -> list[int]`

说明：

- `Outcome` 支持 `==` 与 `hash(...)`。

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

## ExecutionResult

### `ExecutionResult(task_id, qubits, shots, num_qubits, backend=None)`

参数：

- `task_id` (`str`)
- `qubits` (`list[int]`)
- `shots` (`int`)
- `num_qubits` (`int`)
- `backend` (`str | None`)

### 生命周期方法

- `start() -> None`
- `finish(counts) -> None`
- `fail(msg, code) -> None`
- `cancel() -> None`
- `calc_probabilities() -> None`

`finish(counts)` 中 `counts` 类型为 `dict[str, int]`，key 必须是合法二进制比特串。

异常情况：

- `ValueError`：`counts` 的比特串非法。

### 属性

- `task_id -> str`
- `shots -> int`
- `num_qubits -> int`
- `qubits -> list[int]`
- `status -> Status`
- `created_at -> str`
- `started_at -> str | None`
- `finished_at -> str | None`
- `backend -> str | None`
- `counts -> dict[str, int]`
- `probabilities -> dict[str, float] | None`

## 示例

```python
from cqlib.device import ExecutionResult

result = ExecutionResult("task-1", [0, 1], 100, 2, "sim")
print(result.status.kind)  # queued

result.start()
result.finish({"00": 60, "11": 40})
result.calc_probabilities()

print(result.status.kind)   # completed
print(result.counts)        # {'00': 60, '11': 40}
print(result.probabilities) # {'00': 0.6, '11': 0.4}
```

