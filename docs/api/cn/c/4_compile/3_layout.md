# 初始布局（C）

本页覆盖初始布局算法与布局分析工具的 C ABI：四个布局入口（`trivial_layout` / `greedy_layout` / `vf2_perfect_layout` / `sabre_layout`）、线路交互分析与物理布局图查询、SABRE 预备对象与预计算布局管线，以及布局结果 `CLayoutResult` 的读取接口。布局把逻辑比特映射到物理比特，为路由阶段提供起点。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)；映射句柄 `CLayout` 的构造与读写接口见 [Layout](../2_device/3_layout.md)。

---

## 常量

布局目标标签（各布局入口与 `score_layout` 的 `objective` 参数）：

| 常量 | 值 | 目标 |
| --- | --- | --- |
| `LAYOUT_OBJECTIVE_TOPOLOGY_ONLY` | 0 | 纯拓扑目标：距离加方向失配加权。 |
| `LAYOUT_OBJECTIVE_FIDELITY_AWARE` | 1 | 保真度感知目标：在拓扑项之上追加错误率项。 |
| `LAYOUT_OBJECTIVE_AUTO` | 2 | 由设备的校准数据自动选择：物理图含校准数据时为保真度感知目标，否则回退纯拓扑目标。 |
| `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` | 3 | 强制保真度：与保真度感知相同，但物理图不含可用校准数据时报错。 |

VF2 边要求标签（`CVf2LayoutConfig.edge_requirement`）：

| 常量 | 值 | 要求 |
| --- | --- | --- |
| `VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS` | 0 | 只要求正权重交互边匹配。 |
| `VF2_EDGE_REQUIREMENT_ALL_INTERACTIONS` | 1 | 要求全部交互边匹配。 |

---

## 数据结构

### CLayoutScore

布局打分快照，由 `score_layout` 与 `layout_result_score` 写入：

```c
typedef struct CLayoutScore {
  double total;              /* Weighted total according to the objective. */
  double distance;           /* Raw weighted-distance component. */
  double direction;          /* Raw direction-mismatch component. */
  double two_qubit_error;    /* Unweighted effective two-qubit placement cost. */
  double readout_error;      /* Raw readout error component. */
  uint8_t used_fidelity;     /* 1 when the objective used fidelity terms. */
  uint8_t _reserved[7];      /* Reserved padding. */
} CLayoutScore;
```

| 字段 | 含义 |
| --- | --- |
| `total` | 按目标加权后的总得分，越小越好。 |
| `distance` | 原始加权距离分量。 |
| `direction` | 原始方向失配分量。 |
| `two_qubit_error` | 未加权的有效二比特放置代价。 |
| `readout_error` | 原始读出错误分量。 |
| `used_fidelity` | 目标使用了保真度项时为 1。 |

### CInteraction

一条逻辑比特对交互快照，由 `circuit_layout_analysis_interaction` 写入：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `left` | `uint32_t` | 按升序排列的较小逻辑比特端点。 |
| `right` | `uint32_t` | 较大逻辑比特端点。 |
| `weight` | `double` | 该无序比特对的总交互权重。 |
| `directed_weight_left_to_right` | `double` | 按 `left -> right` 操作顺序观测到的权重。 |
| `directed_weight_right_to_left` | `double` | 按 `right -> left` 操作顺序观测到的权重。 |
| `first_seen_order` | `uintptr_t` | 在操作顺序中的首次出现序号。 |

### CVf2LayoutConfig

VF2 精确布局配置：

```c
typedef struct CVf2LayoutConfig {
  uintptr_t candidate_limit;   /* Max complete perfect candidates to score. */
  uintptr_t call_limit;        /* Max partial mapping extensions ((uintptr_t)-1 = none). */
  uint8_t edge_requirement;    /* One of VF2_EDGE_REQUIREMENT_*. */
  uint8_t _reserved[7];        /* Reserved padding. */
} CVf2LayoutConfig;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `candidate_limit` | `uintptr_t` | 参与打分的完整完美候选映射数量上限。 |
| `call_limit` | `uintptr_t` | 尝试的部分映射扩展次数上限；`(uintptr_t)-1` 表示不设上限。 |
| `edge_requirement` | `uint8_t` | 取 `VF2_EDGE_REQUIREMENT_*` 之一。 |
| `_reserved[7]` | `uint8_t[7]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |

### vf2_layout_config_default()

