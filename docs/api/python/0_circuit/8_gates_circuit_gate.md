# CircuitGate / FrozenCircuit

- `cqlib.circuit.gates.FrozenCircuit`
- `cqlib.circuit.gates.CircuitGate`

```python
from cqlib.circuit.gates import FrozenCircuit, CircuitGate
```

`FrozenCircuit` 和 `CircuitGate` 用于将一段已经构造好的线路封装为可复用的复合门。二者通常配合使用：`FrozenCircuit` 用于保存一段不可变的线路定义，`CircuitGate` 则将该定义包装成可以追加到其他线路中的门对象。

---

## `Circuit`、`FrozenCircuit` 和 `CircuitGate`

可以将 `Circuit`、`FrozenCircuit` 和 `CircuitGate` 理解为三个不同的层次：

| 类型 | 作用 | 典型用途 |
|---|---|---|
| `Circuit` | 可变线路容器，可以继续追加门、测量、控制流等操作 | 构造和编辑量子线路 |
| `FrozenCircuit` | 不可变线路快照，保存一段子线路的结构定义 | 作为复合门或底层 IR 的稳定定义 |
| `CircuitGate` | 由 `FrozenCircuit` 定义的复合门 | 将子线路作为一个门复用 |

典型流程如下：

```text
Circuit  ──to_gate(name)──>  CircuitGate  ──append_circuit_gate()──>  Circuit
                                │
                                └──内部持有 FrozenCircuit 定义
```

---

## 构建方式

通过调用 `Circuit.to_gate(name)`可直接构建`CircuitGate`：

```python
Circuit.to_gate(name: str) -> CircuitGate
```

示例：将 Bell 态制备线路封装为复合门，并在更大的线路中复用。

```python
from cqlib import Circuit

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

bell_gate = bell.to_gate("Bell")

circuit = Circuit(4)
circuit.append_circuit_gate(bell_gate, [0, 1])
circuit.append_circuit_gate(bell_gate, [2, 3])
```

在上述示例中，`bell.to_gate("Bell")` 会将两比特 Bell 子线路封装为一个名为 `Bell` 的 `CircuitGate`。随后，该复合门可以像普通门一样追加到其他线路中，并作用在指定的量子比特上。

---

## FrozenCircuit

`FrozenCircuit` 表示一段不可变的线路快照。它保存子线路中的量子比特顺序、操作序列以及可选的经典类型信息。

```python
FrozenCircuit(
    qubits: list[Qubit],
    operations: list[ValueOperation],
    classical_vars: list[ClassicalType] | None = None,
    classical_values: list[ClassicalType] | None = None,
)
```

| 参数 | 说明 |
|---|---|
| `qubits` | 子线路中量子比特的存储顺序。该顺序会影响复合门应用时的量子比特映射。 |
| `operations` | 子线路中的自包含操作序列。 |
| `classical_vars` | 可选的经典变量类型表，通常用于较底层 IR 场景。 |
| `classical_values` | 可选的经典值类型表，通常用于较底层 IR 场景。 |

示例：

```python
from cqlib import Circuit
from cqlib.circuit.gates import FrozenCircuit

sub = Circuit(2)
sub.h(0)
sub.cx(0, 1)

frozen = FrozenCircuit(sub.qubits, sub.operations)
```

| 属性 | 类型 | 说明 |
|---|---|---|
| `qubits` | `list[Qubit]` | 子线路定义中保存的量子比特顺序。 |
| `num_operations` | `int` | 子线路包含的操作数量。 |
| `operations` | `list[ValueOperation]` | 子线路中的自包含操作列表。 |
| `symbols` | `list[str]` | 子线路中出现的自由符号参数名。 |

`FrozenCircuit.symbols` 可用于了解该子线路是否为参数化线路。如果冻结线路中包含符号参数，那么由它生成的 `CircuitGate` 在应用时也需要按照相应符号顺序传入位置参数。

---

## CircuitGate

`CircuitGate` 是由 `FrozenCircuit` 定义的复合门。它将一段子线路包装为一个门对象，使其可以通过 `Circuit.append_circuit_gate()` 添加到其他线路中。

```python
CircuitGate(name: str, circuit: FrozenCircuit)
```

| 参数 | 说明 |
|---|---|
| `name` | 复合门名称，主要用于显示、调试和 IR 表达。 |
| `circuit` | 用于定义该复合门的不可变线路快照。 |

示例：

```python
from cqlib.circuit.gates import CircuitGate

gate = CircuitGate("Bell", frozen)
```

| 属性 | 类型 | 说明 |
|---|---|---|
| `name` | `str` | 复合门名称。 |
| `num_qubits` | `int` | 应用该门时需要提供的量子比特数量。 |
| `num_params` | `int` | 应用该门时需要提供的位置参数数量。 |
| `symbols` | `list[str]` | 子线路中的符号参数名，决定位置参数绑定顺序。 |
| `circuit` | `FrozenCircuit` | 该复合门内部持有的不可变线路定义。 |

---

## 参数化子线路门

如果子线路中包含符号参数，`CircuitGate` 会记录这些符号参数，并在应用时按照 `gate.symbols` 的顺序进行位置绑定。

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

block = Circuit(1)
block.rx(0, theta)
block.rz(0, theta / 2)

gate = block.to_gate("ParamBlock")

circuit = Circuit(1)
circuit.append_circuit_gate(gate, [0], params=[Parameter("alpha")])
```

在上述示例中，子线路 `block` 中包含符号参数 `theta`。当它被封装为 `ParamBlock` 后，外部应用该复合门时传入的 `alpha` 会按照位置参数规则替换内部的 `theta`。

需要注意的是，`CircuitGate` 的参数绑定是按位置绑定。也就是说，应用时传入的第 `i` 个参数会替换 `gate.symbols` 中第 `i` 个符号。

```python
print(gate.symbols)
print(gate.num_params)
```

---

## 追加到 Circuit

`CircuitGate` 需要通过 `Circuit.append_circuit_gate()` 追加到线路中：

```python
Circuit.append_circuit_gate(
    gate: CircuitGate,
    qubits: list[int | Qubit],
    params: list[float | Parameter] | None = None,
) -> None
```

示例：

```python
from cqlib import Circuit, Parameter

alpha = Parameter("alpha")

sub = Circuit(1)
sub.rx(0, Parameter("theta"))

rx_block = sub.to_gate("RxBlock")

main = Circuit(1)
main.append_circuit_gate(rx_block, [0], params=[alpha])
```

追加时需要注意：

- `qubits` 的数量必须等于 `gate.num_qubits`；
- `params` 的数量必须等于 `gate.num_params`；
- `qubits` 的顺序会决定子线路内部量子比特到外部线路量子比特的映射关系；

---

## 反门

```python
CircuitGate.inverse() -> CircuitGate
```

`inverse()` 会返回一个新的 `CircuitGate`。新门的底层冻结线路由原子线路逐操作取逆并反向排列得到。

```python
inverse_gate = gate.inverse()
```

该接口适用于可逆子线路，如果子线路定义中包含测量、`reset` 或其他不可逆操作，反演过程会抛出 `CircuitError`。并且，`inverse()` 不会修改原始 `CircuitGate`，而是返回一个新的复合门定义。

---

## 分解

`Circuit.decompose()` 可以将线路中的 `CircuitGate` 展开回其内部定义的基础操作序列。

```python
outer = Circuit(1)
outer.append_circuit_gate(gate, [0], params=[0.2])

expanded = outer.decompose()
```

分解后，复合门会被替换为其内部的原始操作。对于参数化 `CircuitGate`，应用时传入的位置参数会在分解过程中替换到内部操作中。

