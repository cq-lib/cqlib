# UnitaryGate

- `cqlib_core::circuit::UnitaryGate`
- `cqlib_core::circuit::gate::unitary_gate::UnitaryMatrix`

```rust
use cqlib_core::circuit::UnitaryGate;
```

`UnitaryGate` 用于在 Rust core 中定义标准门集合之外的用户自定义酉门。它适合表示具有明确酉矩阵语义、但不属于 `StandardGate` 枚举的门，例如自定义 oracle、硬件校准门、外部工具生成的黑盒酉矩阵，或需要保留符号矩阵定义的参数化自定义门。

---

## 构造函数

```rust
pub fn new(label: &str, num_qubits: u16, num_params: u16) -> Self
```

`UnitaryGate::new(label, num_qubits, num_params)` 创建一个尚未附加具体定义的自定义酉门对象。

| 参数 | 说明 |
| --- | --- |
| `label` | 门的可读名称，通常用于调试、可视化和 IR 输出。 |
| `num_qubits` | 该门作用的量子比特数量。 |
| `num_params` | 每次应用该门时需要传入的位置参数数量。 |

```rust
use cqlib_core::circuit::UnitaryGate;

let gate = UnitaryGate::new("Oracle", 2, 0);

assert_eq!(gate.label(), "Oracle");
assert_eq!(gate.num_qubits(), 2);
assert_eq!(gate.num_params(), 0);
```

需要注意的是，每次调用 `UnitaryGate::new()` 都会生成新的 UUID。

---

## 属性与查询接口

`UnitaryGate` 提供一组查询方法，用于读取门的元数据和已附加的定义。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `label()` | `&str` | 返回门的可读名称。 |
| `num_qubits()` | `u16` | 返回门作用的量子比特数量。 |
| `num_params()` | `u16` | 返回每次应用该门所需的位置参数数量。 |
| `matrix()` | `Option<&Array2<Complex<f64>>>` | 返回数值矩阵定义；若未使用数值矩阵定义，则返回 `None`。 |
| `symbolic_matrix()` | `Option<&SymbolicMatrix>` | 返回符号矩阵定义。 |
| `matrix_params()` | `Option<&[String]>` | 返回符号矩阵参数名顺序。 |
| `matrix_repr()` | `Option<&UnitaryMatrix>` | 返回原始矩阵表示。 |
| `circuit()` | `&Option<Arc<FrozenCircuit>>` | 返回 circuit-backed 定义。 |

---

## 数值矩阵定义

```rust
pub fn with_matrix(self, mat: Array2<Complex<f64>>) -> Result<Self, CircuitError>
```

`with_matrix()` 用于为自定义门附加固定的数值矩阵定义。矩阵必须是复数方阵，形状必须为：

```text
2^num_qubits × 2^num_qubits
```

此外，使用数值矩阵定义时，`num_params` 必须为 `0`，因为该门的矩阵已经完全确定，不需要在应用时额外传入位置参数。

```rust
use cqlib_core::circuit::UnitaryGate;
use ndarray::array;
use num_complex::Complex64;

let x = array![
    [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
    [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
];

let gate = UnitaryGate::new("CustomX", 1, 0).with_matrix(x)?;

assert!(gate.matrix().is_some());

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 符号矩阵定义

```rust
pub fn with_symbolic_matrix<I, S>(
    self,
    params: I,
    matrix: SymbolicMatrix,
) -> Result<Self, CircuitError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
```

`with_symbolic_matrix()` 用于为自定义门附加符号矩阵定义。符号矩阵中的元素可以包含 `Parameter` 表达式，适合表示带参数的自定义酉门。

其中，`params` 指定门在应用时传入的位置参数如何映射到符号矩阵中的变量名。也就是说，`params` 不只是一个说明性列表，而是定义了参数绑定顺序。调用 `matrix_for_params()` 或将该门追加到线路时，传入的第 `i` 个参数会绑定到 `params[i]` 对应的符号名。

```rust
use cqlib_core::circuit::{Parameter, UnitaryGate};
use cqlib_core::circuit::symbolic_matrix::{SymbolicComplex, SymbolicMatrix};

let theta = Parameter::symbol("theta");

