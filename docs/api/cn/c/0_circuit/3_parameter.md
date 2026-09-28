# Parameter（符号参数）

`CParameter*` 是符号参数表达式的不透明句柄：既可以是自由符号（如 `theta`）或数值常量，也可以是算术组合表达式（如 `2 * theta + pi / 2`）。本页覆盖表达式的解析、构造、算术与数学运算、符号与状态查询、化简替换与求导，以及线路侧的参数驻留、存储形式（`CCircuitParam`）、解析值（`CParameterValue`）与符号表查询；按绑定生成新线路的 `circuit_assign_params` 见 [Circuit](1_circuit.md)。错误码与内存约定见 [Overview](../0_overview.md)。

---

## 表达式解析与求值

### param_parse(expr)

解析符号参数表达式并返回新分配的句柄。表达式支持自由符号名（包括 `θ` 等 Unicode 符号）、数值常量、算术运算（`+`、`-`、`*`、`/`、一元负号）；常量 `pi`/`π` 与 `e` 在求值时自动可用，显式绑定可覆盖。

- `expr` (`const char*`)：NUL 结尾的表达式字符串。

返回：新分配的 `CParameter*`，用 `param_free` 释放；`expr` 为 NULL、非 UTF-8 或无法解析为合法表达式时返回 `NULL`。

### param_free(ptr)

释放参数句柄。允许传 `NULL`。

- `ptr` (`CParameter*`)：参数句柄。

### param_evaluate(ptr, bindings)

在给定绑定下把表达式求值为数值。

- `ptr` (`const CParameter*`)：参数句柄。
- `bindings` (`const char*`)：绑定串，格式为 `"name:value,name2:value2"`；传 NULL 或格式非法（值非有限浮点数）时按不提供绑定处理。

返回：求值结果；`ptr` 为 NULL、表达式仍含未绑定符号或求值失败（含结果为 NaN）时返回 `0.0`。

```c
CParameter *expr = param_parse("2 * theta + pi / 2");
double v = param_evaluate(expr, "theta:0.5");   /* v == 1.0 + pi/2 */
param_free(expr);
```

---

## 表达式构造与算术

### param_symbol(name)

从符号名构造自由符号参数。

- `name` (`const char*`)：NUL 结尾的符号名，支持 `θ` 等 Unicode 符号。

返回：新分配的 `CParameter*`，用 `param_free` 释放；`name` 为 NULL、非 UTF-8 或非法符号名时返回 `NULL`。

### param_pi() / param_e()

构造内置常量 π 与 e。二者是常量表达式（`param_is_constant` 返回 1），无需绑定即可求值。

返回：新分配的 `CParameter*`。

### param_from_double(value)

从数值构造常量参数。

- `value` (`double`)：常量值。

返回：新分配的 `CParameter*`；`value` 非 finite 时返回 `NULL`。

### param_add(a, b) / param_sub(a, b) / param_mul(a, b) / param_div(a, b)

两个表达式的加、减、乘、除，返回组合后的新表达式。除法构造时不检查除零，求值阶段产生非有限值。

- `a`、`b` (`const CParameter*`)：操作数句柄。

返回：新分配的 `CParameter*`；任一输入为 NULL 时返回 `NULL`。

### param_neg(a)

一元负号：返回 `-a`。

- `a` (`const CParameter*`)：操作数句柄。

返回：新分配的 `CParameter*`；`a` 为 NULL 时返回 `NULL`。

### param_pow(a, exp)

幂运算：返回 `a^exp`。

- `a`、`exp` (`const CParameter*`)：底数与指数句柄。

返回：新分配的 `CParameter*`；任一输入为 NULL 时返回 `NULL`。

```c
CParameter *theta = param_symbol("theta");
CParameter *amp = param_mul(param_from_double(0.5), param_sin(theta));
CParameter *shifted = param_add(amp, param_pi());   /* 0.5*sin(theta) + pi */
param_free(shifted);
param_free(amp);
param_free(theta);
```

---

## 数学函数

