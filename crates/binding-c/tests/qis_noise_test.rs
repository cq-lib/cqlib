//! Integration tests for the DensityMatrixNoise C ABI.

#![allow(dead_code)]

use binding_c::circuit::{circuit_cx, circuit_free, circuit_h, circuit_new, circuit_x};
use binding_c::cqlib_string_free;
use binding_c::device::{
    NOISE_BIT_FLIP, noise_model_add_readout, noise_model_add_single_qubit, noise_model_free,
    noise_model_new,
};
use binding_c::qis::*;
use num_complex::Complex64;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

fn cstr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Build a noise model with a bit-flip channel of probability `p` on X@q0.
fn bit_flip_noise_model(p: f64) -> *mut binding_c::device::CNoiseModel {
    let nm = noise_model_new();
    assert!(!nm.is_null());
    let gate = CString::new("X").unwrap();
    assert_eq!(
        noise_model_add_single_qubit(nm, gate.as_ptr(), 0, NOISE_BIT_FLIP, p),
        0
    );
    nm
}

fn read_probs(sim: *const CDensityMatrixNoise, len: usize) -> Vec<f64> {
    let mut probs = vec![0.0f64; len];
    assert_eq!(
        density_matrix_noise_probabilities(sim, probs.as_mut_ptr(), len),
        0
    );
    probs
}

#[test]
fn test_new_free_and_null_handling() {
    let sim = density_matrix_noise_new(2, ptr::null());
    assert!(!sim.is_null());
    assert_eq!(density_matrix_noise_num_qubits(sim), 2);
    density_matrix_noise_free(sim);

    density_matrix_noise_free(ptr::null_mut());
    assert_eq!(density_matrix_noise_num_qubits(ptr::null()), 0);
    assert_eq!(density_matrix_noise_probabilities_len(ptr::null()), 0);
    assert_eq!(density_matrix_noise_apply_h(ptr::null_mut(), 0), -1);
    assert_eq!(density_matrix_noise_measure(ptr::null_mut(), 0), -1);
    assert!(density_matrix_noise_measure_all(ptr::null_mut()).is_null());
    assert!(density_matrix_noise_sample_shots(ptr::null(), 4).is_null());
}

#[test]
fn test_new_with_noise_model_handle() {
    let nm = noise_model_new();
    let sim = density_matrix_noise_new(1, nm);
    assert!(!sim.is_null());
    assert_eq!(density_matrix_noise_num_qubits(sim), 1);
    density_matrix_noise_free(sim);
    // The handle stays owned by the caller and remains usable.
    let sim2 = density_matrix_noise_new(1, nm);
    assert!(!sim2.is_null());
    density_matrix_noise_free(sim2);
    noise_model_free(nm);
}

#[test]
fn test_apply_h_probabilities_normalized() {
    let sim = density_matrix_noise_new(1, ptr::null());
    assert_eq!(density_matrix_noise_apply_h(sim, 0), 0);

    assert_eq!(density_matrix_noise_probabilities_len(sim), 2);
    let probs = read_probs(sim, 2);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[1] - 0.5).abs() < 1e-10);
    assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-10);

    density_matrix_noise_free(sim);
}

#[test]
fn test_gate_noise_shifts_probabilities() {
    // Mirrors core `test_bit_flip_noise`: X followed by 10% bit-flip noise
    // on qubit 0 leaves P(|1>) = 0.9 and P(|0>) = 0.1.
    let nm = bit_flip_noise_model(0.1);
    let sim = density_matrix_noise_new(1, nm);

    assert_eq!(density_matrix_noise_apply_x(sim, 0), 0);
    let probs = read_probs(sim, 2);
    assert!((probs[1] - 0.9).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.1).abs() < 1e-6, "P(|0>) was {}", probs[0]);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

#[test]
fn test_from_circuit_with_noise() {
    let nm = bit_flip_noise_model(0.1);

    let circuit = circuit_new(1);
    assert_eq!(circuit_x(circuit, 0), 0);

    let sim = density_matrix_noise_from_circuit(circuit, nm);
    assert!(!sim.is_null());
    assert_eq!(density_matrix_noise_num_qubits(sim), 1);

    let probs = read_probs(sim, 2);
    assert!((probs[1] - 0.9).abs() < 1e-6);
    assert!((probs[0] - 0.1).abs() < 1e-6);

    density_matrix_noise_free(sim);
    circuit_free(circuit);
    noise_model_free(nm);
}

#[test]
fn test_from_circuit_null_args() {
    assert!(density_matrix_noise_from_circuit(ptr::null(), ptr::null()).is_null());
}

