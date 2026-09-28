# Commutation

`cqlib_core::compile::commutation` provides conservative proofs of quantum operation commutation. A returned `None` means the configured proof sources cannot establish commutation; it does not prove that the two operations do not commute.

## Import

```rust
use cqlib_core::compile::commutation::{
    Commutation, CommutationChecker, CommutationConfig, CommutationResult,
    algebraic_commutation, check_commutation,
};
```

---

## Functions

### `check_commutation(lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params) -> CommutationResult`

Determine whether two operations commute using the shared built-in checker. The parameters are the `&Instruction`, `&[Qubit]` and `&[Parameter]` of the left and right sides respectively.

### `algebraic_commutation(lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params) -> CommutationResult`

Determine commutation using only symbolic algebraic proof sources (including facts such as arity checks, identity, identical operations and disjoint supports). Returning `None` keeps the result conservative and allows the rule or matrix fallback to proceed.

### CommutationResult

`pub type CommutationResult = Option<Commutation>;`

---

## Commutation

The commutation proof result:

```rust
pub enum Commutation {
    Exact,                              // 精确对易
    UpToGlobalPhase(Parameter),         // 相差全局相位（lhs * rhs = exp(i*phase) * rhs * lhs）
}
```

Methods:

- `is_exact(&self) -> bool`: whether it is exact commutation without introducing a global phase.
- `phase(&self) -> Parameter`: the phase carried by the proof; `0.0` for exact commutation.

---

## CommutationConfig

Commutation checker configuration.

```rust
pub struct CommutationConfig {
    pub enable_rule_oracle: bool,
    pub enable_matrix_fallback: bool,
    pub max_matrix_qubits: usize,
}
```

- `enable_rule_oracle`: enable explicit `A; B -> B; A` rule matching from the compiler knowledge library.
- `enable_matrix_fallback`: enable small-scale local matrix comparison as a fallback for concrete gates.
- `max_matrix_qubits`: the maximum number of support qubits allowed for the matrix fallback; it applies to the sorted union of the supports of the two operations rather than being computed separately for each operation.

Implements `Default`: `enable_rule_oracle=true`, `enable_matrix_fallback=true`, `max_matrix_qubits=4`.

---

## CommutationChecker

A reusable commutation checker. The proof order is: structural facts → symbolic algebra over supported gate families → explicit knowledge-library commutation rules (when enabled) → concrete local matrix comparison (when enabled and the size is small enough).

- `CommutationChecker::builtin() -> Self`: built-in rules with the default configuration.
- `CommutationChecker::with_config(config: CommutationConfig) -> Self`: built-in rules with an explicit configuration.
- `CommutationChecker::from_library(library: &RuleLibrary, config: CommutationConfig) -> Self`: use the commutation rules in the rule library as part of the proof.
- `config(&self) -> &CommutationConfig`
- `check(&self, lhs_inst: &Instruction, lhs_qubits: &[Qubit], lhs_params: &[Parameter], rhs_inst: &Instruction, rhs_qubits: &[Qubit], rhs_params: &[Parameter]) -> CommutationResult`

---

## Notes

- Proof results are conservative: `None` only means that a proof cannot be established, not that the operations do not commute.
- Parameter comparison is symbolic and tolerant; expressions that are provably equal are treated as the same application even when their syntactic representations differ.
- Operations related to classical control flow are outside the proof scope of this checker.
- Operations with symbolic parameters keep their symbolic parameters unchanged when participating in a proof.