```c
struct CVf2LayoutConfig vf2_layout_config_default(void);
```

按值返回默认 VF2 配置：`candidate_limit = 10`、`call_limit = (uintptr_t)-1`（无上限）、`edge_requirement = VF2_EDGE_REQUIREMENT_POSITIVE_INTERACTIONS`。

---

## 布局入口

四个入口均不修改输入线路，返回的 `CLayoutResult*` 用 `layout_result_free` 释放。`objective` 取 `LAYOUT_OBJECTIVE_*` 之一；`LAYOUT_OBJECTIVE_AUTO` 依据设备的校准数据解析（物理图由设备派生），`LAYOUT_OBJECTIVE_FIDELITY_REQUIRED` 则要求目标必须携带可用校准数据，否则所有入口均返回 NULL。

### trivial_layout(circuit, device, objective)

```c
struct CLayoutResult *trivial_layout(const struct CCircuit *circuit,
                                     const struct CDevice *device,
                                     uint8_t objective);
```

平凡布局：按现有顺序把逻辑比特映射到可用物理比特（逻辑 i 映射到第 i 个可用物理比特）。

- `circuit` (`const CCircuit*`)：输入线路。
- `device` (`const CDevice*`)：目标设备（构造方式见 [设备与噪声属性](../2_device/2_properties_device.md)）。
- `objective` (`uint8_t`)：布局目标标签。

返回：成功返回新建的 `CLayoutResult*`；任一参数为 NULL、`objective` 标签未知或可用物理比特不足时返回 NULL。

### greedy_layout(circuit, device, objective)

```c
struct CLayoutResult *greedy_layout(const struct CCircuit *circuit,
                                    const struct CDevice *device,
                                    uint8_t objective);
```

贪心布局：按交互强度顺序放置逻辑比特，优先满足强交互对。

参数与返回值同 `trivial_layout`：成功返回新建的 `CLayoutResult*`；任一参数为 NULL、`objective` 标签未知或执行失败返回 NULL。

### vf2_perfect_layout(circuit, device, objective, config)

```c
struct CLayoutResult *vf2_perfect_layout(const struct CCircuit *circuit,
                                         const struct CDevice *device,
                                         uint8_t objective,
                                         const struct CVf2LayoutConfig *config);
```

用非诱导 VF2++ 匹配搜索完美初始布局：当线路交互图能精确嵌入设备拓扑时，每个正权重交互都落在相邻物理边上。

- `circuit` (`const CCircuit*`)：输入线路。
- `device` (`const CDevice*`)：目标设备。
- `objective` (`uint8_t`)：布局目标标签。
- `config` (`const CVf2LayoutConfig*`)：VF2 配置；传 NULL 使用默认配置。

返回：成功返回新建的 `CLayoutResult*`；不存在完美映射、任一指针参数为 NULL、`objective` 标签未知或 `config->edge_requirement` 非法时返回 NULL。

### sabre_layout(circuit, device, objective, config)

```c
struct CLayoutResult *sabre_layout(const struct CCircuit *circuit,
                                   const struct CDevice *device,
                                   uint8_t objective,
                                   const struct SabreConfigC *config);
```

用 SABRE 布局细化选择初始布局：多个候选布局经前向/后向细化与打分后择优。

- `circuit` (`const CCircuit*`)：输入线路。
- `device` (`const CDevice*`)：目标设备。
- `objective` (`uint8_t`)：布局目标标签。
- `config` (`const SabreConfigC*`)：SABRE 配置；传 NULL 使用默认配置，字段说明见 [SABRE 配置](5_sabre.md)。

返回：成功返回新建的 `CLayoutResult*`；任一指针参数为 NULL、`objective` 标签未知或配置非法时返回 NULL。

---

## 线路布局分析

`analyze_circuit_for_layout` 预计算线路的加权逻辑交互，供预计算布局入口复用。

### analyze_circuit_for_layout(circuit)

```c
struct CCircuitLayoutAnalysis *analyze_circuit_for_layout(const struct CCircuit *circuit);
```

分析线路的布局特征（逻辑比特表与加权交互图）。

- `circuit` (`const CCircuit*`)：输入线路。

返回：成功返回新建的 `CCircuitLayoutAnalysis*`（用 `circuit_layout_analysis_free` 释放）；NULL 输入或失败返回 NULL。

### circuit_layout_analysis_free(ptr)

