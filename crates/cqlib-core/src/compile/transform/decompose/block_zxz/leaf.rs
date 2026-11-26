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

//! A.2 carries a diagonal on the two lowest bits across each higher-bit
//! rotation, Hadamard, or controlled X. None of those gates changes either
//! low bit. The last leaf consumes the diagonal without producing another.

use super::factors::{residual, to_faer};
use super::synthesis::{BlockZxzDecomposeOptions, Matrix, Synthesis, failed};
use crate::circuit::{Circuit, Parameter, Qubit, StandardGate};
use crate::compile::CompilerError;
use crate::compile::transform::decompose::unitary::{
    TWO_QUBIT_EXACT_TOLERANCE, TwoQubitUnitaryDecomposeBasis, synthesize_numeric_2q_unitary,
};
use ndarray::{Array1, array};
use num_complex::Complex64 as C;

type Diagonal = Array1<C>;
type Leaf = (Circuit, Option<Diagonal>);

fn kak_circuit(u: &Matrix, atol: f64) -> Result<Circuit, CompilerError> {
    // The first KAK argument represents the MSB of the 4×4 matrix.
    let result = synthesize_numeric_2q_unitary(
        u,
        [Qubit::new(1), Qubit::new(0)],
        TwoQubitUnitaryDecomposeBasis::Cx,
    )?;
    let mut c = Circuit::from_operations(
        vec![Qubit::new(0), Qubit::new(1)],
        result.operations,
        None,
        None,
    )?;
    c.set_global_phase(Parameter::from(result.global_phase));
    // KAK already checked its 4×4 candidate at its own tolerance. A caller
    // requesting higher precision can reject a weak interaction rounded away.
    if atol < TWO_QUBIT_EXACT_TOLERANCE {
        let error = residual(&c.to_matrix(None)?, u);
        if error > atol {
            return Err(failed(format!(
                "KAK leaf residual {error:e} exceeds {atol:e}"
            )));
        }
    }
    Ok(c)
}

/// Shende–Markov–Bullock, Proposition III.3. For S = Y ⊗ Y and M = U S Uᵀ,
/// `U` already lies in the <= 2-CX class iff `Im(tr(γ(U_SU4))) == 0`, where
/// `tr(γ(U_SU4)) = 2 (M[1,2] - M[0,3]) exp(-i arg(det U) / 2)`. Returns `None`
/// in that case so simple leaves are not perturbed by an unnecessary diagonal;
/// otherwise selects two row phases so the SU(4)-normalized spin-flip trace is real.
/// This is an ordinary transpose, not an adjoint. S has signed antidiagonal
/// (-1,1,1,-1), and only entries (1,2) and (0,3) of M enter the equation.
fn reducing_diagonal(u: &Matrix) -> Option<Diagonal> {
    let entry = |i: usize, j: usize| {
        -u[[i, 0]] * u[[j, 3]] + u[[i, 1]] * u[[j, 2]] + u[[i, 2]] * u[[j, 1]]
            - u[[i, 3]] * u[[j, 0]]
    };
    let a = entry(1, 2);
    let b = entry(0, 3);
    let det_arg = to_faer(u.view()).determinant().arg();
    let trace_su4 = (a - b) * C::from_polar(2.0, -0.5 * det_arg);
    if trace_su4.im.abs() <= 1e-13 {
        return None;
    }
    let angle = a.arg() + b.arg() - det_arg;
    let root = (b.norm() * angle.sin()).atan2(a.norm() + b.norm() * angle.cos());
    Some(array![
        C::ONE,
        C::ONE,
        C::from_polar(1.0, root - a.arg()),
        C::from_polar(1.0, angle - root - b.arg())
    ])
}

fn synthesize(u: &Matrix, reduce: bool, atol: f64) -> Result<Leaf, CompilerError> {
    if reduce && let Some(d) = reducing_diagonal(u) {
        let target = u * &d.view().insert_axis(ndarray::Axis(1));
        if let Ok(c) = kak_circuit(&target, atol)
            && c.operations()
                .iter()
                .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                .count()
                <= 2
        {
            // C already owns its KAK phase: C = D U, hence D† C = U.
            return Ok((c, Some(d.mapv(|z| z.conj()))));
        }
    }
    Ok((kak_circuit(u, atol)?, None))
}

fn append_diagonal(output: &mut Circuit, d: &Diagonal) -> Result<(), CompilerError> {
    let phase = d.mapv(|z| z.arg());
    let alpha = phase[2] - phase[0];
    let beta = phase[1] - phase[0];
    let gamma = phase[3] - phase[2] - phase[1] + phase[0];
    let parity = -gamma / 2.0;
    let low = beta + gamma / 2.0;
    let high = alpha + gamma / 2.0;
    let (q0, q1) = (Qubit::new(0), Qubit::new(1));
    output.cx(q1, q0)?;
    output.rz(q0, parity)?;
    output.cx(q1, q0)?;
    output.rz(q0, low)?;
    output.rz(q1, high)?;
    // On |00⟩ every RZ contributes exp(-i θ/2). Restore d00 exactly,
    // including the P(θ) = exp(i θ/2) RZ(θ) conversion factors.
    output.set_global_phase(
        output.global_phase() + Parameter::from(phase[0] + (parity + low + high) / 2.0),
    );
    Ok(())
}

