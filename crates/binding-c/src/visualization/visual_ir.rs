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

//! C ABI for the visualization IR (build a `VisualCircuit`, inspect it, and
//! re-draw it with text or figure backends).
//!
//! The C side mirrors the two-step Rust pipeline: build the backend-agnostic
//! IR with `build_visual_circuit`, then render it with `draw_text_from_visual`
//! or `draw_figure_from_visual`. Individual operations are exposed through
//! per-field getters (label, lanes, params, style) following the accessor
//! style of `src/circuit/dag.rs`.

use crate::circuit::CCircuit;
use crate::error::CqlibError;
use crate::visualization::circuit_render::{FigureDrawerOptionsC, TextDrawerOptionsC};
use cqlib_core::visualization::{
    FigureDrawerOptions, ParameterDisplayMode, ParameterFormatOptions, TextDrawerOptions,
    VisualBuildOptions, VisualCircuit, VisualControlFlowKind, VisualOpStyle, VisualOperation,
};
use std::ffi::CString;
use std::os::raw::c_char;

// ===== build options =====

/// `CParameterFormatOptions::mode`: prefer numeric display when evaluable.
pub const PARAM_MODE_NUMERIC: u32 = 0;
/// `CParameterFormatOptions::mode`: prefer symbolic expression display.
pub const PARAM_MODE_SYMBOLIC: u32 = 1;
/// `CParameterFormatOptions::mode`: symbolic expression with numeric value.
pub const PARAM_MODE_SYMBOLIC_WITH_VALUE: u32 = 2;
/// `CParameterFormatOptions::mode`: prefer `k*pi/n` fraction display.
pub const PARAM_MODE_PI_FRACTION_PREFERRED: u32 = 3;

/// C-side parameter label formatting options. Mirrors the fields of core
/// `ParameterFormatOptions`.
#[repr(C)]
pub struct CParameterFormatOptions {
    /// One of the `PARAM_MODE_*` constants; unknown values fall back to numeric.
    pub mode: u32,
    /// Decimal precision for fixed-point/scientific formatting.
    pub decimal_precision: usize,
    /// Values in `(0, scientific_lower_bound)` use scientific notation.
    pub scientific_lower_bound: f64,
    /// Values `>= scientific_upper_bound` use scientific notation.
    pub scientific_upper_bound: f64,
    /// Tolerance when matching `value / pi` to rational fractions.
    pub pi_tolerance: f64,
    /// Maximum denominator used for pi fraction matching.
    pub pi_max_denominator: i64,
}

/// C-side visual IR build options. Mirrors the fields of core
/// `VisualBuildOptions`; a NULL pointer maps to library defaults.
#[repr(C)]
pub struct CVisualBuildOptions {
    /// Whether to decompose circuit-gates before layout (0/1).
    pub decompose_circuit_gates: u8,
    /// Whether to reserve the full lane span for multi-qubit operations (0/1).
    pub reserve_full_span_for_multi_qubit: u8,
    /// Parameter label formatting options.
    pub parameter_format: CParameterFormatOptions,
}

fn build_options_from_c(options: *const CVisualBuildOptions) -> VisualBuildOptions {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return VisualBuildOptions::default();
    };
    let format = &options.parameter_format;
    let parameter_format = ParameterFormatOptions {
        mode: match format.mode {
            PARAM_MODE_SYMBOLIC => ParameterDisplayMode::Symbolic,
            PARAM_MODE_SYMBOLIC_WITH_VALUE => ParameterDisplayMode::SymbolicWithValue,
            PARAM_MODE_PI_FRACTION_PREFERRED => ParameterDisplayMode::PiFractionPreferred,
            _ => ParameterDisplayMode::Numeric,
        },
        decimal_precision: format.decimal_precision,
        scientific_lower_bound: format.scientific_lower_bound,
        scientific_upper_bound: format.scientific_upper_bound,
        pi_tolerance: format.pi_tolerance,
        pi_max_denominator: format.pi_max_denominator,
    };
    VisualBuildOptions {
        decompose_circuit_gates: options.decompose_circuit_gates != 0,
        reserve_full_span_for_multi_qubit: options.reserve_full_span_for_multi_qubit != 0,
        parameter_format,
    }
}

