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
