# Knowledge

`cqlib.compile.knowledge` provides compilation knowledge rules, a validated rule library and structural matching capabilities. A knowledge rule describes an adjacent operation pattern, optional symbolic parameter conditions and the operation sequence that replaces the pattern; it is the source of the rules for knowledge rule rewriting and gate decomposition.

## Import

```python
from cqlib.compile.knowledge import (
    Rule,
    RuleItem,
    Condition,
    RuleId,
    RuleKind,
    RuleMetadata,
    RuleLibrary,
    MatchBindings,
    loads,
    load,
    dumps,
    dump,
    match_rule_item,
    conditions_hold,
    instantiate_target,
    rule_matches_operations,
)
```

---

## Rule construction

Qubit labels within a rule are non-negative integers and must be contiguous starting from zero. They are placeholders: label `0` may bind to any concrete `Qubit` during matching.

### RuleItem

A single operation in a rule pattern or replacement target.

#### Static methods

- `RuleItem.standard(gate, qubits)`: a standard gate item, preserving the parameters bound to the gate. `qubits` is the list of qubit labels within the rule.
- `RuleItem.mc_gate(gate, qubits)`: a multi-controlled gate item, preserving the bound parameters.

#### Attributes and methods

- `instruction -> Instruction`: the item instruction.
- `qubits -> list[int]`: the qubit labels within the rule.
- `params -> list[...]`: the parameters.
- `symbols() -> set[str]`: the symbols appearing in the item.
- `validate()`: validate the item; raises `ValueError` when it is invalid.
- `equivalent_to(other) -> bool`: item equivalence test.

### Condition

A symbolic parameter condition for rule matching.

#### Static methods

- `Condition.equal(lhs, rhs)`: the constraint `lhs == rhs`.
- `Condition.equal_mod(lhs, rhs, modulus)`: the constraint `lhs == rhs (mod modulus)`.

#### Attributes

- `kind -> str`: the condition kind, taking the values `"equal"` or `"equal_mod"`.
- `lhs`, `rhs`, `modulus`: the condition operands.

#### Methods

- `symbols() -> set[str]`: the symbols appearing in the condition.

### Rule(name, operations, target, conditions=None)

Parameters:

- `name` (`str`): the rule name.
- `operations` (`list[RuleItem]`): the adjacent operation pattern to be matched.
- `target` (`list[RuleItem]`): the replacement target, which may be an empty list (meaning cancellation and deletion).
- `conditions` (`list[Condition] | None`): the symbolic parameter conditions.

#### Attributes and methods

- `name -> str`
- `operations -> list[RuleItem]`
- `conditions -> list[Condition]`
- `target -> list[RuleItem]`
- `num_qubits -> int`: the number of qubit labels involved in the rule.
- `validate()`: structural validation; raises `ValueError` when it is invalid.
- `verify() -> VerifyResult`: the rule validation result.
- `verify_by_sampling(num_bindings, tolerance) -> VerifyResult`: sampling validation; `num_bindings` must be greater than zero.
- `needs_sampling_fallback() -> bool`: whether a sampling fallback is needed.
- `free_symbols() -> set[str]`
- `operation_qubits() -> set[int]`
- `target_qubits() -> set[int]`

#### VerifyResult

Attributes:

- `status -> str`: the validation status.
- `passed -> bool`: whether it passed.
- `num_bindings -> int`: the number of bindings validated.
- `reason -> str | None`: the reason it did not pass.

Rule construction example:

```python
from cqlib.circuit import StandardGate
from cqlib.compile.knowledge import Rule, RuleItem

cancel_h = Rule(
    "cancel_h",
    operations=[
        RuleItem.standard(StandardGate.H, [0]),
        RuleItem.standard(StandardGate.H, [0]),
    ],
    target=[],
)
cancel_h.validate()
assert cancel_h.verify().passed
```

---

## RuleKind

Rule kinds:

- `RuleKind.simplify()`: simplification.
- `RuleKind.cancel()`: cancellation.
- `RuleKind.merge()`: merging.
- `RuleKind.commute()`: commutation.
- `RuleKind.decompose()`: decomposition.
- `RuleKind.canonicalize()`: canonicalization.
- `RuleKind.hardware_native()`: hardware native.
- `RuleKind.other()`: other.

