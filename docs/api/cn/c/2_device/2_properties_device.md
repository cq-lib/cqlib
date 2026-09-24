# Device / Properties（C）

`CDevice` 是设备的不透明句柄：在拓扑之上聚合设备名、原生门集、比特集合、校准时间与属性默认值，并提供校验和误差查询。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

查询比特属性时局部值优先：`device_get_t1` 等接口先读取该比特记录的属性，取不到时回退设备级默认值。比特或边一旦记录了自己的原生指令列表，该列表就是此处能力的完整覆盖，不会回写设备级默认门集。

---

## 属性结构体

### CQubitProp

单个物理比特的属性快照，由 `device_qubit_properties` 写出：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `readout_error` | `double` | 读出误差率，[0, 1]。 |
| `t1` | `double` | T1 弛豫时间（μs）；未设置时为 NaN。 |
| `t2` | `double` | T2 去相干时间（μs）；未设置时为 NaN。 |
| `prob_meas0_prep1` | `double` | P(制备 1 测得 0)；未设置时为 NaN。 |
| `prob_meas1_prep0` | `double` | P(制备 0 测得 1)；未设置时为 NaN。 |
| `frequency` | `double` | 频率（GHz）；未设置时为 NaN。 |
| `num_native_instructions` | `uintptr_t` | 该比特记录的原生指令数，用作 `device_qubit_prop_native_instruction` 的下标上界。 |

### CNativeInstruction

单条原生指令能力快照，由 `device_qubit_prop_native_instruction` / `device_edge_prop_native_instruction` 写出：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `name` | `char*` | 堆上标准门名，用 `cqlib_string_free` 释放。 |
| `error_rate` | `double` | 误差率，[0, 1]。 |
| `length` | `double` | 时长（ns）；未设置时为 NaN。 |

### CEdgeProp

单条有向耦合边的属性快照，由 `device_edge_properties` 写出：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `num_native_instructions` | `uintptr_t` | 该边记录的原生指令数，用作 `device_edge_prop_native_instruction` 的下标上界。 |

### CQubitPropInput

`device_add_qubit_properties` 的输入。可选的 `double` 字段用 NaN 表示未设置；原生指令以长度为 `num_native_instructions` 的并行数组给出（门名、误差率、时长），`native_lengths` 可为 NULL 表示全部时长未设置：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `readout_error` | `double` | 读出误差率，[0, 1]。 |
| `t1` | `double` | T1 弛豫时间（μs）；NaN = 未设置。 |
| `t2` | `double` | T2 去相干时间（μs）；NaN = 未设置。 |
| `prob_meas0_prep1` | `double` | P(制备 1 测得 0)；NaN = 未设置。 |
| `prob_meas1_prep0` | `double` | P(制备 0 测得 1)；NaN = 未设置。 |
| `frequency` | `double` | 频率（GHz）；NaN = 未设置。 |
| `native_gate_names` | `const char *const*` | 标准门名数组，无指令时为 NULL。 |
| `native_error_rates` | `const double*` | 误差率数组，无指令时为 NULL。 |
| `native_lengths` | `const double*` | 时长数组（ns，NaN = 未设置），可为 NULL。 |
| `num_native_instructions` | `uintptr_t` | 原生指令数。 |

### CEdgePropInput

`device_add_edge_properties` 的输入，字段与 `CQubitPropInput` 的原生指令部分一致，且只接受双比特标准门：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `native_gate_names` | `const char *const*` | 标准门名数组，无指令时为 NULL。 |
| `native_error_rates` | `const double*` | 误差率数组，无指令时为 NULL。 |
| `native_lengths` | `const double*` | 时长数组（ns，NaN = 未设置），可为 NULL。 |
| `num_native_instructions` | `uintptr_t` | 原生指令数。 |

---

## 构造

### device_new(name, num_qubits)

创建一个含 `num_qubits` 个孤立物理比特、无耦合的设备；自定义拓扑用 `device_from_edges`。

