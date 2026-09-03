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

use super::*;
use crate::circuit::circuit_to_matrix;
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ClassicalExpr, ClassicalType, Directive,
    Instruction, Operation, Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::transform::layout::PhysicalLayoutGraph;
use crate::compile::transform::{DeviceLowerer, TransformerTestExt};
use crate::compile::{CompilerError, SabreRoutingFailure};
use crate::device::{
    Device, EdgeProp, InstructionProp, Layout, LogicalQubit, PhysicalQubit, QubitProp, Topology,
};
use rayon::ThreadPoolBuilder;
use std::collections::HashSet;

fn sabre_route_device(
    circuit: &Circuit,
    device: &Device,
    initial_layout: &Layout,
    config: &SabreConfig,
) -> Result<SabreRoutingResult, CompilerError> {
    let planning_session = DevicePlanningSession::new(device);
    let physical = PhysicalLayoutGraph::from_device(device)?;
    sabre_route_with_provenance_and_session_on_physical(
        circuit,
        device,
        &physical,
        initial_layout,
        config,
        &planning_session,
    )
    .map(|result| result.routing)
}

fn basis_index_under_layout(index: usize, layout: &Layout, width: usize) -> usize {
    let mut mapped = 0usize;
    for logical in 0..width {
        if index & (1usize << logical) == 0 {
            continue;
        }
        let physical = layout
            .get_physical(LogicalQubit::new(logical as u32))
            .expect("test layout must map every logical qubit");
        mapped |= 1usize << physical.id() as usize;
    }
    mapped
}

#[test]
fn routing_matrix_matches_initial_and_final_layout_permutations() {
    let mut circuit = Circuit::new(4);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.ry(Qubit::new(3), -0.37).unwrap();
    circuit.rzz(Qubit::new(1), Qubit::new(3), 0.41).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(1)).unwrap();
    let initial = Layout::from_pairs(&[(0, 3), (1, 0), (2, 2), (3, 1)], 4).unwrap();
    let device = Device::line("semantic-line", 4).unwrap();

    let routed = sabre_route(
        &circuit,
        &device,
        &initial,
        &SabreConfig::deterministic_seeded(29),
    )
    .unwrap();
    let logical_matrix = circuit_to_matrix(&circuit, None).unwrap();
    let physical_matrix = circuit_to_matrix(&routed.circuit, None).unwrap();
    let dimension = 1usize << 4;

    for logical_input in 0..dimension {
        let physical_input = basis_index_under_layout(logical_input, &routed.initial_layout, 4);
        for logical_output in 0..dimension {
            let physical_output = basis_index_under_layout(logical_output, &routed.final_layout, 4);
            let difference = (physical_matrix[[physical_output, physical_input]]
                - logical_matrix[[logical_output, logical_input]])
            .norm();
            assert!(
                difference < 1e-9,
                "routing changed semantics at logical ({logical_output}, {logical_input}): {difference}"
            );
        }
    }
}

#[test]
fn validate_config_reports_invalid_trial_counts() {
    let config = SabreConfig {
        routing_trials: 0,
        ..SabreConfig::deterministic_seeded(7)
    };

    let error = config.validate().unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("routing_trials"))
    );
}

#[test]
fn normalize_initial_layout_public_api_uses_device_usable_qubits() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 2), (1, 0)], 3).unwrap();

    let normalized = normalize_initial_layout(
        &[LogicalQubit::new(0), LogicalQubit::new(1)],
        &device,
        &layout,
    )
    .unwrap();

    assert_eq!(normalized.num_logical(), 2);
    assert_eq!(normalized.num_physical(), 3);
    assert_eq!(normalized.num_vacant_physical(), 1);
    assert_eq!(
        normalized.get_physical(LogicalQubit::new(0)),
        Some(PhysicalQubit::new(2))
    );
}

#[test]
fn unary_requirement_moves_to_a_locally_supported_physical_qubit() {
    let mut device = Device::line("local-unary", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
        .unwrap();
    device
        .add_qubit_properties(
            PhysicalQubit::new(2),
            QubitProp::new(0.0)
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::H),
                    0.001,
                ))
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0)], 3).unwrap();
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let routed = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(11),
    )
    .unwrap();

    assert_eq!(routed.swap_count, 2);
    assert_eq!(
        routed.final_layout.get_physical(LogicalQubit::new(0)),
        Some(PhysicalQubit::new(2))
    );
    assert!(matches!(
        routed
            .circuit
            .operations()
            .last()
            .map(|operation| &operation.instruction),
        Some(Instruction::Standard(StandardGate::H))
    ));
}

