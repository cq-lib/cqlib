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
use crate::circuit::gate::{ClassicalDataOp, Instruction};
use crate::circuit::{ClassicalControlOp, ClassicalType, Qubit, StandardGate};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cqlib_qasm3_load_{}_{}_{}.qasm",
        std::process::id(),
        test_name,
        nonce
    ))
}

fn assert_standard_gate(circuit: &Circuit, index: usize, gate: StandardGate, qubits: &[u32]) {
    let op = &circuit.operations()[index];
    assert!(matches!(op.instruction, Instruction::Standard(actual) if actual == gate));
    assert_eq!(
        op.qubits.iter().copied().collect::<Vec<_>>(),
        qubits.iter().copied().map(Qubit::new).collect::<Vec<_>>()
    );
}

fn assert_fixed_param(circuit: &Circuit, op_index: usize, param_index: usize, expected: f64) {
    let param = &circuit.operations()[op_index].params[param_index];
    let value = match param {
        crate::circuit::CircuitParam::Fixed(value) => *value,
        crate::circuit::CircuitParam::Index(index) => circuit.parameters()[*index as usize]
            .evaluate(&None)
            .unwrap(),
    };
    assert!(
        (value - expected).abs() < 1e-10,
        "expected {expected}, got {value}"
    );
}

fn assert_err(source: &str, matches: impl FnOnce(&Qasm3ParseError) -> bool) -> Qasm3ParseError {
    let err = loads(source).unwrap_err();
    assert!(matches(&err), "unexpected error: {err:?}");
    err
}

fn loads_without_panicking(source: &str) -> Result<Circuit, Qasm3ParseError> {
    std::panic::catch_unwind(|| loads(source)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        panic!("OpenQASM 3 loader panicked for valid input: {message}");
    })
}

#[test]
fn loads_bell_circuit() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[2] q;
        h q[0];
        cx q[0], q[1];
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations().len(), 2);
    assert_standard_gate(&circuit, 0, StandardGate::H, &[0]);
    assert_standard_gate(&circuit, 1, StandardGate::CX, &[0, 1]);
}

#[test]
fn from_str_alias_matches_loads() {
    let source = r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        h q;
    "#;

    let loaded = loads(source).unwrap();
    let aliased = from_str(source).unwrap();

    assert_eq!(aliased.num_qubits(), loaded.num_qubits());
    assert_eq!(aliased.operations().len(), loaded.operations().len());
}

#[test]
fn loads_openqasm_3_0_header_without_normalization() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit q;
        x q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
}

#[test]
fn loads_indexed_bit_measurement_assignment() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit[2] q;
        bit[2] v;
        v[0] = measure q[0];
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. })
            if target.ty() == ClassicalType::bit_vec(2).unwrap()
    ));
}

#[test]
fn loads_scalar_bit_measurement_assignment_from_indexed_qubit() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit[2] q;
        bit v0;
        v0 = measure q[0];
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. })
            if target.ty() == ClassicalType::Bit
    ));
}

#[test]
fn loads_scalar_bit_measurement_assignment_when_target_is_read_later() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit[1] q;
        bit v0;
        v0 = measure q[0];
        if (v0) {
            x q[0];
        }
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 3);
    assert!(matches!(
        circuit.operations()[2].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
}

#[test]
fn rejects_multi_qubit_measurement_into_indexed_bit() {
    assert_err(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit[2] q;
        bit[2] v;
        v[0] = measure q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::MismatchedQubitCount {
                    expected: 1,
                    actual: 2
                }
            )
        },
    );
}

#[test]
fn load_file_reads_qasm3_source() {
    let dir = std::env::temp_dir().join(format!("cqlib_qasm3_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let source_path = dir.join("main.qasm");

    fs::write(
        &source_path,
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        h q;
        "#,
    )
    .unwrap();

    let circuit = load(&source_path).unwrap();
    assert_eq!(circuit.operations().len(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::H, &[0]);
}

#[test]
fn from_path_alias_reads_file() {
    let source_path = unique_temp_path("from_path");
    fs::write(
        &source_path,
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        x q;
    "#,
    )
    .unwrap();

    let circuit = from_path(source_path.as_path()).unwrap();

    assert_eq!(circuit.operations().len(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
    fs::remove_file(source_path).unwrap();
}

#[test]
fn loads_parameters_and_register_order() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        input angle[64] theta;
        qubit[2] q;
        qubit and;
        rx(pi / 2) q[1];
        rz(theta) and;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 3);
    assert_standard_gate(&circuit, 0, StandardGate::RX, &[1]);
    assert_standard_gate(&circuit, 1, StandardGate::RZ, &[2]);
    assert_fixed_param(&circuit, 0, 0, std::f64::consts::PI / 2.0);
    assert_eq!(circuit.parameters().len(), 1);
    assert!(circuit.symbols().contains("theta"));
}

#[test]
fn loads_u2_as_u_with_inserted_pi_over_two() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        u2(0.25, 0.5) q;
        "#,
    )
    .unwrap();

    assert_standard_gate(&circuit, 0, StandardGate::U, &[0]);
    assert_eq!(circuit.operations()[0].params.len(), 3);
    assert_fixed_param(&circuit, 0, 0, std::f64::consts::PI / 2.0);
    assert_fixed_param(&circuit, 0, 1, 0.25);
    assert_fixed_param(&circuit, 0, 2, 0.5);
}

#[test]
fn loads_single_bit_measurement() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        qubit q;
        bit c;
        c = measure q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.classical_vars(), &[ClassicalType::Bit]);
    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
}

