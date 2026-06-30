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
use crate::circuit::circuit_param::CircuitParam;
use crate::circuit::gate::{ClassicalDataOp, Directive, Instruction, StandardGate};
use crate::circuit::{
    Circuit, ClassicalCast, ClassicalCompareOp, ClassicalControlOp, ClassicalExprKind,
    ClassicalType, Qubit,
};
use crate::ir::qasm2::dump::dumps;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cqlib_qasm2_load_{}_{}_{}.qasm",
        std::process::id(),
        test_name,
        nonce
    ))
}

fn assert_standard_gate(
    circuit: &Circuit,
    op_idx: usize,
    expected_gate: StandardGate,
    expected_qubits: &[u32],
    expected_params: &[f64],
) {
    let ops = circuit.operations();
    assert!(
        op_idx < ops.len(),
        "Op index {} out of bounds (len {})",
        op_idx,
        ops.len()
    );
    let op = &ops[op_idx];

    match &op.instruction {
        Instruction::Standard(g) => assert_eq!(
            *g, expected_gate,
            "Op {}: Expected gate {:?}, found {:?}",
            op_idx, expected_gate, g
        ),
        _ => panic!(
            "Op {}: Expected StandardGate, found {:?}",
            op_idx, op.instruction
        ),
    }

    assert_eq!(
        op.qubits.len(),
        expected_qubits.len(),
        "Op {}: Qubit count mismatch",
        op_idx
    );
    for (i, &q_idx) in expected_qubits.iter().enumerate() {
        assert_eq!(
            op.qubits[i],
            Qubit::new(q_idx),
            "Op {}: Qubit mismatch at index {}",
            op_idx,
            i
        );
    }

    assert_eq!(
        op.params.len(),
        expected_params.len(),
        "Op {}: Param count mismatch",
        op_idx
    );
    for (i, &expected_val) in expected_params.iter().enumerate() {
        let val = match &op.params[i] {
            CircuitParam::Fixed(v) => *v,
            CircuitParam::Index(idx) => {
                let p = &circuit.parameters()[*idx as usize];
                p.evaluate(&None)
                    .unwrap_or_else(|_| panic!("Failed to evaluate param {}", idx))
            }
        };
        assert!(
            (val - expected_val).abs() < 1e-10,
            "Op {}: Param {} mismatch. Expected {}, got {}",
            op_idx,
            i,
            expected_val,
            val
        );
    }
}

#[test]
fn test_from_str_alias_matches_loads() {
    let source = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        h q[0];
    "#;

    let loaded = loads(source).unwrap();
    let aliased = from_str(source).unwrap();

    assert_eq!(aliased.num_qubits(), loaded.num_qubits());
    assert_eq!(aliased.operations().len(), loaded.operations().len());
}

#[test]
fn test_from_path_alias_reads_file() {
    let path = unique_temp_path("from_path");
    fs::write(
        &path,
        r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        x q[0];
    "#,
    )
    .unwrap();

    let circuit = from_path(path.as_path()).unwrap();

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!(circuit.operations().len(), 1);
    fs::remove_file(path).unwrap();
}

fn assert_directive(
    circuit: &Circuit,
    op_idx: usize,
    expected_directive: Directive,
    expected_qubits: &[u32],
) {
    let ops = circuit.operations();
    assert!(op_idx < ops.len(), "Op index {} out of bounds", op_idx);
    let op = &ops[op_idx];

    match &op.instruction {
        Instruction::Directive(d) => assert_eq!(
            *d, expected_directive,
            "Op {}: Expected {:?}, found {:?}",
            op_idx, expected_directive, d
        ),
        _ => panic!(
            "Op {}: Expected Directive, found {:?}",
            op_idx, op.instruction
        ),
    }

    assert_eq!(
        op.qubits.len(),
        expected_qubits.len(),
        "Op {}: Qubit count mismatch",
        op_idx
    );
    for (i, &q_idx) in expected_qubits.iter().enumerate() {
        assert_eq!(
            op.qubits[i],
            Qubit::new(q_idx),
            "Op {}: Qubit mismatch at index {}",
            op_idx,
            i
        );
    }
}

// --- Tests ---

#[test]
fn test_parse_simple_qasm() {
    let qasm = r#"
            OPENQASM 2.0;
            qreg q[2];
            h q[0];
            cx q[0], q[1];
        "#;
    let result = loads(qasm);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let circuit = result.unwrap();

    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(circuit.operations().len(), 2);

    assert_standard_gate(&circuit, 0, StandardGate::H, &[0], &[]);
    assert_standard_gate(&circuit, 1, StandardGate::CX, &[0, 1], &[]);
}

