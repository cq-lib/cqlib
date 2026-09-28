# 设备（C）

C 绑定的设备接口以不透明句柄（`CTopology`、`CDevice`、`CLayout`、`CNoiseModel`、`CExecutionResult`）加自由函数的形式描述量子设备：连接拓扑、校准属性、逻辑到物理的映射、噪声模型与任务执行结果。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 概念关系

- `CDevice` 聚合一张 `CTopology` 与一组属性默认值：拓扑描述物理比特与耦合构成的有向图；设备在其上叠加设备名、原生门集、不可用比特、校准时间与设备级默认属性。查询比特属性时局部值优先，缺失时回退默认值。
- `CLayout` 是逻辑比特与物理比特之间的双向映射，用于路由过程；`layout_bind` / `layout_unbind` / `layout_swap_physical` 是移动逻辑比特的基本操作。
- `CNoiseModel` 独立于设备构建，按「门 + 比特」组合登记噪声信道与读出误差。
- `CExecutionResult` 承载一次任务的执行结果：状态机（排队 → 运行 → 完成 / 失败 / 取消）、测量计数与概率分布。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [Topology](1_topology.md) | `CTopology`：拓扑构造、比特与耦合编辑、连接性与度查询。 |
| [Device / Properties](2_properties_device.md) | `CDevice`：构造工厂、原生门集、校验、误差属性、默认值与属性结构体。 |
| [Layout](3_layout.md) | `CLayout`：逻辑↔物理映射的构造、绑定与交换。 |
| [NoiseModel](4_noise.md) | `CNoiseModel`：单比特、双比特与读出噪声的登记和查询。 |
| [ExecutionResult](5_result.md) | `CExecutionResult` 与 `CCountsList`：任务状态机、计数与概率。 |
| [量子比特标识](6_qubits.md) | `CLogicalQubit` / `CPhysicalQubit`：设备侧强类型比特标识及其与 `CQubit` 的互转。 |

---

## 共同约定

- 所有句柄用对应的 `*_free` 函数释放，传 NULL 均合法：`topology_free`、`device_free`、`layout_free`、`noise_model_free`、`execution_result_free`、`counts_list_free`。
- `device_topology` 返回拓扑的克隆副本，用 `topology_free` 释放。
- 返回 `char*` 的接口（如 `device_name`、`counts_list_get_key`）用 `cqlib_string_free` 释放。
- 数组输出采用两步式：先调用 `*_len` 函数取总数，分配缓冲区后再调用填充函数（如 `device_usable_qubits_len` + `device_usable_qubits`），填充函数同样返回总数。

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 直线拓扑设备：0 -> 1 -> 2 */
    struct CDevice *dev = device_line("demo", 3);
    if (dev == NULL) {
        return 1;
    }

    char *name = device_name(dev);          /* "demo" */
    uintptr_t usable = device_num_qubits(dev);  /* 3 */
    cqlib_string_free(name);

    /* 拓扑克隆副本 */
    struct CTopology *topo = device_topology(dev);
    int32_t directed = topology_supports_directed_coupling(topo, 0, 1);  /* 1 */
    int32_t reverse = topology_supports_directed_coupling(topo, 1, 0);  /* 0 */
    topology_free(topo);

    /* 设备级默认值与回退查询 */
    device_set_default_t1(dev, 40.0);
    double t1 = 0.0;
    device_get_t1(dev, 0, &t1);  /* 0（成功），t1 == 40.0 */

    device_free(dev);
    return 0;
}
```

编译器接口以 `CDevice` 与 `CLayout` 作为设备目标的输入，见 [Compiler](../4_compile/1_compiler.md)。
