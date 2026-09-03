// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use crate::circuit::test_utils::assert_is_unitary;
use std::f64::consts::PI;

fn assert_matrix_approx_eq(a: &Array2<Complex<f64>>, b: &Array2<Complex<f64>>) {
    assert_eq!(a.shape(), b.shape(), "Matrix shapes differ");
    for (x, y) in a.iter().zip(b.iter()) {
        let diff = (x - y).norm();
        if diff > f64::EPSILON {
            panic!(
                "Matrices not equal. Diff: {}, Expected: {}, Actual: {}",
                diff, y, x
            );
        }
    }
}

#[test]
fn test_fixed_gates_unitary() {
    // Batch verify unitarity for all fixed gates
    let gates = vec![
        (&*H_GATE, "H"),
        (&*I_GATE, "I"),
        (&*X_GATE, "X"),
        (&*Y_GATE, "Y"),
        (&*Z_GATE, "Z"),
        (&*S_GATE, "S"),
        (&*SDG_GATE, "SDG"),
        (&*T_GATE, "T"),
        (&*TDG_GATE, "TDG"),
        (&*SWAP_GATE, "SWAP"),
        (&*ISWAP_GATE, "ISWAP"),
        (&*CX_GATE, "CX"),
        (&*CY_GATE, "CY"),
        (&*CZ_GATE, "CZ"),
        (&*CCX_GATE, "CCX"),
        (&*X2P_GATE, "X2P"),
        (&*X2M_GATE, "X2M"),
        (&*Y2P_GATE, "Y2P"),
        (&*Y2M_GATE, "Y2M"),
    ];

    for (gate, _) in gates {
        assert_is_unitary(gate, 1e-10);
    }
}

#[test]
fn test_parameterized_gates_unitary() {
    // Verify unitarity for parameterized gates with random angles
    let thetas = vec![0.0, PI / 2.0, PI, 2.0 * PI, -0.5, 1.234];

    for &theta in &thetas {
        assert_is_unitary(&rx_gate(theta), 1e-10);
        assert_is_unitary(&ry_gate(theta), 1e-10);
        assert_is_unitary(&rz_gate(theta), 1e-10);
        assert_is_unitary(&phase_gate(theta), 1e-10);
        assert_is_unitary(&global_phase_gate(theta), 1e-10);
        assert_is_unitary(&u_gate(theta, 0.5, -0.5), 1e-10);

        // Two-qubit parameterized gates
        assert_is_unitary(&rxx_gate(theta), 1e-10);
        assert_is_unitary(&ryy_gate(theta), 1e-10);
        assert_is_unitary(&rzz_gate(theta), 1e-10);
        assert_is_unitary(&rzx_gate(theta), 1e-10);
        assert_is_unitary(&rxy_gate(theta, 0.5), 1e-10);
        assert_is_unitary(&crx_gate(theta), 1e-10);
        assert_is_unitary(&cry_gate(theta), 1e-10);
        assert_is_unitary(&crz_gate(theta), 1e-10);
        assert_is_unitary(&xy_gate(theta), 1e-10);
        assert_is_unitary(&xy2p_gate(theta), 1e-10);
        assert_is_unitary(&xy2m_gate(theta), 1e-10);

        // fSim
        assert_is_unitary(&fsim_gate(theta, theta / 2.0), 1e-10);
    }
}

#[test]
fn test_specific_physics_identities() {
    // 1. H * H = I
    let hh = H_GATE.dot(&*H_GATE);
    assert_matrix_approx_eq(&hh, &I_GATE);

    // 2. X * X = I
    let xx = X_GATE.dot(&*X_GATE);
    assert_matrix_approx_eq(&xx, &I_GATE);

    // 3. RX(0) = I
    assert_matrix_approx_eq(&rx_gate(0.0), &I_GATE);

    // 4. RX(2*pi) = -I (Global Phase -1, Note: RX(2pi) = cos(pi)I - i*sin(pi)X = -I)
    let rx_2pi = rx_gate(2.0 * PI);
    let neg_eye = &*I_GATE * Complex::new(-1.0, 0.0);
    assert_matrix_approx_eq(&rx_2pi, &neg_eye);

    // 5. RZ(pi) = -i * Z
    // RZ(pi) = [[exp(-i*pi/2), 0], [0, exp(i*pi/2)]] = [[-i, 0], [0, i]] = -i * [[1, 0], [0, -1]] = -iZ
    let rz_pi = rz_gate(PI);
    let expected = &*Z_GATE * Complex::new(0.0, -1.0);
    assert_matrix_approx_eq(&rz_pi, &expected);
}
