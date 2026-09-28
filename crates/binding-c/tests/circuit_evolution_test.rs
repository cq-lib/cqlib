//! Integration tests for the Pauli evolution C ABI.

#![allow(dead_code)]

use binding_c::circuit::{
    circuit_free, circuit_new, circuit_num_operations, circuit_pauli_evolution,
};
use binding_c::qis::{pauli_string_free, pauli_string_parse};

fn parse_pauli(text: &std::ffi::CStr) -> *mut binding_c::qis::CPauliString {
    let ptr = pauli_string_parse(text.as_ptr());
    assert!(!ptr.is_null(), "pauli_string_parse should succeed");
    ptr
}

#[test]
fn test_pauli_evolution_xz() {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    let pauli = parse_pauli(c"XZ");

    let qubits: [u32; 2] = [0, 1];
    let before = circuit_num_operations(circuit);
    assert_eq!(before, 0);

    // exp(-i * pi/4 * XZ) decomposes into basis changes, a CNOT ladder,
    // one RZ, and the inverse ladder, so the operation count must grow.
    assert_eq!(
        circuit_pauli_evolution(
            circuit,
            pauli,
            std::f64::consts::FRAC_PI_2,
            qubits.as_ptr(),
            2
        ),
        0
    );
    let after = circuit_num_operations(circuit);
    assert!(
        after > before,
        "expected operations to grow: {before} -> {after}"
    );

    pauli_string_free(pauli);
    circuit_free(circuit);
}

#[test]
fn test_pauli_evolution_null_arguments() {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());
    let pauli = parse_pauli(c"XZ");
    let qubits: [u32; 2] = [0, 1];

    assert_eq!(
        circuit_pauli_evolution(std::ptr::null_mut(), pauli, 1.0, qubits.as_ptr(), 2),
        -1
    );
    assert_eq!(
        circuit_pauli_evolution(circuit, std::ptr::null(), 1.0, qubits.as_ptr(), 2),
        -1
    );
    // A NULL qubit buffer with a positive length is rejected.
    assert_eq!(
        circuit_pauli_evolution(circuit, pauli, 1.0, std::ptr::null(), 2),
        -1
    );

    pauli_string_free(pauli);
    circuit_free(circuit);
}

#[test]
fn test_pauli_evolution_error_codes() {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null());

    // Out-of-bounds qubit index.
    let pauli = parse_pauli(c"XZ");
    let out_of_bounds: [u32; 2] = [0, 5];
    assert_eq!(
        circuit_pauli_evolution(circuit, pauli, 1.0, out_of_bounds.as_ptr(), 2),
        -2
    );

    // Position count mismatch: a 3-position Pauli string on 2 positions.
    let big = parse_pauli(c"XXX");
    let qubits: [u32; 2] = [0, 1];
    assert_eq!(
        circuit_pauli_evolution(circuit, big, 1.0, qubits.as_ptr(), 2),
        -3
    );

    // Non-finite angle.
    assert_eq!(
        circuit_pauli_evolution(circuit, pauli, f64::NAN, qubits.as_ptr(), 2),
        -8
    );

    pauli_string_free(big);
    pauli_string_free(pauli);
    circuit_free(circuit);
}