#[test]
fn loads_measurement_into_bitvec() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        qubit[2] q;
        bit[2] c;
        c = measure q;
        "#,
    )
    .unwrap();

    assert_eq!(
        circuit.classical_vars(),
        &[ClassicalType::bit_vec(2).unwrap()]
    );
    assert_eq!(
        circuit.classical_values(),
        &[ClassicalType::bit_vec(2).unwrap()]
    );
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
}

#[test]
fn loads_measurement_expression_statement() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        qubit q;
        measure q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.classical_values(), &[ClassicalType::Bit]);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
}

#[test]
fn loads_reset_barrier_and_if_else() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        bool flag = true;
        reset q;
        barrier q;
        if (flag) {
            x q;
        } else {
            z q;
        }
        "#,
    )
    .unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations().last().unwrap().instruction
    else {
        panic!("expected if control op");
    };
    assert_eq!(if_op.then_body().operations().len(), 1);
    assert_eq!(if_op.else_body().unwrap().operations().len(), 1);
    assert!(matches!(
        if_op.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        if_op.else_body().unwrap().operations()[0].instruction,
        Instruction::Standard(StandardGate::Z)
    ));
}

#[test]
fn rejects_while_when_frontend_reports_semantic_error() {
    assert_err(
        r#"
        OPENQASM 3;
        bool flag = true;
        qubit q;
        while (flag) {
            x q;
        }
        "#,
        |err| matches!(err, Qasm3ParseError::SemanticError(_)),
    );
}

#[test]
fn loads_static_for_loop() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[3] q;
        for uint[8] i in [0:2] {
            x q[i];
        }
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 3);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
    assert_standard_gate(&circuit, 1, StandardGate::X, &[1]);
    assert_standard_gate(&circuit, 2, StandardGate::X, &[2]);
}

#[test]
fn loads_static_for_loop_with_step() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[5] q;
        for uint[8] i in [0:2:4] {
            x q[i];
        }
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 3);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
    assert_standard_gate(&circuit, 1, StandardGate::X, &[2]);
    assert_standard_gate(&circuit, 2, StandardGate::X, &[4]);
}

#[test]
fn loads_switch_with_default() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        uint[2] c = 1;
        qubit q;
        switch (c) {
            case 0 {
                x q;
            }
            case 1 {
                z q;
            }
            default {
                h q;
            }
        }
        "#,
    )
    .unwrap();

    assert!(matches!(
        circuit.operations().last().unwrap().instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Switch(_))
    ));
}

#[test]
fn loads_custom_gate() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        gate bell a, b {
            h a;
            cx a, b;
        }
        qubit[2] q;
        bell q[0], q[1];
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 1);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::CircuitGate(_)
    ));
}

#[test]
fn loads_parameterized_custom_gate() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        input angle[64] phi;
        gate phasey(theta) a {
            rx(theta) a;
        }
        qubit q;
        phasey(phi) q;
        "#,
    )
    .unwrap();

    let Instruction::CircuitGate(gate) = &circuit.operations()[0].instruction else {
        panic!("expected circuit gate");
    };
    assert_eq!(gate.num_qubits(), 1);
    assert_eq!(gate.num_params(), 1);
    assert_eq!(circuit.operations()[0].params.len(), 1);
    assert!(circuit.symbols().contains("phi"));
}

#[test]
fn rejects_gate_modifiers_rejected_by_frontend_or_lowering() {
    assert_err(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[2] q;
        ctrl @ x q[0], q[1];
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::UnsupportedFeature(_)
            )
        },
    );
}

#[test]
fn rejects_gate_arity_mismatch() {
    assert_err(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[2] q;
        h q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::MismatchedQubitCount { .. }
            )
        },
    );
}

#[test]
fn rejects_measurement_width_mismatch() {
    assert_err(
        r#"
        OPENQASM 3;
        qubit[2] q;
        bit c;
        c = measure q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::ConversionError(_)
            )
        },
    );
}

#[test]
fn loads_indexed_measurement_assignment_from_scalar_qubit() {
    let circuit = loads(
        r#"
        OPENQASM 3;
        qubit q;
        bit[2] c;
        c[0] = measure q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. })
            if target.ty() == ClassicalType::bit_vec(2).unwrap()
    ));
}

#[test]
fn rejects_out_of_bounds_qubit_index() {
    assert_err(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[1] q;
        x q[1];
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::InvalidArgument(_)
            )
        },
    );
}

#[test]
fn rejects_unsupported_standard_gate() {
    assert_err(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[2] q;
        cp(0.25) q[0], q[1];
        "#,
        |err| matches!(err, Qasm3ParseError::UnsupportedFeature(_)),
    );
}

