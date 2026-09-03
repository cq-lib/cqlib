# Circuit To Matrix

- `cqlib_core::circuit::circuit_to_matrix`
- `cqlib_core::circuit::Circuit::to_matrix`
- `cqlib_core::circuit::symbolic_matrix::circuit_to_symbolic_matrix`

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};
```

`Circuit To Matrix` 相关接口用于将量子线路转换为矩阵表示。Rust core 提供两类矩阵转换能力：

- 数值矩阵转换：将所有参数均已绑定的线路转换为 `Array2<Complex64>`；
- 符号矩阵转换：在线路中保留 `Parameter` 表达式，生成 `SymbolicMatrix`。
  
---

## 数值矩阵转换

```rust
pub fn circuit_to_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[usize]>,
) -> Result<Array2<Complex64>, CircuitError>
```

`circuit_to_matrix()` 是函数式矩阵转换入口，用于将一条线路转换为数值复矩阵。`Circuit::to_matrix()` 是对应的方法式便捷接口，两者语义一致。

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

let matrix = circuit_to_matrix(&circuit, None)?;
assert_eq!(matrix.shape(), &[4, 4]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，线路包含 2 个量子比特，因此矩阵维度为 `4 × 4`。一般来说，若线路包含 `n` 个量子比特，则返回矩阵形状为：

```text
(2^n, 2^n)
```

---

## `Circuit::to_matrix()` 方法式入口

除函数式接口外，也可以直接在线路对象上调用 `to_matrix()`：

```rust
let matrix = circuit.to_matrix(None)?;
# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 量子比特顺序

`qubits_order` 用于指定矩阵基态索引中量子比特的排列顺序。它必须是线路中量子比特编号的完整排列。

规则如下：

- `None`：默认按量子比特编号排序；
- `Some(&[...])`：使用显式指定的量子比特顺序；
- 显式顺序必须与线路量子比特集合完全一致，不能遗漏、重复或包含未知量子比特；
- 顺序中的第一个量子比特对应 basis index 的最低有效位，也就是 little-endian 约定。

```rust
let matrix_default = circuit_to_matrix(&circuit, None)?;
let matrix_reordered = circuit_to_matrix(&circuit, Some(&[1, 0]))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 参数化线路

数值矩阵要求所有符号参数都已绑定为具体数值。若线路中仍存在未绑定符号参数，矩阵转换无法得到确定的复数矩阵。

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit, circuit_to_matrix};
use std::collections::HashMap;

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), Parameter::symbol("theta"))?;

let mut bindings = HashMap::new();
bindings.insert("theta", 0.5);

let bound = circuit.assign_parameters(&Some(bindings))?;
let matrix = circuit_to_matrix(&bound, None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 符号矩阵转换

```rust
use cqlib_core::circuit::symbolic_matrix::circuit_to_symbolic_matrix;

let symbolic = circuit_to_symbolic_matrix(&circuit, None)?;
# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

符号矩阵转换会保留线路中的 `Parameter` 表达式，返回 `SymbolicMatrix`。随后可以通过 `evaluate_symbolic_matrix()` 在不同参数取值下多次求值。

---

## 全局相位

数值矩阵和符号矩阵转换都会包含线路的全局相位。如果线路全局相位为 `theta`，且不考虑全局相位时的线路矩阵为 `U`，则矩阵转换返回的是：

```text
exp(i * theta) * U
```

```rust
use cqlib_core::circuit::{Circuit, Qubit, circuit_to_matrix};

let mut circuit = Circuit::new(1);
circuit.x(Qubit::new(0))?;
circuit.set_global_phase(std::f64::consts::PI.into());

let matrix = circuit_to_matrix(&circuit, None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，线路操作部分对应 `X` 门，全局相位为 `π`，因此整体矩阵等价于 `-X`。