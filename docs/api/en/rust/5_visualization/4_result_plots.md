# Measurement Result Visualization

`cqlib_core::visualization::result`

This page covers measurement result visualization: the histogram `plot_histogram`, the probability distribution plot `plot_distribution`, and the plot options `ResultPlotOptions`. Both entry points accept execution result objects and return an SVG markup string.

## Import

```rust
use cqlib_core::visualization::{ResultPlotOptions, plot_distribution, plot_histogram};
```

---

## `plot_histogram(result, options) -> Result<String, VisualizationError>`

Draw the measurement counts of an execution result as a histogram. The counts are formatted into bitstring labels according to the number of qubits of the result and used as the horizontal axis ticks; the vertical axis starts from zero and no negative region is drawn.

Parameters:

- `result` (`&ExecutionResult`): the execution result containing the measurement counts.
- `options` (`&ResultPlotOptions`): the ordering, color scheme and layout options.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string.

Raises:

- `VisualizationError::InvalidInput`: the execution result contains no counts, or the plot options are inconsistent.

Example:

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

Normalize the measurement counts of an execution result into probabilities and draw them. Each count is divided by the total sum of counts to obtain a probability, and the bar heights and vertical axis ticks are labeled by probability.

Parameters:

- `result` (`&ExecutionResult`): the execution result containing the measurement counts.
- `options` (`&ResultPlotOptions`): the ordering, color scheme and layout options.

Returns:

- `Result<String, VisualizationError>`: an SVG markup string.

Raises:

- `VisualizationError::InvalidInput`: the execution result contains no counts, the sum of counts is zero, or the plot options are inconsistent.

Example:

```rust
let svg = plot_distribution(&result, &ResultPlotOptions::default()).unwrap();
assert!(svg.contains("<svg"));
```

---

## ResultPlotOptions

Result plot options.

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

Fields:

- `figsize` (`Option<(f64, f64)>`): the figure size, in inch-like units, converted into SVG pixels at `100` pixels per unit, with lower bounds of `2` for both the width and the height. The default is `None`, and the canvas is `760 × 480` pixels.
- `color` (`Vec<String>`): the color of each dataset. The default is an empty vector, in which case a built-in qualitative palette is used.
- `number_to_keep` (`Option<usize>`): keep the `k` entries with the largest values and aggregate the rest into an entry labeled `rest`. The default is `None`, and no aggregation is performed. No aggregation is performed when `k` is `0` or not less than the number of entries; ties are ordered by the label of the key, which keeps the output consistent across platforms.
- `sort` (`String`): the ordering strategy. The default is `"asc"`. See the table below for the values.
- `target_string` (`Option<String>`): the target bitstring used by `sort = "hamming"`. The default is `None`.
- `legend` (`Option<Vec<String>>`): the legend entries, one per dataset. The default is `None`, and no legend is drawn. The result drawing entry points produce only one dataset, so when a legend is given its length must be `1`.
- `bar_labels` (`bool`): whether to draw numeric labels on top of the bars. The default is `true`.
- `title` (`Option<String>`): the plot title. The default is `None`, and no title is drawn.

### Ordering strategies

| Value | Description |
| --- | --- |
| `asc` | Ascending lexicographic order of the bitstring labels. |
| `desc` | Descending lexicographic order of the bitstring labels. |
| `value` | Ascending order of the peak value of each label across the datasets; ties in the peak are ordered by ascending label. |
| `value_desc` | Descending order of the peak value of each label across the datasets; ties in the peak are ordered by ascending label. |
| `hamming` | Ascending order of the Hamming distance to `target_string`; ties in distance are ordered by ascending label. The label `rest` is always placed last. |

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

## Example

### Ordering by Hamming distance and aggregating the remaining entries

Reusing the execution result from the previous example, order by the Hamming distance to the target bitstring, keep only the `1` entry with the largest count, and aggregate the rest into `rest`:

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

### Custom canvas size and color scheme

Reusing the execution result from the previous example, specify the canvas size, color scheme, legend and title:

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

## Related pages

- To write drawing results out to a file, see [Writing to file and output](5_render_to_file.md).
