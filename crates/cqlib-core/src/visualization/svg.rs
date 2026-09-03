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

//! Shared SVG output helpers for visualization backends.
//!
//! Visualization routines in this module generate SVG markup first. This file
//! centralizes writing SVG text to disk and rasterizing SVG into PNG so circuit,
//! result, and state plots use the same output contract and error handling.

use crate::visualization::VisualizationError;
use std::fs;
use std::path::Path;

/// Writes SVG markup to an output path.
///
/// `.png` paths are rasterized through `resvg`; all other extensions write the
/// SVG text unchanged. `png_scale` is applied only to PNG outputs and is useful
/// for DPI-style resolution control while keeping the SVG coordinate system stable.
///
/// # Arguments
///
/// * `svg` - Complete SVG document markup.
/// * `output_path` - Target file path. `.png` selects raster output.
/// * `png_scale` - Positive finite scale factor for PNG rasterization.
///
/// # Errors
///
/// Returns [`VisualizationError::InvalidInput`] for invalid scale factors,
/// [`VisualizationError::Io`] for file output failures, and
/// [`VisualizationError::SvgRenderFailed`] when SVG parsing or rasterization fails.
pub(crate) fn render_svg_to_file(
    svg: &str,
    output_path: &str,
    png_scale: f64,
) -> Result<(), VisualizationError> {
    let out_path = Path::new(output_path);
    match out_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => rasterize_svg_to_png_data(svg.as_bytes(), out_path, png_scale),
        _ => fs::write(out_path, svg).map_err(VisualizationError::Io),
    }
}

/// Rasterizes SVG bytes into a PNG file using `resvg`.
///
/// The renderer loads system fonts so text output remains close to direct SVG
/// preview. Scaling changes only the target pixmap dimensions; SVG layout remains
/// in the original coordinate system.
fn rasterize_svg_to_png_data(
    svg_data: &[u8],
    png_path: &Path,
    scale: f64,
) -> Result<(), VisualizationError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(VisualizationError::InvalidInput(format!(
            "png scale must be positive and finite; got {scale}"
        )));
    }

    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(svg_data, &options)
        .map_err(|e| VisualizationError::SvgRenderFailed(e.to_string()))?;
    let size = tree.size().to_int_size();
    let width = scaled_extent(size.width(), scale)?;
    let height = scaled_extent(size.height(), scale)?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        VisualizationError::SvgRenderFailed("failed to allocate pixmap".to_string())
    })?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(png_path)
        .map_err(|e| VisualizationError::SvgRenderFailed(e.to_string()))?;
    Ok(())
}

/// Converts an SVG integer extent into a scaled PNG extent.
///
/// The conversion is checked so very large DPI values fail with a clear error
/// instead of silently overflowing the pixmap dimensions.
fn scaled_extent(value: u32, scale: f64) -> Result<u32, VisualizationError> {
    let scaled = (value as f64 * scale).ceil().max(1.0);
    if scaled > u32::MAX as f64 {
        return Err(VisualizationError::InvalidInput(format!(
            "scaled png extent {scaled} exceeds u32::MAX"
        )));
    }
    Ok(scaled as u32)
}

/// Escapes text-node content for XML/SVG.
pub(crate) fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes XML attribute content for SVG.
pub(crate) fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}
