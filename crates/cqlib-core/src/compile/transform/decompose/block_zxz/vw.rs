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

//! A.2 optimization: two-qubit 2-CNOT + diagonal-remainder synthesis
//! (paper Sec. 5.1).
//!
//! The paper (arXiv:2403.13692 Sec. 5.1) cites the Shende-Markov-Bullock
//! "up to diagonal" technique (PRL version: quant-ph/0406176; membership
//! criterion: quant-ph/0308045, Prop. III.3): for any two-qubit unitary `v`
//! there is a (computational-basis) diagonal `d` such that both `d @ v` and
//! `v @ d` can be implemented with 2 CNOTs; the diagonal remainder commutes
//! through the surrounding gates and merges into the next two-qubit gate.
//!
//! Membership criterion (SMB Prop. III.3): `u in SU(4)` admits a 2-CNOT
//! circuit iff `tr[gamma(u)]` is real, where `gamma(u) = u @ S @ u^T @ S`
//! with `S = sigma_y (x) sigma_y`.
//!
//! Explicit closed-form construction (derived here): for a 4x4 unitary `U`
//! let `M = U @ S @ U^T` (complex symmetric), `A = M[1][2]` and
//! `B = M[0][3]` (the paired entries coincide by symmetry), `alpha = arg A`,
//! `beta = arg B`, `delta = arg det U` and `c0 = alpha + beta - delta`. Then
//! ```text
//! u* = atan2(|B| sin c0, |A| + |B| cos c0),   v* = c0 - u*,
//! D  = diag(1, 1, e^{i(u*-alpha)}, e^{i(v*-beta)}),
//! ```
//! so the SU(4) normalization of `T = D @ U` satisfies `tr[gamma(T)]` real
//! (key identity: `tr[gamma(D @ U)] = 2 (d1 d2 m12 - d0 d3 m03)`; the
//! normalizing phase contributes exactly the `delta` argument to the
//! balancing equation, which is a two-phasor sum and therefore always
//! solvable), hence `T` lies in the 2-CNOT class.
//!
//! Synthesis contract (see `engine::run_a2_chain`):
//! `D_out @ circuit == target` up to global phase, where `circuit`
//! implements `T = D @ target` and `D_out = e^{i phi_T} @ D^-1` - a pure
//! diagonal phase matrix acting on qubits `(q1, q0)`.
//!
//! Migration legality: within the assembled Block-ZXZ/A.1 circuit the
//! non-leaf gates are only `Rz(top)`, `CX(ctrl < top -> top)` and `H(top)`.
//! The diagonal remainder's phase depends only on the `(q1, q0)` values and
//! therefore commutes with every such gate, so it migrates to the next
//! two-qubit leaf unchanged (entering its synthesis target as
//! `U_next @ D_out`), where it is absorbed by that leaf's own synthesis. The
//! temporally last leaf uses the full 0-3 CNOT synthesis to absorb the whole
//! accumulated remainder.
//!
//! Absorption can be numerically infeasible: a remainder-folded target often
//! sits on a Weyl-chamber boundary, where the shared two-qubit backend's KAK
//! canonicalization is accurate to only about `1e-10` and its exactness gate
//! rejects every candidate. [`diagonal_gate_circuit`] provides the exact
//! escape hatch used by `engine::run_a2_chain` in that case.

use crate::circuit::{Circuit, Parameter, Qubit};
use crate::compile::transform::decompose::unitary::{
    TwoQubitUnitaryDecomposeBasis, synthesize_numeric_2q_unitary,
};
use ndarray::Array2;
use num_complex::Complex64;

use super::matrix::{Cm, adjoint, eye, mat_mul, to_faer};

/// `S = sigma_y (x) sigma_y` in basis order `(q1, q0)` (the tensor's first
/// factor is the higher qubit q1).
///
/// From `sigma_y|0> = i|1>` and `sigma_y|1> = -i|0>`:
/// `S|00> = -|11>`, `S|01> = |10>`, `S|10> = |01>`, `S|11> = -|00>`,
/// so `S = [[0,0,0,-1],[0,0,1,0],[0,1,0,0],[-1,0,0,0]]` (real, symmetric,
/// involutory, unitary).
fn syy_matrix() -> Cm {
    let mut s = Array2::<Complex64>::zeros((4, 4));
    let one = Complex64::new(1.0, 0.0);
    s[[0, 3]] = -one;
    s[[1, 2]] = one;
    s[[2, 1]] = one;
    s[[3, 0]] = -one;
    s
}

