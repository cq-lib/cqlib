// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Data normalization and ordering for result/statistics plots.
//!
//! This module validates count series, optionally normalizes them into probability
//! distributions, and prepares grouped bar-chart data for SVG rendering.

use super::options::{PlotSeries, ResultPlotKind, ResultPlotOptions};
use super::plot::render_bar_svg;
use crate::device::ExecutionResult;
use crate::visualization::VisualizationError;
use std::collections::BTreeSet;

pub(crate) struct PreparedResultPlot {
    /// Display labels after applying the selected sort policy.
    pub(crate) labels: Vec<String>,
    /// Per-dataset values aligned to [`PreparedResultPlot::labels`].
    pub(crate) values: Vec<Vec<f64>>,
    /// Plot family, used by renderers to choose axis and label formatting.
    pub(crate) kind: ResultPlotKind,
}

/// Plot an execution result's raw measured counts as an SVG histogram.
///
/// # Arguments
///
/// * `result` - Execution result containing measured counts.
/// * `options` - Sorting, coloring, and layout options.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the execution result contains no counts
/// or plot options are inconsistent.
///
/// # Examples
///
/// ```no_run
/// use cqlib_core::device::ExecutionResult;
/// use cqlib_core::visualization::{ResultPlotOptions, plot_histogram};
///
/// # fn demo(result: &ExecutionResult) {
/// let svg = plot_histogram(result, &ResultPlotOptions::default()).unwrap();
/// assert!(svg.contains("<svg"));
/// # }
/// ```
pub fn plot_histogram(
    result: &ExecutionResult,
    options: &ResultPlotOptions,
) -> Result<String, VisualizationError> {
    let series = execution_result_to_plot_series(result)?;
    let prepared = prepare_result_plot(&[series], options, ResultPlotKind::Histogram)?;
    Ok(render_bar_svg(&prepared, options))
}

/// Plot an execution result's measured counts as a normalized probability distribution.
///
/// # Arguments
///
/// * `result` - Execution result containing measured counts.
/// * `options` - Sorting, coloring, and layout options.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the execution result contains no counts,
/// the measured counts sum to zero, or plot options are inconsistent.
///
/// # Examples
///
/// ```no_run
/// use cqlib_core::device::ExecutionResult;
/// use cqlib_core::visualization::{ResultPlotOptions, plot_distribution};
///
/// # fn demo(result: &ExecutionResult) {
/// let svg = plot_distribution(result, &ResultPlotOptions::default()).unwrap();
/// assert!(svg.contains("<svg"));
/// # }
/// ```
pub fn plot_distribution(
    result: &ExecutionResult,
    options: &ResultPlotOptions,
) -> Result<String, VisualizationError> {
    let series = execution_result_to_plot_series(result)?;
    let prepared = prepare_result_plot(&[series], options, ResultPlotKind::Distribution)?;
    Ok(render_bar_svg(&prepared, options))
}

/// Convert an execution result's measured counts into a plot series keyed by bitstring.
///
/// Bitstrings are formatted using [`crate::device::Outcome::to_bitstring`] with
/// [`ExecutionResult::num_qubits`], preserving Cqlib's measurement-result display
/// convention.
pub(crate) fn execution_result_to_plot_series(
    result: &ExecutionResult,
) -> Result<PlotSeries, VisualizationError> {
    if result.counts().is_empty() {
        return Err(VisualizationError::InvalidInput(
            "execution result contains no counts to plot".to_string(),
        ));
    }

    let num_qubits = result.num_qubits();
    Ok(result
        .counts()
        .iter()
        .map(|(outcome, count)| (outcome.to_bitstring(num_qubits), *count as f64))
        .collect())
}

pub(crate) fn prepare_result_plot(
    data: &[PlotSeries],
    options: &ResultPlotOptions,
    kind: ResultPlotKind,
) -> Result<PreparedResultPlot, VisualizationError> {
    validate_inputs(data, options)?;

    let mut datasets = data.to_vec();
    if kind == ResultPlotKind::Distribution {
        datasets = datasets
            .into_iter()
            .map(normalize_distribution)
            .collect::<Result<Vec<_>, _>>()?;
    }
    if let Some(k) = options.number_to_keep {
        datasets = datasets
            .into_iter()
            .map(|dataset| keep_topk_with_rest(&dataset, k))
            .collect();
    }

    let labels = sorted_labels(&datasets, options)?;
    let values = datasets
        .iter()
        .map(|dataset| {
            labels
                .iter()
                .map(|label| *dataset.get(label).unwrap_or(&0.0))
                .collect()
        })
        .collect();

    Ok(PreparedResultPlot {
        labels,
        values,
        kind,
    })
}