#[test]
fn test_scientific_notation_no_dot() {
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        rx(1e-5) q[0];
    "#;
    let result = loads(qasm);
    assert!(result.is_ok(), "Failed to parse 1e-5: {:?}", result.err());
    let circuit = result.unwrap();

    assert_standard_gate(&circuit, 0, StandardGate::RX, &[0], &[1e-5]);
}

#[test]
fn test_power_associativity() {
    // 2^3^2 should be 2^(3^2) = 512, not (2^3)^2 = 64
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        rx(2^3^2) q[0];
    "#;
    let result = loads(qasm);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let circuit = result.unwrap();

    assert_standard_gate(&circuit, 0, StandardGate::RX, &[0], &[512.0]);
}

#[test]
fn test_identifier_case_compliance() {
    // OpenQASM 2.0 identifiers must start with lowercase
    let qasm = r#"
        OPENQASM 2.0;
        qreg Q[1];
    "#;
    let result = loads(qasm);
    assert!(
        result.is_err(),
        "Should fail for uppercase identifier start 'Q'"
    );
}

#[test]
fn test_uppercase_identifier() {
    // MyGate starts with uppercase, should fail
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        gate MyGate a { h a; }
        MyGate q[0];
    "#;
    let result = loads(qasm);
    assert!(
        result.is_err(),
        "Should fail for uppercase gate name 'MyGate'"
    );
}

#[test]
fn test_u_cx_special_cases() {
    // U and CX are allowed as exceptions
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[2];
        U(pi/2, 0, pi) q[0];
        CX q[0], q[1];
    "#;
    let result = loads(qasm);
    assert!(
        result.is_ok(),
        "Failed to parse U and CX: {:?}",
        result.err()
    );
    let circuit = result.unwrap();

    assert_standard_gate(
        &circuit,
        0,
        StandardGate::U,
        &[0],
        &[std::f64::consts::PI / 2.0, 0.0, std::f64::consts::PI],
    );
    assert_standard_gate(&circuit, 1, StandardGate::CX, &[0, 1], &[]);
}

#[test]
fn test_measure_reset() {
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[2];
        creg c[2];
        reset q[0];
        barrier q;
        measure q[0] -> c[0];
    "#;
    let result = loads(qasm);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let circuit = result.unwrap();

    // 0. Initialize c to zero.
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    // 1. Reset q[0]
    assert_directive(&circuit, 1, Directive::Reset, &[0]);
    // 2. Barrier q[0], q[1] (q expanded)
    assert_directive(&circuit, 2, Directive::Barrier, &[0, 1]);
    // 3-4. Measure q[0] and store the result in c[0].
    assert!(matches!(
        circuit.operations()[3].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[4].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
}

#[test]
fn test_param_gate() {
    let qasm = r#"
            OPENQASM 2.0;
            qreg q[1];
            gate my_rx(theta) a { rx(theta) a; }
            my_rx(3.14) q[0];
        "#;
    let result = loads(qasm);
    assert!(result.is_ok());

    let circuit = result.unwrap();
    let ops = circuit.operations();
    assert_eq!(ops.len(), 1);

    // Verify CircuitGate structure
    if let Instruction::CircuitGate(cg) = &ops[0].instruction {
        assert_eq!(cg.name.as_str(), "my_rx");

        let inner_ops = cg.circuit.circuit.operations();
        assert_eq!(inner_ops.len(), 1);
        if let Instruction::Standard(StandardGate::RX) = &inner_ops[0].instruction {
            // Check if parameter is mapped correctly (Index 0 in inner circuit)
            match &inner_ops[0].params[0] {
                CircuitParam::Index(idx) => assert_eq!(*idx, 0), // 0th param of inner circuit
                _ => panic!("Expected Index parameter in inner gate"),
            }
        } else {
            panic!("Inner gate should be RX");
        }
    } else {
        panic!("Top level gate should be CircuitGate");
    }
}

#[test]
fn test_qelib1_gate_name_cannot_be_redefined() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate h a { rx(pi) a; }
        h q[0];
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ReservedGateName(ref name) if name == "h"));
    assert_eq!(
        error.to_string(),
        "Gate name 'h' is reserved by qelib1.inc and cannot be redefined"
    );
}

