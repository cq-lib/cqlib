# Layout

`Layout` 用于维护路由过程中的逻辑比特与物理比特映射关系。

## 导入

```rust
use std::collections::BTreeMap;

use cqlib_core::device::{Layout, LayoutError, LogicalQubit, PhysicalQubit};
```

## 构造

### `Layout::new(logical, physical, init_map) -> Result<Layout, LayoutError>`

参数：

- `logical: Vec<LogicalQubit>`
- `physical: Vec<PhysicalQubit>`
- `init_map: Option<BTreeMap<LogicalQubit, PhysicalQubit>>`（逻辑 -> 物理）

说明：

- 未出现在 `init_map` 中的逻辑比特按输入顺序映射到剩余的空闲物理比特。

常见错误：

- `LayoutError::TooManyLogicalQubits`
- `LayoutError::DuplicateLogicalQubit`
- `LayoutError::DuplicatePhysicalQubit`
- `LayoutError::InvalidLogicalQubit`
- `LayoutError::InvalidPhysicalQubit`

### `Layout::from_pairs(logical_physical, physical_count) -> Result<Layout, LayoutError>`

参数：

- `logical_physical: &[(u32, u32)]`：`(逻辑编号, 物理编号)` 对。
- `physical_count: u32`：物理比特总数，物理比特为 `0..physical_count`。

说明：

- 逻辑比特即 `logical_physical` 中出现的逻辑编号；未被引用的物理比特是空闲物理比特。

## 只读接口

- `num_logical(&self) -> usize`
- `num_physical(&self) -> usize`
- `num_vacant_physical(&self) -> usize`
- `get_physical(&self, logical: LogicalQubit) -> Option<PhysicalQubit>`
- `get_logical(&self, physical: PhysicalQubit) -> Option<LogicalQubit>`
- `logical_qubits(&self) -> impl Iterator<Item = LogicalQubit>`
- `physical_qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `vacant_physical_qubits(&self) -> impl Iterator<Item = PhysicalQubit>`
- `is_physical_vacant(&self, physical: PhysicalQubit) -> bool`
- `l2p_map(&self) -> &BTreeMap<LogicalQubit, PhysicalQubit>`
- `p2l_map(&self) -> &BTreeMap<PhysicalQubit, LogicalQubit>`

## 更新接口

- `bind(&mut self, logical: LogicalQubit, physical: PhysicalQubit) -> Result<(), LayoutError>`
- `unbind(&mut self, logical: LogicalQubit) -> Result<PhysicalQubit, LayoutError>`
- `swap_physical(&mut self, phys_a: PhysicalQubit, phys_b: PhysicalQubit) -> Result<(), LayoutError>`

交换两个物理比特上承载的逻辑比特，这是路由过程中移动逻辑比特的基本操作。任一物理比特可以空闲：与空闲物理比特交换会把逻辑比特搬到空闲位置。

常见错误：

- `LayoutError::LogicalQubitAlreadyBound`
- `LayoutError::PhysicalQubitAlreadyOccupied`
- `LayoutError::LogicalQubitNotBound`

注意：

- 物理比特不在布局集合内时返回 `LayoutError::InvalidPhysicalQubit`，不触发 panic。

## 示例

```rust
use std::collections::BTreeMap;

use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};

let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
let physical = vec![
    PhysicalQubit::new(10),
    PhysicalQubit::new(11),
    PhysicalQubit::new(12),
];
let init_map = [(LogicalQubit::new(0), PhysicalQubit::new(11))]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

let mut layout = Layout::new(logical, physical, Some(init_map)).unwrap();
assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);
assert_eq!(layout.num_vacant_physical(), 1);
assert_eq!(
    layout.get_physical(LogicalQubit::new(0)),
    Some(PhysicalQubit::new(11))
);
assert!(layout.is_physical_vacant(PhysicalQubit::new(12)));

let before_11 = layout.get_logical(PhysicalQubit::new(11));
let before_12 = layout.get_logical(PhysicalQubit::new(12));
layout
    .swap_physical(PhysicalQubit::new(11), PhysicalQubit::new(12))
    .unwrap();
assert_eq!(layout.get_logical(PhysicalQubit::new(12)), before_11);
assert_eq!(layout.get_logical(PhysicalQubit::new(11)), before_12);

assert_eq!(
    layout.unbind(LogicalQubit::new(0)).unwrap(),
    PhysicalQubit::new(12)
);
assert!(layout.is_physical_vacant(PhysicalQubit::new(12)));
```
