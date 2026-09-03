# Outcome / Status / ExecutionResult

本页覆盖量子任务结果相关类型：

- `Outcome`
- `Status`
- `ExecutionResult`
- `OutcomeError`

## 导入

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, OutcomeError, Status};
use std::collections::HashMap;
use time::OffsetDateTime;
```

## OutcomeError

当前包含：

- `OutcomeError::InvalidCharacter(index, ch)`：比特串含非法字符。

## Outcome

### 构造与转换

- `Outcome::new(chunks: SmallVec<[u64; 4]>) -> Outcome`
- `Outcome::from_bitstring(s: &str) -> Result<Outcome, OutcomeError>`
- `to_string(&self, num_qubits: usize) -> String`

### 查询

- `is_one(&self, index: usize) -> bool`

说明：

- `Outcome` 实现了 `Clone + Eq + Hash`，可直接作为计数字典 `HashMap` 的 key。

## Status

枚举变体：

- `Queued`
- `Running`
- `Completed`
- `Failed { error_msg: String, error_code: i32 }`
- `Cancelled`

方法：

- `is_terminal(&self) -> bool`
- `is_success(&self) -> bool`

## ExecutionResult

### 构造

```rust
ExecutionResult::new(
    task_id: String,
    qubits: Vec<Qubit>,
    shots: usize,
    num_qubits: usize,
    backend: Option<String>,
    created_at: Option<OffsetDateTime>,
) -> ExecutionResult
```

### 生命周期方法

- `start(&mut self, t: Option<OffsetDateTime>) -> &mut Self`
- `finish(&mut self, counts: HashMap<Outcome, usize>, t: Option<OffsetDateTime>) -> &mut Self`
- `fail(&mut self, msg: String, code: i32)`
- `cancel(&mut self)`
- `calc_probabilities(&mut self) -> &mut Self`

### 读取方法

- `task_id(&self) -> &str`
- `shots(&self) -> usize`
- `num_qubits(&self) -> usize`
- `qubits(&self) -> &Vec<Qubit>`
- `status(&self) -> &Status`
- `created_at(&self) -> &OffsetDateTime`
- `started_at(&self) -> &Option<OffsetDateTime>`
- `finished_at(&self) -> &Option<OffsetDateTime>`
- `backend(&self) -> Option<&String>`
- `counts(&self) -> &HashMap<Outcome, usize>`
- `probabilities(&self) -> &Option<HashMap<Outcome, f64>>`

## 示例

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
use std::collections::HashMap;

let mut result = ExecutionResult::new(
    "task-1".to_string(),
    vec![Qubit::new(0), Qubit::new(1)],
    100,
    2,
    Some("sim".to_string()),
    None,
);

assert!(matches!(result.status(), Status::Queued));
result.start(None);
assert!(matches!(result.status(), Status::Running));

let mut counts = HashMap::new();
counts.insert(Outcome::from_bitstring("00").unwrap(), 60);
counts.insert(Outcome::from_bitstring("11").unwrap(), 40);
result.finish(counts, None).calc_probabilities();

assert!(matches!(result.status(), Status::Completed));
assert!(result.probabilities().is_some());
```
