# Parameter

`CParameter` 表示符号或数值参数表达式，用于门角、全局相位等可调参数。参数体系包含表达式解析、求值、释放与整线路绑定四组自由函数。

C 侧的参数对象是**表达式**而不只是符号名：`param_parse("theta/2 + 0.1")` 得到的对象即一棵表达式树，可以直接求值，也可以作为门参数追加进线路。

---

## 表达式解析

### param_parse(expr)

解析符号表达式字符串。

参数：

- `expr` (`const char *`)：表达式文本，如 `"theta/2 + 0.1"`。

支持的语法：

- 符号名（如 `theta`、`phi`）；
- 数值字面量（如 `0.25`、`1e-3`）；
- 运算符 `+`、`-`、`*`、`/` 与括号；
- 常用数学函数。

返回：

- `CParameter *`：堆分配参数对象，需用 `param_free` 释放；解析失败（空串、非法语法等）返回 NULL。

### param_free(ptr)

释放参数对象。允许传 NULL。

```c
CParameter *theta = param_parse("theta/2");
// ... 用作 circuit_*_param 系列参数 ...
param_free(theta);
```

---

## 求值

### param_evaluate(ptr, bindings)

按绑定表对表达式求值。

参数：

- `ptr` (`const CParameter *`)：参数对象。
- `bindings` (`const char *`)：绑定表，格式 `"name:value,name2:value2"`，逗号分隔、冒号赋值。

返回：

- `double`：表达式数值。以下情况返回 **NaN**：
  - `bindings` 为 NULL 或格式非法；
  - 表达式中存在绑定表未覆盖的符号。

数值型参数无需绑定，直接返回其值。

```c
CParameter *expr = param_parse("theta/2 + 0.1");
double v  = param_evaluate(expr, "theta:1.0");    // 0.6
double nv = param_evaluate(expr, "phi:1.0");      // NaN：theta 未绑定
double iv = param_evaluate(expr, NULL);           // NaN
param_free(expr);
```

---

## 整线路参数绑定

### circuit_assign_params(circuit, bindings)

将线路中的符号参数绑定为具体数值，返回绑定后的**新线路**，不修改原线路。适用于将一条参数化线路作为模板，在不同参数值下重复生成具体线路。

参数：

- `circuit` (`const CCircuit *`)：参数化线路。
- `bindings` (`const char *`)：绑定表，格式同 `param_evaluate`。

返回：

- `CCircuit *`：新线路，需用 `circuit_free` 释放；失败返回 NULL。

绑定语义：

- `bindings` 中列出的符号被替换为对应数值；
- 未列出的符号**保留**在返回的新线路中（部分绑定）；
- 全部符号绑定后，`circuit_num_parameters` 返回 `0`。

```c
CCircuit *qc = circuit_new(2);
CParameter *theta = param_parse("theta");
CParameter *phi = param_parse("phi");
circuit_rx_param(qc, 0, theta);
circuit_ry_param(qc, 1, phi);
param_free(phi);
param_free(theta);

/* 部分绑定：只绑 theta，phi 保留 */
CCircuit *half = circuit_assign_params(qc, "theta:1.5707963");
printf("params left = %zu\n", (size_t)circuit_num_parameters(half));   // 1

/* 全量绑定 */
CCircuit *full = circuit_assign_params(half, "phi:0.5");
printf("params left = %zu\n", (size_t)circuit_num_parameters(full));   // 0

circuit_free(full);
circuit_free(half);
circuit_free(qc);
```

---

## 参数化线路复用示例

下面的示例展示典型的"模板线路 + 参数绑定"用法：

```c
/* 模板：RX(theta) - CX - RZ(theta/2) */
CCircuit *layer = circuit_new(2);
CParameter *theta = param_parse("theta");
circuit_rx_param(layer, 0, theta);
circuit_cx(layer, 0, 1);
circuit_rz_param(layer, 1, theta);
param_free(theta);

/* 不同参数值下生成具体线路 */
CCircuit *a = circuit_assign_params(layer, "theta:0.1");
CCircuit *b = circuit_assign_params(layer, "theta:0.2");

circuit_free(b);
circuit_free(a);
circuit_free(layer);
```
