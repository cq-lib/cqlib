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
from typing import Literal

from cqlib.circuit import Circuit, Instruction
from .result import TransformResult

class OptimizeOneQubitRuns:
    """Optimize fixed numeric one-qubit runs with exact phase preservation.

    Logical optimization accepts only strict improvements in logical one-qubit
    count and depth. Basis optimization compares the exact cost after lowering
    to the supplied target basis and never increases the two-qubit count.
    """

    @staticmethod
    def logical() -> OptimizeOneQubitRuns:
        """Create an optimizer using target-neutral logical cost."""
        ...
    @staticmethod
    def basis(target_basis: Sequence[Instruction | str]) -> OptimizeOneQubitRuns:
        """Create an optimizer using exact target-basis lowering cost.

        Basis entries are case-insensitive standard-gate names (e.g. ``'H'``,
        ``'X2P'``) or ``Instruction`` objects for multi-controlled gates.

        Raises:
            CompilerConfigError: If the target basis is empty, contains an
                unsupported instruction, or an unknown gate name.
        """
        ...
    @property
    def policy(self) -> Literal["logical", "basis"]: ...
    @property
    def target_basis(self) -> list[Instruction] | None: ...
    def run(self, circuit: Circuit) -> TransformResult:
        """Return an optimized circuit without modifying ``circuit``."""
        ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __copy__(self) -> OptimizeOneQubitRuns: ...
    def __deepcopy__(self, memo: dict[int, object]) -> OptimizeOneQubitRuns: ...

__all__ = [
    "OptimizeOneQubitRuns",
]