/// Builds the left-multiplying diagonal phase `D` that puts `T = D @ U`
/// into the 2-CNOT class (the explicit solution of SMB Prop. III.3; see
/// the module documentation for the derivation).
pub fn two_cnot_phase_diagonal(u: &Cm) -> Cm {
    let s = syy_matrix();
    let u_t = u.t().to_owned(); // plain transpose (no conjugation)
    let m = mat_mul(&mat_mul(u, &s), &u_t); // M = U S U^T, complex symmetric

    let a = m[[1, 2]]; // by symmetry m[1][2] == m[2][1]
    let b = m[[0, 3]]; // by symmetry m[0][3] == m[3][0]
    let alpha = a.arg();
    let beta = b.arg();
    let r1 = a.norm();
    let r0 = b.norm();

    // delta = arg det U.
    let fm = to_faer(u);
    let det = fm.determinant();
    let delta = Complex64::new(det.re, det.im).arg();

    // Balancing equation (always solvable):
    // (r1 + r0 cos c0) sin u* = r0 sin(c0) cos u*.
    let c0 = alpha + beta - delta;
    let u_star = (r0 * c0.sin()).atan2(r1 + r0 * c0.cos());
    let v_star = c0 - u_star;

    // Simplest assignment of the constraint d1 d2 = e^{i(u* - alpha + ...)}:
    // d0 = d1 = 1.
    let phi_p = u_star - alpha;
    let phi_q = v_star - beta;
    let mut d = eye(4);
    d[[2, 2]] = Complex64::from_polar(1.0, phi_p);
    d[[3, 3]] = Complex64::from_polar(1.0, phi_q);
    d
}

/// Result of the optimized synthesis of one two-qubit leaf in A.2.
pub struct TwoCnotSynthesis {
    /// 2-CNOT (or fewer) circuit that implements `target` together with
    /// [`diagonal_out`](Self::diagonal_out).
    pub circuit: Circuit,
    /// Diagonal remainder `D_out = e^{i phi_T} D^-1` acting on qubits
    /// `(q1, q0)` on the temporally later side (left end of the matrix
    /// product), satisfying `D_out @ circuit == target` up to global phase.
    pub diagonal_out: Cm,
    /// Number of CX gates actually used by this synthesis.
    pub cx_count: usize,
}

/// Decomposes `target` into a 2-CNOT circuit plus a pure diagonal remainder.
///
/// Synthesis contract: `D_out @ circuit == target` up to global phase. The
/// remainder is a unit-modulus diagonal matrix; if `target` already lies in
/// the 2-CNOT class (e.g. local or controlled structure), or if the
/// closed-form construction unexpectedly fails numerically, the full
/// synthesis is returned with an identity remainder.
pub fn synth_two_cnot_with_diagonal(target: &Cm) -> Result<TwoCnotSynthesis, String> {
    // Baseline synthesis (3 CNOTs or fewer) - always a correct fallback.
    let baseline = synthesize_for_matrix(target)?;
    let baseline_cx = count_cx(&baseline.0);
    if baseline_cx <= 2 {
        // The target itself is already in the 2-CNOT class.
        return Ok(TwoCnotSynthesis {
            circuit: baseline.0,
            diagonal_out: eye(4),
            cx_count: baseline_cx,
        });
    }

    // SMB construction: T = D @ target lies in the 2-CNOT class and the
    // remainder D_out = e^{i phi_T} D^-1 is purely diagonal.
    let d = two_cnot_phase_diagonal(target);
    let t = mat_mul(&d, target);
    let (circuit, global_phase) = synthesize_for_matrix(&t)?;
    let cx = count_cx(&circuit);
    if cx <= 2 {
        let phase = Complex64::from_polar(1.0, global_phase);
        let mut d_out = adjoint(&d); // D diagonal unitary => D^-1 = D^H
        d_out.mapv_inplace(|v| v * phase);
        // D_out @ circuit == e^{i phi_T} D^-1 T == D^-1 D target == target.
        return Ok(TwoCnotSynthesis {
            circuit,
            diagonal_out: d_out,
            cx_count: cx,
        });
    }

    // Fallback: plain 3-CNOT synthesis (the chain breaks here but stays
    // numerically correct; should not trigger under the closed form).
    Ok(TwoCnotSynthesis {
        circuit: baseline.0,
        diagonal_out: eye(4),
        cx_count: baseline_cx,
    })
}