以下一元数学函数签名一致：形如 `param_xxx(const CParameter* a)`，返回新分配的 `CParameter*`（`a` 为 NULL 时返回 `NULL`），表示对该表达式施加相应数学函数后的组合表达式；构造阶段不做域检查，求值发生在 `param_evaluate` 或化简时。

### param_abs / param_sqrt / param_exp / param_ln

绝对值、平方根、自然指数、自然对数。

### param_sin / param_cos / param_tan / param_asin / param_acos / param_atan

三角函数与反三角函数。

### param_sinh / param_cosh / param_tanh

双曲函数。

### param_floor / param_ceil / param_round

向下取整、向上取整、四舍五入。

### param_log(a, base)

对指定底取对数。

- `a` (`const CParameter*`)：真数句柄。
- `base` (`const CParameter*`)：底数句柄。

返回：新分配的 `CParameter*`；任一输入为 NULL 时返回 `NULL`。

---

## 符号与状态查询

### param_symbols_len(a)

返回表达式中自由符号的数量（不含 π、e 等内置常量）。

- `a` (`const CParameter*`)：参数句柄。

返回：自由符号数量；`NULL` 时返回 0。

### param_symbols(a, out, len)

两步式读取自由符号名：先调用 `param_symbols_len` 分配缓冲，再把按名称排序的符号名写入 `out`。

- `a` (`const CParameter*`)：参数句柄。
- `out` (`char**`)：输出缓冲，可为 NULL（仅查询数量）；每个写入元素是新分配的 C 字符串，用 `cqlib_string_free` 释放。
- `len` (`uintptr_t`)：缓冲长度。

返回：自由符号总数；`-1` NULL；`-8` 缓冲不足（不写入）。

### param_as_symbol(a)

当表达式恰为单个自由符号本身时返回其名称。

返回：新分配的 C 字符串，用 `cqlib_string_free` 释放；表达式不是平凡符号或输入为 NULL 时返回 `NULL`。

### param_is_constant(a)

判断表达式是否不含自由符号（π、e 等内置常量视为已知量）。

返回：`1` 常量；`0` 含自由符号；`-1` NULL。

### param_is_exact_zero(a) / param_is_zero(a) / param_is_one(a)

- `param_is_exact_zero`：不含自由符号且求值恰为 0 时返回 1；含自由符号时返回 0。
- `param_is_zero`/`param_is_one`：不带绑定直接求值判断（是否为 0 / 是否为 1，容差 `f64::EPSILON`）；无法求值（含自由符号）时返回 0。

返回：`1`/`0`；`-1` NULL。

---

## 化简、替换与求导

### param_simplify(a)

对表达式做域安全的符号化简。

返回：新分配的 `CParameter*`；NULL 输入或化简失败时返回 `NULL`。

### param_canonicalized(a)

化简表达式；若结果不含自由符号，进一步折叠为数值节点（常量求值为非有限值时失败）。

返回：新分配的 `CParameter*`；NULL 输入或化简失败时返回 `NULL`。

### param_replace(a, symbol, replacement)

把表达式中所有 `symbol` 替换为 `replacement`，返回新表达式。

- `a` (`const CParameter*`)：参数句柄。
- `symbol` (`const char*`)：被替换的符号名。
- `replacement` (`const CParameter*`)：替换后的参数句柄。

返回：新分配的 `CParameter*`；NULL 输入或 `symbol` 非 UTF-8 时返回 `NULL`。

### param_substitute_many(a, names, values, len)

一次替换多个符号并对结果化简。`names` 与 `values` 是长度为 `len` 的平行数组。

- `a` (`const CParameter*`)：参数句柄。
- `names` (`const char* const*`)：符号名数组，元素为 NULL 视为错误。
- `values` (`const CParameter* const*`)：与 `names` 平行的新值句柄数组。
- `len` (`uintptr_t`)：数组长度；为 0 时返回 `a` 的克隆。

返回：新分配的 `CParameter*`；NULL 输入、数组元素为 NULL 或名字非 UTF-8 时返回 `NULL`。

