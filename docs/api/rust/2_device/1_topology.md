# Topology

`Topology` 是 `cqlib_core::device` 中的硬件连接图结构，使用有向图表示比特耦合关系。

## 导入

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{Topology, TopologyError};
```

## 构造

### `Topology::new(qubits, coupling_map) -> Topology`

参数：

- `qubits: Vec<Qubit>`
- `coupling_map: Vec<(Qubit, Qubit, String)>`

说明：

- 边按有向语义处理，`(q0, q1)` 与 `(q1, q0)` 不等价。

## 只读接口

- `graph(&self) -> &StableGraph<Qubit, String>`
- `num_qubits(&self) -> usize`
- `num_couplings(&self) -> usize`
- `qubits(&self) -> impl Iterator<Item = Qubit>`
- `is_connected(&self, control: Qubit, target: Qubit) -> bool`
- `neighbors(&self, qubit: Qubit) -> impl Iterator<Item = Qubit>`
- `get_coupling_name(&self, control: Qubit, target: Qubit) -> Option<String>`
- `contains_qubit(&self, qubit: &Qubit) -> bool`
- `degree(&self, qubit: &Qubit) -> usize`

## 修改接口

- `add_qubits(&mut self, qubits) -> Result<(), TopologyError>`
- `add_couplings(&mut self, couplings) -> Result<(), TopologyError>`
- `remove_qubits(&mut self, qubits) -> Result<(), TopologyError>`
- `remove_couplings(&mut self, couplings) -> Result<(), TopologyError>`

常见错误：

- `TopologyError::QubitNotFound`
- `TopologyError::QubitAlreadyExists`
- `TopologyError::CouplingNotFound`
- `TopologyError::CouplingAlreadyExists`

## 示例

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::Topology;

let q0 = Qubit::new(0);
let q1 = Qubit::new(1);
let q2 = Qubit::new(2);

let mut topo = Topology::new(
    vec![q0, q1, q2],
    vec![(q0, q1, "CX".to_string()), (q1, q2, "CZ".to_string())],
);

assert_eq!(topo.num_qubits(), 3);
assert!(topo.is_connected(q1, q2));
assert_eq!(topo.get_coupling_name(q1, q2).as_deref(), Some("CZ"));
assert_eq!(topo.neighbors(q1).collect::<Vec<_>>(), vec![q2]);

topo.add_qubits([Qubit::new(3)]).unwrap();
topo.add_couplings([(q2, Qubit::new(3), "CX".to_string())]).unwrap();
```