#[test]
fn validate_reachable_interactions_public_api_reports_disconnected_pairs() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let topology = Topology::new(vec![p0, p1], vec![]).unwrap();
    let device = Device::new("disconnected", HashSet::from([p0, p1]), topology).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = validate_reachable_interactions(&circuit, &device, &layout).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::NoExecutablePairTerminal { logical: [left, right] }
        ) if left == LogicalQubit::new(0)
            && right == LogicalQubit::new(1)
    ));
}

#[test]
fn exact_device_route_reports_unreachable_unary_placement() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::line("local-unary-without-movement", 2).unwrap();
    device
        .add_qubit_properties(
            p1,
            QubitProp::new(0.0)
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::H),
                    0.001,
                ))
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0)], 2).unwrap();
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();

    let error = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(0),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::SabreRoutingFailed(
            SabreRoutingFailure::UnreachableUnaryPlacement { logical, physical }
        ) if logical == LogicalQubit::new(0) && physical == p0
    ));
}

#[test]
fn adjacent_terminal_gate_does_not_require_a_lowerable_swap_edge() {
    // A directed CX can execute on 0 -> 1, but without a native H the reverse
    // CX needed by the usual SWAP decomposition is unavailable.
    let device = Device::line("directed-cx", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap();

    assert_eq!(result.swap_count, 0);
}

#[test]
fn topology_connectivity_does_not_hide_missing_swap_lowering() {
    let device = Device::line("directed-cx", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::SabreRoutingFailed(SabreRoutingFailure::UnreachablePairPlacement { .. })
    ));
}

#[test]
fn folded_terminal_requires_every_gate_and_direction_to_be_lowerable() {
    let device = Device::line("directed-cx", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let error = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::SabreRoutingFailed(SabreRoutingFailure::UnreachablePairPlacement { .. })
    ));
}

#[test]
fn predicted_native_counts_match_the_actual_device_lowerer() {
    let device = Device::line("native-line", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001)
        .with_default_two_qubit_error(0.01);
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let routed = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap();
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&routed.circuit, None)
        .unwrap()
        .circuit;
    let actual_native_two_qubit = lowered
        .operations()
        .iter()
        .filter(|operation| operation.qubits.len() == 2)
        .count();

    assert_eq!(
        routed.diagnostics.native_two_qubit_count,
        actual_native_two_qubit
    );
    assert_eq!(
        routed.diagnostics.native_operation_count,
        lowered.operations().len()
    );
    assert!(routed.diagnostics.native_total_depth > 0);
}

#[test]
fn native_total_depth_schedules_parallel_native_leaves() {
    let p0 = PhysicalQubit::new(0);
    let p1 = PhysicalQubit::new(1);
    let mut device = Device::line("timed-reverse-cx", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    for physical in [p0, p1] {
        device
            .add_qubit_properties(
                physical,
                QubitProp::new(0.0)
                    .with_native_instruction(
                        InstructionProp::new(Instruction::Standard(StandardGate::H), 0.001)
                            .with_length(10.0),
                    )
                    .unwrap(),
            )
            .unwrap();
    }
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(
                    InstructionProp::new(Instruction::Standard(StandardGate::CX), 0.01)
                        .with_length(100.0),
                )
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let result = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap();

    assert_eq!(result.swap_count, 0);
    // The two H gates before and after the directed CX each form one parallel
    // unit-depth layer: H-pair + CX + H-pair.
    assert_eq!(result.diagnostics.native_total_depth, 3);
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&result.circuit, None)
        .unwrap()
        .circuit;
    assert_eq!(lowered.operations().len(), 5);
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn native_depth_and_count_multiply_static_for_iterations() {
    let p0 = PhysicalQubit::new(0);
    let mut device = Device::line("timed-static-for", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    device
        .add_qubit_properties(
            p0,
            QubitProp::new(0.0)
                .with_native_instruction(
                    InstructionProp::new(Instruction::Standard(StandardGate::H), 0.001)
                        .with_length(10.0),
                )
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0)], 1).unwrap();
    let mut circuit = Circuit::new(1);
    let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(3, 0).unwrap(),
            ClassicalExpr::uint_literal(3, 3).unwrap(),
            ClassicalExpr::uint_literal(3, 1).unwrap(),
            |body, _| {
                body.h(Qubit::new(0))?;
                body.continue_loop()
            },
        )
        .unwrap();

    let result = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap();

    assert_eq!(result.diagnostics.native_total_depth, 3);
    assert_eq!(result.diagnostics.native_operation_count, 3);
}

