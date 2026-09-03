# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Numeric one- and two-qubit unitary synthesis primitives."""

from __future__ import annotations

import numpy as np
from numpy.typing import ArrayLike, NDArray

from cqlib.circuit import Qubit, ValueOperation
from . import TwoQubitUnitaryDecomposeBasis

class OneQubitUnitaryDecomposition:
    """U-gate angles and scalar phase for a one-qubit unitary."""

    @property
    def theta(self) -> float:
        """Polar rotation angle of the synthesized U gate."""
        ...
    @property
    def phi(self) -> float:
        """First azimuthal angle of the synthesized U gate."""
        ...
    @property
    def lambda_(self) -> float:
        """Second azimuthal angle of the synthesized U gate."""
        ...
    @property
    def global_phase(self) -> float:
        """Scalar phase multiplying the synthesized U gate."""
        ...
    def __copy__(self) -> OneQubitUnitaryDecomposition: ...
    def __deepcopy__(self, memo: dict[int, object]) -> OneQubitUnitaryDecomposition: ...
    def __eq__(self, other: object) -> bool: ...

class TwoQubitUnitarySynthesisResult:
    """Standard-gate operations and scalar phase synthesized from a 4x4 unitary."""

    @property
    def operations(self) -> list[ValueOperation]:
        """Independent copy of the synthesized standard-gate operations."""
        ...
    @property
    def global_phase(self) -> float:
        """Scalar phase multiplying the emitted operation sequence."""
        ...
    def __copy__(self) -> TwoQubitUnitarySynthesisResult: ...
    def __deepcopy__(
        self, memo: dict[int, object]
    ) -> TwoQubitUnitarySynthesisResult: ...

class KakDecomposition:
    """Canonical two-qubit KAK decomposition."""

    @property
    def global_phase(self) -> float:
        """Scalar phase multiplying the complete KAK decomposition."""
        ...
    @property
    def k1l(self) -> NDArray[np.complex128]:
        """Independent copy of the left local factor after the Cartan core."""
        ...
    @property
    def k1r(self) -> NDArray[np.complex128]:
        """Independent copy of the right local factor after the Cartan core."""
        ...
    @property
    def k2l(self) -> NDArray[np.complex128]:
        """Independent copy of the left local factor before the Cartan core."""
        ...
    @property
    def k2r(self) -> NDArray[np.complex128]:
        """Independent copy of the right local factor before the Cartan core."""
        ...
    @property
    def a(self) -> float:
        """Canonical Pauli-XX interaction coordinate."""
        ...
    @property
    def b(self) -> float:
        """Canonical Pauli-YY interaction coordinate."""
        ...
    @property
    def c(self) -> float:
        """Canonical Pauli-ZZ interaction coordinate."""
        ...
    def __copy__(self) -> KakDecomposition: ...
    def __deepcopy__(self, memo: dict[int, object]) -> KakDecomposition: ...

def synthesize_numeric_1q_unitary(
    matrix: ArrayLike,
) -> OneQubitUnitaryDecomposition:
    """Decompose a finite 2x2 unitary into U-gate angles and global phase.

    Raises:
        TypeError: If ``matrix`` cannot be converted to a two-dimensional array.
        CompilerConfigError: If the converted matrix is not a finite 2x2
            unitary.
    """
    ...

def synthesize_numeric_2q_unitary(
    matrix: ArrayLike,
    first: int | Qubit,
    second: int | Qubit,
    basis: TwoQubitUnitaryDecomposeBasis | None = None,
) -> TwoQubitUnitarySynthesisResult:
    """Synthesize a finite 4x4 unitary into standard-gate operations.

    Raises:
        TypeError: If ``matrix`` cannot be converted to a two-dimensional array.
        CompilerConfigError: If the matrix is invalid or the qubits are equal.
    """
    ...

def kak_decompose(matrix: ArrayLike) -> KakDecomposition:
    """Return the canonical KAK decomposition of a finite 4x4 unitary.

    Raises:
        TypeError: If ``matrix`` cannot be converted to a two-dimensional array.
        CompilerConfigError: If the converted matrix is not a finite 4x4
            unitary.
    """
    ...

__all__ = [
    "OneQubitUnitaryDecomposition",
    "TwoQubitUnitarySynthesisResult",
    "KakDecomposition",
    "synthesize_numeric_1q_unitary",
    "synthesize_numeric_2q_unitary",
    "kak_decompose",
]
