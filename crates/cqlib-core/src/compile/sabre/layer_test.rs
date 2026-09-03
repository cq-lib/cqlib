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

fn line_distance(
    _requirement: usize,
    placement: RequirementPlacement,
) -> Result<f64, CompilerError> {
    Ok(match placement {
        RequirementPlacement::Unary(physical) => physical as f64,
        RequirementPlacement::Pair([left, right]) => left.abs_diff(right) as f64,
    })
}

fn interaction_distance(
    interaction: usize,
    placement: RequirementPlacement,
) -> Result<f64, CompilerError> {
    let distance = match placement {
        RequirementPlacement::Unary(physical) => physical,
        RequirementPlacement::Pair([left, right]) => left.abs_diff(right),
    };
    Ok((interaction * 10 + distance) as f64)
}

#[test]
fn total_score_after_swap_updates_all_affected_nodes() {
    let mut layer = Layer::new(2, 5);
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 4]),
            &line_distance,
        )
        .unwrap();
    layer
        .insert(
            NodeIndex::new(1),
            0,
            RequirementPlacement::Pair([1, 3]),
            &line_distance,
        )
        .unwrap();

    // After SWAP(0, 1), both interaction distances are 3.
    assert_eq!(
        layer
            .total_score_after_swap([0, 1], &line_distance)
            .unwrap(),
        6.0
    );
}

#[test]
fn apply_swap_updates_cached_score_and_ordered_interaction() {
    let mut layer = Layer::new(1, 4);
    layer
        .insert(
            NodeIndex::new(0),
            7,
            RequirementPlacement::Pair([0, 3]),
            &interaction_distance,
        )
        .unwrap();

    assert_eq!(
        layer
            .total_score_after_swap([0, 1], &interaction_distance)
            .unwrap(),
        72.0
    );
    layer.apply_swap([0, 1], &interaction_distance).unwrap();

    assert_eq!(
        layer.iter().collect::<Vec<_>>(),
        vec![(NodeIndex::new(0), 7, RequirementPlacement::Pair([1, 3]))]
    );
    assert_eq!(
        layer
            .total_score_after_swap([2, 3], &interaction_distance)
            .unwrap(),
        71.0
    );
}

#[test]
fn swapping_both_endpoints_of_one_node_updates_it_once() {
    let mut layer = Layer::new(1, 2);
    layer
        .insert(
            NodeIndex::new(0),
            3,
            RequirementPlacement::Pair([0, 1]),
            &interaction_distance,
        )
        .unwrap();

    layer.apply_swap([0, 1], &interaction_distance).unwrap();

    assert_eq!(
        layer.iter().collect::<Vec<_>>(),
        vec![(NodeIndex::new(0), 3, RequirementPlacement::Pair([1, 0]))]
    );
    assert_eq!(
        layer
            .total_score_after_swap([0, 1], &interaction_distance)
            .unwrap(),
        31.0
    );
}

#[test]
fn replacement_removes_stale_active_endpoints_and_score() {
    let mut layer = Layer::new(1, 4);
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 3]),
            &line_distance,
        )
        .unwrap();
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([1, 3]),
            &line_distance,
        )
        .unwrap();

    assert_eq!(layer.active_indices().collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(
        layer
            .total_score_after_swap([0, 2], &line_distance)
            .unwrap(),
        2.0
    );
}

#[test]
fn routability_uses_the_nodes_interaction_identity() {
    let mut layer = Layer::new(1, 3);
    layer
        .insert(
            NodeIndex::new(0),
            9,
            RequirementPlacement::Pair([0, 2]),
            &line_distance,
        )
        .unwrap();

    assert_eq!(
        layer.routable_node_on_index(0, &|interaction, placement| {
            interaction == 9 && placement == RequirementPlacement::Pair([0, 2])
        }),
        Some(NodeIndex::new(0))
    );
    assert_eq!(
        layer.routable_node_on_index(2, &|interaction, placement| {
            interaction == 9 && placement == RequirementPlacement::Pair([0, 2])
        }),
        Some(NodeIndex::new(0))
    );
}

#[test]
fn remove_and_clear_restore_empty_layer_invariants() {
    let mut layer = Layer::new(2, 4);
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 1]),
            &line_distance,
        )
        .unwrap();
    layer.remove(NodeIndex::new(7), &line_distance).unwrap();
    layer.remove(NodeIndex::new(0), &line_distance).unwrap();
    assert!(layer.is_empty());

    layer
        .insert(
            NodeIndex::new(1),
            0,
            RequirementPlacement::Pair([2, 3]),
            &line_distance,
        )
        .unwrap();
    layer.clear();
    assert!(layer.is_empty());
    assert!(layer.iter().next().is_none());
    assert!(layer.active_indices().next().is_none());
    assert_eq!(layer.routable_node_on_index(2, &|_, _| true), None);
    assert_eq!(layer.routable_node_on_index(3, &|_, _| true), None);
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([2, 3]),
            &line_distance,
        )
        .unwrap();
    assert_eq!(
        layer.routable_node_on_index(2, &|_, _| true),
        Some(NodeIndex::new(0))
    );
}

#[test]
fn empty_layer_total_is_zero() {
    let layer = Layer::new(0, 2);
    assert_eq!(
        layer
            .total_score_after_swap([0, 1], &line_distance)
            .unwrap(),
        0.0
    );
}

#[test]
fn inserting_shared_active_endpoint_is_an_invariant_error() {
    let mut layer = Layer::new(2, 3);
    layer
        .insert(
            NodeIndex::new(0),
            0,
            RequirementPlacement::Pair([0, 1]),
            &line_distance,
        )
        .unwrap();

    let error = layer
        .insert(
            NodeIndex::new(1),
            0,
            RequirementPlacement::Pair([1, 2]),
            &line_distance,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CompilerError::InvariantViolation(message)
            if message.contains("share physical endpoint 1")
    ));
}

#[test]
fn unary_placement_uses_one_active_endpoint_and_moves_independently() {
    let mut layer = Layer::new(1, 4);
    layer
        .insert(
            NodeIndex::new(0),
            4,
            RequirementPlacement::Unary(1),
            &line_distance,
        )
        .unwrap();

    assert_eq!(layer.active_indices().collect::<Vec<_>>(), vec![1]);
    layer.apply_swap([1, 2], &line_distance).unwrap();
    assert_eq!(
        layer.iter().collect::<Vec<_>>(),
        vec![(NodeIndex::new(0), 4, RequirementPlacement::Unary(2))]
    );
}
