# qasm2

提供 OpenQASM 2.0 文本与 `Circuit` 之间的双向转换。

---

## 函数

### qasm2_loads(source)

从 OpenQASM 2.0 字符串解析线路。

参数：

- `source` (`const char *`)：QASM2 文本，通常以 `OPENQASM 2.0;` 头开始。

返回：

- `CCircuit *`：堆分配线路，需 `circuit_free` 释放；失败返回 NULL。

错误码：

- `-4`：QASM2 语法不合法或解析失败。

### qasm2_load(path)

从 QASM2 文件解析线路。

参数：

- `path` (`const char *`)：QASM2 文件路径。

返回：

- `CCircuit *`：失败返回 NULL。

错误码：

- `-5`：文件读取失败（不存在、权限不足等）。
- `-4`：文件内容解析失败。

### qasm2_dumps(circuit)

将线路导出为 QASM2 字符串。

参数：

- `circuit` (`const CCircuit *`)：线路。

返回：

- `char *`：堆分配的 QASM2 文本（含 `OPENQASM 2.0;` 头与寄存器声明），需用 `cqlib_string_free` 释放；失败返回 NULL。

### qasm2_dump(circuit, path)

将线路写入 QASM2 文件。

参数：

- `circuit` (`const CCircuit *`)：线路。
- `path` (`const char *`)：输出文件路径。

返回：

- `int32_t`；成功 `0`，IO 失败 `-5`。

---

## 说明

- 比特编号按寄存器声明顺序从 `0` 开始，与 `Circuit` 的逻辑编号一致；
- `load`/`loads` 与 `dump`/`dumps` 共享同一套解析/序列化实现，`loads(dumps(c))` 可用于往返校验；
- 导出只覆盖支持的门集，线路含不支持的操作时导出失败返回 NULL。

---

## 示例

### 解析与导出

```c
const char *src =
    "OPENQASM 2.0;\n"
    "include \"qelib1.inc\";\n"
    "qreg q[2];\n"
    "h q[0];\n"
    "cx q[0], q[1];\n";

CCircuit *qc = qasm2_loads(src);
if (!qc) { return 1; }

char *text = qasm2_dumps(qc);      // 导出回 QASM2 字符串
printf("%s\n", text);
cqlib_string_free(text);

circuit_free(qc);
```

### 文件读写往返

```c
qasm2_dump(qc, "bell.qasm");                     // 写文件
CCircuit *qc2 = qasm2_load("bell.qasm");   // 从文件读回
printf("ops=%zu\n", (size_t)circuit_num_operations(qc2));
circuit_free(qc2);
circuit_free(qc);
```

跨格式互转借助 `loads` / `dumps` 即可完成（如 QASM2 → QCIS）：

```c
CCircuit *qc = qasm2_loads(qasm_src);
char *qcis_text = qcis_dumps(qc);       // QASM2 -> QCIS
cqlib_string_free(qcis_text);
circuit_free(qc);
```
