# qcis

`cqlib.ir.qcis` 提供 QCIS 文本与 `Circuit` 之间的双向转换接口。

## 导入

```python
from cqlib.ir import qcis
```

---

## 函数

### qcis.loads(qcis)

从 QCIS 字符串解析电路。

参数：

- `qcis` (`str`)：QCIS 文本。

返回：

- `Circuit`

异常情况：

- `ValueError`：QCIS 语法不合法或解析失败（`QCIS parse error: ...`）。

示例：

```python
from cqlib.ir import qcis

qcis_code = """H Q0
CZ Q0 Q1
"""
circuit = qcis.loads(qcis_code)
```

### qcis.load(path)

从 QCIS 文件解析电路。

参数：

- `path` (`str`)：QCIS 文件路径。

返回：

- `Circuit`

异常情况：

- `OSError`：文件读取失败（不存在、权限不足等）。
- `ValueError`：文件内容 QCIS 解析失败（`QCIS parse error: ...`）。

说明：

- `load` 读取“文件路径”；
- `loads` 读取“字符串内容”。

### qcis.dumps(circuit)

将电路序列化为 QCIS 字符串。

参数：

- `circuit` (`Circuit`)

返回：

- `str`

异常情况：

- `ValueError`：导出失败（常见为门不受支持，错误前缀 `QCIS dump error: ...`）。

### qcis.dump(circuit, path)

将电路序列化并写入 QCIS 文件。

参数：

- `circuit` (`Circuit`)
- `path` (`str`)

返回：

- `None`

异常情况：

- `OSError`：文件写入失败。
- `ValueError`：导出失败（线路中含 QCIS 无法表达的门等，错误前缀 `QCIS dump error: ...`）。

## 注意事项

`qcis.dumps` / `qcis.dump` 只支持 QCIS 原生门集。遇到非原生门会报错，需先分解/编译到 QCIS 门集再导出。

当前 QCIS 原生门集为：

- 单比特门：`H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z`
- 含参单比特门：`RX`, `RXY`, `RY`, `RZ`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE`
- 多比特门：`RXX`, `RYY`, `RZX`, `RZZ`, `SWAP`, `CX`, `CCX`, `CY`, `CZ`, `CRX`, `CRY`, `CRZ`, `FSIM`

除门之外，QCIS 文本还表示测量（`M`）、屏障（`B`）与延迟（`I Qn t`，`t` 为延迟刻度数，单位 0.5 ns，必须是非负整数）。

`SDG` 与 `TDG` 在解析时作为 `SD` 与 `TD` 的等价写法接受，导出时统一写成 `SD` 与 `TD`。标准恒等门与全局相位不在 QCIS 的原生门集内，导出时会报错；QCIS 的 `I` 一律表示延迟，不是恒等门。

## 示例

```python
from cqlib.ir import qcis

qcis_code = """H Q0
CZ Q0 Q1
"""
circuit = qcis.loads(qcis_code)

print(qcis.dumps(circuit))

qcis.dump(circuit, "input.qcis")

# 1) 读文件
c = qcis.load("input.qcis")

# 2) 处理电路（示例：分解）
c2 = c.decompose()

# 3) 导出文本或文件
text = qcis.dumps(c2)
qcis.dump(c2, "output.qcis")
```
