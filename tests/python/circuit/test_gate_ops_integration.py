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

"""Integration tests for basic circuit gate-construction APIs (see issue #4).

These cover construction and the ``to_matrix`` semantics of the core gate
operations. Multi-qubit assertions are stated as convention-independent
properties (unitarity, shape, recorded operation names) so they hold regardless
of the internal qubit-ordering endianness.
"""

import numpy as np
import pytest

from cqlib import Circuit, Parameter

# Reference single-qubit gate matrices (standard, unambiguous).
_H = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)
_X = np.array([[0, 1], [1, 0]], dtype=complex)
_Y = np.array([[0, -1j], [1j, 0]], dtype=complex)
_Z = np.array([[1, 0], [0, -1]], dtype=complex)


def _is_unitary(matrix: np.ndarray) -> bool:
    dim = matrix.shape[0]
    return np.allclose(matrix @ matrix.conj().T, np.eye(dim), atol=1e-10)


def _op_names(circuit: Circuit) -> list[str]:
    return [op.instruction.instruction.name for op in circuit.operations]


class TestSingleQubitGates:
    @pytest.mark.parametrize(
        "apply_gate, reference",
        [
            (lambda c: c.h(0), _H),
            (lambda c: c.x(0), _X),
            (lambda c: c.y(0), _Y),
            (lambda c: c.z(0), _Z),
        ],
    )
    def test_single_gate_matrix_matches_reference(self, apply_gate, reference):
        circuit = Circuit(1)
        apply_gate(circuit)
        assert circuit.num_qubits == 1
        assert len(circuit.operations) == 1
        assert np.allclose(circuit.to_matrix(), reference, atol=1e-10)

    def test_double_hadamard_is_identity(self):
        circuit = Circuit(1)
        circuit.h(0)
        circuit.h(0)
        assert np.allclose(circuit.to_matrix(), np.eye(2), atol=1e-10)


class TestBellCircuit:
    def test_construction_records_operations(self):
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)
        assert circuit.num_qubits == 2
        names = _op_names(circuit)
        assert names[0] == "H"
        assert names[1] in {"CX", "CNOT"}

    def test_matrix_is_unitary_with_expected_shape(self):
        circuit = Circuit(2)
        circuit.h(0)
        circuit.cx(0, 1)
        matrix = circuit.to_matrix()
        assert matrix.shape == (4, 4)
        assert _is_unitary(matrix)


class TestParameterizedRotation:
    def test_rx_zero_angle_is_identity(self):
        circuit = Circuit(1)
        circuit.rx(0, 0.0)
        assert np.allclose(circuit.to_matrix(), np.eye(2), atol=1e-10)

    def test_rx_is_unitary_for_arbitrary_angle(self):
        circuit = Circuit(1)
        circuit.rx(0, 1.2345)
        assert _is_unitary(circuit.to_matrix())

    def test_symbolic_rx_assign_parameters(self):
        theta = Parameter("theta")
        circuit = Circuit(1)
        circuit.rx(0, theta)

        bound_zero = circuit.assign_parameters({"theta": 0.0})
        assert np.allclose(bound_zero.to_matrix(), np.eye(2), atol=1e-10)

        # RX(pi) == -i * X
        bound_pi = circuit.assign_parameters({"theta": np.pi})
        assert np.allclose(bound_pi.to_matrix(), -1j * _X, atol=1e-10)


class TestCircuitDimensions:
    @pytest.mark.parametrize("n", [1, 2, 3])
    def test_to_matrix_shape_scales_with_qubits(self, n):
        circuit = Circuit(n)
        for q in range(n):
            circuit.h(q)
        matrix = circuit.to_matrix()
        assert matrix.shape == (2**n, 2**n)
        assert _is_unitary(matrix)

    def test_empty_circuit_has_no_operations(self):
        circuit = Circuit(0)
        assert circuit.num_qubits == 0
        assert len(circuit.operations) == 0
