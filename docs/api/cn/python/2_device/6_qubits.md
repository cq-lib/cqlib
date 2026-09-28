# 量子比特标识

`cqlib.device`

`LogicalQubit` 与 `PhysicalQubit` 是设备侧接口使用的强类型比特标识：前者标识线路中的逻辑比特，后者标识设备上的物理比特。两个类型都以数值标识为内部表示，但类型不同，因此数值相同的两个标识不相等。

## 导入

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit
```

---

## LogicalQubit

逻辑比特标识，用于线路中的逻辑比特。

### LogicalQubit(id)

参数：

- `id` (`int`)：数值标识，非负整数。

异常情况：

- `TypeError`：`id` 不是整数。
- `OverflowError`：`id` 为负数或超出 `u32` 范围。

### 属性

- `id -> int`：数值标识。
- `index -> int`：数值标识的索引形式。
- `qubit -> Qubit`：对应的线路比特。

### 其他行为

- 支持 `==`、`<`、`<=`、`>`、`>=`、`hash`、`copy`、`deepcopy`。
- 比较只在本类型之间成立：与 `Qubit` 或 `PhysicalQubit` 比较时判定为不相等，不抛异常。
- `str(LogicalQubit(0)) == "L0"`；`repr(LogicalQubit(0)) == "LogicalQubit(0)"`。

示例：

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit

logical = LogicalQubit(0)
assert logical.id == 0
assert logical.index == 0
assert logical.qubit == Qubit(0)
assert logical != Qubit(0)
assert logical != PhysicalQubit(0)
assert str(logical) == "L0"
assert repr(logical) == "LogicalQubit(0)"

assert sorted([LogicalQubit(2), LogicalQubit(0)]) == [LogicalQubit(0), LogicalQubit(2)]
assert LogicalQubit(2) >= LogicalQubit(2)
```

---

## PhysicalQubit

物理比特标识，用于设备上的比特位置。

### PhysicalQubit(id)

参数：

- `id` (`int`)：数值标识，非负整数。

异常情况：

- `TypeError`：`id` 不是整数。
- `OverflowError`：`id` 为负数或超出 `u32` 范围。

### 属性

- `id -> int`：数值标识。
- `index -> int`：数值标识的索引形式。
- `qubit -> Qubit`：对应的线路比特。

### 其他行为

- 支持 `==`、`<`、`<=`、`>`、`>=`、`hash`、`copy`、`deepcopy`。
- 比较只在本类型之间成立：与 `Qubit` 或 `LogicalQubit` 比较时判定为不相等，不抛异常。
- `str(PhysicalQubit(11)) == "P11"`；`repr(PhysicalQubit(11)) == "PhysicalQubit(11)"`。

示例：

```python
from cqlib import Qubit
from cqlib.device import LogicalQubit, PhysicalQubit

physical = PhysicalQubit(11)
assert physical.id == 11
assert physical.index == 11
assert physical.qubit == Qubit(11)
assert physical == PhysicalQubit(11)
assert physical != Qubit(11)
assert physical != LogicalQubit(11)
assert str(physical) == "P11"
assert repr(physical) == "PhysicalQubit(11)"

assert sorted([PhysicalQubit(2), PhysicalQubit(0)]) == [PhysicalQubit(0), PhysicalQubit(2)]
assert PhysicalQubit(1) < PhysicalQubit(2)
```

---

## 在布局中的角色

布局维护逻辑比特到物理比特的映射，是两个类型配合使用的主要场景：逻辑比特来自线路，物理比特来自设备，映射结果以强类型标识记录，避免两类数值混用。

示例：

```python
from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit

layout = Layout(logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)})

assert layout.num_logical == 2
assert layout.num_physical == 3
assert layout.num_vacant_physical == 1

assert set(layout.logical_qubits) == {LogicalQubit(0), LogicalQubit(1)}
assert set(layout.physical_qubits) == {PhysicalQubit(10), PhysicalQubit(11), PhysicalQubit(12)}

assert layout.get_physical(0) == PhysicalQubit(11)
assert layout.get_logical(11) == LogicalQubit(0)
assert layout.l2p_map == {LogicalQubit(0): PhysicalQubit(11), LogicalQubit(1): PhysicalQubit(10)}
assert layout.p2l_map == {PhysicalQubit(10): LogicalQubit(1), PhysicalQubit(11): LogicalQubit(0)}

before_11 = layout.get_logical(11)
before_12 = layout.get_logical(12)
layout.swap_physical(11, 12)
assert layout.get_logical(11) == before_12
assert layout.get_logical(12) == before_11

layout.unbind(0)
assert layout.get_physical(0) is None
```

---

## 接受比特参数的接口

设备侧接口在接收比特参数时接受 `int` 或标识对象，也接受 `Qubit`，均按数值标识解析；返回的比特一律是强类型标识对象。逻辑比特参数位置接受 `int | LogicalQubit | Qubit`，物理比特参数位置接受 `int | PhysicalQubit | Qubit`。

- 布局 `Layout`：构造参数 `logical`、`physical`、`init_map`，以及 `get_physical`、`get_logical`、`bind`、`unbind`、`is_physical_vacant`、`swap_physical`。
- 拓扑 `Topology`：构造参数 `qubits`、`couplings`，以及 `add_qubits`、`remove_qubits`、`add_couplings`、`remove_couplings`、`contains_qubit`、`successors`、`predecessors`、`neighbors_undirected`、`out_degree`、`in_degree`、`supports_directed_coupling`、`supports_coupling_either_direction`、`get_coupling_name`。
- 设备 `Device`：构造参数 `qubits` 与 `line_from_qubits` 的参数，以及 `add_qubit_properties`、`add_edge_properties`、`qubit_properties`、`edge_properties`、`get_t1`、`get_t2`、`get_readout_error`、`is_usable_qubit`、`single_qubit_error`、`two_qubit_error`、`edge_error`、`supports_native_instruction`、`set_invalid_qubits`。
- 噪声 `NoiseModel`：`add_readout_error`、`add_single_qubit_error`、`add_two_qubit_error`、`get_readout_error`；`OperationKey` 的 `new_single`、`new_double`、`new_triple`。
