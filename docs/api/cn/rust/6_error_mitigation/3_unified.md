# 统一流水线

`cqlib_core::error_mitigation::unified`

统一流水线把零噪声外推与虚拟蒸馏收敛到同一条顺序式流程：创建实例、执行取值、后处理出结果。本页覆盖统一流水线本身、三个参数枚举、结果对象与模块统一错误类型。

## 导入

```rust
use cqlib_core::error_mitigation::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigatedResult, MitigationMethod,
    ProcessArgs, RunArgs, VirtualDistillationConfig, ZneConfig,
};
```

---

## ErrorMitigation

默认推荐入口。它保存基底线路、选定的缓解方法与流水线状态，把「构造缓解线路并取值」与「由原始数据算出缓解结果」拆成 `run` 与 `get_mitigated` 两步。

```rust
pub struct ErrorMitigation {
    // 字段私有：基底线路、缓解方法与流水线状态
}
```

方法：

- `fn new(circuit: Circuit, method: MitigationMethod) -> Result<Self, ErrorMitigationError>`：创建流水线并校验方法配置。
- `fn run(&mut self, hamiltonian: &Hamiltonian, run_args: RunArgs, estimator: &Estimator<'_>) -> Result<(), ErrorMitigationError>`：构造并执行该方法所需的线路，留存原始数据，不直接返回缓解值。
- `fn get_mitigated(&mut self, process_args: ProcessArgs) -> Result<MitigatedResult, ErrorMitigationError>`：由留存的原始数据算出最终结果。

统一流水线不提供读取原始数据的接口，也不提供修改已选方法的接口，两者都只在内部留存。它实现 `Debug` 与 `Clone`。

### 状态机

每个实例的生命周期固定为创建 → `run()` → `get_mitigated()`，两步各只能成功执行一次：

| 当前状态 | 调用 | 结果 |
| --- | --- | --- |
| 已创建 | `run` | 执行并转入已运行。 |
| 已创建 | `get_mitigated` | `ErrorMitigationError::RunRequiredBeforeMitigation`。 |
| 已运行 | `run` | `ErrorMitigationError::AlreadyRun`。 |
| 已运行 | `get_mitigated`，参数与方法一致 | 返回 `MitigatedResult`，转入已完成。 |
| 已运行 | `get_mitigated`，参数与方法不一致 | `ErrorMitigationError::ProcessArgsMethodMismatch`。 |
| 已完成 | `run` 或 `get_mitigated` | `ErrorMitigationError::AlreadyMitigated`。 |

调用失败时状态保持原值（已完成状态例外，它不再接受任何调用），因此参数不匹配或输入校验失败后可以改正参数重试。需要跑第二条缓解流程时重新创建实例。

### `new(circuit: Circuit, method: MitigationMethod) -> Result<Self, ErrorMitigationError>`

参数：

- `circuit`：基底线路，`run` 阶段的所有派生线路都由它构造。
- `method`：缓解方法与对应配置。

返回：

- `ErrorMitigation`：状态为已创建。

异常情况：

- `ErrorMitigationError::InvalidFoldLevel`：`MitigationMethod::Zne` 的折叠等级中存在负值。
- `ErrorMitigationError::InvalidCopies`：`MitigationMethod::VirtualDistillation` 的副本数小于 2。

### `run(&mut self, hamiltonian: &Hamiltonian, run_args: RunArgs, estimator: &Estimator<'_>) -> Result<(), ErrorMitigationError>`

按选定方法构造缓解线路并逐条取值，结果留在实例内部。

参数：

- `hamiltonian`：待估计的可观测量，其比特数必须等于基底线路宽度。
- `run_args`：执行参数，变体必须与 `MitigationMethod` 配套。
- `estimator`：估计器回调，形态见 [概览](0_overview.md)。

执行细节按方法区分：

- 零噪声外推：按配置的折叠等级构造折叠线路，逐条调用估计器，可观测量参数为 `Some(hamiltonian)`，shot 数为 `RunArgs::Zne::shots`；只取返回值中的期望值。折叠线路、期望值、噪声因子与执行参数一并留存。
- 虚拟蒸馏：构造 copy-swap 线路并把可观测量扩展到该线路宽度，分子线路的估计器收到 `Some(扩展后的可观测量)` 与 `Some(shots_numerator)`，分母线路收到 `None` 与 `Some(shots_denominator)`；分子与分母的均值、方差一并留存。

返回：

- `Result<(), ErrorMitigationError>`：成功仅表示原始数据已就位，缓解值由 `get_mitigated` 给出。

异常情况：

