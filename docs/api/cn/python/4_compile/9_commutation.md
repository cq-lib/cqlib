# Commutation

`cqlib.compile.commutation` 提供量子操作交换的保守证明。该模块证明两个具体 `ValueOperation` 对象交换后是否保持线路语义。返回 `None` 表示配置的证明源无法建立对易性，并不证明两个操作不对易。

## 导入

```python
from cqlib.compile.commutation import (
    Commutation,
    CommutationConfig,
    CommutationChecker,
    check_commutation,
    algebraic_commutation,
)
```

---

## 函数

### check_commutation(lhs, rhs) -> Commutation | None

使用共享内置检查器判断两个自包含操作是否对易。

### algebraic_commutation(lhs, rhs) -> Commutation | None

只使用符号代数证明源判断对易。

示例：

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import StandardGate, ValueOperation
from cqlib.compile.commutation import check_commutation

lhs = ValueOperation.from_standard_gate(StandardGate.H, [Qubit(0)])
rhs = ValueOperation.from_standard_gate(StandardGate.X, [Qubit(1)])

proof = check_commutation(lhs, rhs)
assert proof is not None and proof.is_exact()
```

---

## Commutation

对易证明结果。

### 静态方法

- `Commutation.exact()`：精确对易证明。
- `Commutation.up_to_global_phase(phase)`：相差全局相位的对易证明。

### 属性与方法

- `phase -> Parameter`：证明携带的相位。
- `is_exact() -> bool`：是否为精确对易。

---

## CommutationConfig

对易检查器配置。

### CommutationConfig(*, enable_rule_oracle=true, enable_matrix_fallback=true, max_matrix_qubits=4)

参数：

- `enable_rule_oracle` (`bool`)：启用规则预言机（来自知识库的 commute 规则）。
- `enable_matrix_fallback` (`bool`)：启用矩阵回退。
- `max_matrix_qubits` (`int`)：矩阵回退允许的最大比特数。

### 属性

`enable_rule_oracle`、`enable_matrix_fallback`、`max_matrix_qubits`，与构造参数一一对应。

---

## CommutationChecker

可配置的对易检查器。

### 静态方法

- `CommutationChecker.builtin()`：内置规则与默认配置。
- `CommutationChecker.with_config(config)`：内置规则与显式配置。
- `CommutationChecker.from_library(library, config=None)`：使用已加载规则库中的 commute 规则（即 `RuleKind.commute()` 类别）参与证明。`config=None` 时使用默认配置，与 `builtin()` 一致。当 `enable_rule_oracle` 关闭时抛出 `ValueError`，否则规则库会被静默忽略。

### 属性与方法

- `config -> CommutationConfig`：当前配置副本。
- `check(lhs, rhs) -> Commutation | None`：判断两个自包含操作值是否对易。

---

## 注意事项

- 证明结果保守：`None` 只代表无法证明，不代表不对易。
- 经典控制流相关操作不在本检查器的证明范围内。
- 带符号参数的操作参与证明时保持符号参数不变。
