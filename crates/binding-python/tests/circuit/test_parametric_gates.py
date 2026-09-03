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

"""Parametric gate tests for the Python circuit API."""

import math

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError, ParameterError


@pytest.mark.parametrize(
    ("method", "expected_name"),
    [("rx", "RX"), ("ry", "RY"), ("rz", "RZ"), ("phase", "Phase")],
)
def test_single_parameter_rotation_methods(method, expected_name):
    circuit = Circuit(1)
    getattr(circuit, method)(0, math.pi / 4)

    assert circuit[0].instruction.instruction.name == expected_name
    assert list(circuit[0].params) == [math.pi / 4]


def test_rotation_gates_produce_unitary_matrices():
    circuit = Circuit(1)
    circuit.rx(0, 0.125)
    circuit.ry(0, -0.25)
    circuit.rz(0, 0.5)

    matrix = circuit.to_matrix()
    assert np.allclose(matrix @ matrix.conj().T, np.eye(2), atol=1e-10)


def test_symbolic_parameter_tracking_and_assignment():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    assert list(circuit.parameters) == [theta]

    assigned = circuit.assign_parameters({"theta": math.pi})
    assert assigned[0].instruction.instruction.name == "RX"
    assert list(assigned[0].params) == [math.pi]
    assert list(circuit[0].params) == [theta]
    assert assigned.parameters == []


def test_partial_assignment_leaves_unbound_parameters():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(1)
    circuit.rx(0, theta)
    circuit.ry(0, phi)

    assigned = circuit.assign_parameters({"theta": 0.5})
    assert list(assigned[0].params) == [0.5]
    assert list(assigned[1].params) == [phi]
    assert list(assigned.parameters) == [phi]


def test_parameter_expressions_are_assigned_recursively():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(1)
    circuit.rz(0, 2 * theta + phi)

    assigned = circuit.assign_parameters({"theta": 0.25, "phi": 0.5})
    assert np.isclose(assigned[0].params[0], 1.0)


def test_u_and_rxy_gates_accept_numeric_and_symbolic_parameters():
    theta = Parameter("theta")
    phi = Parameter("phi")
    lam = Parameter("lam")
    circuit = Circuit(2)
    circuit.u(0, theta, 0.1, lam)
    circuit.rxy(1, 0.2, phi)

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "U",
        "RXY",
    ]
    assert list(circuit[0].params) == [theta, 0.1, lam]
    assert list(circuit[1].params) == [0.2, phi]
    assert list(circuit.parameters) == [theta, lam, phi]


def test_unbound_symbolic_matrix_conversion_raises_parameter_error():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    with pytest.raises(CircuitError):
        circuit.to_matrix()


@pytest.mark.parametrize("bad_value", [float("nan"), float("inf"), -float("inf")])
def test_non_finite_gate_parameters_are_rejected(bad_value):
    circuit = Circuit(1)
    with pytest.raises(ParameterError):
        circuit.rx(0, bad_value)
