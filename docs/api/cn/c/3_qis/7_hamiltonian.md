# 哈密顿量（C）

`CHamiltonian` 是可观测量（Hamiltonian）的不透明句柄：若干带复系数的 Pauli 串项之和，支持化简、缩放、稠密矩阵输出与时间演化线路构造。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量

Trotter-Suzuki 分解模式标签（`hamiltonian_to_trotter_circuit` 与 `hamiltonian_to_evolution_circuit` 的 `mode` 参数）：

| 常量 | 值 | 模式 |
| --- | --- | --- |
| `TROTTER_MODE_FIRST_ORDER` | 0 | 一阶 Lie-Trotter。 |
| `TROTTER_MODE_SECOND_ORDER` | 1 | 二阶 Strang 分裂。 |

---

## 构造与释放

### hamiltonian_new(num_qubits)

创建作用在 `num_qubits` 比特上的空 Hamiltonian（无任何 Pauli 项）。

参数：

- `num_qubits` (`uintptr_t`)：量子比特数。

返回：新建的 `CHamiltonian*`。

### hamiltonian_free(ptr)

释放 Hamiltonian 句柄，允许传 NULL。

### hamiltonian_from_pauli(pauli)

由单个 Pauli 串构造 Hamiltonian（系数为 1.0）。

**所有权约定**：该接口接管入参 `CPauliString*` 的所有权并释放它；调用之后不得再使用或释放原句柄。

参数：

- `pauli` (`CPauliString*`)：Pauli 串句柄，所有权被接管。

返回：成功返回新建的 `CHamiltonian*`；NULL 输入返回 NULL。

### hamiltonian_add_term(ptr, pauli, coeff_re, coeff_im)

添加一个带复系数的 Pauli 项。

**所有权约定**：Pauli 串被克隆，入参句柄不被消耗，调用方继续负责释放。

参数：

- `ptr` (`CHamiltonian*`)：Hamiltonian 句柄。
- `pauli` (`const CPauliString*`)：Pauli 串句柄（克隆使用）。
- `coeff_re` (`double`)：系数实部。
- `coeff_im` (`double`)：系数虚部。

返回：0 成功；-1 空指针；-8 Pauli 串比特数与 Hamiltonian 不一致。

### hamiltonian_simplify(ptr)

原地化简：合并 Pauli 串相同的项并移除系数近似为零的项。

返回：0 成功；-1 空指针。

### hamiltonian_scale(ptr, factor_re, factor_im)

原地把全部系数乘以一个复因子。

参数：

- `factor_re` (`double`)：因子实部。
- `factor_im` (`double`)：因子虚部。

返回：0 成功；-1 空指针；-8 因子非有限值（NaN 或无穷）。

---

## 查询

### hamiltonian_num_qubits(ptr)

返回比特数；NULL 返回 0。

### hamiltonian_num_terms(ptr)

返回 Pauli 项数；NULL 返回 0。

### hamiltonian_all_terms_commute(ptr)

判断全部 Pauli 项是否两两对易。全部对易时，时间演化可精确分解为逐项 Pauli 旋转。

返回：1 全部对易；0 存在不对易项；-1 空指针。

---

## 稠密矩阵输出

### hamiltonian_to_matrix_len(ptr) / hamiltonian_to_matrix(ptr, buffer, len)

两步式读取稠密矩阵：`*_len` 返回边长 `2^N`（NULL 或边长无法表示时返回 0），填充接口把 `2^N × 2^N` 矩阵按行主序拷入 `buffer`（`Complex64` 值）。`len` 必须等于边长的平方。

返回（填充接口）：0 成功；-1 空指针；-7 元素数或字节数无法表示，或分配失败；-8 `len` 与矩阵元素数不符。失败不改写缓冲区。

可表示的边长不保证完整矩阵可以分配；调用方应在计算 `dim * dim` 前检查溢出。

---

## 期望值与方差

### CProbMeasurement

`hamiltonian_expectation_probs` 使用的测量描述符：一个被测基及其观测到的结果分布。

```c
typedef struct CProbMeasurement {
  const struct CPauliString *basis;  /* 借用，不消耗 */
  const char *const *states;         /* len 个计算基位串 */
  const double *probs;               /* len 个概率，与 states 按下标配对 */
  uintptr_t len;                     /* states 与 probs 的条目数 */
} CProbMeasurement;
```

