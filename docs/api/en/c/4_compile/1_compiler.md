# Compiler (C)

This page covers the compilation entry points `compile` / `compile_with_device` and every read accessor of the compile-result handle `CCompileResult`: the optimized circuit, change flags, the compile mode, step reports, and device layouts. Error codes, string and handle ownership conventions follow the [Overview](../0_overview.md); the configuration struct and `COMPILE_*` constants are documented in [Overview](0_overview.md).

---

## Compilation Entry Points

### compile(circuit, config)

Runs the compilation workflow with a `CompileConfigC`: `config.mode` selects the Normal or Enhanced workflow; `config.target` selects the `Logical` target, and other target values are handled as logical compilation.

Parameters:

- `circuit` (`const CCircuit*`): input circuit.
- `config` (`struct CompileConfigC`): compile configuration (passed by value).

Returns: a new `CCompileResult*` on success; NULL on NULL input or compilation failure.

### compile_with_device(circuit, mode, device, initial_layout, seed)

Runs the compilation workflow for a concrete device target: the circuit is routed on the device topology and lowered to the device's ordered native capabilities, so the resulting circuit is always accepted by the device.

Parameters:

- `circuit` (`const CCircuit*`): input circuit.
- `mode` (`uint8_t`): compile mode, one of `COMPILE_MODE_*`.
- `device` (`const CDevice*`): target device (construction in [Device / Properties](../2_device/2_properties_device.md)).
- `initial_layout` (`const CLayout*`): optional initial logical-to-physical mapping; NULL lets the workflow choose one.
- `seed` (`int64_t`): deterministic device layout/routing seed; a negative value means "no seed" (heuristic default).

Returns: a new `CCompileResult*` on success; NULL on NULL input or compilation failure.

### compile_result_free(ptr)

Frees a compile result handle; NULL is allowed.

---

## Result Reading

### compile_result_circuit(ptr)

Retrieves the optimized circuit stored in the result. The returned circuit is a deep clone and can be freed independently of `result`.

Returns: a new `CCircuit*` on success (free with `circuit_free`); NULL on NULL input or failure.

### compile_result_changed(ptr)

Tests whether the workflow changed the circuit.

Returns: 1 when changed; 0 otherwise; -1 for NULL.

### compile_result_mode(ptr)

Returns the compile mode.

Returns: `0` (Normal), `1` (Enhanced), or -1 for NULL.

### compile_result_initial_layout(ptr)

Reads the initial logical-to-physical layout recorded by a device-target compilation.

Returns: a new `CLayout*` on success (free with `layout_free`); NULL when the result carries no device metadata (for example after a logical-target compile) or on NULL input.

### compile_result_final_layout(ptr)

Reads the final logical-to-physical layout recorded by a device-target compilation.

Returns: a new `CLayout*` on success (free with `layout_free`); NULL when the result carries no device metadata (for example after a logical-target compile) or on NULL input.

---

## Step Reports

### compile_result_num_steps(ptr)

Returns the number of step reports in the result; 0 for NULL.

### compile_result_step_name(ptr, index)

Returns the name of step `index`.

Returns: a heap-allocated C string; free with `cqlib_string_free`. NULL on out-of-bounds.

### compile_result_step_changed(ptr, index)

Tests whether step `index` changed the circuit.

Returns: 1 when changed; 0 otherwise; -1 on error.

### compile_result_step_reason(ptr, index)

Returns the optional skip or configuration note of step `index`.

Returns: a caller-owned heap-allocated C string; free with `cqlib_string_free`. NULL when the step record carries no reason, when `ptr` is NULL, or when `index` is out of bounds.

### compile_result_step(ptr, name, out_changed, out_skipped)

Finds the first workflow step report named `name` and writes the step's raw `changed` and `skipped` flags to the out pointers.

Parameters:

- `name` (`const char*`): step name.
- `out_changed` (`int32_t*`): changed-flag output.
- `out_skipped` (`int32_t*`): skipped-flag output.

Returns: 1 when a matching step exists; 0 when no step matches; -1 on NULL arguments or invalid UTF-8.

---

## Reusable Workflow (CompilerWorkflow)

