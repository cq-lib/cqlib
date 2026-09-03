# Symbolic Matrix

`cqlib_core::circuit::symbolic_matrix`

```rust
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
    circuit_to_symbolic_matrix,
    evaluate_symbolic_matrix,
    circuits_equivalent,
};
```

`symbolic_matrix` 模块提供线路矩阵转换的符号化能力。与数值矩阵转换函数 `circuit_to_matrix()` 不同，符号矩阵转换会尽量保留线路中的 `Parameter` 表达式，使矩阵元素能够包含未绑定的符号参数。

---

## 核心概念

| 类型 / 函数 | 说明 |
| --- | --- |
| `SymbolicComplex` | 符号复数，实部和虚部均为 `Parameter` 表达式。 |
| `SymbolicMatrix` | 由 `SymbolicComplex` 构成的密集矩阵，通常可理解为 `Array2<SymbolicComplex>`。 |
| `circuit_to_symbolic_matrix()` | 将纯量子线路转换为保留符号参数的矩阵。 |
| `evaluate_symbolic_matrix()` | 为符号矩阵绑定参数并求值得到数值复矩阵。 |
| `symbolic_matrices_equivalent()` | 判断两个符号矩阵是否能证明只差全局相位。 |
| `circuits_equivalent()` | 判断两条线路是否能证明在矩阵意义下全局相位等价。 |

---

## `SymbolicComplex`

`SymbolicComplex` 表示一个符号复数，其实部和虚部都是 `Parameter` 表达式。它可以表示普通复数常量，也可以表示包含符号变量的复数表达式，例如 `exp(iθ)`。

### 常用构造方法

| 方法 | 说明 |
| --- | --- |
| `new(re, im)` | 根据实部和虚部表达式创建符号复数。 |
| `zero()` | 创建 `0 + 0i`。 |
| `one()` | 创建 `1 + 0i`。 |
| `i()` | 创建 `0 + 1i`。 |
| `from_real(value)` | 根据实数或实数表达式创建纯实符号复数。 |
| `from_complex(value)` | 根据 `Complex64` 创建符号复数常量。 |
| `exp_i(theta)` | 创建 `cos(theta) + i sin(theta)`，即 `e^{i theta}`。 |

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::SymbolicComplex;

let theta = Parameter::symbol("theta");

let zero = SymbolicComplex::zero();
let one = SymbolicComplex::one();
let imag = SymbolicComplex::i();
let phase = SymbolicComplex::exp_i(theta);
```

其中，`exp_i(theta)` 常用于定义相位门、对角门和符号化量子演化中的矩阵元素。

### 常用方法

| 方法 | 说明 |
| --- | --- |
| `evaluate(bindings)` | 绑定符号参数并求值为 `Complex64`。 |
| `simplify()` | 化简实部和虚部表达式。 |
| `replace(symbol, value)` | 替换复数中出现的某个符号。 |
| `is_zero_exact()` | 判断是否在语法上精确为零。 |
| `is_one_exact()` | 判断是否在语法上精确为一。 |
| `simplifies_to_zero()` | 化简后判断是否为零。 |

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::SymbolicComplex;
use std::collections::HashMap;

let theta = Parameter::symbol("theta");
let value = SymbolicComplex::exp_i(theta);

let mut bindings = HashMap::new();
bindings.insert("theta", 0.0);

let numeric = value.evaluate(&Some(bindings))?;
assert_eq!(numeric, num_complex::Complex64::new(1.0, 0.0));

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## `SymbolicMatrix`

`SymbolicMatrix` 是由 `SymbolicComplex` 组成的密集矩阵。它可以表示一个带符号参数的量子门矩阵或线路矩阵。

```rust
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
};

