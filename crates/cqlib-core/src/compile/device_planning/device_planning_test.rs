// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.

use super::registry::DevicePlanningNamespace;
use super::*;
use crate::circuit::{Instruction, StandardGate};
use crate::compile::knowledge::RuleLibrary;
use crate::device::{Device, PhysicalQubit};
use smallvec::smallvec;
use std::sync::{Arc, Barrier};

type LeafProjection = (String, Vec<u32>, Option<u64>, Option<u64>);

fn swap(left: u32, right: u32) -> DeviceGateState {
    DeviceGateState::standard(
        StandardGate::SWAP,
        smallvec![PhysicalQubit::new(left), PhysicalQubit::new(right)],
    )
}

fn summary_projection(summary: &NativePlanSummary) -> Vec<LeafProjection> {
    summary
        .leaves
        .iter()
        .map(|leaf| {
            (
                leaf.instruction.to_string(),
                leaf.ordered_qargs.iter().map(|qubit| qubit.id()).collect(),
                leaf.error_rate.map(f64::to_bits),
                leaf.duration.map(f64::to_bits),
            )
        })
        .collect()
}

#[test]
fn catalog_summarizes_the_same_native_swap_plan_used_by_lowering() {
    let device = Device::line("native-plan-summary", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001)
        .with_default_two_qubit_error(0.01);
    let root = DeviceGateState::standard(
        StandardGate::SWAP,
        smallvec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
    );

    let session = DevicePlanningSession::new(&device);
    let catalog = NativePlanCatalog::build_with_session(&session, [root.clone()]).unwrap();
    let summary = catalog.summary(&root).expect("SWAP should be lowerable");

    assert_eq!(summary.native_two_qubit_ops, 3);
    assert_eq!(summary.native_total_ops, 7);
    assert_eq!(
        summary
            .leaves
            .iter()
            .filter(|leaf| matches!(leaf.instruction, Instruction::Standard(StandardGate::CX)))
            .count(),
        3
    );
    assert!(summary.leaves.iter().all(|leaf| match leaf.instruction {
        Instruction::Standard(StandardGate::CX) => leaf.error_rate == Some(0.01),
        Instruction::Standard(StandardGate::H) => leaf.error_rate == Some(0.001),
        _ => false,
    }));
}

