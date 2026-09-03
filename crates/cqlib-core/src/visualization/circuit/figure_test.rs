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

use super::*;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::{
    Circuit, ClassicalExpr, ClassicalType, Parameter, Qubit, StandardGate, UnitaryGate,
};
use crate::visualization::circuit::{GateStyle, ParameterDisplayMode, ParameterFormatOptions};
use crate::visualization::test_utils::assert_svg_visual_match;
use std::collections::HashMap;
use std::f64::consts::PI;

fn q(index: usize) -> Qubit {
    let id = u32::try_from(index).expect("qubit index should fit in u32");
    Qubit::new(id)
}

fn measure_all(circuit: &mut Circuit) {
    for idx in 0..circuit.width() {
        circuit.measure(q(idx)).unwrap();
    }
}

#[test]
fn figure_drawer_options_default_uses_cqlib_style() {
    let options = FigureDrawerOptions::default();

    assert!(options.show_params);
    assert_eq!(options.style, FigureDrawStyle::Cqlib);
    assert_eq!(options.dpi, 160);
    assert!(options.gate_styles.is_empty());
}

fn assert_visual_match(circuit: &Circuit, options: FigureDrawerOptions, filename: &str) {
    assert_svg_visual_match(&["circuit", "figure"], filename, |output_path| {
        render_figure_to_file(circuit, &output_path.to_string_lossy(), &options)
    });
}

#[derive(Debug, Clone, Copy)]
struct SvgRect {
    x: f64,
    width: f64,
}

fn attr_f64(tag: &str, attr: &str) -> f64 {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle).expect("attribute should exist") + needle.len();
    let end = tag[start..]
        .find('"')
        .map(|offset| start + offset)
        .expect("attribute should be closed");
    tag[start..end]
        .parse()
        .expect("attribute should be numeric")
}

fn tag_before(svg: &str, tag_name: &str, text_needle: &str) -> String {
    let text_idx = svg.find(text_needle).expect("text needle should exist");
    let tag_idx = svg[..text_idx]
        .rfind(tag_name)
        .expect("tag should exist before text");
    let tag_end = svg[tag_idx..]
        .find('>')
        .map(|offset| tag_idx + offset + 1)
        .expect("tag should be closed");
    svg[tag_idx..tag_end].to_string()
}

fn rect_before_text(svg: &str, text_needle: &str) -> SvgRect {
    let tag = tag_before(svg, "<rect ", text_needle);
    SvgRect {
        x: attr_f64(&tag, "x"),
        width: attr_f64(&tag, "width"),
    }
}

fn circle_cx_before_text(svg: &str, text_needle: &str) -> f64 {
    let tag = tag_before(svg, "<circle ", text_needle);
    attr_f64(&tag, "cx")
}

fn first_font_size_for_text(svg: &str, text_needle: &str) -> f64 {
    let text_idx = svg.find(text_needle).expect("text needle should exist");
    let tag_idx = svg[..text_idx]
        .rfind("<text ")
        .expect("text tag should exist");
    let tag_end = svg[tag_idx..]
        .find('>')
        .map(|offset| tag_idx + offset + 1)
        .expect("text tag should be closed");
    attr_f64(&svg[tag_idx..tag_end], "font-size")
}

fn make_bell() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    measure_all(&mut circuit);
    circuit
}

