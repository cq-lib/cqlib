use super::{CircuitDag, DagControlFlow, DagNode, DagWire};
use crate::circuit::Parameter;
use crate::circuit::{
    Circuit, CircuitError, CircuitParam, ClassicalControlOp, ClassicalDataOp, ClassicalExpr,
    ClassicalType, ControlBody, IfOp, Instruction, Operation, ParameterValue, Qubit, StandardGate,
    ValueInstruction, ValueOperation,
};
use indexmap::IndexSet;
use proptest::prelude::*;
use rustworkx_core::petgraph::visit::EdgeRef;
use smallvec::smallvec;

fn q(index: u32) -> Qubit {
    Qubit::new(index)
}

fn h_op(qubit: Qubit) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::H),
        qubits: smallvec![qubit],
        params: smallvec![],
        label: None,
    }
}

fn x_op(qubit: Qubit) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::X),
        qubits: smallvec![qubit],
        params: smallvec![],
        label: None,
    }
}

fn store_op(target: crate::circuit::ClassicalVar, value: ClassicalExpr) -> Operation {
    Operation {
        instruction: Instruction::ClassicalData(ClassicalDataOp::Store { target, value }),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    }
}

fn dag_from_ops_with_bool_var(
    var: crate::circuit::ClassicalVar,
    operations: &[Operation],
) -> CircuitDag {
    CircuitDag::from_parts(
        vec![q(0)].into_iter().collect(),
        IndexSet::new(),
        IndexSet::new(),
        vec![var.ty()],
        vec![],
        CircuitParam::Fixed(0.0),
        operations,
    )
    .unwrap()
}

/// Returns the instruction of the node at `index` in topological order.
fn instruction_at(dag: &CircuitDag, index: usize) -> &Instruction {
    let nodes = dag.topological_op_nodes().unwrap();
    &dag.operation(nodes[index]).unwrap().instruction
}

/// Collects operation predecessors of a node (excluding wire sentinels).
fn op_predecessors(
    dag: &CircuitDag,
    node: rustworkx_core::petgraph::prelude::NodeIndex,
) -> Vec<rustworkx_core::petgraph::prelude::NodeIndex> {
    dag.predecessors(node)
        .filter(|n| dag.operation(*n).is_some())
        .collect()
}

fn fixed_param_value(param: &CircuitParam) -> f64 {
    match param {
        CircuitParam::Fixed(value) => *value,
        CircuitParam::Index(_) => panic!("expected fixed parameter"),
    }
}

/// Collects operation successors of a node (excluding wire sentinels).
fn op_successors(
    dag: &CircuitDag,
    node: rustworkx_core::petgraph::prelude::NodeIndex,
) -> Vec<rustworkx_core::petgraph::prelude::NodeIndex> {
    dag.successors(node)
        .filter(|n| dag.operation(*n).is_some())
        .collect()
}

#[test]
fn empty_circuit_round_trips() {
    let circuit = Circuit::new(2);
    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    assert_eq!(dag.num_qubits(), 2);
    assert!(dag.is_empty());
    assert_eq!(dag.depth().unwrap(), 0);

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.num_qubits(), 2);
    assert!(recovered.operations().is_empty());
}

#[test]
fn single_gate_round_trips() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert_eq!(dag.num_ops(), 1);
    assert_eq!(dag.depth().unwrap(), 1);

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.operations().len(), 1);
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn parameterized_gate_round_trips() {
    let mut circuit = Circuit::new(1);
    circuit.rz(q(0), std::f64::consts::FRAC_PI_2).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert_eq!(dag.num_ops(), 1);

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.operations().len(), 1);
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert!(
        (fixed_param_value(&recovered.operations()[0].params[0]) - std::f64::consts::FRAC_PI_2)
            .abs()
            < 1e-12
    );
}

#[test]
fn symbolic_parameter_round_trips() {
    let mut circuit = Circuit::new(1);
    circuit.rx(q(0), ParameterValue::from("theta")).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert_eq!(dag.num_ops(), 1);
    assert!(dag.parameters().iter().any(|p| !p.get_symbols().is_empty()));
    assert!(dag.symbols().contains("theta"));

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.operations().len(), 1);
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    let CircuitParam::Index(index) = recovered.operations()[0].params[0] else {
        panic!("expected symbolic parameter index");
    };
    let recovered_param = recovered.parameters().get_index(index as usize).unwrap();
    assert!(recovered_param.get_symbols().contains("theta"));
}

#[test]
fn global_phase_preserved_across_round_trip() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.set_global_phase(crate::circuit::Parameter::from(1.5));

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let recovered = dag.to_circuit().unwrap();

    let original_phase = circuit.global_phase();
    let recovered_phase = recovered.global_phase();
    assert!(
        (original_phase.evaluate(&None).unwrap() - recovered_phase.evaluate(&None).unwrap()).abs()
            < 1e-12
    );
}

#[test]
fn classical_tables_preserved_across_round_trip() {
    let mut circuit = Circuit::new(1);
    let _var1 = circuit.var(ClassicalType::Bool);
    let _var2 = circuit.var(ClassicalType::uint(8).unwrap());
    let _val = circuit.measure(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let recovered = dag.to_circuit().unwrap();

    assert_eq!(circuit.classical_vars(), recovered.classical_vars());
    assert_eq!(circuit.classical_values(), recovered.classical_values());
    assert_eq!(dag.classical_vars(), recovered.classical_vars());
    assert_eq!(dag.classical_values(), recovered.classical_values());
}

#[test]
fn round_trip_preserves_qubit_set() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let recovered = dag.to_circuit().unwrap();

    assert_eq!(dag.qubits(), recovered.qubits());
    assert_eq!(dag.num_qubits(), recovered.num_qubits());
}
#[test]
fn disjoint_gates_share_front_layer() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].len(), 2);
    assert_eq!(dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap().len(), 1);
    assert_eq!(dag.nodes_on_wire(DagWire::Qubit(q(1))).unwrap().len(), 1);
}

#[test]
fn same_qubit_gates_form_chain() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    assert_eq!(layers.len(), 2);
    assert_eq!(dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap().len(), 2);
}

#[test]
fn two_qubit_gate_depends_on_both_frontiers() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].len(), 2);
    assert_eq!(layers[1].len(), 1);
}

