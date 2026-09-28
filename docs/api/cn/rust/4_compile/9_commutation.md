# Commutation

`cqlib_core::compile::commutation` 提供量子操作交换的保守证明。返回 `None` 表示配置的证明源无法建立对易性，并不证明两个操作不对易。

## 导入

```rust
use cqlib_core::compile::commutation::{
    Commutation, CommutationChecker, CommutationConfig, CommutationResult,
    algebraic_commutation, check_commutation,
};
```

---

## 函数

### `check_commutation(lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params) -> CommutationResult`

使用共享内置检查器判断两个操作是否对易。参数分别为左右两侧的 `&Instruction`、`&[Qubit]`、`&[Parameter]`。

### `algebraic_commutation(lhs_inst, lhs_qubits, lhs_params, rhs_inst, rhs_qubits, rhs_params) -> CommutationResult`

只使用符号代数证明源（含元数校验、恒等、同操作、支撑不相交等事实）判断对易。返回 `None` 保持结果保守，允许规则或矩阵回退继续执行。

### CommutationResult

`pub type CommutationResult = Option<Commutation>;`

---

## Commutation

对易证明结果：

```rust
pub enum Commutation {
    Exact,                              // 精确对易
    UpToGlobalPhase(Parameter),         // 相差全局相位（lhs * rhs = exp(i*phase) * rhs * lhs）
}
```

方法：

- `is_exact(&self) -> bool`：是否为精确对易，不引入全局相位。
- `phase(&self) -> Parameter`：证明携带的相位，精确对易时为 `0.0`。

---

## CommutationConfig

对易检查器配置。

```rust
pub struct CommutationConfig {
    pub enable_rule_oracle: bool,
    pub enable_matrix_fallback: bool,
    pub max_matrix_qubits: usize,
}
```

- `enable_rule_oracle`：启用来自编译器知识库的显式 `A; B -> B; A` 规则匹配。
- `enable_matrix_fallback`：启用小规模局部矩阵比较作为具体门的回退。
- `max_matrix_qubits`：矩阵回退允许的最大支撑比特数，作用于两操作支撑的排序并集，而非每个操作单独计算。

实现 `Default`：`enable_rule_oracle=true`、`enable_matrix_fallback=true`、`max_matrix_qubits=4`。

---

## CommutationChecker

可复用的对易检查器，证明顺序为：结构事实 → 支持门族的符号代数 → 显式知识库对易规则（启用时）→ 具体局部矩阵比较（启用且规模足够小时）。

- `CommutationChecker::builtin() -> Self`：内置规则与默认配置。
- `CommutationChecker::with_config(config: CommutationConfig) -> Self`：内置规则与显式配置。
- `CommutationChecker::from_library(library: &RuleLibrary, config: CommutationConfig) -> Self`：使用规则库中的对易规则参与证明。
- `config(&self) -> &CommutationConfig`
- `check(&self, lhs_inst: &Instruction, lhs_qubits: &[Qubit], lhs_params: &[Parameter], rhs_inst: &Instruction, rhs_qubits: &[Qubit], rhs_params: &[Parameter]) -> CommutationResult`

---

## 注意事项

- 证明结果保守：`None` 只代表无法证明，不代表不对易。
- 参数比较是符号且容差的，可证明相等的表达式被视为同一应用，即使语法表示不同。
- 经典控制流相关操作不在本检查器的证明范围内。
- 带符号参数的操作参与证明时保持符号参数不变。
