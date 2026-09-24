# Ansatz

本页介绍 C API 中的变分线路模板：TwoLocal、BasicEntanglerLayers、QAOAAnsatz、feature map 家族（ZFeatureMap、ZZFeatureMap、IQPFeatureMap、PauliFeatureMap）、StronglyEntanglingLayers 和 Pauli 演化 ansatz，以及 facade 构造函数和纠缠拓扑辅助函数。每个模板都按"创建句柄 → 配置 → 校验 → 构建线路"的流程使用，生成的线路以 `{prefix}` 前缀命名符号参数。错误码与内存管理约定见 [Overview](../0_overview.md)。

---

## 标签常量

纠缠拓扑标签（用于 `two_local_entanglement`、feature map、facade 构造函数和拓扑辅助函数）：

| 拓扑标签 | 值 | 含义 |
| --- | --- | --- |
| `ENTANGLEMENT_LINEAR` | 0 | 线性最近邻链。 |
| `ENTANGLEMENT_CIRCULAR` | 1 | 线性链加首尾环绕边。 |
| `ENTANGLEMENT_FULL` | 2 | 全连接（任意量子比特对）。 |
| `ENTANGLEMENT_CUSTOM` | 3 | 自定义显式连接对。 |

演化策略标签（用于 `qaoa_ansatz_evolution_strategy`）：

| 策略标签 | 值 | 含义 |
| --- | --- | --- |
| `EVOLUTION_STRATEGY_EXACT` | 0 | 逐项精确 Pauli 演化。 |
| `EVOLUTION_STRATEGY_AUTO` | 1 | 自动选择精确或 Trotter 分解（使用 `steps`）。 |
| `EVOLUTION_STRATEGY_TROTTER` | 2 | 显式 Trotter 乘积公式（使用 `mode` / `steps` / `seed`）。 |

Trotter 模式标签（`EVOLUTION_STRATEGY_TROTTER` 接受的 `mode` 取值，以及 `pauli_evolution_ansatz_evolution_info` 报告的 `trotter_mode` 取值）：

| 模式标签 | 值 | 含义 |
| --- | --- | --- |
| `TROTTER_FIRST_ORDER` | 0 | 一阶乘积公式。 |
| `TROTTER_SECOND_ORDER` | 1 | 二阶乘积公式。 |
| `TROTTER_RANDOMIZED` | 2 | 随机化一阶公式（`seed` 为随机种子）。 |
| `TROTTER_MODE_NONE` | -1 | 未生成 Trotter 分解（精确单次路径）；只出现在 `CPauliEvolutionInfo.trotter_mode` 中，不能作为 `mode` 输入。 |

---

## TwoLocal

TwoLocal 由交替出现的单量子比特旋转层和多量子比特纠缠层组成，适用于 VQE、量子机器学习和通用变分线路构造。`two_local_new` 的默认配置为：`reps = 3`、旋转门 `[RY]`、纠缠门 `CX`、线性拓扑、保留最终旋转层。

### two_local_new(num_qubits)

创建一个 TwoLocal 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回新分配的 `CTwoLocal*`。

### two_local_free(ptr)

释放 TwoLocal 句柄。

- `ptr` (`CTwoLocal*`)：待释放句柄，允许传 NULL。

无返回值。

### two_local_reps(ptr, reps)

设置重复层数。

- `ptr` (`CTwoLocal*`)：模板句柄。
- `reps` (`uintptr_t`)：重复层数。

返回 0 表示成功；-1 表示输入为 NULL。

### two_local_rotation_gates(ptr, names, len)

按名称设置每层的单量子比特参数旋转门（必须是 RX、RY 或 RZ，构建时校验）。

- `ptr` (`CTwoLocal*`)：模板句柄。
- `names` (`const char* const*`)：门名称数组。
- `len` (`uintptr_t`)：数组长度。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法；-8 表示门名未知。

### two_local_entanglement_gate(ptr, name)

按名称设置纠缠层使用的双量子比特门（CX、CY 或 CZ，构建时校验）。

- `ptr` (`CTwoLocal*`)：模板句柄。
- `name` (`const char*`)：门名称。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法；-8 表示门名未知。

### two_local_entanglement(ptr, topology, pairs, pairs_len)

设置纠缠拓扑。拓扑为 `ENTANGLEMENT_CUSTOM` 时，`pairs` 是长度为 `pairs_len / 2` 的控制/目标量子比特 id 对的扁平数组。

- `ptr` (`CTwoLocal*`)：模板句柄。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回 0 表示成功；-1 表示输入为 NULL（含 `pairs` 为 NULL 而 `pairs_len > 0`）；-8 表示拓扑未知或 `pairs_len` 为奇数。

### two_local_skip_final_rotation_layer(ptr, skip)

设置是否跳过最终旋转层。

- `ptr` (`CTwoLocal*`)：模板句柄。
- `skip` (`bool`)：true 表示跳过。

返回 0 表示成功；-1 表示输入为 NULL。

### two_local_validate(ptr)

校验模板配置。