- `ErrorMitigationError::RunArgsMethodMismatch`：`run_args` 的变体与配置的方法不一致。
- `ErrorMitigationError::HamiltonianQubitCountMismatch`：哈密顿量的比特数与基底线路宽度不一致。
- `ErrorMitigationError::AlreadyRun` / `ErrorMitigationError::AlreadyMitigated`：当前状态不允许再次执行。
- `ErrorMitigationError::Circuit`：折叠、分解或线路构造失败时透传的线路错误。

### `get_mitigated(&mut self, process_args: ProcessArgs) -> Result<MitigatedResult, ErrorMitigationError>`

对留存的原始数据做方法相关的后处理。

参数：

- `process_args`：后处理参数，变体必须与 `MitigationMethod` 配套。

后处理细节按方法区分：

- 零噪声外推：按 `ProcessArgs::Zne::method` 选择外推方式。`Polynomial` 分支在 `degree` 为 `None` 时取 `min(数据点数 - 1, 1)`，其中数据点数等于折叠等级数，即默认按一次多项式外推；`Exponential` 分支不使用 `degree`。结果中的 `variance` 为 `None`。
- 虚拟蒸馏：按分子与分母的均值、方差组合出比值与方差，结果中的 `variance` 为 `Some`。

返回：

- `MitigatedResult`：缓解后的期望值与可选方差。

异常情况：

- `ErrorMitigationError::RunRequiredBeforeMitigation`：尚未调用 `run`。
- `ErrorMitigationError::ProcessArgsMethodMismatch`：`process_args` 的变体与配置的方法不一致。
- `ErrorMitigationError::AlreadyMitigated`：已经取过一次缓解结果。
- `ErrorMitigationError::ZeroDenominatorMean`、`EmptyNoisyResults`、`NoisyResultsLengthMismatch`、`InvalidPolynomialDegree`、`NonPositiveNoisyResults`、`SingularExponentialFit`、`SingularPolynomialFit`：后处理阶段透传的外推或比值错误，见 [零噪声外推](1_zne.md) 与 [虚拟蒸馏](2_virtual_distillation.md)。

### 示例

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::error_mitigation::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigatedResult, MitigationMethod,
    ProcessArgs, RunArgs, ZneConfig,
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

mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &|circuit, hamiltonian_arg, shots| {
            assert!(hamiltonian_arg.is_some());
            assert_eq!(shots, Some(256));
            (circuit.operations().len() as f64 + 0.5, 0.0)
        },
    )
    .unwrap();

// run 只能成功调用一次
let rerun_err = mitigation
    .run(
        &hamiltonian,
        RunArgs::Zne {
            gate_set: None,
            shots: Some(256),
        },
        &|_circuit, _hamiltonian, _shots| (0.0, 0.0),
    )
    .unwrap_err();
assert!(matches!(rerun_err, ErrorMitigationError::AlreadyRun));

// degree 为 None 时按 min(数据点数 - 1, 1) 取值，此处数据点数为 3
let mitigated = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: None,
    })
    .unwrap();

assert_eq!(
    mitigated,
    MitigatedResult {
        expectation: 0.5,
        variance: None,
    }
);

// get_mitigated 也只能成功调用一次
let second_err = mitigation
    .get_mitigated(ProcessArgs::Zne {
        method: ExtrapolateMethod::Polynomial,
        degree: Some(1),
    })
    .unwrap_err();
assert!(matches!(second_err, ErrorMitigationError::AlreadyMitigated));
```

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

---

## MitigationMethod

缓解方法枚举，携带对应方法的配置。

```rust
pub enum MitigationMethod {
    Zne(ZneConfig),
    VirtualDistillation(VirtualDistillationConfig),
}
```

- `Zne(ZneConfig)`：选择零噪声外推，配置折叠等级。
- `VirtualDistillation(VirtualDistillationConfig)`：选择虚拟蒸馏，配置副本数。

实现 `Debug`、`Clone`、`PartialEq`、`Eq`。方法与两类参数必须配套：

| 方法 | 执行参数 | 后处理参数 |
| --- | --- | --- |
| `MitigationMethod::Zne` | `RunArgs::Zne` | `ProcessArgs::Zne` |
| `MitigationMethod::VirtualDistillation` | `RunArgs::VirtualDistillation` | `ProcessArgs::VirtualDistillation` |

配置结构体见 [零噪声外推](1_zne.md) 与 [虚拟蒸馏](2_virtual_distillation.md)。

---

## RunArgs

执行阶段的参数枚举。

```rust
pub enum RunArgs {
    Zne {
        gate_set: Option<Vec<Instruction>>,
        shots: Option<usize>,
    },
    VirtualDistillation {
        shots_numerator: usize,
        shots_denominator: usize,
    },
}
```

- `Zne::gate_set`：选择性折叠的门集合；`None` 表示全局折叠，匹配按操作名称进行。
- `Zne::shots`：透传给估计器的 shot 数；`None` 表示不指定。
- `VirtualDistillation::shots_numerator`：分子线路的 shot 数。
- `VirtualDistillation::shots_denominator`：分母线路的 shot 数。

实现 `Debug`、`Clone`、`PartialEq`、`Eq`，其中 `PartialEq` 是自定义实现：`Zne` 分支比较 `shots` 与 `gate_set` 中每个操作的名称，要求元素个数一致且逐一名称相同，不比较操作携带的参数与线路定义，因此名称相同的不同操作实例判定为相等；`VirtualDistillation` 分支比较两个 shot 数。变体不同则判定为不相等。

---

## ProcessArgs

后处理阶段的参数枚举。

```rust
pub enum ProcessArgs {
    Zne {
        method: ExtrapolateMethod,
        degree: Option<usize>,
    },
    VirtualDistillation,
}
```

- `Zne::method`：外推方法。
- `Zne::degree`：多项式次数。`ExtrapolateMethod::Polynomial` 分支为 `None` 时取 `min(数据点数 - 1, 1)`，数据点数等于折叠等级数；`ExtrapolateMethod::Exponential` 分支忽略该字段。
- `VirtualDistillation`：无附加字段。

实现 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`。

