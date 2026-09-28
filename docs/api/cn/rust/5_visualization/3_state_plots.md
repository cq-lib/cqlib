# 量子态可视化

`cqlib_core::visualization::state`

本页覆盖量子态的可视化：布洛赫向量图 `plot_bloch_vector`、约化布洛赫向量图 `plot_bloch_multivector`、密度矩阵图 `plot_state_city`、泡利基期望值图 `plot_state_paulivec`，以及绘图选项 `StatePlotOptions`。所有绘图入口直接接受核心态对象，返回 SVG 标记字符串。

## 导入

```rust
use cqlib_core::visualization::{
    StatePlotOptions, plot_bloch_multivector, plot_bloch_vector, plot_state_city,
    plot_state_paulivec,
};
```

---

## `plot_bloch_vector(vector, options) -> Result<String, VisualizationError>`

绘制单个布洛赫向量。

参数：

- `vector` (`[f64; 3]`)：布洛赫坐标 `(x, y, z)`，分量通常落在 `[-1, 1]`。模长大于 `1` 的向量会被投影回单位球面，落在球内的向量保持不变。
- `options` (`&StatePlotOptions`)：绘图选项。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串。向量标签固定为 `q0`。

示例：

```rust
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_vector};

let svg = plot_bloch_vector([0.0, 0.0, 1.0], &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## `plot_bloch_multivector(state, options) -> Result<String, VisualizationError>`

为每个量子比特绘制一个约化布洛赫向量。每个向量由该比特的约化密度矩阵计算得到，标签按量子比特编号写成 `q0`、`q1` 等。

参数：

- `state` (`&S`，`S: StateVisualizationSource + ?Sized`)：输入态，`Statevector` 与 `DensityMatrix` 均可直接传入。
- `options` (`&StatePlotOptions`)：绘图选项。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串。多个球排成网格，每行最多 4 个。

异常情况：

- `VisualizationError::InvalidInput`：状态缓冲长度与量子比特数不符，或量子比特数过大导致矩阵维度溢出。

示例：

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## `plot_state_city(state, options) -> Result<String, VisualizationError>`

绘制密度矩阵的实部与虚部，每部分一个面板。只绘制虚部非零的输入：当所有元素的虚部均为零时，SVG 中不生成虚部面板。每个单元的面积随 `sqrt(|value| / max_abs)` 缩放，使小幅度元素仍可见；`max_abs` 取矩阵元素模长的最大值，下限为 `1e-12`。

参数：

- `state` (`&S`，`S: StateVisualizationSource + ?Sized`)：输入态。纯态先展开为密度矩阵。
- `options` (`&StatePlotOptions`)：绘图选项。填充不透明度由 `alpha` 控制。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串，面板标题为 `Re[rho]` 与 `Im[rho]`。

异常情况：

- `VisualizationError::InvalidInput`：状态缓冲长度与量子比特数不符，或量子比特数过大导致矩阵维度溢出。

示例：

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_state_city};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
)
.unwrap();

let svg = plot_state_city(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("Re[rho]"));
```

---

## `plot_state_paulivec(state, options) -> Result<String, VisualizationError>`

把泡利基期望值绘制为柱状图。标签为 `I`、`X`、`Y`、`Z` 的张量积，共 `4^n` 条，其中 `n` 为量子比特数。非负柱与负值柱使用不同颜色。

参数：

- `state` (`&S`，`S: StateVisualizationSource + ?Sized`)：输入态。
- `options` (`&StatePlotOptions`)：绘图选项。`color` 的第一个元素是非负柱颜色，第二个元素是负值柱颜色。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串。柱数不超过 `64` 时才绘制旋转的横轴标签。

异常情况：

- `VisualizationError::InvalidInput`：状态缓冲长度与量子比特数不符，或量子比特数过大导致矩阵维度溢出。

示例：

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_state_paulivec};
use num_complex::Complex64;

