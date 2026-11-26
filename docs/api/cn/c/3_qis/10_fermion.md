# 费米子代数与 Jordan–Wigner（C）

`CFermionicOp` 是有序升降算符乘积的稀疏复数线性组合。模式从零编号；乘积 `A B` 表示 B 先作用。相同有序乘积自动合并，仅删除精确零；不会隐式正规序或截断小系数。零算符和恒等算符都保留声明的模式数，支持零模式。

## ABI 与数据

所有返回 `int32_t` 的新接口以 `0` 表示成功。对象通过 `**out` 返回，标量通过输出指针返回；失败不改变输出或原对象。对象结果独立拥有，使用 `fermionic_op_free` 释放；输入借用并复制。调用方传入空输出槽，并提供有效、对齐、存活的指针；输出存储不得相互重叠或与句柄内部存储重叠。同一句柄被修改时，不可并发读取。数组长度为零允许 NULL；非空数组必须指向足够存储。

错误码：`-1` 必需指针为空，`-2` 模式或项索引越界，`-8` 非法标签、模式数不匹配、非有限系数或算术溢出、非法容差、容量不足或无法表示的数组长度，`-7` 不支持的 Pauli 表示规模。判断通过 `uint8_t *out` 写入 0/1，成功返回值始终为 0。

```c
typedef struct CFermionicOp CFermionicOp;
#define FERMIONIC_CREATE 0
#define FERMIONIC_ANNIHILATE 1
typedef struct CFermionicLadder {
    size_t mode;
    uint8_t kind;
} CFermionicLadder;
typedef struct CFermionicTerm {
    const CFermionicLadder *ladders;
    size_t len;
    Complex64 coefficient;
} CFermionicTerm;
```

## 构造

```c
int32_t fermionic_op_new(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_identity(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_from_term(size_t num_modes,
                               const CFermionicLadder *term,
                               size_t len,
                               Complex64 coefficient,
                               CFermionicOp **out);

int32_t fermionic_op_from_terms(size_t num_modes,
                                const CFermionicTerm *terms,
                                size_t len,
                                CFermionicOp **out);

int32_t fermionic_op_creation(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_annihilation(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_number(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_total_number(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_excitation_generator(size_t num_modes,
                                          const size_t *sources,
                                          size_t sources_len,
                                          const size_t *targets,
                                          size_t targets_len,
                                          CFermionicOp **out);
```

- `new` 创建零，`identity` 创建一；`from_term` 的空乘积为 identity，`from_terms` 的空求和为 zero。批量输入及嵌套升降算符数组只在调用期间借用。即使系数为零，也校验标签和模式。
- `creation/annihilation/number` 分别构造 a†、a、a†a；`total_number` 为所有模式 number 之和。
- 激发生成元 `G=i(T−T†)`，其中 T 的 target 创建算符按输入顺序排列，source 湮灭算符按逆序排列。两侧必须非空、等长、各自无重复且互不重叠。奇排列改变符号，无排序、阶乘或自旋轨道约定。

## 生命周期与读取

```c
void fermionic_op_free(CFermionicOp *ptr);

int32_t fermionic_op_copy(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_num_modes(const CFermionicOp *ptr, size_t *out);

int32_t fermionic_op_num_terms(const CFermionicOp *ptr, size_t *out);

int32_t fermionic_op_is_zero(const CFermionicOp *ptr, uint8_t *out);

int32_t fermionic_op_equals(const CFermionicOp *left,
                            const CFermionicOp *right,
                            uint8_t *out);

int32_t fermionic_op_term_len(const CFermionicOp *ptr, size_t index, size_t *out);

int32_t fermionic_op_get_term(const CFermionicOp *ptr,
                              size_t index,
                              CFermionicLadder *buffer,
                              size_t capacity,
                              Complex64 *out_coefficient);
```

`free(NULL)` 合法；`copy` 返回独立副本。`is_zero` 检查存储是否为空，`equals` 比较存储结构和模式数（模式数不同返回 false）。代数比较使用 `equiv`。

先调用 `term_len` 获取项长度，再分配 buffer 并调用 `get_term`；capacity 可以大于所需长度，额外元素保持原值。空乘积允许 NULL buffer。按 core 的确定性结构顺序读取；修改算符后应重新查询长度和索引。输出数组是副本，没有需要释放的内部指针。

## 代数

```c
int32_t fermionic_op_add_term(CFermionicOp *ptr,
                              const CFermionicLadder *term,
                              size_t len,
                              Complex64 coefficient);

int32_t fermionic_op_add(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_sub(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_mul(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_scaled(const CFermionicOp *ptr,
                            Complex64 factor,
                            CFermionicOp **out);

int32_t fermionic_op_adjoint(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_normal_order(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_chop(CFermionicOp *ptr, double atol);

int32_t fermionic_op_equiv(const CFermionicOp *left,
                           const CFermionicOp *right,
                           double atol,
                           uint8_t *out);

int32_t fermionic_op_is_hermitian(const CFermionicOp *ptr, double atol, uint8_t *out);
```

`add_term/chop` 原地修改，其余对象运算返回副本。`mul` 不自动正规序；`adjoint` 共轭系数、逆转乘积并交换升降种类。`normal_order` 将创建算符放在湮灭算符前，每组按模式升序，包含反对易收缩项。

`chop` 删除模长 ≤ atol 的系数。`equiv` 对差正规序后比较系数，要求模式数相同；`is_hermitian` 与伴随比较。atol 必须有限且非负，精确比较显式传入 0。它是系数容差，不是矩阵算符范数误差界。

## Jordan–Wigner

```c
int32_t jordan_wigner_map_operator(const CFermionicOp *ptr, CHamiltonian **out);

int32_t jordan_wigner_num_qubits(size_t num_modes, size_t *out);

int32_t jordan_wigner_map_occupation(const uint8_t *occupation,
                                     size_t len,
                                     uint8_t *out_bits,
                                     size_t capacity);
```

无状态映射无需句柄。mode p 对应 qubit p，Z 链覆盖所有更低模式：creation = Z链(X−iY)/2，annihilation = Z链(X+iY)/2。qubit 0 为最低有效位，也是显示的 Pauli 字符串最右字符。

`map_operator` 返回独立 `CHamiltonian`，使用 `hamiltonian_free` 释放；非厄米输入合法，厄米性要求由期望值和演化等下游接口检查。映射不构造稠密矩阵，但 k 个升降算符的乘积在合并前可能展开为 2^k 项。

`map_occupation` 仅接受 0/1，占据长度决定模式数；输出字节 bits[p] 对应 qubit p。capacity 至少为 len，输入输出允许重叠；空输入为零模式真空。

## 示例

```c
#include "cqlib_c.h"

int main(void) {
    const size_t sources[] = {0}, targets[] = {1};
    CFermionicOp *g = NULL;
    CHamiltonian *h = NULL;
    if (fermionic_op_excitation_generator(2, sources, 1, targets, 1, &g) != 0) {
        return 1;
    }
    if (jordan_wigner_map_operator(g, &h) != 0) {
        fermionic_op_free(g);
        return 1;
    }
    CCircuit *u = hamiltonian_to_evolution_circuit(
        h, 0.37, 1, TROTTER_MODE_SECOND_ORDER);
    const int result = u == NULL ? 1 : 0;
    circuit_free(u);
    hamiltonian_free(h);
    fermionic_op_free(g);
    return result;
}
```
