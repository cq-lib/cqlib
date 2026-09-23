# Compile

编译模块将输入线路经过一系列优化/映射 pass，输出优化后的线路与逐步报告。

---

## 编译配置 CompileConfigC

```c
typedef struct CompileConfigC {
  uint8_t mode;                  // COMPILE_MODE_NORMAL / COMPILE_MODE_ENHANCED
  uint8_t target;                // COMPILE_TARGET_LOGICAL / BASIS / DEVICE / TOPOLOGY_BASIS
  uint8_t allow_dirty_ancilla;   // 预留（资源策略）
  uint8_t allow_clean_ancilla;   // 预留（资源策略）
  uint8_t _reserved[4];          // 布局保持稳定
} CompileConfigC;
```

| 字段 | 取值 | 说明 |
| --- | --- | --- |
| `mode` | `COMPILE_MODE_NORMAL`(0) / `COMPILE_MODE_ENHANCED`(1) | 常规优化 / 增强优化（模板匹配等更重的 pass） |
| `target` | `COMPILE_TARGET_LOGICAL`(0) / `BASIS`(1) / `DEVICE`(2) / `TOPOLOGY_BASIS`(3) | 编译目标 |
| `allow_dirty_ancilla` / `allow_clean_ancilla` | 预留 | 当前版本不生效，保持 0 |
| `_reserved` | — | 填充字段，保证 ABI 布局稳定 |

**target 说明**：C ABI 下仅 `Logical` 目标完整支持；`Basis`/`Device`/`TopologyBasis` 目标在 Rust 侧回退为逻辑编译（C ABI 无法解析 `Instruction` 列表与 `Device` 对象，此类目标需从 Rust 侧构造）。

结构体**按值传递**，推荐零初始化后按需覆盖字段：

```c
CompileConfigC config = {0};              // 默认 NORMAL + LOGICAL
config.mode = COMPILE_MODE_ENHANCED;
```

---

## 运行编译

### compile(circuit, config)

对 `circuit` 执行编译工作流。

参数：

- `circuit` (`const CCircuit *`)：输入线路，不被修改。
- `config` (`CompileConfigC`)：按值传递的配置。

返回：

- `CCompileResult *`：堆分配结果对象，需 `compile_result_free` 释放（允许传 NULL）；失败返回 NULL。

---

## 结果提取

### compile_result_circuit(result)

返回优化后的线路。

返回：

- `CCircuit *`：结果中线路的**深拷贝**，生命周期独立于 `result`（取走后即可释放 result），需 `circuit_free` 释放；失败返回 NULL。

### compile_result_changed(result)

工作流是否修改了线路。

返回：

- `int32_t`：`1` 已修改 / `0` 未修改 / `-1` NULL 句柄。

### compile_result_mode(result)

返回编译模式（`COMPILE_MODE_NORMAL` 或 `COMPILE_MODE_ENHANCED`）；NULL 句柄返回 `-1`。

### compile_result_num_steps(result)

返回逐步报告中的步骤数量；NULL 句柄返回 `0`。

### compile_result_step_name(result, index)

返回第 `index` 步的名称。

返回：

- `char *`：堆分配字符串，需 `cqlib_string_free` 释放；越界返回 NULL。

### compile_result_step_changed(result, index)

第 `index` 步是否改变了线路。

返回：

- `int32_t`：`1` / `0` / `-1`（错误）。

### compile_result_free(result)

释放编译结果。允许传 NULL。

---

## 完整示例

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

## 与模拟器/设备配合

编译输出的线路可直接交给 QIS 模拟（[Statevector](../3_qis/1_statevector.md)）或设备校验（[Device](../2_device/2_properties_device.md)）：

```c
CStatevector *sv = statevector_from_circuit(optimized);
CDevice *dev = device_bidirectional_line("demo", 4);
device_validate_circuit(dev, optimized);   // 0 = 兼容
```
