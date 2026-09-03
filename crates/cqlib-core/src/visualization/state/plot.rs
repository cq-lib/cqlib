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

//! Bloch-vector, state-city, and Pauli-vector rendering for quantum states.

use super::data::{StateVisualizationSource, basis_labels, state_to_density_matrix};
use super::math::{clamp_bloch, local_bloch_vectors, pauli_coefficients};
use super::options::StatePlotOptions;
use crate::visualization::VisualizationError;
use crate::visualization::svg::{escape_attr, escape_text, render_svg_to_file};

/// Plot a single Bloch vector as SVG.
///
/// # Arguments
///
/// * `vector` - Bloch coordinates `(x, y, z)` with components typically in `[-1, 1]`.
/// * `options` - Plot styling options such as title and figure size.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::visualization::{StatePlotOptions, plot_bloch_vector};
///
/// let svg = plot_bloch_vector([0.0, 0.0, 1.0], &StatePlotOptions::default()).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn plot_bloch_vector(
    vector: [f64; 3],
    options: &StatePlotOptions,
) -> Result<String, VisualizationError> {
    Ok(render_bloch_grid(
        &[("q0".to_string(), clamp_bloch(vector))],
        options.title.as_deref(),
        options,
    ))
}

/// Plot one reduced Bloch vector per qubit as SVG.
///
/// # Arguments
///
/// * `state` - Input core state object accepted by state visualization routines.
/// * `options` - Plot styling options such as title, colors, and bit order.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the state payload is invalid.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::{StatePlotOptions, plot_bloch_multivector};
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn plot_bloch_multivector<S: StateVisualizationSource + ?Sized>(
    state: &S,
    options: &StatePlotOptions,
) -> Result<String, VisualizationError> {
    let vectors = local_bloch_vectors(state)?;
    let ordered = order_by_display(vectors, options.reverse_bits)
        .into_iter()
        .map(|(qubit, vector)| (format!("q{qubit}"), vector))
        .collect::<Vec<_>>();
    Ok(render_bloch_grid(
        &ordered,
        options.title.as_deref(),
        options,
    ))
}

/// Plot real and imaginary density-matrix components as an SVG state-city view.
///
/// # Arguments
///
/// * `state` - Input core state object accepted by state visualization routines.
/// * `options` - Plot styling options such as title, opacity, and bit order.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the state payload is invalid.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::{StatePlotOptions, plot_state_city};
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let svg = plot_state_city(&state, &StatePlotOptions::default()).unwrap();
/// assert!(svg.contains("Re[rho]"));
/// ```
pub fn plot_state_city<S: StateVisualizationSource + ?Sized>(
    state: &S,
    options: &StatePlotOptions,
) -> Result<String, VisualizationError> {
    let (num_qubits, rho) = state_to_density_matrix(state)?;
    let dim = 1usize << num_qubits;
    let labels = basis_labels(num_qubits, options.reverse_bits);
    // Labels are display-order dependent; matrix lookup must map each displayed
    // row and column back to the stored density-matrix index independently.
    let matrix_value = |row: usize, col: usize| {
        let data_row = display_to_density_index(row, num_qubits, options.reverse_bits);
        let data_col = display_to_density_index(col, num_qubits, options.reverse_bits);
        rho[data_row * dim + data_col]
    };
    let max_abs = rho
        .iter()
        .map(|value| value.norm())
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    let (width, height) = options
        .figsize
        .map(|(w, h)| (w.max(4.0) * 100.0, h.max(3.0) * 100.0))
        .unwrap_or((980.0, 520.0));
    let mut out = svg_start(width, height);
    if let Some(title) = &options.title {
        out.push_str(&svg_text(width / 2.0, 28.0, title, 18, "middle", "#202124"));
    }
    let top = if options.title.is_some() { 52.0 } else { 28.0 };
    let has_imaginary = rho.iter().any(|value| value.im.abs() > 1e-12);
    let panel_gap = 32.0;
    let side_margin = 36.0;
    let panel_h = height - top - 34.0;
    let panel_w = if has_imaginary {
        (width - 2.0 * side_margin - panel_gap) / 2.0
    } else {
        width - 2.0 * side_margin
    };
    draw_matrix_panel(
        &mut out,
        "Re[rho]",
        side_margin,
        top,
        panel_w,
        panel_h,
        dim,
        &labels,
        |row, col| matrix_value(row, col).re,
        max_abs,
        options.alpha,
    );
    if has_imaginary {
        draw_matrix_panel(
            &mut out,
            "Im[rho]",
            side_margin + panel_w + panel_gap,
            top,
            panel_w,
            panel_h,
            dim,
            &labels,
            |row, col| matrix_value(row, col).im,
            max_abs,
            options.alpha,
        );
    } else {
        out.push_str("<!-- Im[rho] omitted: zero imaginary component -->");
    }
    out.push_str("</svg>");
    Ok(out)
}

