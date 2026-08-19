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

"""Multi-qubit gate tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError, MCGate, StandardGate


@pytest.mark.parametrize(
    ("method", "expected_name"),
    [("cx", "CX"), ("cy", "CY"), ("cz", "CZ"), ("swap", "SWAP")],
)
def test_two_qubit_gate_methods(method, expected_name):
    circuit = Circuit(2)
    getattr(circuit, method)(0, 1)

    assert circuit[0].instruction.instruction.name == expected_name
    assert [qubit.index for qubit in circuit[0].qubits] == [0, 1]


@pytest.mark.parametrize(
    ("method", "expected_name"),
    [
        ("rxx", "RXX"),
        ("ryy", "RYY"),
        ("rzz", "RZZ"),
        ("rzx", "RZX"),
        ("crx", "CRX"),
        ("cry", "CRY"),
        ("crz", "CRZ"),
    ],
)
def test_two_qubit_parametric_gate_methods(method, expected_name):
    theta = Parameter("theta")
    circuit = Circuit(2)
    getattr(circuit, method)(0, 1, theta)

    assert circuit[0].instruction.instruction.name == expected_name
    assert list(circuit[0].params) == [theta]
    assert list(circuit.parameters) == [theta]


def test_two_qubit_gate_sequence_and_matrix_shape():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.rzz(0, 1, 0.25)

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "H",
        "CX",
        "RZZ",
    ]
    matrix = circuit.to_matrix()
    assert matrix.shape == (4, 4)
    assert np.allclose(matrix @ matrix.conj().T, np.eye(4), atol=1e-10)


def test_fsim_accepts_two_parameters():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(2)
    circuit.fsim(0, 1, theta, phi)

    assert circuit[0].instruction.instruction.name == "FSIM"
    assert list(circuit[0].params) == [theta, phi]
    assert list(circuit.parameters) == [theta, phi]


def test_ccx_and_explicit_multi_control_gate():
    circuit = Circuit(3)
    circuit.ccx(0, 1, 2)
    assert circuit[0].instruction.instruction.name == "CCX"
    assert [qubit.index for qubit in circuit[0].qubits] == [0, 1, 2]

    controlled_h = MCGate(2, StandardGate.H())
    circuit.append_mc_gate(controlled_h, [0, 1, 2])
    assert circuit[1].instruction.instruction.is_mcgate
    assert circuit[1].instruction.instruction.name == "C2-H"


@pytest.mark.parametrize("method", ["cx", "cy", "cz", "swap"])
def test_two_qubit_gates_reject_duplicate_qubits(method):
    circuit = Circuit(2)
    with pytest.raises(CircuitError):
        getattr(circuit, method)(0, 0)


def test_multi_qubit_gates_reject_unknown_qubits():
    circuit = Circuit(2)

    with pytest.raises(CircuitError):
        circuit.cx(0, 3)

    with pytest.raises(CircuitError):
        circuit.ccx(0, 1, 3)