fn make_all_gate() -> Circuit {
    let mut c = Circuit::new(6);
    let q0 = q(0);
    let q1 = q(1);
    let q2 = q(2);
    let q3 = q(3);
    let q4 = q(4);
    let q5 = q(5);

    c.h(q0).unwrap();
    c.h(q1).unwrap();
    c.x(q0).unwrap();
    c.x(q2).unwrap();
    c.y(q1).unwrap();
    c.y(q3).unwrap();
    c.z(q2).unwrap();
    c.z(q4).unwrap();

    c.rx(q0, PI / 3.0).unwrap();
    c.rx(q1, PI / 4.0).unwrap();
    c.ry(q2, PI / 2.0).unwrap();
    c.ry(q3, PI / 5.0).unwrap();
    c.rz(q4, PI / 3.0).unwrap();
    c.rz(q5, PI / 4.0).unwrap();
    c.rxy(q0, PI / 6.0, PI / 3.0).unwrap();
    c.rxx(q1, q2, PI / 7.0).unwrap();
    c.ryy(q0, q3, PI / 6.0).unwrap();
    c.rzx(q3, q1, PI / 7.0).unwrap();
    c.rzz(q1, q2, PI / 6.0).unwrap();

    c.crx(q0, q4, PI / 3.0).unwrap();
    c.crx(q1, q5, PI / 4.0).unwrap();
    c.cry(q2, q3, 0.12 * PI).unwrap();
    c.cry(q0, q2, PI / 5.0).unwrap();
    c.crz(q3, q5, PI / 3.0).unwrap();
    c.crz(q1, q4, PI / 4.0).unwrap();

    c.x2p(q0).unwrap();
    c.x2p(q1).unwrap();
    c.x2m(q2).unwrap();
    c.x2m(q3).unwrap();
    c.y2p(q4).unwrap();
    c.y2p(q5).unwrap();
    c.y2m(q0).unwrap();
    c.y2m(q1).unwrap();
    c.xy(q2, PI / 8.0).unwrap();
    c.xy(q3, PI / 9.0).unwrap();
    c.xy2p(q4, PI / 10.0).unwrap();
    c.xy2m(q5, PI / 11.0).unwrap();

    c.s(q0).unwrap();
    c.s(q1).unwrap();
    c.sdg(q2).unwrap();
    c.sdg(q3).unwrap();
    c.t(q4).unwrap();
    c.t(q5).unwrap();
    c.tdg(q0).unwrap();
    c.tdg(q1).unwrap();

    c.cx(q0, q1).unwrap();
    c.cx(q2, q3).unwrap();
    c.cz(q1, q4).unwrap();
    c.cz(q3, q5).unwrap();
    c.cy(q0, q5).unwrap();
    c.cy(q2, q4).unwrap();
    c.swap(q1, q4).unwrap();

    c.ccx(q0, q1, q2).unwrap();
    c.u(q0, PI / 3.0, PI / 4.0, PI / 5.0).unwrap();
    c.u(q1, PI / 2.0, PI / 3.0, PI / 4.0).unwrap();
    c.u(q2, 0.34, 0.13, 0.56).unwrap();
    measure_all(&mut c);
    c
}

fn make_directive_and_fsim() -> Circuit {
    let mut circuit = Circuit::new(4);
    circuit.h(q(0)).unwrap();
    circuit.fsim(q(1), q(2), 0.21, -0.44).unwrap();
    circuit.barrier(vec![]).unwrap();
    circuit.delay(q(0), ParameterValue::from(40.0)).unwrap();
    circuit.reset(q(3)).unwrap();
    measure_all(&mut circuit);
    circuit
}