#[test]
fn classical_measurement_dependency_is_tracked() {
    let mut circuit = Circuit::new(2);
    let flag = circuit.var(ClassicalType::Bool);
    let measured = circuit.measure(q(0)).unwrap();
    circuit
        .store(flag, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let value_wire = DagWire::ClassicalValue(measured.value());

    assert_eq!(dag.nodes_on_wire(value_wire).unwrap().len(), 2);
    assert_eq!(dag.layers().unwrap().len(), 2);
    assert_eq!(dag.to_circuit().unwrap().operations().len(), 2);
}

#[test]
fn classical_value_wire_tracks_multiple_reads_in_order() {
    let mut circuit = Circuit::new(1);
    let first_flag = circuit.var(ClassicalType::Bool);
    let second_flag = circuit.var(ClassicalType::Bool);
    let measured = circuit.measure(q(0)).unwrap();
    let expr = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit.store(first_flag, expr.clone()).unwrap();
    circuit.store(second_flag, expr).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let value_wire = DagWire::ClassicalValue(measured.value());

    assert_eq!(dag.nodes_on_wire(value_wire).unwrap().len(), 3);
    assert_eq!(dag.layers().unwrap().len(), 3);
}

#[test]
fn store_creates_classical_var_wire() {
    let mut circuit = Circuit::new(1);
    let target = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .store(target, ClassicalExpr::uint_literal(8, 42).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let var_wire = DagWire::ClassicalVar(target);

    assert!(dag.wire_in(var_wire).is_some());
    assert!(dag.wire_out(var_wire).is_some());
    assert_eq!(dag.nodes_on_wire(var_wire).unwrap().len(), 1);
}

#[test]
fn store_expression_reads_track_classical_values() {
    let mut circuit = Circuit::new(1);
    let measured = circuit.measure(q(0)).unwrap();
    let dest = circuit.var(ClassicalType::Bool);
    circuit
        .store(dest, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    // The store reads the measurement value and writes to the var.
    // Both wires should include the store node.
    let value_wire = DagWire::ClassicalValue(measured.value());
    let var_wire = DagWire::ClassicalVar(dest);
    assert_eq!(dag.nodes_on_wire(value_wire).unwrap().len(), 2);
    assert_eq!(dag.nodes_on_wire(var_wire).unwrap().len(), 1);
}

#[test]
fn specific_qubit_barrier_synchronizes_only_listed() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.h(q(2)).unwrap();
    // Barrier on q0 and q1 only; q2 is free to proceed.
    circuit.barrier(vec![q(0), q(1)]).unwrap();
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.h(q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    // Layer 0: h(0), h(1), h(2)
    // Layer 1: barrier(0,1)
    // Layer 2: h(0), h(1), h(2)  — but h(2) from before barrier can be in layer 1
    //          alongside the barrier since they don't share a wire.
    // Actually h(2)_late depends on h(2)_early (same qubit), and barrier
    // doesn't touch q2, so h(2)_late can be in layer 1 or 2 depending on
    // whether h(2)_early is in layer 0.
    // h(2)_early is layer 0, h(2)_late depends only on h(2)_early -> layer 1.
    // So: layer 0 = {h0, h1, h2}, layer 1 = {barrier, h2_late}, layer 2 = {h0_late, h1_late}
    assert_eq!(layers.len(), 3);
    assert_eq!(layers[0].len(), 3);
    // Layer 1: barrier + h(2) (h(2) doesn't wait for barrier)
    assert_eq!(layers[1].len(), 2);
    // Layer 2: h(0) and h(1) must wait for barrier
    assert_eq!(layers[2].len(), 2);
}

#[test]
fn reset_appears_on_qubit_wire() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.reset(q(0)).unwrap();
    circuit.x(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let wire = DagWire::Qubit(q(0));

    let nodes = dag.nodes_on_wire(wire).unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(dag.depth().unwrap(), 3);
}

#[test]
fn cx_chain_on_overlapping_qubits() {
    let mut circuit = Circuit::new(3);
    circuit.cx(q(0), q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();
    circuit.cx(q(0), q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    // cx(0,1) -> cx(1,2) (share q1)
    // cx(1,2) -> cx(0,2) (share q2)
    // cx(0,1) -> cx(0,2) (share q0) — transitive but also direct via q0
    assert_eq!(dag.depth().unwrap(), 3);

    let q0_nodes = dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap();
    assert_eq!(q0_nodes.len(), 2); // cx(0,1) and cx(0,2)
}

#[test]
fn topological_order_uses_edges_not_order_field() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.op_nodes().collect::<Vec<_>>();
    let first = nodes[0];
    let second = nodes[1];
    if let DagNode::Operation { order, .. } = &mut dag.graph[first] {
        *order = 1;
    }
    if let DagNode::Operation { order, .. } = &mut dag.graph[second] {
        *order = 0;
    }

    let topological = dag.topological_op_nodes().unwrap();
    assert_eq!(topological, vec![first, second]);
    let recovered = dag.to_circuit().unwrap();
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn topological_order_respects_dependencies() {
    let mut circuit = Circuit::new(3);
    circuit.cx(q(0), q(1)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();
    circuit.h(q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let topo = dag.topological_op_nodes().unwrap();

    // Every node must appear after all its operation predecessors.
    let position: std::collections::HashMap<_, usize> =
        topo.iter().enumerate().map(|(i, n)| (*n, i)).collect();

    for &node in &topo {
        let pos = position[&node];
        for pred in op_predecessors(&dag, node) {
            let pred_pos = position[&pred];
            assert!(
                pred_pos < pos,
                "predecessor {:?} at {} appears after node {:?} at {}",
                pred,
                pred_pos,
                node,
                pos
            );
        }
    }
}

#[test]
fn topological_order_is_deterministic() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.h(q(2)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let first = dag.topological_op_nodes().unwrap();
    let second = dag.topological_op_nodes().unwrap();

    assert_eq!(first, second);
}
#[test]
fn layers_use_topological_order_not_order_field() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.op_nodes().collect::<Vec<_>>();
    let first = nodes[0];
    let second = nodes[1];
    if let DagNode::Operation { order, .. } = &mut dag.graph[first] {
        *order = 1;
    }
    if let DagNode::Operation { order, .. } = &mut dag.graph[second] {
        *order = 0;
    }

    let layers = dag.layers().unwrap();
    assert_eq!(layers, vec![vec![first], vec![second]]);
}

#[test]
fn empty_barrier_synchronizes_all_qubits() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.barrier(vec![]).unwrap();
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    assert_eq!(layers.len(), 3);
    assert_eq!(layers[0].len(), 2);
    assert_eq!(layers[1].len(), 1);
    assert_eq!(layers[2].len(), 2);
}

#[test]
fn layer_membership_identifies_correct_nodes() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap(); // layer 0
    circuit.x(q(1)).unwrap(); // layer 0
    circuit.cx(q(0), q(1)).unwrap(); // layer 1
    circuit.h(q(1)).unwrap(); // layer 2

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    assert_eq!(layers.len(), 3);

    // Layer 0: H(q0), X(q1)
    assert_eq!(layers[0].len(), 2);
    for &node in &layers[0] {
        let inst = dag.operation(node).unwrap();
        assert!(matches!(
            inst.instruction,
            Instruction::Standard(StandardGate::H | StandardGate::X)
        ));
    }

    // Layer 1: CX(q0, q1)
    assert_eq!(layers[1].len(), 1);
    assert!(matches!(
        dag.operation(layers[1][0]).unwrap().instruction,
        Instruction::Standard(StandardGate::CX)
    ));

    // Layer 2: H(q1)
    assert_eq!(layers[2].len(), 1);
    assert!(matches!(
        dag.operation(layers[2][0]).unwrap().instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn front_layer_contains_only_root_operations() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let front = dag.front_layer().unwrap();

    // Only H(q0) and X(q1) have no operation predecessors.
    assert_eq!(front.len(), 2);
    for &node in &front {
        assert!(op_predecessors(&dag, node).is_empty());
    }
}

#[test]
fn front_layer_empty_for_chained_circuit() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let front = dag.front_layer().unwrap();
    assert_eq!(front.len(), 1);
    assert!(op_predecessors(&dag, front[0]).is_empty());
}

#[test]
fn depth_cross_validates_with_circuit_depth() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();
    circuit.h(q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let dag_depth = dag.depth().unwrap();
    let circuit_depth = circuit.depth(false).unwrap();

    assert_eq!(dag_depth, circuit_depth);
}

#[test]
fn depth_cross_validates_with_barrier() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.barrier(vec![q(0), q(1), q(2)]).unwrap();
    circuit.h(q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let dag_depth = dag.depth().unwrap();
    let circuit_depth = circuit.depth(false).unwrap();

    assert_eq!(dag_depth, circuit_depth);
}

#[test]
fn layers_partition_all_operations() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();
    circuit.h(q(2)).unwrap();
    circuit.x(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();

    let total: usize = layers.iter().map(|l| l.len()).sum();
    assert_eq!(total, dag.num_ops());

    // No node appears in two layers.
    let mut seen = std::collections::HashSet::new();
    for layer in &layers {
        for &node in layer {
            assert!(
                seen.insert(node),
                "node {:?} appears in multiple layers",
                node
            );
        }
    }
}

#[test]
fn depth_of_empty_circuit_is_zero() {
    let circuit = Circuit::new(2);
    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert_eq!(dag.depth().unwrap(), 0);
}

#[test]
fn depth_of_parallel_gates_is_one() {
    let mut circuit = Circuit::new(4);
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.h(q(2)).unwrap();
    circuit.h(q(3)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert_eq!(dag.depth().unwrap(), 1);
}

#[test]
fn nodes_on_wire_returns_wire_traversal_order() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    circuit.z(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let nodes = dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap();

    assert_eq!(nodes.len(), 3);
    assert!(matches!(
        dag.operation(nodes[0]).unwrap().instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        dag.operation(nodes[1]).unwrap().instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        dag.operation(nodes[2]).unwrap().instruction,
        Instruction::Standard(StandardGate::Z)
    ));
}

#[test]
fn nodes_on_wire_for_classical_value() {
    let mut circuit = Circuit::new(1);
    let measured = circuit.measure(q(0)).unwrap();
    let dest = circuit.var(ClassicalType::Bool);
    circuit
        .store(dest, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let wire = DagWire::ClassicalValue(measured.value());
    let nodes = dag.nodes_on_wire(wire).unwrap();

    // Measure produces the value, Store reads it.
    assert_eq!(nodes.len(), 2);
    match &dag.operation(nodes[0]).unwrap().instruction {
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) => {
            assert_eq!(*result, measured.value());
        }
        other => panic!("expected measurement producer, got {other:?}"),
    }
    match &dag.operation(nodes[1]).unwrap().instruction {
        Instruction::ClassicalData(ClassicalDataOp::Store { target, .. }) => {
            assert_eq!(*target, dest);
        }
        other => panic!("expected store reader, got {other:?}"),
    }
}

#[test]
fn nodes_on_wire_empty_wire_returns_empty() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let nodes = dag.nodes_on_wire(DagWire::Qubit(q(1))).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn predecessors_and_successors_correct() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let nodes = dag.topological_op_nodes().unwrap();
    let h_node = nodes[0];
    let cx_node = nodes[1];

    // H has no operation predecessors, CX as successor.
    assert!(op_predecessors(&dag, h_node).is_empty());
    assert_eq!(op_successors(&dag, h_node), vec![cx_node]);

    // CX has H as predecessor, no operation successors.
    assert_eq!(op_predecessors(&dag, cx_node), vec![h_node]);
    assert!(op_successors(&dag, cx_node).is_empty());
}

#[test]
fn wire_in_and_out_exist_for_all_resources() {
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(q(0)).unwrap();
    let dest = circuit.var(ClassicalType::Bool);
    circuit
        .store(dest, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    // Qubit wires
    for i in 0..2 {
        let wire = DagWire::Qubit(q(i));
        assert!(
            dag.wire_in(wire).is_some(),
            "missing wire_in for {:?}",
            wire
        );
        assert!(
            dag.wire_out(wire).is_some(),
            "missing wire_out for {:?}",
            wire
        );
    }

    // Classical value wire
    let value_wire = DagWire::ClassicalValue(measured.value());
    assert!(dag.wire_in(value_wire).is_some());
    assert!(dag.wire_out(value_wire).is_some());

    // Classical var wire
    let var_wire = DagWire::ClassicalVar(dest);
    assert!(dag.wire_in(var_wire).is_some());
    assert!(dag.wire_out(var_wire).is_some());

    // Global order wire always exists
    assert!(dag.wire_in(DagWire::GlobalOrder).is_some());
    assert!(dag.wire_out(DagWire::GlobalOrder).is_some());
}

#[test]
fn wire_queries_report_materialized_and_idle_wires() {
    let mut circuit = Circuit::new(2);
    let unused_var = circuit.var(ClassicalType::Bool);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let wires = dag.wires().collect::<Vec<_>>();

    assert!(wires.contains(&DagWire::GlobalOrder));
    assert!(wires.contains(&DagWire::Qubit(q(0))));
    assert!(wires.contains(&DagWire::Qubit(q(1))));
    assert!(dag.has_wire(DagWire::Qubit(q(0))));
    assert!(!dag.has_wire(DagWire::ClassicalVar(unused_var)));
    assert!(!dag.is_wire_idle(DagWire::Qubit(q(0))).unwrap());
    assert!(dag.is_wire_idle(DagWire::Qubit(q(1))).unwrap());
    assert!(dag.is_wire_idle(DagWire::ClassicalVar(unused_var)).unwrap());
}

#[test]
fn operation_accessor_returns_correct_data() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let node = dag.topological_op_nodes().unwrap()[0];
    let op = dag.operation(node).unwrap();

    assert!(matches!(
        op.instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(op.qubits.len(), 1);
    assert_eq!(op.qubits[0], q(0));
}

#[test]
fn operation_accessor_returns_none_for_wire_nodes() {
    let circuit = Circuit::new(1);
    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    let wire_in = dag.wire_in(DagWire::Qubit(q(0))).unwrap();
    assert!(dag.operation(wire_in).is_none());

    let wire_out = dag.wire_out(DagWire::Qubit(q(0))).unwrap();
    assert!(dag.operation(wire_out).is_none());
}

#[test]
fn control_flow_accessor_returns_none_for_non_control_node() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let node = dag.topological_op_nodes().unwrap()[0];
    assert!(dag.control_flow(node).is_none());
}

#[test]
fn dependency_filters_separate_quantum_and_classical_edges() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    let measured = circuit.measure(q(0)).unwrap();
    let dest = circuit.var(ClassicalType::Bool);
    circuit
        .store(dest, ClassicalExpr::bit_to_bool(measured.expr()).unwrap())
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let nodes = dag.topological_op_nodes().unwrap();
    let h = nodes[0];
    let x = nodes[1];
    let cx = nodes[2];
    let measure = nodes[3];
    let store = nodes[4];

    assert_eq!(dag.quantum_predecessors(cx).unwrap(), vec![h, x]);
    assert_eq!(dag.quantum_successors(cx).unwrap(), vec![measure]);
    assert!(dag.classical_predecessors(cx).unwrap().is_empty());
    assert_eq!(dag.classical_predecessors(store).unwrap(), vec![measure]);
    assert!(dag.quantum_predecessors(store).unwrap().is_empty());
}

#[test]
fn filtered_neighbors_reject_missing_node() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let missing = rustworkx_core::petgraph::prelude::NodeIndex::new(usize::MAX / 2);

    assert!(matches!(
        dag.quantum_predecessors(missing),
        Err(CircuitError::InvalidDag(_))
    ));
    assert!(matches!(
        dag.quantum_successors(missing),
        Err(CircuitError::InvalidDag(_))
    ));
    assert!(matches!(
        dag.classical_predecessors(missing),
        Err(CircuitError::InvalidDag(_))
    ));
    assert!(matches!(
        dag.classical_successors(missing),
        Err(CircuitError::InvalidDag(_))
    ));
}

#[test]
fn wire_specific_neighbors_use_requested_resource() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.z(q(0)).unwrap();
    circuit.h(q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let nodes = dag.topological_op_nodes().unwrap();
    let h0 = nodes[0];
    let x1 = nodes[1];
    let cx = nodes[2];
    let z0 = nodes[3];
    let h1 = nodes[4];

    assert_eq!(
        dag.predecessors_on_wire(cx, DagWire::Qubit(q(0))).unwrap(),
        vec![h0]
    );
    assert_eq!(
        dag.predecessors_on_wire(cx, DagWire::Qubit(q(1))).unwrap(),
        vec![x1]
    );
    assert_eq!(
        dag.successors_on_wire(cx, DagWire::Qubit(q(0))).unwrap(),
        vec![z0]
    );
    assert_eq!(
        dag.successors_on_wire(cx, DagWire::Qubit(q(1))).unwrap(),
        vec![h1]
    );
}

#[test]
fn node_layers_match_layers_and_respect_edges() {
    let mut circuit = Circuit::new(3);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.h(q(2)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let layers = dag.layers().unwrap();
    let node_layers = dag.node_layers().unwrap();

    assert_eq!(node_layers.len(), dag.num_ops());
    for (layer_index, layer) in layers.iter().enumerate() {
        for node in layer {
            assert_eq!(node_layers.get(node).copied(), Some(layer_index));
        }
    }
    for node in dag.topological_op_nodes().unwrap() {
        let node_layer = node_layers[&node];
        for pred in op_predecessors(&dag, node) {
            assert!(node_layers[&pred] < node_layer);
        }
    }
}

#[test]
fn operation_count_and_fact_queries_cover_top_level_operations() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.h(q(1)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    let measured = circuit.measure(q(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.x(q(1))?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let counts = dag.operation_count_by_name();

    assert!(dag.has_control_flow());
    assert!(dag.has_measurement());
    assert_eq!(counts.get("H").copied(), Some(2));
    assert_eq!(counts.get("CX").copied(), Some(1));
    assert_eq!(counts.get("measure_bit").copied(), Some(1));
    assert_eq!(counts.get("if").copied(), Some(1));
    assert_eq!(counts.get("X"), None);
}

#[test]
fn recursive_operation_count_includes_control_flow_bodies() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    let loop_var = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .for_uint(
            loop_var,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.x(q(0))?;
                body.if_(ClassicalExpr::bool_literal(true), |inner| {
                    inner.z(q(1))?;
                    Ok(())
                })?;
                Ok(())
            },
        )
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let top_level = dag.operation_count_by_name();
    let recursive = dag.operation_count_by_name_recursive();

    assert!(dag.has_control_flow());
    assert!(dag.has_nested_control_flow());
    assert_eq!(top_level.get("H").copied(), Some(1));
    assert_eq!(top_level.get("for").copied(), Some(1));
    assert_eq!(top_level.get("X"), None);
    assert_eq!(recursive.get("H").copied(), Some(1));
    assert_eq!(recursive.get("for").copied(), Some(1));
    assert_eq!(recursive.get("if").copied(), Some(1));
    assert_eq!(recursive.get("X").copied(), Some(1));
    assert_eq!(recursive.get("Z").copied(), Some(1));
}

#[test]
fn nested_control_flow_query_ignores_single_top_level_control_op() {
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q(0))?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();

    assert!(dag.has_control_flow());
    assert!(!dag.has_nested_control_flow());
}

#[test]
fn one_qubit_runs_are_collected_per_wire_and_split_by_blockers() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.h(q(0)).unwrap();
    circuit.reset(q(0)).unwrap();
    circuit.z(q(0)).unwrap();
    circuit.x(q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let q0_runs = dag
        .collect_runs_on_wire(DagWire::Qubit(q(0)), |operation| {
            matches!(operation.instruction, Instruction::Standard(_)) && operation.qubits.len() == 1
        })
        .unwrap();
    let all_runs = dag.collect_1q_runs().unwrap();

    assert_eq!(
        q0_runs.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![2, 1, 1]
    );
    assert!(all_runs.iter().any(|run| run.len() == 2));
    assert!(all_runs.iter().any(|run| {
        run.iter().any(|node| {
            matches!(
                dag.operation(*node).unwrap().instruction,
                Instruction::Standard(StandardGate::X)
            ) && dag.operation(*node).unwrap().qubits.as_slice() == &[q(1)]
        })
    }));
}

#[test]
fn two_qubit_runs_are_collected_per_wire_and_split_by_blockers() {
    let mut circuit = Circuit::new(3);
    circuit.cx(q(0), q(1)).unwrap();
    circuit.cz(q(0), q(1)).unwrap();
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(2)).unwrap();
    circuit.reset(q(2)).unwrap();
    circuit.cx(q(0), q(2)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let runs = dag.collect_2q_runs().unwrap();
    let run_lengths = runs.iter().map(Vec::len).collect::<Vec<_>>();

    assert!(run_lengths.contains(&2));
    assert!(runs.iter().any(|run| {
        run.iter().all(|node| {
            dag.operation(*node)
                .is_some_and(|operation| operation.qubits.len() == 2)
        })
    }));
}
#[test]
fn edit_api_remove_and_substitute_round_trip() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let first = dag.topological_op_nodes().unwrap()[0];
    let removed = dag.remove_op_node(first).unwrap();
    assert!(matches!(
        removed.instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert_eq!(dag.num_ops(), 1);

    let remaining = dag.topological_op_nodes().unwrap()[0];
    dag.substitute_node(remaining, h_op(q(0))).unwrap();
    let recovered = dag.to_circuit().unwrap();
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn edit_api_substitute_node_with_dag() {
    let mut circuit = Circuit::new(1);
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement_circuit = Circuit::new(1);
    replacement_circuit.h(q(0)).unwrap();
    replacement_circuit.x(q(0)).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement_circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    dag.substitute_node_with_dag(node, replacement_dag).unwrap();
    assert_eq!(dag.num_ops(), 2);
    assert_eq!(dag.depth().unwrap(), 2);
}

#[test]
fn substitute_node_with_dag_rejects_extra_wire() {
    let mut circuit = Circuit::new(1);
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement_circuit = Circuit::new(2);
    replacement_circuit.cx(q(0), q(1)).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement_circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    let error = dag
        .substitute_node_with_dag(node, replacement_dag)
        .unwrap_err();
    assert!(matches!(error, CircuitError::InvalidDag(_)));
}

#[test]
fn substitute_node_with_dag_uses_original_dag_for_empty_barrier_footprint() {
    let mut circuit = Circuit::new(1);
    circuit.barrier(vec![]).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement_circuit = Circuit::from_qubits(vec![q(1)]).unwrap();
    replacement_circuit.h(q(1)).unwrap();
    replacement_circuit.barrier(vec![]).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement_circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    let error = dag
        .substitute_node_with_dag(node, replacement_dag)
        .unwrap_err();
    assert!(matches!(error, CircuitError::InvalidDag(_)));
}

#[test]
fn substitute_node_with_dag_rejects_classical_write_outside_old_write_footprint() {
    let mut circuit = Circuit::new(1);
    let flag = circuit.var(ClassicalType::Bool);
    let old = Operation {
        instruction: Instruction::ClassicalControl(ClassicalControlOp::If(
            IfOp::new(flag.expr(), ControlBody::new(vec![h_op(q(0))]), None).unwrap(),
        )),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    };
    let replacement = Operation {
        instruction: Instruction::ClassicalControl(ClassicalControlOp::If(
            IfOp::new(
                flag.expr(),
                ControlBody::new(vec![store_op(flag, ClassicalExpr::bool_literal(true))]),
                None,
            )
            .unwrap(),
        )),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    };
    let mut dag = dag_from_ops_with_bool_var(flag, &[old]);
    let replacement_dag = dag_from_ops_with_bool_var(flag, &[replacement]);

    let node = dag.topological_op_nodes().unwrap()[0];
    let error = dag
        .substitute_node_with_dag(node, replacement_dag)
        .unwrap_err();
    assert!(matches!(error, CircuitError::InvalidDag(_)));
}

#[test]
fn substitute_node_with_dag_rejects_control_flow_flattening() {
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q(0))?;
            Ok(())
        })
        .unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement_circuit = Circuit::new(1);
    replacement_circuit.h(q(0)).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement_circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    let error = dag
        .substitute_node_with_dag(node, replacement_dag)
        .unwrap_err();
    assert!(matches!(error, CircuitError::InvalidDag(_)));
}

#[test]
fn substitute_node_with_dag_allows_same_kind_control_flow_replacement() {
    let mut circuit = Circuit::new(1);
    circuit
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.h(q(0))?;
            Ok(())
        })
        .unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement_circuit = Circuit::new(1);
    replacement_circuit
        .if_(ClassicalExpr::bool_literal(false), |body| {
            body.x(q(0))?;
            Ok(())
        })
        .unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement_circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    dag.substitute_node_with_dag(node, replacement_dag).unwrap();
    assert_eq!(dag.num_ops(), 1);
    assert!(matches!(
        instruction_at(&dag, 0),
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
}

#[test]
fn apply_operation_back_appends() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let new_node = dag.apply_operation_back(x_op(q(0))).unwrap();
    assert_eq!(dag.num_ops(), 2);
    assert!(matches!(
        dag.operation(new_node).unwrap().instruction,
        Instruction::Standard(StandardGate::X)
    ));

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.operations().len(), 2);
    assert!(matches!(
        recovered.operations()[1].instruction,
        Instruction::Standard(StandardGate::X)
    ));
}

#[test]
fn apply_operation_front_prepends() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let new_node = dag.apply_operation_front(x_op(q(0))).unwrap();
    assert_eq!(dag.num_ops(), 2);
    assert!(matches!(
        dag.operation(new_node).unwrap().instruction,
        Instruction::Standard(StandardGate::X)
    ));

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(recovered.operations().len(), 2);
    assert!(matches!(
        recovered.operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        recovered.operations()[1].instruction,
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn apply_value_operations_lower_parameters() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();
    let theta = Parameter::symbol("theta");

    let rx = ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RX)),
        qubits: smallvec![q(0)],
        params: smallvec![ParameterValue::Param(theta.clone())],
        label: None,
    };
    let rz = ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RZ)),
        qubits: smallvec![q(0)],
        params: smallvec![ParameterValue::Fixed(0.25)],
        label: None,
    };

    let front = dag.apply_value_operation_front(rx).unwrap();
    let back = dag.apply_value_operation_back(rz).unwrap();

    assert!(matches!(
        dag.operation(front).unwrap().instruction,
        Instruction::Standard(StandardGate::RX)
    ));
    assert!(matches!(
        dag.operation(back).unwrap().instruction,
        Instruction::Standard(StandardGate::RZ)
    ));
    assert!(dag.symbols().contains("theta"));
    assert!(dag.parameters().iter().any(|param| param == &theta));

    let recovered = dag.to_circuit().unwrap();
    assert_eq!(
        recovered
            .operations()
            .iter()
            .map(|operation| operation.instruction.to_string())
            .collect::<Vec<_>>(),
        vec!["RX", "H", "RZ"]
    );
}