// ===== visual circuit handle =====

/// Opaque handle around a built [`VisualCircuit`] IR.
pub struct CVisualCircuit {
    pub inner: VisualCircuit,
}

/// Builds the backend-agnostic visualization IR from a circuit.
///
/// `options` may be NULL to use library defaults. Returns a newly allocated
/// `CVisualCircuit*` (free with `visual_circuit_free`), or NULL on NULL
/// input or build failure.
#[unsafe(no_mangle)]
pub extern "C" fn build_visual_circuit(
    circuit: *const CCircuit,
    options: *const CVisualBuildOptions,
) -> *mut CVisualCircuit {
    if circuit.is_null() {
        return std::ptr::null_mut();
    }
    let options = build_options_from_c(options);
    match cqlib_core::visualization::build_visual_circuit(unsafe { &(*circuit).inner }, &options) {
        Ok(inner) => Box::into_raw(Box::new(CVisualCircuit { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a visual-circuit handle. NULL is allowed.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_free(ptr: *mut CVisualCircuit) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Returns the number of qubit lanes, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_num_qubits(ptr: *const CVisualCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_qubits() }
}

/// Returns the number of operations, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_num_operations(ptr: *const CVisualCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.operations.len() }
}

/// Returns the number of occupied columns, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_num_columns(ptr: *const CVisualCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.num_columns }
}

/// Returns the number of qubit ids, or 0 for NULL.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_qubits_len(ptr: *const CVisualCircuit) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).inner.qubits.len() }
}

/// Copies the qubit ids in display order into `buffer` (two-step pattern;
/// pair with `visual_circuit_qubits_len`). Returns 0 on success, -1 for NULL
/// input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_qubits(
    ptr: *const CVisualCircuit,
    buffer: *mut u32,
    len: usize,
) -> i32 {
    if ptr.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let qubits = unsafe { &(*ptr).inner.qubits };
    if len < qubits.len() {
        return CqlibError::InvalidParam as i32;
    }
    if qubits.is_empty() {
        return CqlibError::Ok as i32;
    }
    if buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, qubits.len()) };
    for (slot, qubit) in slice.iter_mut().zip(qubits.iter()) {
        *slot = qubit.id();
    }
    CqlibError::Ok as i32
}

/// Borrows the operation at `index`, or None for NULL / out-of-bounds.
fn operation_at<'a>(ptr: *const CVisualCircuit, index: usize) -> Option<&'a VisualOperation> {
    let handle = (unsafe { ptr.as_ref() })?;
    handle.inner.operations.get(index)
}

/// Returns the primary display label of operation `index` as a
/// heap-allocated C string. Caller must free with `cqlib_string_free`.
/// Returns NULL on NULL input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_label(
    ptr: *const CVisualCircuit,
    index: usize,
) -> *mut c_char {
    match operation_at(ptr, index) {
        Some(op) => match CString::new(op.label.as_str()) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Returns the time column of operation `index`, or `usize::MAX` on NULL
/// input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_column(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(usize::MAX, |op| op.column)
}

/// Returns the number of logical columns reserved by operation `index`, or
/// `usize::MAX` on NULL input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_span_cols(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(usize::MAX, |op| op.span_cols)
}

/// Returns whether operation `index` is drawn as a span box: 1 or 0, or -1
/// on NULL input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_is_span_box(
    ptr: *const CVisualCircuit,
    index: usize,
) -> i32 {
    operation_at(ptr, index).map_or(-1, |op| op.span_box as i32)
}

/// Returns the number of operand lanes of operation `index`, or 0 on NULL
/// input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_lanes_len(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(0, |op| op.lanes.len())
}