`CCompilerWorkflow` is a reusable compiler-workflow handle: built once from a `CompileConfigC`, it can then run over any number of circuits, and the input circuit is never modified. Workflow results are read with the same accessors as `compile` results (see "Result Reading" and "Step Reports" above).

### compiler_workflow_new(config)

```c
struct CCompilerWorkflow *compiler_workflow_new(struct CompileConfigC config);
```

Creates a reusable compiler workflow from a C configuration. `config.mode` selects the Normal or Enhanced workflow; `config.target` only supports `COMPILE_TARGET_LOGICAL`, and other target values return NULL. Target preparation is validated eagerly at construction, so an invalid configuration returns NULL.

- `config` (`struct CompileConfigC`): compile configuration, passed by value (field reference in [Overview](0_overview.md)).

Returns: a new `CCompilerWorkflow*` on success (free with `compiler_workflow_free`); NULL when `config.mode` is invalid, `config.target` is not `COMPILE_TARGET_LOGICAL`, or configuration validation fails.

### compiler_workflow_free(ptr)

```c
void compiler_workflow_free(struct CCompilerWorkflow *ptr);
```

Frees a workflow handle; NULL is allowed.

### compiler_workflow_run(workflow, circuit)

```c
struct CCompileResult *compiler_workflow_run(const struct CCompilerWorkflow *workflow,
                                             const struct CCircuit *circuit);
```

Runs the workflow over one circuit. The workflow can be run repeatedly over different circuits; the input circuit is never modified.

- `workflow` (`const CCompilerWorkflow*`): workflow handle.
- `circuit` (`const CCircuit*`): input circuit.

Returns: a new `CCompileResult*` on success (free with `compile_result_free`); NULL when either argument is NULL or compilation fails.

---

## Device Compilation Metadata (DeviceCompilationMetadata)

A device-target compilation (`compile_with_device`) records device metadata in the result: the initial and final layouts (the same data read by `compile_result_initial_layout` / `compile_result_final_layout`) plus the virtual output permutation — the mapping from original output qubits to rewritten output qubits, used to map measurement results back to the original qubit order. A logical-target compile carries no device metadata.

### compile_result_has_device_metadata(ptr)

```c
int32_t compile_result_has_device_metadata(const struct CCompileResult *ptr);
```

Tests whether the result carries device compilation metadata (a device-target compile).

Returns: `1` when device metadata is present; `0` when it is not (for example after a logical-target compile); `-1` for NULL.

### compile_result_device_metadata_initial_layout(ptr)

```c
struct CLayout *compile_result_device_metadata_initial_layout(const struct CCompileResult *ptr);
```

Reads the initial logical-to-physical layout recorded by a device-target compilation (before routing).

Returns: a new `CLayout*` on success (free with `layout_free`); NULL when the result carries no device metadata or on NULL input.

### compile_result_device_metadata_final_layout(ptr)

```c
struct CLayout *compile_result_device_metadata_final_layout(const struct CCompileResult *ptr);
```

Reads the final logical-to-physical layout recorded by a device-target compilation (after all routing SWAPs).

Returns: a new `CLayout*` on success (free with `layout_free`); NULL when the result carries no device metadata or on NULL input.

### compile_result_device_metadata_permutation_len(ptr)

```c
uintptr_t compile_result_device_metadata_permutation_len(const struct CCompileResult *ptr);
```

Returns the number of entries in the virtual output permutation, used to size the buffers for `compile_result_device_metadata_permutation`.

Returns: the entry count; 0 for NULL input or a result without device metadata.

### compile_result_device_metadata_permutation(ptr, original, rewritten, len)

```c
int32_t compile_result_device_metadata_permutation(const struct CCompileResult *ptr,
                                                   uint32_t *original,
                                                   uint32_t *rewritten,
                                                   uintptr_t len);
```

Two-step read of the virtual output permutation: copies qubit-ID pairs into `original` and `rewritten` in ascending original-output order. Index `i` records that the state of original output `original[i]` is carried by rewritten output `rewritten[i]`; `len` must equal `compile_result_device_metadata_permutation_len(ptr)`.

- `ptr` (`const CCompileResult*`): compile result handle.
- `original` (`uint32_t*`): buffer for the original output qubit IDs.
- `rewritten` (`uint32_t*`): buffer for the rewritten output qubit IDs.
- `len` (`uintptr_t`): entry count.