参数：

- `name` (`const char*`)：设备名。
- `num_qubits` (`uint32_t`)：物理比特数。

返回：成功返回新建的 `CDevice*`；失败返回 NULL。

### device_line(name, num_qubits)

创建有向直线拓扑 `0 -> 1 -> ... -> n-1` 的设备。

### device_bidirectional_line(name, num_qubits)

创建双向直线拓扑的设备。

### device_ring(name, num_qubits)

创建双向环形拓扑的设备。

### device_grid(name, rows, cols)

创建双向网格拓扑的设备，比特按行优先编号。

参数：

- `rows` (`uint32_t`)：行数。
- `cols` (`uint32_t`)：列数。

### device_star(name, num_qubits, center)

创建以 `center` 为中心的双向星形拓扑设备。

参数：

- `num_qubits` (`uint32_t`)：比特总数。
- `center` (`uint32_t`)：中心比特 ID。

### device_from_edges(name, num_qubits, edges, num_edges)

由显式的有向边创建设备。`edges` 指向 `2 * num_edges` 个 u32，按 `(control, target)` 对连续排列。

参数：

- `num_qubits` (`uint32_t`)：物理比特数。
- `edges` (`const uint32_t*`)：耦合边数组。
- `num_edges` (`uintptr_t`)：边的数量。

返回：成功返回新建的 `CDevice*`；失败返回 NULL。

### device_line_from_qubits(name, qubits, num_qubits)

按给定顺序把比特 ID 数组连成有向直线，创建设备。

参数：

- `qubits` (`const uint32_t*`)：物理比特 ID 数组。
- `num_qubits` (`uintptr_t`)：数组长度。

返回：成功返回新建的 `CDevice*`；失败返回 NULL。

### device_free(ptr)

释放 `CDevice`，允许传 NULL。

参数：

- `ptr` (`struct CDevice*`)：待释放的句柄。

---

## 基本信息与原生门

### device_name(ptr)

返回设备名。

返回：成功返回堆上 C 字符串（用 `cqlib_string_free` 释放）；失败返回 NULL。

### device_num_qubits(ptr)

返回可用物理比特数；`ptr` 为 NULL 时返回 0。

### device_native_gates(ptr)

返回原生门名列表。

返回：成功返回逗号分隔的堆上 C 字符串（如 `"H,CX"`），用 `cqlib_string_free` 释放；失败或门集为空时返回 NULL。

### device_with_native_gates(ptr, gate_names)

用 `gate_names`（逗号分隔，如 `"H,CX,RZ"`）替换设备的原生门集。

参数：

- `gate_names` (`const char*`)：门名列表。

返回：0 成功；-4 任一门名未知；其他负值为错误码。

### device_topology(ptr)

返回设备拓扑的克隆副本，用 `topology_free` 释放。

返回：成功返回新建的 `CTopology*`；失败返回 NULL。

---

## 校验

### device_validate_circuit(ptr, circuit)

校验线路是否与设备兼容。

参数：

- `circuit` (`const struct CCircuit*`)：待校验的线路。

返回：0 兼容；-3 校验失败；其他负值为错误码。

### device_validate_operation(ptr, operation)

在物理比特 ID 空间中校验一个来自 `circuit_index` 的已解析操作快照。

参数：

- `operation` (`const struct CValueOperation*`)：操作快照。

返回：0 合法；-3 设备拒绝该操作；-1 空指针。

### device_validate_value_operation(ptr, operation)

在物理比特 ID 空间中校验一个来自 `circuit_index` 的值级操作快照；控制流体会被递归校验。

参数：

- `operation` (`const struct CValueOperation*`)：操作快照。

返回：0 合法；-3 设备拒绝该操作；-1 空指针。

### device_supports_native_instruction(ptr, gate_name, qargs, num_qargs)

判断标准门 `gate_name` 能否在给定有序物理比特上原生执行。

参数：

