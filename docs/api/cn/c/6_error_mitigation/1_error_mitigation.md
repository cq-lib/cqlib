# ErrorMitigation

统一误差缓解管线，通过后处理降低噪声对期望值估计的影响。提供零噪声外推（ZNE）与虚拟蒸馏（Virtual Distillation）两条路线。

使用模式固定为三段式：`error_mitigation_new` 创建 → `error_mitigation_run` 采样（调用方提供的估计器回调）→ `error_mitigation_get_mitigated` 取最终估计。

---

## 方法与常量

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `MITIGATION_ZNE` | 0 | 零噪声外推（需 `fold_levels`） |
| `MITIGATION_VIRTUAL_DISTILLATION` | 1 | 虚拟蒸馏（需 `copies`） |
| `PROCESS_ZNE_POLYNOMIAL` | 0 | ZNE 多项式外推后处理 |
| `PROCESS_ZNE_EXPONENTIAL` | 1 | ZNE 指数外推后处理 |
| `PROCESS_VIRTUAL_DISTILLATION` | 2 | 虚拟蒸馏后处理 |

---

## 函数

### error_mitigation_new(circuit, method, fold_levels, num_levels, copies)

创建缓解管线。

参数：

- `circuit` (`const CCircuit *`)：目标线路，不被修改。
- `method` (`uint8_t`)：`MITIGATION_ZNE` 或 `MITIGATION_VIRTUAL_DISTILLATION`。
- `fold_levels` (`const int32_t *`)：ZNE 折叠等级数组（如 `{0, 1, 2}`），仅 `method = MITIGATION_ZNE` 时读取。
- `num_levels` (`uintptr_t`)：折叠等级数量。
- `copies` (`uintptr_t`)：虚拟蒸馏副本份数，仅 `method = MITIGATION_VIRTUAL_DISTILLATION` 时读取。

返回：

- `CErrorMitigation *`：堆分配管线，需 `error_mitigation_free` 释放；失败或参数与方法不匹配（如 ZNE 未给 `fold_levels`）返回 NULL。

### error_mitigation_run(ptr, method, hamiltonian, shots_a, shots_b, estimator)

执行采样阶段，逐线路调用 `estimator` 收集各噪声因子下的期望估计。

参数：

- `method` (`uint8_t`)：必须与 `error_mitigation_new` 配置一致。
- `hamiltonian` (`const CHamiltonian *`)：待估计的可观测量。
- `shots_a` (`uintptr_t`)：ZNE 的 shot 数；虚拟蒸馏的分子线路 shot 数。
- `shots_b` (`uintptr_t`)：虚拟蒸馏的分母线路 shot 数（ZNE 忽略）。
- `estimator` (`CEstimatorFn`)：估计器回调，不得为 NULL。

**shots 语义**：

- ZNE：仅使用 `shots_a`，`0` 表示"未指定 shots"（由回调自行决定）；`shots_b` 被忽略。
- VirtualDistillation：`shots_a` 与 `shots_b` 分别作为分子/分母线路的 shot 数，**都必须 > 0**，否则失败。

返回：

- `int32_t`；成功 `0`，失败返回负错误码。

### error_mitigation_get_mitigated(ptr, process_method, degree, out_expectation, out_variance)

`run` 之后产出最终缓解估计。

参数：

- `process_method` (`uint8_t`)：`PROCESS_*` 常量，需与管线方法匹配（ZNE 管线用 `PROCESS_ZNE_POLYNOMIAL`/`PROCESS_ZNE_EXPONENTIAL`，虚拟蒸馏管线用 `PROCESS_VIRTUAL_DISTILLATION`）。
- `degree` (`uintptr_t`)：多项式外推次数（仅 `PROCESS_ZNE_POLYNOMIAL`），`0` 表示自动选择。
- `out_expectation` (`double *`)：缓解后的期望值。
- `out_variance` (`double *`)：估计方差，不可用时为 NaN。

返回：

- `int32_t`；成功 `0`；未先 `run` 返回 `-7`。

### error_mitigation_free(ptr)

释放对象。允许传 NULL。

---

## 估计器回调 CEstimatorFn

```c
typedef void (*CEstimatorFn)(const struct CCircuit *circuit,
                             const struct CHamiltonian *hamiltonian,
                             uintptr_t shots,
                             double *expectation, double *variance);
```

回调契约：

- `run` 过程中**逐线路**调用（ZNE 对每个噪声因子各调用一次；虚拟蒸馏对分子/分母线路分别调用）；
- 回调方在其中执行实际采样（Statevector/密度矩阵模拟或设备任务均可），把估计期望值写入 `*expectation`；
- 方差写入 `*variance`，未知可写 NaN；
- `shots` 为 `0` 表示调用方未指定 shot 数，可自行选择；
- **所有指针仅在回调期间有效**：不得保存 `circuit`/`hamiltonian` 供回调返回后使用，**不得在回调内释放 `hamiltonian`**；
- 回调不应长期阻塞——它会在采样阶段被反复调用。

---

## 完整示例：ZNE

```c
/* 估计器：真实场景替换为带噪声执行 */
static void counting_estimator(const CCircuit *qc,
                               const CHamiltonian *obs,
                               uintptr_t shots,
                               double *expectation, double *variance) {
    CStatevector *sv = statevector_from_circuit(qc);
    double e = 0.0;
    statevector_expectation(sv, obs, &e);   // 无噪声参考实现
    statevector_free(sv);
    *expectation = e;
    *variance = 0.0;
    (void)shots;
}

int32_t levels[3] = {0, 1, 2};
CErrorMitigation *em =
    error_mitigation_new(qc, MITIGATION_ZNE, levels, 3, 0);
if (!em) { return 1; }

if (error_mitigation_run(em, MITIGATION_ZNE, obs, 4000, 0,
                         counting_estimator) != 0) {
    error_mitigation_free(em);
    return 1;
}

double mitigated, variance;
error_mitigation_get_mitigated(em, PROCESS_ZNE_POLYNOMIAL, 0,
                               &mitigated, &variance);
printf("mitigated = %.6f (var %.6f)\n", mitigated, variance);

error_mitigation_free(em);
```

### 虚拟蒸馏路线

```c
CErrorMitigation *em =
    error_mitigation_new(qc, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, 2);

/* 分子 4000 shots，分母 4000 shots（都必须 > 0） */
error_mitigation_run(em, MITIGATION_VIRTUAL_DISTILLATION, obs,
                     4000, 4000, counting_estimator);

double mitigated, variance;
error_mitigation_get_mitigated(em, PROCESS_VIRTUAL_DISTILLATION, 0,
                               &mitigated, &variance);
error_mitigation_free(em);
```

需要直接检查折叠线路时见 [ZNEMitigation](2_zne_mitigation.md)；虚拟蒸馏协议细节见 [VirtualDistillation](3_virtual_distillation.md)。
