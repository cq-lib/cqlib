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

//! C ABI for measurement-result plotting (histograms, distributions).
//!
//! Core result entries accept an `ExecutionResult` handle directly, so the C
//! side takes a `CExecutionResult*` (see `src/device/result.rs` for handle
//! construction). Entries return heap-allocated SVG markup the caller frees
//! with `cqlib_string_free`.

use crate::device::CExecutionResult;
use crate::error::CqlibError;
use cqlib_core::visualization::ResultPlotOptions;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// C-side result-plot options. Mirrors the fields of core
/// `ResultPlotOptions`; NULL optional fields use library defaults.
#[repr(C)]
pub struct CResultPlotOptions {
    /// Non-zero when `fig_width` / `fig_height` carry a custom figure size.
    pub has_figsize: u8,
    /// Custom figure width in inch-like units; ignored when `has_figsize == 0`.
    pub fig_width: f64,
    /// Custom figure height in inch-like units; ignored when `has_figsize == 0`.
    pub fig_height: f64,
    /// Per-dataset colors; NULL or `color_len == 0` uses the built-in palette.
    pub color: *const *const c_char,
    /// Number of entries in `color`.
    pub color_len: usize,
    /// Keep the largest k bars and aggregate the rest into `rest`;
    /// negative disables aggregation.
    pub number_to_keep: isize,
    /// Sort policy: `asc`, `desc`, `value`, `value_desc`, or `hamming`;
    /// NULL defaults to `asc`.
    pub sort: *const c_char,
    /// Target bitstring used by `sort = "hamming"`; NULL means unset.
    pub target_string: *const c_char,
    /// Optional legend entries; NULL draws no legend.
    pub legend: *const *const c_char,
    /// Number of entries in `legend`.
    pub legend_len: usize,
    /// Whether to draw numeric labels above bars (0/1).
    pub bar_labels: u8,
    /// Optional chart title; NULL draws no title.
    pub title: *const c_char,
}

/// Reads a UTF-8 C string; maps invalid UTF-8 to `ParseError`.
unsafe fn utf8_str<'a>(ptr: *const c_char) -> Result<&'a str, CqlibError> {
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| CqlibError::ParseError)
}

/// Converts C-side result-plot options; a NULL pointer maps to defaults.
fn result_options_from_c(
    options: *const CResultPlotOptions,
) -> Result<ResultPlotOptions, CqlibError> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(ResultPlotOptions::default());
    };
    let mut result = ResultPlotOptions::default();
    if options.has_figsize != 0 {
        result.figsize = Some((options.fig_width, options.fig_height));
    }
    if options.color_len > 0 {
        if options.color.is_null() {
            return Err(CqlibError::NullPtr);
        }
        let mut colors = Vec::with_capacity(options.color_len);
        for idx in 0..options.color_len {
            let entry = unsafe { *options.color.add(idx) };
            if entry.is_null() {
                return Err(CqlibError::NullPtr);
            }
            colors.push(unsafe { utf8_str(entry) }?.to_string());
        }
        result.color = colors;
    }
    if options.number_to_keep >= 0 {
        result.number_to_keep = Some(options.number_to_keep as usize);
    }
    if !options.sort.is_null() {
        result.sort = unsafe { utf8_str(options.sort) }?.to_string();
    }
    if !options.target_string.is_null() {
        result.target_string = Some(unsafe { utf8_str(options.target_string) }?.to_string());
    }
    if !options.legend.is_null() {
        let mut legend = Vec::with_capacity(options.legend_len);
        for idx in 0..options.legend_len {
            let entry = unsafe { *options.legend.add(idx) };
            if entry.is_null() {
                return Err(CqlibError::NullPtr);
            }
            legend.push(unsafe { utf8_str(entry) }?.to_string());
        }
        result.legend = Some(legend);
    }
    result.bar_labels = options.bar_labels != 0;
    if !options.title.is_null() {
        result.title = Some(unsafe { utf8_str(options.title) }?.to_string());
    }
    Ok(result)
}

/// Packs an SVG string into a heap-allocated C string; NULL on any error.
fn svg_into_c_string<E>(svg: Result<String, E>) -> *mut c_char {
    match svg {
        Ok(svg) => match CString::new(svg) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Plots an execution result's measured counts as an SVG histogram.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error (including a result with no
/// counts).
#[unsafe(no_mangle)]
pub extern "C" fn plot_histogram(
    result: *const CExecutionResult,
    options: *const CResultPlotOptions,
) -> *mut c_char {
    if result.is_null() {
        return std::ptr::null_mut();
    }
    let options = match result_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let result = unsafe { &(*result).inner };
    svg_into_c_string(cqlib_core::visualization::plot_histogram(result, &options))
}

/// Plots an execution result's measured counts as a normalized SVG
/// probability distribution.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error (including a result with no
/// counts or counts summing to zero).
#[unsafe(no_mangle)]
pub extern "C" fn plot_distribution(
    result: *const CExecutionResult,
    options: *const CResultPlotOptions,
) -> *mut c_char {
    if result.is_null() {
        return std::ptr::null_mut();
    }
    let options = match result_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let result = unsafe { &(*result).inner };
    svg_into_c_string(cqlib_core::visualization::plot_distribution(
        result, &options,
    ))
}