- `ptr` (`const CTwoLocal*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### two_local_num_parameters(ptr)

返回模板生成线路所需的独立参数数量。

- `ptr` (`const CTwoLocal*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### two_local_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CTwoLocal*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### two_local_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CTwoLocal*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### TwoLocal 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CTwoLocal *ansatz = two_local_new(3);

    two_local_reps(ansatz, 2);
    const char *rotations[] = {"RY", "RZ"};
    two_local_rotation_gates(ansatz, rotations, 2);
    two_local_entanglement_gate(ansatz, "CX");
    two_local_entanglement(ansatz, ENTANGLEMENT_LINEAR, NULL, 0);

    if (two_local_validate(ansatz) != 0) {
        two_local_free(ansatz);
        return 1;
    }

    /* 参数名形如 "theta_0", "theta_1", ... */
    struct CCircuit *circuit = two_local_build_circuit(ansatz, "theta");
    if (circuit == NULL) {
        two_local_free(ansatz);
        return 1;
    }
    printf("parameters: %zu, qubits: %zu\n",
           two_local_num_parameters(ansatz),
           two_local_num_qubits(ansatz));

    circuit_free(circuit);
    two_local_free(ansatz);
    return 0;
}
```

---

## BasicEntanglerLayers

BasicEntanglerLayers 由单量子比特旋转层和固定模式的纠缠层组成，适合构造结构简单、参数数量可控的训练线路。`basic_entangler_layers_new` 的默认配置为：`reps = 1`、旋转门 `RX`、纠缠门 `CX`。

### basic_entangler_layers_new(num_qubits)

创建一个 BasicEntanglerLayers 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回新分配的 `CBasicEntanglerLayers*`。

### basic_entangler_layers_free(ptr)

释放 BasicEntanglerLayers 句柄。

- `ptr` (`CBasicEntanglerLayers*`)：待释放句柄，允许传 NULL。

无返回值。

### basic_entangler_layers_reps(ptr, reps)

设置层数。

- `ptr` (`CBasicEntanglerLayers*`)：模板句柄。
- `reps` (`uintptr_t`)：层数。

返回 0 表示成功；-1 表示输入为 NULL。

### basic_entangler_layers_rotation_gate(ptr, name)

按名称设置单量子比特旋转门（RX、RY、RZ 或 Phase）。

- `ptr` (`CBasicEntanglerLayers*`)：模板句柄。
- `name` (`const char*`)：门名称。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法；-8 表示门名未知。

### basic_entangler_layers_entanglement_gate(ptr, name)

按名称设置纠缠门（CX、CY 或 CZ）。

- `ptr` (`CBasicEntanglerLayers*`)：模板句柄。
- `name` (`const char*`)：门名称。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法；-8 表示门名未知。

### basic_entangler_layers_num_parameters(ptr)

返回模板生成线路所需的独立参数数量。

- `ptr` (`const CBasicEntanglerLayers*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### basic_entangler_layers_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CBasicEntanglerLayers*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### basic_entangler_layers_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CBasicEntanglerLayers*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

---

## QAOAAnsatz

QAOAAnsatz 用于构造量子近似优化算法（QAOA）的参数化线路，交替应用 cost Hamiltonian 与 mixer Hamiltonian 的时间演化：

```text
U(β, γ) = ∏_l exp(-i β_l H_M) exp(-i γ_l H_C)
```

`qaoa_ansatz_new` 从 cost Hamiltonian 推断量子比特数量，mixer 默认为标准 X mixer，`reps = 1`。每层包含一个 `γ_l` 和一个 `β_l` 参数，因此参数总数为 `2 * reps`，参数命名为 `"{prefix}_gamma_{layer}"` / `"{prefix}_beta_{layer}"`。

### qaoa_ansatz_new(cost)

从 cost Hamiltonian 创建 QAOA 模板（Hamiltonian 会被克隆）。

- `cost` (`const CHamiltonian*`)：cost Hamiltonian 句柄。

返回新分配的 `CQAOAAnsatz*`；输入为 NULL 或 cost 算子非法时返回 NULL。

### qaoa_ansatz_free(ptr)

释放 QAOA 句柄。

- `ptr` (`CQAOAAnsatz*`)：待释放句柄，允许传 NULL。

无返回值。

### qaoa_ansatz_reps(ptr, reps)

设置交替层数（QAOA 深度 p）。

- `ptr` (`CQAOAAnsatz*`)：模板句柄。
- `reps` (`uintptr_t`)：层数。

返回 0 表示成功；-1 表示输入为 NULL。

### qaoa_ansatz_mixer(ptr, mixer)

覆盖 mixer Hamiltonian（会被克隆；量子比特数量必须与 cost 算子一致）。

- `ptr` (`CQAOAAnsatz*`)：模板句柄。
- `mixer` (`const CHamiltonian*`)：mixer Hamiltonian 句柄。

返回 0 表示成功；-1 表示输入为 NULL；-3 表示量子比特数量不匹配或 mixer 非法。

### qaoa_ansatz_initial_state(ptr, circuit)

设置前置初态线路（会被克隆；量子比特数量必须一致）。

- `ptr` (`CQAOAAnsatz*`)：模板句柄。
- `circuit` (`const CCircuit*`)：初态线路句柄。

返回 0 表示成功；-1 表示输入为 NULL；-3 表示量子比特数量不匹配。

### qaoa_ansatz_evolution_strategy(ptr, strategy, mode, steps, seed)

