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

"""Higher-level circuit workflow tests for the Python circuit API."""

import numpy as np

from cqlib import Circuit, Parameter
from cqlib.circuit import ClassicalExpr


def test_ghz_workflow_matrix_is_unitary():
    circuit = Circuit(3)
    circuit.h(0)
    circuit.cx(0, 1)
    circuit.cx(1, 2)

    matrix = circuit.to_matrix()

    assert [op.instruction.instruction.name for op in circuit.operations] == [
        "H",
        "CX",
        "CX",
    ]
    assert matrix.shape == (8, 8)
    assert np.allclose(matrix @ matrix.conj().T, np.eye(8), atol=1e-10)


def test_variational_circuit_can_be_bound_multiple_times():
    theta = Parameter("theta")
    phi = Parameter("phi")
    circuit = Circuit(2)
    circuit.rx(0, theta)
    circuit.ry(1, phi)
    circuit.cx(0, 1)

    first = circuit.assign_parameters({"theta": 0.1, "phi": 0.2})
    second = circuit.assign_parameters({"theta": 0.3, "phi": 0.4})

    assert list(first[0].params) == [0.1]
    assert list(second[1].params) == [0.4]
    assert not np.allclose(first.to_matrix(), second.to_matrix())


def test_compose_appends_operations_with_mapping():
    main = Circuit(3)
    main.h(0)
    subcircuit = Circuit(2)
    subcircuit.cx(0, 1)

    result = main.compose(subcircuit, [1, 2])

    assert result is None
    assert [op.instruction.instruction.name for op in main.operations] == ["H", "CX"]
    assert [qubit.index for qubit in main[1].qubits] == [1, 2]


def test_decompose_after_compose_expands_nested_gate():
    subcircuit = Circuit(1)
    subcircuit.x(0)
    gate_wrapper = Circuit(1)
    gate_wrapper.append_circuit_gate(subcircuit.to_gate("Flip"), [0])

    main = Circuit(1)
    main.h(0)
    main.compose(gate_wrapper)

    decomposed = main.decompose()

    assert [op.instruction.instruction.name for op in decomposed.operations] == [
        "H",
        "X",
    ]


def test_dynamic_circuit_validate_and_operation_order():
    circuit = Circuit(1)
    circuit.measure(0)
    circuit.if_(ClassicalExpr.bool_literal(True), lambda body: body.x(0))

    assert circuit.validate() is None
    assert [
        op.instruction.classical_control.kind
        if op.instruction.is_classical_control
        else op.instruction.instruction.name
        for op in circuit.operations
    ] == ["measure_bit", "if"]
