# QCIS 支持

QCIS 是面向硬件指令和工程交付的量子线路文本格式。它以“每行一条指令”的方式描述量子操作，适合把 Cqlib 线路导出到硬件相关工具链，或者从已有 QCIS 文件中恢复 `Circuit` 进行仿真、可视化和二次编译。

对应 Python 模块：

```python
from cqlib.ir import qcis
```

## 1. API 总览

| 函数 | 用途 | 示例 |
|---|---|---|
| `qcis.loads(text)` | 从 QCIS 字符串解析 `Circuit` | `circuit = qcis.loads("H Q0\nM Q0\n")` |
| `qcis.load(path)` | 从 QCIS 文件解析 `Circuit` | `circuit = qcis.load("input.qcis")` |
| `qcis.dumps(circuit)` | 将 `Circuit` 导出为 QCIS 字符串 | `text = qcis.dumps(circuit)` |
| `qcis.dump(circuit, path)` | 将 `Circuit` 写入 QCIS 文件 | `qcis.dump(circuit, "output.qcis")` |

## 2. QCIS 文本结构

QCIS 是行式格式。每行通常由三部分构成：

```text
OPCODE QUBIT_LIST [PARAMETER_LIST]
```

示例：

```text
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0 Q1
```

含义：

- `H Q0`：在 `Q0` 上作用 Hadamard 门。
- `CZ Q0 Q1`：在 `Q0`、`Q1` 上作用 CZ 门。
- `RZ Q0 pi/2`：在 `Q0` 上作用参数为 `pi/2` 的 RZ 门。
- `M Q0 Q1`：测量 `Q0`、`Q1`。

QCIS 支持以 `//` 开头的注释，也支持行内注释。

```text
// prepare Bell state
H Q0
CZ Q0 Q1  // entangle Q0 and Q1
```

## 3. 从 QCIS 字符串加载

```python
from cqlib.ir import qcis

qcis_code = """
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0 Q1
"""

circuit = qcis.loads(qcis_code)
print(circuit.num_qubits)
print(len(circuit.operations))
```

加载后的对象是标准 Cqlib `Circuit`，可以继续用于可视化、仿真、编译优化或再次导出。

## 4. 从 QCIS 文件加载

```python
from cqlib.ir import qcis

circuit = qcis.load("input.qcis")
```

如果文件不存在，会抛出 I/O 相关异常；如果 QCIS 内容语法错误或门参数不匹配，会抛出 `ValueError`。

## 5. 导出 QCIS 字符串

```python
from cqlib import Circuit
from cqlib.ir import qcis

circuit = Circuit(2)
circuit.h(0)
circuit.cz(0, 1)
circuit.rz(0, 3.141592653589793 / 2)
circuit.measure(0)
circuit.measure(1)

text = qcis.dumps(circuit)
print(text)
```

典型输出：

```text
H Q0
CZ Q0 Q1
RZ Q0 pi/2
M Q0
M Q1
```

## 6. 导出 QCIS 文件

```python
from cqlib.ir import qcis

qcis.dump(circuit, "output.qcis")
```

写文件失败时会抛出 I/O 异常；如果线路包含 QCIS 无法表示的指令，会抛出 `ValueError`。

## 7. 支持的指令类型

当前 QCIS 模块覆盖 Cqlib 中可表示为 QCIS 文本的标准门和指令。

| 类型 | 支持内容 |
|---|---|
| 单量子比特门 | `H`, `S`, `SD`, `T`, `TD`, `X`, `X2P`, `X2M`, `Y`, `Y2P`, `Y2M`, `Z` |
| 参数化单比特门 | `RX`, `RY`, `RZ`, `RXY`, `U`, `XY`, `XY2P`, `XY2M`, `PHASE` |
| 多量子比特门 | `CX`, `CY`, `CZ`, `SWAP`, `CCX`, `CRX`, `CRY`, `CRZ`, `RXX`, `RYY`, `RZZ`, `RZX`, `FSIM` |
| 指令 | `M` 测量、`B`/`Barrier` 屏障 |
| 延迟 | `I Qn t`，表示在 `Qn` 上延迟 `t` 个 tick |

别名规则：

- `SDG` 可被加载，导出时规范化为 `SD`。
- `TDG` 可被加载，导出时规范化为 `TD`。

## 8. QCIS 中的 `I` 不是普通恒等门

QCIS 的 `I Qn t` 表示延迟指令，不是 Cqlib 标准门里的 identity gate。

```text
I Q0 10
```

含义是在 `Q0` 上空闲指定时长。Cqlib 加载后会把它表示为 `Delay` 指令。导出时，如果用户在线路中直接放了普通恒等门，QCIS dumper 会拒绝导出，避免把“无时长的恒等门”错误解释为“有时长的硬件延迟”。

## 9. 测量语义

QCIS 只描述测量指令本身，不描述 OpenQASM 那种显式 classical register 赋值。因此：

```text
M Q0 Q1
```

加载到 Cqlib 后，会变成 `Circuit` 中的测量操作；如果后续导出为 OpenQASM 3，Cqlib 会自动补充可读回的 classical 目标，例如：

```text
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
bit[2] meas;

meas[0] = measure q[0];
meas[1] = measure q[1];
```

这一步是格式转换边界的正常行为：QCIS 没有 classical 赋值语法，OpenQASM 3 有，因此 Cqlib 在导出时补齐了目标寄存器。

## 10. QCIS 到 OpenQASM 的转换

```python
from cqlib.ir import qcis, qasm3

qcis_code = """
H Q0
CZ Q0 Q1
M Q0 Q1
"""

circuit = qcis.loads(qcis_code)
qasm3_text = qasm3.dumps(circuit)
print(qasm3_text)
```

这个流程适合把硬件侧 QCIS 文件转换成更通用的 OpenQASM 3 文本，然后交给其他支持 OpenQASM 的工具读取。

## 11. 不支持或需要先处理的情况

QCIS 是硬件指令风格格式，不适合表达所有高级线路语义。以下内容通常不能直接导出为 QCIS：

- 任意矩阵形式的 `UnitaryGate`。
- 用户自定义 `CircuitGate`，除非先分解为 QCIS 支持的基础门。
- 多控制门的泛化形式，除非已经被分解。
- `if/else`、`for`、`while`、`switch` 等复杂经典控制流。
- 标准恒等门和全局相位 `GPhase`。

推荐处理方式：

```python
compiled = circuit.decompose()
text = qcis.dumps(compiled)
```

如果仍然失败，说明分解后的线路中仍包含 QCIS 无法表达的指令，需要先经过编译优化或目标门集映射。

## 12. 常见错误排查

| 现象 | 常见原因 | 处理方式 |
|---|---|---|
| `ValueError: QCIS parse error` | 文本中存在未知门、量子比特格式错误或参数数量不匹配 | 检查是否使用 `Q0` 形式，检查每个门的参数数量 |
| 导出时报 unsupported gate | `Circuit` 中包含 QCIS 不支持的门或控制流 | 先 `decompose()`，再做目标门集映射 |
| `I` 指令行为和预期不同 | QCIS 的 `I` 是 delay，不是普通 identity | 如果只是普通恒等门，不建议导出为 QCIS |
| Qubit 数量看起来变化 | Cqlib 根据实际出现的最大 qubit 编号构造线路 | 检查 QCIS 文件是否从 `Q1` 开始而没有 `Q0` |

## 下一步

- [OpenQASM 2.0 支持](2_qasm2.md)
- [OpenQASM 3.0 支持](3_qasm3.md)
- [格式转换工作流](4_conversion_workflow.md)
