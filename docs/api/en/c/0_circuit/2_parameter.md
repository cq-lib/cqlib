# Parameter

`CParameter` represents a symbolic or numeric parameter expression, used for adjustable parameters such as gate angles and the global phase. The parameter system contains four groups of free functions: expression parsing, evaluation, release and whole-circuit binding.

A parameter object on the C side is an **expression** rather than just a symbol name: the object obtained from `param_parse("theta/2 + 0.1")` is an expression tree, which can be evaluated directly or appended to a circuit as a gate parameter.

---

## Expression parsing

### param_parse(expr)

Parse a symbolic expression string.

Parameters:

- `expr` (`const char *`): the expression text, such as `"theta/2 + 0.1"`.

Supported syntax:

- symbol names (such as `theta` and `phi`);
- numeric literals (such as `0.25` and `1e-3`);
- the operators `+`, `-`, `*` and `/`, and parentheses;
- common mathematical functions.

Returns:

- `CParameter *`: a heap-allocated parameter object, to be released with `param_free`; returns NULL when parsing fails (an empty string, invalid syntax and so on).

### param_free(ptr)

Release the parameter object. NULL is allowed.

```c
CParameter *theta = param_parse("theta/2");
// ... 用作 circuit_*_param 系列参数 ...
param_free(theta);
```

---

## Evaluation

### param_evaluate(ptr, bindings)

Evaluate the expression against a binding table.

Parameters:

- `ptr` (`const CParameter *`): the parameter object.
- `bindings` (`const char *`): the binding table, in the format `"name:value,name2:value2"`, with comma separators and colon assignment.

Returns:

- `double`: the value of the expression. **NaN** is returned in the following cases:
  - `bindings` is NULL or its format is invalid;
  - the expression contains a symbol that the binding table does not cover.

A numeric parameter needs no binding and returns its value directly.

```c
CParameter *expr = param_parse("theta/2 + 0.1");
double v  = param_evaluate(expr, "theta:1.0");    // 0.6
double nv = param_evaluate(expr, "phi:1.0");      // NaN：theta 未绑定
double iv = param_evaluate(expr, NULL);           // NaN
param_free(expr);
```

---

## Whole-circuit parameter binding

### circuit_assign_params(circuit, bindings)

Bind the symbolic parameters in a circuit to concrete values and return a **new circuit** after binding, without modifying the original circuit. It is suited to using a parameterized circuit as a template and generating concrete circuits repeatedly under different parameter values.

Parameters:

- `circuit` (`const CCircuit *`): the parameterized circuit.
- `bindings` (`const char *`): the binding table, in the same format as `param_evaluate`.

Returns:

- `CCircuit *`: the new circuit, to be released with `circuit_free`; returns NULL on failure.

Binding semantics:

- symbols listed in `bindings` are replaced with the corresponding values;
- symbols that are not listed are **kept** in the returned new circuit (partial binding);
- after all symbols are bound, `circuit_num_parameters` returns `0`.

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

## Parameterized circuit reuse example

The example below shows the typical "template circuit + parameter binding" usage:

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
