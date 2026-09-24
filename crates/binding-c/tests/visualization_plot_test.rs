//! Integration tests for state/result plotting and visual IR C ABI.

use binding_c::circuit::{CCircuit, circuit_cx, circuit_free, circuit_h, circuit_new};
use binding_c::cqlib_string_free;
use binding_c::device::{
    CExecutionResult, execution_result_finish, execution_result_free, execution_result_new,
};
use binding_c::qis::{
    CDensityMatrix, CStatevector, density_matrix_free, density_matrix_from_state, statevector_free,
    statevector_from_state,
};
use binding_c::visualization::*;
use num_complex::Complex64;
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

fn assert_svg(ptr: *mut c_char) {
    assert!(!ptr.is_null(), "plot should return non-NULL SVG");
    let svg = cstr_to_string(ptr);
    cqlib_string_free(ptr);
    assert!(svg.starts_with("<svg"), "plot output should be SVG markup");
}

fn bell_amplitudes() -> Vec<Complex64> {
    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
    vec![
        Complex64::new(inv_sqrt2, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(inv_sqrt2, 0.0),
    ]
}

fn bell_statevector() -> *mut CStatevector {
    let amplitudes = bell_amplitudes();
    let state = statevector_from_state(2, amplitudes.as_ptr(), amplitudes.len());
    assert!(!state.is_null());
    state
}

fn bell_density_matrix() -> *mut CDensityMatrix {
    let amplitudes = bell_amplitudes();
    let state = density_matrix_from_state(2, amplitudes.as_ptr(), amplitudes.len());
    assert!(!state.is_null());
    state
}

fn bell_result() -> *mut CExecutionResult {
    let task_id = CString::new("task-visualization").unwrap();
    let qubits = [0u32, 1];
    let result = execution_result_new(task_id.as_ptr(), qubits.as_ptr(), 2, 7, std::ptr::null());
    assert!(!result.is_null());
    let bits = [CString::new("00").unwrap(), CString::new("11").unwrap()];
    let bit_ptrs = [bits[0].as_ptr(), bits[1].as_ptr()];
    let counts = [2u64, 5];
    assert_eq!(
        execution_result_finish(result, bit_ptrs.as_ptr(), counts.as_ptr(), 2),
        0
    );
    result
}

fn bell_circuit() -> *mut CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    circuit
}

#[test]
fn test_plot_bloch_vector() {
    assert_svg(plot_bloch_vector(0.0, 0.0, 1.0, std::ptr::null()));
    // Vectors longer than the unit sphere are clamped, not rejected.
    assert_svg(plot_bloch_vector(2.0, 0.0, 0.0, std::ptr::null()));
}

#[test]
fn test_plot_state_functions_on_statevector() {
    let state = bell_statevector();
    assert_svg(plot_bloch_multivector(state, std::ptr::null()));
    assert_svg(plot_state_city(state, std::ptr::null()));
    assert_svg(plot_state_paulivec(state, std::ptr::null()));

    let svg_ptr = plot_bloch_multivector(state, std::ptr::null());
    let svg = cstr_to_string(svg_ptr);
    cqlib_string_free(svg_ptr);
    assert!(svg.contains("q0"), "Bloch grid should label q0");
    assert!(svg.contains("q1"), "Bloch grid should label q1");
    statevector_free(state);
}

#[test]
fn test_plot_state_functions_on_density_matrix() {
    let state = bell_density_matrix();
    assert_svg(plot_bloch_multivector_density_matrix(
        state,
        std::ptr::null(),
    ));
    assert_svg(plot_state_city_density_matrix(state, std::ptr::null()));
    assert_svg(plot_state_paulivec_density_matrix(state, std::ptr::null()));
    density_matrix_free(state);
}

#[test]
fn test_plot_state_options() {
    let state = bell_statevector();
    let title = CString::new("Bell state").unwrap();
    let color = CString::new("#4569d4").unwrap();
    let colors = [color.as_ptr()];
    let options = CStatePlotOptions {
        title: title.as_ptr(),
        color: colors.as_ptr(),
        color_len: 1,
        alpha: 0.75,
        reverse_bits: 1,
        has_figsize: 1,
        fig_width: 4.0,
        fig_height: 3.0,
    };
    let svg_ptr = plot_state_city(state, &options);
    assert!(!svg_ptr.is_null());
    let svg = cstr_to_string(svg_ptr);
    cqlib_string_free(svg_ptr);
    assert!(svg.contains("Bell state"), "title should be rendered");
    statevector_free(state);
}

