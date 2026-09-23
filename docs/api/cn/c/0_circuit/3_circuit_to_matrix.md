# Circuit To Matrix

将小规模纯量子门线路导出为稠密酉矩阵。API 采用统一的两步式输出：先 `circuit_to_matrix_len` 获取维度，再 `circuit_to_matrix` 填充缓冲区。

矩阵元素以**行主序、实虚交错**的 `double` 写出，布局为 `[re₀, im₀, re₁, im₁, ...]`；第 `(r, c)` 个复元素位于 `out[2 * (r * d + c)]`（实部）与 `out[2 * (r * d + c) + 1]`（虚部），其中 `d = 2^N` 为矩阵边长。

---

## 函数

### circuit_to_matrix_len(circuit, qubits_order, order_len)

返回矩阵的复元素总数 `d * d`（`d = 2^N`）。

参数：

- `circuit` (`const CCircuit *`)：纯量子门线路。
- `qubits_order` (`const uintptr_t *`)：比特顺序数组，NULL 使用默认顺序。
- `order_len` (`uintptr_t`)：顺序数组长度，`qubits_order` 为 NULL 时传 0。

返回：

- `uintptr_t`：矩阵复元素总数；错误（NULL 句柄、线路含非酉操作等）返回 `0`。

### circuit_to_matrix(circuit, qubits_order, order_len, out, buffer_len)

将矩阵写入 `out`。

参数：

- `circuit`、`qubits_order`、`order_len`：同上。
- `out` (`double *`)：输出缓冲区，需至少容纳 `2 * circuit_to_matrix_len(...)` 个 `double`。
- `buffer_len` (`uintptr_t`)：`out` 的容量（按 `double` 计）。

返回：

- `uintptr_t`：实际写入的复元素数；错误返回 `0`。

错误场景：

- 线路为 NULL 或含 `measure`/`reset` 等非酉操作（线路不对应酉矩阵）；
- 缓冲区容量不足；
- `qubits_order` 非法（长度与线路比特数不符或重复引用同一比特）。

---

## qubits_order 语义

`qubits_order` 控制矩阵张量因子的排列顺序：

- 默认（NULL）：比特 `0` 为最低位张量因子，矩阵按 `q_{N-1} ⊗ ... ⊗ q_1 ⊗ q_0` 展开；
- 传入排列（如 `[1, 0]`）：交换高低位顺序，得到比特重排后的矩阵。

对角线顺序影响仅体现在矩阵行列排列，酉性与谱不变。

---

## 示例

### Bell 线路的酉矩阵

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

uintptr_t d = circuit_to_matrix_len(qc, NULL, 0);        // d * d = 16，d = 4
double *mat = malloc(sizeof(double) * 2 * d);
if (circuit_to_matrix(qc, NULL, 0, mat, 2 * d) == 0) {
    /* (0,0) 元素 = 1/√2 + 0i */
    printf("m[0][0] = %f%+fi\n", mat[0], mat[1]);
}

free(mat);
circuit_free(qc);
```

### 比特重排

```c
uintptr_t order[2] = {1, 0};
uintptr_t d = circuit_to_matrix_len(qc, order, 2);
double *mat = malloc(sizeof(double) * 2 * d);
circuit_to_matrix(qc, order, 2, mat, 2 * d);
/* mat 现在是比特交换顺序下的酉矩阵 */

free(mat);
```

更大的线路矩阵规模随比特数指数增长（N 比特需 `2 * 4^N` 个 `double`），该接口适合 N 较小的精确核对场景。
