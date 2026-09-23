# Topology

`Topology` 是 `cqlib_core::device` 中的硬件连接图结构，使用有向图表示比特耦合关系。

## 导入

```rust
use cqlib_core::device::{PhysicalQubit, Topology, TopologyError};
```

## 构造

### `Topology::new(qubits, coupling_map) -> Result<Topology, TopologyError>`

参数：

- `qubits: Vec<PhysicalQubit>`
- `coupling_map: Vec<(PhysicalQubit, PhysicalQubit, String)>`

说明：

- 边按有向语义处理，`(q0, q1)` 与 `(q1, q0)` 不等价。
- 同一有序比特对只允许一条耦合，名称不同也算重复；重复比特与自耦合返回错误。

### `Topology::line(qubits) -> Result<Topology, TopologyError>`

参数：

- `qubits: Vec<PhysicalQubit>`

说明：

- 按给定顺序构造有向直线 `qubits[0] -> qubits[1] -> ...`，耦合数为 `len(qubits) - 1`，耦合名为空串。

## 只读接口

- `graph(&self) -> &StableGraph<PhysicalQubit, String>`
- `num_qubits(&self) -> usize`
- `num_couplings(&self) -> usize`
- `qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `supports_directed_coupling(&self, control: PhysicalQubit, target: PhysicalQubit) -> bool`
- `supports_coupling_either_direction(&self, a: PhysicalQubit, b: PhysicalQubit) -> bool`
- `successors(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `predecessors(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `neighbors_undirected(&self, qubit: PhysicalQubit) -> impl Iterator<Item = PhysicalQubit>`
- `undirected_edges(&self) -> impl Iterator<Item = (PhysicalQubit, PhysicalQubit)>`
- `get_coupling_name(&self, control: PhysicalQubit, target: PhysicalQubit) -> Option<String>`
- `contains_qubit(&self, qubit: &PhysicalQubit) -> bool`
- `out_degree(&self, qubit: &PhysicalQubit) -> usize`
- `in_degree(&self, qubit: &PhysicalQubit) -> usize`

说明：

- `successors` / `out_degree` 基于出边统计，`predecessors` / `in_degree` 基于入边统计。
- `undirected_edges` 的每对按键值排序，方向相反的两条耦合折叠为一条。
- 比特不在拓扑中时，邻居类查询返回空迭代，度数查询返回 `0`。

## 修改接口

- `add_qubits(&mut self, qubits: impl IntoIterator<Item = PhysicalQubit>) -> Result<(), TopologyError>`
- `add_couplings(&mut self, couplings: impl IntoIterator<Item = (PhysicalQubit, PhysicalQubit, String)>) -> Result<(), TopologyError>`
- `remove_qubits(&mut self, qubits: impl IntoIterator<Item = PhysicalQubit>) -> Result<(), TopologyError>`
- `remove_couplings(&mut self, couplings: impl IntoIterator<Item = (PhysicalQubit, PhysicalQubit)>) -> Result<(), TopologyError>`

常见错误：

- `TopologyError::QubitNotFound`
- `TopologyError::QubitAlreadyExists`
- `TopologyError::CouplingNotFound`
- `TopologyError::CouplingAlreadyExists`
- `TopologyError::SelfCoupling`
- `TopologyError::DuplicateQubitRemoval`
- `TopologyError::DuplicateCouplingRemoval`

## 示例

```rust
use cqlib_core::device::{PhysicalQubit, Topology};

let q0 = PhysicalQubit::new(0);
let q1 = PhysicalQubit::new(1);
let q2 = PhysicalQubit::new(2);

let mut topo = Topology::new(
    vec![q0, q1, q2],
    vec![(q0, q1, "CX".to_string()), (q1, q2, "CZ".to_string())],
)
.unwrap();

assert_eq!(topo.num_qubits(), 3);
assert_eq!(topo.num_couplings(), 2);
assert!(topo.supports_directed_coupling(q1, q2));
assert!(!topo.supports_directed_coupling(q2, q1));
assert!(topo.supports_coupling_either_direction(q2, q1));
assert_eq!(topo.get_coupling_name(q1, q2).as_deref(), Some("CZ"));
assert_eq!(topo.successors(q1).collect::<Vec<_>>(), vec![q2]);
assert_eq!(topo.neighbors_undirected(q1).collect::<Vec<_>>(), vec![q0, q2]);
assert_eq!(topo.out_degree(&q1), 1);
assert_eq!(topo.in_degree(&q1), 1);

topo.add_qubits([PhysicalQubit::new(3)]).unwrap();
topo.add_couplings([(q2, PhysicalQubit::new(3), "CX".to_string())])
    .unwrap();
topo.remove_couplings([(q2, PhysicalQubit::new(3))]).unwrap();
topo.remove_qubits([PhysicalQubit::new(3)]).unwrap();
assert_eq!(topo.num_qubits(), 3);
```
