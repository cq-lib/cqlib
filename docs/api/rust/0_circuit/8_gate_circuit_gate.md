# CircuitGate / FrozenCircuit

- `cqlib_core::circuit::CircuitGate`
- `cqlib_core::circuit::gate::FrozenCircuit`

```rust
use cqlib_core::circuit::{Circuit, CircuitGate};
use cqlib_core::circuit::gate::FrozenCircuit;
```

`FrozenCircuit` 和 `CircuitGate` 用于在 Rust core 中表达“由一段线路定义的复合门”。其中，`FrozenCircuit` 是不可变线路定义，保存子线路的结构和操作序列；`CircuitGate` 则是由该定义构成的可复用门对象，可以像普通门一样被追加到其它线路中。

---

## `FrozenCircuit`

`FrozenCircuit` 是不可变线路定义，用于保存一个已经构造完成的 `Circuit`。创建后，内部线路作为门定义使用，不应再被外部修改。

```rust
pub fn new(circuit: Circuit) -> Self
pub fn circuit(&self) -> &Circuit
pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError>
```

### 创建不可变线路定义

`FrozenCircuit::new(circuit)` 会移动传入的 `Circuit`，并将其保存为不可变定义。由于传入线路被移动，调用者不能再继续通过原变量修改这段线路，从而避免复合门定义在复用过程中发生变化。

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let mut inner = Circuit::new(2);
inner.h(Qubit::new(0))?;
inner.cx(Qubit::new(0), Qubit::new(1))?;

let frozen = FrozenCircuit::new(inner);