#[test]
fn dynamic_nonzero_loop_reports_unknown_count_and_one_body_depth() {
    let p0 = PhysicalQubit::new(0);
    let mut device = Device::line("timed-dynamic-loop", 1)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    device
        .add_qubit_properties(
            p0,
            QubitProp::new(0.0)
                .with_native_instruction(
                    InstructionProp::new(Instruction::Standard(StandardGate::H), 0.001)
                        .with_length(10.0),
                )
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0)], 1).unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(0))?;
            body.break_loop()
        })
        .unwrap();

    let result = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(5),
    )
    .unwrap();

    assert_eq!(result.diagnostics.native_total_depth, 1);
    assert_eq!(result.diagnostics.unknown_loop_count, 1);
}

#[test]
fn uniform_native_cost_does_not_increase_swaps_over_topology_scoring() {
    let topology_device = Device::line("uniform-topology-line", 6).unwrap();
    let native_device = Device::line("uniform-native-line", 6)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap()
        .with_default_single_qubit_error(0.001)
        .with_default_two_qubit_error(0.01);
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)], 6).unwrap();
    let mut circuit = Circuit::new(6);
    for (left, right) in [
        (0, 5),
        (1, 4),
        (2, 3),
        (0, 3),
        (5, 2),
        (1, 5),
        (4, 0),
        (2, 4),
        (3, 1),
        (0, 5),
    ] {
        circuit.cx(Qubit::new(left), Qubit::new(right)).unwrap();
    }

    for seed in 0..8 {
        let config = SabreConfig {
            routing_trials: 3,
            seed: Some(seed),
            ..SabreConfig::deterministic_seeded(seed)
        };
        let topology = sabre_route(&circuit, &topology_device, &layout, &config).unwrap();
        let native = sabre_route_device(&circuit, &native_device, &layout, &config).unwrap();

        assert!(
            native.swap_count <= topology.swap_count,
            "uniform native scoring added SWAPs for seed {seed}: topology={}, native={}",
            topology.swap_count,
            native.swap_count
        );
    }
}

#[test]
fn route_keeps_adjacent_two_qubit_gate_without_swap() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 0);
    assert_eq!(result.circuit.operations().len(), 1);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    assert_eq!(
        result.final_layout.get_physical(LogicalQubit::new(0)),
        Some(PhysicalQubit::new(0))
    );
}

#[test]
fn route_preserves_empty_barrier_as_global_ordering_boundary() {
    let q0 = Qubit::new(0);
    let device = Device::line("line", 1).unwrap();
    let layout = Layout::from_pairs(&[(0, 0)], 1).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(1);
    circuit.x(q0).unwrap();
    circuit.barrier(Vec::new()).unwrap();
    circuit.x(q0).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.circuit.operations().len(), 3);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        result.circuit.operations()[1].instruction,
        Instruction::Directive(Directive::Barrier)
    ));
    assert!(result.circuit.operations()[1].qubits.is_empty());
    assert!(matches!(
        result.circuit.operations()[2].instruction,
        Instruction::Standard(StandardGate::X)
    ));
}

#[test]
fn route_inserts_swap_on_line_topology() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 1);
    assert_eq!(result.circuit.operations().len(), 2);
    assert!(matches!(
        result.circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::SWAP)
    ));
    let gate_qubits = &result.circuit.operations()[1].qubits;
    assert_eq!(gate_qubits.len(), 2);
    assert!(
        are_adjacent(gate_qubits[0], gate_qubits[1]),
        "routed two-qubit operation must be adjacent"
    );
}

#[test]
fn route_does_not_fold_overlapping_two_qubit_gates() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert!(result.swap_count > 0);
    assert_all_two_qubit_operations_are_adjacent_on_line(&result.circuit);
}

#[test]
fn route_may_fold_consecutive_two_qubit_gates_on_same_pair() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 0);
    assert_eq!(result.circuit.operations().len(), 2);
    assert_all_two_qubit_operations_are_adjacent_on_line(&result.circuit);
}

