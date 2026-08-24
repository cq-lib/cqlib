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
use crate::circuit::{Circuit, Qubit};
use crate::compile::CompilerError;
use crate::compile::sabre::SabreConfig;
use crate::compile::test_utils::{
    assert_two_qubit_operations_supported_by_topology, generated_small_routable_circuit,
};
use crate::device::{Device, Layout, LogicalQubit, PhysicalQubit, Topology};
use proptest::prelude::*;
use std::collections::{BTreeMap, HashSet};

#[test]
fn sabre_routing_auto_layout_routes_non_embeddable_interactions() {
    let device = Device::line("line", 3).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert!(result.changed(&circuit));
    assert!(result.swap_count() > 0);
    assert_eq!(result.circuit().qubits().len(), 3);
    assert_all_two_qubit_operations_are_adjacent_on_line(result.circuit());
    assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
}

#[test]
fn sabre_routing_keeps_adjacent_two_qubit_circuit_without_swap() {
    let device = Device::line("line", 2).unwrap();
    let objective = LayoutObjective::topology_only();
    let mut config = SabreConfig::deterministic_seeded(7);
    config.layout_trials = 8;
    config.routing_trials = 4;
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert!(!result.changed(&circuit));
    assert_eq!(result.swap_count(), 0);
    assert_eq!(result.diagnostics().trials_evaluated, 1);
    assert_eq!(result.layout_diagnostics().candidates_evaluated, 1);
    assert_eq!(result.circuit().operations().len(), 1);
}

#[test]
fn sabre_routing_keeps_parameterized_single_qubit_circuit_unchanged() {
    let device = Device::line("line", 1).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(1);
    circuit.rx(Qubit::new(0), 0.25).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert!(!result.changed(&circuit));
    assert_eq!(result.swap_count(), 0);
    assert_eq!(result.circuit().operations().len(), 1);
}

#[test]
fn sabre_routing_keeps_empty_and_interaction_free_circuits_topology_valid() {
    let device = Device::star("star", 5, 0).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let empty = Circuit::new(0);
    let mut single_qubit_layers = Circuit::new(5);
    for index in 0..5 {
        single_qubit_layers.h(Qubit::new(index)).unwrap();
        single_qubit_layers
            .rz(Qubit::new(index), index as f64 * 0.125)
            .unwrap();
    }

    for circuit in [&empty, &single_qubit_layers] {
        let result = route_sabre(circuit, &device, &objective, &config).unwrap();

        assert_eq!(result.swap_count(), 0);
        assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
    }
}

#[test]
fn sabre_routing_outputs_only_supported_edges_on_star_device() {
    let device = Device::star("star", 5, 0).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(17);
    let mut circuit = Circuit::new(5);
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(3), Qubit::new(4)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(4)).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert!(result.changed(&circuit));
    assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
}

#[test]
fn sabre_routing_preserves_adjacent_line_circuit_without_swaps() {
    let device = Device::line("line", 4).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(29);
    let mut circuit = Circuit::new(4);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.swap(Qubit::new(2), Qubit::new(3)).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert_eq!(result.swap_count(), 0);
    assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn sabre_routed_generated_circuits_use_only_supported_topology_edges(
        circuit in generated_small_routable_circuit()
    ) {
        let device = Device::line("property-line", 5).unwrap();
        let objective = LayoutObjective::topology_only();
        let config = SabreConfig::deterministic_seeded(31);

        let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

        assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
    }
}

#[test]
fn sabre_identity_no_swap_rebuild_reports_unchanged_without_panicking() {
    let device = Device::ring("regression-ring", 5).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(31);
    let mut circuit = Circuit::new(5);
    circuit.crx(Qubit::new(4), Qubit::new(0), 0.0).unwrap();
    circuit.i(Qubit::new(1)).unwrap();

    let result = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert!(!result.changed(&circuit));
    assert_eq!(
        format!("{:?}", result.circuit().operations()),
        format!("{:?}", circuit.operations())
    );
    assert_two_qubit_operations_supported_by_topology(result.circuit(), device.topology());
}

