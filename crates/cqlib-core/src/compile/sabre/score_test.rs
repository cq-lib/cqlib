// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in
// the root directory of this source tree or at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::*;
use crate::circuit::{Circuit, ClassicalExpr, ClassicalType, Qubit};
use crate::compile::device_planning::DevicePlanningSession;
use crate::compile::sabre::cost::{MetricAvailability, RobustDurationKey, RobustErrorKey};
use crate::device::{EdgeProp, InstructionProp, PhysicalQubit, Topology};
use rayon::ThreadPoolBuilder;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Barrier, mpsc};
use std::thread;
use std::time::Duration;

fn routing_target_from_device(
    device: &Device,
    physical: &PhysicalLayoutGraph,
    sabre: &SabreDag,
) -> Result<RoutingTarget, CompilerError> {
    let planning_session = DevicePlanningSession::new(device);
    RoutingTarget::from_device_with_session(device, physical, sabre, &planning_session)
}

fn route_trial_for_test(
    sabre: &SabreDag,
    target: &RoutingTarget,
    layout: &Layout,
    heuristic: &SabreHeuristicConfig,
    seed: u64,
) -> Result<TrialResult, CompilerError> {
    let metadata = PreparedRouteMetadata::new(sabre, target)?;
    let unscored =
        route_unscored_trial_with_metadata(sabre, target, &metadata, layout, heuristic, seed)?;
    RankedTrial::from_unscored(unscored, target)?.finish(target)
}

#[test]
fn prepared_metadata_distinguishes_high_pair_reuse() {
    let device = Device::line("pair-reuse-metadata", 4).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();

    let mut repeated = Circuit::new(4);
    for _ in 0..HIGH_PAIR_REUSE_FACTOR {
        repeated.cx(Qubit::new(0), Qubit::new(1)).unwrap();
        repeated.h(Qubit::new(0)).unwrap();
        repeated.cx(Qubit::new(2), Qubit::new(3)).unwrap();
        repeated.h(Qubit::new(2)).unwrap();
    }
    let repeated_dag = SabreDag::from_operations(repeated.operations()).unwrap();
    let repeated_target = RoutingTarget::from_physical(&physical).unwrap();
    let repeated_metadata = PreparedRouteMetadata::new(&repeated_dag, &repeated_target).unwrap();
    assert!(repeated_metadata.high_pair_reuse);

    let mut sparse = Circuit::new(4);
    sparse.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    sparse.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    sparse.cx(Qubit::new(0), Qubit::new(3)).unwrap();
    let sparse_dag = SabreDag::from_operations(sparse.operations()).unwrap();
    let sparse_target = RoutingTarget::from_physical(&physical).unwrap();
    let sparse_metadata = PreparedRouteMetadata::new(&sparse_dag, &sparse_target).unwrap();
    assert!(!sparse_metadata.high_pair_reuse);
}

#[test]
fn layout_only_refinement_matches_full_output_final_layout() {
    let device = Device::line("refinement-parity", 4).unwrap();
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    let sabre = SabreDag::refinement_workload(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 0), (1, 1), (2, 3)], 4).unwrap();
    let heuristic = SabreConfig::deterministic_seeded(17).heuristic;

    let full =
        route_unscored_trial_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 29)
            .unwrap();
    let layout_only =
        refine_layout_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 29).unwrap();

    assert_eq!(layout_only, full.final_layout);
}

#[test]
fn layout_only_control_flow_routing_matches_full_output_restoration() {
    let device = Device::line("control-flow-refinement-parity", 3).unwrap();
    let mut circuit = Circuit::new(3);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.cx(Qubit::new(0), Qubit::new(2))
        })
        .unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).unwrap();
    let heuristic = SabreConfig::deterministic_seeded(31).heuristic;

    let full =
        route_unscored_trial_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 37)
            .unwrap();
    let layout_only =
        refine_layout_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 37).unwrap();

    assert_eq!(layout_only, full.final_layout);
    assert_eq!(layout_only, initial);
    assert!(!full.materialize_operations(&target).unwrap().is_empty());
}

#[test]
fn compact_route_plan_matches_materialized_incremental_quality() {
    let device = Device::line("compact-route-quality", 3)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = routing_target_from_device(&device, &physical, &sabre).unwrap();
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 0), (1, 2)], 3).unwrap();
    let heuristic = SabreConfig::deterministic_seeded(41).heuristic;

    let trial =
        route_unscored_trial_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 43)
            .unwrap();
    let operations = trial.materialize_operations(&target).unwrap();
    let rescanned = native_plan_cost_for_operations(&operations, &target).unwrap();

    assert!(trial.swap_count > 0);
    assert_eq!(trial.operation_count, operation_count(&operations));
    assert_eq!(
        trial.two_qubit_operation_count,
        two_qubit_operation_count(&operations)
    );
    assert_eq!(trial.native_cost.static_native, rescanned.static_native);
    assert_eq!(trial.native_cost.path, rescanned.path);
}