#[test]
fn test_qelib1_gate_name_cannot_be_declared_opaque() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        opaque h a;
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ReservedGateName(ref name) if name == "h"));
}

#[test]
fn test_builtin_qelib1_gate_declarations_are_allowed() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        ch q[0], q[1];
        "#,
    )
    .unwrap();

    let Instruction::CircuitGate(gate) = &circuit.operations()[0].instruction else {
        panic!("expected qelib1 ch to load as CircuitGate");
    };
    assert_eq!(gate.name.as_str(), "ch");
    assert_eq!(gate.num_qubits(), 2);
}

#[test]
fn test_register_broadcast_standard_gates() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        qreg r[3];
        h q;
        cx q,r;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 6);
    assert_standard_gate(&circuit, 0, StandardGate::H, &[0], &[]);
    assert_standard_gate(&circuit, 1, StandardGate::H, &[1], &[]);
    assert_standard_gate(&circuit, 2, StandardGate::H, &[2], &[]);
    assert_standard_gate(&circuit, 3, StandardGate::CX, &[0, 3], &[]);
    assert_standard_gate(&circuit, 4, StandardGate::CX, &[1, 4], &[]);
    assert_standard_gate(&circuit, 5, StandardGate::CX, &[2, 5], &[]);
}

#[test]
fn test_integer_overflow_reports_parse_error_instead_of_panicking() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        if(c==2037035976334486086268445688409378161051468393665936250636140449354381299763336706183397376) h q[0];
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ParseError(_)));
}

#[test]
fn test_qelib1_reserved_gate_names_match_static_tables() {
    use std::collections::HashSet;

    let declared_names: HashSet<&str> = QELIB1_DIRECT_GATES
        .iter()
        .chain(QELIB1_CUSTOM_GATES.iter())
        .copied()
        .collect();
    let reserved_names: HashSet<&str> = QELIB1_RESERVED_GATE_NAMES.iter().copied().collect();

    assert_eq!(reserved_names, declared_names);
}

#[test]
fn test_nested_gate() {
    let qasm = r#"
            OPENQASM 2.0;
            qreg q[2];
            gate my_h a { h a; }
            gate my_hh a, b { my_h a; my_h b; }
            my_hh q[0], q[1];
        "#;
    let result = loads(qasm);
    assert!(result.is_ok(), "Nested gate failed: {:?}", result.err());
    let circuit = result.unwrap();
    let ops = circuit.operations();
    assert_eq!(ops.len(), 1);

    if let Instruction::CircuitGate(cg) = &ops[0].instruction {
        assert_eq!(cg.name.as_str(), "my_hh");
        let inner_ops = cg.circuit.circuit.operations();
        assert_eq!(inner_ops.len(), 2);
        // Verify inner ops are ALSO CircuitGates (my_h)
        for op in inner_ops {
            if let Instruction::CircuitGate(inner_cg) = &op.instruction {
                assert_eq!(inner_cg.name.as_str(), "my_h");
            } else {
                panic!("Expected inner gate to be CircuitGate(my_h)");
            }
        }
    } else {
        panic!("Top level gate should be CircuitGate(my_hh)");
    }
}

#[test]
fn test_recursion_limit() {
    // This test checks for circular gate dependency detection
    // gate recursive a { recursive a; } is a self-referential gate (circular dependency)
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        gate recursive a { recursive a; }
        recursive q[0];
    "#;
    let result = loads(qasm);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Circular gate dependency") || err.contains("recursion"),
        "Got error: {}",
        err
    );
}

#[test]
fn test_circular_gate_dependency() {
    // Test A calls B, B calls A (circular dependency)
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        gate a q {
            b q;
        }
        gate b q {
            a q;
        }
        a q[0];
    "#;
    let result = loads(qasm);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Circular gate dependency"),
        "Got error: {}",
        err
    );
}

#[test]
fn test_compile_gate_rolls_back_state_after_circular_dependency() {
    let program = parser::ProgramBodyParser::new()
        .parse(
            r#"
            gate a q { b q; }
            gate b q { a q; }
            "#,
        )
        .unwrap();
    let mut converter = AstToCircuit::new(None, Box::new(NullResolver));
    converter.discovery_pass(&program).unwrap();

    assert!(matches!(
        converter.compile_gate_if_needed("a"),
        Err(QasmParseError::CircularGateDependency { .. })
    ));
    assert_eq!(converter.recursion_depth, 0);
    assert!(converter.compiling_gates.is_empty());
}

