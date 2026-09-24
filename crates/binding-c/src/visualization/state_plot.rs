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

//! C ABI for quantum-state plotting (Bloch vectors, state city, Pauli basis).
//!
//! Core plotting entries are generic over `StateVisualizationSource`; on the
//! C side the two implementors (`Statevector`, `DensityMatrix`) are distinct
//! opaque handles, so each entry is exposed twice: the base name accepts a
//! `CStatevector` and the `_density_matrix` variant accepts a
//! `CDensityMatrix`. All entries return heap-allocated SVG markup the caller
//! frees with `cqlib_string_free`.

use crate::error::CqlibError;
use crate::qis::{CDensityMatrix, CStatevector};
use cqlib_core::visualization::StatePlotOptions;
use cqlib_core::visualization::{
    local_bloch_vectors as core_local_bloch_vectors,
    state_to_density_matrix as core_state_to_density_matrix,
};
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// C-side state-plot options. Mirrors the fields of core `StatePlotOptions`;
/// NULL optional fields use library defaults.
#[repr(C)]
pub struct CStatePlotOptions {
    /// Optional chart title; NULL draws no title.
    pub title: *const c_char,
    /// Optional color array; NULL or `color_len == 0` uses plot-family defaults.
    pub color: *const *const c_char,
    /// Number of entries in `color`.
    pub color_len: usize,
    /// Fill opacity for state-city bars.
    pub alpha: f64,
    /// Whether to reverse displayed computational-basis bit order (0/1).
    pub reverse_bits: u8,
    /// Non-zero when `fig_width` / `fig_height` carry a custom figure size.
    pub has_figsize: u8,
    /// Custom figure width in inch-like units; ignored when `has_figsize == 0`.
    pub fig_width: f64,
    /// Custom figure height in inch-like units; ignored when `has_figsize == 0`.
    pub fig_height: f64,
}

/// Reads a UTF-8 C string; maps invalid UTF-8 to `ParseError`.
unsafe fn utf8_str<'a>(ptr: *const c_char) -> Result<&'a str, CqlibError> {
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| CqlibError::ParseError)
}

/// Converts C-side state-plot options; a NULL pointer maps to library defaults.
fn state_options_from_c(options: *const CStatePlotOptions) -> Result<StatePlotOptions, CqlibError> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(StatePlotOptions::default());
    };
    let mut result = StatePlotOptions::default();
    if !options.title.is_null() {
        result.title = Some(unsafe { utf8_str(options.title) }?.to_string());
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
    result.alpha = options.alpha;
    result.reverse_bits = options.reverse_bits != 0;
    if options.has_figsize != 0 {
        result.figsize = Some((options.fig_width, options.fig_height));
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

/// Plots a single Bloch vector as SVG.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_bloch_vector(
    x: f64,
    y: f64,
    z: f64,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    svg_into_c_string(cqlib_core::visualization::plot_bloch_vector(
        [x, y, z],
        &options,
    ))
}

/// Plots one reduced Bloch vector per qubit of a statevector as SVG.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_bloch_multivector(
    state: *const CStatevector,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_bloch_multivector(
        state, &options,
    ))
}

/// `plot_bloch_multivector` for a density-matrix input.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_bloch_multivector_density_matrix(
    state: *const CDensityMatrix,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_bloch_multivector(
        state, &options,
    ))
}

/// Plots the real and imaginary density-matrix panels of a statevector.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_state_city(
    state: *const CStatevector,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_state_city(state, &options))
}

/// `plot_state_city` for a density-matrix input.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_state_city_density_matrix(
    state: *const CDensityMatrix,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_state_city(state, &options))
}

/// Plots Pauli-basis expectation values of a statevector as an SVG bar chart.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_state_paulivec(
    state: *const CStatevector,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_state_paulivec(
        state, &options,
    ))
}

/// `plot_state_paulivec` for a density-matrix input.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error.
#[unsafe(no_mangle)]
pub extern "C" fn plot_state_paulivec_density_matrix(
    state: *const CDensityMatrix,
    options: *const CStatePlotOptions,
) -> *mut c_char {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let options = match state_options_from_c(options) {
        Ok(options) => options,
        Err(_) => return std::ptr::null_mut(),
    };
    let state = unsafe { &(*state).inner };
    svg_into_c_string(cqlib_core::visualization::plot_state_paulivec(
        state, &options,
    ))
}

// =====  State inspection helpers (two-step array outputs)  =====

/// Returns `3 * num_qubits`, the flattened buffer length required by
/// `local_bloch_vectors`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn local_bloch_vectors_len(state: *const CStatevector) -> usize {
    if state.is_null() {
        return 0;
    }
    3 * unsafe { (*state).inner.num_qubits }
}

/// Compute one reduced single-qubit Bloch vector per qubit of a statevector.
///
/// `out` receives `3 * num_qubits` doubles laid out flat as
/// `[x0, y0, z0, x1, y1, z1, ...]`, where entry `q * 3 .. q * 3 + 3` holds the
/// `<X>`, `<Y>`, `<Z>` expectation values of qubit `q`'s reduced state.
/// `len` must equal `local_bloch_vectors_len(state)`. Returns 0 on success,
/// -1 on NULL pointers, or -8 when `len` does not match or the state payload
/// is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn local_bloch_vectors(
    state: *const CStatevector,
    out: *mut f64,
    len: usize,
) -> i32 {
    if state.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let state = unsafe { &(*state).inner };
    let vectors = match core_local_bloch_vectors(state) {
        Ok(vectors) => vectors,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    if len != vectors.len() * 3 {
        return CqlibError::InvalidParam as i32;
    }
    for (qubit, vector) in &vectors {
        unsafe {
            *out.add(3 * qubit) = vector[0];
            *out.add(3 * qubit + 1) = vector[1];
            *out.add(3 * qubit + 2) = vector[2];
        }
    }
    CqlibError::Ok as i32
}

/// Returns `4^num_qubits`, the flattened buffer length required by
/// `state_to_density_matrix`, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn state_to_density_matrix_len(state: *const CStatevector) -> usize {
    if state.is_null() {
        return 0;
    }
    let num_qubits = unsafe { (*state).inner.num_qubits };
    if 2 * num_qubits >= usize::BITS as usize {
        return 0;
    }
    1usize << (2 * num_qubits)
}

/// Convert a statevector into its density matrix.
///
/// `out` receives `4^num_qubits` `Complex64` values laid out row-major as an
/// `N x N` matrix with `N = 2^num_qubits`; element `(row, col)` is stored at
/// index `row * N + col`. `len` must equal
/// `state_to_density_matrix_len(state)`. Returns 0 on success, -1 on NULL
/// pointers, or -8 when `len` does not match or the state payload is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn state_to_density_matrix(
    state: *const CStatevector,
    out: *mut Complex64,
    len: usize,
) -> i32 {
    if state.is_null() || out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let state = unsafe { &(*state).inner };
    let (_, rho) = match core_state_to_density_matrix(state) {
        Ok(result) => result,
        Err(_) => return CqlibError::InvalidParam as i32,
    };
    if rho.len() != len {
        return CqlibError::InvalidParam as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(rho.as_ptr(), out, len);
    }
    CqlibError::Ok as i32
}