---

## MitigatedResult

最终缓解结果。

```rust
pub struct MitigatedResult {
    pub expectation: f64,
    pub variance: Option<f64>,
}
```

字段：

- `expectation`：缓解后的期望值。
- `variance`：缓解结果的方差，只有虚拟蒸馏给出；零噪声外推为 `None`。

实现 `Debug`、`Clone`、`PartialEq`。

---

## ErrorMitigationError

模块统一错误类型，配置校验、输入校验、流水线状态与外推拟合的错误都通过它返回。

```rust
pub enum ErrorMitigationError {
    Circuit(CircuitError),
    Qis(QisError),
    InvalidCopies(usize),
    HamiltonianQubitCountMismatch { expected: usize, actual: usize },
    ZeroDenominatorMean,
    InvalidFoldLevel(i32),
    RunRequiredBeforeMitigation,
    AlreadyRun,
    AlreadyMitigated,
    RunArgsMethodMismatch,
    ProcessArgsMethodMismatch,
    EmptyNoisyResults,
    NoisyResultsLengthMismatch { expected: usize, actual: usize },
    InvalidPolynomialDegree { degree: usize, num_points: usize },
    NonPositiveNoisyResults,
    SingularExponentialFit,
    SingularPolynomialFit,
}
```

前两个变体透传来自线路模块与量子信息模块的错误，其余变体给出具体数值。实现 `Debug` 与 `Display`。

| 错误 | 触发场景 |
| --- | --- |
| `Circuit(CircuitError)` | 折叠、线路分解或线路构造失败。 |
| `Qis(QisError)` | 量子信息层面的计算失败。 |
| `InvalidCopies(usize)` | 虚拟蒸馏副本数小于 2：`VirtualDistillation::new`、`VirtualDistillation::set_copies` 与 `ErrorMitigation::new` 都会检查。 |
| `HamiltonianQubitCountMismatch { expected, actual }` | 哈密顿量比特数与基底线路宽度不一致，`expected` 为线路宽度，`actual` 为哈密顿量比特数。 |
| `ZeroDenominatorMean` | 虚拟蒸馏分母线路的均值为零。 |
| `InvalidFoldLevel(i32)` | `ErrorMitigation::new` 收到负的折叠等级。 |
| `RunRequiredBeforeMitigation` | 未调用 `run` 就调用 `get_mitigated`。 |
| `AlreadyRun` | 同一实例第二次调用 `run`。 |
| `AlreadyMitigated` | 已完成缓解的实例再次调用 `run` 或 `get_mitigated`。 |
| `RunArgsMethodMismatch` | `run` 的参数变体与配置的方法不一致。 |
| `ProcessArgsMethodMismatch` | `get_mitigated` 的参数变体与配置的方法不一致。 |
| `EmptyNoisyResults` | 外推收到的结果序列为空，例如折叠等级为空向量。 |
| `NoisyResultsLengthMismatch { expected, actual }` | 外推结果序列长度不等于噪声因子个数。 |
| `InvalidPolynomialDegree { degree, num_points }` | 多项式次数不小于数据点数。 |
| `NonPositiveNoisyResults` | 指数外推收到非正的结果值。 |
| `SingularExponentialFit` | 指数外推的线性回归退化。 |
| `SingularPolynomialFit` | 多项式外推的正规方程矩阵奇异。 |
