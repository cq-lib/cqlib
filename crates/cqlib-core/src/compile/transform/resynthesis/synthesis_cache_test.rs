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

use super::*;
use crate::circuit::{Circuit, Instruction, StandardGate, ValueInstruction, ValueOperation};
use crate::compile::test_utils::build_device_synthesis_context;
use crate::compile::transform::decompose::unitary::unitary_2q::{
    TargetAwareSynthesisCost, plan_numeric_2q_unitary_for_device,
};
use crate::compile::transform::decompose::unitary::{
    DeviceSynthesisPlacement, TwoQubitUnitaryDecomposeBasis,
};
use crate::device::Device;
use smallvec::smallvec;

fn matrix_with(real: f64) -> Array2<Complex64> {
    let mut matrix = Array2::eye(4);
    matrix[(0, 0)] = Complex64::new(real, 0.0);
    matrix
}

#[test]
fn exact_key_distinguishes_qargs_bits_and_signed_zero() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let plus_zero = matrix_with(0.0);
    let minus_zero = matrix_with(-0.0);
    let next_bit = matrix_with(f64::from_bits(1));
    let nan_payload_one = matrix_with(f64::from_bits(0x7ff8_0000_0000_0001));
    let nan_payload_two = matrix_with(f64::from_bits(0x7ff8_0000_0000_0002));

    let base = ExactTwoQubitSynthesisKey::new(&plus_zero, [q0, q1]).unwrap();
    assert_eq!(
        base,
        ExactTwoQubitSynthesisKey::new(&plus_zero, [q0, q1]).unwrap()
    );
    assert_ne!(
        base,
        ExactTwoQubitSynthesisKey::new(&plus_zero, [q1, q0]).unwrap()
    );
    assert_ne!(
        base,
        ExactTwoQubitSynthesisKey::new(&minus_zero, [q0, q1]).unwrap()
    );
    assert_ne!(
        base,
        ExactTwoQubitSynthesisKey::new(&next_bit, [q0, q1]).unwrap()
    );
    assert_ne!(
        ExactTwoQubitSynthesisKey::new(&nan_payload_one, [q0, q1]).unwrap(),
        ExactTwoQubitSynthesisKey::new(&nan_payload_two, [q0, q1]).unwrap()
    );
}

#[test]
fn key_rejects_non_four_by_four_matrix() {
    let error = ExactTwoQubitSynthesisKey::new(&Array2::eye(2), [Qubit::new(0), Qubit::new(1)])
        .unwrap_err();
    assert!(matches!(error, CompilerError::InvariantViolation(_)));
}

#[test]
fn failed_and_empty_generic_plans_are_cached() {
    let matrix = Array2::eye(4);
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let mut failed = TwoQubitSynthesisCache::new(2);
    let mut calls = 0;
    for _ in 0..2 {
        let is_failed = failed
            .with_generic_plan(
                &matrix,
                qubits,
                |_| {
                    calls += 1;
                    Err(CompilerError::InvariantViolation("planned failure".into()))
                },
                |plan| matches!(plan, CachedPlanView::Failed),
            )
            .unwrap();
        assert!(is_failed);
    }
    assert_eq!(calls, 1);
    assert_eq!(failed.stats().generic_misses, 1);
    assert_eq!(failed.stats().generic_hits, 1);
    assert_eq!(failed.stats().failed_plan_hits, 1);

    let mut empty = TwoQubitSynthesisCache::new(2);
    let mut calls = 0;
    for _ in 0..2 {
        let length = empty
            .with_generic_plan(
                &matrix,
                qubits,
                |_| {
                    calls += 1;
                    Ok(Vec::new())
                },
                |plan| match plan {
                    CachedPlanView::Candidates(candidates) => candidates.len(),
                    CachedPlanView::Failed => usize::MAX,
                },
            )
            .unwrap();
        assert_eq!(length, 0);
    }
    assert_eq!(calls, 1);
}

#[test]
fn successful_generic_plan_preserves_candidates_and_order() {
    let matrix = Array2::eye(4);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;
    let (first, second) = {
        let mut observe = || {
            cache
                .with_generic_plan(
                    &matrix,
                    [q0, q1],
                    |_| {
                        calls += 1;
                        Ok(vec![
                            TwoQubitSynthesisCandidate {
                                backend: TwoQubitUnitaryDecomposeBasis::Cx,
                                operations: Vec::new(),
                                global_phase: 0.25,
                                cost: TargetAwareSynthesisCost::default(),
                            },
                            TwoQubitSynthesisCandidate {
                                backend: TwoQubitUnitaryDecomposeBasis::Cz,
                                operations: Vec::new(),
                                global_phase: -0.5,
                                cost: TargetAwareSynthesisCost::default(),
                            },
                        ])
                    },
                    |plan| match plan {
                        CachedPlanView::Candidates(candidates) => candidates
                            .iter()
                            .map(|candidate| (candidate.backend, candidate.global_phase.to_bits()))
                            .collect::<Vec<_>>(),
                        CachedPlanView::Failed => Vec::new(),
                    },
                )
                .unwrap()
        };
        (observe(), observe())
    };

    assert_eq!(calls, 1);
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(|item| item.0).collect::<Vec<_>>(),
        vec![
            TwoQubitUnitaryDecomposeBasis::Cx,
            TwoQubitUnitaryDecomposeBasis::Cz
        ]
    );
}

