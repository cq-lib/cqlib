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

//! Public types and options for result/statistics plots.
//!
//! # Sort Policies
//!
//! | Value | Description |
//! |-------|-------------|
//! | `asc` | Sort bitstring labels lexicographically ascending |
//! | `desc` | Sort bitstring labels lexicographically descending |
//! | `value` | Sort by peak value across datasets, ascending |
//! | `value_desc` | Sort by peak value across datasets, descending |
//! | `hamming` | Sort by Hamming distance to [`ResultPlotOptions::target_string`] |

use std::collections::BTreeMap;

pub(crate) const DEFAULT_COLORS: [&str; 8] = [
    "#4569d4", "#d64b5f", "#2f9d73", "#f0a33a", "#8a5bd7", "#2aa7b8", "#bd6b33", "#56616f",
];

/// Internal result plot family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultPlotKind {
    /// Plot raw counts.
    Histogram,
    /// Normalize counts into probabilities before plotting.
    Distribution,
}

/// Options for histogram and probability-distribution plots.
///
/// Use [`ResultPlotOptions::default`] for library defaults, then override only the fields
/// needed for a specific chart.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::visualization::ResultPlotOptions;
///
/// let options = ResultPlotOptions {
///     sort: "value_desc".to_string(),
///     number_to_keep: Some(8),
///     title: Some("Measurement counts".to_string()),
///     ..ResultPlotOptions::default()
/// };
/// assert_eq!(options.sort, "value_desc");
/// ```
#[derive(Debug, Clone)]
pub struct ResultPlotOptions {
    /// Optional figure size in inches-like units, scaled to SVG pixels.
    pub figsize: Option<(f64, f64)>,
    /// Per-dataset colors. Defaults to Cqlib's built-in qualitative palette.
    pub color: Vec<String>,
    /// Keep the largest `k` bars and aggregate the rest into a `rest` bar.
    pub number_to_keep: Option<usize>,
    /// Sort policy: `asc`, `desc`, `value`, `value_desc`, or `hamming`.
    pub sort: String,
    /// Target bitstring used by `sort = "hamming"`.
    pub target_string: Option<String>,
    /// Optional legend entries, one per dataset.
    pub legend: Option<Vec<String>>,
    /// Whether to draw numeric labels above bars.
    pub bar_labels: bool,
    /// Optional chart title.
    pub title: Option<String>,
}

impl ResultPlotOptions {
    /// Resolve user colors or fall back to Cqlib's built-in qualitative palette.
    pub(super) fn resolved_colors(&self) -> Vec<String> {
        if self.color.is_empty() {
            DEFAULT_COLORS
                .iter()
                .map(|color| color.to_string())
                .collect()
        } else {
            self.color.clone()
        }
    }
}

impl Default for ResultPlotOptions {
    fn default() -> Self {
        Self {
            figsize: None,
            color: Vec::new(),
            number_to_keep: None,
            sort: "asc".to_string(),
            target_string: None,
            legend: None,
            bar_labels: true,
            title: None,
        }
    }
}

/// Internal result series keyed by formatted bitstring labels.
///
/// Public plotting APIs accept [`crate::device::ExecutionResult`] directly; this alias is kept
/// inside the result-plot data pipeline for normalization, sorting, and rendering preparation.
pub(crate) type PlotSeries = BTreeMap<String, f64>;
