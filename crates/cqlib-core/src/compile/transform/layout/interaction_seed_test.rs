// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0.
// You may obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

use super::*;
use crate::circuit::{Circuit, Qubit};
use crate::compile::transform::layout::analyze_circuit_for_layout;
use crate::device::Device;

#[test]
fn path_motif_finds_a_zero_swap_grid_embedding_without_device_tables() {
    let mut circuit = Circuit::new(16);
    for index in 0..15 {
        circuit
            .cx(Qubit::new(index), Qubit::new(index + 1))
            .unwrap();
    }
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    let physical =
        PhysicalLayoutGraph::from_device(&Device::grid("generic-grid", 4, 4).unwrap()).unwrap();

    let candidates = interaction_aware_layouts(&analysis, &physical, 6).unwrap();

    assert!(candidates.iter().any(|layout| {
        analysis
            .interactions
            .interactions()
            .iter()
            .all(|interaction| {
                physical.is_adjacent_undirected(
                    layout.get_physical(interaction.left).unwrap(),
                    layout.get_physical(interaction.right).unwrap(),
                )
            })
    }));
}

#[test]
fn cycle_motif_finds_a_closed_grid_embedding() {
    let mut circuit = Circuit::new(16);
    for index in 0..16 {
        circuit
            .cx(Qubit::new(index), Qubit::new((index + 1) % 16))
            .unwrap();
    }
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    let physical =
        PhysicalLayoutGraph::from_device(&Device::grid("generic-grid", 4, 4).unwrap()).unwrap();

    let candidates = interaction_aware_layouts(&analysis, &physical, 6).unwrap();

    assert!(candidates.iter().any(|layout| {
        analysis
            .interactions
            .interactions()
            .iter()
            .all(|interaction| {
                physical.is_adjacent_undirected(
                    layout.get_physical(interaction.left).unwrap(),
                    layout.get_physical(interaction.right).unwrap(),
                )
            })
    }));
}

#[test]
fn demand_graph_values_early_interactions_more_than_late_interactions() {
    let mut circuit = Circuit::new(3);
    for _ in 0..3 {
        circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    }
    for _ in 0..12 {
        circuit.h(Qubit::new(0)).unwrap();
    }
    for _ in 0..3 {
        circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    }
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    let demand = DemandGraph::new(&analysis);
    let early = demand
        .edges
        .iter()
        .find(|edge| [edge.left, edge.right] == [0, 1])
        .unwrap();
    let late = demand
        .edges
        .iter()
        .find(|edge| [edge.left, edge.right] == [1, 2])
        .unwrap();

    assert!(early.weight > late.weight);
}

#[test]
fn local_improvement_can_move_into_a_vacant_physical_qubit() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let analysis = analyze_circuit_for_layout(&circuit).unwrap();
    let demand = DemandGraph::new(&analysis);
    let physical =
        PhysicalLayoutGraph::from_device(&Device::line("vacancy-line", 5).unwrap()).unwrap();
    let mut mapping = vec![0, 4];
    let before = demand.cost(&mapping, &physical);

    improve_mapping(&mut mapping, &demand, &physical);

    assert!(demand.cost(&mapping, &physical) < before);
    assert_eq!(physical.distance_by_index(mapping[0], mapping[1]), Some(1));
    assert!(mapping.contains(&1) || mapping.contains(&3));
}
