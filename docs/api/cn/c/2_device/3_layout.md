# Layout

`CLayout` 描述逻辑比特到物理比特的映射，用于编译映射与结果回填。

---

## 函数

### layout_new(logical, num_logical, physical, num_physical)

按顺序将 `logical[i]` 映射到 `physical[i]`，构造一一对应的布局。

参数：

- `logical` (`const uint32_t *`)：逻辑比特编号数组。
- `num_logical` (`uintptr_t`)：逻辑数组长度。
- `physical` (`const uint32_t *`)：物理比特编号数组。
- `num_physical` (`uintptr_t`)：物理数组长度。

约束：

- 两数组长度必须相等且非零，否则返回 NULL。

返回：

- `CLayout *`：堆分配布局，需 `layout_free` 释放；失败返回 NULL。

```c
uint32_t logical[2]  = {0, 1};
uint32_t physical[2] = {5, 7};
CLayout *layout = layout_new(logical, 2, physical, 2);
```

### layout_from_pairs(pairs, num_pairs, physical_count)

由 `(logical, physical)` 对构造布局，适合稀疏映射。

参数：

- `pairs` (`const uint32_t *`)：`2 * num_pairs` 个 u32，按 `(logical, physical)` 对连续排列。
- `num_pairs` (`uintptr_t`)：映射对数。
- `physical_count` (`uint32_t`)：物理比特总数，取值范围 `0..physical_count-1`；未被引用的物理比特保持**空置**。

返回：

- `CLayout *`；失败返回 NULL。

### layout_get(layout, logical)

返回 `logical` 映射的物理比特。

返回：

- `uint32_t`：物理比特编号；`logical` **未映射**或发生错误时返回 `UINT32_MAX`（`u32::MAX`）。

调用方应将 `UINT32_MAX` 视为哨兵值，而不是有效映射。

### layout_num_logical(layout)

返回已映射的逻辑比特数量。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### layout_num_physical(layout)

返回物理比特总数（含空置比特）。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### layout_free(ptr)

释放布局对象。允许传 NULL。

---

## 示例

```c
uint32_t pairs[6] = {0, 2,  1, 0,  2, 3};   // L0->P2, L1->P0, L2->P3
CLayout *layout = layout_from_pairs(pairs, 3, 4);

printf("logical=%zu physical=%zu\n",
       (size_t)layout_num_logical(layout),    // 3
       (size_t)layout_num_physical(layout));  // 4（P1 空置）

uint32_t phys = layout_get(layout, 1);        // 0
uint32_t none = layout_get(layout, 3);        // UINT32_MAX（未映射）
if (none != UINT32_MAX) {
    printf("L3 -> P%u\n", none);
}

layout_free(layout);
```
