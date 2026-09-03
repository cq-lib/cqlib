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

use std::collections::HashSet;
use std::f64::consts::{FRAC_PI_2, PI};

use ndarray::Array2;
use num_complex::Complex64;

use super::{
    EULER_ANGLE_EPS, Euler1qCandidate, Euler1qFamily, Euler1qGate, push_normalized_rz,
    synthesize_euler_1q_candidates,
};
use crate::circuit::StandardGate;
use crate::compile::transform::decompose::unitary::unitary_1q::OneQubitUnitaryDecomposition;

const MATRIX_EQ_EPS: f64 = 1e-12;

fn decomposition(theta: f64, phi: f64, lambda: f64) -> OneQubitUnitaryDecomposition {
    OneQubitUnitaryDecomposition {
        theta,
        phi,
        lambda,
        global_phase: 0.17,
    }
}

fn available(gates: &[StandardGate]) -> impl Fn(StandardGate) -> bool + '_ {
    let set: HashSet<StandardGate> = gates.iter().copied().collect();
    move |gate| set.contains(&gate)
}

fn candidate_matrix(candidate: &Euler1qCandidate) -> Array2<Complex64> {
    let mut matrix = Array2::eye(2);
    for gate in &candidate.gates {
        let params: [f64; 1] = [gate.param.unwrap_or(0.0)];
        let gate_matrix = gate
            .gate
            .matrix(&params[..gate.param.is_some() as usize])
            .unwrap();
        matrix = gate_matrix.dot(&matrix);
    }
    matrix * Complex64::new(0.0, candidate.global_phase).exp()
}

fn expected_matrix(decomposition: &OneQubitUnitaryDecomposition) -> Array2<Complex64> {
    let u = StandardGate::U
        .matrix(&[decomposition.theta, decomposition.phi, decomposition.lambda])
        .unwrap();
    u.into_owned() * Complex64::new(0.0, decomposition.global_phase).exp()
}

fn assert_candidate_equivalent(
    candidate: &Euler1qCandidate,
    decomposition: &OneQubitUnitaryDecomposition,
) {
    let actual = candidate_matrix(candidate);
    let expected = expected_matrix(decomposition);
    for index in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let diff = (actual[index] - expected[index]).norm();
        assert!(
            diff <= MATRIX_EQ_EPS,
            "{:?} candidate differs at {index:?} by {diff}: {candidate:?}",
            candidate.family
        );
    }
}

fn assert_all_candidates_equivalent(theta: f64, phi: f64, lambda: f64) {
    let decomposition = decomposition(theta, phi, lambda);
    let candidates = synthesize_euler_1q_candidates(
        decomposition,
        &available(&[
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::X2M,
            StandardGate::X,
        ]),
    )
    .unwrap();
    assert_eq!(candidates.len(), 4);
    for candidate in &candidates {
        assert_candidate_equivalent(candidate, &decomposition);
    }
}

#[test]
fn normalized_rz_handles_zero_and_two_pi_multiples() {
    for (angle, expected_k) in [
        (0.0, 0.0),
        (2.0 * PI, 1.0),
        (4.0 * PI, 2.0),
        (-2.0 * PI, -1.0),
    ] {
        let mut gates = Default::default();
        let mut phase = 1.25;
        push_normalized_rz(angle, &mut gates, &mut phase).unwrap();
        assert!(gates.is_empty(), "angle {angle} should elide to identity");
        assert!(
            ((phase - 1.25) - expected_k * PI).abs() <= EULER_ANGLE_EPS,
            "angle {angle}: phase {} misses {expected_k}*pi",
            phase - 1.25
        );
    }
}