/// Plot Pauli-basis expectation values as an SVG bar chart.
///
/// # Arguments
///
/// * `state` - Input core state object accepted by state visualization routines.
/// * `options` - Plot styling options such as title, colors, and bit order.
///
/// # Returns
///
/// SVG markup as a UTF-8 string.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] when the state payload is invalid.
///
/// # Examples
///
/// ```rust
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::{StatePlotOptions, plot_state_paulivec};
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let svg = plot_state_paulivec(&state, &StatePlotOptions::default()).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn plot_state_paulivec<S: StateVisualizationSource + ?Sized>(
    state: &S,
    options: &StatePlotOptions,
) -> Result<String, VisualizationError> {
    let (num_qubits, rho) = state_to_density_matrix(state)?;
    let coeffs = pauli_coefficients(num_qubits, &rho, options.reverse_bits)?;
    let max_abs = coeffs
        .iter()
        .map(|(_, value)| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let (width, height) = options
        .figsize
        .map(|(w, h)| (w.max(4.0) * 100.0, h.max(3.0) * 100.0))
        .unwrap_or((820.0, 480.0));
    let left = 66.0;
    let right = 28.0;
    let top = if options.title.is_some() { 58.0 } else { 34.0 };
    let bottom = 94.0;
    let plot_w = width - left - right;
    let plot_h = height - top - bottom;
    let zero_y = top + plot_h / 2.0;
    let bar_w = (plot_w / coeffs.len().max(1) as f64 * 0.72).clamp(2.0, 28.0);
    let color_pos = options
        .color
        .first()
        .map(String::as_str)
        .unwrap_or("#4569d4");
    let color_neg = options
        .color
        .get(1)
        .map(String::as_str)
        .unwrap_or("#d64b5f");

    let mut out = svg_start(width, height);
    if let Some(title) = &options.title {
        out.push_str(&svg_text(width / 2.0, 28.0, title, 18, "middle", "#202124"));
    }
    for tick in -2..=2 {
        let value = tick as f64 * max_abs / 2.0;
        let y = zero_y - (value / max_abs) * (plot_h / 2.0);
        out.push_str(&svg_line(left, y, left + plot_w, y, "#dfe3eb", 1.0));
        out.push_str(&svg_text(
            left - 8.0,
            y + 4.0,
            &format!("{value:.2}"),
            11,
            "end",
            "#5f6673",
        ));
    }
    out.push_str(&svg_line(left, top, left, top + plot_h, "#303642", 1.2));
    out.push_str(&svg_line(
        left,
        zero_y,
        left + plot_w,
        zero_y,
        "#303642",
        1.2,
    ));

    for (idx, (label, value)) in coeffs.iter().enumerate() {
        let cx = left + (idx as f64 + 0.5) * plot_w / coeffs.len() as f64;
        let h = (value.abs() / max_abs) * (plot_h / 2.0);
        let y = if *value >= 0.0 { zero_y - h } else { zero_y };
        let color = if *value >= 0.0 { color_pos } else { color_neg };
        out.push_str(&format!(
            "<rect x=\"{:.3}\" y=\"{y:.3}\" width=\"{bar_w:.3}\" height=\"{:.3}\" rx=\"2\" fill=\"{}\"/>",
            cx - bar_w / 2.0,
            h.max(1.0),
            escape_attr(color)
        ));
        if coeffs.len() <= 64 {
            let ly = top + plot_h + 18.0;
            out.push_str(&format!(
                "<text x=\"{cx:.3}\" y=\"{ly:.3}\" font-family=\"Arial, sans-serif\" font-size=\"10\" fill=\"#303642\" text-anchor=\"end\" transform=\"rotate(-55 {cx:.3} {ly:.3})\">{}</text>",
                escape_text(label)
            ));
        }
    }
    out.push_str("</svg>");
    Ok(out)
}

/// Render SVG markup to `.svg` or rasterized `.png`.
///
/// # Arguments
///
/// * `svg` - SVG markup produced by a state plotting routine.
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
/// use cqlib_core::qis::Statevector;
/// use cqlib_core::visualization::{
///     StatePlotOptions, plot_bloch_multivector, render_state_plot_to_file,
/// };
/// use num_complex::Complex64;
///
/// let state = Statevector::from_state(
///     1,
///     vec![Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
/// ).unwrap();
/// let svg = plot_bloch_multivector(&state, &StatePlotOptions::default()).unwrap();
/// render_state_plot_to_file(&svg, "bloch.svg").unwrap();
/// ```
pub fn render_state_plot_to_file(svg: &str, output_path: &str) -> Result<(), VisualizationError> {
    render_svg_to_file(svg, output_path, 1.0)
}

fn render_bloch_grid(
    items: &[(String, [f64; 3])],
    title: Option<&str>,
    options: &StatePlotOptions,
) -> String {
    let count = items.len().max(1);
    let cols = count.min(4);
    let rows = count.div_ceil(cols);
    let title_h = if title.is_some() { 42.0 } else { 14.0 };
    let (width, height) = options
        .figsize
        .map(|(w, h)| (w.max(3.0) * 100.0, h.max(3.0) * 100.0))
        .unwrap_or((
            (cols as f64 * 270.0).max(300.0),
            rows as f64 * 270.0 + title_h,
        ));
    let mut out = svg_start(width, height);
    if let Some(title) = title {
        out.push_str(&svg_text(width / 2.0, 28.0, title, 18, "middle", "#202124"));
    }
    let cell_w = width / cols as f64;
    let cell_h = (height - title_h) / rows as f64;
    for (idx, (label, vector)) in items.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let cx = col as f64 * cell_w + cell_w / 2.0;
        let cy = title_h + row as f64 * cell_h + cell_h / 2.0 + 8.0;
        draw_bloch(&mut out, cx, cy, cell_w.min(cell_h) * 0.39, label, *vector);
    }
    out.push_str("</svg>");
    out
}