#[test]
fn test_apply_circuit_bell_state() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), 0);
    assert_eq!(circuit_cx(circuit, 0, 1), 0);

    let sim = density_matrix_noise_new(2, ptr::null());
    assert_eq!(density_matrix_noise_apply_circuit(sim, circuit), 0);

    let probs = read_probs(sim, 4);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!(probs[1] < 1e-10);
    assert!(probs[2] < 1e-10);
    assert!((probs[3] - 0.5).abs() < 1e-10);

    // NULL arguments are rejected.
    assert_eq!(density_matrix_noise_apply_circuit(sim, ptr::null()), -1);

    density_matrix_noise_free(sim);
    circuit_free(circuit);
}

#[test]
fn test_qubit_out_of_bounds() {
    let sim = density_matrix_noise_new(1, ptr::null());
    assert_eq!(density_matrix_noise_apply_h(sim, 5), -2);
    assert_eq!(density_matrix_noise_apply_cx(sim, 0, 5), -2);
    assert_eq!(density_matrix_noise_apply_ccx(sim, 0, 0, 5), -2);
    assert_eq!(density_matrix_noise_reset(sim, 5), -2);
    assert_eq!(density_matrix_noise_measure(sim, 5), -2);
    density_matrix_noise_free(sim);
}

#[test]
fn test_readout_error_difference() {
    // Mirrors core `test_readout_error`: |0> with P(1|0) = 0.1 readout error.
    let nm = noise_model_new();
    assert_eq!(noise_model_add_readout(nm, 0, 0.0, 0.1), 0);

    let sim = density_matrix_noise_new(1, nm);

    // Ideal distribution is unaffected by readout configuration.
    let ideal = read_probs(sim, 2);
    assert!((ideal[0] - 1.0).abs() < 1e-10);
    assert!(ideal[1] < 1e-10);

    // With readout: P(|0>) = 0.9, P(|1>) = 0.1.
    assert_eq!(density_matrix_noise_probabilities_with_readout_len(sim), 2);
    let qs = [0u32];
    let mut probs = vec![0.0f64; 2];
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(sim, probs.as_mut_ptr(), 2, qs.as_ptr(), 1),
        0
    );
    assert!((probs[0] - 0.9).abs() < 1e-6, "P(|0>) was {}", probs[0]);
    assert!((probs[1] - 0.1).abs() < 1e-6, "P(|1>) was {}", probs[1]);

    // Empty qubit selection behaves like the ideal distribution.
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(sim, probs.as_mut_ptr(), 2, ptr::null(), 0),
        0
    );
    assert!((probs[0] - 1.0).abs() < 1e-10);

    // Out-of-bounds readout qubit is rejected.
    let bad = [99u32];
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(
            sim,
            probs.as_mut_ptr(),
            2,
            bad.as_ptr(),
            1
        ),
        -2
    );

    // Wrong buffer length is rejected.
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(sim, probs.as_mut_ptr(), 4, qs.as_ptr(), 1),
        -8
    );

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

#[test]
fn test_multi_qubit_readout() {
    // Mirrors core `test_multi_qubit_readout_error`.
    let nm = noise_model_new();
    assert_eq!(noise_model_add_readout(nm, 0, 0.1, 0.2), 0);
    assert_eq!(noise_model_add_readout(nm, 1, 0.3, 0.4), 0);

    let sim = density_matrix_noise_new(2, nm);
    assert_eq!(density_matrix_noise_apply_x(sim, 0), 0);

    let qs = [0u32, 1u32];
    let mut probs = vec![0.0f64; 4];
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(sim, probs.as_mut_ptr(), 4, qs.as_ptr(), 2),
        0
    );
    assert!((probs[0] - 0.06).abs() < 1e-6, "P(|00>) was {}", probs[0]);
    assert!((probs[1] - 0.54).abs() < 1e-6, "P(|01>) was {}", probs[1]);
    assert!((probs[2] - 0.04).abs() < 1e-6, "P(|10>) was {}", probs[2]);
    assert!((probs[3] - 0.36).abs() < 1e-6, "P(|11>) was {}", probs[3]);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

#[test]
fn test_probabilities_len_mismatch() {
    let sim = density_matrix_noise_new(1, ptr::null());
    let mut probs = vec![0.0f64; 4];
    assert_eq!(
        density_matrix_noise_probabilities(sim, probs.as_mut_ptr(), 4),
        -8
    );
    density_matrix_noise_free(sim);
}

#[test]
fn test_measure_all_bell_state() {
    let sim = density_matrix_noise_new(2, ptr::null());
    assert_eq!(density_matrix_noise_apply_h(sim, 0), 0);
    assert_eq!(density_matrix_noise_apply_cx(sim, 0, 1), 0);

    let bitstring = density_matrix_noise_measure_all(sim);
    let s = cstr_to_string(bitstring);
    cqlib_string_free(bitstring);
    assert!(s == "00" || s == "11", "outcome was {}", s);

    density_matrix_noise_free(sim);
}

