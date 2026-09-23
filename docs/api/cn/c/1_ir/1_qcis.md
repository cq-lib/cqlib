# qcis

提供 QCIS 文本与 `Circuit` 之间的双向转换。QCIS 是中国电信量子计算机的原生指令格式，基于原生门集（`H`、`X2P`、`RZ`、`CZ` 等）。

---

## 函数

### qcis_loads(source)

从 QCIS 字符串解析线路。

参数：

- `source` (`const char *`)：QCIS 文本。

返回：

- `CCircuit *`：堆分配线路，需 `circuit_free` 释放；失败返回 NULL。

错误码：

- `-4`：QCIS 语法不合法或解析失败。

示例：

```c
const char *src = "H Q0\nX2P Q1\nCZ Q0 Q1\n";
CCircuit *qc = qcis_loads(src);
circuit_free(qc);
```

### qcis_load(path)

从 QCIS 文件解析线路。

参数：

- `path` (`const char *`)：QCIS 文件路径。

返回：

- `CCircuit *`：失败返回 NULL。

错误码：

- `-5`：文件读取失败（不存在、权限不足等）。
- `-4`：文件内容 QCIS 解析失败。

### qcis_dumps(circuit)

将线路导出为 QCIS 字符串。

参数：

- `circuit` (`const CCircuit *`)：线路。

返回：

- `char *`：堆分配的 QCIS 文本，需用 `cqlib_string_free` 释放；失败返回 NULL。

### qcis_dump(circuit, path)

将线路写入 QCIS 文件。

参数：

- `circuit` (`const CCircuit *`)：线路。
- `path` (`const char *`)：输出文件路径。

返回：

- `int32_t`；成功 `0`，IO 失败 `-5`。

---

## 注意事项

QCIS 导出只支持 QCIS 原生门集，遇到非原生门会导出失败，需先经 [compile](../4_compile/1_compile.md) 或 `circuit_decompose` 分解到原生门集再导出。

当前 QCIS 原生门集为：

- `X2P`, `X2M`, `Y2P`, `Y2M`, `XY2P`, `XY2M`
- `CZ`, `RZ`, `I`, `X`, `Y`, `Z`, `H`, `S`, `SD`, `T`, `TD`
- `RX`, `RY`, `RXY`

`load` 读取"文件路径"，`loads` 读取"字符串内容"；`dump` 写文件，`dumps` 返回字符串，四者共享同一套解析/序列化实现。

---

## 示例

### 解析与导出

```c
const char *src = "H Q0\nX2P Q1\nCZ Q0 Q1\n";
CCircuit *qc = qcis_loads(src);
if (!qc) { return 1; }

printf("qubits=%zu ops=%zu\n",
       (size_t)circuit_num_qubits(qc),
       (size_t)circuit_num_operations(qc));

char *out = qcis_dumps(qc);
printf("%s\n", out);
cqlib_string_free(out);

circuit_free(qc);
```

### 文件读写与分解

```c
qcis_dump(qc, "input.qcis");              // 写文件

CCircuit *c = qcis_load("input.qcis");   // 读文件
CCircuit *c2 = circuit_decompose(c);     // 分解复合门

char *text = qcis_dumps(c2);
cqlib_string_free(text);
qcis_dump(c2, "output.qcis");

circuit_free(c2);
circuit_free(c);
```