/// Draw a pseudo-3D Bloch sphere and one vector arrow into the active SVG buffer.
///
/// The sphere is still pure SVG: grid curves are sampled in logical Bloch coordinates
/// and projected into two dimensions by [`project_bloch_point`].
fn draw_bloch(out: &mut String, cx: f64, cy: f64, r: f64, label: &str, vector: [f64; 3]) {
    let grad_id = format!("bloch-grad-{:.0}-{:.0}", cx, cy);
    let arrow_id = format!("bloch-arrow-{:.0}-{:.0}", cx, cy);
    let clip_id = format!("bloch-clip-{:.0}-{:.0}", cx, cy);
    out.push_str(&format!(
        "<defs><radialGradient id=\"{}\" cx=\"36%\" cy=\"24%\" r=\"78%\"><stop offset=\"0%\" stop-color=\"#ffffff\" stop-opacity=\"0.88\"/><stop offset=\"58%\" stop-color=\"#d7e7ee\" stop-opacity=\"0.50\"/><stop offset=\"100%\" stop-color=\"#b6ccd6\" stop-opacity=\"0.38\"/></radialGradient><marker id=\"{}\" markerWidth=\"7\" markerHeight=\"7\" refX=\"6.0\" refY=\"3.5\" orient=\"auto\" markerUnits=\"strokeWidth\"><path d=\"M0,0 L7,3.5 L0,7 z\" fill=\"#D62728\"/></marker><clipPath id=\"{}\"><circle cx=\"{cx:.3}\" cy=\"{cy:.3}\" r=\"{r:.3}\"/></clipPath></defs>",
        escape_attr(&grad_id),
        escape_attr(&arrow_id),
        escape_attr(&clip_id)
    ));
    out.push_str("<g data-cqlib-bloch-3d=\"true\">");
    out.push_str(&format!(
        "<circle cx=\"{cx:.3}\" cy=\"{cy:.3}\" r=\"{r:.3}\" fill=\"url(#{})\" stroke=\"#c7c7c7\" stroke-width=\"1.0\"/>",
        escape_attr(&grad_id)
    ));
    out.push_str(&format!(
        "<g clip-path=\"url(#{})\">",
        escape_attr(&clip_id)
    ));

    for z in [
        -0.83, -0.67, -0.50, -0.33, -0.17, 0.17, 0.33, 0.50, 0.67, 0.83,
    ] {
        let radius = (1.0_f64 - z * z).sqrt();
        let points = (0..=96)
            .map(|idx| {
                let theta = 2.0 * std::f64::consts::PI * idx as f64 / 96.0;
                project_bloch_point(cx, cy, r, [radius * theta.cos(), radius * theta.sin(), z])
            })
            .collect::<Vec<_>>();
        out.push_str(&svg_projected_path(&points, "#8c8c8c", 0.55, 0.36, None));
    }

    for phi in (0..180).step_by(15) {
        let phi = (phi as f64).to_radians();
        let points = (0..=96)
            .map(|idx| {
                let theta = std::f64::consts::PI * idx as f64 / 96.0;
                project_bloch_point(
                    cx,
                    cy,
                    r,
                    [
                        theta.sin() * phi.cos(),
                        theta.sin() * phi.sin(),
                        theta.cos(),
                    ],
                )
            })
            .collect::<Vec<_>>();
        out.push_str(&svg_projected_path(&points, "#8c8c8c", 0.50, 0.30, None));
    }

    draw_reference_circle(out, cx, cy, r, ReferenceCircle::Equator);
    draw_reference_circle(out, cx, cy, r, ReferenceCircle::PrimeMeridian);
    out.push_str("</g>");

    draw_bloch_axis(out, cx, cy, r, [1.0, 0.0, 0.0], "X");
    draw_bloch_axis(out, cx, cy, r, [0.0, 1.0, 0.0], "Y");
    draw_bloch_axis(out, cx, cy, r, [0.0, 0.0, 1.0], "Z");

    let (ex, ey, _) = project_bloch_point(cx, cy, r * 0.92, vector);
    let (sx, sy, _) = project_bloch_point(cx, cy, r, [0.0, 0.0, 0.0]);
    out.push_str(&format!(
        "<line x1=\"{sx:.3}\" y1=\"{sy:.3}\" x2=\"{ex:.3}\" y2=\"{ey:.3}\" stroke=\"#D62728\" stroke-width=\"2.3\" stroke-linecap=\"round\" marker-end=\"url(#{})\"/>",
        escape_attr(&arrow_id)
    ));
    out.push_str("</g>");
    out.push_str(&svg_text(cx, cy - r - 16.0, label, 18, "middle", "#000000"));
}

