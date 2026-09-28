//! Integration tests for IR module C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_free, circuit_num_qubits};
use binding_c::cqlib_string_free;
use binding_c::ir::{qasm2_dump, qasm2_dumps, qasm2_loads, qasm3_dumps, qcis_dumps, qcis_loads};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

const QASM2_SAMPLE: &str = r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
h q[0];
cx q[0], q[1];
"#;

const QCIS_SAMPLE: &str = "H Q0\nCZ Q0 Q1\nM Q0 Q1";

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn test_qasm2_loads_and_dumps() {
    let source = CString::new(QASM2_SAMPLE).unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(!circuit.is_null(), "qasm2_loads should succeed");

    let num_qubits = circuit_num_qubits(circuit);
    assert_eq!(num_qubits, 2);

    let dump_ptr = qasm2_dumps(circuit);
    assert!(!dump_ptr.is_null(), "qasm2_dumps should succeed");
    let dumped = cstr_to_string(dump_ptr);
    cqlib_string_free(dump_ptr);
    assert!(dumped.contains("h"));
    assert!(dumped.contains("cx"));

    circuit_free(circuit);
}

#[test]
fn test_qasm2_loads_null() {
    let circuit = qasm2_loads(std::ptr::null());
    assert!(circuit.is_null(), "NULL source should return NULL");
}

#[test]
fn test_qasm2_loads_invalid() {
    let source = CString::new("not valid qasm").unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(circuit.is_null(), "invalid source should return NULL");
}

#[test]
fn test_qasm2_dumps_null() {
    let dump_ptr = qasm2_dumps(std::ptr::null());
    assert!(dump_ptr.is_null(), "NULL circuit should return NULL");
}

#[test]
fn test_qasm3_dumps() {
    let source = CString::new(QASM2_SAMPLE).unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(!circuit.is_null());

    let dump_ptr = qasm3_dumps(circuit);
    assert!(!dump_ptr.is_null(), "qasm3_dumps should succeed");
    cqlib_string_free(dump_ptr);

    circuit_free(circuit);
}

#[test]
fn test_qcis_loads_and_dumps() {
    let source = CString::new(QCIS_SAMPLE).unwrap();
    let circuit = qcis_loads(source.as_ptr());
    assert!(!circuit.is_null(), "qcis_loads should succeed");

    let num_qubits = circuit_num_qubits(circuit);
    assert_eq!(num_qubits, 2);

    let dump_ptr = qcis_dumps(circuit);
    assert!(!dump_ptr.is_null(), "qcis_dumps should succeed");
    let dumped = cstr_to_string(dump_ptr);
    cqlib_string_free(dump_ptr);
    assert!(dumped.to_lowercase().contains("h"));

    circuit_free(circuit);
}

#[test]
fn test_qcis_loads_null() {
    let circuit = qcis_loads(std::ptr::null());
    assert!(circuit.is_null());
}

#[test]
fn test_qcis_loads_invalid() {
    let source = CString::new("@@@INVALID_GARBAGE@@@").unwrap();
    let circuit = qcis_loads(source.as_ptr());
    assert!(circuit.is_null(), "invalid QCIS should return NULL");
}

#[test]
fn test_qasm2_dump_to_file() {
    let source = CString::new(QASM2_SAMPLE).unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(!circuit.is_null());

    let tmp = std::env::temp_dir().join("cqlib_test_qasm2_dump.qasm");
    let path = CString::new(tmp.to_str().unwrap()).unwrap();
    let ret = qasm2_dump(circuit, path.as_ptr());
    assert_eq!(ret, 0, "qasm2_dump should return 0 on success");

    let content = std::fs::read_to_string(&tmp);
    assert!(content.is_ok());
    assert!(content.unwrap().contains("h"));
    let _ = std::fs::remove_file(&tmp);

    circuit_free(circuit);
}

#[test]
fn test_qasm2_dump_null_circuit() {
    let path = CString::new("target/dummy.qasm").unwrap();
    let ret = qasm2_dump(std::ptr::null(), path.as_ptr());
    assert_eq!(ret, -1, "NULL circuit should return NullPtr error");
}

#[test]
fn test_qasm2_dump_null_path() {
    let source = CString::new(QASM2_SAMPLE).unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(!circuit.is_null());

    let ret = qasm2_dump(circuit, std::ptr::null());
    assert_eq!(ret, -1, "NULL path should return NullPtr error");

    circuit_free(circuit);
}