#[test]
fn remove_op_node_preserves_wire_connectivity() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    circuit.z(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    // Remove the middle X gate.
    let nodes = dag.topological_op_nodes().unwrap();
    let removed = dag.remove_op_node(nodes[1]).unwrap();
    assert!(matches!(
        removed.instruction,
        Instruction::Standard(StandardGate::X)
    ));

    // H and Z should still be chained on q0.
    assert_eq!(dag.num_ops(), 2);
    assert_eq!(dag.depth().unwrap(), 2);

    let wire_nodes = dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap();
    assert_eq!(wire_nodes.len(), 2);
    assert!(matches!(
        dag.operation(wire_nodes[0]).unwrap().instruction,
        Instruction::Standard(StandardGate::H)
    ));
    assert!(matches!(
        dag.operation(wire_nodes[1]).unwrap().instruction,
        Instruction::Standard(StandardGate::Z)
    ));
}

#[test]
fn remove_op_node_from_parallel_branch() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.x(q(1)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.topological_op_nodes().unwrap();
    dag.remove_op_node(nodes[0]).unwrap();

    assert_eq!(dag.num_ops(), 1);
    assert_eq!(dag.depth().unwrap(), 1);
}

#[test]
fn substitute_node_preserves_position_and_dependencies() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.topological_op_nodes().unwrap();
    // Replace H with Z — should stay in position 0.
    dag.substitute_node(
        nodes[0],
        Operation {
            instruction: Instruction::Standard(StandardGate::Z),
            qubits: smallvec![q(0)],
            params: smallvec![],
            label: None,
        },
    )
    .unwrap();

    assert_eq!(dag.num_ops(), 2);
    assert!(matches!(
        instruction_at(&dag, 0),
        Instruction::Standard(StandardGate::Z)
    ));
    assert!(matches!(
        instruction_at(&dag, 1),
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(dag.depth().unwrap(), 2);
}

#[test]
fn substitute_value_node_lowers_symbolic_parameters() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();
    let theta = Parameter::symbol("theta");
    let replacement = ValueOperation {
        instruction: ValueInstruction::from_instruction(Instruction::Standard(StandardGate::RX)),
        qubits: smallvec![q(0)],
        params: smallvec![ParameterValue::Param(theta.clone())],
        label: None,
    };

    let node = dag.topological_op_nodes().unwrap()[0];
    dag.substitute_value_node(node, replacement).unwrap();

    assert!(matches!(
        instruction_at(&dag, 0),
        Instruction::Standard(StandardGate::RX)
    ));
    assert!(dag.symbols().contains("theta"));
    let recovered = dag.to_circuit().unwrap();
    let CircuitParam::Index(index) = recovered.operations()[0].params[0] else {
        panic!("expected substituted symbolic parameter");
    };
    assert_eq!(
        recovered.parameters().get_index(index as usize).unwrap(),
        &theta
    );
}

