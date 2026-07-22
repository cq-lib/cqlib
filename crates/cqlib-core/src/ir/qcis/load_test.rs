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
use crate::circuit::gate::{Instruction, StandardGate};
use crate::ir::qcis_dumps;
use std::error::Error;
use std::f64::consts::PI;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cqlib_qcis_load_{}_{}_{}.qcis",
        std::process::id(),
        test_name,
        nonce
    ))
}

#[test]
fn test_parse_simple_params() {
    let p = parse_param("1.0").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - 1.0).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }

    let p = parse_param("42").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - 42.0).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }
}

#[test]
fn test_parse_pi_expression() {
    // pi
    let p = parse_param("pi").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - PI).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }

    // pi/2
    let p = parse_param("pi/2").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - PI / 2.0).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }

    // pi/2+1
    let p = parse_param("pi/2+1").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - (PI / 2.0 + 1.0)).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }
}

#[test]
fn test_parse_complex_expression() {
    let p = parse_param("2*pi").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - 2.0 * PI).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }

    let p = parse_param("pi/4+0.5").unwrap();
    match p {
        ParameterValue::Fixed(v) => assert!((v - (PI / 4.0 + 0.5)).abs() < 1e-10),
        _ => panic!("Expected fixed value"),
    }
}

#[test]
fn test_parse_symbolic() {
    let p = parse_param("theta").unwrap();
    match p {
        ParameterValue::Param(param) => {
            let mut expected = std::collections::HashSet::new();
            expected.insert("theta".to_string());
            assert_eq!(param.get_symbols(), expected);
        }
        _ => panic!("Expected param"),
    }
}

#[test]
fn test_loads_qcis() {
    let qcis = r#"
            RX Q0 1.0
            RY Q1 pi/2
            RZ Q2 pi/4+0.5
            RXY Q3 pi/2 3.14
            X Q0
            CZ Q0 Q1
        "#;

    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 4);
    assert_eq!(
        "RX Q0 1\nRY Q1 pi/2\nRZ Q2 1.2853981634\nRXY Q3 pi/2 3.14\nX Q0\nCZ Q0 Q1\n",
        qcis_dumps(&circuit).unwrap()
    );
}

#[test]
fn test_from_str_alias_matches_loads() {
    let source = "H Q0\nCZ Q0 Q1\n";

    let loaded = loads(source).unwrap();
    let aliased = from_str(source).unwrap();

    assert_eq!(aliased.num_qubits(), loaded.num_qubits());
    assert_eq!(qcis_dumps(&aliased).unwrap(), qcis_dumps(&loaded).unwrap());
}

#[test]
fn test_load_file_accepts_path_like_inputs() {
    let path = unique_temp_path("success");
    fs::write(&path, "H Q0\n").unwrap();

    let circuit = load(path.as_path()).unwrap();
    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!("H Q0\n", qcis_dumps(&circuit).unwrap());

    let circuit = load(path.to_str().unwrap()).unwrap();
    assert_eq!(circuit.num_qubits(), 1);

    fs::remove_file(&path).unwrap();
}

#[test]
fn test_from_path_alias_reads_file() {
    let path = unique_temp_path("from_path");
    fs::write(&path, "X Q0\n").unwrap();

    let circuit = from_path(path.as_path()).unwrap();

    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!("X Q0\n", qcis_dumps(&circuit).unwrap());
    fs::remove_file(path).unwrap();
}

#[test]
fn test_load_file_returns_io_error() {
    let path = unique_temp_path("missing");
    let _ = fs::remove_file(&path);

    let result = load(path.as_path());

    assert!(matches!(
        result,
        Err(QcisParseError::IoError(error)) if error.kind() == std::io::ErrorKind::NotFound
    ));

    let error = load(path.as_path()).unwrap_err();
    assert!(error.source().is_some());
}

#[test]
fn test_load_file_returns_parse_error() {
    let path = unique_temp_path("invalid");
    fs::write(&path, "RX q0 1.0\n").unwrap();

    let result = load(&path);

    assert!(matches!(
        result,
        Err(QcisParseError::InvalidQubitFormat(token)) if token == "q0"
    ));
    fs::remove_file(&path).unwrap();
}

#[test]
fn test_loads_with_comments() {
    let qcis = r#"
            // This is a comment
            RX Q0 1.0 // inline comment
            // Another comment
            RY Q1 pi/2
            M Q0 Q1
        "#;

    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(
        "RX Q0 1\nRY Q1 pi/2\nM Q0\nM Q1\n",
        qcis_dumps(&circuit).unwrap()
    );
}

#[test]
fn test_invalid_qubit_format() {
    let qcis = "RX q0 1.0"; // lowercase 'q'
    let result = loads(qcis);
    assert!(matches!(result, Err(QcisParseError::InvalidQubitFormat(_))));
}