#[test]
fn topology_route_plan_defers_ordinary_operation_materialization() {
    let device = Device::line("deferred-route-operations", 2).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 1), (1, 0)], 2).unwrap();
    let heuristic = SabreConfig::deterministic_seeded(45).heuristic;

    let trial =
        route_unscored_trial_with_metadata(&sabre, &target, &metadata, &initial, &heuristic, 47)
            .unwrap();
    assert!(
        trial
            .plan
            .steps
            .iter()
            .all(|step| matches!(step, CompactRouteStep::Mapped { .. }))
    );

    let dense_layout = DenseRoutingLayout::from_layout(&initial, &target.physical_qubits).unwrap();
    let expected = circuit
        .operations()
        .iter()
        .map(|operation| map_operation_dense(operation, &dense_layout, &target).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(trial.materialize_operations(&target).unwrap(), expected);
}

#[test]
fn topology_lookahead_skips_unary_work_but_keeps_future_two_qubit_requirements() {
    let device = Device::line("lookahead-routing-horizon", 4).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(4);
    circuit.cx(Qubit::new(0), Qubit::new(3)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    assert!(!target.native_cost_enabled);
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2), (3, 3)], 4).unwrap();
    let heuristic = SabreHeuristicConfig {
        lookahead_weights: vec![0.5],
        ..SabreHeuristicConfig::default()
    };
    let mut state =
        RoutingState::new(&sabre, &target, &metadata, &initial, &heuristic, 47).unwrap();
    let mut output = TrialOutput::new(47, false);

    state
        .update_route(
            &sabre,
            &target,
            &metadata,
            &heuristic,
            &mut output,
            &sabre.first_layer,
            None,
        )
        .unwrap();
    state.populate_extended_set(&sabre, &target).unwrap();

    let future = state.lookahead_layers[0].iter_nodes().collect::<Vec<_>>();
    assert_eq!(future.len(), 1);
    assert!(matches!(
        sabre.graph[future[0]].kind,
        SabreNodeKind::TwoQ(_)
    ));
}

#[test]
fn device_lookahead_keeps_placement_sensitive_unary_requirements() {
    let device = Device::line("device-lookahead-unary", 4)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(4);
    circuit.cx(Qubit::new(0), Qubit::new(3)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let target = routing_target_from_device(&device, &physical, &sabre).unwrap();
    assert!(target.native_cost_enabled);
    let metadata = PreparedRouteMetadata::new(&sabre, &target).unwrap();
    let initial = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2), (3, 3)], 4).unwrap();
    let heuristic = SabreHeuristicConfig {
        lookahead_weights: vec![0.5],
        ..SabreHeuristicConfig::default()
    };
    let mut state =
        RoutingState::new(&sabre, &target, &metadata, &initial, &heuristic, 53).unwrap();
    let mut output = TrialOutput::new(53, false);

    state
        .update_route(
            &sabre,
            &target,
            &metadata,
            &heuristic,
            &mut output,
            &sabre.first_layer,
            None,
        )
        .unwrap();
    state.populate_extended_set(&sabre, &target).unwrap();

    let future = state.lookahead_layers[0].iter_nodes().collect::<Vec<_>>();
    assert_eq!(future.len(), 1);
    assert!(matches!(
        sabre.graph[future[0]].kind,
        SabreNodeKind::Unary(_)
    ));
}

fn movement_edge(left: usize, right: usize, cost: NativePlanCost) -> MovementEdge {
    MovementEdge {
        endpoints: [left, right],
        swap: VerifiedSwap {
            emitted_indices: [left, right],
            cost,
        },
    }
}

fn shared_lazy_cache_len(target: &RoutingTarget) -> usize {
    target
        .lazy_pair_cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .values
        .len()
}

fn line_distance(
    _interaction: usize,
    placement: RequirementPlacement,
) -> Result<f64, CompilerError> {
    Ok(match placement {
        RequirementPlacement::Unary(physical) => physical as f64,
        RequirementPlacement::Pair([left, right]) => left.abs_diff(right) as f64,
    })
}

#[test]
fn heuristic_keeps_front_sum_and_scales_lookahead_by_active_layer() {
    let mut front = Layer::new(2, 5);
    front
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 4]),
            &line_distance,
        )
        .unwrap();
    front
        .insert(
            NodeIndex::new(1),
            0,
            RequirementPlacement::Pair([1, 3]),
            &line_distance,
        )
        .unwrap();
    let mut lookahead = Layer::new(1, 5);
    lookahead
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 2]),
            &line_distance,
        )
        .unwrap();
    let empty = Layer::new(0, 5);
    let heuristic = SabreHeuristicConfig {
        basic_weight: 1.0,
        lookahead_weights: vec![0.5, 20.0],
        ..SabreHeuristicConfig::default()
    };

    let score = heuristic_score_after_swap(
        &front,
        &[lookahead, empty],
        &heuristic,
        [0, 1],
        &line_distance,
    )
    .unwrap();

    // The front remains a sum, while the one-node lookahead contributes 0.5.
    assert!((score - 6.5).abs() < 1e-12);
}