let state = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_state_paulivec(&state, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## StatePlotOptions

量子态绘图选项。

```rust
pub struct StatePlotOptions {
    pub title: Option<String>,
    pub color: Vec<String>,
    pub alpha: f64,
    pub reverse_bits: bool,
    pub figsize: Option<(f64, f64)>,
}
```

字段：

- `title` (`Option<String>`)：图标题。默认 `None`，不绘制标题。
- `color` (`Vec<String>`)：部分绘图族使用的配色。`plot_state_paulivec` 取第一个元素作为非负柱颜色，缺省为 `#4569d4`；取第二个元素作为负值柱颜色，缺省为 `#d64b5f`。其余绘图族使用固定配色。默认空向量。
- `alpha` (`f64`)：`plot_state_city` 柱体的填充不透明度。默认 `1.0`。
- `reverse_bits` (`bool`)：是否反转计算基的显示位序。默认 `false`。置为 `true` 时，密度矩阵图的行列标签按相反顺序排列，矩阵元素按位序映射回存储位置；布洛赫图与泡利图的标签顺序随之反转。
- `figsize` (`Option<(f64, f64)>)`：图尺寸，单位为类英寸单位，按每单位 `100` 像素换算为 SVG 像素。默认 `None`，各绘图族按自身布局计算画布尺寸。给定尺寸时，密度矩阵图与泡利图对宽、高的下限分别为 `4` 与 `3`，布洛赫图对宽、高的下限均为 `3`。

```rust
use cqlib_core::visualization::StatePlotOptions;

let options = StatePlotOptions {
    title: Some("State city".to_string()),
    reverse_bits: true,
    ..StatePlotOptions::default()
};
```

---

## 辅助 API

### StateVisualizationSource

量子态绘图入口接受的输入 trait。它把核心态对象适配为密度矩阵数据，而不是另一套数据容器。

```rust
pub trait StateVisualizationSource {
    fn num_qubits(&self) -> usize;
    fn density_matrix_data(&self) -> Result<Vec<Complex64>, VisualizationError>;
}
```

实现者：

- `Statevector`：按 `ρ = |ψ⟩⟨ψ|` 展开为密度矩阵。
- `DensityMatrix`：直接返回其数据。

实现者需要返回行主序的密度矩阵数据，长度为 `4^num_qubits`；维度非法时以 `VisualizationError::InvalidInput` 报错。

### `state_to_density_matrix(state) -> Result<(usize, Vec<Complex64>), VisualizationError>`

把核心态对象转换为行主序密度矩阵。返回量子比特数与长度为 `4^num_qubits` 的数据。

异常情况：

- `VisualizationError::InvalidInput`：量子比特数与缓冲长度不一致，或量子比特数过大导致矩阵维度溢出。

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::state_to_density_matrix;
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let (num_qubits, rho) = state_to_density_matrix(&state).unwrap();
assert_eq!(num_qubits, 1);
assert_eq!(rho.len(), 4);
```

### `local_bloch_vectors(state) -> Result<Vec<(usize, [f64; 3])>, VisualizationError>`

计算每个量子比特的约化布洛赫向量，返回 `(比特编号, [x, y, z])` 列表，按比特编号升序排列。

异常情况：

- `VisualizationError::InvalidInput`：状态缓冲长度与量子比特数不符。

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::local_bloch_vectors;
use num_complex::Complex64;

let state = Statevector::from_state(1, vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)])
    .unwrap();
let vectors = local_bloch_vectors(&state).unwrap();
assert_eq!(vectors.len(), 1);
assert!((vectors[0].1[2] - 1.0).abs() < 1e-10);
```

---

## 示例

### 纯态与混合态使用同一组入口

`Statevector` 与 `DensityMatrix` 都实现 `StateVisualizationSource`，同一组绘图入口对二者通用：

```rust
use cqlib_core::qis::{DensityMatrix, Statevector};
use cqlib_core::visualization::{StatePlotOptions, plot_state_city, plot_state_paulivec};
use num_complex::Complex64;

let statevector = Statevector::from_state(
    1,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();
let density_matrix =
    DensityMatrix::from_state(statevector.num_qubits, statevector.data().to_vec()).unwrap();

let sv_svg = plot_state_paulivec(&statevector, &StatePlotOptions::default()).unwrap();
let dm_svg = plot_state_paulivec(&density_matrix, &StatePlotOptions::default()).unwrap();
assert_eq!(sv_svg, dm_svg);
```

### 多比特态的布洛赫图

多比特态为每个量子比特绘制一个约化布洛赫向量，可用于观察纠缠态中各比特的局部信息：

```rust
use cqlib_core::qis::Statevector;
use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
use num_complex::Complex64;

let bell = Statevector::from_state(
    2,
    vec![
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0),
    ],
)
.unwrap();

let svg = plot_bloch_multivector(&bell, &StatePlotOptions::default()).unwrap();
assert!(svg.contains("q0"));
assert!(svg.contains("q1"));
```

---

## 相关页面

- 把绘制结果写出为文件见 [落盘与输出](5_render_to_file.md)。
