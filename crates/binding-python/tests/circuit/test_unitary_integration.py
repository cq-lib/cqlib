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

"""Custom UnitaryGate integration tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import (
    CircuitError,
    FrozenCircuit,
    SymbolicComplex,
    SymbolicMatrix,
    UnitaryGate,
)


def test_custom_single_qubit_unitary_gate():
    x_gate = UnitaryGate("XLike", 1).with_matrix(
        np.array([[0, 1], [1, 0]], dtype=complex)
    )

    circuit = Circuit(1)
    circuit.append_unitary_gate(x_gate, [0])

    assert circuit[0].instruction.instruction.name == "XLike"
    assert np.allclose(circuit.to_matrix(), np.array([[0, 1], [1, 0]], dtype=complex))


def test_custom_two_qubit_unitary_gate():
    swap = UnitaryGate("SwapLike", 2).with_matrix(
        np.array(
            [
                [1, 0, 0, 0],
                [0, 0, 1, 0],
                [0, 1, 0, 0],
                [0, 0, 0, 1],
            ],
            dtype=complex,
        )
    )

    circuit = Circuit(2)
    circuit.append_unitary_gate(swap, [0, 1])

    assert circuit[0].instruction.instruction.name == "SwapLike"
    assert circuit.to_matrix().shape == (4, 4)


def test_symbolic_custom_unitary_gate_with_parameters():
    theta = Parameter("theta")
    symbolic = SymbolicMatrix(
        [
            [SymbolicComplex.one(), SymbolicComplex.zero()],
            [SymbolicComplex.zero(), SymbolicComplex.exp_i(theta)],
        ]
    )
    gate = UnitaryGate("PhaseLike", 1, 1).with_symbolic_matrix(symbolic, ["theta"])

    circuit = Circuit(1)
    circuit.append_unitary_gate(gate, [0], [0.5])

    expected = np.array(
        [[1, 0], [0, np.cos(0.5) + 1j * np.sin(0.5)]],
        dtype=complex,
    )
    assert gate.matrix_params == ["theta"]
    assert np.allclose(circuit.to_matrix(), expected)


def test_unitary_gate_backed_by_circuit():
    subcircuit = Circuit(1)
    subcircuit.h(0)
    frozen = FrozenCircuit(subcircuit.qubits, subcircuit.operations)
    gate = UnitaryGate("HadamardLike", 1).with_circuit(frozen)

    circuit = Circuit(1)
    circuit.append_unitary_gate(gate, [0])

    assert gate.circuit.num_operations == 1
    assert np.allclose(circuit.to_matrix(), subcircuit.to_matrix())


def test_undefined_unitary_matrix_fails_when_used():
    gate = UnitaryGate("Undefined", 1)
    circuit = Circuit(1)
    circuit.append_unitary_gate(gate, [0])

    with pytest.raises(CircuitError):
        circuit.to_matrix()


def test_unitary_gate_qubit_count_mismatch_is_rejected():
    gate = UnitaryGate("XLike", 1).with_matrix(
        np.array([[0, 1], [1, 0]], dtype=complex)
    )
    circuit = Circuit(2)

    with pytest.raises(CircuitError):
        circuit.append_unitary_gate(gate, [0, 1])