#[test]
fn topology_score_is_independent_of_congestion_decay() {
    let mut front = Layer::new(1, 4);
    front
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 3]),
            &line_distance,
        )
        .unwrap();
    let heuristic = SabreHeuristicConfig {
        basic_weight: 2.0,
        lookahead_weights: Vec::new(),
        ..SabreHeuristicConfig::default()
    };

    let score =
        heuristic_score_after_swap(&front, &[], &heuristic, [0, 1], &line_distance).unwrap();

    assert_eq!(score, 4.0);
}

#[test]
fn zero_width_and_empty_layers_have_a_finite_zero_score() {
    let front = Layer::new(0, 0);
    let lookahead = Layer::new(0, 0);
    let heuristic = SabreHeuristicConfig {
        lookahead_weights: vec![1.0],
        ..SabreHeuristicConfig::default()
    };

    let score =
        heuristic_score_after_swap(&front, &[lookahead], &heuristic, [0, 0], &line_distance)
            .unwrap();

    assert_eq!(score, 0.0);
    assert!(score.is_finite());
}

#[test]
fn scored_candidate_caches_unreachable_native_route_cost() {
    let mut candidate = ScoredCandidate {
        choice: SwapChoice {
            physical: [PhysicalQubit::new(0), PhysicalQubit::new(1)],
            emitted: [PhysicalQubit::new(0), PhysicalQubit::new(1)],
            indices: [0, 1],
            cost: NativePlanCost::default(),
        },
        adjusted_score: 1.0,
        route_cost: None,
    };
    let mut calls = 0usize;

    for _ in 0..2 {
        assert_eq!(
            candidate.cached_route_cost(|_| {
                calls += 1;
                None
            }),
            None
        );
    }

    assert_eq!(calls, 1);
}

#[test]
fn pair_state_distance_preserves_terminal_direction_and_disconnection() {
    let neighbors = default_movement_adjacency(4, &[(0, 1), (1, 2)]);
    let terminals = BTreeMap::from([([0, 1], NativePlanCost::default())]);

    let distances = pair_route_lower_bounds(&neighbors, &terminals);

    assert_eq!(
        distances.get(0, 1).map(|bound| bound.remaining_swaps),
        Some(0)
    );
    assert_eq!(
        distances.get(1, 0).map(|bound| bound.remaining_swaps),
        Some(1)
    );
    assert_eq!(
        distances.get(2, 0).map(|bound| bound.remaining_swaps),
        Some(2)
    );
    assert_eq!(
        distances.get(0, 2).map(|bound| bound.remaining_swaps),
        Some(1)
    );
    assert_eq!(distances.get(3, 0), None);
    assert_eq!(distances.get(0, 3), None);
}

#[test]
fn lazy_pair_state_search_matches_the_eager_table() {
    let neighbors = default_movement_adjacency(4, &[(0, 1), (1, 2), (2, 3)]);
    let terminals = BTreeMap::from([([0, 1], NativePlanCost::default())]);
    let eager = pair_route_lower_bounds(&neighbors, &terminals);

    for left in 0..4 {
        for right in 0..4 {
            if left == right {
                continue;
            }
            assert_eq!(
                pair_route_lower_bound_from_state(&neighbors, &terminals, [left, right]),
                eager.get(left, right),
                "lazy/eager mismatch for ({left}, {right})"
            );
        }
    }
}

#[test]
fn eager_and_lazy_pair_costs_match_with_binary_exact_native_costs() {
    let first = NativePlanCost {
        native_two_qubit_ops: 3,
        native_total_ops: 5,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: 0.125,
        }),
        duration: MetricAvailability::Available(RobustDurationKey {
            unavailable_count: 0,
            imputed_count: 0,
            duration_work: 8.0,
        }),
    };
    let second = NativePlanCost {
        native_two_qubit_ops: 2,
        native_total_ops: 4,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 1,
            log_error: 0.25,
        }),
        duration: MetricAvailability::Available(RobustDurationKey {
            unavailable_count: 0,
            imputed_count: 1,
            duration_work: 16.0,
        }),
    };
    let terminal = NativePlanCost {
        native_two_qubit_ops: 1,
        native_total_ops: 1,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: 0.5,
        }),
        duration: MetricAvailability::Available(RobustDurationKey {
            unavailable_count: 0,
            imputed_count: 0,
            duration_work: 4.0,
        }),
    };
    let neighbors = movement_adjacency(
        4,
        &[
            movement_edge(0, 1, first),
            movement_edge(1, 2, second),
            movement_edge(2, 3, first),
        ],
    );
    let terminals = BTreeMap::from([([0, 1], terminal)]);
    let eager = pair_route_lower_bounds(&neighbors, &terminals);

    for left in 0..4 {
        for right in 0..4 {
            if left == right {
                continue;
            }
            assert_eq!(
                pair_route_lower_bound_from_state(&neighbors, &terminals, [left, right]),
                eager.get(left, right),
                "lazy/eager mismatch for ({left}, {right})"
            );
        }
    }
}