fn make_module_unitary_fallback() -> Circuit {
    let mut circuit = Circuit::new(4);

    let mut sub_label = Circuit::new(2);
    sub_label.h(q(0)).unwrap();
    sub_label.cx(q(0), q(1)).unwrap();
    let labeled_gate = sub_label.to_gate("SUB_DEMO_LABEL").unwrap();
    circuit
        .append(
            labeled_gate,
            vec![q(0), q(1)],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();

    let mut sub_empty = Circuit::new(2);
    sub_empty.x(q(0)).unwrap();
    sub_empty.y(q(1)).unwrap();
    let fallback_gate = sub_empty.to_gate("").unwrap();
    circuit
        .append(
            fallback_gate,
            vec![q(2), q(3)],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();

    let labeled_unitary = UnitaryGate::new("U_DEMO_LABEL", 2, 0);
    circuit.unitary(labeled_unitary, vec![q(0), q(2)]).unwrap();

    let fallback_unitary = UnitaryGate::new("", 2, 0);
    circuit.unitary(fallback_unitary, vec![q(1), q(3)]).unwrap();
    circuit
}

fn make_module_for_decompose() -> Circuit {
    let mut sub = Circuit::new(2);
    sub.h(q(0)).unwrap();
    sub.cx(q(0), q(1)).unwrap();
    sub.ry(q(1), 0.45).unwrap();
    let sub_gate = sub.to_gate("SUB_BELL").unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .append(
            sub_gate,
            vec![q(0), q(1)],
            Vec::<ParameterValue>::new(),
            None,
        )
        .unwrap();
    circuit.rz(q(0), -0.32).unwrap();
    circuit
}

fn make_while_control_flow() -> Circuit {
    let mut circuit = Circuit::new(3);
    circuit.measure(q(1)).unwrap();
    let condition = ClassicalExpr::bool_literal(true);
    circuit
        .while_(condition, |body| {
            body.h(q(0))?;
            body.cx(q(0), q(2))?;
            body.measure(q(0)).map(|_| ())
        })
        .unwrap();
    circuit.reset(q(2)).unwrap();
    circuit
}

fn make_for_control_flow() -> Circuit {
    let mut circuit = Circuit::new(2);
    let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(3, 0).unwrap(),
            ClassicalExpr::uint_literal(3, 4).unwrap(),
            ClassicalExpr::uint_literal(3, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                body.cx(q(0), q(1))
            },
        )
        .unwrap();
    circuit.measure(q(1)).unwrap();
    circuit
}

fn make_switch_control_flow() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |cases| {
            cases.value(0, |body| body.x(q(0)))?;
            cases.value(1, |body| body.z(q(1)))?;
            cases.default(|body| {
                body.h(q(0))?;
                body.cx(q(0), q(1))
            })
        })
        .unwrap();
    circuit
}

fn make_break_control_flow() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(0))?;
            body.break_loop()
        })
        .unwrap();
    circuit.z(q(1)).unwrap();
    circuit
}

fn make_continue_control_flow() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q(0))?;
            body.continue_loop()
        })
        .unwrap();
    circuit.z(q(1)).unwrap();
    circuit
}

fn make_advanced_control_flow() -> Circuit {
    let mut circuit = Circuit::new(1);
    let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(3, 0).unwrap(),
            ClassicalExpr::uint_literal(3, 3).unwrap(),
            ClassicalExpr::uint_literal(3, 1).unwrap(),
            |body, _| body.x(q(0)),
        )
        .unwrap();
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |cases| {
            cases.value(0, |body| body.h(q(0)))?;
            cases.default(|body| body.z(q(0)))
        })
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(0))?;
            body.break_loop()
        })
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(0))?;
            body.continue_loop()
        })
        .unwrap();
    circuit
}

fn make_if_no_else_control_flow() -> Circuit {
    let mut circuit = Circuit::new(3);
    circuit.measure(q(0)).unwrap();
    let condition = ClassicalExpr::bool_literal(false);
    circuit
        .if_(condition, |body| {
            body.x(q(1))?;
            body.rz(q(2), 0.32)
        })
        .unwrap();
    circuit
}

fn make_mcgate_and_phase() -> Circuit {
    let mut circuit = Circuit::new(5);
    circuit.phase(q(4), 0.22).unwrap();
    circuit
        .multi_control(
            StandardGate::RY,
            [q(0), q(1), q(2)],
            vec![q(3)],
            vec![ParameterValue::from(0.31)],
        )
        .unwrap();
    circuit
}

fn make_fold_stress() -> Circuit {
    let mut circuit = Circuit::new(3);
    for i in 0..14 {
        circuit.h(q(0)).unwrap();
        circuit.cx(q(0), q(1)).unwrap();
        circuit.ry(q(2), (i as f64) * 0.07 - 0.3).unwrap();
    }
    measure_all(&mut circuit);
    circuit
}

#[test]
fn test_bell_default_style() {
    assert_visual_match(
        &make_bell(),
        FigureDrawerOptions::default(),
        "bell_default.png",
    );
}

#[test]
fn test_canvas_background_is_white() {
    let svg = circuit_to_figure(&make_bell(), &FigureDrawerOptions::default()).unwrap();

    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(!svg.contains("fill=\"#dcdcdc\""));
}

