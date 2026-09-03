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

"""Single-qubit gate tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError


@pytest.mark.parametrize(
    ("method", "expected_name"),
    [
        ("i", "I"),
        ("x", "X"),
        ("y", "Y"),
        ("z", "Z"),
        ("h", "H"),
        ("s", "S"),
        ("sdg", "SDG"),
        ("t", "T"),
        ("tdg", "TDG"),
        ("x2p", "X2P"),
        ("x2m", "X2M"),
        ("y2p", "Y2P"),
        ("y2m", "Y2M"),
    ],
)
def test_single_qubit_gate_methods_append_expected_instruction(method, expected_name):
    circuit = Circuit(1)
    getattr(circuit, method)(0)

    assert len(circuit.operations) == 1
    assert circuit[0].instruction.instruction.name == expected_name
    assert [qubit.index for qubit in circuit[0].qubits] == [0]


def test_single_qubit_gate_order_is_preserved():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.x(0)
    circuit.z(0)

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "H",
        "X",
        "Z",
    ]


def test_hadamard_and_pauli_x_matrices():
    hadamard = Circuit(1)
    hadamard.h(0)
    assert np.allclose(
        hadamard.to_matrix(),
        np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2),
    )

    pauli_x = Circuit(1)
    pauli_x.x(0)
    assert np.allclose(pauli_x.to_matrix(), np.array([[0, 1], [1, 0]], dtype=complex))


def test_square_root_gates_are_unitary():
    for method in ("x2p", "x2m", "y2p", "y2m"):
        circuit = Circuit(1)
        getattr(circuit, method)(0)
        matrix = circuit.to_matrix()
        assert np.allclose(matrix @ matrix.conj().T, np.eye(2), atol=1e-10)


@pytest.mark.parametrize(
    ("method", "expected_name", "params"),
    [
        ("xy", "XY", [0.1]),
        ("xy2p", "XY2P", [0.3]),
        ("xy2m", "XY2M", [0.4]),
    ],
)
def test_xy_family_gates(method, expected_name, params):
    circuit = Circuit(1)
    getattr(circuit, method)(0, *params)

    assert circuit[0].instruction.instruction.name == expected_name
    assert list(circuit[0].params) == params


def test_xy_gate_accepts_symbolic_parameters():
    theta = Parameter("theta")
    circuit = Circuit(1)
    circuit.xy(0, theta)

    assert circuit[0].instruction.instruction.name == "XY"
    assert list(circuit[0].params) == [theta]
    assert list(circuit.parameters) == [theta]


def test_single_qubit_gate_rejects_unknown_qubit():
    circuit = Circuit(1)
    with pytest.raises(CircuitError):
        circuit.h(3)