### param_derivative(a, var)

返回对 `var` 的符号偏导数。

- `a` (`const CParameter*`)：参数句柄。
- `var` (`const char*`)：求导变量名。

返回：新分配的 `CParameter*`；NULL 输入、`var` 非 UTF-8 或求导失败时返回 `NULL`。

```c
CParameter *expr = param_parse("2 * theta");
CParameter *deriv = param_derivative(expr, "theta");   /* deriv == 2 */
double v = param_evaluate(deriv, NULL);                /* v == 2.0 */
param_free(deriv);
param_free(expr);
```

---

## 等价性判断

### param_provably_equal(a, b, tolerance)

保守判断两个表达式在 `tolerance` 内相等（直接相等或均可无绑定求值且数值差不超过容差）。

- `a`、`b` (`const CParameter*`)：待比较句柄。
- `tolerance` (`double`)：数值容差。

返回：`1` 相等；`0` 不相等或无法证明；`-1` NULL。

### param_provably_equal_modulo(a, b, modulus, tolerance)

保守判断 `a - b` 是 `modulus` 的整数倍（差值直测或模数化简后数值差不超过 `tolerance`）。

- `a`、`b` (`const CParameter*`)：待比较句柄。
- `modulus` (`const CParameter*`)：模数句柄。
- `tolerance` (`double`)：数值容差。

返回：`1` 等价；`0` 不等价或无法证明；`-1` NULL。

```c
CParameter *p = param_pi();
CParameter *three_pi = param_mul(param_from_double(3.0), param_pi());
CParameter *mod = param_mul(param_from_double(2.0), param_pi());
/* p 与 three_pi 相差 2*pi 的整数倍 */
int32_t eq = param_provably_equal_modulo(p, three_pi, mod, 1e-9);   /* eq == 1 */
param_free(mod);
param_free(three_pi);
param_free(p);
```

---

## 线路参数驻留

线路维护一张参数表：符号参数经规范化驻留后，以固定值或表下标两种存储形式出现在操作与全局相位中。驻留只注册表达式本身，不会把参数标记为被引用。

### circuit_add_parameter(ptr, param)

把符号参数克隆并驻留进线路参数表。驻留仅注册表达式及其符号，不标记为被使用。

- `ptr` (`CCircuit*`)：线路句柄。
- `param` (`const CParameter*`)：参数句柄。

返回：`0` 成功；`-1` NULL。

### circuit_parameters_len(ptr)

返回参数表中驻留的参数数量。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：驻留参数数量；`NULL` 时返回 0。

### circuit_parameters(ptr, out, len)

两步式读取参数表：先调用 `circuit_parameters_len` 分配缓冲，再把克隆的参数句柄写入 `out`。

- `ptr` (`const CCircuit*`)：线路句柄。
- `out` (`CParameter**`)：输出缓冲，可为 NULL（仅查询数量）；每个写入元素是新分配的 `CParameter*`，逐个用 `param_free` 释放。
- `len` (`uintptr_t`)：缓冲长度。

返回：驻留参数总数。

---

## 存储形式与解析值

操作与全局相位中的参数以存储形式 `CCircuitParam` 表示；读取时既可还原为完整符号表达式，也可解析为固定数值或符号形式。

### CCircuitParam

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `tag` | `uint8_t` | `CIRCUIT_PARAM_TAG_FIXED`（0，固定数值）或 `CIRCUIT_PARAM_TAG_INDEX`（1，参数表下标）。 |
| `index` | `uint32_t` | `tag` 为 `INDEX` 时的参数表下标。 |
| `value` | `double` | `tag` 为 `FIXED` 时的固定数值。 |

### CParameterValue

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `tag` | `uint8_t` | `PARAMETER_VALUE_TAG_FIXED`（0，固定数值）或 `PARAMETER_VALUE_TAG_PARAM`（1，符号参数）。 |
| `value` | `double` | `tag` 为 `FIXED` 时的固定数值。 |
| `param` | `CParameter*` | `tag` 为 `PARAM` 时新分配的符号参数句柄，用 `param_free` 释放；固定情形为 NULL。 |