/// Draw one labelled Bloch axis through the sphere center.
fn draw_bloch_axis(out: &mut String, cx: f64, cy: f64, r: f64, axis: [f64; 3], label: &str) {
    let neg = [-axis[0], -axis[1], -axis[2]];
    let (x1, y1, _) = project_bloch_point(cx, cy, r, neg);
    let (x2, y2, _) = project_bloch_point(cx, cy, r, axis);
    out.push_str(&svg_line(x1, y1, x2, y2, "#444444", 1.0));
    let (tx, ty, _) = project_bloch_point(cx, cy, r * 1.08, axis);
    out.push_str(&svg_text(tx, ty + 4.0, label, 15, "middle", "#000000"));
}

#[derive(Clone, Copy)]
enum ReferenceCircle {
    Equator,
    PrimeMeridian,
}

fn draw_reference_circle(out: &mut String, cx: f64, cy: f64, r: f64, circle: ReferenceCircle) {
    let points = (0..=160)
        .map(|idx| {
            let theta = 2.0 * std::f64::consts::PI * idx as f64 / 160.0;
            let point = match circle {
                ReferenceCircle::Equator => [theta.cos(), theta.sin(), 0.0],
                ReferenceCircle::PrimeMeridian => [0.0, theta.sin(), theta.cos()],
            };
            project_bloch_point(cx, cy, r, point)
        })
        .collect::<Vec<_>>();
    match circle {
        ReferenceCircle::Equator => {
            out.push_str(&svg_projected_path(&points, "#000000", 1.10, 0.92, None));
        }
        ReferenceCircle::PrimeMeridian => {
            out.push_str(&svg_projected_path(
                &points,
                "#000000",
                1.00,
                0.88,
                Some("5 4"),
            ));
        }
    }
}

/// Project a Bloch-sphere point into SVG coordinates.
///
/// The returned depth is reserved for callers that need front/back ordering; current
/// grid rendering keeps a deterministic draw order instead.
fn project_bloch_point(cx: f64, cy: f64, r: f64, point: [f64; 3]) -> (f64, f64, f64) {
    let [x, y, z] = point;
    let sx = 0.88 * x + 0.42 * y;
    let sy = 0.08 * x - 0.40 * y - 0.90 * z;
    let depth = 0.54 * x + 0.58 * y + 0.24 * z;
    (cx + sx * r, cy + sy * r, depth)
}

/// Reorder qubit-indexed vectors for display without changing their labels.
fn order_by_display(
    mut vectors: Vec<(usize, [f64; 3])>,
    reverse_bits: bool,
) -> Vec<(usize, [f64; 3])> {
    if reverse_bits {
        vectors.reverse();
    }
    vectors
}