```c
void circuit_layout_analysis_free(struct CCircuitLayoutAnalysis *ptr);
```

释放线路布局分析句柄。允许传 NULL。

### circuit_layout_analysis_num_logical(ptr)

```c
uintptr_t circuit_layout_analysis_num_logical(const struct CCircuitLayoutAnalysis *ptr);
```

返回分析中的逻辑比特数。

返回：逻辑比特数；NULL 输入返回 0。

### circuit_layout_analysis_logical_qubits(ptr, out, len)

```c
uintptr_t circuit_layout_analysis_logical_qubits(const struct CCircuitLayoutAnalysis *ptr,
                                                 uint32_t *out,
                                                 uintptr_t len);
```

两步式读取：按源线路顺序把逻辑比特 ID 拷入 `out`。先以 `len = 0`、`out = NULL` 调用查总数，再分配缓冲后以 `len` 等于总数调用填充。

- `ptr` (`const CCircuitLayoutAnalysis*`)：分析句柄。
- `out` (`uint32_t*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲容量。

返回：逻辑比特总数；NULL 输入返回 0。

### circuit_layout_analysis_interactions_len(ptr)

```c
uintptr_t circuit_layout_analysis_interactions_len(const struct CCircuitLayoutAnalysis *ptr);
```

返回加权逻辑交互条数。

返回：交互条数；NULL 输入返回 0。

### circuit_layout_analysis_interaction(ptr, index, out)

```c
int32_t circuit_layout_analysis_interaction(const struct CCircuitLayoutAnalysis *ptr,
                                            uintptr_t index,
                                            struct CInteraction *out);
```

把第 `index` 条交互快照写入 `*out`（字段见上文 `CInteraction` 表）。

- `ptr` (`const CCircuitLayoutAnalysis*`)：分析句柄。
- `index` (`uintptr_t`)：交互下标，必须小于 `circuit_layout_analysis_interactions_len`。
- `out` (`CInteraction*`)：交互快照输出。

返回：`0` 成功；`-1` 任一参数为 NULL；`-8` `index` 越界。

---

## 物理布局图

`CPhysicalLayoutGraph` 是设备的编译器侧物理拓扑与校准视图：可用物理比特、最短距离、耦合方向与校准错误率。

### physical_layout_graph_from_device(device)

```c
struct CPhysicalLayoutGraph *physical_layout_graph_from_device(const struct CDevice *device);
```

由设备构建物理布局图。

- `device` (`const CDevice*`)：目标设备。

返回：成功返回新建的 `CPhysicalLayoutGraph*`（用 `physical_layout_graph_free` 释放）；NULL 输入或失败返回 NULL。

### physical_layout_graph_free(ptr)

```c
void physical_layout_graph_free(struct CPhysicalLayoutGraph *ptr);
```

释放物理布局图句柄。允许传 NULL。

### physical_layout_graph_num_physical(ptr)

```c
uintptr_t physical_layout_graph_num_physical(const struct CPhysicalLayoutGraph *ptr);
```

返回可用物理比特数。

返回：可用物理比特数；NULL 输入返回 0。

### physical_layout_graph_physical_qubits(ptr, out, len)

```c
uintptr_t physical_layout_graph_physical_qubits(const struct CPhysicalLayoutGraph *ptr,
                                                uint32_t *out,
                                                uintptr_t len);
```

两步式读取：按升序把可用物理比特 ID 拷入 `out`。先查总数（`physical_layout_graph_num_physical`），再分配缓冲填充。

- `ptr` (`const CPhysicalLayoutGraph*`)：物理布局图句柄。
- `out` (`uint32_t*`)：输出缓冲。
- `len` (`uintptr_t`)：缓冲容量。

返回：可用物理比特总数；NULL 输入返回 0。

### physical_layout_graph_distance(ptr, a, b)

```c
uint32_t physical_layout_graph_distance(const struct CPhysicalLayoutGraph *ptr,
                                        uint32_t a,
                                        uint32_t b);
```

返回两个物理比特之间的无向最短路径距离。

- `ptr` (`const CPhysicalLayoutGraph*`)：物理布局图句柄。
- `a`、`b` (`uint32_t`)：物理比特 ID。

返回：最短距离；两比特不连通、未知或句柄为 NULL 时返回 `UINT32_MAX`。

### physical_layout_graph_is_adjacent_undirected(ptr, a, b)

```c
int32_t physical_layout_graph_is_adjacent_undirected(const struct CPhysicalLayoutGraph *ptr,
                                                     uint32_t a,
                                                     uint32_t b);
```

判断两个物理比特是否无向相邻（直接耦合）。

返回：`1` 相邻；`0` 不相邻；`-1` 空指针。

### physical_layout_graph_readout_error(ptr, qubit, out)

```c
int32_t physical_layout_graph_readout_error(const struct CPhysicalLayoutGraph *ptr,
                                            uint32_t qubit,
                                            double *out);
```

读取一个物理比特的读出错误率。

- `ptr` (`const CPhysicalLayoutGraph*`)：物理布局图句柄。
- `qubit` (`uint32_t`)：物理比特 ID。
- `out` (`double*`)：错误率输出。

返回：`0` 成功（值写入 `*out`）；`-1` 任一参数为 NULL；`-8` 该比特没有记录读出数据。

### physical_layout_graph_supports_directed_coupling(ptr, control, target)

```c
int32_t physical_layout_graph_supports_directed_coupling(const struct CPhysicalLayoutGraph *ptr,
                                                         uint32_t control,
                                                         uint32_t target);
```

判断有向耦合 `control -> target` 是否存在。

- `control`、`target` (`uint32_t`)：物理比特 ID。

返回：`1` 有向耦合存在；`0` 不存在；`-1` 空指针。

### physical_layout_graph_supports_two_qubit_gate_directed(ptr, gate_name, control, target)

```c
int32_t physical_layout_graph_supports_two_qubit_gate_directed(const struct CPhysicalLayoutGraph *ptr,
                                                               const char *gate_name,
                                                               uint32_t control,
                                                               uint32_t target);
```

判断 `gate_name` 是否是有向边 `control -> target` 上的原生能力。

- `gate_name` (`const char*`)：标准门名称（如 `"CX"`、`"SWAP"`）。
- `control`、`target` (`uint32_t`)：物理比特 ID。

返回：`1` 支持；`0` 不支持；`-1` 空指针；`-4` 门名称未知。

### physical_layout_graph_two_qubit_gate_error_directed(ptr, gate_name, control, target, out)

```c
int32_t physical_layout_graph_two_qubit_gate_error_directed(const struct CPhysicalLayoutGraph *ptr,
                                                            const char *gate_name,
                                                            uint32_t control,
                                                            uint32_t target,
                                                            double *out);
```

读取 `gate_name` 在有向边 `control -> target` 上的校准错误率。

- `gate_name` (`const char*`)：标准门名称。
- `control`、`target` (`uint32_t`)：物理比特 ID。
- `out` (`double*`)：错误率输出。

返回：`0` 成功（值写入 `*out`）；`-1` 空指针；`-4` 门名称未知；`-8` 该门在此边上不受支持或未校准。

### physical_layout_graph_has_fidelity_data(ptr) / has_readout_error_data / has_two_qubit_error_data

```c
int32_t physical_layout_graph_has_fidelity_data(const struct CPhysicalLayoutGraph *ptr);
int32_t physical_layout_graph_has_readout_error_data(const struct CPhysicalLayoutGraph *ptr);
int32_t physical_layout_graph_has_two_qubit_error_data(const struct CPhysicalLayoutGraph *ptr);
```

判断校准数据的可用性：任一读出或二比特校准数据 / 读出错误数据 / 二比特错误数据是否可用。`LAYOUT_OBJECTIVE_AUTO` 依据这些标志选择保真度感知或纯拓扑目标。

返回：`1` 数据可用；`0` 不可用；`-1` 空指针。

---

## SABRE 预备对象

预计算管线把 SABRE 需要的线路侧与设备侧数据一次构建、多次复用：`prepare_sabre_circuit` 准备线路侧分析，`prepare_sabre_device_target` 准备设备侧精确数据。

### prepare_sabre_circuit(circuit)

```c
struct CPreparedSabreCircuit *prepare_sabre_circuit(const struct CCircuit *circuit);
```

准备 SABRE 使用的线路侧分析与依赖模型。

- `circuit` (`const CCircuit*`)：输入线路。

返回：成功返回新建的 `CPreparedSabreCircuit*`（用 `prepared_sabre_circuit_free` 释放）；NULL 输入或失败返回 NULL。

### prepared_sabre_circuit_free(ptr)

```c
void prepared_sabre_circuit_free(struct CPreparedSabreCircuit *ptr);
```

释放预备线路句柄。允许传 NULL。

### prepared_sabre_circuit_logical_qubits_len(ptr)

```c
uintptr_t prepared_sabre_circuit_logical_qubits_len(const struct CPreparedSabreCircuit *ptr);
```

返回预备线路的逻辑比特数。

返回：逻辑比特数；NULL 输入返回 0。

### prepared_sabre_circuit_logical_qubits(ptr, out, len)

```c
uintptr_t prepared_sabre_circuit_logical_qubits(const struct CPreparedSabreCircuit *ptr,
                                                uint32_t *out,
                                                uintptr_t len);
```

两步式读取：按源线路顺序把预备线路的逻辑比特 ID 拷入 `out`。先查总数，再分配缓冲填充。

返回：逻辑比特总数；NULL 输入返回 0。

### prepare_sabre_device_target(prepared, device)

```c
struct CPreparedSabreTarget *prepare_sabre_device_target(const struct CPreparedSabreCircuit *prepared,
                                                         const struct CDevice *device);
```

为预备线路准备精确设备原生的 SABRE 数据：包含原生操作可行性、方向与实现代价。

**前提**：设备必须声明路由能力，即满足以下条件之一——设备级原生门集非空（例如调用 `device_with_native_gates` 设置，见 [设备与噪声属性](../2_device/2_properties_device.md)）；至少一个可用物理比特的属性含原生指令；或至少一条可用耦合边的属性含原生指令。未声明路由能力的设备会让此调用返回 NULL。

- `prepared` (`const CPreparedSabreCircuit*`)：预备线路句柄（`prepare_sabre_circuit` 的结果）。
- `device` (`const CDevice*`)：目标设备。

返回：成功返回新建的 `CPreparedSabreTarget*`（用 `prepared_sabre_target_free` 释放）；任一参数为 NULL、设备未声明路由能力或失败返回 NULL。

### prepared_sabre_target_free(ptr)

```c
void prepared_sabre_target_free(struct CPreparedSabreTarget *ptr);
```

释放预备目标句柄。允许传 NULL。

### prepared_sabre_target_physical(ptr)

```c
struct CPhysicalLayoutGraph *prepared_sabre_target_physical(const struct CPreparedSabreTarget *ptr);
```

取出预备目标使用的物理布局图（深拷贝），用于布局目标打分。

返回：成功返回新建的 `CPhysicalLayoutGraph*`（用 `physical_layout_graph_free` 释放）；NULL 输入返回 NULL。

---

## 预计算布局算法

预计算入口接受 `analyze_circuit_for_layout` 的分析与 `CPhysicalLayoutGraph`（或 SABRE 预备对象），跳过重复分析。

### trivial_layout_prepared(analysis, physical, objective)

```c
struct CLayoutResult *trivial_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                              const struct CPhysicalLayoutGraph *physical,
                                              uint8_t objective);