#[test]
fn route_with_decay_is_reproducible_for_same_seed() {
    let device = Device::line("line", 5).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 4), (2, 2)], 5).unwrap();
    let config = SabreConfig {
        routing_trials: 4,
        seed: Some(23),
        heuristic: SabreHeuristicConfig {
            decay_increment: Some(0.05),
            decay_reset: 2,
            lookahead_weights: vec![0.5, 0.25],
            attempt_limit: 20,
            ..SabreHeuristicConfig::default()
        },
        ..SabreConfig::deterministic_seeded(7)
    };
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let first = sabre_route(&circuit, &device, &layout, &config).unwrap();
    let second = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(first.swap_count, second.swap_count);
    assert_eq!(first.final_layout.l2p_map(), second.final_layout.l2p_map());
    assert_eq!(
        first.diagnostics.selected_trial_index,
        second.diagnostics.selected_trial_index
    );
    assert_eq!(
        first.diagnostics.operation_count,
        second.diagnostics.operation_count
    );
}

#[test]
fn fixed_seed_is_independent_of_rayon_thread_count() {
    let device = Device::line("line", 6)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 5), (2, 2), (3, 4)], 6).unwrap();
    let config = SabreConfig {
        routing_trials: 8,
        seed: Some(29),
        heuristic: SabreHeuristicConfig {
            lookahead_weights: vec![0.5, 0.25],
            decay_increment: Some(0.01),
            ..SabreHeuristicConfig::default()
        },
        ..SabreConfig::deterministic_seeded(29)
    };
    let mut circuit = Circuit::new(4);
    for (left, right) in [(0, 1), (2, 3), (0, 3), (1, 2), (0, 2), (1, 3)] {
        circuit.cx(Qubit::new(left), Qubit::new(right)).unwrap();
    }
    let route_in_pool = |threads| {
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| sabre_route(&circuit, &device, &layout, &config))
            .unwrap()
    };

    let single_threaded = route_in_pool(1);
    for threads in [2, 4, 8] {
        let parallel = route_in_pool(threads);
        assert_eq!(single_threaded.swap_count, parallel.swap_count);
        assert_eq!(
            single_threaded.final_layout.l2p_map(),
            parallel.final_layout.l2p_map()
        );
        assert_eq!(single_threaded.diagnostics, parallel.diagnostics);
        assert_eq!(single_threaded.circuit, parallel.circuit);
    }

    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&single_threaded.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn fixed_route_quality_is_structurally_deterministic_across_thread_counts() {
    let device = Device::line("objective-corpus", 6)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 5), (2, 2), (3, 4)], 6).unwrap();
    let mut circuit = Circuit::new(4);
    for (left, right) in [(0, 1), (2, 3), (0, 3), (1, 2), (0, 2), (1, 3)] {
        circuit.cx(Qubit::new(left), Qubit::new(right)).unwrap();
    }

    let config = SabreConfig {
        routing_trials: 8,
        seed: Some(43),
        ..SabreConfig::deterministic_seeded(43)
    };
    let route = |threads| {
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| sabre_route(&circuit, &device, &layout, &config))
            .unwrap()
    };

    let single = route(1);
    let parallel = route(4);
    assert_eq!(single.circuit, parallel.circuit);
    assert_eq!(single.initial_layout, parallel.initial_layout);
    assert_eq!(single.final_layout, parallel.final_layout);
    assert_eq!(single.swap_count, parallel.swap_count);
    assert_eq!(single.diagnostics, parallel.diagnostics);

    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&single.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn seeded_grid_unary_and_control_flow_corpus_is_thread_deterministic() {
    let verify = |circuit: &Circuit, device: &Device, layout: &Layout, config: &SabreConfig| {
        let route = |threads| {
            ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| sabre_route_device(circuit, device, layout, config))
                .unwrap()
        };
        let single = route(1);
        let parallel = route(4);

        assert_eq!(single.swap_count, parallel.swap_count);
        assert_eq!(
            single.initial_layout.l2p_map(),
            parallel.initial_layout.l2p_map()
        );
        assert_eq!(
            single.final_layout.l2p_map(),
            parallel.final_layout.l2p_map()
        );
        assert_eq!(single.diagnostics, parallel.diagnostics);
        assert_eq!(single.circuit, parallel.circuit);
        let lowered = DeviceLowerer::new(device)
            .transform_resolved(&single.circuit, None)
            .unwrap()
            .circuit;
        device.validate_circuit(&lowered).unwrap();
    };

    let grid = Device::grid("seeded-grid", 3, 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let grid_layout = Layout::from_pairs(&[(0, 0), (1, 8), (2, 2), (3, 6), (4, 4)], 9).unwrap();
    let mut grid_circuit = Circuit::new(5);
    for (left, right) in [(0, 1), (2, 3), (0, 4), (1, 3), (2, 4), (0, 2)] {
        grid_circuit
            .cx(Qubit::new(left), Qubit::new(right))
            .unwrap();
    }
    let grid_config = SabreConfig {
        routing_trials: 6,
        seed: Some(37),
        ..SabreConfig::deterministic_seeded(37)
    };
    verify(&grid_circuit, &grid, &grid_layout, &grid_config);

    let mut local_unary = Device::line("seeded-local-unary", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
        .unwrap();
    local_unary
        .add_qubit_properties(
            PhysicalQubit::new(2),
            QubitProp::new(0.0)
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::H),
                    0.001,
                ))
                .unwrap(),
        )
        .unwrap();
    let unary_layout = Layout::from_pairs(&[(0, 0)], 3).unwrap();
    let mut unary_circuit = Circuit::new(1);
    unary_circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(0))
        })
        .unwrap();
    let unary_config = SabreConfig {
        routing_trials: 4,
        seed: Some(41),
        ..SabreConfig::deterministic_seeded(41)
    };
    verify(&unary_circuit, &local_unary, &unary_layout, &unary_config);
}

