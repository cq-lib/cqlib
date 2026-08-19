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

"""Boundary and error behavior tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Parameter, Qubit
from cqlib.circuit import CircuitError, ParameterError, QubitError


def test_zero_qubit_circuit_matrix_shape():
    circuit = Circuit(0)

    assert circuit.num_qubits == 0
    assert circuit.to_matrix().shape == (1, 1)
    assert np.allclose(circuit.to_matrix(), np.array([[1]], dtype=complex))


def test_from_operations_accepts_empty_inputs():
    circuit = Circuit.from_operations([], [])

    assert circuit.num_qubits == 0
    assert circuit.operations == []


def test_operation_values_can_be_reused_in_new_circuit():
    source = Circuit(1)
    source.h(0)

    target = Circuit.from_operations(source.qubits, source.operations)

    assert target[0].instruction.instruction.name == "H"
    assert np.allclose(target.to_matrix(), source.to_matrix())


def test_many_symbolic_parameters_preserve_first_seen_order():
    params = [Parameter(f"p{i}") for i in range(5)]
    circuit = Circuit(1)
    for param in params:
        circuit.rx(0, param)

    assert list(circuit.parameters) == params


def test_long_ascii_gate_label_is_preserved():
    subcircuit = Circuit(1)
    subcircuit.x(0)
    label = "very_long_ascii_gate_label_for_round_trip_testing"

    circuit = Circuit(1)
    circuit.append_circuit_gate(subcircuit.to_gate(label), [0])

    assert circuit[0].instruction.instruction.name == label


def test_invalid_qubit_and_parameter_inputs_are_rejected():
    with pytest.raises(QubitError):
        Qubit(-1)

    with pytest.raises(ParameterError):
        Parameter("")


def test_compose_reports_incompatible_mapping():
    main = Circuit(1)
    other = Circuit(2)

    with pytest.raises(CircuitError):
        main.compose(other, [0])

    with pytest.raises(CircuitError):
        main.compose(other, [0, 2])
