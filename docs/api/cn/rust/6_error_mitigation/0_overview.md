# 错误缓解

`cqlib_core::error_mitigation`

`cqlib_core::error_mitigation` 在期望值层面降低噪声带来的系统性偏差，提供零噪声外推与虚拟蒸馏两条实现路径，以及把两者收敛到同一套执行顺序的统一流水线。缓解只修正可观测量期望值的估计结果，不属于量子纠错；代价是线路执行次数增加，且执行本身由调用方通过估计器提供。

## Overview

错误缓解作用于可观测量期望值：同一条基底线路按不同噪声强度重复构造并执行，再由拟合或比值把结果修正回理想极限。构造出来的线路（折叠线路、copy-swap 线路）最终由调用方提供的估计器执行并取值，本模块只做构造、编排与后处理。

### 三种入口

- **低层 `ZNEMitigation`**（`cqlib_core::error_mitigation::zne_mitigation`）：把零噪声外推拆成可独立调用的步骤——`fold_circuits` 只构造折叠线路，`run_em_sequence` 与 `run_em_sequence_with_shots` 逐条执行并收集期望值，`extrapolate` 等三个方法单独做外推。
- **低层 `VirtualDistillation`**（`cqlib_core::error_mitigation::virtual_distillation`）：`build_copy_swap_circuit` 构造 copy-swap 线路，`run_denominator_circuit` 与 `run_numerator_circuit` 分别取出分母与分子的均值与方差，`run_vd` 一步给出比值结果。
- **统一流水线 `ErrorMitigation`**（`cqlib_core::error_mitigation::unified`）：按 `new` → `run` → `get_mitigated` 顺序推进的状态机。缓解方法由 `MitigationMethod` 选定，`run` 与 `get_mitigated` 的输入分别由 `RunArgs`、`ProcessArgs` 指定，两者都必须与选定方法一致。统一流水线内部沿用与低层入口相同的折叠、copy-swap 与外推实现，因此两条路径得到的结果一致。

需要逐步检查中间产物（折叠线路、分子与分母统计）时用低层入口；只关心最终期望值时用统一流水线。

### 估计器

本模块不自带仿真或执行后端，全部缓解方法通过同一个回调取值：

```rust
pub type Estimator<'a> = dyn Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) + 'a;
```

参数依次为待执行线路、待估计的可观测量与 shot 数，返回 `(期望值, 方差)`。分母线路不携带可观测量，第二个参数为 `None`；`ZNEMitigation::run_em_sequence` 不指定 shot 数，第三个参数为 `None`。公开方法统一以 `&Estimator<'_>` 接收它，即 trait 对象引用 `&dyn Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64)`，闭包与函数项都可以直接取引用传入。返回值中的方差只有虚拟蒸馏会使用，零噪声外推只取期望值。

### 缓解流水线状态

`ErrorMitigation` 的每个实例遵循固定生命周期：创建 → `run()` → `get_mitigated()`。`run` 与 `get_mitigated` 各只能成功调用一次，顺序颠倒或重复调用都返回错误；调用失败时实例状态保持不变，可以修正参数后重试。需要多次缓解时重新创建实例。

---

## 常用入口

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{
    ErrorMitigation, ExtrapolateMethod, MitigationMethod, ProcessArgs, RunArgs, ZneConfig,
};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.x(q0).unwrap();

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let mut mitigation = ErrorMitigation::new(
    circuit,
    MitigationMethod::Zne(ZneConfig {
        fold_levels: vec![0, 1, 2],
    }),
)
.unwrap();

// 对接模拟器或真机后端：返回 (期望值, 方差)
let estimator = |circuit: &Circuit, hamiltonian: Option<&Hamiltonian>, shots: Option<usize>| {
    assert!(hamiltonian.is_some());
    assert_eq!(shots, Some(256));
    (circuit.operations().len() as f64 + 0.5, 0.0)
};

mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &estimator,
    )
    .unwrap();

let mitigated = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: None,
    })
    .unwrap();