#[test]
fn control_flow_body_is_routed_and_restored() {
    let device = Device::line("native-line", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.cx(Qubit::new(0), Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.diagnostics.control_flow_blocks_routed, 1);
    match &result.circuit.operations()[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            let epilogue_and_route_swaps = op
                .then_body()
                .operations()
                .iter()
                .filter(|operation| {
                    matches!(
                        operation.instruction,
                        Instruction::Standard(StandardGate::SWAP)
                    )
                })
                .count();
            assert!(epilogue_and_route_swaps >= 2);
            assert_eq!(result.swap_count, epilogue_and_route_swaps);
        }
        other => panic!("expected routed if/else operation, got {other:?}"),
    }
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&result.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn control_flow_epilogue_avoids_topology_edges_without_a_native_swap_plan() {
    let [p0, p1, p2] = [0, 1, 2].map(PhysicalQubit::new);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![
            (p0, p1, "native".to_string()),
            (p1, p2, "native".to_string()),
            (p0, p2, "terminal-only".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new("selective-swap", HashSet::from([p0, p1, p2]), topology).unwrap();
    let edge_properties = |gates: &[StandardGate]| {
        let mut properties = EdgeProp::new();
        for gate in gates {
            properties
                .set_native_instruction(InstructionProp::new(Instruction::Standard(*gate), 0.01))
                .unwrap();
        }
        properties
    };
    device
        .add_edge_properties(
            p0,
            p1,
            edge_properties(&[StandardGate::SWAP, StandardGate::CZ]),
        )
        .unwrap();
    device
        .add_edge_properties(
            p1,
            p2,
            edge_properties(&[StandardGate::SWAP, StandardGate::CZ]),
        )
        .unwrap();
    device
        .add_edge_properties(p0, p2, edge_properties(&[StandardGate::CX]))
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.cz(Qubit::new(0), Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = sabre_route_device(
        &circuit,
        &device,
        &layout,
        &SabreConfig::deterministic_seeded(17),
    )
    .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected routed if operation");
    };
    let swaps = op
        .then_body()
        .operations()
        .iter()
        .filter(|operation| {
            matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::SWAP)
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.swap_count, swaps.len());
    assert!(!swaps.is_empty());
    assert!(swaps.iter().all(|operation| {
        operation
            .qubits
            .iter()
            .copied()
            .map(PhysicalQubit::from_qubit)
            .collect::<HashSet<_>>()
            != HashSet::from([p0, p2])
    }));
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&result.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn measurement_driven_control_flow_preserves_classical_identity() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(Qubit::new(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.cx(Qubit::new(0), Qubit::new(1))?;
            Ok(())
        })
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.circuit.id(), circuit.id());
    assert_eq!(
        result.circuit.classical_values(),
        circuit.classical_values()
    );
    result.circuit.validate().unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.circuit.operations()[1].instruction
    else {
        panic!("expected routed if operation");
    };
    assert!(if_op.classical_value_reads().contains(&measured.value()));
}

#[test]
fn if_else_routes_both_branches_and_restores_layout() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |body| {
                body.cx(Qubit::new(0), Qubit::new(1))?;
                Ok(())
            },
            |body| {
                body.cx(Qubit::new(1), Qubit::new(0))?;
                Ok(())
            },
        )
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.diagnostics.control_flow_blocks_routed, 2);
    match &result.circuit.operations()[0].instruction {
        Instruction::ClassicalControl(ClassicalControlOp::If(op)) => {
            assert!(op.then_body().operations().iter().any(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::SWAP)
            )));
            assert!(
                op.else_body()
                    .unwrap()
                    .operations()
                    .iter()
                    .any(|operation| matches!(
                        operation.instruction,
                        Instruction::Standard(StandardGate::SWAP)
                    ))
            );
        }
        other => panic!("expected routed if/else operation, got {other:?}"),
    }
}