#[test]
fn test_compile_gate_rolls_back_state_after_body_build_failure() {
    let program = parser::ProgramBodyParser::new()
        .parse("gate broken q { x missing; }")
        .unwrap();
    let mut converter = AstToCircuit::new(None, Box::new(NullResolver));
    converter.discovery_pass(&program).unwrap();

    assert!(matches!(
        converter.compile_gate_if_needed("broken"),
        Err(QasmParseError::UndefinedQubit(name)) if name == "missing"
    ));
    assert_eq!(converter.recursion_depth, 0);
    assert!(converter.compiling_gates.is_empty());
}

#[test]
fn test_dumps() {
    let qs = (0..3).map(Qubit::new).collect::<Vec<_>>();
    let mut c = Circuit::new(3);
    c.h(qs[0]).unwrap();
    c.cx(qs[1], qs[2]).unwrap();
    let g = c.to_gate("g").unwrap();

    let qs = (0..4).map(Qubit::new).collect::<Vec<_>>();
    let mut c = Circuit::new(4);
    c.h(qs[0]).unwrap();
    c.append(g, vec![qs[1], qs[0], qs[3]], vec![], None)
        .unwrap();

    let qasm = dumps(&c);
    println!("{}", qasm.unwrap());

    // let c = load("/Users/gaojianjian/work/code/jianjian001/cqlib2/tests/qft_n18.qasm").unwrap();
    // let qasm = dumps(&c);
    // println!("{:?}", qasm);
}

#[test]
fn test_parameter_count_mismatch() {
    // Test U2 gate with only 1 parameter (should fail, needs 2)
    let qasm_u2_wrong = r#"
        OPENQASM 2.0;
        qreg q[1];
        u2(0.5) q[0];
    "#;

    let result = loads(qasm_u2_wrong);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Mismatched parameter count"),
        "Expected parameter count error, got: {}",
        err
    );

    // Test U3 gate with only 2 parameters (should fail, needs 3)
    let qasm_u3_wrong = r#"
        OPENQASM 2.0;
        qreg q[1];
        u3(0.5, 0.3) q[0];
    "#;

    let result = loads(qasm_u3_wrong);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Mismatched parameter count"),
        "Expected parameter count error, got: {}",
        err
    );

    // Test RX gate with no parameters (should fail, needs 1)
    let qasm_rx_wrong = r#"
        OPENQASM 2.0;
        qreg q[1];
        rx q[0];
    "#;

    let result = loads(qasm_rx_wrong);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Mismatched parameter count"),
        "Expected parameter count error, got: {}",
        err
    );

    // Test U2 gate with correct 2 parameters (should succeed)
    let qasm_u2_correct = r#"
        OPENQASM 2.0;
        qreg q[1];
        u2(0.5, 0.3) q[0];
    "#;

    let result = loads(qasm_u2_correct);
    assert!(result.is_ok(), "U2 with 2 params should succeed");

    // Test U3 gate with correct 3 parameters (should succeed)
    let qasm_u3_correct = r#"
        OPENQASM 2.0;
        qreg q[1];
        u3(0.5, 0.3, 0.2) q[0];
    "#;

    let result = loads(qasm_u3_correct);
    assert!(result.is_ok(), "U3 with 3 params should succeed");
}

#[test]
fn test_if_statement_uses_classical_data_and_control_ir() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[2];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        if (c == 1) x q[1];
        "#,
    )
    .unwrap();

    assert_eq!(
        circuit.classical_vars(),
        &[ClassicalType::bit_vec(1).unwrap()]
    );
    assert_eq!(circuit.operations().len(), 5);
    assert!(matches!(
        circuit.operations()[2].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        circuit.operations()[3].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations()[4].instruction
    else {
        panic!("expected expression-based if operation");
    };
    assert!(if_op.else_body().is_none());
    assert_eq!(if_op.then_body().operations().len(), 1);
    assert!(matches!(
        if_op.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    let ClassicalExprKind::Compare { op, lhs, rhs } = if_op.condition().kind() else {
        panic!("expected equality condition");
    };
    assert_eq!(*op, ClassicalCompareOp::Eq);
    assert!(matches!(
        lhs.kind(),
        ClassicalExprKind::Cast {
            cast: ClassicalCast::BitVecToUInt,
            ..
        }
    ));
    assert!(matches!(
        rhs.kind(),
        ClassicalExprKind::UIntLiteral { value: 1, .. }
    ));
}

#[test]
fn test_if_statement_allows_conditional_measurement() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        creg d[1];
        if (c == 1) measure q[0] -> d[0];
        "#,
    )
    .unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations()[2].instruction
    else {
        panic!("expected if operation");
    };
    let body = if_op.then_body().operations();
    assert_eq!(body.len(), 2);
    assert!(matches!(
        body[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { .. })
    ));
    assert!(matches!(
        body[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
}

#[test]
fn test_if_statement_rejects_conditional_barrier() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        if (c == 1) barrier q;
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ParseError(_)));
}

#[test]
fn test_indexed_condition_is_rejected() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[3];
        if (c[2] == 1) x q[0];
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ParseError(_)));
}

