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

//! C ABI for writing state and result plots to `.svg` / `.png` files.

use crate::error::CqlibError;
use cqlib_core::visualization::VisualizationError;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Shared body for the two file-rendering entries: validates inputs, then
/// delegates to a core renderer that maps failures to `IoError`.
fn render_svg_file(
    svg: *const c_char,
    output_path: *const c_char,
    render: impl FnOnce(&str, &str) -> Result<(), VisualizationError>,
) -> i32 {
    if svg.is_null() || output_path.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let svg = match unsafe { CStr::from_ptr(svg) }.to_str() {
        Ok(svg) => svg,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let path = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(path) => path,
        Err(_) => return CqlibError::ParseError as i32,
    };
    match render(svg, path) {
        Ok(()) => CqlibError::Ok as i32,
        Err(_) => CqlibError::IoError as i32,
    }
}

/// Writes state-plot SVG markup to `output_path` (`.svg` writes vector
/// output, `.png` writes raster output).
///
/// Returns 0 on success, -1 for NULL input, -4 for invalid UTF-8, or -5 on
/// I/O or rendering failure.
#[unsafe(no_mangle)]
pub extern "C" fn render_state_plot_to_file(svg: *const c_char, output_path: *const c_char) -> i32 {
    render_svg_file(
        svg,
        output_path,
        cqlib_core::visualization::render_state_plot_to_file,
    )
}

/// Writes result-plot SVG markup to `output_path` (`.svg` writes vector
/// output, `.png` writes raster output).
///
/// Returns 0 on success, -1 for NULL input, -4 for invalid UTF-8, or -5 on
/// I/O or rendering failure.
#[unsafe(no_mangle)]
pub extern "C" fn render_result_plot_to_file(
    svg: *const c_char,
    output_path: *const c_char,
) -> i32 {
    render_svg_file(
        svg,
        output_path,
        cqlib_core::visualization::render_result_plot_to_file,
    )
}
