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

//! SVG rendering and file output for result/statistics plots.
//!
//! This module converts prepared bar-chart data into SVG markup and writes `.svg` or
//! rasterized `.png` output files.

use super::data::PreparedResultPlot;
use super::options::{ResultPlotKind, ResultPlotOptions};
use crate::visualization::VisualizationError;
use crate::visualization::svg::{escape_attr, escape_text, render_svg_to_file};

/// Gap between the y-axis tick labels' right edge and the y-axis line, in pixels.
const TICK_GAP: f64 = 10.0;
/// Gap between the y-axis tick labels' left edge and the rotated y-axis title, in pixels.
/// Mirrors matplotlib's default `labelpad`.
const TITLE_PAD: f64 = 6.0;
/// Half the visual width of the rotated y-axis title (font-size 13 / 2).
const TITLE_HALF: f64 = 6.5;
/// Safety padding from the rotated y-axis title to the canvas' left edge, in pixels.
const LEFT_PAD: f64 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LegendPlacement {
    None,
    Top,
    Right,
}

struct BarChartLayout {
    /// Outer SVG width in pixels.
    width: f64,
    /// Outer SVG height in pixels.
    height: f64,
    /// Left margin reserved for y-axis labels.
    margin_left: f64,
    /// Top margin reserved for the optional title.
    margin_top: f64,
    /// Width of the drawable plotting region.
    plot_w: f64,
    /// Height of the drawable plotting region.
    plot_h: f64,
    /// Minimum y-axis value after padding.
    min_y: f64,
    /// Maximum y-axis value after padding.
    max_y: f64,
    /// Pixel y-coordinate corresponding to value `0`.
    zero_y: f64,
    /// Estimated pixel width of the widest y-axis tick label.
    max_tick_w: f64,
    /// Width allocated to one x-axis label group.
    group_w: f64,
    /// Width allocated to one dataset bar inside a group.
    bar_w: f64,
    /// Where to place the optional legend.
    legend_placement: LegendPlacement,
    /// Whether x-axis labels need rotation.
    x_labels_rotated: bool,
}