#[test]
fn test_gate_body_rejects_measurement_reset_if_and_indexed_barrier() {
    for qasm in [
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        gate bad a { measure a -> c[0]; }
        bad q[0];
        "#,
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate bad a { reset a; }
        bad q[0];
        "#,
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        gate bad a { if (c == 1) x a; }
        bad q[0];
        "#,
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate bad a { barrier a[0]; }
        bad q[0];
        "#,
    ] {
        assert!(
            loads(qasm).is_err(),
            "invalid gate body should fail: {qasm}"
        );
    }
}

#[test]
fn test_condition_references_correct_classical_register() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg first[2];
        creg second[3];
        if (second == 1) x q[0];
        "#,
    )
    .unwrap();

    let Instruction::ClassicalData(ClassicalDataOp::Store {
        target: first_var, ..
    }) = &circuit.operations()[0].instruction
    else {
        panic!("expected first classical-register initializer");
    };
    let Instruction::ClassicalData(ClassicalDataOp::Store {
        target: second_var, ..
    }) = &circuit.operations()[1].instruction
    else {
        panic!("expected second classical-register initializer");
    };
    assert_ne!(first_var, second_var);
    assert_eq!(first_var.ty(), ClassicalType::bit_vec(2).unwrap());
    assert_eq!(second_var.ty(), ClassicalType::bit_vec(3).unwrap());

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations()[2].instruction
    else {
        panic!("expected if operation");
    };
    let ClassicalExprKind::Compare { lhs, .. } = if_op.condition().kind() else {
        panic!("expected equality condition");
    };
    let ClassicalExprKind::Cast {
        cast: ClassicalCast::BitVecToUInt,
        expr,
    } = lhs.kind()
    else {
        panic!("expected bit-vector to uint cast");
    };
    assert!(matches!(
        expr.kind(),
        ClassicalExprKind::Var(var) if var == second_var
    ));
    assert!(!matches!(
        expr.kind(),
        ClassicalExprKind::Var(var) if var == first_var
    ));
}

#[test]
fn test_if_before_measurement_reads_zero_initialized_creg() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        if (c == 0) x q[0];
        "#,
    )
    .unwrap();

    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
}

#[test]
fn test_multibit_creg_condition_preserves_little_endian_value() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[3];
        creg c[3];
        measure q -> c;
        if (c == 5) x q[0];
        "#,
    )
    .unwrap();

    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert!(matches!(
        circuit.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. })
    ));
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations()[3].instruction
    else {
        panic!("expected if operation");
    };
    let ClassicalExprKind::Compare { lhs, rhs, .. } = if_op.condition().kind() else {
        panic!("expected equality condition");
    };
    assert!(matches!(
        lhs.kind(),
        ClassicalExprKind::Cast {
            cast: ClassicalCast::BitVecToUInt,
            ..
        }
    ));
    assert!(matches!(
        rhs.kind(),
        ClassicalExprKind::UIntLiteral { value: 5, .. }
    ));
}

#[test]
fn test_condition_value_must_fit_creg_width() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[2];
        if (c == 4) x q[0];
        "#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("does not fit"));
}

#[test]
fn test_zero_width_register_is_rejected() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[0];
        "#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("positive width"));
}

