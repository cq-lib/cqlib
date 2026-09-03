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

"""Multi-controlled standard gates.

:class:`MCGate` wraps a :class:`StandardGate` with additional control qubits.
Parameters bound to the base gate are preserved across control promotion.
"""

import numpy as np
from numpy.typing import NDArray
from ..parameter import Parameter
from .standard import StandardGate

class MCGate:
    """Multi-controlled standard gate with optional bound parameters.

    Example::

        # Create a Toffoli-like gate (CCX) with 2 controls
        ccx = MCGate(2, StandardGate.X)

        # Multi-controlled Hadamard
        mcch = MCGate(3, StandardGate.H)
    """
    def __init__(self, num_controls: int, gate: StandardGate) -> None:
        """Create a multi-controlled gate.

        Args:
            num_controls: Number of control qubits to add.
            gate: The base :class:`StandardGate` to control.
        """
        ...
    def matrix(self, params: list[float] | None = ...) -> NDArray[np.complex128]:
        """Compute the unitary matrix.

        Args:
            params: Optional numeric parameters for the base gate (if parametric).
        """
        ...
    def inverse(self) -> MCGate:
        """Return the inverse (Hermitian conjugate) gate.

        The inverse of C(U) is C(U†).
        """
        ...
    @property
    def num_ctrl_qubits(self) -> int:
        """Number of control qubits."""
        ...
    @property
    def num_qubits(self) -> int:
        """Total qubits (controls + targets)."""
        ...
    @property
    def num_params(self) -> int:
        """Number of parameters required by the base gate."""
        ...
    @property
    def base_gate(self) -> StandardGate:
        """The base :class:`StandardGate` (without controls)."""
        ...
    @property
    def params(self) -> list[Parameter]:
        """Parameters bound to the base gate."""
        ...
    def __repr__(self) -> str: ...
    def __copy__(self) -> MCGate: ...
    def __deepcopy__(self, memo: dict) -> MCGate: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
