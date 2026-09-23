# Layout

`CLayout` describes the mapping from logical qubits to physical qubits, used for compilation mapping and for writing results back.

---

## Functions

### layout_new(logical, num_logical, physical, num_physical)

Map `logical[i]` to `physical[i]` in order, constructing a one-to-one layout.

Parameters:

- `logical` (`const uint32_t *`): the array of logical qubit indices.
- `num_logical` (`uintptr_t`): the length of the logical array.
- `physical` (`const uint32_t *`): the array of physical qubit indices.
- `num_physical` (`uintptr_t`): the length of the physical array.

Constraints:

- The two arrays must have the same non-zero length; otherwise NULL is returned.

Returns:

- `CLayout *`: a heap-allocated layout; release it with `layout_free`; NULL on failure.

```c
uint32_t logical[2]  = {0, 1};
uint32_t physical[2] = {5, 7};
CLayout *layout = layout_new(logical, 2, physical, 2);
```

### layout_from_pairs(pairs, num_pairs, physical_count)

Construct a layout from `(logical, physical)` pairs, suitable for sparse mappings.

Parameters:

- `pairs` (`const uint32_t *`): `2 * num_pairs` u32 values, laid out consecutively as `(logical, physical)` pairs.
- `num_pairs` (`uintptr_t`): the number of mapped pairs.
- `physical_count` (`uint32_t`): the total number of physical qubits; the index range is `0..physical_count-1`; physical qubits that are not referenced stay **vacant**.

Returns:

- `CLayout *`; NULL on failure.

### layout_get(layout, logical)

Return the physical qubit that `logical` is mapped to.

Returns:

- `uint32_t`: the physical qubit index; `UINT32_MAX` (`u32::MAX`) when `logical` is **not mapped** or an error occurs.

The caller should treat `UINT32_MAX` as a sentinel value rather than a valid mapping.

### layout_num_logical(layout)

Return the number of logical qubits that are mapped.

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### layout_num_physical(layout)

Return the total number of physical qubits (including vacant qubits).

Returns:

- `uintptr_t`; a NULL handle returns `0`.

### layout_free(ptr)

Release a layout object. Passing NULL is allowed.

---

## Example

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