设置 Hamiltonian 演化策略。`EVOLUTION_STRATEGY_EXACT` 忽略其余参数；`EVOLUTION_STRATEGY_AUTO` 使用 `steps`；`EVOLUTION_STRATEGY_TROTTER` 使用 `mode`（`TROTTER_FIRST_ORDER`、`TROTTER_SECOND_ORDER` 或 `TROTTER_RANDOMIZED`）、`steps` 和 `seed`。

- `ptr` (`CQAOAAnsatz*`)：模板句柄。
- `strategy` (`int32_t`)：`EVOLUTION_STRATEGY_*` 策略标签。
- `mode` (`int32_t`)：Trotter 模式标签，仅 TROTTER 策略使用。
- `steps` (`uintptr_t`)：Trotter 步数，EXACT 策略忽略。
- `seed` (`uint64_t`)：随机种子，仅 `TROTTER_RANDOMIZED` 使用。

返回 0 表示成功；-1 表示输入为 NULL；-8 表示标签未知。

### qaoa_ansatz_validate(ptr)

校验模板配置。

- `ptr` (`const CQAOAAnsatz*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### qaoa_ansatz_num_parameters(ptr)

返回参数数量（每层 2 个）。

- `ptr` (`const CQAOAAnsatz*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### qaoa_ansatz_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CQAOAAnsatz*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### qaoa_ansatz_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_gamma_{layer}"` / `"{prefix}_beta_{layer}"`。

- `ptr` (`const CQAOAAnsatz*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### QAOA 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* cost Hamiltonian：H_C = Z0 * Z1 */
    struct CPauliString *zz = pauli_string_parse("ZZ");
    struct CHamiltonian *cost = hamiltonian_from_pauli(zz);  /* 接管 zz 句柄 */

    struct CQAOAAnsatz *ansatz = qaoa_ansatz_new(cost);
    qaoa_ansatz_reps(ansatz, 2);
    qaoa_ansatz_evolution_strategy(ansatz, EVOLUTION_STRATEGY_AUTO,
                                   TROTTER_FIRST_ORDER, 1, 0);

    if (qaoa_ansatz_validate(ansatz) != 0) {
        qaoa_ansatz_free(ansatz);
        hamiltonian_free(cost);
        return 1;
    }

    /* 参数名形如 "q_gamma_0", "q_beta_0", "q_gamma_1", "q_beta_1" */
    struct CCircuit *circuit = qaoa_ansatz_build_circuit(ansatz, "q");
    if (circuit == NULL) {
        qaoa_ansatz_free(ansatz);
        hamiltonian_free(cost);
        return 1;
    }
    printf("parameters: %zu, qubits: %zu\n",
           qaoa_ansatz_num_parameters(ansatz),
           qaoa_ansatz_num_qubits(ansatz));

    circuit_free(circuit);
    qaoa_ansatz_free(ansatz);
    hamiltonian_free(cost);
    return 0;
}
```

---

## Feature maps

Feature map（特征映射）将经典特征向量 `x_0, x_1, ...` 编码到量子态中，是量子机器学习和量子核方法的标准输入层。四个模板（ZFeatureMap、ZZFeatureMap、IQPFeatureMap、PauliFeatureMap）遵循相同的约定：每个重复层以 Hadamard 层开始，符号参数命名为 `"{prefix}_{index}"`，每个输入特征对应一个参数，且相同参数对象在各重复层之间复用，因此 `*_num_parameters` 返回 `num_qubits`（`reps = 0` 时返回 0，构建出空线路）。注意 `circuit_num_parameters` 统计的是不同的角度表达式对象：内嵌在角度表达式中的常量（如 `pi`）不算独立参数（见下方 ZZFeatureMap 示例）。

### ZFeatureMap

ZFeatureMap 是不含纠缠的一阶映射：每个重复层先对所有量子比特应用 Hadamard 层，再对每个量子比特应用独立的 `RZ(2 * x_i)` 旋转。`z_feature_map_new` 默认 `reps = 2`。

### z_feature_map_new(num_qubits)

创建一个 ZFeatureMap 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。

返回新分配的 `CZFeatureMap*`。

### z_feature_map_free(ptr)

释放 ZFeatureMap 句柄。

- `ptr` (`CZFeatureMap*`)：待释放句柄，允许传 NULL。

无返回值。

### z_feature_map_reps(ptr, reps)

设置重复层数。

- `ptr` (`CZFeatureMap*`)：模板句柄。
- `reps` (`uintptr_t`)：重复层数。

返回 0 表示成功；-1 表示输入为 NULL。

### z_feature_map_validate(ptr)

校验模板配置（至少需要 1 个量子比特）。

- `ptr` (`const CZFeatureMap*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### z_feature_map_num_parameters(ptr)

返回输入特征数量（每个量子比特一个参数；`reps = 0` 时为 0）。

- `ptr` (`const CZFeatureMap*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### z_feature_map_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CZFeatureMap*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### z_feature_map_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CZFeatureMap*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### ZZFeatureMap

ZZFeatureMap 在此基础上引入二阶 ZZ 相互作用：每个重复层先应用 Hadamard 层和每个量子比特上的 `RZ(2 * x_i)`，然后对纠缠拓扑中的每一对 `(i, j)` 应用 Pauli 演化 `e^{-i * 2 * (pi - x_i) * (pi - x_j) * Z_i Z_j}`。默认配置：`reps = 2`、全连接拓扑。自定义拓扑由 setter 保存、由校验检查：越界下标、自环和重复（无向）边都会被拒绝。

### zz_feature_map_new(num_qubits)

创建一个 ZZFeatureMap 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。

返回新分配的 `CZZFeatureMap*`。

### zz_feature_map_free(ptr)

释放 ZZFeatureMap 句柄。

- `ptr` (`CZZFeatureMap*`)：待释放句柄，允许传 NULL。

无返回值。

### zz_feature_map_reps(ptr, reps)

设置重复层数。

- `ptr` (`CZZFeatureMap*`)：模板句柄。
- `reps` (`uintptr_t`)：重复层数。

返回 0 表示成功；-1 表示输入为 NULL。

### zz_feature_map_entanglement(ptr, topology, pairs, pairs_len)

设置 ZZ 相互作用的纠缠拓扑。拓扑为 `ENTANGLEMENT_CUSTOM` 时，`pairs` 是长度为 `pairs_len / 2` 的控制/目标量子比特 id 对的扁平数组。

- `ptr` (`CZZFeatureMap*`)：模板句柄。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回 0 表示成功；-1 表示输入为 NULL（含 `pairs` 为 NULL 而 `pairs_len > 0`）；-8 表示拓扑未知或 `pairs_len` 为奇数。

### zz_feature_map_validate(ptr)

校验模板配置（至少 1 个量子比特；自定义拓扑要求下标不越界、无自环、无重复边）。

- `ptr` (`const CZZFeatureMap*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### zz_feature_map_num_parameters(ptr)

返回输入特征数量（每个量子比特一个参数；`reps = 0` 时为 0）。

- `ptr` (`const CZZFeatureMap*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### zz_feature_map_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CZZFeatureMap*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### zz_feature_map_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CZZFeatureMap*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### ZZFeatureMap 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CZZFeatureMap *map = zz_feature_map_new(2);

    /* 返回 2 个输入特征 x_0、x_1。 */
    printf("features: %zu\n", zz_feature_map_num_parameters(map));

    /* circuit_num_parameters 统计不同的角度表达式：2 个 RZ 角度
       加上共享的 ZZ 表达式 4 * (pi - x_i) * (pi - x_j)；表达式内的
       pi 常量不算独立参数。 */
    struct CCircuit *circuit = zz_feature_map_build_circuit(map, "x");
    if (circuit == NULL) {
        zz_feature_map_free(map);
        return 1;
    }
    printf("circuit parameters: %zu\n", circuit_num_parameters(circuit));

    circuit_free(circuit);
    zz_feature_map_free(map);
    return 0;
}
```

### IQPFeatureMap

IQPFeatureMap 是 IQP 风格的对角编码，与 ZZFeatureMap 使用相同的线路结构——Hadamard 层、一阶 Z 旋转和按纠缠拓扑展开的二阶 ZZ 相互作用——但提供独立句柄。默认配置：`reps = 2`、全连接拓扑。

### iqp_feature_map_new(num_qubits)

创建一个 IQPFeatureMap 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。

返回新分配的 `CIQPFeatureMap*`。

### iqp_feature_map_free(ptr)

释放 IQPFeatureMap 句柄。

- `ptr` (`CIQPFeatureMap*`)：待释放句柄，允许传 NULL。

无返回值。

### iqp_feature_map_reps(ptr, reps)

设置重复层数。

- `ptr` (`CIQPFeatureMap*`)：模板句柄。
- `reps` (`uintptr_t`)：重复层数。

返回 0 表示成功；-1 表示输入为 NULL。

### iqp_feature_map_entanglement(ptr, topology, pairs, pairs_len)

设置双量子比特对角相互作用的纠缠拓扑。拓扑为 `ENTANGLEMENT_CUSTOM` 时，`pairs` 是长度为 `pairs_len / 2` 的控制/目标量子比特 id 对的扁平数组。

- `ptr` (`CIQPFeatureMap*`)：模板句柄。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回 0 表示成功；-1 表示输入为 NULL（含 `pairs` 为 NULL 而 `pairs_len > 0`）；-8 表示拓扑未知或 `pairs_len` 为奇数。

### iqp_feature_map_validate(ptr)

校验模板配置（至少 1 个量子比特；自定义拓扑要求下标不越界、无自环、无重复边）。

- `ptr` (`const CIQPFeatureMap*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### iqp_feature_map_num_parameters(ptr)

返回输入特征数量（每个量子比特一个参数；`reps = 0` 时为 0）。

- `ptr` (`const CIQPFeatureMap*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### iqp_feature_map_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CIQPFeatureMap*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### iqp_feature_map_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CIQPFeatureMap*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### PauliFeatureMap

PauliFeatureMap 把 Z 和 ZZ 映射推广到任意 Pauli 模板。每个重复层先应用 Hadamard 层，然后对每个配置的 Pauli 模板（支持集大小为 `k`）以及从纠缠拓扑中取出的每个 `k` 元组应用 Pauli 演化：

- `k = 1`：角度为 `2 * x_i`，实现 `e^{-i * x_i * P}`；
- `k >= 2`：角度为 `4 * prod_j (pi - x_j)`，实现 `e^{-i * 2 * prod_j (pi - x_j) * P}`。

支持集为空的模板只贡献全局相位，会被跳过。默认配置：`reps = 2`、Pauli 模板 `["Z", "ZZ"]`、全连接拓扑、参数前缀 `"x"`。`pauli_feature_map_build_circuit` 收到空前缀时回退到已配置的参数前缀。

### pauli_feature_map_new(num_qubits)

创建一个 PauliFeatureMap 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。

返回新分配的 `CPauliFeatureMap*`。

### pauli_feature_map_free(ptr)

释放 PauliFeatureMap 句柄。

- `ptr` (`CPauliFeatureMap*`)：待释放句柄，允许传 NULL。

无返回值。

### pauli_feature_map_reps(ptr, reps)

设置重复层数。

- `ptr` (`CPauliFeatureMap*`)：模板句柄。
- `reps` (`uintptr_t`)：重复层数。

返回 0 表示成功；-1 表示输入为 NULL。

### pauli_feature_map_paulis(ptr, texts, labels, len)

设置 Pauli 演化模板。`texts` 和 `labels` 是长度均为 `len` 的平行 NUL 结尾 C 字符串数组；每个 text 按 PauliString 解析（例如 `"Z"`、`"ZZ"`、`"XY"`），每个 label 为模板命名。长度超过量子比特数量的模板会被保存，但校验会失败。调用失败时保留原有配置。

- `ptr` (`CPauliFeatureMap*`)：模板句柄。
- `texts` (`const char* const*`)：Pauli 字符串文本数组。
- `labels` (`const char* const*`)：模板标签数组。
- `len` (`uintptr_t`)：数组长度。

返回 0 表示成功；-1 表示输入为 NULL（句柄、`len > 0` 时的数组、或单个元素为 NULL）；-4 表示 UTF-8 非法；-8 表示 Pauli 字符串解析失败。

### pauli_feature_map_entanglement(ptr, topology, pairs, pairs_len)

设置多量子比特相互作用的纠缠拓扑。拓扑为 `ENTANGLEMENT_CUSTOM` 时，`pairs` 是长度为 `pairs_len / 2` 的控制/目标量子比特 id 对的扁平数组。

- `ptr` (`CPauliFeatureMap*`)：模板句柄。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回 0 表示成功；-1 表示输入为 NULL（含 `pairs` 为 NULL 而 `pairs_len > 0`）；-8 表示拓扑未知或 `pairs_len` 为奇数。

### pauli_feature_map_parameter_prefix(ptr, prefix)

设置回退参数前缀，`pauli_feature_map_build_circuit` 收到空前缀时使用。

- `ptr` (`CPauliFeatureMap*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法。

### pauli_feature_map_validate(ptr)

校验模板配置（至少 1 个量子比特；Pauli 模板长度不超过量子比特数量；自定义拓扑要求下标不越界、无自环、无重复边）。

- `ptr` (`const CPauliFeatureMap*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### pauli_feature_map_num_parameters(ptr)

返回输入特征数量（每个量子比特一个参数；`reps = 0` 时为 0）。

- `ptr` (`const CPauliFeatureMap*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### pauli_feature_map_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CPauliFeatureMap*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### pauli_feature_map_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。空前缀回退到已配置的参数前缀。

- `ptr` (`const CPauliFeatureMap*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀（空字符串使用已配置的回退前缀）。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### PauliFeatureMap 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CPauliFeatureMap *map = pauli_feature_map_new(2);

    const char *texts[] = {"Z", "ZZ"};
    const char *labels[] = {"Z", "ZZ"};
    pauli_feature_map_paulis(map, texts, labels, 2);
    pauli_feature_map_parameter_prefix(map, "p");

    if (pauli_feature_map_validate(map) != 0) {
        pauli_feature_map_free(map);
        return 1;
    }

    /* 空前缀回退到 "p"：参数名形如 "p_0", "p_1"。 */
    struct CCircuit *circuit = pauli_feature_map_build_circuit(map, "");
    if (circuit == NULL) {
        pauli_feature_map_free(map);
        return 1;
    }
    printf("features: %zu\n", pauli_feature_map_num_parameters(map));

    circuit_free(circuit);
    pauli_feature_map_free(map);
    return 0;
}
```

---

## Facade 构造函数

facade 构造函数一次调用即可创建常用模板的预配置句柄。它们返回与对应 `*_new` 构造函数相同的句柄类型，因此结果对象可以配合所有 `two_local_*`、`zz_feature_map_*` 和 `pauli_feature_map_*` 函数使用。拓扑参数非法（标签未知、自定义连接对数量为奇数、或 `pairs` 为 NULL 而 `pairs_len > 0`）时四个函数都返回 NULL；`pauli_feature_map_build` 在 Pauli 字符串解析失败时同样返回 NULL。

### efficient_su2(num_qubits, reps, topology, pairs, pairs_len)

创建 EfficientSU2 ansatz：`[RY, RZ]` 旋转层配合 `CX` 纠缠，保留最终旋转层，参数总数为 `(reps + 1) * num_qubits * 2`。

- `num_qubits` (`uintptr_t`)：量子比特数量。
- `reps` (`uintptr_t`)：重复层数。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回新分配的 `CTwoLocal*`；拓扑参数非法时返回 NULL。

### real_amplitudes(num_qubits, reps, topology, pairs, pairs_len)

创建 RealAmplitudes ansatz：`[RY]` 旋转层配合 `CX` 纠缠，保留最终旋转层，参数总数为 `(reps + 1) * num_qubits`。

- `num_qubits` (`uintptr_t`)：量子比特数量。
- `reps` (`uintptr_t`)：重复层数。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回新分配的 `CTwoLocal*`；拓扑参数非法时返回 NULL。

### zz_feature_map_build(num_qubits, reps, topology, pairs, pairs_len)

创建指定重复层数和拓扑的 ZZFeatureMap。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。
- `reps` (`uintptr_t`)：重复层数。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回新分配的 `CZZFeatureMap*`；拓扑参数非法时返回 NULL。

### pauli_feature_map_build(num_qubits, reps, texts, labels, paulis_len, topology, pairs, pairs_len)

创建带显式 Pauli 模板的 PauliFeatureMap。`texts` 和 `labels` 是长度均为 `paulis_len` 的平行 NUL 结尾 C 字符串数组；每个 text 按 PauliString 解析。

- `num_qubits` (`uintptr_t`)：量子比特数量（即输入特征数量）。
- `reps` (`uintptr_t`)：重复层数。
- `texts` (`const char* const*`)：Pauli 字符串文本数组。
- `labels` (`const char* const*`)：模板标签数组。
- `paulis_len` (`uintptr_t`)：两个数组的长度。
- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。

返回新分配的 `CPauliFeatureMap*`；拓扑参数非法或 Pauli 字符串解析失败时返回 NULL。

### Facade 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* EfficientSU2：[RY, RZ] + CX -> (1 + 1) * 2 * 2 = 8 个参数 */
    struct CTwoLocal *su2 = efficient_su2(2, 1, ENTANGLEMENT_FULL, NULL, 0);
    printf("efficient_su2 parameters: %zu\n", two_local_num_parameters(su2));
    two_local_free(su2);

    /* RealAmplitudes：[RY] + CX -> (2 + 1) * 3 = 9 个参数 */
    struct CTwoLocal *ra = real_amplitudes(3, 2, ENTANGLEMENT_LINEAR, NULL, 0);
    printf("real_amplitudes parameters: %zu\n", two_local_num_parameters(ra));
    two_local_free(ra);

    /* 环形拓扑的 ZZ feature map */
    struct CZZFeatureMap *zz = zz_feature_map_build(3, 2, ENTANGLEMENT_CIRCULAR, NULL, 0);
    printf("zz_feature_map features: %zu\n", zz_feature_map_num_parameters(zz));
    zz_feature_map_free(zz);
    return 0;
}
```

---

## 纠缠拓扑辅助函数

这些辅助函数把 `ENTANGLEMENT_*` 拓扑标签展开为显式量子比特分组，语义与各模板一致。输出采用两步模式：先调用 `*_len` 函数获取所需的扁平长度，分配缓冲区后再调用拷贝函数。

### entanglement_generate_pairs_len(topology, pairs, pairs_len, num_qubits)

返回 `entanglement_generate_pairs` 生成的扁平长度（每对 2 个元素）；参数非法（拓扑标签未知、自定义连接对数量为奇数、或 `pairs` 为 NULL 而 `pairs_len > 0`）时返回 0。拓扑为 `ENTANGLEMENT_CUSTOM` 时原样返回自定义连接对，忽略 `num_qubits`。

- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。
- `num_qubits` (`uintptr_t`)：预定义拓扑使用的量子比特数量。

返回 `entanglement_generate_pairs` 所需的 `out` 缓冲区长度；参数非法时返回 0。

### entanglement_generate_pairs(topology, pairs, pairs_len, num_qubits, out, len)

把扁平的连接对 `[c0, t0, c1, t1, ...]` 拷贝到 `out`。预定义拓扑生成：linear 为 `(0,1), (1,2), ...`；circular 额外增加环绕边 `(n-1, 0)`；full 为按字典序排列的全连接对。

- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。
- `num_qubits` (`uintptr_t`)：预定义拓扑使用的量子比特数量。
- `out` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，即 `entanglement_generate_pairs_len` 的返回值。

返回 0 表示成功；-1 表示输入为 NULL（`len > 0` 时 `out` 为 NULL）；-8 表示 `len` 与所需长度不符或参数非法。

### entanglement_generate_k_tuples_len(topology, pairs, pairs_len, k, num_qubits)

返回 `entanglement_generate_k_tuples` 生成的扁平长度（每个元组 `k` 个元素）；参数组合非法（拓扑未知或自定义连接对非法、`k = 0`、`k > num_qubits`、或 `num_qubits = 0`）时返回 0。

- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。
- `k` (`uintptr_t`)：元组大小。
- `num_qubits` (`uintptr_t`)：量子比特数量。

返回 `entanglement_generate_k_tuples` 所需的 `out` 缓冲区长度；参数非法时返回 0。

### entanglement_generate_k_tuples(topology, pairs, pairs_len, k, num_qubits, out, len)

把扁平的 k 元组 `[t0_0, t0_1, ..., t1_0, ...]` 拷贝到 `out`。`k = 1` 时每个量子比特单独成组；`k = 2` 时返回拓扑的连接对（含自定义连接对）；`k >= 3` 时 linear 生成连续窗口 `(i, ..., i+k-1)`，circular 生成模 n 的旋转窗口，full 生成按字典序排列的 k 组合——自定义拓扑只定义连接对，此时回退到 full。

- `topology` (`int32_t`)：`ENTANGLEMENT_*` 拓扑标签。
- `pairs` (`const uint32_t*`)：自定义连接对的扁平数组（其他拓扑传 NULL）。
- `pairs_len` (`uintptr_t`)：扁平数组长度。
- `k` (`uintptr_t`)：元组大小。
- `num_qubits` (`uintptr_t`)：量子比特数量。
- `out` (`uint32_t*`)：输出缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，即 `entanglement_generate_k_tuples_len` 的返回值。

返回 0 表示成功；-1 表示输入为 NULL（`len > 0` 时 `out` 为 NULL）；-8 表示 `len` 与所需长度不符或参数非法。

### 纠缠拓扑辅助函数示例

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 两步模式：先查长度，再拷贝。 */
    uintptr_t len = entanglement_generate_pairs_len(ENTANGLEMENT_CIRCULAR, NULL, 0, 4);
    uint32_t *pairs = malloc(len * sizeof(uint32_t));

    if (entanglement_generate_pairs(ENTANGLEMENT_CIRCULAR, NULL, 0, 4, pairs, len) == 0) {
        /* 输出：0 1 1 2 2 3 3 0 */
        for (uintptr_t i = 0; i < len; i++) {
            printf("%u ", pairs[i]);
        }
        printf("\n");
    }
    free(pairs);
    return 0;
}
```

---

## StronglyEntanglingLayers

StronglyEntanglingLayers 的每一层先对每个量子比特应用一个 U 门（3 个参数），再用纠缠门把量子比特 `i` 连接到 `(i + r) mod num_qubits`，其中 `r` 是该层的 range。默认配置：`reps = 1`、纠缠门 `CX`、range 按 `1..num_qubits` 循环取值（第 `l` 层使用 `(l mod (num_qubits - 1)) + 1`）。参数命名为 `"{prefix}_{index}"` 并连续编号，总数为 `reps * num_qubits * 3`。

### strongly_entangling_layers_new(num_qubits)

创建一个 StronglyEntanglingLayers 模板句柄。

- `num_qubits` (`uintptr_t`)：量子比特数量。

返回新分配的 `CStronglyEntanglingLayers*`。

### strongly_entangling_layers_free(ptr)

释放 StronglyEntanglingLayers 句柄。

- `ptr` (`CStronglyEntanglingLayers*`)：待释放句柄，允许传 NULL。

无返回值。

### strongly_entangling_layers_reps(ptr, reps)

设置层数。

- `ptr` (`CStronglyEntanglingLayers*`)：模板句柄。
- `reps` (`uintptr_t`)：层数。

返回 0 表示成功；-1 表示输入为 NULL。

### strongly_entangling_layers_entanglement_gate(ptr, name)

按名称设置纠缠层使用的双量子比特门（CX、CY 或 CZ）。

- `ptr` (`CStronglyEntanglingLayers*`)：模板句柄。
- `name` (`const char*`)：门名称。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法；-8 表示门名未知。

### strongly_entangling_layers_ranges(ptr, ranges, len)

设置显式 range 列表，在各层之间循环复用（第 `l` 层使用 `ranges[l mod len]`）。校验时每个 range 必须落在 `1..num_qubits` 内。空列表（`len == 0`）会保存空列表，在被替换之前校验会失败。

- `ptr` (`CStronglyEntanglingLayers*`)：模板句柄。
- `ranges` (`const uintptr_t*`)：range 数组。
- `len` (`uintptr_t`)：数组长度。

返回 0 表示成功；-1 表示输入为 NULL（句柄为 NULL，或 `len > 0` 时 `ranges` 为 NULL）。

### strongly_entangling_layers_validate(ptr)

校验模板配置（至少 1 个量子比特、纠缠门合法，以及——若已设置——range 列表非空且每个 range 都在 `1..num_qubits` 内）。

- `ptr` (`const CStronglyEntanglingLayers*`)：模板句柄。

返回 0 表示配置合法；-1 表示输入为 NULL；-3 表示配置非法。

### strongly_entangling_layers_num_parameters(ptr)

返回独立参数数量（`reps * num_qubits * 3`）。

- `ptr` (`const CStronglyEntanglingLayers*`)：模板句柄。

返回参数数量；句柄为 NULL 时返回 0。

### strongly_entangling_layers_num_qubits(ptr)

返回模板作用的量子比特数量。

- `ptr` (`const CStronglyEntanglingLayers*`)：模板句柄。

返回量子比特数量；句柄为 NULL 时返回 0。

### strongly_entangling_layers_build_circuit(ptr, prefix)

根据当前配置构建参数化线路，参数命名为 `"{prefix}_{index}"`。

- `ptr` (`const CStronglyEntanglingLayers*`)：模板句柄。
- `prefix` (`const char*`)：参数名前缀。

返回新分配的 `CCircuit*`（用 `circuit_free` 释放）；输入为 NULL、UTF-8 非法或构建失败时返回 NULL。

### StronglyEntanglingLayers 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    struct CStronglyEntanglingLayers *layers = strongly_entangling_layers_new(3);

    strongly_entangling_layers_reps(layers, 2);
    uintptr_t ranges[] = {1, 2};  /* 循环复用：第 0 层 -> 1，第 1 层 -> 2 */
    strongly_entangling_layers_ranges(layers, ranges, 2);
    strongly_entangling_layers_entanglement_gate(layers, "CZ");

    if (strongly_entangling_layers_validate(layers) != 0) {
        strongly_entangling_layers_free(layers);
        return 1;
    }

    /* 2 层 * 3 量子比特 * 3 个 U 门角度 = 18 个参数，名为 "t_0", "t_1", ... */
    struct CCircuit *circuit = strongly_entangling_layers_build_circuit(layers, "t");
    if (circuit == NULL) {
        strongly_entangling_layers_free(layers);
        return 1;
    }
    printf("parameters: %zu\n", strongly_entangling_layers_num_parameters(layers));

    circuit_free(circuit);
    strongly_entangling_layers_free(layers);
    return 0;
}
```

---

## PauliEvolutionAnsatz

PauliEvolutionAnsatz 表示 Hamiltonian 下的时间演化。`pauli_evolution_ansatz_new` 会克隆输入 Hamiltonian 并做简化（合并重复项、把相位吸收进系数、移除近似为零的项）；默认演化策略为自动精确/Trotter 选择、步数为 1。

### pauli_evolution_ansatz_new(hamiltonian)

从 Hamiltonian 创建 Pauli 演化 ansatz（过程中会被克隆并简化）。

- `hamiltonian` (`const CHamiltonian*`)：Hamiltonian 句柄。

返回新分配的 `CPauliEvolutionAnsatz*`；输入为 NULL 或算子为空时返回 NULL。

### pauli_evolution_ansatz_free(ptr)

释放 Pauli 演化 ansatz 句柄。

- `ptr` (`CPauliEvolutionAnsatz*`)：待释放句柄，允许传 NULL。

无返回值。

### pauli_evolution_ansatz_with_time_param_name(ptr, name)

覆盖演化 lowering 使用的时间参数符号。默认按 `"{prefix}_t"` 派生；设置名称后使用该名称。空名称重置为默认值。

- `ptr` (`CPauliEvolutionAnsatz*`)：模板句柄。
- `name` (`const char*`)：新的时间参数符号（空字符串重置为默认值）。

返回 0 表示成功；-1 表示输入为 NULL；-4 表示 UTF-8 非法。

### pauli_evolution_ansatz_evolution_info(ptr, out)

读取演化策略信息（核心层 `PauliEvolutionAnsatz::evolution_info` 的对应接口）写入 `out`。`out` 描述实际使用的策略：当前配置生成的分解与简化后 Hamiltonian 的结构信息，也可据此得知 `EVOLUTION_STRATEGY_AUTO` 的选择结果。

- `ptr` (`const CPauliEvolutionAnsatz*`)：模板句柄。
- `out` (`CPauliEvolutionInfo*`)：接收快照的输出结构体。

返回 0 表示成功；-1 表示输入为 NULL（`ptr` 或 `out`）。

`CPauliEvolutionInfo` 是布局稳定的透明结构体：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `is_exact` | `uint8_t` | 分解在数学上精确时为 1，否则为 0。 |
| `all_terms_commute` | `uint8_t` | 所有 Hamiltonian 项相互对易时为 1，否则为 0。 |
| `_pad` | `uint8_t[6]` | 保留填充字段，用于保持结构体布局稳定。 |
| `steps` | `uintptr_t` | 写入线路的分解重复次数。 |
| `num_terms` | `uintptr_t` | （简化后）Hamiltonian 的 Pauli 项数。 |
| `trotter_mode` | `int32_t` | `TROTTER_FIRST_ORDER`、`TROTTER_SECOND_ORDER` 或 `TROTTER_RANDOMIZED`；走精确单次路径时为 `TROTTER_MODE_NONE`。 |
| `trotter_seed` | `uint64_t` | `TROTTER_RANDOMIZED` 的随机种子；其他情况为 0。 |

### PauliEvolutionAnsatz 示例

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 单量子比特 H = X 下的演化。 */
    struct CPauliString *px = pauli_string_parse("X");
    struct CHamiltonian *ham = hamiltonian_from_pauli(px);  /* 接管 px 句柄 */

    struct CPauliEvolutionAnsatz *ansatz = pauli_evolution_ansatz_new(ham);
    if (ansatz == NULL) {
        hamiltonian_free(ham);
        return 1;
    }

    /* 重命名时间参数，再重置为默认值。 */
    pauli_evolution_ansatz_with_time_param_name(ansatz, "tau");
    pauli_evolution_ansatz_with_time_param_name(ansatz, "");

    /* 读取实际使用的策略：唯一一项且对易，走精确单次路径
       （trotter_mode == TROTTER_MODE_NONE）。 */
    struct CPauliEvolutionInfo info;
    if (pauli_evolution_ansatz_evolution_info(ansatz, &info) == 0) {
        printf("exact=%d commute=%d steps=%zu terms=%zu mode=%d\n",
               info.is_exact, info.all_terms_commute,
               info.steps, info.num_terms, info.trotter_mode);
    }

    pauli_evolution_ansatz_free(ansatz);
    hamiltonian_free(ham);
    return 0;
}
```

---

Hamiltonian 构造函数（`hamiltonian_from_pauli` / `hamiltonian_free`）见 [Hamiltonian](../3_qis/7_hamiltonian.md)，Pauli 字符串解析（`pauli_string_parse`）见 [Pauli](../3_qis/6_pauli.md)，线路句柄管理（`circuit_free`、`circuit_num_parameters`）见 [Circuit](1_circuit.md)。
