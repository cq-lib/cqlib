# Circuit

`cqlib_core::circuit::Circuit`  

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
```

`Circuit` 是 Rust core 中的量子线路主容器，用于保存一条量子程序的核心结构信息，包括逻辑量子比特集合、操作序列、参数表、经典变量与经典值、控制流作用域以及全局相位等。

---

## 创建线路

### `Circuit::new`

```rust
pub fn new(num_qubits: usize) -> Self
```

`Circuit::new(num_qubits)` 用于创建一条包含连续逻辑量子比特的空线路。量子比特编号从 `0` 开始，依次为 `Qubit::new(0)` 到 `Qubit::new(num_qubits - 1)`。

```rust
use cqlib_core::circuit::Circuit;

let circuit = Circuit::new(3);
assert_eq!(circuit.num_qubits(), 3);
assert_eq!(circuit.width(), 3);
```

### `Circuit::from_qubits`

```rust
pub fn from_qubits(qubits: Vec<Qubit>) -> Result<Circuit, CircuitError>
```

`Circuit::from_qubits()` 用于使用指定的逻辑量子比特集合创建线路。该接口允许使用稀疏逻辑编号，例如 `Qubit::new(2)` 和 `Qubit::new(5)`。

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let circuit = Circuit::from_qubits(vec![Qubit::new(2), Qubit::new(5)])?;
assert_eq!(circuit.width(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

### `Circuit::from_operations`

```rust
pub fn from_operations(
    qubits: Vec<Qubit>,
    operations: impl IntoIterator<Item = ValueOperation>,
    classical_vars: Option<Vec<ClassicalType>>,
    classical_values: Option<Vec<ClassicalType>>,
) -> Result<Self, CircuitError>
```

`Circuit::from_operations()` 用于从构造层 IR 创建线路。

---

## 基础属性

`Circuit` 提供了一组只读访问方法，用于查看线路的基本结构信息。

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `id()` | `CircuitId` | 当前线路的经典句柄身份，用于区分不同线路中的经典变量和值。 |
| `width()` | `usize` | 线路宽度，即量子比特数量。 |
| `num_qubits()` | `usize` | 量子比特数量，与 `width()` 语义一致。 |
| `qubits()` | `Vec<Qubit>` | 按插入顺序返回线路中的逻辑量子比特。 |
| `operations()` | `&[Operation]` | 返回内部存储层操作序列。 |
| `parameters()` | `&IndexSet<Parameter>` | 返回已驻留的参数表达式集合。 |
| `symbols()` | `&IndexSet<String>` | 返回线路中出现的自由符号名集合。 |
| `classical_vars()` | `&[ClassicalType]` | 返回已分配的可变经典变量类型表。 |
| `classical_values()` | `&[ClassicalType]` | 返回不可变经典值类型表，通常由测量产生。 |
| `global_phase()` | `Parameter` | 返回线路全局相位的参数表达式。 |
| `global_phase_param()` | `&CircuitParam` | 返回内部存储形式的全局相位参数。 |

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);

let mut c = Circuit::new(2);
c.h(q0)?;
c.cx(q0, q1)?;

assert_eq!(c.operations().len(), 2);
assert_eq!(c.num_qubits(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 量子比特和参数管理

`Circuit` 内部会维护量子比特集合和参数表。量子比特用于描述操作作用对象，参数表用于统一管理线路中的符号参数和参数表达式。

| 方法 | 说明 |
| --- | --- |
| `add_qubits(new_qubits)` | 向线路追加新的逻辑量子比特；若包含重复量子比特则返回错误。 |
| `add_parameter(param)` | 将参数插入线路参数表，返回 `(index, is_new)`。 |
| `resolve_parameter(param)` | 将内部 `CircuitParam` 还原为 `Parameter`。 |
| `parameter_value(param)` | 将内部 `CircuitParam` 转换为构造层 `ParameterValue`。 |
| `map_param(param)` | 将 `Parameter` 映射到线路参数表，并返回内部 `CircuitParam`。 |
| `set_global_phase(phase)` | 设置线路全局相位。 |

---

## 追加操作

### `append`

```rust
pub fn append<Q, P>(
    &mut self,
    instruction: Instruction,
    qubits: Q,
    params: P,
    label: Option<&str>,
) -> Result<(), CircuitError>
where
    Q: IntoIterator,
    Q::Item: Into<Qubit>,
    P: IntoIterator<Item = ParameterValue>
