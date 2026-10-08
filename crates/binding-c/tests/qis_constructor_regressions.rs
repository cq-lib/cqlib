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

//! Invalid dimensions must return NULL instead of unwinding across the C ABI.
use binding_c::qis::*;

#[test]
fn invalid_dimensions_return_null() {
    for n in [usize::BITS as usize - 4, usize::BITS as usize, usize::MAX] {
        assert!(statevector_new(n).is_null());
    }
    for n in [
        (usize::BITS as usize - 4).div_ceil(2),
        usize::BITS as usize,
        usize::MAX,
    ] {
        assert!(density_matrix_new(n).is_null());
        assert!(density_matrix_zeros(n).is_null());
        assert!(density_matrix_maximally_mixed(n).is_null());
        assert!(density_matrix_noise_new(n, std::ptr::null()).is_null());
    }
}

#[test]
fn normal_and_zero_qubit_constructors_remain_valid() {
    for n in [0, 1, 3] {
        let sv = statevector_new(n);
        assert!(!sv.is_null());
        statevector_free(sv);
        for dm in [
            density_matrix_new(n),
            density_matrix_zeros(n),
            density_matrix_maximally_mixed(n),
        ] {
            assert!(!dm.is_null());
            density_matrix_free(dm);
        }
        let noisy = density_matrix_noise_new(n, std::ptr::null());
        assert!(!noisy.is_null());
        density_matrix_noise_free(noisy);
    }
}