#[test]
fn substitute_node_with_dag_preserves_external_wires() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    circuit.z(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    // Replace the middle X with H;Y.
    let nodes = dag.topological_op_nodes().unwrap();
    let mut replacement = Circuit::new(1);
    replacement.h(q(0)).unwrap();
    replacement.y(q(0)).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement).unwrap();

    dag.substitute_node_with_dag(nodes[1], replacement_dag)
        .unwrap();

    assert_eq!(dag.num_ops(), 4);
    // H -> H -> Y -> Z on q0, depth 4.
    assert_eq!(dag.depth().unwrap(), 4);
}

#[test]
fn sequential_edits_compose() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    // Append X, then append Z, then remove H.
    dag.apply_operation_back(x_op(q(0))).unwrap();
    dag.apply_operation_back(Operation {
        instruction: Instruction::Standard(StandardGate::Z),
        qubits: smallvec![q(0)],
        params: smallvec![],
        label: None,
    })
    .unwrap();

    let nodes = dag.topological_op_nodes().unwrap();
    dag.remove_op_node(nodes[0]).unwrap(); // Remove original H.

    assert_eq!(dag.num_ops(), 2);
    assert!(matches!(
        instruction_at(&dag, 0),
        Instruction::Standard(StandardGate::X)
    ));
    assert!(matches!(
        instruction_at(&dag, 1),
        Instruction::Standard(StandardGate::Z)
    ));
}

