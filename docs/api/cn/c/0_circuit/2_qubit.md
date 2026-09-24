# 量子比特（C）

本页覆盖逻辑量子比特标识 `CQubit` 的 C ABI。`CQubit` 是按值传递的轻量句柄，内部保存一个 `uint32_t` 编号，用于在量子线路、编译 IR、映射表和集合结构中稳定标识某个逻辑量子比特。错误码与字符串释放约定遵循 [Overview](../0_overview.md)。

---

## 结构体

`CQubit` 按值传递，无需释放：

```c
typedef struct CQubit {
  uint32_t id;
} CQubit;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `uint32_t` | 逻辑量子比特的数值编号。 |

编号是逻辑标识：不携带线路身份，也不对应态矢量或矩阵中的存储位置；实际顺序由 `Circuit` 保存的量子比特列表与各转换接口的顺序约定决定。编号允许稀疏（如 10、20），调用方不应假设它是密集存储的有效下标。

---

## 创建与访问

### qubit_new(id)

```c
struct CQubit qubit_new(uint32_t id);
```

根据数值编号创建逻辑量子比特，按值返回。

- `id` (`uint32_t`)：逻辑量子比特编号。

### qubit_id(qubit)

```c
uint32_t qubit_id(struct CQubit qubit);
```

返回原始内部编号（紧凑 4 字节表示）。

- `qubit` (`struct CQubit`)：逻辑量子比特。

### qubit_index(qubit)

```c
uintptr_t qubit_index(struct CQubit qubit);
```

将编号转换为 `uintptr_t`，便于需要下标类型的场景。该值不映射到任何线路、矩阵或模拟器中的位置。

---

## 受检整数转换

### qubit_try_from_i64(value, out)

```c
int32_t qubit_try_from_i64(int64_t value, struct CQubit *out);
```

从有符号整数受检转换：负数或超出 `uint32_t` 范围的值不会静默截断，而是返回错误。

- `value` (`int64_t`)：输入整数。
- `out` (`struct CQubit*`)：输出量子比特。

返回：`0` 成功；`-1` `out` 为 NULL；`-8` `value` 为负或超出 `uint32_t` 范围。

### qubit_try_from_u64(value, out)

```c
int32_t qubit_try_from_u64(uint64_t value, struct CQubit *out);
```

从无符号整数受检转换：超出 `uint32_t` 范围的值返回错误。

- `value` (`uint64_t`)：输入整数。
- `out` (`struct CQubit*`)：输出量子比特。

返回：`0` 成功；`-1` `out` 为 NULL；`-8` `value` 超出 `uint32_t` 范围。

---

## 比较与排序

比较、排序与哈希均基于内部编号：两个 `CQubit` 编号相同即视为同一逻辑比特。

### qubit_equal(a, b)

```c
int32_t qubit_equal(struct CQubit a, struct CQubit b);
```

返回 `1` 表示两者编号相同，`0` 表示不同。

- `a`、`b` (`struct CQubit`)：待比较的量子比特。

### qubit_compare(a, b)

```c
int32_t qubit_compare(struct CQubit a, struct CQubit b);
```

按编号比较：`-1` 表示 `a < b`，`0` 表示相等，`1` 表示 `a > b`。

- `a`、`b` (`struct CQubit`)：待比较的量子比特。

---

## 字符串表示

### qubit_to_string(qubit)

```c
char *qubit_to_string(struct CQubit qubit);
```

格式化为 `Q<id>`（如 `Q12`）。返回的字符串用 [cqlib_string_free](../0_overview.md) 释放；分配失败时返回 NULL。

- `qubit` (`struct CQubit`)：逻辑量子比特。

---

## 错误码

| 函数 | `0` | `-1` | `-8` |
| --- | --- | --- | --- |
| `qubit_try_from_i64` | 转换成功 | `out` 为 NULL | 值为负或超出 `uint32_t` 范围 |
| `qubit_try_from_u64` | 转换成功 | `out` 为 NULL | 值超出 `uint32_t` 范围 |

`qubit_new`、`qubit_id`、`qubit_index`、`qubit_equal`、`qubit_compare` 不失败；`qubit_to_string` 仅在分配失败时返回 NULL。

---

## 示例

```c
#include <cqlib_c.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* 创建与访问 */
    CQubit q = qubit_new(12);
    assert(q.id == 12);
    assert(qubit_id(q) == 12u);
    assert(qubit_index(q) == 12u);

    /* 受检转换：负数与超范围都报错，不截断 */
    CQubit out = {0};
    assert(qubit_try_from_i64(-1, &out) == -8);
    assert(qubit_try_from_i64(3, &out) == 0);
    assert(qubit_id(out) == 3u);

    /* 比较 */
    assert(qubit_equal(q, qubit_new(12)) == 1);
    assert(qubit_compare(qubit_new(0), qubit_new(1)) == -1);

    /* 字符串表示 */
    char *text = qubit_to_string(q);
    assert(strcmp(text, "Q12") == 0);
    cqlib_string_free(text);

    /* 与线路交互：稀疏逻辑编号 10、20 */
    const uint32_t ids[2] = {qubit_id(qubit_new(10)), qubit_id(qubit_new(20))};
    CCircuit *qc = circuit_from_qubits(ids, 2);
    assert(circuit_num_qubits(qc) == 2);
    circuit_free(qc);
    return 0;
}
```

---

## 相关页面

- [Circuit](1_circuit.md)：`circuit_from_qubits` 接受显式编号列表（含稀疏编号）。
- [量子比特标识](../2_device/6_qubits.md)：设备侧 `CLogicalQubit` / `CPhysicalQubit`。
- [Overview](../0_overview.md)：错误码、内存所有权与全局约定。