assert_eq!(mitigated.expectation, 0.5);
```

---

## 核心概念与术语

| 术语 | 说明 |
| --- | --- |
| **错误缓解** | 在期望值层面修正噪声导致的系统性偏差的方法集合，不是量子纠错。 |
| **估计器（Estimator）** | 由调用方提供的回调，形态为 `Estimator`；接收待执行线路、可选可观测量与可选 shot 数，返回 `(期望值, 方差)`。 |
| **零噪声外推（ZNE）** | 通过门折叠放大线路噪声，得到一组不同噪声强度下的期望值，再外推到零噪声极限的方法。 |
| **折叠等级** | 零噪声外推的噪声放大级别。等级 0 对应原始线路，等级 `l` 对应噪声因子 `2 * l + 1`。 |
| **噪声因子** | 折叠线路相对原始线路的噪声放大倍数，恒等于 `2 * 折叠等级 + 1`。 |
| **门折叠** | 把线路改写为 `U -> U (U† U)^level` 的构造方式；只对指定门折叠时称为选择性折叠。 |
| **虚拟蒸馏** | 通过副本与 copy-swap 线路估计 `Tr(O ρ^M) / Tr(ρ^M)` 的方法，`M` 为副本数。 |
| **副本（copy）** | 虚拟蒸馏中并行准备的基底线路份数，最少为 2。 |
| **copy-swap 线路** | 把多份基底线路并排放在互不重叠的比特区间、并在首份与其他每份之间插入逐比特 SWAP 的线路，宽度等于副本数乘以基底线路宽度。 |
| **分子线路 / 分母线路** | 虚拟蒸馏的两条统计线路。分子线路携带扩展后的可观测量；分母线路不携带可观测量，估计器收到 `None`。 |
| **缓解流水线** | `ErrorMitigation` 表示的顺序式流程：`new` 创建、`run` 收集原始数据、`get_mitigated` 做方法相关的后处理。 |

---

## `cqlib_core::error_mitigation` API 概览

### 低层入口

| 名字 | 简介 |
| --- | --- |
| [`ZNEMitigation`](1_zne.md) | 零噪声外推的低层入口：门折叠、批量取值与外推。 |
| [`ExtrapolateMethod`](1_zne.md) | 外推方法枚举，包括多项式与指数。 |
| [`ZneConfig`](1_zne.md) | 零噪声外推的折叠等级配置，供统一流水线与方法枚举使用。 |
| [`VirtualDistillation`](2_virtual_distillation.md) | 虚拟蒸馏的低层入口：copy-swap 线路构造与分子、分母统计。 |
| [`VirtualDistillationConfig`](2_virtual_distillation.md) | 虚拟蒸馏的副本数配置，供统一流水线与方法枚举使用。 |

### 统一流水线

| 名字 | 简介 |
| --- | --- |
| [`ErrorMitigation`](3_unified.md) | 顺序式统一入口，编排构造、执行与后处理。 |
| [`MitigationMethod`](3_unified.md) | 缓解方法枚举，携带对应方法的配置。 |
| [`RunArgs`](3_unified.md) | 执行阶段的参数枚举。 |
| [`ProcessArgs`](3_unified.md) | 后处理阶段的参数枚举。 |
| [`MitigatedResult`](3_unified.md) | 最终缓解结果，包含期望值与可选方差。 |

### 回调与错误

| 名字 | 简介 |
| --- | --- |
| [`Estimator`](0_overview.md) | 估计器回调类型别名，全部缓解方法共用。 |
| [`ErrorMitigationError`](3_unified.md) | 模块统一错误类型。 |

---

## 快速示例

### 1. 用统一流水线执行虚拟蒸馏

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::{
    ErrorMitigation, MitigationMethod, ProcessArgs, RunArgs, VirtualDistillationConfig,
};
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let mut mitigation = ErrorMitigation::new(
    Circuit::new(1),
    MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
)
.unwrap();

// 分子线路带可观测量，分母线路不带
mitigation
    .run(
        &hamiltonian,
        RunArgs::VirtualDistillation {
            shots_numerator: 3,
            shots_denominator: 2,
        },
        &|_circuit, hamiltonian_arg, shots| {
            if hamiltonian_arg.is_some() {
                assert_eq!(shots, Some(3));
                (1.5, 0.25)
            } else {
                assert_eq!(shots, Some(2));
                (2.0, 1.0)
            }
        },
    )
    .unwrap();

let mitigated = mitigation
    .get_mitigated(ProcessArgs::VirtualDistillation)
    .unwrap();

assert!((mitigated.expectation - 0.75).abs() < 1e-12);
assert!((mitigated.variance.unwrap() - 0.203125).abs() < 1e-12);
```

### 2. 全局折叠与选择性折叠

