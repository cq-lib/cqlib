// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.

use super::*;
use crate::circuit::{Instruction, StandardGate};
use smallvec::smallvec;

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
    let first_plan = session.selected_plan(&root).unwrap();
    let second_plan = session.selected_plan(&root).unwrap();
    assert!(Arc::ptr_eq(&first_plan, &second_plan));
}
