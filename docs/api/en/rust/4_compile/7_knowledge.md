# Knowledge

`cqlib_core::compile::knowledge` provides compilation knowledge rules, a validated rule library and structural matching. A knowledge rule describes an adjacent operation pattern, optional symbolic parameter constraints and the operation sequence that replaces that pattern, and it is the source of rules for knowledge rule rewriting and gate decomposition.

## Import

```rust
use cqlib_core::compile::knowledge::{
    ConcreteOperationView, MatchBindings, MatchError, MatchedReplacement, RuleId, RuleKind,
    RuleLibrary, RuleLibraryError, RuleMetadata, conditions_hold, instantiate_target,
    match_rule_item, rule_matches_operations,
};
use cqlib_core::compile::knowledge::rule::{Condition, Rule, RuleItem, RuleValidationError};
```

---

## Rule construction

Qubit labels inside a rule are non-negative integers and must be contiguous starting from zero. They are placeholders: the label `0` can bind to any concrete `Qubit` during matching.

### RuleItem

A single operation in a rule pattern or in a replacement target.

- `RuleItem::standard(gate: StandardGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self`
- `RuleItem::mc_gate(gate: MCGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self`
- `symbols(&self) -> HashSet<String>`: the symbols appearing in the item.
- `validate(&self) -> Result<(), RuleValidationError>`: structural validation.
- `equivalent_to(&self, other: &Self) -> bool`: item equivalence test.

### Condition

Symbolic parameter constraints for rule matching:

```rust
pub enum Condition {
    Eq(Parameter, Parameter),
    EqMod(Parameter, Parameter, Parameter),
}
```

- `symbols(&self) -> HashSet<String>`: the symbols appearing in the constraint.

### Rule

- `Rule::new(name: &str, ops: Vec<RuleItem>, target: Vec<RuleItem>) -> Rule`: construct a rule; conditions are attached by subsequent setters or equivalent methods.
- `validate(&self) -> Result<(), RuleValidationError>`
- `num_qubits(&self) -> usize`: the number of qubit labels involved in the rule.
- `collect_free_symbols(&self) -> HashSet<String>`
- `operation_qubits(&self) -> BTreeSet<u32>`, `target_qubits(&self) -> BTreeSet<u32>`
- `operation_symbols`, `target_symbols`, `condition_symbols`: the symbol set of each part.

---

## RuleKind

Rule categories:

```rust
pub enum RuleKind {
    Simplify,      // 化简
    Cancel,        // 抵消
    Merge,         // 合并
    Commute,       // 显式对易（A; B -> B; A）
    Decompose,     // 分解或降级
    Canonicalize,  // 规范化
    HardwareNative,// 硬件原生
    Other,
}
```

---

## RuleLibrary

The rule library, responsible for loading, validating, classifying and querying rules.

- `RuleLibrary::new() -> Self`
- `RuleLibrary::builtin_rules() -> Result<&'static RuleLibrary, RuleLibraryError>`
- `RuleLibrary::from_rules(rules: Vec<Rule>, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `RuleLibrary::from_dsl_str(source: &str, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `RuleLibrary::from_dsl_file(path: impl AsRef<Path>, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `add_rule(&mut self, rule: Rule, kind: RuleKind) -> Result<RuleId, RuleLibraryError>`
- `extend_rules(&mut self, rules: Vec<Rule>, kind: RuleKind) -> Result<Vec<RuleId>, RuleLibraryError>`
- `len(&self) -> usize`, `is_empty(&self) -> bool`
- `rules(&self) -> &[Rule]`
- `get(&self, id: RuleId) -> Option<&Rule>`, `metadata(&self, id: RuleId) -> Option<&RuleMetadata>`
- `id_by_name(&self, name: &str) -> Option<RuleId>`, `get_by_name(&self, name: &str) -> Option<&Rule>`, `contains(&self, name: &str) -> bool`
- `candidates_for_first_instruction(&self, instruction: &Instruction) -> Result<&[RuleId], RuleLibraryError>`
- `rules_by_kind(&self, kind: RuleKind) -> &[RuleId]`
- `filter_rule_ids_by_instruction_keys(&self, op_instructions: &[Instruction], target_instructions: &[Instruction]) -> Result<Vec<RuleId>, RuleLibraryError>`

### RuleId

An index of a rule within the library; the tuple struct `RuleId(index)` supports comparison and hashing.

### RuleMetadata

```rust
pub struct RuleMetadata {
    pub id: RuleId,
    pub kind: RuleKind,
    pub pattern_len: usize,
    pub rewrite_len: usize,
    pub qubit_count: usize,
    pub first_instruction: Instruction,
    pub cost_delta: isize,
    pub has_conditions: bool,
}
```

---

## Structural matching

### MatchBindings

The match binding result:

- `MatchBindings::new() -> Self`
- `qubits(&self) -> &HashMap<u32, Qubit>`: the binding from rule qubit labels to concrete qubits.
- `qubit(&self, rule_qubit: u32) -> Option<Qubit>`
- `params(&self) -> &HashMap<String, Parameter>`: symbolic parameter bindings.
- `param(&self, symbol: &str) -> Option<&Parameter>`

### Matching functions

- `match_rule_item(item: &RuleItem, concrete: ConcreteOperationView<'_>, bindings: &mut MatchBindings) -> Result<bool, MatchError>`: match a single rule item against a concrete operation view, updating the bindings in place.
- `conditions_hold(conditions: Option<&[Condition]>, bindings: &MatchBindings) -> bool`
- `instantiate_target(target: &[RuleItem], bindings: &MatchBindings) -> Result<Vec<MatchedReplacement>, MatchError>`
- `rule_matches_operations(rule: &Rule, operations: &[ConcreteOperationView<'_>]) -> Result<Option<MatchBindings>, MatchError>`: a non-match returns `Ok(None)`.

The matching functions return `MatchError`; common variants include `MatchError::UnsupportedRuleInstruction` and others.

---

## Raises

- `RuleLibraryError`: rule library construction or validation failure.
- `RuleValidationError`: rule structural validation failure.
- `MatchError`: matching failure.