impl BarChartLayout {
    /// Compute chart geometry from data range and user figure options.
    ///
    /// Histograms anchor the y-axis at zero. Distribution plots keep a small negative range
    /// only when negative values are present in the generic numeric input.
    fn new(plot: &PreparedResultPlot, options: &ResultPlotOptions) -> Self {
        let (width, height) = options
            .figsize
            .map(|(w, h)| (w.max(2.0) * 100.0, h.max(2.0) * 100.0))
            .unwrap_or((760.0, 480.0));
        let zeroish = plot
            .values
            .iter()
            .flat_map(|values| values.iter())
            .all(|value| value.abs() < 1e-12);
        let raw_min = plot
            .values
            .iter()
            .flat_map(|values| values.iter().copied())
            .fold(0.0_f64, f64::min);
        let raw_max = plot
            .values
            .iter()
            .flat_map(|values| values.iter().copied())
            .fold(0.0_f64, f64::max);

        let min_y = if plot.kind == ResultPlotKind::Distribution {
            (raw_min * 1.12).min(0.0)
        } else {
            0.0
        };
        let max_y = if zeroish {
            1.0
        } else {
            (raw_max * 1.12).max(1e-3)
        };
        let max_tick_w = (0..=5)
            .map(|tick| {
                let t = tick as f64 / 5.0;
                let value = min_y + (max_y - min_y).max(1e-9) * t;
                estimate_text_width(&format_tick(value, plot.kind), 11.0)
            })
            .fold(0.0_f64, f64::max);
        // The left gutter must hold, from the y-axis outward: the tick gap, the tick
        // labels, a title pad, the rotated y-axis title's width, and a left edge pad.
        // This mirrors how matplotlib's `tight_layout` reserves room for the title.
        let margin_left =
            (TICK_GAP + max_tick_w + TITLE_PAD + 2.0 * TITLE_HALF + LEFT_PAD).clamp(48.0, 110.0);

        let right_legend_margin = options.legend.as_ref().map_or(0.0, |legend| {
            legend
                .iter()
                .map(|item| legend_item_width(item, 12))
                .fold(0.0_f64, f64::max)
                + 28.0
        });
        let right_plot_w = width - margin_left - right_legend_margin.max(28.0);
        let legend_placement = if options.legend.is_some() {
            if width <= 420.0 || right_plot_w < width * 0.45 {
                LegendPlacement::Top
            } else {
                LegendPlacement::Right
            }
        } else {
            LegendPlacement::None
        };
        let margin_right = match legend_placement {
            LegendPlacement::Right => right_legend_margin.max(72.0),
            LegendPlacement::Top | LegendPlacement::None => 28.0,
        };
        let margin_top = match (options.title.is_some(), legend_placement) {
            (true, LegendPlacement::Top) => 80.0,
            (false, LegendPlacement::Top) => 50.0,
            (true, _) => 58.0,
            (false, _) => 32.0,
        };
        let max_label_w = plot
            .labels
            .iter()
            .map(|label| estimate_text_width(label, 11.0))
            .fold(0.0_f64, f64::max);
        let n_labels = plot.labels.len().max(1);
        let preliminary_plot_w = (width - margin_left - margin_right).max(40.0);
        let preliminary_group_w = preliminary_plot_w / n_labels as f64;
        let x_labels_rotated = max_label_w > preliminary_group_w * 0.75 && max_label_w > 16.0;
        let margin_bottom = if x_labels_rotated { 82.0 } else { 44.0 };
        let plot_w = preliminary_plot_w;
        let plot_h = (height - margin_top - margin_bottom).max(40.0);

        let span = (max_y - min_y).max(1e-9);
        let zero_y = margin_top + plot_h - ((0.0 - min_y) / span) * plot_h;
        let n_sets = plot.values.len().max(1);
        let group_w = plot_w / n_labels as f64;
        let bar_w = (group_w / (n_sets as f64 + 1.0)).clamp(2.0, 44.0);

        Self {
            width,
            height,
            margin_left,
            margin_top,
            plot_w,
            plot_h,
            min_y,
            max_y,
            zero_y,
            max_tick_w,
            group_w,
            bar_w,
            legend_placement,
            x_labels_rotated,
        }
    }

    /// Map a data value into SVG y-coordinate space.
    fn y_for_value(&self, value: f64) -> f64 {
        self.margin_top + self.plot_h - ((value - self.min_y) / self.span()) * self.plot_h
    }

    /// Numerically safe y-axis span used by coordinate conversion.
    fn span(&self) -> f64 {
        (self.max_y - self.min_y).max(1e-9)
    }
}

/// Write result-plot SVG markup to an output file.
///
/// # Arguments
///
/// * `svg` - SVG markup produced by [`crate::visualization::plot_histogram`] or
///   [`crate::visualization::plot_distribution`].
/// * `output_path` - Target file path. `.svg` writes vector output, `.png` writes raster output.
///
/// # Errors
///
/// Returns [`VisualizationError::Io`] or [`VisualizationError::SvgRenderFailed`] when file
/// output or PNG rasterization fails.
///
/// # Examples
///
/// ```no_run
/// use cqlib_core::visualization::{
///     ResultPlotOptions, plot_histogram, render_result_plot_to_file,
/// };
///
/// # fn demo(result: &cqlib_core::device::ExecutionResult) {
/// let svg = plot_histogram(result, &ResultPlotOptions::default()).unwrap();
/// render_result_plot_to_file(&svg, "hist.svg").unwrap();
/// # }
/// ```
pub fn render_result_plot_to_file(svg: &str, output_path: &str) -> Result<(), VisualizationError> {
    render_svg_to_file(svg, output_path, 1.0)
}

