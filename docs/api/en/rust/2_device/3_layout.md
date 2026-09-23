# Layout

`Layout` maintains the mapping between logical qubits and physical qubits during routing.

## Import

```rust
use std::collections::BTreeMap;

use cqlib_core::device::{Layout, LayoutError, LogicalQubit, PhysicalQubit};
```

## Construction

### `Layout::new(logical, physical, init_map) -> Result<Layout, LayoutError>`

Parameters:

- `logical: Vec<LogicalQubit>`
- `physical: Vec<PhysicalQubit>`
- `init_map: Option<BTreeMap<LogicalQubit, PhysicalQubit>>` (logical -> physical)

Notes:

- Logical qubits that do not appear in `init_map` are mapped to the remaining vacant physical qubits in input order.

Common errors:

- `LayoutError::TooManyLogicalQubits`
- `LayoutError::DuplicateLogicalQubit`
- `LayoutError::DuplicatePhysicalQubit`
- `LayoutError::InvalidLogicalQubit`
- `LayoutError::InvalidPhysicalQubit`

### `Layout::from_pairs(logical_physical, physical_count) -> Result<Layout, LayoutError>`

Parameters:

- `logical_physical: &[(u32, u32)]`: `(logical index, physical index)` pairs.
- `physical_count: u32`: the total number of physical qubits; physical qubits are `0..physical_count`.

Notes:

- The logical qubits are the logical indices appearing in `logical_physical`; physical qubits that are not referenced are vacant physical qubits.

## Read-only interface

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

## Update interface

- `bind(&mut self, logical: LogicalQubit, physical: PhysicalQubit) -> Result<(), LayoutError>`
- `unbind(&mut self, logical: LogicalQubit) -> Result<PhysicalQubit, LayoutError>`
- `swap_physical(&mut self, phys_a: PhysicalQubit, phys_b: PhysicalQubit) -> Result<(), LayoutError>`

Swaps the logical qubits carried by two physical qubits, which is the basic operation for moving logical qubits during routing. Either physical qubit may be vacant: swapping with a vacant physical qubit moves a logical qubit to the vacant position.

Common errors:

- `LayoutError::LogicalQubitAlreadyBound`
- `LayoutError::PhysicalQubitAlreadyOccupied`
- `LayoutError::LogicalQubitNotBound`

Note:

- When a physical qubit is not in the layout set, `LayoutError::InvalidPhysicalQubit` is returned and no panic is triggered.

## Example

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
