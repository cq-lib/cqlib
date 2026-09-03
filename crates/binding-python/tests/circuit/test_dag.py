# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

import sys
import copy

import pytest

from cqlib import CircuitDag as TopLevelCircuitDag
from cqlib import DagWire as TopLevelDagWire
from cqlib.circuit import (
    Circuit,
    CircuitDag,
    ClassicalExpr,
    ClassicalType,
    DagControlFlow,
    DagSwitchCase,
    DagWire,
    Instruction,
    Parameter,
    ValueOperation,
)
from cqlib.circuit.gates import Directive, MCGate, StandardGate, UnitaryGate


def test_dag_types_are_registered_and_exported() -> None:
    assert "cqlib._native.circuit" in sys.modules
    assert CircuitDag is TopLevelCircuitDag
    assert DagWire is TopLevelDagWire
    assert DagControlFlow.__module__ == "cqlib.circuit"
    assert DagSwitchCase.__module__ == "cqlib.circuit"
    assert CircuitDag.__module__ == "cqlib.circuit"
    assert DagWire.__module__ == "cqlib.circuit"


def test_instruction_names_match_stable_core_names() -> None:
    cases = [
        (Instruction.from_standard_gate(StandardGate.H), "H", "standard"),
        (Instruction.from_mc_gate(MCGate(2, StandardGate.X)), "C2-X", "mcgate"),
        (
            Instruction.from_unitary_gate(UnitaryGate("oracle", 1)),
            "oracle",
            "unitary",
        ),
        (
            Instruction.from_directive(Directive.barrier()),
            "barrier",
            "directive",
        ),
        (Instruction.delay(), "delay", "delay"),
    ]

    for instruction, expected_name, expected_type in cases:
        assert instruction.name == expected_name
        assert instruction.instruction_type == expected_type

    assert Directive.barrier().name() == "barrier"
    assert str(Directive.measure()) == "measure"
    assert Directive.reset().name() == "reset"


def test_circuit_dag_basic_layers_depth_and_round_trip() -> None:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.x(1)
    circuit.cx(0, 1)

    dag = circuit.dag()
    same = CircuitDag.from_circuit(circuit)

    assert dag.num_qubits == 2
    assert dag.num_ops == 3
    assert len(dag) == 3
    assert dag.is_empty is False
    assert dag.qubits == circuit.qubits
    assert dag.topological_op_nodes() == same.topological_op_nodes()
    assert len(dag.front_layer()) == 2
    assert [len(layer) for layer in dag.layers()] == [2, 1]
    assert dag.depth() == 2

    recovered = dag.to_circuit()
    assert len(recovered.operations) == 3
    assert [operation.name for operation in recovered.operations] == ["H", "X", "CX"]
    assert copy.copy(dag).topological_op_nodes() == dag.topological_op_nodes()
    assert copy.deepcopy(dag).topological_op_nodes() == dag.topological_op_nodes()


def test_dag_wire_queries_and_dependency_filters() -> None:
    circuit = Circuit(1)
    measured = circuit.measure(0)
    flag = circuit.var(ClassicalType.bool())
    circuit.store(flag, measured.expr().bit_to_bool())

    dag = circuit.dag()
    nodes = dag.topological_op_nodes()
    measure_node, store_node = nodes
    qubit_wire = DagWire.qubit(0)
    value_wire = DagWire.classical_value(measured.value)
    var_wire = DagWire.classical_var(flag)

    assert qubit_wire.kind == "qubit"
    assert qubit_wire.qubit_value == circuit.qubits[0]
    assert value_wire.classical_value_value == measured.value
    assert var_wire.classical_var_value == flag
    assert DagWire.global_order().kind == "global_order"

    assert dag.has_wire(qubit_wire)
    assert dag.has_wire(value_wire)
    assert dag.nodes_on_wire(value_wire) == [measure_node, store_node]
    assert dag.node_kind(measure_node) == "operation"
    assert dag.is_operation(measure_node)
    assert dag.wire_in(qubit_wire) in dag.predecessors(measure_node)
    assert dag.node_kind(dag.wire_in(qubit_wire)) == "wire_in"
    assert dag.node_kind(dag.wire_out(var_wire)) == "wire_out"
    assert dag.quantum_successors(measure_node) == []
    assert dag.classical_predecessors(store_node) == [measure_node]
    assert dag.predecessors_on_wire(store_node, value_wire) == [measure_node]


def test_dag_operation_returns_value_operation_with_parameters() -> None:
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    dag = circuit.dag()
    node = dag.topological_op_nodes()[0]
    operation = dag.operation(node)

    assert operation is not None
    assert operation.name == "RX"
    assert operation.params[0] == theta
    assert dag.parameters == [theta]
    assert dag.symbols == ["theta"]


def test_ordered_mapping_results_preserve_dag_order() -> None:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.x(1)
    circuit.cx(0, 1)
    circuit.z(0)
    circuit.s(1)

    dag = circuit.dag()

    assert list(dag.node_layers().keys()) == dag.topological_op_nodes()
    assert list(dag.operation_count_by_name().keys()) == ["H", "X", "CX", "Z", "S"]


