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
use crate::circuit::{
    Circuit, CircuitParam, ClassicalControlOp, ClassicalExpr, ClassicalType, Instruction,
    Operation, Parameter, ParameterValue, Qubit, StandardGate,
};
use crate::compile::CompilerError;
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit, Topology};
use std::collections::HashSet;

#[test]
fn deterministic_seeded_config_uses_compact_reproducible_settings() {
    let config = SabreConfig::deterministic_seeded(7);

    assert_eq!(config.layout_trials, 2);
    assert_eq!(config.refinement_iterations, 1);
    assert_eq!(config.layout_scoring_trials, 1);
    assert_eq!(config.routing_trials, 1);
    assert_eq!(config.trial_objective, SabreTrialObjective::SwapThenDepth);
    assert_eq!(config.seed, Some(7));
    assert_eq!(config.heuristic.lookahead_weights, vec![0.5]);
    assert_eq!(config.heuristic.attempt_limit, 20);
}

#[test]
fn validate_config_reports_invalid_trial_counts() {
    let config = SabreConfig {
        routing_trials: 0,
        ..SabreConfig::deterministic_seeded(7)
    };

    let error = validate_config(&config).unwrap_err();

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
        CompilerError::InvalidInput(message) if message.contains("disconnected")
    ));
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
fn control_flow_body_is_routed_and_restored() {
    let device = Device::line("line", 3).unwrap();
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
            assert!(op.then_body().operations().iter().any(|operation| matches!(
                operation.instruction,
                Instruction::Standard(StandardGate::SWAP)
            )));
        }
        other => panic!("expected routed if/else operation, got {other:?}"),
    }
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

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("disconnected"))
    );
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
    let device = Device::line("line", 4).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 3)], 4).unwrap();
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
}

#[test]
fn trial_objective_controls_quality_tie_breaking() {
    let left = super::routing::TrialQuality {
        swap_count: 1,
        two_qubit_depth: 5,
        operation_count: 20,
    };
    let right = super::routing::TrialQuality {
        swap_count: 1,
        two_qubit_depth: 2,
        operation_count: 8,
    };

    assert!(
        super::routing::compare_trial_quality(SabreTrialObjective::SwapCount, left, 0, right, 1)
            .is_lt()
    );
    assert!(
        super::routing::compare_trial_quality(
            SabreTrialObjective::SwapThenDepth,
            left,
            0,
            right,
            1
        )
        .is_gt()
    );
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
        layout_scoring_trials: 0,
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
