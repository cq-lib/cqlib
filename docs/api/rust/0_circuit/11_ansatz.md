# Ansatz

`cqlib_core::circuit::ansatz`

```rust
use cqlib_core::circuit::ansatz::{
    Ansatz,
    EntanglementTopology,
    TwoLocal,
    BasicEntanglerLayers,
    StronglyEntanglingLayers,
    AngleEncoding,
    BasisEncoding,
    ZFeatureMap,
    IQPFeatureMap,
    ZZFeatureMap,
    PauliFeatureMap,
    QAOAAnsatz,
    EvolutionStrategy,
    PauliEvolutionAnsatz,
};
```

`ansatz` 模块提供一组可复用的参数化量子线路模板，主要用于变分量子算法、量子机器学习、组合优化和 Hamiltonian 时间演化等场景。通过这些模板，用户可以用较少的配置生成结构清晰、参数命名一致的 `Circuit`，避免在算法开发中重复手写常见线路结构。

---

## `Ansatz` trait

`Ansatz` trait 定义了 ansatz 模板的统一接口。实现该 trait 的模板都可以进行配置校验、线路构建、参数数量查询和量子比特数量查询。

```rust
pub trait Ansatz {
    fn validate(&self) -> Result<(), CircuitError> { ... }
    fn build_circuit(&self, prefix: &str) -> Result<Circuit, CircuitError>;
    fn num_parameters(&self) -> usize;
    fn num_qubits(&self) -> usize;
}
```

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `validate()` | `Result<(), CircuitError>` | 检查模板配置是否合法，例如量子比特数量、门 arity、拓扑和 Hamiltonian 是否满足要求。 |
| `build_circuit(prefix)` | `Result<Circuit, CircuitError>` | 根据当前模板配置生成参数化线路。 |
| `num_parameters()` | `usize` | 返回该模板生成线路所需的独立参数数量。 |
| `num_qubits()` | `usize` | 返回该模板作用的量子比特数量。 |

```rust
use cqlib_core::circuit::ansatz::{Ansatz, TwoLocal};

let ansatz = TwoLocal::new(3).reps(2);

ansatz.validate()?;

let circuit = ansatz.build_circuit("theta")?;

assert_eq!(ansatz.num_qubits(), 3);
assert_eq!(circuit.symbols().len(), ansatz.num_parameters());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Builder 风格与参数前缀

`ansatz` 模块中的模板通常采用 builder 风格配置。配置方法会返回配置后的模板对象，使用户可以通过链式调用逐步指定重复层数、旋转门、纠缠门、纠缠拓扑或演化策略。

```rust
use cqlib_core::circuit::StandardGate;
use cqlib_core::circuit::ansatz::{Ansatz, EntanglementTopology, TwoLocal};

let ansatz = TwoLocal::new(4)
    .reps(2)
    .rotation_gates(vec![StandardGate::RY, StandardGate::RZ])
    .entanglement_gate(StandardGate::CX)
    .entanglement(EntanglementTopology::Linear);

let circuit = ansatz.build_circuit("theta")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`build_circuit(prefix)` 中的 `prefix` 用于生成线路中的符号参数名。例如，常见模板会生成类似 `{prefix}_0`、`{prefix}_1` 的参数名称；QAOA 模板会生成带有 `gamma` / `beta` 语义的参数名；Hamiltonian evolution 模板通常使用时间参数名。

在同一个模型中组合多个 ansatz 时，建议为不同模块使用不同前缀，以避免符号名冲突。例如：

```text
encoder_0, encoder_1, ...
ansatz_0, ansatz_1, ...
qaoa_gamma_0, qaoa_beta_0, ...
```

---

## `EntanglementTopology`

```rust
pub enum EntanglementTopology {
    Linear,
    Circular,
    Full,
    Custom(Vec<(usize, usize)>),
}
```

`EntanglementTopology` 用于描述多量子比特模板中的纠缠连接方式。它决定每一层中哪些量子比特之间会添加双量子比特门或多体 Pauli 演化项。

| 拓扑 | 说明 |
| --- | --- |
| `Linear` | 最近邻链式连接，例如 `(0, 1), (1, 2), ...`。 |
| `Circular` | 在 `Linear` 基础上增加首尾连接。 |
| `Full` | 全连接拓扑，任意两个量子比特之间都可连接。 |
| `Custom(Vec<(usize, usize)>)` | 用户显式指定连接对。 |

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `generate_pairs(num_qubits)` | 根据拓扑生成二量子比特连接对。 |
| `generate_k_tuples(k, num_qubits)` | 根据拓扑生成 k-local 作用量子比特组。 |

```rust
use cqlib_core::circuit::ansatz::EntanglementTopology;

let topology = EntanglementTopology::Linear;
let pairs = topology.generate_pairs(4)?;

assert_eq!(pairs, vec![(0, 1), (1, 2), (2, 3)]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `TwoLocal`

`TwoLocal` 是常见的硬件友好 ansatz。它由交替出现的单量子比特旋转层和多量子比特纠缠层组成，适用于 VQE、量子机器学习和通用变分线路构造。

```rust
use cqlib_core::circuit::StandardGate;
use cqlib_core::circuit::ansatz::{Ansatz, EntanglementTopology, TwoLocal};

let ansatz = TwoLocal::new(3)
    .reps(2)
    .rotation_gates(vec![StandardGate::RY, StandardGate::RZ])
    .entanglement_gate(StandardGate::CX)
    .entanglement(EntanglementTopology::Linear);

let circuit = ansatz.build_circuit("theta")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

配置方法如下：

