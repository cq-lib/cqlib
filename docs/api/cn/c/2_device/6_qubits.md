# 量子比特标识（C）

本页覆盖设备侧强类型比特标识 `CLogicalQubit` 与 `CPhysicalQubit` 的 C ABI：前者标识线路中的逻辑比特，后者标识设备上的物理比特。两个类型内部表示相同（一个 `uint32_t` 编号），但在类型层面互相区分；两者都只与 `CQubit` 互转，互相之间不能直接转换，编号相同也不代表是同一标识。错误码与字符串释放约定遵循 [Overview](../0_overview.md)。

---

## 结构体

两个类型都按值传递，无需释放：

```c
typedef struct CLogicalQubit {
  uint32_t id;
} CLogicalQubit;

typedef struct CPhysicalQubit {
  uint32_t id;
} CPhysicalQubit;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `uint32_t` | 数值标识。 |

---

## CLogicalQubit

逻辑比特标识，用于线路中的逻辑比特。

### logical_qubit_new(id)

```c
struct CLogicalQubit logical_qubit_new(uint32_t id);
```

从数值标识构造，按值返回。

- `id` (`uint32_t`)：数值标识。

### logical_qubit_from_qubit(qubit)

```c
struct CLogicalQubit logical_qubit_from_qubit(struct CQubit qubit);
```

包装既有的线路比特。

- `qubit` (`struct CQubit`)：线路比特标识。

### logical_qubit_qubit(qubit)

```c
struct CQubit logical_qubit_qubit(struct CLogicalQubit qubit);
```

返回底层线路比特。

- `qubit` (`struct CLogicalQubit`)：逻辑比特标识。

### logical_qubit_id(qubit)

```c
uint32_t logical_qubit_id(struct CLogicalQubit qubit);
```

返回数值标识。

- `qubit` (`struct CLogicalQubit`)：逻辑比特标识。

### logical_qubit_equal(a, b) / logical_qubit_compare(a, b)

```c
int32_t logical_qubit_equal(struct CLogicalQubit a, struct CLogicalQubit b);
int32_t logical_qubit_compare(struct CLogicalQubit a, struct CLogicalQubit b);
```

比较与排序基于数值标识：`equal` 返回 `1`（相同）或 `0`（不同）；`compare` 返回 `-1`（`a < b`）、`0`（相等）或 `1`（`a > b`）。

- `a`、`b` (`struct CLogicalQubit`)：待比较的标识。

### logical_qubit_to_string(qubit)

```c
char *logical_qubit_to_string(struct CLogicalQubit qubit);
```

格式化为 `L<id>`（如 `L3`）。返回的字符串用 [cqlib_string_free](../0_overview.md) 释放；分配失败时返回 NULL。

- `qubit` (`struct CLogicalQubit`)：逻辑比特标识。

---

## CPhysicalQubit

物理比特标识，用于设备上的比特位置。

### physical_qubit_new(id)

```c
struct CPhysicalQubit physical_qubit_new(uint32_t id);
```

从数值标识构造，按值返回。

- `id` (`uint32_t`)：数值标识。

### physical_qubit_from_qubit(qubit)

```c
struct CPhysicalQubit physical_qubit_from_qubit(struct CQubit qubit);
```

包装既有比特形式的标识。

- `qubit` (`struct CQubit`)：比特形式的标识。

### physical_qubit_qubit(qubit)

```c
struct CQubit physical_qubit_qubit(struct CPhysicalQubit qubit);
```

返回底层比特标识。

- `qubit` (`struct CPhysicalQubit`)：物理比特标识。

### physical_qubit_id(qubit)

```c
uint32_t physical_qubit_id(struct CPhysicalQubit qubit);
```

返回数值标识。

- `qubit` (`struct CPhysicalQubit`)：物理比特标识。

### physical_qubit_equal(a, b) / physical_qubit_compare(a, b)

```c
int32_t physical_qubit_equal(struct CPhysicalQubit a, struct CPhysicalQubit b);
int32_t physical_qubit_compare(struct CPhysicalQubit a, struct CPhysicalQubit b);
```

比较与排序基于数值标识：`equal` 返回 `1`（相同）或 `0`（不同）；`compare` 返回 `-1`（`a < b`）、`0`（相等）或 `1`（`a > b`）。

- `a`、`b` (`struct CPhysicalQubit`)：待比较的标识。

### physical_qubit_to_string(qubit)

```c
char *physical_qubit_to_string(struct CPhysicalQubit qubit);
```

格式化为 `P<id>`（如 `P11`）。返回的字符串用 [cqlib_string_free](../0_overview.md) 释放；分配失败时返回 NULL。

- `qubit` (`struct CPhysicalQubit`)：物理比特标识。

---

## 错误码

本页全部接口都不返回错误码：构造、访问与比较不失败；`*_to_string` 仅在分配失败时返回 NULL。

---

## 示例

```c
#include <cqlib_c.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* 线路比特包装为逻辑标识，并可还原 */
    CLogicalQubit logical = logical_qubit_from_qubit(qubit_new(3));
    assert(logical_qubit_id(logical) == 3u);
    assert(qubit_id(logical_qubit_qubit(logical)) == 3u);

    /* 物理标识独立构造 */
    CPhysicalQubit physical = physical_qubit_new(100);
    assert(physical_qubit_id(physical) == 100u);

    /* 比较：逻辑侧 L2 < L0 不成立，L0 < L2 成立 */
    assert(logical_qubit_compare(logical_qubit_new(0), logical_qubit_new(2)) == -1);
    assert(physical_qubit_compare(physical, physical_qubit_new(101)) == -1);

    /* 字符串表示 */
    char *ltext = logical_qubit_to_string(logical);
    assert(strcmp(ltext, "L3") == 0);
    cqlib_string_free(ltext);

    char *ptext = physical_qubit_to_string(physical);
    assert(strcmp(ptext, "P100") == 0);
    cqlib_string_free(ptext);

    /* 布局场景：逻辑 1 -> 物理 102 的映射用编号登记与查询 */
    const uint32_t pairs[2] = {1u, 102u};
    CLayout *layout = layout_from_pairs(pairs, 1, 3);
    assert(layout != NULL);
    assert(layout_get(layout, 1) == physical_qubit_id(physical_qubit_new(102)));
    layout_free(layout);
    return 0;
}
```

---

## 相关页面

- [量子比特](../0_circuit/2_qubit.md)：`CQubit` 线路比特标识与 `CQubit` 互转的入口。
- [Layout](3_layout.md)：逻辑↔物理映射的绑定与查询。
- [Overview](../0_overview.md)：错误码、内存所有权与全局约定。
