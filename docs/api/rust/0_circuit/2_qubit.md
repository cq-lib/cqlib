# Qubit

`cqlib_core::circuit::Qubit`

```rust
use cqlib_core::circuit::Qubit;
```

`Qubit` 是 Rust core 中用于表示逻辑量子比特的轻量级句柄。它内部保存一个 `u32` 编号，用于在量子线路、编译 IR、映射表和集合结构中稳定标识某个逻辑量子比特。

---

## 创建 `Qubit`

```rust
pub const fn new(id: u32) -> Self
```

`Qubit::new(id)` 根据一个 `u32` 编号创建逻辑量子比特。由于输入类型已经是无符号 32 位整数，因此该构造函数本身不会失败，并且可以在常量上下文中使用。

```rust
use cqlib_core::circuit::Qubit;

let q0 = Qubit::new(0);
let q5 = Qubit::new(5);
```
---

## 访问编号

`Qubit` 提供两个常用编号访问接口：

| 方法 | 返回 | 说明 |
| --- | --- | --- |
| `id()` | `u32` | 返回原始内部编号。 |
| `index()` | `usize` | 将编号转换为 `usize`，便于在需要数组下标类型的场景中使用。 |

```rust
use cqlib_core::circuit::Qubit;

let q = Qubit::new(12);

assert_eq!(q.id(), 12);
assert_eq!(q.index(), 12usize);
assert_eq!(format!("{q}"), "Q12");
```

---

## 整数转换

`Qubit` 支持从不会溢出的无符号整数类型直接转换。例如，`u8`、`u16` 和 `u32` 可以通过 `From` 转换为 `Qubit`。

```rust
use cqlib_core::circuit::Qubit;

let a: Qubit = 0u8.into();
let b: Qubit = 10u16.into();
let c: Qubit = 20u32.into();

assert_eq!(a, Qubit::new(0));
assert_eq!(b, Qubit::new(10));
assert_eq!(c, Qubit::new(20));
```

对于可能为负数或可能超出 `u32` 范围的整数类型，应使用 `TryFrom`。

```rust
use cqlib_core::circuit::{Qubit, QubitError};

assert_eq!(Qubit::try_from(3i32).unwrap(), Qubit::new(3));

assert!(matches!(
    Qubit::try_from(-1i32),
    Err(QubitError::NegativeIndex(-1))
));
```

常见转换错误如下：

| 错误 | 说明 |
| --- | --- |
| `QubitError::NegativeIndex(i128)` | 输入为有符号整数，且值为负数。 |
| `QubitError::IndexTooLarge(u128)` | 输入值无法表示为 `u32`。 |

---

## 比较、排序与哈希

`Qubit` 的比较和哈希均基于内部编号。也就是说，两个 `Qubit` 只要编号相同，就被视为同一个逻辑量子比特句柄。

```rust
use std::collections::{BTreeSet, HashMap};
use cqlib_core::circuit::Qubit;

assert_eq!(Qubit::new(0), Qubit::new(0));
assert!(Qubit::new(0) < Qubit::new(1));

let mut map = HashMap::new();
map.insert(Qubit::new(0), "ancilla");
assert_eq!(map.get(&Qubit::new(0)), Some(&"ancilla"));

let mut set = BTreeSet::new();
set.insert(Qubit::new(2));
set.insert(Qubit::new(0));
set.insert(Qubit::new(1));

let ordered: Vec<_> = set.into_iter().collect();
assert_eq!(ordered, vec![Qubit::new(0), Qubit::new(1), Qubit::new(2)]);
```

---

## 逻辑编号与存储位置

`Qubit` 的编号是逻辑标识。在线路中，量子比特的实际顺序由 `Circuit` 保存的量子比特列表决定；在矩阵或状态向量中，轴顺序也由相关转换接口的量子比特顺序约定决定。

例如，下面的线路只包含两个量子比特，但它们的逻辑编号分别是 `10` 和 `20`：

```rust
use cqlib_core::circuit::{Circuit, Qubit};

let circuit = Circuit::from_qubits(vec![Qubit::new(10), Qubit::new(20)])?;

assert_eq!(circuit.num_qubits(), 2);
assert_eq!(circuit.qubits(), vec![Qubit::new(10), Qubit::new(20)]);

# Ok::<(), cqlib_core::circuit::CircuitError>(())
```