/// Draw one real or imaginary density-matrix panel for the state-city plot.
///
/// Cell area scales with `sqrt(abs(value) / max_abs)` so small amplitudes remain visible
/// while the sign is encoded by color.
#[allow(clippy::too_many_arguments)]
fn draw_matrix_panel<F>(
    out: &mut String,
    title: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    dim: usize,
    labels: &[String],
    value_at: F,
    max_abs: f64,
    alpha: f64,
) where
    F: Fn(usize, usize) -> f64,
{
    out.push_str(&svg_text(
        x + w / 2.0,
        y + 18.0,
        title,
        14,
        "middle",
        "#202124",
    ));
    let title_h = 32.0;
    let left_label_w = if dim <= 16 { 34.0 } else { 10.0 };
    let bottom_label_h = if dim <= 16 { 46.0 } else { 12.0 };
    let right_pad = 10.0;
    let available_w = (w - left_label_w - right_pad).max(40.0);
    let available_h = (h - title_h - bottom_label_h).max(40.0);
    let grid = available_w.min(available_h).max(40.0);
    let gx = x + left_label_w + ((available_w - grid) / 2.0).max(0.0);
    let gy = y + title_h + ((available_h - grid) / 2.0).max(0.0);
    let cell = grid / dim as f64;
    out.push_str(&format!(
        "<rect x=\"{gx:.3}\" y=\"{gy:.3}\" width=\"{grid:.3}\" height=\"{grid:.3}\" fill=\"#f7f8fb\" stroke=\"#c7ceda\"/>"
    ));
    for row in 0..dim {
        for col in 0..dim {
            let value = value_at(row, col);
            let mag = (value.abs() / max_abs).min(1.0);
            let fill = if value >= 0.0 { "#4569d4" } else { "#d64b5f" };
            let inset = cell * (1.0 - mag.sqrt()) * 0.42;
            let rx = gx + col as f64 * cell + inset;
            let ry = gy + row as f64 * cell + inset;
            let size = (cell - 2.0 * inset).max(1.0);
            out.push_str(&format!(
                "<rect x=\"{rx:.3}\" y=\"{ry:.3}\" width=\"{size:.3}\" height=\"{size:.3}\" fill=\"{fill}\" fill-opacity=\"{:.3}\"/>",
                alpha.clamp(0.05, 1.0)
            ));
        }
    }
    if dim <= 16 {
        for (idx, label) in labels.iter().enumerate() {
            let tx = gx + (idx as f64 + 0.5) * cell;
            let ty = gy + grid + 14.0;
            out.push_str(&format!(
                "<text x=\"{tx:.3}\" y=\"{ty:.3}\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#303642\" text-anchor=\"end\" transform=\"rotate(-55 {tx:.3} {ty:.3})\">{}</text>",
                escape_text(label)
            ));
            out.push_str(&svg_text(
                gx - 6.0,
                gy + (idx as f64 + 0.5) * cell + 3.0,
                label,
                12,
                "end",
                "#303642",
            ));
        }
    }
}

/// Map a displayed basis row or column back to the density-matrix storage index.
fn display_to_density_index(index: usize, num_qubits: usize, reverse_bits: bool) -> usize {
    if reverse_bits {
        bit_reverse_index(index, num_qubits)
    } else {
        index
    }
}

/// Reverse bit order within the fixed-width basis index.
fn bit_reverse_index(index: usize, num_bits: usize) -> usize {
    let mut out = 0usize;
    for bit in 0..num_bits {
        if ((index >> bit) & 1) == 1 {
            out |= 1usize << (num_bits - 1 - bit);
        }
    }
    out
}

/// Start a white-background SVG document.
fn svg_start(width: f64, height: f64) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.3} {height:.3}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>"
    )
}

/// SVG line element with escaped color attribute.
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

/// Convert projected points into an SVG path.
fn svg_projected_path(
    points: &[(f64, f64, f64)],
    color: &str,
    width: f64,
    opacity: f64,
    dasharray: Option<&str>,
) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut path = format!("M {:.3} {:.3}", points[0].0, points[0].1);
    for (x, y, _) in points.iter().skip(1) {
        path.push_str(&format!(" L {x:.3} {y:.3}"));
    }
    let dash = dasharray
        .map(|value| format!(" stroke-dasharray=\"{}\"", escape_attr(value)))
        .unwrap_or_default();
    format!(
        "<path d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width:.3}\" stroke-opacity=\"{opacity:.3}\"{dash}/>",
        escape_attr(color)
    )
}
