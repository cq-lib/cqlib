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

"""Pre-routing basis legalization transform."""

from __future__ import annotations

from collections.abc import Sequence

from cqlib.circuit import Circuit, Instruction
from .result import TransformResult

class LowerToRoutingBasis:
    """Lower gate-like operations to SABRE's 0/1/2-qubit input contract.

    ``preferred_basis`` is a hint for choosing the 2-qubit family used when
    lowering CCX. CZ is preferred only when the basis contains CZ and does not
    contain CX. Final native-basis translation remains a separate compiler
    stage.
    """

    def __init__(self, preferred_basis: Sequence[Instruction] | None = None) -> None:
        """Create a routing-basis lowering transform."""
        ...

    @property
    def preferred_basis(self) -> list[Instruction] | None:
        """A copy of the preferred basis hint, if configured."""
        ...

    def run(self, circuit: Circuit) -> TransformResult:
        """Lower ``circuit`` without modifying it.

        Raises:
            CompilerConfigError: If lowering cannot satisfy SABRE's 0/1/2-
                qubit gate-like operation contract.
        """
        ...
    def __copy__(self) -> LowerToRoutingBasis: ...
    def __deepcopy__(self, memo: dict[int, object]) -> LowerToRoutingBasis: ...
    def __repr__(self) -> str: ...

def lower_to_routing_basis(
    circuit: Circuit, preferred_basis: Sequence[Instruction] | None = None
) -> TransformResult:
    """Lower ``circuit`` to SABRE's routing input contract.

    ``preferred_basis`` is a CCX-lowering hint, not an exact final output
    basis. The input circuit is not modified.

    Raises:
        CompilerConfigError: If lowering cannot satisfy SABRE's 0/1/2-qubit
            gate-like operation contract.
    """
    ...