#[test]
fn normalized_rz_maps_minus_pi_to_plus_pi_with_phase() {
    let mut gates = Default::default();
    let mut phase = 0.0;
    push_normalized_rz(-PI, &mut gates, &mut phase).unwrap();
    assert_eq!(
        gates.as_slice(),
        &[Euler1qGate {
            gate: StandardGate::RZ,
            param: Some(PI)
        }]
    );
    assert!((phase + PI).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn normalized_rz_keeps_boundaries_and_residues() {
    let cases: [(f64, f64, f64); 4] = [
        (PI, PI, 0.0),
        (3.0 * PI, PI, PI),
        (2.0 * PI + 5e-9, 5e-9, PI),
        (-3.0 * PI, PI, -2.0 * PI),
    ];
    for (angle, expected_angle, expected_phase) in cases {
        let mut gates = Default::default();
        let mut phase = 0.0;
        push_normalized_rz(angle, &mut gates, &mut phase).unwrap();
        assert_eq!(gates.len(), 1, "angle {angle} should keep one RZ");
        let actual = gates[0].param.unwrap();
        assert!(
            (actual - expected_angle).abs() <= EULER_ANGLE_EPS,
            "angle {angle}: normalized {actual} != {expected_angle}"
        );
        assert!(
            (phase - expected_phase).abs() <= EULER_ANGLE_EPS,
            "angle {angle}: phase {phase} != {expected_phase}"
        );
    }
}

#[test]
fn normalized_rz_rejects_non_finite_angles() {
    for angle in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut gates = Default::default();
        let mut phase = 0.0;
        assert!(push_normalized_rz(angle, &mut gates, &mut phase).is_err());
    }
}

#[test]
fn theta_zero_collapses_to_single_rz() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(0.0, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P]),
    )
    .unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.family, Euler1qFamily::Zsx);
    assert_eq!(candidate.gates.len(), 1);
    assert_eq!(candidate.gates[0].gate, StandardGate::RZ);
    assert!((candidate.gates[0].param.unwrap() - 1.0).abs() <= EULER_ANGLE_EPS);
    assert!((candidate.global_phase - (0.17 + 0.5)).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn theta_half_pi_uses_short_x2p_form() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(FRAC_PI_2, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P]),
    )
    .unwrap();
    let candidate = &candidates[0];
    assert_eq!(candidate.gates.len(), 3);
    assert_eq!(candidate.gates[0].gate, StandardGate::RZ);
    assert!((candidate.gates[0].param.unwrap() - (0.7 - FRAC_PI_2)).abs() <= EULER_ANGLE_EPS);
    assert_eq!(
        candidate.gates[1],
        Euler1qGate {
            gate: StandardGate::X2P,
            param: None
        }
    );
    assert_eq!(candidate.gates[2].gate, StandardGate::RZ);
    assert!((candidate.gates[2].param.unwrap() - (0.3 + FRAC_PI_2)).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn theta_pi_uses_native_x_when_available() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(PI, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P, StandardGate::X]),
    )
    .unwrap();
    let best = candidates
        .iter()
        .min_by_key(|candidate| candidate.gates.len())
        .unwrap();
    assert_eq!(best.family, Euler1qFamily::Zsxx);
    assert!(best.gates.len() <= 2);
    assert_eq!(
        best.gates[0],
        Euler1qGate {
            gate: StandardGate::X,
            param: None
        }
    );
    assert!((best.global_phase - (0.17 + 0.5 - FRAC_PI_2)).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn theta_pi_without_x_uses_two_x2p() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(PI, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P]),
    )
    .unwrap();
    let candidate = &candidates[0];
    assert_eq!(candidate.family, Euler1qFamily::Zsx);
    assert!(candidate.gates.len() <= 3);
    assert_eq!(candidate.gates[0].gate, StandardGate::X2P);
    assert_eq!(candidate.gates[1].gate, StandardGate::X2P);
    assert_eq!(candidate.gates[2].gate, StandardGate::RZ);
}