#[test]
fn repeated_edits_keep_order_index_consistent() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let appended = dag
        .apply_operation_back(Operation {
            instruction: Instruction::Standard(StandardGate::Z),
            qubits: smallvec![q(0)],
            params: smallvec![],
            label: None,
        })
        .unwrap();
    assert!(matches!(
        dag.operation(appended).unwrap().instruction,
        Instruction::Standard(StandardGate::Z)
    ));

    let first = dag.topological_op_nodes().unwrap()[0];
    dag.remove_op_node(first).unwrap();
    let prepended = dag.apply_operation_front(h_op(q(0))).unwrap();

    assert_eq!(dag.topological_op_nodes().unwrap()[0], prepended);
    dag.validate().unwrap();
}

#[test]
fn substitute_node_with_dag_inline_replacement() {
    // Replace a single gate with another single-gate DAG.
    let mut circuit = Circuit::new(1);
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let mut replacement = Circuit::new(1);
    replacement.h(q(0)).unwrap();
    let replacement_dag = CircuitDag::from_circuit(&replacement).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    dag.substitute_node_with_dag(node, replacement_dag).unwrap();

    assert_eq!(dag.num_ops(), 1);
    assert!(matches!(
        instruction_at(&dag, 0),
        Instruction::Standard(StandardGate::H)
    ));
}