#[test]
fn test_reverse_bits() {
    assert_visual_match(
        &make_bell(),
        FigureDrawerOptions {
            reverse_bits: true,
            ..FigureDrawerOptions::default()
        },
        "bell_reverse_bits.png",
    );
}

#[test]
fn test_initial_state() {
    assert_visual_match(
        &make_bell(),
        FigureDrawerOptions {
            initial_state: true,
            ..FigureDrawerOptions::default()
        },
        "show_initial_state.png",
    );
}

#[test]
fn test_all_gate() {
    assert_visual_match(
        &make_all_gate(),
        FigureDrawerOptions::default(),
        "all_gate.png",
    );
}

#[test]
fn test_directive_and_fsim() {
    assert_visual_match(
        &make_directive_and_fsim(),
        FigureDrawerOptions::default(),
        "directive_and_fsim.png",
    );
}

#[test]
fn test_module_unitary_label_and_fallback() {
    assert_visual_match(
        &make_module_unitary_fallback(),
        FigureDrawerOptions::default(),
        "module_unitary_label_fallback.png",
    );
}

#[test]
fn test_decompose_circuit_gates() {
    assert_visual_match(
        &make_module_for_decompose(),
        FigureDrawerOptions {
            decompose_circuit_gates: true,
            ..FigureDrawerOptions::default()
        },
        "module_decompose.png",
    );
}

#[test]
fn test_barrier() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.barrier(vec![q(0)]).unwrap();
    circuit.barrier(vec![q(1)]).unwrap();
    assert_visual_match(&circuit, FigureDrawerOptions::default(), "barrier.png");
}

#[test]
fn test_swap() {
    let mut circuit = Circuit::new(2);
    circuit.x(q(0)).unwrap();
    circuit.cz(q(0), q(1)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.swap(q(0), q(1)).unwrap();
    assert_visual_match(&circuit, FigureDrawerOptions::default(), "swap.png");
}

#[test]
fn test_long_theta() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.rx(q(1), PI).unwrap();
    circuit.rx(q(1), PI / 3.0).unwrap();
    circuit.rx(q(0), 1.0 / 3.0).unwrap();
    circuit.rx(q(1), PI * 13.0 / 3.0).unwrap();
    measure_all(&mut circuit);
    assert_visual_match(&circuit, FigureDrawerOptions::default(), "long_theta.png");
}

#[test]
fn test_moment() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(1)).unwrap();
    circuit.cx(q(0), q(2)).unwrap();
    measure_all(&mut circuit);
    assert_visual_match(&circuit, FigureDrawerOptions::default(), "moment.png");
}

#[test]
fn test_parameter_numeric() {
    let theta = 0.35;
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.rx(q(1), theta).unwrap();
    circuit.cry(q(0), q(1), theta).unwrap();
    measure_all(&mut circuit);
    assert_visual_match(
        &circuit,
        FigureDrawerOptions::default(),
        "parameter_numeric.png",
    );
}

#[test]
fn test_parameter_small_non_zero_uses_scientific_notation() {
    let mut circuit = Circuit::new(1);
    circuit.rx(q(0), 0.0004).unwrap();
    assert_visual_match(
        &circuit,
        FigureDrawerOptions::default(),
        "parameter_small_non_zero.png",
    );
}

#[test]
fn test_parameter_pi_fraction_preferred() {
    let mut circuit = Circuit::new(1);
    circuit.rx(q(0), PI / 2.0).unwrap();

    assert_visual_match(
        &circuit,
        FigureDrawerOptions {
            parameter_format: ParameterFormatOptions {
                mode: ParameterDisplayMode::PiFractionPreferred,
                ..ParameterFormatOptions::default()
            },
            ..FigureDrawerOptions::default()
        },
        "parameter_pi_fraction_preferred.png",
    );
}

#[test]
fn test_parameter_symbolic_with_value_for_symbolic_expr() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    circuit.rx(q(0), theta + 1.0).unwrap();

    assert_visual_match(
        &circuit,
        FigureDrawerOptions {
            parameter_format: ParameterFormatOptions {
                mode: ParameterDisplayMode::SymbolicWithValue,
                ..ParameterFormatOptions::default()
            },
            ..FigureDrawerOptions::default()
        },
        "parameter_symbolic_with_value.png",
    );
}