pub(super) fn append(
    output: &mut Circuit,
    u: &Matrix,
    pending: &mut Option<Diagonal>,
    reduce: bool,
    atol: f64,
) -> Result<(), CompilerError> {
    append_with(output, u, pending, atol, |target| {
        synthesize(target, reduce, atol)
    })
}

fn append_with(
    output: &mut Circuit,
    u: &Matrix,
    pending: &mut Option<Diagonal>,
    atol: f64,
    mut synth: impl FnMut(&Matrix) -> Result<Leaf, CompilerError>,
) -> Result<(), CompilerError> {
    if let Some(d) = pending.as_ref() {
        // The pending diagonal executes first; scale columns to form U D.
        match synth(&(u * d)) {
            Ok((c, next)) => {
                output.compose(&c, None)?;
                *pending = next;
                return Ok(());
            }
            Err(CompilerError::TransformFailed { .. }) => {}
            Err(error) => return Err(error),
        }
    }
    let (c, next) = match synth(u) {
        Ok(leaf) => leaf,
        Err(CompilerError::TransformFailed { .. }) => (euler_fallback(u, atol)?, None),
        Err(error) => return Err(error),
    };
    if let Some(d) = pending.as_ref() {
        append_diagonal(output, d)?;
    }
    output.compose(&c, None)?;
    *pending = next;
    Ok(())
}

fn euler_fallback(u: &Matrix, atol: f64) -> Result<Circuit, CompilerError> {
    // A nearly degenerate KAK can exhaust its CX templates. One more
    // Block-ZXZ layer ends at Euler leaves and is cheap to verify at 4×4.
    let options = BlockZxzDecomposeOptions {
        opt_a1: false,
        opt_a2: false,
        opt_a2_force: false,
    };
    let mut fallback = Synthesis::new(4, atol, options);
    fallback.append_split(u, true)?;
    let error = residual(&fallback.output.to_matrix(None)?, u);
    if error > atol {
        return Err(failed(format!(
            "two-qubit Euler fallback: residual {error:e} exceeds {atol:e}"
        )));
    }
    Ok(fallback.output)
}

#[cfg(test)]
mod tests {
    use super::super::synthesis::synthesis_test::{assert_matrix_close, seeded_unitary};
    use super::*;

    #[test]
    fn a2_remainder_includes_the_kak_phase_exactly_once() {
        let target = seeded_unitary(4, 45) * C::from_polar(1.0, 0.37);
        let (c, d) = synthesize(&target, true, 1e-10).unwrap();
        assert!(c.global_phase().evaluate(&None).unwrap().abs() > 1e-4);
        let d = d.unwrap();
        assert!(d.iter().any(|z| (z - C::ONE).norm() > 1e-3));
        let actual = Matrix::from_diag(&d).dot(&c.to_matrix(None).unwrap());
        assert_matrix_close(&actual, &target, 1e-10);
        // Negative control: the historical extra KAK phase breaks this fixture.
        let duplicate = &actual * C::from_polar(1.0, c.global_phase().evaluate(&None).unwrap());
        let old = residual(&duplicate, &target);
        assert!(old > 1e-3);
    }

    #[test]
    fn explicit_diagonal_preserves_constant_and_relative_phases() {
        for phases in [Array1::from_elem(4, 0.37), array![-0.2, 0.53, 1.7, -2.98]] {
            let d = phases.mapv(|p| C::from_polar(1.0, p));
            let mut c = Circuit::new(2);
            append_diagonal(&mut c, &d).unwrap();
            assert_matrix_close(&c.to_matrix(None).unwrap(), &Matrix::from_diag(&d), 1e-14);
        }
    }

    #[test]
    fn pending_diagonal_fallback_in_middle_and_last_leaf() {
        for fail_at in [1, 2] {
            let mut c = Circuit::new(2);
            let mut pending = None;
            let mut expected = Matrix::eye(4);
            for i in 0..3 {
                // A fixed Fourier transform with different row phases on each leaf.
                // It does not commute with a non-scalar incoming diagonal.
                let u = Matrix::from_shape_fn((4, 4), |(row, col)| {
                    let angle = std::f64::consts::FRAC_PI_2 * (row * col) as f64
                        + 0.23 * ((i + 1) * row * row) as f64;
                    C::from_polar(0.5, angle)
                });
                let mut attempts = 0;
                append_with(&mut c, &u, &mut pending, 1e-10, |target| {
                    attempts += 1;
                    if i == fail_at && attempts == 1 {
                        return Err(failed("forced folded KAK failure".into()));
                    }
                    synthesize(target, i != 2, 1e-10)
                })
                .unwrap();
                assert_eq!(attempts, if i == fail_at { 2 } else { 1 });
                expected = u.dot(&expected);
            }
            assert!(pending.is_none());
            assert_matrix_close(&c.to_matrix(None).unwrap(), &expected, 1e-10);
        }
    }