#[test]
fn sabre_changed_detects_non_identity_layout_without_swaps() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let layout = Layout::new(
        vec![LogicalQubit::new(0), LogicalQubit::new(1)],
        vec![PhysicalQubit::new(0), PhysicalQubit::new(1)],
        Some(BTreeMap::from([
            (LogicalQubit::new(0), PhysicalQubit::new(1)),
            (LogicalQubit::new(1), PhysicalQubit::new(0)),
        ])),
    )
    .unwrap();

    let routed = RoutedCircuit {
        circuit: circuit.clone(),
        initial_layout: layout.clone(),
        final_layout: layout,
        swap_count: 0,
        diagnostics: Default::default(),
    };
    assert!(routed.changed(&circuit));
}

#[test]
fn routed_circuit_equality_is_structural() {
    let circuit = Circuit::new(1);
    let layout = Layout::from_pairs(&[(0, 0)], 1).unwrap();
    let first = RoutedCircuit {
        circuit: circuit.clone(),
        initial_layout: layout.clone(),
        final_layout: layout.clone(),
        swap_count: 0,
        diagnostics: Default::default(),
    };
    let second = RoutedCircuit {
        circuit,
        initial_layout: layout.clone(),
        final_layout: layout,
        swap_count: 0,
        diagnostics: Default::default(),
    };

    assert_eq!(first, second);
}

#[test]
fn sabre_routing_is_reproducible_for_same_seed() {
    let device = Device::line("line", 4).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let first = route_sabre(&circuit, &device, &objective, &config).unwrap();
    let second = route_sabre(&circuit, &device, &objective, &config).unwrap();

    assert_eq!(
        first.initial_layout().l2p_map(),
        second.initial_layout().l2p_map()
    );
    assert_eq!(
        first.final_layout().l2p_map(),
        second.final_layout().l2p_map()
    );
    assert_eq!(first.swap_count(), second.swap_count());
    assert_eq!(
        first.diagnostics().operation_count,
        second.diagnostics().operation_count
    );
    assert_eq!(
        first.layout_score().map(|score| score.total),
        second.layout_score().map(|score| score.total)
    );
}

#[test]
fn sabre_routing_rejects_invalid_config() {
    let device = Device::line("line", 2).unwrap();
    let objective = LayoutObjective::topology_only();
    let mut config = SabreConfig::deterministic_seeded(7);
    config.routing_trials = 0;
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();

    let error = route_sabre(&circuit, &device, &objective, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("routing_trials"))
    );
}

#[test]
fn sabre_routing_rejects_insufficient_physical_qubits() {
    let p0 = PhysicalQubit::new(0);
    let topology = Topology::new(vec![p0], vec![]).unwrap();
    let device = Device::new("one", HashSet::from_iter([p0]), topology).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let circuit = Circuit::new(2);

    let error = route_sabre(&circuit, &device, &objective, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("2 logical qubits") && message.contains("1 usable physical qubits"))
    );
}

#[test]
fn sabre_routing_rejects_undecomposed_three_qubit_gate() {
    let device = Device::line("line", 3).unwrap();
    let objective = LayoutObjective::topology_only();
    let config = SabreConfig::deterministic_seeded(7);
    let mut circuit = Circuit::new(3);
    circuit
        .ccx(Qubit::new(0), Qubit::new(1), Qubit::new(2))
        .unwrap();

    let error = route_sabre(&circuit, &device, &objective, &config).unwrap_err();

    assert!(
        matches!(error, CompilerError::InvalidInput(message) if message.contains("more than two qubits"))
    );
}

fn assert_all_two_qubit_operations_are_adjacent_on_line(circuit: &Circuit) {
    for operation in circuit.operations() {
        if operation.qubits.len() == 2 {
            assert!(
                operation.qubits[0].id().abs_diff(operation.qubits[1].id()) == 1,
                "operation {operation:?} is not adjacent on line topology"
            );
        }
    }
}
