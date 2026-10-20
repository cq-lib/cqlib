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

//! End-to-end check of the Block-ZXZ C ABI, calling the exported
//! `extern "C"` functions exactly the way a C client would.

use binding_c::compile::decompose::{
    block_zxz_decompose, block_zxz_options_default, block_zxz_result_circuit,
    block_zxz_result_cx_count, block_zxz_result_free, block_zxz_result_num_qubits,
    BLOCK_ZXZ_OPT_A1, BLOCK_ZXZ_OPT_A2,
};
use ndarray::Array2;
use num_complex::Complex64;

/// Deterministic pseudo-random unitary via xorshift + Gram-Schmidt.
fn seeded_unitary(dim: usize, seed: u64) -> Array2<Complex64> {
    let mut state = seed | 1;
    let mut uniform = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };

    let mut basis: Vec<Vec<Complex64>> = Vec::with_capacity(dim);
    for _ in 0..dim {
        let mut w: Vec<Complex64> = (0..dim)
            .map(|_| Complex64::new(uniform() * 2.0 - 1.0, uniform() * 2.0 - 1.0))
            .collect();
        let mut attempts = 0usize;
        loop {
            for b in &basis {
                // Projection coefficient <b, w> = sum(conj(b_i) * w_i).
                let dot: Complex64 = w.iter().zip(b.iter()).map(|(x, y)| y.conj() * x).sum();
                for (wi, bi) in w.iter_mut().zip(b.iter()) {
                    *wi -= dot * bi;
                }
            }
            let norm: f64 = w.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            if norm > 1e-6 {
                for wi in &mut w {
                    *wi /= norm;
                }
                break;
            }
            // Degenerate draw (measure-zero event): redraw this column.
            attempts += 1;
            assert!(attempts < 32, "Gram-Schmidt failed to converge");
            for wi in &mut w {
                *wi = Complex64::new(uniform() * 2.0 - 1.0, uniform() * 2.0 - 1.0);
            }
        }
        basis.push(w);
    }

    let mut unitary = Array2::<Complex64>::zeros((dim, dim));
    for (col, b) in basis.iter().enumerate() {
        for (row, &z) in b.iter().enumerate() {
            unitary[[row, col]] = z;
        }
    }
    unitary
}

/// Max elementwise deviation after optimal global-phase alignment.
fn phase_aligned_error(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
    let overlap: Complex64 = b
        .iter()
        .zip(a.iter())
        .map(|(y, x)| y.conj() * x)
        .sum();
    if overlap.norm() < 1e-8 {
        return a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (*x - *y).norm())
            .fold(0.0, f64::max);
    }
    let phase = Complex64::from_polar(1.0, overlap.arg());
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (*x - y * phase).norm())
        .fold(0.0, f64::max)
}

#[test]
fn block_zxz_c_api_counts_and_reconstruction() {
    assert_eq!(block_zxz_options_default(), BLOCK_ZXZ_OPT_A1 | BLOCK_ZXZ_OPT_A2);

    // (num_qubits, full-optimization CX, unoptimized baseline CX)
    for (num_qubits, full_cx, base_cx) in
        [(2usize, 3usize, 3usize), (3, 19, 24), (4, 95, 120)]
    {
        let dim = 1usize << num_qubits;
        let unitary = seeded_unitary(dim, 42 + num_qubits as u64);
        let flat: Vec<Complex64> = unitary.iter().cloned().collect();

        // Fully optimized defaults.
        let full = block_zxz_decompose(
            flat.as_ptr(),
            num_qubits,
            1e-10,
            block_zxz_options_default(),
        );
        assert!(!full.is_null(), "n={num_qubits}: decompose returned NULL");
        unsafe {
            assert_eq!(
                block_zxz_result_num_qubits(full),
                num_qubits,
                "n={num_qubits}: qubit count mismatch"
            );
            assert_eq!(
                block_zxz_result_cx_count(full),
                full_cx,
                "n={num_qubits}: CX count mismatch (paper Table 1)"
            );

            let reconstructed = (*full)
                .inner
                .circuit
                .to_matrix(None)
                .expect("to_matrix failed");
            let error = phase_aligned_error(&unitary, &reconstructed);
            assert!(
                error < 1e-9,
                "n={num_qubits}: reconstruction error {error:.3e}"
            );

            // Extracted circuit handle is valid and independent.
            let circuit = block_zxz_result_circuit(full);
            assert!(!circuit.is_null());
            assert_eq!(
                (*circuit).inner.num_qubits(),
                num_qubits
            );
            drop(Box::from_raw(circuit));
            block_zxz_result_free(full);
        }

        // Flag 0: unoptimized baseline.
        let base = block_zxz_decompose(flat.as_ptr(), num_qubits, 1e-10, 0);
        assert!(!base.is_null(), "n={num_qubits}: baseline returned NULL");
        assert_eq!(
            block_zxz_result_cx_count(base),
            base_cx,
            "n={num_qubits}: baseline CX count mismatch"
        );
        block_zxz_result_free(base);
    }
}

#[test]
fn block_zxz_c_api_rejects_invalid_input() {
    let identity_2x2 = [
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
    ];

    // NULL matrix.
    assert!(block_zxz_decompose(std::ptr::null(), 1, 1e-10, 0).is_null());

    // Out-of-range qubit counts (0 and beyond the documented cap).
    assert!(block_zxz_decompose(identity_2x2.as_ptr(), 0, 1e-10, 0).is_null());
    assert!(block_zxz_decompose(identity_2x2.as_ptr(), 13, 1e-10, 0).is_null());

    // Non-unitary 2x2 input.
    let garbage = [
        Complex64::new(2.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
    ];
    assert!(block_zxz_decompose(garbage.as_ptr(), 1, 1e-10, 0).is_null());

    // NULL-safe accessors and free.
    assert_eq!(block_zxz_result_num_qubits(std::ptr::null()), 0);
    assert_eq!(block_zxz_result_cx_count(std::ptr::null()), 0);
    assert!(block_zxz_result_circuit(std::ptr::null()).is_null());
    block_zxz_result_free(std::ptr::null_mut());
}
