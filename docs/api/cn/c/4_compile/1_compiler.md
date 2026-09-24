# 编译器（C）

本页覆盖编译入口 `compile` / `compile_with_device` 与编译结果句柄 `CCompileResult` 的全部读取接口：优化线路、变更标记、编译模式、步骤报告与设备布局。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；配置结构与 `COMPILE_*` 常量见 [Overview](0_overview.md)。

---

## 编译入口

### compile(circuit, config)

按 `CompileConfigC` 配置运行编译工作流：`config.mode` 选择普通或增强模式；`config.target` 选择 `Logical` 目标，其他目标值按逻辑编译处理。

参数：

- `circuit` (`const CCircuit*`)：输入线路。
- `config` (`struct CompileConfigC`)：编译配置（按值传递）。

返回：成功返回新建的 `CCompileResult*`；空指针或编译失败返回 NULL。

### compile_with_device(circuit, mode, device, initial_layout, seed)

针对具体设备目标运行编译工作流：线路在设备拓扑上路由，并降级到设备的有序原生门集，结果线路始终被该设备接受。

参数：

- `circuit` (`const CCircuit*`)：输入线路。
- `mode` (`uint8_t`)：编译模式，取 `COMPILE_MODE_*` 之一。
- `device` (`const CDevice*`)：目标设备（构造方式见 [设备与噪声属性](../2_device/2_properties_device.md)）。
- `initial_layout` (`const CLayout*`)：可选的初始逻辑→物理映射；NULL 时由工作流自行选择。
- `seed` (`int64_t`)：设备布局/路由的确定性种子；负值表示无种子（使用启发式默认值）。

返回：成功返回新建的 `CCompileResult*`；空指针或编译失败返回 NULL。

### compile_result_free(ptr)

释放编译结果句柄，允许传 NULL。

---

## 结果读取

### compile_result_circuit(ptr)

取出结果中保存的优化线路。返回的是深拷贝，可与 `result` 独立释放。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；空指针或失败返回 NULL。

### compile_result_changed(ptr)

判断工作流是否改动了线路。

返回：1 有改动；0 无改动；-1 空指针。

### compile_result_mode(ptr)

返回编译模式。

返回：`0`（Normal）、`1`（Enhanced）或 -1（空指针）。

### compile_result_initial_layout(ptr)

读取设备目标编译记录的初始逻辑→物理布局。

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；结果不含设备元数据（例如逻辑目标编译）或空指针输入时返回 NULL。

### compile_result_final_layout(ptr)

读取设备目标编译记录的最终逻辑→物理布局。

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；结果不含设备元数据（例如逻辑目标编译）或空指针输入时返回 NULL。

---

## 步骤报告

### compile_result_num_steps(ptr)

返回结果中的步骤报告数；NULL 返回 0。

### compile_result_step_name(ptr, index)

返回第 `index` 个步骤的名称。

返回：堆上 C 字符串，用 `cqlib_string_free` 释放；越界返回 NULL。

### compile_result_step_changed(ptr, index)

判断第 `index` 个步骤是否改动了线路。

返回：1 有改动；0 无改动；-1 出错。

### compile_result_step_reason(ptr, index)

返回第 `index` 个步骤的可选跳过/配置说明。

返回：调用方拥有的堆上 C 字符串，用 `cqlib_string_free` 释放。该步骤记录没有原因、`ptr` 为 NULL 或 `index` 越界时返回 NULL。

### compile_result_step(ptr, name, out_changed, out_skipped)

按名称查找第一个匹配的步骤报告，并把该步骤的原始 `changed` 与 `skipped` 标志写到出参。

参数：

- `name` (`const char*`)：步骤名称。
- `out_changed` (`int32_t*`)：changed 标志输出。
- `out_skipped` (`int32_t*`)：skipped 标志输出。

返回：1 找到匹配步骤；0 无匹配；-1 空指针或非法 UTF-8。

---

## 可复用工作流（CompilerWorkflow）

`CCompilerWorkflow` 是可复用的编译工作流句柄：按 `CompileConfigC` 一次构建后，可在多条线路上重复执行，输入线路不会被修改。工作流结果与 `compile` 的结果使用同一套读取接口（见上文"结果读取"与"步骤报告"）。

### compiler_workflow_new(config)

```c
struct CCompilerWorkflow *compiler_workflow_new(struct CompileConfigC config);
```

由 C 配置创建可复用编译工作流。`config.mode` 选择普通或增强模式；`config.target` 仅支持 `COMPILE_TARGET_LOGICAL`，其他目标值返回 NULL。目标准备在构造时即时校验，非法配置返回 NULL。

- `config` (`struct CompileConfigC`)：编译配置，按值传递（字段说明见 [Overview](0_overview.md)）。

返回：成功返回新建的 `CCompilerWorkflow*`（用 `compiler_workflow_free` 释放）；`config.mode` 非法、`config.target` 非 `COMPILE_TARGET_LOGICAL` 或配置校验失败返回 NULL。

### compiler_workflow_free(ptr)

```c
void compiler_workflow_free(struct CCompilerWorkflow *ptr);
```

释放工作流句柄。允许传 NULL。

### compiler_workflow_run(workflow, circuit)

```c
struct CCompileResult *compiler_workflow_run(const struct CCompilerWorkflow *workflow,
                                             const struct CCircuit *circuit);
```

