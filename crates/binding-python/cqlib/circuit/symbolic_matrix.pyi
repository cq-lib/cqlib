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

"""Dense symbolic complex matrices.

Symbolic matrices preserve :class:`~.parameter.Parameter` expressions until
explicit evaluation.  Their storage grows as O(4ⁿ), so they are intended for
small circuits, compiler rewrites, and custom gate definitions — not for
simulation-scale work.

:class:`SymbolicComplex` is the scalar element type; :class:`SymbolicMatrix`
is the matrix container.
"""

from collections.abc import Mapping
import numpy as np
from numpy.typing import NDArray
from .parameter import Parameter

class SymbolicComplex:
    """Complex scalar whose real and imaginary parts are :class:`Parameter` expressions.

    Convenience constructors::

        zero = SymbolicComplex.zero()
        one  = SymbolicComplex.one()
        i    = SymbolicComplex.i()
        eⁱᶿ  = SymbolicComplex.exp_i(theta)
    """
    def __init__(self, real: Parameter, imag: Parameter) -> None:
        """Create from real and imaginary :class:`Parameter` parts."""
        ...
    @staticmethod
    def zero() -> SymbolicComplex:
        """The complex zero ``0 + 0i``."""
        ...
    @staticmethod
    def one() -> SymbolicComplex:
        """The complex one ``1 + 0i``."""
        ...
    @staticmethod
    def i() -> SymbolicComplex:
        """The imaginary unit ``0 + 1i``."""
        ...
    @staticmethod
    def from_real(value: Parameter) -> SymbolicComplex:
        """Create a real-only value ``value + 0i``."""
        ...
    @staticmethod
    def exp_i(theta: Parameter) -> SymbolicComplex:
        """Complex exponential ``cos(θ) + i·sin(θ)``."""
        ...
    @property
    def real(self) -> Parameter:
        """The symbolic real part."""
        ...
    @property
    def imag(self) -> Parameter:
        """The symbolic imaginary part."""
        ...
    @property
    def symbols(self) -> list[str]:
        """All free symbols in deterministic order."""
        ...
    def evaluate(self, bindings: Mapping[str, float] | None = ...) -> complex:
        """Evaluate as a Python ``complex`` after binding symbols."""
        ...
    def simplify(self) -> SymbolicComplex:
        """Simplify both symbolic components."""
        ...
    def replace(self, symbol: str, value: Parameter) -> SymbolicComplex:
        """Replace a symbol with another parameter expression."""
        ...
    def is_zero_exact(self) -> bool:
        """True if both components are exactly zero."""
        ...
    def is_one_exact(self) -> bool:
        """True if real is exactly one and imaginary is exactly zero."""
        ...
    def simplifies_to_zero(self) -> bool:
        """True if this simplifies to exactly zero."""
        ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> SymbolicComplex: ...
    def __deepcopy__(self, memo: dict) -> SymbolicComplex: ...
    def __repr__(self) -> str: ...

class SymbolicMatrix:
    """Dense row-major matrix of :class:`SymbolicComplex` values.

    Supports indexing (including negative indices), evaluation, simplification,
    and symbolic substitution.
    """
    def __init__(self, rows: list[list[SymbolicComplex]]) -> None: ...
    @property
    def shape(self) -> tuple[int, int]:
        """Return ``(rows, cols)``."""
        ...
    @property
    def symbols(self) -> list[str]:
        """All free symbols in deterministic order."""
        ...
    def evaluate(self, bindings: Mapping[str, float] | None = ...) -> NDArray[np.complex128]:
        """Evaluate as a NumPy complex128 array after binding symbols."""
        ...
    def simplify(self) -> SymbolicMatrix:
        """Simplify every matrix element."""
        ...
    def substitute(self, replacements: Mapping[str, Parameter]) -> SymbolicMatrix:
        """Simultaneously substitute symbols with :class:`Parameter` expressions."""
        ...
    def rows(self) -> list[list[SymbolicComplex]]:
        """Return a nested row representation."""
        ...
    def __getitem__(self, index: tuple[int, int]) -> SymbolicComplex:
        """Return the element at ``(row, col)``.  Negative indices are supported."""
        ...
    def __len__(self) -> int:
        """Return the number of rows."""
        ...
    def __copy__(self) -> SymbolicMatrix: ...
    def __deepcopy__(self, memo: dict) -> SymbolicMatrix: ...
    def __repr__(self) -> str: ...
