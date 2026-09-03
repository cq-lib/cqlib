# Layout

`Layout` 用于维护路由过程中的逻辑比特与物理比特映射关系。

## 导入

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{Layout, LayoutError};
use std::collections::HashMap;
```

## 构造

### `Layout::new(logical, physical, init_map) -> Result<Layout, LayoutError>`

参数：

- `logical: Vec<Qubit>`
- `physical: Vec<Qubit>`
- `init_map: Option<HashMap<Qubit, Qubit>>`（逻辑 -> 物理）

常见错误：

- `LayoutError::TooManyLogicalQubits`
- `LayoutError::InvalidVirtualQubit`
- `LayoutError::InvalidPhysicalQubit`
- `LayoutError::DuplicatePhysicalMapping`

## 只读接口

- `num_logical(&self) -> usize`
- `num_ancilla(&self) -> usize`
- `num_physical(&self) -> usize`
- `get_physical(&self, virtual_id: Qubit) -> Option<Qubit>`
- `get_virtual(&self, physical_id: Qubit) -> Option<Qubit>`
- `logical_qubits(&self) -> impl Iterator<Item = Qubit>`
- `ancilla_qubits(&self) -> impl Iterator<Item = Qubit>`
- `physical_qubits(&self) -> impl Iterator<Item = Qubit>`
- `v2p_map(&self) -> &HashMap<Qubit, Qubit>`
- `p2v_map(&self) -> &HashMap<Qubit, Qubit>`

## 更新接口

### `swap_physical(&mut self, phys_a: Qubit, phys_b: Qubit)`

交换两个物理比特上的虚拟比特映射。

注意：

- 若 `phys_a/phys_b` 不在布局物理集合内，当前实现会触发 `panic!`（断言失败）。

## 示例

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::Layout;
use std::collections::HashMap;

let logical = vec![Qubit::new(0), Qubit::new(1)];
let physical = vec![Qubit::new(10), Qubit::new(11), Qubit::new(12)];

let mut init = HashMap::new();
init.insert(Qubit::new(0), Qubit::new(11));

let mut layout = Layout::new(logical, physical, Some(init)).unwrap();
assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);

let before_11 = layout.get_virtual(Qubit::new(11));
let before_12 = layout.get_virtual(Qubit::new(12));
layout.swap_physical(Qubit::new(11), Qubit::new(12));
let after_11 = layout.get_virtual(Qubit::new(11));
let after_12 = layout.get_virtual(Qubit::new(12));

assert_eq!(before_11, after_12);
assert_eq!(before_12, after_11);
```

