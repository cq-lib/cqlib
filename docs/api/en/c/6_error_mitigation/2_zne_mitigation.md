# ZNEMitigation

The standalone entry point of ZNE (zero-noise extrapolation). It suits scenarios that need to **inspect the folded circuits directly** and organize the sampling flow on their own; for the end-to-end mitigation flow (sampling plus extrapolation post-processing) see [ErrorMitigation](1_error_mitigation.md).

---

## Folding and noise factors

The core idea of ZNE: artificially amplify noise by gate folding on the circuit, measure the same observable at several noise factors, and then extrapolate to the zero-noise point.

- The fold level `level` corresponds to the noise factor `2 * level + 1`;

| Level | Noise factor | Meaning |
| --- | --- | --- |
| 0 | 1 | The original circuit (no folding) |
| 1 | 3 | Each gate is effectively executed 3 times (gate-inverse-gate) |
| 2 | 5 | Each gate is effectively executed 5 times |

- **Global folding**: the whole circuit is repeated according to the noise factor;
- **Selective folding**: only the gates with the specified gate names are folded, and the other gates stay unchanged.

---

## Functions

### zne_mitigation_new(circuit, fold_levels, num_levels)

Create a ZNE helper object from the fold levels.

Parameters:

- `circuit` (`const CCircuit *`): the target circuit.
- `fold_levels` (`const int32_t *`): the array of fold levels, such as `{0, 1, 2}`.
- `num_levels` (`uintptr_t`): the number of levels.

Returns:

- `CZneMitigation *`: a heap-allocated object that must be released with `zne_mitigation_free`; NULL is returned on failure.

### zne_mitigation_fold_circuit(ptr, gate_names)

Generate the folded circuits of all levels.

Parameters:

- `gate_names` (`const char *`): NULL means global folding; a comma-separated list of gate names (such as `"H,CX"`) means that only those gates are folded.

Returns:

- `CCircuitList *`: an **independently owned** list of circuits (one per level) that must be released with `circuit_list_free`; NULL is returned on failure.

### zne_mitigation_noise_factor(ptr, index)

Return the noise factor of level `index` (`2 * level + 1`).

Returns:

- `int32_t`; `0` on error.

### zne_mitigation_free(ptr)

Release the object. Passing NULL is allowed.

---

## Traversing a CircuitList

```c
uintptr_t circuit_list_len(const struct CCircuitList *ptr);
struct CCircuit *circuit_list_get(const struct CCircuitList *ptr, uintptr_t index);
void circuit_list_free(struct CCircuitList *ptr);
```

| Function | Return | Description |
| --- | --- | --- |
| `circuit_list_len` | Count | The number of folded circuits (equal to the number of levels); NULL returns `0` |
| `circuit_list_get(index)` | `CCircuit *` | An **independently owned** copy of the circuit, to be released with `circuit_free`; NULL is returned when the index is out of range |
| `circuit_list_free` | `void` | Release the list; passing NULL is allowed |

Note that the circuit returned by `circuit_list_get` is independent of the list: releasing only the list without releasing the circuits taken out leaks; conversely, releasing a circuit taken out before the list is released is safe.

---

## Example

### Inspecting the folded circuits of all levels

```c
int32_t levels[3] = {0, 1, 2};
CZneMitigation *zne = zne_mitigation_new(qc, levels, 3);
if (!zne) { return 1; }

CCircuitList *folded = zne_mitigation_fold_circuit(zne, "H,CX");
for (uintptr_t i = 0; i < circuit_list_len(folded); i++) {
    CCircuit *fc = circuit_list_get(folded, i);
    printf("factor=%d ops=%zu\n", zne_mitigation_noise_factor(zne, i),
           (size_t)circuit_num_operations(fc));
    /* fc 可直接送入模拟器或设备执行 */
    circuit_free(fc);
}
circuit_list_free(folded);
zne_mitigation_free(zne);
```

### Manual extrapolation

After sampling at each noise factor to obtain a sequence of expectation values, the extrapolation is performed by the caller (polynomial fitting, exponential fitting and so on, with any numerical library). When built-in extrapolation is needed, use the `run` + `get_mitigated` flow of [ErrorMitigation](1_error_mitigation.md) directly.
