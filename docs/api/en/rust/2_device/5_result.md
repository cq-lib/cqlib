# Outcome / Status / ExecutionResult

This page covers the types related to quantum task results:

- `Outcome`
- `Status`
- `ExecutionResult`
- `OutcomeError`

## Import

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::result::OutcomeError;
use cqlib_core::device::{ExecutionResult, Outcome, Status};
use std::collections::HashMap;
use time::OffsetDateTime;
```

## OutcomeError

Defined in `cqlib_core::device::result`; not re-exported from the `cqlib_core::device` module root.

Currently contains:

- `OutcomeError::InvalidCharacter(index, ch)`: the bitstring contains an invalid character.

## Outcome

### Construction and conversion

- `Outcome::new(chunks: SmallVec<[u64; 4]>) -> Outcome`
- `Outcome::from_indices(width: usize, indices: impl IntoIterator<Item = usize>) -> Outcome`: construct a result of width `width` whose bits are `1` only at the positions listed in `indices`; indices beyond `width` are ignored.
- `Outcome::from_bitstring(s: &str) -> Result<Outcome, OutcomeError>`
- Implements `FromStr`; `s.parse::<Outcome>()` is equivalent to `Outcome::from_bitstring(s)`, with the same error type `OutcomeError`.
- `to_bitstring(&self, num_qubits: usize) -> String`: output the bitstring with the given width.

Notes:

- The bitstring is always written in standard binary form (most significant bit on the left), while internally it is stored in little-endian blocks.
- The bitstring width is not stored by `Outcome`: parsing `"001"` and parsing `"1"` yield the same `Outcome`. The output width must therefore be specified by the caller through `to_bitstring`; when the requested width is smaller than the stored value, the high bits are truncated.
- `Outcome` implements `Clone + Eq + Hash` and can be used directly as the key of a counts `HashMap`.

### Queries

- `is_one(&self, index: usize) -> bool`

## Status

Enum variants:

- `Queued`
- `Running`
- `Completed`
- `Failed { error_msg: String, error_code: i32 }`
- `Cancelled`

Methods:

- `is_terminal(&self) -> bool`
- `is_success(&self) -> bool`

## ExecutionResult

### Construction

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

Notes:

- `from_counts` applies construction, `start`, `finish` and `calc_probabilities` in order, obtaining an already completed result directly.

### Lifecycle methods

- `start(&mut self, t: Option<OffsetDateTime>) -> &mut Self`
- `finish(&mut self, counts: HashMap<Outcome, usize>, t: Option<OffsetDateTime>) -> &mut Self`
- `fail(&mut self, msg: String, code: i32)`
- `cancel(&mut self)`
- `calc_probabilities(&mut self) -> &mut Self`

Notes:

- `finish` clears the computed probabilities when it writes the counts; `calc_probabilities` normalizes by the sum of the recorded counts, and produces no probabilities when the total count is 0.

### Read methods

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

Notes:

- `qubits[i]` corresponds to bit `i` of the bitstring (weight `2^i`); the bitstring is written with the most significant bit on the left, so the string order is the reverse of the `qubits` order.

## Example

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
