# Device

`CDevice` 描述量子处理器的物理属性：比特数量、耦合拓扑与原生门集。

---

## 构造与释放

### device_new(name, num_qubits)

创建包含 `num_qubits` 个**孤立**物理比特、无任何耦合的设备。

### device_line(name, num_qubits)

创建有向线拓扑 `0 -> 1 -> ... -> n-1` 的设备。

### device_bidirectional_line(name, num_qubits)

创建双向线拓扑的设备。

### device_ring(name, num_qubits)

创建双向环拓扑的设备。

### device_grid(name, rows, cols)

创建双向网格拓扑的设备，比特编号按行主序（`row * cols + col`）分配，总比特数为 `rows * cols`。

### device_star(name, num_qubits, center)

创建围绕 `center` 比特的双向星形拓扑设备。

### device_from_edges(name, num_qubits, edges, num_edges)

由显式有向边创建设备。

参数：

- `name` (`const char *`)：设备名称。
- `num_qubits` (`uint32_t`)：物理比特总数。
- `edges` (`const uint32_t *`)：`2 * num_edges` 个 u32，按 `(control, target)` 对连续排列。
- `num_edges` (`uintptr_t`)：边数。

注意：

- `num_edges` 不做奇偶校验，按对解析；引用超出 `num_qubits` 范围的比特会导致构造失败。

以上构造函数全部返回：

- `CDevice *`：堆分配设备，需 `device_free` 释放；失败返回 NULL。

### device_free(ptr)

释放设备对象。允许传 NULL。

---

## 属性

### device_name(device)

返回设备名称。

返回：

- `char *`：堆分配字符串，需 `cqlib_string_free` 释放；失败返回 NULL。

### device_num_qubits(device)

返回可用物理比特数量。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### device_native_gates(device)

返回逗号分隔的原生门名列表。

返回：

- `char *`：堆分配字符串，需 `cqlib_string_free` 释放；门集为空或失败返回 NULL。

### device_topology(device)

返回设备拓扑的独立克隆，详见 [Topology](1_topology.md)。

---

## 原生门集与校验

### device_with_native_gates(device, gate_names)

以逗号分隔的门名列表**替换**设备的原生门集。

参数：

- `gate_names` (`const char *`)：如 `"H,CX,RZ"`。

返回：

- `int32_t`；成功 `0`，任一门名未知返回 `-4`。

### device_validate_circuit(device, circuit)

校验线路与设备是否兼容：线路使用的门是否都在原生门集内、双比特门是否落在耦合边上。

参数：

- `circuit` (`const CCircuit *`)：待校验线路。

返回：

- `int32_t`；兼容 `0`，不兼容 `-3`，参数错误返回负错误码。

---

## 示例

### 构造与查询

```c
CDevice *dev = device_bidirectional_line("line-4", 4);
device_with_native_gates(dev, "H,CX,RZ,X2P");

char *name = device_name(dev);            // "line-4"
char *gates = device_native_gates(dev);   // "H,CX,RZ,X2P"
printf("%s: %s (%zu qubits)\n", name, gates,
       (size_t)device_num_qubits(dev));
cqlib_string_free(gates);
cqlib_string_free(name);

device_free(dev);
```

### 自定义拓扑与线路校验

```c
/* 0<->1, 1<->2 双向耦合 */
uint32_t edges[4] = {0, 1, 1, 0, 1, 2, 2, 1};
CDevice *dev = device_from_edges("custom", 3, edges, 4);
device_with_native_gates(dev, "H,CZ,RZ");

CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cz(qc, 0, 1);

int32_t ok = device_validate_circuit(dev, qc);
printf("compatible=%d\n", ok == 0);

circuit_free(qc);
device_free(dev);
```

比特布局（逻辑 → 物理映射）见 [Layout](3_layout.md)；噪声模型见 [NoiseModel](4_noise.md)。