```rust
use cqlib_core::circuit::gate::{Instruction, StandardGate};
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{ExtrapolateMethod, ZNEMitigation};

let q0 = Qubit::new(0);
let mut circuit = Circuit::new(1);
circuit.h(q0).unwrap();
circuit.s(q0).unwrap();

let zne = ZNEMitigation::new(circuit, vec![0, 1, 2]);
assert_eq!(zne.noise_factors(), &[1, 3, 5]);

// 全局折叠：三个等级各得到一条折叠线路
let global = zne.fold_circuits(None).unwrap();
assert_eq!(global.len(), 3);

// 选择性折叠：只放大 S 门，H 门保持原样
let gate_set = vec![Instruction::Standard(StandardGate::S)];
let selective = zne.fold_circuits(Some(&gate_set)).unwrap();
assert_eq!(selective[1].operations().len(), 4);

// 以噪声因子为自变量外推到零噪声
let extrapolated = zne
    .extrapolate(&[1.5, 3.5, 5.5], ExtrapolateMethod::Polynomial, 1)
    .unwrap();
assert!((extrapolated - 0.5).abs() < 1e-10);
```

### 3. 低层虚拟蒸馏的分子与分母

```rust
use cqlib_core::circuit::Circuit;
use cqlib_core::error_mitigation::VirtualDistillation;
use cqlib_core::qis::{Hamiltonian, Pauli, PauliString};
use num_complex::Complex64;

let mut pauli = PauliString::new(1);
pauli.set_pauli(0, Pauli::Z);
let hamiltonian = Hamiltonian::from_list(vec![(pauli, Complex64::new(1.0, 0.0))]).unwrap();

let vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();

// 两份基底线路加一对逐比特 SWAP
let copy_swap = vd.build_copy_swap_circuit().unwrap();
assert_eq!(copy_swap.width(), 2);

let (mu_vd, var_vd) = vd
    .run_vd(&hamiltonian, 3, 2, &|_circuit, hamiltonian_arg, shots| {
        if hamiltonian_arg.is_some() {
            assert_eq!(shots, Some(3));
            (1.5, 0.25)
        } else {
            assert_eq!(shots, Some(2));
            (2.0, 1.0)
        }
    })
    .unwrap();

assert!((mu_vd - 0.75).abs() < 1e-12);
assert!((var_vd - 0.203125).abs() < 1e-12);
```

---

## 校验与错误处理

缓解过程中的校验分两处：配置层面的校验发生在创建阶段，输入层面的校验发生在折叠、执行与外推阶段，两者都通过 `ErrorMitigationError` 返回。折叠等级为负时，低层折叠入口返回的是线路错误 `CircuitError::InvalidControlOperation`，而统一流水线的创建阶段直接拒绝该配置。

| 错误 | 触发场景 |
| --- | --- |
| `ErrorMitigationError::InvalidCopies` | 虚拟蒸馏的副本数小于 2。 |
| `ErrorMitigationError::InvalidFoldLevel` | 零噪声外推的折叠等级为负，在 `ErrorMitigation::new` 中被拒绝。 |
| `ErrorMitigationError::HamiltonianQubitCountMismatch` | 哈密顿量的比特数与基底线路宽度不一致，见 [ZNE](1_zne.md) 与 [虚拟蒸馏](2_virtual_distillation.md)。 |
| `ErrorMitigationError::ZeroDenominatorMean` | 虚拟蒸馏的分母线路均值为零，比值无法计算。 |
| `ErrorMitigationError::RunRequiredBeforeMitigation` / `AlreadyRun` / `AlreadyMitigated` | 缓解流水线的调用顺序错误或重复调用。 |
| `ErrorMitigationError::RunArgsMethodMismatch` / `ProcessArgsMethodMismatch` | 执行参数或后处理参数与 `MitigationMethod` 不一致。 |
| `ErrorMitigationError::EmptyNoisyResults` / `NoisyResultsLengthMismatch` / `InvalidPolynomialDegree` / `NonPositiveNoisyResults` / `SingularExponentialFit` / `SingularPolynomialFit` | 外推输入非法或拟合失败。 |
| `ErrorMitigationError::Circuit` / `Qis` | 由线路操作或量子信息计算透传的错误。 |

全部变体的完整定义见 [统一流水线](3_unified.md)。

---

## 下一步

- [零噪声外推](1_zne.md)：折叠语义、噪声因子与外推方法。
- [虚拟蒸馏](2_virtual_distillation.md)：copy-swap 线路、分子与分母统计。
- [统一流水线](3_unified.md)：`ErrorMitigation` 状态机、参数枚举与错误类型。