#[test]
fn test_svg_long_single_qubit_parameter_expands_gate_box() {
    let mut circuit = Circuit::new(1);
    let long_param = Parameter::symbol("thetaveryverylongparametername");
    circuit.rx(q(0), long_param).unwrap();

    let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();

    let rect = rect_before_text(&svg, "thetaveryverylongparametername");
    let param_fs = first_font_size_for_text(&svg, "thetaveryverylongparametername");
    assert!(rect.width > 105.6);
    assert!(param_fs >= 28.0);
    assert!(!svg.contains("..."));
}

#[test]
fn test_svg_gate_style_font_size_drives_measurement_and_rendering() {
    let mut circuit = Circuit::new(1);
    let long_param = Parameter::symbol("styledlongparametername");
    circuit.rz(q(0), long_param).unwrap();

    let default_svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();
    let default_rect = rect_before_text(&default_svg, "styledlongparametername");

    let mut gate_styles = HashMap::new();
    gate_styles.insert(
        "RZ".to_string(),
        GateStyle {
            font_size: Some(16.0),
            ..GateStyle::default()
        },
    );
    let styled_svg = circuit_to_figure(
        &circuit,
        &FigureDrawerOptions {
            gate_styles,
            ..FigureDrawerOptions::default()
        },
    )
    .unwrap();

    let styled_rect = rect_before_text(&styled_svg, "styledlongparametername");
    let name_fs = first_font_size_for_text(&styled_svg, ">RZ</text>");
    let param_fs = first_font_size_for_text(&styled_svg, "styledlongparametername");
    assert_eq!(name_fs, 16.0);
    assert!((param_fs - 16.0 * 0.78).abs() < 1e-6);
    assert!(styled_rect.width < default_rect.width * 0.6);
}

#[test]
fn test_svg_long_expression_expands_without_truncation() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let lambda = Parameter::symbol("lambda");
    let expr = Parameter::from(2.0) * theta + phi / Parameter::from(3.0) - lambda;
    circuit.ry(q(0), expr).unwrap();

    let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();

    assert!(svg.contains("theta"));
    assert!(svg.contains("phi"));
    assert!(svg.contains("lambda"));
    assert!(!svg.contains("..."));
    assert!(rect_before_text(&svg, "theta").width > 105.6);
}

#[test]
fn test_svg_controlled_long_parameter_expands_target_box() {
    let mut circuit = Circuit::new(2);
    let long_param = Parameter::symbol("controlledlongtheta");
    circuit.crx(q(0), q(1), long_param).unwrap();

    let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();

    let rect = rect_before_text(&svg, "controlledlongtheta");
    let circle_cx = circle_cx_before_text(&svg, "controlledlongtheta");
    assert!(rect.width > 105.6);
    assert!(((rect.x + rect.width / 2.0) - circle_cx).abs() < 0.01);
}

#[test]
fn test_svg_long_module_label_uses_measured_width() {
    let mut sub = Circuit::new(2);
    sub.h(q(0)).unwrap();
    sub.cx(q(0), q(1)).unwrap();
    let gate = sub.to_gate("VERY_LONG_MODULE_LABEL").unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .append(gate, vec![q(0), q(1)], Vec::<ParameterValue>::new(), None)
        .unwrap();

    let svg = circuit_to_figure(&circuit, &FigureDrawerOptions::default()).unwrap();

    assert!(rect_before_text(&svg, "VERY_LONG_MODULE_LABEL").width > 105.6);
}

#[test]
fn test_svg_fold_long_parameter_does_not_overlap_next_gate() {
    let mut circuit = Circuit::new(1);
    let long_param = Parameter::symbol("foldlongparametername");
    circuit.rx(q(0), long_param).unwrap();
    circuit.h(q(0)).unwrap();

    let svg = circuit_to_figure(
        &circuit,
        &FigureDrawerOptions {
            fold: 4,
            ..FigureDrawerOptions::default()
        },
    )
    .unwrap();

    let long_rect = rect_before_text(&svg, "foldlongparametername");
    let h_rect = rect_before_text(&svg, ">H</text>");
    assert!(long_rect.x + long_rect.width < h_rect.x);
}

