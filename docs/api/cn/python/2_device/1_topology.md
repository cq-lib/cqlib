# Topology

`cqlib.device.Topology` 用于描述硬件物理比特与耦合边关系。  
`cqlib` 顶层也重导出了同一类型，可从 `cqlib` 直接导入。

## 导入

```python
from cqlib.device import Topology
```

---

## 构造

### `Topology(qubits, couplings)`

参数：

- `qubits` (`list[int | Qubit | PhysicalQubit]`)：物理比特列表。
- `couplings` (`list[tuple[control, target, name]]`)：有向耦合列表，每项为 `(control, target, name)` 三元组。
  - `control -> target`：耦合方向。
  - `name`：耦合的名称标签，仅作标识，不参与连接性与重复判定。

说明：

- 拓扑按有向边语义存储，`(control, target)` 与 `(target, control)` 是两条不同耦合。
- 同一有序比特对只允许一条耦合，名称不同也算重复；重复比特与自耦合都会被拒绝。

### `Topology.line(qubits)`

静态方法，按给定顺序构造有向直线拓扑 `qubits[0] -> qubits[1] -> ...`，耦合数为 `len(qubits) - 1`，耦合名为空串。

异常情况：

- `ValueError`：比特列表中存在重复比特。

## 属性

- `num_qubits -> int`
- `num_couplings -> int`
- `qubits -> list[PhysicalQubit]`

## 修改方法

### `add_qubits(qubits)`

添加物理比特。

异常情况：

- `ValueError`：待添加列表中比特重复，或与拓扑中已有比特重复。

### `add_couplings(couplings)`

添加有向耦合，参数为 `(control, target, name)` 三元组列表。

异常情况：

- `ValueError`：端点比特不在拓扑中、耦合已存在或出现自耦合。

### `remove_qubits(qubits)`

删除物理比特（关联耦合边会被一并删除）。

异常情况：

- `ValueError`：待删除比特不存在或重复出现。

### `remove_couplings(couplings)`

删除有向耦合。参数格式为 `list[tuple[control, target]]`，只删除指定方向，反向耦合不受影响。

异常情况：

- `ValueError`：待删除耦合不存在或重复出现。

## 查询方法

- `supports_directed_coupling(control, target) -> bool`：`control -> target` 方向是否存在耦合。
- `supports_coupling_either_direction(a, b) -> bool`：两个比特之间任一方向是否存在耦合。
- `successors(qubit) -> list[PhysicalQubit]`：出边邻居。
- `predecessors(qubit) -> list[PhysicalQubit]`：入边邻居。
- `neighbors_undirected(qubit) -> list[PhysicalQubit]`：无向邻居，两个方向只出现一次。
- `undirected_edges() -> list[tuple[PhysicalQubit, PhysicalQubit]]`：无向边列表，每对按键值排序，方向相反的两条耦合折叠为一条。
- `get_coupling_name(control, target) -> str | None`：耦合名，耦合不存在时为 `None`。
- `contains_qubit(qubit) -> bool`：比特是否在拓扑中。
- `out_degree(qubit) -> int`：出边数。
- `in_degree(qubit) -> int`：入边数。

说明：

- `successors` / `out_degree` 基于出边统计，`predecessors` / `in_degree` 基于入边统计。
- 比特不在拓扑中时，邻居类查询返回空列表，度数查询返回 `0`，不抛异常。

## 其他行为

- 支持 `copy`、`deepcopy`。
- `repr(topology) == "Topology(num_qubits=3, num_couplings=2)"`。

## 示例

```python
from cqlib.device import PhysicalQubit, Topology

topo = Topology([0, 1, 2], [(0, 1, "G1"), (1, 2, "G2")])

assert topo.num_qubits == 3
assert topo.num_couplings == 2
assert topo.supports_directed_coupling(1, 2) is True
assert topo.supports_directed_coupling(2, 1) is False
assert topo.supports_coupling_either_direction(2, 1) is True
assert topo.get_coupling_name(1, 2) == "G2"
assert set(topo.successors(1)) == {PhysicalQubit(2)}
assert set(topo.neighbors_undirected(1)) == {PhysicalQubit(0), PhysicalQubit(2)}
assert topo.out_degree(1) == 1
assert topo.in_degree(1) == 1

topo.add_qubits([3])
topo.add_couplings([(2, 3, "G2")])
assert topo.supports_coupling_either_direction(2, 3) is True
topo.remove_couplings([(2, 3)])
topo.remove_qubits([3])
assert topo.num_qubits == 3
```