Attributes and methods:

- `name -> str`: the kind name.
- `label() -> str`: the kind label.

---

## RuleLibrary

Rule library, responsible for loading, validating, classifying and querying rules.

### Construction and loading

- `RuleLibrary()`: an empty rule library.
- `RuleLibrary.builtin()`: the built-in rule library.
- `RuleLibrary.from_rules(rules, kind)`: construct from a rule list; all rules are assigned to the `kind` category.
- `RuleLibrary.from_dsl(source, kind)`: parse and construct from a `.rule` DSL string.
- `RuleLibrary.from_dsl_file(path, kind)`: parse and construct from a `.rule` DSL file.

### Modification

- `add_rule(rule, kind) -> RuleId`: add a rule and return its ID.
- `extend_rules(rules, kind) -> list[RuleId]`: add rules in bulk.

### Queries

- `rules() -> list[Rule]`
- `get(id) -> Rule | None`
- `metadata(id) -> RuleMetadata | None`
- `id_by_name(name) -> RuleId | None`
- `get_by_name(name) -> Rule | None`
- `name in library -> bool` (`__contains__`)
- `len(library) -> int`
- `candidates_for_first_instruction(instruction) -> list[RuleId]`: filter candidate rules by first instruction.
- `rules_by_kind(kind) -> list[RuleId]`
- `filter_rule_ids_by_instruction_keys(op_instructions, target_instructions) -> list[RuleId]`: filter rule IDs by operation and target instruction keys.

### RuleId

- `index -> int`: the index within the library; supports `int(rule_id)`.

### RuleMetadata

Rule metadata:

- `id -> RuleId`
- `kind -> RuleKind`
- `pattern_len -> int`: the pattern length.
- `rewrite_len -> int`: the replacement length.
- `qubit_count -> int`: the number of qubits involved.
- `first_instruction -> Instruction`: the first instruction.
- `cost_delta -> int`: the cost change.
- `has_conditions -> bool`: whether conditions are present.

---

## DSL reading and writing

### loads(source) -> list[Rule]

Parse a rule list from a `.rule` DSL string.

### load(path) -> list[Rule]

Parse a rule list from a `.rule` DSL file.

### dumps(rules) -> str

Serialize a rule list to a DSL string.

### dump(rules, path)

Write a rule list to a DSL file.

Example:

```python
from cqlib.compile.knowledge import loads

rule = loads('''
rule cancel_x {
    match { X 0, X 0 }
    rewrite {}
}
''')[0]
rule.validate()
assert rule.verify().passed
```

---

## Structural matching

### MatchBindings

Match binding result.

Attributes and methods:

- `qubits() -> dict[int, Qubit]`: the binding from rule qubit labels to concrete qubits.
- `qubit(rule_qubit) -> Qubit | None`: the binding of a single label.
- `params() -> dict[str, Parameter]`: the symbolic parameter bindings.
- `param(symbol) -> Parameter | None`: the binding of a single symbol.

### rule_matches_operations(rule, operations) -> MatchBindings | None

Match a rule against an adjacent `ValueOperation` sequence; returns `None` when there is no match.

### match_rule_item(item, operation, bindings) -> bool

Match a single `RuleItem` against one `ValueOperation`, updating the bindings in place.

### conditions_hold(conditions, bindings) -> bool

Check whether the conditions hold under the given bindings.

### instantiate_target(target, bindings) -> list[ValueOperation]

Instantiate the replacement target with the bindings.

Example:

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation
from cqlib.compile.knowledge import loads, rule_matches_operations

rule = loads('''
rule cancel_x {
    match { X 0, X 0 }
    rewrite {}
}
''')[0]

operations = [
    ValueOperation.from_standard_gate(StandardGate.X, [Qubit(3)]),
    ValueOperation.from_standard_gate(StandardGate.X, [Qubit(3)]),
]

bindings = rule_matches_operations(rule, operations)
assert bindings is not None
assert bindings.qubit(0) == Qubit(3)
```

---

## Raises

- `ValueError`: the rule structure is invalid, DSL parsing fails, rule library construction or validation fails (error messages carry prefixes such as `Rule library error: ...`), or the sampling validation parameters are invalid.
