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

"""Native standard-gate instruction set.

:class:`StandardGate` provides the built-in quantum gates as class attributes.
Parametric gates (like ``RX``, ``RY``, ``RZ``) are called with parameters to
bind them::

    from cqlib.circuit.gates import StandardGate

    hadamard = StandardGate.H         # non-parametric
    rotation = StandardGate.RX(0.5)   # bind a numeric parameter
    symbolic = StandardGate.RZ(Parameter("theta"))  # bind a symbolic param

Use :meth:`~.circuit.Circuit.append_gate` to add a bound gate to a circuit.
"""

from __future__ import annotations

import numpy as np
import numpy.typing as npt
from typing_extensions import final
from ..parameter import Parameter
from .mc_gate import MCGate

@final
class StandardGate:
    """The set of standard quantum logic gates supported natively by Cqlib.

    Includes Pauli gates, Clifford gates, parametric rotations, two-qubit
    gates, and multi-controlled gates.

    Cannot be instantiated directly — use static class attributes like
    ``StandardGate.H`` or ``StandardGate.RX(0.5)``.
    """

    # Static factory attributes
    I: StandardGate
    H: StandardGate
    X: StandardGate
    Y: StandardGate
    Z: StandardGate
    S: StandardGate
    SDG: StandardGate
    T: StandardGate
    TDG: StandardGate
    RX: StandardGate
    RY: StandardGate
    RZ: StandardGate
    U: StandardGate
    Phase: StandardGate
    GPhase: StandardGate
    RXX: StandardGate
    RXY: StandardGate
    RYY: StandardGate
    RZX: StandardGate
    RZZ: StandardGate
    CX: StandardGate
    CY: StandardGate
    CZ: StandardGate
    CCX: StandardGate
    SWAP: StandardGate
    CRX: StandardGate
    CRY: StandardGate
    CRZ: StandardGate
    XY: StandardGate
    X2P: StandardGate
    X2M: StandardGate
    XY2P: StandardGate
    XY2M: StandardGate
    Y2P: StandardGate
    Y2M: StandardGate
    FSIM: StandardGate

    @staticmethod
    def all() -> list[StandardGate]:
        """Return all unbound standard gate definitions in enum order."""
        ...

    @staticmethod
    def from_name(name: str) -> StandardGate:
        """Resolve a case-insensitive standard-gate name to its definition.

        ``StandardGate.from_name('cx')`` returns ``StandardGate.CX``.

        Raises:
            ValueError: If ``name`` does not match any standard gate.
        """
        ...

    @property
    def num_qubits(self) -> int:
        """Total number of qubits this gate acts on."""
        ...

    @property
    def num_ctrl_qubits(self) -> int:
        """Number of control qubits defined for this gate."""
        ...

    @property
    def num_params(self) -> int:
        """Number of floating-point parameters this gate accepts."""
        ...

    @property
    def params(self) -> list[Parameter]:
        """Parameters bound to this gate instance."""
        ...

    def __call__(self, *args: float | Parameter) -> StandardGate:
        """Bind parameters to a parametric gate.

        For non-parametric gates (H, X, CX, etc.), calling with no arguments
        returns a clone.  For parametric gates (RX, RZ, etc.), positional
        arguments must match :attr:`num_params` exactly.

        Example::

            pi_over_2 = StandardGate.RX(1.5708)  # RX(π/2)
            symbolic  = StandardGate.RZ(Parameter("theta"))
        """
        ...

    def matrix(self, params: list[float] | None = None) -> npt.NDArray[np.complex128]:
        """Compute the unitary matrix.

        Args:
            params: Optional concrete values for parametric gates.
                If omitted, uses internally bound (constant) parameters.

        Raises:
            CircuitError: If parameter count does not match.
            ParameterError: If symbolic parameters cannot be evaluated.
        """
        ...

    def control(self, num_controls: int) -> MCGate:
        """Return a multi-controlled form of this gate.

        Args:
            num_controls: Number of additional control qubits.
        """
        ...

    def inverse(self) -> StandardGate:
        """Return the inverse (Hermitian conjugate) gate."""
        ...
    def __copy__(self) -> StandardGate: ...
    def __deepcopy__(self, memo: dict) -> StandardGate: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
