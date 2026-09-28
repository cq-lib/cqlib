# 中间表达

`cqlib_core::ir`

`cqlib_core::ir` 是 Cqlib 的中间表达模块，提供 `Circuit` 与三种线路文本格式之间的双向转换：QCIS、OpenQASM 2.0 与 OpenQASM 3.0。crate 根为每种格式导出四个函数别名，格式模块内部再按解析与导出分文件组织，并各自持有独立的错误类型。

## Overview

中间表达解决的是线路的文本化问题：线路在文件、工具与服务之间传递时是文本，而 Cqlib 内部统一用 `Circuit` 表示。`cqlib_core::ir` 的每个格式模块都提供「文本进、`Circuit` 出」与「`Circuit` 进、文本出」两个方向，转换过程不修改传入的 `Circuit`。

### crate 根别名

crate 根把每个格式的四个入口按 `<格式>_<动作>` 重新导出，它们与格式模块内的同名函数是同一个条目：

| 动作 | QCIS | OpenQASM 2.0 | OpenQASM 3.0 |
| --- | --- | --- | --- |
| 字符串 → `Circuit` | `qcis_loads` | `qasm2_loads` | `qasm3_loads` |
| 文件 → `Circuit` | `qcis_load` | `qasm2_load` | `qasm3_load` |
| `Circuit` → 字符串 | `qcis_dumps` | `qasm2_dumps` | `qasm3_dumps` |
| `Circuit` → 文件 | `qcis_dump` | `qasm2_dump` | `qasm3_dump` |

### 格式模块

每个格式模块（`cqlib_core::ir::qcis`、`cqlib_core::ir::qasm2`、`cqlib_core::ir::qasm3`）由 `load` 与 `dump` 两个子模块组成，转发同名函数并额外提供 `from_str` / `from_path` 与 `to_string` / `to_path`。`qasm2::ast` 是公开的抽象语法树模块，解析产物可以在这里单独查看。

### 错误类型

每个格式的解析与导出各有一个独立的错误枚举，crate 不做统一封装：

| 格式 | 解析错误 | 导出错误 |
| --- | --- | --- |
| QCIS | `qcis::load::QcisParseError` | `qcis::dump::QcisDumpError` |
| OpenQASM 2.0 | `qasm2::load::QasmParseError` | `qasm2::dump::QasmDumpError` |
| OpenQASM 3.0 | `qasm3::load::Qasm3ParseError` | `qasm3::dump::Qasm3DumpError` |

文件入口在 I/O 失败时返回带原始 `std::io::Error` 的 `IoError` 变体，原始错误可以用 `Error::source` 逐层取回。

### 双向转换与边界

`loads(dumps(circuit))` 得到的线路与输入线路在操作序列上等价，这就是双向转换。但文本格式的表达能力有限：

- QCIS 只接受原生门集内的门，线路中若含其他门，需先分解到该门集再导出；经典存储（`store`）与经典控制不在 QCIS 的表达范围内，测量与屏障可以表达，含符号的门参数会被原样写成符号文本。
- 导出失败时枚举会指出具体原因，例如不受支持的门、无法表达的经典操作，或不合法的延时刻度。

### 与编译的关系

导出是编译产物的可选落盘形式：线路降到目标门集之后即可用对应格式写出。格式差异只体现在门集与文本头上，`Circuit` 一侧的接口保持不变，因此也可以由一种格式读入、另一种格式写出，完成格式转换。

---

## 常用入口

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let mut circuit = Circuit::new(2);
circuit.h(Qubit::new(0)).unwrap();
circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

let qcis = qcis_dumps(&circuit).unwrap();
let parsed = qcis_loads(&qcis).unwrap();

assert_eq!(parsed.num_qubits(), 2);
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **中间表达** | `Circuit` 与线路文本格式之间的表示层，由 `cqlib_core::ir` 下的格式模块提供。 |
| **解析** | 文本到 `Circuit` 的方向，对应 `loads`（字符串）与 `load`（文件）。 |
| **导出** | `Circuit` 到文本的方向，对应 `dumps`（字符串）与 `dump`（文件）。 |
| **双向转换** | 先导出再解析回到等价线路，即 `loads(dumps(circuit))`。 |
| **格式转换** | 一种格式读入、另一种格式写出，例如把 OpenQASM 2.0 文本写成 QCIS 文本。 |
| **原生门集** | 文本格式自身定义的门集合；导出时线路中的门必须都能落在该集合内。 |

---

## `cqlib_core::ir` API 概览

### crate 根别名

| 名字 | 简介 |
| --- | --- |
| [`qcis_loads`](1_qcis.md) / [`qcis_load`](1_qcis.md) | 从 QCIS 字符串或文件解析 `Circuit`。 |
| [`qcis_dumps`](1_qcis.md) / [`qcis_dump`](1_qcis.md) | 把 `Circuit` 导出为 QCIS 字符串或文件。 |
| [`qasm2_loads`](2_qasm2.md) / [`qasm2_load`](2_qasm2.md) | 从 OpenQASM 2.0 字符串或文件解析 `Circuit`。 |
| [`qasm2_dumps`](2_qasm2.md) / [`qasm2_dump`](2_qasm2.md) | 把 `Circuit` 导出为 OpenQASM 2.0 字符串或文件。 |
| [`qasm3_loads`](3_qasm3.md) / [`qasm3_load`](3_qasm3.md) | 从 OpenQASM 3.0 字符串或文件解析 `Circuit`。 |
| [`qasm3_dumps`](3_qasm3.md) / [`qasm3_dump`](3_qasm3.md) | 把 `Circuit` 导出为 OpenQASM 3.0 字符串或文件。 |