#[test]
fn empty_control_flow_bodies_route_without_layout_drift() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit
        .if_else(ClassicalExpr::bool_literal(true), |_| Ok(()), |_| Ok(()))
        .unwrap();
    circuit
        .while_(ClassicalExpr::bool_literal(false), |_| Ok(()))
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 0);
    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.diagnostics.control_flow_blocks_routed, 3);
}

#[test]
fn route_keeps_grid_adjacent_gates_without_swap() {
    let device = Device::grid("grid", 2, 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2), (3, 3)], 4).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(4);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(3)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 0);
    assert_eq!(result.circuit.operations().len(), 2);
    for operation in result.circuit.operations() {
        assert!(operation_is_adjacent_on_grid(operation, 2));
    }
}

#[test]
fn route_handles_empty_and_single_qubit_circuits_without_swap() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);

    let empty = Circuit::new(2);
    let empty_result = sabre_route(&empty, &device, &layout, &config).unwrap();
    assert_eq!(empty_result.swap_count, 0);
    assert!(empty_result.circuit.operations().is_empty());

    let mut single_qubit = Circuit::new(2);
    single_qubit.h(Qubit::new(0)).unwrap();
    single_qubit.x(Qubit::new(1)).unwrap();
    let single_result = sabre_route(&single_qubit, &device, &layout, &config).unwrap();
    assert_eq!(single_result.swap_count, 0);
    assert_eq!(single_result.circuit.operations().len(), 2);
    assert_eq!(single_result.final_layout.l2p_map(), layout.l2p_map());
}

#[test]
fn route_rejects_three_qubit_gate_before_routing() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(3);
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    let error = sabre_route(&circuit, &device, &layout, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("more than two qubits"))
    );
}

#[test]
fn route_rejects_incomplete_initial_layout() {
    let device = Device::line("line", 2).unwrap();
    let incomplete = Layout::from_pairs(&[(0, 0)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = sabre_route(&circuit, &device, &incomplete, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("does not map logical qubit"))
    );
}

#[test]
fn route_disconnected_topology_errors_without_panic() {
    let qubits = vec![PhysicalQubit::new(0), PhysicalQubit::new(1)];
    let topology = Topology::new(
        qubits.clone(),
        Vec::<(PhysicalQubit, PhysicalQubit, String)>::new(),
    )
    .unwrap();
    let device = Device::new(
        "disconnected",
        qubits.iter().copied().collect::<HashSet<_>>(),
        topology,
    )
    .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = sabre_route(&circuit, &device, &layout, &config).unwrap_err();

    assert!(matches!(
        error,
        CompilerError::SabreRoutingFailed(SabreRoutingFailure::NoExecutablePairTerminal { .. })
    ));
}

#[test]
fn route_preserves_parameterized_gate_parameters() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let theta = Parameter::symbol("theta");
    let mut circuit = Circuit::new(2);
    circuit
        .rx(Qubit::new(0), ParameterValue::Param(theta.clone()))
        .unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert!(result.circuit.parameters().contains(&theta));
    let rx = result
        .circuit
        .operations()
        .iter()
        .find(|operation| {
            matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::RX)
            )
        })
        .expect("routed circuit preserves RX operation");
    assert!(matches!(rx.params.as_slice(), [CircuitParam::Index(_)]));
}

