# 测量结果可视化

`cqlib_core::visualization::result`

本页覆盖测量结果的可视化：直方图 `plot_histogram`、概率分布图 `plot_distribution`，以及绘图选项 `ResultPlotOptions`。两个入口都接受执行结果对象，返回 SVG 标记字符串。

## 导入

```rust
use cqlib_core::visualization::{ResultPlotOptions, plot_distribution, plot_histogram};
```

---

## `plot_histogram(result, options) -> Result<String, VisualizationError>`

把执行结果的测量计数绘制为直方图。计数按结果的量子比特数格式化为比特串标签，作为横轴刻度；纵轴从零起算，不绘制负值区间。

参数：

- `result` (`&ExecutionResult`)：包含测量计数的执行结果。
- `options` (`&ResultPlotOptions`)：排序、配色与布局选项。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串。

异常情况：

- `VisualizationError::InvalidInput`：执行结果不含计数，或绘图选项不一致。

示例：

```rust
use cqlib_core::circuit::Qubit;
use cqlib_core::device::{ExecutionResult, Outcome};
use cqlib_core::visualization::{ResultPlotOptions, plot_histogram};
use std::collections::HashMap;

let mut result = ExecutionResult::new(
    "task-visualization".to_string(),
    (0..2).map(|idx| Qubit::new(idx as u32)).collect::<Vec<_>>(),
    7,
    2,
    Some("simulator".to_string()),
    None,
);
let counts = [("00", 2usize), ("11", 5)]
    .into_iter()
    .map(|(bits, count)| (Outcome::from_bitstring(bits).unwrap(), count))
    .collect::<HashMap<_, _>>();
result.finish(counts, None);

let svg = plot_histogram(&result, &ResultPlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## `plot_distribution(result, options) -> Result<String, VisualizationError>`

把执行结果的测量计数归一化为概率后绘制。每一条计数除以计数总和得到概率，柱高与纵轴刻度按概率标注。

参数：

- `result` (`&ExecutionResult`)：包含测量计数的执行结果。
- `options` (`&ResultPlotOptions`)：排序、配色与布局选项。

返回：

- `Result<String, VisualizationError>`：SVG 标记字符串。

异常情况：

- `VisualizationError::InvalidInput`：执行结果不含计数，计数求和为零，或绘图选项不一致。

示例：

```rust
let svg = plot_distribution(&result, &ResultPlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## ResultPlotOptions

结果图选项。

```rust
pub struct ResultPlotOptions {
    pub figsize: Option<(f64, f64)>,
    pub color: Vec<String>,
    pub number_to_keep: Option<usize>,
    pub sort: String,
    pub target_string: Option<String>,
    pub legend: Option<Vec<String>>,
    pub bar_labels: bool,
    pub title: Option<String>,
}
```

字段：

- `figsize` (`Option<(f64, f64)>`)：图尺寸，单位为类英寸单位，按每单位 `100` 像素换算为 SVG 像素，宽、高的下限均为 `2`。默认 `None`，画布为 `760 × 480` 像素。
- `color` (`Vec<String>`)：每个数据集的颜色。默认空向量，此时使用内置的定性调色板。
- `number_to_keep` (`Option<usize>`)：保留取值最大的 `k` 条，其余聚合成标签为 `rest` 的条目。默认 `None`，不聚合。`k` 为 `0` 或不小于条目数时不聚合；取值并列时按键的标签排序，保证输出跨平台一致。
- `sort` (`String`)：排序策略。默认 `"asc"`。取值见下表。
- `target_string` (`Option<String>`)：`sort = "hamming"` 使用的目标比特串。默认 `None`。
- `legend` (`Option<Vec<String>>`)：图例条目，每个数据集一条。默认 `None`，不绘制图例。结果绘制入口只产生一个数据集，因此给出图例时其长度必须为 `1`。
- `bar_labels` (`bool`)：是否在柱顶绘制数值标签。默认 `true`。
- `title` (`Option<String>`)：图标题。默认 `None`，不绘制标题。

### 排序策略

| 取值 | 说明 |
| --- | --- |
| `asc` | 按比特串标签的字典序升序。 |
| `desc` | 按比特串标签的字典序降序。 |
| `value` | 按标签在各数据集中的峰值升序；峰值并列时按标签升序。 |
| `value_desc` | 按标签在各数据集中的峰值降序；峰值并列时按标签升序。 |
| `hamming` | 按与 `target_string` 的汉明距离升序；距离并列时按标签升序。标签 `rest` 始终排在末尾。 |

```rust
use cqlib_core::visualization::ResultPlotOptions;

let options = ResultPlotOptions {
    sort: "value_desc".to_string(),
    number_to_keep: Some(8),
    title: Some("Measurement counts".to_string()),
    ..ResultPlotOptions::default()
};
```

---

## 示例

### 按汉明距离排序并聚合其余条目

沿用上例的执行结果，按与目标比特串的汉明距离排序，只保留计数最大的 `1` 条，其余聚合成 `rest`：

```rust
use cqlib_core::visualization::{ResultPlotOptions, plot_histogram};

let options = ResultPlotOptions {
    color: vec!["#123456".to_string()],
    number_to_keep: Some(1),
    sort: "hamming".to_string(),
    target_string: Some("00".to_string()),
    legend: Some(vec!["sim".to_string()]),
    bar_labels: false,
    title: Some("Hamming order".to_string()),
    ..ResultPlotOptions::default()
};

let svg = plot_histogram(&result, &options).unwrap();
assert!(svg.contains("rest"));
```

### 自定义画布尺寸与配色

沿用上例的执行结果，指定画布尺寸、配色、图例与标题：

```rust
use cqlib_core::visualization::{ResultPlotOptions, plot_distribution};

let options = ResultPlotOptions {
    figsize: Some((3.2, 2.4)),
    color: vec!["#0f766e".to_string()],
    legend: Some(vec!["probability".to_string()]),
    bar_labels: false,
    title: Some("Distribution options".to_string()),
    ..ResultPlotOptions::default()
};

let svg = plot_distribution(&result, &options).unwrap();
assert!(svg.contains("width=\"320\""));
assert!(svg.contains("height=\"240\""));
```

---

## 相关页面

- 把绘制结果写出为文件见 [落盘与输出](5_render_to_file.md)。
