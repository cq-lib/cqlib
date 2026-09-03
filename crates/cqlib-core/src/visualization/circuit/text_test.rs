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
use crate::circuit::{Circuit, ClassicalExpr, ClassicalType, Parameter, Qubit};
use crate::visualization::circuit::layout::build_visual_circuit;
use crate::visualization::circuit::text::draw_text_from_visual;
use crate::visualization::circuit::{ParameterDisplayMode, VisualBuildOptions};
use std::f64::consts::PI;

fn norm(s: &str) -> String {
    let mut s = s.replace("\r\n", "\n");
    if s.starts_with('\n') {
        s.remove(0);
    }
    s = s.trim_end_matches('\n').to_string();
    let trimmed = s
        .split('\n')
        .map(|line| line.trim_end_matches(' '))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{trimmed}\n\n")
}

fn assert_diagram(actual: &str, expected: &str) {
    assert_eq!(norm(actual), norm(expected));
}

#[test]
fn text_drawer_options_default_matches_public_api() {
    let options = TextDrawerOptions::default();

    assert!(options.show_params);
    assert!(!options.decompose_circuit_gates);
    assert_eq!(options.line_width, 80);
    assert!(!options.initial_state);
    assert!(!options.reverse_bits);
}

#[test]
fn test_basic() {
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(1)).unwrap();
    circuit.measure(Qubit::new(2)).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                
 Q0: ───H──■──M─
           │    
 Q1: ──────┼──M─
           │    
 Q2: ──────X──M─
                

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_barrier() {
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.barrier(vec![Qubit::new(0), Qubit::new(2)]).unwrap();
    circuit.delay(Qubit::new(0), 20.0.into()).unwrap();
    circuit.reset(Qubit::new(2)).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                    
 Q0: ───H──│──D(20)─
                    
 Q1: ───────────────
           │        
 Q2: ──────│───|0>──
                    

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_width_wrap() {
    let mut circuit = Circuit::new(1);
    for _ in 0..10 {
        circuit.h(Qubit::new(0)).unwrap();
    }
    let options = TextDrawerOptions {
        line_width: 12,
        ..TextDrawerOptions::default()
    };
    let text = circuit_to_text(&circuit, &options).unwrap();
    let resp = r#"
                   »
 Q0: ───H──H──H──H─»
                   »

«                   »
« Q0: ───H──H──H──H─»
«                   »

«             
« Q0: ───H──H─
«             

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_empty_circuit() {
    let circuit = Circuit::new(0);
    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    assert_eq!(text, "empty circuit");
}

#[test]
fn test_show_params_false() {
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), 0.5).unwrap();
    let options = TextDrawerOptions {
        show_params: false,
        ..TextDrawerOptions::default()
    };
    let text = circuit_to_text(&circuit, &options).unwrap();
    let resp = r#"
          
 Q0: ───RX─
          

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_no_wrap_when_line_width_negative() {
    let mut circuit = Circuit::new(1);
    for _ in 0..10 {
        circuit.h(Qubit::new(0)).unwrap();
    }
    let options = TextDrawerOptions {
        line_width: -1,
        ..TextDrawerOptions::default()
    };
    let text = circuit_to_text(&circuit, &options).unwrap();
    assert!(!text.contains("«"));
    assert!(!text.contains("»"));
}

#[test]
fn test_draw_text_from_visual_matches_circuit_to_text() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let options = TextDrawerOptions::default();

    let direct = circuit_to_text(&circuit, &options).unwrap();
    let visual = build_visual_circuit(&circuit, &VisualBuildOptions::default()).unwrap();
    let from_visual = draw_text_from_visual(&visual, &options).unwrap();
    assert_eq!(norm(&direct), norm(&from_visual));
}

