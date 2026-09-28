// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// Licensed under the Apache License, Version 2.0; see LICENSE.txt.

//! Regression tests for symbolic QCIS rewriting.

use super::{KnowledgeRewriter, RewriteConfig};
use crate::circuit::circuit_to_matrix::circuit_to_matrix;
use crate::circuit::test_utils::assert_matrix_approx_eq;
use crate::circuit::{Circuit, Instruction, Parameter, Qubit, StandardGate};
use crate::compile::knowledge::rule_dsl::load::load_rules_from_str;
use crate::compile::test_utils::standard_ops;
use std::collections::HashMap;
use std::f64::consts::PI;

fn source_circuit(source: &str) -> Circuit {
    let rule = load_rules_from_str(&format!(
        "rule input {{ match {{ {source} }} rewrite {{}} }}"
    ))
    .unwrap()
    .remove(0);
    let mut circuit = Circuit::new(rule.num_qubits());
    for item in rule.operations {
        circuit
            .append(
                item.instruction,
                item.qubits.into_iter().map(Qubit::new),
                item.params.unwrap_or_default(),
                None,
            )
            .unwrap();
    }
    circuit
}

fn assert_strict_equivalence(source: &Circuit, actual: &Circuit) {
    for angle in [0.0, 0.317, -1.239, PI / 2.0, PI, 2.0 * PI, -4.0 * PI] {
        let bindings = Some(HashMap::from([
            ("a", angle),
            ("b", 0.73 - angle),
            ("c", -0.91),
            ("theta", angle),
            ("phi", 0.19 + angle),
            ("lambda", -0.39),
        ]));
        let lhs = circuit_to_matrix(&source.assign_parameters(&bindings).unwrap(), None).unwrap();
        let rhs = circuit_to_matrix(&actual.assign_parameters(&bindings).unwrap(), None).unwrap();
        assert_matrix_approx_eq(&lhs, &rhs, 1e-9);
    }
}

#[test]
fn symbolic_qcis_rules_shorten_and_reach_a_fixed_point() {
    use StandardGate::*;
    for (source, expected) in [
        ("XY(a) 0, XY(b) 0", vec![RZ]),
        ("RZ(a) 0, XY(b) 0", vec![XY]),
        ("XY(b) 0, RZ(a) 0", vec![XY]),
        ("RZ(a) 0, XY2P(b) 0, RZ(c) 0", vec![XY2P, RZ]),
        ("RZ(a) 0, XY2P(b) 0, RZ(-a) 0", vec![XY2P]),
        ("RZ(a) 0, XY2M(b) 0, RZ(2*π-a) 0", vec![XY2M]),
        ("RZ(a) 0, RXY(theta,b) 0, RZ(-a) 0", vec![RXY]),
        ("X2P 0, RZ(a) 0, X2M 0", vec![RY]),
        ("X2M 0, RZ(a) 0, X2P 0", vec![RY]),
        ("Y2M 0, RZ(a) 0, Y2P 0", vec![RX]),
        ("Y2P 0, RZ(a) 0, Y2M 0", vec![RX]),
        ("XY2P(phi) 0, XY2P(phi+π) 0", vec![]),
        ("XY2M(phi) 0, XY2M(phi-3*π) 0", vec![]),
        ("XY2P(phi) 0, XY2P(phi+2*π) 0", vec![XY]),
        ("XY2M(phi) 0, XY2M(phi-2*π) 0", vec![XY]),
        ("RXY(a,phi) 0, RXY(b,phi+2*π) 0", vec![RXY]),
        ("RXY(a,phi) 0, RXY(b,phi+π) 0", vec![RXY]),
        ("RXY(2*π,phi) 0", vec![]),
        ("RXY(-2*π,phi) 0", vec![]),
    ] {
        let input = source_circuit(source);
        let result = KnowledgeRewriter::production().run(&input).unwrap();
        assert_eq!(standard_ops(&result.circuit), expected, "{source}");
        assert!(result.stats.reached_fixpoint, "{source}");
        assert!(
            !KnowledgeRewriter::production()
                .run(&result.circuit)
                .unwrap()
                .changed,
            "{source}"
        );
        assert_strict_equivalence(&input, &result.circuit);
    }
}

