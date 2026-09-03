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
use crate::circuit::{Circuit, ClassicalExpr, Qubit};

fn interaction_nodes(dag: &SabreDag) -> Vec<NodeIndex> {
    dag.graph
        .node_indices()
        .filter(|&node| matches!(dag.graph[node].kind, SabreNodeKind::TwoQ(_)))
        .collect()
}

fn unary_nodes(dag: &SabreDag) -> Vec<NodeIndex> {
    dag.graph
        .node_indices()
        .filter(|&node| matches!(dag.graph[node].kind, SabreNodeKind::Unary(_)))
        .collect()
}

#[test]
fn refinement_orders_alternative_branches_that_share_a_wire() {
    let mut circuit = Circuit::new(3);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |then_body| {
                then_body.cx(Qubit::new(0), Qubit::new(1))?;
                Ok(())
            },
            |else_body| {
                else_body.cx(Qubit::new(0), Qubit::new(2))?;
                Ok(())
            },
        )
        .unwrap();

    let dag = SabreDag::refinement_workload(circuit.operations()).unwrap();
    let interactions = interaction_nodes(&dag);

    assert_eq!(interactions.len(), 2);
    assert!(
        dag.graph
            .find_edge(interactions[0], interactions[1])
            .is_some(),
        "stable branch order must serialize interactions sharing q0"
    );
}

#[test]
fn refinement_keeps_disjoint_alternative_branches_parallel() {
    let mut circuit = Circuit::new(4);
    circuit
        .if_else(
            ClassicalExpr::bool_literal(true),
            |then_body| {
                then_body.cx(Qubit::new(0), Qubit::new(1))?;
                Ok(())
            },
            |else_body| {
                else_body.cx(Qubit::new(2), Qubit::new(3))?;
                Ok(())
            },
        )
        .unwrap();

    let dag = SabreDag::refinement_workload(circuit.operations()).unwrap();
    let interactions = interaction_nodes(&dag);

    assert_eq!(interactions.len(), 2);
    assert!(
        dag.graph
            .find_edge(interactions[0], interactions[1])
            .is_none()
    );
    assert!(
        dag.graph
            .find_edge(interactions[1], interactions[0])
            .is_none()
    );
}

#[test]
fn refinement_recursively_includes_nested_body_interactions() {
    let mut circuit = Circuit::new(3);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.if_(ClassicalExpr::bool_literal(true), |nested| {
                nested.cx(Qubit::new(0), Qubit::new(2))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let dag = SabreDag::refinement_workload(circuit.operations()).unwrap();
    assert_eq!(interaction_nodes(&dag).len(), 1);
}

#[test]
fn refinement_preserves_explicit_wire_barrier_dependencies() {
    let mut circuit = Circuit::new(4);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.barrier(vec![Qubit::new(0), Qubit::new(2)]).unwrap();
    circuit.cx(Qubit::new(2), Qubit::new(3)).unwrap();

    let dag = SabreDag::refinement_workload(circuit.operations()).unwrap();
    let interactions = interaction_nodes(&dag);
    let barrier = dag
        .graph
        .node_indices()
        .find(|&node| {
            matches!(dag.graph[node].kind, SabreNodeKind::Synchronize)
                && dag.graph.find_edge(interactions[0], node).is_some()
                && dag.graph.find_edge(node, interactions[1]).is_some()
        })
        .expect("wire barrier must synchronize the two interaction frontiers");

    assert!(matches!(
        dag.graph[barrier].kind,
        SabreNodeKind::Synchronize
    ));
}

#[test]
fn routing_dag_folds_consecutive_operations_on_the_same_pair() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cz(Qubit::new(0), Qubit::new(1)).unwrap();

    let dag = SabreDag::from_operations(circuit.operations()).unwrap();
    let interactions = interaction_nodes(&dag);

    assert_eq!(interactions.len(), 1);
    assert_eq!(dag.graph[interactions[0]].operations.len(), 2);
}

#[test]
fn routing_dag_keeps_overlapping_different_pairs_separate() {
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();

    let dag = SabreDag::from_operations(circuit.operations()).unwrap();
    let interactions = interaction_nodes(&dag);

    assert_eq!(interactions.len(), 2);
    assert!(
        dag.graph
            .find_edge(interactions[0], interactions[1])
            .is_some()
    );
}

#[test]
fn routing_dag_folds_only_consecutive_unary_operations_on_the_same_wire() {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.x(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(1)).unwrap();

    let dag = SabreDag::from_operations(circuit.operations()).unwrap();
    let unary = unary_nodes(&dag);

    assert_eq!(unary.len(), 2);
    assert_eq!(dag.graph[unary[0]].operations.len(), 2);
    assert_eq!(dag.graph[unary[1]].operations.len(), 1);
}

#[test]
fn routing_dag_does_not_fold_unary_into_pair_or_reverse_ordered_pair() {
    let mut circuit = Circuit::new(2);
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(1), Qubit::new(0)).unwrap();

    let dag = SabreDag::from_operations(circuit.operations()).unwrap();
    let pairs = interaction_nodes(&dag);
    let unary = unary_nodes(&dag);

    assert_eq!(pairs.len(), 2);
    assert_eq!(unary.len(), 1);
    assert!(dag.graph.find_edge(pairs[0], unary[0]).is_some());
    assert!(dag.graph.find_edge(unary[0], pairs[1]).is_some());
}