let matrix = SymbolicMatrix::from_shape_vec(
    (2, 2),
    vec![
        SymbolicComplex::zero(),
        SymbolicComplex::i(),
        SymbolicComplex::i(),
        SymbolicComplex::zero(),
    ],
).expect("valid 2x2 symbolic matrix");
```

使用符号矩阵时，应确保矩阵维度与目标门或线路的量子比特数量一致。例如，单量子比特门矩阵应为 `2 × 2`，两量子比特门矩阵应为 `4 × 4`。

---

## 矩阵工具函数

`symbolic_matrix` 模块提供了一组常用工具函数，用于构造、替换、求值、转换和等价性判断。

| 函数 | 说明 |
| --- | --- |
| `symbolic_eye(dim)` | 创建 `dim × dim` 的符号单位矩阵。 |
| `substitute_symbolic_matrix(matrix, replacements)` | 对矩阵中的符号执行同时替换。 |
| `evaluate_symbolic_matrix(matrix, bindings)` | 将符号矩阵求值为 `Array2<Complex64>`。 |
| `standard_gate_symbolic_matrix(gate, params)` | 生成标准门的符号矩阵。 |
| `circuit_to_symbolic_matrix(circuit, qubits_order)` | 将线路转换为符号酉矩阵。 |
| `symbolic_matrices_equivalent(lhs, rhs)` | 判断两个符号矩阵是否能证明只差全局相位。 |
| `circuits_equivalent(lhs, rhs, qubits_order)` | 判断两条线路是否能证明全局相位等价。 |

---

## 电路符号矩阵转换

```rust
pub fn circuit_to_symbolic_matrix(
    circuit: &Circuit,
    qubits_order: Option<&[Qubit]>,
) -> Result<SymbolicMatrix, CircuitError>
```

`circuit_to_symbolic_matrix()` 用于将一条量子线路转换为保留 `Parameter` 表达式的符号矩阵。

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::circuit::symbolic_matrix::{
    circuit_to_symbolic_matrix,
    evaluate_symbolic_matrix,
};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), theta)?;

let symbolic = circuit_to_symbolic_matrix(&circuit, None)?;

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::FRAC_PI_2);

let numeric = evaluate_symbolic_matrix(&symbolic, &Some(bindings))?;
assert_eq!(numeric.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

在上述示例中，`RX(theta)` 的参数不会在符号矩阵构造阶段被数值化，而是保留为符号表达式。后续可以通过 `evaluate_symbolic_matrix()` 绑定 `theta` 的具体取值并得到数值矩阵。

---

## 量子比特顺序约定

`circuit_to_symbolic_matrix(circuit, qubits_order)` 需要确定矩阵基态索引与线路量子比特之间的对应关系。其顺序约定如下：

- `qubits_order = None`：默认按量子比特编号排序；
- `qubits_order = Some(order)`：使用调用者显式给出的顺序；
- 显式顺序必须与线路中的量子比特集合完全一致，不能遗漏、重复或包含未知量子比特；
- 顺序中的第一个量子比特对应 basis index 的 least-significant bit。

例如，如果线路量子比特为 `[Qubit(0), Qubit(2)]`，显式顺序可以写为 `[Qubit(0), Qubit(2)]` 或 `[Qubit(2), Qubit(0)]`，二者会得到不同轴顺序下的矩阵表示。

---

## 符号矩阵求值

```rust
pub fn evaluate_symbolic_matrix(
    matrix: &SymbolicMatrix,
    bindings: &Option<HashMap<&str, f64>>,
) -> Result<Array2<Complex64>, ParameterError>
```

`evaluate_symbolic_matrix()` 用于将符号矩阵转换为数值复矩阵。它会对每个 `SymbolicComplex` 元素分别执行参数绑定和求值。

```rust
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex,
    SymbolicMatrix,
    evaluate_symbolic_matrix,
};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let matrix = SymbolicMatrix::from_shape_vec(
    (2, 2),
    vec![
        SymbolicComplex::one(),
        SymbolicComplex::zero(),
        SymbolicComplex::zero(),
        SymbolicComplex::exp_i(theta),
    ],
).expect("valid symbolic matrix");

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::PI);

let numeric = evaluate_symbolic_matrix(&matrix, &Some(bindings))?;
assert_eq!(numeric.shape(), &[2, 2]);

# Ok::<(), cqlib_core::circuit::error::ParameterError>(())
```

---

## 符号替换

`substitute_symbolic_matrix(matrix, replacements)` 用于对矩阵中的符号进行同时替换。它适合参数重命名、复合门展开、模板参数替换和自定义符号门绑定等场景。

典型场景包括：

- 将子线路中的内部参数 `theta` 替换为外层参数 `alpha`；
- 在编译 pass 中重命名参数，避免符号冲突；
- 将一个通用符号矩阵实例化为另一个参数化门定义。

---

## 等价性检查

`symbolic_matrix` 模块提供全局相位意义下的等价性检查接口。

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::circuit::symbolic_matrix::circuits_equivalent;

let mut lhs = Circuit::new(1);
lhs.x(Qubit::new(0))?;
lhs.set_global_phase(std::f64::consts::PI.into());

let mut rhs = Circuit::new(1);
rhs.x(Qubit::new(0))?;

assert!(circuits_equivalent(&lhs, &rhs, None)?);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```