```

在预计算分析与物理图上执行平凡布局。`LAYOUT_OBJECTIVE_AUTO` 依据传入物理图的校准数据解析。

- `analysis` (`const CCircuitLayoutAnalysis*`)：线路布局分析。
- `physical` (`const CPhysicalLayoutGraph*`)：物理布局图。
- `objective` (`uint8_t`)：布局目标标签。

返回：成功返回新建的 `CLayoutResult*`；任一参数为 NULL、`objective` 标签未知或执行失败返回 NULL。

### greedy_layout_prepared(analysis, physical, objective)

```c
struct CLayoutResult *greedy_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                             const struct CPhysicalLayoutGraph *physical,
                                             uint8_t objective);
```

在预计算分析与物理图上执行贪心布局。参数与返回值同 `trivial_layout_prepared`。

### vf2_perfect_layout_prepared(analysis, physical, objective, config)

```c
struct CLayoutResult *vf2_perfect_layout_prepared(const struct CCircuitLayoutAnalysis *analysis,
                                                  const struct CPhysicalLayoutGraph *physical,
                                                  uint8_t objective,
                                                  const struct CVf2LayoutConfig *config);
```

在预计算分析与物理图上执行 VF2 精确布局；`config` 传 NULL 使用默认配置。

返回：成功返回新建的 `CLayoutResult*`；不存在完美映射、参数非法或执行失败返回 NULL。

### sabre_layout_prepared(prepared, prepared_target, objective, config)

```c
struct CLayoutResult *sabre_layout_prepared(const struct CPreparedSabreCircuit *prepared,
                                            const struct CPreparedSabreTarget *prepared_target,
                                            uint8_t objective,
                                            const struct SabreConfigC *config);