#[test]
fn test_qubit_count_mismatch_rx() {
    // RX requires exactly 1 qubit
    let qcis = "RX Q0 Q1 1.0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::QubitCountMismatch {
            gate,
            expected: 1,
            actual: 2,
        }) if gate == "RX"
    ));
}

#[test]
fn test_parameter_count_mismatch_rx() {
    // RX requires exactly 1 parameter
    let qcis = "RX Q0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 1,
            actual: 0,
        }) if gate == "RX"
    ));
}

#[test]
fn test_parameter_count_mismatch_rxy() {
    // RXY requires exactly 2 parameters
    let qcis = "RXY Q0 1.0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 2,
            actual: 1,
        }) if gate == "RXY"
    ));
}

#[test]
fn test_cz_qubit_count_mismatch() {
    // CZ requires exactly 2 qubits
    let qcis = "CZ Q0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::QubitCountMismatch {
            gate,
            expected: 2,
            actual: 1,
        }) if gate == "CZ"
    ));
}

#[test]
fn test_cz_too_many_qubits() {
    // CZ requires exactly 2 qubits
    let qcis = "CZ Q0 Q1 Q2";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::QubitCountMismatch {
            gate,
            expected: 2,
            actual: 3,
        }) if gate == "CZ"
    ));
}

#[test]
fn test_single_qubit_gate_no_params() {
    // X, Y, Z, H, S, T gates require 0 parameters
    let qcis = "X Q0 1.0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 0,
            actual: 1,
        }) if gate == "X"
    ));
}

#[test]
fn test_barrier_single_qubit() {
    // Barrier supports 1 or more qubits
    let qcis = "B Q0";
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!("B Q0\n", qcis_dumps(&circuit).unwrap());
}

#[test]
fn test_barrier_multiple_qubits() {
    let qcis = "B Q0 Q1 Q2 Q3";
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 4);
    assert_eq!("B Q0 Q1 Q2 Q3\n", qcis_dumps(&circuit).unwrap());
}

#[test]
fn test_measurement_single_qubit() {
    let qcis = "M Q0";
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 1);
    assert_eq!("M Q0\n", qcis_dumps(&circuit).unwrap());
}

#[test]
fn test_measurement_multiple_qubits() {
    let qcis = "M Q0 Q1 Q2";
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 3);
    assert_eq!("M Q0\nM Q1\nM Q2\n", qcis_dumps(&circuit).unwrap());
}

#[test]
fn test_xy2p_requires_one_param() {
    // XY2P requires exactly 1 parameter
    let qcis = "XY2P Q0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 1,
            actual: 0,
        }) if gate == "XY2P"
    ));
}

#[test]
fn test_delay_requires_one_param() {
    // I (delay) requires exactly 1 parameter
    let qcis = "I Q0";
    let result = loads(qcis);
    assert!(matches!(
        result,
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 1,
            actual: 0,
        }) if gate == "I"
    ));
}

#[test]
fn test_qcis_i_loads_as_delay_not_identity_gate() {
    let circuit = loads("I Q0 1").unwrap();
    let operations = circuit.operations();

    assert_eq!(operations.len(), 1);
    assert!(matches!(operations[0].instruction, Instruction::Delay));
    assert_eq!("I Q0 1\n", qcis_dumps(&circuit).unwrap());
}

#[test]
fn test_qcis_delay_rejects_non_integer_tick() {
    let result = loads("I Q0 1.5");

    assert!(matches!(
        result,
        Err(QcisParseError::InvalidParameter { gate, param, .. })
            if gate == "I" && param == "1.5"
    ));
}

#[test]
fn test_qcis_delay_rejects_symbolic_tick() {
    let result = loads("I Q0 theta");

    assert!(matches!(
        result,
        Err(QcisParseError::InvalidParameter { gate, param, .. })
            if gate == "I" && param == "theta"
    ));
}

#[test]
fn test_valid_native_gates() {
    let qcis = r#"
        X2P Q0
        X2M Q1
        Y2P Q2
        Y2M Q3
        XY2P Q4 1.0
        XY2M Q5 pi/2
        CZ Q0 Q1
        RZ Q2 0.5
        I Q3 1.0
    "#;
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 6);
    assert_eq!(
        "X2P Q0\nX2M Q1\nY2P Q2\nY2M Q3\nXY2P Q4 1\nXY2M Q5 pi/2\nCZ Q0 Q1\nRZ Q2 0.5\nI Q3 1\n",
        qcis_dumps(&circuit).unwrap()
    );
}