#[test]
fn test_decompose_circuit_gates_option() {
    let mut sub = Circuit::new(2);
    sub.h(Qubit::new(0)).unwrap();
    sub.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sub_gate = sub.to_gate("SUB_BELL").unwrap();

    let mut circuit = Circuit::new(2);
    circuit
        .append(
            sub_gate,
            vec![Qubit::new(0), Qubit::new(1)],
            std::iter::empty::<ParameterValue>(),
            None,
        )
        .unwrap();

    let text_no_decompose = circuit_to_text(
        &circuit,
        &TextDrawerOptions {
            decompose_circuit_gates: false,
            ..TextDrawerOptions::default()
        },
    )
    .unwrap();
    let resp_no_decompose = r#"
        ┌──────────┐ 
 Q0: ───│          │─
        │ SUB_BELL │ 
 Q1: ───│          │─
        └──────────┘ 

"#;
    assert_diagram(&text_no_decompose, resp_no_decompose);

    let text_decompose = circuit_to_text(
        &circuit,
        &TextDrawerOptions {
            decompose_circuit_gates: true,
            ..TextDrawerOptions::default()
        },
    )
    .unwrap();
    let resp_decompose = r#"
                
 Q0: ───H──■─
           │    
 Q1: ──────X─
                

"#;
    assert_diagram(&text_decompose, resp_decompose);
}

#[test]
fn test_initial_state() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    let options = TextDrawerOptions {
        initial_state: true,
        ..TextDrawerOptions::default()
    };
    let text = circuit_to_text(&circuit, &options).unwrap();
    let resp = r#"
             
 Q0: |0>───H─
             

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_reverse_bits() {
    let mut circuit = Circuit::new(2);
    circuit.x(Qubit::new(0)).unwrap();
    let options = TextDrawerOptions {
        reverse_bits: true,
        ..TextDrawerOptions::default()
    };
    let text = circuit_to_text(&circuit, &options).unwrap();
    let resp = r#"
          
 Q1: ─────
          
 Q0: ───X─
          

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_if_label() {
    let mut circuit = Circuit::new(2);
    let condition = ClassicalExpr::bool_literal(true);
    circuit
        .if_(condition, |body| body.x(Qubit::new(1)))
        .unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"

 Q0: ───────────────────────────────
        ┌───────────┐     ┌───────┐
 Q1: ───┤ If-0 true ├──X──┤ End-0 ├─
        └───────────┘     └───────┘

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_if_else() {
    let mut circuit = Circuit::new(2);
    circuit.measure(Qubit::new(0)).unwrap();
    let condition = ClassicalExpr::bool_literal(false);
    circuit
        .if_else(
            condition,
            |body| body.x(Qubit::new(1)),
            |body| body.z(Qubit::new(1)),
        )
        .unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"

 Q0: ───M──────────────────────────────────────────────
           ┌────────────┐     ┌────────┐     ┌───────┐
 Q1: ──────┤ If-0 false ├──X──┤ Else-0 ├──Z──┤ End-0 ├─
           └────────────┘     └────────┘     └───────┘

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_while() {
    let mut circuit = Circuit::new(2);
    let condition = ClassicalExpr::bool_literal(false);
    circuit
        .while_(condition, |body| {
            body.h(Qubit::new(0))?;
            body.cx(Qubit::new(0), Qubit::new(1))?;
            body.measure(Qubit::new(0)).map(|_| ())
        })
        .unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
        ┌───────────────┐           ┌───────┐ 
 Q0: ───┤               ├──H──■──M──┤       ├─
        │ While-0 false │     │     │ End-0 │ 
 Q1: ───┤               ├─────X─────┤       ├─
        └───────────────┘           └───────┘ 

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_for_switch_break_continue_markers() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(1)).unwrap();
    let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(3, 0).unwrap(),
            ClassicalExpr::uint_literal(3, 3).unwrap(),
            ClassicalExpr::uint_literal(3, 1).unwrap(),
            |body, _| body.x(Qubit::new(0)),
        )
        .unwrap();
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |cases| {
            cases.value(0, |body| body.h(Qubit::new(0)))?;
            cases.default(|body| body.z(Qubit::new(0)))
        })
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(Qubit::new(0))?;
            body.break_loop()
        })
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.x(Qubit::new(0))?;
            body.continue_loop()
        })
        .unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
           ┌──────────────────┐     ┌───────┐  ┌────────────┐  ┌──────────┐    »
 Q0: ──────┤ For-0 range(0,3) ├──X──┤ End-0 ├──┤ Switch-1 1 ├──┤ Case-1 0 ├──H─»
           └──────────────────┘     └───────┘  └────────────┘  └──────────┘    »
 Q1: ───H──────────────────────────────────────────────────────────────────────»
                                                                               »

