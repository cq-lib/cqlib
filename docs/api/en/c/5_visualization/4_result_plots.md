# Measurement Result Visualization (C)

This page covers measurement-result visualization: the histogram `plot_histogram` and the probability distribution `plot_distribution`, plus the options struct `CResultPlotOptions`. Both entries accept a `CExecutionResult` handle and return a heap-allocated SVG markup string freed with `cqlib_string_free`. Error codes and the conventions for freeing strings and handles follow the [Overview](../0_overview.md).

The `CExecutionResult` handle is created with `execution_result_new` and populated with measurement counts via `execution_result_finish`; see [Result](../2_device/5_result.md) for construction.

---

## plot_histogram(result, options)

Plots an execution result's measured counts as a histogram. Counts are formatted into bitstring labels sized to the result's qubit count, serving as x-axis ticks; the y-axis starts at zero and no negative range is drawn.

```c
char *plot_histogram(const struct CExecutionResult *result,
                     const struct CResultPlotOptions *options);
```

Parameters:

- `result` (`const CExecutionResult*`): the execution result holding the measurement counts.
- `options` (`const CResultPlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

Failure scenarios:

- `result` is NULL, or the execution result carries no counts.
- `options` contains a string that is not valid UTF-8, a NULL entry inside the `color` / `legend` arrays, or inconsistent plot options.

---

## plot_distribution(result, options)

Plots an execution result's measured counts as a normalized probability distribution. Each count is divided by the total to obtain a probability; bar heights and y-axis ticks are labeled by probability.

```c
char *plot_distribution(const struct CExecutionResult *result,
                        const struct CResultPlotOptions *options);
```

Parameters:

- `result` (`const CExecutionResult*`): the execution result holding the measurement counts.
- `options` (`const CResultPlotOptions*`): plot options; NULL uses library defaults.

Returns: a heap-allocated C string (SVG markup), freed with `cqlib_string_free`; NULL on error.

Failure scenarios: same as [plot_histogram](#plothistogramresult-options), plus counts summing to zero.

---

## CResultPlotOptions

Result-plot options, filled by value; passing NULL for `options` uses library defaults. Strings inside the options must be valid UTF-8, otherwise the plot entry returns NULL.

| Field | Type | Meaning | Value behavior |
| --- | --- | --- | --- |
| `has_figsize` | `uint8_t` | Whether `fig_width` / `fig_height` carry a custom figure size. | `0` ignores width/height; non-zero enables them. |
| `fig_width` | `double` | Figure width in inch-like units, converted to SVG pixels at `100` pixels per unit. | Ignored when `has_figsize == 0`. When given, width/height are lower-bounded at `2`. |
| `fig_height` | `double` | Figure height, same units and conversion as `fig_width`. | Ignored when `has_figsize == 0`; same lower bounds as `fig_width`. |
| `color` | `const char* const*` | Per-dataset colors; length given by `color_len`. | NULL or `color_len == 0` uses the built-in qualitative palette; a NULL entry inside the array makes the entry return NULL. |
| `color_len` | `uintptr_t` | Number of entries in `color`. | `0` is treated as no colors provided. |
| `number_to_keep` | `intptr_t` | Keep the largest `k` bars and aggregate the rest into an entry labeled `rest`. | Negative disables aggregation; `k` of `0` or at least the entry count performs no aggregation. Ties are broken by key label so the output is consistent across platforms. |
| `sort` | `const char*` | Sort policy. | NULL defaults to `asc`; values listed below. Invalid UTF-8 makes the entry return NULL. |
| `target_string` | `const char*` | Target bitstring used by `sort = "hamming"`. | NULL means unset. |
| `legend` | `const char* const*` | Legend entries, one per dataset; length given by `legend_len`. | NULL draws no legend. The result-plot entries produce a single dataset, so a provided legend must have length `1`. |
| `legend_len` | `uintptr_t` | Number of entries in `legend`. | Paired with `legend`. |
| `bar_labels` | `uint8_t` | Whether to draw numeric labels above bars. | `0` off, `1` on; the library default draws them (applied only when `options` is NULL). |
| `title` | `const char*` | Chart title. | NULL draws no title. |

### Sort policies

| `sort` value | Description |
| --- | --- |
| `asc` | Ascending lexicographic order by bitstring label. |
| `desc` | Descending lexicographic order by bitstring label. |
| `value` | Ascending by each label's peak across datasets; ties broken by ascending label. |
| `value_desc` | Descending by each label's peak across datasets; ties broken by ascending label. |
| `hamming` | Ascending by Hamming distance to `target_string`; ties broken by ascending label. The `rest` label always sorts last. |

---

## Example

```c
#include <stdio.h>
#include <string.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build an execution result with measurement counts */
    const char *task_id = "task-visualization";
    uint32_t qubits[2] = {0, 1};
    struct CExecutionResult *result =
        execution_result_new(task_id, qubits, 2, 7, NULL);
    if (result == NULL) {
        return 1;
    }
    const char *bitstrings[2] = {"00", "11"};
    uint64_t counts[2] = {2, 5};
    if (execution_result_finish(result, bitstrings, counts, 2) != 0) {
        execution_result_free(result);
        return 1;
    }

    /* 2. Histogram with default options */
    char *svg = plot_histogram(result, NULL);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    /* 3. Distribution with explicit options: custom canvas and title */
    struct CResultPlotOptions options;
    memset(&options, 0, sizeof(options));
    options.has_figsize = 1;
    options.fig_width = 3.2;
    options.fig_height = 2.4;
    options.number_to_keep = -1;  /* negative disables aggregation */
    options.bar_labels = 1;
    options.title = "Measurement counts";
    svg = plot_distribution(result, &options);
    if (svg != NULL) {
        printf("%s\n", svg);
        cqlib_string_free(svg);
    }

    execution_result_free(result);
    return 0;
}
```

To sort by Hamming distance to a target bitstring and aggregate the rest, set `sort` to `"hamming"`, `target_string` to the target bitstring, and `number_to_keep` to the number of bars to keep; the remaining entries are aggregated into `rest`. To write the plots out to files, see [File Output](5_render_to_file.md).