assert_eq!(frozen.circuit().num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### 读取内部线路

```rust
pub fn circuit(&self) -> &Circuit
```

`circuit()` 返回内部不可变线路引用。该接口适合用于查看定义中的量子比特数量、操作序列、符号参数或进行编译分析。

### 符号矩阵缓存

```rust
pub fn symbolic_matrix(&self) -> Result<Arc<SymbolicMatrix>, CircuitError>
```

`symbolic_matrix()` 用于计算并返回内部线路的符号矩阵。该方法通常会在首次调用时计算符号矩阵，并将结果缓存起来；后续调用会复用缓存结果，以减少重复计算开销。

---

## `CircuitGate::new`

```rust
pub fn new(name: impl Into<String>, circuit: FrozenCircuit) -> Result<Self, CircuitError>
```

`CircuitGate::new(name, circuit)` 根据一个 `FrozenCircuit` 创建复合门定义。该方法会自动使用冻结线路中的自由符号作为调用签名。

```rust
use cqlib_core::circuit::{Circuit, CircuitGate, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let mut inner = Circuit::new(1);
inner.h(Qubit::new(0))?;

let gate = CircuitGate::new("HBlock", FrozenCircuit::new(inner))?;

assert_eq!(gate.name(), "HBlock");
assert_eq!(gate.num_qubits(), 1);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，`HBlock` 是一个由单量子比特 `H` 门线路定义的复合门。由于内部线路没有符号参数，因此该复合门的参数数量为 0。

---

## `CircuitGate::with_signature`

```rust
pub fn with_signature(
    name: impl Into<String>,
    circuit: FrozenCircuit,
    params: impl IntoIterator<Item = String>,
) -> Result<Self, CircuitError>
```

`with_signature()` 用于显式声明复合门的调用参数签名。

---

## 属性方法

`CircuitGate` 提供以下常用查询接口：

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `name()` | `&str` | 返回复合门名称。 |
| `num_qubits()` | `usize` | 返回该复合门应用时需要的量子比特数量。 |
| `num_params()` | `usize` | 返回调用签名中的位置参数数量。 |
| `signature_params()` | `&IndexSet<String>` | 返回复合门调用签名中的参数列表，顺序具有绑定语义。 |
| `used_symbols()` | `&IndexSet<String>` | 返回内部线路实际引用的符号集合。 |
| `symbols()` | `IndexSet<String>` | 返回 `used_symbols()` 的克隆。 |
| `circuit()` | `Arc<FrozenCircuit>` | 返回复合门的冻结线路定义。 |
| `symbolic_matrix()` | `Result<Arc<SymbolicMatrix>, CircuitError>` | 返回内部定义的符号矩阵，通常会复用缓存。 |

---

## 参数签名与绑定语义

`CircuitGate` 的参数绑定采用位置参数语义。也就是说，调用复合门时传入的第 `i` 个参数，会替换 `signature_params()[i]` 对应的内部符号。

例如，如果签名为：

```text
["theta", "phi"]
```

那么调用时传入：

```text
[alpha, beta]
```

表示：

```text
theta -> alpha
phi   -> beta
```

这种替换是同时进行的，而不是按顺序逐步替换。因此，对于 `a -> b`、`b -> a` 这类互换场景，不会因为替换顺序而产生中间冲突。

---

## 参数化复合门示例

下面的示例构造了一个带符号参数的子线路，并将其封装为 `CircuitGate`。外层线路调用该复合门时，可以传入新的参数表达式替换内部符号。

```rust
use cqlib_core::circuit::{Circuit, CircuitGate, Parameter, ParameterValue, Qubit};
use cqlib_core::circuit::gate::FrozenCircuit;

let theta = Parameter::symbol("theta");

let mut inner = Circuit::new(1);
inner.rx(Qubit::new(0), theta)?;

let gate = CircuitGate::new("RxBlock", FrozenCircuit::new(inner))?;

let mut outer = Circuit::new(1);
outer.circuit_gate(
    gate,
    vec![Qubit::new(0)],
    vec![ParameterValue::from("alpha")],
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在这个例子中，外层调用参数 `alpha` 会替换复合门定义中的内部符号 `theta`。如果内部线路包含多个符号，则调用时传入的参数数量和顺序必须与 `signature_params()` 保持一致。

---

## 反门

```rust
pub fn inverse(&self) -> Result<Self, CircuitError>
```

`inverse()` 返回一个新的 `CircuitGate`，其底层线路为原冻结线路逐操作反演并逆序排列后的结果。默认情况下，新门名称会在原名称后添加 `_dg` 后缀，并保留原有参数签名。

```rust
let inv = gate.inverse()?;
assert_eq!(inv.name(), "HBlock_dg");

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 追加到 `Circuit`

`CircuitGate` 可以通过 `Circuit::circuit_gate()` 追加到外层线路中。

```rust
circuit.circuit_gate(gate, qubits, params)?;
```

其中：

- `gate` 是要应用的复合门；
- `qubits` 是外层线路中实际作用的量子比特列表；
- `params` 是外层调用时提供的位置参数列表。

```rust
use cqlib_core::circuit::{Circuit, ParameterValue, Qubit};

let mut outer = Circuit::new(1);

outer.circuit_gate(
    gate,
    vec![Qubit::new(0)],
    Vec::<ParameterValue>::new(),
)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 分解与矩阵分析

由于 `CircuitGate` 保留内部线路结构，因此它可以在需要时被展开或用于矩阵分析。

典型流程包括：

- 使用 `Circuit::decompose()` 将复合门展开为内部基础操作；
- 使用 `CircuitGate::symbolic_matrix()` 获取内部定义的符号矩阵；
- 使用 `Circuit::to_matrix()` 在外层线路中验证整体矩阵行为；
- 使用 `inverse()` 生成对应反门。

与黑盒矩阵门相比，`CircuitGate` 的优势在于它保留了子线路的操作结构，因此更适合编译器分析、门分解、资源统计和可视化。

---

## 与 `UnitaryGate` 的区别

| 类型 | 适合场景 | 特点 |
| --- | --- | --- |
| `CircuitGate` | 将一段子线路作为复合门复用 | 保留线路结构，可分解、可反演、可缓存符号矩阵。 |
| `UnitaryGate::with_matrix()` | 只关心黑盒数值矩阵 | 不保留内部线路结构，适合固定自定义矩阵。 |
| `UnitaryGate::with_symbolic_matrix()` | 只关心黑盒符号矩阵 | 适合参数化自定义矩阵门。 |
| `UnitaryGate::with_circuit()` | 希望以 custom unitary 形式保存 circuit-backed 定义 | 保留 circuit-backed 定义，但语义上更偏自定义酉门。 |

---

## 签名与内部符号

`signature_params()` 和 `used_symbols()` 用于区分复合门的外部调用接口和内部实现细节。

| 概念 | 说明 |
| --- | --- |
| `signature_params()` | 调用者必须按顺序提供的位置参数列表。 |
| `used_symbols()` | 内部线路实际引用的自由符号集合。 |
| `num_params()` | 等于签名参数数量，而不一定等于内部实际使用符号数量。 |
