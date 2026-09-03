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

"""Parameter assignment tests for the Python circuit API."""

import math
import numpy as np
import pytest

from cqlib import Circuit, Parameter


def test_assign_single_parameter():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    assigned = circuit.assign_parameters({"theta": math.pi / 2})

    assert list(assigned[0].params) == [math.pi / 2]
    assert list(assigned.parameters) == []
    assert list(circuit[0].params) == [theta]


def test_assign_multiple_parameters():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(1)
    circuit.rx(0, theta)
    circuit.ry(0, phi)

    assigned = circuit.assign_parameters({"theta": 0.25, "phi": 0.5})

    assert [list(op.params) for op in assigned.operations] == [[0.25], [0.5]]
    assert assigned.parameters == []


def test_partial_assignment_keeps_remaining_symbols():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(1)
    circuit.rx(0, theta)
    circuit.rz(0, theta + phi)

    assigned = circuit.assign_parameters({"theta": 0.25})

    assert list(assigned[0].params) == [0.25]
    assert str(assigned[1].params[0]) == "0.25 + phi"
    assert [str(param) for param in assigned.parameters] == ["0.25 + phi"]


def test_empty_assignment_returns_equivalent_circuit():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    assigned = circuit.assign_parameters({})

    assert list(assigned[0].params) == [theta]
    assert list(assigned.parameters) == [theta]
    assert assigned is not circuit


def test_expression_assignment_updates_matrix_result():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, 2 * theta)

    assigned = circuit.assign_parameters({"theta": math.pi / 4})
    expected = Circuit(1)
    expected.rx(0, math.pi / 2)

    assert np.allclose(assigned.to_matrix(), expected.to_matrix())


def test_multiple_assignments_are_independent():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    first = circuit.assign_parameters({"theta": 0.1})
    second = circuit.assign_parameters({"theta": 0.2})

    assert list(first[0].params) == [0.1]
    assert list(second[0].params) == [0.2]
    assert list(circuit[0].params) == [theta]


def test_assignment_rejects_non_numeric_values():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    with pytest.raises(TypeError):
        circuit.assign_parameters({"theta": object()})