#[test]
fn successful_generic_plan_remaps_canonical_roles_to_requested_qargs() {
    let matrix = Array2::eye(4);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;

    for qubits in [[q0, q1], [q0, q1], [q1, q0]] {
        let cached_qubits = cache
            .with_generic_plan(
                &matrix,
                qubits,
                |_| {
                    calls += 1;
                    Ok(vec![TwoQubitSynthesisCandidate {
                        backend: TwoQubitUnitaryDecomposeBasis::Cx,
                        operations: vec![ValueOperation {
                            instruction: ValueInstruction::from_instruction(Instruction::Standard(
                                StandardGate::CX,
                            )),
                            qubits: smallvec![qubits[0], qubits[1]],
                            params: Default::default(),
                            label: None,
                        }],
                        global_phase: 0.0,
                        cost: TargetAwareSynthesisCost::default(),
                    }])
                },
                |plan| match plan {
                    CachedPlanView::Candidates(candidates) => {
                        candidates[0].operations[0].qubits.clone()
                    }
                    CachedPlanView::Failed => unreachable!("successful planner"),
                },
            )
            .unwrap();
        assert_eq!(cached_qubits.as_slice(), &qubits);
    }

    assert_eq!(calls, 1);
    assert_eq!(cache.stats().generic_misses, 1);
    assert_eq!(cache.stats().generic_hits, 2);
}

#[test]
fn cache_lookup_is_bit_exact_for_matrix_values() {
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let matrices = [
        matrix_with(0.0),
        matrix_with(-0.0),
        matrix_with(f64::from_bits(1)),
        matrix_with(f64::from_bits(0x7ff8_0000_0000_0001)),
        matrix_with(f64::from_bits(0x7ff8_0000_0000_0002)),
    ];
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;

    for matrix in &matrices {
        cache
            .with_generic_plan(
                matrix,
                qubits,
                |_| {
                    calls += 1;
                    Ok(Vec::new())
                },
                |_| (),
            )
            .unwrap();
    }

    assert_eq!(calls, 5);
    assert_eq!(cache.stats().generic_misses, 5);
    assert_eq!(cache.stats().generic_hits, 0);
}

