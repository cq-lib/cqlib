# Transform

`cqlib_core::compile::transform` 提供可复用的编译 pass。所有 pass 实现 `Transformer` trait，返回 `TransformOutcome`，不修改输入线路。端到端 `compile()` 自动编排这些 pass；需要单独观察或调试某个阶段时可以直接调用。

## 导入

```rust
use cqlib_core::compile::transform::{
    CircuitAnalysis, CanonicalizeConfig, CanonicalizeResult, Canonicalizer,
    canonicalize_circuit, CommutativeCancellation, KnowledgeRewriteResult, KnowledgeRewriteStats,
    KnowledgeRewriter, LowerToRoutingBasis, OptimizeOneQubitRuns, RewriteConfig, RewriteMode,
    TransformOutcome, Transformer, rewrite_circuit,
};
```

---

## Transformer / TransformOutcome

transform 的统一 trait 与结果对象。

```rust
pub trait Transformer {
    fn name(&self) -> &'static str;
    fn transform(
        &self,
        circuit: &Circuit,
        analysis: Option<&CircuitAnalysis>,
    ) -> Result<TransformOutcome, CompilerError>;
}
```

`transform` 的 `analysis` 参数为可选预计算线路分析：传入可避免跨工作流阶段重复的结构扫描，`None` 时 transform 自行推导所需事实。`TransformOutcome` 方法：

- `changed(&self) -> bool`：该 pass 是否改变了线路。
- `into_circuit(self, original: &Circuit) -> Circuit`：取出输出线路；`Unchanged` 时与输入等价（核心工作流应直接匹配枚举以保持零拷贝）。

---

## CircuitAnalysis

线路特征快照，供工作流决定执行哪些 pass。字段公开：

```rust
pub struct CircuitAnalysis {
    pub has_classical_data: bool,
    pub has_classical_control: bool,
    pub has_measurement: bool,
    pub has_classical_values: bool,
    pub has_classical_vars: bool,
    pub has_runtime_classical: bool,
    pub needs_classical_handle_preservation: bool,
    pub has_circuit_gate_definitions: bool,
    pub has_unitary_circuit_definitions: bool,
    pub has_unitary_gates: bool,
    pub has_mc_gates: bool,
}
```

- `CircuitAnalysis::analyze(circuit: &Circuit) -> Self`：分析线路并返回快照。

---

## Canonicalizer / canonicalize_circuit

规范化 pass，统一指令形式、折叠全局相位、消除空操作。

### CanonicalizeConfig

字段私有，通过构建器方法配置：

- `CanonicalizeConfig::new() -> Self`：生产默认配置。
- `CanonicalizeConfig::production() -> Self`：生产默认配置（同 `new`）。
- `with_round_limit(self, round_limit: u8) -> Self`：最大规范化轮数。
- `recurse_control_flow(self, enabled: bool) -> Self`：是否递归进入控制流块。
- 其余 `fold_gphase`、`canonicalize_instruction_form`、`drop_noops`、`canonicalize_barriers` 均有同名构建器方法。

### Canonicalizer

- `Canonicalizer::new(config: CanonicalizeConfig) -> Self`
- `Canonicalizer::production() -> Self`：生产默认配置。
- `config(&self) -> &CanonicalizeConfig`
- `run(&self, circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError>`
- 实现 `Transformer` 与 `Default`。

### CanonicalizeResult

```rust
pub struct CanonicalizeResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub rounds: u8,
}
```

### canonicalize_circuit(circuit: &Circuit) -> Result<CanonicalizeResult, CompilerError>

函数式入口，使用生产默认配置。

---

## KnowledgeRewriter / rewrite_circuit

基于知识规则的模式重写 pass，是编译管线逻辑优化的核心。

### RewriteMode

```rust
pub enum RewriteMode {
    Optimize,
    Lowering,
}
```

- `Optimize`：保守优化，接受的替换必须严格改善局部代价。
- `Lowering`：显式降级，分解与硬件原生规则可以局部扩张。

### RewriteConfig

字段私有，通过构建器方法配置：

- `RewriteConfig::new() -> Self`：生产默认配置。
- `RewriteConfig::production() -> Self`：生产优化配置。
- `RewriteConfig::lowering() -> Self`：降级配置。
- `with_max_rounds(self, max_rounds: u8) -> Self`
- `with_max_window_ops(self, max_window_ops: usize) -> Self`
- `with_max_pattern_len(self, max_pattern_len: usize) -> Self`
- `recurse_control_flow(self, enabled: bool) -> Self`
- `skip_labeled_ops(self, enabled: bool) -> Self`
- `with_enabled_kinds(self, kinds: Vec<RuleKind>) -> Self`
- `with_mode(self, mode: RewriteMode) -> Self`
- `with_target_instructions(self, instructions: Vec<Instruction>) -> Self`
- `try_with_target_instructions(self, instructions: Vec<Instruction>) -> Result<Self, CompilerError>`
- `enabled_kinds(&self) -> &[RuleKind]`
- `target_instruction_basis(&self) -> Option<Vec<Instruction>>`

### KnowledgeRewriter

- `KnowledgeRewriter::new(config: RewriteConfig) -> Self`
- `KnowledgeRewriter::production() -> Self`
- `KnowledgeRewriter::lowering() -> Self`
- `config(&self) -> &RewriteConfig`
- `run(&self, circuit: &Circuit) -> Result<KnowledgeRewriteResult, CompilerError>`
- 实现 `Transformer`。

### KnowledgeRewriteResult

```rust
pub struct KnowledgeRewriteResult {
    pub circuit: Circuit,
    pub changed: bool,
    pub stats: KnowledgeRewriteStats,
}
```

### KnowledgeRewriteStats

```rust
pub struct KnowledgeRewriteStats {
    pub rounds_executed: u8,
    pub rules_applied: usize,
    pub changed_sequences: usize,
    pub reached_fixpoint: bool,
}
```

### rewrite_circuit(circuit: &Circuit, config: &RewriteConfig) -> Result<KnowledgeRewriteResult, CompilerError>

函数式入口。

---

## OptimizeOneQubitRuns

单比特连续门串优化。

- `OptimizeOneQubitRuns::logical() -> Self`：目标中立的优化器，使用严格逻辑代价。
- `OptimizeOneQubitRuns::basis(target_basis: Vec<Instruction>) -> Result<Self, CompilerError>`：按精确降级到目标门集后的代价优化。
- 实现 `Transformer`。

---

## CommutativeCancellation

基于对易性的自反门抵消 pass。

- `CommutativeCancellation::new() -> Self`
- 实现 `Transformer`。

---

## LowerToRoutingBasis

路由门集降级 pass，把线路转换为适合路由阶段处理的形式。

- `LowerToRoutingBasis::new(preferred_basis: Option<Vec<Instruction>>) -> Self`
- 实现 `Transformer`。

---

## 示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::compile::transform::{CanonicalizeConfig, Canonicalizer, Transformer};

let mut circuit = Circuit::new(1);
circuit.i(Qubit::new(0)).unwrap();

let outcome = Canonicalizer::new(CanonicalizeConfig::production())
    .transform(&circuit, None)
    .unwrap();
assert!(outcome.changed());
```
