# 量子比特标识

`cqlib_core::device`

`LogicalQubit` 与 `PhysicalQubit` 是设备侧接口使用的强类型比特标识：前者标识线路中的逻辑比特，后者标识设备上的物理比特。两个类型都是 `Qubit` 的新类型包装，内部表示相同，但在类型层面互相区分。

## 导入

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
```

---

## LogicalQubit

逻辑比特标识，用于线路中的逻辑比特。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct LogicalQubit(Qubit);
```

字段：

- 内部的 `Qubit` 字段为私有，不通过公开接口直接访问。

### 构造

- `LogicalQubit::new(id: u32) -> LogicalQubit`：从数值标识构造，`const fn`。
- `LogicalQubit::from_qubit(qubit: Qubit) -> LogicalQubit`：包装既有的线路比特，`const fn`。
- 实现 `From<Qubit>`，等价于 `from_qubit`。

### 方法

- `fn qubit(self) -> Qubit`：返回底层线路比特，`const fn`。
- `fn id(self) -> u32`：返回数值标识，`const fn`。

### trait 实现

- `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`、`Ord`、`PartialOrd`：派生实现，顺序与哈希基于数值标识。
- `Display`：输出 `L{id}`，例如 `L0`。
- `From<Qubit> for LogicalQubit`：等价于 `LogicalQubit::from_qubit`。
- `From<LogicalQubit> for Qubit`：等价于 `LogicalQubit::qubit`。

示例：

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::LogicalQubit;

let qubit = Qubit::new(3);
let logical = LogicalQubit::from_qubit(qubit);

assert_eq!(logical.id(), 3);
assert_eq!(logical.qubit(), qubit);
assert_eq!(Qubit::from(logical), qubit);
assert_eq!(logical.to_string(), "L3");

let ordered = [LogicalQubit::new(2), LogicalQubit::new(0)];
assert!(ordered[1] < ordered[0]);
```

---

## PhysicalQubit

物理比特标识，用于设备上的比特位置。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct PhysicalQubit(Qubit);
```

字段：

- 内部的 `Qubit` 字段为私有，不通过公开接口直接访问。

### 构造

- `PhysicalQubit::new(id: u32) -> PhysicalQubit`：从数值标识构造，`const fn`。
- `PhysicalQubit::from_qubit(qubit: Qubit) -> PhysicalQubit`：包装比特形式的标识，`const fn`。
- 实现 `From<Qubit>`，等价于 `from_qubit`。

### 方法

- `fn qubit(self) -> Qubit`：返回底层的比特标识，`const fn`。
- `fn id(self) -> u32`：返回数值标识，`const fn`。

### trait 实现

- `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`、`Ord`、`PartialOrd`：派生实现，顺序与哈希基于数值标识。
- `Display`：输出 `P{id}`，例如 `P100`。
- `From<Qubit> for PhysicalQubit`：等价于 `PhysicalQubit::from_qubit`。
- `From<PhysicalQubit> for Qubit`：等价于 `PhysicalQubit::qubit`。

示例：

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::PhysicalQubit;

let qubit = Qubit::new(11);
let physical = PhysicalQubit::new(qubit.id());

assert_eq!(physical.id(), 11);
assert_eq!(physical.qubit(), qubit);
assert_eq!(physical.to_string(), "P11");

assert!(PhysicalQubit::new(0) < PhysicalQubit::new(2));
```

---

## 在布局中的角色

布局维护逻辑比特到物理比特的映射，是两个类型配合使用的主要场景：逻辑比特来自线路，物理比特来自设备，映射结果以强类型标识记录。两个类型之间没有相互转换，也不能直接比较，数值标识相同并不表示两者是同一个标识。

示例：

```rust
use cqlib_core::device::{Layout, LogicalQubit, PhysicalQubit};
use std::collections::BTreeMap;

let logical = vec![LogicalQubit::new(0), LogicalQubit::new(1)];
let physical = vec![
    PhysicalQubit::new(100),
    PhysicalQubit::new(101),
    PhysicalQubit::new(102),
];

let init_map = [(LogicalQubit::new(1), PhysicalQubit::new(102))]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

let layout = Layout::new(logical, physical, Some(init_map)).unwrap();

assert_eq!(layout.num_logical(), 2);
assert_eq!(layout.num_physical(), 3);
assert_eq!(layout.num_vacant_physical(), 1);
assert_eq!(
    layout.get_physical(LogicalQubit::new(0)),
    Some(PhysicalQubit::new(100))
);
assert_eq!(
    layout.get_physical(LogicalQubit::new(1)),
    Some(PhysicalQubit::new(102))
);
assert_eq!(
    layout.vacant_physical_qubits().collect::<Vec<_>>(),
    vec![PhysicalQubit::new(101)]
);
```