/// Render prepared grouped bar-chart data as SVG.
///
/// This function assumes labels and per-dataset values are already aligned by
/// [`crate::visualization::result::data::prepare_result_plot`].
pub(crate) fn render_bar_svg(plot: &PreparedResultPlot, options: &ResultPlotOptions) -> String {
    let layout = BarChartLayout::new(plot, options);
    let colors = options.resolved_colors();

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.3} {:.3}\">",
        layout.width, layout.height, layout.width, layout.height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(title) = &options.title {
        out.push_str(&svg_text(
            layout.width / 2.0,
            28.0,
            title,
            18,
            "middle",
            "#202124",
        ));
    }

    for tick in 0..=5 {
        let t = tick as f64 / 5.0;
        let y = layout.margin_top + layout.plot_h * (1.0 - t);
        let value = layout.min_y + layout.span() * t;
        out.push_str(&svg_line(
            layout.margin_left,
            y,
            layout.margin_left + layout.plot_w,
            y,
            "#dfe3eb",
            1.0,
        ));
        out.push_str(&svg_text(
            layout.margin_left - 10.0,
            y + 4.0,
            &format_tick(value, plot.kind),
            11,
            "end",
            "#5f6673",
        ));
    }
    out.push_str(&svg_line(
        layout.margin_left,
        layout.margin_top,
        layout.margin_left,
        layout.margin_top + layout.plot_h,
        "#303642",
        1.2,
    ));
    out.push_str(&svg_line(
        layout.margin_left,
        layout.zero_y,
        layout.margin_left + layout.plot_w,
        layout.zero_y,
        "#303642",
        1.2,
    ));

    for (set_idx, set_values) in plot.values.iter().enumerate() {
        for (label_idx, value) in set_values.iter().enumerate() {
            if value.abs() < 1e-12 {
                continue;
            }
            let center =
                layout.margin_left + label_idx as f64 * layout.group_w + layout.group_w / 2.0;
            let x = center
                + (set_idx as f64 - (plot.values.len() as f64 - 1.0) / 2.0) * layout.bar_w
                - layout.bar_w / 2.0;
            let y_value = layout.y_for_value(*value);
            let y = layout.zero_y.min(y_value);
            let h = (layout.zero_y - y_value).abs().max(1.0);
            let color = &colors[set_idx % colors.len()];
            out.push_str(&format!(
                "<rect x=\"{x:.3}\" y=\"{y:.3}\" width=\"{:.3}\" height=\"{h:.3}\" rx=\"2\" fill=\"{}\"/>",
                layout.bar_w,
                escape_attr(color)
            ));
            if options.bar_labels {
                let label_y = if *value >= 0.0 { y - 5.0 } else { y + h + 13.0 };
                out.push_str(&svg_text(
                    x + layout.bar_w / 2.0,
                    label_y,
                    &format_bar_value(*value, plot.kind),
                    10,
                    "middle",
                    "#303642",
                ));
            }
        }
    }

    for (idx, label) in plot.labels.iter().enumerate() {
        let x = layout.margin_left + idx as f64 * layout.group_w + layout.group_w / 2.0;
        let y = layout.margin_top + layout.plot_h + 18.0;
        if layout.x_labels_rotated {
            out.push_str(&format!(
                "<text x=\"{x:.3}\" y=\"{y:.3}\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#303642\" text-anchor=\"end\" transform=\"rotate(-55 {x:.3} {y:.3})\">{}</text>",
                escape_text(label)
            ));
        } else {
            out.push_str(&svg_text(x, y, label, 11, "middle", "#303642"));
        }
    }
    let y_label = if plot.kind == ResultPlotKind::Histogram {
        "Count"
    } else {
        "Probability"
    };
    // Place the rotated y-axis title just to the left of the tick labels, leaving
    // a fixed pad so the title never overlaps the tick numbers regardless of how
    // wide they get. This mirrors matplotlib positioning the axis label after the
    // tick labels when computing the layout.
    let y_title_x = layout.margin_left - TICK_GAP - layout.max_tick_w - TITLE_PAD - TITLE_HALF;
    let y_title_y = layout.margin_top + layout.plot_h / 2.0;
    out.push_str(&format!(
        "<text x=\"{:.3}\" y=\"{:.3}\" font-family=\"Arial, sans-serif\" font-size=\"13\" fill=\"#303642\" text-anchor=\"middle\" transform=\"rotate(-90 {:.3} {:.3})\">{}</text>",
        y_title_x,
        y_title_y,
        y_title_x,
        y_title_y,
        y_label
    ));

    if let Some(legend) = &options.legend {
        match layout.legend_placement {
            LegendPlacement::Top => {
                let total_w = legend_total_width(legend, 12);
                let mut x = ((layout.width - total_w) / 2.0).max(layout.margin_left);
                let y = if options.title.is_some() { 56.0 } else { 28.0 };
                for (idx, item) in legend.iter().enumerate() {
                    out.push_str(&legend_item_svg(x, y, item, &colors[idx % colors.len()]));
                    x += legend_item_width(item, 12) + 18.0;
                }
            }
            LegendPlacement::Right => {
                let x = layout.margin_left + layout.plot_w + 18.0;
                let mut y = layout.margin_top + 10.0;
                for (idx, item) in legend.iter().enumerate() {
                    out.push_str(&legend_item_svg(x, y, item, &colors[idx % colors.len()]));
                    y += 22.0;
                }
            }
            LegendPlacement::None => {}
        }
    }
    out.push_str("</svg>");
    out
}