    #[test]
    fn when_both_kak_attempts_fail_the_euler_fallback_composes_with_the_remainder() {
        let diag = array![-0.3, 1.7, -2.1, 0.47].mapv(|a| C::from_polar(1., a));
        let mut pending = Some(diag.clone());
        let u = seeded_unitary(4, 89);
        let mut output = Circuit::new(2);
        let mut attempts = 0;
        append_with(&mut output, &u, &mut pending, 1e-10, |target| {
            attempts += 1;
            let expected = if attempts == 1 { &u * &diag } else { u.clone() };
            assert_matrix_close(target, &expected, 1e-15);
            Err(failed("forced KAK failure".into()))
        })
        .unwrap();
        assert_eq!(attempts, 2);
        assert!(pending.is_none());
        assert_matrix_close(&output.to_matrix(None).unwrap(), &(&u * &diag), 1e-12);
    }

    #[test]
    fn a_non_numeric_failure_preserves_the_pending_diagonal_and_output() {
        let diag = array![C::i(), C::ONE, -C::i(), -C::ONE];
        let mut pending = Some(diag.clone());
        let mut output = Circuit::new(2);
        let err = append_with(&mut output, &Matrix::eye(4), &mut pending, 1e-10, |_| {
            Err(CompilerError::InvariantViolation("broken template".into()))
        })
        .unwrap_err();
        assert!(matches!(err, CompilerError::InvariantViolation(_)));
        assert_eq!(pending.unwrap(), diag);
        assert!(output.operations().is_empty());
    }

    #[test]
    fn smb_criterion_skips_leaves_already_within_two_cx() {
        let phase = C::from_polar(1.0, 0.41);
        // 0-CX product gate (I ⊗ I)
        let id = Matrix::eye(4) * phase;
        assert!(reducing_diagonal(&id).is_none());
        let (c_id, rem_id) = synthesize(&id, true, 1e-10).unwrap();
        assert!(rem_id.is_none());
        assert_eq!(
            c_id.operations()
                .iter()
                .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                .count(),
            0
        );

        // 1-CX gate (CX)
        let cx = ndarray::array![
            [C::ONE, C::ZERO, C::ZERO, C::ZERO],
            [C::ZERO, C::ONE, C::ZERO, C::ZERO],
            [C::ZERO, C::ZERO, C::ZERO, C::ONE],
            [C::ZERO, C::ZERO, C::ONE, C::ZERO],
        ] * phase;
        assert!(reducing_diagonal(&cx).is_none());
        let (c_cx, rem_cx) = synthesize(&cx, true, 1e-10).unwrap();
        assert!(rem_cx.is_none());
        assert_eq!(
            c_cx.operations()
                .iter()
                .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                .count(),
            1
        );

        // 2-CX gate (iSWAP)
        let iswap = ndarray::array![
            [C::ONE, C::ZERO, C::ZERO, C::ZERO],
            [C::ZERO, C::ZERO, C::i(), C::ZERO],
            [C::ZERO, C::i(), C::ZERO, C::ZERO],
            [C::ZERO, C::ZERO, C::ZERO, C::ONE],
        ] * phase;
        assert!(reducing_diagonal(&iswap).is_none());
        let (c_iswap, rem_iswap) = synthesize(&iswap, true, 1e-10).unwrap();
        assert!(rem_iswap.is_none());
        assert_eq!(
            c_iswap
                .operations()
                .iter()
                .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                .count(),
            2
        );

        // 3-CX gates (SWAP and generic random) still emit a reducing diagonal and 2 CX.
        let swap = ndarray::array![
            [C::ONE, C::ZERO, C::ZERO, C::ZERO],
            [C::ZERO, C::ZERO, C::ONE, C::ZERO],
            [C::ZERO, C::ONE, C::ZERO, C::ZERO],
            [C::ZERO, C::ZERO, C::ZERO, C::ONE],
        ] * phase;
        for target in [swap, seeded_unitary(4, 46) * phase] {
            assert!(reducing_diagonal(&target).is_some());
            let (c, rem) = synthesize(&target, true, 1e-10).unwrap();
            let d = rem.unwrap();
            assert_eq!(
                c.operations()
                    .iter()
                    .filter(|op| op.instruction.standard_gate() == Some(StandardGate::CX))
                    .count(),
                2
            );
            assert_matrix_close(
                &Matrix::from_diag(&d).dot(&c.to_matrix(None).unwrap()),
                &target,
                1e-10,
            );
        }
    }
}
