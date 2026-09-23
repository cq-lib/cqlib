# 运行时经典状态

`cqlib.qis.state`

`ClassicalState` 与 `RuntimeValue` 描述线路执行过程中产生的运行时经典数据。`RuntimeValue` 是单个经典值在一次执行中的带类型取值；`ClassicalState` 是一次执行结束后全部运行时经典数据的快照，按句柄查询取值。两者都由模拟器在执行完成后给出，没有公开的构造方法。

## 导入

```python
from cqlib.qis.state import ClassicalState, RuntimeValue
```

---

## RuntimeValue

单个经典值的运行时取值。取值种类由 `kind` 区分，静态类型由 `ty` 给出。

### 属性

- `kind -> str`：取值种类，为 `"bit"`、`"bool"`、`"uint"`、`"bit_vec"` 之一。
- `ty -> ClassicalType`：该取值对应的经典类型。

### 方法

- `to_bitstring() -> str | None`：取值为 `bit` 或 `bit_vec` 时返回比特串，其余种类返回 `None`。
- `as_bit() -> bool`：按 `bit` 读出取值。
- `as_bool() -> bool`：按 `bool` 读出取值。
- `as_uint() -> int`：按 `uint` 读出取值。
- `as_bitvec_outcome() -> Outcome`：按 `bit_vec` 读出取值，返回测量回执。

### 其他行为

- 相等性按值判定：种类、位宽与载荷都相同才相等；与其它类型比较一律为假。
- 相等的取值哈希相同，可直接放入集合去重。
- 支持 `copy`、`deepcopy`。
- `repr` 形如 `RuntimeValue.bit(true)`、`RuntimeValue.uint(width=8, value=3)`、`RuntimeValue.bit_vec(width=2, bits='10')`。

### 异常情况

- `TypeError`：把 `as_bit`、`as_bool`、`as_uint`、`as_bitvec_outcome` 用于不匹配的取值种类时抛出。

---

## ClassicalState

一次线路执行结束后的运行时经典状态快照，按经典值句柄或经典变量句柄查询取值。

### 方法

- `value(value) -> RuntimeValue | None`：查询某个不可变经典值本次执行产生的运行时取值；没有取值时返回 `None`。`value` 为 `ClassicalValue`，通常取自测量回执。
- `var(var) -> RuntimeValue | None`：查询某个可变经典变量的当前运行时取值；没有取值时返回 `None`。`var` 为 `ClassicalVar`。

### 其他行为

- 没有公开构造方法，只能从执行结果取得。
- 支持 `copy`、`deepcopy`。
- `repr` 固定为 `ClassicalState()`。

---

## 示例

### 查询测量产生的取值

```python
from cqlib.circuit import Circuit
from cqlib.qis.state import StabilizerState

circuit = Circuit(2)
circuit.x(0)
measurement = circuit.measure(0)
circuit.reset(0)
circuit.h(1)

result = StabilizerState.run_circuit(circuit)
measured = result.classical.value(measurement.value)
assert measured.kind == "bit"
assert measured.as_bit() is True
assert measured.to_bitstring() == "1"
```

### 查询可变经典变量

```python
from cqlib.circuit import Circuit, ClassicalExpr, ClassicalType
from cqlib.qis.state import StabilizerState

circuit = Circuit(1)
flag = circuit.var(ClassicalType.bool())
circuit.store(flag, ClassicalExpr.bool_literal(True))

result = StabilizerState.run_circuit(circuit)
stored = result.classical.var(flag)
assert stored.kind == "bool"
assert stored.as_bool() is True
assert stored.to_bitstring() is None
```

### 取值比较与去重

同类同载荷的取值相等且哈希一致，可直接用于集合去重；不同种类的取值即使语义相近也不相等。

```python
from cqlib.circuit import Circuit
from cqlib.qis.state import StabilizerState


def measured_bit():
    circuit = Circuit(1)
    circuit.x(0)
    measurement = circuit.measure(0)
    return StabilizerState.run_circuit(circuit).classical.value(measurement.value)


first = measured_bit()
second = measured_bit()
assert first == second
assert hash(first) == hash(second)
assert len({first, second}) == 1
```

---

## 校验与错误处理

| 异常 | 触发场景 |
| --- | --- |
| `TypeError` | 用不匹配的读出方法读取 `RuntimeValue`，例如对 `bit` 取值调用 `as_uint()`。 |

经典类型、经典变量与经典值句柄的定义见 [经典数据与控制流](../0_circuit/9_classical_control_flow.md)；`Outcome` 的定义见 [结果与执行](../2_device/5_result.md)。
