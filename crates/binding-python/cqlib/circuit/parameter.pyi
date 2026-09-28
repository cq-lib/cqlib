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

"""Symbolic and numeric circuit parameters.

:class:`Parameter` is the expression type used for gate angles in
parameterized quantum circuits.  Strings are parsed as mathematical
expressions; numeric values become immutable constants.  Arithmetic
builds new immutable expressions, and evaluation errors are reported
via :class:`~.circuit.ParameterError`.

Quick start::

    from cqlib import Parameter

    theta = Parameter("theta")
    expr = 2 * theta + Parameter.pi() / 2  # 2*θ + π/2
    result = expr.evaluate({"theta": 1.0})  # → 2.5708...
    d = expr.derivative("theta")            # → 2
"""

class Parameter:
    """Immutable symbolic or numeric expression used as a circuit parameter.

    A plain identifier such as ``"theta"`` is parsed as a symbol. Invalid
    expression syntax raises :class:`~.circuit.ParameterError` instead of
    silently creating a symbol with the invalid text.

    Supports :class:`float` and :class:`int` input for numeric constants::

        Parameter(3.14)      # numeric constant
        Parameter("theta")   # symbolic variable
        Parameter("x + 1")   # expression
    """

    def __init__(self, value: int | float | str) -> None:
        """Create a new parameter.

        Args:
            value: A number (creates constant) or expression string (parsed).

        Raises:
            ParameterError: If the string is not a valid expression.
            TypeError: If the value is not a number or string.
        """
        ...

    @staticmethod
    def from_expression(expr: str) -> Parameter:
        """Parse a mathematical expression string into a Parameter.

        Supported syntax: numbers, constants (``pi``, ``e``), variables,
        operators (``+``, ``-``, ``*``, ``/``), functions (``sin``, ``cos``,
        ``exp``, ``sqrt``, ``ln``, etc.), and parentheses.
        """
        ...

    @staticmethod
    def pi() -> Parameter:
        """The mathematical constant π."""
        ...

    @staticmethod
    def e() -> Parameter:
        """Euler's number e."""
        ...

    def evaluate(self, bindings: dict[str, float] | None = None) -> float:
        """Evaluate the expression with concrete variable bindings.

        Args:
            bindings: Mapping from symbol names to numeric values.

        Raises:
            ParameterError: If evaluation fails (e.g. unbound symbol).
        """
        ...

    def simplify(self) -> Parameter:
        """Return a domain-safe algebraically simplified copy."""
        ...

    def derivative(self, var: str) -> Parameter:
        """Compute the symbolic derivative with respect to a variable."""
        ...

    @property
    def symbols(self) -> list[str]:
        """All unique symbols in sorted order."""
        ...

    def canonicalized(self) -> Parameter:
        """Return the canonical storage form used by circuit parameter interning."""
        ...

    def is_exact_zero(self) -> bool:
        """True if this expression is exactly the numeric constant zero."""
        ...

    def is_constant(self) -> bool:
        """True if this parameter has no free variables."""
        ...

    def is_zero(self) -> bool:
        """True if this parameter evaluates to zero (False if unbound symbols)."""
        ...

    def is_one(self) -> bool:
        """True if this parameter evaluates to one (False if unbound symbols)."""
        ...

    def as_symbol(self) -> str | None:
        """Return the symbol name when this expression is exactly one symbol."""
        ...

    def substitute(self, bindings: dict[str, Parameter]) -> Parameter:
        """Substitute multiple symbols and simplify the resulting expression."""
        ...

    def provably_equal(self, other: Parameter, tolerance: float = 1e-12) -> bool:
        """Conservative equality check within a numeric tolerance."""
        ...

    def provably_equal_modulo(
        self, other: Parameter, modulus: Parameter, tolerance: float = 1e-12
    ) -> bool:
        """Conservative equality check modulo *modulus* within tolerance."""
        ...

    def pow(self, val: Parameter | float) -> Parameter:
        """Return ``self ** val``.

        Args:
            val: Exponent (:class:`Parameter` or :class:`float`).
        """
        ...

    def replace(self, symbol: str, param: Parameter) -> Parameter:
        """Replace all occurrences of a symbol with another expression.

        Example::

            expr = Parameter("x") + Parameter(2)
            y = Parameter("y")
            result = expr.replace("x", y * 3)  # (y*3) + 2
        """
        ...

    # Arithmetic operators
    def __add__(self, other: Parameter | float) -> Parameter: ...
    def __radd__(self, other: float) -> Parameter: ...
    def __sub__(self, other: Parameter | float) -> Parameter: ...
    def __rsub__(self, other: float) -> Parameter: ...
    def __mul__(self, other: Parameter | float) -> Parameter: ...
    def __rmul__(self, other: float) -> Parameter: ...
    def __truediv__(self, other: Parameter | float) -> Parameter: ...
    def __rtruediv__(self, other: float) -> Parameter: ...
    def __pow__(self, other: Parameter | float) -> Parameter: ...
    def __neg__(self) -> Parameter: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> Parameter: ...
    def __deepcopy__(self, memo: dict) -> Parameter: ...

    # Mathematical functions
    def sin(self) -> Parameter: ...
    def cos(self) -> Parameter: ...
    def tan(self) -> Parameter: ...
    def asin(self) -> Parameter: ...
    def acos(self) -> Parameter: ...
    def atan(self) -> Parameter: ...
    def exp(self) -> Parameter: ...
    def ln(self) -> Parameter: ...
    def log(self, base: Parameter | None = None) -> Parameter: ...
    def sqrt(self) -> Parameter: ...
    def abs(self) -> Parameter: ...
    def sinh(self) -> Parameter: ...
    def cosh(self) -> Parameter: ...
    def tanh(self) -> Parameter: ...
    def floor(self) -> Parameter: ...
    def ceil(self) -> Parameter: ...
    def round(self) -> Parameter: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