#[test]
fn eager_and_lazy_pair_costs_match_when_a_longer_route_has_lower_native_cost() {
    let swap_cost = |native_two_qubit_ops| NativePlanCost {
        native_two_qubit_ops,
        native_total_ops: native_two_qubit_ops,
        ..NativePlanCost::default()
    };
    let neighbors = movement_adjacency(
        3,
        &[
            movement_edge(0, 1, swap_cost(1)),
            movement_edge(0, 2, swap_cost(1)),
            movement_edge(1, 2, swap_cost(3)),
        ],
    );
    let terminals = BTreeMap::from([
        ([0, 1], NativePlanCost::default()),
        ([1, 0], NativePlanCost::default()),
    ]);
    let eager = pair_route_lower_bounds(&neighbors, &terminals);
    let eager_start = eager.get(0, 2).expect("the pair can reach a terminal");
    let lazy_start = pair_route_lower_bound_from_state(&neighbors, &terminals, [0, 2])
        .expect("the pair can reach a terminal");

    assert_eq!(eager_start.native.native_two_qubit_ops, 2);
    assert_eq!(eager_start.remaining_swaps, 2);
    assert_eq!(lazy_start, eager_start);
}

#[test]
fn eager_and_lazy_nonbinary_costs_preserve_route_ordering() {
    let cost = |error, duration| NativePlanCost {
        native_two_qubit_ops: 1,
        native_total_ops: 3,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: error,
        }),
        duration: MetricAvailability::Available(RobustDurationKey {
            unavailable_count: 0,
            imputed_count: 0,
            duration_work: duration,
        }),
    };
    let neighbors = movement_adjacency(
        4,
        &[
            movement_edge(0, 1, cost(0.1, 1.1)),
            movement_edge(1, 2, cost(0.2, 2.2)),
            movement_edge(2, 3, cost(0.3, 3.3)),
        ],
    );
    let terminals = BTreeMap::from([([0, 1], cost(0.4, 4.4))]);
    let eager = pair_route_lower_bounds(&neighbors, &terminals);
    let states = (0..4)
        .flat_map(|left| {
            (0..4)
                .filter(move |right| left != *right)
                .map(move |right| [left, right])
        })
        .collect::<Vec<_>>();
    let lazy = states
        .iter()
        .map(|state| pair_route_lower_bound_from_state(&neighbors, &terminals, *state).unwrap())
        .collect::<Vec<_>>();

    for (index, state) in states.iter().enumerate() {
        assert_eq!(
            eager.get(state[0], state[1]).unwrap().remaining_swaps,
            lazy[index].remaining_swaps
        );
    }
    for left in 0..states.len() {
        for right in 0..states.len() {
            assert_eq!(
                eager
                    .get(states[left][0], states[left][1])
                    .unwrap()
                    .compare(eager.get(states[right][0], states[right][1]).unwrap()),
                lazy[left].compare(lazy[right]),
                "ordering mismatch for {:?} ({:?}/{:?}) and {:?} ({:?}/{:?})",
                states[left],
                eager.get(states[left][0], states[left][1]).unwrap(),
                lazy[left],
                states[right],
                eager.get(states[right][0], states[right][1]).unwrap(),
                lazy[right],
            );
        }
    }
}

#[test]
fn pair_state_table_omits_impossible_diagonal_states() {
    let table = PairStateTable::<RouteLowerBound>::new(100);

    assert_eq!(table.state_count(), 100 * 99);
    assert_eq!(PairStateTable::<()>::index(100, 4, 4), None);
}

#[test]
fn movement_cost_storage_tracks_sparse_edges() {
    let edges = (0..999)
        .map(|left| movement_edge(left, left + 1, NativePlanCost::default()))
        .collect::<Vec<_>>();
    let neighbors = movement_adjacency(1_000, &edges);

    assert_eq!(neighbors.len(), 1_000);
    assert_eq!(neighbors.iter().map(Vec::len).sum::<usize>(), 2 * 999);
}