### circuit_map_param(ptr, param, out)

把符号参数规范化并驻留进线路参数表，写出其存储形式（固定数值或表下标）。

- `ptr` (`CCircuit*`)：线路句柄。
- `param` (`const CParameter*`)：参数句柄。
- `out` (`CCircuitParam*`)：写出的存储形式。

返回：`0` 成功；`-1` NULL；`-3` 求值失败。

### circuit_resolve_parameter(ptr, param)

把存储形式还原为完整的符号参数表达式。

- `ptr` (`const CCircuit*`)：线路句柄。
- `param` (`const CCircuitParam*`)：存储形式。

返回：新分配的 `CParameter*`，用 `param_free` 释放；NULL 输入、非法 tag 或解析失败时返回 `NULL`。

### circuit_parameter_value(ptr, param, out)

解析存储形式的当前取值，不引入新的驻留。

- `ptr` (`const CCircuit*`)：线路句柄。
- `param` (`const CCircuitParam*`)：存储形式。
- `out` (`CParameterValue*`)：写出的解析值；符号情形的 `param` 字段用 `param_free` 释放。

返回：`0` 成功；`-1` NULL；`-3` 表下标无效；`-8` tag 无效。

```c
CCircuit *qc = circuit_new(1);
CParameter *theta = param_parse("theta");
circuit_rx_param(qc, 0, theta);

CCircuitParam storage;
CParameterValue value;
if (circuit_map_param(qc, theta, &storage) == 0 &&
    circuit_parameter_value(qc, &storage, &value) == 0) {
    if (value.tag == PARAMETER_VALUE_TAG_PARAM) {
        param_free(value.param);   /* 符号情形：释放句柄 */
    }
}
param_free(theta);
circuit_free(qc);
```

---

## 符号表查询

线路区分两个符号集合：注册表（可能包含已驻留但不再被操作引用的符号）与可执行 IR 实际引用的符号（全局相位与操作参数，含嵌套控制流体）。

### circuit_symbols_len(ptr)

返回注册表中的符号名数量；注册表可能包含已驻留但不再被操作引用的符号。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：符号数量；`NULL` 时返回 0。

### circuit_symbols(ptr, out, len)

两步式读取注册表符号名：每个写入元素是新分配的 C 字符串，用 `cqlib_string_free` 释放。

- `ptr` (`const CCircuit*`)：线路句柄。
- `out` (`char**`)：输出缓冲，可为 NULL（仅查询数量）。
- `len` (`uintptr_t`)：缓冲长度。

返回：注册表符号总数。

### circuit_used_symbols_len(ptr)

返回可执行 IR 实际引用的符号数量。

- `ptr` (`const CCircuit*`)：线路句柄。

返回：符号数量；`NULL` 时返回 0。

### circuit_used_symbols(ptr, out, len)

两步式读取 IR 引用的符号名：每个写入元素是新分配的 C 字符串，用 `cqlib_string_free` 释放。

- `ptr` (`const CCircuit*`)：线路句柄。
- `out` (`char**`)：输出缓冲，可为 NULL（仅查询数量）。
- `len` (`uintptr_t`)：缓冲长度。

返回：IR 引用的符号总数。

### circuit_uses_symbol(ptr, name)

判断符号是否被可执行 IR 引用（全局相位与操作参数，含嵌套控制流体）。

- `ptr` (`const CCircuit*`)：线路句柄。
- `name` (`const char*`)：符号名。

返回：`1` 被引用；`0` 未被引用；`-1` NULL；`-4` 符号名非 UTF-8。

```c
uintptr_t n = circuit_used_symbols_len(qc);
char **names = malloc(n * sizeof(char *));
circuit_used_symbols(qc, names, n);
for (uintptr_t i = 0; i < n; ++i) {
    if (circuit_uses_symbol(qc, names[i]) == 1) {
        /* names[i] 被 IR 引用 */
    }
    cqlib_string_free(names[i]);
}
free(names);
```
