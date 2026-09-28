# Measurement Result Visualization

`cqlib.visualization`

The measurement result visualization entry points render the measurement counts of an execution result into SVG plots. The two share the same set of options: `plot_histogram` uses the raw counts as bar heights, and `plot_distribution` normalizes the counts into probabilities before drawing.

## Import

```python
from cqlib.visualization import plot_distribution, plot_histogram
```

---

## Input and result types

The input of both entry points is `cqlib.device.ExecutionResult`. A completed result object can be constructed directly from counts:

```python
from cqlib.device import ExecutionResult

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1],
    7,
    2,
    {"00": 2, "11": 5},
)
```

The count keys are bitstrings whose length must match the number of qubits, otherwise an error is raised when the result object is constructed. The bitstring labels in the plot are the count keys in the result object.

---

## Common parameters

Apart from the first positional parameter, the two entry points take exactly the same parameters, all of which are keyword arguments:

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `figsize` | [`FigureSize`](5_render_to_file.md) `\| None` | `None` | The plot size in inches; when not passed, the default size of that plot family is used. |
| `color` | `list[str] \| None` | `None` | One color per dataset; when not passed, a built-in palette is used. |
| `number_to_keep` | `int \| None` | `None` | Keep only the k bars with the largest values and aggregate the remaining counts into a single bar labeled `rest`; a value of 0, or a value not less than the number of bars, performs no aggregation. |
| `sort` | `str` | `"asc"` | The bar sort strategy; for the values see [Sort strategies](#sort-strategies). |
| `target_string` | `str \| None` | `None` | The target bitstring used by `sort="hamming"`. |
| `legend` | `list[str] \| None` | `None` | Legend entries, one per dataset. Rendering from the counts of an execution result involves only one dataset, so one entry is sufficient; an error is raised when the number of entries does not match the number of datasets. |
| `bar_labels` | `bool` | `True` | Whether to label the bars with their values. |
| `title` | `str \| None` | `None` | The chart title; when not passed, no title is drawn. |
| `output_path` | `str \| None` | `None` | The output path; when given, the same SVG is written to that path, see [Rendering output and file writing](5_render_to_file.md). |

---

## Functions

### plot_histogram(result, *, figsize=None, color=None, number_to_keep=None, sort="asc", target_string=None, legend=None, bar_labels=True, title=None, output_path=None)

Render the measurement counts of an execution result as a counts histogram, with counts on the vertical axis.

Parameters:

- `result` (`ExecutionResult`): the execution result to render. When the result has no counts this is reported as a `ValueError`.
- For the remaining parameters see [Common parameters](#common-parameters).

Returns:

- `_InlineSvg`: SVG text, whose type is a subclass of `str`; it is displayed inline when used as the last expression of a cell in a notebook frontend.

Raises:

- `TypeError`: `result` is not an `ExecutionResult` object.
- `ValueError`: the result has no counts, or a parameter value is invalid, for example `sort` is not in the allowed value set, `sort="hamming"` is used without `target_string`, or the number of `legend` entries does not match the number of datasets.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

### plot_distribution(result, *, figsize=None, color=None, number_to_keep=None, sort="asc", target_string=None, legend=None, bar_labels=True, title=None, output_path=None)

Normalize the measurement counts of an execution result into probabilities and render them as a probability distribution plot, with probability on the vertical axis.

Parameters:

- `result` (`ExecutionResult`): the execution result to render. When the result has no counts, or the counts sum to zero, this is reported as a `ValueError`.
- For the remaining parameters see [Common parameters](#common-parameters). Normalization is completed before the `rest` bar is aggregated, so the heights of the bars in the plot sum to 1.

Returns:

- `_InlineSvg`: SVG text.

Raises:

- `TypeError`: `result` is not an `ExecutionResult` object.
- `ValueError`: the result has no counts, the counts sum to zero, or a parameter value is invalid, for example `sort` is not in the allowed value set, `sort="hamming"` is used without `target_string`, or the number of `legend` entries does not match the number of datasets.
- `IOError`: writing to the path pointed at by `output_path` fails.

---

## Sort strategies

The values of `sort` and their meanings:

| Value | Description |
| --- | --- |
| `asc` | Sort by bitstring label in ascending order; the default value. |
| `desc` | Sort by bitstring label in descending order. |
| `value` | Sort by bar value in ascending order. |
| `value_desc` | Sort by bar value in descending order. |
| `hamming` | Sort by ascending [Hamming distance](0_overview.md) to `target_string`; the aggregated `rest` bar is placed last. |

The `hamming` sort requires all bitstrings to have the same length as `target_string` and requires `target_string` to be provided; when this is not satisfied it is reported as a `ValueError`. The other values do not use `target_string`.

---

## Examples

### 1. Counts histogram

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_histogram

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1],
    7,
    2,
    {"00": 2, "11": 5},
)

svg = plot_histogram(result, output_path="histogram.svg")
assert "<svg" in svg
assert "Count" in svg
```

### 2. Sort by Hamming distance and aggregate the tail counts

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_histogram

result = ExecutionResult.from_counts(
    "vis-test",
    [0, 1, 2],
    10,
    3,
    {"100": 4, "101": 3, "010": 2, "001": 1},
)

svg = plot_histogram(
    result,
    sort="hamming",
    target_string="100",
    number_to_keep=3,
    color=["#123456"],
    legend=["sim"],
    bar_labels=False,
    title="Hamming order",
    output_path="histogram_hamming.svg",
)
assert "Hamming order" in svg
assert "#123456" in svg
assert ">sim</text>" in svg
assert ">rest</text>" in svg
```

### 3. Probability distribution plot

```python
from cqlib.device import ExecutionResult
from cqlib.visualization import plot_distribution

result = ExecutionResult.from_counts(
    "vis-test",
    [0],
    100,
    1,
    {"0": 25, "1": 75},
)

svg = plot_distribution(
    result,
    figsize=(3.2, 2.4),
    color=["#0f766e"],
    legend=["probability"],
    bar_labels=False,
    title="Distribution options",
    output_path="distribution.svg",
)
assert 'width="320"' in svg
assert 'height="240"' in svg
assert "#0f766e" in svg
assert ">probability</text>" in svg
```
