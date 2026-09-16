# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

from typing_extensions import deprecated
from ..circuit import Instruction, MCGate, Qubit, StandardGate, ValueOperation
from collections.abc import Sequence

@deprecated("Use ValueOperation.from_standard_gate or ValueOperation.from_instruction")
def InstructionData(
    instruction: StandardGate | MCGate | Instruction,
    qubits: int | Qubit | Sequence[int | Qubit],
) -> ValueOperation: ...
