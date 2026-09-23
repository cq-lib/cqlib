# SVG 线路图

`cqlib_core::visualization::circuit::figure`

本页覆盖线路的图形后端：绘制入口 `circuit_to_figure`、基于可视化 IR 的入口 `draw_figure_from_visual`、绘制选项 `FigureDrawerOptions`，以及选项引用到的样式与参数格式化类型。图形后端把线路渲染为 SVG 标记字符串，画布底色为白色。

## 导入

```rust
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};
```

---

## `circuit_to_figure(circuit, options) -> Result<String, VisualizationError>`

生成线路的 SVG 标记。内部先构建可视化 IR，再交由图形后端绘制。

参数：

- `circuit` (`&Circuit`)：待绘制的线路。
- `options` (`&FigureDrawerOptions`)：图形绘制选项。

返回：

- `Result<String, VisualizationError>`：成功时为 SVG 标记字符串。

异常情况：

- `VisualizationError::CircuitBuild`：`decompose_circuit_gates` 为 `true` 时展开线路门定义失败。
- `VisualizationError::UnknownQubit`：操作引用了不在线路量子比特列表中的量子比特。
- `VisualizationError::ParameterIndexOutOfBounds`：符号参数的索引超出线路参数表的长度。

示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, circuit_to_figure};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();

let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## FigureDrawerOptions

图形绘制选项。

```rust
pub struct FigureDrawerOptions {
    pub show_params: bool,
    pub decompose_circuit_gates: bool,
    pub parameter_format: ParameterFormatOptions,
    pub width_per_column: f64,
    pub height_per_qubit: f64,
    pub dpi: u32,
    pub gate_width: f64,
    pub gate_height: f64,
    pub moment_spacing: f64,
    pub connect_height: f64,
    pub fold: i32,
    pub style: FigureDrawStyle,
    pub gate_styles: HashMap<String, GateStyle>,
    pub initial_state: bool,
    pub reverse_bits: bool,
}
```

字段：

- `show_params` (`bool`)：是否在门标签后附加参数文本。默认 `true`。
- `decompose_circuit_gates` (`bool`)：绘制前是否展开线路门定义。默认 `false`。
- `parameter_format` (`ParameterFormatOptions`)：门参数文本的格式化选项。默认 `ParameterFormatOptions::default()`。
- `width_per_column` (`f64`)：每个逻辑列的图宽缩放系数。默认 `1.2`。
- `height_per_qubit` (`f64`)：每个量子比特的图高缩放系数。默认 `0.9`。
- `dpi` (`u32`)：导出 PNG 时的栅格化 DPI。默认 `160`，此时 PNG 缩放为 `1.0`，尺寸与 SVG 原生像素一致；更大的取值按比例放大 PNG 尺寸，SVG 输出不受影响。取值为 `0` 时绘制失败。
- `gate_width` (`f64`)：门的基础宽度（数据单位），同时作为列宽下限。默认 `1.1`。
- `gate_height` (`f64`)：门的基础高度（数据单位）。默认 `1.5`。
- `moment_spacing` (`f64`)：相邻列之间的水平间距。默认 `0.3`。
- `connect_height` (`f64`)：折叠后各行之间的垂直间距。默认 `2.0`。
- `fold` (`i32`)：每行最大列数。默认 `18`；取负值或 `0` 时不折叠。
- `style` (`FigureDrawStyle`)：样式预设。默认 `FigureDrawStyle::Cqlib`。
- `gate_styles` (`HashMap<String, GateStyle>`)：按门名覆盖样式，覆盖项合并到基础样式表之上。默认空映射。
- `initial_state` (`bool`)：是否在量子比特标签中显示 `|0>`。默认 `false`。
- `reverse_bits` (`bool`)：是否反转量子比特的显示顺序。默认 `false`。

默认值由 `FigureDrawerOptions::default()` 提供：

```rust
use cqlib_core::visualization::FigureDrawerOptions;

let options = FigureDrawerOptions {
    fold: 8,
    ..FigureDrawerOptions::default()
};
```

---

## FigureDrawStyle

图形样式预设。

```rust
pub enum FigureDrawStyle {
    /// Cqlib 默认样式。
    Cqlib,
}
```

`FigureDrawStyle::Cqlib` 决定线路走线、连线、门边框、门填充与标签的默认配色和线宽。

---

## GateStyle

单个门的样式覆盖项。所有字段均为可选，为 `None` 时沿用基础样式。

```rust
pub struct GateStyle {
    pub border_color: Option<String>,
    pub background_color: Option<String>,
    pub font_size: Option<f64>,
    pub text_color: Option<String>,
    pub line_color: Option<String>,
    pub line_width: Option<f64>,
}
```

字段：

- `border_color` (`Option<String>`)：门边框颜色。默认 `None`。
- `background_color` (`Option<String>`)：门填充颜色。默认 `None`。
- `font_size` (`Option<f64>`)：门标签字号，同时参与门宽测量。默认 `None`。
- `text_color` (`Option<String>`)：门标签颜色。默认 `None`。
- `line_color` (`Option<String>`)：门的连线颜色。默认 `None`。
- `line_width` (`Option<f64>`)：门的线宽。默认 `None`。

以门名为键写入 `FigureDrawerOptions::gate_styles` 即可覆盖该门的样式：

