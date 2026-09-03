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

"""Instruction and operation types.

:class:`Instruction` and :class:`ValueInstruction` are the two sum types that
represent *what* an operation does (standard gate, custom unitary, directive,
or classical control).  :class:`ValueOperation` pairs an instruction with
qubits and parameters to form a complete circuit operation.
"""

import numpy as np
from numpy.typing import NDArray
from .bit import Qubit
from .control_flow import ClassicalControlOp
from .gates import CircuitGate, Directive, MCGate, StandardGate, UnitaryGate
from .parameter import Parameter

class Instruction:
    """Storage-IR instruction — a gate or directive without parameters.

    Instructions define the operation type but do not own parameters.
    For parameterised operations, use :class:`ValueOperation` factories.
    """
    @staticmethod
    def from_name(name: str) -> Instruction:
        """Create from a case-insensitive standard-gate name (e.g. ``'H'``).

        Only standard gates are addressable by name; multi-controlled and
        custom gates must be built from their own types.

        Raises:
            ValueError: If ``name`` does not match any standard gate.
        """
        ...
    @staticmethod
    def from_standard_gate(gate: StandardGate) -> Instruction:
        """Create from a :class:`StandardGate` (no bound parameters)."""
        ...
    @staticmethod
    def from_mc_gate(gate: MCGate) -> Instruction:
        """Create from a :class:`MCGate` (no bound parameters)."""
        ...
    @staticmethod
    def from_unitary_gate(gate: UnitaryGate) -> Instruction:
        """Create from a :class:`UnitaryGate`."""
        ...
    @staticmethod
    def from_circuit_gate(gate: CircuitGate) -> Instruction:
        """Create from a :class:`CircuitGate`."""
        ...
    @staticmethod
    def from_directive(directive: Directive) -> Instruction:
        """Create from a :class:`Directive` (barrier, measure, reset)."""
        ...
    @staticmethod
    def delay() -> Instruction:
        """Create a delay instruction."""
        ...
    @property
    def name(self) -> str:
        """Human-readable name (e.g. ``"H"``, ``"CX"``, ``"Measure"``)."""
        ...
    @property
    def instruction_type(self) -> str:
        """One of ``"standard"``, ``"mcgate"``, ``"unitary"``, ``"circuit"``,
        ``"directive"``, ``"classical_data"``, ``"classical_control"``, ``"delay"``."""
        ...
    @property
    def is_standard(self) -> bool: ...
    @property
    def is_mcgate(self) -> bool: ...
    @property
    def is_unitary(self) -> bool: ...
    @property
    def is_circuit_gate(self) -> bool: ...
    @property
    def is_directive(self) -> bool: ...
    @property
    def is_classical_control(self) -> bool: ...
    @property
    def is_classical_data(self) -> bool: ...
    @property
    def is_delay(self) -> bool: ...
    @property
    def standard_gate(self) -> StandardGate | None:
        """The :class:`StandardGate` if this is a standard-gate instruction."""
        ...
    @property
    def directive(self) -> Directive | None:
        """The :class:`Directive` if this is a directive instruction."""
        ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Instruction: ...
    def __deepcopy__(self, memo: dict) -> Instruction: ...
    def __repr__(self) -> str: ...