#[test]
fn test_if_statement_parameter_evaluation() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        if (c == 0) rx(pi/2) q[0];
        "#,
    )
    .unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &circuit.operations()[1].instruction
    else {
        panic!("expected if operation");
    };
    let body_op = &if_op.then_body().operations()[0];
    assert!(matches!(
        body_op.instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    assert!(matches!(
        body_op.params[0],
        CircuitParam::Fixed(value) if (value - std::f64::consts::FRAC_PI_2).abs() < 1e-10
    ));
}

#[test]
fn test_if_statement_undefined_symbol_fails() {
    // OpenQASM 2.0 does not support global variables or parameters.
    // Using undefined symbols like 'theta' should be an error.
    let qasm_undefined = r#"
        OPENQASM 2.0;
        qreg q[1];
        creg c[1];
        if (c==1) rx(theta) q[0];
    "#;

    let result = loads(qasm_undefined);
    assert!(
        result.is_err(),
        "Undefined symbol 'theta' should cause an error"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Unknown parameter") || err.contains("Evaluation error"),
        "Expected 'Unknown parameter' or 'Evaluation error', got: {}",
        err
    );
}

#[test]
fn test_memory_resolver_include() {
    // Test the source resolver abstraction using a mock memory resolver
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    /// A mock resolver that serves files from memory
    struct MemoryResolver {
        files: HashMap<PathBuf, String>,
    }

    impl MemoryResolver {
        fn new() -> Self {
            let mut files = HashMap::new();
            // Add a mock include file
            files.insert(
                PathBuf::from("mylib.inc"),
                r#"
                gate my_gate a {
                    h a;
                    x a;
                }
                "#
                .to_string(),
            );
            Self { files }
        }
    }

    impl QasmSourceResolver for MemoryResolver {
        fn resolve_source(&self, path: &Path) -> Result<String, io::Error> {
            self.files.get(path).cloned().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("File not found: {path:?}"))
            })
        }
    }

    // Test parsing with the mock resolver
    let qasm_with_include = r#"
        OPENQASM 2.0;
        include "mylib.inc";
        qreg q[1];
        my_gate q[0];
    "#;

    // Use the internal parser with our mock resolver
    let resolver = Box::new(MemoryResolver::new());
    let result = parse_qasm_with_context(qasm_with_include, None, resolver);

    assert!(
        result.is_ok(),
        "Memory resolver should work: {:?}",
        result.err()
    );
    let circuit = result.unwrap();
    assert_eq!(circuit.num_qubits(), 1);
}

#[test]
fn test_qelib1_gate_name_cannot_be_redefined_in_external_include() {
    use std::io;
    use std::path::Path;

    struct ReservedGateResolver;

    impl QasmSourceResolver for ReservedGateResolver {
        fn resolve_source(&self, path: &Path) -> Result<String, io::Error> {
            if path == Path::new("reserved.inc") {
                Ok("gate h a { rx(pi) a; }".to_string())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File not found: {path:?}"),
                ))
            }
        }
    }

    let error = parse_qasm_with_context(
        r#"
        OPENQASM 2.0;
        include "reserved.inc";
        qreg q[1];
        "#,
        None,
        Box::new(ReservedGateResolver),
    )
    .unwrap_err();

    assert!(matches!(error, QasmParseError::ReservedGateName(ref name) if name == "h"));
}

#[test]
fn test_load_file_io_error_preserves_source() {
    let error = load("definitely_missing_qasm2_file.qasm").unwrap_err();
    assert!(matches!(error, QasmParseError::IoError(_)));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn test_null_resolver_recludes_includes() {
    // Test that NullResolver rejects include statements
    let qasm_with_include = r#"
        OPENQASM 2.0;
        include "somefile.inc";
        qreg q[1];
    "#;

    // loads uses NullResolver, so includes should fail
    let result = loads(qasm_with_include);
    assert!(result.is_err(), "Include in raw string mode should fail");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("raw string mode") || err.contains("Cannot include"),
        "Expected error about raw string mode, got: {}",
        err
    );
}

#[test]
fn test_custom_gate_parameter_binding_uses_declared_order() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate g(a, b) q {
            rx(b) q;
            ry(a) q;
        }
        g(1.0, 2.0) q[0];
        "#,
    )
    .unwrap();

    let decomposed = circuit.decompose().unwrap();
    assert_standard_gate(&decomposed, 0, StandardGate::RX, &[0], &[2.0]);
    assert_standard_gate(&decomposed, 1, StandardGate::RY, &[0], &[1.0]);
}

#[test]
fn test_custom_gate_declared_but_unused_parameters_keep_call_arity() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate g(theta) q { x q; }
        g(pi) q[0];
        "#,
    )
    .unwrap();

    let decomposed = circuit.decompose().unwrap();
    assert_standard_gate(&decomposed, 0, StandardGate::X, &[0], &[]);
}

