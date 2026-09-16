# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Convert legacy construction inputs into the native value IR."""

from ..circuit import Instruction, MCGate, Qubit, StandardGate, ValueOperation
from .deprecation import warn


def operation_from_gate(instruction, qubits):
    if isinstance(qubits, (int, Qubit)):
        qubits = [qubits]
    qubits = [q if isinstance(q, Qubit) else Qubit(q) for q in qubits]
    if isinstance(instruction, StandardGate):
        return ValueOperation.from_standard_gate(instruction, qubits)
    if isinstance(instruction, MCGate):
        return ValueOperation.from_mc_gate(instruction, qubits)
    if isinstance(instruction, Instruction):
        return ValueOperation.from_instruction(instruction, qubits)
    raise TypeError(
        "legacy append expects a native StandardGate, MCGate or Instruction; "
        "use append(ValueOperation(...)) for other instructions"
    )


def InstructionData(instruction, qubits) -> ValueOperation:
    """Deprecated factory returning a ValueOperation, with bound gate parameters."""
    warn(
        "InstructionData(instruction, qubits)",
        "ValueOperation.from_standard_gate(gate, qubits)",
    )
    return operation_from_gate(instruction, qubits)