#[test]
fn test_plot_state_null_handling() {
    assert!(plot_bloch_multivector(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_state_city(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_state_paulivec(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_bloch_multivector_density_matrix(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_state_city_density_matrix(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_state_paulivec_density_matrix(std::ptr::null(), std::ptr::null()).is_null());
}

#[test]
fn test_plot_histogram_and_distribution() {
    let result = bell_result();
    assert_svg(plot_histogram(result, std::ptr::null()));
    assert_svg(plot_distribution(result, std::ptr::null()));
    execution_result_free(result);
}

#[test]
fn test_plot_histogram_options() {
    let result = bell_result();
    let title = CString::new("Measurement counts").unwrap();
    let sort = CString::new("value_desc").unwrap();
    let legend = CString::new("sim").unwrap();
    let legends = [legend.as_ptr()];
    let options = CResultPlotOptions {
        has_figsize: 1,
        fig_width: 3.2,
        fig_height: 2.4,
        color: std::ptr::null(),
        color_len: 0,
        number_to_keep: -1,
        sort: sort.as_ptr(),
        target_string: std::ptr::null(),
        legend: legends.as_ptr(),
        legend_len: 1,
        bar_labels: 1,
        title: title.as_ptr(),
    };
    let svg_ptr = plot_distribution(result, &options);
    assert!(!svg_ptr.is_null());
    let svg = cstr_to_string(svg_ptr);
    cqlib_string_free(svg_ptr);
    assert!(svg.contains("Measurement counts"), "title should render");
    assert!(svg.contains("sim"), "legend entry should render");
    execution_result_free(result);
}

#[test]
fn test_plot_result_null_handling() {
    assert!(plot_histogram(std::ptr::null(), std::ptr::null()).is_null());
    assert!(plot_distribution(std::ptr::null(), std::ptr::null()).is_null());
}

#[test]
fn test_render_state_plot_to_file() {
    let svg_ptr = plot_bloch_vector(0.0, 0.0, 1.0, std::ptr::null());
    assert!(!svg_ptr.is_null());
    let path = std::env::temp_dir().join("cqlib_c_binding_test_state_plot.svg");
    let path_c = CString::new(path.to_string_lossy().into_owned()).unwrap();
    assert_eq!(render_state_plot_to_file(svg_ptr, path_c.as_ptr()), 0);
    assert!(path.exists(), "rendered SVG file should exist");
    let _ = std::fs::remove_file(&path);
    cqlib_string_free(svg_ptr);
}

#[test]
fn test_render_result_plot_to_file() {
    let result = bell_result();
    let svg_ptr = plot_histogram(result, std::ptr::null());
    assert!(!svg_ptr.is_null());
    let path = std::env::temp_dir().join("cqlib_c_binding_test_result_plot.svg");
    let path_c = CString::new(path.to_string_lossy().into_owned()).unwrap();
    assert_eq!(render_result_plot_to_file(svg_ptr, path_c.as_ptr()), 0);
    assert!(path.exists(), "rendered SVG file should exist");
    let _ = std::fs::remove_file(&path);
    cqlib_string_free(svg_ptr);
    execution_result_free(result);
}

#[test]
fn test_render_plot_null_handling() {
    assert_eq!(
        render_state_plot_to_file(std::ptr::null(), std::ptr::null()),
        -1
    );
    assert_eq!(
        render_result_plot_to_file(std::ptr::null(), std::ptr::null()),
        -1
    );
}

#[test]
fn test_build_visual_circuit_and_accessors() {
    let circuit = bell_circuit();
    let visual = build_visual_circuit(circuit, std::ptr::null());
    assert!(!visual.is_null());
    assert_eq!(visual_circuit_num_qubits(visual), 2);
    assert_eq!(visual_circuit_num_operations(visual), 2);
    assert_eq!(visual_circuit_num_columns(visual), 2);

    let mut qubits = [0u32; 2];
    assert_eq!(visual_circuit_qubits_len(visual), 2);
    assert_eq!(
        visual_circuit_qubits(visual, qubits.as_mut_ptr(), 2),
        0,
        "qubit copy should succeed"
    );
    assert_eq!(qubits, [0, 1]);

    let label_ptr = visual_circuit_operation_label(visual, 0);
    assert!(!label_ptr.is_null());
    let label = cstr_to_string(label_ptr);
    cqlib_string_free(label_ptr);
    assert_eq!(label, "H");

    assert_eq!(visual_circuit_operation_column(visual, 0), 0);
    assert_eq!(visual_circuit_operation_column(visual, 1), 1);
    assert_eq!(
        visual_circuit_operation_style(visual, 0),
        VISUAL_OP_STYLE_GATE
    );
    assert_eq!(
        visual_circuit_operation_style(visual, 1),
        VISUAL_OP_STYLE_CONTROLLED
    );
    assert_eq!(visual_circuit_operation_num_controls(visual, 1), 1);
    assert_eq!(visual_circuit_operation_is_span_box(visual, 0), 0);

    assert_eq!(visual_circuit_operation_lanes_len(visual, 1), 2);
    let mut lanes = [0usize; 2];
    assert_eq!(
        visual_circuit_operation_lanes(visual, 1, lanes.as_mut_ptr(), 2),
        0
    );
    assert_eq!(lanes, [0, 1]);
    assert_eq!(
        visual_circuit_operation_covered_lanes_len(visual, 1),
        2,
        "CX should cover both lanes"
    );
    assert_eq!(
        visual_circuit_operation_covered_lanes(visual, 1, lanes.as_mut_ptr(), 2),
        0
    );
    assert_eq!(lanes, [0, 1]);

    assert_eq!(visual_circuit_operation_params_len(visual, 0), 0);
    assert_eq!(
        visual_circuit_operation_params(visual, 0, std::ptr::null_mut(), 0),
        0,
        "copying zero params should succeed"
    );

    visual_circuit_free(visual);
    circuit_free(circuit);
}

#[test]
fn test_draw_from_visual() {
    let circuit = bell_circuit();
    let visual = build_visual_circuit(circuit, std::ptr::null());
    assert!(!visual.is_null());

    let text_ptr = draw_text_from_visual(visual, std::ptr::null());
    assert!(!text_ptr.is_null());
    let text = cstr_to_string(text_ptr);
    cqlib_string_free(text_ptr);
    assert!(text.contains('H'), "text drawing should contain H gate");

    let svg_ptr = draw_figure_from_visual(visual, std::ptr::null());
    assert_svg(svg_ptr);

    // Explicit drawer options also work.
    let options = TextDrawerOptionsC {
        show_params: 1,
        decompose_circuit_gates: 0,
        line_width: -1,
        initial_state: 1,
        reverse_bits: 0,
    };
    let text_ptr = draw_text_from_visual(visual, &options);
    assert!(!text_ptr.is_null());
    cqlib_string_free(text_ptr);

    let figure_options = FigureDrawerOptionsC {
        show_params: 1,
        decompose_circuit_gates: 0,
        width_per_column: 1.5,
        height_per_qubit: 1.0,
        dpi: 160,
        fold: -1,
        initial_state: 0,
        reverse_bits: 0,
    };
    let svg_ptr = draw_figure_from_visual(visual, &figure_options);
    assert_svg(svg_ptr);

    visual_circuit_free(visual);
    circuit_free(circuit);
}

#[test]
fn test_visual_ir_build_options() {
    let circuit = bell_circuit();
    let options = CVisualBuildOptions {
        decompose_circuit_gates: 0,
        reserve_full_span_for_multi_qubit: 1,
        parameter_format: CParameterFormatOptions {
            mode: PARAM_MODE_PI_FRACTION_PREFERRED,
            decimal_precision: 2,
            scientific_lower_bound: 1e-3,
            scientific_upper_bound: 1e4,
            pi_tolerance: 1e-3,
            pi_max_denominator: 16,
        },
    };
    let visual = build_visual_circuit(circuit, &options);
    assert!(!visual.is_null());
    assert_eq!(visual_circuit_num_operations(visual), 2);
    visual_circuit_free(visual);
    circuit_free(circuit);
}

#[test]
fn test_visual_ir_null_handling() {
    assert!(build_visual_circuit(std::ptr::null(), std::ptr::null()).is_null());
    assert_eq!(visual_circuit_num_qubits(std::ptr::null()), 0);
    assert_eq!(visual_circuit_num_operations(std::ptr::null()), 0);
    assert_eq!(visual_circuit_num_columns(std::ptr::null()), 0);
    assert_eq!(visual_circuit_qubits_len(std::ptr::null()), 0);
    assert_eq!(
        visual_circuit_qubits(std::ptr::null(), std::ptr::null_mut(), 0),
        -1
    );
    assert!(visual_circuit_operation_label(std::ptr::null(), 0).is_null());
    assert_eq!(
        visual_circuit_operation_column(std::ptr::null(), 0),
        usize::MAX
    );
    assert_eq!(
        visual_circuit_operation_style(std::ptr::null(), 0),
        u32::MAX
    );
    assert_eq!(
        visual_circuit_operation_control_flow_kind(std::ptr::null(), 0),
        u32::MAX
    );
    assert!(draw_text_from_visual(std::ptr::null(), std::ptr::null()).is_null());
    assert!(draw_figure_from_visual(std::ptr::null(), std::ptr::null()).is_null());
    visual_circuit_free(std::ptr::null_mut());

    // Out-of-bounds access on a valid handle must fail gracefully.
    let circuit = bell_circuit();
    let visual = build_visual_circuit(circuit, std::ptr::null());
    assert!(!visual.is_null());
    assert!(visual_circuit_operation_label(visual, 2).is_null());
    assert_eq!(visual_circuit_operation_column(visual, 2), usize::MAX);
    assert_eq!(visual_circuit_operation_lanes_len(visual, 2), 0);
    visual_circuit_free(visual);
    circuit_free(circuit);
}