let matrix = SymbolicMatrix::from_shape_vec(
    (2, 2),
    vec![
        SymbolicComplex::one(),
        SymbolicComplex::zero(),
        SymbolicComplex::zero(),
        SymbolicComplex::exp_i(theta),
    ],
).expect("valid 2x2 symbolic matrix");

let gate = UnitaryGate::new("PhaseLike", 1, 1)
    .with_symbolic_matrix(["theta"], matrix)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，符号矩阵包含变量 `theta`，而 `with_symbolic_matrix(["theta"], matrix)` 声明该门应用时只有一个位置参数，并且该参数会绑定到矩阵中的 `theta`。

---

## Circuit-backed 定义

```rust
pub fn with_circuit(self, circuit: Arc<FrozenCircuit>) -> Result<Self, CircuitError>
```

`with_circuit()` 用于将一个 `FrozenCircuit` 作为 `UnitaryGate` 的定义。当门的行为可以由一段子线路描述时，可以使用该方式保留内部电路结构。求矩阵时，Cqlib 会根据门应用时传入的参数对内部线路进行绑定，并调用 `circuit_to_matrix()` 计算矩阵。

```rust
use cqlib_core::circuit::{Circuit, Qubit, UnitaryGate};
use cqlib_core::circuit::gate::FrozenCircuit;
use std::sync::Arc;

let mut inner = Circuit::new(2);
inner.h(Qubit::new(0))?;
inner.cx(Qubit::new(0), Qubit::new(1))?;

let gate = UnitaryGate::new("BellPrep", 2, 0)
    .with_circuit(Arc::new(FrozenCircuit::new(inner)))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 求具体矩阵

```rust
pub fn matrix_for_params(
    &self,
    params: &[f64],
) -> Result<Cow<'_, Array2<Complex<f64>>>, CircuitError>
```

`matrix_for_params(params)` 用于在给定位置参数后获取 `UnitaryGate` 的具体数值矩阵。该方法会根据门的定义方式自动分派：

| 定义方式 | 求矩阵方式 |
| --- | --- |
| 数值矩阵 | 直接返回已保存的矩阵，通常以 borrowed 形式返回。 |
| 符号矩阵 | 按 `matrix_params()` 中的顺序将 `params` 绑定到符号名，然后对 `SymbolicMatrix` 求值。 |
| Circuit-backed 定义 | 按内部电路 `symbols()` 顺序绑定参数，再对内部线路调用 `circuit_to_matrix()`。 |

---

## 追加到 `Circuit`

`UnitaryGate` 可以通过 `Circuit` 提供的自定义酉门追加接口加入线路。

```rust
circuit.unitary(gate, qubits)?;
circuit.unitary_with_params(gate, qubits, params)?;
```

- `unitary(gate, qubits)` 适用于无位置参数的自定义门；
- `unitary_with_params(gate, qubits, params)` 适用于带位置参数的自定义门。

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(1);
circuit.unitary(gate, vec![Qubit::new(0)])?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 定义方式对比

| 定义方式 | 参数支持 | 是否保留电路结构 | 矩阵来源 | 典型用途 |
| --- | --- | --- | --- | --- |
| `with_matrix()` | 不支持应用参数，`num_params` 必须为 0 | 否 | 直接保存 `Array2<Complex64>` | 固定黑盒矩阵、硬件校准门、测试 oracle。 |
| `with_symbolic_matrix()` | 支持 `num_params` 个位置参数 | 否 | 对 `SymbolicMatrix` 求值 | 参数化自定义矩阵门、符号相位门。 |
| `with_circuit()` | 支持内部电路符号绑定 | 是 | 对内部线路绑定参数后调用 `circuit_to_matrix()` | 希望以自定义酉门形式携带线路定义。 |

---

## 与 `StandardGate` 和 `CircuitGate` 的区别

| 类型 | 适合场景 | 特点 |
| --- | --- | --- |
| `StandardGate` | Cqlib 原生支持的标准门 | 轻量、可优化、门语义明确，适合作为编译 basis。 |
| `CircuitGate` | 将子线路封装为复合门复用 | 保留模块边界，便于分解和结构化分析。 |
| `UnitaryGate` | 标准门之外的自定义酉门 | 可由矩阵、符号矩阵或冻结线路定义，适合黑盒或特殊酉门。 |
