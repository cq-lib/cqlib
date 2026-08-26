// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of the License in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::test_utils::build_device_synthesis_context;
use crate::compile::transform::{ResynthesizeTwoQubitBlocks, TransformerTestExt};
use crate::device::{EdgeProp, InstructionProp, PhysicalQubit};
use std::sync::Arc;

fn native_optimizer<'a>(
    device: &'a Device,
    resynthesis: TwoQubitBlockResynthesisConfig,
    max_rounds: u8,
    max_stale_rounds: u8,
) -> NativeOptimizer<'a> {
    NativeOptimizer::with_session(
        device,
        resynthesis,
        max_rounds,
        max_stale_rounds,
        Arc::new(DevicePlanningSession::new(device)),
    )
}

fn assert_same_native_result(
    reused: &NativeOptimizationResult,
    rebuilt: &NativeOptimizationResult,
) {
    assert_eq!(reused.circuit, rebuilt.circuit);
    assert_eq!(reused.changed, rebuilt.changed);
    assert_eq!(reused.rounds, rebuilt.rounds);
    assert_eq!(reused.restored_best, rebuilt.restored_best);
    assert_eq!(reused.before, rebuilt.before);
    assert_eq!(reused.after, rebuilt.after);
}

/// Reference implementation of the pre-reuse behavior. It deliberately rebuilds
/// an exact context for every consumer so tests can compare semantic results.
fn run_rebuild_every_use(
    optimizer: &NativeOptimizer<'_>,
    circuit: &Circuit,
) -> Result<NativeOptimizationResult, CompilerError> {
    let initial = Canonicalizer::production()
        .transform_resolved(circuit, None)?
        .circuit;
    optimizer.device().validate_circuit(&initial)?;
    let mut current = initial.clone();
    let mut best = initial;
    let initial_context = build_device_synthesis_context(
        optimizer.device(),
        &best,
        DeviceSynthesisPlacement::ExactPhysical,
    )?;
    let mut best_costs =
        scope_costs_with_context(&best, &initial_context).map_err(scope_cost_error)?;
    let before = summarize_scope_costs(&best_costs);
    let mut rounds = 0;
    let mut stale = 0_u8;

    while rounds < optimizer.max_rounds && stale < optimizer.max_stale_rounds {
        rounds += 1;
        let resynthesis_context = build_device_synthesis_context(
            optimizer.device(),
            &current,
            DeviceSynthesisPlacement::ExactPhysical,
        )?;
        let resynthesized = ResynthesizeTwoQubitBlocks::new_device_aware(
            optimizer.resynthesis.clone(),
            resynthesis_context,
        )
        .transform_resolved(&current, None)?
        .circuit;
        let local_context = build_device_synthesis_context(
            optimizer.device(),
            &resynthesized,
            DeviceSynthesisPlacement::ExactPhysical,
        )?;
        let locally_optimized = OptimizeNativeLocalGates::new(local_context)
            .transform_resolved(&resynthesized, None)?
            .circuit;
        let legalized = match DeviceLowerer::new(optimizer.device())
            .transform_resolved(&locally_optimized, None)
        {
            Ok(result) => result.circuit,
            Err(CompilerError::DeviceLoweringFailed(_)) => {
                DeviceLowerer::new(optimizer.device())
                    .transform_resolved(&resynthesized, None)?
                    .circuit
            }
            Err(error) => return Err(error),
        };
        let candidate = Canonicalizer::production()
            .transform_resolved(&legalized, None)?
            .circuit;
        optimizer.device().validate_circuit(&candidate)?;
        if candidate == current {
            current = candidate;
            break;
        }

        let candidate_context = build_device_synthesis_context(
            optimizer.device(),
            &candidate,
            DeviceSynthesisPlacement::ExactPhysical,
        )?;
        let candidate_costs =
            scope_costs_with_context(&candidate, &candidate_context).map_err(scope_cost_error)?;
        if scope_costs_dominate(&candidate_costs, &best_costs) {
            best = candidate.clone();
            best_costs = candidate_costs;
            stale = 0;
        } else {
            stale = stale.saturating_add(1);
        }
        current = candidate;
    }

    let restored_best = current != best;
    let after = summarize_scope_costs(&best_costs);
    Ok(NativeOptimizationResult {
        changed: best != *circuit,
        circuit: best,
        rounds,
        restored_best,
        before,
        after,
    })
}

#[test]
fn native_optimizer_stops_after_one_stable_round() {
    let device = Device::line("native-fixed-point", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::U)])
        .unwrap();
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.u(q0, 0.2, 0.3, 0.4).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        8,
        3,
    );

    let result = optimizer.run(&circuit).unwrap();

    assert_eq!(result.rounds, 1);
    assert!(!result.changed);
    assert!(!result.restored_best);
    assert_eq!(result.circuit, circuit);
}

