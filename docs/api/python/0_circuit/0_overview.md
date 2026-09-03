# 量子电路

`cqlib.circuit`

`cqlib.circuit` 是 Cqlib 中用于构建、表示和分析量子线路的核心模块。它提供了从基础量子门、参数化线路、复合门、测量与经典数据，到结构化控制流和矩阵转换的一套统一接口，是使用 Cqlib 进行量子程序开发的主要入口。

`cqlib.circuit` 主要覆盖以下能力：

- **基础线路构造**：创建逻辑量子比特集合，并按顺序追加量子门、测量、重置、屏障等操作。
- **参数化线路建模**：使用 `Parameter` 表示门角、全局相位或其他可调参数，适用于 VQE、QAOA、量子机器学习和参数扫描等场景。
- **门与指令体系**：通过 `StandardGate`、`UnitaryGate`、`MCGate`、`CircuitGate` 等对象描述标准门、自定义门、多控制门和复合门。
- **操作级中间表示**：使用 `Instruction`、`ValueInstruction` 和 `ValueOperation` 表示“做什么”“作用在哪些量子比特上”“携带哪些参数”。
- **经典数据与控制流表示**：提供 `ClassicalType`、`ClassicalVar`、`ClassicalExpr` 和结构化控制流接口，用于描述条件分支、循环和多分支选择等程序结构。
- **矩阵与符号矩阵转换**：支持将小规模纯量子门线路转换为数值矩阵或符号矩阵，用于教学、单元测试、门验证和编译规则检查。
- **参数化模板线路**：通过 `cqlib.circuit.ansatz` 提供 TwoLocal、feature map、QAOA 和 Pauli evolution 等常用线路模板。

---

## 核心概念与术语

本文档给出 `cqlib.circuit` 文档中常见术语的简要说明。更详细的接口签名、参数约束和示例请参考后续 API 文档。

| 术语 | 说明 |
| --- | --- |
| **抽象线路** | 面向算法描述的量子线路，使用逻辑量子比特和高层操作表示量子程序，不直接绑定具体硬件拓扑或原生门集。 |
| **物理线路** | 已根据目标设备约束完成布局、路由和门集转换的线路，通常只包含后端支持的物理量子比特连接和原生操作。 |
| **量子比特** | 量子信息的基本逻辑单位。在 Cqlib Python API 中由 `Qubit` 表示，也可以在多数接口中用整数索引简写。 |
| **量子门** | 通常具有酉矩阵表示的可逆量子操作。标准门由 `StandardGate` 表示，自定义门可由 `UnitaryGate`、`MCGate` 或 `CircuitGate` 表示。 |
| **指令** | 线路中的操作类型，可以是量子门，也可以是 `barrier`、`measure`、`reset`、`delay`、经典数据操作或控制流结构。 |
| **操作** | 指令在线路中的一次具体应用，包含指令类型、作用量子比特、参数列表和可选标签。Python 构造层通常使用 `ValueOperation` 表示。 |
| **参数** | 构造期符号表达式，常用于门角、全局相位和变分线路参数。由 `Parameter` 表示，可在后续通过 `assign_parameters()` 绑定为数值。 |
| **测量** | 计算基测量操作，会产生经典结果。测量是非酉操作，不能直接参与普通酉矩阵转换。 |
| **经典数据** | 线路中用于表达条件、变量和测量结果的经典侧对象，例如 `ClassicalType`、`ClassicalVar`、`ClassicalValue` 和 `ClassicalExpr`。 |
| **控制流** | 基于经典表达式组织线路结构的机制，例如 `if_`、`while_`、`for_uint` 和 `switch`。具体后端是否支持相关语义，需要结合编译和执行流程确认。 |
| **全局相位** | 线路整体相位因子。它通常不影响单独测量概率，但在线路组合、受控操作、矩阵等价检查和编译重写中仍然具有意义。 |
| **Ansatz** | 参数化线路模板，常用于变分算法和量子机器学习。`cqlib.circuit.ansatz` 提供多种内置模板。 |

---

## `cqlib.circuit` API 概览

### 1. 核心线路容器

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `Circuit` | [Circuit](1_circuit.md) | 量子线路主容器，用于创建量子比特集合、追加量子门和非酉指令、绑定参数、组合线路、分解复合门、构造控制流和生成矩阵。 |

### 2. 量子比特与参数

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `Qubit` | [Qubit](2_qubit.md) | 轻量逻辑量子比特句柄，保存非负整数编号，可比较、可哈希。 |
| `Parameter` | [Parameter](3_parameter.md) | 符号或数值参数表达式，支持解析、求值、化简、替换、求导和保守等价判断。 |

### 3. 操作、指令与门

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `Instruction` | [Operation / Instruction](4_operation_instruction.md) | 存储层指令，用于表示标准门、多控制门、自定义酉门、子线路门、directive、delay 等操作类型。 |
| `ValueInstruction` | [Operation / Instruction](4_operation_instruction.md) | 构造层指令，可包裹普通 `Instruction` 或控制流操作。 |
| `ValueOperation` | [Operation / Instruction](4_operation_instruction.md) | 自包含操作，包含指令、量子比特、参数和可选标签，适合序列化、导入器和编译器输出。 |
| `Directive` | [Operation / Instruction](4_operation_instruction.md) | 非酉 directive，包括 `barrier`、`measure` 和 `reset`。 |

