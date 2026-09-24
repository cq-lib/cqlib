# 编译优化（C）

编译模块以自由函数 + 不透明句柄（`CCompileResult`）的形式提供线路优化工作流：按 `CompileConfigC` 配置运行逻辑编译，或针对 `CDevice` 设备目标运行路由与原生门降级，并把优化后的线路、变更标记与逐步骤报告写入编译结果。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 概念关系

- `compile` 按值接收 `CompileConfigC`：`mode` 选择普通或增强模式，`target` 选择编译目标；非 `Logical` 目标按逻辑编译处理。
- `compile_with_device` 是设备目标入口：线路在设备拓扑上路由，并降级到设备的有序原生门集，使结果线路始终被该设备接受；可选的初始布局与随机种子控制映射过程。
- `CCompileResult` 承载优化后的线路（深拷贝取出）、整体变更标记、编译模式与逐步骤报告；设备目标编译额外记录初始与最终布局（`CLayout`，见 [Layout](../2_device/3_layout.md)）。

---

## 页面导航

| 页面 | 内容 |
| --- | --- |
| [Compiler](1_compiler.md) | `compile` / `compile_with_device` 与 `CCompileResult` 的构造、结果读取、步骤报告与布局读取。 |
| [Transform](2_transform.md) | 独立变换 pass：结构分析 `circuit_analyze`、规范化 `canonicalize_circuit`、知识重写 `rewrite_circuit` 与 `transform_*` 封装。 |
| [Initial Layout](3_layout.md) | 初始布局算法 `trivial_layout` / `greedy_layout` / `vf2_perfect_layout` / `sabre_layout`、线路交互分析、物理布局图、SABRE 预备对象与布局结果读取。 |
| [Routing](4_routing.md) | 路由入口 `route_with_layout` / `route_sabre`、`CRoutedCircuit` 路由结果与布局可达性校验。 |
| [SABRE Configuration](5_sabre.md) | `SabreConfigC` 配置族与已知初始布局条件下的 `sabre_route` 路由。 |
| [Decompose / Resynthesis](6_decompose_resynthesis.md) | 定义展开、酉门合成、多控门分解与双比特块重综合。 |
| [Knowledge](7_knowledge.md) | 知识规则的构造、校验与结构匹配。 |
| [Resource](8_resource.md) | Ancilla 资源策略、上限与租借管理。 |
| [Commutation](9_commutation.md) | 操作间对易性的保守检查。 |

---

## CompileConfigC

`compile` 的输入配置，按值传递：

```c
typedef struct CompileConfigC {
  uint8_t mode;                 /* One of COMPILE_MODE_*. */
  uint8_t target;               /* One of COMPILE_TARGET_*. */
  uint8_t allow_dirty_ancilla;  /* Reserved for future use. */
  uint8_t allow_clean_ancilla;  /* Reserved for future use. */
  uint8_t _reserved[4];         /* Reserved padding. */
} CompileConfigC;
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `mode` | `uint8_t` | 编译模式，取 `COMPILE_MODE_*` 之一。 |
| `target` | `uint8_t` | 编译目标，取 `COMPILE_TARGET_*` 之一；`Logical` 目标只读取 `mode`。 |
| `allow_dirty_ancilla` | `uint8_t` | 保留字段（资源策略标志），当前不参与编译。 |
| `allow_clean_ancilla` | `uint8_t` | 保留字段（资源策略标志），当前不参与编译。 |
| `_reserved[4]` | `uint8_t[4]` | 填充字段，保持结构体布局稳定，勿写入非零值。 |

`Logical` 目标只使用 `mode`；其余目标值按逻辑编译处理。全部字段保留在结构体中以便后续无 ABI 破坏地扩展。

---

## 常量

编译模式标签（`CompileConfigC.mode` 与 `compile_with_device` 的 `mode` 参数）：

| 常量 | 值 | 模式 |
| --- | --- | --- |
| `COMPILE_MODE_NORMAL` | 0 | 普通编译模式。 |
| `COMPILE_MODE_ENHANCED` | 1 | 增强编译模式。 |

编译目标标签（`CompileConfigC.target`）：

| 常量 | 值 | 目标 |
| --- | --- | --- |
| `COMPILE_TARGET_LOGICAL` | 0 | 逻辑比特目标。 |
| `COMPILE_TARGET_BASIS` | 1 | 基门集目标。 |
| `COMPILE_TARGET_DEVICE` | 2 | 设备目标。 |
| `COMPILE_TARGET_TOPOLOGY_BASIS` | 3 | 拓扑基门集目标。 |

---

## 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* 增强模式逻辑编译 */
    struct CompileConfigC config = {0};
    config.mode = COMPILE_MODE_ENHANCED;
    config.target = COMPILE_TARGET_LOGICAL;

    struct CCompileResult *result = compile(qc, config);
    if (result == NULL) {
        circuit_free(qc);
        return 1;
    }

    int32_t changed = compile_result_changed(result);   /* 1：工作流改动了线路 */
    struct CCircuit *optimized = compile_result_circuit(result);
    if (optimized != NULL) {
        circuit_free(optimized);  /* 深拷贝，独立于 result 释放 */
    }

    compile_result_free(result);
    circuit_free(qc);
    return 0;
}
```

设备目标编译（含路由与布局读取）的完整流程见 [Compiler](1_compiler.md)。