#[test]
fn native_optimizer_reuse_matches_rebuild_every_use() {
    let device = Device::line("native-context-reference", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::U)])
        .unwrap()
        .with_default_single_qubit_error(0.001);
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.u(q0, 0.2, 0.3, 0.4).unwrap();
    circuit.u(q0, -0.1, 0.5, -0.2).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        8,
        3,
    );

    let reused = optimizer.run(&circuit).unwrap();
    let rebuilt = run_rebuild_every_use(&optimizer, &circuit).unwrap();

    assert_same_native_result(&reused, &rebuilt);
}

#[test]
fn native_optimizer_reuse_matches_rebuild_on_calibrated_two_qubit_workload() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::bidirectional_line("native-context-calibrated", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001)
        .with_default_two_qubit_error(0.02);
    for (control, target, error) in [(p0, p1, 0.005), (p1, p0, 0.04)] {
        device
            .add_edge_properties(
                control,
                target,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::CX),
                        error,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let mut circuit = Circuit::new(2);
    circuit.u(q0, 0.2, -0.1, 0.4).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.u(q1, -0.3, 0.5, 0.2).unwrap();
    circuit.cx(q0, q1).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        4,
        2,
    );

    let reused = optimizer.run(&circuit).unwrap();
    let rebuilt = run_rebuild_every_use(&optimizer, &circuit).unwrap();

    assert_same_native_result(&reused, &rebuilt);
}

#[test]
fn incremental_resynthesis_matches_full_scan_and_reuses_clean_flat_anchors() {
    let device = Device::bidirectional_line("native-incremental-reference", 6)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let mut circuit = Circuit::new(6);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(3)).unwrap();
    circuit.cx(Qubit::new(4), Qubit::new(5)).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        4,
        2,
    );

    let (incremental, incremental_stats) = optimizer
        .run_with_policy(&circuit, NativeResynthesisPolicy::Incremental)
        .unwrap();
    let (full_scan, full_stats) = optimizer
        .run_with_policy(&circuit, NativeResynthesisPolicy::FullScan)
        .unwrap();

    assert_same_native_result(&incremental, &full_scan);
    assert!(
        incremental_stats.anchors_reused >= 2,
        "incremental result={incremental:?}, stats={incremental_stats:?}"
    );
    assert_eq!(full_stats.anchors_reused, 0);
    assert_eq!(
        full_stats.anchors_recomputed, full_stats.anchors_total,
        "the reference policy must recompute every anchor"
    );
}

#[test]
fn native_rounds_reuse_clean_terminal_resynthesis_decisions() {
    let device = Device::bidirectional_line("native-resynthesis-session-cache", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001);
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let q2 = Qubit::new(2);
    let mut circuit = Circuit::new(3);
    // This already-native 2Q block is reconsidered after the disjoint q2 run
    // is fused in round one, so round two exercises the session warm path.
    circuit.u(q0, 0.2, -0.3, 0.4).unwrap();
    circuit.cx(q0, q1).unwrap();
    circuit.u(q2, 0.1, 0.2, 0.3).unwrap();
    circuit.u(q2, -0.2, 0.4, -0.1).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        4,
        2,
    );

    let (result, stats) = optimizer
        .run_with_policy(&circuit, NativeResynthesisPolicy::Incremental)
        .unwrap();

    assert!(result.rounds >= 2, "result={result:?}");
    assert_eq!(
        stats.synthesis_cache.block_fact_misses, 1,
        "stats={stats:?}"
    );
    assert_eq!(stats.synthesis_cache.device_misses, 1, "stats={stats:?}");
    assert!(stats.synthesis_cache.kak_entries > 0, "stats={stats:?}");
    assert!(
        stats.synthesis_cache.terminal_decision_hits > 0,
        "stats={stats:?}"
    );
    assert!(stats.anchors_reused > 0, "stats={stats:?}");
}

#[test]
fn native_optimizer_lazily_prepares_candidate_context() {
    let device = Device::line("native-context-fallback", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::U),
            Instruction::Standard(StandardGate::FSIM),
        ])
        .unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        2,
        1,
    );
    let initial = Circuit::new(2);
    let mut context =
        build_device_synthesis_context(&device, &initial, DeviceSynthesisPlacement::ExactPhysical)
            .unwrap();
    let mut candidate = Circuit::new(2);
    candidate
        .fsim(Qubit::new(0), Qubit::new(1), 0.2, -0.3)
        .unwrap();

    assert!(scope_costs_with_context(&candidate, &context).is_ok());
    let costs = optimizer
        .candidate_costs_with_reuse(&candidate, &mut context)
        .unwrap();

    assert_eq!(costs.len(), 1);
    assert!(scope_costs_with_context(&candidate, &context).is_ok());
}

