# ZNEMitigation

ZNE（零噪声外推）的独立使用入口。适合需要**直接检查折叠线路**、自行组织采样流程的场景；端到端缓解流程（采样 + 外推后处理）见 [ErrorMitigation](1_error_mitigation.md)。

---

## 折叠与噪声因子

ZNE 的核心思想：对线路做门级折叠（gate folding）人为放大噪声，在多个噪声因子下测量同一观测量，再外推到零噪声点。

- 折叠等级 `level` 对应噪声因子 `2 * level + 1`；

| level | 噪声因子 | 含义 |
| --- | --- | --- |
| 0 | 1 | 原线路（不折叠） |
| 1 | 3 | 每个门等效执行 3 次（门-逆-门） |
| 2 | 5 | 每个门等效执行 5 次 |

- **全局折叠**：整条线路按噪声因子重复；
- **选择性折叠**：仅对指定门名的门折叠，其余门保持不变。

---

## 函数

### zne_mitigation_new(circuit, fold_levels, num_levels)

按折叠等级创建 ZNE 辅助对象。

参数：

- `circuit` (`const CCircuit *`)：目标线路。
- `fold_levels` (`const int32_t *`)：折叠等级数组，如 `{0, 1, 2}`。
- `num_levels` (`uintptr_t`)：等级数量。

返回：

- `CZneMitigation *`：堆分配对象，需 `zne_mitigation_free` 释放；失败返回 NULL。

### zne_mitigation_fold_circuit(ptr, gate_names)

生成各级折叠线路。

参数：

- `gate_names` (`const char *`)：NULL 表示全局折叠；传逗号分隔门名（如 `"H,CX"`）表示仅折叠这些门。

返回：

- `CCircuitList *`：**独立拥有**的线路列表（每个等级一条），需 `circuit_list_free` 释放；失败返回 NULL。

### zne_mitigation_noise_factor(ptr, index)

返回第 `index` 级的噪声因子（`2 * level + 1`）。

返回：

- `int32_t`；错误返回 `0`。

### zne_mitigation_free(ptr)

释放对象。允许传 NULL。

---

## CircuitList 遍历

```c
uintptr_t circuit_list_len(const struct CCircuitList *ptr);
struct CCircuit *circuit_list_get(const struct CCircuitList *ptr, uintptr_t index);
void circuit_list_free(struct CCircuitList *ptr);
```

| 函数 | 返回 | 说明 |
| --- | --- | --- |
| `circuit_list_len` | 数量 | 折叠线路条数（等于等级数），NULL 返回 `0` |
| `circuit_list_get(index)` | `CCircuit *` | **独立拥有**的线路副本，需 `circuit_free`；越界返回 NULL |
| `circuit_list_free` | `void` | 释放列表，允许传 NULL |

注意 `circuit_list_get` 返回的线路与列表独立：只释放列表不释放取出的线路会泄漏；反之在释放列表前释放取出的线路是安全的。

---

## 示例

### 检查各级折叠线路

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

### 自行外推

对每个噪声因子执行采样得到期望值序列后，由调用方自行外推（多项式拟合、指数拟合等，可用任何数值库）。需要内置外推时直接使用 [ErrorMitigation](1_error_mitigation.md) 的 `run` + `get_mitigated` 流程。
