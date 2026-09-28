//! Integration tests for visualization module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new};
use binding_c::cqlib_string_free;
use binding_c::visualization::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn build_bell_circuit() -> *mut binding_c::circuit::CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    circuit
}

#[test]
fn test_circuit_to_text() {
    let circuit = build_bell_circuit();
    let text_ptr = circuit_to_text(circuit, std::ptr::null());
    assert!(!text_ptr.is_null());
    let text = cstr_to_string(text_ptr);
    cqlib_string_free(text_ptr);
    assert!(text.contains("H"), "text drawing should contain H gate");
    // CX renders as a control dot with an "X" target label.
    assert!(
        text.contains('X'),
        "text drawing should contain X target of CX"
    );
    circuit_free(circuit);
}

#[test]
fn test_circuit_to_text_with_options() {
    let circuit = build_bell_circuit();
    let options = TextDrawerOptionsC {
        show_params: 1,
        decompose_circuit_gates: 0,
        line_width: -1,
        initial_state: 1,
        reverse_bits: 0,
    };
    let text_ptr = circuit_to_text(circuit, &options);
    assert!(!text_ptr.is_null());
    cqlib_string_free(text_ptr);
    circuit_free(circuit);
}

#[test]
fn test_circuit_to_figure_svg() {
    let circuit = build_bell_circuit();
    let svg_ptr = circuit_to_figure(circuit, std::ptr::null());
    assert!(!svg_ptr.is_null());
    let svg = cstr_to_string(svg_ptr);
    cqlib_string_free(svg_ptr);
    assert!(svg.contains("<svg"), "figure output should be SVG markup");
    circuit_free(circuit);
}

#[test]
fn test_circuit_to_figure_with_options() {
    let circuit = build_bell_circuit();
    let options = FigureDrawerOptionsC {
        show_params: 1,
        decompose_circuit_gates: 0,
        width_per_column: 1.5,
        height_per_qubit: 1.0,
        dpi: 160,
        fold: -1,
        initial_state: 0,
        reverse_bits: 0,
    };
    let svg_ptr = circuit_to_figure(circuit, &options);
    assert!(!svg_ptr.is_null());
    cqlib_string_free(svg_ptr);
    circuit_free(circuit);
}

#[test]
fn test_render_figure_to_file_svg() {
    let circuit = build_bell_circuit();
    let path = std::env::temp_dir().join("cqlib_c_binding_test_circuit.svg");
    let path_str = path.to_string_lossy().into_owned();
    let path_c = CString::new(path_str).unwrap();

    let ret = render_figure_to_file(circuit, path_c.as_ptr(), std::ptr::null());
    assert_eq!(ret, 0, "render to svg file should succeed");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
    circuit_free(circuit);
}

#[test]
fn test_visualization_null_handling() {
    assert!(circuit_to_text(std::ptr::null(), std::ptr::null()).is_null());
    assert!(circuit_to_figure(std::ptr::null(), std::ptr::null()).is_null());
    assert_eq!(
        render_figure_to_file(std::ptr::null(), std::ptr::null(), std::ptr::null()),
        -1
    );
}