```

在预备线路与预备目标上执行 SABRE 布局；`LAYOUT_OBJECTIVE_AUTO` 依据预备目标的物理图解析，`config` 传 NULL 使用默认配置。

- `prepared` (`const CPreparedSabreCircuit*`)：预备线路。
- `prepared_target` (`const CPreparedSabreTarget*`)：预备目标（含物理图与原生代价）。
- `objective` (`uint8_t`)：布局目标标签。
- `config` (`const SabreConfigC*`)：SABRE 配置，见 [SABRE 配置](5_sabre.md)。

返回：成功返回新建的 `CLayoutResult*`；任一指针参数为 NULL、`objective` 标签未知或配置非法时返回 NULL。

### score_layout(objective, analysis, physical, layout, out)

```c
int32_t score_layout(uint8_t objective,
                     const struct CCircuitLayoutAnalysis *analysis,
                     const struct CPhysicalLayoutGraph *physical,
                     const struct CLayout *layout,
                     struct CLayoutScore *out);
```

按布局目标给候选布局打分，快照写入 `*out`（字段见上文 `CLayoutScore` 表）。`layout` 必须把 `analysis` 中的逻辑比特映射到 `physical` 中的物理比特。

- `objective` (`uint8_t`)：布局目标标签。
- `analysis` (`const CCircuitLayoutAnalysis*`)：线路布局分析。
- `physical` (`const CPhysicalLayoutGraph*`)：物理布局图。
- `layout` (`const CLayout*`)：候选布局，见 [Layout](../2_device/3_layout.md)。
- `out` (`CLayoutScore*`)：打分快照输出。

返回：`0` 成功；`-1` 任一参数为 NULL；`-8` `objective` 标签未知或目标无法解析（如物理图无校准数据时的 `LAYOUT_OBJECTIVE_FIDELITY_REQUIRED`）；`-6` 核心打分失败。

---

## 布局结果访问

`CLayoutResult` 承载选中的映射、可选打分与诊断信息。

### layout_result_free(ptr)

```c
void layout_result_free(struct CLayoutResult *ptr);
```

释放布局结果句柄。允许传 NULL。

### layout_result_layout(ptr)

```c
struct CLayout *layout_result_layout(const struct CLayoutResult *ptr);
```

取出选中的逻辑→物理映射（深拷贝）。

返回：成功返回新建的 `CLayout*`（用 `layout_free` 释放）；NULL 输入返回 NULL。

### layout_result_has_score(ptr)

```c
int32_t layout_result_has_score(const struct CLayoutResult *ptr);
```

判断结果是否携带观测打分。

返回：`1` 有打分；`0` 无打分；`-1` 空指针。

### layout_result_score(ptr, out)

```c
int32_t layout_result_score(const struct CLayoutResult *ptr, struct CLayoutScore *out);
```

把打分快照写入 `*out`。

返回：`0` 成功；`-1` 任一参数为 NULL；`-8` 结果没有打分。

### layout_result_is_perfect(ptr)

```c
int32_t layout_result_is_perfect(const struct CLayoutResult *ptr);
```

判断是否为完美布局：每个正权重交互都落在相邻物理边上。

返回：`1` 完美；`0` 不完美；`-1` 空指针。

### layout_result_candidates_evaluated(ptr)

```c
uintptr_t layout_result_candidates_evaluated(const struct CLayoutResult *ptr);
```

返回算法考察过的候选布局数。

返回：候选数；NULL 输入返回 0。

### layout_result_used_fidelity(ptr)

```c
int32_t layout_result_used_fidelity(const struct CLayoutResult *ptr);
```

判断保真度数据是否参与了选中布局的打分。

返回：`1` 参与了打分；`0` 未参与；`-1` 空指针。

### layout_result_notes_len(ptr) / layout_result_note(ptr, index)

```c
uintptr_t layout_result_notes_len(const struct CLayoutResult *ptr);
char *layout_result_note(const struct CLayoutResult *ptr, uintptr_t index);
```

两步式读取诊断说明：`notes_len` 返回说明条数（NULL 返回 0），`note` 返回第 `index` 条说明的堆上 C 字符串（用 `cqlib_string_free` 释放）；越界返回 NULL。

---

## 示例

四个布局入口与结果读取：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* Directed line 0 -> 1 -> 2 built from an edge list */
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);   /* non-adjacent pair on the line */
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* Trivial layout maps logical i to physical i */
    struct CLayoutResult *trivial =
        trivial_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    if (trivial != NULL) {
        struct CLayout *mapping = layout_result_layout(trivial);
        uintptr_t n = layout_num_logical(mapping);   /* 3 */
        layout_free(mapping);
        layout_result_free(trivial);
    }

    /* Greedy and SABRE layouts run on the same circuit */
    struct CLayoutResult *greedy =
        greedy_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY);
    if (greedy != NULL) {
        layout_result_free(greedy);
    }

    struct CLayoutResult *sabre =
        sabre_layout(circuit, device, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
    if (sabre != NULL) {
        uintptr_t evaluated = layout_result_candidates_evaluated(sabre);  /* >= 1 */
        layout_result_free(sabre);
    }

    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

预计算管线：线路分析、物理图、SABRE 预备对象与打分（设备先声明原生门集）：

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    uint32_t edges[4] = {0, 1, 1, 2};
    struct CDevice *device = device_from_edges("line-3", 3, edges, 2);
    struct CCircuit *circuit = circuit_new(3);
    circuit_cx(circuit, 0, 2);
    if (device == NULL || circuit == NULL) {
        return 1;
    }

    /* Exact device-native SABRE requires declared routing capability */
    if (device_with_native_gates(device, "H,CX,SWAP") != 0) {
        circuit_free(circuit);
        device_free(device);
        return 1;
    }

    /* Circuit-side analysis: one cx(0, 2) collapses to one interaction pair */
    struct CCircuitLayoutAnalysis *analysis = analyze_circuit_for_layout(circuit);
    uintptr_t num_interactions = circuit_layout_analysis_interactions_len(analysis);
    for (uintptr_t i = 0; i < num_interactions; i++) {
        struct CInteraction interaction;
        if (circuit_layout_analysis_interaction(analysis, i, &interaction) == 0) {
            printf("interaction %u-%u weight %f\n",
                   interaction.left, interaction.right, interaction.weight);
        }
    }

    /* Physical graph view of the device */
    struct CPhysicalLayoutGraph *graph = physical_layout_graph_from_device(device);
    uintptr_t num_physical = physical_layout_graph_num_physical(graph);  /* 3 */
    uint32_t *physical_ids = malloc(num_physical * sizeof(uint32_t));
    physical_layout_graph_physical_qubits(graph, physical_ids, num_physical);
    uint32_t dist = physical_layout_graph_distance(graph, 0, 2);  /* 2 */
    int32_t adjacent =
        physical_layout_graph_is_adjacent_undirected(graph, 0, 2); /* 0 */
    free(physical_ids);

    /* Prepared SABRE pipeline */
    struct CPreparedSabreCircuit *prepared = prepare_sabre_circuit(circuit);
    struct CPreparedSabreTarget *target = prepare_sabre_device_target(prepared, device);
    if (prepared != NULL && target != NULL) {
        struct CPhysicalLayoutGraph *target_graph = prepared_sabre_target_physical(target);
        physical_layout_graph_free(target_graph);

        struct CLayoutResult *result = sabre_layout_prepared(
            prepared, target, LAYOUT_OBJECTIVE_TOPOLOGY_ONLY, NULL);
        if (result != NULL) {
            struct CLayout *mapping = layout_result_layout(result);

            /* Score the selected mapping against the prepared analysis */
            struct CLayoutScore score;
            if (score_layout(LAYOUT_OBJECTIVE_TOPOLOGY_ONLY,
                             analysis, graph, mapping, &score) == 0) {
                printf("total=%f distance=%f\n", score.total, score.distance);
            }
            layout_free(mapping);
            layout_result_free(result);
        }
        prepared_sabre_target_free(target);
        prepared_sabre_circuit_free(prepared);
    }

    physical_layout_graph_free(graph);
    circuit_layout_analysis_free(analysis);
    circuit_free(circuit);
    device_free(device);
    return 0;
}
```

选中布局交给路由阶段插入 SWAP 的流程见 [路由](4_routing.md)；设备目标编译一步完成布局与路由的入口见 [编译器](1_compiler.md)。