```

`append()` 是通用的存储层指令追加入口，用于向线路中追加任意 `Instruction`。调用时需要显式提供指令、作用量子比特、参数列表和可选标签。

该方法会执行多项检查，包括：

- 指令作用的量子比特数量是否匹配；
- 参数数量是否与指令定义一致；
- 量子比特是否属于当前线路；
- 同一次操作中是否重复引用同一个量子比特；
- 固定数值参数是否为有限值，即不能为 `NaN` 或无穷大；
- 参数表达式是否能够正确映射到线路参数表。

### `append_value_operation`

```rust
pub fn append_value_operation(&mut self, operation: ValueOperation) -> Result<(), CircuitError>
```

`append_value_operation()` 用于追加自包含的构造层操作。`ValueOperation` 中已经包含指令、量子比特、参数和标签信息，因此该接口特别适合 IR 导入、反序列化和编译器 pass 输出。

### `index`

```rust
pub fn index(&self, i: usize) -> Result<ValueOperation, CircuitError>
```

`index(i)` 用于读取第 `i` 条操作，并将内部存储层参数表示还原为构造层 `ParameterValue`。

---

## 标准门便捷方法

`Circuit` 提供了一组常用标准门的便捷方法。所有便捷方法都会修改当前线路，并返回 `Result<(), CircuitError>`。用户可以使用 `?` 或显式 `match` 处理可能出现的错误。

### 单量子比特固定门

| 方法 | 标准门 |
| --- | --- |
| `i(qubit)` | `StandardGate::I` |
| `h(qubit)` | `StandardGate::H` |
| `x(qubit)` | `StandardGate::X` |
| `y(qubit)` | `StandardGate::Y` |
| `z(qubit)` | `StandardGate::Z` |
| `s(qubit)` | `StandardGate::S` |
| `sdg(qubit)` | `StandardGate::SDG` |
| `t(qubit)` | `StandardGate::T` |
| `tdg(qubit)` | `StandardGate::TDG` |
| `x2p(qubit)` | `StandardGate::X2P` |
| `x2m(qubit)` | `StandardGate::X2M` |
| `y2p(qubit)` | `StandardGate::Y2P` |
| `y2m(qubit)` | `StandardGate::Y2M` |

### 参数化单量子比特门

| 方法 | 说明 |
| --- | --- |
| `rx(qubit, theta)` | 绕 X 轴旋转。 |
| `ry(qubit, theta)` | 绕 Y 轴旋转。 |
| `rz(qubit, theta)` | 绕 Z 轴旋转。 |
| `phase(qubit, lambda)` | 相位门。 |
| `u(qubit, theta, phi, lambda)` | 通用单量子比特门。 |
| `xy(qubit, theta)` | XY 交互族门。 |
| `xy2p(qubit, theta)` | 正半角 XY 门。 |
| `xy2m(qubit, theta)` | 负半角 XY 门。 |
| `rxy(qubit, theta, phi)` | XY 平面任意轴旋转。 |

### 多量子比特门

| 方法 | 说明 |
| --- | --- |
| `cx(control, target)` | controlled-X，也称 CNOT。 |
| `cy(control, target)` | controlled-Y。 |
| `cz(control, target)` | controlled-Z。 |
| `swap(a, b)` | 交换两个量子比特状态。 |
| `ccx(control1, control2, target)` | Toffoli 门。 |
| `rxx(a, b, theta)` | XX 旋转。 |
| `ryy(a, b, theta)` | YY 旋转。 |
| `rzz(a, b, theta)` | ZZ 旋转。 |
| `rzx(a, b, theta)` | ZX 旋转。 |
| `crx(control, target, theta)` | controlled-RX。 |
| `cry(control, target, theta)` | controlled-RY。 |
| `crz(control, target, theta)` | controlled-RZ。 |
| `fsim(a, b, theta, phi)` | fSim 门。 |

示例：

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let theta = Parameter::symbol("theta");

let mut c = Circuit::new(2);
c.h(q0)?;
c.cx(q0, q1)?;
c.rzz(q0, q1, theta)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## 其它量子操作

除标准门便捷方法外，`Circuit` 还提供了 barrier、reset、delay、多控制门、自定义酉门和复合门等操作入口。

| 方法 | 说明 |
| --- | --- |
| `barrier(qubits)` | 插入 barrier，用于约束编译器或调度器不要跨该边界重排相关量子比特上的操作。 |
| `reset(qubit)` | 将量子比特复位到 `|0>`。 |
| `delay(qubit, duration)` | 在指定量子比特上插入空闲时间。 |
| `multi_control(controls, targets, gate, params)` | 构造多控制标准门。 |
| `unitary(gate, qubits)` | 追加无额外位置参数的 `UnitaryGate`。 |
| `unitary_with_params(gate, qubits, params)` | 追加带位置参数的 `UnitaryGate`。 |
| `circuit_gate(gate, qubits, params)` | 追加 `CircuitGate`。 |

---

## 经典数据和控制流入口

`Circuit` 也提供经典数据和结构化控制流相关接口。它们用于构造条件分支、循环和多分支选择等程序结构。

| 方法 | 说明 |
| --- | --- |
| `var(ty)` | 分配经典变量。 |
| `store(target, value)` | 将经典表达式写入经典变量。 |
| `measure(qubit)` | 测量一个量子比特。 |
| `measure_bits(qubits)` | 测量多个量子比特。 |
| `measure_into(qubit, target)` | 测量并写入已有经典变量。 |
| `measure_bits_into(qubits, target)` | 多量子比特测量并写入已有经典变量。 |
| `if_()` / `if_else()` | 条件分支。 |
| `while_()` | `while` 循环。 |
| `for_uint()` | 基于 `UInt` 的半开区间循环。 |
| `switch()` | 多分支选择。 |
| `append_control(op)` | 追加低层控制流对象。 |
| `break_loop()` / `continue_loop()` | 控制流跳转。 |

---

## 分析和变换

`Circuit` 提供多种线路分析和结构变换方法。

| 方法 | 说明 |
| --- | --- |
| `depth(recurse)` | 计算线路深度。若线路包含控制流且 `recurse = false`，通常返回 `ControlFlowPresent`。 |
| `validate()` | 验证经典句柄、控制流作用域、数据依赖和线路结构不变量。 |
| `inverse()` | 返回当前线路的反线路。若包含不可逆操作则返回错误。 |
| `decompose()` | 展开由线路定义的复合门。 |
| `to_gate(name)` | 将线路转换为 `Instruction::CircuitGate`，用于作为复合门复用。 |
| `to_matrix(qubits_order)` | 返回小规模纯酉线路的密集数值矩阵。 |
| `assign_parameters(bindings)` | 绑定符号参数，返回新线路。 |
| `compose(other, qubits_map)` | 将另一条线路追加到当前线路中，可指定量子比特映射。 |

---

## 参数绑定

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut c = Circuit::new(1);
c.rx(Qubit::new(0), theta)?;

let mut bindings = HashMap::new();
bindings.insert("theta", 0.5);

let bound = c.assign_parameters(&Some(bindings))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`assign_parameters()` 用于将线路中的符号参数替换为具体数值。

---

## 线路组合

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut lhs = Circuit::new(3);
let mut rhs = Circuit::new(2);

rhs.cx(Qubit::new(0), Qubit::new(1))?;

lhs.compose(&rhs, Some(&[Qubit::new(1), Qubit::new(2)]))?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`compose(other, qubits_map)` 用于将另一条线路追加到当前线路末尾。传入 `qubits_map` 时，会按照 `other.qubits()` 的顺序，将右侧线路的量子比特映射到当前线路中的目标量子比特。