#[test]
fn test_two_qubit_rotation() {
    let mut circuit = Circuit::new(4);
    circuit.rxx(q(0), q(1), PI / 3.0).unwrap();
    circuit.ryy(q(1), q(2), PI / 4.0).unwrap();
    circuit.rzz(q(2), q(3), PI / 5.0).unwrap();
    circuit.rzx(q(0), q(3), PI / 6.0).unwrap();
    assert_visual_match(
        &circuit,
        FigureDrawerOptions::default(),
        "two_qubit_rotation.png",
    );
}

#[test]
fn test_unitary() {
    let mut circuit = Circuit::new(4);
    let unitary = UnitaryGate::new("UNITARY", 3, 0);
    circuit.unitary(unitary, vec![q(0), q(1), q(3)]).unwrap();
    assert_visual_match(&circuit, FigureDrawerOptions::default(), "unitary.png");
}

#[test]
fn test_control_flow_expansion() {
    let mut circuit = Circuit::new(2);
    let condition = ClassicalExpr::bool_literal(true);
    circuit
        .if_else(condition, |body| body.x(q(1)), |body| body.z(q(1)))
        .unwrap();

    assert_visual_match(
        &circuit,
        FigureDrawerOptions::default(),
        "if_else_control_flow.png",
    );
}

#[test]
fn test_control_flow_while() {
    assert_visual_match(
        &make_while_control_flow(),
        FigureDrawerOptions::default(),
        "while_control_flow.png",
    );
}

#[test]
fn test_control_flow_if_without_else() {
    assert_visual_match(
        &make_if_no_else_control_flow(),
        FigureDrawerOptions::default(),
        "if_no_else_control_flow.png",
    );
}

#[test]
fn test_control_flow_for() {
    assert_visual_match(
        &make_for_control_flow(),
        FigureDrawerOptions::default(),
        "for_control_flow.png",
    );
}

#[test]
fn test_control_flow_switch() {
    assert_visual_match(
        &make_switch_control_flow(),
        FigureDrawerOptions::default(),
        "switch_control_flow.png",
    );
}

#[test]
fn test_control_flow_break() {
    assert_visual_match(
        &make_break_control_flow(),
        FigureDrawerOptions::default(),
        "break_control_flow.png",
    );
}

#[test]
fn test_control_flow_continue() {
    assert_visual_match(
        &make_continue_control_flow(),
        FigureDrawerOptions::default(),
        "continue_control_flow.png",
    );
}

#[test]
fn test_advanced_control_flow() {
    assert_visual_match(
        &make_advanced_control_flow(),
        FigureDrawerOptions::default(),
        "advanced_control_flow.png",
    );
}

#[test]
fn test_default_style_is_applied() {
    let mut circuit = Circuit::new(1);
    circuit.x(q(0)).unwrap();
    assert_visual_match(
        &circuit,
        FigureDrawerOptions::default(),
        "default_style_applied.png",
    );
}

#[test]
fn test_show_params_false() {
    let mut circuit = Circuit::new(2);
    circuit.rx(q(0), 0.66).unwrap();
    circuit.crz(q(0), q(1), -0.44).unwrap();
    circuit.u(q(1), 0.4, -0.2, 0.1).unwrap();
    assert_visual_match(
        &circuit,
        FigureDrawerOptions {
            show_params: false,
            ..FigureDrawerOptions::default()
        },
        "show_params_false.png",
    );
}

#[test]
fn test_multicontrol_and_phase() {
    assert_visual_match(
        &make_mcgate_and_phase(),
        FigureDrawerOptions::default(),
        "multicontrol_and_phase.png",
    );
}

#[test]
fn test_fold_layout() {
    assert_visual_match(
        &make_fold_stress(),
        FigureDrawerOptions {
            fold: 8,
            ..FigureDrawerOptions::default()
        },
        "fold_layout.png",
    );
}