- `gate_name` (`const char*`)：标准门名。
- `qargs` (`const uint32_t*`)：物理比特 ID 数组；`GPhase` 允许空数组。
- `num_qargs` (`uintptr_t`)：数组长度。

返回：1 支持；0 不支持；-1 空指针；-4 门名未知。校准数据不影响该能力查询。

---

## 误差与相干属性

### device_get_t1(ptr, qubit, out)

返回比特的 T1 弛豫时间（μs），局部属性优先，缺失时回退设备默认值。

参数：

- `qubit` (`uint32_t`)：物理比特 ID。
- `out` (`double*`)：写出结果。

返回：0 并写入 `*out`；-8 该比特无 T1 数据。

### device_get_t2(ptr, qubit, out)

返回比特的 T2 去相干时间（μs），回退规则同上。返回：0 并写入 `*out`；-8 未设置。

### device_get_readout_error(ptr, qubit, out)

返回比特的读出误差率，回退规则同上。返回：0 并写入 `*out`；-8 未设置。

### device_single_qubit_error(ptr, gate_name, qubit, out)

返回单比特标准门 `gate_name` 在 `qubit` 上的误差率，局部原生指令属性优先，缺失时回退设备默认单比特误差率。

参数：

- `gate_name` (`const char*`)：标准门名。
- `qubit` (`uint32_t`)：物理比特 ID。
- `out` (`double*`)：写出结果。

返回：0 并写入 `*out`；-4 门名未知；-8 该比特不支持此门。

### device_two_qubit_error(ptr, gate_name, control, target, out)

返回双比特标准门 `gate_name` 在有向耦合 `control -> target` 上的误差率，回退规则同上。

返回：0 并写入 `*out`；-4 门名未知；-8 该耦合不支持此门。

### device_edge_error(ptr, control, target, out)

返回有向耦合 `control -> target` 的方向性误差：该边上标定最好的原生双比特误差，缺失时取设备默认值。

返回：0 并写入 `*out`；-8 该有向耦合不存在。

### device_calibration_time(ptr, out_unix_ms)

返回系统标定时间戳（Unix 纪元起的毫秒数）。

参数：

- `out_unix_ms` (`int64_t*`)：写出结果。

返回：0 并写入 `*out`；-8 未设置。

---

## 比特属性的读取与设置

以下 getter 读取单个比特记录的属性值；与 `device_get_t1` 不同，频率与制备-测量概率的 getter 没有设备级默认值回退。setter 只修改该比特已记录属性（例如通过 `device_add_qubit_properties` 写入）中的单个字段，其余字段全部保留，也不会创建属性记录。带下标的变体寻址原生指令列表，上界为 `device_qubit_properties` 快照中的 `num_native_instructions`。未设置的可选值以 -8 报告，并向 `*out` 写入 NaN。

### device_get_frequency(ptr, qubit, out)

返回比特记录的频率（GHz），无默认值回退。

参数：

- `qubit` (`uint32_t`)：物理比特 ID。
- `out` (`double*`)：写出结果。

返回：0 并写入 `*out`；-8 该比特没有记录属性或未设置频率。

### device_get_prob_meas0_prep1(ptr, qubit, out)

返回比特记录的 P(制备 1 测得 0)，无默认值回退。

返回：0 并写入 `*out`；-8 该比特没有记录属性或未设置该概率。

### device_get_prob_meas1_prep0(ptr, qubit, out)

返回比特记录的 P(制备 0 测得 1)，无默认值回退。

返回：0 并写入 `*out`；-8 该比特没有记录属性或未设置该概率。

### device_get_error_rate(ptr, qubit, index, out)

返回比特记录的下标为 `index` 的原生指令误差率。

参数：

- `index` (`uintptr_t`)：原生指令下标。
- `out` (`double*`)：写出结果。

返回：0 并写入 `*out`；-8 该比特没有记录属性或下标越界。

### device_get_length(ptr, qubit, index, out)

返回比特记录的下标为 `index` 的原生指令时长（ns）。

