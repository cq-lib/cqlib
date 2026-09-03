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

"""Value IR wrapper tests for the Python circuit API."""

import numpy as np
import pytest

from cqlib import Circuit, Qubit
from cqlib.circuit import (
    ClassicalControlOp,
    ClassicalExpr,
    Directive,
    Instruction,
    StandardGate,
    ValueControlBody,
    ValueInstruction,
    ValueOperation,
)


def test_operation_exposes_value_ir_fields():
    circuit = Circuit(1)
    circuit.rx(0, 0.5)
    operation = circuit[0]

    assert isinstance(operation, ValueOperation)
    assert operation.name == "RX"
    assert operation.num_qubits == 1
    assert operation.num_params == 1
    assert operation.instruction_type == "standard"
    assert operation.is_standard
    assert operation.instruction.name == "RX"
    assert list(operation.params) == [0.5]
    assert [qubit.index for qubit in operation.qubits] == [0]


def test_instruction_factory_methods_expose_names_and_kinds():
    standard = Instruction.from_standard_gate(StandardGate.H())
    directive = Instruction.from_directive(Directive.barrier())

    assert standard.name == "H"
    assert standard.is_standard
    assert directive.name == "barrier"
    assert directive.is_directive


def test_value_instruction_wraps_instruction_and_control_variants():
    standard = ValueInstruction.from_instruction(
        Instruction.from_standard_gate(StandardGate.X())
    )
    assert standard.is_instruction
    assert standard.name == "X"
    assert standard.instruction_type == "standard"
    assert standard.is_standard
    assert standard.instruction.name == "X"
    assert repr(standard.standard_gate) == "StandardGate.X"

    body_circuit = Circuit(1)
    body_circuit.h(0)
    control = ClassicalControlOp.if_(
        ClassicalExpr.bool_literal(True),
        ValueControlBody(body_circuit.operations),
    )
    controlled = ValueInstruction.from_classical_control(control)

    assert controlled.is_classical_control
    assert controlled.name == "if"
    assert controlled.instruction_type == "classical_control"
    assert controlled.classical_control.kind == "if"


def test_value_operation_factories_can_build_and_append_operations():
    operation = ValueOperation.from_standard_gate(
        StandardGate.RX(0.25),
        [Qubit(0)],
        label="rx-label",
    )

    circuit = Circuit(1)
    circuit.append(operation)

    assert operation.label == "rx-label"
    assert operation.name == "RX"
    assert operation.num_qubits == 1
    assert operation.num_params == 1
    assert list(operation.params) == [0.25]
    assert circuit[0].name == "RX"


def test_value_operation_matrix_for_unitary_gate():
    operation = ValueOperation.from_standard_gate(StandardGate.X(), [Qubit(0)])

    assert np.allclose(operation.matrix(), np.array([[0, 1], [1, 0]], dtype=complex))


def test_value_operation_matrix_rejects_directive_without_matrix():
    instruction = Instruction.from_directive(Directive.barrier())
    operation = ValueOperation.from_instruction(instruction, [Qubit(0)])

    assert operation.name == "barrier"
    assert operation.is_directive
    assert operation.instruction.is_directive
    assert operation.instruction.directive.name() == "barrier"

    with pytest.raises(ValueError):
        operation.matrix()


def test_value_operation_from_classical_control():
    body_circuit = Circuit(1)
    body_circuit.z(0)
    control = ClassicalControlOp.if_(
        ClassicalExpr.bool_literal(True),
        ValueControlBody(body_circuit.operations),
    )
    operation = ValueOperation.from_classical_control(control)

    assert operation.name == "if"
    assert operation.is_classical_control
    assert operation.instruction.is_classical_control
    assert operation.instruction.classical_control.kind == "if"
    assert [
        op.name for op in operation.instruction.classical_control.then_body.operations
    ] == ["Z"]