#[test]
fn rejects_unsupported_gate_modifier() {
    assert_err(
        r#"
        OPENQASM 3;
        include "stdgates.inc";
        qubit[2] q;
        negctrl @ x q[0], q[1];
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::UnsupportedFeature(_)
            )
        },
    );
}

#[test]
fn rejects_measurement_in_gate_body() {
    assert_err(
        r#"
        OPENQASM 3;
        gate bad a {
            measure a;
        }
        qubit q;
        bad q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_) | Qasm3ParseError::UnsupportedFeature(_)
            )
        },
    );
}

#[test]
fn rejects_recursive_gate_definition() {
    assert_err(
        r#"
        OPENQASM 3;
        gate rec a {
            rec a;
        }
        qubit q;
        rec q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::SemanticError(_)
                    | Qasm3ParseError::CircularGateDependency { .. }
                    | Qasm3ParseError::RecursionLimitExceeded(_)
            )
        },
    );
}

#[test]
fn rejects_non_angle_input_declaration() {
    assert_err(
        r#"
        OPENQASM 3;
        input bit flag;
        qubit q;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::ParseError(_) | Qasm3ParseError::UnsupportedFeature(_)
            )
        },
    );
}

#[test]
fn rejects_runtime_uint_arithmetic_store() {
    assert_err(
        r#"
        OPENQASM 3;
        uint[8] a = 1;
        uint[8] b = 2;
        a = b + 1;
        "#,
        |err| {
            matches!(
                err,
                Qasm3ParseError::ParseError(_) | Qasm3ParseError::UnsupportedFeature(_)
            )
        },
    );
}

#[test]
fn rejects_unsupported_defcal() {
    let err = loads(
        r#"
        OPENQASM 3;
        defcalgrammar "openpulse";
        defcal x $0 {
        }
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        Qasm3ParseError::ParseError(_)
            | Qasm3ParseError::SemanticError(_)
            | Qasm3ParseError::UnsupportedFeature(_)
    ));
}

#[test]
fn loads_top_level_anonymous_scope() {
    let circuit = loads_without_panicking(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit q;
        {
            x q;
        }
        "#,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!(circuit.operations().len(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
}

#[test]
fn rejects_qubit_declaration_in_anonymous_scope_without_panicking() {
    let err = loads_without_panicking(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        {
            qubit q;
            x q;
        }
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        Qasm3ParseError::ParseError(_) | Qasm3ParseError::SemanticError(_)
    ));
}

#[test]
fn loads_single_quoted_include_without_panicking() {
    let circuit = loads_without_panicking(
        r#"
        OPENQASM 3.0;
        include 'stdgates.inc';
        qubit q;
        x q;
        "#,
    )
    .expect("StringLiteral accepts single-quoted strings");

    assert_eq!(circuit.num_qubits(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
}

#[test]
fn preserves_classical_initializer_statement_order() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit q;
        x q;
        bool flag = true;
        z q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 3);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert_standard_gate(&circuit, 2, StandardGate::Z, &[0]);
}

#[test]
fn normalizes_actual_header_when_comment_contains_short_header() {
    let circuit = loads(
        r#"
        // OPENQASM 3;
        OPENQASM 3;
        include "stdgates.inc";
        qubit q;
        x q;
        "#,
    )
    .expect("header normalization must ignore comments");

    assert_eq!(circuit.num_qubits(), 1);
    assert_standard_gate(&circuit, 0, StandardGate::X, &[0]);
}

#[test]
fn loads_scalar_measurement_assignment_with_unicode_identifiers() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        qubit[1] 量子;
        bit 结果;
        结果 = measure 量子[0];
        "#,
    )
    .expect("Identifier accepts valid Unicode letters");

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. })
            if target.ty() == ClassicalType::Bit
    ));
}

#[test]
fn loads_classical_declaration_in_control_flow_scope() {
    let circuit = loads(
        r#"
        OPENQASM 3.0;
        include "stdgates.inc";
        qubit q;
        if (true) {
            bit flag = false;
            if (flag) {
                x q;
            }
        }
        "#,
    )
    .expect("scope accepts classicalDeclarationStatement");

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!(circuit.classical_vars(), &[ClassicalType::Bit]);
}

#[test]
fn rejects_pragma_without_remaining_line_content() {
    let err = loads(
        r#"
        OPENQASM 3.0;
        pragma
        qubit q;
        "#,
    )
    .unwrap_err();

    assert!(
        matches!(err, Qasm3ParseError::ParseError(_)),
        "pragma requires RemainingLineContent, got {err:?}"
    );
}

#[test]
fn rejects_nested_block_comment() {
    let err = loads(
        r#"
        OPENQASM 3.0;
        /* outer /* inner */ outer */
        qubit q;
        "#,
    )
    .unwrap_err();

    assert!(
        matches!(err, Qasm3ParseError::ParseError(_)),
        "BlockComment is non-nesting in qasm3Lexer, got {err:?}"
    );
}
