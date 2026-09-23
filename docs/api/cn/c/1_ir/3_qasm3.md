# qasm3

提供 OpenQASM 3.0 文本与 `Circuit` 之间的双向转换，接口形态与 [qasm2](2_qasm2.md) 完全一致。

---

## 函数

### qasm3_loads(source)

从 OpenQASM 3.0 字符串解析线路。

参数：

- `source` (`const char *`)：QASM3 文本，通常以 `OPENQASM 3.0;` 头开始，使用 `qubit[2] q;` 形式声明比特。

返回：

- `CCircuit *`：堆分配线路，需 `circuit_free` 释放；失败返回 NULL。

错误码：

- `-4`：QASM3 语法不合法或解析失败。

### qasm3_load(path)

从 QASM3 文件解析线路。

参数：

- `path` (`const char *`)：QASM3 文件路径。

返回：

- `CCircuit *`：失败返回 NULL。

错误码：

- `-5`：文件读取失败（不存在、权限不足等）。
- `-4`：文件内容解析失败。

### qasm3_dumps(circuit)

将线路导出为 QASM3 字符串。

参数：

- `circuit` (`const CCircuit *`)：线路。

返回：

- `char *`：堆分配的 QASM3 文本，需用 `cqlib_string_free` 释放；失败返回 NULL。

### qasm3_dump(circuit, path)

将线路写入 QASM3 文件。

参数：

- `circuit` (`const CCircuit *`)：线路。
- `path` (`const char *`)：输出文件路径。

返回：

- `int32_t`；成功 `0`，IO 失败 `-5`。

---

## 说明

当前导出侧重在核心门集，复杂控制流的导出保真度以 `qasm3_dumps` 实际输出为准。其余约定（比特编号、往返校验、跨格式互转）与 [qasm2](2_qasm2.md) 一致。

---

## 示例

### 解析与导出

```c
const char *src =
    "OPENQASM 3.0;\n"
    "include \"stdgates.inc\";\n"
    "qubit[2] q;\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

CCircuit *qc = qasm3_loads(src);
if (!qc) { return 1; }

char *text = qasm3_dumps(qc);
printf("%s\n", text);
cqlib_string_free(text);

qasm3_dump(qc, "bell.qasm");                     // 写文件
CCircuit *qc2 = qasm3_load("bell.qasm");   // 从文件读回
circuit_free(qc2);
circuit_free(qc);
```

### 跨格式互转

```c
CCircuit *qc = qasm3_loads(qasm3_src);
char *qasm2_text = qasm2_dumps(qc);      // QASM3 -> QASM2
cqlib_string_free(qasm2_text);
circuit_free(qc);
```
