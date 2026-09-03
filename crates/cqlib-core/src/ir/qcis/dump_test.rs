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
use crate::circuit::Qubit;
use crate::circuit::circuit_param::ParameterValue;
use crate::circuit::parameter::Parameter;
use crate::circuit::{Circuit, ClassicalExpr, ClassicalType};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cqlib_qcis_dump_{}_{}_{}.qcis",
        std::process::id(),
        test_name,
        nonce
    ))
}

#[test]
fn test_dump_simple_gates() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.x(q0).unwrap();
    c.y(q1).unwrap();
    c.z(q0).unwrap();
    c.h(q1).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"X Q0
Y Q1
Z Q0
H Q1
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_file_accepts_path_like_inputs() {
    let mut c = Circuit::new(1);
    c.h(Qubit::new(0)).unwrap();

    let path = unique_temp_path("path");
    dump(&c, path.as_path()).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "H Q0\n");
    fs::remove_file(&path).unwrap();

    let path = unique_temp_path("str");
    dump(&c, path.to_str().unwrap()).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "H Q0\n");
    fs::remove_file(&path).unwrap();
}

#[test]
fn test_to_string_alias_matches_dumps() {
    let mut c = Circuit::new(1);
    c.h(Qubit::new(0)).unwrap();

    assert_eq!(to_string(&c).unwrap(), dumps(&c).unwrap());
}

#[test]
fn test_to_path_alias_writes_file() {
    let mut c = Circuit::new(1);
    c.x(Qubit::new(0)).unwrap();
    let path = unique_temp_path("to_path");

    to_path(&c, path.as_path()).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), dumps(&c).unwrap());
    fs::remove_file(path).unwrap();
}

#[test]
fn test_dump_file_io_error_preserves_source() {
    let circuit = Circuit::new(0);
    let path = unique_temp_path("missing_parent").join("out.qcis");

    let error = dump(&circuit, path).unwrap_err();

    assert!(matches!(error, QcisDumpError::IoError(_)));
    assert!(error.source().is_some());
}

#[test]
fn test_dump_native_qcis_gates() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.x2p(q0).unwrap();
    c.x2m(q1).unwrap();
    c.cz(q0, q1).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"X2P Q0
X2M Q1
CZ Q0 Q1
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_xy_gates() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.xy2p(q0, 1.0).unwrap();
    c.xy2m(q1, std::f64::consts::PI / 2.0).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"XY2P Q0 1
XY2M Q1 pi/2
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_parameterized_gates() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.rx(q0, 1.0).unwrap();
    c.ry(q1, std::f64::consts::PI / 2.0).unwrap();
    c.rz(q0, std::f64::consts::PI).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"RX Q0 1
RY Q1 pi/2
RZ Q0 pi
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_rxy_gate() {
    let mut c = Circuit::new(1);
    let q0 = Qubit::new(0);

    c.rxy(q0, 1.0, 2.0).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = "RXY Q0 1 2\n";
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_s_and_t_gates() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.s(q0).unwrap();
    c.sdg(q1).unwrap();
    c.t(q0).unwrap();
    c.tdg(q1).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"S Q0
SD Q1
T Q0
TD Q1
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_measurement() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.measure(q0).unwrap();
    c.measure(q1).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"M Q0
M Q1
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_measure_bits_preserves_qubit_order() {
    let mut c = Circuit::new(3);
    c.measure_bits([Qubit::new(2), Qubit::new(0), Qubit::new(1)])
        .unwrap();

    assert_eq!(dumps(&c).unwrap(), "M Q2 Q0 Q1\n");
}

#[test]
fn test_dump_rejects_classical_store() {
    let mut c = Circuit::new(1);
    let var = c.var(ClassicalType::Bit);
    c.store(var, ClassicalExpr::bit_literal(false)).unwrap();

    assert!(matches!(
        dumps(&c),
        Err(QcisDumpError::UnsupportedClassicalData(_))
    ));
}

#[test]
fn test_dump_rejects_classical_control() {
    let mut c = Circuit::new(1);
    c.if_(ClassicalExpr::bool_literal(true), |body| {
        body.x(Qubit::new(0))?;
        Ok(())
    })
    .unwrap();

    assert!(matches!(
        dumps(&c),
        Err(QcisDumpError::UnsupportedClassicalControl(_))
    ));
}

#[test]
fn test_dump_barrier() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.barrier(vec![q0, q1]).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = "B Q0 Q1\n";
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_delay() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.delay(q0, ParameterValue::Fixed(100.0)).unwrap();
    c.delay(q1, ParameterValue::Fixed(50.0)).unwrap();

    let qcis = dumps(&c).unwrap();
    let expected = r#"I Q0 100
I Q1 50
"#;
    assert_eq!(qcis, expected);
}