/// Validate cross-dataset options before normalization or rendering.
///
/// The data layer checks structural consistency here so renderers can assume at least one
/// dataset and, when present, one legend item per dataset.
fn validate_inputs(
    data: &[PlotSeries],
    options: &ResultPlotOptions,
) -> Result<(), VisualizationError> {
    if data.is_empty() {
        return Err(VisualizationError::InvalidInput(
            "plot data must contain at least one dataset".to_string(),
        ));
    }
    if let Some(legend) = &options.legend
        && legend.len() != data.len()
    {
        return Err(VisualizationError::InvalidInput(format!(
            "legend length ({}) does not match dataset count ({})",
            legend.len(),
            data.len()
        )));
    }
    Ok(())
}

/// Normalize one count-like series into probabilities.
///
/// Values are divided by their arithmetic sum. Negative entries are not rejected here because
/// result visualization accepts generic numeric series; callers that need physical probability
/// constraints should validate before calling this function.
pub(crate) fn normalize_distribution(data: PlotSeries) -> Result<PlotSeries, VisualizationError> {
    let total: f64 = data.values().sum();
    if total.abs() < 1e-12 {
        return Err(VisualizationError::InvalidInput(
            "distribution data sum is zero".to_string(),
        ));
    }
    Ok(data
        .into_iter()
        .map(|(label, value)| (label, value / total))
        .collect())
}

/// Keep the largest `k` entries by value and aggregate the remainder as `rest`.
///
/// Ties are resolved by bitstring label to keep output deterministic across platforms.
pub(crate) fn keep_topk_with_rest(data: &PlotSeries, k: usize) -> PlotSeries {
    if k == 0 || k >= data.len() {
        return data.clone();
    }

    let mut items: Vec<_> = data
        .iter()
        .map(|(label, value)| (label.clone(), *value))
        .collect();
    items.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let rest = items[..items.len() - k]
        .iter()
        .map(|(_, value)| *value)
        .sum();
    let mut out = items[items.len() - k..]
        .iter()
        .cloned()
        .collect::<PlotSeries>();
    out.insert("rest".to_string(), rest);
    out
}

/// Collect and sort display labels across all datasets.
///
/// Missing values are filled as zero during preparation, so this returns the union of all
/// labels. The special `rest` bucket is kept at the end for Hamming-distance sorting.
pub(crate) fn sorted_labels(
    datasets: &[PlotSeries],
    options: &ResultPlotOptions,
) -> Result<Vec<String>, VisualizationError> {
    let mut labels: Vec<String> = datasets
        .iter()
        .flat_map(|dataset| dataset.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    match options.sort.as_str() {
        "asc" => {}
        "desc" => labels.reverse(),
        "value" => {
            labels.sort_by(|a, b| {
                let av = peak_value(datasets, a);
                let bv = peak_value(datasets, b);
                av.total_cmp(&bv).then_with(|| a.cmp(b))
            });
        }
        "value_desc" => {
            labels.sort_by(|a, b| {
                let av = peak_value(datasets, a);
                let bv = peak_value(datasets, b);
                bv.total_cmp(&av).then_with(|| a.cmp(b))
            });
        }
        "hamming" => {
            let target = options.target_string.as_ref().ok_or_else(|| {
                VisualizationError::InvalidInput(
                    "sort='hamming' requires target_string".to_string(),
                )
            })?;
            labels.sort_by(|a, b| {
                if a == "rest" {
                    return std::cmp::Ordering::Greater;
                }
                if b == "rest" {
                    return std::cmp::Ordering::Less;
                }
                let da = hamming_distance(a, target).unwrap_or(usize::MAX);
                let db = hamming_distance(b, target).unwrap_or(usize::MAX);
                da.cmp(&db).then_with(|| a.cmp(b))
            });
            for label in labels.iter().filter(|label| label.as_str() != "rest") {
                hamming_distance(label, target)?;
            }
        }
        other => {
            return Err(VisualizationError::InvalidInput(format!(
                "sort must be one of asc, desc, value, value_desc, hamming; got '{other}'"
            )));
        }
    }

    Ok(labels)
}

/// Maximum value associated with `label` across datasets.
fn peak_value(datasets: &[PlotSeries], label: &str) -> f64 {
    datasets
        .iter()
        .map(|dataset| dataset.get(label).copied().unwrap_or(0.0))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Compute Hamming distance between equal-length bitstrings.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the two labels have different lengths.
pub(crate) fn hamming_distance(a: &str, b: &str) -> Result<usize, VisualizationError> {
    if a.len() != b.len() {
        return Err(VisualizationError::InvalidInput(
            "hamming sort needs equal-length bitstrings".to_string(),
        ));
    }
    Ok(a.chars().zip(b.chars()).filter(|(x, y)| x != y).count())
}