class ValueInstruction:
    """Construction-IR instruction — self-contained, may include classical control.

    Either wraps an :class:`Instruction` or a :class:`ClassicalControlOp`.
    """
    @staticmethod
    def from_instruction(instruction: Instruction) -> ValueInstruction:
        """Wrap a storage :class:`Instruction`."""
        ...
    @staticmethod
    def from_classical_control(control: ClassicalControlOp) -> ValueInstruction:
        """Wrap a :class:`ClassicalControlOp`."""
        ...
    @property
    def is_classical_control(self) -> bool: ...
    @property
    def is_instruction(self) -> bool: ...
    @property
    def name(self) -> str:
        """Human-readable name (e.g. ``"H"``, ``"CX"``, ``"Measure"``)."""
        ...
    @property
    def instruction_type(self) -> str:
        """One of ``"standard"``, ``"mcgate"``, ``"unitary"``, ``"circuit"``,
        ``"directive"``, ``"classical_data"``, ``"classical_control"``, ``"delay"``."""
        ...
    @property
    def is_standard(self) -> bool: ...
    @property
    def is_mcgate(self) -> bool: ...
    @property
    def is_unitary(self) -> bool: ...
    @property
    def is_circuit_gate(self) -> bool: ...
    @property
    def is_directive(self) -> bool: ...
    @property
    def is_classical_data(self) -> bool: ...
    @property
    def is_delay(self) -> bool: ...
    @property
    def standard_gate(self) -> StandardGate | None:
        """The :class:`StandardGate` if this is a standard-gate instruction."""
        ...
    @property
    def directive(self) -> Directive | None:
        """The :class:`Directive` if this is a directive instruction."""
        ...
    @property
    def instruction(self) -> Instruction | None:
        """The inner :class:`Instruction` when this is a plain instruction."""
        ...
    @property
    def classical_control(self) -> ClassicalControlOp | None:
        """The inner :class:`ClassicalControlOp` when this is classical control."""
        ...
    def __str__(self) -> str: ...
    def __copy__(self) -> ValueInstruction: ...
    def __deepcopy__(self, memo: dict) -> ValueInstruction: ...
    def __repr__(self) -> str: ...

class ValueOperation:
    """Self-contained construction-IR operation (instruction + qubits + params).

    This is the public construction boundary.  Use the static factories to
    create operations from gates with their bound parameters preserved.
    """
    def __init__(
        self,
        instruction: ValueInstruction,
        qubits: list[Qubit],
        params: list[float | Parameter] | None = ...,
        label: str | None = ...,
    ) -> None: ...
    @staticmethod
    def from_instruction(
        instruction: Instruction,
        qubits: list[Qubit],
        params: list[float | Parameter] | None = ...,
        label: str | None = ...,
    ) -> ValueOperation:
        """Create from a storage :class:`Instruction` with explicit parameters."""
        ...
    @staticmethod
    def from_standard_gate(
        gate: StandardGate, qubits: list[Qubit], label: str | None = ...
    ) -> ValueOperation:
        """Create while preserving parameters bound to a :class:`StandardGate`."""
        ...
    @staticmethod
    def from_mc_gate(
        gate: MCGate, qubits: list[Qubit], label: str | None = ...
    ) -> ValueOperation:
        """Create while preserving parameters bound to an :class:`MCGate`."""
        ...
    @staticmethod
    def from_classical_control(control: ClassicalControlOp) -> ValueOperation:
        """Create from a :class:`ClassicalControlOp`."""
        ...
    @property
    def instruction(self) -> ValueInstruction: ...
    @property
    def qubits(self) -> list[Qubit]: ...
    @property
    def params(self) -> list[float | Parameter]: ...
    @property
    def name(self) -> str:
        """Human-readable instruction name for this operation."""
        ...
    @property
    def num_qubits(self) -> int:
        """Number of qubits used by this operation instance."""
        ...
    @property
    def num_params(self) -> int:
        """Number of parameters carried by this operation instance."""
        ...
    @property
    def instruction_type(self) -> str:
        """One of ``"standard"``, ``"mcgate"``, ``"unitary"``, ``"circuit"``,
        ``"directive"``, ``"classical_data"``, ``"classical_control"``, ``"delay"``."""
        ...
    @property
    def is_standard(self) -> bool: ...
    @property
    def is_mcgate(self) -> bool: ...
    @property
    def is_unitary(self) -> bool: ...
    @property
    def is_circuit_gate(self) -> bool: ...
    @property
    def is_directive(self) -> bool: ...
    @property
    def is_classical_data(self) -> bool: ...
    @property
    def is_classical_control(self) -> bool: ...
    @property
    def is_delay(self) -> bool: ...
    @property
    def label(self) -> str | None: ...
    def matrix(self) -> NDArray[np.complex128]:
        """Compute the unitary matrix (fixed-parameter operations only).

        Raises:
            ValueError: For classical control or symbolic parameters without context.
        """
        ...
    def __str__(self) -> str: ...
    def __copy__(self) -> ValueOperation: ...
    def __deepcopy__(self, memo: dict) -> ValueOperation: ...
    def __repr__(self) -> str: ...
