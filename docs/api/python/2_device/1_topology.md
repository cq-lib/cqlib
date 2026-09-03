# Topology

`cqlib.device.Topology` 用于描述硬件物理比特与耦合边关系。  
在 Python 绑定中，它是 `cqlib.compiler.Topology` 的同一类型别名。

## 导入

```python
from cqlib.device import Topology
```

---

## 构造

### `Topology(qubits, couplings)`

参数：

- `qubits` (`list[int]`)：物理比特 ID 列表。
- `couplings` (`list[tuple[int, int] | tuple[int, int, str]]`)：耦合边列表。
  - `(u, v)`：门名默认 `CX`
  - `(u, v, name)`：显式指定耦合门名

说明：

- 拓扑按有向边语义存储，`(u, v)` 与 `(v, u)` 是两条不同耦合。

### `Topology.line(qubits)`

静态方法，快速构造线性拓扑（相邻比特以 `CX` 连接）。

## 属性

- `num_qubits -> int`
- `num_couplings -> int`
- `qubits -> list[int]`

## 修改方法

### `add_qubits(qubits)`

添加物理比特。

异常情况：

- `ValueError`：存在重复比特。

### `add_couplings(couplings)`

添加耦合边。

异常情况：

- `ValueError`：边端点比特不在拓扑中。

### `remove_qubits(qubits)`

删除物理比特（关联耦合边会被一并删除）。

异常情况：

- `ValueError`：待删除比特不存在。

### `remove_couplings(couplings)`

删除耦合边。参数格式为 `list[tuple[int, int]]`。

异常情况：

- `ValueError`：待删除耦合不存在，或端点比特不存在。

## 查询方法

- `is_connected(u, v) -> bool`
- `neighbors(qubit) -> list[int]`
- `get_coupling_name(u, v) -> str | None`
- `contains_qubit(qubit) -> bool`
- `degree(qubit) -> int`

说明：

- `neighbors/degree` 基于出边统计。

## 示例

```python
from cqlib.device import Topology

topo = Topology([0, 1, 2], [(0, 1), (1, 2, "CZ")])

assert topo.num_qubits == 3
assert topo.num_couplings == 2
assert topo.is_connected(1, 2) is True
assert topo.get_coupling_name(1, 2) == "CZ"
assert topo.neighbors(1) == [2]

topo.add_qubits([3])
topo.add_couplings([(2, 3, "CX")])
topo.remove_couplings([(2, 3)])
topo.remove_qubits([3])
```