在工作流上运行一条线路。工作流可在不同线路上反复调用，输入线路不被修改。

- `workflow` (`const CCompilerWorkflow*`)：工作流句柄。
- `circuit` (`const CCircuit*`)：输入线路。

返回：成功返回新建的 `CCompileResult*`（用 `compile_result_free` 释放）；任一参数为 NULL 或编译失败返回 NULL。

---

## 设备编译元数据（DeviceCompilationMetadata）

设备目标编译（`compile_with_device`）在结果中记录设备元数据：初始与最终布局（与 `compile_result_initial_layout` / `compile_result_final_layout` 读取同一份数据），以及虚拟输出置换——原始输出比特到重写后输出比特的对应关系，用于把测量结果映射回原始比特序。逻辑目标编译不携带设备元数据。

### compile_result_has_device_metadata(ptr)

```c
int32_t compile_result_has_device_metadata(const struct CCompileResult *ptr);
```

判断结果是否携带设备编译元数据（即设备目标编译）。

返回：`1` 有设备元数据；`0` 没有（例如逻辑目标编译）；`-1` 空指针。

### compile_result_device_metadata_initial_layout(ptr)

```c
struct CLayout *compile_result_device_metadata_initial_layout(const struct CCompileResult *ptr);
```

读取设备目标编译记录的初始逻辑→物理布局（路由开始前）。

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；结果不含设备元数据或空指针输入时返回 NULL。

### compile_result_device_metadata_final_layout(ptr)

```c
struct CLayout *compile_result_device_metadata_final_layout(const struct CCompileResult *ptr);
```

读取设备目标编译记录的最终逻辑→物理布局（全部路由 SWAP 之后）。

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；结果不含设备元数据或空指针输入时返回 NULL。

### compile_result_device_metadata_permutation_len(ptr)

```c
uintptr_t compile_result_device_metadata_permutation_len(const struct CCompileResult *ptr);
```

返回虚拟输出置换的条目数，用于为 `compile_result_device_metadata_permutation` 分配缓冲。

返回：置换条目数；空指针输入或结果不含设备元数据时返回 0。

### compile_result_device_metadata_permutation(ptr, original, rewritten, len)

```c
int32_t compile_result_device_metadata_permutation(const struct CCompileResult *ptr,
                                                   uint32_t *original,
                                                   uint32_t *rewritten,
                                                   uintptr_t len);
```

两步式读取虚拟输出置换：按原始输出升序把 qubit ID 对拷入 `original` 与 `rewritten`。下标 `i` 记录"原始输出 `original[i]` 的量子态由重写后输出 `rewritten[i]` 承载"；`len` 必须等于 `compile_result_device_metadata_permutation_len(ptr)`。

- `ptr` (`const CCompileResult*`)：编译结果句柄。
- `original` (`uint32_t*`)：原始输出比特 ID 缓冲。
- `rewritten` (`uint32_t*`)：重写后输出比特 ID 缓冲。
- `len` (`uintptr_t`)：条目数。

返回：`0` 成功（含无设备元数据且 `len = 0`）；`-1` 空指针（`ptr` 为 NULL，或 `len > 0` 而任一缓冲为 NULL）；`-8` `len` 与置换条目数不符（含无设备元数据但 `len > 0`）。

---

## 示例：可复用工作流与设备元数据

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

设备目标编译后 `compile_result_has_device_metadata` 返回 1，初始/最终布局与输出置换可用；对应流程见下文"示例"一节的 `compile_with_device` 用法。

---

## 示例

编译线路并读取步骤报告（设备目标流程）：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 输入线路与直线拓扑设备 0 - 1 - 2 */
    struct CCircuit *qc = circuit_new(3);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 2);   /* 非相邻耦合，需要路由 */

    struct CDevice *dev = device_line("demo", 3);
    if (dev == NULL) {
        circuit_free(qc);
        return 1;
    }

    /* 2. 设备目标编译：无初始布局、无种子 */
    struct CCompileResult *result =
        compile_with_device(qc, COMPILE_MODE_ENHANCED, dev, NULL, -1);
    if (result == NULL) {
        device_free(dev);
        circuit_free(qc);
        return 1;
    }

    /* 3. 读取优化线路与整体变更标记 */
    int32_t changed = compile_result_changed(result);          /* 1：路由改动了线路 */
    struct CCircuit *optimized = compile_result_circuit(result);
    if (optimized != NULL) {
        circuit_free(optimized);
    }

    /* 4. 遍历步骤报告 */
    uintptr_t steps = compile_result_num_steps(result);
    for (uintptr_t i = 0; i < steps; i++) {
        char *name = compile_result_step_name(result, i);
        int32_t step_changed = compile_result_step_changed(result, i);
        printf("step %s: changed=%d\n", name, step_changed);
        cqlib_string_free(name);
    }

    /* 5. 按名称查找步骤并读取原始标志（步骤名形如 "route.sabre"、"validate.device"） */
    int32_t flag_changed = 0, flag_skipped = 0;
    int32_t found = compile_result_step(result, "route.sabre",
                                         &flag_changed, &flag_skipped);

    /* 6. 设备编译元数据：初始与最终布局 */
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

逻辑目标编译（`CompileConfigC` 按值传参）的示例见 [Overview](0_overview.md)；布局句柄的读取接口见 [Layout](../2_device/3_layout.md)。
