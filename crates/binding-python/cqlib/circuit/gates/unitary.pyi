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

"""User-defined unitary gates.

:class:`UnitaryGate` supports three definition styles:

1. **Numeric matrix** — attach a concrete NumPy matrix with :meth:`with_matrix`.
2. **Symbolic matrix** — attach a :class:`~.symbolic_matrix.SymbolicMatrix` with :meth:`with_symbolic_matrix`.
3. **Frozen circuit** — attach a :class:`~.circuit_gate.FrozenCircuit` with :meth:`with_circuit`.

Once defined, use :meth:`~.circuit.Circuit.append_unitary_gate` to add it
to a circuit with positional parameter bindings.
"""

from typing import Any
import numpy as np
from numpy.typing import ArrayLike, NDArray
from .circuit_gate import FrozenCircuit
from ..symbolic_matrix import SymbolicMatrix

class UnitaryGate:
    """User-defined unitary gate with stable definition identity.

    Args:
        label: Descriptive name (e.g. ``"QFT"``, ``"Oracle"``).
        num_qubits: Number of qubits the gate acts on.
        num_params: Number of positional parameters per application.
    """
    def __init__(self, label: str, num_qubits: int, num_params: int = ...) -> None: ...
    def with_matrix(self, matrix: ArrayLike) -> UnitaryGate:
        """Attach a numeric unitary matrix.

        Args:
            matrix: A 2D square NumPy array or array-like of shape (2ⁿ, 2ⁿ).
        """
        ...
    def with_symbolic_matrix(self, matrix: SymbolicMatrix, params: list[str]) -> UnitaryGate:
        """Attach a symbolic matrix with positional parameter names.

        Args:
            matrix: A :class:`SymbolicMatrix`.
            params: Parameter names in positional order.
        """
        ...
    def with_circuit(self, circuit: FrozenCircuit) -> UnitaryGate:
        """Attach an immutable circuit definition.

        Args:
            circuit: A :class:`FrozenCircuit` defining the gate's behavior.
        """
        ...
    @property
    def label(self) -> str:
        """The gate's descriptive name."""
        ...
    @property
    def num_qubits(self) -> int:
        """Number of qubits this gate acts on."""
        ...
    @property
    def num_params(self) -> int:
        """Number of positional parameters per application."""
        ...
    @property
    def symbolic_matrix(self) -> SymbolicMatrix | None:
        """The symbolic matrix definition when present."""
        ...
    @property
    def matrix_params(self) -> list[str] | None:
        """Positional symbolic-matrix parameter names when present."""
        ...
    @property
    def circuit(self) -> FrozenCircuit | None:
        """The frozen circuit definition when present."""
        ...
    def matrix(self) -> NDArray[np.complex128]:
        """Return the numeric unitary matrix.

        Raises:
            CircuitError: If no matrix was attached to the gate.
        """
        ...
    def matrix_for_params(self, params: list[float]) -> NDArray[np.complex128]:
        """Evaluate a symbolic matrix definition for concrete parameters.

        Args:
            params: Numeric values for each positional parameter.
        """
        ...
    def __array__(self, dtype: Any | None = ..., copy: bool | None = ...) -> NDArray[np.complex128]:
        """NumPy array protocol — allows ``np.array(gate)``."""
        ...
    def __copy__(self) -> UnitaryGate: ...
    def __deepcopy__(self, memo: dict) -> UnitaryGate: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...