| 方法 | 说明 |
| --- | --- |
| `reps(reps)` | 设置重复层数。 |
| `rotation_gates(gates)` | 设置每层使用的单量子比特参数旋转门。 |
| `entanglement_gate(gate)` | 设置纠缠层使用的双量子比特门。 |
| `entanglement(topology)` | 设置纠缠拓扑。 |
| `skip_final_rotation_layer(skip)` | 设置是否跳过最终旋转层。 |

使用 `TwoLocal` 时，应确保旋转门是单量子比特参数门，纠缠门是合适的多量子比特门。若门 arity 与模板要求不匹配，`validate()` 或 `build_circuit()` 会返回错误。

---

## Feature Maps

Feature map 用于将经典数据映射到量子线路参数中，是量子机器学习和量子核方法中的常见组件。

| 类型 | 说明 |
| --- | --- |
| `AngleEncoding` | 将每个输入特征映射为一个单量子比特旋转角。 |
| `BasisEncoding` | 根据 bitstring 准备计算基态，不引入连续参数。 |
| `ZFeatureMap` | 一阶 Pauli-Z feature map。 |
| `IQPFeatureMap` | IQP 风格对角 feature map。 |
| `ZZFeatureMap` | 二阶 ZZ feature map，常用于量子核方法。 |
| `PauliFeatureMap` | 支持任意 Pauli string 模板的特征映射。 |

```rust
use cqlib_core::circuit::ansatz::{Ansatz, EntanglementTopology, ZZFeatureMap};

let fm = ZZFeatureMap::new(3)
    .reps(2)
    .entanglement(EntanglementTopology::Full);

let circuit = fm.build_circuit("x")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## Layer 模板

除通用 `TwoLocal` 外，`ansatz` 模块还提供常见层状模板，用于快速构造规律性较强的可训练线路。

### 1. `BasicEntanglerLayers`

`BasicEntanglerLayers` 通常由单量子比特旋转层和固定模式的纠缠层组成，适合构造结构简单、参数数量可控的训练线路。

```rust
use cqlib_core::circuit::StandardGate;
use cqlib_core::circuit::ansatz::{Ansatz, BasicEntanglerLayers};

let ansatz = BasicEntanglerLayers::new(4)
    .reps(3)
    .rotation_gate(StandardGate::RY)
    .entanglement_gate(StandardGate::CX);

let circuit = ansatz.build_circuit("w")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 2. `StronglyEntanglingLayers`

`StronglyEntanglingLayers` 通常使用更强的单量子比特旋转和范围化纠缠模式，适合需要更强表达能力的变分模型。

```rust
use cqlib_core::circuit::StandardGate;
use cqlib_core::circuit::ansatz::{Ansatz, StronglyEntanglingLayers};

let ansatz = StronglyEntanglingLayers::new(4)
    .reps(3)
    .entanglement_gate(StandardGate::CX)
    .ranges(vec![1, 2]);

let circuit = ansatz.build_circuit("w")?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `QAOAAnsatz`

`QAOAAnsatz` 用于构造量子近似优化算法（QAOA）的参数化线路。QAOA 在线路结构上交替应用 cost Hamiltonian 和 mixer Hamiltonian 的时间演化。

典型形式可理解为：

```text
U(β, γ) = ∏_l exp(-i β_l H_M) exp(-i γ_l H_C)
```

其中：

- `H_C` 是问题 Hamiltonian，也称 cost operator；
- `H_M` 是 mixer Hamiltonian，默认通常为 X mixer；
- `p = reps` 表示 QAOA 层数；
- 每一层通常包含一个 `γ_l` 和一个 `β_l` 参数，因此 `num_parameters()` 通常为 `2 * reps`。

配置方法如下：

| 方法 | 说明 |
| --- | --- |
| `new(cost_operator)` | 根据 cost Hamiltonian 创建 QAOA ansatz。 |
| `reps(reps)` | 设置 QAOA 层数 `p`。 |
| `mixer(mixer_operator)` | 设置自定义 mixer Hamiltonian。 |
| `initial_state(circuit)` | 设置初态线路。 |
| `evolution_strategy(strategy)` | 设置 Hamiltonian 演化策略。 |

使用 QAOA 时，应确保 cost Hamiltonian、mixer Hamiltonian 和初态线路的量子比特数量一致。若维度不匹配，构建线路时会返回错误。

---

## Hamiltonian Evolution

Hamiltonian evolution 模板用于构造形如 `exp(-i H t)` 的参数化时间演化线路。该类模板常用于模拟量子系统演化、构造 QAOA 子模块，或验证 Pauli Hamiltonian 的门级分解。

### 1. `EvolutionStrategy`

`EvolutionStrategy` 用于指定 Hamiltonian 演化如何转换为量子门序列。

| 变体 / 工厂方法 | 说明 |
| --- | --- |
| `exact()` | 对两两对易的 Hamiltonian 使用精确 Pauli rotation 分解；若非对易，构建时返回错误。 |
| `auto(steps)` | 自动选择策略：对易时使用 exact，非对易时使用一阶 Trotter。 |
| `trotter(mode, steps)` | 显式使用指定 Trotter-Suzuki 分解模式和步数。 |

### 2. `EvolutionInfo`

`EvolutionInfo` 描述实际采用的演化策略和 Hamiltonian 结构信息，通常由 `PauliEvolutionAnsatz::evolution_info()` 返回。

常见字段包括：

- `is_exact`
- `steps`
- `trotter_mode`
- `all_terms_commute`
- `num_terms`

这些信息可用于调试演化分解、记录实验配置，或在报告中说明某个演化线路是否为精确分解。

### 3. `PauliEvolutionAnsatz`

`PauliEvolutionAnsatz` 用于构造 Pauli Hamiltonian 的时间演化线路。该模板生成的线路通常只包含一个时间参数。默认参数名可能为 `{prefix}_t`，也可以通过 `with_time_param_name()` 指定，例如 `tau`。