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

from cqlib.circuit import Circuit
from .result import TransformResult

class CommutativeCancellation:
    """Cancel self-inverse gate pairs over unbounded commutation sets.

    Pairs of self-inverse gates from ``{CX, CY, CZ, H, Y, X, Z}`` cancel
    when the operations between them on the shared wires are proven to
    commute exactly (zero-phase identities only). Labeled operations,
    non-unitary instructions, and control flow act as hard barriers;
    control-flow bodies are analyzed recursively.
    """

    def __init__(self) -> None: ...
    def run(self, circuit: Circuit) -> TransformResult:
        """Return a circuit with cancellable pairs removed.

        The input ``circuit`` is never modified.

        Raises:
            CompilerTransformError: If the circuit contains corrupted IR
                (for example, unresolvable symbolic parameters).
        """
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> CommutativeCancellation: ...
    def __deepcopy__(self, memo: dict[int, object]) -> CommutativeCancellation: ...

__all__ = [
    "CommutativeCancellation",
]