### 4. 门定义

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `StandardGate` | [StandardGate](5_gates_standard.md) | Cqlib 原生标准门集合，包括 Pauli、Clifford、旋转门、受控门、二体相互作用门和 fSim 门。 |
| `UnitaryGate` | [UnitaryGate](6_gates_unitary.md) | 用户自定义酉门，可由数值矩阵、符号矩阵或冻结线路定义。 |
| `MCGate` | [MCGate](7_gates_mc_gate.md) | 多控制标准门，可在一个 `StandardGate` 外增加任意数量控制位。 |
| `FrozenCircuit` | [CircuitGate / FrozenCircuit](8_gates_circuit_gate.md) | 不可变线路快照，通常用于复合门定义。 |
| `CircuitGate` | [CircuitGate / FrozenCircuit](8_gates_circuit_gate.md) | 由 `FrozenCircuit` 定义的复合门，可在其他线路中作为单个门复用。 |

### 5. 内置特殊指令

| 指令 | 构造方式 | 说明 |
| --- | --- | --- |
| `barrier` | `Circuit.barrier(qubits)` 或 `Directive.barrier()` | 插入编译边界，限制相关量子比特上的操作跨越该边界重排。 |
| `measure` | `Circuit.measure(qubit)`、`Circuit.measure_bits(qubits)` | 执行计算基测量，产生 `Measurement` 回执和经典结果。 |
| `reset` | `Circuit.reset(qubit)` 或 `Directive.reset()` | 将量子比特复位到 `|0>`，通常会破坏原有相干性。 |
| `delay` | `Circuit.delay(qubit, duration)` | 在指定量子比特上插入空闲时间或调度延迟。 |
| `store` | `Circuit.store(target, value)` | 将经典表达式写入经典变量。 |

### 6. 经典数据与控制流

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `CircuitId` | [Classical / Control Flow](9_classical_control_flow.md) | 电路本地经典句柄身份，用于防止跨线路误用经典变量和值。 |
| `ClassicalType` | [Classical / Control Flow](9_classical_control_flow.md) | 经典数据类型，包括 bit、bool、uint 和 bit vector。 |
| `ClassicalVar` | [Classical / Control Flow](9_classical_control_flow.md) | 可变经典存储句柄，由 `Circuit.var()` 创建。 |
| `ClassicalValue` | [Classical / Control Flow](9_classical_control_flow.md) | 不可变经典值，通常由测量产生。 |
| `Measurement` | [Classical / Control Flow](9_classical_control_flow.md) | 测量回执，包含测量值和被测量的量子比特顺序。 |
| `ClassicalExpr` | [Classical / Control Flow](9_classical_control_flow.md) | 类型化经典表达式 AST，用于条件、比较、位抽取和表达式组合。 |
| `ClassicalControlOp` | [Classical / Control Flow](9_classical_control_flow.md) | 结构化控制流 IR，包含 `if`、`while`、`for`、`switch`、`break` 和 `continue`。 |
| `ValueControlBody` | [Classical / Control Flow](9_classical_control_flow.md) | 构造层控制流体，包含若干 `ValueOperation`。 |
| `ValueSwitchCase` | [Classical / Control Flow](9_classical_control_flow.md) | 构造层 `switch` case，包含匹配值和分支体。 |

### 7. 矩阵与符号矩阵工具

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `circuit_to_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | 函数式接口，用于计算线路的稠密数值矩阵。 |
| `Circuit.to_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | 方法式接口，与 `circuit_to_matrix()` 等价。 |
| `Circuit.to_symbolic_matrix` | [Circuit To Matrix](12_circuit_to_matrix.md) | 生成保留 `Parameter` 表达式的符号矩阵。 |
| `SymbolicComplex` | [SymbolicMatrix](10_symbolic_matrix.md) | 实部和虚部均为 `Parameter` 的符号复数。 |
| `SymbolicMatrix` | [SymbolicMatrix](10_symbolic_matrix.md) | 密集符号矩阵，适合小规模线路分析和自定义符号门。 |

### 8. 线路模板与 ansatz

| 对象 | 页面 | 简介 |
| --- | --- | --- |
| `EntanglementTopology` | [Ansatz](11_ansatz.md) | 纠缠拓扑，包括 linear、circular、full 和 custom。 |
| `TwoLocal` | [Ansatz](11_ansatz.md) | 旋转层与纠缠层交替的硬件友好 ansatz。 |
| `AngleEncoding`, `BasisEncoding` | [Ansatz](11_ansatz.md) | 基础数据编码线路。 |
| `ZFeatureMap`, `IQPFeatureMap`, `ZZFeatureMap`, `PauliFeatureMap` | [Ansatz](11_ansatz.md) | 量子机器学习 feature map 模板。 |
| `BasicEntanglerLayers`, `StronglyEntanglingLayers` | [Ansatz](11_ansatz.md) | 常见层状可训练线路模板。 |
| `QAOAAnsatz` | [Ansatz](11_ansatz.md) | QAOA cost/mixer 交替结构。 |
| `EvolutionStrategy`, `EvolutionInfo`, `PauliEvolutionAnsatz` | [Ansatz](11_ansatz.md) | Hamiltonian 时间演化线路模板。 |
| `real_amplitudes`, `efficient_su2`, `zz_feature_map`, `pauli_feature_map` | [Ansatz](11_ansatz.md) | 常用模板的便捷构造函数。 |