/// Format y-axis tick values for the selected plot family.
fn format_tick(value: f64, kind: ResultPlotKind) -> String {
    if kind == ResultPlotKind::Histogram && value.abs() >= 10.0 {
        format!("{value:.0}")
    } else if value.abs() >= 1.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
    }
}

/// Format bar-top labels for the selected plot family.
fn format_bar_value(value: f64, kind: ResultPlotKind) -> String {
    if kind == ResultPlotKind::Histogram {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
    }
}

fn legend_item_svg(x: f64, y: f64, item: &str, color: &str) -> String {
    format!(
        "<rect x=\"{x:.3}\" y=\"{:.3}\" width=\"13\" height=\"13\" rx=\"2\" fill=\"{}\"/>{}",
        y - 10.0,
        escape_attr(color),
        svg_text(x + 20.0, y + 1.0, item, 12, "start", "#303642")
    )
}

fn legend_item_width(item: &str, font_size: u8) -> f64 {
    20.0 + estimate_text_width(item, font_size as f64)
}

fn legend_total_width(legend: &[String], font_size: u8) -> f64 {
    let item_width: f64 = legend
        .iter()
        .map(|item| legend_item_width(item, font_size))
        .sum();
    item_width + 18.0 * legend.len().saturating_sub(1) as f64
}

fn estimate_text_width(value: &str, font_size: f64) -> f64 {
    value
        .chars()
        .map(|ch| {
            let em = match ch {
                ' ' => 0.33,
                'i' | 'j' | 'l' | 'I' | '|' | '!' | '.' | ',' | ':' | ';' | '\'' => 0.28,
                '(' | ')' | '[' | ']' | '{' | '}' => 0.35,
                '*' | '+' | '-' | '/' | '=' => 0.55,
                '0'..='9' => 0.58,
                'A'..='Z' => match ch {
                    'M' | 'W' => 0.90,
                    _ => 0.68,
                },
                'a'..='z' => match ch {
                    'm' | 'w' => 0.82,
                    'f' | 'r' | 't' => 0.42,
                    _ => 0.58,
                },
                '_' => 0.52,
                _ if ch.is_ascii() => 0.62,
                _ => 0.82,
            };
            em * font_size
        })
        .sum()
}

/// SVG line element with attribute escaping for color.
fn svg_line(x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) -> String {
    format!(
        "<line x1=\"{x1:.3}\" y1=\"{y1:.3}\" x2=\"{x2:.3}\" y2=\"{y2:.3}\" stroke=\"{}\" stroke-width=\"{width:.3}\"/>",
        escape_attr(color)
    )
}

/// SVG text element with escaped text and attributes.
fn svg_text(x: f64, y: f64, text: &str, size: u8, anchor: &str, color: &str) -> String {
    format!(
        "<text x=\"{x:.3}\" y=\"{y:.3}\" font-family=\"Arial, sans-serif\" font-size=\"{size}\" fill=\"{}\" text-anchor=\"{}\">{}</text>",
        escape_attr(color),
        escape_attr(anchor),
        escape_text(text)
    )
}