#[test]
fn test_custom_gate_parameter_expressions_substitute_by_formal_name() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate g(a, b) q { rx(a + b) q; }
        g(1.0, 2.0) q[0];
        "#,
    )
    .unwrap();

    let decomposed = circuit.decompose().unwrap();
    assert_standard_gate(&decomposed, 0, StandardGate::RX, &[0], &[3.0]);
}

#[test]
fn test_gate_invocation_broadcasts_single_register_argument() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[4];
        x q;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 4);
    for index in 0..4 {
        assert_standard_gate(&circuit, index, StandardGate::X, &[index as u32], &[]);
    }
}

#[test]
fn test_gate_invocation_broadcasts_multiple_register_arguments_by_index() {
    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg a[3];
        qreg b[3];
        cx a, b;
        "#,
    )
    .unwrap();

    assert_eq!(circuit.operations().len(), 3);
    assert_standard_gate(&circuit, 0, StandardGate::CX, &[0, 3], &[]);
    assert_standard_gate(&circuit, 1, StandardGate::CX, &[1, 4], &[]);
    assert_standard_gate(&circuit, 2, StandardGate::CX, &[2, 5], &[]);
}

#[test]
fn test_gate_invocation_rejects_mismatched_register_broadcast_widths() {
    let error = loads(
        r#"
        OPENQASM 2.0;
        qreg a[2];
        qreg b[3];
        cx a, b;
        "#,
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("width")
            || error.to_string().contains("broadcast")
            || error.to_string().contains("Mismatched")
    );
}

#[test]
fn test_parser_rejects_empty_lists_and_trailing_commas() {
    for qasm in [
        r#"OPENQASM 2.0; qreg q[1]; gate g { }"#,
        r#"OPENQASM 2.0; qreg q[1]; barrier ;"#,
        r#"OPENQASM 2.0; qreg q[1]; x ;"#,
        r#"OPENQASM 2.0; qreg q[1]; x q[0],;"#,
    ] {
        assert!(
            parser::MainParser::new().parse(qasm).is_err(),
            "invalid list syntax should fail to parse: {qasm}"
        );
    }
}

#[test]
fn test_parser_rejects_non_uop_statements_inside_gate_body() {
    for qasm in [
        r#"OPENQASM 2.0; qreg q[1]; creg c[1]; gate bad a { measure a -> c[0]; }"#,
        r#"OPENQASM 2.0; qreg q[1]; gate bad a { reset a; }"#,
        r#"OPENQASM 2.0; qreg q[1]; creg c[1]; gate bad a { if (c == 1) x a; }"#,
    ] {
        assert!(
            parser::MainParser::new().parse(qasm).is_err(),
            "invalid gate body should fail to parse: {qasm}"
        );
    }
}

#[test]
fn test_parser_rejects_non_qop_if_body() {
    for qasm in [
        r#"OPENQASM 2.0; qreg q[1]; creg c[1]; if (c == 1) barrier q;"#,
        r#"OPENQASM 2.0; qreg q[1]; creg c[1]; if (c == 1) qreg r[1];"#,
        r#"OPENQASM 2.0; qreg q[1]; creg c[1]; if (c == 1) gate g a { x a; }"#,
    ] {
        assert!(
            parser::MainParser::new().parse(qasm).is_err(),
            "invalid if-body should fail to parse: {qasm}"
        );
    }
}

#[test]
fn test_duplicate_declarations_and_namespace_conflicts_are_rejected() {
    for qasm in [
        r#"OPENQASM 2.0; qreg q[1]; qreg q[2];"#,
        r#"OPENQASM 2.0; qreg x[1]; creg x[1];"#,
        r#"OPENQASM 2.0; gate g a { x a; } gate g b { h b; } qreg q[1]; g q[0];"#,
    ] {
        assert!(
            loads(qasm).is_err(),
            "duplicate declaration or namespace conflict should fail: {qasm}"
        );
    }
}

#[test]
fn test_gate_formal_name_conflicts_are_rejected() {
    for qasm in [
        r#"OPENQASM 2.0; gate g a, a { x a; } qreg q[1]; g q[0];"#,
        r#"OPENQASM 2.0; gate g(theta, theta) q { rx(theta) q; } qreg q[1]; g(1.0) q[0];"#,
        r#"OPENQASM 2.0; gate g(theta) theta { x theta; } qreg q[1]; g(1.0) q[0];"#,
    ] {
        assert!(
            loads(qasm).is_err(),
            "duplicate or conflicting gate formals should fail: {qasm}"
        );
    }
}

