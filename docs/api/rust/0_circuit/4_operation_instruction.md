# Operation / Instruction

模块路径：

- `cqlib_core::circuit::Instruction`
- `cqlib_core::circuit::Operation`
- `cqlib_core::circuit::ValueInstruction`
- `cqlib_core::circuit::ValueOperation`
- `cqlib_core::circuit::Directive`
- `cqlib_core::circuit::ClassicalDataOp`

这些类型共同构成 `cqlib_core::circuit` 模块中的操作级中间表示。它们用于描述线路中的每一条操作，包括量子门、非酉指令、经典数据操作、结构化控制流以及延迟等特殊指令。

从语义上看，可以将这些类型分为两层：

- 存储层 IR：用于 `Circuit` 内部高效存储，依赖线路内部参数表；
- 构造层 IR：用于导入、导出、序列化、编译器 pass 输出和测试，操作本身尽量自包含。

---

## IR 层次

| 类型 | 层次 | 说明 |
| --- | --- | --- |
| `Instruction` | 存储层 IR | 描述“执行什么指令”，可以是标准门、多控制门、自定义酉门、子线路门、directive、经典数据操作、经典控制流或 `Delay`。 |
| `Operation` | 存储层 IR | 表示线路内部的一条操作，由 `Instruction + qubits + CircuitParam + label` 组成。其参数可能引用所属 `Circuit` 的参数表。 |
| `ValueInstruction` | 构造层 IR | 可包裹普通 `Instruction`，也可包裹构造层经典控制流操作。 |
| `ValueOperation` | 构造层 IR | 表示自包含操作，由 `ValueInstruction + qubits + ParameterValue + label` 组成，适合跨线路传递、序列化和导入。 |

`Circuit::operations()` 返回的是内部存储层操作序列：

```rust
pub fn operations(&self) -> &[Operation]
```

如果需要读取可脱离电路内部参数表的自包含操作，可以使用：

```rust
pub fn index(&self, i: usize) -> Result<ValueOperation, CircuitError>
```

---

## `Instruction`

`Instruction` 描述一条操作“做什么”。它只描述指令类型和门定义本身，不包含该指令作用在哪些量子比特上，也不包含当前操作实例携带的参数值。

```rust
pub enum Instruction {
    Standard(StandardGate),
    McGate(Box<MCGate>),
    UnitaryGate(Box<UnitaryGate>),
    CircuitGate(Box<CircuitGate>),
    Directive(Directive),
    ClassicalData(ClassicalDataOp),
    ClassicalControl(ClassicalControlOp),
    Delay,
}
```

### 1. 变体说明

| 变体 | 说明 |
| --- | --- |
| `Standard(StandardGate)` | Cqlib 内置标准量子门，例如 `H`、`CX`、`RZ` 等。 |
| `McGate(Box<MCGate>)` | 多控制标准门，由基础标准门添加若干控制位得到。 |
| `UnitaryGate(Box<UnitaryGate>)` | 用户自定义酉门，可由矩阵、符号矩阵或线路定义。 |
| `CircuitGate(Box<CircuitGate>)` | 由冻结线路定义的复合门。 |
| `Directive(Directive)` | 非酉指令，例如 `Barrier`、`Measure`、`Reset`。 |
| `ClassicalData(ClassicalDataOp)` | 经典数据操作，例如 `store` 或测量结果写入。 |
| `ClassicalControl(ClassicalControlOp)` | 结构化控制流，例如 `if`、`while`、`for`、`switch`。 |
| `Delay` | 延迟或空闲时间指令，通常用于调度或硬件时序语义。 |

在普通手写线路中，用户通常不需要直接构造 `Instruction`。但在编写编译器 pass、IR 导入器、序列化工具或底层测试时，显式操作 `Instruction` 可以更精确地控制线路结构。

### 2. 常用方法

| 方法 | 说明 |
| --- | --- |
| `has_measurement()` | 判断当前指令或其递归控制流体中是否包含测量。 |
| `reads_value(value)` | 判断当前指令是否读取指定 `ClassicalValue`。 |
| `gate_arity()` | 返回门需要的量子比特数量和参数数量；非门指令通常返回 `None`。 |
| `matrix(params)` | 尝试返回数值矩阵；非酉或不支持矩阵表示的指令返回 `None`。 |
| `inverse(params)` | 尝试返回反指令及反参数；不可逆指令返回 `None`。 |
| `control(num_new_ctrls)` | 尝试将当前指令提升为受控指令。 |