#[test]
fn topology_only_routing_uses_compact_distances_without_pair_caches() {
    let device = Device::line("lazy-pair-line", 4).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 3)], 4).unwrap();
    let requirement = target
        .interaction_id_for_node(&sabre, sabre.first_layer[0])
        .unwrap();
    let RequirementTable::Pair { terminals, .. } = &target.requirements[requirement] else {
        panic!("CX must use a pair requirement");
    };

    assert!(!terminals.is_empty());
    assert!(!target.neighbors_by_index[0].is_empty());
    assert!(
        pair_route_lower_bound_from_state(&target.neighbors_by_index, terminals, [0, 3]).is_some()
    );
    assert!(
        target
            .route_lower_bound_for_cached(requirement, RequirementPlacement::Pair([0, 3]), None,)
            .is_some()
    );

    let routed = route_trial_for_test(
        &sabre,
        &target,
        &layout,
        &SabreConfig::deterministic_seeded(7).heuristic,
        11,
    )
    .unwrap();
    assert_eq!(target.eager_pair_state_count, 0);
    assert!(routed.swap_count > 0);
    assert_eq!(routed.lazy_pair_l1_lookup_count, 0);
    assert_eq!(routed.lazy_pair_l1_cached_count, 0);
    assert_eq!(shared_lazy_cache_len(&target), 0);
}

#[test]
fn trial_pair_cache_avoids_repeating_shared_lazy_searches() {
    let device = Device::line("trial-pair-cache", 4).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let mut target = RoutingTarget::from_physical(&physical).unwrap();
    target.native_cost_enabled = true;
    let requirement = target
        .interaction_id_for_node(&sabre, sabre.first_layer[0])
        .unwrap();
    let cache = TrialPairCache::default();

    for _ in 0..2 {
        assert!(
            target
                .route_lower_bound_for_cached(
                    requirement,
                    RequirementPlacement::Pair([0, 3]),
                    Some(&cache),
                )
                .is_some()
        );
        assert_eq!(
            target.route_lower_bound_for_cached(
                requirement,
                RequirementPlacement::Pair([0, 0]),
                Some(&cache),
            ),
            None
        );
    }

    let stats = cache.stats();
    assert_eq!(
        (stats.lookup_count, stats.hit_count, stats.cached_count),
        (4, 2, 2)
    );
    assert_eq!(shared_lazy_cache_len(&target), 2);
}

#[test]
fn lazy_pair_single_flight_publishes_reachable_and_unreachable_results_once() {
    for expected in [
        Some(RouteLowerBound {
            remaining_swaps: 3,
            native: NativePlanCost::default(),
        }),
        None,
    ] {
        let cache = Arc::new(Mutex::new(LazyPairCache::default()));
        let computations = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(9));
        let threads = (0..8)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let computations = Arc::clone(&computations);
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    lazy_pair_lookup_or_compute(&cache, (7, 2, 5), || {
                        computations.fetch_add(1, AtomicOrdering::SeqCst);
                        thread::sleep(Duration::from_millis(25));
                        expected
                    })
                })
            })
            .collect::<Vec<_>>();
        start.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();

        assert!(results.into_iter().all(|result| result == expected));
        assert_eq!(computations.load(AtomicOrdering::SeqCst), 1);
        let cache = cache.lock().unwrap();
        assert_eq!(cache.values.get(&(7, 2, 5)).copied(), Some(expected));
        assert!(cache.flights.is_empty());
    }
}

#[test]
fn lazy_pair_single_flight_recovers_after_the_computing_thread_panics() {
    let cache = Arc::new(Mutex::new(LazyPairCache::default()));
    let computations = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = mpsc::sync_channel(0);
    let (release_tx, release_rx) = mpsc::sync_channel(0);

    let panicking = {
        let cache = Arc::clone(&cache);
        let computations = Arc::clone(&computations);
        thread::spawn(move || {
            std::panic::catch_unwind(|| {
                lazy_pair_lookup_or_compute(&cache, (11, 1, 4), || {
                    computations.fetch_add(1, AtomicOrdering::SeqCst);
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    panic!("intentional single-flight abort")
                })
            })
        })
    };
    started_rx.recv().unwrap();

    let expected = Some(RouteLowerBound {
        remaining_swaps: 2,
        native: NativePlanCost::default(),
    });
    let (result_tx, result_rx) = mpsc::sync_channel(0);
    let waiter = {
        let cache = Arc::clone(&cache);
        let computations = Arc::clone(&computations);
        thread::spawn(move || {
            let result = lazy_pair_lookup_or_compute(&cache, (11, 1, 4), || {
                computations.fetch_add(1, AtomicOrdering::SeqCst);
                expected
            });
            result_tx.send(result).unwrap();
        })
    };

    release_tx.send(()).unwrap();
    assert!(panicking.join().unwrap().is_err());
    assert_eq!(
        result_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        expected
    );
    waiter.join().unwrap();
    assert_eq!(computations.load(AtomicOrdering::SeqCst), 2);
    assert!(cache.lock().unwrap().flights.is_empty());
}