#[test]
fn route_remaps_parameters_used_only_inside_control_flow_bodies() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let body_theta = Parameter::symbol("body_theta");
    let top_theta = Parameter::symbol("top_theta");
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.rx(Qubit::new(0), ParameterValue::Param(body_theta.clone()))?;
            Ok(())
        })
        .unwrap();
    circuit
        .rz(Qubit::new(1), ParameterValue::Param(top_theta.clone()))
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    let Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected routed if operation");
    };
    let body_rx = &if_op.then_body().operations()[0];
    assert_eq!(
        result
            .circuit
            .resolve_parameter(&body_rx.params[0])
            .unwrap(),
        body_theta
    );
    assert_eq!(
        result
            .circuit
            .resolve_parameter(&result.circuit.operations()[1].params[0])
            .unwrap(),
        top_theta
    );
}

#[test]
fn route_preserves_multiple_parameters_and_global_phase() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let theta = Parameter::symbol("theta");
    let phi = Parameter::symbol("phi");
    let gamma = Parameter::symbol("gamma");
    let mut circuit = Circuit::new(2);
    circuit.set_global_phase(gamma.clone());
    circuit
        .rx(Qubit::new(0), ParameterValue::Param(theta.clone()))
        .unwrap();
    circuit
        .rz(Qubit::new(1), ParameterValue::Param(phi.clone()))
        .unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.circuit.global_phase(), gamma);
    assert!(result.circuit.parameters().contains(&theta));
    assert!(result.circuit.parameters().contains(&phi));
    assert!(result.circuit.parameters().contains(&gamma));
    assert!(result.circuit.operations().iter().any(|operation| matches!(
        operation.instruction,
        Instruction::Standard(StandardGate::RX)
    ) && matches!(
        operation.params.as_slice(),
        [CircuitParam::Index(_)]
    )));
    assert!(result.circuit.operations().iter().any(|operation| matches!(
        operation.instruction,
        Instruction::Standard(StandardGate::RZ)
    ) && matches!(
        operation.params.as_slice(),
        [CircuitParam::Index(_)]
    )));
}

#[test]
fn nested_control_flow_is_routed_and_restored() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |then_body| {
            then_body.while_(ClassicalExpr::bool_literal(true), |while_body| {
                while_body.cx(Qubit::new(0), Qubit::new(1))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.diagnostics.control_flow_blocks_routed, 2);
}

#[test]
fn for_and_switch_bodies_are_routed_with_control_transfers_preserved() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(2);
    let loop_var = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(2, 0).unwrap(),
            ClassicalExpr::uint_literal(2, 2).unwrap(),
            ClassicalExpr::uint_literal(2, 1).unwrap(),
            |body, _| {
                body.cx(Qubit::new(0), Qubit::new(1))?;
                body.continue_loop()?;
                Ok(())
            },
        )
        .unwrap();
    circuit
        .switch(ClassicalExpr::uint_literal(2, 1).unwrap(), |switch| {
            switch.value(1, |body| {
                body.cx(Qubit::new(1), Qubit::new(0))?;
                body.break_loop()?;
                Ok(())
            })?;
            switch.default(|_| Ok(()))?;
            Ok(())
        })
        .unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.final_layout.l2p_map(), layout.l2p_map());
    assert_eq!(result.diagnostics.control_flow_blocks_routed, 3);
    let Instruction::ClassicalControl(ClassicalControlOp::For(for_op)) =
        &result.circuit.operations()[0].instruction
    else {
        panic!("expected routed for operation");
    };
    assert!(matches!(
        for_op.body().operations().last().map(|op| &op.instruction),
        Some(Instruction::ClassicalControl(ClassicalControlOp::Continue))
    ));
    let Instruction::ClassicalControl(ClassicalControlOp::Switch(switch_op)) =
        &result.circuit.operations()[1].instruction
    else {
        panic!("expected routed switch operation");
    };
    assert!(matches!(
        switch_op.cases()[0]
            .body()
            .operations()
            .last()
            .map(|op| &op.instruction),
        Some(Instruction::ClassicalControl(ClassicalControlOp::Break))
    ));
}