### 格式模块

| 名字 | 简介 |
| --- | --- |
| [`cqlib_core::ir::qcis`](1_qcis.md) | QCIS 格式，含 `load` / `dump` 子模块与 `from_str`、`to_string` 等辅助入口。 |
| [`cqlib_core::ir::qasm2`](2_qasm2.md) | OpenQASM 2.0 格式，另含公开的 `ast` 子模块。 |
| [`cqlib_core::ir::qasm3`](3_qasm3.md) | OpenQASM 3.0 格式。 |

### 错误类型

| 名字 | 简介 |
| --- | --- |
| [`QcisParseError`](1_qcis.md) / [`QcisDumpError`](1_qcis.md) | QCIS 解析与导出错误。 |
| [`QasmParseError`](2_qasm2.md) / [`QasmDumpError`](2_qasm2.md) | OpenQASM 2.0 解析与导出错误。 |
| [`Qasm3ParseError`](3_qasm3.md) / [`Qasm3DumpError`](3_qasm3.md) | OpenQASM 3.0 解析与导出错误。 |

---

## 快速示例

### 1. 解析 QCIS 文本并回写

```rust
use cqlib_core::ir::{qcis_dumps, qcis_loads};

let qcis = r#"
        RX Q0 1.0
        RY Q1 pi/2
        X Q0
        CZ Q0 Q1
    "#;

let circuit = qcis_loads(qcis).unwrap();
assert_eq!(circuit.num_qubits(), 2);
```

### 2. 导出结果的精确文本

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::qcis_dumps;

let mut c = Circuit::new(2);
let q0 = Qubit::new(0);
let q1 = Qubit::new(1);

c.x(q0).unwrap();
c.y(q1).unwrap();
c.z(q0).unwrap();
c.h(q1).unwrap();

let qcis = qcis_dumps(&c).unwrap();
assert_eq!(qcis, "X Q0\nY Q1\nZ Q0\nH Q1\n");
```

### 3. 写入文件并检查错误来源

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::ir::qcis::dump::QcisDumpError;
use cqlib_core::ir::qcis_dump;
use std::error::Error;

let mut c = Circuit::new(1);
c.h(Qubit::new(0)).unwrap();

let path = std::env::temp_dir().join("cqlib_ir_overview.qcis");
qcis_dump(&c, &path).unwrap();

let missing = path.join("out.qcis");
let error = qcis_dump(&c, missing).unwrap_err();
assert!(matches!(error, QcisDumpError::IoError(_)));
assert!(error.source().is_some());
```

---

## 校验与错误处理

解析错误与导出错误分开定义，文件失败用 `IoError` 变体表示，其余变体是内容层面的原因。

| 错误 | 触发场景 |
| --- | --- |
| `QcisParseError::IoError` | `qcis_load` 读取文件失败。 |
| `QcisParseError::{InvalidQubitFormat, InvalidQubitId, QubitCountMismatch, ParameterCountMismatch, MissingParameter, InvalidParameter, UnknownGate, EmptyLine}` | QCIS 文本中的比特写法、未知门、门元数或参数个数不匹配，以及空行或没有有效内容。 |
| `QcisDumpError::{UnsupportedGate, UnsupportedClassicalData, UnsupportedClassicalControl, SymbolicParameter, InvalidDelayParameter}` | 导出 QCIS 时遇到无法表达的门、经典数据、经典控制、符号参数或非法延时参数。 |
| `QasmParseError::{ParseError, ConversionError, UndefinedQubit, UndefinedRegister, UndefinedGate, ReservedGateName, InvalidArgument, MismatchedQubitCount, MismatchedParameterCount, RecursionLimitExceeded, EvaluationError, CircularGateDependency, UnsupportedVersion, DuplicateDeclaration, IncludeCycle, UnsupportedOpaqueGate}` | OpenQASM 2.0 语法、语义、门定义与版本声明问题。 |
| `QasmDumpError::{FormatError, MeasureInGateNotAllowed, ResetInGateNotAllowed, InvalidQubitIndex, UnsupportedClassicalType, UnsupportedClassicalData, UnsupportedClassicalControl, ConflictingGateDefinition}` | 导出 OpenQASM 2.0 时遇到非法生成内容、门定义中的测量或复位、非法比特下标、无法表达的经典类型与重名自定义门。 |
| `Qasm3ParseError::{ParseError, SemanticError, ConversionError, UnsupportedFeature, UndefinedSymbol, UndefinedGate, TypeError, InvalidArgument, MismatchedQubitCount, MismatchedParameterCount, RecursionLimitExceeded, CircularGateDependency}` | OpenQASM 3.0 语法、语义、符号解析、类型检查与降级失败。 |
| `Qasm3DumpError::{FormatError, UnsupportedInstruction, UnsupportedClassicalData, UnsupportedClassicalControl, MeasureInGateNotAllowed, ConflictingGateDefinition}` | 导出 OpenQASM 3.0 时遇到无法表达的指令、经典数据、经典控制与重名自定义门。 |
| `*DumpError::IoError` / `*ParseError::IoError` | 文件写入或读取失败；原始 I/O 错误可用 `Error::source` 取回。 |

导出前线路必须已经落在目标格式的门集内，否则先做门分解再导出。