#[test]
fn parameter_reducing_specializations_are_not_rejected_as_equal_cost() {
    use StandardGate::*;
    for (source, expected) in [
        ("RXY(π/2,phi) 0", XY2P),
        ("RXY(-π/2,phi) 0", XY2M),
        ("RXY(9*π,phi) 0", XY),
        ("RXY(theta,2*π) 0", RX),
        ("RXY(theta,π) 0", RX),
        ("RXY(theta,π/2) 0", RY),
        ("RXY(theta,-π/2) 0", RY),
        ("XY2P(3*π) 0", X2M),
        ("XY2M(-π) 0", X2P),
    ] {
        let input = source_circuit(source);
        let result = KnowledgeRewriter::production().run(&input).unwrap();
        assert_eq!(standard_ops(&result.circuit), vec![expected], "{source}");
        assert!(result.stats.reached_fixpoint, "{source}");
        assert_strict_equivalence(&input, &result.circuit);
    }
}

#[test]
fn cz_pi_pulse_echo_removes_both_entanglers_for_either_wire_order() {
    for pulse in ["Y", "XY(phi)"] {
        for active in [0, 1] {
            for reversed in [false, true] {
                let last = if reversed { "1 0" } else { "0 1" };
                let input = source_circuit(&format!("CZ 0 1, {pulse} {active}, CZ {last}"));
                let result = KnowledgeRewriter::production().run(&input).unwrap();
                assert_eq!(result.circuit.operations().len(), 2);
                assert!(!standard_ops(&result.circuit).contains(&StandardGate::CZ));
                assert_strict_equivalence(&input, &result.circuit);
            }
        }
    }
}

#[test]
fn cz_echo_stays_in_the_requested_native_basis() {
    let input = source_circuit("CZ 0 1, XY(phi) 1, CZ 1 0");
    let basis = [StandardGate::CZ, StandardGate::XY, StandardGate::RZ];
    let config = RewriteConfig::production()
        .with_target_instructions(basis.map(Instruction::Standard).to_vec())
        .unwrap();
    let result = KnowledgeRewriter::new(config).run(&input).unwrap();
    assert_eq!(result.circuit.operations().len(), 2);
    assert!(!standard_ops(&result.circuit).contains(&StandardGate::CZ));
    assert!(
        standard_ops(&result.circuit)
            .iter()
            .all(|gate| basis.contains(gate))
    );
    assert_strict_equivalence(&input, &result.circuit);
}

#[test]
fn half_turn_echo_keeps_the_phase_of_the_original_rz_angle() {
    for pulse in ["X2P", "X2M", "Y2P", "Y2M", "XY2P(phi)", "XY2M(phi)"] {
        for angle in ["π", "3*π", "-π"] {
            let input = source_circuit(&format!("{pulse} 0, RZ({angle}) 0, {pulse} 0"));
            let result = KnowledgeRewriter::production().run(&input).unwrap();
            assert_eq!(result.circuit.operations().len(), 1, "{pulse}, {angle}");
            assert_strict_equivalence(&input, &result.circuit);
        }
    }
}

#[test]
fn qcis_rules_respect_angles_barriers_and_labels() {
    for source in [
        "XY2P(phi) 0, XY2P(phi+π+0.001) 0",
        "RXY(5*π/2,phi) 0",            // differs from XY2P by a minus sign
        "CZ 0 1, XY2P(phi) 0, CZ 0 1", // half pulse is not a Pauli echo
    ] {
        let input = source_circuit(source);
        assert!(
            !KnowledgeRewriter::production().run(&input).unwrap().changed,
            "{source}"
        );
    }
    for barrier in [false, true] {
        let mut input = Circuit::new(2);
        input.cz(Qubit::new(0), Qubit::new(1)).unwrap();
        if barrier {
            input.barrier(vec![Qubit::new(0)]).unwrap();
            input.xy(Qubit::new(0), Parameter::symbol("phi")).unwrap();
        } else {
            input
                .append(
                    Instruction::Standard(StandardGate::XY),
                    [Qubit::new(0)],
                    [Parameter::symbol("phi").into()],
                    Some("keep pulse"),
                )
                .unwrap();
        }
        input.cz(Qubit::new(0), Qubit::new(1)).unwrap();
        assert!(!KnowledgeRewriter::production().run(&input).unwrap().changed);
    }
}