Returns: `0` on success (including a no-metadata result with `len = 0`); `-1` on NULL pointers (`ptr` is NULL, or `len > 0` with either buffer NULL); `-8` when `len` does not match the permutation length (including a no-metadata result with `len > 0`).

---

## Example: Reusable Workflow and Device Metadata

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* Enhanced-mode logical workflow, built once */
    struct CompileConfigC config;
    memset(&config, 0, sizeof(config));
    config.mode = COMPILE_MODE_ENHANCED;
    config.target = COMPILE_TARGET_LOGICAL;
    struct CCompilerWorkflow *workflow = compiler_workflow_new(config);
    if (workflow == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* Run the workflow; it is reusable across circuits */
    struct CCompileResult *result = compiler_workflow_run(workflow, qc);
    if (result == NULL) {
        compiler_workflow_free(workflow);
        circuit_free(qc);
        return 1;
    }
    struct CCompileResult *second = compiler_workflow_run(workflow, qc);
    if (second != NULL) {
        compile_result_free(second);   /* independent result handle */
    }

    /* A logical-target compile carries no device metadata */
    int32_t has_meta = compile_result_has_device_metadata(result);  /* 0 */
    struct CLayout *initial = compile_result_device_metadata_initial_layout(result);
    if (initial != NULL) {
        layout_free(initial);
    }
    uintptr_t perm_len = compile_result_device_metadata_permutation_len(result);
    uint32_t *original = malloc(perm_len * sizeof(uint32_t));
    uint32_t *rewritten = malloc(perm_len * sizeof(uint32_t));
    int32_t rc = compile_result_device_metadata_permutation(
        result, original, rewritten, perm_len);   /* 0: len == 0 */
    free(original);
    free(rewritten);

    compile_result_free(result);
    compiler_workflow_free(workflow);
    circuit_free(qc);
    return 0;
}
```

After a device-target compile, `compile_result_has_device_metadata` returns 1 and the layouts and output permutation become available; that flow is the `compile_with_device` usage shown in the "Example" section below.

---

## Example

Compiling a circuit and reading the step reports (device-target flow):

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Input circuit and a line-topology device 0 - 1 - 2 */
    struct CCircuit *qc = circuit_new(3);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 2);   /* non-adjacent coupling: requires routing */

    struct CDevice *dev = device_line("demo", 3);
    if (dev == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* 2. Device-target compilation: no initial layout, no seed */
    struct CCompileResult *result =
        compile_with_device(qc, COMPILE_MODE_ENHANCED, dev, NULL, -1);
    if (result == NULL) {
        device_free(dev);
        circuit_free(qc);
        return 1;
    }

    /* 3. Optimized circuit and overall change flag */
    int32_t changed = compile_result_changed(result);          /* 1: routing changed the circuit */
    struct CCircuit *optimized = compile_result_circuit(result);
    if (optimized != NULL) {
        circuit_free(optimized);
    }

    /* 4. Traverse the step reports */
    uintptr_t steps = compile_result_num_steps(result);
    for (uintptr_t i = 0; i < steps; i++) {
        char *name = compile_result_step_name(result, i);
        int32_t step_changed = compile_result_step_changed(result, i);
        printf("step %s: changed=%d\n", name, step_changed);
        cqlib_string_free(name);
    }

    /* 5. Look up a step by name (names look like "route.sabre", "validate.device") */
    int32_t flag_changed = 0, flag_skipped = 0;
    int32_t found = compile_result_step(result, "route.sabre",
                                         &flag_changed, &flag_skipped);

    /* 6. Device compilation metadata: initial and final layouts */
    struct CLayout *initial = compile_result_initial_layout(result);
    struct CLayout *final = compile_result_final_layout(result);
    if (initial != NULL) {
        layout_free(initial);
    }
    if (final != NULL) {
        layout_free(final);
    }

    compile_result_free(result);
    device_free(dev);
    circuit_free(qc);
    return 0;
}
```

The logical-target flow (with `CompileConfigC` passed by value) is shown in [Overview](0_overview.md); layout handle accessors are documented in [Layout](../2_device/3_layout.md).