#[test]
fn test_nested_include_paths_are_resolved_relative_to_including_file() {
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    struct NestedResolver {
        files: HashMap<PathBuf, String>,
    }

    impl QasmSourceResolver for NestedResolver {
        fn resolve_source(&self, path: &Path) -> Result<String, io::Error> {
            self.files.get(path).cloned().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("File not found: {path:?}"))
            })
        }
    }

    let files = HashMap::from([
        (
            PathBuf::from("root/lib/a.inc"),
            r#"include "b.inc"; gate outer a { inner a; }"#.to_string(),
        ),
        (
            PathBuf::from("root/lib/b.inc"),
            r#"gate inner a { x a; }"#.to_string(),
        ),
    ]);
    let resolver = Box::new(NestedResolver { files });

    let circuit = parse_qasm_with_context(
        r#"
        OPENQASM 2.0;
        include "lib/a.inc";
        qreg q[1];
        outer q[0];
        "#,
        Some(PathBuf::from("root")),
        resolver,
    )
    .unwrap();

    assert_eq!(circuit.num_qubits(), 1);
}

#[test]
fn test_qasm2_loader_rejects_unsupported_version_and_invalid_integer_forms() {
    assert!(
        loads("OPENQASM 3.0; qreg q[1];").is_err(),
        "QASM2 loader should reject non-2.0 versions"
    );
    assert!(
        parser::MainParser::new()
            .parse("OPENQASM 2.0; qreg q[01];")
            .is_err(),
        "nninteger should reject leading zeroes"
    );
    assert!(
        loads("OPENQASM 2.0; qreg q[999999999999999999999999999999];").is_err(),
        "oversized integer literals should return an error instead of panicking"
    );
}

#[test]
fn test_uppercase_gate_names_follow_qasm2_identifier_rules() {
    for qasm in [
        r#"OPENQASM 2.0; qreg q[1]; X q[0];"#,
        r#"OPENQASM 2.0; qreg q[1]; U2(0, 0) q[0];"#,
        r#"OPENQASM 2.0; qreg q[1]; RX(0) q[0];"#,
    ] {
        assert!(
            parser::MainParser::new().parse(qasm).is_err(),
            "uppercase non-special gate should fail to parse: {qasm}"
        );
    }

    assert!(
        parser::MainParser::new()
            .parse("OPENQASM 2.0; qreg q[2]; U(0, 0, 0) q[0]; CX q[0], q[1];")
            .is_ok(),
        "uppercase U and CX are QASM2 special cases"
    );
}

#[test]
fn test_opaque_invocations_and_duplicate_opaque_names_are_rejected() {
    for qasm in [
        r#"OPENQASM 2.0; opaque foo a; qreg q[1]; foo q[0];"#,
        r#"OPENQASM 2.0; opaque foo(theta) a; qreg q[1]; foo(pi) q[0];"#,
        r#"OPENQASM 2.0; opaque foo a; gate foo b { x b; } qreg q[1]; foo q[0];"#,
    ] {
        assert!(
            loads(qasm).is_err(),
            "opaque invocation or duplicate opaque name should fail: {qasm}"
        );
    }
}

#[test]
fn test_division_by_zero_is_reported_without_panic_after_parameter_binding() {
    assert!(
        loads("OPENQASM 2.0; qreg q[1]; rx(1/0) q[0];").is_err(),
        "constant division by zero should fail during load"
    );

    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate g(theta) q { rx(1/theta) q; }
        g(2) q[0];
        "#,
    )
    .unwrap();
    let decomposed = circuit.decompose().unwrap();
    assert_standard_gate(&decomposed, 0, StandardGate::RX, &[0], &[0.5]);

    let circuit = loads(
        r#"
        OPENQASM 2.0;
        qreg q[1];
        gate g(theta) q { rx(1/theta) q; }
        g(0) q[0];
        "#,
    )
    .unwrap();
    let result = std::panic::catch_unwind(|| circuit.decompose());
    assert!(
        result.is_ok(),
        "decompose should return an error, not panic"
    );
    assert!(
        result.unwrap().is_err(),
        "division by zero after parameter binding should be an error"
    );
}
