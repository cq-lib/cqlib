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

//! Rust FFI parity tests for the third QIS binding batch:
//! `statevector_data_mut`, `density_matrix_is_positive_semidefinite`, and
//! `pauli_mul_with_phase`.

use binding_c::qis::{
    PAULI_I, PAULI_X, PAULI_Y, PAULI_Z, density_matrix_free, density_matrix_from_state,
    density_matrix_is_positive_semidefinite, density_matrix_maximally_mixed, density_matrix_new,
    density_matrix_zeros, pauli_mul_with_phase, statevector_apply_z, statevector_data,
    statevector_data_len, statevector_data_mut, statevector_free, statevector_new,
};
use cqlib_core::qis::{Pauli, Phase};
use num_complex::Complex64;
use std::ptr;

fn read_amplitudes(ptr: *const binding_c::qis::CStatevector) -> Vec<Complex64> {
    let len = statevector_data_len(ptr);
    let mut buffer = vec![Complex64::new(0.0, 0.0); len];
    assert_eq!(statevector_data(ptr, buffer.as_mut_ptr(), len), 0);
    buffer
}

#[test]
fn statevector_data_mut_overwrites_amplitudes() {
    let sv = statevector_new(1);
    assert_eq!(statevector_data_len(sv), 2);

    // Overwrite |0> with |1>.
    let one = [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)];
    assert_eq!(statevector_data_mut(sv, one.as_ptr(), 2), 0);
    let amps = read_amplitudes(sv);
    assert_eq!(amps[0], Complex64::new(0.0, 0.0));
    assert_eq!(amps[1], Complex64::new(1.0, 0.0));

    // The simulator keeps using the written amplitudes: Z|1> = -|1>.
    assert_eq!(statevector_apply_z(sv, 0), 0);
    let amps = read_amplitudes(sv);
    assert!(amps[0].norm() < 1e-12);
    assert_eq!(amps[1], Complex64::new(-1.0, 0.0));

    // A normalized superposition can also be written directly.
    let inv = std::f64::consts::FRAC_1_SQRT_2;
    let plus = [Complex64::new(inv, 0.0), Complex64::new(inv, 0.0)];
    assert_eq!(statevector_data_mut(sv, plus.as_ptr(), 2), 0);
    let amps = read_amplitudes(sv);
    assert!((amps[0].re - inv).abs() < 1e-12);
    assert!((amps[1].re - inv).abs() < 1e-12);

    // A length mismatch is rejected and leaves the state untouched.
    let short = [Complex64::new(9.0, 0.0)];
    assert_eq!(statevector_data_mut(sv, short.as_ptr(), 1), -8);
    let amps = read_amplitudes(sv);
    assert!((amps[0].re - inv).abs() < 1e-12);

    // NULL pointers are rejected.
    assert_eq!(statevector_data_mut(ptr::null_mut(), plus.as_ptr(), 2), -1);
    assert_eq!(statevector_data_mut(sv, ptr::null(), 2), -1);

    statevector_free(sv);
}

