# OpenQASM 2.0 API

`cqlib_core::ir` 提供 OpenQASM 2.0 的导入与导出接口。

## 导入

```rust
use cqlib_core::ir::{qasm2_load, qasm2_loads, qasm2_dump, qasm2_dumps};
```

## 接口一览

- `qasm2_load(path) -> Result<Circuit, QasmParseError>`
- `qasm2_loads(source: &str) -> Result<Circuit, QasmParseError>`
- `qasm2_dump(circuit: &Circuit, path) -> io::Result<()>`
- `qasm2_dumps(circuit: &Circuit) -> Result<String, std::fmt::Error>`

## 解析（load / loads）

### `qasm2_loads`

从 QASM 字符串解析电路。

### `qasm2_load`

从文件读取并解析电路。

两者失败时返回 `QasmParseError`，常见变体：

- `IoError`
- `ParseError`
- `ConversionError`
- `UndefinedQubit`
- `UndefinedRegister`
- `UndefinedGate`
- `InvalidArgument`
- `MismatchedQubitCount`
- `MismatchedParameterCount`
- `RecursionLimitExceeded`
- `EvaluationError`

## 导出（dump / dumps）

### `qasm2_dumps`

将电路导出为 QASM 字符串。输出包含标准头部与 `qreg/creg` 声明。

### `qasm2_dump`

将导出结果写入文件，返回 `io::Result<()>`。

## 支持能力（概览）

根据当前实现，QASM2 导入导出支持：

- 标准门与参数门
- 测量、屏障、重置等指令
- `CircuitGate` 导出定义
- 部分扩展门定义输出（如 `crx/cry/rzz/rxx/ryy/rzx`）

## 最小示例

```rust
use cqlib_core::ir::{qasm2_dumps, qasm2_loads};

let qasm = r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0],q[1];
"#;

let c = qasm2_loads(qasm).unwrap();
let out = qasm2_dumps(&c).unwrap();
assert!(out.contains("OPENQASM 2.0;"));
```
