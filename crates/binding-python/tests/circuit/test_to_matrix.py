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

"""Matrix conversion tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError, circuit_to_matrix


def test_empty_circuit_matrix_is_scalar_identity():
    circuit = Circuit(0)
    assert np.allclose(circuit.to_matrix(), np.array([[1]], dtype=complex))


def test_hadamard_matrix():
    circuit = Circuit(1)
    circuit.h(0)

    assert np.allclose(
        circuit.to_matrix(),
        np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2),
    )


def test_bell_circuit_matrix_is_unitary():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)

    matrix = circuit.to_matrix()

    assert matrix.shape == (4, 4)
    assert np.allclose(matrix @ matrix.conj().T, np.eye(4), atol=1e-10)


def test_circuit_to_matrix_function_matches_method():
    circuit = Circuit(1)
    circuit.rx(0, 0.125)
    circuit.rz(0, 0.25)

    assert np.allclose(circuit_to_matrix(circuit), circuit.to_matrix())


def test_custom_qubit_order_changes_matrix_layout():
    circuit = Circuit([0, 2])
    circuit.cx(0, 2)

    normal_order = circuit.to_matrix()
    reversed_order = circuit.to_matrix([2, 0])

    assert normal_order.shape == (4, 4)
    assert reversed_order.shape == (4, 4)
    assert not np.allclose(normal_order, reversed_order)


def test_unbound_parameter_matrix_conversion_raises():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    with pytest.raises(CircuitError):
        circuit.to_matrix()


def test_non_unitary_matrix_conversion_raises():
    circuit = Circuit(1)
    circuit.measure(0)

    with pytest.raises(CircuitError):
        circuit.to_matrix()
