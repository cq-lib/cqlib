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

from __future__ import annotations

from collections.abc import Sequence

from cqlib.circuit import Circuit, Instruction, Qubit, StandardGate, ValueOperation
from .result import TransformResult

_QubitLike = int | Qubit

class TargetBasisLowerer:
    """Lower circuits deterministically to an explicit instruction basis."""

    def __init__(self, target_basis: Sequence[Instruction | str]) -> None:
        """Create lowering plans for a non-empty gate-like basis.

        Basis entries are case-insensitive standard-gate names (e.g. ``'H'``,
        ``'X2P'``) or ``Instruction`` objects for multi-controlled gates.

        Raises:
            CompilerConfigError: If the basis is empty, contains an unknown
                gate name, or cannot lower all required gates.
            CompilerInternalError: If the built-in rule library is invalid.
        """
        ...
    @property
    def target_basis(self) -> list[Instruction]: ...
    def run(self, circuit: Circuit) -> TransformResult:
        """Lower ``circuit`` without modifying it.

        Raises:
            CompilerConfigError: If an operation cannot be lowered.
            CircuitError: If circuit rebuilding fails.
            CompilerInternalError: If lowering violates its contract.
        """
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> TargetBasisLowerer: ...
    def __deepcopy__(self, memo: dict[int, object]) -> TargetBasisLowerer: ...

class TargetBasisSignature:
    """Canonical, order- and duplicate-insensitive target identity."""

    @staticmethod
    def from_standard_gates(
        gates: Sequence[StandardGate],
    ) -> TargetBasisSignature: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> TargetBasisSignature: ...
    def __deepcopy__(self, memo: dict[int, object]) -> TargetBasisSignature: ...

class TargetBasisCost:
    """Exact operation and depth cost after target-basis lowering."""

    @property
    def two_qubit_ops(self) -> int: ...
    @property
    def depth(self) -> int: ...
    @property
    def total_ops(self) -> int: ...
    @property
    def parameterized_ops(self) -> int: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> TargetBasisCost: ...
    def __deepcopy__(self, memo: dict[int, object]) -> TargetBasisCost: ...

class TargetBasisCostModel:
    """Reusable exact target-basis cost evaluator."""

    def __init__(self, target_basis: Sequence[Instruction | str]) -> None:
        """Build an evaluator for a non-empty standard-instruction basis.

        Basis entries are case-insensitive standard-gate names (e.g. ``'H'``,
        ``'X2P'``) or ``Instruction`` objects; the basis must contain only
        standard instructions.

        Raises:
            CompilerConfigError: If the basis is empty, contains an unknown
                gate name, or contains a non-standard instruction.
            CompilerInternalError: If the built-in rule library is invalid.
        """
        ...
    @property
    def signature(self) -> TargetBasisSignature: ...
    @property
    def target_basis(self) -> list[Instruction]: ...
    def cost_of_fixed_operations(
        self,
        qubits: Sequence[_QubitLike],
        operations: Sequence[ValueOperation],
    ) -> TargetBasisCost:
        """Return the exact lowered cost of fixed finite operations.

        Raises:
            CompilerConfigError: If an operation cannot be lowered.
            CircuitError: If the supplied operation sequence is invalid.
            CompilerInternalError: If lowering violates its contract.
        """
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> TargetBasisCostModel: ...
    def __deepcopy__(self, memo: dict[int, object]) -> TargetBasisCostModel: ...

__all__ = [
    "TargetBasisLowerer",
    "TargetBasisSignature",
    "TargetBasisCost",
    "TargetBasisCostModel",
]