#[test]
fn catalog_results_are_independent_of_root_input_order() {
    let device = Device::line("native-plan-root-order", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let swap = DeviceGateState::standard(
        StandardGate::SWAP,
        smallvec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
    );
    let h = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);

    let forward_session = DevicePlanningSession::new(&device);
    let reverse_session = DevicePlanningSession::new(&device);
    let forward =
        NativePlanCatalog::build_with_session(&forward_session, [swap.clone(), h.clone()]).unwrap();
    let reverse =
        NativePlanCatalog::build_with_session(&reverse_session, [h.clone(), swap.clone()]).unwrap();
    for root in [&swap, &h] {
        let project = |catalog: &NativePlanCatalog| {
            let summary = catalog.summary(root).expect("root should be lowerable");
            (
                summary.native_two_qubit_ops,
                summary.native_total_ops,
                summary
                    .leaves
                    .iter()
                    .map(|leaf| (leaf.instruction.clone(), leaf.ordered_qargs.clone()))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(project(&forward), project(&reverse));
    }
}

#[test]
fn catalog_distinguishes_unsupported_from_unprepared_roots() {
    let device = Device::line("native-plan-availability", 2).unwrap();
    let unsupported = DeviceGateState::standard(
        StandardGate::CX,
        smallvec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
    );
    let unprepared = DeviceGateState::standard(
        StandardGate::CZ,
        smallvec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
    );

    let session = DevicePlanningSession::new(&device);
    let catalog = NativePlanCatalog::build_with_session(&session, [unsupported.clone()]).unwrap();

    assert!(matches!(
        catalog.availability(&unsupported),
        Some(NativePlanAvailability::Unsupported(_))
    ));
    assert!(catalog.availability(&unprepared).is_none());
}

#[test]
fn planning_session_reuses_prepared_roots_across_catalog_views() {
    let device = Device::line("native-plan-session", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let root = DeviceGateState::standard(
        StandardGate::CX,
        smallvec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
    );
    let session = DevicePlanningSession::new(&device);

    let first = NativePlanCatalog::build_with_session(&session, [root.clone()]).unwrap();
    let second = NativePlanCatalog::build_with_session(&session, [root.clone()]).unwrap();

    assert!(first.summary(&root).is_some());
    assert!(second.summary(&root).is_some());
    let first_plan = session
        .prepare([root.clone()])
        .unwrap()
        .selected_plan(&root)
        .unwrap();
    let second_plan = session
        .prepare([root.clone()])
        .unwrap()
        .selected_plan(&root)
        .unwrap();
    assert!(Arc::ptr_eq(&first_plan, &second_plan));
}

#[test]
fn planning_session_serves_subset_requests_from_a_prepared_batch() {
    let device = Device::line("native-plan-subset", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let first = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);
    let second = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(1)]);
    let session = DevicePlanningSession::new(&device);

    let batch = session.prepare([first.clone(), second]).unwrap();
    let subset = session.prepare([first.clone()]).unwrap();

    assert!(Arc::ptr_eq(
        &batch.selected_plan(&first).unwrap(),
        &subset.selected_plan(&first).unwrap()
    ));
}

#[test]
fn planning_session_reuses_overlap_and_plans_only_missing_roots() {
    let device = Device::line("native-plan-overlap", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let first = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);
    let shared = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(1)]);
    let last = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(2)]);
    let session = DevicePlanningSession::new(&device);

    let initial = session.prepare([first, shared.clone()]).unwrap();
    let shared_plan = initial.selected_plan(&shared).unwrap();
    let overlapping = session.prepare([shared.clone(), last.clone()]).unwrap();

    assert!(Arc::ptr_eq(
        &shared_plan,
        &overlapping.selected_plan(&shared).unwrap()
    ));
    assert!(overlapping.selected_plan(&last).is_some());
}

#[test]
fn planning_session_evicts_old_circuit_specific_batches() {
    let device = Device::line("native-plan-bounded-cache", 24)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let first_root = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);
    let session = DevicePlanningSession::new(&device);
    let first = session
        .prepare([first_root.clone()])
        .unwrap()
        .selected_plan(&first_root)
        .unwrap();

    for index in 1..24 {
        let root = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(index)]);
        session.prepare([root]).unwrap();
    }

    let reloaded = session
        .prepare([first_root.clone()])
        .unwrap()
        .selected_plan(&first_root)
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &reloaded));
}

#[test]
fn concurrent_requests_share_one_session_local_batch() {
    const WORKERS: usize = 8;
    let device = Device::line("concurrent-local-session", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let session = Arc::new(DevicePlanningSession::new(&device));
    let barrier = Arc::new(Barrier::new(WORKERS));
    let handles = (0..WORKERS)
        .map(|_| {
            let session = Arc::clone(&session);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let root =
                    DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);
                barrier.wait();
                session
                    .prepare([root.clone()])
                    .unwrap()
                    .selected_plan(&root)
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let plans = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert!(plans[1..].iter().all(|plan| Arc::ptr_eq(&plans[0], plan)));
}

#[test]
fn process_registry_reuses_semantically_identical_devices_and_invalidates_changes() {
    let build = |name: &str, error: f64| {
        Device::bidirectional_line(name, 2)
            .unwrap()
            .with_native_gates(vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CX),
            ])
            .unwrap()
            .with_default_single_qubit_error(0.211)
            .with_default_two_qubit_error(error)
    };
    let root = swap(0, 1);
    let first = DevicePlanningSession::new(&build("registry-first-name", 0.212));
    let first_plans = first.prepare([root.clone()]).unwrap();
    let first_plan = first_plans.selected_plan(&root).unwrap();

    // Device identity and descriptive metadata are intentionally irrelevant.
    let renamed_device = build("registry-second-name", 0.212);
    assert_eq!(
        DevicePlanningNamespace::from_device(&build("registry-first-name", 0.212)),
        DevicePlanningNamespace::from_device(&renamed_device)
    );
    let renamed = DevicePlanningSession::new(&renamed_device);
    let renamed_plans = renamed.prepare([root.clone()]).unwrap();
    let renamed_plan = renamed_plans.selected_plan(&root).unwrap();
    assert_eq!(
        summary_projection(&first_plan.summary),
        summary_projection(&renamed_plan.summary)
    );

    // Exact effective calibration bits are part of the namespace.
    let recalibrated_device = build("registry-first-name", 0.213);
    assert_ne!(
        DevicePlanningNamespace::from_device(&renamed_device),
        DevicePlanningNamespace::from_device(&recalibrated_device)
    );
    let directional_device = Device::line("registry-direction-change", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.211)
        .with_default_two_qubit_error(0.212);
    assert_ne!(
        DevicePlanningNamespace::from_device(&renamed_device),
        DevicePlanningNamespace::from_device(&directional_device)
    );
    let recalibrated = DevicePlanningSession::new(&recalibrated_device);
    let recalibrated_plans = recalibrated.prepare([root.clone()]).unwrap();
    let recalibrated_plan = recalibrated_plans.selected_plan(&root).unwrap();
    assert!(!Arc::ptr_eq(&first_plan, &recalibrated_plan));
    assert_ne!(
        summary_projection(&first_plan.summary),
        summary_projection(&recalibrated_plan.summary)
    );
}