### 3. `From` 转换

`Instruction` 支持从多种底层类型转换而来，便于在构造 IR 或编译 pass 中快速创建指令。

常见转换包括：

- `StandardGate`
- `Directive`
- `ClassicalControlOp`
- `ClassicalDataOp`

```rust
use cqlib_core::circuit::{Instruction, StandardGate};

let inst: Instruction = StandardGate::H.into();
```

---

## `Directive`

`Directive` 表示不属于普通酉门的特殊线路指令。

```rust
pub enum Directive {
    Barrier,
    Measure,
    Reset,
}
```

| 变体 | 说明 |
| --- | --- |
| `Barrier` | 编译或调度屏障，用于阻止相关量子比特上的操作跨越该边界重排。 |
| `Measure` | 计算基测量，会产生经典结果，是非酉操作。 |
| `Reset` | 将量子比特复位到 `|0>`，通常破坏原有量子态相干性。 |

`Directive` 的公开反演接口如下：

```rust
pub fn inverse(&self) -> Option<Self>
```

其中，`Barrier` 可视为自身的逆；`Measure` 和 `Reset` 不可逆，因此返回 `None`。

```rust
use cqlib_core::circuit::Directive;

assert_eq!(Directive::Barrier.inverse(), Some(Directive::Barrier));
assert_eq!(Directive::Measure.inverse(), None);
assert_eq!(Directive::Reset.inverse(), None);
```

需要注意的是，`Directive` 通常不具有普通酉矩阵表示。包含 `Measure` 或 `Reset` 的线路不能作为纯量子门线路直接调用矩阵转换接口。

---

## `ClassicalDataOp`

`ClassicalDataOp` 表示线路中的经典数据操作，主要用于描述经典变量写入和测量结果产生。

```rust
pub enum ClassicalDataOp {
    Store { target: ClassicalVar, value: ClassicalExpr },
    MeasureBit { result: ClassicalValue },
    MeasureBits { result: ClassicalValue },
}
```

| 变体 | 说明 |
| --- | --- |
| `Store { target, value }` | 将一个经典表达式写入可变经典变量。 |
| `MeasureBit { result }` | 单量子比特测量产生一个经典值。 |
| `MeasureBits { result }` | 多量子比特测量产生一个 bit vector 经典值。 |

常用访问方法如下：

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `target()` | `Option<ClassicalVar>` | 当操作为 `Store` 时返回目标变量。 |
| `result()` | `Option<ClassicalValue>` | 当操作为测量结果产生操作时返回结果值。 |
| `value()` | `Option<&ClassicalExpr>` | 当操作为 `Store` 时返回写入表达式。 |

普通用户通常通过高层 `Circuit` 方法构造这些操作，例如：

- `Circuit::store()`
- `Circuit::measure()`
- `Circuit::measure_bits()`
- `Circuit::measure_into()`
- `Circuit::measure_bits_into()`

---

## `Operation`

`Operation` 是 `Circuit` 内部使用的存储层操作表示。它记录一条指令在当前线路中的一次具体应用。

```rust
pub struct Operation {
    pub instruction: Instruction,
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[CircuitParam; 1]>,
    pub label: Option<Box<str>>,
}
```

字段说明：

| 字段 | 说明 |
| --- | --- |
| `instruction` | 当前操作执行的指令类型。 |
| `qubits` | 当前操作作用的逻辑量子比特。 |
| `params` | 当前操作携带的参数，使用 `CircuitParam` 表示。 |
| `label` | 可选元数据标签，不改变操作的数学语义。 |

### `Operation::matrix`

```rust
pub fn matrix(&self) -> Result<Cow<'_, Array2<Complex64>>, CircuitError>
```

`Operation::matrix()` 用于计算单个操作的数值矩阵。

---

## `ValueInstruction`

`ValueInstruction` 是构造层指令表示。它可以包裹普通存储层 `Instruction`，也可以包裹构造层经典控制流对象。

```rust
pub enum ValueInstruction {
    Instruction(Instruction),
    ClassicalControl(ValueClassicalControlOp),
}
```

常用方法如下：

