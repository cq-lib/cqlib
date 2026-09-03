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

"""Inverse operation tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError


def test_unitary_circuit_inverse_multiplies_to_identity():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.rz(1, 0.25)

    inverse = circuit.inverse()
    product = inverse.to_matrix() @ circuit.to_matrix()

    assert np.allclose(product, np.eye(4), atol=1e-10)


def test_inverse_reverses_operation_order():
    circuit = Circuit(1)
    circuit.rx(0, 0.1)
    circuit.ry(0, 0.2)
    circuit.rz(0, 0.3)

    inverse = circuit.inverse()

    assert [op.instruction.instruction.name for op in inverse.operations] == [
        "RZ",
        "RY",
        "RX",
    ]
    assert list(inverse[0].params) == [-0.3]


def test_barrier_is_preserved_in_inverse():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.barrier([0])
    circuit.x(0)

    inverse = circuit.inverse()

    assert [op.instruction.instruction.name for op in inverse.operations] == [
        "X",
        "barrier",
        "H",
    ]


def test_non_unitary_operations_are_not_invertible():
    measured = Circuit(1)
    measured.measure(0)
    with pytest.raises(CircuitError):
        measured.inverse()

    reset = Circuit(1)
    reset.reset(0)
    with pytest.raises(CircuitError):
        reset.inverse()


def test_symbolic_inverse_keeps_symbolic_parameters():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.rx(0, theta)

    inverse = circuit.inverse()

    assert inverse[0].instruction.instruction.name == "RX"
    assert str(inverse[0].params[0]) == "-theta"


def test_empty_circuit_inverse_is_empty():
    circuit = Circuit(0)
    inverse = circuit.inverse()

    assert inverse.num_qubits == 0
    assert len(inverse.operations) == 0
