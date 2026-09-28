//! Integration tests for QIS entropy / metrics / evolution C ABI
//! (gap checklist sections 2.1 - 2.4).

#![allow(dead_code)]

use binding_c::circuit::{circuit_free, circuit_num_qubits};
use binding_c::qis::*;
use std::ffi::CString;

/// Layout-compatible mirror of the `Complex64` struct injected by build.rs.
#[repr(C)]
#[derive(Clone, Copy)]
struct CComplex {
    re: f64,
    im: f64,
}

fn bell_dm() -> *mut CDensityMatrix {
    let dm = density_matrix_new(2);
    assert!(!dm.is_null());
    assert_eq!(density_matrix_apply_h(dm, 0), 0);
    assert_eq!(density_matrix_apply_cx(dm, 0, 1), 0);
    dm
}

fn bell_sv() -> *mut CStatevector {
    let sv = statevector_new(2);
    assert!(!sv.is_null());
    assert_eq!(statevector_apply_h(sv, 0), 0);
    assert_eq!(statevector_apply_cx(sv, 0, 1), 0);
    sv
}

/// One-qubit maximally mixed state I/2 (partial trace of the Bell state).
fn maximally_mixed_1q() -> *mut CDensityMatrix {
    let bell = bell_dm();
    let keep = [0u32];
    let reduced = density_matrix_partial_trace(bell, keep.as_ptr(), 1);
    assert!(!reduced.is_null());
    density_matrix_free(bell);
    reduced
}

fn hamiltonian_with(pauli: &str, coeff_re: f64) -> *mut CHamiltonian {
    let p = pauli_string_parse(CString::new(pauli).unwrap().as_ptr());
    assert!(!p.is_null());
    let h = hamiltonian_new(pauli.len());
    assert!(!h.is_null());
    assert_eq!(hamiltonian_add_term(h, p, coeff_re, 0.0), 0);
    pauli_string_free(p);
    h
}

// ---------------------------------------------------------------------------
// 2.1 Entropy and entanglement measures
// ---------------------------------------------------------------------------

#[test]
fn test_linear_entropy() {
    let mut out = -1.0f64;

    // Pure Bell state: linear entropy ~ 0.
    let bell = bell_dm();
    assert_eq!(density_matrix_linear_entropy(bell, &mut out), 0);
    assert!(out.abs() < 1e-10);

    // Maximally mixed 1-qubit state I/2: 1 - Tr(rho^2) = 0.5.
    let mixed = maximally_mixed_1q();
    assert_eq!(density_matrix_linear_entropy(mixed, &mut out), 0);
    assert!((out - 0.5).abs() < 1e-10);

    assert_eq!(
        density_matrix_linear_entropy(std::ptr::null(), &mut out),
        -1
    );
    assert_eq!(
        density_matrix_linear_entropy(bell, std::ptr::null_mut()),
        -1
    );

    density_matrix_free(mixed);
    density_matrix_free(bell);
}

