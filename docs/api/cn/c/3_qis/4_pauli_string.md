# PauliString

`CPauliString` 表示 Pauli 算符串（I/X/Y/Z 序列）。可用作稳定子期望计算的观测量，也可经 [Hamiltonian](5_hamiltonian.md) 加权组合为一般可观测量。

---

## 函数

### pauli_string_new(num_qubits)

创建 n 比特的全 `I` 串。

参数：

- `num_qubits` (`uintptr_t`)：比特数。

返回：

- `CPauliString *`：堆分配对象，需 `pauli_string_free` 释放；`num_qubits = 0` 或失败返回 NULL。

### pauli_string_parse(source)

解析 Pauli 串字符串（如 `"XYZ"`、`"ZZI"`）。

参数：

- `source` (`const char *`)：仅含 `I/X/Y/Z` 的字符串。

返回：

- `CPauliString *`；解析失败返回 NULL。

**方向约定**：从左到右依次对应**高位到低位**比特（首字符为最高位 qubit），与 `measure_all` 输出的大端比特串方向一致。例如 `parse("XYZ")` 后 `get_pauli(0)` 为 `PAULI_Z`。

### pauli_string_set_pauli(ptr, idx, pauli)

将下标 `idx` 处的算符设为 `pauli`。

参数：

- `idx` (`uintptr_t`)：比特下标（`0` 为最低位）。
- `pauli` (`uint8_t`)：常量 `PAULI_I` / `PAULI_X` / `PAULI_Y` / `PAULI_Z`（0–3）。

返回：

- `int32_t`；成功 `0`，下标越界或常量非法返回负错误码。

### pauli_string_get_pauli(ptr, idx)

读取下标 `idx` 处的算符。

返回：

- `uint8_t`：`PAULI_I/X/Y/Z` 之一；错误返回 `0xFF`。

### pauli_string_num_qubits(ptr)

返回比特数；NULL 句柄返回 `0`。

### pauli_string_to_string(ptr)

格式化为带符号位的字符串（如 `"+XYZ"`）。

返回：

- `char *`：堆分配字符串，需 `cqlib_string_free` 释放；失败返回 NULL。

### pauli_string_matrix_len(ptr) / pauli_string_matrix(ptr, buffer, len)

两步式导出 `2^N × 2^N` 稠密矩阵：

- `pauli_string_matrix_len` 返回矩阵**边长** `2^N`，缓冲区需 `2^N × 2^N` 个 `Complex64`；
- `pauli_string_matrix` 按行主序 `Complex64` 填充；成功 `0`，缓冲区不足返回负错误码。

### pauli_string_free(ptr)

释放对象。允许传 NULL。

---

## 示例

### 构造与读写

```c
/* parse 与逐位 set 两种构造方式等价 */
CPauliString *p = pauli_string_parse("XYZ");
printf("%u\n", pauli_string_get_pauli(p, 0));   // PAULI_Z（首字符对应最高位）

pauli_string_set_pauli(p, 2, PAULI_I);          // 修改最高位为 I
char *text = pauli_string_to_string(p);         // "+IZ"
printf("%s\n", text);
cqlib_string_free(text);

pauli_string_free(p);
```

### 稠密矩阵导出

```c
CPauliString *x = pauli_string_parse("X");

uintptr_t side = pauli_string_matrix_len(x);        // 2
Complex64 *mat = malloc(sizeof(Complex64) * side * side);
pauli_string_matrix(x, mat, side * side);

/* mat 行主序 = [[0, 1], [1, 0]] */
printf("%f%+fi %f%+fi\n", mat[0].re, mat[0].im, mat[1].re, mat[1].im);

free(mat);
pauli_string_free(x);
```

与稳定子期望配合见 [StabilizerState § 稳定子读取](3_stabilizer_state.md#稳定子读取)。