- `basis` (`const CPauliString*`)：作为测量基的 Pauli 串；借用，不消耗，由调用方负责释放。
- `states` (`const char* const*`)：`len` 个计算基位串组成的数组，每个量子比特一个 `'0'`/`'1'` 字符，大端序（量子比特 0 是最后一个字符）。
- `probs` (`const double*`)：`len` 个观测概率组成的数组，与 `states` 按下标配对。
- `len` (`uintptr_t`)：`states` 与 `probs` 的条目数。

### hamiltonian_expectation_probs(ptr, measurements, measurements_len, out)

由测量结果概率计算 Hamiltonian 的期望值。`measurements` 每个条目对应一个被测基；Hamiltonian 的每一项在第一个与之兼容的被测基中求值——兼容指被测基在该项的所有非恒等因子上与之一致。结果写入 `out`。分布只需覆盖观测到的结果，未列出的结果贡献为零。

```c
int32_t hamiltonian_expectation_probs(const struct CHamiltonian *ptr,
                                      const struct CProbMeasurement *measurements,
                                      uintptr_t measurements_len,
                                      double *out);
```

参数：

- `ptr` (`const CHamiltonian*`)：Hamiltonian 句柄。
- `measurements` (`const CProbMeasurement*`)：`measurements_len` 个测量描述符组成的数组。
- `measurements_len` (`uintptr_t`)：`measurements` 的条目数；`0` 表示未提供任何被测基。
- `out` (`double*`)：接收期望值。

错误码：

| 值 | 场景 |
| --- | --- |
| `0` | 成功。 |
| `-1` | `ptr` 或 `out` 为 NULL；`measurements` 非零条目数时为 NULL；或条目内 `basis` 为 NULL，或 `len` 非零时 `states`/`probs`/位串为 NULL。 |
| `-4` | `states` 中的位串不是有效 UTF-8。 |
| `-7` | 某一项没有兼容的测量基、位串格式非法或长度与 Hamiltonian 比特数不符，或其他核心错误。 |

### hamiltonian_variance_statevector(ptr, sv, out)

计算 Hamiltonian 在态矢量下的方差 `Var(H) = <H^2> - <H>^2`，写入 `out`。

```c
int32_t hamiltonian_variance_statevector(const struct CHamiltonian *ptr,
                                         const struct CStatevector *sv,
                                         double *out);
```

参数：

- `ptr` (`const CHamiltonian*`)：Hamiltonian 句柄。
- `sv` (`const CStatevector*`)：输入态矢量，比特数必须与 Hamiltonian 一致。
- `out` (`double*`)：接收方差。

错误码：

| 值 | 场景 |
| --- | --- |
| `0` | 成功。 |
| `-1` | `ptr`、`sv` 或 `out` 为 NULL。 |
| `-7` | Hamiltonian 含非厄米项，或其他模拟错误。 |
| `-8` | 态矢量比特数与 Hamiltonian 不一致，或其他非法参数。 |

---

## 时间演化线路

### hamiltonian_to_trotter_circuit(ptr, time, steps, mode)

把 Hamiltonian 转换为 Trotter 化时间演化线路，逼近 `U(t) = exp(-i H t)`。

参数：

- `ptr` (`const CHamiltonian*`)：Hamiltonian 句柄。
- `time` (`double`)：演化时间，必须为有限值。
- `steps` (`uintptr_t`)：Trotter 步数。
- `mode` (`uint32_t`)：`TROTTER_MODE_FIRST_ORDER` 或 `TROTTER_MODE_SECOND_ORDER`。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；空指针、非有限时间、非法模式或构造失败时返回 NULL。

### hamiltonian_to_evolution_circuit(ptr, time, steps, mode)

把 Hamiltonian 转换为时间演化线路：全部项相互对易时使用精确的单趟分解，否则回退到 Trotter 近似（`mode`、`steps`）。

参数：