#[test]
fn device_plan_is_cached_for_exact_matrix_and_qargs() {
    let device = Device::line("resynthesis-cache", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let context =
        build_device_synthesis_context(&device, &circuit, DeviceSynthesisPlacement::ExactPhysical)
            .unwrap();
    let matrix = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;

    for _ in 0..2 {
        let count = cache
            .with_device_plan(
                &matrix,
                [q0, q1],
                context.placement(),
                |_| {
                    calls += 1;
                    plan_numeric_2q_unitary_for_device(&matrix, [q0, q1], &context)
                },
                |plan| match plan {
                    CachedPlanView::Candidates(candidates) => candidates.len(),
                    CachedPlanView::Failed => 0,
                },
            )
            .unwrap();
        assert!(count > 0);
    }

    let reversed_count = cache
        .with_device_plan(
            &matrix,
            [q1, q0],
            context.placement(),
            |_| {
                calls += 1;
                plan_numeric_2q_unitary_for_device(&matrix, [q1, q0], &context)
            },
            |plan| match plan {
                CachedPlanView::Candidates(candidates) => candidates.len(),
                CachedPlanView::Failed => 0,
            },
        )
        .unwrap();
    assert!(reversed_count > 0);

    assert_eq!(calls, 2);
    assert_eq!(cache.stats().device_misses, 2);
    assert_eq!(cache.stats().device_hits, 1);
    assert_eq!(cache.stats().device_entries, 2);
}

#[test]
fn pre_layout_device_plan_reuses_canonical_logical_qargs() {
    let device = Device::line("resynthesis-pre-layout-cache", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    circuit.cx(q0, q1).unwrap();
    let context = build_device_synthesis_context(
        &device,
        &circuit,
        DeviceSynthesisPlacement::PreLayoutEnvelope,
    )
    .unwrap();
    let matrix = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;

    for qargs in [[q0, q1], [q1, q2]] {
        let references_requested_qargs = cache
            .with_device_plan(
                &matrix,
                qargs,
                context.placement(),
                |_| {
                    calls += 1;
                    plan_numeric_2q_unitary_for_device(&matrix, qargs, &context)
                },
                |plan| match plan {
                    CachedPlanView::Candidates(candidates) => candidates.iter().all(|candidate| {
                        candidate
                            .candidate
                            .operations
                            .iter()
                            .flat_map(|operation| operation.qubits.iter())
                            .all(|qubit| qargs.contains(qubit))
                    }),
                    CachedPlanView::Failed => false,
                },
            )
            .unwrap();
        assert!(references_requested_qargs);
    }

    assert_eq!(calls, 1);
    assert_eq!(cache.stats().device_misses, 1);
    assert_eq!(cache.stats().device_hits, 1);
    assert_eq!(cache.stats().device_entries, 1);
}

#[test]
fn namespace_reuses_context_clones_and_invalidates_rebuilds() {
    let device = Device::line("resynthesis-cache-namespace", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.cx(q0, q1).unwrap();
    let context =
        build_device_synthesis_context(&device, &circuit, DeviceSynthesisPlacement::ExactPhysical)
            .unwrap();
    let cloned = context.clone();
    let rebuilt =
        build_device_synthesis_context(&device, &circuit, DeviceSynthesisPlacement::ExactPhysical)
            .unwrap();
    assert_eq!(context.generation(), cloned.generation());
    assert_ne!(context.generation(), rebuilt.generation());

    let config = TwoQubitBlockResynthesisConfig::normal(Default::default());
    let matrix = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    let mut cache = TwoQubitSynthesisCache::new(4);
    let mut calls = 0;

    cache.ensure_namespace(&config, Some(&context));
    cache
        .with_device_plan(
            &matrix,
            [q0, q1],
            context.placement(),
            |_| {
                calls += 1;
                Ok(Vec::new())
            },
            |_| (),
        )
        .unwrap();
    cache.ensure_namespace(&config, Some(&cloned));
    cache
        .with_device_plan(
            &matrix,
            [q0, q1],
            cloned.placement(),
            |_| {
                calls += 1;
                Ok(Vec::new())
            },
            |_| (),
        )
        .unwrap();
    assert_eq!(calls, 1);

    cache.ensure_namespace(&config, Some(&rebuilt));
    cache
        .with_device_plan(
            &matrix,
            [q0, q1],
            rebuilt.placement(),
            |_| {
                calls += 1;
                Ok(Vec::new())
            },
            |_| (),
        )
        .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(cache.stats().namespace_invalidations, 1);
}

#[test]
fn kak_cache_is_exact_and_survives_candidate_namespace_changes() {
    let matrix = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    let mut cache = TwoQubitSynthesisCache::new_native_session();
    let config = TwoQubitBlockResynthesisConfig::normal(Default::default());
    cache.ensure_namespace(&config, None);

    let first = cache.kak_decomposition(&matrix).unwrap().unwrap();
    let second = cache.kak_decomposition(&matrix).unwrap().unwrap();
    assert!(Arc::ptr_eq(&first, &second));

    let mut changed = config;
    changed.max_block_ops = changed.max_block_ops.saturating_add(1);
    cache.ensure_namespace(&changed, None);
    let after_namespace_change = cache.kak_decomposition(&matrix).unwrap().unwrap();

    assert!(Arc::ptr_eq(&first, &after_namespace_change));
    assert_eq!(cache.stats().kak_misses, 1);
    assert_eq!(cache.stats().kak_hits, 2);
    assert_eq!(cache.stats().kak_entries, 1);
    assert_eq!(cache.stats().namespace_invalidations, 1);
}

#[test]
fn failed_kak_decomposition_is_cached_exactly() {
    let non_unitary = matrix_with(2.0);
    let mut cache = TwoQubitSynthesisCache::new_native_session();

    assert!(cache.kak_decomposition(&non_unitary).unwrap().is_none());
    assert!(cache.kak_decomposition(&non_unitary).unwrap().is_none());

    assert_eq!(cache.stats().kak_misses, 1);
    assert_eq!(cache.stats().kak_hits, 1);
    assert_eq!(cache.stats().kak_failed_hits, 1);
}

#[test]
fn failed_device_plan_is_cached() {
    let matrix = Array2::eye(4);
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let mut cache = TwoQubitSynthesisCache::new(1);
    let mut calls = 0;

    for _ in 0..2 {
        let failed = cache
            .with_device_plan(
                &matrix,
                qubits,
                DeviceSynthesisPlacement::ExactPhysical,
                |_| {
                    calls += 1;
                    Err(CompilerError::InvariantViolation("planned failure".into()))
                },
                |plan| matches!(plan, CachedPlanView::Failed),
            )
            .unwrap();
        assert!(failed);
    }

    assert_eq!(calls, 1);
    assert_eq!(cache.stats().device_misses, 1);
    assert_eq!(cache.stats().device_hits, 1);
    assert_eq!(cache.stats().failed_plan_hits, 1);
}

#[test]
fn admission_budget_does_not_change_uncached_result() {
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let mut cache = TwoQubitSynthesisCache::new(1);
    for value in [1.0, 2.0, 2.0] {
        let matrix = matrix_with(value);
        let failed = cache
            .with_generic_plan(
                &matrix,
                qubits,
                |_| Err(CompilerError::InvariantViolation("failure".into())),
                |plan| matches!(plan, CachedPlanView::Failed),
            )
            .unwrap();
        assert!(failed);
    }
    assert_eq!(cache.stats().generic_entries, 1);
    assert_eq!(cache.stats().generic_misses, 3);
    assert_eq!(cache.stats().capacity_rejections, 2);
}

#[test]
fn production_budget_bounds_entry_count() {
    let qubits = [Qubit::new(0), Qubit::new(1)];
    let mut cache = TwoQubitSynthesisCache::default();
    for index in 0..=RESYNTHESIS_SYNTHESIS_CACHE_BUDGET {
        let matrix = matrix_with(f64::from_bits(index as u64));
        cache
            .with_generic_plan(&matrix, qubits, |_| Ok(Vec::new()), |_| ())
            .unwrap();
    }

    assert_eq!(
        cache.stats().generic_entries,
        RESYNTHESIS_SYNTHESIS_CACHE_BUDGET
    );
    assert_eq!(cache.stats().capacity_rejections, 1);
}

#[test]
fn selection_measurements_reset_without_resetting_cache_counters() {
    let mut cache = TwoQubitSynthesisCache::default();
    cache.selection_stats_mut().input_blocks = 7;
    cache.stats.generic_hits = 3;

    cache.begin_selection_pass();

    assert_eq!(
        cache.stats().selection,
        ResynthesisSelectionStats::default()
    );
    assert_eq!(cache.stats().generic_hits, 3);
}

#[test]
fn selected_candidate_validation_is_cached_across_qubit_labels() {
    let matrix = Array2::eye(4);
    let candidate_for = |qubit| TwoQubitSynthesisCandidate {
        backend: TwoQubitUnitaryDecomposeBasis::Cx,
        operations: vec![
            ValueOperation::from_standard(StandardGate::X, [qubit], []),
            ValueOperation::from_standard(StandardGate::X, [qubit], []),
        ],
        global_phase: 0.0,
        cost: TargetAwareSynthesisCost::default(),
    };
    let first = candidate_for(Qubit::new(0));
    let relabeled = candidate_for(Qubit::new(7));
    let mut cache = TwoQubitSynthesisCache::new(4);

    cache
        .check_selected_candidate(&matrix, [Qubit::new(0), Qubit::new(1)], &first)
        .unwrap();
    cache
        .check_selected_candidate(&matrix, [Qubit::new(7), Qubit::new(9)], &relabeled)
        .unwrap();

    let stats = cache.stats();
    assert_eq!(stats.candidate_validation_lookups, 2);
    assert_eq!(stats.candidate_validation_misses, 1);
    assert_eq!(stats.candidate_validation_hits, 1);
    assert_eq!(stats.candidate_validation_entries, 1);
    assert_eq!(stats.candidate_validation_failures, 0);
}

#[test]
fn inexact_selected_candidate_validation_is_cached() {
    let matrix = Array2::eye(4);
    let candidate = TwoQubitSynthesisCandidate {
        backend: TwoQubitUnitaryDecomposeBasis::Cx,
        operations: Vec::new(),
        global_phase: 0.25,
        cost: TargetAwareSynthesisCost::default(),
    };
    let mut cache = TwoQubitSynthesisCache::new(4);

    let first = cache
        .check_selected_candidate(&matrix, [Qubit::new(0), Qubit::new(1)], &candidate)
        .unwrap();
    let second = cache
        .check_selected_candidate(&matrix, [Qubit::new(0), Qubit::new(1)], &candidate)
        .unwrap();

    assert!(matches!(
        first,
        NumericTwoQubitCandidateValidation::Inexact { .. }
    ));
    assert_eq!(first, second);
    let stats = cache.stats();
    assert_eq!(stats.candidate_validation_misses, 1);
    assert_eq!(stats.candidate_validation_hits, 1);
    assert_eq!(stats.candidate_validation_entries, 1);
    assert_eq!(stats.candidate_validation_failures, 1);
}
