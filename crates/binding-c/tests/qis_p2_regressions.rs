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

use binding_c::qis::*;
use num_complex::Complex64;

#[test]
fn nonfinite_states_return_null() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        for value in [Complex64::new(bad, 0.0), Complex64::new(0.0, bad)] {
            let data = [value, Complex64::new(0.0, 0.0)];
            assert!(statevector_from_state(1, data.as_ptr(), data.len()).is_null());
            assert!(density_matrix_from_state(1, data.as_ptr(), data.len()).is_null());
        }
    }
}

#[test]
fn nonhermitian_expectation_reports_error_without_overwriting_output() {
    let sv = statevector_new(1);
    let dm = density_matrix_new(1);
    let h = hamiltonian_new(1);
    let p = pauli_string_parse(c"Z".as_ptr());
    assert_eq!(hamiltonian_add_term(h, p, 0.0, 1.0), 0);
    let mut out = 123.0;
    assert_ne!(statevector_expectation(sv, h, &mut out), 0);
    assert_eq!(out, 123.0);
    assert_ne!(density_matrix_expectation(dm, h, &mut out), 0);
    assert_eq!(out, 123.0);
    // The imaginary parts cancel: the observable is now Z.
    assert_eq!(hamiltonian_add_term(h, p, 1.0, -1.0), 0);
    assert_eq!(statevector_expectation(sv, h, &mut out), 0);
    assert_eq!(out, 1.0);
    assert_eq!(density_matrix_expectation(dm, h, &mut out), 0);
    assert_eq!(out, 1.0);
    pauli_string_free(p);
    hamiltonian_free(h);
    statevector_free(sv);
    density_matrix_free(dm);
}
