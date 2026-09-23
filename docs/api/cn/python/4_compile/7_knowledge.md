# Knowledge

`cqlib.compile.knowledge` 提供编译知识规则、验证后的规则库与结构匹配能力。知识规则描述一个相邻操作模式、可选符号参数约束以及替换该模式的操作序列，是知识规则重写与门分解的规则来源。

## 导入

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

## 规则构造

规则内量子比特标签是非负整数且必须从零开始连续排列。它们是占位符：标签 `0` 在匹配时可以绑定任意具体 `Qubit`。

### RuleItem

规则模式或替换目标中的单条操作。

#### 静态方法

- `RuleItem.standard(gate, qubits)`：标准门条目，保留绑定在门上的参数。`qubits` 为规则内比特标签列表。
- `RuleItem.mc_gate(gate, qubits)`：多控制门条目，保留绑定参数。

#### 属性与方法

- `instruction -> Instruction`：条目指令。
- `qubits -> list[int]`：规则内比特标签。
- `params -> list[...]`：参数。
- `symbols() -> set[str]`：条目中出现的符号。
- `validate()`：校验条目，非法时抛出 `ValueError`。
- `equivalent_to(other) -> bool`：条目等价判断。

### Condition

规则匹配的符号参数约束。

#### 静态方法

- `Condition.equal(lhs, rhs)`：`lhs == rhs` 约束。
- `Condition.equal_mod(lhs, rhs, modulus)`：`lhs == rhs (mod modulus)` 约束。

#### 属性

- `kind -> str`：约束类型，取值 `"equal"` 或 `"equal_mod"`。
- `lhs`、`rhs`、`modulus`：约束操作数。

#### 方法

- `symbols() -> set[str]`：约束中出现的符号。

### Rule(name, operations, target, conditions=None)

参数：

- `name` (`str`)：规则名。
- `operations` (`list[RuleItem]`)：被匹配的相邻操作模式。
- `target` (`list[RuleItem]`)：替换目标，可为空列表（表示抵消删除）。
- `conditions` (`list[Condition] | None`)：符号参数约束。

#### 属性与方法

- `name -> str`
- `operations -> list[RuleItem]`
- `conditions -> list[Condition]`
- `target -> list[RuleItem]`
- `num_qubits -> int`：规则涉及比特标签数。
- `validate()`：结构校验，非法时抛出 `ValueError`。
- `verify() -> VerifyResult`：规则验证结果。
- `verify_by_sampling(num_bindings, tolerance) -> VerifyResult`：采样验证，`num_bindings` 必须大于零。
- `needs_sampling_fallback() -> bool`：是否需要采样回退。
- `free_symbols() -> set[str]`
- `operation_qubits() -> set[int]`
- `target_qubits() -> set[int]`

#### VerifyResult

属性：

- `status -> str`：验证状态。
- `passed -> bool`：是否通过。
- `num_bindings -> int`：验证的绑定数量。
- `reason -> str | None`：未通过原因。

规则构造示例：

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

规则类别：

- `RuleKind.simplify()`：化简。
- `RuleKind.cancel()`：抵消。
- `RuleKind.merge()`：合并。
- `RuleKind.commute()`：对易。
- `RuleKind.decompose()`：分解。
- `RuleKind.canonicalize()`：规范化。
- `RuleKind.hardware_native()`：硬件原生。
- `RuleKind.other()`：其他。

属性与方法：

- `name -> str`：类别名。
- `label() -> str`：类别标签。

---

## RuleLibrary

规则库，负责规则的加载、验证、分类与查询。

### 构造与加载

- `RuleLibrary()`：空规则库。
- `RuleLibrary.builtin()`：内置规则库。
- `RuleLibrary.from_rules(rules, kind)`：从规则列表构造，所有规则归入 `kind` 类别。
- `RuleLibrary.from_dsl(source, kind)`：从 `.rule` DSL 字符串解析并构造。
- `RuleLibrary.from_dsl_file(path, kind)`：从 `.rule` DSL 文件解析并构造。

### 修改

- `add_rule(rule, kind) -> RuleId`：添加规则，返回其 ID。
- `extend_rules(rules, kind) -> list[RuleId]`：批量添加规则。

### 查询

- `rules() -> list[Rule]`
- `get(id) -> Rule | None`
- `metadata(id) -> RuleMetadata | None`
- `id_by_name(name) -> RuleId | None`
- `get_by_name(name) -> Rule | None`
- `name in library -> bool`（`__contains__`）
- `len(library) -> int`
- `candidates_for_first_instruction(instruction) -> list[RuleId]`：按首指令筛选候选规则。
- `rules_by_kind(kind) -> list[RuleId]`
- `filter_rule_ids_by_instruction_keys(op_instructions, target_instructions) -> list[RuleId]`：按操作与目标指令键筛选规则 ID。

### RuleId

- `index -> int`：库内索引，支持 `int(rule_id)`。

### RuleMetadata

规则元数据：

- `id -> RuleId`
- `kind -> RuleKind`
- `pattern_len -> int`：模式长度。
- `rewrite_len -> int`：替换长度。
- `qubit_count -> int`：涉及比特数。
- `first_instruction -> Instruction`：首条指令。
- `cost_delta -> int`：代价变化。
- `has_conditions -> bool`：是否带约束。

---

## DSL 读写

### loads(source) -> list[Rule]

从 `.rule` DSL 字符串解析规则列表。

### load(path) -> list[Rule]

从 `.rule` DSL 文件解析规则列表。

### dumps(rules) -> str

把规则列表序列化为 DSL 字符串。

### dump(rules, path)

把规则列表写入 DSL 文件。

示例：

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

## 结构匹配

### MatchBindings

匹配绑定结果。

属性与方法：

- `qubits() -> dict[int, Qubit]`：规则比特标签到具体比特的绑定。
- `qubit(rule_qubit) -> Qubit | None`：单个标签的绑定。
- `params() -> dict[str, Parameter]`：符号参数绑定。
- `param(symbol) -> Parameter | None`：单个符号的绑定。

### rule_matches_operations(rule, operations) -> MatchBindings | None

把规则匹配到相邻 `ValueOperation` 序列；不匹配返回 `None`。

### match_rule_item(item, operation, bindings) -> bool

把单条 `RuleItem` 匹配到一个 `ValueOperation`，就地更新绑定。

### conditions_hold(conditions, bindings) -> bool

检查约束在给定绑定下是否成立。

### instantiate_target(target, bindings) -> list[ValueOperation]

用绑定实例化替换目标。

示例：

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

## 异常情况

- `ValueError`：规则结构非法、DSL 解析失败、规则库构造或验证失败（错误信息带 `Rule library error: ...` 等前缀）、采样验证参数非法。
