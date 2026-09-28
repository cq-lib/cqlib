# Outcome / Status / ExecutionResult

本页覆盖量子任务结果相关类型：

- `Outcome`
- `Status`
- `ExecutionResult`
- `OutcomeError`

## 导入

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::result::OutcomeError;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
use std::collections::HashMap;
use time::OffsetDateTime;
```

## OutcomeError

定义在 `cqlib_core::device::result`，不在 `cqlib_core::device` 模块根重导出。

当前包含：

- `OutcomeError::InvalidCharacter(index, ch)`：比特串含非法字符。

## Outcome

### 构造与转换

- `Outcome::new(chunks: SmallVec<[u64; 4]>) -> Outcome`
- `Outcome::from_indices(width: usize, indices: impl IntoIterator<Item = usize>) -> Outcome`：构造宽度为 `width`、仅在 `indices` 列出的位置取值为 `1` 的结果，超出 `width` 的下标被忽略。
- `Outcome::from_bitstring(s: &str) -> Result<Outcome, OutcomeError>`
- 实现 `FromStr`，`s.parse::<Outcome>()` 等价于 `Outcome::from_bitstring(s)`，错误类型同为 `OutcomeError`。
- `to_bitstring(&self, num_qubits: usize) -> String`：按给定宽度输出比特串。

说明：

- 比特串一律按标准二进制形式书写（最高位在左），而内部按小端分块存储。
- 位串宽度不由 `Outcome` 保存：解析 `"001"` 与解析 `"1"` 得到同一个 `Outcome`。因此输出宽度必须由调用方通过 `to_bitstring` 指定；请求宽度小于已存储值时，高位被截断。
- `Outcome` 实现了 `Clone + Eq + Hash`，可直接作为计数字典 `HashMap` 的 key。

### 查询

- `is_one(&self, index: usize) -> bool`

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

```rust
ExecutionResult::from_counts(
    task_id: String,
    qubits: Vec<Qubit>,
    shots: usize,
    num_qubits: usize,
    backend: Option<String>,
    counts: HashMap<Outcome, usize>,
) -> ExecutionResult
```

说明：

- `from_counts` 依次应用构造、`start`、`finish`、`calc_probabilities`，直接得到已完成的结果。

### 生命周期方法

- `start(&mut self, t: Option<OffsetDateTime>) -> &mut Self`
- `finish(&mut self, counts: HashMap<Outcome, usize>, t: Option<OffsetDateTime>) -> &mut Self`
- `fail(&mut self, msg: String, code: i32)`
- `cancel(&mut self)`
- `calc_probabilities(&mut self) -> &mut Self`

说明：

- `finish` 写入计数时清除已计算的概率；`calc_probabilities` 按已记录计数的总和归一化，计数总和为 0 时不产生概率。

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

说明：

- `qubits[i]` 对应比特串第 `i` 位（权重 `2^i`）；比特串按最高位在左书写，字符串顺序与 `qubits` 顺序相反。

## 示例

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
use std::collections::HashMap;

// 比特串按最高位在左书写：位 2 在左，位 0 在右。
let outcome = Outcome::from_indices(3, [0, 2]);
assert!(outcome.is_one(0));
assert!(!outcome.is_one(1));
assert_eq!(outcome.to_bitstring(3), "101");
assert_eq!(Outcome::from_bitstring("101").unwrap(), outcome);
assert!(Outcome::from_bitstring("10a").is_err());

let mut result = ExecutionResult::new(
    "task-1".to_string(),
    vec![Qubit::new(0), Qubit::new(1)],
    1000,
    2,
    Some("sim".to_string()),
    None,
);

assert_eq!(result.status(), &Status::Queued);
result.start(None);
assert_eq!(result.status(), &Status::Running);

let mut counts = HashMap::new();
counts.insert(Outcome::from_bitstring("00").unwrap(), 750);
counts.insert(Outcome::from_bitstring("11").unwrap(), 250);
result.finish(counts, None).calc_probabilities();

assert_eq!(result.status(), &Status::Completed);
assert_eq!(result.counts().len(), 2);
assert_eq!(result.probabilities().as_ref().unwrap().len(), 2);

let done = ExecutionResult::from_counts(
    "task-2".to_string(),
    vec![Qubit::new(0), Qubit::new(1)],
    1000,
    2,
    Some("sim".to_string()),
    HashMap::from([(Outcome::from_bitstring("01").unwrap(), 400)]),
);
assert_eq!(done.status(), &Status::Completed);
assert!(done.counts().contains_key(&Outcome::from_bitstring("01").unwrap()));
assert!(done.probabilities().is_some());
```