#[test]
fn circuit_specific_roots_remain_session_local() {
    let device = Device::line("local-root-overlay", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap()
        .with_default_single_qubit_error(0.219);
    let root = DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(0)]);
    let first = DevicePlanningSession::new(&device);
    let second = DevicePlanningSession::new(&device);
    let first_plans = first.prepare([root.clone()]).unwrap();
    let second_plans = second.prepare([root.clone()]).unwrap();

    assert!(!Arc::ptr_eq(
        &first_plans.selected_plan(&root).unwrap(),
        &second_plans.selected_plan(&root).unwrap()
    ));
}

#[test]
fn equivalent_swap_plans_match_the_original_batch_planner_exactly() {
    let device = Device::bidirectional_line("equivalent-swap-baseline", 5)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.221)
        .with_default_two_qubit_error(0.222);
    let roots = (0..4)
        .flat_map(|left| [swap(left, left + 1), swap(left + 1, left)])
        .collect::<Vec<_>>();

    let session = DevicePlanningSession::new(&device);
    let plans = session.prepare(roots.iter().cloned()).unwrap();

    let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
    let estimator = Arc::new(CalibrationEstimator::from_device(&device, &physical_qubits));
    let planner = DevicePlanner::build_with_estimator(
        &device,
        RuleLibrary::builtin_rules().unwrap(),
        roots.iter().cloned(),
        estimator,
    )
    .unwrap();

    for root in roots {
        let optimized = plans.selected_plan(&root).unwrap();
        let baseline_id = planner.selected_plan_for(&root).unwrap();
        assert_eq!(
            optimized.choice,
            planner.choice_for_plan(baseline_id).unwrap()
        );
        assert_eq!(
            optimized.physical_cost,
            planner.cost_for_plan(baseline_id).unwrap()
        );
        assert_eq!(
            summary_projection(&optimized.summary),
            summary_projection(&planner.summary_for_plan(baseline_id).unwrap())
        );
    }
}