#[test]
fn control_flow_builds_child_dag() {
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(q(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.x(q(1))?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag.topological_op_nodes().unwrap()[1];
    let control = dag.control_flow(control_node).unwrap();

    match control {
        DagControlFlow::If {
            then_body,
            else_body,
        } => {
            assert_eq!(then_body.num_ops(), 1);
            assert!(else_body.is_none());
        }
        _ => panic!("expected if control-flow payload"),
    }
}

#[test]
fn if_else_builds_both_bodies() {
    let mut circuit = Circuit::new(1);
    let measured = circuit.measure(q(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_else(
            condition,
            |then_body| {
                then_body.x(q(0))?;
                Ok(())
            },
            |else_body| {
                else_body.z(q(0))?;
                Ok(())
            },
        )
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag.topological_op_nodes().unwrap()[1];
    let control = dag.control_flow(control_node).unwrap();

    match control {
        DagControlFlow::If {
            then_body,
            else_body,
        } => {
            assert_eq!(then_body.num_ops(), 1);
            assert!(matches!(
                then_body
                    .topological_op_nodes()
                    .unwrap()
                    .first()
                    .and_then(|n| then_body.operation(*n))
                    .map(|op| &op.instruction),
                Some(Instruction::Standard(StandardGate::X))
            ));

            let else_body = else_body.as_ref().expect("else body should exist");
            assert_eq!(else_body.num_ops(), 1);
            assert!(matches!(
                else_body
                    .topological_op_nodes()
                    .unwrap()
                    .first()
                    .and_then(|n| else_body.operation(*n))
                    .map(|op| &op.instruction),
                Some(Instruction::Standard(StandardGate::Z))
            ));
        }
        _ => panic!("expected if control-flow payload"),
    }
}

#[test]
fn while_loop_builds_body_dag() {
    let mut circuit = Circuit::new(1);
    let flag = circuit.var(ClassicalType::Bool);
    circuit
        .while_(flag.expr(), |body| {
            body.h(q(0))?;
            body.break_loop()?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .find(|n| dag.control_flow(*n).is_some())
        .expect("should have a control-flow node");

    match dag.control_flow(control_node).unwrap() {
        DagControlFlow::While { body } => {
            // body has H + break = 2 ops
            assert_eq!(body.num_ops(), 2);
        }
        _ => panic!("expected while control-flow payload"),
    }
}

#[test]
fn for_loop_builds_body_dag() {
    let mut circuit = Circuit::new(1);
    let counter = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 3).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                Ok(())
            },
        )
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .find(|n| dag.control_flow(*n).is_some())
        .expect("should have a control-flow node");

    match dag.control_flow(control_node).unwrap() {
        DagControlFlow::For { body } => {
            assert_eq!(body.num_ops(), 1);
            assert!(matches!(
                body.topological_op_nodes()
                    .unwrap()
                    .first()
                    .and_then(|n| body.operation(*n))
                    .map(|op| &op.instruction),
                Some(Instruction::Standard(StandardGate::H))
            ));
        }
        _ => panic!("expected for control-flow payload"),
    }
}

#[test]
fn switch_with_default_builds_all_cases() {
    let mut circuit = Circuit::new(1);
    let state = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .switch(state.expr(), |cases| {
            cases.value(0, |body| {
                body.x(q(0))?;
                Ok(())
            })?;
            cases.value(1, |body| {
                body.z(q(0))?;
                Ok(())
            })?;
            cases.default(|body| {
                body.h(q(0))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .find(|n| dag.control_flow(*n).is_some())
        .expect("should have a control-flow node");

    match dag.control_flow(control_node).unwrap() {
        DagControlFlow::Switch { cases, default } => {
            assert_eq!(cases.len(), 2);
            assert_eq!(cases[0].value, 0);
            assert_eq!(cases[0].body.num_ops(), 1);
            assert_eq!(cases[1].value, 1);
            assert_eq!(cases[1].body.num_ops(), 1);
            assert!(default.is_some());
            assert_eq!(default.as_ref().unwrap().num_ops(), 1);
        }
        _ => panic!("expected switch control-flow payload"),
    }
}

#[test]
fn switch_without_default_has_none_default() {
    let mut circuit = Circuit::new(1);
    let state = circuit.var(ClassicalType::uint(2).unwrap());
    circuit
        .switch(state.expr(), |cases| {
            cases.value(0, |body| {
                body.x(q(0))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let control_node = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .find(|n| dag.control_flow(*n).is_some())
        .expect("should have a control-flow node");

    match dag.control_flow(control_node).unwrap() {
        DagControlFlow::Switch { cases, default } => {
            assert_eq!(cases.len(), 1);
            assert!(default.is_none());
        }
        _ => panic!("expected switch control-flow payload"),
    }
}

#[test]
fn nested_if_inside_for_builds_child_dags() {
    let mut circuit = Circuit::new(1);
    let counter = circuit.var(ClassicalType::uint(8).unwrap());
    let flag = circuit.var(ClassicalType::Bool);
    circuit
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 3).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.if_(flag.expr(), |inner| {
                    inner.h(q(0))?;
                    Ok(())
                })?;
                Ok(())
            },
        )
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let for_node = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .find(|n| dag.control_flow(*n).is_some())
        .expect("should have a for control-flow node");

    let for_cf = match dag.control_flow(for_node).unwrap() {
        DagControlFlow::For { body } => body,
        _ => panic!("expected for control-flow payload"),
    };

    // The for body contains one if_ op.
    assert_eq!(for_cf.num_ops(), 1);
    let if_node = for_cf.topological_op_nodes().unwrap()[0];
    let if_cf = for_cf
        .control_flow(if_node)
        .expect("body should contain if");

    match if_cf {
        DagControlFlow::If { then_body, .. } => {
            assert_eq!(then_body.num_ops(), 1);
        }
        _ => panic!("expected if inside for body"),
    }
}

#[test]
fn break_and_continue_appear_in_body_dag() {
    let mut circuit = Circuit::new(1);
    let counter = circuit.var(ClassicalType::uint(8).unwrap());
    circuit
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 10).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(0))?;
                body.break_loop()?;
                Ok(())
            },
        )
        .unwrap();
    circuit
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 10).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.x(q(0))?;
                body.continue_loop()?;
                Ok(())
            },
        )
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let for_nodes = dag
        .topological_op_nodes()
        .unwrap()
        .into_iter()
        .filter_map(|node| match dag.control_flow(node) {
            Some(DagControlFlow::For { body }) => Some(body),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(for_nodes.len(), 2);
    assert_eq!(for_nodes[0].num_ops(), 2);
    assert_eq!(for_nodes[1].num_ops(), 2);

    let first_body_has_break = for_nodes[0]
        .topological_op_nodes()
        .unwrap()
        .iter()
        .any(|n| matches!(for_nodes[0].control_flow(*n), Some(DagControlFlow::Break)));
    assert!(
        first_body_has_break,
        "first body should contain a break node"
    );

    let second_body_has_continue = for_nodes[1]
        .topological_op_nodes()
        .unwrap()
        .iter()
        .any(|n| {
            matches!(
                for_nodes[1].control_flow(*n),
                Some(DagControlFlow::Continue)
            )
        });
    assert!(
        second_body_has_continue,
        "second body should contain a continue node"
    );
}

#[test]
fn control_flow_round_trips() {
    let mut circuit = Circuit::new(2);
    let measured = circuit.measure(q(0)).unwrap();
    let condition = ClassicalExpr::bit_to_bool(measured.expr()).unwrap();
    circuit
        .if_(condition, |body| {
            body.x(q(1))?;
            Ok(())
        })
        .unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    let recovered = dag.to_circuit().unwrap();

    assert_eq!(recovered.operations().len(), circuit.operations().len());
    let recovered_if = recovered
        .operations()
        .iter()
        .find_map(|op| match &op.instruction {
            Instruction::ClassicalControl(ClassicalControlOp::If(if_op)) => Some(if_op),
            _ => None,
        })
        .expect("round-trip should preserve the if op");
    assert!(
        recovered_if
            .condition()
            .values()
            .contains(&measured.value())
    );
    assert_eq!(recovered_if.then_body().operations().len(), 1);
    assert!(matches!(
        recovered_if.then_body().operations()[0].instruction,
        Instruction::Standard(StandardGate::X)
    ));
    assert_eq!(
        recovered_if.then_body().operations()[0].qubits.as_slice(),
        &[q(1)]
    );
    assert!(recovered_if.else_body().is_none());
}

#[test]
fn invalid_qubit_in_operation_rejected() {
    let circuit = Circuit::new(1);
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    // Try to append an operation with a qubit not in the DAG.
    let bad_op = Operation {
        instruction: Instruction::Standard(StandardGate::H),
        qubits: smallvec![q(99)],
        params: smallvec![],
        label: None,
    };
    let err = dag.apply_operation_back(bad_op).unwrap_err();
    assert!(matches!(err, CircuitError::QubitNotFound(99)));
}

#[test]
fn foreign_classical_var_in_dag_operation_rejected() {
    let mut circuit1 = Circuit::new(1);
    let var_from_circuit1 = circuit1.var(ClassicalType::Bool);

    let circuit2 = Circuit::new(1);
    let mut dag = CircuitDag::from_circuit(&circuit2).unwrap();
    let error = dag
        .apply_operation_back(Operation {
            instruction: Instruction::ClassicalData(ClassicalDataOp::Store {
                target: var_from_circuit1,
                value: ClassicalExpr::bool_literal(true),
            }),
            qubits: smallvec![],
            params: smallvec![],
            label: None,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        CircuitError::ForeignClassicalHandle {
            kind: "classical var",
            ..
        }
    ));
}

#[test]
fn dag_validation_passes_for_valid_circuit() {
    let mut circuit = Circuit::new(2);
    circuit.h(q(0)).unwrap();
    circuit.cx(q(0), q(1)).unwrap();
    circuit.measure(q(1)).unwrap();

    let dag = CircuitDag::from_circuit(&circuit).unwrap();
    assert!(dag.validate().is_ok());
}

#[test]
fn validate_rejects_cycle() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.topological_op_nodes().unwrap();
    dag.graph.add_edge(nodes[1], nodes[0], DagWire::Qubit(q(0)));

    assert!(matches!(dag.validate(), Err(CircuitError::InvalidDag(_))));
}

#[test]
fn validate_rejects_duplicate_operation_order() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.op_nodes().collect::<Vec<_>>();
    if let DagNode::Operation { order, .. } = &mut dag.graph[nodes[1]] {
        *order = 0;
    }

    assert!(matches!(dag.validate(), Err(CircuitError::InvalidDag(_))));
}

#[test]
fn nodes_on_wire_rejects_branched_wire() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    circuit.x(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let nodes = dag.topological_op_nodes().unwrap();
    let output = dag.wire_out(DagWire::Qubit(q(0))).unwrap();
    dag.graph.add_edge(nodes[0], output, DagWire::Qubit(q(0)));

    assert!(matches!(
        dag.nodes_on_wire(DagWire::Qubit(q(0))),
        Err(CircuitError::InvalidDag(_))
    ));
}

#[test]
fn nodes_on_wire_rejects_broken_wire() {
    let mut circuit = Circuit::new(1);
    circuit.h(q(0)).unwrap();
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();

    let node = dag.topological_op_nodes().unwrap()[0];
    let wire = DagWire::Qubit(q(0));
    let edge = dag
        .graph
        .edges_directed(node, rustworkx_core::petgraph::Outgoing)
        .find(|edge| *edge.weight() == wire)
        .map(|edge| edge.id())
        .expect("gate should connect to wire output");
    dag.graph.remove_edge(edge);

    assert!(matches!(
        dag.nodes_on_wire(wire),
        Err(CircuitError::InvalidDag(_))
    ));
}

#[test]
fn validate_rejects_foreign_edge_wire() {
    let circuit = Circuit::new(1);
    let mut dag = CircuitDag::from_circuit(&circuit).unwrap();
    let input = dag.wire_in(DagWire::GlobalOrder).unwrap();
    let output = dag.wire_out(DagWire::GlobalOrder).unwrap();
    dag.graph.add_edge(input, output, DagWire::Qubit(q(99)));

    assert!(matches!(
        dag.validate(),
        Err(CircuitError::QubitNotFound(99))
    ));
}

proptest! {
    #[test]
    fn round_trip_preserves_operation_count(
        num_gates in 0..40usize,
        num_qubits in 1..4usize,
    ) {
        let mut circuit = Circuit::new(num_qubits);
        for i in 0..num_gates {
            let target = Qubit::new((i % num_qubits) as u32);
            circuit.h(target).unwrap();
        }

        let dag = CircuitDag::from_circuit(&circuit).unwrap();
        let recovered = dag.to_circuit().unwrap();

        prop_assert_eq!(circuit.operations().len(), recovered.operations().len());
        prop_assert_eq!(dag.num_ops(), recovered.operations().len());
    }

    #[test]
    fn round_trip_preserves_qubit_count(
        num_qubits in 1..8usize,
    ) {
        let mut circuit = Circuit::new(num_qubits);
        if num_qubits > 0 {
            circuit.h(Qubit::new(0)).unwrap();
        }

        let dag = CircuitDag::from_circuit(&circuit).unwrap();
        let recovered = dag.to_circuit().unwrap();

        prop_assert_eq!(recovered.num_qubits(), num_qubits);
        prop_assert_eq!(dag.num_qubits(), num_qubits);
    }

    #[test]
    fn depth_matches_circuit_depth_for_gate_only_circuits(
        num_gates in 0..30usize,
        num_qubits in 1..3usize,
    ) {
        let mut circuit = Circuit::new(num_qubits);
        for i in 0..num_gates {
            let target = Qubit::new((i % num_qubits) as u32);
            circuit.h(target).unwrap();
        }

        let dag = CircuitDag::from_circuit(&circuit).unwrap();
        let dag_depth = dag.depth().unwrap();
        let circuit_depth = circuit.depth(false).unwrap();

        prop_assert_eq!(dag_depth, circuit_depth);
    }

    #[test]
    fn layers_count_equals_depth(
        num_gates in 0..30usize,
        num_qubits in 1..3usize,
    ) {
        let mut circuit = Circuit::new(num_qubits);
        for i in 0..num_gates {
            let target = Qubit::new((i % num_qubits) as u32);
            circuit.h(target).unwrap();
        }

        let dag = CircuitDag::from_circuit(&circuit).unwrap();
        let layers = dag.layers().unwrap();
        let depth = dag.depth().unwrap();

        prop_assert_eq!(layers.len(), depth);
    }

    #[test]
    fn topological_order_respects_all_dependencies(
        num_gates in 1..25usize,
        num_qubits in 1..3usize,
    ) {
        let mut circuit = Circuit::new(num_qubits);
        for i in 0..num_gates {
            let target = Qubit::new((i % num_qubits) as u32);
            circuit.h(target).unwrap();
        }

        let dag = CircuitDag::from_circuit(&circuit).unwrap();
        let topo = dag.topological_op_nodes().unwrap();

        let position: std::collections::HashMap<_, usize> = topo
            .iter()
            .enumerate()
            .map(|(i, n)| (*n, i))
            .collect();

        for &node in &topo {
            let pos = position[&node];
            for pred in dag.predecessors(node) {
                if dag.operation(pred).is_some() {
                    let pred_pos = position[&pred];
                    prop_assert!(
                        pred_pos < pos,
                        "predecessor at {} appears after node at {}",
                        pred_pos,
                        pos
                    );
                }
            }
        }
    }
}