返回：0 并写入 `*out`；-8 该比特没有记录属性、下标越界或时长未设置。

### device_set_t1(ptr, qubit, t1)

设置比特记录的 T1 弛豫时间（μs）。

返回：0 成功；-1 空指针；-8 该比特没有记录属性。

### device_set_t2(ptr, qubit, t2)

设置比特记录的 T2 去相干时间（μs）。

返回：0 成功；-1 空指针；-8 该比特没有记录属性。

### device_set_frequency(ptr, qubit, frequency)

设置比特记录的频率（GHz）。

返回：0 成功；-1 空指针；-8 该比特没有记录属性。

### device_set_prob_meas0_prep1(ptr, qubit, prob)

设置比特记录的 P(制备 1 测得 0)。

返回：0 成功；-1 空指针；-8 该比特没有记录属性。

### device_set_prob_meas1_prep0(ptr, qubit, prob)

设置比特记录的 P(制备 0 测得 1)。

返回：0 成功；-1 空指针；-8 该比特没有记录属性。

### device_set_error_rate(ptr, qubit, index, error_rate)

设置比特记录的下标为 `index` 的原生指令误差率。

返回：0 成功；-1 空指针；-8 该比特没有记录属性或下标越界。

### device_set_length(ptr, qubit, index, length)

设置比特记录的下标为 `index` 的原生指令时长（ns）。

返回：0 成功；-1 空指针；-8 该比特没有记录属性或下标越界。

### device_set_instruction(ptr, qubit, index, gate_name)

替换比特记录的下标为 `index` 的原生指令所承载的标准门。编辑后的列表会重新校验，把单比特门改成双比特门（或反过来）会被拒绝。

参数：

- `index` (`uintptr_t`)：原生指令下标。
- `gate_name` (`const char*`)：替换后的标准门名。

返回：0 成功；-1 空指针；-4 门名未知；-8 该比特没有记录属性、下标越界或编辑后的列表未通过校验。

### device_set_native_instruction(ptr, qubit, gate_name, error_rate, length)

向比特记录的属性追加一条原生单比特指令（标准门 `gate_name`，误差率 `error_rate`，时长 `length`（ns）；`length` 为 NaN 表示未设置）。

参数：

- `gate_name` (`const char*`)：标准门名。
- `error_rate` (`double`)：误差率，[0, 1]。
- `length` (`double`)：时长（ns）；NaN = 未设置。

返回：0 成功；-1 空指针；-4 门名未知；-8 该比特没有记录属性，或指令被拒绝（元数不符或标定值非法）。

---

## 默认值

### device_default_t1(ptr, out)

返回设备级默认 T1 时间（μs）。返回：0 并写入 `*out`；-8 未设置默认值。

### device_default_t2(ptr, out)

返回设备级默认 T2 时间（μs）。返回：0 并写入 `*out`；-8 未设置。

### device_default_readout_error(ptr, out)

返回设备级默认读出误差率。返回：0 并写入 `*out`；-8 未设置。

### device_default_single_qubit_error(ptr, out)

返回设备级默认单比特门误差率。返回：0 并写入 `*out`；-8 未设置。

### device_default_two_qubit_error(ptr, out)

返回设备级默认双比特门误差率。返回：0 并写入 `*out`；-8 未设置。

### device_set_default_t1(ptr, t1)

设置设备级默认 T1 时间（μs）。返回：0 成功；负值为错误码。

### device_set_default_t2(ptr, t2)

设置设备级默认 T2 时间（μs）。返回：0 成功；负值为错误码。

### device_set_default_readout_error(ptr, error)

设置设备级默认读出误差率。返回：0 成功；负值为错误码。

### device_set_default_single_qubit_error(ptr, error)

设置设备级默认单比特门误差率。返回：0 成功；负值为错误码。

### device_set_default_two_qubit_error(ptr, error)

设置设备级默认双比特门误差率。返回：0 成功；负值为错误码。

---

## 可用比特

