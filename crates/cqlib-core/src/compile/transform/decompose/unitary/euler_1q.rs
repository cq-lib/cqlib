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

//! Parameter-aware numeric one-qubit Euler synthesis.
//!
//! Given one numeric `U(theta, phi, lambda)` decomposition, this module emits
//! candidate gate sequences for the half-rotation target families. Every
//! template below is derived from Cqlib's own gate matrices:
//!
//! ```text
//! RZ(a) = diag(exp(-ia/2), exp(ia/2))
//! X2P = RX(pi/2), X2M = RX(-pi/2)   (exactly, no extra phase)
//! U(t, p, l) = exp(i(p+l)/2) * RZ(p) * RY(t) * RZ(l)
//!            = exp(i(p+l)/2) * RZ(p+pi/2) * RX(t) * RZ(l-pi/2)
//! ```
//!
//! The generator is a pure function: it does not build circuit operations and
//! never calls the target-basis lowerer or its cost model, so callers can use
//! it from inside lowering without creating a dependency cycle.

use std::f64::consts::{FRAC_PI_2, PI};

use smallvec::SmallVec;

use crate::circuit::StandardGate;
use crate::compile::{CompilerError, PARAMETER_EQ_TOLERANCE};

use super::unitary_1q::OneQubitUnitaryDecomposition;

const TWO_PI: f64 = 2.0 * PI;

/// Tolerance for the degenerate-angle branches and near-zero RZ elision.
const EULER_ANGLE_EPS: f64 = PARAMETER_EQ_TOLERANCE;

/// Half-rotation gate families a target basis can support.
///
/// Declaration order is the deterministic tie-break order between equally
/// costly candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Euler1qFamily {
    /// `{RZ, X2P}`: single-direction half rotation.
    Zsx,
    /// `{RZ, X2P, X}`: `Zsx` plus a native Pauli X.
    Zsxx,
    /// `{RZ, X2P, X2M}`: bidirectional half rotations.
    Zxpm,
    /// `{RZ, X2P, X2M, X}`: `Zxpm` plus a native Pauli X.
    Zxpmx,
}

/// One synthesized gate: `RZ(param)` when `param` is set, otherwise a fixed
/// half rotation or Pauli gate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Euler1qGate {
    pub gate: StandardGate,
    pub param: Option<f64>,
}

/// One complete synthesis candidate for one family.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Euler1qCandidate {
    /// Gates in application order (first element is applied first).
    pub gates: SmallVec<[Euler1qGate; 5]>,
    /// Scalar phase multiplying the gate sequence, so that
    /// `exp(i * global_phase) * gates == exp(i * decomposition.global_phase) * U`.
    pub global_phase: f64,
    pub family: Euler1qFamily,
}

impl Euler1qCandidate {
    /// Physical output cost as `(total_ops, parameterized_ops)`. The sequence
    /// is single-qubit only, so its depth equals `total_ops`.
    pub(crate) fn physical_cost(&self) -> (usize, usize) {
        let parameterized_ops = self
            .gates
            .iter()
            .filter(|gate| gate.param.is_some())
            .count();
        (self.gates.len(), parameterized_ops)
    }
}

impl Euler1qFamily {
    fn required_gates(self) -> &'static [StandardGate] {
        match self {
            Self::Zsx => &[StandardGate::RZ, StandardGate::X2P],
            Self::Zsxx => &[StandardGate::RZ, StandardGate::X2P, StandardGate::X],
            Self::Zxpm => &[StandardGate::RZ, StandardGate::X2P, StandardGate::X2M],
            Self::Zxpmx => &[
                StandardGate::RZ,
                StandardGate::X2P,
                StandardGate::X2M,
                StandardGate::X,
            ],
        }
    }

    fn is_supported_by(self, is_available: &dyn Fn(StandardGate) -> bool) -> bool {
        self.required_gates().iter().all(|gate| is_available(*gate))
    }

    const fn has_native_x(self) -> bool {
        matches!(self, Self::Zsxx | Self::Zxpmx)
    }

    const fn has_x2m(self) -> bool {
        matches!(self, Self::Zxpm | Self::Zxpmx)
    }
}

/// Appends `RZ(angle)` normalized to `(-pi, pi]`, or nothing when the
/// normalized angle is numerically zero.
///
/// `RZ(a + 2*pi*k) = (-1)^k * RZ(a)`, so every removed `2*pi` contributes a
/// `pi` phase shift. Solving `angle = normalized + 2*pi*k` with
/// `normalized in (-pi, pi]` in one step makes the compensation happen exactly
/// once, including the `-pi -> +pi` boundary (`k = -1`).
fn push_normalized_rz(
    angle: f64,
    gates: &mut SmallVec<[Euler1qGate; 5]>,
    phase: &mut f64,
) -> Result<(), CompilerError> {
    if !angle.is_finite() {
        return Err(CompilerError::InvalidInput(format!(
            "euler 1q synthesis received a non-finite RZ angle: {angle}"
        )));
    }
    let k = ((angle - PI) / TWO_PI).ceil();
    let normalized = angle - TWO_PI * k;
    *phase += k * PI;
    if normalized.abs() <= EULER_ANGLE_EPS {
        return Ok(());
    }
    gates.push(Euler1qGate {
        gate: StandardGate::RZ,
        param: Some(normalized),
    });
    Ok(())
}