#[test]
fn lazy_pair_cache_does_not_change_seeded_results_across_thread_counts() {
    let device = Device::line("lazy-pair-parallel", 6).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 5), (2, 2), (3, 4)], 6).unwrap();
    let mut circuit = Circuit::new(4);
    for (left, right) in [(0, 1), (2, 3), (0, 3), (1, 2), (0, 2), (1, 3)] {
        circuit.cx(Qubit::new(left), Qubit::new(right)).unwrap();
    }
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let seeds = [3_u64, 5, 7, 11, 13, 17, 19, 23];
    let heuristic = SabreHeuristicConfig {
        lookahead_weights: vec![0.5, 0.25],
        decay_increment: Some(0.01),
        ..SabreHeuristicConfig::default()
    };
    let route_in_pool = |threads| {
        let target = RoutingTarget::from_physical(&physical).unwrap();
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                seeds
                    .into_par_iter()
                    .map(|seed| {
                        let result =
                            route_trial_for_test(&sabre, &target, &layout, &heuristic, seed)
                                .unwrap();
                        (
                            result.swap_count,
                            result.final_layout,
                            result.quality,
                            format!("{:?}", result.operations),
                        )
                    })
                    .collect::<Vec<_>>()
            })
    };

    let single_threaded = route_in_pool(1);
    let four_threaded = route_in_pool(4);

    assert_eq!(single_threaded, four_threaded);
}

#[test]
fn component_relations_come_from_terminals_without_lazy_pair_search() {
    let qubits = (0..4).map(PhysicalQubit::new).collect::<Vec<_>>();
    let [p0, p1, p2, p3] = [qubits[0], qubits[1], qubits[2], qubits[3]];
    let topology = Topology::new(
        qubits.clone(),
        vec![
            (p0, p2, "swap-a".to_string()),
            (p1, p3, "swap-b".to_string()),
            (p0, p1, "cx-cross".to_string()),
        ],
    )
    .unwrap();
    let mut device = Device::new(
        "terminal-components",
        qubits.iter().copied().collect(),
        topology,
    )
    .unwrap();
    for (left, right) in [(p0, p2), (p1, p3)] {
        device
            .add_edge_properties(
                left,
                right,
                EdgeProp::new()
                    .with_native_instruction(InstructionProp::new(
                        Instruction::Standard(StandardGate::SWAP),
                        0.01,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }
    device
        .add_edge_properties(
            p0,
            p1,
            EdgeProp::new()
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::CX),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let planning_session = DevicePlanningSession::new(&device);
    let target = RoutingTarget::from_device_with_pair_state_budget_and_session(
        &device,
        &physical,
        &sabre,
        0,
        &planning_session,
    )
    .unwrap();

    let assignment = movement_component_assignment(
        &sabre,
        &target,
        &[LogicalQubit::new(0), LogicalQubit::new(1)],
        100,
    )
    .unwrap();
    assert!(matches!(assignment, ComponentAssignmentSearch::Found(_)));
    assert_eq!(shared_lazy_cache_len(&target), 0);
}

#[test]
fn structural_native_scoring_descends_into_control_flow() {
    let p0 = PhysicalQubit::new(0);
    let mut device = Device::line("nested-native-validation", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::SWAP)])
        .unwrap();
    device
        .add_qubit_properties(
            p0,
            crate::device::QubitProp::new(0.0)
                .with_native_instruction(InstructionProp::new(
                    Instruction::Standard(StandardGate::H),
                    0.01,
                ))
                .unwrap(),
        )
        .unwrap();
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(Qubit::new(0))
        })
        .unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut target = routing_target_from_device(&device, &physical, &sabre).unwrap();
    target
        .native_plans
        .remove(&DeviceGateState::standard(StandardGate::H, smallvec![p0]));

    let error = native_plan_cost_for_operations(circuit.operations(), &target).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvariantViolation(message) if message.contains("was not prepared"))
    );
}

#[test]
fn mapping_cycle_detection_verifies_a_repeated_layout() {
    let device = Device::line("mapping-cycle", 3).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let layout = Layout::from_pairs(&[(0, 0), (1, 1)], 3).unwrap();
    let mut dense = DenseRoutingLayout::from_layout(&layout, &target.physical_qubits).unwrap();
    let initial_hash = MappingCycleDetector::mapping_hash(dense.signature());
    let mut detector = MappingCycleDetector::new(initial_hash);

    let before = [dense.signature()[0], dense.signature()[1]];
    dense.swap_physical_indices(0, 1);
    let swapped_hash =
        MappingCycleDetector::hash_after_swap(initial_hash, [0, 1], before[0], before[1]);
    assert_eq!(
        swapped_hash,
        MappingCycleDetector::mapping_hash(dense.signature())
    );
    assert!(!detector.record_hash(swapped_hash));

    let before = [dense.signature()[0], dense.signature()[1]];
    dense.swap_physical_indices(0, 1);
    let restored_hash =
        MappingCycleDetector::hash_after_swap(swapped_hash, [0, 1], before[0], before[1]);
    assert_eq!(restored_hash, initial_hash);
    assert!(detector.record_hash(restored_hash));
}

