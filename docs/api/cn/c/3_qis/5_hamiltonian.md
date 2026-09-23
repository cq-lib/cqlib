# Hamiltonian

`CHamiltonian` 表示厄米可观测量 `H = Σ c_i P_i`（Pauli 串的复系数线性组合）。配合 [Statevector](1_statevector.md#概率与测量) / [DensityMatrix](2_density_matrix.md) 的 `expectation` 接口计算 `⟨H⟩`，也可作为 [误差缓解](../6_error_mitigation/1_error_mitigation.md) 的观测对象。

---

## 函数

### hamiltonian_new(num_qubits)

创建作用在 n 比特上的空哈密顿量（0 项）。

返回：

- `CHamiltonian *`：堆分配对象，需 `hamiltonian_free` 释放；失败返回 NULL。

### hamiltonian_from_pauli(pauli)

由单个 Pauli 串构造（系数 1.0）。

**所有权**：本函数**接管并释放** `pauli`，调用返回后不得再使用（更不得二次释放）该对象。

返回：

- `CHamiltonian *`；失败返回 NULL。

### hamiltonian_add_term(ptr, pauli, coeff_re, coeff_im)

追加复系数项 `coeff * pauli`，其中 `coeff = coeff_re + coeff_im * i`。

**所有权**：Pauli 串被**克隆**，`pauli` 的所有权保留在调用方，之后仍需自行 `pauli_string_free`。

参数：

- `pauli` (`const CPauliString *`)：Pauli 串。
- `coeff_re`, `coeff_im` (`double`)：系数实部与虚部。

返回：

- `int32_t`；成功 `0`，`pauli` 比特数与哈密顿量不一致返回 `-8`。

### hamiltonian_simplify(ptr)

**就地**化简：合并相同 Pauli 串的项，并清除近零系数项。

返回：

- `int32_t`；成功 `0`。

### hamiltonian_num_qubits(ptr) / hamiltonian_num_terms(ptr)

分别返回比特数 / 项数；NULL 句柄返回 `0`。

### hamiltonian_free(ptr)

释放对象。允许传 NULL。

---

## 示例

### 构造 H = 0.5·Z₀ + 0.5·Z₁ + 0.25·X₀X₁

```c
CPauliString *z0 = pauli_string_parse("ZI");   // 首字符为最高位
CPauliString *z1 = pauli_string_parse("IZ");
CPauliString *xx = pauli_string_parse("XX");

CHamiltonian *h = hamiltonian_new(2);
hamiltonian_add_term(h, z0, 0.5, 0.0);
hamiltonian_add_term(h, z1, 0.5, 0.0);
hamiltonian_add_term(h, xx, 0.25, 0.0);
hamiltonian_simplify(h);

printf("terms=%zu\n", (size_t)hamiltonian_num_terms(h));   // 3

/* <H> = <psi|H|psi> */
CStatevector *sv = statevector_from_circuit(qc);
double energy;
statevector_expectation(sv, h, &energy);
printf("<H> = %f\n", energy);

statevector_free(sv);

hamiltonian_free(h);   // add_term 克隆了 Pauli 串，所有权仍在调用方
pauli_string_free(z0);
pauli_string_free(z1);
pauli_string_free(xx);
```

### from_pauli 的所有权转移

```c
CPauliString *z = pauli_string_parse("Z");
CHamiltonian *h = hamiltonian_from_pauli(z);   // z 已被接管并释放
/* 此处不得再使用 z */

hamiltonian_free(h);
```

同一 Pauli 串重复添加后 `simplify` 会合并系数（如两次 `0.5·Z` 合并为 `1.0·Z`），可用于累加能量项。