#[test]
fn test_renyi_entropy() {
    let mut out = -1.0f64;

    // Pure state: all Renyi entropies vanish.
    let bell = bell_dm();
    assert_eq!(density_matrix_renyi_entropy(bell, 2.0, &mut out), 0);
    assert!(out.abs() < 1e-10);

    // Collision entropy of I/2: -log2(1/2) = 1.
    let mixed = maximally_mixed_1q();
    assert_eq!(density_matrix_renyi_entropy(mixed, 2.0, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Alpha equal to 1 falls back to the Von Neumann entropy (1 bit).
    assert_eq!(density_matrix_renyi_entropy(mixed, 1.0, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Non-positive alpha is rejected.
    assert_eq!(density_matrix_renyi_entropy(mixed, 0.0, &mut out), -8);

    density_matrix_free(mixed);
    density_matrix_free(bell);
}

#[test]
fn test_entanglement_entropy() {
    let mut out = -1.0f64;

    // Bell state: maximally entangled, S(rho_A) = 1 bit.
    let bell = bell_sv();
    let subsys = [0u32];
    assert_eq!(
        statevector_entanglement_entropy(bell, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!((out - 1.0).abs() < 1e-10);

    // Product state: no entanglement.
    let product = statevector_new(2);
    statevector_apply_h(product, 0);
    assert_eq!(
        statevector_entanglement_entropy(product, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!(out.abs() < 1e-10);

    // Invalid subsystems: empty, full system, out-of-bounds.
    assert_eq!(
        statevector_entanglement_entropy(bell, std::ptr::null(), 0, &mut out),
        -7
    );
    let full = [0u32, 1];
    assert_eq!(
        statevector_entanglement_entropy(bell, full.as_ptr(), 2, &mut out),
        -7
    );
    let oob = [5u32];
    assert_eq!(
        statevector_entanglement_entropy(bell, oob.as_ptr(), 1, &mut out),
        -2
    );
    // NULL qubit array with positive length.
    assert_eq!(
        statevector_entanglement_entropy(bell, std::ptr::null(), 1, &mut out),
        -1
    );

    statevector_free(product);
    statevector_free(bell);
}

#[test]
fn test_negativity() {
    let mut out = -1.0f64;

    // Bell state: negativity = 0.5.
    let bell = bell_dm();
    let subsys = [0u32];
    assert_eq!(
        density_matrix_negativity(bell, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!((out - 0.5).abs() < 1e-10);

    // Separable pure state: negativity = 0.
    let sep = density_matrix_new(2);
    assert_eq!(
        density_matrix_negativity(sep, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!(out.abs() < 1e-10);

    assert_eq!(
        density_matrix_negativity(bell, std::ptr::null(), 1, &mut out),
        -1
    );

    density_matrix_free(sep);
    density_matrix_free(bell);
}

#[test]
fn test_concurrence() {
    let mut out = -1.0f64;

    // Bell state: concurrence = 1.
    let bell = bell_dm();
    assert_eq!(density_matrix_concurrence(bell, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Separable pure state: concurrence = 0.
    let sep = density_matrix_new(2);
    assert_eq!(density_matrix_concurrence(sep, &mut out), 0);
    assert!(out.abs() < 1e-10);

    // Only 2-qubit states are supported.
    let three = density_matrix_new(3);
    assert_eq!(density_matrix_concurrence(three, &mut out), -7);

    density_matrix_free(three);
    density_matrix_free(sep);
    density_matrix_free(bell);
}

#[test]
fn test_entanglement_of_formation() {
    let mut out = -1.0f64;

    // Bell state: E_F = 1 bit.
    let bell = bell_dm();
    assert_eq!(density_matrix_entanglement_of_formation(bell, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Separable pure state: E_F = 0.
    let sep = density_matrix_new(2);
    assert_eq!(density_matrix_entanglement_of_formation(sep, &mut out), 0);
    assert!(out.abs() < 1e-10);

    assert_eq!(
        density_matrix_entanglement_of_formation(std::ptr::null(), &mut out),
        -1
    );

    density_matrix_free(sep);
    density_matrix_free(bell);
}

// ---------------------------------------------------------------------------
// 2.2 Metrics
// ---------------------------------------------------------------------------

#[test]
fn test_statevector_purity() {
    let mut out = 0.0f64;

    let sv = statevector_new(2);
    assert_eq!(statevector_purity(sv, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    let bell = bell_sv();
    assert_eq!(statevector_purity(bell, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    assert_eq!(statevector_purity(std::ptr::null(), &mut out), -1);

    statevector_free(bell);
    statevector_free(sv);
}

#[test]
fn test_density_matrix_purity() {
    let mut out = 0.0f64;

    let bell = bell_dm();
    assert_eq!(density_matrix_purity(bell, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    let mixed = maximally_mixed_1q();
    assert_eq!(density_matrix_purity(mixed, &mut out), 0);
    assert!((out - 0.5).abs() < 1e-10);

    density_matrix_free(mixed);
    density_matrix_free(bell);
}

#[test]
fn test_statevector_fidelity_and_trace_distance() {
    let mut out = -1.0f64;

    let bell = bell_sv();
    let same = bell_sv();
    assert_eq!(statevector_fidelity(bell, same, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);
    assert_eq!(statevector_trace_distance(bell, same, &mut out), 0);
    assert!(out.abs() < 1e-10);

    // Orthogonal pure states: fidelity 0, trace distance 1.
    let zero = statevector_new(1);
    let one = statevector_new(1);
    statevector_apply_x(one, 0);
    assert_eq!(statevector_fidelity(zero, one, &mut out), 0);
    assert!(out.abs() < 1e-10);
    assert_eq!(statevector_trace_distance(zero, one, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Qubit-count mismatch.
    assert_eq!(statevector_fidelity(bell, zero, &mut out), -8);

    assert_eq!(statevector_fidelity(std::ptr::null(), zero, &mut out), -1);
    assert_eq!(
        statevector_trace_distance(zero, std::ptr::null(), &mut out),
        -1
    );

    statevector_free(one);
    statevector_free(zero);
    statevector_free(same);
    statevector_free(bell);
}

#[test]
fn test_statevector_fidelity_pure_mixed() {
    let mut out = -1.0f64;

    // <Phi+| rho_Bell |Phi+> = 1.
    let bell = bell_sv();
    let bell_dm_ptr = bell_dm();
    assert_eq!(
        statevector_fidelity_pure_mixed(bell, bell_dm_ptr, &mut out),
        0
    );
    assert!((out - 1.0).abs() < 1e-10);

    // <0| I/2 |0> = 0.5.
    let zero = statevector_new(1);
    let mixed = maximally_mixed_1q();
    assert_eq!(statevector_fidelity_pure_mixed(zero, mixed, &mut out), 0);
    assert!((out - 0.5).abs() < 1e-10);

    // Qubit-count mismatch.
    assert_eq!(statevector_fidelity_pure_mixed(bell, mixed, &mut out), -8);

    density_matrix_free(mixed);
    density_matrix_free(bell_dm_ptr);
    statevector_free(zero);
    statevector_free(bell);
}

#[test]
fn test_density_matrix_entropy() {
    let mut out = -1.0f64;

    // Von Neumann entropy of I/2 is 1 bit.
    let mixed = maximally_mixed_1q();
    assert_eq!(density_matrix_entropy(mixed, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Pure state: entropy 0.
    let bell = bell_dm();
    assert_eq!(density_matrix_entropy(bell, &mut out), 0);
    assert!(out.abs() < 1e-10);

    density_matrix_free(bell);
    density_matrix_free(mixed);
}

#[test]
fn test_density_matrix_trace_distance_and_fidelity() {
    let mut out = -1.0f64;

    let bell = bell_dm();
    let same = bell_dm();

    assert_eq!(density_matrix_trace_distance(bell, same, &mut out), 0);
    assert!(out.abs() < 1e-10);
    assert_eq!(density_matrix_fidelity(bell, same, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // Orthogonal pure states (Bell vs |01>): distance 1, fidelity 0.
    let sep = density_matrix_new(2);
    assert_eq!(density_matrix_apply_x(sep, 0), 0);
    assert_eq!(density_matrix_trace_distance(bell, sep, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);
    assert_eq!(density_matrix_fidelity(bell, sep, &mut out), 0);
    assert!(out.abs() < 1e-10);

    // Qubit-count mismatch.
    let one_qubit = density_matrix_new(1);
    assert_eq!(density_matrix_trace_distance(bell, one_qubit, &mut out), -8);
    assert_eq!(density_matrix_fidelity(bell, one_qubit, &mut out), -8);

    density_matrix_free(one_qubit);
    density_matrix_free(sep);
    density_matrix_free(same);
    density_matrix_free(bell);
}

#[test]
fn test_partial_transpose() {
    // Partial transpose of the Bell state on qubit 0 has eigenvalues
    // (1/2, 1/2, 1/2, -1/2), so its purity is still 1.0.
    let bell = bell_dm();
    let qubits = [0u32];
    let pt = density_matrix_partial_transpose(bell, qubits.as_ptr(), 1);
    assert!(!pt.is_null());
    assert_eq!(density_matrix_num_qubits(pt), 2);

    let mut out = 0.0f64;
    assert_eq!(density_matrix_purity(pt, &mut out), 0);
    assert!((out - 1.0).abs() < 1e-10);

    // NULL qubit array with positive length is rejected.
    assert!(density_matrix_partial_transpose(bell, std::ptr::null(), 1).is_null());
    // Out-of-bounds qubit is rejected.
    let oob = [7u32];
    assert!(density_matrix_partial_transpose(bell, oob.as_ptr(), 1).is_null());

    density_matrix_free(pt);
    density_matrix_free(bell);
}

#[test]
fn test_logarithmic_negativity() {
    let mut out = -1.0f64;

    // Bell state: log2(||rho^T_A||_1) = log2(2) = 1.
    let bell = bell_dm();
    let subsys = [0u32];
    assert_eq!(
        density_matrix_logarithmic_negativity(bell, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!((out - 1.0).abs() < 1e-10);

    // Separable state: 0.
    let sep = density_matrix_new(2);
    assert_eq!(
        density_matrix_logarithmic_negativity(sep, subsys.as_ptr(), 1, &mut out),
        0
    );
    assert!(out.abs() < 1e-10);

    density_matrix_free(sep);
    density_matrix_free(bell);
}

// ---------------------------------------------------------------------------
// 2.3 Evolution circuits
// ---------------------------------------------------------------------------

#[test]
fn test_trotter_circuit() {
    // H = ZZ on 2 qubits.
    let h = hamiltonian_with("ZZ", 1.0);

    let circuit = hamiltonian_to_trotter_circuit(h, 1.0, 3, TROTTER_MODE_FIRST_ORDER);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    circuit_free(circuit);

    let circuit = hamiltonian_to_trotter_circuit(h, 1.0, 3, TROTTER_MODE_SECOND_ORDER);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    circuit_free(circuit);

    // Invalid mode tag, zero steps, and NULL handle all yield NULL.
    assert!(hamiltonian_to_trotter_circuit(h, 1.0, 3, 99).is_null());
    assert!(hamiltonian_to_trotter_circuit(h, 1.0, 0, TROTTER_MODE_FIRST_ORDER).is_null());
    assert!(
        hamiltonian_to_trotter_circuit(std::ptr::null(), 1.0, 3, TROTTER_MODE_FIRST_ORDER)
            .is_null()
    );

    hamiltonian_free(h);
}

#[test]
fn test_evolution_circuit() {
    // Commuting Hamiltonian: exact single-pass decomposition.
    let h = hamiltonian_with("ZZ", 0.5);
    let circuit = hamiltonian_to_evolution_circuit(h, 1.0, 1, TROTTER_MODE_FIRST_ORDER);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 2);
    circuit_free(circuit);
    hamiltonian_free(h);

    // Non-commuting Hamiltonian: falls back to Trotter.
    let h2 = hamiltonian_new(1);
    let px = pauli_string_parse(CString::new("X").unwrap().as_ptr());
    let pz = pauli_string_parse(CString::new("Z").unwrap().as_ptr());
    assert_eq!(hamiltonian_add_term(h2, px, 1.0, 0.0), 0);
    assert_eq!(hamiltonian_add_term(h2, pz, 1.0, 0.0), 0);
    pauli_string_free(px);
    pauli_string_free(pz);

    let circuit = hamiltonian_to_evolution_circuit(h2, 0.5, 10, TROTTER_MODE_SECOND_ORDER);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 1);
    circuit_free(circuit);
    hamiltonian_free(h2);

    // Empty Hamiltonian: NULL.
    let empty = hamiltonian_new(2);
    assert!(hamiltonian_to_evolution_circuit(empty, 1.0, 5, TROTTER_MODE_FIRST_ORDER).is_null());
    hamiltonian_free(empty);
}

// ---------------------------------------------------------------------------
// 2.4 Hamiltonian supplements
// ---------------------------------------------------------------------------

#[test]
fn test_hamiltonian_scale() {
    // 0.5 * ZZ scaled by 2.0 must equal ZZ.
    let h = hamiltonian_with("ZZ", 0.5);
    assert_eq!(hamiltonian_scale(h, 2.0, 0.0), 0);

    let len = hamiltonian_to_matrix_len(h);
    assert_eq!(len, 4);
    let mut buffer = vec![CComplex { re: 0.0, im: 0.0 }; len * len];
    assert_eq!(
        hamiltonian_to_matrix(h, buffer.as_mut_ptr() as *mut _, buffer.len()),
        0
    );

    // diag(1, -1, -1, 1) for ZZ.
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j {
                [1.0, -1.0, -1.0, 1.0][i]
            } else {
                0.0
            };
            assert!((buffer[i * 4 + j].re - expected).abs() < 1e-10);
            assert!(buffer[i * 4 + j].im.abs() < 1e-10);
        }
    }

    assert_eq!(hamiltonian_scale(std::ptr::null_mut(), 2.0, 0.0), -1);
    hamiltonian_free(h);
}

#[test]
fn test_hamiltonian_to_matrix_errors() {
    let h = hamiltonian_with("Z", 1.0);
    assert_eq!(hamiltonian_to_matrix_len(h), 2);

    let mut buffer = vec![CComplex { re: 0.0, im: 0.0 }; 4];
    // Wrong length.
    assert_eq!(
        hamiltonian_to_matrix(h, buffer.as_mut_ptr() as *mut _, 3),
        -8
    );
    // NULL buffer.
    assert_eq!(hamiltonian_to_matrix(h, std::ptr::null_mut(), 4), -1);
    // NULL handle.
    assert_eq!(hamiltonian_to_matrix_len(std::ptr::null()), 0);

    // Correct read: Z = diag(1, -1).
    assert_eq!(
        hamiltonian_to_matrix(h, buffer.as_mut_ptr() as *mut _, 4),
        0
    );
    assert!((buffer[0].re - 1.0).abs() < 1e-10);
    assert!((buffer[3].re + 1.0).abs() < 1e-10);

    hamiltonian_free(h);
}

#[test]
fn test_hamiltonian_all_terms_commute() {
    // ZZ and ZI commute.
    let h = hamiltonian_new(2);
    let zz = pauli_string_parse(CString::new("ZZ").unwrap().as_ptr());
    let zi = pauli_string_parse(CString::new("ZI").unwrap().as_ptr());
    assert_eq!(hamiltonian_add_term(h, zz, 1.0, 0.0), 0);
    assert_eq!(hamiltonian_add_term(h, zi, 0.5, 0.0), 0);
    assert_eq!(hamiltonian_all_terms_commute(h), 1);
    pauli_string_free(zz);
    pauli_string_free(zi);
    hamiltonian_free(h);

    // X and Z anticommute.
    let h2 = hamiltonian_new(1);
    let px = pauli_string_parse(CString::new("X").unwrap().as_ptr());
    let pz = pauli_string_parse(CString::new("Z").unwrap().as_ptr());
    assert_eq!(hamiltonian_add_term(h2, px, 1.0, 0.0), 0);
    assert_eq!(hamiltonian_add_term(h2, pz, 1.0, 0.0), 0);
    assert_eq!(hamiltonian_all_terms_commute(h2), 0);
    pauli_string_free(px);
    pauli_string_free(pz);
    hamiltonian_free(h2);

    assert_eq!(hamiltonian_all_terms_commute(std::ptr::null()), -1);
}

// ---------------------------------------------------------------------------
// 2.4 PauliString supplements
// ---------------------------------------------------------------------------

#[test]
fn test_pauli_string_commutes_with() {
    // X⊗Z and Z⊗X commute.
    let a = pauli_string_parse(CString::new("XZ").unwrap().as_ptr());
    let b = pauli_string_parse(CString::new("ZX").unwrap().as_ptr());
    assert_eq!(pauli_string_commutes_with(a, b), 1);
    assert_eq!(pauli_string_commutes_with(b, a), 1);

    // X and Z anticommute.
    let x = pauli_string_parse(CString::new("XI").unwrap().as_ptr());
    let z = pauli_string_parse(CString::new("ZI").unwrap().as_ptr());
    assert_eq!(pauli_string_commutes_with(x, z), 0);

    // Qubit-count mismatch and NULL handles.
    let single = pauli_string_parse(CString::new("X").unwrap().as_ptr());
    assert_eq!(pauli_string_commutes_with(a, single), -8);
    assert_eq!(pauli_string_commutes_with(std::ptr::null(), a), -1);

    pauli_string_free(single);
    pauli_string_free(z);
    pauli_string_free(x);
    pauli_string_free(b);
    pauli_string_free(a);
}

#[test]
fn test_pauli_string_masks_and_y_phase() {
    // "XY": qubit 1 = X, qubit 0 = Y.
    let ps = pauli_string_parse(CString::new("XY").unwrap().as_ptr());
    assert!(!ps.is_null());
    assert_eq!(pauli_string_x_mask(ps), 0b11);
    assert_eq!(pauli_string_z_mask(ps), 0b01);

    let mut phase = CComplex { re: 0.0, im: 0.0 };
    assert_eq!(
        pauli_string_y_phase(ps, &mut phase as *mut CComplex as *mut _),
        0
    );
    assert!((phase.re - 0.0).abs() < 1e-10);
    assert!((phase.im - 1.0).abs() < 1e-10);
    pauli_string_free(ps);

    // "YY": two Y operators give phase i^2 = -1.
    let ps = pauli_string_parse(CString::new("YY").unwrap().as_ptr());
    assert_eq!(
        pauli_string_y_phase(ps, &mut phase as *mut CComplex as *mut _),
        0
    );
    assert!((phase.re + 1.0).abs() < 1e-10);
    assert!(phase.im.abs() < 1e-10);
    assert_eq!(pauli_string_x_mask(ps), 0b11);
    assert_eq!(pauli_string_z_mask(ps), 0b11);
    pauli_string_free(ps);

    // No Y operators: phase = 1.
    let ps = pauli_string_parse(CString::new("XZ").unwrap().as_ptr());
    assert_eq!(
        pauli_string_y_phase(ps, &mut phase as *mut CComplex as *mut _),
        0
    );
    assert!((phase.re - 1.0).abs() < 1e-10);
    pauli_string_free(ps);

    // NULL handling.
    assert_eq!(pauli_string_x_mask(std::ptr::null()), 0);
    assert_eq!(pauli_string_z_mask(std::ptr::null()), 0);
    assert_eq!(
        pauli_string_y_phase(std::ptr::null(), &mut phase as *mut CComplex as *mut _),
        -1
    );
}

#[test]
fn test_pauli_string_support() {
    // "XZI": qubit 2 = X, qubit 1 = Z, qubit 0 = I -> support [1, 2].
    let ps = pauli_string_parse(CString::new("XZI").unwrap().as_ptr());
    assert_eq!(pauli_string_support_len(ps), 2);

    let mut buffer = [0u32; 2];
    assert_eq!(pauli_string_support(ps, buffer.as_mut_ptr(), 2), 0);
    assert_eq!(buffer, [1, 2]);

    // Wrong length is rejected.
    assert_eq!(pauli_string_support(ps, buffer.as_mut_ptr(), 3), -8);
    assert_eq!(pauli_string_support(ps, std::ptr::null_mut(), 2), -1);

    pauli_string_free(ps);

    // Identity string has empty support.
    let ps = pauli_string_parse(CString::new("III").unwrap().as_ptr());
    assert_eq!(pauli_string_support_len(ps), 0);
    assert_eq!(pauli_string_support(ps, buffer.as_mut_ptr(), 0), 0);
    pauli_string_free(ps);

    assert_eq!(pauli_string_support_len(std::ptr::null()), 0);
}

#[test]
fn test_pauli_string_try_get_set_pauli() {
    // "XZI": qubit 2 = X, qubit 1 = Z, qubit 0 = I.
    let ps = pauli_string_parse(CString::new("XZI").unwrap().as_ptr());
    let mut tag: u8 = 0xFF;
    assert_eq!(pauli_string_try_get_pauli(ps, 0, &mut tag), 0);
    assert_eq!(tag, PAULI_I);
    assert_eq!(pauli_string_try_get_pauli(ps, 1, &mut tag), 0);
    assert_eq!(tag, PAULI_Z);
    assert_eq!(pauli_string_try_get_pauli(ps, 2, &mut tag), 0);
    assert_eq!(tag, PAULI_X);

    // Out of bounds: -2 (IndexOutOfBounds).
    assert_eq!(pauli_string_try_get_pauli(ps, 3, &mut tag), -2);
    assert_eq!(
        pauli_string_try_get_pauli(std::ptr::null(), 0, &mut tag),
        -1
    );
    pauli_string_free(ps);

    // try_set_pauli: roundtrip and error paths.
    let ps = pauli_string_new(2);
    assert!(!ps.is_null());
    assert_eq!(pauli_string_try_set_pauli(ps, 0, PAULI_Y), 0);
    assert_eq!(pauli_string_try_get_pauli(ps, 0, &mut tag), 0);
    assert_eq!(tag, PAULI_Y);
    assert_eq!(pauli_string_try_set_pauli(ps, 5, PAULI_X), -2);
    assert_eq!(pauli_string_try_set_pauli(ps, 0, 200), -8);
    assert_eq!(
        pauli_string_try_set_pauli(std::ptr::null_mut(), 0, PAULI_X),
        -1
    );
    pauli_string_free(ps);
}

#[test]
fn test_pauli_tag_roundtrip() {
    // Every Pauli tag survives a set/get roundtrip through the symplectic
    // representation.
    let tags = [PAULI_I, PAULI_X, PAULI_Y, PAULI_Z];
    for (idx, &expected) in tags.iter().enumerate() {
        let ps = pauli_string_new(4);
        assert!(!ps.is_null());
        assert_eq!(pauli_string_try_set_pauli(ps, idx, expected), 0);
        let mut tag: u8 = 0xFF;
        assert_eq!(pauli_string_try_get_pauli(ps, idx, &mut tag), 0);
        assert_eq!(tag, expected);
        pauli_string_free(ps);
    }
}