#[test]
fn cost_to_go_charges_terminal_cost_for_every_reachable_state() {
    let neighbors = default_movement_adjacency(3, &[(0, 1), (1, 2)]);
    let terminal = NativePlanCost {
        native_two_qubit_ops: 5,
        native_total_ops: 7,
        error: MetricAvailability::Disabled,
        duration: MetricAvailability::Disabled,
    };
    let terminals = vec![None, None, Some(terminal)];
    let bounds = unary_route_lower_bounds(&neighbors, &terminals);

    assert_eq!(bounds[2].unwrap().remaining_swaps, 0);
    assert_eq!(bounds[1].unwrap().remaining_swaps, 1);
    assert_eq!(bounds[0].unwrap().remaining_swaps, 2);
    assert!(bounds.iter().flatten().all(|bound| {
        bound.native.native_two_qubit_ops == terminal.native_two_qubit_ops
            && bound.native.native_total_ops == terminal.native_total_ops
    }));
}

#[test]
fn cost_to_go_preserves_enabled_calibration_through_zero_cost_terminal() {
    let terminal = NativePlanCost {
        error: MetricAvailability::Available(RobustErrorKey::default()),
        duration: MetricAvailability::Available(RobustDurationKey::default()),
        ..NativePlanCost::default()
    };
    let swap = NativePlanCost {
        native_two_qubit_ops: 3,
        native_total_ops: 7,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: 0.125,
        }),
        duration: MetricAvailability::Available(RobustDurationKey {
            unavailable_count: 0,
            imputed_count: 0,
            duration_work: 40.0,
        }),
    };
    let terminals = vec![None, Some(terminal)];
    let neighbors = movement_adjacency(2, &[movement_edge(0, 1, swap)]);

    let bounds = unary_route_lower_bounds(&neighbors, &terminals);
    let routed = bounds[0].expect("the calibrated SWAP reaches the terminal");

    assert_eq!(routed.remaining_swaps, 1);
    assert_eq!(routed.native, swap);
}

fn default_movement_adjacency(
    count: usize,
    edges: &[(usize, usize)],
) -> Vec<Vec<MovementNeighbor>> {
    let edges = edges
        .iter()
        .copied()
        .map(|(left, right)| movement_edge(left, right, NativePlanCost::default()))
        .collect::<Vec<_>>();
    movement_adjacency(count, &edges)
}

#[test]
fn native_cost_comparison_uses_only_structural_two_qubit_count() {
    let fewer_gates_with_unknown_error = NativePlanCost {
        native_two_qubit_ops: 2,
        native_total_ops: 8,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 1,
            imputed_count: 0,
            log_error: 0.0,
        }),
        duration: MetricAvailability::Disabled,
    };
    let more_gates_with_known_error = NativePlanCost {
        native_two_qubit_ops: 3,
        native_total_ops: 3,
        error: MetricAvailability::Available(RobustErrorKey {
            unavailable_count: 0,
            imputed_count: 0,
            log_error: 0.01,
        }),
        duration: MetricAvailability::Disabled,
    };
    let equal_gates_with_known_error = NativePlanCost {
        native_two_qubit_ops: 2,
        ..more_gates_with_known_error
    };

    assert_eq!(
        compare_optional_native_cost(
            Some(fewer_gates_with_unknown_error),
            Some(more_gates_with_known_error)
        ),
        Ordering::Less
    );
    assert_eq!(
        compare_optional_native_cost(
            Some(equal_gates_with_known_error),
            Some(fewer_gates_with_unknown_error)
        ),
        Ordering::Equal
    );
    assert_eq!(
        compare_optional_native_cost(Some(equal_gates_with_known_error), None),
        Ordering::Less
    );
}

#[test]
fn exclusive_control_flow_chooses_one_coherent_worst_structural_path() {
    let low_path = ExecutionPathCost {
        native_two_qubit_ops: 2,
        native_total_ops: 8,
        ..ExecutionPathCost::default()
    };
    let high_path = ExecutionPathCost {
        native_two_qubit_ops: 3,
        native_total_ops: 5,
        ..ExecutionPathCost::default()
    };

    assert_eq!(low_path.worse(high_path), high_path);
}

#[test]
fn execution_resources_use_the_worst_exclusive_branch() {
    let then_cost = NativeCircuitCost {
        path: ExecutionPathCost {
            native_two_qubit_ops: 2,
            native_total_ops: 8,
            ..ExecutionPathCost::default()
        },
        ..NativeCircuitCost::default()
    };
    let else_cost = NativeCircuitCost {
        path: ExecutionPathCost {
            native_two_qubit_ops: 3,
            native_total_ops: 5,
            ..ExecutionPathCost::default()
        },
        ..NativeCircuitCost::default()
    };
    let mut total = NativeCircuitCost {
        path: ExecutionPathCost {
            native_two_qubit_ops: 1,
            native_total_ops: 1,
            ..ExecutionPathCost::default()
        },
        ..NativeCircuitCost::default()
    };

    total.append_worst_branch(then_cost, else_cost);

    assert_eq!(total.path.native_two_qubit_ops, 4);
    assert_eq!(total.path.native_total_ops, 6);
}