/// Copies the operand lanes of operation `index` into `buffer` (two-step
/// pattern; pair with `visual_circuit_operation_lanes_len`). Returns 0 on
/// success, -1 for NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_lanes(
    ptr: *const CVisualCircuit,
    index: usize,
    buffer: *mut usize,
    len: usize,
) -> i32 {
    let Some(op) = operation_at(ptr, index) else {
        return CqlibError::NullPtr as i32;
    };
    let lanes = &op.lanes;
    if len < lanes.len() {
        return CqlibError::InvalidParam as i32;
    }
    if lanes.is_empty() {
        return CqlibError::Ok as i32;
    }
    if buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, lanes.len()) };
    slice.copy_from_slice(lanes);
    CqlibError::Ok as i32
}

/// Returns the number of lanes reserved by operation `index` in its column,
/// or 0 on NULL input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_covered_lanes_len(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(0, |op| op.covered_lanes.len())
}

/// Copies the lanes reserved by operation `index` into `buffer` (two-step
/// pattern; pair with `visual_circuit_operation_covered_lanes_len`).
/// Returns 0 on success, -1 for NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_covered_lanes(
    ptr: *const CVisualCircuit,
    index: usize,
    buffer: *mut usize,
    len: usize,
) -> i32 {
    let Some(op) = operation_at(ptr, index) else {
        return CqlibError::NullPtr as i32;
    };
    let lanes = &op.covered_lanes;
    if len < lanes.len() {
        return CqlibError::InvalidParam as i32;
    }
    if lanes.is_empty() {
        return CqlibError::Ok as i32;
    }
    if buffer.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buffer, lanes.len()) };
    slice.copy_from_slice(lanes);
    CqlibError::Ok as i32
}

/// Returns the number of formatted parameter labels of operation `index`,
/// or 0 on NULL input or out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_params_len(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(0, |op| op.params.len())
}

/// Copies the formatted parameter labels of operation `index` into `out`
/// (two-step pattern; pair with
/// `visual_circuit_operation_params_len`). Each written element is a freshly
/// allocated C string the caller frees with `cqlib_string_free`. Returns 0
/// on success, -1 for NULL input, -8 when `len` is too small.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_params(
    ptr: *const CVisualCircuit,
    index: usize,
    out: *mut *mut c_char,
    len: usize,
) -> i32 {
    let Some(op) = operation_at(ptr, index) else {
        return CqlibError::NullPtr as i32;
    };
    let params = &op.params;
    if len < params.len() {
        return CqlibError::InvalidParam as i32;
    }
    if params.is_empty() {
        return CqlibError::Ok as i32;
    }
    if out.is_null() {
        return CqlibError::NullPtr as i32;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(out, params.len()) };
    for (slot, param) in slice.iter_mut().zip(params.iter()) {
        match CString::new(param.as_str()) {
            Ok(text) => *slot = text.into_raw(),
            Err(_) => return CqlibError::CircuitError as i32,
        }
    }
    CqlibError::Ok as i32
}

// ===== operation style tags =====

/// `visual_circuit_operation_style` tag: generic gate-like box.
pub const VISUAL_OP_STYLE_GATE: u32 = 0;
/// `visual_circuit_operation_style` tag: controlled operation; query the
/// control count with `visual_circuit_operation_num_controls`.
pub const VISUAL_OP_STYLE_CONTROLLED: u32 = 1;
/// `visual_circuit_operation_style` tag: controlled-Z marker.
pub const VISUAL_OP_STYLE_CZ: u32 = 2;
/// `visual_circuit_operation_style` tag: swap marker.
pub const VISUAL_OP_STYLE_SWAP: u32 = 3;
/// `visual_circuit_operation_style` tag: barrier marker.
pub const VISUAL_OP_STYLE_BARRIER: u32 = 4;
/// `visual_circuit_operation_style` tag: measurement marker.
pub const VISUAL_OP_STYLE_MEASURE: u32 = 5;
/// `visual_circuit_operation_style` tag: reset marker.
pub const VISUAL_OP_STYLE_RESET: u32 = 6;
/// `visual_circuit_operation_style` tag: delay marker.
pub const VISUAL_OP_STYLE_DELAY: u32 = 7;
/// `visual_circuit_operation_style` tag: control-flow marker; query the
/// kind with `visual_circuit_operation_control_flow_kind`.
pub const VISUAL_OP_STYLE_CONTROL_FLOW: u32 = 8;

