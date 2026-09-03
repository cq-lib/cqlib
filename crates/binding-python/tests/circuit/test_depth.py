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

"""Tests for ``Circuit.depth``."""

import pytest

from cqlib import Circuit
from cqlib.circuit import CircuitError, ClassicalExpr, ClassicalType


def test_empty_circuit_depth_zero():
    assert Circuit(3).depth() == 0


def test_single_qubit_chain_depth_three():
    c = Circuit(1)
    c.h(0)
    c.x(0)
    c.z(0)
    assert c.depth() == 3


def test_parallel_single_qubit_gates_depth_one():
    c = Circuit(3)
    c.h(0)
    c.h(1)
    c.h(2)
    assert c.depth() == 1


def test_cx_chain_depth_two():
    c = Circuit(3)
    c.cx(0, 1)
    c.cx(1, 2)
    assert c.depth() == 2


def test_barrier_forces_serialization():
    c = Circuit(2)
    c.h(0)
    c.h(1)
    c.barrier([0, 1])
    c.h(0)
    c.h(1)
    # layer + barrier + layer = 3
    assert c.depth() == 3


def test_global_barrier_serializes_all_qubits():
    c = Circuit(2)
    c.h(0)
    # Empty-qubit barrier is global: synchronizes q0 and q1.
    c.barrier([])
    c.h(1)
    assert c.depth() == 3


def test_measure_and_reset_count_as_layers():
    c = Circuit(1)
    c.measure(0)
    c.reset(0)
    assert c.depth() == 2


def test_circuit_gate_is_opaque():
    inner = Circuit(2)
    inner.h(0)
    inner.cx(0, 1)
    gate = inner.to_gate("mygate")

    c = Circuit(2)
    c.append_circuit_gate(gate, [0, 1], None)
    # The sub-circuit has depth 2, but as an opaque gate it counts as 1.
    assert c.depth() == 1
    # Decomposing first surfaces the internal depth.
    assert c.decompose().depth() == 2


def test_depth_default_is_no_recurse():
    c = Circuit(1)
    c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))
    # `depth()` with no argument must behave like recurse=False.
    with pytest.raises(CircuitError):
        c.depth()


def test_recurse_false_raises_on_control_flow():
    c = Circuit(1)
    c.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))
    with pytest.raises(CircuitError):
        c.depth(recurse=False)


def test_recurse_false_raises_on_nested_control_flow():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(4))

    def outer(body, _index):
        body.if_(ClassicalExpr.bool_literal(True), lambda inner: inner.x(0))

    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(4, 0),
        ClassicalExpr.uint_literal(4, 2),
        ClassicalExpr.uint_literal(4, 1),
        outer,
    )
    with pytest.raises(CircuitError):
        c.depth(recurse=False)


def test_recurse_true_if_else_takes_max_branch():
    c = Circuit(1)
    c.if_else(
        ClassicalExpr.bool_literal(True),
        lambda then: (then.x(0), then.z(0)),  # depth 2
        lambda otherwise: otherwise.y(0),  # depth 1
    )
    # 1 + max(2, 1) = 3
    assert c.depth(recurse=True) == 3


def test_recurse_true_while_counts_body_once():
    c = Circuit(1)

    def body(b):
        b.h(0)
        b.x(0)
        b.break_loop()

    c.while_(ClassicalExpr.bool_literal(True), body)
    # 1 + body_depth(2) = 3
    assert c.depth(recurse=True) == 3


def test_recurse_true_for_uint_unrolled():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))
    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 0),
        ClassicalExpr.uint_literal(8, 3),
        ClassicalExpr.uint_literal(8, 1),
        lambda body, _i: body.h(0),
    )
    # 1 + 3 iterations * body_depth(1) = 4
    assert c.depth(recurse=True) == 4


def test_recurse_true_for_var_range_falls_back_to_once():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))
    runtime_start = c.var(ClassicalType.uint(8))
    c.for_uint(
        counter,
        runtime_start.expr(),
        ClassicalExpr.uint_literal(8, 3),
        ClassicalExpr.uint_literal(8, 1),
        lambda body, _i: body.h(0),
    )
    # Non-static start -> body counted once -> 1 + 1 = 2
    assert c.depth(recurse=True) == 2


def test_recurse_true_for_step_zero_falls_back_to_once():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))
    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 0),
        ClassicalExpr.uint_literal(8, 3),
        ClassicalExpr.uint_literal(8, 0),  # step == 0
        lambda body, _i: body.h(0),
    )
    # step 0 -> body counted once -> 1 + 1 = 2
    assert c.depth(recurse=True) == 2


def test_recurse_true_for_empty_range():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))
    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 5),
        ClassicalExpr.uint_literal(8, 2),
        ClassicalExpr.uint_literal(8, 1),
        lambda body, _i: body.h(0),
    )
    # Empty range -> 0 iterations -> 1 + 0 = 1
    assert c.depth(recurse=True) == 1


def test_recurse_true_for_uneven_step_uses_ceil():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))
    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 0),
        ClassicalExpr.uint_literal(8, 5),
        ClassicalExpr.uint_literal(8, 2),
        lambda body, _i: body.h(0),
    )
    # ceil((5-0)/2) = 3 iterations (0, 2, 4) -> 1 + 3 = 4
    assert c.depth(recurse=True) == 4


def test_recurse_true_switch_takes_max_branch():
    c = Circuit(1)
    state = c.var(ClassicalType.uint(2))

    def build(builder):
        builder.value(0, lambda body: body.x(0))  # depth 1
        builder.value(1, lambda body: (body.x(0), body.z(0)))  # depth 2
        builder.default(lambda body: body.y(0))  # depth 1

    c.switch(state.expr(), build)
    # 1 + max(1, 2, 1) = 3
    assert c.depth(recurse=True) == 3


def test_recurse_true_break_contributes_zero():
    c = Circuit(1)
    c.while_(ClassicalExpr.bool_literal(True), lambda body: body.break_loop())
    # break is depth 0 -> 1 + 0 = 1
    assert c.depth(recurse=True) == 1


def test_recurse_true_nested_if_in_for():
    c = Circuit(1)
    counter = c.var(ClassicalType.uint(8))

    def outer(body, _i):
        body.if_(
            ClassicalExpr.bool_literal(True),
            lambda inner: (inner.x(0), inner.y(0)),  # depth 2
        )

    c.for_uint(
        counter,
        ClassicalExpr.uint_literal(8, 0),
        ClassicalExpr.uint_literal(8, 2),
        ClassicalExpr.uint_literal(8, 1),
        outer,
    )
    # if local = 1 + 2 = 3; for body depth = 3; 2 iterations -> 1 + 2*3 = 7
    assert c.depth(recurse=True) == 7


def test_recurse_true_control_flow_synchronizes_union_qubits():
    c = Circuit(4)
    c.if_else(
        ClassicalExpr.bool_literal(True),
        lambda then: then.cx(0, 1),  # uses q0, q1
        lambda otherwise: otherwise.cx(2, 3),  # uses q2, q3
    )
    c.h(0)
    # The if occupies {0,1,2,3}; h(0) waits on q0 -> 1 + max(1,1) + 1 = 3.
    assert c.depth(recurse=True) == 3