#[test]
fn native_optimizer_does_not_rebuild_unsupported_candidate() {
    let device = Device::line("native-context-unsupported", 2).unwrap();
    let optimizer = native_optimizer(
        &device,
        TwoQubitBlockResynthesisConfig::normal(Default::default()),
        2,
        1,
    );
    let mut candidate = Circuit::new(2);
    candidate.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let mut context = build_device_synthesis_context(
        &device,
        &candidate,
        DeviceSynthesisPlacement::ExactPhysical,
    )
    .unwrap();
    assert!(matches!(
        scope_costs_with_context(&candidate, &context),
        Err(ScopeCostError::Context(
            DeviceContextCostFailure::Unsupported(_)
        ))
    ));

    let error = optimizer
        .candidate_costs_with_reuse(&candidate, &mut context)
        .unwrap_err();

    assert!(matches!(error, CompilerError::InvariantViolation(_)));
    assert!(matches!(
        scope_costs_with_context(&candidate, &context),
        Err(ScopeCostError::Context(
            DeviceContextCostFailure::Unsupported(_)
        ))
    ));
}

#[test]
fn one_qubit_fusion_requires_exact_physical_improvement() {
    let device = Device::line("native-fusion", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::U)])
        .unwrap()
        .with_default_single_qubit_error(0.001);
    let q0 = Qubit::new(0);
    let mut circuit = Circuit::new(1);
    circuit.u(q0, 0.2, 0.3, 0.4).unwrap();
    circuit.u(q0, -0.1, 0.5, -0.2).unwrap();
    let context =
        build_device_synthesis_context(&device, &circuit, DeviceSynthesisPlacement::ExactPhysical)
            .unwrap();

    let result = OptimizeNativeLocalGates::new(context)
        .transform_resolved(&circuit, None)
        .unwrap();

    assert!(result.changed);
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::U)
    ));
}

#[test]
fn cx_propagates_target_z_to_both_qubits() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let operations = vec![
        ValueOperation::from_standard(StandardGate::Z, [q1], []),
        ValueOperation::from_standard(StandardGate::CX, [q0, q1], []),
    ];

    let rewrite = propagate_frames(operations).unwrap();
    let gates = rewrite
        .operations
        .iter()
        .filter_map(|operation| match operation.instruction {
            ValueInstruction::Instruction(Instruction::Standard(gate)) => Some(gate),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        gates,
        vec![StandardGate::CX, StandardGate::Z, StandardGate::Z]
    );
}

#[test]
fn swap_exchanges_pending_frames() {
    let q0 = Qubit::new(0);
    let q1 = Qubit::new(1);
    let rewrite = propagate_frames(vec![
        ValueOperation::from_standard(StandardGate::Z, [q0], []),
        ValueOperation::from_standard(StandardGate::SWAP, [q0, q1], []),
    ])
    .unwrap();

    assert!(matches!(
        rewrite.operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::SWAP))
    ));
    assert!(matches!(
        rewrite.operations[1].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::Z))
    ));
    assert_eq!(rewrite.operations[1].qubits.as_slice(), &[q1]);
}

#[test]
fn measurement_drops_a_pending_z_frame() {
    let q0 = Qubit::new(0);
    let measurement = ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Directive(Directive::Measure)),
        qubits: smallvec![q0],
        params: SmallVec::new(),
        label: None,
    };

    let rewrite = propagate_frames(vec![
        ValueOperation::from_standard(StandardGate::RZ, [q0], [ParameterValue::Fixed(0.4)]),
        measurement,
    ])
    .unwrap();

    assert_eq!(rewrite.operations.len(), 1);
    assert!(matches!(
        rewrite.operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Directive(Directive::Measure))
    ));
}

#[test]
fn phase_carrier_records_rz_global_phase_difference() {
    let q0 = Qubit::new(0);
    let rewrite = propagate_frames(vec![ValueOperation::from_standard(
        StandardGate::Phase,
        [q0],
        [ParameterValue::Fixed(0.6)],
    )])
    .unwrap();

    assert!((rewrite.phase_delta - 0.3).abs() < PHASE_EPS);
    assert!(matches!(
        rewrite.operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZ))
    ));
}

#[test]
fn z_frame_is_absorbed_into_xy_axis() {
    let q0 = Qubit::new(0);
    let rewrite = propagate_frames(vec![
        ValueOperation::from_standard(StandardGate::RZ, [q0], [ParameterValue::Fixed(0.2)]),
        ValueOperation::from_standard(StandardGate::XY, [q0], [ParameterValue::Fixed(0.7)]),
    ])
    .unwrap();

    assert!(matches!(
        rewrite.operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::XY))
    ));
    let ParameterValue::Fixed(axis) = rewrite.operations[0].params[0] else {
        panic!("XY axis must remain numeric");
    };
    assert!((axis - 0.5).abs() < PHASE_EPS);
    assert!(matches!(
        rewrite.operations[1].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::RZ))
    ));
}

#[test]
fn pauli_product_uses_circuit_time_order() {
    let q0 = Qubit::new(0);
    let rewrite = propagate_frames(vec![
        ValueOperation::from_standard(StandardGate::X, [q0], []),
        ValueOperation::from_standard(StandardGate::Z, [q0], []),
    ])
    .unwrap();

    assert!((rewrite.phase_delta - FRAC_PI_2).abs() < PHASE_EPS);
    assert!(matches!(
        rewrite.operations[0].instruction,
        ValueInstruction::Instruction(Instruction::Standard(StandardGate::Y))
    ));
}