def test_recursive_operation_counts_and_runs() -> None:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.x(0)
    circuit.cx(0, 1)
    circuit.cz(0, 1)
    counter = circuit.var(ClassicalType.uint(8))
    circuit.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 0),
        ClassicalExpr.uint_literal(8, 2),
        ClassicalExpr.uint_literal(8, 1),
        lambda body, _: body.z(1),
    )

    dag = circuit.dag()
    top_counts = dag.operation_count_by_name()
    recursive_counts = dag.operation_count_by_name_recursive()
    oneq_runs = dag.collect_1q_runs()
    twoq_runs = dag.collect_2q_runs()

    assert dag.has_control_flow()
    assert dag.has_nested_control_flow() is False
    assert top_counts["H"] == 1
    assert top_counts["for"] == 1
    assert "Z" not in top_counts
    assert recursive_counts["Z"] == 1
    assert any(len(run) == 2 for run in oneq_runs)
    assert any(len(run) == 2 for run in twoq_runs)


def test_control_flow_payload_exposes_child_dags() -> None:
    circuit = Circuit(1)
    flag = circuit.var(ClassicalType.bool())
    circuit.if_else(
        flag.expr(),
        lambda body: body.x(0),
        lambda body: body.z(0),
    )

    dag = circuit.dag()
    control = dag.control_flow(dag.topological_op_nodes()[0])

    assert control is not None
    assert control.kind == "if"
    assert control.then_body is not None
    assert control.else_body is not None
    assert [op.name for op in control.then_body.to_circuit().operations] == ["X"]
    assert [op.name for op in control.else_body.to_circuit().operations] == ["Z"]


def test_switch_control_flow_payload_exposes_cases_and_default() -> None:
    circuit = Circuit(1)
    target = circuit.var(ClassicalType.uint(8))

    circuit.switch(
        target.expr(),
        lambda case: (
            case.value(1, lambda body: body.x(0)),
            case.value(2, lambda body: body.y(0)),
            case.default(lambda body: body.z(0)),
        ),
    )

    dag = circuit.dag()
    control = dag.control_flow(dag.topological_op_nodes()[0])

    assert control is not None
    assert control.kind == "switch"
    assert [case.value for case in control.cases] == [1, 2]
    assert [op.name for op in control.cases[0].body.to_circuit().operations] == ["X"]
    assert control.default_body is not None
    assert [op.name for op in control.default_body.to_circuit().operations] == ["Z"]


def test_dag_editing_api_mutates_graph() -> None:
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    dag = circuit.dag()
    q0, q1 = dag.qubits

    front = ValueOperation.from_standard_gate(StandardGate.X, [q1])
    back = ValueOperation.from_standard_gate(StandardGate.Z, [q0])
    front_node = dag.apply_operation_front(front)
    back_node = dag.apply_operation_back(back)

    assert dag.operation(front_node).name == "X"
    assert dag.operation(back_node).name == "Z"
    assert [op.name for op in dag.to_circuit().operations] == ["X", "H", "CX", "Z"]

    nodes = dag.topological_op_nodes()
    removed = dag.remove_op_node(nodes[1])
    assert removed.name == "H"
    assert [op.name for op in dag.to_circuit().operations] == ["X", "CX", "Z"]

    nodes = dag.topological_op_nodes()
    replacement = ValueOperation.from_standard_gate(StandardGate.Y, [q1])
    dag.substitute_node(nodes[0], replacement)
    assert [op.name for op in dag.to_circuit().operations] == ["Y", "CX", "Z"]


def test_substitute_node_with_dag_from_python() -> None:
    circuit = Circuit(1)
    circuit.h(0)
    dag = circuit.dag()

    replacement_circuit = Circuit(1)
    replacement_circuit.x(0)
    replacement_circuit.z(0)
    replacement = replacement_circuit.dag()

    dag.substitute_node_with_dag(dag.topological_op_nodes()[0], replacement)

    assert [op.name for op in dag.to_circuit().operations] == ["X", "Z"]


def test_invalid_node_errors_are_mapped_to_circuit_error() -> None:
    circuit = Circuit(1)
    circuit.h(0)
    dag = circuit.dag()

    with pytest.raises(Exception, match="not in the DAG"):
        dag.quantum_predecessors(999_999)


def test_instruction_repr_round_trips_for_gates_and_delay() -> None:
    namespace = {
        "Instruction": Instruction,
        "MCGate": MCGate,
        "StandardGate": StandardGate,
    }
    cases = [
        Instruction.from_standard_gate(StandardGate.H),
        Instruction.from_name("CX"),
        Instruction.from_mc_gate(MCGate(2, StandardGate.X)),
        Instruction.delay(),
    ]

    for instruction in cases:
        rebuilt = eval(repr(instruction), dict(namespace))
        assert rebuilt.name == instruction.name
        assert rebuilt.instruction_type == instruction.instruction_type

    assert repr(cases[0]) == "Instruction.from_name('H')"
    assert repr(cases[2]) == "Instruction.from_mc_gate(MCGate(2, StandardGate.X))"
    assert repr(cases[3]) == "Instruction.delay()"


def test_instruction_from_name_resolves_standard_gates() -> None:
    assert Instruction.from_name("H").name == "H"
    assert Instruction.from_name("cx").name == "CX"

    with pytest.raises(ValueError, match="unknown standard gate"):
        Instruction.from_name("C2-X")
