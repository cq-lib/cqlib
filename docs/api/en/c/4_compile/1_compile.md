# Compile

The compilation module runs the input circuit through a series of optimization/mapping passes and outputs the optimized circuit together with a step-by-step report.

---

## Compilation configuration CompileConfigC

```c
typedef struct CompileConfigC {
  uint8_t mode;                  // COMPILE_MODE_NORMAL / COMPILE_MODE_ENHANCED
  uint8_t target;                // COMPILE_TARGET_LOGICAL / BASIS / DEVICE / TOPOLOGY_BASIS
  uint8_t allow_dirty_ancilla;   // 预留（资源策略）
  uint8_t allow_clean_ancilla;   // 预留（资源策略）
  uint8_t _reserved[4];          // 布局保持稳定
} CompileConfigC;
```

| Field | Value | Description |
| --- | --- | --- |
| `mode` | `COMPILE_MODE_NORMAL`(0) / `COMPILE_MODE_ENHANCED`(1) | Normal optimization / enhanced optimization (heavier passes such as template matching) |
| `target` | `COMPILE_TARGET_LOGICAL`(0) / `BASIS`(1) / `DEVICE`(2) / `TOPOLOGY_BASIS`(3) | Compilation target |
| `allow_dirty_ancilla` / `allow_clean_ancilla` | Reserved | Not in effect in the current version; keep 0 |
| `_reserved` | — | A padding field that keeps the ABI layout stable |

**Notes on target**: under the C ABI only the `Logical` target is fully supported; the `Basis`/`Device`/`TopologyBasis` targets fall back to logical compilation on the Rust side (the C ABI cannot parse the `Instruction` list or the `Device` object, so such targets must be constructed from the Rust side).

The structure is **passed by value**, and zero-initializing it first and then overriding the fields as needed is recommended:

```c
CompileConfigC config = {0};              // 默认 NORMAL + LOGICAL
config.mode = COMPILE_MODE_ENHANCED;
```

---

## Running compilation

### compile(circuit, config)

Run the compilation workflow on `circuit`.

Parameters:

- `circuit` (`const CCircuit *`): the input circuit, which is not modified.
- `config` (`CompileConfigC`): the configuration, passed by value.

Returns:

- `CCompileResult *`: a heap-allocated result object that must be released with `compile_result_free` (passing NULL is allowed); NULL is returned on failure.

---

## Result extraction

### compile_result_circuit(result)

Return the optimized circuit.

Returns:

- `CCircuit *`: a **deep copy** of the circuit in the result, whose lifetime is independent of `result` (`result` may be released right after it is taken), and it must be released with `circuit_free`; NULL is returned on failure.

### compile_result_changed(result)

Whether the workflow modified the circuit.

Returns:

- `int32_t`: `1` modified / `0` not modified / `-1` NULL handle.

### compile_result_mode(result)

Return the compilation mode (`COMPILE_MODE_NORMAL` or `COMPILE_MODE_ENHANCED`); a NULL handle returns `-1`.

### compile_result_num_steps(result)

Return the number of steps in the step-by-step report; a NULL handle returns `0`.

### compile_result_step_name(result, index)

Return the name of step `index`.

Returns:

- `char *`: a heap-allocated string that must be released with `cqlib_string_free`; NULL is returned when the index is out of range.

### compile_result_step_changed(result, index)

Whether step `index` changed the circuit.

Returns:

- `int32_t`: `1` / `0` / `-1` (error).

### compile_result_free(result)

Release the compilation result. Passing NULL is allowed.

---

## Complete example

```c
CompileConfigC config = {0};
config.mode = COMPILE_MODE_ENHANCED;
config.target = COMPILE_TARGET_LOGICAL;

CCompileResult *result = compile(qc, config);
if (!result) { return 1; }

printf("mode=%d changed=%d\n",
       compile_result_mode(result),
       compile_result_changed(result));

/* 逐步报告 */
uintptr_t steps = compile_result_num_steps(result);
for (uintptr_t i = 0; i < steps; i++) {
    char *name = compile_result_step_name(result, i);
    printf("step %zu: %s (changed=%d)\n",
           (size_t)i, name, compile_result_step_changed(result, i));
    cqlib_string_free(name);
}

/* 取走优化线路后即可释放 result */
CCircuit *optimized = compile_result_circuit(result);
printf("optimized ops=%zu\n", (size_t)circuit_num_operations(optimized));

circuit_free(optimized);
compile_result_free(result);
```

---

## Working with simulators and devices

The circuit output by compilation can be handed directly to QIS simulation ([Statevector](../3_qis/1_statevector.md)) or to device validation ([Device](../2_device/2_properties_device.md)):

```c
CStatevector *sv = statevector_from_circuit(optimized);
CDevice *dev = device_bidirectional_line("demo", 4);
device_validate_circuit(dev, optimized);   // 0 = 兼容
```
