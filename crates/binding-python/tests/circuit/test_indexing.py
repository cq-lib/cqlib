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

"""Indexing tests for the Python circuit API."""

import pytest

from cqlib import Circuit
from cqlib.circuit import CircuitError, ClassicalType


def test_operation_access_by_positive_index():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.cx(0, 1)

    assert circuit[0].instruction.instruction.name == "H"
    assert circuit[1].instruction.instruction.name == "CX"
    assert [qubit.index for qubit in circuit[1].qubits] == [0, 1]


def test_operations_property_returns_ordered_operations():
    circuit = Circuit(1)
    circuit.x(0)
    circuit.y(0)
    circuit.z(0)

    operations = circuit.operations
    assert len(operations) == 3
    assert [op.instruction.instruction.name for op in operations] == ["X", "Y", "Z"]


def test_operation_method_and_getitem_report_out_of_range():
    circuit = Circuit(1)
    circuit.h(0)

    with pytest.raises(IndexError) as excinfo:
        circuit.operation(1)
    assert not isinstance(excinfo.value, CircuitError)

    with pytest.raises(IndexError):
        circuit[1]

    empty = Circuit(1)
    with pytest.raises(IndexError):
        empty[0]
    with pytest.raises(IndexError):
        empty.operation(0)


def test_remove_operation_returns_removed_operation():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.x(0)
    circuit.z(0)

    removed = circuit.remove_operation(1)

    assert removed.instruction.name == "X"
    assert [op.instruction.name for op in circuit.operations] == ["H", "Z"]


def test_remove_operations_returns_original_index_order_and_deduplicates():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.x(0)
    circuit.z(0)
    circuit.y(0)

    removed = circuit.remove_operations([3, 1, 1])

    assert [op.instruction.name for op in removed] == ["X", "Y"]
    assert [op.instruction.name for op in circuit.operations] == ["H", "Z"]


def test_remove_operation_out_of_bounds_raises_index_error():
    circuit = Circuit(1)
    circuit.h(0)

    with pytest.raises(IndexError):
        circuit.remove_operation(5)

    assert [op.instruction.name for op in circuit.operations] == ["H"]


def test_remove_operations_out_of_bounds_does_not_mutate():
    circuit = Circuit(1)
    circuit.h(0)
    circuit.x(0)

    with pytest.raises(IndexError):
        circuit.remove_operations([0, 2])

    assert [op.instruction.name for op in circuit.operations] == ["H", "X"]


def test_remove_operations_deletes_measurement_with_same_batch_store():
    circuit = Circuit(1)
    target = circuit.var(ClassicalType.bit())
    measured = circuit.measure(0)
    circuit.store(target, measured.expr())

    removed = circuit.remove_operations([0, 1])

    assert [op.instruction.name for op in removed] == ["measure_bit", "store"]
    assert circuit.operations == []
    assert circuit.classical_values == []


def test_remove_operations_rejects_measurement_still_used_by_store():
    circuit = Circuit(1)
    target = circuit.var(ClassicalType.bit())
    measured = circuit.measure(0)
    circuit.store(target, measured.expr())

    with pytest.raises(CircuitError):
        circuit.remove_operations([0])

    assert [op.instruction.name for op in circuit.operations] == [
        "measure_bit",
        "store",
    ]
    assert len(circuit.classical_values) == 1


def test_remove_operations_deletes_measurement_with_same_batch_control_flow():
    circuit = Circuit(1)
    measured = circuit.measure(0)
    circuit.if_(measured.expr().to_bool(), lambda body: body.x(0))

    removed = circuit.remove_operations([0, 1])

    assert [op.instruction.name for op in removed] == ["measure_bit", "if"]
    assert circuit.operations == []
    assert circuit.classical_values == []
