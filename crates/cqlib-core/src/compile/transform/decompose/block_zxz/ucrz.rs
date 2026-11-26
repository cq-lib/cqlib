// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2025-2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Uniformly controlled RZ on the MSB implements D ⊕ D†. Equation (4)
//! is a Walsh–Hadamard transform with Gray-permuted columns.

use crate::circuit::{Circuit, CircuitError, Qubit};
use ndarray::Array1;
use num_complex::Complex64;

pub(super) enum Chain {
    Full,
    OpenTail,
    OpenHead,
}

pub(super) fn append(
    output: &mut Circuit,
    d: &Array1<Complex64>,
    chain: Chain,
) -> Result<(), CircuitError> {
    let mut angles: Vec<_> = d.iter().map(|z| -2.0 * z.arg()).collect();
    let n = angles.len();
    let mut stride = 1;
    while stride < n {
        for chunk in angles.chunks_exact_mut(2 * stride) {
            let (left, right) = chunk.split_at_mut(stride);
            for (a, b) in left.iter_mut().zip(right) {
                (*a, *b) = (*a + *b, *a - *b);
            }
        }
        stride *= 2;
    }
    let target = Qubit::new(n.ilog2());
    let gray = |i: usize| i ^ (i >> 1);
    for step in 0..n {
        // Reversal transposes the full chain. Its diagonal matrix is symmetric.
        let reverse = matches!(chain, Chain::OpenHead);
        let i = if reverse { n - 1 - step } else { step };
        let control = Qubit::new((gray(i) ^ gray((i + 1) % n)).trailing_zeros());
        if reverse && i + 1 < n {
            output.cx(control, target)?;
        }
        output.rz(target, angles[gray(i)] / n as f64)?;
        if !reverse && (i + 1 < n || matches!(chain, Chain::Full)) {
            output.cx(control, target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::synthesis::{Matrix, synthesis_test::assert_matrix_close};
    use super::*;

    #[test]
    fn restoring_either_open_boundary_reconstructs_every_control_word() {
        // Close the missing CX on the appropriate side independently of the
        // Gray-code generator. Each diagonal phase labels a distinct control word.
        for controls in 1..=4 {
            let size = 1 << controls;
            let d = Array1::from_shape_fn(size, |i| {
                Complex64::from_polar(1., 0.21 * (i * i + 1) as f64)
            });
            let expected = Matrix::from_diag(&ndarray::concatenate![
                ndarray::Axis(0),
                d,
                d.mapv(|z| z.conj())
            ]);
            for chain in [Chain::Full, Chain::OpenHead, Chain::OpenTail] {
                let mut circuit = Circuit::new(controls + 1);
                let top = Qubit::new(controls as u32);
                let control = Qubit::new(controls as u32 - 1);
                let open_tail = matches!(chain, Chain::OpenTail);
                if matches!(chain, Chain::OpenHead) {
                    circuit.cx(control, top).unwrap();
                }
                append(&mut circuit, &d, chain).unwrap();
                if open_tail {
                    circuit.cx(control, top).unwrap();
                }
                assert_matrix_close(&circuit.to_matrix(None).unwrap(), &expected, 1e-13);
            }
        }
    }
}