#[test]
fn density_matrix_psd_reports_physical_states() {
    // Ground state |0><0| is positive semidefinite.
    let dm = density_matrix_new(1);
    let mut positive = false;
    assert_eq!(
        density_matrix_is_positive_semidefinite(dm, 1e-10, &mut positive),
        0
    );
    assert!(positive);
    density_matrix_free(dm);

    // Maximally mixed state I/4 is positive semidefinite.
    let mixed = density_matrix_maximally_mixed(2);
    assert_eq!(
        density_matrix_is_positive_semidefinite(mixed, 1e-10, &mut positive),
        0
    );
    assert!(positive);
    density_matrix_free(mixed);

    // Pure |+><+| state is positive semidefinite.
    let inv = std::f64::consts::FRAC_1_SQRT_2;
    let plus = [Complex64::new(inv, 0.0), Complex64::new(inv, 0.0)];
    let pure = density_matrix_from_state(1, plus.as_ptr(), 2);
    assert!(!pure.is_null());
    assert_eq!(
        density_matrix_is_positive_semidefinite(pure, 1e-10, &mut positive),
        0
    );
    assert!(positive);
    density_matrix_free(pure);

    // The zero matrix has zero eigenvalues and also passes.
    let zero = density_matrix_zeros(1);
    assert_eq!(
        density_matrix_is_positive_semidefinite(zero, 1e-10, &mut positive),
        0
    );
    assert!(positive);
    density_matrix_free(zero);

    // Error paths.
    assert_eq!(
        density_matrix_is_positive_semidefinite(ptr::null(), 1e-10, &mut positive),
        -1
    );
    let valid = density_matrix_new(1);
    assert_eq!(
        density_matrix_is_positive_semidefinite(valid, 1e-10, ptr::null_mut()),
        -1
    );
    assert_eq!(
        density_matrix_is_positive_semidefinite(valid, f64::NAN, &mut positive),
        -8
    );
    density_matrix_free(valid);
}

#[test]
fn pauli_mul_with_phase_matches_core() {
    // X * Z = -iY.
    let mut tag = 0xFFu8;
    let mut phase = Complex64::new(9.0, 9.0);
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, PAULI_Z, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_Y);
    assert_eq!(phase, Complex64::new(0.0, -1.0));

    // Z * X = iY (anticommutation reverses the sign).
    assert_eq!(
        pauli_mul_with_phase(PAULI_Z, PAULI_X, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_Y);
    assert_eq!(phase, Complex64::new(0.0, 1.0));

    // X * X = I with phase +1; X * Y = iZ.
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, PAULI_X, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_I);
    assert_eq!(phase, Complex64::new(1.0, 0.0));
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, PAULI_Y, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_Z);
    assert_eq!(phase, Complex64::new(0.0, 1.0));

    // Identity leaves the other operator untouched with phase +1.
    assert_eq!(
        pauli_mul_with_phase(PAULI_I, PAULI_Y, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_Y);
    assert_eq!(phase, Complex64::new(1.0, 0.0));
    assert_eq!(
        pauli_mul_with_phase(PAULI_Y, PAULI_I, &mut tag, &mut phase),
        0
    );
    assert_eq!(tag, PAULI_Y);
    assert_eq!(phase, Complex64::new(1.0, 0.0));

    // Full 4x4 parity check against the core implementation.
    let tags = [PAULI_I, PAULI_X, PAULI_Y, PAULI_Z];
    let core = [Pauli::I, Pauli::X, Pauli::Y, Pauli::Z];
    for (l_tag, l_core) in tags.iter().zip(core.iter()) {
        for (r_tag, r_core) in tags.iter().zip(core.iter()) {
            let (expected_pauli, expected_phase) = l_core.mul_with_phase(*r_core);
            assert_eq!(
                pauli_mul_with_phase(*l_tag, *r_tag, &mut tag, &mut phase),
                0
            );
            let expected_tag = match expected_pauli {
                Pauli::I => PAULI_I,
                Pauli::X => PAULI_X,
                Pauli::Y => PAULI_Y,
                Pauli::Z => PAULI_Z,
            };
            assert_eq!(tag, expected_tag);
            assert_eq!(phase, expected_phase.to_complex());
            // Sanity: core phases are always a fourth root of unity.
            assert!(matches!(
                expected_phase,
                Phase::Plus | Phase::I | Phase::Minus | Phase::MinusI
            ));
        }
    }

    // Invalid tags are rejected, and NULL output pointers return -1.
    assert_eq!(pauli_mul_with_phase(9, PAULI_X, &mut tag, &mut phase), -8);
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, 0xFF, &mut tag, &mut phase),
        -8
    );
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, PAULI_Z, ptr::null_mut(), &mut phase),
        -1
    );
    assert_eq!(
        pauli_mul_with_phase(PAULI_X, PAULI_Z, &mut tag, ptr::null_mut()),
        -1
    );
}
