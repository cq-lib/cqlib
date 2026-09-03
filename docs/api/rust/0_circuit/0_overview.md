# 量子电路

`cqlib_core::circuit`

`cqlib_core::circuit` 是 Cqlib Rust core 中的量子电路中间表示模块，负责提供类型安全的线路构造、量子比特管理、参数表达式、门定义、经典数据、结构化控制流、控制流图、矩阵转换以及参数化线路模板等基础能力。

`cqlib_core::circuit` 主要提供以下能力：

- **量子线路构造**：通过 `Circuit` 创建逻辑量子比特集合，并追加标准门、自定义门、复合门、测量、reset、barrier 等操作。
- **参数化表达式**：通过 `Parameter` 表示门角、全局相位和变分线路参数，支持表达式解析、求值、化简、替换、求导和等价判断。
- **操作级 IR**：通过 `Instruction`、`Operation`、`ValueInstruction` 和 `ValueOperation` 表示线路中的指令和操作，支持构造层 IR 与存储层 IR 分离。
- **门定义体系**：提供 `StandardGate`、`UnitaryGate`、`MCGate`、`CircuitGate` 和 `FrozenCircuit` 等门定义类型，覆盖标准门、自定义酉门、多控制门和子线路复合门。
- **经典数据与控制流**：通过 `ClassicalType`、`ClassicalExpr`、`ClassicalControlOp` 等类型描述测量结果、经典变量和结构化控制流。
- **控制流图分析**：通过 `CircuitCFG`、`BasicBlock`、`Terminator` 等类型为编译器 pass 提供控制流图视图。
- **矩阵与符号矩阵转换**：支持将小规模纯量子线路转换为数值矩阵或保留参数的符号矩阵，并支持全局相位等价检查。
- **参数化线路模板**：通过 `ansatz` 模块提供 VQE、QAOA、feature map 和 Hamiltonian evolution 等常见模板。

---

## API 导航

| 分类 | 页面 | 主要对象 | 说明 |
| --- | --- | --- | --- |
| 电路容器 | [Circuit](1_circuit.md) | `Circuit` | 线路构造、门方法、参数绑定、反演、分解、组合和矩阵转换。 |
| 量子比特 | [Qubit](2_qubit.md) | `Qubit`, `QubitError` | `u32` 逻辑量子比特句柄与安全整数转换。 |
| 参数系统 | [Parameter](3_parameter.md) | `Parameter`, `ParameterError`, `EvalError` | 符号表达式、求值、求导、替换和化简。 |
| 操作 IR | [Operation / Instruction](4_operation_instruction.md) | `Instruction`, `Operation`, `ValueInstruction`, `ValueOperation` | 存储层 IR 与构造层 IR。 |
| 标准门 | [Standard Gates](5_gate_standard.md) | `StandardGate` | 原生门枚举、门元数据、矩阵和反门。 |
| 自定义酉门 | [Unitary Gates](6_gate_unitary.md) | `UnitaryGate`, `UnitaryMatrix` | 数值矩阵门、符号矩阵门和 circuit-backed gate。 |
| 多控制门 | [Multi-Controlled Gates](7_gate_mc_gate.md) | `MCGate` | 在标准门前添加控制位，表达多控制门语义。 |
| 子线路门 | [Circuit Gates](8_gate_circuit_gate.md) | `FrozenCircuit`, `CircuitGate` | 将线路冻结并封装为可复用复合门。 |
| 经典数据与控制流 | [Classical / Control Flow](9_classical_control_flow.md) | `ClassicalType`, `ClassicalExpr`, `ClassicalControlOp` | 测量、经典数据、表达式和结构化控制流。 |
| 符号矩阵 | [Symbolic Matrix](10_symbolic_matrix.md) | `SymbolicComplex`, `SymbolicMatrix` | 保留 `Parameter` 的密集符号矩阵和等价检查。 |
| Ansatz | [Ansatz](11_ansatz.md) | `Ansatz`, `TwoLocal`, `QAOAAnsatz`, `PauliEvolutionAnsatz` | 参数化线路模板。 |
| 控制流图 | [CFG](12_cfg.md) | `CircuitCFG`, `BasicBlock`, `Terminator` | 线路控制流图视图、分析和重建。 |
| 矩阵转换 | [Circuit To Matrix](13_circuit_to_matrix.md) | `circuit_to_matrix`, `Circuit::to_matrix` | 数值矩阵转换、量子比特顺序和全局相位。 |

---

## 核心 IR 分层

Rust core 中的 circuit IR 分为构造层和存储层。理解这一区分对于编写导入器、序列化工具和编译器 pass 很重要。

| 层次 | 操作类型 | 指令类型 | 参数类型 | 适用场景 |
| --- | --- | --- | --- | --- |
| 构造层 IR | `ValueOperation` | `ValueInstruction` | `ParameterValue` | 外部构造、序列化、导入器、编译 pass 输出。 |
| 存储层 IR | `Operation` | `Instruction` | `CircuitParam` | `Circuit` 内部紧凑存储、参数驻留和运行验证。 |

构造层 IR 是自包含的，参数以 `ParameterValue` 形式保存，可以直接跨线路传递或写入外部格式。存储层 IR 更紧凑，参数可能以 `CircuitParam::Index` 的形式引用所属 `Circuit` 的内部参数表，因此必须结合具体线路上下文解释。

典型转换关系如下：

```text
ValueOperation
    └── append / from_operations
          └── Operation stored in Circuit

Operation
    └── Circuit::index(i)
          └── ValueOperation
```

---

## 最小示例：静态线路