#[test]
fn generic_theta_zsx_uses_two_x2p_template() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(0.4, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P]),
    )
    .unwrap();
    let candidate = &candidates[0];
    assert_eq!(
        candidate
            .gates
            .iter()
            .map(|gate| gate.gate)
            .collect::<Vec<_>>(),
        vec![
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::RZ
        ]
    );
    assert!((candidate.gates[0].param.unwrap() - 0.7).abs() <= EULER_ANGLE_EPS);
    assert!((candidate.gates[2].param.unwrap() - (0.4 - PI)).abs() <= EULER_ANGLE_EPS);
    // RZ(0.3 + pi) normalizes to RZ(0.3 - pi) with a pi phase shift.
    assert!((candidate.gates[4].param.unwrap() - (0.3 - PI)).abs() <= EULER_ANGLE_EPS);
    assert!((candidate.global_phase - (0.17 + 0.5 + PI)).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn generic_theta_zxpm_uses_bidirectional_template() {
    let candidates = synthesize_euler_1q_candidates(
        decomposition(0.4, 0.3, 0.7),
        &available(&[StandardGate::RZ, StandardGate::X2P, StandardGate::X2M]),
    )
    .unwrap();
    let zxpm = candidates
        .iter()
        .find(|candidate| candidate.family == Euler1qFamily::Zxpm)
        .unwrap();
    assert_eq!(
        zxpm.gates.iter().map(|gate| gate.gate).collect::<Vec<_>>(),
        vec![
            StandardGate::RZ,
            StandardGate::X2P,
            StandardGate::RZ,
            StandardGate::X2M,
            StandardGate::RZ
        ]
    );
    assert!((zxpm.gates[0].param.unwrap() - 0.7).abs() <= EULER_ANGLE_EPS);
    assert!((zxpm.gates[2].param.unwrap() - 0.4).abs() <= EULER_ANGLE_EPS);
    assert!((zxpm.gates[4].param.unwrap() - 0.3).abs() <= EULER_ANGLE_EPS);
    assert!((zxpm.global_phase - (0.17 + 0.5)).abs() <= EULER_ANGLE_EPS);
}

#[test]
fn candidates_match_source_unitary_including_global_phase() {
    for (theta, phi, lambda) in [
        (0.0, 0.3, 0.7),
        (FRAC_PI_2, 0.3, 0.7),
        (PI, 0.3, 0.7),
        (0.4, 0.3, 0.7),
        (2.7, -1.1, 2.9),
        (1.0, -2.9, -0.4),
        (0.0, -0.8, 0.8),
        (FRAC_PI_2, 1.9, -2.2),
    ] {
        assert_all_candidates_equivalent(theta, phi, lambda);
    }
}

#[test]
fn angles_near_special_values_do_not_short_circuit() {
    for special in [0.0, FRAC_PI_2, PI] {
        let theta = special + 5e-9;
        let candidates = synthesize_euler_1q_candidates(
            decomposition(theta, 0.3, 0.7),
            &available(&[StandardGate::RZ, StandardGate::X2P]),
        )
        .unwrap();
        assert_eq!(
            candidates[0].gates.len(),
            5,
            "theta={theta} must not take the degenerate branch"
        );
        assert_all_candidates_equivalent(theta, 0.3, 0.7);
    }
}

#[test]
fn family_selection_follows_available_gates() {
    let decomposition = decomposition(0.4, 0.3, 0.7);
    let families_of = |gates: &[StandardGate]| {
        synthesize_euler_1q_candidates(decomposition, &available(gates))
            .unwrap()
            .into_iter()
            .map(|candidate| candidate.family)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        families_of(&[StandardGate::RZ, StandardGate::X2P]),
        vec![Euler1qFamily::Zsx]
    );
    assert_eq!(
        families_of(&[StandardGate::RZ, StandardGate::X2P, StandardGate::X]),
        vec![Euler1qFamily::Zsx, Euler1qFamily::Zsxx]
    );
    assert_eq!(
        families_of(&[StandardGate::RZ, StandardGate::X2P, StandardGate::X2M]),
        vec![Euler1qFamily::Zsx, Euler1qFamily::Zxpm]
    );
    assert!(
        families_of(&[StandardGate::RZ, StandardGate::X2M]).is_empty(),
        "no complete family means an empty candidate list"
    );
}

#[test]
fn synthesis_is_deterministic() {
    let decomposition = decomposition(0.4, 0.3, 0.7);
    let gates = available(&[
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::X,
    ]);
    let first = synthesize_euler_1q_candidates(decomposition, &gates).unwrap();
    let second = synthesize_euler_1q_candidates(decomposition, &gates).unwrap();
    assert_eq!(first, second);
}

struct Xorshift64(u64);

impl Xorshift64 {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.next_f64()
    }
}

#[test]
fn random_unitaries_match_source_matrix_including_global_phase() {
    let mut rng = Xorshift64(0x9E37_79B9_7F4A_7C15);
    let gates = available(&[
        StandardGate::RZ,
        StandardGate::X2P,
        StandardGate::X2M,
        StandardGate::X,
    ]);
    for sample in 0..200 {
        let decomposition = OneQubitUnitaryDecomposition {
            theta: rng.range(0.0, PI),
            phi: rng.range(-4.0 * PI, 4.0 * PI),
            lambda: rng.range(-4.0 * PI, 4.0 * PI),
            global_phase: rng.range(-PI, PI),
        };
        let candidates = synthesize_euler_1q_candidates(decomposition, &gates).unwrap();
        assert_eq!(candidates.len(), 4, "sample {sample}");
        for candidate in &candidates {
            assert_candidate_equivalent(candidate, &decomposition);
        }
    }
}

#[test]
fn non_finite_decomposition_is_rejected() {
    for theta in [f64::NAN, f64::INFINITY] {
        assert!(
            synthesize_euler_1q_candidates(
                decomposition(theta, 0.3, 0.7),
                &available(&[StandardGate::RZ, StandardGate::X2P]),
            )
            .is_err()
        );
    }
}