```rust
use cqlib_core::circuit::{Circuit, Parameter, Qubit};
use cqlib_core::visualization::{FigureDrawerOptions, GateStyle, circuit_to_figure};
use std::collections::HashMap;

let mut circuit = Circuit::new(1);
let long_param = Parameter::symbol("styledlongparametername");
circuit.rz(Qubit::new(0), long_param).unwrap();

let mut gate_styles = HashMap::new();
gate_styles.insert(
    "RZ".to_string(),
    GateStyle {
        font_size: Some(16.0),
        ..GateStyle::default()
    },
);

let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        gate_styles,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
assert!(svg.contains("<svg"));
```

---

## 参数格式化

`FigureDrawerOptions::parameter_format` 控制门参数在标签中的文本形态。

### ParameterDisplayMode

```rust
pub enum ParameterDisplayMode {
    /// 可求值时优先显示数值。
    Numeric,
    /// 优先显示符号表达式。
    Symbolic,
    /// 显示符号表达式，并在可求值时追加数值。
    SymbolicWithValue,
    /// 对接近常见角度的取值优先使用 `kπ/n` 形式。
    PiFractionPreferred,
}
```

### ParameterFormatOptions

```rust
pub struct ParameterFormatOptions {
    pub mode: ParameterDisplayMode,
    pub decimal_precision: usize,
    pub scientific_lower_bound: f64,
    pub scientific_upper_bound: f64,
    pub pi_tolerance: f64,
    pub pi_max_denominator: i64,
}
```

字段：

- `mode` (`ParameterDisplayMode`)：显示模式。默认 `ParameterDisplayMode::Numeric`。
- `decimal_precision` (`usize`)：定点与科学计数法的十进制精度。默认 `2`。
- `scientific_lower_bound` (`f64`)：绝对值落在 `(0, scientific_lower_bound)` 内的取值使用科学计数法。默认 `1e-3`。
- `scientific_upper_bound` (`f64`)：绝对值不小于该值的取值使用科学计数法。默认 `1e4`。
- `pi_tolerance` (`f64`)：把取值与 π 的比值匹配到有理分数时的容差。默认 `1e-3`。
- `pi_max_denominator` (`i64`)：π 分数匹配使用的最大分母。默认 `16`。

数值模式下，`1.2345` 显示为 `1.23`；超出科学计数法阈值的取值显示为 `4e-4`、`1.5e4` 这样的形式。`PiFractionPreferred` 模式下，`π/2` 显示为 `π/2`。

```rust
use cqlib_core::visualization::{FigureDrawerOptions, ParameterDisplayMode, ParameterFormatOptions};

let options = FigureDrawerOptions {
    parameter_format: ParameterFormatOptions {
        mode: ParameterDisplayMode::PiFractionPreferred,
        ..ParameterFormatOptions::default()
    },
    ..FigureDrawerOptions::default()
};
```

### ParameterFormatter

把单个参数条目格式化为显示标签的格式化器，可视化 IR 构建过程中使用它把参数写入每个操作的标签。

```rust
pub struct ParameterFormatter {
    options: ParameterFormatOptions,
}
```

方法：

- `ParameterFormatter::new(options: ParameterFormatOptions) -> Self`：按给定选项构造格式化器。
- `fn format_circuit_param(&self, circuit: &Circuit, param: &CircuitParam) -> Result<String, VisualizationError>`：格式化一个参数条目。`CircuitParam::Fixed` 直接使用其数值；`CircuitParam::Index` 通过线路的参数表解析为符号表达式。

异常情况：

- `VisualizationError::ParameterIndexOutOfBounds`：`CircuitParam::Index` 的索引超出线路参数表的长度。

```rust
use cqlib_core::circuit::{Circuit, CircuitParam};
use cqlib_core::visualization::{ParameterFormatOptions, ParameterFormatter};

let formatter = ParameterFormatter::new(ParameterFormatOptions::default());
let circuit = Circuit::new(0);

assert_eq!(
    formatter
        .format_circuit_param(&circuit, &CircuitParam::Fixed(1.2345))
        .unwrap(),
    "1.23"
);
```

---

## `draw_figure_from_visual(visual, options, output_path) -> String`

从已构建的可视化 IR 绘制 SVG。已有缓存 IR 时使用本入口，可避免重复构建。

参数：

- `visual` (`&VisualCircuit`)：由 `build_visual_circuit` 构建的可视化 IR。
- `options` (`&FigureDrawerOptions`)：图形绘制选项。
- `output_path` (`Option<&str>`)：保留参数，当前被忽略。

返回：

- `String`：SVG 标记字符串。

```rust
use cqlib_core::circuit::{Circuit, Qubit};
use cqlib_core::visualization::{
    FigureDrawerOptions, VisualBuildOptions, build_visual_circuit, draw_figure_from_visual,
};

let mut circuit = Circuit::new(1);
circuit.h(Qubit::new(0)).unwrap();
let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
let svg = draw_figure_from_visual(&visual, &FigureDrawerOptions::default(), None);
assert!(svg.contains("<svg"));
```

---

## 输出形态

### 折叠

`fold` 限制每行的列数，超出的列被排到后续行。折叠后的每一行保留完整量子比特线，行与行之间的间距由 `connect_height` 控制：

```rust
let svg = circuit_to_figure(
    &circuit,
    &FigureDrawerOptions {
        fold: 4,
        ..FigureDrawerOptions::default()
    },
)
.unwrap();
```

### 展开线路门与显示初始态

`decompose_circuit_gates` 为 `false` 时，线路门作为一个带标签的门块绘制；置为 `true` 时先展开为其定义线路再绘制。`initial_state` 为 `true` 时在量子比特标签旁标出初始态。

---

## 相关页面

- 文本后端的绘制入口与选项见 [文本线路图](1_draw_text.md)。
- 将 SVG 写出为文件见 [落盘与输出](5_render_to_file.md)。