- `ptr` (`const CHamiltonian*`)：Hamiltonian 句柄。
- `time` (`double`)：演化时间，必须为有限值。
- `steps` (`uintptr_t`)：Trotter 步数。
- `mode` (`uint32_t`)：`TROTTER_MODE_FIRST_ORDER` 或 `TROTTER_MODE_SECOND_ORDER`。

返回：成功返回新建的 `CCircuit*`（用 `circuit_free` 释放）；空指针、非有限时间、非法模式或构造失败时返回 NULL。

---

## 示例

```c
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 构造 H = 0.5 * ZI + 0.5 * IZ（"ZI"：比特1=Z、比特0=I） */
    struct CHamiltonian *ham = hamiltonian_new(2);
    struct CPauliString *zi = pauli_string_parse("ZI");
    struct CPauliString *iz = pauli_string_parse("IZ");
    hamiltonian_add_term(ham, zi, 0.5, 0.0);  /* 克隆，句柄保留 */
    hamiltonian_add_term(ham, iz, 0.5, 0.0);
    pauli_string_free(zi);
    pauli_string_free(iz);

    /* 2. 查询与化简 */
    uintptr_t nq = hamiltonian_num_qubits(ham);       /* 2 */
    uintptr_t nt = hamiltonian_num_terms(ham);        /* 2 */
    int32_t commute = hamiltonian_all_terms_commute(ham);  /* 1（两单项对易） */
    printf("qubits=%" PRIuPTR ", terms=%" PRIuPTR ", commute=%" PRId32 "\n",
           nq, nt, commute);
    hamiltonian_simplify(ham);

    /* 3. 两步式读取 4x4 稠密矩阵 */
    uintptr_t dim = hamiltonian_to_matrix_len(ham);   /* 4 */
    Complex64 *matrix = malloc(dim * dim * sizeof(Complex64));
    hamiltonian_to_matrix(ham, matrix, dim * dim);
    free(matrix);

    /* 4. 时间演化线路（对易时精确分解） */
    struct CCircuit *evo =
        hamiltonian_to_evolution_circuit(ham, 0.1, 1, TROTTER_MODE_FIRST_ORDER);
    if (evo != NULL) {
        circuit_free(evo);
    }

    /* 5. 由单个 Pauli 串构造（接管所有权） */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *from_pauli = hamiltonian_from_pauli(zz);  /* zz 已被接管 */

    /* 6. 由测量结果概率计算期望值，并计算态矢量方差
       （H = Z⊗Z 作用于贝尔态，P(00) = P(11) = 0.5） */
    struct CStatevector *bell = statevector_new(2);
    statevector_apply_h(bell, 0);
    statevector_apply_cx(bell, 0, 1);

    struct CPauliString *basis = pauli_string_parse("ZZ");
    const char *states[4] = {"00", "01", "10", "11"};
    double probs[4] = {0.5, 0.0, 0.0, 0.5};
    struct CProbMeasurement measurement;
    measurement.basis = basis;      /* 借用，不消耗 */
    measurement.states = states;
    measurement.probs = probs;
    measurement.len = 4;

    double expectation = 0.0;
    hamiltonian_expectation_probs(from_pauli, &measurement, 1, &expectation);  /* 1.0 */

    double variance = 0.0;
    hamiltonian_variance_statevector(from_pauli, bell, &variance);  /* 0.0（Z⊗Z 的本征态） */

    pauli_string_free(basis);
    statevector_free(bell);

    hamiltonian_free(from_pauli);
    hamiltonian_free(ham);
    return 0;
}
```

哈密顿量作为可观测量用于误差缓解流程（见 [零噪声外推](../6_error_mitigation/1_zne.md)、[虚拟蒸馏](../6_error_mitigation/2_virtual_distillation.md)），其期望值可直接由态矢量模拟器计算（见 [态矢量](1_statevector.md)）。


## 带校验的稀疏 Pauli 代数

`CHamiltonian` 同时表示一般非厄米稀疏算符。以下新接口采用与[费米子绑定](10_fermion.md)相同的错误码和输出约定：成功为 0，通过 `**out` 返回独立句柄，失败不改写输出或输入。输入 Pauli 句柄借用并复制，不消耗。保留旧 `from_pauli` 的所有权转移、原地 `scale` 和近零截断 `simplify` 行为。

