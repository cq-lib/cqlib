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

//! Integration tests for the QIS simulator supplement and Measurement
//! C ABI (checklist 2.5 + 2.6).

use binding_c::circuit::{
    CQLIB_CLASSICAL_EXPR_VALUE, CQLIB_CLASSICAL_TYPE_BIT_VEC, CQLIB_CLASSICAL_TYPE_UINT,
    circuit_cx, circuit_free, circuit_h, circuit_measure_bits, circuit_new, classical_expr_free,
    classical_expr_kind,
};
use binding_c::cqlib_string_free;
use binding_c::qis::*;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Reads a heap-allocated C string and frees it with `cqlib_string_free`.
fn cstr_to_string(p: *mut c_char) -> String {
    assert!(!p.is_null());
    let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    cqlib_string_free(p);
    s
}

fn gate(name: &str) -> CString {
    CString::new(name).unwrap()
}

fn read_statevector(sv: *const CStatevector) -> Vec<Complex64> {
    let len = statevector_data_len(sv);
    let mut buffer = vec![Complex64::new(0.0, 0.0); len];
    assert_eq!(statevector_data(sv, buffer.as_mut_ptr(), len), 0);
    buffer
}

fn read_density_matrix(dm: *const CDensityMatrix) -> Vec<Complex64> {
    let len = density_matrix_data_len(dm);
    let mut buffer = vec![Complex64::new(0.0, 0.0); len];
    assert_eq!(density_matrix_data(dm, buffer.as_mut_ptr(), len), 0);
    buffer
}