/// Returns the drawing style of operation `index` as one of the
/// `VISUAL_OP_STYLE_*` constants, or `u32::MAX` on NULL input or
/// out-of-bounds.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_style(ptr: *const CVisualCircuit, index: usize) -> u32 {
    let Some(op) = operation_at(ptr, index) else {
        return u32::MAX;
    };
    match op.style {
        VisualOpStyle::Gate => VISUAL_OP_STYLE_GATE,
        VisualOpStyle::Controlled { .. } => VISUAL_OP_STYLE_CONTROLLED,
        VisualOpStyle::Cz => VISUAL_OP_STYLE_CZ,
        VisualOpStyle::Swap => VISUAL_OP_STYLE_SWAP,
        VisualOpStyle::Barrier => VISUAL_OP_STYLE_BARRIER,
        VisualOpStyle::Measure => VISUAL_OP_STYLE_MEASURE,
        VisualOpStyle::Reset => VISUAL_OP_STYLE_RESET,
        VisualOpStyle::Delay => VISUAL_OP_STYLE_DELAY,
        VisualOpStyle::ControlFlow { .. } => VISUAL_OP_STYLE_CONTROL_FLOW,
    }
}

/// Returns the number of control qubits of operation `index`; only
/// meaningful when the style is `VISUAL_OP_STYLE_CONTROLLED`. Returns 0 on
/// NULL input, out-of-bounds, or non-controlled styles.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_num_controls(
    ptr: *const CVisualCircuit,
    index: usize,
) -> usize {
    operation_at(ptr, index).map_or(0, |op| match op.style {
        VisualOpStyle::Controlled { num_controls } => num_controls,
        _ => 0,
    })
}

// ===== control-flow kind tags =====

/// `visual_circuit_operation_control_flow_kind` tag: source IfElse block.
pub const VISUAL_CF_KIND_IF_ELSE_BLOCK: u32 = 0;
/// `visual_circuit_operation_control_flow_kind` tag: source while-loop block.
pub const VISUAL_CF_KIND_WHILE_BLOCK: u32 = 1;
/// `visual_circuit_operation_control_flow_kind` tag: source for-loop block.
pub const VISUAL_CF_KIND_FOR_BLOCK: u32 = 2;
/// `visual_circuit_operation_control_flow_kind` tag: source switch block.
pub const VISUAL_CF_KIND_SWITCH_BLOCK: u32 = 3;
/// `visual_circuit_operation_control_flow_kind` tag: structured break.
pub const VISUAL_CF_KIND_BREAK: u32 = 4;
/// `visual_circuit_operation_control_flow_kind` tag: structured continue.
pub const VISUAL_CF_KIND_CONTINUE: u32 = 5;
/// `visual_circuit_operation_control_flow_kind` tag: flattened if start.
pub const VISUAL_CF_KIND_IF_START: u32 = 6;
/// `visual_circuit_operation_control_flow_kind` tag: flattened else start.
pub const VISUAL_CF_KIND_ELSE_START: u32 = 7;
/// `visual_circuit_operation_control_flow_kind` tag: flattened while start.
pub const VISUAL_CF_KIND_WHILE_START: u32 = 8;
/// `visual_circuit_operation_control_flow_kind` tag: flattened for start.
pub const VISUAL_CF_KIND_FOR_START: u32 = 9;
/// `visual_circuit_operation_control_flow_kind` tag: flattened switch start.
pub const VISUAL_CF_KIND_SWITCH_START: u32 = 10;
/// `visual_circuit_operation_control_flow_kind` tag: flattened case start.
pub const VISUAL_CF_KIND_CASE_START: u32 = 11;
/// `visual_circuit_operation_control_flow_kind` tag: flattened default start.
pub const VISUAL_CF_KIND_DEFAULT_START: u32 = 12;
/// `visual_circuit_operation_control_flow_kind` tag: flattened block end.
pub const VISUAL_CF_KIND_END: u32 = 13;