下面的示例创建一个两量子比特 Bell 线路，并依次添加 `H` 和 `CX` 门。

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

circuit.h(Qubit::new(0))?;
circuit.cx(Qubit::new(0), Qubit::new(1))?;

assert_eq!(circuit.operations().len(), 2);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`Circuit::new(2)` 会创建逻辑量子比特 `Qubit(0)` 和 `Qubit(1)`。大多数高层门方法都会返回 `Result<(), CircuitError>`，用于在追加操作时尽早暴露量子比特不存在、参数数量不匹配或门 arity 不匹配等错误。

---

## 最小示例：参数化线路

参数化线路允许门角使用符号表达式表示。构造完成后，可以通过 `assign_parameters()` 绑定具体数值，并继续进行矩阵转换或后续编译。

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use std::collections::HashMap;

let theta = Parameter::symbol("theta");

let mut circuit = Circuit::new(1);
circuit.rx(Qubit::new(0), theta)?;

let mut bindings = HashMap::new();
bindings.insert("theta", std::f64::consts::FRAC_PI_2);

let bound = circuit.assign_parameters(&Some(bindings))?;
let matrix = bound.to_matrix(None)?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`assign_parameters()` 会返回新的线路对象，原始参数化线路仍可作为模板继续复用。对于 VQE、QAOA 和参数扫描任务，建议保留模板线路，并为不同参数组生成绑定后的线路。

---

## 最小示例：经典数据与控制流

Rust core 支持在电路中表示测量、经典表达式和结构化控制流。下面的示例测量第 `0` 个量子比特，并根据测量结果条件性地对第 `1` 个量子比特施加 `X` 门。

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let mut circuit = Circuit::new(2);

let m = circuit.measure(Qubit::new(0))?;

circuit.if_(m.expr().to_bool()?, |body| {
    body.x(Qubit::new(1))?;
    Ok(())
})?;

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

这类线路包含运行时经典数据和控制流结构，通常不再具有单一固定的无条件酉矩阵表示。因此，包含测量或控制流的线路不应直接调用 `to_matrix()`，除非已经提取出其中的纯量子子线路。

具体后端是否支持这类控制流结构，需要结合目标编译器、IR 和执行后端确认。

---

## 最小示例：从构造层 IR 创建线路

当线路来自外部格式、序列化数据或编译器 pass 输出时，可以先构造 `ValueOperation` 列表，再通过 `Circuit::from_operations()` 生成线路。

```rust
use cqlib_core::circuit::{
    Circuit,
    ParameterValue,
    Qubit,
    StandardGate,
    ValueOperation,
};

let ops = vec![
    ValueOperation::from_standard(StandardGate::H, [Qubit::new(0)], []),
    ValueOperation::from_standard(
        StandardGate::RZ,
        [Qubit::new(0)],
        [ParameterValue::from("theta")],
    ),
];

let circuit = Circuit::from_operations(
    vec![Qubit::new(0)],
    ops,
    None,
    None,
)?;

assert!(circuit.symbols().contains("theta"));

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```

`Circuit::from_operations()` 会将构造层参数驻留到线路内部参数表中，并对量子比特、参数数量、经典句柄和控制流结构进行校验。

---

## 类型与参数

| 名称 | Rust 表达 | 使用位置 | 说明 |
| --- | --- | --- | --- |
| 逻辑量子比特 | `Qubit` | 线路、门操作、矩阵顺序 | 内部保存 `u32` 编号，是逻辑标识。 |
| 参数表达式 | `Parameter` | 门角、全局相位、符号矩阵 | 构造期符号表达式。 |
| 构造层参数 | `ParameterValue` | `ValueOperation` | 可为固定数值或完整 `Parameter`。 |
| 存储层参数 | `CircuitParam` | `Operation` | 可为固定数值或线路参数表索引。 |
| 经典类型 | `ClassicalType` | 经典变量、测量值、表达式 | 包括 `Bit`、`Bool`、`UInt` 和 `BitVec`。 |
| 经典表达式 | `ClassicalExpr` | 控制流条件、比较、选择 | 运行时经典侧表达式 AST。 |

受控门通常采用“控制位在前、目标位在后”的顺序。例如，多控制 X 门应用到 `[q0, q1, q2]` 时，通常表示 `q0` 和 `q1` 为控制位，`q2` 为目标位。

---

## 错误与返回值

Rust API 不使用异常。可能失败的接口通常返回 `Result<_, CircuitError>`、`Result<_, ParameterError>` 或相关错误类型。调用者应使用 `?` 传播错误，或根据具体错误类型进行处理。

| 错误类型 | 说明 |
| --- | --- |
| `CircuitError` | 电路构造、验证、矩阵转换、反演、控制流和门定义错误。 |
| `ParameterError` | 参数表达式解析、化简、替换、求导或符号求值错误。 |
| `EvalError` | 参数数值求值错误。 |
| `QubitError` | 将整数转换为 `Qubit` 时出现负数或越界。 |

常见触发场景包括：

- 操作引用了线路中不存在的量子比特；
- 同一次操作重复使用同一个量子比特；
- 门作用量子比特数量与门定义不匹配；
- 参数数量与门定义不匹配；
- 固定参数为 `NaN` 或无穷大；
- 对包含未绑定参数的线路请求数值矩阵；
- 对测量、reset、delay 或控制流请求矩阵或反演；
- 控制流中的经典值越作用域使用。

对于外部导入、程序自动生成或编译器 pass 输出的线路，建议在进入后续流程前调用：

```rust
circuit.validate()?;
```
