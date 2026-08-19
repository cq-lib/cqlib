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

"""Classical control-flow tests for the Python circuit API."""

import pytest

from cqlib import Circuit
from cqlib.circuit import (
    CircuitError,
    ClassicalControlOp,
    ClassicalExpr,
    ClassicalType,
    ValueControlBody,
    ValueOperation,
    ValueSwitchCase,
)


def test_if_control_flow_from_callback():
    circuit = Circuit(1)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

    control = circuit[0].instruction.classical_control
    assert control.kind == "if"
    assert str(control.condition) == "ClassicalExpr(BoolLiteral(true))"
    assert [op.instruction.instruction.name for op in control.then_body.operations] == [
        "X"
    ]


def test_if_else_control_flow_from_callbacks():
    circuit = Circuit(1)
    circuit.if_else(
        ClassicalExpr.bool_literal(False),
        lambda body: body.x(0),
        lambda body: body.z(0),
    )

    control = circuit[0].instruction.classical_control
    assert control.kind == "if"
    assert [op.instruction.instruction.name for op in control.then_body.operations] == [
        "X"
    ]
    assert [op.instruction.instruction.name for op in control.else_body.operations] == [
        "Z"
    ]


def test_while_control_flow_from_callback():
    circuit = Circuit(1)
    circuit.while_(ClassicalExpr.bool_literal(True), lambda body: body.h(0))

    control = circuit[0].instruction.classical_control
    assert control.kind == "while"
    assert [op.instruction.instruction.name for op in control.body.operations] == ["H"]


def test_for_uint_control_flow_uses_loop_expression():
    circuit = Circuit(1)
    loop_var = circuit.var(ClassicalType.uint(3))

    def body(builder, index_expr):
        assert str(index_expr.ty) == "ClassicalType.uint(3)"
        builder.rx(0, 0.25)

    circuit.for_uint(
        loop_var,
        ClassicalExpr.uint_literal(3, 0),
        ClassicalExpr.uint_literal(3, 3),
        ClassicalExpr.uint_literal(3, 1),
        body,
    )

    control = circuit[0].instruction.classical_control
    assert control.kind == "for"
    assert str(control.start) == "ClassicalExpr(UIntLiteral { width: 3, value: 0 })"
    assert [op.instruction.instruction.name for op in control.body.operations] == ["RX"]


def test_switch_control_flow_from_builder():
    circuit = Circuit(1)

    def build_switch(builder):
        builder.value(1, lambda body: body.x(0))
        builder.default(lambda body: body.z(0))

    circuit.switch(ClassicalExpr.uint_literal(2, 1), build_switch)

    control = circuit[0].instruction.classical_control
    assert control.kind == "switch"
    assert len(control.cases) == 1
    assert control.cases[0].value == 1
    assert [
        op.instruction.instruction.name for op in control.cases[0].body.operations
    ] == ["X"]
    assert [op.instruction.instruction.name for op in control.default.operations] == [
        "Z"
    ]


def test_explicit_classical_control_operation_can_be_appended():
    body_circuit = Circuit(1)
    body_circuit.y(0)
    control = ClassicalControlOp.if_(
        ClassicalExpr.bool_literal(True),
        ValueControlBody(body_circuit.operations),
    )

    circuit = Circuit(1)
    circuit.append(ValueOperation.from_classical_control(control))

    assert circuit[0].instruction.is_classical_control
    assert circuit[0].instruction.classical_control.kind == "if"
    assert [
        op.instruction.instruction.name
        for op in circuit[0].instruction.classical_control.then_body.operations
    ] == ["Y"]


def test_measurement_and_control_flow_order():
    circuit = Circuit(1)
    circuit.measure(0)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

    assert [
        op.instruction.classical_control.kind
        if op.instruction.is_classical_control
        else op.instruction.instruction.name
        for op in circuit.operations
    ] == ["measure_bit", "if"]


def test_control_flow_callback_failure_does_not_append_operation():
    circuit = Circuit(1)

    def failing_body(body):
        body.x(0)
        raise RuntimeError("fail during body construction")

    with pytest.raises(RuntimeError):
        circuit.if_(ClassicalExpr.bool_literal(True), failing_body)

    assert circuit.operations == []


def test_control_flow_rejects_body_with_unknown_qubit():
    body_circuit = Circuit([1])
    body_circuit.x(1)
    control = ClassicalControlOp.if_(
        ClassicalExpr.bool_literal(True),
        ValueControlBody(body_circuit.operations),
    )

    circuit = Circuit(1)
    with pytest.raises(CircuitError):
        circuit.append(ValueOperation.from_classical_control(control))


def test_for_uint_factory_rejects_mismatched_stop_type():
    circuit = Circuit(1)
    loop_var = circuit.var(ClassicalType.uint(3))
    with pytest.raises(CircuitError, match="stop type"):
        ClassicalControlOp.for_uint(
            loop_var,
            ClassicalExpr.uint_literal(3, 0),
            ClassicalExpr.uint_literal(4, 3),
            ClassicalExpr.uint_literal(3, 1),
            ValueControlBody([]),
        )


def test_for_uint_factory_rejects_mismatched_step_type():
    circuit = Circuit(1)
    loop_var = circuit.var(ClassicalType.uint(3))
    with pytest.raises(CircuitError, match="step type"):
        ClassicalControlOp.for_uint(
            loop_var,
            ClassicalExpr.uint_literal(3, 0),
            ClassicalExpr.uint_literal(3, 3),
            ClassicalExpr.uint_literal(4, 1),
            ValueControlBody([]),
        )


def test_switch_factory_rejects_case_value_out_of_width():
    with pytest.raises(CircuitError, match="does not fit"):
        ClassicalControlOp.switch(
            ClassicalExpr.uint_literal(3, 0),
            [ValueSwitchCase(8, ValueControlBody([]))],
        )


def test_switch_factory_rejects_duplicate_case_values():
    with pytest.raises(CircuitError, match="duplicate switch case value"):
        ClassicalControlOp.switch(
            ClassicalExpr.uint_literal(3, 0),
            [
                ValueSwitchCase(1, ValueControlBody([])),
                ValueSwitchCase(1, ValueControlBody([])),
            ],
        )


def test_control_factories_accept_valid_arguments():
    circuit = Circuit(1)
    loop_var = circuit.var(ClassicalType.uint(3))
    for_op = ClassicalControlOp.for_uint(
        loop_var,
        ClassicalExpr.uint_literal(3, 0),
        ClassicalExpr.uint_literal(3, 3),
        ClassicalExpr.uint_literal(3, 1),
        ValueControlBody([]),
    )
    assert for_op.kind == "for"

    switch_op = ClassicalControlOp.switch(
        ClassicalExpr.uint_literal(3, 1),
        [ValueSwitchCase(1, ValueControlBody([]))],
        ValueControlBody([]),
    )
    assert switch_op.kind == "switch"
    assert switch_op.cases[0].value == 1
