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

"""Typed, side-effect-free classical expression AST.

:class:`ClassicalExpr` values model the classical side of dynamic circuits.
They are constructed from variables (:meth:`var`), values (:meth:`value`),
or literals, then combined with logical, arithmetic, and comparison
operations.  Every expression carries a static :class:`~.classical.ClassicalType`.

Python operators ``~``, ``&``, ``|``, ``^`` are supported for bitwise/logical
operations where the type system allows them.
"""

from .classical import ClassicalType, ClassicalValue, ClassicalVar

class ClassicalExpr:
    """Typed classical expression used by dynamic-circuit control flow.

    Construct via static factory methods, then combine with operators
    or comparison methods::

        cond = ClassicalExpr.var(my_var).to_bool()
        is_one = ClassicalExpr.equal(cond, ClassicalExpr.bool_literal(True))
    """
    @staticmethod
    def var(var: ClassicalVar) -> ClassicalExpr:
        """Create an expression that reads a mutable classical variable."""
        ...
    @staticmethod
    def value(value: ClassicalValue) -> ClassicalExpr:
        """Create an expression that reads an immutable classical value."""
        ...
    @staticmethod
    def bool_literal(value: bool) -> ClassicalExpr:
        """Create a Bool-typed literal."""
        ...
    @staticmethod
    def bit_literal(value: bool) -> ClassicalExpr:
        """Create a Bit-typed literal."""
        ...
    @staticmethod
    def uint_literal(width: int, value: int) -> ClassicalExpr:
        """Create a UInt-typed literal with the given bit-width."""
        ...
    @staticmethod
    def bit_vec_literal(width: int, value: int) -> ClassicalExpr:
        """Create a BitVec-typed literal with the given bit-width."""
        ...
    @property
    def ty(self) -> ClassicalType:
        """The static :class:`~.classical.ClassicalType` of this expression."""
        ...
    def not_(self) -> ClassicalExpr:
        """Logical NOT (Bool or Bit)."""
        ...
    def and_(self, rhs: ClassicalExpr) -> ClassicalExpr:
        """Logical AND (Bool or Bit)."""
        ...
    def or_(self, rhs: ClassicalExpr) -> ClassicalExpr:
        """Logical OR (Bool or Bit)."""
        ...
    def xor(self, rhs: ClassicalExpr) -> ClassicalExpr:
        """Logical XOR (Bool or Bit)."""
        ...
    def bit_to_bool(self) -> ClassicalExpr:
        """Convert a Bit expression to Bool (0→False, 1→True)."""
        ...
    def to_bool(self) -> ClassicalExpr:
        """Convert a Bit or UInt expression to Bool."""
        ...
    def bit_vec_to_uint(self) -> ClassicalExpr:
        """Convert a BitVec expression to UInt."""
        ...
    def to_uint(self) -> ClassicalExpr:
        """Convert a Bit or BitVec expression to UInt."""
        ...
    @staticmethod
    def equal(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed equality comparison."""
        ...
    @staticmethod
    def not_equal(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed inequality comparison."""
        ...
    @staticmethod
    def lt(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed less-than comparison."""
        ...
    @staticmethod
    def le(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed less-or-equal comparison."""
        ...
    @staticmethod
    def gt(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed greater-than comparison."""
        ...
    @staticmethod
    def ge(lhs: ClassicalExpr, rhs: ClassicalExpr) -> ClassicalExpr:
        """Bool-typed greater-or-equal comparison."""
        ...
    @staticmethod
    def select(condition: ClassicalExpr, then_expr: ClassicalExpr, else_expr: ClassicalExpr) -> ClassicalExpr:
        """Ternary conditional: returns ``then_expr`` if condition is true, else ``else_expr``."""
        ...
    def extract_bit(self, index: int) -> ClassicalExpr:
        """Extract a single bit from a UInt or BitVec expression."""
        ...
    def extract_bits(self, offset: int, width: int) -> ClassicalExpr:
        """Extract a contiguous bit range from a UInt or BitVec expression."""
        ...
    @staticmethod
    def concat(parts: list[ClassicalExpr]) -> ClassicalExpr:
        """Concatenate multiple BitVec expressions into one."""
        ...
    @staticmethod
    def pack_bits(bits: list[ClassicalExpr]) -> ClassicalExpr:
        """Pack individual Bit expressions into a BitVec."""
        ...
    def simplified(self) -> ClassicalExpr:
        """Return an algebraically simplified copy of this expression."""
        ...
    def is_bool_true(self) -> bool:
        """True if this is the constant Bool-typed ``True`` literal."""
        ...
    def is_bool_false(self) -> bool:
        """True if this is the constant Bool-typed ``False`` literal."""
        ...
    def is_bit_true(self) -> bool:
        """True if this is the constant Bit-typed ``1`` literal."""
        ...
    def is_bit_false(self) -> bool:
        """True if this is the constant Bit-typed ``0`` literal."""
        ...
    def __invert__(self) -> ClassicalExpr: ...
    def __and__(self, rhs: ClassicalExpr) -> ClassicalExpr: ...
    def __or__(self, rhs: ClassicalExpr) -> ClassicalExpr: ...
    def __xor__(self, rhs: ClassicalExpr) -> ClassicalExpr: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __copy__(self) -> ClassicalExpr: ...
    def __deepcopy__(self, memo: dict) -> ClassicalExpr: ...
    def __repr__(self) -> str: ...
