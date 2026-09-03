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

"""Creation and basic construction tests for the Python circuit API."""

import pytest

from cqlib import Circuit, Parameter, Qubit
from cqlib.circuit import CircuitError


def test_empty_and_fixed_width_circuits():
    empty = Circuit(0)
    assert empty.num_qubits == 0
    assert empty.width == 0
    assert empty.qubits == []
    assert len(empty.operations) == 0

    circuit = Circuit(3)
    assert circuit.num_qubits == 3
    assert circuit.width == 3
    assert [qubit.index for qubit in circuit.qubits] == [0, 1, 2]


def test_circuit_from_indices_and_qubits():
    by_index = Circuit([2, 0, 5])
    assert by_index.num_qubits == 3
    assert [qubit.index for qubit in by_index.qubits] == [2, 0, 5]

    by_qubit = Circuit([Qubit(4), Qubit(1)])
    assert by_qubit.num_qubits == 2
    assert [qubit.index for qubit in by_qubit.qubits] == [4, 1]


def test_circuit_rejects_duplicate_qubits():
    with pytest.raises(CircuitError):
        Circuit([0, 1, 1])

    with pytest.raises(CircuitError):
        Circuit([Qubit(0), Qubit(0)])


def test_add_qubits_preserves_existing_operations():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.add_qubits([2, 4])

    assert circuit.num_qubits == 3
    assert [qubit.index for qubit in circuit.qubits] == [0, 2, 4]
    assert len(circuit.operations) == 1
    assert circuit[0].instruction.instruction.name == "H"


def test_global_phase_accepts_numeric_and_symbolic_values():
    circuit = Circuit(1)
    assert circuit.global_phase.is_zero()

    circuit.set_global_phase(0.25)
    assert circuit.global_phase.evaluate({}) == 0.25

    theta = Parameter("theta")
    circuit.set_global_phase(theta)
    assert str(circuit.global_phase) == "theta"
    assert list(circuit.parameters) == [theta]


def test_from_operations_round_trip():
    source = Circuit(2)
    source.h(0)
    source.cx(0, 1)

    restored = Circuit.from_operations(source.qubits, source.operations)
    assert restored.num_qubits == 2
    assert [op.instruction.instruction.name for op in restored.operations] == [
        "H",
        "CX",
    ]
    assert [qubit.index for qubit in restored.operations[1].qubits] == [0, 1]


def test_invalid_operation_qubit_is_reported():
    circuit = Circuit(1)
    with pytest.raises(CircuitError):
        circuit.h(2)