#[test]
fn fixed_for_loop_multiplies_execution_path_two_qubit_depth() {
    let mut circuit = Circuit::new(2);
    let loop_var = circuit.var(ClassicalType::uint(3).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(3, 1).unwrap(),
            ClassicalExpr::uint_literal(3, 7).unwrap(),
            ClassicalExpr::uint_literal(3, 2).unwrap(),
            |body, _| {
                body.cx(Qubit::new(0), Qubit::new(1))?;
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(two_qubit_depth(circuit.operations()), 3);
}

#[test]
fn abstract_swap_depth_uses_its_two_qubit_decomposition_weight() {
    let mut circuit = Circuit::new(3);
    circuit.swap(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    assert_eq!(two_qubit_depth(circuit.operations()), 4);
}

fn ranked_trial_for_test(
    native_two_qubit_ops: usize,
    native_two_qubit_depth: usize,
    native_total_depth: usize,
) -> RankedTrial {
    RankedTrial {
        trial: UnscoredTrial {
            plan: CompactRoutePlan::default(),
            final_layout: Layout::default(),
            swap_count: 0,
            fallback_count: 0,
            control_flow_blocks_routed: 0,
            lazy_pair_l1_lookup_count: 0,
            lazy_pair_l1_hit_count: 0,
            lazy_pair_l1_cached_count: 0,
            native_cost: NativeCircuitCost::default(),
            two_qubit_operation_count: 0,
            operation_count: 0,
        },
        abstract_quality: AbstractTrialQuality::default(),
        native_two_qubit_ops,
        native_total_ops: 0,
        unknown_loop_count: 0,
        native_two_qubit_depth: Some(native_two_qubit_depth),
        native_total_depth: Some(native_total_depth),
        materialized_operations: None,
    }
}

#[test]
fn trial_quality_uses_native_count_depth_and_total_depth_in_declared_order() {
    let device = Device::line("ranked-trial-comparison", 2).unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let target = RoutingTarget::from_physical(&physical).unwrap();
    let mut baseline = ranked_trial_for_test(10, 5, 12);
    let mut fewer_native_gates = ranked_trial_for_test(9, 100, 100);
    let mut shallower_native_depth = ranked_trial_for_test(10, 4, 100);
    let mut shallower_total_depth = ranked_trial_for_test(10, 5, 11);
    let mut tied = ranked_trial_for_test(10, 5, 12);

    assert!(
        compare_ranked_trials(
            &mut fewer_native_gates,
            (0, 1),
            &mut baseline,
            (0, 0),
            &target,
        )
        .unwrap()
        .is_lt()
    );
    assert!(
        compare_ranked_trials(
            &mut shallower_native_depth,
            (0, 1),
            &mut baseline,
            (0, 0),
            &target,
        )
        .unwrap()
        .is_lt()
    );
    assert!(
        compare_ranked_trials(
            &mut shallower_total_depth,
            (0, 1),
            &mut baseline,
            (0, 0),
            &target,
        )
        .unwrap()
        .is_lt()
    );
    assert_eq!(
        compare_ranked_trials(&mut baseline, (0, 0), &mut tied, (0, 1), &target).unwrap(),
        Ordering::Less
    );
}

#[test]
fn routing_trials_share_one_fixed_heuristic_configuration() {
    let base = SabreHeuristicConfig {
        lookahead_weights: vec![0.5, 0.25],
        decay_increment: Some(0.01),
        ..SabreHeuristicConfig::default()
    };
    let config = SabreConfig {
        routing_trials: 4,
        heuristic: base.clone(),
        ..SabreConfig::default()
    };

    assert_eq!(config.heuristic, base);
    assert_eq!(base.lookahead_weights, vec![0.5, 0.25]);
    assert_eq!(base.decay_increment, Some(0.01));
}

#[test]
fn device_target_prepares_both_orderings_of_emitted_swap() {
    let device = Device::line("ordered-swap", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let physical = PhysicalLayoutGraph::from_device(&device).unwrap();
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let sabre = SabreDag::from_operations(circuit.operations()).unwrap();
    let target = routing_target_from_device(&device, &physical, &sabre).unwrap();
    let reversed = DeviceGateState::standard(
        StandardGate::SWAP,
        smallvec![PhysicalQubit::new(1), PhysicalQubit::new(0)],
    );

    assert!(target.native_cost(&reversed).is_some());
}
