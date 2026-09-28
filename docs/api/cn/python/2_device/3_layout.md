# Layout

`Layout` 用于维护逻辑比特到物理比特的映射。

## 导入

```python
from cqlib.device import Layout
```

---

## 构造函数

### `Layout(logical, physical, init_map=None)`

参数：

- `logical` (`list[int | Qubit | LogicalQubit]`)：逻辑比特列表。
- `physical` (`list[int | Qubit | PhysicalQubit]`)：物理比特列表，可以多于逻辑比特。
- `init_map` (`dict[LogicalQubit, PhysicalQubit] | None`)：可选初始映射（逻辑 -> 物理），键值也可用 `int` 或 `Qubit`。

异常情况：

- `ValueError`：逻辑比特多于物理比特、逻辑比特或物理比特重复、映射中的比特不属于布局，或同一逻辑比特、物理比特被重复映射。

### `Layout.from_pairs(pairs, physical_count)`

静态方法，由 `(逻辑编号, 物理编号)` 对与物理比特总数构造布局。物理比特为 `0..physical_count`，未被 `pairs` 引用的物理比特是空闲物理比特。

异常情况：

- `ValueError`：逻辑编号重复、物理编号重复，或物理编号超出 `0..physical_count`。

## 属性

- `num_logical -> int`
- `num_physical -> int`
- `num_vacant_physical -> int`
- `logical_qubits -> list[LogicalQubit]`
- `physical_qubits -> list[PhysicalQubit]`
- `vacant_physical_qubits -> list[PhysicalQubit]`
- `l2p_map -> dict[LogicalQubit, PhysicalQubit]`
- `p2l_map -> dict[PhysicalQubit, LogicalQubit]`

## 方法

### `get_physical(logical_id) -> PhysicalQubit | None`

根据逻辑比特查物理比特，未绑定时返回 `None`。

### `get_logical(physical_id) -> LogicalQubit | None`

根据物理比特查逻辑比特，物理比特空闲时返回 `None`。

### `is_physical_vacant(physical_id) -> bool`

物理比特是否空闲。

### `bind(logical_id, physical_id) -> None`

把逻辑比特绑定到空闲物理比特。

异常情况：

- `ValueError`：物理比特不属于布局、逻辑比特已绑定，或物理比特已被占用。

### `unbind(logical_id) -> PhysicalQubit`

解除逻辑比特的绑定，返回释放出的物理比特。

异常情况：

- `ValueError`：该逻辑比特未绑定。

### `swap_physical(phys_a, phys_b) -> None`

交换两个物理比特上承载的逻辑比特，这是路由过程中移动逻辑比特的基本操作。任一物理比特可以空闲：与空闲物理比特交换会把逻辑比特搬到空闲位置。

异常情况：

- `ValueError`：`phys_a` 或 `phys_b` 不在布局的物理比特集合中。

## 其他行为

- 支持 `copy`、`deepcopy`。
- `repr(layout) == "Layout(num_logical=2, num_vacant_physical=1, num_physical=3)"`。

## 示例

```python
from cqlib import Qubit
from cqlib.device import Layout, LogicalQubit, PhysicalQubit

layout = Layout(logical=[0, 1], physical=[10, 11, 12], init_map={Qubit(0): Qubit(11)})

print(layout.num_logical)          # 2
print(layout.num_physical)         # 3
print(layout.num_vacant_physical)  # 1

assert layout.get_physical(0) == PhysicalQubit(11)
assert layout.get_logical(11) == LogicalQubit(0)
assert layout.get_logical(12) is None
assert layout.is_physical_vacant(12) is True

before_11 = layout.get_logical(11)
before_12 = layout.get_logical(12)
layout.swap_physical(11, 12)
assert layout.get_logical(12) == before_11
assert layout.get_logical(11) == before_12

released = layout.unbind(0)
assert released == PhysicalQubit(12)
assert layout.is_physical_vacant(12) is True
```