#[test]
fn test_statevector_data_and_from_state() {
    let sv = statevector_new(2);
    assert_eq!(statevector_apply_h(sv, 0), 0);
    assert_eq!(statevector_apply_cx(sv, 0, 1), 0);

    // Bell state: amplitudes at |00> (index 0) and |11> (index 3).
    assert_eq!(statevector_data_len(sv), 4);
    let amps = read_statevector(sv);
    let inv = std::f64::consts::FRAC_1_SQRT_2;
    assert!((amps[0].re - inv).abs() < 1e-12 && amps[0].im.abs() < 1e-12);
    assert!(amps[1].norm() < 1e-12 && amps[2].norm() < 1e-12);
    assert!((amps[3].re - inv).abs() < 1e-12 && amps[3].im.abs() < 1e-12);

    // Error paths for the two-step data access.
    let mut out = vec![Complex64::new(0.0, 0.0); 4];
    assert_eq!(statevector_data(sv, out.as_mut_ptr(), 3), -8);
    assert_eq!(statevector_data(sv, ptr::null_mut(), 4), -1);
    assert_eq!(statevector_data(ptr::null(), out.as_mut_ptr(), 4), -1);
    assert_eq!(statevector_data_len(ptr::null()), 0);
    statevector_free(sv);

    // Reconstruct the Bell state via from_state and compare.
    let bell = vec![
        Complex64::new(inv, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(inv, 0.0),
    ];
    let rebuilt = statevector_from_state(2, bell.as_ptr(), bell.len());
    assert!(!rebuilt.is_null());
    assert_eq!(read_statevector(rebuilt), bell);
    statevector_free(rebuilt);

    // from_state error paths: wrong dimension, NULL input, not normalized.
    assert!(statevector_from_state(2, bell.as_ptr(), 3).is_null());
    assert!(statevector_from_state(2, ptr::null(), 4).is_null());
    let unnormalized = [Complex64::new(1.0, 0.0), Complex64::new(1.0, 0.0)];
    assert!(statevector_from_state(1, unnormalized.as_ptr(), 2).is_null());
}

#[test]
fn test_statevector_apply_standard_gate() {
    // "H" by name must match the dedicated apply_h entry point.
    let via_helper = statevector_new(1);
    assert_eq!(statevector_apply_h(via_helper, 0), 0);
    let via_name = statevector_new(1);
    let h = gate("H");
    let targets = [0u32];
    assert_eq!(
        statevector_apply_standard_gate(via_name, h.as_ptr(), targets.as_ptr(), 1, ptr::null(), 0),
        0
    );
    assert_eq!(read_statevector(via_name), read_statevector(via_helper));
    statevector_free(via_helper);
    statevector_free(via_name);

    // RX(pi) maps |0> to -i|1>.
    let sv = statevector_new(1);
    let rx = gate("RX");
    let params = [std::f64::consts::PI];
    assert_eq!(
        statevector_apply_standard_gate(sv, rx.as_ptr(), targets.as_ptr(), 1, params.as_ptr(), 1),
        0
    );
    let amps = read_statevector(sv);
    assert!(amps[0].norm() < 1e-12);
    assert!((amps[1].re - 0.0).abs() < 1e-12 && (amps[1].im + 1.0).abs() < 1e-12);
    statevector_free(sv);

    // Error paths.
    let sv = statevector_new(1);
    let unknown = gate("NOPE");
    assert_eq!(
        statevector_apply_standard_gate(sv, unknown.as_ptr(), targets.as_ptr(), 1, ptr::null(), 0),
        -8
    );
    assert_eq!(
        statevector_apply_standard_gate(sv, ptr::null(), targets.as_ptr(), 1, ptr::null(), 0),
        -1
    );
    let oob = [5u32];
    assert_eq!(
        statevector_apply_standard_gate(sv, h.as_ptr(), oob.as_ptr(), 1, ptr::null(), 0),
        -2
    );
    assert_eq!(
        statevector_apply_standard_gate(sv, rx.as_ptr(), targets.as_ptr(), 1, ptr::null(), 1),
        -1
    );
    let nan = [f64::NAN];
    assert_eq!(
        statevector_apply_standard_gate(sv, rx.as_ptr(), targets.as_ptr(), 1, nan.as_ptr(), 1),
        -8
    );
    assert_eq!(
        statevector_apply_standard_gate(
            ptr::null_mut(),
            h.as_ptr(),
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -1
    );
    statevector_free(sv);
}

#[test]
fn test_statevector_apply_pauli_rotation() {
    // exp(-i * pi / 2 * X) maps |0> to -i|1>.
    let pauli = pauli_string_new(1);
    assert!(!pauli.is_null());
    assert_eq!(pauli_string_set_pauli(pauli, 0, PAULI_X), 0);

    let sv = statevector_new(1);
    assert_eq!(
        statevector_apply_pauli_rotation(sv, pauli, std::f64::consts::PI),
        0
    );
    let amps = read_statevector(sv);
    assert!(amps[0].norm() < 1e-12);
    assert!((amps[1].im + 1.0).abs() < 1e-12 && amps[1].re.abs() < 1e-12);

    // Error paths.
    assert_eq!(statevector_apply_pauli_rotation(sv, pauli, f64::NAN), -8);
    assert_eq!(statevector_apply_pauli_rotation(sv, ptr::null(), 0.0), -1);
    assert_eq!(
        statevector_apply_pauli_rotation(ptr::null_mut(), pauli, 0.0),
        -1
    );

    statevector_free(sv);
    pauli_string_free(pauli);
}

#[test]
fn test_statevector_apply_unitary_gate() {
    // X matrix [[0, 1], [1, 0]] maps |0> to |1>.
    let matrix = [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
    ];
    let targets = [0u32];
    let sv = statevector_new(1);
    assert_eq!(
        statevector_apply_unitary_gate(sv, targets.as_ptr(), 1, matrix.as_ptr(), 2),
        0
    );
    let amps = read_statevector(sv);
    assert!(amps[0].norm() < 1e-12);
    assert!((amps[1].re - 1.0).abs() < 1e-12 && amps[1].im.abs() < 1e-12);

    // Error paths.
    assert_eq!(
        statevector_apply_unitary_gate(sv, targets.as_ptr(), 1, ptr::null(), 2),
        -1
    );
    assert_eq!(
        statevector_apply_unitary_gate(sv, ptr::null(), 1, matrix.as_ptr(), 2),
        -1
    );
    assert_eq!(
        statevector_apply_unitary_gate(ptr::null_mut(), targets.as_ptr(), 1, matrix.as_ptr(), 2),
        -1
    );
    // dim inconsistent with the qubit count is rejected by the core.
    assert!(statevector_apply_unitary_gate(sv, targets.as_ptr(), 1, matrix.as_ptr(), 4) < 0);
    statevector_free(sv);
}

#[test]
fn test_density_matrix_supplement() {
    let inv = std::f64::consts::FRAC_1_SQRT_2;

    // rho = |+><+| has all entries 0.5.
    let plus = [Complex64::new(inv, 0.0), Complex64::new(inv, 0.0)];
    let dm = density_matrix_from_state(1, plus.as_ptr(), plus.len());
    assert!(!dm.is_null());
    assert_eq!(density_matrix_data_len(dm), 4);
    let data = read_density_matrix(dm);
    for entry in &data {
        assert!((entry.re - 0.5).abs() < 1e-12 && entry.im.abs() < 1e-12);
    }

    // Trace of a density matrix is 1.
    let (mut re, mut im) = (0.0f64, 0.0f64);
    assert_eq!(density_matrix_trace(dm, &mut re, &mut im), 0);
    assert!((re - 1.0).abs() < 1e-12 && im.abs() < 1e-12);

    // |+><+| is Hermitian.
    let mut hermitian = false;
    assert_eq!(density_matrix_is_hermitian(dm, 1e-10, &mut hermitian), 0);
    assert!(hermitian);
    assert_eq!(
        density_matrix_is_hermitian(dm, f64::NAN, &mut hermitian),
        -8
    );

    // Error paths.
    assert_eq!(density_matrix_trace(dm, ptr::null_mut(), &mut im), -1);
    let mut out = vec![Complex64::new(0.0, 0.0); 4];
    assert_eq!(density_matrix_data(dm, out.as_mut_ptr(), 3), -8);
    assert_eq!(density_matrix_data(ptr::null(), out.as_mut_ptr(), 4), -1);
    assert!(density_matrix_from_state(1, plus.as_ptr(), 3).is_null());
    density_matrix_free(dm);

    // Maximally mixed state of 2 qubits: trace 1, diagonal 0.25.
    let mm = density_matrix_maximally_mixed(2);
    assert!(!mm.is_null());
    assert_eq!(density_matrix_data_len(mm), 16);
    let data = read_density_matrix(mm);
    for i in 0..4 {
        assert!((data[i * 4 + i].re - 0.25).abs() < 1e-12);
    }
    assert_eq!(density_matrix_trace(mm, &mut re, &mut im), 0);
    assert!((re - 1.0).abs() < 1e-12 && im.abs() < 1e-12);
    density_matrix_free(mm);

    // "X" by name flips |0><0| to |1><1|.
    let ket0 = [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)];
    let dm = density_matrix_from_state(1, ket0.as_ptr(), ket0.len());
    let x = gate("X");
    let targets = [0u32];
    assert_eq!(
        density_matrix_apply_standard_gate(dm, x.as_ptr(), targets.as_ptr(), 1, ptr::null(), 0),
        0
    );
    let data = read_density_matrix(dm);
    assert!(data[0].norm() < 1e-12);
    assert!((data[3].re - 1.0).abs() < 1e-12);
    assert_eq!(
        density_matrix_apply_standard_gate(
            dm,
            gate("NOPE").as_ptr(),
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );
    density_matrix_free(dm);
}

#[test]
fn test_stabilizer_supplement() {
    // Fresh 1-qubit state: destabilizer +X, stabilizer +Z, stim export "+Z\n".
    let s = stabilizer_new(1);
    assert!(!s.is_null());
    assert_eq!(stabilizer_get_destabilizers_len(s), 1);
    assert_eq!(cstr_to_string(stabilizer_get_destabilizer(s, 0)), "+X");
    assert!(stabilizer_get_destabilizer(s, 5).is_null());
    assert_eq!(cstr_to_string(stabilizer_to_stim_format(s)), "+Z\n");
    assert_eq!(stabilizer_get_destabilizers_len(ptr::null()), 0);
    assert!(stabilizer_get_destabilizer(ptr::null(), 0).is_null());
    assert!(stabilizer_to_stim_format(ptr::null()).is_null());
    stabilizer_free(s);

    // Bell state probabilities.
    let bell = stabilizer_new(2);
    assert_eq!(stabilizer_apply_h(bell, 0), 0);
    assert_eq!(stabilizer_apply_cx(bell, 0, 1), 0);
    let mut probability = 0.0f64;
    assert_eq!(
        stabilizer_probability_of(bell, [false, false].as_ptr(), 2, &mut probability),
        0
    );
    assert!((probability - 0.5).abs() < 1e-12);
    assert_eq!(
        stabilizer_probability_of(bell, [true, true].as_ptr(), 2, &mut probability),
        0
    );
    assert!((probability - 0.5).abs() < 1e-12);
    assert_eq!(
        stabilizer_probability_of(bell, [false, true].as_ptr(), 2, &mut probability),
        0
    );
    assert!(probability < 1e-12);
    // Length mismatch maps to -8 (QubitMismatch).
    assert_eq!(
        stabilizer_probability_of(bell, [false].as_ptr(), 1, &mut probability),
        -8
    );
    assert_eq!(
        stabilizer_probability_of(ptr::null(), [false, false].as_ptr(), 2, &mut probability),
        -1
    );
    stabilizer_free(bell);

    // Standard gate by name: H works, non-Clifford T is rejected with -7.
    let s = stabilizer_new(1);
    let h = gate("H");
    let targets = [0u32];
    assert_eq!(
        stabilizer_apply_standard_gate(s, h.as_ptr(), targets.as_ptr(), 1, ptr::null(), 0),
        0
    );
    assert_eq!(cstr_to_string(stabilizer_get_stabilizer(s, 0)), "+X");
    let t = gate("T");
    assert_eq!(
        stabilizer_apply_standard_gate(s, t.as_ptr(), targets.as_ptr(), 1, ptr::null(), 0),
        -7
    );
    assert_eq!(
        stabilizer_apply_standard_gate(
            s,
            gate("NOPE").as_ptr(),
            targets.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );
    stabilizer_free(s);
}

#[test]
fn test_stabilizer_run_circuit_collapses_measurements() {
    // Bell circuit with a terminal measure_bits: run_circuit collapses the
    // state (unlike from_circuit, which ignores the measurement).
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);
    let qubits = [0u32, 1];
    let measured = circuit_measure_bits(circuit, qubits.as_ptr(), 2);
    assert!(!measured.is_null());
    classical_expr_free(measured);

    let state = stabilizer_run_circuit(circuit);
    assert!(!state.is_null());
    circuit_free(circuit);

    // The collapsed state is either |00> or |11>, never a superposition.
    let mut p00 = 0.0f64;
    let mut p11 = 0.0f64;
    let mut p01 = 0.0f64;
    assert_eq!(
        stabilizer_probability_of(state, [false, false].as_ptr(), 2, &mut p00),
        0
    );
    assert_eq!(
        stabilizer_probability_of(state, [true, true].as_ptr(), 2, &mut p11),
        0
    );
    assert_eq!(
        stabilizer_probability_of(state, [false, true].as_ptr(), 2, &mut p01),
        0
    );
    assert!((p00 + p11 - 1.0).abs() < 1e-12);
    assert!(p01 < 1e-12);
    stabilizer_free(state);

    assert!(stabilizer_run_circuit(ptr::null()).is_null());
}

#[test]
fn test_measurement_lifecycle() {
    let qubits = [2u32, 0];
    let m = measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits.as_ptr(), 2);
    assert!(!m.is_null());

    // Getters: qubits keep the given (result bit) order.
    assert_eq!(measurement_qubits_len(m), 2);
    assert_eq!(measurement_width(m), 2);
    let mut buffer = [99u32; 2];
    assert_eq!(measurement_qubits(m, buffer.as_mut_ptr(), 2), 0);
    assert_eq!(buffer, [2, 0]);
    assert_eq!(measurement_qubits(m, buffer.as_mut_ptr(), 1), -8);
    assert_eq!(measurement_qubits(m, ptr::null_mut(), 2), -1);

    let (mut tag, mut width) = (99u32, 99u32);
    assert_eq!(measurement_ty(m, &mut tag, &mut width), 0);
    assert_eq!(tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(width, 2);

    // Value / expression chain.
    let value = measurement_value(m);
    assert!(!value.is_null());
    assert_eq!(classical_value_index(value), 0);
    assert_eq!(classical_value_ty(value, &mut tag, &mut width), 0);
    assert_eq!(tag, CQLIB_CLASSICAL_TYPE_BIT_VEC);
    assert_eq!(width, 2);
    let value_expr = classical_value_expr(value);
    assert!(!value_expr.is_null());
    let mut kind = 99u32;
    assert_eq!(classical_expr_kind(value_expr, &mut kind), 0);
    assert_eq!(kind, CQLIB_CLASSICAL_EXPR_VALUE);
    classical_expr_free(value_expr);
    classical_value_free(value);

    let expr = measurement_expr(m);
    assert!(!expr.is_null());
    assert_eq!(classical_expr_kind(expr, &mut kind), 0);
    assert_eq!(kind, CQLIB_CLASSICAL_EXPR_VALUE);
    classical_expr_free(expr);

    // Qubit-order validation against a register size.
    assert_eq!(measurement_check_qubits(m, 3), 0);
    assert_eq!(measurement_check_qubits(m, 2), -2);
    assert_eq!(measurement_check_qubits(ptr::null(), 3), -1);

    // Projection: register "101" (MSB left) has q2=1 and q0=1, and the
    // measurement reads qubits [2, 0], so both result bits are 1 -> "11".
    let bitstring = CString::new("101").unwrap();
    assert_eq!(
        cstr_to_string(measurement_project(m, bitstring.as_ptr())),
        "11"
    );
    assert_eq!(cstr_to_string(measurement_project_basis(m, 0b101)), "11");
    // Only q0 is set in "001": result bit 0 (qubit 2) is 0, bit 1 (qubit 0) is 1.
    let bitstring = CString::new("001").unwrap();
    assert_eq!(
        cstr_to_string(measurement_project(m, bitstring.as_ptr())),
        "10"
    );
    assert!(measurement_project(m, ptr::null()).is_null());
    let bad = CString::new("10a").unwrap();
    assert!(measurement_project(m, bad.as_ptr()).is_null());

    measurement_free(m);

    // Constructor error paths.
    assert!(measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, ptr::null(), 2).is_null());
    assert!(measurement_new(0, CQLIB_CLASSICAL_TYPE_BIT_VEC, 2, qubits.as_ptr(), 0).is_null());
    assert!(measurement_new(0, 99, 2, qubits.as_ptr(), 2).is_null());
    assert!(measurement_new(0, CQLIB_CLASSICAL_TYPE_UINT, 0, qubits.as_ptr(), 2).is_null());
}

#[test]
fn test_measurement_and_value_null_handling() {
    assert!(measurement_value(ptr::null()).is_null());
    assert!(measurement_expr(ptr::null()).is_null());
    assert_eq!(measurement_qubits_len(ptr::null()), 0);
    assert_eq!(measurement_width(ptr::null()), 0);
    assert_eq!(
        measurement_ty(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        -1
    );
    assert!(measurement_project(ptr::null(), ptr::null()).is_null());
    assert!(measurement_project_basis(ptr::null(), 0).is_null());

    classical_value_free(ptr::null_mut());
    measurement_free(ptr::null_mut());
    assert_eq!(classical_value_index(ptr::null()), u32::MAX);
    assert_eq!(
        classical_value_ty(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        -1
    );
    assert!(classical_value_expr(ptr::null()).is_null());
}