«        ┌───────────┐     ┌───────┐  ┌──────────────┐     ┌───────┐  ┌───────┐ »
« Q0: ───┤ Default-1 ├──Z──┤ End-1 ├──┤ While-2 true ├──X──┤ Break ├──┤ End-2 ├─»
«        └───────────┘     └───────┘  └──────────────┘     └───────┘  └───────┘ »
« Q1: ──────────────────────────────────────────────────────────────────────────»
«                                                                               »

«        ┌──────────────┐     ┌──────────┐  ┌───────┐
« Q0: ───┤ While-3 true ├──X──┤ Continue ├──┤ End-3 ├─
«        └──────────────┘     └──────────┘  └───────┘
« Q1: ────────────────────────────────────────────────
«

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_fsim() {
    let mut circuit = Circuit::new(2);
    circuit
        .fsim(Qubit::new(0), Qubit::new(1), 0.11, 0.22)
        .unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
               
 Q0: ────FSIM───
           │    
 Q1: ────FSIM───
               

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_single_qubit_gates_snapshot() {
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.rx(Qubit::new(0), 0.125).unwrap();
    circuit.z(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                           
 Q0: ───H──RX(0.12)──Z──M─
                           

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_two_qubit_gates_snapshot() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(1)).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                   
 Q0: ───■──■──X──M─
        │  │  │    
 Q1: ───X──■──X──M─
                   

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_mixed_single_two_qubit_gates_snapshot() {
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.x(Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.ry(Qubit::new(1), 0.25).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.measure(Qubit::new(0)).unwrap();
    circuit.measure(Qubit::new(1)).unwrap();
    circuit.measure(Qubit::new(2)).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                             
 Q0: ───H──■─────M───────────
           │                 
 Q1: ──────X──RY(0.25)──■──M─
                        │    
 Q2: ───X───────────────■──M─
                             

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_parameter_small_non_zero_uses_scientific_notation_in_text() {
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), 0.0004).unwrap();

    let text = circuit_to_text(&circuit, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
               
 Q0: ───RX(4e-4)─
               

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_parameter_pi_fraction_preferred_in_text_from_visual() {
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), PI / 2.0).unwrap();

    let mut build_options = VisualBuildOptions::default();
    build_options.parameter_format.mode = ParameterDisplayMode::PiFractionPreferred;
    let visual = build_visual_circuit(&circuit, &build_options).unwrap();

    let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
              
 Q0: ───RX(π/2)─
              

"#;
    assert_diagram(&text, resp);
}

#[test]
fn test_parameter_symbolic_with_value_for_symbolic_expr_in_text_from_visual() {
    let mut circuit = Circuit::new(1);
    let theta = Parameter::symbol("theta");
    circuit.rx(Qubit::new(0), theta + 1.0).unwrap();

    let mut build_options = VisualBuildOptions::default();
    build_options.parameter_format.mode = ParameterDisplayMode::SymbolicWithValue;
    let visual = build_visual_circuit(&circuit, &build_options).unwrap();

    let text = draw_text_from_visual(&visual, &TextDrawerOptions::default()).unwrap();
    let resp = r#"
                  
 Q0: ───RX(1 + theta)─
                  

"#;
    assert_diagram(&text, resp);
}