#[test]
fn test_valid_standard_gates() {
    let qcis = r#"
        X Q0
        Y Q1
        Z Q2
        H Q3
        S Q4
        SD Q5
        T Q6
        TD Q7
        RX Q8 1.0
        RY Q9 pi/2
        RXY Q10 1.0 0.5
    "#;
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 11);
    assert_eq!(
        "X Q0\nY Q1\nZ Q2\nH Q3\nS Q4\nSD Q5\nT Q6\nTD Q7\nRX Q8 1\nRY Q9 pi/2\nRXY Q10 1 0.5\n",
        qcis_dumps(&circuit).unwrap()
    );
}

#[test]
fn test_all_supported_standard_gates_roundtrip() {
    let cases = [
        ("H Q0", StandardGate::H),
        ("RX Q0 0.1", StandardGate::RX),
        ("RXX Q0 Q1 0.1", StandardGate::RXX),
        ("RXY Q0 0.1 0.2", StandardGate::RXY),
        ("RY Q0 0.1", StandardGate::RY),
        ("RYY Q0 Q1 0.1", StandardGate::RYY),
        ("RZ Q0 0.1", StandardGate::RZ),
        ("RZX Q0 Q1 0.1", StandardGate::RZX),
        ("RZZ Q0 Q1 0.1", StandardGate::RZZ),
        ("S Q0", StandardGate::S),
        ("SD Q0", StandardGate::SDG),
        ("SWAP Q0 Q1", StandardGate::SWAP),
        ("T Q0", StandardGate::T),
        ("TD Q0", StandardGate::TDG),
        ("U Q0 0.1 0.2 0.3", StandardGate::U),
        ("X Q0", StandardGate::X),
        ("XY Q0 0.1", StandardGate::XY),
        ("X2P Q0", StandardGate::X2P),
        ("X2M Q0", StandardGate::X2M),
        ("XY2P Q0 0.1", StandardGate::XY2P),
        ("XY2M Q0 0.1", StandardGate::XY2M),
        ("Y Q0", StandardGate::Y),
        ("Y2P Q0", StandardGate::Y2P),
        ("Y2M Q0", StandardGate::Y2M),
        ("Z Q0", StandardGate::Z),
        ("PHASE Q0 0.1", StandardGate::Phase),
        ("CX Q0 Q1", StandardGate::CX),
        ("CCX Q0 Q1 Q2", StandardGate::CCX),
        ("CY Q0 Q1", StandardGate::CY),
        ("CZ Q0 Q1", StandardGate::CZ),
        ("CRX Q0 Q1 0.1", StandardGate::CRX),
        ("CRY Q0 Q1 0.1", StandardGate::CRY),
        ("CRZ Q0 Q1 0.1", StandardGate::CRZ),
        ("FSIM Q0 Q1 0.1 0.2", StandardGate::FSIM),
    ];

    for (source, expected_gate) in cases {
        let circuit = loads(source).unwrap_or_else(|error| panic!("{source}: {error}"));
        let operations = circuit.operations();
        assert_eq!(operations.len(), 1, "{source}");
        assert!(
            matches!(operations[0].instruction, Instruction::Standard(gate) if gate == expected_gate),
            "{source}"
        );
        assert_eq!(qcis_dumps(&circuit).unwrap(), format!("{source}\n"));
    }
}

#[test]
fn test_gphase_is_not_supported() {
    assert!(matches!(
        loads("GPHASE 0.1"),
        Err(QcisParseError::UnknownGate(gate)) if gate == "GPHASE"
    ));
}

#[test]
fn test_new_standard_gate_argument_counts_are_validated() {
    assert!(matches!(
        loads("CCX Q0 Q1"),
        Err(QcisParseError::QubitCountMismatch {
            gate,
            expected: 3,
            actual: 2,
        }) if gate == "CCX"
    ));
    assert!(matches!(
        loads("FSIM Q0 Q1 0.1"),
        Err(QcisParseError::ParameterCountMismatch {
            gate,
            expected: 2,
            actual: 1,
        }) if gate == "FSIM"
    ));
}

#[test]
fn test_mixed_qcis_circuit() {
    let qcis = r#"
        // Initialize
        X2P Q0
        X2M Q1

        // Entangle
        CZ Q0 Q1

        // Rotate
        RZ Q0 pi/4
        RZ Q1 pi/4

        // Measure all
        M Q0 Q1
    "#;
    let circuit = loads(qcis).unwrap();
    assert_eq!(circuit.num_qubits(), 2);
    assert_eq!(
        "X2P Q0\nX2M Q1\nCZ Q0 Q1\nRZ Q0 pi/4\nRZ Q1 pi/4\nM Q0\nM Q1\n",
        qcis_dumps(&circuit).unwrap()
    );
}
