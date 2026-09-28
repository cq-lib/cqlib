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

在上述示例中，`bell.to_gate("Bell")` 会将双比特 Bell 子线路封装为一个名为 `Bell` 的 `CircuitGate`。随后，该复合门可以像普通门一样追加到其他线路中，并作用在指定的量子比特上。

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
| `symbols` | `list[str]` | 子线路内部按插入顺序登记的符号名，可能包含已不再被引用的符号。 |
| `used_symbols` | `list[str]` | 冻结后实际被引用的符号名。 |

`FrozenCircuit.used_symbols` 可用于了解该子线路是否为参数化线路。需要注意 `symbols` 与 `used_symbols` 的区别：前者是一份稳定的符号登记表，删除或替换操作之后其中可能残留不再被引用的符号；后者只包含当前实际被引用的符号。省略参数签名构造的 `CircuitGate` 会以 `used_symbols` 作为位置参数签名，因此由它生成的复合门在应用时也需要按照 `used_symbols` 的顺序传入位置参数。

---

## CircuitGate

`CircuitGate` 是由 `FrozenCircuit` 定义的复合门。它将一段子线路包装为一个门对象，使其可以通过 `Circuit.append_circuit_gate()` 添加到其他线路中。

```python
CircuitGate(
    name: str,
    circuit: FrozenCircuit,
    signature_params: list[str] | None = None,
) -> CircuitGate
```

| 参数 | 说明 |
|---|---|
| `name` | 复合门名称，主要用于显示、调试和 IR 表达。 |
| `circuit` | 用于定义该复合门的不可变线路快照。 |
| `signature_params` | 位置参数签名。省略时取 `circuit.used_symbols`；显式给出时按该列表的顺序绑定参数。显式签名中允许出现子线路未引用的参数名，这些参数仍然计入门的参数数量。 |

示例：

```python
from cqlib.circuit.gates import CircuitGate

gate = CircuitGate("Bell", frozen)
```

| 属性 | 类型 | 说明 |
|---|---|---|
| `name` | `str` | 复合门名称。 |
| `num_qubits` | `int` | 应用该门时需要提供的量子比特数量。 |
| `num_params` | `int` | 应用该门时需要提供的位置参数数量，等于 `signature_params` 的长度。 |
| `signature_params` | `list[str]` | 位置参数签名，决定应用该门时位置参数的绑定顺序与数量。 |
| `used_symbols` | `list[str]` | 该门所依据的子线路中实际被引用的符号名。 |
| `symbols` | `list[str]` | `used_symbols` 的向后兼容别名，不代表位置参数的绑定顺序。 |
| `circuit` | `FrozenCircuit` | 该复合门内部持有的不可变线路定义。 |

---

## 显式参数签名

当门的位置参数需要与子线路中实际出现的符号解耦时，可以使用静态方法 `CircuitGate.with_signature()` 显式声明调用签名：

```python
CircuitGate.with_signature(
    name: str,
    circuit: FrozenCircuit,
    signature_params: list[str],
) -> CircuitGate
```

```python
from cqlib import Circuit, Parameter
from cqlib.circuit.gates import CircuitGate, FrozenCircuit

body = Circuit(1)
body.rx(0, Parameter("used"))

frozen_body = FrozenCircuit(body.qubits, body.operations)

gate = CircuitGate.with_signature("Declared", frozen_body, ["used", "declared_unused"])
```

显式声明签名时，`signature_params` 中不允许出现重复的参数名；同时子线路中实际引用的每一个符号都必须已在 `signature_params` 中声明。上述两种情形都会抛出 `CircuitError`。

被声明但子线路并未引用的参数仍然计入门的参数数量，因此 `gate.num_params` 等于 `signature_params` 的长度，而不是 `gate.used_symbols` 的长度。

---

## 参数化子线路门

如果子线路中包含符号参数，`CircuitGate` 会记录这些符号参数，并在应用时按照 `gate.signature_params` 的顺序进行位置绑定。

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

需要注意的是，`CircuitGate` 的参数绑定是按位置绑定。也就是说，应用时传入的第 `i` 个参数会替换 `gate.signature_params` 中第 `i` 个符号。

```python
print(gate.signature_params)
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

`inverse()` 会返回一个新的 `CircuitGate`。新门的底层冻结线路由原子线路逐操作取逆并反向排列得到，同时新门的名称在原名称后追加 `_dg` 后缀，位置参数签名保持不变。

```python
inverse_gate = gate.inverse()
```

该接口适用于可逆子线路。如果子线路定义中包含测量、`reset` 或其他不可逆操作，例如经典数据操作、控制流，以及无法取逆的指令，反演过程会抛出 `CircuitError`。并且，`inverse()` 不会修改原始 `CircuitGate`，而是返回一个新的复合门定义。

---

## 分解

`Circuit.decompose()` 可以将线路中的 `CircuitGate` 展开回其内部定义的基础操作序列。

```python
outer = Circuit(1)
outer.append_circuit_gate(gate, [0], params=[0.2])

expanded = outer.decompose()
```

分解后，复合门会被替换为其内部的原始操作。对于参数化 `CircuitGate`，应用时传入的位置参数会在分解过程中替换到内部操作中。

---

## 其他行为

- `FrozenCircuit` 与 `CircuitGate` 都支持结构相等比较 `==`。`FrozenCircuit` 的比较忽略线路标识与符号矩阵缓存，只比较定义本身；`CircuitGate` 的比较包含名称、有序签名与底层定义线路。
- `repr` 形如 `FrozenCircuit(qubits=2, operations=2)` 与 `CircuitGate("Bell", qubits=2, params=0)`。
- 两者都支持 `copy`、`deepcopy`。由于二者都是不可变对象，复制结果与原对象相等。