/// Emits a two-qubit diagonal unitary as standard gates.
///
/// For `D = diag(d00, d01, d10, d11)` in basis order `(q1, q0)` (index
/// `2 q1 + q0`) with unit-modulus entries and phases `phi_ij`, the phase
/// function of a two-bit diagonal splits into a constant, two local parts
/// and one interaction part, so
/// ```text
/// D = e^{i phi00} * P(alpha; q1) * P(beta; q0) * CP(gamma; q1 -> q0)
/// ```
/// with `alpha = phi10 - phi00`, `beta = phi01 - phi00` and
/// `gamma = phi11 - phi10 - phi01 + phi00` (the interaction). The
/// controlled phase itself is emitted CX-natively,
/// `CP(gamma) = P(gamma/2; q1) * P(gamma/2; q0) * CX * P(-gamma/2; q0) * CX`
/// so the gate costs exactly two CX and the reported CX count stays
/// truthful; the two `gamma/2` local parts are folded into the `alpha` and
/// `beta` phases of the construction.
///
/// This is the exact remainder gate of the A.2 fallback: when folding the
/// accumulated diagonal remainder into a leaf's synthesis target is not
/// numerically feasible, the leaf is synthesized directly and the remainder
/// is emitted here instead. The overall phase `e^{i phi00}` is dropped, as
/// only relative phases are observable.
pub fn diagonal_gate_circuit(d: &Cm) -> Result<Circuit, String> {
    if d.dim() != (4, 4) {
        return Err("diagonal gate synthesis requires a 4x4 matrix".into());
    }
    let mut phases = [0.0f64; 4];
    for i in 0..4 {
        for j in 0..4 {
            if i != j && d[[i, j]].norm() > 1e-12 {
                return Err("diagonal gate synthesis requires a diagonal matrix".into());
            }
        }
        let value = d[[i, i]];
        if !value.re.is_finite() || !value.im.is_finite() {
            return Err("diagonal gate synthesis requires finite entries".into());
        }
        if (value.norm() - 1.0).abs() > 1e-9 {
            return Err("diagonal gate synthesis requires unit-modulus entries".into());
        }
        phases[i] = value.arg();
    }

    let alpha = phases[2] - phases[0];
    let beta = phases[1] - phases[0];
    let gamma = phases[3] - phases[2] - phases[1] + phases[0];

    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    // CX * Rz(-gamma/2; q0) * CX == diag(1, e^{-i gamma/2}, e^{-i gamma/2}, 1)
    // up to a global phase (`Rz` is the engine's emitted gate; its own global
    // phase is irrelevant here).
    circuit.cx(q1, q0).map_err(|e| e.to_string())?;
    circuit.rz(q0, -gamma / 2.0).map_err(|e| e.to_string())?;
    circuit.cx(q1, q0).map_err(|e| e.to_string())?;
    circuit
        .rz(q0, beta + gamma / 2.0)
        .map_err(|e| e.to_string())?;
    circuit
        .rz(q1, alpha + gamma / 2.0)
        .map_err(|e| e.to_string())?;
    Ok(circuit)
}

/// Counts `CX` gates in a circuit, including nested composite gates.
pub fn count_cx(circuit: &Circuit) -> usize {
    use crate::circuit::gate::Instruction;
    let mut total = 0;
    for op in circuit.operations() {
        match &op.instruction {
            Instruction::Standard(gate) => {
                if gate.name() == "CX" {
                    total += 1;
                }
            }
            Instruction::CircuitGate(cg) => {
                total += count_cx(cg.circuit().as_ref().circuit());
            }
            _ => {}
        }
    }
    total
}

/// Synthesizes a 4x4 unitary on qubits `[1, 0]` in the Cx basis (consistent
/// with the whole engine); returns `(circuit, global_phase)` with
/// `e^{i phi} circuit == matrix`.
fn synthesize_for_matrix(matrix: &Cm) -> Result<(Circuit, f64), String> {
    let synthesis = synthesize_numeric_2q_unitary(
        matrix,
        [Qubit::new(1), Qubit::new(0)],
        TwoQubitUnitaryDecomposeBasis::Cx,
    )
    .map_err(|e| e.to_string())?;
    let mut circuit = Circuit::from_operations(
        vec![Qubit::new(0), Qubit::new(1)],
        synthesis.operations,
        None,
        None,
    )
    .map_err(|e| e.to_string())?;
    circuit.set_global_phase(Parameter::from(synthesis.global_phase));
    Ok((circuit, synthesis.global_phase))
}

/// Full (0-3 CX adaptive) synthesis for the last / unoptimized leaf.
pub fn synth_leaf_full(target: &Cm) -> Result<Circuit, String> {
    let (circuit, _) = synthesize_for_matrix(target)?;
    Ok(circuit)
}
