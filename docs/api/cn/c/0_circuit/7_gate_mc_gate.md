# 多控制门

本页介绍 C API 中向任意标准门追加控制位的 `circuit_multi_control` 接口；错误码与内存管理的全局约定见 [Overview](../0_overview.md)。

---

## circuit_multi_control(ptr, gate_name, controls, controls_len, targets, targets_len, params, params_len)

向线路追加一个多控制标准门：仅当所有控制位处于 `1` 态时，基础门才作用于其目标比特。

参数：

- `ptr` (`struct CCircuit *`)：目标线路句柄。
- `gate_name` (`const char *`)：基础标准门名，取值见下文门名表。
- `controls` (`const uint32_t *`)：控制比特索引数组。
- `controls_len` (`uintptr_t`)：控制位数量，可为 0。
- `targets` (`const uint32_t *`)：基础门作用比特数组，长度须等于基础门自身的比特数。
- `targets_len` (`uintptr_t`)：`targets` 的长度。
- `params` (`const struct CParameter *const *`)：基础门参数句柄数组，长度须等于基础门的参数数；无参数门传 `NULL, 0`。
- `params_len` (`uintptr_t`)：`params` 的长度。

操作的比特列表为 `[controls..., targets...]`：新增控制位排在最前，其后是基础门自身所需的比特；若基础门自带控制位（如 `"CX"`），其内部控制位仍按该门原有顺序保留。例如以 `"X"` 为基础门、两个控制位作用于 `[c0, c1, t]` 时，`c0`、`c1` 为控制位，`t` 为翻转目标；以 `"CX"` 为基础门、一个新增控制位时，三个比特依次为 `[新控制位, CX 控制位, CX 目标位]`。

返回：`int32_t`，`0` 成功；`-1` NULL 输入（`ptr` 或 `gate_name` 为 NULL，或长度非 0 时数组为 NULL）；`-2` 比特越界；`-3` 基础门不可受控或比特、参数数量不匹配；`-4` `gate_name` 含无效 UTF-8；`-8` 未知门名。

参数以 `CParameter` 符号表达式传入（数值以表达式形式书写，如 `"pi/2"`），按位置对应基础门的参数（例如 `"U"` 需 3 个、`"RXY"` 与 `"fSim"` 各需 2 个、`"RX"` 需 1 个）。

### 支持的基础门名

`I`、`H`、`X`、`Y`、`Z`、`S`、`SDG`、`T`、`TDG`、`X2P`、`X2M`、`Y2P`、`Y2M`、`RX`、`RY`、`RZ`、`Phase`、`GPhase`、`U`、`XY`、`XY2P`、`XY2M`、`RXY`、`SWAP`、`RXX`、`RYY`、`RZZ`、`RZX`、`CX`、`CY`、`CZ`、`CCX`、`CRX`、`CRY`、`CRZ`、`fSim`（`FSIM` 与 `fSim` 均可）。

### MCX 示例

```c
#include "cqlib_c.h"

CCircuit *c = circuit_new(4);

/* 双控制 X（Toffoli 语义）：controls = {0, 1}, target = 2 */
uint32_t ccx_controls[2] = {0, 1};
uint32_t ccx_target[1] = {2};

if (circuit_multi_control(c, "X",
                          ccx_controls, 2,
                          ccx_target, 1,
                          NULL, 0) != 0) {
    /* 处理错误 */
}

/* 三控制 H */
uint32_t mch_controls[3] = {0, 1, 2};
uint32_t mch_target[1] = {3};

if (circuit_multi_control(c, "H",
                          mch_controls, 3,
                          mch_target, 1,
                          NULL, 0) != 0) {
    /* 处理错误 */
}

/* 单控制 RZ(theta)，角度为符号表达式 */
CParameter *theta = param_parse("theta");
const struct CParameter *crz_params[1] = {theta};
uint32_t crz_control[1] = {0};
uint32_t crz_target[1] = {1};

if (circuit_multi_control(c, "RZ",
                          crz_control, 1,
                          crz_target, 1,
                          crz_params, 1) != 0) {
    /* 处理错误 */
}

param_free(theta);
circuit_free(c);
```

---

## 与标准受控门的关系

常见受控门是 `circuit_multi_control` 以特定基础门调用的特例：

| 标准门函数 | 等价调用 |
| --- | --- |
| `circuit_cx(ptr, c, t)` | `circuit_multi_control(ptr, "X", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_cy(ptr, c, t)` | `circuit_multi_control(ptr, "Y", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_cz(ptr, c, t)` | `circuit_multi_control(ptr, "Z", {c}, 1, {t}, 1, NULL, 0)` |
| `circuit_ccx(ptr, c1, c2, t)` | `circuit_multi_control(ptr, "X", {c1, c2}, 2, {t}, 1, NULL, 0)` |
| `circuit_crx(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RX", {c}, 1, {t}, 1, {θ}, 1)` |
| `circuit_cry(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RY", {c}, 1, {t}, 1, {θ}, 1)` |
| `circuit_crz(ptr, c, t, θ)` | `circuit_multi_control(ptr, "RZ", {c}, 1, {t}, 1, {θ}, 1)` |

需要更多控制位或对任意标准门添加控制时，使用 `circuit_multi_control`；单控制/双控制的基础受控门直接调用[标准门](5_gate_standard.md)函数即可。

---

## 相关页面

- [标准门](5_gate_standard.md)：基础门集合与各门参数个数。
- [参数](3_parameter.md)：`CParameter` 的创建与表达式语法。
