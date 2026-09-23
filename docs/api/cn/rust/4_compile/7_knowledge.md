# Knowledge

`cqlib_core::compile::knowledge` 提供编译知识规则、验证后的规则库与结构匹配能力。知识规则描述一个相邻操作模式、可选符号参数约束以及替换该模式的操作序列，是知识规则重写与门分解的规则来源。

## 导入

```rust
use cqlib_core::compile::knowledge::{
    ConcreteOperationView, MatchBindings, MatchError, MatchedReplacement, RuleId, RuleKind,
    RuleLibrary, RuleLibraryError, RuleMetadata, conditions_hold, instantiate_target,
    match_rule_item, rule_matches_operations,
};
use cqlib_core::compile::knowledge::rule::{Condition, Rule, RuleItem, RuleValidationError};
```

---

## 规则构造

规则内量子比特标签是非负整数且必须从零开始连续排列。它们是占位符：标签 `0` 在匹配时可以绑定任意具体 `Qubit`。

### RuleItem

规则模式或替换目标中的单条操作。

- `RuleItem::standard(gate: StandardGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self`
- `RuleItem::mc_gate(gate: MCGate, qubits: &[u32], params: Vec<ParameterValue>) -> Self`
- `symbols(&self) -> HashSet<String>`：条目中出现的符号。
- `validate(&self) -> Result<(), RuleValidationError>`：结构校验。
- `equivalent_to(&self, other: &Self) -> bool`：条目等价判断。

### Condition

规则匹配的符号参数约束：

```rust
pub enum Condition {
    Eq(Parameter, Parameter),
    EqMod(Parameter, Parameter, Parameter),
}
```

- `symbols(&self) -> HashSet<String>`：约束中出现的符号。

### Rule

- `Rule::new(name: &str, ops: Vec<RuleItem>, target: Vec<RuleItem>) -> Rule`：构造规则；条件由后续 setter 或等价方法附加。
- `validate(&self) -> Result<(), RuleValidationError>`
- `num_qubits(&self) -> usize`：规则涉及比特标签数。
- `collect_free_symbols(&self) -> HashSet<String>`
- `operation_qubits(&self) -> BTreeSet<u32>`、`target_qubits(&self) -> BTreeSet<u32>`
- `operation_symbols`、`target_symbols`、`condition_symbols`：各部分的符号集合。

---

## RuleKind

规则类别：

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

规则库，负责规则的加载、验证、分类与查询。

- `RuleLibrary::new() -> Self`
- `RuleLibrary::builtin_rules() -> Result<&'static RuleLibrary, RuleLibraryError>`
- `RuleLibrary::from_rules(rules: Vec<Rule>, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `RuleLibrary::from_dsl_str(source: &str, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `RuleLibrary::from_dsl_file(path: impl AsRef<Path>, kind: RuleKind) -> Result<Self, RuleLibraryError>`
- `add_rule(&mut self, rule: Rule, kind: RuleKind) -> Result<RuleId, RuleLibraryError>`
- `extend_rules(&mut self, rules: Vec<Rule>, kind: RuleKind) -> Result<Vec<RuleId>, RuleLibraryError>`
- `len(&self) -> usize`、`is_empty(&self) -> bool`
- `rules(&self) -> &[Rule]`
- `get(&self, id: RuleId) -> Option<&Rule>`、`metadata(&self, id: RuleId) -> Option<&RuleMetadata>`
- `id_by_name(&self, name: &str) -> Option<RuleId>`、`get_by_name(&self, name: &str) -> Option<&Rule>`、`contains(&self, name: &str) -> bool`
- `candidates_for_first_instruction(&self, instruction: &Instruction) -> Result<&[RuleId], RuleLibraryError>`
- `rules_by_kind(&self, kind: RuleKind) -> &[RuleId]`
- `filter_rule_ids_by_instruction_keys(&self, op_instructions: &[Instruction], target_instructions: &[Instruction]) -> Result<Vec<RuleId>, RuleLibraryError>`

### RuleId

库内规则索引，`RuleId(index)` 元组结构，支持比较与哈希。

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

## 结构匹配

### MatchBindings

匹配绑定结果：

- `MatchBindings::new() -> Self`
- `qubits(&self) -> &HashMap<u32, Qubit>`：规则比特标签到具体比特的绑定。
- `qubit(&self, rule_qubit: u32) -> Option<Qubit>`
- `params(&self) -> &HashMap<String, Parameter>`：符号参数绑定。
- `param(&self, symbol: &str) -> Option<&Parameter>`

### 匹配函数

- `match_rule_item(item: &RuleItem, concrete: ConcreteOperationView<'_>, bindings: &mut MatchBindings) -> Result<bool, MatchError>`：把单条规则条目匹配到一个具体操作视图，就地更新绑定。
- `conditions_hold(conditions: Option<&[Condition]>, bindings: &MatchBindings) -> bool`
- `instantiate_target(target: &[RuleItem], bindings: &MatchBindings) -> Result<Vec<MatchedReplacement>, MatchError>`
- `rule_matches_operations(rule: &Rule, operations: &[ConcreteOperationView<'_>]) -> Result<Option<MatchBindings>, MatchError>`：不匹配返回 `Ok(None)`。

匹配函数返回 `MatchError`，常见变体包括 `MatchError::UnsupportedRuleInstruction` 等。

---

## 异常情况

- `RuleLibraryError`：规则库构造或验证失败。
- `RuleValidationError`：规则结构校验失败。
- `MatchError`：匹配失败。
