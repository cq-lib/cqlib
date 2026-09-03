# Layout

`Layout` 用于维护逻辑比特（virtual qubit）到物理比特（physical qubit）的双向映射。

## 导入

```python
from cqlib.device import Layout
```

---

## 构造函数

### `Layout(logical, physical, init_map=None)`

参数：

- `logical` (`list[int]`)：逻辑比特 ID 列表
- `physical` (`list[int]`)：物理比特 ID 列表
- `init_map` (`dict[int, int] | None`)：可选初始映射（逻辑 -> 物理）

异常情况：

- `ValueError`：输入不合法（例如逻辑比特多于物理比特、`init_map` 非法映射等）。

## 属性

- `num_logical -> int`
- `num_ancilla -> int`
- `num_physical -> int`
- `logical_qubits -> list[int]`
- `ancilla_qubits -> list[int]`
- `physical_qubits -> list[int]`
- `v2p_map -> dict[int, int]`
- `p2v_map -> dict[int, int]`

## 方法

### `get_physical(virtual_id) -> int | None`

根据虚拟比特查物理比特。

### `get_virtual(physical_id) -> int | None`

根据物理比特查虚拟比特。

### `swap_physical(phys_a, phys_b) -> None`

交换两个物理比特上的虚拟比特映射。

异常情况：

- `ValueError`：`phys_a` 或 `phys_b` 不在布局的物理比特集合中。

## 示例

```python
from cqlib.device import Layout

layout = Layout(logical=[0, 1], physical=[10, 11, 12], init_map={0: 11})

print(layout.num_logical)   # 2
print(layout.num_ancilla)   # 1
print(layout.v2p_map)

before_11 = layout.get_virtual(11)
before_12 = layout.get_virtual(12)
layout.swap_physical(11, 12)
after_11 = layout.get_virtual(11)
after_12 = layout.get_virtual(12)

print(before_11, before_12)
print(after_11, after_12)
```

