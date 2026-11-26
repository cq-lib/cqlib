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

"""Decomposition tests for the Python circuit API."""

from cqlib import Circuit, Parameter
from cqlib.qis import DensityMatrix, Statevector
import numpy as np
import pytest


@pytest.mark.parametrize(
    "width,has_operations", [(0, False), (1, False), (1, True), (2, True)]
)
@pytest.mark.parametrize("repetitions", [0, 1, 2])
def test_decompose_preserves_global_phase_of_each_definition(
    width, has_operations, repetitions
):
    inner = Circuit(width)
    inner.set_global_phase(0.37)
    if has_operations:
        inner.x(0)
        if width == 2:
            inner.cx(0, 1)
    gate = inner.to_gate("Phased")
    outer = Circuit(2)
    outer.set_global_phase(0.12)
    expected = Circuit(2)
    expected.set_global_phase(0.12 + repetitions * 0.37)
    for _ in range(repetitions):
        outer.append_circuit_gate(gate, [1, 0][:width])
        if has_operations:
            expected.x(1)
            if width == 2:
                expected.cx(1, 0)
    flat = outer.decompose()
    assert flat.global_phase.evaluate() == pytest.approx(0.12 + repetitions * 0.37)
    for candidate in (outer, flat, flat.decompose()):
        np.testing.assert_allclose(
            candidate.to_matrix(), expected.to_matrix(), rtol=0, atol=1e-10
        )
    assert len(flat) == len(expected)
    assert outer.global_phase.evaluate() == 0.12
    reference_state = expected.to_matrix()[:, 0]
    np.testing.assert_allclose(
        Statevector.from_circuit(outer).data, reference_state, rtol=0, atol=1e-10
    )
    np.testing.assert_allclose(
        DensityMatrix.from_circuit(outer).data,
        np.outer(reference_state, reference_state.conj()),
        rtol=0,
        atol=1e-10,
    )


@pytest.mark.parametrize("inverse", [False, True])
def test_decompose_preserves_nested_symbolic_global_phase(inverse):
    theta = Parameter("theta")
    inner = Circuit([7, 2])
    inner.set_global_phase(theta / 2)
    inner.rx(7, theta)
    inner.cx(7, 2)
    middle = Circuit([5, 1])
    middle.set_global_phase(theta.sin())
    middle.append_circuit_gate(inner.to_gate("Inner"), [5, 1], [theta + 1])
    outer = Circuit(2)
    x = Parameter("x")
    outer.set_global_phase(x / 3)
    outer.append_circuit_gate(middle.to_gate("Middle"), [1, 0], [2 * x])
    if inverse:
        outer = outer.inverse()
    flat = outer.decompose()
    symbolic = flat.to_symbolic_matrix()
    for value in (0.37, -0.61):
        phase = value / 3 + np.sin(2 * value) + (2 * value + 1) / 2
        expected = Circuit(2)
        expected.set_global_phase(phase)
        expected.rx(1, 2 * value + 1)
        expected.cx(1, 0)
        if inverse:
            expected = expected.inverse()
            phase = -phase
        bindings = {"x": value}
        reference = expected.to_matrix()
        assert flat.global_phase.evaluate(bindings) == pytest.approx(phase)
        for candidate in (
            flat.assign_parameters(bindings),
            outer.assign_parameters(bindings).decompose(),
            outer.assign_parameters(bindings),
        ):
            np.testing.assert_allclose(
                candidate.to_matrix(), reference, rtol=0, atol=1e-10
            )
        np.testing.assert_allclose(
            symbolic.evaluate(bindings), reference, rtol=0, atol=1e-10
        )


def test_decompose_keeps_symbols_used_only_by_global_phase():
    inner = Circuit(1)
    inner.set_global_phase(Parameter("theta"))
    outer = Circuit(1)
    outer.set_global_phase(Parameter("phi"))
    outer.append_circuit_gate(inner.to_gate("PhaseOnly"), [0], [2 * Parameter("x")])
    flat = outer.decompose()
    assert len(flat) == 0
    assert set(flat.used_symbols) == {"phi", "x"}
    bindings = {"phi": 0.12, "x": 0.37}
    assert flat.global_phase.evaluate(bindings) == pytest.approx(0.86)
    np.testing.assert_allclose(
        flat.assign_parameters(bindings).to_matrix(),
        np.exp(0.86j) * np.eye(2),
        rtol=0,
        atol=1e-10,
    )


def test_decompose_keeps_standard_gates_unchanged():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)

    decomposed = circuit.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == [
        "H",
        "CX",
    ]


def test_decompose_expands_circuit_gate():
    subcircuit = Circuit(2)
    subcircuit.h(0)
    subcircuit.cx(0, 1)
    gate = subcircuit.to_gate("Bell")

    circuit = Circuit(2)
    circuit.append_circuit_gate(gate, [0, 1])

    decomposed = circuit.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == [
        "H",
        "CX",
    ]


def test_decompose_multiple_circuit_gates_preserves_order():
    first = Circuit(1)
    first.x(0)
    second = Circuit(1)
    second.h(0)

    circuit = Circuit(1)
    circuit.append_circuit_gate(first.to_gate("First"), [0])
    circuit.append_circuit_gate(second.to_gate("Second"), [0])

    decomposed = circuit.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == [
        "X",
        "H",
    ]


def test_decompose_preserves_parameters_in_circuit_gate():
    theta = Parameter("theta")
    subcircuit = Circuit(1)
    subcircuit.rx(0, theta)
    gate = subcircuit.to_gate("ParamBlock")

    circuit = Circuit(1)
    circuit.append_circuit_gate(gate, [0], [0.75])

    decomposed = circuit.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == ["RX"]
    assert list(decomposed[0].params) == [0.75]


def test_decompose_keeps_top_level_barrier():
    subcircuit = Circuit(1)
    subcircuit.x(0)

    circuit = Circuit(1)
    circuit.h(0)
    circuit.barrier([0])
    circuit.append_circuit_gate(subcircuit.to_gate("Flip"), [0])

    decomposed = circuit.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == [
        "H",
        "barrier",
        "X",
    ]
