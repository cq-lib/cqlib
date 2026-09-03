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

use ndarray::{Array2, array};
use num_complex::Complex64;

pub(super) const fn c(re: f64, im: f64) -> Complex64 {
    Complex64::new(re, im)
}

pub(super) fn mat2(
    a00: Complex64,
    a01: Complex64,
    a10: Complex64,
    a11: Complex64,
) -> Array2<Complex64> {
    array![[a00, a01], [a10, a11]]
}

pub(super) fn dagger(matrix: &Array2<Complex64>) -> Array2<Complex64> {
    matrix.t().mapv(|value| value.conj())
}

pub(super) fn det_2x2(m: &Array2<Complex64>) -> Complex64 {
    m[[0, 0]] * m[[1, 1]] - m[[0, 1]] * m[[1, 0]]
}