| 方法 | 说明 |
| --- | --- |
| `from_instruction(inst)` | 将普通 `Instruction` 包装为 `ValueInstruction`。 |
| `is_classical_control()` | 判断是否为构造层控制流。 |
| `is_instruction()` | 判断是否为普通指令。 |
| `as_instruction()` | 读取内部 `Instruction` 引用。 |
| `into_instruction()` | 消耗对象并取出内部 `Instruction`。 |

---

## `ValueOperation`

`ValueOperation` 是构造层自包含操作表示，适合在不同线路之间传递，也适合作为序列化、导入器和编译器输出的边界对象。

```rust
pub struct ValueOperation {
    pub instruction: ValueInstruction,
    pub qubits: SmallVec<[Qubit; 3]>,
    pub params: SmallVec<[ParameterValue; 1]>,
    pub label: Option<Box<str>>,
}
```

字段说明：

| 字段 | 说明 |
| --- | --- |
| `instruction` | 构造层指令，可表示普通指令或控制流。 |
| `qubits` | 作用量子比特列表。 |
| `params` | 自包含参数，使用 `ParameterValue` 表示，可为固定值或完整 `Parameter`。 |
| `label` | 可选标签，仅作为元数据使用。 |

### `ValueOperation::from_standard`

```rust
pub fn from_standard(
    gate: StandardGate,
    qubits: impl IntoIterator<Item = Qubit>,
    params: impl IntoIterator<Item = ParameterValue>,
) -> Self
```

```rust
use cqlib_core::circuit::{ParameterValue, Qubit, StandardGate, ValueOperation};

let op = ValueOperation::from_standard(
    StandardGate::RX,
    [Qubit::new(0)],
    [ParameterValue::from(0.5_f64)],
);
```

---

## `ParameterValue` 与 `CircuitParam`

操作参数在构造层和存储层使用不同表示。

| 类型 | 说明 |
| --- | --- |
| `ParameterValue` | 构造层参数表示，可为固定数值或完整 `Parameter` 表达式。 |
| `CircuitParam` | 存储层参数表示，可为固定数值，也可为所属 `Circuit` 参数表中的索引。 |

常见转换示例：

```rust
use cqlib_core::circuit::{Parameter, ParameterValue};

let fixed = ParameterValue::from(0.5_f64);
let symbolic = ParameterValue::from(Parameter::symbol("theta"));
let also_symbolic = ParameterValue::from("phi");
```

---

## 手动构造并导入线路

`ValueOperation` 常用于从外部格式构造线路，或作为编译器 pass 输出的中间结果。下面的示例展示如何手动构造操作序列并通过 `Circuit::from_operations()` 创建线路。

```rust
use cqlib_core::circuit::{
    Circuit, ParameterValue, Qubit, StandardGate, ValueOperation,
};

let ops = vec![
    ValueOperation::from_standard(StandardGate::H, [Qubit::new(0)], []),
    ValueOperation::from_standard(StandardGate::CX, [Qubit::new(0), Qubit::new(1)], []),
    ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(1)],
        [ParameterValue::from("theta")],
    ),
];

let circuit = Circuit::from_operations(
    vec![Qubit::new(0), Qubit::new(1)],
    ops,
    None,
    None,
)?;

assert!(circuit.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

---

## `label` 标签

`label` 是附加在操作实例上的可选元数据，用于记录调试信息、导入源位置、校准标签、可视化名称或编译 pass 标记。

```rust
use cqlib_core::circuit::{ParameterValue, Qubit, StandardGate, ValueOperation};

let mut op = ValueOperation::from_standard(
    StandardGate::RZ,
    [Qubit::new(0)],
    [ParameterValue::from(0.25_f64)],
);

op.label = Some("calibrated-rz".into());
```

在编译优化中，是否保留、修改或丢弃 `label` 应作为元数据处理策略，而不是线路语义的一部分。

---

## 验证边界

`Circuit::append()` 和 `Circuit::from_operations()` 会在追加或导入时检查常见结构错误，包括：

- 量子比特数量是否匹配指令 arity；
- 参数数量是否匹配指令定义；
- 操作中的量子比特是否重复；
- 操作中的量子比特是否属于当前线路；
- 固定参数是否为有限数值；
- 经典数据句柄是否属于当前线路；
- 控制流中的作用域、跳转和经典值读取是否满足约束。

对于自动生成的 IR，建议在进入编译器后续阶段或后端执行前调用：

```rust
circuit.validate()?;
```

这样可以尽早发现 IR 构造问题，避免错误延迟到矩阵转换、设备映射或真实后端执行阶段。