### device_invalid_qubits_len(ptr)

返回不可用（离线 / 故障）比特数；`ptr` 为 NULL 时返回 0。

### device_invalid_qubits(ptr, out, len)

两步式输出不可用比特 ID，与 `device_invalid_qubits_len` 配对；返回总数。

### device_set_invalid_qubits(ptr, qubits, len)

替换不可用比特集合。

参数：

- `qubits` (`const uint32_t*`)：比特 ID 数组。
- `len` (`uintptr_t`)：数组长度。

返回：0 成功；-1 空指针；-2 任一比特未在设备中登记（此时原集合保持不变）。

### device_is_usable_qubit(ptr, qubit)

判断比特是否已登记且未标记为不可用。

返回：1 可用；0 不可用或未登记；-1 空指针。

### device_num_usable_qubits(ptr)

返回可用比特数；`ptr` 为 NULL 时返回 0。

### device_usable_qubits_len(ptr) / device_usable_qubits(ptr, out, len)

两步式输出可用比特 ID；填充函数返回总数。

### device_qubits_len(ptr) / device_qubits(ptr, out, len)

两步式输出全部已登记的物理比特 ID（含不可用比特）；填充函数返回总数。

---

## 比特与边属性

### device_qubit_properties(ptr, qubit, out)

写出比特的属性快照。

参数：

- `qubit` (`uint32_t`)：物理比特 ID。
- `out` (`struct CQubitProp*`)：写出快照。

返回：0 成功；-8 该比特没有记录属性（快照中的可选字段为 NaN）。

### device_qubit_prop_native_instruction(ptr, qubit, index, out)

写出该比特下标为 `index` 的原生指令。

参数：

- `qubit` (`uint32_t`)：物理比特 ID。
- `index` (`uintptr_t`)：指令下标，上界为快照中的 `num_native_instructions`。
- `out` (`struct CNativeInstruction*`)：写出快照；`name` 需用 `cqlib_string_free` 释放。

返回：0 成功；-8 该比特没有记录属性或下标越界。

### device_edge_properties(ptr, control, target, out)

写出有向耦合 `control -> target` 的属性快照。

返回：0 成功；-8 该边没有记录属性。

### device_edge_prop_native_instruction(ptr, control, target, index, out)

写出该有向边下标为 `index` 的原生指令（双比特标准门）。

返回：0 成功；-8 该边没有记录属性或下标越界；`name` 需用 `cqlib_string_free` 释放。

### device_add_qubit_properties(ptr, qubit, input)

记录（或替换）比特的属性。

参数：

- `qubit` (`uint32_t`)：物理比特 ID。
- `input` (`const struct CQubitPropInput*`)：属性输入。

返回：0 成功；-1 空指针；-2 比特未在设备中登记或不在其拓扑中；-4 原生门名未知；-8 原生指令标定值非法。

### device_add_edge_properties(ptr, control, target, input)

记录（或替换）有向耦合 `control -> target` 的属性。

参数：

- `input` (`const struct CEdgePropInput*`)：属性输入。