#[test]
fn equivalent_local_plans_match_the_original_batch_planner_exactly() {
    let device = Device::bidirectional_line("equivalent-local-baseline", 4)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.231)
        .with_default_two_qubit_error(0.232);
    let mut roots = (0..4)
        .map(|qarg| DeviceGateState::standard(StandardGate::H, smallvec![PhysicalQubit::new(qarg)]))
        .collect::<Vec<_>>();
    roots.extend((0..3).flat_map(|left| {
        [
            DeviceGateState::standard(
                StandardGate::CZ,
                smallvec![PhysicalQubit::new(left), PhysicalQubit::new(left + 1)],
            ),
            DeviceGateState::standard(
                StandardGate::CZ,
                smallvec![PhysicalQubit::new(left + 1), PhysicalQubit::new(left)],
            ),
        ]
    }));

    let session = DevicePlanningSession::new(&device);
    let optimized = session.prepare(roots.iter().cloned()).unwrap();
    let physical_qubits = device.usable_qubits().collect::<Vec<_>>();
    let baseline = DevicePlanner::build_with_estimator(
        &device,
        RuleLibrary::builtin_rules().unwrap(),
        roots.iter().cloned(),
        Arc::new(CalibrationEstimator::from_device(&device, &physical_qubits)),
    )
    .unwrap();

    for root in roots {
        let optimized = optimized.selected_plan(&root).unwrap();
        let baseline_id = baseline.selected_plan_for(&root).unwrap();
        assert_eq!(
            optimized.choice,
            baseline.choice_for_plan(baseline_id).unwrap()
        );
        assert_eq!(
            optimized.physical_cost,
            baseline.cost_for_plan(baseline_id).unwrap()
        );
        assert_eq!(
            summary_projection(&optimized.summary),
            summary_projection(&baseline.summary_for_plan(baseline_id).unwrap())
        );
    }
}

#[test]
fn equivalent_unsupported_swaps_remap_structured_failures() {
    let device = Device::bidirectional_line("unsupported-swap-remap", 4).unwrap();
    let roots = (0..3)
        .flat_map(|left| [swap(left, left + 1), swap(left + 1, left)])
        .collect::<Vec<_>>();
    let session = DevicePlanningSession::new(&device);
    let catalog = NativePlanCatalog::build_with_session(&session, roots.clone()).unwrap();

    for root in roots {
        let NativePlanAvailability::Unsupported(failure) = catalog.availability(&root).unwrap()
        else {
            panic!("SWAP should be unsupported without native gates");
        };
        assert_eq!(failure.qargs, root.ordered_qargs.as_slice());
        assert!(failure.attempted_candidates.iter().all(|candidate| {
            candidate.unsatisfied_dependencies.iter().all(|dependency| {
                dependency
                    .qargs
                    .iter()
                    .all(|qarg| root.ordered_qargs.contains(qarg))
            })
        }));
    }
}

#[test]
fn concurrent_sessions_singleflight_a_common_swap() {
    const WORKERS: usize = 8;
    let device = Arc::new(
        Device::bidirectional_line("concurrent-swap-registry", 2)
            .unwrap()
            .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
            .unwrap()
            .with_default_two_qubit_error(0.231),
    );
    let barrier = Arc::new(Barrier::new(WORKERS));
    let handles = (0..WORKERS)
        .map(|_| {
            let device = Arc::clone(&device);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let root = swap(0, 1);
                barrier.wait();
                let session = DevicePlanningSession::new(&device);
                session
                    .prepare([root.clone()])
                    .unwrap()
                    .selected_plan(&root)
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let plans = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert!(plans[1..].iter().all(|plan| Arc::ptr_eq(&plans[0], plan)));
}

#[test]
fn process_registry_evicts_the_least_recently_used_device() {
    let root = swap(0, 1);
    let devices = (0..9)
        .map(|index| {
            Device::bidirectional_line(format!("lru-device-{index}"), 2)
                .unwrap()
                .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
                .unwrap()
                .with_default_two_qubit_error(0.301 + f64::from(index) * 0.001)
        })
        .collect::<Vec<_>>();

    let first_session = DevicePlanningSession::new(&devices[0]);
    let first_plans = first_session.prepare([root.clone()]).unwrap();
    let first_plan = first_plans.selected_plan(&root).unwrap();
    for device in &devices[1..] {
        let session = DevicePlanningSession::new(device);
        session.prepare([root.clone()]).unwrap();
    }

    let reloaded = DevicePlanningSession::new(&devices[0]);
    let reloaded_plans = reloaded.prepare([root.clone()]).unwrap();
    assert!(!Arc::ptr_eq(
        &first_plan,
        &reloaded_plans.selected_plan(&root).unwrap()
    ));
}