#[test]
fn test_dump_rejects_identity_gate_as_qcis_delay() {
    let mut c = Circuit::new(1);
    c.i(Qubit::new(0)).unwrap();

    let result = dumps(&c);

    assert!(matches!(
        result,
        Err(QcisDumpError::UnsupportedGate(gate)) if gate == "I"
    ));
}

#[test]
fn test_dump_rejects_non_integer_delay_tick() {
    let mut c = Circuit::new(1);
    c.delay(Qubit::new(0), ParameterValue::Fixed(1.5)).unwrap();

    let result = dumps(&c);

    assert!(matches!(
        result,
        Err(QcisDumpError::InvalidDelayParameter(reason)) if reason.contains("integer")
    ));
}

#[test]
fn test_format_float() {
    assert_eq!(format_float(0.0), "0");
    assert_eq!(format_float(1.0), "1");
    assert_eq!(format_float(-1.0), "-1");
    assert_eq!(format_float(std::f64::consts::PI), "pi");
    assert_eq!(format_float(std::f64::consts::PI / 2.0), "pi/2");
    assert_eq!(format_float(-std::f64::consts::PI / 2.0), "-pi/2");
    assert_eq!(format_float(std::f64::consts::PI / 4.0), "pi/4");
    assert_eq!(format_float(42.0), "42");
    assert_eq!(format_float(2.5), "2.5");
}

#[test]
fn test_roundtrip_load_dump() {
    use crate::ir::qcis::load::loads;

    let original_qcis = r#"
        RX Q0 1.0
        RY Q1 pi/2
        CZ Q0 Q1
        M Q0 Q1
    "#;

    let circuit = loads(original_qcis).unwrap();
    let dumped_qcis = dumps(&circuit).unwrap();

    // Verify the dumped QCIS content
    let expected = "RX Q0 1\nRY Q1 pi/2\nCZ Q0 Q1\nM Q0\nM Q1\n";
    assert_eq!(dumped_qcis, expected);

    // Load the dumped QCIS and verify it works
    let circuit2 = loads(&dumped_qcis).unwrap();
    assert_eq!(circuit2.num_qubits(), 2);

    // Verify the reloaded circuit has the same operations
    let dumped_again = dumps(&circuit2).unwrap();
    assert_eq!(dumped_again, expected);
}

#[test]
fn test_dump_cx() {
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    c.cx(q0, q1).unwrap();

    assert_eq!(dumps(&c).unwrap(), "CX Q0 Q1\n");
}

#[test]
fn test_dump_rejects_gphase() {
    let mut c = Circuit::new(0);
    c.append(
        crate::circuit::gate::Instruction::Standard(crate::circuit::gate::StandardGate::GPhase),
        std::iter::empty::<Qubit>(),
        [ParameterValue::Fixed(0.1)],
        None,
    )
    .unwrap();

    assert!(matches!(
        dumps(&c),
        Err(QcisDumpError::UnsupportedGate(gate)) if gate == "GPhase"
    ));
}

#[test]
fn test_dump_symbolic_parameters() {
    // Test that symbolic parameters are exported for visual inspection
    // Note: This is for visualization only, QCIS backend does not support symbolic parameters
    let mut c = Circuit::new(2);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);

    let theta = Parameter::try_from("theta").unwrap();
    let phi = Parameter::try_from("phi").unwrap();

    c.rx(q0, theta.clone()).unwrap();
    c.ry(q1, phi.clone()).unwrap();
    c.rz(q0, theta.clone() + 0.5).unwrap();
    c.rxy(q1, theta.clone(), phi.clone()).unwrap();

    let qcis = dumps(&c).unwrap();

    // Verify symbolic parameters appear in output
    let expected = r#"RX Q0 theta
RY Q1 phi
RZ Q0 0.5 + theta
RXY Q1 theta phi
"#;
    assert_eq!(qcis, expected);
}
