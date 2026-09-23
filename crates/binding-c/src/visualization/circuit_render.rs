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

//! C ABI entry points for circuit rendering and matrix export.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use cqlib_core::visualization::{FigureDrawerOptions, TextDrawerOptions};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// C-side text drawing options. Mirrors the subset of `TextDrawerOptions`
/// that makes sense from C; unset fields use library defaults.
#[repr(C)]
pub struct TextDrawerOptionsC {
    /// Whether to append parameter text to gate labels (0/1).
    pub show_params: u8,
    /// Whether to decompose circuit-gates before drawing (0/1).
    pub decompose_circuit_gates: u8,
    /// Max width per wrapped segment; `<= 0` disables wrapping.
    pub line_width: isize,
    /// Whether to show `|0>` at the start of each qubit wire (0/1).
    pub initial_state: u8,
    /// Whether to reverse the displayed qubit order (0/1).
    pub reverse_bits: u8,
}

/// C-side figure drawing options. Mirrors the subset of
/// `FigureDrawerOptions` that makes sense from C.
#[repr(C)]
pub struct FigureDrawerOptionsC {
    /// Whether to append parameter text to gate labels (0/1).
    pub show_params: u8,
    /// Whether to decompose circuit-gates before drawing (0/1).
    pub decompose_circuit_gates: u8,
    /// Figure width scale per logical column.
    pub width_per_column: f64,
    /// Figure height scale per qubit.
    pub height_per_qubit: f64,
    /// Rasterization DPI when exporting PNG (must be > 0).
    pub dpi: u32,
    /// Maximum columns per row; `<= 0` disables folding.
    pub fold: i32,
    /// Whether to show `|0>` in qubit labels (0/1).
    pub initial_state: u8,
    /// Whether to reverse display order of qubits (0/1).
    pub reverse_bits: u8,
}

fn text_options_from_c(options: *const TextDrawerOptionsC) -> TextDrawerOptions {
    let defaults = TextDrawerOptions::default();
    let Some(options) = (unsafe { options.as_ref() }) else {
        return defaults;
    };
    TextDrawerOptions {
        cell_width: defaults.cell_width,
        show_params: options.show_params != 0,
        decompose_circuit_gates: options.decompose_circuit_gates != 0,
        line_width: options.line_width,
        initial_state: options.initial_state != 0,
        reverse_bits: options.reverse_bits != 0,
    }
}

fn figure_options_from_c(options: *const FigureDrawerOptionsC) -> FigureDrawerOptions {
    let mut result = FigureDrawerOptions::default();
    let Some(options) = (unsafe { options.as_ref() }) else {
        return result;
    };
    result.show_params = options.show_params != 0;
    result.decompose_circuit_gates = options.decompose_circuit_gates != 0;
    if options.width_per_column > 0.0 {
        result.width_per_column = options.width_per_column;
    }
    if options.height_per_qubit > 0.0 {
        result.height_per_qubit = options.height_per_qubit;
    }
    result.dpi = options.dpi;
    result.fold = if options.fold > 0 { options.fold } else { -1 };
    result.initial_state = options.initial_state != 0;
    result.reverse_bits = options.reverse_bits != 0;
    result
}

/// Draws the circuit as UTF-8 text (box-drawing characters).
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string; caller must free with `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_text(
    circuit: *const CCircuit,
    options: *const TextDrawerOptionsC,
) -> *mut c_char {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    let opts = text_options_from_c(options);
    match cqlib_core::visualization::circuit_to_text(circuit, &opts) {
        Ok(text) => match CString::new(text) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Draws the circuit as an SVG figure.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn circuit_to_figure(
    circuit: *const CCircuit,
    options: *const FigureDrawerOptionsC,
) -> *mut c_char {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let circuit = unsafe { &(*circuit).inner };
    let opts = figure_options_from_c(options);
    match cqlib_core::visualization::circuit_to_figure(circuit, &opts) {
        Ok(svg) => match CString::new(svg) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Renders the circuit to an output file (`.svg` or `.png`).
///
/// Returns 0 on success, -5 on I/O or rendering failure, or -1 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn render_figure_to_file(
    circuit: *const CCircuit,
    output_path: *const c_char,
    options: *const FigureDrawerOptionsC,
) -> i32 {
    if circuit.is_null() || output_path.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let path = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return CqlibError::ParseError as i32,
    };
    let circuit = unsafe { &(*circuit).inner };
    let opts = figure_options_from_c(options);
    match cqlib_core::visualization::render_figure_to_file(circuit, path, &opts) {
        Ok(()) => 0,
        Err(_) => CqlibError::IoError as i32,
    }
}

// Note: unitary-matrix export (`circuit_to_matrix(&circuit, qubits_order)`)
// is already covered by `circuit_to_matrix_len` / `circuit_to_matrix`
// in the circuit module (src/circuit/properties.rs), so no duplicate C
// symbols are defined here.