/// Returns the control-flow kind of operation `index` as one of the
/// `VISUAL_CF_KIND_*` constants; only meaningful when the style is
/// `VISUAL_OP_STYLE_CONTROL_FLOW`. Returns `u32::MAX` on NULL input,
/// out-of-bounds, or non-control-flow styles.
#[unsafe(no_mangle)]
pub extern "C" fn visual_circuit_operation_control_flow_kind(
    ptr: *const CVisualCircuit,
    index: usize,
) -> u32 {
    let Some(op) = operation_at(ptr, index) else {
        return u32::MAX;
    };
    match &op.style {
        VisualOpStyle::ControlFlow { kind } => match kind {
            VisualControlFlowKind::IfElseBlock { .. } => VISUAL_CF_KIND_IF_ELSE_BLOCK,
            VisualControlFlowKind::WhileBlock { .. } => VISUAL_CF_KIND_WHILE_BLOCK,
            VisualControlFlowKind::ForBlock { .. } => VISUAL_CF_KIND_FOR_BLOCK,
            VisualControlFlowKind::SwitchBlock { .. } => VISUAL_CF_KIND_SWITCH_BLOCK,
            VisualControlFlowKind::Break => VISUAL_CF_KIND_BREAK,
            VisualControlFlowKind::Continue => VISUAL_CF_KIND_CONTINUE,
            VisualControlFlowKind::IfStart => VISUAL_CF_KIND_IF_START,
            VisualControlFlowKind::ElseStart => VISUAL_CF_KIND_ELSE_START,
            VisualControlFlowKind::WhileStart => VISUAL_CF_KIND_WHILE_START,
            VisualControlFlowKind::ForStart => VISUAL_CF_KIND_FOR_START,
            VisualControlFlowKind::SwitchStart => VISUAL_CF_KIND_SWITCH_START,
            VisualControlFlowKind::CaseStart => VISUAL_CF_KIND_CASE_START,
            VisualControlFlowKind::DefaultStart => VISUAL_CF_KIND_DEFAULT_START,
            VisualControlFlowKind::End => VISUAL_CF_KIND_END,
        },
        _ => u32::MAX,
    }
}

// ===== drawing from pre-built IR =====

/// Converts C-side text-drawer options; NULL maps to library defaults.
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

/// Converts C-side figure-drawer options; NULL maps to library defaults.
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

/// Draws pre-built visualization IR as UTF-8 text (box-drawing characters).
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string; caller must free with `cqlib_string_free`. Returns NULL on
/// error.
#[unsafe(no_mangle)]
pub extern "C" fn draw_text_from_visual(
    visual: *const CVisualCircuit,
    options: *const TextDrawerOptionsC,
) -> *mut c_char {
    if visual.is_null() {
        return std::ptr::null_mut();
    }
    let options = text_options_from_c(options);
    let visual = unsafe { &(*visual).inner };
    match cqlib_core::visualization::draw_text_from_visual(visual, &options) {
        Ok(text) => match CString::new(text) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Draws pre-built visualization IR as an SVG figure.
///
/// `options` may be NULL to use library defaults. Returns a heap-allocated
/// C string containing SVG markup; caller must free with
/// `cqlib_string_free`. Returns NULL on error. The core signature keeps an
/// output-path parameter for compatibility; it is ignored and therefore
/// not exposed here.
#[unsafe(no_mangle)]
pub extern "C" fn draw_figure_from_visual(
    visual: *const CVisualCircuit,
    options: *const FigureDrawerOptionsC,
) -> *mut c_char {
    if visual.is_null() {
        return std::ptr::null_mut();
    }
    let options = figure_options_from_c(options);
    let visual = unsafe { &(*visual).inner };
    match CString::new(cqlib_core::visualization::draw_figure_from_visual(
        visual, &options, None,
    )) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// Note: freeing a label or param string with `cqlib_string_free` matches the
// `CString::into_raw` allocation used above; no separate free entry is needed.