返回：0 成功；-1 空指针；-2 该有向耦合不在设备拓扑中；-4 门名未知；-8 原生指令不是双比特标准门或标定值非法。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 直线拓扑：0 -> 1 -> 2 */
    struct CDevice *dev = device_line("mock_backend", 3);
    if (dev == NULL) {
        return 1;
    }

    /* 设备级默认值 */
    device_set_default_t1(dev, 50.0);
    device_set_default_t2(dev, 35.0);
    device_set_default_readout_error(dev, 0.05);

    /* 比特 0 的局部属性：读出误差 0.02、T1 80、原生 H 指令 */
    struct CQubitPropInput input;
    input.readout_error = 0.02;
    input.t1 = 80.0;
    input.t2 = 70.0;
    input.prob_meas0_prep1 = 0.01;
    input.prob_meas1_prep0 = 0.03;
    input.frequency = 5.0;

    const char *names[1] = {"H"};
    const double rates[1] = {0.001};
    input.native_gate_names = names;
    input.native_error_rates = rates;
    input.native_lengths = NULL;
    input.num_native_instructions = 1;

    int32_t rc = device_add_qubit_properties(dev, 0, &input);  /* 0 */

    /* 边 0 -> 1 的局部属性：原生 CX 指令 */
    struct CEdgePropInput einput;
    const char *enames[1] = {"CX"};
    const double erates[1] = {0.02};
    einput.native_gate_names = enames;
    einput.native_error_rates = erates;
    einput.native_lengths = NULL;
    einput.num_native_instructions = 1;

    rc = device_add_edge_properties(dev, 0, 1, &einput);  /* 0 */

    /* 查询：局部优先、默认值回退 */
    double t1_0 = 0.0, t1_2 = 0.0;
    device_get_t1(dev, 0, &t1_0);  /* 0，t1_0 == 80.0（局部） */
    device_get_t1(dev, 2, &t1_2);  /* 0，t1_2 == 50.0（默认值） */

    double sq_err = 0.0, tq_err = 0.0, edge_err = 0.0;
    device_single_qubit_error(dev, "H", 0, &sq_err);   /* 0，0.001 */
    device_two_qubit_error(dev, "CX", 0, 1, &tq_err);  /* 0，0.02 */
    device_edge_error(dev, 0, 1, &edge_err);           /* 0，0.02 */

    /* 比特属性 getter（无默认值回退） */
    double freq = 0.0, pm01 = 0.0, pm10 = 0.0, rate = 0.0, len = 0.0;
    device_get_frequency(dev, 0, &freq);         /* 0，freq == 5.0 */
    device_get_prob_meas0_prep1(dev, 0, &pm01);  /* 0，pm01 == 0.01 */
    device_get_prob_meas1_prep0(dev, 0, &pm10);  /* 0，pm10 == 0.03 */
    device_get_error_rate(dev, 0, 0, &rate);     /* 0，rate == 0.001 */
    device_get_length(dev, 0, 0, &len);          /* -8：时长未设置，len == NaN */

    /* 比特属性 setter：每次改一个字段，其余保留 */
    device_set_t1(dev, 0, 85.0);                 /* 0 */
    device_set_frequency(dev, 0, 5.1);           /* 0 */
    device_get_frequency(dev, 0, &freq);         /* 0，freq == 5.1 */
    device_set_error_rate(dev, 0, 0, 0.002);     /* 0 */
    device_set_length(dev, 0, 0, 50.0);          /* 0 */
    device_set_t1(dev, 2, 1.0);                  /* -8：比特 2 没有记录属性 */

    /* 原生指令编辑 */
    device_set_instruction(dev, 0, 0, "X");                   /* 0：H -> X，标定值保留 */
    device_set_native_instruction(dev, 0, "Z", 0.003, 80.0);  /* 0：追加 */

    /* 属性快照与原生指令 */
    struct CQubitProp prop;
    if (device_qubit_properties(dev, 0, &prop) == 0) {
        for (uintptr_t i = 0; i < prop.num_native_instructions; i++) {
            struct CNativeInstruction ni;
            if (device_qubit_prop_native_instruction(dev, 0, i, &ni) == 0) {
                printf("%s rate=%f\n", ni.name, ni.error_rate);
                cqlib_string_free(ni.name);
            }
        }
    }

    /* 不可用比特 */
    uint32_t bad[1] = {2};
    device_set_invalid_qubits(dev, bad, 1);
    int32_t usable = device_is_usable_qubit(dev, 2);      /* 0 */
    uintptr_t n_usable = device_num_usable_qubits(dev);  /* 2 */

    /* 原生门集 */
    rc = device_with_native_gates(dev, "H,CX,RZ");  /* 0 */
    char *gates = device_native_gates(dev);          /* "H,CX,RZ" */
    cqlib_string_free(gates);

    device_free(dev);
    return 0;
}
```