#[test]
fn test_measure_and_reset() {
    let sim = density_matrix_noise_new(1, ptr::null());
    assert_eq!(density_matrix_noise_apply_x(sim, 0), 0);
    assert_eq!(density_matrix_noise_measure(sim, 0), 1);

    assert_eq!(density_matrix_noise_reset(sim, 0), 0);
    let probs = read_probs(sim, 2);
    assert!((probs[0] - 1.0).abs() < 1e-10);
    assert!(probs[1] < 1e-10);

    density_matrix_noise_free(sim);
}

#[test]
fn test_sample_shots_bell_state() {
    let sim = density_matrix_noise_new(2, ptr::null());
    assert_eq!(density_matrix_noise_apply_h(sim, 0), 0);
    assert_eq!(density_matrix_noise_apply_cx(sim, 0, 1), 0);

    let list = density_matrix_noise_sample_shots(sim, 64);
    assert!(!list.is_null());
    assert_eq!(outcome_list_len(list), 64);

    for i in 0..outcome_list_len(list) {
        let ptr = outcome_list_get(list, i);
        let s = cstr_to_string(ptr);
        cqlib_string_free(ptr);
        assert!(s == "00" || s == "11", "outcome was {}", s);
    }

    outcome_list_free(list);
    density_matrix_noise_free(sim);
}

#[test]
fn test_expectation_value() {
    // <Z> = -1 on |1>.
    let sim = density_matrix_noise_new(1, ptr::null());
    assert_eq!(density_matrix_noise_apply_x(sim, 0), 0);

    let z = CString::new("Z").unwrap();
    let pauli = pauli_string_parse(z.as_ptr());
    assert!(!pauli.is_null());
    let ham = hamiltonian_from_pauli(pauli);
    assert!(!ham.is_null());

    let mut value = 0.0f64;
    assert_eq!(density_matrix_noise_expectation(sim, ham, &mut value), 0);
    assert!((value + 1.0).abs() < 1e-10, "<Z> was {}", value);

    // NULL arguments are rejected.
    assert_eq!(
        density_matrix_noise_expectation(sim, ptr::null(), &mut value),
        -1
    );

    hamiltonian_free(ham);
    // `hamiltonian_from_pauli` consumed the `pauli` handle; do not free it.
    density_matrix_noise_free(sim);
}

#[test]
fn test_apply_standard_gate_noise() {
    let sim = density_matrix_noise_new(1, ptr::null());

    // "H" via the generic entry point.
    let gate = CString::new("H").unwrap();
    let qs = [0u32];
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            gate.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        0
    );
    let probs = read_probs(sim, 2);
    assert!((probs[0] - 0.5).abs() < 1e-10);
    assert!((probs[1] - 0.5).abs() < 1e-10);

    // Parameterised gate "RY"(pi/2) maps |+> ... just check it succeeds and
    // stays normalized; then "RZ" with a parameter.
    let ry = CString::new("RY").unwrap();
    let params = [std::f64::consts::FRAC_PI_2];
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            ry.as_ptr(),
            qs.as_ptr(),
            1,
            params.as_ptr(),
            1
        ),
        0
    );
    let probs = read_probs(sim, 2);
    assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-10);

    // Unknown gate name -> -8.
    let bogus = CString::new("NOT_A_GATE").unwrap();
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            bogus.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );

    // Wrong arity -> -8.
    let cx = CString::new("CX").unwrap();
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            cx.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );

    // Wrong parameter count -> -8.
    let rx = CString::new("RX").unwrap();
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            rx.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -8
    );

    // Out-of-bounds qubit -> -2.
    let oob = [5u32];
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            gate.as_ptr(),
            oob.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -2
    );

    // NULL handle / NULL name -> -1.
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            ptr::null_mut(),
            gate.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -1
    );
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            ptr::null(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        -1
    );

    density_matrix_noise_free(sim);
}

#[test]
fn test_apply_standard_gate_noise_applies_noise() {
    // The generic entry point must inject gate noise like the dedicated one.
    let nm = bit_flip_noise_model(0.1);
    let sim = density_matrix_noise_new(1, nm);

    let gate = CString::new("X").unwrap();
    let qs = [0u32];
    assert_eq!(
        density_matrix_noise_apply_standard_gate_noise(
            sim,
            gate.as_ptr(),
            qs.as_ptr(),
            1,
            ptr::null(),
            0
        ),
        0
    );
    let probs = read_probs(sim, 2);
    assert!((probs[1] - 0.9).abs() < 1e-6, "P(|1>) was {}", probs[1]);
    assert!((probs[0] - 0.1).abs() < 1e-6, "P(|0>) was {}", probs[0]);

    density_matrix_noise_free(sim);
    noise_model_free(nm);
}