#[test]
fn routing_trials_select_no_worse_than_first_trial() {
    let device = Device::line("line", 4).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 3), (2, 1)], 4).unwrap();
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let first = sabre_route(
        &circuit,
        &device,
        &layout,
        &SabreConfig {
            routing_trials: 1,
            seed: Some(19),
            ..SabreConfig::deterministic_seeded(7)
        },
    )
    .unwrap();
    let multi = sabre_route(
        &circuit,
        &device,
        &layout,
        &SabreConfig {
            routing_trials: 5,
            seed: Some(19),
            ..SabreConfig::deterministic_seeded(7)
        },
    )
    .unwrap();

    assert_eq!(multi.diagnostics.trials_evaluated, 5);
    assert!(multi.swap_count <= first.swap_count);
}

#[test]
fn fallback_triggers_when_attempt_limit_is_zero() {
    // One greedy SWAP would make the pair adjacent. With a zero attempt
    // budget, fallback must run before that greedy choice is attempted.
    let device = Device::line("native-line", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig {
        heuristic: SabreHeuristicConfig {
            attempt_limit: 0,
            ..SabreConfig::deterministic_seeded(7).heuristic
        },
        ..SabreConfig::deterministic_seeded(7)
    };
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert!(result.swap_count > 0);
    assert!(result.diagnostics.fallback_count > 0);
    assert_all_two_qubit_operations_are_adjacent_on_line(&result.circuit);
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&result.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn fallback_emits_the_verified_order_of_a_directional_native_swap() {
    let [p0, p1, p2] = [0, 1, 2].map(PhysicalQubit::new);
    let topology = Topology::new(
        vec![p0, p1, p2],
        vec![
            (p1, p0, "terminal".to_string()),
            (p1, p2, "movement".to_string()),
        ],
    )
    .unwrap();
    let mut device =
        Device::new("directional-swap", HashSet::from([p0, p1, p2]), topology).unwrap();
    device
        .add_edge_properties(
            p1,
            p0,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    device
        .add_edge_properties(
            p1,
            p2,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::SWAP),
                    0.02,
                ))
                .unwrap(),
        )
        .unwrap();
    let layout = Layout::from_pairs(&[(0, 2), (1, 0)], 3).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let config = SabreConfig {
        heuristic: SabreHeuristicConfig {
            attempt_limit: 0,
            ..SabreConfig::deterministic_seeded(29).heuristic
        },
        ..SabreConfig::deterministic_seeded(29)
    };

    let result = sabre_route_device(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.diagnostics.fallback_count, 1);
    assert_eq!(result.swap_count, 1);
    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        [p1.qubit(), p2.qubit()]
    );
    let lowered = DeviceLowerer::new(&device)
        .transform_resolved(&result.circuit, None)
        .unwrap()
        .circuit;
    device.validate_circuit(&lowered).unwrap();
}

#[test]
fn route_diagnostics_report_selected_quality_metrics() {
    let device = Device::line("line", 3).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let config = SabreConfig {
        routing_trials: 3,
        seed: Some(11),
        ..SabreConfig::deterministic_seeded(7)
    };
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.diagnostics.trials_evaluated, 3);
    assert!(result.diagnostics.selected_trial_index < 3);
    assert!(result.diagnostics.two_qubit_depth > 0);
    assert_eq!(
        result.diagnostics.operation_count,
        result.circuit.operations().len()
    );
}

#[test]
fn layout_only_trial_counts_do_not_block_routing() {
    let device = Device::line("line", 2).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let config = SabreConfig {
        layout_trials: 0,
        ..SabreConfig::deterministic_seeded(7)
    };

    let result = sabre_route(&circuit, &device, &layout, &config).unwrap();

    assert_eq!(result.swap_count, 0);
    assert_eq!(result.diagnostics.trials_evaluated, config.routing_trials);
}

fn are_adjacent(left: Qubit, right: Qubit) -> bool {
    left.id().abs_diff(right.id()) == 1
}

fn assert_all_two_qubit_operations_are_adjacent_on_line(circuit: &Circuit) {
    for operation in circuit.operations() {
        if operation.qubits.len() == 2 {
            assert!(
                are_adjacent(operation.qubits[0], operation.qubits[1]),
                "operation {operation:?} is not adjacent on line topology"
            );
        }
    }
}

fn operation_is_adjacent_on_grid(operation: &Operation, cols: u32) -> bool {
    if operation.qubits.len() != 2 {
        return true;
    }
    let left = operation.qubits[0].id();
    let right = operation.qubits[1].id();
    left.abs_diff(right) == 1 && left / cols == right / cols || left.abs_diff(right) == cols
}