---

## Circuit 表示方式

`Circuit` 内部同时记录量子比特集合、参数信息、经典数据和操作序列。

```text
Circuit
├── id: CircuitId
├── qubits: list[Qubit]
├── parameters: list[Parameter]
├── symbols: list[str]
├── global_phase: Parameter
├── classical_vars: list[ClassicalType]
├── classical_values: list[ClassicalType]
└── operations: list[ValueOperation]
    ├── instruction: ValueInstruction
    │   ├── Instruction(...)
    │   └── ClassicalControlOp(...)
    ├── qubits: list[Qubit]
    ├── params: list[float | Parameter]
    └── label: str | None
```

### 1. 量子数据

量子数据由 `Qubit` 标识。`Circuit(3)` 会创建逻辑量子比特 `0`、`1`、`2`；`Circuit([0, 2, 4])` 则会创建稀疏逻辑编号。

```python
from cqlib import Circuit, Qubit

circuit = Circuit([Qubit(10), Qubit(20)])
assert circuit.num_qubits == 2
```

### 2. 操作与指令

一条线路操作由“做什么”和“作用在哪里”共同决定。`Instruction` 描述操作类型，例如 `H` 门、`RZ` 门、测量或控制流；`ValueOperation` 则进一步绑定作用量子比特、参数和可选标签。

```python
from cqlib import Circuit, Qubit
from cqlib.circuit import ValueOperation
from cqlib.circuit.gates import StandardGate

op = ValueOperation.from_standard_gate(StandardGate.H, [Qubit(0)])
circuit = Circuit.from_operations([Qubit(0)], [op])
```

这种分层表示适合导入导出、反序列化、编译 pass 输出和底层测试。对于普通用户，直接使用 `Circuit.h()`、`Circuit.cx()` 等便捷方法通常更简单。

### 3. 参数

`Parameter` 用于构造参数化线路。线路会自动收集其中出现的自由符号，供后续 `assign_parameters()` 进行数值绑定。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
circuit = Circuit(1)
circuit.rx(0, theta)

assert circuit.symbols == ["theta"]
bound = circuit.assign_parameters({"theta": 0.5})
```

### 4. 经典数据与控制流

经典数据由 `ClassicalType`、`ClassicalVar`、`ClassicalValue` 和 `ClassicalExpr` 等对象表示。

```python
from cqlib import Circuit

circuit = Circuit(2)
measurement = circuit.measure(0)

circuit.if_(
    measurement.expr().to_bool(),
    lambda body: body.x(1),
)
```

---

## 快速示例

### 1. Bell 态制备

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()
```

该示例创建两比特 Bell 态线路，并计算其小规模矩阵表示。

### 2. 参数化旋转层

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")
phi = Parameter("phi")

circuit = Circuit(1)
circuit.rx(0, theta)
circuit.rz(0, theta + phi / 2)

bound = circuit.assign_parameters({"theta": 0.1, "phi": 0.2})
```

该示例展示如何使用 `Parameter` 构造可重复绑定的参数化线路模板。

### 3. 可复用子线路门

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(2)
block.rx(0, theta)
block.cx(0, 1)

block_gate = block.to_gate("Block")

main = Circuit(4)
main.append_circuit_gate(block_gate, [0, 1], params=[Parameter("a")])
main.append_circuit_gate(block_gate, [2, 3], params=[Parameter("b")])
```

该示例将一段参数化子线路封装为 `CircuitGate`，并在更大的线路中复用。

### 4. 基于测量结果的条件结构

```python
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
result = circuit.measure(0)

circuit.if_(
    result.expr().to_bool(),
    lambda body: body.x(1),
)
```

该示例展示如何基于测量结果构造条件分支。实际后端是否支持此类结构，需要结合目标设备和编译流程确认。

---

## 校验与错误处理

高层 `Circuit` 方法会尽早检查常见错误，例如量子比特不存在、重复量子比特、门作用比特数量不匹配、参数数量不匹配等。对于从外部 IR 导入、程序自动生成或手动拼装的线路，建议显式调用：

```python
circuit.validate()
```

常见异常包括：

| 异常 | 触发场景 |
| --- | --- |
| `CircuitError` | 线路结构或操作非法，例如量子比特不存在、门作用对象错误、非酉操作求矩阵、控制流作用域错误等。 |
| `ParameterError` | 参数表达式解析、绑定、化简或求值失败。 |
| `QubitError` | 量子比特编号非法，例如负数、超出内部范围或输入类型不正确。 |
| `CqlibError` | Cqlib 专用异常基类，可用于统一捕获 Cqlib 相关异常。 |