#[test]
fn test_apply_unitary_gate() {
    let sim = density_matrix_noise_new(1, ptr::null());

    // Pauli-X as a 2x2 row-major Complex64 matrix.
    let matrix = [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
    ];
    let qs = [0u32];
    assert_eq!(
        density_matrix_noise_apply_unitary_gate(sim, qs.as_ptr(), 1, matrix.as_ptr(), 2),
        0
    );
    let probs = read_probs(sim, 2);
    assert!((probs[1] - 1.0).abs() < 1e-10);
    assert!(probs[0] < 1e-10);

    // Dimension inconsistent with qubit count -> -8.
    let two_qs = [0u32, 0];
    assert_eq!(
        density_matrix_noise_apply_unitary_gate(sim, two_qs.as_ptr(), 2, matrix.as_ptr(), 2),
        -8
    );

    // Out-of-bounds qubit -> -2.
    let oob = [5u32];
    assert_eq!(
        density_matrix_noise_apply_unitary_gate(sim, oob.as_ptr(), 1, matrix.as_ptr(), 2),
        -2
    );

    // NULL arguments -> -1.
    assert_eq!(
        density_matrix_noise_apply_unitary_gate(
            ptr::null_mut(),
            qs.as_ptr(),
            1,
            matrix.as_ptr(),
            2
        ),
        -1
    );
    assert_eq!(
        density_matrix_noise_apply_unitary_gate(sim, qs.as_ptr(), 1, ptr::null(), 2),
        -1
    );

    density_matrix_noise_free(sim);
}

#[test]
fn test_parameterised_gates_and_gphase() {
    let sim = density_matrix_noise_new(1, ptr::null());

    assert_eq!(
        density_matrix_noise_apply_rx(sim, 0, std::f64::consts::PI),
        0
    );
    assert_eq!(density_matrix_noise_apply_u(sim, 0, 0.1, 0.2, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_rxy(sim, 0, 0.1, 0.2), 0);
    assert_eq!(density_matrix_noise_apply_xy2p(sim, 0, 0.4), 0);
    assert_eq!(density_matrix_noise_apply_xy2m(sim, 0, 0.4), 0);
    assert_eq!(density_matrix_noise_apply_x2p(sim, 0), 0);
    assert_eq!(density_matrix_noise_apply_y2m(sim, 0), 0);
    assert_eq!(density_matrix_noise_apply_gphase(sim, 0.7), 0);

    let probs = read_probs(sim, 2);
    assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-10);

    // Non-finite parameter -> -8.
    assert_eq!(density_matrix_noise_apply_ry(sim, 0, f64::NAN), -8);
    assert_eq!(density_matrix_noise_apply_gphase(sim, f64::NAN), -8);

    density_matrix_noise_free(sim);
}

#[test]
fn test_two_qubit_gates() {
    let sim = density_matrix_noise_new(2, ptr::null());
    assert_eq!(density_matrix_noise_apply_h(sim, 0), 0);

    assert_eq!(density_matrix_noise_apply_cz(sim, 0, 1), 0);
    assert_eq!(density_matrix_noise_apply_swap(sim, 0, 1), 0);
    assert_eq!(density_matrix_noise_apply_rxx(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_ryy(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_rzz(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_rzx(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_crx(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_cry(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_crz(sim, 0, 1, 0.3), 0);
    assert_eq!(density_matrix_noise_apply_fsim(sim, 0, 1, 0.3, 0.2), 0);
    assert_eq!(density_matrix_noise_apply_cy(sim, 0, 1), 0);

    let probs = read_probs(sim, 4);
    assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-10);

    density_matrix_noise_free(sim);
}

#[test]
fn test_ccx_gate() {
    // Mirrors core `test_ccx`.
    let sim = density_matrix_noise_new(3, ptr::null());
    assert_eq!(density_matrix_noise_apply_x(sim, 0), 0);
    assert_eq!(density_matrix_noise_apply_x(sim, 1), 0);
    assert_eq!(density_matrix_noise_apply_ccx(sim, 0, 1, 2), 0);

    let qs = [2u32];
    let mut probs = vec![0.0f64; 8];
    assert_eq!(
        density_matrix_noise_probabilities_with_readout(sim, probs.as_mut_ptr(), 8, qs.as_ptr(), 1),
        0
    );
    let p1: f64 = probs
        .iter()
        .enumerate()
        .filter(|(s, _)| (s >> 2) & 1 == 1)
        .map(|(_, p)| p)
        .sum();
    assert!((p1 - 1.0).abs() < 1e-6);

    density_matrix_noise_free(sim);
}