```c
typedef struct CHamiltonianTerm {
    const CPauliString *pauli;
    Complex64 coefficient;
} CHamiltonianTerm;

int32_t hamiltonian_identity(size_t num_qubits, CHamiltonian **out);

int32_t hamiltonian_from_terms(size_t num_qubits,
                               const CHamiltonianTerm *terms,
                               size_t len,
                               CHamiltonian **out);

int32_t hamiltonian_copy(const CHamiltonian *ptr, CHamiltonian **out);

int32_t hamiltonian_normalized(const CHamiltonian *ptr, CHamiltonian **out);

int32_t hamiltonian_normalized_hermitian(const CHamiltonian *ptr, CHamiltonian **out);

int32_t hamiltonian_add(const CHamiltonian *left,
                        const CHamiltonian *right,
                        CHamiltonian **out);

int32_t hamiltonian_sub(const CHamiltonian *left,
                        const CHamiltonian *right,
                        CHamiltonian **out);

int32_t hamiltonian_mul(const CHamiltonian *left,
                        const CHamiltonian *right,
                        CHamiltonian **out);

int32_t hamiltonian_scaled(const CHamiltonian *ptr,
                           Complex64 factor,
                           CHamiltonian **out);

int32_t hamiltonian_adjoint(const CHamiltonian *ptr, CHamiltonian **out);

int32_t hamiltonian_tensor(const CHamiltonian *left,
                           const CHamiltonian *right,
                           CHamiltonian **out);

int32_t hamiltonian_remap_qubits(const CHamiltonian *ptr,
                                 const size_t *mapping,
                                 size_t len,
                                 size_t target_width,
                                 CHamiltonian **out);

int32_t hamiltonian_chop(CHamiltonian *ptr, double atol);

int32_t hamiltonian_equiv(const CHamiltonian *left,
                          const CHamiltonian *right,
                          double atol,
                          uint8_t *out);

int32_t hamiltonian_is_hermitian(const CHamiltonian *ptr, double atol, uint8_t *out);

int32_t hamiltonian_get_term(const CHamiltonian *ptr,
                             size_t index,
                             CPauliString **out_pauli,
                             Complex64 *out_coefficient);
```

| 接口 | 行为 |
| --- | --- |
| `identity/from_terms` | identity 支持零量子位标量一；空 terms 保留声明宽度。批量构造校验每项宽度和系数、吸收 Pauli 相位、合并重复项，仅删除精确零。 |
| `copy/get_term` | 独立存储副本。get_term 返回项的 Pauli 副本及系数，不自动规范化；Pauli 用 pauli_string_free 释放，零宽度也合法。索引错误为 -2。 |
| `normalized` | 返回经过校验和精确合并的副本，无近零截断。 |
| `normalized_hermitian` | 规范化后将绝对值 ≤1e-10 的虚部投影到零，更大的虚部返回 -8；保留非零实系数。 |
| `add/sub/mul/scaled/adjoint` | 调用 core checked 方法；校验输入与结果系数，非有限值或溢出返回 -8。二元运算要求宽度相同，mul 右侧先作用。 |
| `tensor` | 左侧 ⊗ 右侧，右侧占低位 qubit；宽度相加，溢出返回 -8。 |
| `remap_qubits` | mapping[source]=destination，长度等于输入宽度，目标唯一且小于 target_width；空闲 qubit 填 identity。不是费米模式置换。 |
| `chop` | 原地规范化后删除模长 ≤atol 的系数；失败不修改对象。 |
| `equiv/is_hermitian` | 写入 0/1；atol 有限且非负，0 为精确比较。equiv 要求宽度相同；厄米性检查等价于与伴随比较，不投影系数。 |

对象结果用 `hamiltonian_free` 释放，输出槽须为空且有效。必需指针为空返回 -1；参数、宽度、系数或容差错误返回 -8；identity、tensor、remap 无法表示的 Pauli 位向量宽度返回 -7。零求和可以保留无需物化的较大声明宽度。

atol 是系数容差；规范化后 `is_hermitian` 使用 `2*abs(imag) <= atol`，与 `normalized_hermitian` 的投影阈值是两个独立契约。稀疏代数不构造稠密矩阵，也不隐式调用旧 simplify。
