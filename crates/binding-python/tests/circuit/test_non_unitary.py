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

"""Directive and non-unitary operation tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter
from cqlib.circuit import CircuitError, ClassicalExpr, ClassicalType, Directive


def test_measure_and_measure_bits_append_operations_and_values():
    circuit = Circuit(2)
    circuit.measure(0)
    circuit.measure_bits([0, 1])

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "measure_bit",
        "measure_bits",
    ]
    assert [qubit.index for qubit in circuit[0].qubits] == [0]
    assert [qubit.index for qubit in circuit[1].qubits] == [0, 1]
    assert [str(value) for value in circuit.classical_values] == [
        "ClassicalType.bit()",
        "ClassicalType.bit_vec(2)",
    ]


def test_measure_into_and_measure_bits_into_store_to_existing_variables():
    circuit = Circuit(2)
    bit_var = circuit.var(ClassicalType.bit())
    bits_var = circuit.var(ClassicalType.bit_vec(2))
    circuit.measure_into(0, bit_var)
    circuit.measure_bits_into([0, 1], bits_var)

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "measure_bit",
        "store",
        "measure_bits",
        "store",
    ]
    assert [str(value) for value in circuit.classical_vars] == [
        "ClassicalType.bit()",
        "ClassicalType.bit_vec(2)",
    ]


def test_store_appends_classical_store_instruction():
    circuit = Circuit(1)
    flag = circuit.var(ClassicalType.bool())
    circuit.store(flag, ClassicalExpr.bool_literal(True))

    assert len(circuit.operations) == 1
    assert circuit[0].instruction.instruction.name == "store"
    assert [str(value) for value in circuit.classical_vars] == ["ClassicalType.bool()"]


def test_barrier_reset_and_delay_append_expected_directives():
    tau = Parameter("tau")
    circuit = Circuit(2)
    circuit.barrier([0, 1])
    circuit.reset(0)
    circuit.delay(1, tau)

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "barrier",
        "reset",
        "delay",
    ]
    assert list(circuit.parameters) == [tau]


def test_measure_and_reset_are_not_matrix_convertible():
    measured = Circuit(1)
    measured.measure(0)
    with pytest.raises(CircuitError):
        measured.to_matrix()

    reset = Circuit(1)
    reset.reset(0)
    with pytest.raises(CircuitError):
        reset.to_matrix()


def test_barrier_does_not_change_matrix():
    circuit = Circuit(2)
    circuit.h(0)
    circuit.barrier([0, 1])
    circuit.cx(0, 1)

    matrix = circuit.to_matrix()
    assert matrix.shape == (4, 4)
    assert np.allclose(matrix @ matrix.conj().T, np.eye(4), atol=1e-10)


def test_directive_factories_and_inverse_behavior():
    barrier = Directive.barrier()
    measure = Directive.measure()
    reset = Directive.reset()

    assert barrier.is_barrier()
    assert measure.is_measure()
    assert reset.is_reset()
    assert barrier.inverse().name() == "barrier"

    assert measure.inverse() is None
    assert reset.inverse() is None
