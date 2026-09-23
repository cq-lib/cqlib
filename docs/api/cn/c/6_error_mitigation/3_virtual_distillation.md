# VirtualDistillation

虚拟蒸馏（Virtual Distillation）误差缓解的构建入口。通过对量子态做多次副本复制与 copy-swap 干涉，在期望值估计中二次压低噪声贡献。端到端缓解流程（含采样与后处理）见 [ErrorMitigation](1_error_mitigation.md)（`method = MITIGATION_VIRTUAL_DISTILLATION`）。

---

## 函数

### virtual_distillation_new(circuit, copies)

对 `circuit` 创建 `copies` 份副本寄存器的虚拟蒸馏辅助对象。

参数：

- `circuit` (`const CCircuit *`)：目标线路。
- `copies` (`uintptr_t`)：副本寄存器份数，需 > 1。

返回：

- `CVirtualDistillation *`：堆分配对象，需 `virtual_distillation_free` 释放；失败返回 NULL。

### virtual_distillation_build_circuit(ptr)

构建协议所需的 copy-swap 线路。

返回：

- `CCircuit *`：**独立拥有**的新线路，需 `circuit_free` 释放；失败返回 NULL。

构建出的线路比特数多于原线路（需容纳全部副本寄存器），可用于检查协议结构或送入模拟器验证。

### virtual_distillation_free(ptr)

释放对象。允许传 NULL。

---

## 与 ErrorMitigation 的关系

完整的"采样 + 后处理"流程由 [ErrorMitigation](1_error_mitigation.md) 承担：

1. `error_mitigation_new(circuit, MITIGATION_VIRTUAL_DISTILLATION, NULL, 0, copies)`；
2. `error_mitigation_run(..., shots_a, shots_b, estimator)`——`shots_a`/`shots_b` 分别是分子/分母线路的 shot 数，都必须 > 0；
3. `error_mitigation_get_mitigated(..., PROCESS_VIRTUAL_DISTILLATION, ...)`。

本页接口适合只需要构建和检查协议线路的场景。

---

## 示例

```c
CVirtualDistillation *vd = virtual_distillation_new(qc, 2);
if (!vd) { return 1; }

CCircuit *protocol = virtual_distillation_build_circuit(vd);
printf("protocol qubits=%zu (原线路 %zu)\n",
       (size_t)circuit_num_qubits(protocol),
       (size_t)circuit_num_qubits(qc));
circuit_free(protocol);

virtual_distillation_free(vd);
```
