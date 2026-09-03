# Circuit To Matrix

- `cqlib.circuit.circuit_to_matrix`
- `cqlib.circuit.Circuit.to_matrix`
- `cqlib.circuit.Circuit.to_symbolic_matrix`

```python
from cqlib.circuit import circuit_to_matrix
```

`Circuit To Matrix` 相关接口用于将量子线路转换为矩阵表示。Cqlib 同时提供数值矩阵和符号矩阵两类能力：数值矩阵用于得到确定的复数酉矩阵，符号矩阵用于保留线路中的 `Parameter` 表达式。

---

## 功能概览

Cqlib 提供以下三个常用入口：

| 接口 | 返回 | 说明 |
| --- | --- | --- |
| `Circuit.to_matrix(qubits_order=None)` | `numpy.ndarray[np.complex128]` | 将整条线路转换为数值复矩阵。 |
| `circuit_to_matrix(circuit, qubits_order=None)` | `numpy.ndarray[np.complex128]` | 与 `Circuit.to_matrix()` 等价的函数式入口。 |
| `Circuit.to_symbolic_matrix(qubits_order=None)` | `SymbolicMatrix` | 将线路转换为保留符号参数的符号矩阵。 |

## 数值矩阵转换

数值矩阵转换用于将一条纯量子门线路转换为确定的稠密复数矩阵。该矩阵描述了线路对量子态的整体线性变换。

```python
Circuit.to_matrix(
    qubits_order: list[int] | None = None,
) -> np.ndarray[np.complex128]

circuit_to_matrix(
    circuit: Circuit,
    qubits_order: list[int] | None = None,
) -> np.ndarray[np.complex128]
```

下面的示例构造一条 Bell 线路，并分别通过对象方法和函数式入口计算矩阵：

```python
from cqlib import Circuit
from cqlib.circuit import circuit_to_matrix

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

a = circuit.to_matrix()
b = circuit_to_matrix(circuit)

assert (a == b).all()
```

## 输出形状

如果线路包含 `n` 个量子比特，则完整矩阵的形状为：

```text
(2**n, 2**n)
```

例如，3 比特线路对应 `8 × 8` 矩阵：

```python
from cqlib import Circuit

circuit = Circuit(3)
matrix = circuit.to_matrix()

assert matrix.shape == (8, 8)
```

需要注意的是，矩阵维度与线路中的量子比特数量有关。例如，`Circuit([0, 10])` 仍然是 2 比特线路，其矩阵形状为 `(4, 4)`。

## 量子比特顺序

`qubits_order` 用于指定矩阵表示中的量子比特排列顺序。默认情况下，Cqlib 会按照线路内部保存的量子比特顺序生成矩阵。对于连续编号线路，这通常符合用户直觉；但对于稀疏逻辑编号或手动指定量子比特顺序的线路，建议显式传入 `qubits_order`，以避免矩阵解释上的歧义。

```python
from cqlib import Circuit

circuit = Circuit([0, 2])
circuit.cx(0, 2)

default = circuit.to_matrix()
reordered = circuit.to_matrix([2, 0])

print(default.shape)
print(reordered.shape)
```

在上述示例中，线路只包含逻辑量子比特 `0` 和 `2`，因此矩阵仍然是 2 比特矩阵。`qubits_order=[2, 0]` 表示在矩阵张量轴中使用相反的逻辑比特顺序。

使用 `qubits_order` 时需要满足以下要求：

- 传入的量子比特必须全部属于当前线路；
- 列表长度应与线路量子比特数量一致；
- 不能包含重复量子比特；

## 参数化线路

数值矩阵要求线路中的所有符号参数都已经绑定为具体数值。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rx(0, theta)

bound = circuit.assign_parameters({"theta": 0.5})
matrix = bound.to_matrix()
```

## 符号矩阵转换

符号矩阵转换用于保留线路中的 `Parameter` 表达式。该接口返回 `SymbolicMatrix`，矩阵元素可以包含符号复数和参数表达式。

```python
Circuit.to_symbolic_matrix(
    qubits_order: list[int] | None = None,
) -> SymbolicMatrix
```

示例：

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

circuit = Circuit(1)
circuit.rz(0, theta)

symbolic = circuit.to_symbolic_matrix()
assert "theta" in symbolic.symbols

numeric = symbolic.evaluate({"theta": 0.25})
```

在上述示例中，`symbolic` 保留了参数 `theta`。后续可以通过 `evaluate()` 传入具体参数值，将符号矩阵求值为数值矩阵。

## 典型应用场景

### 1. 检查 Bell 线路矩阵

```python
import numpy as np
from cqlib import Circuit

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)

matrix = circuit.to_matrix()

assert matrix.shape == (4, 4)
assert np.allclose(matrix.conj().T @ matrix, np.eye(4))
```

上述代码通过 `matrix.conj().T @ matrix` 检查矩阵是否接近单位矩阵，用于验证线路整体是否满足酉性。

### 2. 验证自定义门矩阵

```python
import numpy as np
from cqlib.circuit.gates import UnitaryGate

mat = np.array([[0, 1], [1, 0]], dtype=np.complex128)
gate = UnitaryGate("CustomX", 1).with_matrix(mat)

assert np.allclose(gate.matrix(), mat)
```

当用户通过 `UnitaryGate` 定义自定义门时，可以先验证门矩阵本身，再将其追加到线路中使用。

### 3. 比较复合门展开前后的矩阵

```python
from cqlib import Circuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

gate = sub.to_gate("Bell")

outer = Circuit(2)
outer.append_circuit_gate(gate, [0, 1])

assert outer.to_matrix().shape == sub.to_matrix().shape
```

对于由子线路封装得到的 `CircuitGate`，可以通过矩阵比较检查封装前后是否保持相同的量子行为。对于更严格的验证，可以进一步使用 `np.allclose()` 比较两个矩阵元素。