fn push_fixed(gates: &mut SmallVec<[Euler1qGate; 5]>, gate: StandardGate) {
    gates.push(Euler1qGate { gate, param: None });
}

/// Enumerates one synthesis candidate per family supported by the available
/// gate set, cheapest-shortlist style: degenerate angles take dedicated short
/// templates, generic angles take the family template.
///
/// `is_available` reports whether a standard gate belongs to the target set;
/// a predicate keeps this module decoupled from any caller-side gate
/// collection and avoids duplicating the caller's storage.
///
/// # Errors
///
/// Returns [`CompilerError::InvalidInput`] when any decomposition angle or
/// derived RZ angle is not finite.
pub(crate) fn synthesize_euler_1q_candidates(
    decomposition: OneQubitUnitaryDecomposition,
    is_available: &dyn Fn(StandardGate) -> bool,
) -> Result<Vec<Euler1qCandidate>, CompilerError> {
    let OneQubitUnitaryDecomposition {
        theta,
        phi,
        lambda,
        global_phase,
    } = decomposition;
    for (name, angle) in [
        ("theta", theta),
        ("phi", phi),
        ("lambda", lambda),
        ("global_phase", global_phase),
    ] {
        if !angle.is_finite() {
            return Err(CompilerError::InvalidInput(format!(
                "euler 1q synthesis received a non-finite {name}: {angle}"
            )));
        }
    }

    let mut candidates = Vec::new();
    for family in [
        Euler1qFamily::Zsx,
        Euler1qFamily::Zsxx,
        Euler1qFamily::Zxpm,
        Euler1qFamily::Zxpmx,
    ] {
        if !family.is_supported_by(is_available) {
            continue;
        }
        let mut gates = SmallVec::new();
        // Every template carries the `exp(i(phi+lambda)/2)` convention factor
        // of Cqlib's `U` plus the decomposition's own scalar phase.
        let mut phase = global_phase + (phi + lambda) / 2.0;

        if theta.abs() <= EULER_ANGLE_EPS {
            // U(0, p, l) = exp(i(p+l)/2) * RZ(p+l)
            push_normalized_rz(phi + lambda, &mut gates, &mut phase)?;
        } else if (theta - FRAC_PI_2).abs() <= EULER_ANGLE_EPS {
            // U(pi/2, p, l) = exp(i(p+l)/2) * RZ(p+pi/2) * X2P * RZ(l-pi/2)
            push_normalized_rz(lambda - FRAC_PI_2, &mut gates, &mut phase)?;
            push_fixed(&mut gates, StandardGate::X2P);
            push_normalized_rz(phi + FRAC_PI_2, &mut gates, &mut phase)?;
        } else if (theta - PI).abs() <= EULER_ANGLE_EPS {
            if family.has_native_x() {
                // RX(pi) = -iX and X * RZ(a) = RZ(-a) * X, so
                // U(pi, p, l) = exp(i(p+l)/2 - i*pi/2) * RZ(p-l+pi) * X
                phase -= FRAC_PI_2;
                push_fixed(&mut gates, StandardGate::X);
                push_normalized_rz(phi - lambda + PI, &mut gates, &mut phase)?;
            } else {
                // X2P * X2P = RX(pi) exactly, and
                // X2P * X2P * RZ(a) = RZ(-a) * X2P * X2P, so
                // U(pi, p, l) = exp(i(p+l)/2) * RZ(p-l+pi) * X2P * X2P
                push_fixed(&mut gates, StandardGate::X2P);
                push_fixed(&mut gates, StandardGate::X2P);
                push_normalized_rz(phi - lambda + PI, &mut gates, &mut phase)?;
            }
        } else if family.has_x2m() {
            // U = exp(i(p+l)/2) * RZ(p) * X2M * RZ(t) * X2P * RZ(l)
            push_normalized_rz(lambda, &mut gates, &mut phase)?;
            push_fixed(&mut gates, StandardGate::X2P);
            push_normalized_rz(theta, &mut gates, &mut phase)?;
            push_fixed(&mut gates, StandardGate::X2M);
            push_normalized_rz(phi, &mut gates, &mut phase)?;
        } else {
            // X2M = RZ(pi) * X2P * RZ(-pi) (exact), substituted above:
            // U = exp(i(p+l)/2) * RZ(p+pi) * X2P * RZ(t-pi) * X2P * RZ(l)
            push_normalized_rz(lambda, &mut gates, &mut phase)?;
            push_fixed(&mut gates, StandardGate::X2P);
            push_normalized_rz(theta - PI, &mut gates, &mut phase)?;
            push_fixed(&mut gates, StandardGate::X2P);
            push_normalized_rz(phi + PI, &mut gates, &mut phase)?;
        }

        debug_assert!(
            gates.iter().all(|gate| is_available(gate.gate)),
            "euler 1q synthesis emitted a gate outside the available set"
        );
        candidates.push(Euler1qCandidate {
            gates,
            global_phase: phase,
            family,
        });
    }
    Ok(candidates)
}

#[cfg(test)]
#[path = "./euler_1q_test.rs"]
mod euler_1q_test;
